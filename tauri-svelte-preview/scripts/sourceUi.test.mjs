import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const pageSource = await readFile(new URL('../src/routes/+page.svelte', import.meta.url), 'utf8');
const editorSource = await readFile(
  new URL('../src/lib/MonacoSourceEditor.svelte', import.meta.url),
  'utf8'
);
const appearanceSource = await readFile(
  new URL('../src/lib/sourcePreviewAppearance.ts', import.meta.url),
  'utf8'
);

function blockFor(selector) {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = new RegExp(`${escapedSelector}\\s*\\{(?<body>[^}]*)\\}`, 'm').exec(pageSource);
  assert.ok(match?.groups?.body, `Missing style block for ${selector}`);
  return match.groups.body;
}

function assertDeclaration(selector, declaration) {
  assert.match(blockFor(selector), new RegExp(`(^|\\n)\\s*${declaration.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\s*;`), `${selector} should include ${declaration}`);
}

assertDeclaration('.source-browser-stack', 'overflow: hidden');
assertDeclaration('.shell', 'grid-template-columns: var(--side-pane-width) 8px minmax(0, 1fr)');
assertDeclaration('.activity-shell', 'min-width: 0');
assertDeclaration('.topbar > div:first-child', 'min-width: 0');
assertDeclaration('.topbar h2', 'text-overflow: ellipsis');
assertDeclaration('.activity-rail', 'width: 54px');
assertDeclaration('.layout-preset-group', 'display: inline-flex');
assertDeclaration('.layout-preset-group button.active', 'background: rgba(92, 226, 207, 0.18)');
assertDeclaration('.side-pane-resizer', 'cursor: col-resize');
assertDeclaration('.activity-panel', 'grid-template-rows: auto auto minmax(0, 1fr)');
assertDeclaration('.activity-panel-list', 'overflow-y: auto');
assertDeclaration('.activity-filter-box', 'grid-template-columns: 18px minmax(0, 1fr)');
assertDeclaration('.editor-body-grid', 'grid-template-columns: minmax(0, 1fr) 8px var(--editor-insight-width)');
assertDeclaration('.editor-insight-resizer', 'cursor: col-resize');
assertDeclaration('.context-panel-grid.collapsed', 'display: none');
assertDeclaration('.source-list-panel', 'overflow: hidden');
assertDeclaration('.file-tree', 'overflow-y: auto');
assertDeclaration('.file-tree', 'overflow-x: hidden');
assertDeclaration('.file-tree', 'scrollbar-gutter: stable');
assertDeclaration('.file-tree', 'scrollbar-width: thin');
assert.ok(pageSource.includes("type SourceActivityMode = 'files' | 'conversations' | 'sessions' | 'agents' | 'worktrees' | 'git'"), 'Source shell should define switchable activity modes');
assert.ok(pageSource.includes('sourceActivityModeStorageKey'), 'Source shell should persist the active activity mode');
assert.ok(pageSource.includes('sidePaneWidthStorageKey'), 'Source shell should persist the side pane width');
assert.ok(pageSource.includes('sourceLayoutPresetStorageKey'), 'Source shell should persist the selected layout preset');
assert.ok(pageSource.includes('sourceTerminalAppStorageKey'), 'Source shell should persist the selected terminal app');
assert.ok(pageSource.includes("type SourceLayoutPresetID = 'review' | 'code' | 'git' | 'sessions' | 'custom'"), 'Source shell should define named layout presets plus custom');
assert.ok(pageSource.includes("type SourceTerminalApp = 'Warp' | 'Terminal' | 'iTerm' | 'iTerm2' | 'Ghostty' | 'WezTerm' | 'Alacritty'"), 'Source shell should define supported terminal apps');
assert.ok(pageSource.includes('const sourceLayoutPresets'), 'Source shell should define reusable layout presets');
assert.ok(pageSource.includes('const sourceTerminalApps'), 'Source shell should define reusable terminal app choices');
assert.ok(pageSource.includes('let sourceActivityMode'), 'Source shell should track the active side pane mode');
assert.ok(pageSource.includes('let sourceActivityFilter'), 'Source shell should track the side pane activity filter');
assert.ok(pageSource.includes('let sidePaneWidth'), 'Source shell should track the resizable side pane width');
assert.ok(pageSource.includes('let sourceLayoutPreset'), 'Source shell should track the active layout preset');
assert.ok(pageSource.includes('let sourceTerminalApp'), 'Source shell should track the selected terminal app');
assert.ok(pageSource.includes('function applySourceLayoutPreset'), 'Source shell should expose layout preset application');
assert.ok(pageSource.includes('function selectSourceTerminalApp'), 'Source shell should expose terminal app selection');
assert.ok(pageSource.includes('function markSourceLayoutCustom'), 'Manual layout changes should mark the layout as custom');
assert.ok(pageSource.includes('function copyTextToClipboard'), 'Activity rows should share clipboard copy behavior');
assert.ok(pageSource.includes('function copyActivityCommand'), 'Activity rows should copy resume commands and paths');
assert.ok(pageSource.includes('function openActivityPath'), 'Activity rows should open repo and worktree paths');
assert.ok(pageSource.includes('function revealActivityPath'), 'Activity rows should reveal repo and worktree paths');
assert.ok(pageSource.includes('function openActivityTerminalPath'), 'Activity rows should open repo and worktree paths in a terminal');
assert.ok(pageSource.includes('function activityTextMatchesFilter'), 'Activity rows should share filter matching logic');
assert.ok(pageSource.includes('function sourceActivityFilterPlaceholder'), 'Activity filter placeholder should match the active panel');
assert.ok(pageSource.includes('function selectSourceActivityMode'), 'Source shell should expose activity mode selection');
assert.ok(pageSource.includes('function beginSidePaneResize'), 'Source shell should expose side pane drag resizing');
assert.ok(pageSource.includes('function projectWorktreeActivityLabel'), 'Worktree rows should format last activity labels');
assert.ok(pageSource.includes('editorInsightWidthStorageKey'), 'Editor shell should persist the inspector width');
assert.ok(pageSource.includes('contextPanelCollapsedStorageKey'), 'Workspace should persist collapsed context cards');
assert.ok(pageSource.includes('let editorInsightWidth'), 'Editor shell should track inspector width');
assert.ok(pageSource.includes('let contextPanelCollapsed'), 'Workspace should track context card collapse state');
assert.ok(pageSource.includes('function beginEditorInsightResize'), 'Editor shell should expose inspector drag resizing');
assert.ok(pageSource.includes('function toggleContextPanelCollapsed'), 'Workspace should expose context card collapse');
assert.ok(pageSource.includes('if (storedWidth === null) return sidePaneDefaultWidth'), 'Missing side pane storage should use the designed default width');
assert.ok(pageSource.includes('if (storedWidth === null) return editorInsightDefaultWidth'), 'Missing editor inspector storage should use the designed default width');
assert.ok(pageSource.includes('class="activity-rail"'), 'Source shell should render an activity rail');
assert.ok(pageSource.includes('class="layout-preset-group"'), 'Source shell should render layout preset controls');
assert.ok(pageSource.includes('aria-label="Workspace layout presets"'), 'Layout preset controls should be grouped for assistive tech');
assert.ok(pageSource.includes('aria-label="Terminal app"'), 'Terminal app picker should be accessible');
assert.ok(pageSource.includes('openTerminalPathFromTauri(path, sourceTerminalApp)'), 'Terminal open actions should use the selected terminal app');
assert.ok(pageSource.includes('aria-label="Workspace views"'), 'Activity rail should be labeled');
assert.ok(pageSource.includes('aria-label="Files"'), 'Activity rail should expose files');
assert.ok(pageSource.includes('aria-label="Conversations"'), 'Activity rail should expose conversations');
assert.ok(pageSource.includes('aria-label="Active sessions"'), 'Activity rail should expose active sessions');
assert.ok(pageSource.includes('aria-label="Agents"'), 'Activity rail should expose agents');
assert.ok(pageSource.includes('aria-label="Worktrees"'), 'Activity rail should expose worktrees');
assert.ok(pageSource.includes('aria-label="Git and tasks"'), 'Activity rail should expose Git and tasks');
assert.ok(pageSource.includes('class="side-pane-resizer"'), 'Source shell should render a side pane resizer');
assert.ok(pageSource.includes('aria-label="Resize side pane"'), 'Side pane resizer should be labeled');
assert.ok(pageSource.includes('class="workspace-context-toggle"'), 'Workspace should render a context card collapse control');
assert.ok(pageSource.includes('aria-label="Toggle workspace context cards"'), 'Context card collapse control should be labeled');
assert.ok(pageSource.includes('class="editor-insight-resizer"'), 'Editor shell should render an inspector resizer');
assert.ok(pageSource.includes('aria-label="Resize editor insights"'), 'Inspector resizer should be labeled');
assert.ok(pageSource.includes('class="activity-panel"'), 'Non-file side modes should render activity panels');
assert.ok(pageSource.includes('class="activity-filter-box"'), 'Activity panels should render a filter field');
assert.ok(pageSource.includes('class="activity-panel-list"'), 'Activity panels should render scrollable lists');
assert.ok(pageSource.includes('class="activity-row-actions"'), 'Activity rows should render compact action controls');
assert.ok(pageSource.includes('placeholder={sourceActivityFilterPlaceholder(sourceActivityMode)}'), 'Activity filter placeholder should be dynamic');
assert.ok(pageSource.includes('aria-label="Filter workspace activity"'), 'Activity filter should be accessible');
assert.ok(pageSource.includes('filteredProjectAgentSessions'), 'Activity panels should filter conversation and agent rows');
assert.ok(pageSource.includes('filteredProjectWorktrees'), 'Activity panels should filter worktree rows');
assert.ok(pageSource.includes('worktree.taskID'), 'Worktree rows should expose parsed task IDs');
assert.ok(pageSource.includes('gitTaskUrl(worktree.taskID)'), 'Worktree rows should link parsed task IDs to Notion');
assert.ok(pageSource.includes('aria-label="Open worktree task"'), 'Worktree task links should be accessible');
assert.ok(pageSource.includes('projectWorktreeActivityLabel(worktree)'), 'Worktree rows should show last activity');
assert.ok(pageSource.includes('filteredGitRepositorySummaries'), 'Activity panels should filter repository rows');
assert.ok(pageSource.includes('filteredGitCommitHistory'), 'Activity panels should filter commit rows');
assert.ok(pageSource.includes('aria-label="Copy agent resume command"'), 'Agent rows should expose resume command copy');
assert.ok(pageSource.includes('aria-label="Open worktree path"'), 'Worktree rows should expose native open');
assert.ok(pageSource.includes('aria-label="Open worktree in terminal"'), 'Worktree rows should expose terminal open');
assert.ok(pageSource.includes('aria-label="Open repository in terminal"'), 'Repository rows should expose terminal open');
assert.ok(pageSource.includes('aria-label="Open active session in terminal"'), 'Active session rows should expose terminal open');
assert.ok(pageSource.includes('aria-label="Reveal repository path"'), 'Repository rows should expose native reveal');
assert.ok(pageSource.includes('.file-tree::-webkit-scrollbar'), 'Tree view should style WebKit scrollbars');
assert.ok(pageSource.includes('function cancelSourceScan()'), 'Source scans should expose a cancel action');
assert.ok(
  pageSource.includes('onclick={scanning ? cancelSourceScan : () => scanProject(selectedProject, undefined, { force: true })}'),
  'Scan button should become a cancel button while a scan is running'
);
assert.ok(pageSource.includes("aria-label={scanning ? 'Stop source scan' : 'Scan source files'}"), 'Scan button should announce stop state');
assert.ok(pageSource.includes("<span>{scanning ? 'Stop' : 'Scan'}</span>"), 'Scan button label should switch to Stop while scanning');
assert.ok(pageSource.includes('virtualizeSourceTreeRows'), 'Source tree should use virtualized row slicing');
assert.ok(pageSource.includes('bind:this={fileTreeElement}'), 'File tree should bind the scroll container for viewport measurement');
assert.ok(pageSource.includes('onscroll={handleFileTreeScroll}'), 'File tree should track scroll position for virtualization');
assert.ok(pageSource.includes('class="tree-virtual-spacer"'), 'File tree should render spacer rows for offscreen content');
assertDeclaration('.tree-virtual-spacer', 'height: var(--tree-spacer-height)');
assert.ok(pageSource.includes('scrollTopForSourceTreeReveal'), 'Source tree should calculate scroll reveal positions');
assert.ok(pageSource.includes('let pendingTreeRevealPath'), 'Source tree should track a pending selected file reveal');
assert.ok(pageSource.includes('function requestSourceTreeReveal'), 'Source tree should expose a selected file reveal request');
assert.ok(pageSource.includes('requestSourceTreeReveal(record)'), 'Selecting a source record should request tree reveal');
assert.ok(pageSource.includes('let pendingTreeFocusRowIndex'), 'Source tree should track pending keyboard focus');
assert.ok(pageSource.includes('function focusTreeRowAtIndex'), 'Source tree should expose indexed row focus');
assert.ok(pageSource.includes('function handleTreeRowKeydown'), 'Source tree should handle keyboard navigation');
assert.ok(pageSource.includes('data-tree-row-index={virtualizedTreeRows.startIndex + virtualTreeRowIndex}'), 'Rendered tree rows should expose their absolute row index');
assert.ok(pageSource.includes('onkeydown={(event) => handleTreeRowKeydown(row, event)}'), 'Rendered tree rows should bind keyboard handling');
assert.ok(pageSource.includes('class="icon-button quick-open-trigger"'), 'Project controls should expose a visible quick-open trigger');
assert.ok(pageSource.includes('onclick={openQuickOpen}'), 'Quick-open trigger should call the existing quick-open opener');
assert.ok(pageSource.includes('cancelSourceScanFromTauri'), 'Stop should call the native scan cancellation command');
assert.ok(pageSource.includes('sourceScanProgress'), 'Source preview should track native scan progress');
assert.ok(pageSource.includes('nativeSourceScanProgressEvent'), 'Source preview should subscribe to native scan progress events');
assert.ok(pageSource.includes('scanSummaryLabel'), 'Source tree should expose an explicit scan summary label');
assert.ok(pageSource.includes('class="scan-summary"'), 'Source tree should render the scan summary below the heading');
assertDeclaration('.scan-summary', 'overflow: hidden');
assertDeclaration('.scan-summary', 'text-overflow: ellipsis');
assert.ok(pageSource.includes('expandedSourceScanLimit'), 'Source preview should expose an expanded scan limit');
assert.ok(pageSource.includes('{#if scanLimitReached && !scanning}'), 'Truncated source scans should expose a rescan-more action');
assert.ok(pageSource.includes('class="scan-more-button"'), 'Expanded source scans should use a compact action button');
assert.ok(
  pageSource.includes('scanProject(selectedProject, selectedRecord?.path, { force: true, limit: expandedSourceScanLimit })'),
  'Expanded source scans should rescan the active project at the larger limit'
);
assertDeclaration('.scan-more-button', 'white-space: nowrap');
assert.ok(editorSource.includes('basic-languages/dart/dart.contribution'), 'Editor should load Dart highlighting');
assert.ok(editorSource.includes('basic-languages/hcl/hcl.contribution'), 'Editor should load HCL highlighting');
assert.ok(editorSource.includes('basic-languages/lua/lua.contribution'), 'Editor should load Lua highlighting');
assert.ok(editorSource.includes('basic-languages/php/php.contribution'), 'Editor should load PHP highlighting');
assert.ok(editorSource.includes('basic-languages/protobuf/protobuf.contribution'), 'Editor should load Protobuf highlighting');
assert.ok(editorSource.includes('editable?: boolean'), 'Editor should expose an editable mode prop');
assert.ok(editorSource.includes('content?: string'), 'Editor should allow external draft content');
assert.ok(
  editorSource.includes('onContentChange?: (content: string) => void'),
  'Editor should report Monaco content changes to the parent'
);
assert.ok(
  editorSource.includes('onDidChangeModelContent'),
  'Editor should subscribe to Monaco content changes'
);
assert.ok(pageSource.includes('sourceDraftContentByPath'), 'Source page should track draft content per path');
assert.ok(pageSource.includes('savedSourceContentByPath'), 'Source page should track saved content per path');
assert.ok(pageSource.includes('writeSourceToTauri'), 'Source page should use the native write command');
assert.ok(pageSource.includes('function saveSelectedSourceFile'), 'Source page should expose a save action');
assert.ok(pageSource.includes('function revertSelectedSourceFile'), 'Source page should expose a revert action');
assert.ok(
  pageSource.includes('class:dirty={isSourcePathDirty(tab.path)}'),
  'Source tabs should show dirty state per path'
);
assert.ok(pageSource.includes('class="tab-dirty-dot"'), 'Dirty tabs should include a compact dirty marker');
assert.ok(pageSource.includes('aria-label="Save source file"'), 'Editor toolbar should expose save');
assert.ok(pageSource.includes('aria-label="Revert source file"'), 'Editor toolbar should expose revert');
assert.ok(
  pageSource.includes('content={selectedSourceDraftContent}'),
  'Monaco should receive the active source draft content'
);
assert.ok(pageSource.includes('editable={true}'), 'Monaco should run in editable mode');
assert.ok(
  pageSource.includes('onContentChange={updateSelectedSourceDraft}'),
  'Monaco should update the active draft content'
);
assert.ok(
  editorSource.includes('vs/language/typescript/monaco.contribution'),
  'Editor should load the Monaco TypeScript language service'
);
assert.ok(
  editorSource.includes('vs/language/typescript/ts.worker?worker'),
  'Editor should route TypeScript language requests to the TypeScript worker'
);
assert.ok(
  editorSource.includes('function configureTypeScriptLanguageService'),
  'Editor should configure TypeScript diagnostics'
);
assert.ok(
  editorSource.includes('onDidChangeMarkers'),
  'Editor should subscribe to Monaco diagnostics marker changes'
);
assert.ok(
  editorSource.includes('onDiagnosticsChange?: (diagnostics: SourceDiagnostic[]) => void'),
  'Editor should report diagnostics to the parent'
);
assert.ok(
  editorSource.includes('onSymbolsChange?: (symbols: SourceSymbol[]) => void'),
  'Editor should report symbols to the parent'
);
assert.ok(editorSource.includes('intelligenceCommand?: SourceEditorIntelligenceCommand | null'), 'Editor should accept language-intelligence commands');
assert.ok(
  editorSource.includes('onDefinitionLookup?: (symbolName: string) => void'),
  'Editor should report the current word for project definition lookup'
);
assert.ok(
  editorSource.includes('onReferenceLookup?: (symbolName: string) => void'),
  'Editor should report the current word for project reference lookup'
);
assert.ok(editorSource.includes('getWordAtPosition'), 'Editor should read the symbol under the cursor');
assert.ok(editorSource.includes('editor.action.showHover'), 'Editor should expose a hover command');
assert.ok(editorSource.includes('editor.action.revealDefinition'), 'Editor should expose go-to-definition');
assert.ok(pageSource.includes('sourceDiagnostics'), 'Source page should track diagnostics');
assert.ok(pageSource.includes('sourceSymbols'), 'Source page should track symbols');
assert.ok(pageSource.includes('sourceDefinitionTargets'), 'Source page should track project definition lookup targets');
assert.ok(pageSource.includes('sourceReferenceTargets'), 'Source page should track project reference lookup targets');
assert.ok(pageSource.includes('formatSourceDiagnosticSummary'), 'Source page should summarize diagnostics');
assert.ok(pageSource.includes('sourceSupportsLanguageIntelligence'), 'Source page should gate Monaco language actions');
assert.ok(pageSource.includes('requestSourceIntelligenceAction'), 'Source page should dispatch language actions');
assert.ok(pageSource.includes('findSourceDefinitionsFromTauri'), 'Source page should call native project definition lookup');
assert.ok(pageSource.includes('findSourceDefinitionTargets'), 'Source page should fall back to browser definition lookup');
assert.ok(pageSource.includes('findSourceReferencesFromTauri'), 'Source page should call native project reference lookup');
assert.ok(pageSource.includes('findSourceReferenceTargets'), 'Source page should fall back to browser reference lookup');
assert.ok(pageSource.includes('function runSourceDefinitionLookup'), 'Source page should expose project definition lookup');
assert.ok(pageSource.includes('function runSourceReferenceLookup'), 'Source page should expose project reference lookup');
assert.ok(pageSource.includes('function handleEditorDefinitionLookup'), 'Source page should receive editor definition lookup requests');
assert.ok(pageSource.includes('function handleEditorReferenceLookup'), 'Source page should receive editor reference lookup requests');
assert.ok(pageSource.includes('onDefinitionLookup={handleEditorDefinitionLookup}'), 'Editor should be wired to project definition lookup');
assert.ok(pageSource.includes('onReferenceLookup={handleEditorReferenceLookup}'), 'Editor should be wired to project reference lookup');
assert.ok(pageSource.includes('source-intelligence-panel'), 'Source page should render language intelligence panel');
assert.ok(pageSource.includes('aria-label="Show hover"'), 'Editor controls should expose hover');
assert.ok(pageSource.includes('aria-label="Go to definition"'), 'Editor controls should expose definition');
assert.ok(pageSource.includes('aria-label="Find references"'), 'Editor controls should expose references');
assert.ok(pageSource.includes('Problems'), 'Language panel should include Problems');
assert.ok(pageSource.includes('Symbols'), 'Language panel should include Symbols');
assert.ok(pageSource.includes('selectSourceSymbol'), 'Symbol rows should reveal source lines');
assert.ok(pageSource.includes('class="definition-results"'), 'Language panel should show project definition lookup results');
assert.ok(pageSource.includes('class="reference-results"'), 'Language panel should show project reference lookup results');
assertDeclaration('.definition-results', 'overflow-y: auto');
assertDeclaration('.definition-results', 'scrollbar-width: thin');
assertDeclaration('.reference-results', 'overflow-y: auto');
assertDeclaration('.reference-results', 'scrollbar-width: thin');
assert.ok(
  appearanceSource.includes('encodedTokensColors'),
  'Theme should include an encoded token palette for sharper Monaco token color separation'
);
assert.ok(
  appearanceSource.includes("token: 'keyword.public'"),
  'Theme should target C# modifier keyword tokens'
);
assert.ok(
  appearanceSource.includes("token: 'delimiter.bracket'"),
  'Theme should target Monaco bracket delimiter tokens'
);
assert.ok(
  editorSource.includes('sourceSemanticTokenLegend'),
  'Editor should import the source semantic-token legend'
);
assert.ok(
  editorSource.includes('extractSourceSemanticTokens'),
  'Editor should extract semantic tokens for Monaco'
);
assert.ok(
  editorSource.includes('registerDocumentSemanticTokensProvider'),
  'Editor should register a semantic-token provider'
);
assert.ok(
  /["']semanticHighlighting\.enabled["']:\s*true/.test(editorSource),
  'Editor should enable semantic highlighting explicitly'
);
assert.ok(pageSource.includes('backgroundIndexingProjectIDs'), 'Source page should track projects indexing in the background');
assert.ok(pageSource.includes('function indexProjectsInBackground'), 'Source page should start background project indexing');
assert.ok(pageSource.includes('selectBackgroundIndexProjects'), 'Source page should use the background index selection helper');
assert.ok(pageSource.includes('formatSourceIndexSummary'), 'Source page should format visible project index status');
assert.ok(pageSource.includes('class="index-summary"'), 'Source Browser should render an index status line');
assertDeclaration('.index-summary', 'overflow: hidden');
assertDeclaration('.index-summary', 'text-overflow: ellipsis');
assert.ok(pageSource.includes('readProjectGitStatusFromTauri'), 'Source page should load native project Git status');
assert.ok(pageSource.includes('readSourceGitDiffFromTauri'), 'Source page should load native selected-file Git diff');
assert.ok(pageSource.includes('stageGitPathsFromTauri'), 'Source page should stage Git paths through Tauri');
assert.ok(pageSource.includes('unstageGitPathsFromTauri'), 'Source page should unstage Git paths through Tauri');
assert.ok(pageSource.includes('commitGitRepositoryFromTauri'), 'Source page should commit staged Git paths through Tauri');
assert.ok(pageSource.includes('fetchGitRepositoryFromTauri'), 'Source page should fetch Git remotes through Tauri');
assert.ok(pageSource.includes('pullGitRepositoryFromTauri'), 'Source page should pull Git remotes through Tauri');
assert.ok(pageSource.includes('pushGitRepositoryFromTauri'), 'Source page should push Git remotes through Tauri');
assert.ok(pageSource.includes('readGitCommitHistoryFromTauri'), 'Source page should load native Git commit history');
assert.ok(pageSource.includes('projectGitStatus'), 'Source page should track selected project Git status');
assert.ok(pageSource.includes('gitStatusByRelativePath'), 'Source page should map Git file status by relative path');
assert.ok(pageSource.includes('gitCommitHistory'), 'Source page should track selected project commit history');
assert.ok(pageSource.includes('function loadGitCommitHistory'), 'Source page should expose a Git commit history loader');
assert.ok(pageSource.includes('function gitTaskUrl'), 'Source page should expose Notion task links for recognized task IDs');
assert.ok(pageSource.includes('gitCommitMessage'), 'Source page should track a Git commit message draft');
assert.ok(pageSource.includes('function runGitPathAction'), 'Source page should expose reusable stage/unstage handling');
assert.ok(pageSource.includes('function runGitRemoteAction'), 'Source page should expose reusable fetch/pull/push handling');
assert.ok(pageSource.includes('function commitGitChanges'), 'Source page should expose a commit action');
assert.ok(pageSource.includes('selectedSourceGitDiff'), 'Source page should track selected source Git diff');
assert.ok(pageSource.includes('function loadSelectedSourceGitDiff'), 'Source page should expose a selected-file Git diff loader');
assert.ok(pageSource.includes('formatSourceContextIdentity'), 'Source page should format a visible context identity');
assert.ok(pageSource.includes('formatSourceContextGitSummary'), 'Source page should use the shared Git context summary');
assert.ok(pageSource.includes('sourceContextIdentity'), 'Source page should derive the current source context identity');
assert.ok(pageSource.includes('aria-label="Current source context"'), 'Workspace should expose the current context strip');
assert.ok(pageSource.includes('class="context-identity-strip"'), 'Workspace should render a context identity strip');
assert.ok(pageSource.includes('class="context-identity-pill"'), 'Context strip should render compact labeled pills');
assert.ok(pageSource.includes('<span>Project</span>'), 'Context strip should label the project');
assert.ok(pageSource.includes('<span>Root</span>'), 'Context strip should label the root/worktree');
assert.ok(pageSource.includes('<span>Branch</span>'), 'Context strip should label the branch/Git state');
assert.ok(pageSource.includes('<span>Runtime</span>'), 'Context strip should label the runtime surface');
assert.ok(pageSource.includes('class="project-git-pill"'), 'Topbar should render a compact project Git branch/status pill');
assert.ok(pageSource.includes('class="git-status-badge"'), 'Tree and tabs should render file-level Git status badges');
assert.ok(pageSource.includes('<span>Git</span>'), 'Language panel should include a Git tab');
assert.ok(pageSource.includes('class="git-diff-panel"'), 'Source page should render a selected-file Git diff panel');
assert.ok(pageSource.includes('class="git-status-list"'), 'Git tab should render changed file rows');
assert.ok(pageSource.includes('aria-label="Stage selected source file"'), 'Git tab should expose staging for the selected file');
assert.ok(pageSource.includes('aria-label="Unstage selected source file"'), 'Git tab should expose unstaging for the selected file');
assert.ok(pageSource.includes('aria-label="Fetch selected repository"'), 'Git tab should expose repository fetch');
assert.ok(pageSource.includes('aria-label="Pull selected repository"'), 'Git tab should expose repository pull');
assert.ok(pageSource.includes('aria-label="Push selected repository"'), 'Git tab should expose repository push');
assert.ok(pageSource.includes('aria-label="Commit staged Git changes"'), 'Git tab should expose a guarded commit action');
assert.ok(pageSource.includes('aria-label="Git commit history"'), 'Git tab should expose commit history');
assert.ok(pageSource.includes('class="git-history-list"'), 'Git tab should render commit history rows');
assert.ok(pageSource.includes('class="git-history-row"'), 'Git tab should render individual commit history rows');
assert.ok(pageSource.includes('class="git-task-link"'), 'Git history should render task links when task metadata is present');
assert.ok(pageSource.includes('class="git-diff-block"'), 'Source page should render diff text in a monospace block');
assertDeclaration('.git-status-badge', 'font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace');
assertDeclaration('.project-git-pill', 'overflow: hidden');
assertDeclaration('.git-diff-panel', 'overflow: hidden');
assertDeclaration('.git-status-list', 'overflow-y: auto');
assertDeclaration('.git-history-list', 'overflow-y: auto');
assertDeclaration('.git-history-list', 'scrollbar-width: thin');
assertDeclaration('.git-history-row', 'min-width: 0');
assertDeclaration('.git-task-link', 'white-space: nowrap');
assertDeclaration('.git-commit-input', 'resize: none');
assertDeclaration('.git-remote-row', 'grid-template-columns: repeat(3, minmax(0, 1fr))');
assertDeclaration('.git-diff-block', 'overflow: auto');
assertDeclaration('.git-diff-block', 'font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace');
assertDeclaration('.context-identity-strip', 'overflow: hidden');
assertDeclaration('.context-identity-pill', 'min-width: 0');
assertDeclaration('.context-identity-pill strong', 'overflow: hidden');
assert.ok(pageSource.includes('searchSourceFilesFromTauri'), 'Source page should call native global source search');
assert.ok(pageSource.includes('sourceSearchQuery'), 'Source page should track a global source search query');
assert.ok(pageSource.includes('sourceSearchResults'), 'Source page should track global source search results');
assert.ok(pageSource.includes('function runGlobalSourceSearch'), 'Source page should expose a global source search action');
assert.ok(pageSource.includes('function selectSourceSearchResult'), 'Source page should open search results at their line');
assert.ok(pageSource.includes('class="global-search-panel"'), 'Source page should render a global source search panel');
assert.ok(pageSource.includes('class="source-search-results"'), 'Source page should render source search results');
assertDeclaration('.source-search-results', 'overflow-y: auto');
assertDeclaration('.source-search-results', 'scrollbar-width: thin');
assert.ok(pageSource.includes('listRuntimeContextsFromTauri'), 'Source page should load native runtime contexts');
assert.ok(pageSource.includes('runtimeContexts'), 'Source page should track local runtime contexts');
assert.ok(pageSource.includes('selectedProjectRuntimeContexts'), 'Source page should filter contexts to the selected project');
assert.ok(pageSource.includes('function loadRuntimeContexts'), 'Source page should expose a runtime context refresh action');
assert.ok(pageSource.includes('aria-label="Refresh runtime contexts"'), 'Runtime context panel should expose a refresh action');
assert.ok(pageSource.includes('class="runtime-context-panel"'), 'Source page should render a runtime context panel');
assert.ok(pageSource.includes('class="runtime-context-list"'), 'Runtime context panel should render a scrollable list');
assert.ok(pageSource.includes('class="runtime-port"'), 'Runtime context rows should show the listening port');
assert.ok(pageSource.includes('Runtime Contexts'), 'Runtime context panel should have a clear heading');
assertDeclaration('.runtime-context-list', 'overflow-y: auto');
assertDeclaration('.runtime-context-list', 'scrollbar-width: thin');
assert.ok(pageSource.includes('listProjectWorktreesFromTauri'), 'Source page should load native project worktrees');
assert.ok(pageSource.includes('projectWorktrees'), 'Source page should track project worktrees');
assert.ok(pageSource.includes('function loadProjectWorktrees'), 'Source page should expose a worktree refresh action');
assert.ok(pageSource.includes('aria-label="Refresh worktrees"'), 'Worktree panel should expose a refresh action');
assert.ok(pageSource.includes('class="worktree-context-panel"'), 'Source page should render a worktree safety panel');
assert.ok(pageSource.includes('class="worktree-context-list"'), 'Worktree panel should render a scrollable list');
assert.ok(pageSource.includes('class="worktree-status-badge"'), 'Worktree rows should show delete eligibility');
assert.ok(pageSource.includes('Worktree Safety'), 'Worktree panel should have a clear heading');
assertDeclaration('.worktree-context-list', 'overflow-y: auto');
assertDeclaration('.worktree-context-list', 'scrollbar-width: thin');
assert.ok(pageSource.includes('listGitRepositorySummariesFromTauri'), 'Source page should load native repository summaries');
assert.ok(pageSource.includes('gitRepositorySummaries'), 'Source page should track repository dashboard summaries');
assert.ok(pageSource.includes('function loadGitRepositorySummaries'), 'Source page should expose a repository dashboard refresh action');
assert.ok(pageSource.includes('aria-label="Refresh repository dashboard"'), 'Repository dashboard should expose a refresh action');
assert.ok(pageSource.includes('class="repo-dashboard-panel"'), 'Source page should render a repository dashboard panel');
assert.ok(pageSource.includes('class="repo-dashboard-list"'), 'Repository dashboard should render a scrollable list');
assert.ok(pageSource.includes('class="repo-dashboard-row"'), 'Repository dashboard should render repository summary rows');
assert.ok(pageSource.includes('<span>Task</span>'), 'Repository dashboard should label inferred task IDs');
assert.ok(pageSource.includes('repoDashboardTaskUrl'), 'Repository dashboard should link recognized task IDs');
assert.ok(pageSource.includes('class="repo-task-link"'), 'Repository dashboard should render task badges as links');
assert.ok(pageSource.includes('<span>Dirty</span>'), 'Repository dashboard should label dirty age/count');
assertDeclaration('.repo-dashboard-list', 'overflow-y: auto');
assertDeclaration('.repo-dashboard-list', 'scrollbar-width: thin');
assertDeclaration('.repo-task-link', 'white-space: nowrap');
assert.ok(pageSource.includes('listAgentSessionsFromTauri'), 'Source page should load native agent sessions');
assert.ok(pageSource.includes('agentSessions'), 'Source page should track agent sessions');
assert.ok(pageSource.includes('selectedProjectAgentSessions'), 'Source page should filter sessions to the selected project');
assert.ok(pageSource.includes('function loadAgentSessions'), 'Source page should expose an agent session refresh action');
assert.ok(pageSource.includes('aria-label="Refresh agent sessions"'), 'Agent session panel should expose a refresh action');
assert.ok(pageSource.includes('class="agent-session-panel"'), 'Source page should render an agent session panel');
assert.ok(pageSource.includes('class="agent-session-list"'), 'Agent session panel should render a scrollable list');
assert.ok(pageSource.includes('class="agent-provider-badge"'), 'Agent session rows should show the provider');
assert.ok(pageSource.includes('Agent Sessions'), 'Agent session panel should have a clear heading');
assertDeclaration('.agent-session-list', 'overflow-y: auto');
assertDeclaration('.agent-session-list', 'scrollbar-width: thin');
