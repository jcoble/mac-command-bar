<script lang="ts">
  /**
   * EditorPanel.svelte — the /next code-reading panel.
   *
   * Thin by construction. It owns three things and nothing else:
   *  1. the strip of open files,
   *  2. reading a file when one is requested, and
   *  3. handing `MonacoSourceEditor` the lookup callbacks from
   *     `sourceIntelligence`.
   *
   * Rules it exists to keep:
   *  - **One editor, ever.** Monaco registers its providers for the whole
   *    page, so a second editor anywhere would quietly take over every
   *    lookup. Switching files swaps the file INSIDE this one editor.
   *  - **Nothing loads at start-up.** The panel subscribes to open-file
   *    requests when it mounts (free, no backend), and Monaco itself is only
   *    downloaded once a file is actually opened. The language server is
   *    warmed on the first file opened per project, never before.
   *  - **No `$effect` calls the backend.** Every read is started by a user
   *    action: a file-open request, or a click in the strip.
   */
  import { onMount } from 'svelte';

  import { upgradeUnknownLanguage } from './editor/editorLanguage.ts';
  import {
    createLanguageServerGate,
    readLanguageServerState,
    statusMessageIsAboutThisFile,
    type LanguageServerStatusMessage
  } from './editor/languageServerStatus.ts';
  import FileIcon from './explorer/FileIcon.svelte';
  import LanguageServerStatusChip from './LanguageServerStatusChip.svelte';
  import { onOpenFile, type OpenFileRequest } from '$lib/shell/openFileBus';
  import { hasBackendCapability } from '$lib/shell/backendCapabilities';
  import { countInvoke } from '$lib/shell/devInvokeCounter.svelte';
  import {
    activateEditor,
    activeEditorFile,
    closeEditorFile,
    editorFileFor,
    editorState,
    markEditorFileLoading,
    openEditorFile,
    revealEditorLine,
    setActiveEditorFile,
    setEditorFileDraft,
    setEditorFileError,
    setEditorFilePreview,
    setEditorFileSaving,
    setEditorSymbols
  } from '$lib/shell/editor/editorStore.svelte';
  import { needsRead } from '$lib/shell/editor/editorStoreOps';
  import {
    sourceIntelligence,
    type SourceInlayHintRequest
  } from '$lib/shell/editor/sourceIntelligence';
  import { sourceRecordFromPath } from '$lib/shell/editor/sourceRecordFromPath';
  import {
    isNativeTauriRuntime,
    readSourceFromTauri,
    readSourceLspStatusFromTauri,
    warmSourceLspForRootFromTauri,
    writeSourceToTauri
  } from '$lib/tauriSource';
  import {
    dotnetWorkspaceSessionRequest,
    type DotnetWorkspaceAction,
    type WorkspaceCommandSessionRequest
  } from '$lib/workspaceCodeLens';
  import type MonacoSourceEditor from '$lib/MonacoSourceEditor.svelte';
  import type {
    SourceDiagnostic,
    SourceInlayHint,
    SourceRecord,
    SourceSymbol
  } from '$lib/sourceData';

  /**
   * The code editor is a large download, so it is fetched with the first file
   * the user opens rather than with the shell. The import here is types only —
   * it adds nothing to the page — and the real module arrives in
   * `ensureCodeEditor`.
   */
  interface Props {
    /** Called when an open-file request has become a real open, so the shell can
     * bring this panel's tab to the front. The panel itself stays unaware of the
     * tab area — it just says a file arrived. */
    onFileOpened?: () => void;
    /** Start a fixed workspace command as an ordinary owned terminal session. */
    onStartWorkspaceCommand?: (
      request: WorkspaceCommandSessionRequest
    ) => Promise<string | null>;
  }
  let { onFileOpened, onStartWorkspaceCommand }: Props = $props();

  type CodeEditorComponent = typeof MonacoSourceEditor;
  let CodeEditor = $state<CodeEditorComponent | null>(null);
  let editorLoadError = $state<string | null>(null);
  let loadingEditorComponent = false;
  let nativeCsharpRoot = $state<string | null>(null);
  let nativeCsharpPath = $state<string | null>(null);
  let stopNativeCsharpActions: (() => void) | null = null;
  let stopNativeCsharpDiagnostics: (() => void) | null = null;

  /** Paths whose read is in flight, so a double click cannot read twice. */
  const readsInFlight = new Set<string>();
  /** Projects whose language server has already been pointed at the project. */
  const warmedProjectRoots = new Set<string>();

  let destroyed = false;

  const activeFile = $derived(activeEditorFile());
  const nativeCsharpActive = $derived(
    activeFile?.language === 'csharp' &&
      nativeCsharpRoot === editorState.projectRoot &&
      nativeCsharpPath === activeFile.path
  );

  /**
   * What the language server says is wrong, per file. Kept per file rather than
   * for "the file on screen" because switching tabs must not show the last
   * file's squiggles on this one while a fresh read is still in flight.
   */
  let diagnosticsByPath = $state<Record<string, SourceDiagnostic[]>>({});
  /**
   * How long to wait before asking a second time, in milliseconds.
   *
   * LOAD-BEARING. The language server answers a file it has only just opened
   * with an empty list, so the first read after a file lands usually comes back
   * with nothing and the squiggles appear only when something else happens to
   * ask again. The old shell has hidden this behind the same wait since it was
   * written (`src/routes/+page.svelte`, `scheduleSourceLspDiagnostics`). Without
   * it, "it worked when I clicked around" is the bug report.
   */
  const DIAGNOSTICS_SETTLE_MS = 650;
  /** The pending second read, so switching files quickly does not queue several. */
  let diagnosticsTimer: ReturnType<typeof setTimeout> | null = null;

  /**
   * The last thing the desktop app said about the open file's language server —
   * either the answer to `read_source_lsp_status` or a pushed
   * `source-lsp-status-changed` message. Both carry the same `state` and
   * `detail` fields, so either one can be shown as-is.
   *
   * `null` means nobody has said anything: a browser tab (there is no language
   * server behind a browser) or a desktop build older than these fields. In
   * both cases no chip appears and the panel behaves exactly as it used to.
   */
  let languageServerStatus = $state<unknown>(null);
  /** Which project and language that answer was about, so a stale one is dropped. */
  let languageServerSubject: { root: string; language: string } | null = null;
  /**
   * Holds back work that a server which is still starting up or reading the
   * project could not answer anyway. It lets everything through the moment the
   * server says it is ready — and, if that never happens, on its own time limit.
   */
  const languageServerGate = createLanguageServerGate();
  /** Counts inline-hint requests, so only the newest one survives a wait. */
  let inlayHintRequestCount = 0;

  /** The language of the file on screen, or null when nothing is open. */
  function activeFileLanguage(): string | null {
    return activeEditorFile()?.language ?? null;
  }

  /** Record what the desktop app just said, and let waiting work know. */
  function applyLanguageServerStatus(
    status: unknown,
    subject: { root: string; language: string } | null
  ): void {
    languageServerStatus = status;
    languageServerSubject = subject;
    languageServerGate.setState(readLanguageServerState(status));
  }

  /**
   * Ask what the language server for this file is doing. Called when a file
   * opens or the strip selection changes — never on a timer. Everything after
   * that arrives as a pushed message.
   */
  async function refreshLanguageServerStatus(): Promise<void> {
    const root = editorState.projectRoot;
    const language = activeFileLanguage();

    // A different project or a different language means the last answer was
    // about someone else's server; drop it rather than show it against this file.
    if (
      languageServerSubject &&
      (languageServerSubject.root !== root || languageServerSubject.language !== language)
    ) {
      applyLanguageServerStatus(null, null);
    }

    if (!root || !language || !isNativeTauriRuntime()) return;

    countInvoke('read_source_lsp_status');
    let answer: unknown = null;
    try {
      answer = await readSourceLspStatusFromTauri(root, language);
    } catch {
      // No answer is not a state worth reporting, so the chip stays away.
      answer = null;
    }
    // Superseded: the user moved to another file while this was in flight.
    if (destroyed || editorState.projectRoot !== root || activeFileLanguage() !== language) return;
    applyLanguageServerStatus(answer, { root, language });
  }

  /** Listen for the desktop app telling us the server moved on. */
  async function listenForLanguageServerStatus(): Promise<(() => void) | null> {
    if (!isNativeTauriRuntime()) return null;
    // An older desktop build never sends these, and asking it to listen would
    // leave the panel waiting for a message that cannot arrive.
    if (!(await hasBackendCapability('lspStatusEvents'))) return null;

    const { listen } = await import('@tauri-apps/api/event');
    return listen<LanguageServerStatusMessage>('source-lsp-status-changed', (event) => {
      const root = editorState.projectRoot;
      const language = activeFileLanguage();
      if (!root || !language) return;
      // Several servers can be running at once, so a message about another
      // project or another language must not move this file's chip.
      if (!statusMessageIsAboutThisFile(event.payload, root, language)) return;
      applyLanguageServerStatus(event.payload, { root, language });
    });
  }

  /** Ask what is wrong with the file on screen, and remember it against that file. */
  async function loadDiagnosticsForActiveFile(): Promise<void> {
    const path = editorState.activePath;
    if (!path) return;
    const diagnostics = await sourceIntelligence.loadActiveFileDiagnostics();
    // Superseded: the user moved on while this was in flight, so this answer is
    // about a file that is no longer on screen.
    if (destroyed || editorState.activePath !== path) return;
    diagnosticsByPath = { ...diagnosticsByPath, [path]: diagnostics };
  }

  /** Read the diagnostics now, and once more after the server has settled. */
  function refreshDiagnosticsForActiveFile(): void {
    if (diagnosticsTimer !== null) clearTimeout(diagnosticsTimer);
    diagnosticsTimer = null;
    void loadDiagnosticsForActiveFile();
    scheduleSecondDiagnosticsRead();
  }

  /**
   * Line up the second read.
   *
   * While the server is still starting up or reading the project the second
   * read is pointless — it would come back empty for the same reason the first
   * one did — so it waits for the server to say it is ready and only then
   * starts the settle wait. When nothing is known about the server (an older
   * desktop build, or a browser tab) this is the original behaviour with no
   * delay of any kind added: the timer starts immediately, as it always did.
   */
  function scheduleSecondDiagnosticsRead(): void {
    if (!languageServerGate.isBusy()) {
      startDiagnosticsSettleTimer();
      return;
    }
    const path = editorState.activePath;
    void languageServerGate.waitUntilReady().then(() => {
      if (destroyed || editorState.activePath !== path) return;
      startDiagnosticsSettleTimer();
    });
  }

  function startDiagnosticsSettleTimer(): void {
    if (diagnosticsTimer !== null) clearTimeout(diagnosticsTimer);
    diagnosticsTimer = setTimeout(() => {
      diagnosticsTimer = null;
      if (!destroyed) void loadDiagnosticsForActiveFile();
    }, DIAGNOSTICS_SETTLE_MS);
  }

  /**
   * Inline type hints, held back until the server can actually answer.
   *
   * The editor asks for these the instant a file appears. A server that is
   * still reading the project answers "no hints", the editor believes it, and
   * nothing asks again — which is why hints used to be missing for the rest of
   * the session on a cold start. Making the request WAIT instead of answering
   * it emptily means the same request is answered properly once the server is
   * ready. With nothing known about the server this passes straight through.
   */
  async function lookupInlayHintsWhenServerCanAnswer(
    request: SourceInlayHintRequest
  ): Promise<SourceInlayHint[]> {
    if (languageServerGate.isBusy()) {
      const path = editorState.activePath;
      const ticket = ++inlayHintRequestCount;
      await languageServerGate.waitUntilReady();
      // Only the newest request survives the wait. The editor asks again for
      // every scroll, so answering a whole queue of stale ranges at once would
      // simply move the pile-up to the end of the wait instead of removing it.
      if (destroyed || ticket !== inlayHintRequestCount || editorState.activePath !== path) {
        return [];
      }
    }
    return sourceIntelligence.callbacks.onInlayHintLookup(request);
  }

  function describeError(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  /**
   * The record for a path, with its language filled in when the usual mapping
   * did not recognise the file. SQL is the clearest case: `.sql` was not on the
   * old list, so those files opened as flat grey text even though the editor has
   * had the SQL colouring rules loaded all along. Files the old mapping already
   * knew are untouched.
   *
   * LOAD-BEARING ORDER: the record is what the read wrapper copies the language
   * back out of, so it has to be right before the file is read, not after.
   */
  function recordForPath(path: string): SourceRecord {
    const record = sourceRecordFromPath(editorState.projectRoot, path);
    const language = upgradeUnknownLanguage(record.path, record.language);
    return language === record.language ? record : { ...record, language };
  }

  /** Download the code editor the first time it is needed. */
  async function ensureCodeEditor(): Promise<void> {
    if (loadingEditorComponent) return;
    loadingEditorComponent = true;
    try {
      const root = editorState.projectRoot;
      let native: typeof import('$lib/shell/editor/csharpLanguageClient') | null = null;
      if (root) {
        try {
          const editorServices = await import('$lib/shell/editor/csharpLanguageClient');
          await editorServices.prepareNativeCsharpEditorServices(root);
          if (isNativeTauriRuntime()) native = editorServices;
        } catch (error) {
          console.error('Could not prepare Monaco editor services', error);
        }
      }
      if (!CodeEditor) {
        const module = await import('$lib/MonacoSourceEditor.svelte');
        if (!destroyed) CodeEditor = module.default;
      }
      // Roslyn project loading is deliberately not on the file-rendering path.
      // The editor appears as soon as Monaco is ready; native CodeLens cuts over
      // only after this background attachment has opened the real document.
      if (native && activeFileLanguage() === 'csharp' && editorState.activePath) {
        void ensureNativeCsharpForActiveFile(editorState.activePath, native).catch((error) => {
          console.error('Could not attach the native C# document', error);
        });
      }
    } catch (error) {
      if (!destroyed) editorLoadError = `Could not start the code editor: ${describeError(error)}`;
    } finally {
      loadingEditorComponent = false;
    }
  }

  async function ensureNativeCsharpForActiveFile(
    path = editorState.activePath,
    loadedNative?: typeof import('$lib/shell/editor/csharpLanguageClient')
  ): Promise<void> {
    const root = editorState.projectRoot;
    const entry = path ? editorFileFor(path) : null;
    if (
      !root ||
      !path ||
      entry?.language !== 'csharp' ||
      !isNativeTauriRuntime() ||
      (nativeCsharpRoot === root && nativeCsharpPath === path)
    ) {
      return;
    }
    const native = loadedNative ?? (await import('$lib/shell/editor/csharpLanguageClient'));
    const ensured = await native.ensureNativeCsharpDocument(root, path);
    if (
      destroyed ||
      editorState.projectRoot !== root ||
      editorState.activePath !== path ||
      activeFileLanguage() !== 'csharp'
    ) {
      return;
    }
    nativeCsharpRoot = ensured.root;
    nativeCsharpPath = ensured.path;
    stopNativeCsharpActions?.();
    stopNativeCsharpActions = native.setNativeCsharpDocumentActions({
      build: (documentUri) => runDotnetWorkspaceAction('build', documentUri),
      test: (documentUri) => runDotnetWorkspaceAction('test', documentUri)
    });
    stopNativeCsharpDiagnostics?.();
    stopNativeCsharpDiagnostics = native.subscribeNativeCsharpDiagnostics((diagnostics) => {
      if (destroyed || editorState.projectRoot !== root) return;
      const next = { ...diagnosticsByPath };
      for (const path of Object.keys(next)) {
        if (path.endsWith('.cs')) delete next[path];
      }
      for (const diagnostic of diagnostics) {
        const current = next[diagnostic.path] ?? [];
        next[diagnostic.path] = [...current, diagnostic];
      }
      diagnosticsByPath = next;
    });
  }

  /**
   * Tell the language server which project this file belongs to, once per
   * project. A no-op unless a server is already running for that language.
   */
  function warmLanguageServer(projectRoot: string | null): void {
    if (!projectRoot || warmedProjectRoots.has(projectRoot)) return;
    warmedProjectRoots.add(projectRoot);
    countInvoke('warm_source_lsp_for_root');
    void warmSourceLspForRootFromTauri(projectRoot).catch(() => {
      // Warming is best effort: a failure costs nothing but a cold first lookup.
    });
  }

  /** Keep the lookup service pointed at whatever is on screen. */
  function syncIntelligenceWithActiveFile(): void {
    sourceIntelligence.setProjectRoot(editorState.projectRoot);
    sourceIntelligence.setActivePreview(activeEditorFile()?.preview ?? null);
  }

  // Session restore and project switching update the shared editor store
  // without calling this panel's open/select handlers. Keep the extracted
  // intelligence service attached to that state continuously; otherwise the
  // file can be visible while counts are asked with projectRoot = null.
  $effect(() => {
    editorState.projectRoot;
    editorState.activePath;
    activeEditorFile()?.preview;
    syncIntelligenceWithActiveFile();
  });

  /** EXPLICIT IO: read one file and show it. */
  async function readFileIntoEditor(record: SourceRecord): Promise<void> {
    if (readsInFlight.has(record.path)) return;
    readsInFlight.add(record.path);
    markEditorFileLoading(record.path);
    if (record.language !== 'csharp' || !isNativeTauriRuntime()) {
      warmLanguageServer(editorState.projectRoot);
    }
    try {
      // The record has to be built first: the read wrapper copies the relative
      // path, language and size back out of it onto the preview it returns.
      countInvoke('read_source_file');
      const preview = await readSourceFromTauri(record);
      if (destroyed || !editorFileFor(record.path)) return;
      if (preview) {
        setEditorFilePreview(record.path, preview);
      } else {
        setEditorFileError(record.path, 'This file could not be read from here.');
      }
    } catch (error) {
      if (!destroyed && editorFileFor(record.path)) {
        setEditorFileError(record.path, `Could not read this file: ${describeError(error)}`);
      }
    } finally {
      readsInFlight.delete(record.path);
      syncIntelligenceWithActiveFile();
      void refreshEditorIntelligenceForActiveFile();
    }
  }

  function updateActiveDraft(content: string): void {
    const file = activeEditorFile();
    if (!file) return;
    setEditorFileDraft(file.path, content);
  }

  async function saveActiveFile(): Promise<void> {
    const file = activeEditorFile();
    if (!file?.preview || !file.dirty || file.saving) return;
    const content = file.draftContent ?? file.preview.content;
    setEditorFileSaving(file.path, true);
    try {
      const saved = await writeSourceToTauri(recordForPath(file.path), content);
      if (!saved) throw new Error('The file could not be written from here.');
      if (!destroyed && editorFileFor(file.path)) setEditorFilePreview(file.path, saved);
    } catch (error) {
      if (!destroyed && editorFileFor(file.path)) {
        setEditorFileSaving(file.path, false);
        setEditorFileError(file.path, `Could not save this file: ${describeError(error)}`);
      }
    }
  }

  /**
   * Find out what the language server is doing, then ask it about the file.
   *
   * The order is the point: knowing the server is still reading the project is
   * what lets the panel hold back the work it could not answer yet, instead of
   * firing everything at it the instant a file lands.
   */
  async function refreshEditorIntelligenceForActiveFile(): Promise<void> {
    await refreshLanguageServerStatus();
    if (destroyed) return;
    refreshDiagnosticsForActiveFile();
  }

  /**
   * Open a file by path — the one entry point. Used by the open-file bus, by
   * the strip, and by "jump to definition" landing in another file. Opening a
   * file IS a user action, so it may load even if the panel has not been shown
   * yet; that is also what marks the editor as in use.
   *
   * Returns whether the file was actually taken on — a blank path opens nothing.
   */
  function openPath(
    path: string,
    line?: number | null,
    projectRoot?: string
  ): boolean {
    if (!path.trim()) return false;
    activateEditor(projectRoot);
    const record = recordForPath(path);
    const entry = openEditorFile(record);
    if (typeof line === 'number' && line > 0) revealEditorLine(record.path, line);
    syncIntelligenceWithActiveFile();
    void ensureCodeEditor();
    if (needsRead(entry)) void readFileIntoEditor(record);
    return true;
  }

  function handleOpenFileRequest(request: OpenFileRequest): void {
    // Only once the file is in the strip. The read runs after this and may still
    // fail — the tab is the right place to show that, so it stays in front.
    if (openPath(request.path, request.line, request.projectRoot)) onFileOpened?.();
  }

  function selectOpenFile(path: string): void {
    setActiveEditorFile(path);
    syncIntelligenceWithActiveFile();
    void refreshEditorIntelligenceForActiveFile();
    const entry = editorFileFor(path);
    if (entry?.language === 'csharp') void ensureCodeEditor();
    if (entry && needsRead(entry)) {
      void readFileIntoEditor(recordForPath(path));
    }
  }

  function closeOpenFileAt(path: string): void {
    closeEditorFile(path);
    sourceIntelligence.releasePreview(path);
    syncIntelligenceWithActiveFile();
    // Whatever is in front now may be a different language, with a different
    // server behind it — so the chip must not keep the closed file's answer.
    void refreshLanguageServerStatus();
    // A closed file stops holding its diagnostics; nothing can show them now.
    const { [path]: _closed, ...rest } = diagnosticsByPath;
    diagnosticsByPath = rest;
  }

  function retryRead(path: string): void {
    void readFileIntoEditor(recordForPath(path));
  }

  /** Monaco followed a definition or a reference into another file. */
  function navigateToExternalSource(request: {
    path: string;
    line: number;
    column: number;
  }): void {
    openPath(request.path, request.line);
  }

  function handleSymbolsChange(symbols: SourceSymbol[]): void {
    setEditorSymbols(symbols);
  }

  async function runDotnetWorkspaceAction(
    action: DotnetWorkspaceAction,
    documentUri?: string
  ): Promise<void> {
    const root = editorState.projectRoot;
    if (!root) throw new Error('No workspace is active.');
    if (documentUri) {
      const documentPath = decodeURIComponent(new URL(documentUri).pathname);
      const normalizedRoot = root.replaceAll('\\', '/').replace(/\/+$/, '');
      const normalizedDocument = documentPath.replaceAll('\\', '/');
      if (
        normalizedDocument !== normalizedRoot &&
        !normalizedDocument.startsWith(`${normalizedRoot}/`)
      ) {
        throw new Error('The CodeLens document is outside the active workspace.');
      }
    }
    if (!onStartWorkspaceCommand) {
      throw new Error('The shell has not wired workspace commands to terminal sessions.');
    }

    const ownedId = await onStartWorkspaceCommand(dotnetWorkspaceSessionRequest(action, root));
    if (!ownedId) throw new Error(`No terminal opened for .NET ${action}.`);
  }

  onMount(() => {
    // Subscribing costs nothing and loads nothing; it just means a click in the
    // explorer made before this panel was ever shown still opens its file.
    const unsubscribe = onOpenFile(handleOpenFileRequest);

    // Listening for status updates is likewise free, and it is the only way the
    // chip ever changes after a file opens — nothing here polls.
    let stopStatusUpdates: (() => void) | null = null;
    let panelClosed = false;
    void listenForLanguageServerStatus().then((stop) => {
      if (panelClosed) stop?.();
      else stopStatusUpdates = stop;
    });

    return () => {
      destroyed = true;
      panelClosed = true;
      if (diagnosticsTimer !== null) clearTimeout(diagnosticsTimer);
      diagnosticsTimer = null;
      stopStatusUpdates?.();
      stopNativeCsharpActions?.();
      stopNativeCsharpDiagnostics?.();
      // Anything still waiting on the server has nowhere to go now.
      languageServerGate.releaseAll();
      unsubscribe();
    };
  });
</script>

<div class="editor-panel">
  {#if editorState.openFiles.length === 0}
    <div class="editor-empty">
      <p class="empty-title">No file open</p>
      <p class="empty-hint">Open a file from the explorer or palette.</p>
    </div>
  {:else}
    <div class="editor-header">
      <div class="file-strip" role="tablist" aria-label="Open files">
        {#each editorState.openFiles as file (file.path)}
          <div class="file-chip" class:active={file.path === editorState.activePath}>
            <button
              type="button"
              role="tab"
              aria-selected={file.path === editorState.activePath}
              class="file-name"
              title={file.relativePath}
              onclick={() => selectOpenFile(file.path)}
            >
              <FileIcon fileName={file.fileName} size={13} />
              {file.fileName}
              {#if file.loading}<span class="chip-note">reading</span>{/if}
              {#if file.error}<span class="chip-note error">failed</span>{/if}
            </button>
            <button
              type="button"
              class="file-close"
              aria-label={`Close ${file.fileName}`}
              title={`Close ${file.fileName}`}
              onclick={() => closeOpenFileAt(file.path)}
            >
              ×
            </button>
          </div>
        {/each}
      </div>

      <!-- Nothing renders here in a browser tab or on an older desktop build:
           there is no language server to report on, so there is no chip. -->
      <LanguageServerStatusChip
        language={activeFile?.language ?? null}
        status={languageServerStatus}
      />
    </div>

    <div class="editor-canvas">
      {#if editorLoadError}
        <p class="canvas-message error">{editorLoadError}</p>
      {:else if activeFile?.error}
        <div class="canvas-message error">
          <p>{activeFile.error}</p>
          <button
            type="button"
            class="retry"
            onclick={() => activeFile && retryRead(activeFile.path)}
          >
            Try again
          </button>
        </div>
      {:else if activeFile?.preview}
        {#if CodeEditor}
          {#key nativeCsharpActive}
            <CodeEditor
              {...sourceIntelligence.callbacks}
              onInlayHintLookup={lookupInlayHintsWhenServerCanAnswer}
              preview={activeFile.preview}
              content={activeFile.draftContent ?? activeFile.preview.content}
              editable={true}
              loading={activeFile.loading}
              targetLine={activeFile.targetLine}
              targetLineRequestId={activeFile.targetLineRequestId}
              externalDiagnostics={diagnosticsByPath[activeFile.path] ?? []}
              nativeCsharpLanguageClient={nativeCsharpActive}
              onExternalNavigation={navigateToExternalSource}
              onDotnetBuildRequest={() => runDotnetWorkspaceAction('build')}
              onDotnetTestRequest={() => runDotnetWorkspaceAction('test')}
              onContentChange={updateActiveDraft}
              onSaveRequest={() => void saveActiveFile()}
              onSymbolsChange={handleSymbolsChange}
            />
          {/key}
        {:else}
          <p class="canvas-message">Starting the code editor…</p>
        {/if}
      {:else}
        <p class="canvas-message">Reading {activeFile?.fileName ?? 'file'}…</p>
      {/if}
    </div>

    <div class="editor-status">
      <span class="status-path">{activeFile?.relativePath ?? ''}</span>
      <span class="status-detail">
        {activeFile?.language ?? ''}
        {#if editorState.symbols.length > 0}
          · {editorState.symbols.length}
          {editorState.symbols.length === 1 ? 'symbol' : 'symbols'}
        {/if}
        {#if activeFile?.saving}
          · saving
        {:else if activeFile?.dirty}
          · unsaved
        {:else}
          · editable
        {/if}
      </span>
    </div>
  {/if}
</div>

<style>
  .editor-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    overflow: hidden;
    background: var(--color-bg);
    color: var(--color-text);
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
  }

  .editor-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    height: 100%;
    color: var(--color-text-2);
    font-size: 13px;
    user-select: none;
  }

  .editor-empty p {
    margin: 0;
  }

  .empty-hint {
    color: var(--color-text-3);
    font-size: 12px;
  }

  /* The strip of open files and, pinned to the right, what the language server
   * is doing. The strip scrolls when there are many files; the chip does not go
   * with it, so it stays readable however many tabs are open. */
  .editor-header {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 0 0 auto;
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
    padding: 3px 8px 3px 4px;
    min-width: 0;
  }

  .file-strip {
    display: flex;
    align-items: stretch;
    gap: 2px;
    flex: 1 1 auto;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: thin;
  }

  .file-chip {
    display: flex;
    align-items: center;
    border: 1px solid transparent;
    border-radius: 5px;
    background: transparent;
  }

  .file-chip.active {
    background: var(--color-elevated);
    border-color: var(--color-border);
  }

  .file-name,
  .file-close {
    background: transparent;
    border: none;
    color: var(--color-text-2);
    cursor: pointer;
    font-family: inherit;
    font-size: 12px;
    padding: 3px 4px 3px 8px;
    white-space: nowrap;
  }

  /* The file-type icon sits on the same line as the name, and the gap is what
   * keeps it off the text. */
  .file-name {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .file-close {
    padding: 3px 7px 3px 3px;
    font-size: 13px;
    line-height: 1;
  }

  .file-chip.active .file-name {
    color: var(--color-text);
  }

  .file-name:hover,
  .file-close:hover {
    color: var(--color-text);
  }

  .chip-note {
    color: var(--color-text-3);
    font-size: 12px;
    margin-left: 5px;
  }

  .chip-note.error {
    color: var(--color-bad);
  }

  .editor-canvas {
    position: relative;
    flex: 1 1 auto;
    min-height: 0;
    overflow: hidden;
  }

  .canvas-message {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 100%;
    margin: 0;
    color: var(--color-text-2);
    font-size: 13px;
    text-align: center;
    padding: 0 16px;
  }

  .canvas-message p {
    margin: 0;
  }

  .canvas-message.error {
    color: var(--color-bad);
  }

  .retry {
    background: transparent;
    border: 1px solid var(--color-border);
    border-radius: 5px;
    color: var(--color-text-2);
    cursor: pointer;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 12px;
    padding: 2px 7px;
  }

  .retry:hover {
    color: var(--color-text);
    border-color: var(--color-text-3);
  }

  .editor-status {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex: 0 0 auto;
    background: var(--color-surface);
    border-top: 1px solid var(--color-border);
    color: var(--color-text-2);
    font-family: ui-monospace, Menlo, monospace;
    font-size: 12px;
    padding: 3px 8px;
  }

  .status-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-detail {
    color: var(--color-text-3);
    white-space: nowrap;
  }
</style>
