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
   *    downloaded once a file is on screen in front of the reader — not merely
   *    in the strip, because putting a session's files back fills the strip out
   *    of sight. The language server is warmed on the first file opened per
   *    project, never before.
   *  - **Read mode is the default.** Opening a file colours it and stops
   *    there. A language server starts only for a project whose switch in the
   *    top strip has been turned on, and turning it off stops that server. The
   *    choice belongs to the project — one server serves every session and
   *    every view on it — and is remembered between launches.
   *  - **No `$effect` reads a file.** Every read is started by a user action: a
   *    file-open request, or a click in the strip. The one effect that starts
   *    anything starts the editor, and only for a file already on screen.
   */
  import { onMount } from 'svelte';
  import X from '@lucide/svelte/icons/x';

  import { upgradeUnknownLanguage } from './editor/editorLanguage.ts';
  import {
    createLanguageServerGate,
    readLanguageServerState,
    statusMessageIsAboutThisFile,
    type LanguageServerStatusMessage
  } from './editor/languageServerStatus.ts';
  import FileIcon from './explorer/FileIcon.svelte';
  import {
    isMarkdownFile,
    markdownPreviewDefault,
    type MarkdownView
  } from './editor/markdownPreview.ts';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import SourceMarkdownPreview from '$lib/SourceMarkdownPreview.svelte';
  import { SegmentedControl } from '$lib/components/ui/segmented-control/index.js';
  import {
    languageIntelligenceOn,
    launchRestoreFor,
    readLanguageIntelligenceChoices,
    withLanguageIntelligenceChoice,
    writeLanguageIntelligenceChoices,
    workspaceKey,
    type LanguageIntelligenceChoices
  } from '$lib/shell/editor/languageIntelligenceMode';
  import {
    clearLanguageIntelligenceBar,
    publishLanguageIntelligenceBar,
    setLanguageIntelligenceSwitch
  } from '$lib/shell/editor/languageIntelligenceBar.svelte';
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
    setWorkspaceLanguageIntelligenceFromTauri,
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
    /** Whether this panel is the center tab in front. Putting a session's files
     * back opens them out of sight, and the code editor is far too expensive to
     * start for a file nobody is looking at. */
    showing?: boolean;
    /** Start a fixed workspace command as an ordinary owned terminal session. */
    onStartWorkspaceCommand?: (
      request: WorkspaceCommandSessionRequest
    ) => Promise<string | null>;
  }
  let { onFileOpened, showing = false, onStartWorkspaceCommand }: Props = $props();

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

  /** Where the remembered per-project choices live, when there is anywhere. */
  function modeStorage(): Storage | null {
    return typeof localStorage === 'undefined' ? null : localStorage;
  }

  /** The remembered choices. Read once; every change writes them back. */
  let languageIntelligenceChoices = $state<LanguageIntelligenceChoices>(
    readLanguageIntelligenceChoices(modeStorage())
  );
  /** What the desktop app last said about this project's mode. */
  let languageIntelligenceNote = $state<string | null>(null);
  /** True while a switch is being acted on, so it cannot be flipped twice. */
  let languageIntelligenceBusy = $state(false);
  /** Language-server processes the desktop app reports for this project. */
  let languageServerPids = $state<number[]>([]);
  /**
   * Projects whose saved choice has been handed to the desktop app, with the
   * handing-over itself so callers can wait for it. One project at a time and
   * only when it is opened: restoring the whole remembered list at launch would
   * wake servers for projects nobody is looking at.
   */
  const modeRestores = new Map<string, Promise<void>>();

  let destroyed = false;

  const activeFile = $derived(activeEditorFile());
  /** Is this project in full mode — language server allowed to run? */
  const fullMode = $derived(
    languageIntelligenceOn(languageIntelligenceChoices, editorState.projectRoot)
  );
  /**
   * The sentence on hover: what the mode means for this project, and the
   * process numbers behind it so they can be found in the resource view.
   */
  const languageIntelligenceTitle = $derived.by(() => {
    if (!editorState.projectRoot) return 'Open a file in a project to switch this on.';
    const note =
      languageIntelligenceNote ??
      (fullMode
        ? 'Language intelligence is on for this project. One language server serves every session and view on it.'
        : 'Read mode: files open with colouring only, and no language server is started.');
    if (languageServerPids.length === 0) return note;
    const numbers = languageServerPids.join(', ');
    const noun = languageServerPids.length === 1 ? 'Process' : 'Processes';
    return `${note} ${noun} ${numbers}.`;
  });
  /**
   * Which view each Markdown file is on. A file is written in here the first
   * time it opens — rendered when it was reached from a jump, source when it
   * was picked in the strip — and the toggle in the header writes it again.
   * Keyed by path so switching tabs comes back to what was on screen.
   */
  let markdownViewByPath = $state<Record<string, MarkdownView>>({});
  const activeFileIsMarkdown = $derived(isMarkdownFile(activeFile?.fileName));
  const markdownView = $derived(
    activeFile && activeFileIsMarkdown ? (markdownViewByPath[activeFile.path] ?? 'raw') : 'raw'
  );
  const MARKDOWN_VIEW_ITEMS = [
    { value: 'raw', label: 'Source' },
    { value: 'rendered', label: 'Preview' }
  ] as const;

  /** Record the first view for a file, without overwriting a person's choice. */
  function rememberMarkdownDefault(path: string, fileName: string, origin: 'jump' | 'strip'): void {
    if (!isMarkdownFile(fileName) || markdownViewByPath[path]) return;
    markdownViewByPath = {
      ...markdownViewByPath,
      [path]: markdownPreviewDefault(fileName, origin)
    };
  }

  function setMarkdownView(view: MarkdownView): void {
    const path = activeFile?.path;
    if (!path) return;
    markdownViewByPath = { ...markdownViewByPath, [path]: view };
  }

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
          // Preparing the editor services starts no server — it is what lets
          // Monaco open a file at all — so it runs in both modes.
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
    // Attaching the document is what brings Roslyn up. In read mode the file is
    // shown with colouring and nothing is started.
    await applySavedModeForProject(root);
    if (destroyed || !languageIntelligenceOn(languageIntelligenceChoices, root)) return;
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
   * Hand the desktop app this project's remembered choice, once per project.
   *
   * Called when a file in the project is opened — never at start-up for every
   * project that was ever switched on. Saying "on" starts nothing by itself;
   * the file being opened is what does that, and only in full mode.
   */
  function applySavedModeForProject(projectRoot: string): Promise<void> {
    const root = workspaceKey(projectRoot);
    const already = modeRestores.get(root);
    if (already) return already;
    const restore = (async () => {
      const decision = launchRestoreFor(languageIntelligenceChoices, root);
      if (!decision || !isNativeTauriRuntime()) return;
      countInvoke('set_workspace_language_intelligence');
      try {
        const answer = await setWorkspaceLanguageIntelligenceFromTauri(
          decision.root,
          decision.enabled
        );
        if (destroyed || !answer) return;
        if (editorState.projectRoot && workspaceKey(editorState.projectRoot) === root) {
          languageServerPids = answer.serverPids;
        }
      } catch {
        // A desktop build that has never heard of the switch leaves every
        // project in read mode, which is the safe half of the choice.
      }
    })();
    modeRestores.set(root, restore);
    return restore;
  }

  /**
   * Tell the language server which project this file belongs to, once per
   * project. A no-op unless a server is already running for that language,
   * and never called at all for a project in read mode.
   */
  async function warmLanguageServer(projectRoot: string | null): Promise<void> {
    if (!projectRoot) return;
    await applySavedModeForProject(projectRoot);
    if (destroyed || !languageIntelligenceOn(languageIntelligenceChoices, projectRoot)) return;
    if (warmedProjectRoots.has(projectRoot)) return;
    warmedProjectRoots.add(projectRoot);
    countInvoke('warm_source_lsp_for_root');
    try {
      await warmSourceLspForRootFromTauri(projectRoot);
    } catch (error) {
      // Warming only costs a cold first lookup, but a reader who has the switch
      // on is owed the reason rather than a project that quietly stays cold.
      if (!destroyed) {
        languageIntelligenceNote = `This project's language server could not be warmed: ${describeError(error)}`;
      }
    }
  }

  /**
   * Turn language intelligence on or off for the project on screen.
   *
   * Off closes this project's language client first — that is what sends the
   * server its goodbye — and then asks the desktop app to stop the process and
   * reclaim its memory. On records the choice and starts the server for the
   * file already open, saying so — including when there is no server to start.
   */
  async function switchLanguageIntelligence(enabled: boolean): Promise<void> {
    const root = editorState.projectRoot;
    if (!root || languageIntelligenceBusy) return;
    languageIntelligenceBusy = true;
    try {
      if (!enabled && isNativeTauriRuntime()) {
        try {
          const native = await import('$lib/shell/editor/csharpLanguageClient');
          await native.stopNativeCsharpLanguageClient(root);
        } catch {
          // No client to close, or it could not be closed cleanly. The desktop
          // app stops the process either way.
        }
        nativeCsharpRoot = null;
        nativeCsharpPath = null;
        stopNativeCsharpActions?.();
        stopNativeCsharpActions = null;
        stopNativeCsharpDiagnostics?.();
        stopNativeCsharpDiagnostics = null;
      }

      // Remembered first, so the switch keeps its position even if the desktop
      // app is an older build that cannot act on it.
      languageIntelligenceChoices = withLanguageIntelligenceChoice(
        languageIntelligenceChoices,
        root,
        enabled
      );
      writeLanguageIntelligenceChoices(languageIntelligenceChoices, modeStorage());
      modeRestores.set(workspaceKey(root), Promise.resolve());
      if (!enabled) {
        warmedProjectRoots.delete(root);
        diagnosticsByPath = {};
      }

      // Settings is where the C# server is switched off, and that holds. This
      // switch used to lift that setting back on whenever it was flipped on for
      // a project — so turning the server off in Settings kept undoing itself,
      // and the two switches read as one that "would not stay off". A C# start
      // that Settings has refused now says so in the note under the switch.

      countInvoke('set_workspace_language_intelligence');
      // The file already on screen is the one the reader wants answered, so its
      // language is what the desktop app starts a server for — now, rather than
      // at the next file opened. With nothing open there is nothing to start,
      // and the answer says so.
      const answer = await setWorkspaceLanguageIntelligenceFromTauri(
        root,
        enabled,
        enabled ? activeFileLanguage() : null
      );
      if (destroyed) return;
      languageIntelligenceNote = answer?.message ?? null;
      languageServerPids = answer?.serverPids ?? [];

      if (enabled) {
        await warmLanguageServer(root);
        if (destroyed) return;
        void ensureCodeEditor();
        // C# comes up through its own client, and a failure there is the reader's
        // to see: the switch is on, so an empty editor with no explanation is the
        // one thing this must not do.
        await ensureNativeCsharpForActiveFile().catch((error) => {
          if (!destroyed) {
            languageIntelligenceNote = `The C# language server could not be started: ${describeError(error)}`;
          }
        });
      }
      if (!destroyed) void refreshEditorIntelligenceForActiveFile();
    } catch (error) {
      if (!destroyed) {
        languageIntelligenceNote = `The switch could not be changed: ${describeError(error)}`;
      }
    } finally {
      if (!destroyed) languageIntelligenceBusy = false;
    }
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
      void warmLanguageServer(editorState.projectRoot);
    } else {
      // C# warms through its own client, but the saved choice still has to
      // reach the desktop app before anything asks it for an endpoint.
      const root = editorState.projectRoot;
      if (root) void applySavedModeForProject(root);
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
    projectRoot?: string,
    origin: 'jump' | 'strip' = 'jump'
  ): boolean {
    if (!path.trim()) return false;
    activateEditor(projectRoot);
    const record = recordForPath(path);
    const entry = openEditorFile(record);
    rememberMarkdownDefault(record.path, entry.fileName, origin);
    if (typeof line === 'number' && line > 0) revealEditorLine(record.path, line);
    syncIntelligenceWithActiveFile();
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
    // A file restored into the strip never went through `openPath`, so this is
    // where it gets its first view. Picking a tab is editing, not reading.
    if (entry) rememberMarkdownDefault(path, entry.fileName, 'strip');
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

  /**
   * Fetch and start the code editor for the file on screen.
   *
   * This is the only route to `ensureCodeEditor` other than the language switch,
   * and it is deliberately tied to the panel being in front rather than to a
   * file arriving in the strip. Starting the editor means starting Monaco and
   * the VS Code service container with it, which is seconds of work in the
   * desktop app's webview — putting a session's files back used to pay all of it
   * at launch, out of sight, with every click dead while it ran.
   */
  $effect(() => {
    if (showing && editorState.activePath) void ensureCodeEditor();
  });

  /**
   * Keep the top strip's copy of the language-server controls in step.
   *
   * It calls nothing: it copies state the panel already holds into the shared
   * module the top strip reads, so the chip and the switch can sit beside the
   * run button while this panel stays their only owner. The status pipeline is
   * not run twice.
   */
  $effect(() => {
    publishLanguageIntelligenceBar({
      language: activeFile?.language ?? null,
      status: languageServerStatus,
      fullMode,
      busy: languageIntelligenceBusy,
      hasProject: Boolean(editorState.projectRoot),
      title: languageIntelligenceTitle
    });
  });

  onMount(() => {
    // Subscribing costs nothing and loads nothing; it just means a click in the
    // explorer made before this panel was ever shown still opens its file.
    const unsubscribe = onOpenFile(handleOpenFileRequest);

    // The switch in the top strip is this panel's own; flipping it there runs
    // exactly the same code as flipping it here once did.
    setLanguageIntelligenceSwitch((enabled) => void switchLanguageIntelligence(enabled));

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
      // With no panel there is nothing truthful to show in the top strip.
      clearLanguageIntelligenceBar();
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
            <IconButton
              label={`Close ${file.fileName}`}
              size="sm"
              side="bottom"
              class="file-close text-[var(--color-text-2)] hover:bg-[var(--color-elevated)] hover:text-[var(--color-text)]"
              onclick={() => closeOpenFileAt(file.path)}
            >
              <X class="size-3.5" aria-hidden="true" />
            </IconButton>
          </div>
        {/each}
      </div>

      <!-- Only the open file's own controls belong here. The project's
           language-server chip and switch sit in the strip along the top of the
           shell, in `LanguageIntelligenceControls.svelte`. -->
      <div class="editor-controls">
        <!-- Markdown reads two ways, so the file says which one it is on. Source
             is the ordinary editor; Preview is the same document rendered. -->
        {#if activeFileIsMarkdown}
          <span data-testid="markdown-view-toggle" class="shrink-0">
            <SegmentedControl
              size="sm"
              items={MARKDOWN_VIEW_ITEMS}
              value={markdownView}
              aria-label="Markdown view"
              onValueChange={(value) => setMarkdownView(value as MarkdownView)}
            />
          </span>
        {/if}
      </div>
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
      {:else if activeFile?.preview && activeFileIsMarkdown && markdownView === 'rendered'}
        <SourceMarkdownPreview
          content={activeFile.draftContent ?? activeFile.preview.content}
          fileName={activeFile.fileName}
          relativePath={activeFile.relativePath}
          dirty={activeFile.dirty ?? false}
        />
      {:else if activeFile?.preview}
        {#if CodeEditor}
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
    min-width: 0;
    overflow: hidden;
    /* No fill of its own. This is a panel body in normal flow covering nothing,
       and filling it with the backdrop colour painted a black rectangle over
       the card underneath — which carries the surface colour and the light
       gradient down its top edge. Most visible with no file open, where the
       body IS the whole panel. The header and status rows still state their own
       surface, because those are bands rather than the body. */
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

  /* The strip of open files and, pinned to the right, the controls that belong
   * to the open file. The strip scrolls when there are many files; those
   * controls do not go with it, so they stay reachable however many tabs are
   * open.
   *
   * The height is stated rather than left to the tallest child because the
   * centre pane's pill tabs are placed directly beneath this row and read the
   * same value. It is what the row already measured — a 28px close button
   * between 3px of padding, over a hairline — so nothing moves. */
  .editor-header {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 0 0 auto;
    box-sizing: border-box;
    width: 100%;
    min-width: 0;
    height: var(--editor-tab-row-height);
    overflow: hidden;
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
    padding: 3px 8px 3px 4px;
  }

  .file-strip {
    display: flex;
    align-items: stretch;
    gap: 2px;
    flex: 1 1 auto;
    min-width: 0;
    overflow-x: auto;
    overflow-y: hidden;
    /* No visible bar, the same as the right panel's tab strip. A `thin` bar is
       still a bar that CLAIMS HEIGHT on a Mac set to show scrollbars always:
       once the tabs overflowed, the row grew by those pixels and everything
       positioned against its stated height landed inside it instead of below.
       Chrome's overlay bars hide that, which is why it survived a browser
       check. Scrolling by wheel and drag is unaffected. */
    scrollbar-width: none;
  }

  .file-strip::-webkit-scrollbar {
    display: none;
  }

  .file-chip {
    display: flex;
    align-items: center;
    flex: 0 0 auto;
    border: 1px solid transparent;
    border-radius: 5px;
    background: transparent;
  }

  .file-chip.active {
    background: var(--color-elevated);
    border-color: var(--color-border);
  }

  .file-name {
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

  .file-chip.active .file-name {
    color: var(--color-text);
  }

  .file-name:hover {
    color: var(--color-text);
  }

  /* The complete right edge is one non-shrinking sibling of the scrollable
   * file strip. Tabs can move underneath their own clip, but these controls
   * retain their full width and never enter that scrolling region. */
  .editor-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 0 0 auto;
    min-width: max-content;
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
    background: var(--color-elevated);
    border: 0;
    border-radius: 5px;
    color: var(--color-text-2);
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 2px 7px;
  }

  .retry:hover {
    color: var(--color-text);
    background: var(--color-hover);
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
    font-family: var(--font-mono);
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
