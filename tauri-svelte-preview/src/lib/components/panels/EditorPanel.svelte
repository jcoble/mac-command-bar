<script lang="ts">
  /**
   * EditorPanel.svelte — the "Source editor" dock: the per-file tab stack, the
   * editor toolbar, and the editor canvas (Monaco source editor or the Markdown
   * preview).
   *
   * Presentational only: it renders chrome + the Monaco/Markdown surface and
   * emits every action through callbacks. It holds NO `$state` of its own. The
   * page owns all editor/files/LSP state, the 24 Monaco lookup handlers, the
   * Tauri save/load calls, and every Dockview teleport action; they are passed
   * in as props.
   *
   * Teleport: the root `<section>` keeps `use:panelAction={'editor'}` and each
   * open tab's pane keeps `use:filePanelAction={…}`. The actions stay in the
   * page (they wire the nested Dockview workspaces) and are supplied as props.
   *
   * Insights seam: the git/intelligence aside is a SEPARATE Dockview panel
   * (`use:sourceDockviewPanelAction={'insights'}`) that physically nests inside
   * `.editor-body-grid`. To keep its teleport registration + all git state and
   * git CSS in the page, the page passes that whole subtree as the `insights`
   * snippet, which we `{@render}` inside the body grid.
   */
  import type { Action } from 'svelte/action';
  import type { ComponentProps, Snippet } from 'svelte';
  import {
    Activity,
    BookOpen,
    Braces,
    Check,
    Copy,
    ExternalLink,
    FileCode2,
    FolderGit2,
    FolderSearch,
    MoreHorizontal,
    RefreshCw,
    RotateCcw,
    Save,
    SplitSquareHorizontal,
    Terminal,
    X
  } from '@lucide/svelte';
  import MonacoSourceEditor from '$lib/MonacoSourceEditor.svelte';
  import SourceMarkdownPreview from '$lib/SourceMarkdownPreview.svelte';
  import SourceDockviewShell from '$lib/SourceDockviewShell.svelte';
  import Chip from '$lib/components/Chip.svelte';
  import type { SourceDockPanelID } from '$lib/sourceDockLayout';
  import type { SourceEditorFilePanelID } from '$lib/sourceEditorPanels';
  import type {
    ProjectGitFileStatus,
    ProjectRoot,
    SourceOpenTab,
    SourcePreview,
    SourceRecord
  } from '$lib/sourceData';

  type MonacoProps = ComponentProps<typeof MonacoSourceEditor>;
  type SourceEditorDisplayMode = 'source' | 'preview';
  /** The Markdown action a toolbar item requests (Format / Rename / Quick fix). */
  type SourceIntelligenceAction = 'format' | 'rename' | 'quick-fix';

  /**
   * The 24 Monaco lookup/edit callbacks, grouped into one object prop so the
   * panel surface stays manageable. Typed against MonacoSourceEditor itself so
   * the handlers stay in lockstep with the editor component.
   */
  type EditorMonacoCallbacks = Pick<
    MonacoProps,
    | 'onCodeActionLookup'
    | 'onContentChange'
    | 'onCommandPaletteRequest'
    | 'onCompletionLookup'
    | 'onDiagnosticsChange'
    | 'onDefinitionLookup'
    | 'onDocumentHighlightLookup'
    | 'onExternalNavigation'
    | 'onExternalPreviewLookup'
    | 'onFormatDocument'
    | 'onGoToLineRequest'
    | 'onHoverLookup'
    | 'onImplementationLookup'
    | 'onInlayHintLookup'
    | 'onNavigateBackRequest'
    | 'onNavigateForwardRequest'
    | 'onNextProblemRequest'
    | 'onPreviousProblemRequest'
    | 'onQuickOpenRequest'
    | 'onReferenceCountLookup'
    | 'onReferenceLookup'
    | 'onRename'
    | 'onSaveRequest'
    | 'onSemanticTokensLookup'
    | 'onSignatureHelpLookup'
    | 'onSymbolsChange'
    | 'onTypeDefinitionLookup'
    | 'onWorkspaceEditAction'
  >;

  interface Props {
    // ── teleport actions (kept on the root + per-tab pane nodes) ──
    /** Dockview teleport action for the `'editor'` panel — the root node. */
    panelAction: Action<HTMLElement, SourceDockPanelID>;
    /** Dockview teleport action for each per-file pane (nested tab stack). */
    filePanelAction: Action<HTMLElement, SourceEditorFilePanelID>;
    /** Maps an open tab to its `file:${path}` panel id. */
    filePanelID: (tab: Pick<SourceOpenTab, 'path'>) => SourceEditorFilePanelID;
    /** Nested editor-files Dockview shell wiring. */
    fileDockviewReady: boolean;
    fileDockviewError: string;
    fileDockviewHostAction: Action<HTMLElement>;

    // ── tabs / selection / preview (store-backed values) ──
    openTabs: SourceOpenTab[];
    selectedPath: string | null | undefined;
    preview: SourcePreview | null;
    draftContent: string;
    loading: boolean;
    selectedSourceLine: number | null;
    selectedSourceLineRequestId: number;
    fileActionStatus: string;
    fileActionBusy: string;
    error: string;
    selectedIndex: number;
    recordCount: number;
    /** The selected project (passed to `onLoadLspStatus`). */
    selectedProject: ProjectRoot;

    // ── dirty / counts ──
    isDirty: (path: string) => boolean;
    selectedDirty: boolean;
    dirtyCount: number;
    cleanTabCount: number;
    otherCleanTabCount: number;

    // ── markdown / display mode ──
    markdownPreviewAvailable: boolean;
    displayMode: SourceEditorDisplayMode;
    /** Bumped to force-remount Monaco when editor appearance changes. */
    appearanceKey: string;
    appearanceOverride: MonacoProps['appearanceOverride'];

    // ── LSP / intelligence inputs ──
    intelligenceAvailable: boolean;
    intelligenceCommand: MonacoProps['intelligenceCommand'];
    lspDiagnostics: MonacoProps['externalDiagnostics'];
    lspStatus: { available?: boolean } | null;
    lspStatusLoading: boolean;
    lspStatusLabel: () => string;
    lspStatusTitle: () => string;
    lspInstallCommand: () => string;
    gitBadge: () => string;

    // ── insight toggle (right-rail Git panel) ──
    insightCollapsed: boolean;
    /** Editor-action overflow menu open state (page-owned `$state`). */
    actionMenuOpen: boolean;

    // ── grouped Monaco passthrough callbacks ──
    monaco: EditorMonacoCallbacks;

    // ── callbacks (all page-owned) ──
    onSelectTab: (tab: SourceOpenTab) => void;
    gitStatusForRecord: (record: SourceRecord | SourceOpenTab | null) => ProjectGitFileStatus | null;
    onSetDisplayMode: (mode: SourceEditorDisplayMode) => void;
    onToggleInsightCollapsed: () => void;
    onToggleActionMenu: () => void;
    onCloseActionMenu: () => void;
    onSave: () => void;
    onSaveAll: () => void;
    onCloseTab: () => void;
    onCloseOtherClean: () => void;
    onCloseAllClean: () => void;
    onIntelligenceAction: (action: SourceIntelligenceAction) => void;
    onRevert: () => void;
    onShowInsightPanel: (panel: 'git') => void;
    onCopyPath: () => void;
    onOpenInIDE: () => void;
    onRevealFile: () => void;
    onLoadLspStatus: (preview: SourcePreview | null, project: ProjectRoot) => void;
    onCopyLspReport: () => void;
    onCopyLspInstall: () => void;

    /** Insights/git panel subtree (a SEPARATE teleport panel; page owns it). */
    insights: Snippet;
  }

  let {
    panelAction,
    filePanelAction,
    filePanelID,
    fileDockviewReady,
    fileDockviewError,
    fileDockviewHostAction,
    openTabs,
    selectedPath,
    preview,
    draftContent,
    loading,
    selectedSourceLine,
    selectedSourceLineRequestId,
    fileActionStatus,
    fileActionBusy,
    error,
    selectedIndex,
    recordCount,
    selectedProject,
    isDirty,
    selectedDirty,
    dirtyCount,
    cleanTabCount,
    otherCleanTabCount,
    markdownPreviewAvailable,
    displayMode,
    appearanceKey,
    appearanceOverride,
    intelligenceAvailable,
    intelligenceCommand,
    lspDiagnostics,
    lspStatus,
    lspStatusLoading,
    lspStatusLabel,
    lspStatusTitle,
    lspInstallCommand,
    gitBadge,
    insightCollapsed,
    actionMenuOpen,
    monaco,
    onSelectTab,
    gitStatusForRecord,
    onSetDisplayMode,
    onToggleInsightCollapsed,
    onToggleActionMenu,
    onCloseActionMenu,
    onSave,
    onSaveAll,
    onCloseTab,
    onCloseOtherClean,
    onCloseAllClean,
    onIntelligenceAction,
    onRevert,
    onShowInsightPanel,
    onCopyPath,
    onOpenInIDE,
    onRevealFile,
    onLoadLspStatus,
    onCopyLspReport,
    onCopyLspInstall,
    insights
  }: Props = $props();
</script>

<section class="source-editor-dock-panel" aria-label="Source editor" use:panelAction={'editor'}>

  {#if openTabs.length > 0}
    <SourceDockviewShell
      shellClass="source-dockview-editor-files-shell"
      hostClass="source-dockview-editor-files-host"
      errorClass="source-dockview-editor-files-error"
      enabled={true}
      ready={fileDockviewReady}
      error={fileDockviewError}
      hostAction={fileDockviewHostAction}
    >
      {#each openTabs as tab (tab.path)}
        {@const tabGitStatus = gitStatusForRecord(tab)}
        <section
          class="source-editor-file-pane"
          class:active={tab.path === selectedPath}
          class:dirty={isDirty(tab.path)}
          aria-label={`Source editor for ${tab.fileName}`}
          data-source-path={tab.path}
          use:filePanelAction={filePanelID(tab)}
        >
          {#if tab.path === selectedPath}

  {#if fileActionStatus}
    <div class="file-action-feedback">{fileActionStatus}</div>
  {/if}

  {#if error}
    <div class="inline-error">
      <Activity size={15} strokeWidth={1.8} />
      <span>{error}</span>
    </div>
  {/if}

  {#if preview}
    <div class="editor-frame" class:is-loading={loading}>
      <div class="editor-toolbar" aria-label="Editor controls">
        <div class="editor-file-state" title={preview.relativePath}>
          <span class="editor-file-glyph" aria-hidden="true">
            <FileCode2 size={13} strokeWidth={1.8} />
          </span>
          <div class="editor-file-title">
            <strong>{preview.fileName}</strong>
            <small>
              {selectedIndex} / {recordCount}
              {#if selectedSourceLine}
                · line {selectedSourceLine}
              {/if}
            </small>
          </div>
          {#if selectedDirty}
            <Chip size="xs" tone="attention">modified</Chip>
          {/if}
          {#if intelligenceAvailable}
            <span class="editor-lsp-chip" title={lspStatusTitle()}>
              <Chip
                size="xs"
                tone={lspStatus?.available ? 'good' : (!lspStatusLoading ? 'attention' : 'muted')}
              >
                {lspStatusLabel()}
              </Chip>
            </span>
          {/if}
        </div>
        {#if intelligenceAvailable}
          <div class="editor-lsp-recovery-strip" aria-label="Language server recovery actions">
            <button
              class="editor-lsp-recovery-action"
              type="button"
              aria-label="Retry language server status"
              title="Retry language server status"
              disabled={lspStatusLoading}
              onclick={() => onLoadLspStatus(preview, selectedProject)}
            >
              <RefreshCw size={13} strokeWidth={2} />
            </button>
            <button
              class="editor-lsp-recovery-action"
              type="button"
              aria-label="Copy language server status"
              title="Copy language server status"
              onclick={onCopyLspReport}
            >
              <Copy size={13} strokeWidth={2} />
            </button>
            <button
              class="editor-lsp-recovery-action"
              type="button"
              aria-label="Copy language server install command"
              title="Copy language server install command"
              disabled={lspStatus?.available || !lspInstallCommand()}
              onclick={onCopyLspInstall}
            >
              <Terminal size={13} strokeWidth={2} />
            </button>
          </div>
        {/if}
        {#if markdownPreviewAvailable}
          <div class="editor-mode-toggle" role="tablist" aria-label="Markdown editor mode">
            <button
              class:active={displayMode === 'source'}
              type="button"
              role="tab"
              aria-selected={displayMode === 'source'}
              title="Edit Markdown source"
              onclick={() => onSetDisplayMode('source')}
            >
              <FileCode2 size={12} strokeWidth={2} />
              <span>Source</span>
            </button>
            <button
              class:active={displayMode === 'preview'}
              type="button"
              role="tab"
              aria-selected={displayMode === 'preview'}
              title="Preview Markdown"
              onclick={() => onSetDisplayMode('preview')}
            >
              <BookOpen size={12} strokeWidth={2} />
              <span>Preview</span>
            </button>
          </div>
        {/if}
        <div class="editor-menu-anchor">
          <button
            class="editor-icon-button"
            class:active={!insightCollapsed}
            type="button"
            aria-label={insightCollapsed ? 'Show editor insights' : 'Hide editor insights'}
            title={insightCollapsed ? 'Show editor insights' : 'Hide editor insights'}
            onclick={onToggleInsightCollapsed}
          >
            <SplitSquareHorizontal size={14} strokeWidth={2} />
          </button>
          <button
            class="editor-icon-button"
            type="button"
            aria-label="Editor actions"
            aria-haspopup="menu"
            aria-expanded={actionMenuOpen}
            title="Editor actions"
            onclick={onToggleActionMenu}
          >
            <MoreHorizontal size={15} strokeWidth={2} />
          </button>
          {#if actionMenuOpen}
            <div class="editor-action-menu" role="menu" aria-label="Editor actions">
              <button
                type="button"
                role="menuitem"
                aria-label="Save source file"
                disabled={!selectedDirty || fileActionBusy === 'save'}
                onclick={() => {
                  onCloseActionMenu();
                  onSave();
                }}
              >
                <Save size={13} strokeWidth={2} />
                <span>Save</span>
                <kbd>Cmd+S</kbd>
              </button>
              <button
                type="button"
                role="menuitem"
                aria-label="Save all source files"
                disabled={dirtyCount === 0 || fileActionBusy === 'save-all'}
                onclick={() => {
                  onCloseActionMenu();
                  onSaveAll();
                }}
              >
                <Save size={13} strokeWidth={2} />
                <span>Save all</span>
                <kbd>{dirtyCount}</kbd>
              </button>
              <button
                type="button"
                role="menuitem"
                aria-label="Close current source tab"
                disabled={!preview}
                onclick={() => {
                  onCloseActionMenu();
                  onCloseTab();
                }}
              >
                <X size={13} strokeWidth={2} />
                <span>Close tab</span>
              </button>
              <button
                type="button"
                role="menuitem"
                aria-label="Close other clean source tabs"
                disabled={!preview || otherCleanTabCount === 0}
                onclick={() => {
                  onCloseActionMenu();
                  onCloseOtherClean();
                }}
              >
                <X size={13} strokeWidth={2} />
                <span>Close other clean</span>
                <kbd>{otherCleanTabCount}</kbd>
              </button>
              <button
                type="button"
                role="menuitem"
                aria-label="Close all clean source tabs"
                disabled={cleanTabCount === 0}
                onclick={() => {
                  onCloseActionMenu();
                  onCloseAllClean();
                }}
              >
                <X size={13} strokeWidth={2} />
                <span>Close clean tabs</span>
                <kbd>{cleanTabCount}</kbd>
              </button>
              <button
                type="button"
                role="menuitem"
                aria-label="Format source file"
                disabled={!preview || loading || !intelligenceAvailable}
                onclick={() => {
                  onCloseActionMenu();
                  onIntelligenceAction('format');
                }}
              >
                <Braces size={13} strokeWidth={2} />
                <span>Format</span>
              </button>
              <button
                type="button"
                role="menuitem"
                aria-label="Rename symbol"
                disabled={!preview || loading || !intelligenceAvailable}
                onclick={() => {
                  onCloseActionMenu();
                  onIntelligenceAction('rename');
                }}
              >
                <Braces size={13} strokeWidth={2} />
                <span>Rename</span>
                <kbd>F2</kbd>
              </button>
              <button
                type="button"
                role="menuitem"
                aria-label="Quick fix"
                disabled={!preview || loading || !intelligenceAvailable}
                onclick={() => {
                  onCloseActionMenu();
                  onIntelligenceAction('quick-fix');
                }}
              >
                <Activity size={13} strokeWidth={2} />
                <span>Quick fix</span>
                <kbd>Alt+Enter</kbd>
              </button>
              <button
                type="button"
                role="menuitem"
                aria-label="Revert source file"
                disabled={!selectedDirty || fileActionBusy === 'save'}
                onclick={() => {
                  onCloseActionMenu();
                  onRevert();
                }}
              >
                <RotateCcw size={13} strokeWidth={2} />
                <span>Revert</span>
              </button>
              <hr />
              <button
                type="button"
                role="menuitem"
                aria-label={insightCollapsed ? 'Show editor insights' : 'Hide editor insights'}
                onclick={() => {
                  onCloseActionMenu();
                  onToggleInsightCollapsed();
                }}
              >
                <SplitSquareHorizontal size={13} strokeWidth={2} />
                <span>{insightCollapsed ? 'Show insights' : 'Hide insights'}</span>
              </button>
              <button
                type="button"
                role="menuitem"
                aria-label="Show Git panel"
                onclick={() => {
                  onCloseActionMenu();
                  onShowInsightPanel('git');
                }}
              >
                <FolderGit2 size={13} strokeWidth={2} />
                <span>Git</span>
                <kbd>{gitBadge()}</kbd>
              </button>
              <hr />
              <button
                type="button"
                role="menuitem"
                disabled={fileActionBusy === 'copy'}
                onclick={() => {
                  onCloseActionMenu();
                  onCopyPath();
                }}
              >
                {#if fileActionStatus === 'Path copied'}
                  <Check size={13} strokeWidth={2} />
                {:else}
                  <Copy size={13} strokeWidth={2} />
                {/if}
                <span>Copy path</span>
              </button>
              <button
                type="button"
                role="menuitem"
                disabled={fileActionBusy === 'open'}
                onclick={() => {
                  onCloseActionMenu();
                  onOpenInIDE();
                }}
              >
                <ExternalLink size={13} strokeWidth={2} />
                <span>Open in IDE</span>
              </button>
              <button
                type="button"
                role="menuitem"
                disabled={fileActionBusy === 'reveal'}
                onclick={() => {
                  onCloseActionMenu();
                  onRevealFile();
                }}
              >
                <FolderSearch size={13} strokeWidth={2} />
                <span>Reveal file</span>
              </button>
            </div>
          {/if}
        </div>
      </div>

      <div class="editor-body-grid">
        <div class="editor-canvas">
          {#if markdownPreviewAvailable && displayMode === 'preview'}
            <SourceMarkdownPreview
              content={draftContent}
              fileName={preview.fileName}
              relativePath={preview.relativePath}
              dirty={selectedDirty}
            />
          {:else}
            {#key appearanceKey}
              <MonacoSourceEditor
                preview={preview}
                content={draftContent}
                editable={true}
                appearanceOverride={appearanceOverride}
                externalDiagnostics={lspDiagnostics}
                loading={loading}
                targetLine={selectedSourceLine}
                targetLineRequestId={selectedSourceLineRequestId}
                intelligenceCommand={intelligenceCommand}
                {...monaco}
              />
            {/key}
          {/if}
        </div>

        {@render insights()}
      </div>
    </div>
  {:else}
    <div class="empty-preview">
      <FileCode2 size={34} strokeWidth={1.55} />
      <strong>No source file loaded</strong>
      <span>Scan a project or choose a file from the tree.</span>
    </div>
  {/if}
          {:else}
            <button
              class="editor-file-placeholder"
              type="button"
              title={tab.relativePath}
              onclick={() => onSelectTab(tab)}
            >
              <FileCode2 size={28} strokeWidth={1.55} />
              <strong>{tab.fileName}</strong>
              <span>{tab.relativePath}</span>
              <small>
                {tab.language}
                {#if tabGitStatus}
                  · {tabGitStatus.status}
                {/if}
                {#if isDirty(tab.path)}
                  · modified
                {/if}
              </small>
            </button>
          {/if}
        </section>
      {/each}
    </SourceDockviewShell>
  {:else}
    {#if fileActionStatus}
      <div class="file-action-feedback">{fileActionStatus}</div>
    {/if}

    {#if error}
      <div class="inline-error">
        <Activity size={15} strokeWidth={1.8} />
        <span>{error}</span>
      </div>
    {/if}

    <div class="empty-preview">
      <FileCode2 size={34} strokeWidth={1.55} />
      <strong>No source file loaded</strong>
      <span>Scan a project or choose a file from the tree.</span>
    </div>
  {/if}
</section>

<style>
  /*
   * Editor-scoped chrome. The base `.empty-tree, .empty-preview` layout rule
   * stays page-side (it is shared with the file-tree empty state); the
   * `.empty-preview`-specific overrides are re-declared here so this panel's
   * empty state renders identically in isolation. The ancestor-qualified
   * `:global(.source-dockview-*-shell .editor-frame/.empty-preview/…)` height
   * rules stay in the page — they are keyed off page-rendered shell ancestors
   * and `:global` selectors match the teleported nodes wherever they land.
   */
  .file-action-feedback {
    margin: -4px 0 10px;
    color: #7ce5d5;
    font-size: 11px;
    font-weight: 750;
  }

  .inline-error {
    display: inline-grid;
    grid-auto-flow: column;
    align-items: center;
    gap: 7px;
    margin-bottom: 12px;
    color: #f1b8a4;
    font-size: 12px;
    font-weight: 650;
  }

  .empty-preview {
    display: grid;
    place-items: center;
    gap: 8px;
    flex: 1 1 auto;
    min-height: 520px;
    color: #9aa5a1;
    text-align: center;
    border: 1px dashed rgba(255, 255, 255, 0.13);
    border-radius: 13px;
    background: rgba(255, 255, 255, 0.035);
    font-size: 12px;
    font-weight: 700;
  }

  .empty-preview strong {
    color: #f3f6f5;
    font-size: 16px;
  }

  .empty-preview span {
    color: #9aa5a1;
    font-weight: 650;
  }

  .editor-frame {
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    height: auto;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.11);
    border-radius: 8px;
    background: #17191e;
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.07),
      0 18px 45px rgba(0, 0, 0, 0.2);
  }

  .source-editor-dock-panel {
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .source-editor-file-pane {
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .editor-file-placeholder {
    display: grid;
    place-items: center;
    align-content: center;
    gap: 5px;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    color: #8f9a96;
    border: 0;
    background: #17191e;
    cursor: pointer;
  }

  .editor-file-placeholder strong,
  .editor-file-placeholder span,
  .editor-file-placeholder small {
    max-width: min(520px, 86%);
    overflow: hidden;
    text-align: center;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-file-placeholder strong {
    color: #f2f6f5;
    font-size: 13px;
    font-weight: 820;
  }

  .editor-file-placeholder span,
  .editor-file-placeholder small {
    font-size: 10px;
    font-weight: 760;
  }

  .editor-file-placeholder:hover,
  .editor-file-placeholder:focus-visible {
    color: #bff8ef;
    outline: 0;
  }

  .editor-toolbar {
    display: grid;
    position: relative;
    grid-template-columns: minmax(0, 1fr) repeat(3, auto);
    align-items: center;
    gap: var(--space-2);
    height: 28px;
    padding: 0 var(--space-2);
    border-bottom: 1px solid var(--color-border);
    background: transparent;
  }

  .editor-mode-toggle {
    display: inline-grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    align-items: center;
    width: 128px;
    min-width: 0;
    height: 22px;
    padding: 2px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--color-surface);
  }

  .editor-mode-toggle button {
    display: inline-grid;
    grid-template-columns: 12px minmax(0, 1fr);
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
    height: 18px;
    padding: 0 var(--space-1);
    color: var(--color-text-2);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  .editor-mode-toggle button.active {
    color: var(--color-live);
    background: var(--color-live-bg);
  }

  .editor-mode-toggle button:hover,
  .editor-mode-toggle button:focus-visible {
    color: var(--color-text);
    outline: 0;
  }

  .editor-mode-toggle span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-file-state {
    display: inline-grid;
    grid-template-columns: 14px minmax(0, 1fr) auto auto;
    align-items: center;
    justify-self: start;
    gap: var(--space-2);
    min-width: 0;
    max-width: 100%;
    height: 20px;
    padding: 0 var(--space-1);
    color: var(--color-text);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
  }

  .editor-file-glyph {
    display: inline-flex;
    align-items: center;
    color: var(--color-text-3);
  }

  .editor-file-state strong,
  .editor-file-state small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-file-title {
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-2);
    min-width: 0;
  }

  .editor-file-state strong {
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .editor-file-state small {
    flex: 0 0 auto;
    color: var(--color-text-3);
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: var(--text-xs);
    font-weight: var(--weight-normal);
  }

  .editor-lsp-chip {
    display: inline-flex;
    align-items: center;
    min-width: 0;
  }

  .editor-lsp-recovery-strip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
    height: 22px;
    padding: 0 var(--space-1);
    border-radius: var(--radius-sm);
    background: transparent;
  }

  .editor-lsp-recovery-action {
    display: grid;
    place-items: center;
    width: 24px;
    height: 20px;
    padding: 0;
    color: var(--color-text-2);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    cursor: pointer;
  }

  .editor-lsp-recovery-action:hover:not(:disabled),
  .editor-lsp-recovery-action:focus-visible {
    color: var(--color-text);
    outline: 0;
    background: var(--color-live-bg);
  }

  .editor-lsp-recovery-action:disabled {
    cursor: default;
    opacity: 0.38;
  }

  .editor-menu-anchor {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
  }

  .editor-icon-button {
    display: grid;
    place-items: center;
    width: 24px;
    height: 22px;
    color: var(--color-text-2);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    cursor: pointer;
  }

  .editor-icon-button:hover,
  .editor-icon-button:focus-visible {
    color: var(--color-text);
    outline: 0;
    background: var(--color-live-bg);
  }

  .editor-icon-button.active {
    color: var(--color-live);
    background: var(--color-live-bg);
  }

  .editor-action-menu {
    position: absolute;
    z-index: 8;
    top: calc(100% + 3px);
    right: 0;
    display: grid;
    width: 230px;
    min-width: 0;
    padding: 5px;
    border: 1px solid rgba(255, 255, 255, 0.11);
    border-radius: 8px;
    background: rgba(22, 25, 25, 0.98);
    box-shadow: 0 18px 44px rgba(0, 0, 0, 0.34);
  }

  .editor-action-menu button {
    display: grid;
    grid-template-columns: 15px minmax(0, 1fr) auto;
    align-items: center;
    gap: 7px;
    min-width: 0;
    height: 26px;
    padding: 0 7px;
    color: #cbd3d1;
    border: 0;
    border-radius: 5px;
    background: transparent;
    font-size: 11px;
    font-weight: 730;
    text-align: left;
    cursor: pointer;
  }

  .editor-action-menu button:hover:not(:disabled),
  .editor-action-menu button:focus-visible {
    color: #f2f6f5;
    outline: 0;
    background: rgba(92, 226, 207, 0.1);
  }

  .editor-action-menu button:disabled {
    cursor: default;
    opacity: 0.48;
  }

  .editor-action-menu span,
  .editor-action-menu kbd {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-action-menu kbd {
    color: #8d9995;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
    font-size: 9px;
    font-weight: 760;
  }

  .editor-action-menu hr {
    width: 100%;
    height: 1px;
    margin: 4px 0;
    border: 0;
    background: rgba(255, 255, 255, 0.08);
  }

  .editor-body-grid {
    display: grid;
    flex: 1 1 auto;
    grid-template-columns: minmax(0, 1fr);
    min-height: 0;
  }

  .editor-canvas {
    position: relative;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  @media (max-width: 720px) {
    .editor-frame {
      height: 520px;
    }
  }
</style>
