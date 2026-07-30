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
  import FileIcon from './explorer/FileIcon.svelte';
  import { onOpenFile, type OpenFileRequest } from '$lib/shell/openFileBus';
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
    setEditorFileError,
    setEditorFilePreview,
    setEditorSymbols
  } from '$lib/shell/editor/editorStore.svelte';
  import { needsRead } from '$lib/shell/editor/editorStoreOps';
  import { sourceIntelligence } from '$lib/shell/editor/sourceIntelligence';
  import { sourceRecordFromPath } from '$lib/shell/editor/sourceRecordFromPath';
  import { readSourceFromTauri, warmSourceLspForRootFromTauri } from '$lib/tauriSource';
  import type MonacoSourceEditor from '$lib/MonacoSourceEditor.svelte';
  import type { SourceDiagnostic, SourceRecord, SourceSymbol } from '$lib/sourceData';

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
  }
  let { onFileOpened }: Props = $props();

  type CodeEditorComponent = typeof MonacoSourceEditor;
  let CodeEditor = $state<CodeEditorComponent | null>(null);
  let editorLoadError = $state<string | null>(null);
  let loadingEditorComponent = false;

  /** Paths whose read is in flight, so a double click cannot read twice. */
  const readsInFlight = new Set<string>();
  /** Projects whose language server has already been pointed at the project. */
  const warmedProjectRoots = new Set<string>();

  let destroyed = false;

  const activeFile = $derived(activeEditorFile());

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
    void loadDiagnosticsForActiveFile();
    diagnosticsTimer = setTimeout(() => {
      diagnosticsTimer = null;
      if (!destroyed) void loadDiagnosticsForActiveFile();
    }, DIAGNOSTICS_SETTLE_MS);
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
    if (CodeEditor || loadingEditorComponent) return;
    loadingEditorComponent = true;
    try {
      const module = await import('$lib/MonacoSourceEditor.svelte');
      if (!destroyed) CodeEditor = module.default;
    } catch (error) {
      if (!destroyed) editorLoadError = `Could not start the code editor: ${describeError(error)}`;
    } finally {
      loadingEditorComponent = false;
    }
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

  /** EXPLICIT IO: read one file and show it. */
  async function readFileIntoEditor(record: SourceRecord): Promise<void> {
    if (readsInFlight.has(record.path)) return;
    readsInFlight.add(record.path);
    markEditorFileLoading(record.path);
    warmLanguageServer(editorState.projectRoot);
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
      refreshDiagnosticsForActiveFile();
    }
  }

  /**
   * Open a file by path — the one entry point. Used by the open-file bus, by
   * the strip, and by "jump to definition" landing in another file. Opening a
   * file IS a user action, so it may load even if the panel has not been shown
   * yet; that is also what marks the editor as in use.
   *
   * Returns whether the file was actually taken on — a blank path opens nothing.
   */
  function openPath(path: string, line?: number | null): boolean {
    if (!path.trim()) return false;
    activateEditor();
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
    if (openPath(request.path, request.line)) onFileOpened?.();
  }

  function selectOpenFile(path: string): void {
    setActiveEditorFile(path);
    syncIntelligenceWithActiveFile();
    refreshDiagnosticsForActiveFile();
    const entry = editorFileFor(path);
    if (entry && needsRead(entry)) {
      void readFileIntoEditor(recordForPath(path));
    }
  }

  function closeOpenFileAt(path: string): void {
    closeEditorFile(path);
    sourceIntelligence.invalidatePreview(path);
    syncIntelligenceWithActiveFile();
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

  onMount(() => {
    // Subscribing costs nothing and loads nothing; it just means a click in the
    // explorer made before this panel was ever shown still opens its file.
    const unsubscribe = onOpenFile(handleOpenFileRequest);
    return () => {
      destroyed = true;
      if (diagnosticsTimer !== null) clearTimeout(diagnosticsTimer);
      diagnosticsTimer = null;
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
          <CodeEditor
            {...sourceIntelligence.callbacks}
            preview={activeFile.preview}
            content={activeFile.preview.content}
            editable={false}
            loading={activeFile.loading}
            targetLine={activeFile.targetLine}
            targetLineRequestId={activeFile.targetLineRequestId}
            externalDiagnostics={diagnosticsByPath[activeFile.path] ?? []}
            onExternalNavigation={navigateToExternalSource}
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
        · read only
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

  .file-strip {
    display: flex;
    align-items: stretch;
    gap: 2px;
    flex: 0 0 auto;
    overflow-x: auto;
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
    padding: 3px 4px;
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
