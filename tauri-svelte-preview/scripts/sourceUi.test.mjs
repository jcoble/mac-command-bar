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
const tauriSource = await readFile(new URL('../src/lib/tauriSource.ts', import.meta.url), 'utf8');

function blockFor(selector) {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = new RegExp(`(^|\\n)\\s*${escapedSelector}\\s*\\{(?<body>[^}]*)\\}`, 'm').exec(pageSource);
  assert.ok(match?.groups?.body, `Missing style block for ${selector}`);
  return match.groups.body;
}

function assertDeclaration(selector, declaration) {
  assert.match(blockFor(selector), new RegExp(`(^|\\n)\\s*${declaration.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\s*;`), `${selector} should include ${declaration}`);
}

assertDeclaration('.source-browser-stack', 'overflow: hidden');
assertDeclaration('.shell', 'grid-template-columns: var(--side-pane-width) 6px minmax(0, 1fr)');
assertDeclaration('.shell.side-right', 'grid-template-columns: minmax(0, 1fr) 6px var(--side-pane-width)');
assertDeclaration('.activity-shell', 'min-width: 0');
assertDeclaration('.topbar > div:first-child', 'min-width: 0');
assertDeclaration('.topbar h2', 'text-overflow: ellipsis');
assertDeclaration('.activity-rail', 'width: 46px');
assertDeclaration('.topbar-command-button', 'height: 26px');
assertDeclaration('.view-menu', 'position: absolute');
assertDeclaration('.view-menu', 'overflow-y: auto');
assertDeclaration('.view-menu-button-grid', 'grid-template-columns: repeat(3, minmax(0, 1fr))');
assertDeclaration('.dock-panel-manager-row', 'grid-template-columns: minmax(0, 74px) minmax(0, 1fr)');
assertDeclaration('.dock-panel-manager-actions', 'display: flex');
assertDeclaration('.workspace', 'grid-template-rows: auto auto auto minmax(0, 1fr)');
assertDeclaration('.workspace-arrangement', 'grid-template-rows: minmax(0, 1fr)');
assertDeclaration('.workspace-arrangement.context-top', 'grid-template-rows: auto minmax(0, 1fr)');
assertDeclaration('.context-identity-strip', 'display: flex');
assertDeclaration('.context-identity-strip', 'min-height: 18px');
assertDeclaration('.context-identity-item', 'display: inline-flex');
assertDeclaration('.context-identity-value', 'text-overflow: ellipsis');
assertDeclaration('.dock-panel-tabs', 'min-height: 22px');
assertDeclaration('.dock-panel-tabs.empty', 'height: 0');
assertDeclaration('.dock-panel-tab-group', 'display: inline-flex');
assertDeclaration('.dock-panel-tab', 'display: inline-flex');
assertDeclaration('.dock-panel-tab-label', 'height: 18px');
assertDeclaration('.dock-panel-tab-move', 'width: 18px');
assertDeclaration('.dock-panel-tab-close', 'width: 17px');
assertDeclaration('.workspace-arrangement.context-side', 'grid-template-columns: minmax(0, 1fr) 6px var(--context-pane-width)');
assertDeclaration('.workspace-arrangement.context-bottom', 'grid-template-rows: minmax(0, 1fr) 6px minmax(180px, var(--context-pane-height))');
assertDeclaration('.context-panel-grid.stacked', 'grid-template-columns: minmax(0, 1fr)');
assertDeclaration('.context-stack-tabs', 'display: flex');
assertDeclaration('.context-stack-tabs button', 'height: 24px');
assertDeclaration('.workspace-arrangement.context-side .context-panel-grid', 'overflow-y: auto');
assertDeclaration('.workspace-arrangement.context-bottom .context-panel-grid', 'overflow-y: auto');
assertDeclaration('.side-pane-resizer', 'cursor: col-resize');
assertDeclaration('.context-pane-resizer', 'cursor: col-resize');
assertDeclaration('.workspace-arrangement.context-bottom .context-pane-resizer', 'cursor: row-resize');
assertDeclaration('.activity-panel', 'grid-template-rows: auto auto minmax(0, 1fr)');
assertDeclaration('.activity-panel-list', 'overflow-y: auto');
assertDeclaration('.workspace-snapshot-row.active', 'border-color: color-mix(in srgb, var(--accent) 62%, transparent)');
assertDeclaration('.conversation-session-row.active', 'border-color: color-mix(in srgb, var(--accent) 62%, transparent)');
assertDeclaration('.conversation-session-open', 'grid-template-columns: auto minmax(0, 1fr)');
assert.ok(pageSource.includes('min-height: 58px'), 'Worktree rows should have stable dense height');
assertDeclaration('.activity-filter-box', 'grid-template-columns: 18px minmax(0, 1fr)');
assertDeclaration('.worktree-row-main', 'gap: 2px');
assertDeclaration('.worktree-safety-line', 'display: flex');
assertDeclaration('.worktree-context-main', 'display: grid');
assertDeclaration('.worktree-context-actions', 'display: inline-flex');
assertDeclaration('.worktree-status-badge.blocked', 'background: #d8aa55');
assertDeclaration('.run-loop-row', 'text-overflow: ellipsis');
assertDeclaration('.paste-cleanup-panel', 'overflow: hidden');
assertDeclaration('.paste-cleanup-grid', 'grid-template-columns: minmax(0, 1fr)');
assertDeclaration('.paste-cleanup-textarea', 'resize: none');
assertDeclaration('.editor-body-grid', 'grid-template-columns: minmax(0, 1fr) 8px var(--editor-insight-width)');
assertDeclaration('.editor-body-grid.insights-hidden', 'grid-template-columns: minmax(0, 1fr)');
assertDeclaration('.editor-canvas', 'position: relative');
assertDeclaration('.editor-insight-resizer', 'cursor: col-resize');
assertDeclaration('.terminal-launchpad', 'max-height: 392px');
assertDeclaration('.embedded-terminal-panel', 'display: grid');
assertDeclaration('.embedded-terminal-host', 'height: 168px');
assertDeclaration('.embedded-terminal-host', 'overflow: hidden');
assertDeclaration('.terminal-launchpad-grid', 'grid-template-columns: repeat(4, minmax(0, 1fr))');
assertDeclaration('.terminal-launchpad-row', 'display: grid');
assertDeclaration('.terminal-launchpad-row', 'grid-template-columns: auto minmax(0, 1fr) 24px');
assertDeclaration('.browser-dock', 'max-height: 428px');
assertDeclaration('.browser-frame-wrap', 'min-height: 280px');
assertDeclaration('.browser-frame', 'width: 100%');
assertDeclaration('.browser-runtime-list', 'overflow-x: auto');
assertDeclaration('.editor-toolbar', 'height: 24px');
assertDeclaration('.editor-file-state', 'height: 18px');
assertDeclaration('.editor-file-state', 'border: 0');
assertDeclaration('.editor-icon-button', 'height: 20px');
assertDeclaration('.editor-lookup-popover', 'position: absolute');
assertDeclaration('.git-command-drawer', 'flex: 0 0 auto');
assertDeclaration('.context-panel-grid.collapsed', 'display: none');
assertDeclaration('.editor-frame', 'height: auto');
assertDeclaration('.source-list-panel', 'overflow: hidden');
assertDeclaration('.file-tree', 'overflow-y: auto');
assertDeclaration('.file-tree', 'overflow-x: hidden');
assertDeclaration('.file-tree', 'scrollbar-gutter: stable');
assertDeclaration('.file-tree', 'scrollbar-width: thin');
assertDeclaration('.file-tree button.file-row small', 'display: none');
assert.ok(pageSource.includes("| 'runs'"), 'Source shell should define an orchestration runs activity mode');
assert.ok(pageSource.includes('sourceActivityModeStorageKey'), 'Source shell should persist the active activity mode');
assert.ok(pageSource.includes('pasteCleanupModeStorageKey'), 'Clipboard cleanup should persist its cleanup mode');
assert.ok(pageSource.includes('sidePaneWidthStorageKey'), 'Source shell should persist the side pane width');
assert.ok(pageSource.includes('contextPaneWidthStorageKey'), 'Workspace should persist the side context pane width');
assert.ok(pageSource.includes('sidePanePositionStorageKey'), 'Source shell should persist the side pane position');
assert.ok(pageSource.includes('sourceLayoutPresetStorageKey'), 'Source shell should persist the selected layout preset');
assert.ok(pageSource.includes('sourceLayoutVersionStorageKey'), 'Source shell should version layout storage migrations');
assert.ok(pageSource.includes('sourceLayoutPresetOverridesStorageKey'), 'Source shell should persist custom layout preset overrides');
assert.ok(pageSource.includes('sourceTerminalAppStorageKey'), 'Source shell should persist the selected terminal app');
assert.ok(pageSource.includes('sourceDockLayoutStorageKey'), 'Workspace should persist dock layout state');
assert.ok(pageSource.includes('browserDockUrlStorageKey'), 'Workspace should persist the browser dock URL');
assert.ok(pageSource.includes('snapshotStorageKey'), 'Workspace should persist conversation workspace snapshots');
assert.ok(pageSource.includes('activeWorkspaceSessionStorageKey'), 'Workspace should persist the active conversation session key');
assert.ok(pageSource.includes('contextPanelModeStorageKey'), 'Workspace should persist the context card layout mode');
assert.ok(pageSource.includes('contextPanelPlacementStorageKey'), 'Workspace should persist the context card placement');
assert.ok(pageSource.includes('hiddenContextCardsStorageKey'), 'Workspace should persist hidden context cards');
assert.ok(pageSource.includes('activeContextCardStorageKey'), 'Workspace should persist the active stacked context card');
assert.ok(pageSource.includes("type SourceLayoutPresetID = 'review' | 'code' | 'git' | 'runs' | 'sessions' | 'custom'"), 'Source shell should define named layout presets plus custom');
assert.ok(pageSource.includes("type SourceTerminalApp = 'Warp' | 'Terminal' | 'iTerm' | 'iTerm2' | 'Ghostty' | 'WezTerm' | 'Alacritty'"), 'Source shell should define supported terminal apps');
assert.ok(pageSource.includes("type SourceContextPanelMode = 'grid' | 'stack'"), 'Workspace should define context card layout modes');
assert.ok(pageSource.includes("type SourceContextPanelPlacement = 'top' | 'side' | 'bottom'"), 'Workspace should define context card placement modes');
assert.ok(pageSource.includes("type SourceSidePanePosition = 'left' | 'right'"), 'Source shell should define side pane positions');
assert.ok(pageSource.includes("type SourceContextCardID = 'orchestration' | 'runtime' | 'agents' | 'worktrees' | 'repo'"), 'Workspace should define hideable context cards');
assert.ok(pageSource.includes('const contextCardOrder'), 'Workspace should define a stable context card order');
assert.ok(pageSource.includes('const contextCardLabels'), 'Workspace should define compact context card labels');
assert.ok(pageSource.includes('contextPanelMode: SourceContextPanelMode'), 'Layout presets should include context card layout mode');
assert.ok(pageSource.includes('contextPanelPlacement: SourceContextPanelPlacement'), 'Layout presets should include context card placement');
assert.ok(pageSource.includes('sidePanePosition: SourceSidePanePosition'), 'Layout presets should include side pane position');
assert.ok(pageSource.includes('const sourceLayoutPresets'), 'Source shell should define reusable layout presets');
assert.ok(pageSource.includes('const managedDockPanelIDs'), 'Source shell should define managed dock panels');
assert.ok(pageSource.includes('const hideableDockPanelIDs'), 'Source shell should only expose hide controls for renderable hideable panels');
assert.ok(pageSource.includes("const sourceLayoutVersion = '2026-06-editor-canvas'"), 'Source shell should define the compact layout migration version');
assert.ok(pageSource.includes('const sourceTerminalApps'), 'Source shell should define reusable terminal app choices');
assert.ok(pageSource.includes('let sourceActivityMode'), 'Source shell should track the active side pane mode');
assert.ok(pageSource.includes('let sourceActivityFilter'), 'Source shell should track the side pane activity filter');
assert.ok(pageSource.includes('let pasteCleanupInput'), 'Clipboard cleanup should track the source text');
assert.ok(pageSource.includes('let pasteCleanupMode'), 'Clipboard cleanup should track the cleanup mode');
assert.ok(pageSource.includes('pasteCleanupOutput'), 'Clipboard cleanup should derive cleaned output');
assert.ok(pageSource.includes('buildWorktreeSafetySummary'), 'Worktree UI should use the shared safety model');
assert.ok(pageSource.includes('buildWorktreeCleanupBrief'), 'Worktree UI should use the shared cleanup brief model');
assert.ok(pageSource.includes('let projectWorktreeSafetyStats'), 'Worktree context should summarize safety counts');
assert.ok(pageSource.includes('let sidePaneWidth'), 'Source shell should track the resizable side pane width');
assert.ok(pageSource.includes('let contextPaneWidth'), 'Workspace should track the resizable side context pane width');
assert.ok(pageSource.includes('let sidePanePosition'), 'Source shell should track the side pane position');
assert.ok(pageSource.includes('let sourceLayoutPreset'), 'Source shell should track the active layout preset');
assert.ok(pageSource.includes('let sourceLayoutPresetOverrides'), 'Source shell should track saved layout preset overrides');
assert.ok(pageSource.includes('let sourceTerminalApp'), 'Source shell should track the selected terminal app');
assert.ok(pageSource.includes('let embeddedTerminalSession'), 'Source shell should track an embedded terminal session');
assert.ok(pageSource.includes('let embeddedTerminalSessions'), 'Source shell should track all embedded terminal sessions');
assert.ok(pageSource.includes('let workspaceSnapshots'), 'Source shell should track conversation workspace snapshots');
assert.ok(pageSource.includes('let activeWorkspaceSessionKey'), 'Source shell should track the active conversation workspace');
assert.ok(pageSource.includes('let activeWorkspaceSnapshot'), 'Source shell should derive the active saved workspace snapshot');
assert.ok(pageSource.includes('let embeddedTerminalElement'), 'Source shell should bind the embedded terminal host');
assert.ok(pageSource.includes('let contextPanelMode'), 'Workspace should track the context card layout mode');
assert.ok(pageSource.includes('let contextPanelPlacement'), 'Workspace should track context card placement');
assert.ok(pageSource.includes('let hiddenContextCardIDs'), 'Workspace should track hidden context cards');
assert.ok(pageSource.includes('let activeContextCardID'), 'Workspace should track the active stacked context card');
assert.ok(pageSource.includes('let viewMenuOpen'), 'Source shell should track the compact view menu');
assert.ok(pageSource.includes('let commandPaletteVisible'), 'Source shell should track command palette visibility');
assert.ok(pageSource.includes('function applySourceLayoutPreset'), 'Source shell should expose layout preset application');
assert.ok(pageSource.includes('function saveSourceLayoutPresetOverride'), 'Source shell should save custom layout presets');
assert.ok(pageSource.includes('function resetSourceLayoutPresetOverride'), 'Source shell should reset custom layout presets');
assert.ok(pageSource.includes('function captureSourceLayoutPresetOverride'), 'Source shell should capture a dock layout override');
assert.ok(pageSource.includes('function selectSourceTerminalApp'), 'Source shell should expose terminal app selection');
assert.ok(pageSource.includes('function selectContextPanelMode'), 'Workspace should expose context card layout selection');
assert.ok(pageSource.includes('function selectContextPanelPlacement'), 'Workspace should expose context card placement selection');
assert.ok(pageSource.includes('function moveDockPanelToGroup'), 'Workspace should move real panes through the dock model');
assert.ok(pageSource.includes('function hideDockPanel'), 'Workspace should hide real panes through the dock model');
assert.ok(pageSource.includes('function showDockPanel'), 'Workspace should restore real panes through the dock model');
assert.ok(pageSource.includes('function resetSourceDockLayout'), 'Workspace should reset dock layout to a known preset');
assert.ok(pageSource.includes('function toggleDockPanelVisibility'), 'Workspace should toggle dock panels from compact controls');
assert.ok(pageSource.includes('function dockPanelMoveTargets'), 'Workspace should constrain panel moves to renderable dock targets');
assert.ok(pageSource.includes('function sourceDockPanelVisible'), 'Workspace should detect visible dock panels');
assert.ok(pageSource.includes('function normalizeBrowserDockUrl'), 'Browser dock should normalize localhost and http URLs');
assert.ok(pageSource.includes('function openBrowserDock'), 'Browser dock should open a URL in the dock surface');
assert.ok(pageSource.includes('function reloadBrowserFrame'), 'Browser dock should reload the embedded preview frame');
assert.ok(pageSource.includes('function terminalDockSummary'), 'Terminal dock should summarize launch targets');
assert.ok(pageSource.includes('function ensureEmbeddedTerminalRenderer'), 'Terminal dock should lazily load xterm');
assert.ok(pageSource.includes('function startEmbeddedTerminalSession'), 'Terminal dock should start native PTY sessions');
assert.ok(pageSource.includes("startupCommand = ''"), 'Terminal dock should optionally start a PTY with an initial command');
assert.ok(pageSource.includes('function openPathEmbeddedTerminal'), 'Terminal dock should start embedded shells for arbitrary project paths');
assert.ok(pageSource.includes('function openWorkspaceSnapshotEmbeddedTerminal'), 'Saved workspace snapshots should resume inside embedded terminals');
assert.ok(pageSource.includes('function resumeAgentSessionEmbeddedTerminal'), 'Terminal dock should resume agents inside the embedded PTY');
assert.ok(pageSource.includes('function loadEmbeddedTerminalSessions'), 'Terminal dock should refresh native PTY sessions');
assert.ok(pageSource.includes('function attachEmbeddedTerminalSession'), 'Terminal dock should attach to existing native PTY sessions');
assert.ok(pageSource.includes('function closeListedEmbeddedTerminalSession'), 'Terminal dock should close listed native PTY sessions');
assert.ok(pageSource.includes('function closeEmbeddedTerminalSession'), 'Terminal dock should close native PTY sessions');
assert.ok(pageSource.includes('function fitEmbeddedTerminal'), 'Terminal dock should fit and resize native PTY sessions');
assert.ok(pageSource.includes('readTerminalSessionScrollbackFromTauri'), 'Terminal dock should restore native PTY scrollback when attaching');
assert.ok(pageSource.includes('function handleTerminalOutput'), 'Terminal dock should consume native PTY output events');
assert.ok(pageSource.includes('function disposeEmbeddedTerminal'), 'Terminal dock should dispose xterm and PTY resources');
assert.ok(pageSource.includes('function persistSourceDockLayout'), 'Workspace should persist source dock layout state');
assert.ok(pageSource.includes('function loadStoredSourceDockLayout'), 'Workspace should restore source dock layout state');
assert.ok(pageSource.includes('function captureCurrentWorkspaceSnapshot'), 'Workspace should capture the current conversation context');
assert.ok(pageSource.includes('function captureAgentSessionWorkspaceSnapshot'), 'Workspace should capture a specific agent session context');
assert.ok(pageSource.includes('function workspaceSnapshotEmbeddedTerminal'), 'Workspace snapshots should preserve embedded terminal context');
assert.ok(pageSource.includes('function captureActiveWorkspaceBeforeSwitch'), 'Workspace should refresh the active snapshot before switching conversations');
assert.ok(pageSource.includes('function restoreConversationWorkspaceSnapshot'), 'Workspace should restore a saved conversation context');
assert.ok(pageSource.includes('function restoreWorkspaceEmbeddedTerminal'), 'Workspace restore should reconnect a live embedded terminal when possible');
assert.ok(pageSource.includes('function restoreAgentSessionWorkspaceSnapshot'), 'Workspace should restore a specific agent session context');
assert.ok(pageSource.includes('function openAgentSessionWorkspace'), 'Conversation rows should open their saved workspace on row click');
assert.ok(pageSource.includes('function openWorkspaceSnapshotTerminal'), 'Workspace snapshots should launch saved resume commands in a terminal');
assert.ok(pageSource.includes('captureAgentSessionWorkspaceSnapshot(session);'), 'Agent terminal resume should refresh that session workspace snapshot');
assert.ok(pageSource.includes('function deleteWorkspaceSnapshot'), 'Workspace snapshots should be removable from local history');
assert.ok(pageSource.includes('function workspaceSnapshotForAgentSession'), 'Workspace should find saved context for a session row');
assert.ok(pageSource.includes('function workspaceSnapshotProjectForSession'), 'Workspace should store agent worktrees as restorable project roots');
assert.ok(pageSource.includes('function workspaceSnapshotSessionIDForSession'), 'Workspace snapshots should qualify cmux session IDs by concrete agent provider');
assert.ok(pageSource.includes("provider.startsWith('cmux-')"), 'Workspace snapshots should group cmux-backed sessions under the cmux provider');
assert.ok(pageSource.includes('function activateWorkspaceSnapshotProject'), 'Workspace restore should force a full project scan before reopening files');
assert.ok(pageSource.includes('function sourceRecordFromRestoredPath'), 'Workspace restore should reopen saved paths even when they are missing from the index');
assert.ok(pageSource.includes('function sourceLanguageForRestoredPath'), 'Workspace restore should infer language for direct saved-path previews');
assert.ok(pageSource.includes('function persistWorkspaceSnapshots'), 'Workspace should persist captured conversation contexts');
assert.ok(pageSource.includes('function loadStoredActiveWorkspaceSessionKey'), 'Workspace should restore the active conversation key');
assert.ok(pageSource.includes('function persistActiveWorkspaceSessionKey'), 'Workspace should persist active conversation switches');
assert.ok(pageSource.includes('function markAgentSessionWorkspaceActive'), 'Workspace should share active-session marking across open and resume actions');
assert.ok(
  (pageSource.match(/captureActiveWorkspaceBeforeSwitch\(\);/g) ?? []).length >= 2,
  'Conversation workspace or terminal switching should save the previous active workspace first'
);
assert.ok(
  (pageSource.match(/markAgentSessionWorkspaceActive\(session\);/g) ?? []).length >= 2,
  'Opening or resuming a conversation should mark it as active'
);
assert.ok(pageSource.includes('class:active={activeWorkspaceSessionKey === snapshot.id}'), 'Saved workspace rows should show the active workspace');
assert.ok(pageSource.includes('class:active={activeWorkspaceSessionKey === workspaceSnapshotIDForAgentSession(session)}'), 'Conversation session rows should show the active workspace');
assert.ok(pageSource.includes("id: 'conversation-restore-active'"), 'Command palette should restore the active workspace snapshot');
assert.ok(pageSource.includes("activeWorkspaceSnapshot?.title ?? 'No active workspace'"), 'Active workspace command should explain when nothing can be restored');
assert.ok(pageSource.includes('restoreConversationWorkspaceSnapshot(activeWorkspaceSnapshot)'), 'Active workspace command should restore the active snapshot');
assert.ok(pageSource.includes('function hideContextCard'), 'Workspace should expose per-card hiding');
assert.ok(pageSource.includes('function showAllContextCards'), 'Workspace should expose hidden-card restore');
assert.ok(pageSource.includes('function shouldRenderContextCard'), 'Workspace should render stacked context cards through one predicate');
assert.ok(pageSource.includes('function selectActiveContextCard'), 'Workspace should expose stacked context card selection');
assert.ok(pageSource.includes('function toggleViewMenu'), 'Source shell should expose a compact view menu');
assert.ok(pageSource.includes('function closeViewMenu'), 'Source shell should close the compact view menu');
assert.ok(pageSource.includes('function openCommandPalette'), 'Source shell should expose a command palette');
assert.ok(pageSource.includes('function runCommandPaletteItem'), 'Command palette should execute selected commands');
assert.ok(pageSource.includes('function openWorkspaceSymbolQuickOpen'), 'Command palette should expose workspace symbol quick-open');
assert.ok(pageSource.includes('function selectSidePanePosition'), 'Source shell should expose side pane placement selection');
assert.ok(pageSource.includes('function markSourceLayoutCustom'), 'Manual layout changes should mark the layout as custom');
assert.ok(pageSource.includes('function shouldMigrateSourceLayout'), 'Source shell should migrate old dense layout storage');
assert.ok(pageSource.includes('function persistSourceLayoutVersion'), 'Source shell should persist layout migration state');
assert.ok(pageSource.includes('function resetProjectScanCache'), 'Source shell should reset stale project indexes');
assert.ok(pageSource.includes('function sourceOnboardingScanStatus'), 'Project onboarding should explain expanded source scans');
assert.ok(pageSource.includes('function clearSourceRecordsForIncomingProject'), 'Source shell should clear stale records when switching roots');
assert.ok(pageSource.includes('function sourceRecordBelongsToProject'), 'Source shell should guard scan records by project path');
assert.ok(pageSource.includes('function activateDuplicateProjectRoot'), 'Duplicate project adds should activate the existing root');
assert.ok(
  pageSource.includes("'Project already listed. Switching to it now.'"),
  'Duplicate project adds should explain that the existing root is being activated'
);
assert.ok(pageSource.includes('function copyTextToClipboard'), 'Activity rows should share clipboard copy behavior');
assert.ok(pageSource.includes('function copyActivityCommand'), 'Activity rows should copy resume commands and paths');
assert.ok(pageSource.includes('function openActivityPath'), 'Activity rows should open repo and worktree paths');
assert.ok(pageSource.includes('function revealActivityPath'), 'Activity rows should reveal repo and worktree paths');
assert.ok(pageSource.includes('function openActivityTerminalPath'), 'Activity rows should open repo and worktree paths in a terminal');
assert.ok(pageSource.includes('function openAgentSessionTerminal'), 'Agent rows should resume sessions in a terminal');
assert.ok(pageSource.includes('function activityTextMatchesFilter'), 'Activity rows should share filter matching logic');
assert.ok(pageSource.includes('function sourceActivityFilterPlaceholder'), 'Activity filter placeholder should match the active panel');
assert.ok(pageSource.includes('function selectSourceActivityMode'), 'Source shell should expose activity mode selection');
assert.ok(pageSource.includes('function beginSidePaneResize'), 'Source shell should expose side pane drag resizing');
assert.ok(pageSource.includes('function beginContextPaneResize'), 'Workspace should expose context pane drag resizing');
assert.ok(pageSource.includes('resizeSourceDockGroup'), 'Workspace should persist pane resize changes into the dock model');
assert.ok(pageSource.includes('sourceDockGroupSize'), 'Workspace should restore pane sizes from the dock model');
assert.ok(pageSource.includes('function persistDockGroupSize'), 'Workspace should share dock size persistence across resizers');
assert.ok(pageSource.includes('activateSourceDockPanel'), 'Workspace should activate stacked dock panels through the dock model');
assert.ok(pageSource.includes('function selectDockPanel'), 'Workspace should expose dock tab selection');
assert.ok(pageSource.includes('function activeDockPanelForGroup'), 'Workspace should resolve active panels for dock groups');
assert.ok(pageSource.includes('class="dock-panel-tabs"'), 'Workspace should render compact dock panel tabs');
assert.ok(pageSource.includes('class="dock-panel-tab"'), 'Dock tabs should wrap labels and actions in compact tab items');
assert.ok(pageSource.includes('class="dock-panel-tab-move"'), 'Dock tabs should expose direct move controls');
assert.ok(pageSource.includes('function dockGroupShortcutLabel'), 'Dock tab move controls should use compact placement labels');
assert.ok(pageSource.includes('aria-label={`Move ${dockPanelLabel(panelID)} panel from tab`}'), 'Dock tab move controls should be accessible');
assert.ok(pageSource.includes('onchange={(event) => moveDockPanelFromTab(panelID, event)}'), 'Dock tab move controls should use the dock model');
assert.ok(pageSource.includes('aria-label={`Hide ${dockPanelLabel(panelID)} panel from tab`}'), 'Dock tabs should expose direct close controls for hideable panels');
assert.ok(pageSource.includes('onclick={() => hideDockPanel(panelID)}'), 'Dock tab close controls should hide panels through the dock model');
assert.ok(pageSource.includes('function projectWorktreeActivityLabel'), 'Worktree rows should format last activity labels');
assert.ok(pageSource.includes('function projectWorktreeSafety'), 'Worktree rows should derive cleanup safety details');
assert.ok(pageSource.includes('function formatProjectWorktreeSafetyStats'), 'Worktree context should derive compact safety stats');
assert.ok(pageSource.includes('projectWorktreeCleanupBrief'), 'Worktree context should derive a cleanup brief');
assert.ok(pageSource.includes('function copyWorktreeCleanupPlan'), 'Worktree rows should copy cleanup plans');
assert.ok(pageSource.includes('function copyWorktreeAuditCommand'), 'Worktree commands should copy audit commands');
assert.ok(pageSource.includes('function copyWorktreeCleanupCommand'), 'Worktree commands should copy remove commands');
assert.ok(pageSource.includes('function copyWorktreeBackupCommand'), 'Worktree rows should copy backup commands');
assert.ok(pageSource.includes('function copyProjectWorktreeCleanupScript'), 'Worktree panel should copy a guarded cleanup script');
assert.ok(pageSource.includes('worktreePrimaryAction'), 'Worktree rows should derive a recommended next action');
assert.ok(pageSource.includes('function copyWorktreePrimaryAction'), 'Worktree rows should copy the recommended next action command');
assert.ok(pageSource.includes('function runWorktreePrimaryAction'), 'Worktree rows should execute safe recommended actions');
assert.ok(pageSource.includes('aria-label={`${primaryAction.label} worktree: ${worktree.branch}`}'), 'Worktree rows should expose a compact recommended action button');
assert.ok(pageSource.includes('Recommended worktree action'), 'Command palette should expose recommended worktree actions');
assert.ok(pageSource.includes('function agentSessionResumePlan'), 'Agent rows should expose a copyable resume plan');
assert.ok(pageSource.includes('function agentSessionResumeShellCommand'), 'Agent rows should expose shell-ready resume commands');
assert.ok(pageSource.includes('function agentSessionModelLabel'), 'Agent rows should expose model metadata when session scanners find it');
assert.ok(pageSource.includes('session.model'), 'Agent session filtering and rows should include model metadata');
assert.ok(pageSource.includes('model: session?.model ?? null'), 'Workspace snapshots should preserve the agent session model');
assert.ok(pageSource.includes('function copyAgentSessionResumePlan'), 'Agent commands should copy session resume plans');
assert.ok(pageSource.includes('function copyAgentSessionResumeShellCommand'), 'Agent commands should copy shell-ready resume commands');
assert.ok(pageSource.includes('editorInsightWidthStorageKey'), 'Editor shell should persist the inspector width');
assert.ok(pageSource.includes('editorInsightCollapsedStorageKey'), 'Editor shell should persist inspector visibility');
assert.ok(pageSource.includes('contextPanelCollapsedStorageKey'), 'Workspace should persist collapsed context cards');
assert.ok(pageSource.includes('let editorInsightWidth'), 'Editor shell should track inspector width');
assert.ok(pageSource.includes('let editorInsightCollapsed'), 'Editor shell should track inspector visibility');
assert.ok(pageSource.includes('let contextPanelCollapsed'), 'Workspace should track context card collapse state');
assert.ok(pageSource.includes('function beginEditorInsightResize'), 'Editor shell should expose inspector drag resizing');
assert.ok(pageSource.includes('function toggleEditorInsightCollapsed'), 'Editor shell should expose inspector collapse');
assert.ok(pageSource.includes('function showEditorInsightPanel'), 'Editor shell should reopen a requested inspector panel');
assert.ok(pageSource.includes('function clearSourceLookupResults'), 'Editor shell should clear inline lookup results');
assert.ok(pageSource.includes('function toggleContextPanelCollapsed'), 'Workspace should expose context card collapse');
assert.ok(pageSource.includes('function showContextCard'), 'Workspace should expose individual context card restore');
assert.ok(pageSource.includes('if (storedWidth === null) return sidePaneDefaultWidth'), 'Missing side pane storage should use the designed default width');
assert.ok(pageSource.includes('if (storedWidth === null) return editorInsightDefaultWidth'), 'Missing editor inspector storage should use the designed default width');
assert.ok(pageSource.includes('if (storedWidth === null) return contextPaneDefaultWidth'), 'Missing context pane storage should use the designed default width');
assert.ok(pageSource.includes('class="activity-rail"'), 'Source shell should render an activity rail');
assert.ok(pageSource.includes("class:side-right={sidePanePosition === 'right'}"), 'Source shell should support moving the side pane to the right');
assert.ok(pageSource.includes("class:context-top={contextPanelPlacement === 'top' && shouldRenderDockPanel('context')}"), 'Workspace should support top context placement only when the context dock is active');
assert.ok(pageSource.includes("class:context-side={contextPanelPlacement === 'side' && shouldRenderDockPanel('context')}"), 'Workspace should support side context placement only when the context dock is active');
assert.ok(pageSource.includes("class:context-bottom={contextPanelPlacement === 'bottom' && shouldRenderDockPanel('context')}"), 'Workspace should support bottom context placement only when the context dock is active');
assert.ok(pageSource.includes('class="workspace-arrangement"'), 'Workspace should wrap context and editor into a rearrangeable layout');
assert.ok(pageSource.includes('class="context-identity-item"'), 'Workspace context identity should render as a compact status line');
assert.ok(pageSource.includes('class="workspace-main-column"'), 'Workspace should isolate the editor column');
assert.ok(pageSource.includes('class="workspace-context-column"'), 'Workspace should isolate the context column');
assert.ok(pageSource.includes('class="topbar-command-button"'), 'Topbar should expose compact command controls');
assert.ok(pageSource.includes('class="view-menu"'), 'Topbar should tuck layout controls into a view menu');
assert.ok(pageSource.includes('aria-label="Workspace layout presets"'), 'View menu should group layout presets for assistive tech');
assert.ok(pageSource.includes('aria-label="Terminal app"'), 'Terminal app picker should be accessible');
assert.ok(pageSource.includes('openTerminalPathFromTauri(path, sourceTerminalApp)'), 'Terminal open actions should use the selected terminal app');
assert.ok(pageSource.includes('openTerminalCommandFromTauri(path, command, sourceTerminalApp)'), 'Agent resume actions should use the selected terminal app');
assert.ok(pageSource.includes('aria-label="Workspace views"'), 'Activity rail should be labeled');
assert.ok(pageSource.includes('aria-label="Files"'), 'Activity rail should expose files');
assert.ok(pageSource.includes('aria-label="Clipboard"'), 'Activity rail should expose clipboard cleanup');
assert.ok(pageSource.includes('aria-label="Conversations"'), 'Activity rail should expose conversations');
assert.ok(pageSource.includes('aria-label="Runs"'), 'Activity rail should expose orchestration runs');
assert.ok(pageSource.includes('aria-label="Active sessions"'), 'Activity rail should expose active sessions');
assert.ok(pageSource.includes('aria-label="Agents"'), 'Activity rail should expose agents');
assert.ok(pageSource.includes('aria-label="Worktrees"'), 'Activity rail should expose worktrees');
assert.ok(pageSource.includes('aria-label="Git and tasks"'), 'Activity rail should expose Git and tasks');
assert.ok(pageSource.includes('aria-label="Copy worktree cleanup plan"'), 'Worktree rows should expose copyable cleanup plans');
assert.ok(pageSource.includes('aria-label={`${primaryAction.label} worktree: ${worktree.branch}`}'), 'Worktree rows should expose a recommended cleanup action');
assert.ok(pageSource.includes('class="side-pane-resizer"'), 'Source shell should render a side pane resizer');
assert.ok(pageSource.includes('aria-label="Resize side pane"'), 'Side pane resizer should be labeled');
assert.ok(pageSource.includes('class="context-pane-resizer"'), 'Workspace should render a context pane resizer');
assert.ok(pageSource.includes('aria-label="Resize context pane"'), 'Context pane resizer should be labeled');
assert.ok(pageSource.includes('context-card-close'), 'Workspace context cards should render per-card close controls');
assert.ok(pageSource.includes('class="context-restore-button"'), 'Workspace should render hidden-card restore when needed');
assert.ok(pageSource.includes('class="context-stack-tabs"'), 'Workspace should render tabs for stacked context cards');
assert.ok(pageSource.includes('aria-label="Context card tabs"'), 'Stacked context tabs should be accessible');
assert.ok(pageSource.includes('class="terminal-launchpad"'), 'Workspace should render a terminal launchpad dock');
assert.ok(pageSource.includes('aria-label="Terminal dock"'), 'Terminal launchpad should be accessible');
assert.ok(pageSource.includes('aria-label="Terminal dock app"'), 'Terminal launchpad should expose terminal app selection');
assert.ok(pageSource.includes('aria-label="Embedded terminal"'), 'Terminal launchpad should render an embedded terminal surface');
assert.ok(pageSource.includes('aria-label="Embedded terminal sessions"'), 'Terminal launchpad should list embedded PTY sessions');
assert.ok(pageSource.includes('bind:this={embeddedTerminalElement}'), 'Embedded terminal should bind its xterm host');
assert.ok(pageSource.includes('aria-label="Start embedded terminal"'), 'Terminal launchpad should start embedded sessions');
assert.ok(pageSource.includes('aria-label="Stop embedded terminal"'), 'Terminal launchpad should stop embedded sessions');
assert.ok(pageSource.includes('aria-label="Fit embedded terminal"'), 'Terminal launchpad should resize embedded sessions');
assert.ok(pageSource.includes('aria-label="Attach embedded terminal session"'), 'Terminal launchpad should attach embedded sessions');
assert.ok(pageSource.includes('aria-label="Close listed embedded terminal session"'), 'Terminal launchpad should close listed embedded sessions');
assert.ok(pageSource.includes('aria-label="Resume agent from terminal dock"'), 'Terminal launchpad should resume agents');
assert.ok(pageSource.includes('onclick={() => resumeAgentSessionEmbeddedTerminal(session)}'), 'Terminal launchpad should run agent resumes inside the embedded PTY');
assert.ok(pageSource.includes('aria-label="Open worktree from terminal dock"'), 'Terminal launchpad should open worktrees');
assert.ok(pageSource.includes('onclick={() => openPathEmbeddedTerminal(worktree.path)}'), 'Terminal worktree shortcuts should open embedded worktree shells');
assert.ok(pageSource.includes('onclick={() => openPathEmbeddedTerminal(context.cwd)}'), 'Terminal runtime shortcuts should open embedded shells at active context cwd');
assert.ok(pageSource.includes('onclick={() => openWorkspaceSnapshotEmbeddedTerminal(snapshot)}'), 'Saved workspace shortcuts should open embedded snapshot shells');
assert.ok(pageSource.includes('onclick={() => resumeAgentSessionEmbeddedTerminal(session)}'), 'Conversation shortcuts should resume agents inside embedded terminals');
assert.ok(pageSource.includes('disabled={!selectedProject.path || embeddedTerminalStarting}'), 'Terminal dock should allow a new embedded session while another session is active');
assert.ok(!pageSource.includes('Stop or attach a matching terminal before resume'), 'Agent resume should start a matching embedded session instead of blocking on another cwd');
assert.ok(pageSource.includes('aria-label="Browser dock"'), 'Workspace should render a browser dock');
assert.ok(pageSource.includes('class="browser-frame"'), 'Browser dock should render an iframe surface');
assert.ok(pageSource.includes('bind:value={browserInputUrl}'), 'Browser dock should expose an editable URL field');
assert.ok(pageSource.includes('onclick={() => openRuntimeContextInBrowserDock(context)}'), 'Browser dock should open runtime contexts inline');
assert.ok(pageSource.includes('listenToTerminalOutput(handleTerminalOutput)'), 'Source shell should subscribe to terminal output events');
assert.ok(pageSource.includes('writeTerminalSessionFromTauri(embeddedTerminalSession.sessionId, data)'), 'Embedded terminal should write input to the native PTY');
assert.ok(pageSource.includes('embeddedTerminal.write(scrollback)'), 'Embedded terminal attach should replay buffered scrollback');
assert.ok(pageSource.includes('resizeTerminalSessionFromTauri('), 'Embedded terminal should call the native PTY resize command');
assert.ok(pageSource.includes('embeddedTerminalSession.sessionId'), 'Embedded terminal should address the active native PTY session');
assert.ok(pageSource.includes('closeTerminalSessionFromTauri(session.sessionId)'), 'Embedded terminal should close the native PTY');
assert.ok(pageSource.includes("id: 'terminal-refresh-embedded'"), 'Command palette should refresh embedded terminal sessions');
assert.ok(pageSource.includes('id: `terminal-embedded-resume-${session.provider}-${session.id}`'), 'Command palette should expose embedded agent resumes');
assert.ok(pageSource.includes('id: `terminal-attach-${session.sessionId}`'), 'Command palette should attach embedded terminal sessions');
assert.ok(pageSource.includes('id: `terminal-close-${session.sessionId}`'), 'Command palette should close embedded terminal sessions');
assert.ok(pageSource.includes('class="command-palette-layer"'), 'Source shell should render a command palette overlay');
assert.ok(pageSource.includes('aria-label="Command palette"'), 'Command palette should be accessible');
assert.ok(pageSource.includes("event.key.toLowerCase() === 'k'"), 'Command palette should open from Cmd+K');
assert.ok(pageSource.includes("id: 'go-to-line'"), 'Command palette should expose current-file line navigation');
assert.ok(pageSource.includes("id: 'workspace-symbols'"), 'Command palette should expose workspace symbol search');
assert.ok(pageSource.includes('function openCurrentFileGoToLine'), 'Line navigation should reuse quick open');
assert.ok(pageSource.includes('quickOpenQuery = `${selectedRecord.relativePath}:`'), 'Line navigation should prefill the current file path');
assert.ok(pageSource.includes("quickOpenQuery = '#'"), 'Workspace symbol search should prefill the quick-open symbol prefix');
assert.ok(pageSource.includes('function chooseQuickOpenWorkspaceSymbol'), 'Quick open should select workspace symbols directly');
assert.ok(pageSource.includes("id: 'scan-project-expanded'"), 'Command palette should expose expanded source scans');
assert.ok(pageSource.includes("id: 'scan-reset-index'"), 'Command palette should expose source index reset');
assert.ok(pageSource.includes("id: 'scan-stop'"), 'Command palette should expose scan cancellation');
assert.ok(pageSource.includes("id: 'project-add-folder'"), 'Command palette should expose project folder selection');
assert.ok(pageSource.includes("id: 'project-open-folder'"), 'Command palette should open the current project folder');
assert.ok(pageSource.includes("id: 'project-reveal-folder'"), 'Command palette should reveal the current project folder');
assert.ok(pageSource.includes("id: 'project-open-terminal'"), 'Command palette should open the current project in the selected terminal');
assert.ok(pageSource.includes('showContextCard(cardID)'), 'Command palette should restore individual context cards');
assert.ok(pageSource.includes("id: 'activity-clipboard'"), 'Command palette should switch to clipboard cleanup');
assert.ok(pageSource.includes("id: 'activity-conversations'"), 'Command palette should switch to conversations');
assert.ok(pageSource.includes("id: 'activity-refresh'"), 'Command palette should refresh the current activity lane');
assert.ok(pageSource.includes('worktree-cleanup-plan-${worktree.path}'), 'Command palette should copy worktree cleanup plans');
assert.ok(pageSource.includes('worktree-audit-command-${worktree.path}'), 'Command palette should copy worktree audit commands');
assert.ok(pageSource.includes('worktree-remove-command-${worktree.path}'), 'Command palette should copy worktree remove commands');
assert.ok(pageSource.includes('worktree-backup-command-${worktree.path}'), 'Command palette should copy worktree backup commands');
assert.ok(pageSource.includes("id: 'worktree-cleanup-brief'"), 'Command palette should copy the aggregate worktree cleanup brief');
assert.ok(pageSource.includes("id: 'worktree-cleanup-script'"), 'Command palette should copy the guarded worktree cleanup script');
assert.ok(pageSource.includes('agent-resume-${session.provider}-${session.id}'), 'Command palette should expose agent resume targets');
assert.ok(pageSource.includes('agent-copy-plan-${session.provider}-${session.id}'), 'Command palette should copy session resume plans');
assert.ok(pageSource.includes('agent-copy-shell-command-${session.provider}-${session.id}'), 'Command palette should copy shell-ready session commands');
assert.ok(pageSource.includes('agent-copy-resume-command-${session.provider}-${session.id}'), 'Command palette should copy raw session resume commands');
assert.ok(pageSource.includes("id: 'lsp-retry-status'"), 'Command palette should retry LSP status checks');
assert.ok(pageSource.includes("id: 'lsp-copy-status'"), 'Command palette should copy LSP status reports');
assert.ok(pageSource.includes("id: 'lsp-copy-install'"), 'Command palette should copy LSP install commands');
assert.ok(pageSource.includes('Go to symbol: ${symbol.name}'), 'Command palette should expose current-file symbols');
assert.ok(pageSource.includes('selectSourceSymbol(symbol)'), 'Command palette symbol commands should reveal source lines');
assert.ok(pageSource.includes('Go to problem: ${diagnostic.message}'), 'Command palette should expose current-file diagnostics');
assert.ok(pageSource.includes('selectSourceDiagnostic(diagnostic)'), 'Command palette diagnostic commands should reveal source lines');
assert.ok(pageSource.includes('class="view-menu-button-grid"'), 'Workspace should render context card layout controls inside the view menu');
assert.ok(pageSource.includes('aria-label="Context card layout"'), 'Context card layout controls should be accessible');
assert.ok(pageSource.includes('aria-label="Side pane position"'), 'Side pane placement controls should be accessible');
assert.ok(pageSource.includes('aria-label="Context card layout"'), 'Context placement controls should be accessible');
assert.ok(pageSource.includes('aria-label="Save current layout preset"'), 'View menu should save the current layout as a preset override');
assert.ok(pageSource.includes('aria-label="Reset saved layout preset"'), 'View menu should reset the active saved layout override');
assert.ok(pageSource.includes("selectSidePanePosition('left')"), 'Side pane controls should select the left position');
assert.ok(pageSource.includes("selectSidePanePosition('right')"), 'Side pane controls should select the right position');
assert.ok(pageSource.includes("selectContextPanelPlacement('top')"), 'Context placement controls should select top placement');
assert.ok(pageSource.includes("selectContextPanelPlacement('side')"), 'Context placement controls should select side placement');
assert.ok(pageSource.includes("moveDockPanelToGroup('context', 'bottom')"), 'Context placement controls should dock context at the bottom');
assert.ok(pageSource.includes("id: 'context-bottom'"), 'Command palette should dock context at the bottom');
assert.ok(pageSource.includes("id: 'layout-reset-dock'"), 'Command palette should reset the dock layout');
assert.ok(pageSource.includes('layout-save-${preset.id}'), 'Command palette should save custom layout presets');
assert.ok(pageSource.includes('layout-reset-${preset.id}'), 'Command palette should reset saved layout presets');
assert.ok(pageSource.includes('dock-move-${panelID}-${groupID}'), 'Command palette should expose valid panel move targets');
assert.ok(pageSource.includes('dock-toggle-${panelID}'), 'Command palette should expose panel visibility toggles');
assert.ok(pageSource.includes('dock-panel-manager'), 'View menu should expose a compact dock panel manager');
assert.ok(pageSource.includes('aria-label="Dock panels"'), 'Dock panel manager should be accessible');
assert.ok(pageSource.includes("id: 'dock-show-terminal'"), 'Command palette should expose the future terminal dock panel');
assert.ok(pageSource.includes("id: 'dock-hide-terminal'"), 'Command palette should hide the terminal dock panel');
assert.ok(pageSource.includes("id: 'dock-show-browser'"), 'Command palette should expose the browser dock panel');
assert.ok(pageSource.includes("id: 'browser-open-runtime'"), 'Command palette should open the active runtime in the browser dock');
assert.ok(pageSource.includes("id: 'terminal-open-project'"), 'Command palette should open the current project shell');
assert.ok(pageSource.includes("id: 'conversation-save-snapshot'"), 'Command palette should save the current conversation workspace');
assert.ok(pageSource.includes("id: 'conversation-restore-latest'"), 'Command palette should restore the latest conversation workspace');
assert.ok(pageSource.includes("id: 'conversation-resume-latest'"), 'Command palette should resume the latest workspace snapshot in terminal');
assert.ok(pageSource.includes("id: 'conversation-resume-latest-embedded'"), 'Command palette should resume the latest workspace snapshot in the embedded terminal');
assert.ok(pageSource.includes("id: 'conversation-delete-latest'"), 'Command palette should delete the latest workspace snapshot');
assert.ok(pageSource.includes('conversation-save-session-workspace'), 'Command palette should save a specific session workspace');
assert.ok(pageSource.includes('conversation-restore-session-workspace'), 'Command palette should restore a specific session workspace');
assert.ok(pageSource.includes('orchestrationLoopTallyText'), 'Runs should render shared orchestration loop tally text');
assert.ok(pageSource.includes('orchestrationRunStage(run, runMetrics)'), 'Runs should derive a compact current stage label');
assert.ok(pageSource.includes('orchestrationLoopStageMetrics(runMetrics)'), 'Runs should derive compact loop stage metrics');
assert.ok(pageSource.includes('class="run-loop-row"'), 'Runs should render a compact loop tally row');
assert.ok(pageSource.includes('class={`run-stage-badge ${runStage.tone}`}'), 'Runs should render a current stage badge');
assert.ok(pageSource.includes('class="run-loop-stage-strip"'), 'Runs should render a dense loop stage strip');
assert.ok(pageSource.includes('aria-label="Run loop stages"'), 'Run loop stage strip should be accessible');
assert.ok(pageSource.includes('aria-label="Run loop tally"'), 'Run loop tally should be accessible');
assert.ok(pageSource.includes("selectContextPanelMode('grid')"), 'Context layout controls should select grid mode');
assert.ok(pageSource.includes("selectContextPanelMode('stack')"), 'Context layout controls should select stack mode');
assert.ok(pageSource.includes("class:stacked={contextPanelMode === 'stack'}"), 'Context card grid should support stacked layout');
assert.ok(pageSource.includes('class="editor-insight-resizer"'), 'Editor shell should render an inspector resizer');
assert.ok(pageSource.includes('aria-label="Resize editor insights"'), 'Inspector resizer should be labeled');
assert.ok(pageSource.includes("class:insights-hidden={editorInsightCollapsed || !shouldRenderDockPanel('insights')}"), 'Editor shell should remove the inspector from layout when collapsed or another dock tab is active');
assert.ok(pageSource.includes('class="editor-canvas"'), 'Editor shell should wrap Monaco in a canvas for overlays');
assert.ok(pageSource.includes('class="editor-lookup-popover"'), 'Editor shell should render inline lookup results over the editor');
assert.ok(pageSource.includes("showEditorInsightPanel('git')"), 'Editor actions should reopen the Git inspector');
assert.ok(pageSource.includes('class="git-command-drawer"'), 'Git inspector should tuck write actions into a compact drawer');
assert.ok(pageSource.includes('class="activity-panel"'), 'Non-file side modes should render activity panels');
assert.ok(pageSource.includes('class="activity-filter-box"'), 'Activity panels should render a filter field');
assert.ok(pageSource.includes('class="activity-panel-list"'), 'Activity panels should render scrollable lists');
assert.ok(pageSource.includes('class="paste-cleanup-panel"'), 'Clipboard mode should render the paste cleanup panel');
assert.ok(pageSource.includes('aria-label="Paste cleanup input"'), 'Paste cleanup input should be accessible');
assert.ok(pageSource.includes('aria-label="Cleaned paste output"'), 'Paste cleanup output should be accessible');
assert.ok(pageSource.includes('onclick={readPasteCleanupClipboard}'), 'Paste cleanup should read the clipboard on demand');
assert.ok(pageSource.includes('onclick={copyPasteCleanupOutput}'), 'Paste cleanup should copy the cleaned output');
assert.ok(pageSource.includes('cleanupPasteText(pasteCleanupInput, pasteCleanupMode)'), 'Paste cleanup should use the shared cleanup helper');
assert.ok(pageSource.includes('class="activity-row-actions"'), 'Activity rows should render compact action controls');
assert.ok(pageSource.includes('placeholder={sourceActivityFilterPlaceholder(sourceActivityMode)}'), 'Activity filter placeholder should be dynamic');
assert.ok(pageSource.includes('aria-label="Filter workspace activity"'), 'Activity filter should be accessible');
assert.ok(pageSource.includes('filteredProjectAgentSessions'), 'Activity panels should filter conversation and agent rows');
assert.ok(pageSource.includes('filteredProjectOrchestrationRuns'), 'Activity panels should filter orchestration run rows');
assert.ok(pageSource.includes('listOrchestrationRunsFromTauri'), 'Source page should load orchestration runs from the native event store');
assert.ok(pageSource.includes('class="activity-run-row"'), 'Runs mode should render orchestration run cards');
assert.ok(pageSource.includes('class="orchestration-context-panel"'), 'Workspace context should include orchestration run status');
assert.ok(pageSource.includes('runMetrics.stepCount'), 'Orchestration run cards should expose step counts');
assert.ok(pageSource.includes('runMetrics.agentCount'), 'Orchestration run cards should expose agent counts');
assert.ok(pageSource.includes('runMetrics.artifactCount'), 'Orchestration run cards should expose artifact counts');
assert.ok(pageSource.includes('orchestrationRunMetrics(run)'), 'Orchestration run cards should derive status metrics');
assert.ok(pageSource.includes('orchestrationRunSummaryText'), 'Orchestration run cards should use shared copy summary formatting');
assert.ok(pageSource.includes('orchestrationTimelineItems(run, 6)'), 'Orchestration run cards should render timeline items');
assert.ok(pageSource.includes('orchestrationCurrentActivity(run)'), 'Orchestration run cards should expose current activity');
assert.ok(pageSource.includes('orchestrationArtifactChips(run)'), 'Orchestration run cards should expose artifact chips');
assert.ok(pageSource.includes('orchestrationLinkChips(run)'), 'Orchestration run cards should expose link chips');
assert.ok(pageSource.includes('function focusOrchestrationRun'), 'Command palette should focus a specific orchestration run');
assert.ok(pageSource.includes('function copyOrchestrationRunSummary'), 'Runs mode should copy a full run summary');
assert.ok(pageSource.includes('function copyOrchestrationCurrentActivity'), 'Runs mode should copy the current run activity');
assert.ok(pageSource.includes('function copyOrchestrationTaskReference'), 'Runs mode should copy task references from run metadata');
assert.ok(pageSource.includes('id: `run-focus-${run.id}`'), 'Command palette should expose run focus commands');
assert.ok(pageSource.includes('id: `run-copy-summary-${run.id}`'), 'Command palette should expose run summary copy commands');
assert.ok(pageSource.includes('class="run-current-activity"'), 'Runs mode should render current activity');
assert.ok(pageSource.includes('class="run-timeline"'), 'Runs mode should render a timeline');
assert.ok(pageSource.includes('class="run-artifact-row"'), 'Runs mode should render artifact/link chips');
assert.ok(pageSource.includes('aria-label="Copy run summary"'), 'Run rows should expose summary copy actions');
assert.ok(pageSource.includes('aria-label="Copy current run activity"'), 'Run rows should expose current activity copy actions');
assert.ok(pageSource.includes('aria-label="Copy run task reference"'), 'Run rows should expose task reference copy actions');
assert.ok(pageSource.includes('{runMetrics.retryCount} retries'), 'Runs mode should show retry counts');
assert.ok(pageSource.includes('{runMetrics.approvalCount} sign-off'), 'Runs mode should show sign-off counts');
assert.ok(pageSource.includes('filteredProjectWorktrees'), 'Activity panels should filter worktree rows');
assert.ok(pageSource.includes('worktree.taskID'), 'Worktree rows should expose parsed task IDs');
assert.ok(pageSource.includes('gitTaskUrl(worktree.taskID)'), 'Worktree rows should link parsed task IDs to Notion');
assert.ok(pageSource.includes('aria-label="Open worktree task"'), 'Worktree task links should be accessible');
assert.ok(pageSource.includes('projectWorktreeActivityLabel(worktree)'), 'Worktree rows should show last activity');
assert.ok(pageSource.includes('filteredGitRepositorySummaries'), 'Activity panels should filter repository rows');
assert.ok(pageSource.includes('filteredGitCommitHistory'), 'Activity panels should filter commit rows');
assert.ok(pageSource.includes('aria-label="Copy agent resume command"'), 'Agent rows should expose resume command copy');
assert.ok(pageSource.includes('aria-label="Resume agent in embedded terminal"'), 'Agent rows should expose one-click embedded terminal resume');
assert.ok(pageSource.includes('aria-label="Saved workspace snapshots"'), 'Conversations activity should render saved workspace snapshots');
assert.ok(pageSource.includes('aria-label="Save current workspace snapshot"'), 'Conversations activity should expose snapshot capture');
assert.ok(pageSource.includes('aria-label="Resume workspace snapshot in embedded terminal"'), 'Saved workspace rows should launch their resume command in the embedded terminal');
assert.ok(pageSource.includes('aria-label="Delete workspace snapshot"'), 'Saved workspace rows should expose snapshot removal');
assert.ok(pageSource.includes('aria-label="Open conversation workspace"'), 'Conversation row body should open that session workspace');
assert.ok(pageSource.includes('class="activity-session-row conversation-session-row"'), 'Conversation rows should use a compact clickable row layout');
assert.ok(pageSource.includes('aria-label="Save conversation workspace snapshot"'), 'Conversation rows should capture their session workspace');
assert.ok(pageSource.includes('aria-label="Restore conversation workspace"'), 'Conversation rows should restore their saved session workspace');
assert.ok(pageSource.includes('restoreConversationWorkspaceSnapshot(snapshot)'), 'Snapshot rows should restore saved workspace context');
assert.ok(pageSource.includes('await restoreWorkspaceEmbeddedTerminal(restored.embeddedTerminal)'), 'Snapshot restore should reattach saved embedded terminal context');
assert.ok(pageSource.includes('workspaceSnapshotForAgentSession(session)'), 'Conversation rows should read their saved workspace context');
assert.ok(pageSource.includes('aria-label="Open worktree path"'), 'Worktree rows should expose native open');
assert.ok(pageSource.includes('aria-label="Open worktree in terminal"'), 'Worktree rows should expose terminal open');
assert.ok(pageSource.includes('aria-label="Open worktree in embedded terminal"'), 'Worktree rows should expose embedded terminal open');
assert.ok(pageSource.includes('function runWorktreePrimaryAction'), 'Worktree rows should execute safe primary actions');
assert.ok(pageSource.includes('removeProjectWorktreeFromTauri'), 'Worktree cleanup should use the native guarded remove command');
assert.ok(pageSource.includes('perform: () => runWorktreePrimaryAction(worktree)'), 'Command palette recommended worktree actions should execute the guarded handler');
assert.ok(pageSource.includes('aria-label={`${primaryAction.label} worktree: ${worktree.branch}`}'), 'Worktree primary action labels should describe the real action');
assert.ok(pageSource.includes('aria-label="Open repository in terminal"'), 'Repository rows should expose terminal open');
assert.ok(pageSource.includes('aria-label="Open active session in terminal"'), 'Active session rows should expose terminal open');
assert.ok(pageSource.includes('aria-label="Reveal repository path"'), 'Repository rows should expose native reveal');
assert.ok(pageSource.includes('.file-tree::-webkit-scrollbar'), 'Tree view should style WebKit scrollbars');
assert.ok(pageSource.includes('function cancelSourceScan()'), 'Source scans should expose a cancel action');
assert.ok(
  pageSource.includes('onclick={scanning ? cancelSourceScan : () => scanProject(selectedProject, undefined, { force: true, limit: expandedSourceScanLimit })}'),
  'Scan button should become a cancel button and otherwise run an expanded source scan'
);
assert.ok(
  pageSource.includes("aria-label={scanning ? 'Stop source scan' : `Scan up to ${expandedSourceScanLimit.toLocaleString()} source files`}"),
  'Scan button should announce the expanded scan limit and stop state'
);
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
assert.ok(
  !pageSource.includes('demoRecordsForProject'),
  'Source preview should not show seeded demo files while waiting for a real project scan'
);
assert.ok(
  pageSource.includes('let records = $state<SourceRecord[]>([])'),
  'Source preview should start with an empty index until the real scanner responds'
);
assert.ok(
  pageSource.includes("let runtime = $state('pending source scan')"),
  'Source preview should label the startup state as a pending scan'
);
assert.ok(pageSource.includes('sourceScanNeedsAttention'), 'Source tree should detect suspiciously tiny scan indexes');
assert.ok(pageSource.includes('function formatSourceScanHealthNote'), 'Source tree should explain suspicious scan results');
assert.ok(pageSource.includes('sourceScanStatsLabel'), 'Source tree should derive a compact native scan telemetry label');
assert.ok(pageSource.includes('formatSourceScanStats'), 'Source tree should format native scan telemetry');
assert.ok(pageSource.includes('class="scan-stats"'), 'Source tree should render scan telemetry below the index summary');
assert.ok(pageSource.includes('tauriScan.stats'), 'Source scans should preserve native scanner telemetry');
assert.ok(pageSource.includes('skipTinyIndexRepair'), 'Source scans should avoid repair loops for genuinely tiny projects');
assert.ok(pageSource.includes('shouldRepairSuspiciousSourceScan'), 'Source scans should auto-repair suspiciously tiny indexes');
assert.ok(
  pageSource.includes('fileActionStatus = `Only ${nextRecords.length.toLocaleString()} files indexed for ${project.name}. Rebuilding the project index.`'),
  'Suspicious scan repair should explain that the project index is being rebuilt'
);
assert.ok(pageSource.includes('class="scan-summary"'), 'Source tree should render the scan summary below the heading');
assert.ok(pageSource.includes('class="scan-health-note"'), 'Source tree should render compact scan health guidance');
assertDeclaration('.scan-summary', 'overflow: hidden');
assertDeclaration('.scan-summary', 'text-overflow: ellipsis');
assertDeclaration('.scan-health-note', 'grid-template-columns: minmax(0, 1fr) auto');
assertDeclaration('.scan-health-note span', 'text-overflow: ellipsis');
assert.ok(pageSource.includes('expandedSourceScanLimit'), 'Source preview should expose an expanded scan limit');
assert.ok(
  pageSource.includes('expandedSourceScanLimitShortLabel'),
  'Expanded scan button should derive its compact label from the scan limit'
);
assert.ok(!pageSource.includes('Scan 5K'), 'Expanded scan button should not hard-code the old scan cap');
assert.ok(pageSource.includes('{#if scanLimitReached && !scanning}'), 'Truncated source scans should expose a rescan-more action');
assert.ok(pageSource.includes('class="scan-more-button"'), 'Expanded source scans should use a compact action button');
assert.ok(
  pageSource.includes('scanProject(selectedProject, selectedRecord?.path, { force: true, limit: expandedSourceScanLimit })'),
  'Expanded source scans should rescan the active project at the larger limit'
);
assert.ok(
  pageSource.includes('scanProject(storedProject, storedSelectedSourcePaths[storedProject.id], { limit: expandedSourceScanLimit })'),
  'Startup should request an expanded project scan'
);
assert.ok(
  pageSource.includes('getSourceScanCacheEntry(\n      sourceScanCache,\n      selectedProject,\n      expandedSourceScanLimit'),
  'Selected project index status should describe the expanded project scan cache'
);
assert.ok(
  pageSource.includes('await activateProject(nextProject, { projects: projectOptions, scanLimit: expandedSourceScanLimit })'),
  'Project switching should request an expanded project scan'
);
assert.ok(pageSource.includes('removeSourceScanCacheEntries(sourceScanCache, project)'), 'Index reset should discard cached scans for the selected project');
assert.ok(pageSource.includes('limit = expandedSourceScanLimit'), 'Index reset should rescan at the expanded source limit by default');
assert.ok(pageSource.includes('forceScan: true'), 'New or duplicate project activation should force a fresh scan');
assert.ok(pageSource.includes('scanLimit: expandedSourceScanLimit'), 'New or duplicate project activation should run expanded onboarding scans');
assert.ok(
  pageSource.includes('selectBackgroundIndexProjects(\n      projects,\n      selectedProject.id,\n      sourceScanCache,\n      Date.now(),\n      sourceScanCacheMaxAgeMs,\n      expandedSourceScanLimit'),
  'Background project indexing should use the same expanded project scan limit'
);
assert.ok(pageSource.includes('fileActionStatus = sourceOnboardingScanStatus(project)'), 'Duplicate project selection should announce the expanded onboarding scan');
assert.ok(pageSource.includes('fileActionStatus = sourceOnboardingScanStatus(nextProject)'), 'New project selection should announce the expanded onboarding scan');
assert.ok(pageSource.includes('type ProjectActivationOptions'), 'Project activation should accept scan and project-list options');
assertDeclaration('.scan-more-button', 'white-space: nowrap');
assert.ok(editorSource.includes('basic-languages/dart/dart.contribution'), 'Editor should load Dart highlighting');
assert.ok(editorSource.includes('basic-languages/hcl/hcl.contribution'), 'Editor should load HCL highlighting');
assert.ok(editorSource.includes('basic-languages/lua/lua.contribution'), 'Editor should load Lua highlighting');
assert.ok(editorSource.includes('basic-languages/php/php.contribution'), 'Editor should load PHP highlighting');
assert.ok(editorSource.includes('basic-languages/protobuf/protobuf.contribution'), 'Editor should load Protobuf highlighting');
assert.ok(editorSource.includes('automaticLayout: false'), 'Editor should avoid Monaco automatic ResizeObserver layout');
assert.ok(editorSource.includes('layoutObserver = new ResizeObserver'), 'Editor should own a deferred layout observer');
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
assert.ok(pageSource.includes('function saveAllDirtySourceFiles'), 'Source page should expose a save-all action');
assert.ok(pageSource.includes('function revertSelectedSourceFile'), 'Source page should expose a revert action');
assert.ok(pageSource.includes('const hasUnsavedDraft'), 'Source page should preserve only real unsaved drafts');
assert.ok(
  pageSource.includes('existingDraft !== existingSaved'),
  'Real source loads should replace stale demo drafts instead of marking files dirty'
);
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
  pageSource.includes('externalDiagnostics={sourceLspDiagnostics}'),
  'Monaco should receive native LSP diagnostics from the page'
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
  editorSource.includes('externalDiagnostics?: SourceDiagnostic[]'),
  'Editor should accept native LSP diagnostics as external markers'
);
assert.ok(
  editorSource.includes('registerSourceDefinitionProvider'),
  'Editor should register native definition results with Monaco'
);
assert.ok(
  editorSource.includes('registerDefinitionProvider'),
  'Editor should expose native definition lookup to Monaco peek'
);
assert.ok(
  editorSource.includes('registerSourceReferenceProvider'),
  'Editor should register native reference results with Monaco'
);
assert.ok(
  editorSource.includes('registerReferenceProvider'),
  'Editor should expose native references to Monaco peek'
);
assert.ok(
  editorSource.includes('registerSourceDocumentHighlightProvider'),
  'Editor should register native document highlights with Monaco'
);
assert.ok(
  editorSource.includes('registerDocumentHighlightProvider'),
  'Editor should expose native symbol occurrence highlights to Monaco'
);
assert.ok(
  editorSource.includes('registerSourceImplementationProvider'),
  'Editor should register native implementation results with Monaco'
);
assert.ok(
  editorSource.includes('registerImplementationProvider'),
  'Editor should expose native implementations to Monaco peek'
);
assert.ok(
  editorSource.includes('registerSourceTypeDefinitionProvider'),
  'Editor should register native type-definition results with Monaco'
);
assert.ok(
  editorSource.includes('registerTypeDefinitionProvider'),
  'Editor should expose native type definitions to Monaco peek'
);
assert.ok(
  editorSource.includes('registerSourceFormattingProvider'),
  'Editor should register native document formatting results with Monaco'
);
assert.ok(
  editorSource.includes('registerDocumentFormattingEditProvider'),
  'Editor should expose native formatting to Monaco'
);
assert.ok(
  editorSource.includes('registerSourceCodeActionProvider'),
  'Editor should register native code actions with Monaco'
);
assert.ok(
  editorSource.includes('registerCodeActionProvider'),
  'Editor should expose native quick fixes to Monaco'
);
assert.ok(
  editorSource.includes('registerSourceCompletionProvider'),
  'Editor should register native completion results with Monaco'
);
assert.ok(
  editorSource.includes('registerCompletionItemProvider'),
  'Editor should expose native completions to Monaco suggestions'
);
assert.ok(
  editorSource.includes('registerSourceSignatureHelpProvider'),
  'Editor should register native signature help with Monaco'
);
assert.ok(
  editorSource.includes('registerSignatureHelpProvider'),
  'Editor should expose native parameter hints to Monaco'
);
assert.ok(
  editorSource.includes('registerSourceDocumentSymbolProvider'),
  'Editor should register source symbols with Monaco'
);
assert.ok(
  editorSource.includes('registerDocumentSymbolProvider'),
  'Editor should expose source symbols to Monaco quick outline'
);
assert.ok(
  editorSource.includes('sourceCompletionItemToSuggestion'),
  'Editor should map native completion items into Monaco suggestions'
);
assert.ok(
  editorSource.includes('sourceInlayHintToMonacoHint'),
  'Editor should map native inlay hints into Monaco hints'
);
assert.ok(
  editorSource.includes('sourceSymbolToDocumentSymbol'),
  'Editor should map source symbols into Monaco document symbols'
);
assert.ok(
  editorSource.includes('sourceDefinitionTargetToLocation'),
  'Editor should map native definition targets into Monaco locations'
);
assert.ok(
  editorSource.includes('sourceReferenceTargetToLocation'),
  'Editor should map native reference targets into Monaco locations'
);
assert.ok(
  editorSource.includes('sourceImplementationTargetToLocation'),
  'Editor should map native implementation targets into Monaco locations'
);
assert.ok(
  editorSource.includes('sourceTypeDefinitionTargetToLocation'),
  'Editor should map native type-definition targets into Monaco locations'
);
assert.ok(
  editorSource.includes('sourceTextEditToMonacoEdit'),
  'Editor should map native text edits into Monaco formatting edits'
);
assert.ok(
  editorSource.includes('externalWorkspaceEditCommandId'),
  'Editor should register a command for external workspace edits'
);
assert.ok(
  editorSource.includes('onWorkspaceEditAction?.(action)'),
  'Editor should notify the page when a selected code action has external edits'
);
assert.ok(
  pageSource.includes('async function handleEditorDefinitionLookup'),
  'Source page should return definition lookup results to Monaco'
);
assert.ok(
  pageSource.includes('async function handleEditorReferenceLookup'),
  'Source page should return reference lookup results to Monaco'
);
assert.ok(
  pageSource.includes('async function handleEditorCompletionLookup'),
  'Source page should return completion lookup results to Monaco'
);
assert.ok(
  editorSource.includes('function applyExternalDiagnostics'),
  'Editor should apply native LSP diagnostics to Monaco markers'
);
assert.ok(
  editorSource.includes('monacoApi.editor.setModelMarkers(model, "mcb-lsp"'),
  'Editor should keep native LSP diagnostics in a dedicated Monaco marker owner'
);
assert.ok(
  editorSource.includes('onSymbolsChange?: (symbols: SourceSymbol[]) => void'),
  'Editor should report symbols to the parent'
);
assert.ok(editorSource.includes('intelligenceCommand?: SourceEditorIntelligenceCommand | null'), 'Editor should accept language-intelligence commands');
assert.ok(
  editorSource.includes('type SourceEditorLookupRequest'),
  'Editor should define a position-aware lookup request'
);
assert.ok(
  editorSource.includes('onDefinitionLookup?: SourceEditorDefinitionLookup'),
  'Editor should return native definition targets to Monaco'
);
assert.ok(
  editorSource.includes('onReferenceLookup?: SourceEditorReferenceLookup'),
  'Editor should return native reference targets to Monaco'
);
assert.ok(
  editorSource.includes('onImplementationLookup?: SourceEditorImplementationLookup'),
  'Editor should return native implementation targets to Monaco'
);
assert.ok(
  editorSource.includes('onTypeDefinitionLookup?: SourceEditorTypeDefinitionLookup'),
  'Editor should return native type-definition targets to Monaco'
);
assert.ok(
  editorSource.includes('onFormatDocument?: SourceEditorFormatDocument'),
  'Editor should return native formatting edits to Monaco'
);
assert.ok(
  editorSource.includes('onRename?: SourceEditorRename'),
  'Editor should return native rename edits to Monaco'
);
assert.ok(
  editorSource.includes('onCompletionLookup?: SourceEditorCompletionLookup'),
  'Editor should return native completion items to Monaco'
);
assert.ok(
  editorSource.includes('onInlayHintLookup?: SourceEditorInlayHintLookup'),
  'Editor should return native inlay hints to Monaco'
);
assert.ok(
  editorSource.includes('type SourceEditorSemanticTokensLookup'),
  'Editor should define a native semantic-token callback'
);
assert.ok(
  editorSource.includes('onSemanticTokensLookup?: SourceEditorSemanticTokensLookup'),
  'Editor should return native semantic tokens to Monaco'
);
assert.ok(
  editorSource.includes('await onSemanticTokensLookup?.(modelPreview)'),
  'Editor should prefer native LSP semantic tokens for the active model before local fallback'
);
assert.ok(editorSource.includes('getWordAtPosition'), 'Editor should read the symbol under the cursor');
assert.ok(editorSource.includes('editor.action.showHover'), 'Editor should expose a hover command');
assert.ok(editorSource.includes('editor.action.revealDefinition'), 'Editor should expose go-to-definition');
assert.ok(editorSource.includes('editor.action.referenceSearch.trigger'), 'Editor should expose Monaco reference search');
assert.ok(editorSource.includes('editor.action.goToImplementation'), 'Editor should expose go-to-implementation');
assert.ok(editorSource.includes('editor.action.goToTypeDefinition'), 'Editor should expose go-to-type-definition');
assert.ok(editorSource.includes('editor.action.formatDocument'), 'Editor should expose format-document');
assert.ok(editorSource.includes('editor.action.quickOutline'), 'Editor should expose Monaco quick outline');
assert.ok(editorSource.includes('onSaveRequest?: () => void'), 'Editor should accept a native save shortcut callback');
assert.ok(editorSource.includes('onQuickOpenRequest?: () => void'), 'Editor should accept a native quick-open shortcut callback');
assert.ok(editorSource.includes('onCommandPaletteRequest?: () => void'), 'Editor should accept a native command-palette shortcut callback');
assert.ok(editorSource.includes('onGoToLineRequest?: () => void'), 'Editor should accept a native go-to-line shortcut callback');
assert.ok(editorSource.includes('onNavigateBackRequest?: () => void'), 'Editor should accept a source back-navigation shortcut callback');
assert.ok(editorSource.includes('onNavigateForwardRequest?: () => void'), 'Editor should accept a source forward-navigation shortcut callback');
assert.ok(editorSource.includes('onProblemsRequest?: () => void'), 'Editor should accept a native problems shortcut callback');
assert.ok(editorSource.includes('onSymbolsRequest?: () => void'), 'Editor should accept a native symbols shortcut callback');
assert.ok(editorSource.includes('monaco.KeyCode.F12'), 'Editor should bind F12 for definition lookup');
assert.ok(editorSource.includes('monaco.KeyMod.Shift | monaco.KeyCode.F12'), 'Editor should bind Shift+F12 for references');
assert.ok(editorSource.includes('monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS'), 'Editor should bind Cmd+S inside Monaco');
assert.ok(editorSource.includes('monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyP'), 'Editor should bind Cmd+P inside Monaco');
assert.ok(editorSource.includes('monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyK'), 'Editor should bind Cmd+K inside Monaco');
assert.ok(editorSource.includes('monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyG'), 'Editor should bind Cmd+G inside Monaco');
assert.ok(editorSource.includes('monaco.KeyMod.CtrlCmd | monaco.KeyCode.BracketLeft'), 'Editor should bind Cmd+[ inside Monaco');
assert.ok(editorSource.includes('monaco.KeyMod.CtrlCmd | monaco.KeyCode.BracketRight'), 'Editor should bind Cmd+] inside Monaco');
assert.ok(editorSource.includes('monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.KeyO'), 'Editor should bind Cmd+Shift+O inside Monaco');
assert.ok(editorSource.includes('monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.KeyM'), 'Editor should bind Cmd+Shift+M inside Monaco');
assert.ok(editorSource.includes('editor.addAction'), 'Editor should register Monaco command-palette/context-menu actions');
assert.ok(editorSource.includes('registerHoverProvider'), 'Editor should register an in-editor hover provider');
assert.ok(editorSource.includes('editor.onMouseDown'), 'Editor should handle mouse navigation gestures');
assert.ok(editorSource.includes('MouseTargetType.CONTENT_TEXT'), 'Editor mouse navigation should only run for text tokens');
assert.ok(
  editorSource.includes('browserEvent.metaKey') && editorSource.includes('browserEvent.ctrlKey'),
  'Editor should support Cmd/Ctrl-click definition lookup'
);
assert.ok(editorSource.includes('function requestDefinitionAtPosition'), 'Editor should request definitions from clicked token positions');
assert.ok(pageSource.includes('sourceDiagnostics'), 'Source page should track diagnostics');
assert.ok(pageSource.includes('sourceSymbols'), 'Source page should track symbols');
assert.ok(pageSource.includes('sourceDefinitionTargets'), 'Source page should track project definition lookup targets');
assert.ok(pageSource.includes('sourceReferenceTargets'), 'Source page should track project reference lookup targets');
assert.ok(pageSource.includes('sourceImplementationTargets'), 'Source page should track implementation lookup targets');
assert.ok(pageSource.includes('sourceTypeDefinitionTargets'), 'Source page should track type-definition lookup targets');
assert.ok(pageSource.includes('formatSourceDiagnosticSummary'), 'Source page should summarize diagnostics');
assert.ok(pageSource.includes('sourceSupportsLanguageIntelligence'), 'Source page should gate Monaco language actions');
assert.ok(pageSource.includes('requestSourceIntelligenceAction'), 'Source page should dispatch language actions');
assert.ok(pageSource.includes('findSourceDefinitionsFromTauri'), 'Source page should call native project definition lookup');
assert.ok(pageSource.includes('findSourceLspDefinitionsFromTauri'), 'Source page should try LSP definition lookup before project index lookup');
assert.ok(pageSource.includes('findSourceLspReferencesFromTauri'), 'Source page should try LSP reference lookup before project index lookup');
assert.ok(pageSource.includes('findSourceLspImplementationsFromTauri'), 'Source page should use LSP implementation lookup');
assert.ok(pageSource.includes('findSourceLspTypeDefinitionsFromTauri'), 'Source page should use LSP type-definition lookup');
assert.ok(pageSource.includes('findSourceLspDocumentHighlightsFromTauri'), 'Source page should use LSP document highlights');
assert.ok(pageSource.includes('formatSourceWithLspFromTauri'), 'Source page should use LSP document formatting');
assert.ok(pageSource.includes('renameSourceWithLspFromTauri'), 'Source page should use LSP rename');
assert.ok(pageSource.includes('findSourceLspCodeActionsFromTauri'), 'Source page should use LSP code actions');
assert.ok(pageSource.includes('findSourceLspCompletionsFromTauri'), 'Source page should use LSP for completion lookup');
assert.ok(pageSource.includes('findSourceLspSignatureHelpFromTauri'), 'Source page should use LSP signature help');
assert.ok(pageSource.includes('findSourceLspInlayHintsFromTauri'), 'Source page should use LSP inlay hints');
assert.ok(pageSource.includes('findSourceLspSymbolsFromTauri'), 'Source page should prefer LSP document symbols when available');
assert.ok(pageSource.includes('findSourceLspWorkspaceSymbolsFromTauri'), 'Source page should search native LSP workspace symbols');
assert.ok(pageSource.includes('readSourceLspStatusFromTauri'), 'Source page should read LSP availability for the selected source file');
assert.ok(pageSource.includes('readSourceLspDiagnosticsFromTauri'), 'Source page should read native LSP diagnostics for the selected source file');
assert.ok(pageSource.includes('sourceLspStatus'), 'Source page should track LSP availability');
assert.ok(pageSource.includes('sourceLspDiagnostics'), 'Source page should track native LSP diagnostics separately');
assert.ok(pageSource.includes('function sourceLspInstallCommand'), 'Source page should expose LSP install command guidance');
assert.ok(pageSource.includes('function sourceLspStatusReport'), 'Source page should build copyable LSP status reports');
assert.ok(pageSource.includes('function copySourceLspInstallCommand'), 'Source page should copy LSP install commands');
assert.ok(pageSource.includes('dotnet tool install --global csharp-ls'), 'C# LSP guidance should install csharp-ls');
assert.ok(pageSource.includes('npm install -g typescript typescript-language-server'), 'TypeScript LSP guidance should install the TS language server');
assert.ok(pageSource.includes('function loadSourceLspDiagnostics'), 'Source page should expose native LSP diagnostics loading');
assert.ok(pageSource.includes('function loadSourceLspSymbols'), 'Source page should expose native LSP symbol loading');
assert.ok(pageSource.includes('function loadSourceLspWorkspaceSymbols'), 'Source page should expose native LSP workspace symbol loading');
assert.ok(pageSource.includes('findSourceDefinitionTargets'), 'Source page should fall back to browser definition lookup');
assert.ok(pageSource.includes('findSourceReferencesFromTauri'), 'Source page should call native project reference lookup');
assert.ok(pageSource.includes('findSourceReferenceTargets'), 'Source page should fall back to browser reference lookup');
assert.ok(pageSource.includes('sourceNavigationBackStack'), 'Source page should track source navigation back stack');
assert.ok(pageSource.includes('sourceNavigationForwardStack'), 'Source page should track source navigation forward stack');
assert.ok(pageSource.includes('pushSourceNavigationHistory'), 'Source page should push current source locations before jumps');
assert.ok(pageSource.includes('function navigateSourceBack'), 'Source page should expose back navigation');
assert.ok(pageSource.includes('function navigateSourceForward'), 'Source page should expose forward navigation');
assert.ok(pageSource.includes("id: 'navigate-back'"), 'Command palette should expose source back navigation');
assert.ok(pageSource.includes("id: 'navigate-forward'"), 'Command palette should expose source forward navigation');
assert.ok(pageSource.includes("event.key === '['"), 'Window shortcuts should bind Cmd+[ for source back navigation');
assert.ok(pageSource.includes("event.key === ']'"), 'Window shortcuts should bind Cmd+] for source forward navigation');
assert.ok(pageSource.includes('function runSourceDefinitionLookup'), 'Source page should expose project definition lookup');
assert.ok(pageSource.includes('function runSourceReferenceLookup'), 'Source page should expose project reference lookup');
assert.ok(pageSource.includes('function runSourceImplementationLookup'), 'Source page should expose implementation lookup');
assert.ok(pageSource.includes('function runSourceTypeDefinitionLookup'), 'Source page should expose type-definition lookup');
assert.ok(pageSource.includes('function handleEditorDefinitionLookup'), 'Source page should receive editor definition lookup requests');
assert.ok(pageSource.includes('function handleEditorReferenceLookup'), 'Source page should receive editor reference lookup requests');
assert.ok(pageSource.includes('function handleEditorDocumentHighlightLookup'), 'Source page should receive editor document highlight requests');
assert.ok(pageSource.includes('function handleEditorImplementationLookup'), 'Source page should receive editor implementation lookup requests');
assert.ok(pageSource.includes('function handleEditorTypeDefinitionLookup'), 'Source page should receive editor type-definition lookup requests');
assert.ok(pageSource.includes('function handleEditorCompletionLookup'), 'Source page should receive editor completion lookup requests');
assert.ok(pageSource.includes('function handleEditorCodeActionLookup'), 'Source page should receive editor code action requests');
assert.ok(pageSource.includes('function handleEditorSignatureHelpLookup'), 'Source page should receive editor signature help requests');
assert.ok(pageSource.includes('function handleEditorInlayHintLookup'), 'Source page should receive editor inlay hint requests');
assert.ok(pageSource.includes('function handleEditorSemanticTokensLookup'), 'Source page should receive editor semantic token requests');
assert.ok(pageSource.includes('function handleEditorFormatDocument'), 'Source page should receive editor formatting requests');
assert.ok(pageSource.includes('function handleEditorRename'), 'Source page should receive editor rename requests');
assert.ok(pageSource.includes('function stageExternalWorkspaceEditDrafts'), 'Source page should stage external workspace edits as drafts');
assert.ok(pageSource.includes('applySourceTextEdits'), 'Source page should apply external LSP text edits to drafts');
assert.ok(pageSource.includes('workspaceEditSourceRecordsByPath'), 'Source page should track workspace-edited dirty files');
assert.ok(pageSource.includes('function handleEditorWorkspaceEditAction'), 'Source page should stage selected quick-fix workspace edits');
assert.ok(pageSource.includes('onDefinitionLookup={handleEditorDefinitionLookup}'), 'Editor should be wired to project definition lookup');
assert.ok(pageSource.includes('onReferenceLookup={handleEditorReferenceLookup}'), 'Editor should be wired to project reference lookup');
assert.ok(pageSource.includes('onDocumentHighlightLookup={handleEditorDocumentHighlightLookup}'), 'Editor should be wired to LSP document highlights');
assert.ok(pageSource.includes('onImplementationLookup={handleEditorImplementationLookup}'), 'Editor should be wired to LSP implementation lookup');
assert.ok(pageSource.includes('onTypeDefinitionLookup={handleEditorTypeDefinitionLookup}'), 'Editor should be wired to LSP type-definition lookup');
assert.ok(pageSource.includes('onWorkspaceEditAction={handleEditorWorkspaceEditAction}'), 'Editor should be wired to selected workspace edits');
assert.ok(pageSource.includes('onCompletionLookup={handleEditorCompletionLookup}'), 'Editor should be wired to LSP completion lookup');
assert.ok(pageSource.includes('onCodeActionLookup={handleEditorCodeActionLookup}'), 'Editor should be wired to LSP code actions');
assert.ok(pageSource.includes('onSignatureHelpLookup={handleEditorSignatureHelpLookup}'), 'Editor should be wired to LSP signature help');
assert.ok(pageSource.includes('onInlayHintLookup={handleEditorInlayHintLookup}'), 'Editor should be wired to LSP inlay hints');
assert.ok(pageSource.includes('onSemanticTokensLookup={handleEditorSemanticTokensLookup}'), 'Editor should be wired to LSP semantic tokens');
assert.ok(pageSource.includes('onFormatDocument={handleEditorFormatDocument}'), 'Editor should be wired to LSP document formatting');
assert.ok(pageSource.includes('onRename={handleEditorRename}'), 'Editor should be wired to LSP rename');
assert.ok(pageSource.includes('onSaveRequest={saveSelectedSourceFile}'), 'Editor Cmd+S should call the page save path');
assert.ok(pageSource.includes("id: 'save-all-files'"), 'Command palette should expose Save All');
assert.ok(pageSource.includes('aria-label="Save all source files"'), 'Editor action menu should expose Save All');
assert.ok(pageSource.includes('onQuickOpenRequest={openQuickOpen}'), 'Editor Cmd+P should call the page quick-open path');
assert.ok(pageSource.includes('onCommandPaletteRequest={openCommandPalette}'), 'Editor Cmd+K should call the page command palette');
assert.ok(pageSource.includes('onGoToLineRequest={openCurrentFileGoToLine}'), 'Editor Cmd+G should call current-file line navigation');
assert.ok(pageSource.includes('onNavigateBackRequest={navigateSourceBack}'), 'Editor Cmd+[ should call page source back navigation');
assert.ok(pageSource.includes('onNavigateForwardRequest={navigateSourceForward}'), 'Editor Cmd+] should call page source forward navigation');
assert.ok(pageSource.includes("onProblemsRequest={() => showEditorInsightPanel('problems')}"), 'Editor Cmd+Shift+M should show problems');
assert.ok(pageSource.includes('title={sourceLspStatusTitle()}'), 'LSP badge should explain server or fallback status');
assert.ok(pageSource.includes('class:unavailable={!sourceLspStatusLoading && !sourceLspStatus?.available}'), 'LSP badge should style fallback status distinctly');
assertDeclaration('.editor-file-state .editor-lsp-state.unavailable', 'color: #d8aa55');
assert.ok(pageSource.includes("onSymbolsRequest={() => showEditorInsightPanel('symbols')}"), 'Editor Cmd+Shift+O should show symbols');
assert.ok(pageSource.includes('source-intelligence-panel'), 'Source page should render language intelligence panel');
assert.ok(pageSource.includes('aria-label="Show hover"'), 'Editor controls should expose hover');
assert.ok(pageSource.includes('aria-label="Go to definition"'), 'Editor controls should expose definition');
assert.ok(pageSource.includes('aria-label="Find references"'), 'Editor controls should expose references');
assert.ok(pageSource.includes('aria-label="Find implementations"'), 'Editor controls should expose implementations');
assert.ok(pageSource.includes('aria-label="Go to type definition"'), 'Editor controls should expose type-definition lookup');
assert.ok(pageSource.includes('aria-label="Format source file"'), 'Editor controls should expose formatting');
assert.ok(pageSource.includes('Problems'), 'Language panel should include Problems');
assert.ok(pageSource.includes('Symbols'), 'Language panel should include Symbols');
assert.ok(pageSource.includes('selectSourceSymbol'), 'Symbol rows should reveal source lines');
assert.ok(pageSource.includes('class="definition-results"'), 'Language panel should show project definition lookup results');
assert.ok(pageSource.includes('class="reference-results"'), 'Language panel should show project reference lookup results');
assert.ok(pageSource.includes('class="implementation-results"'), 'Language panel should show implementation lookup results');
assert.ok(pageSource.includes('class="type-definition-results"'), 'Language panel should show type-definition lookup results');
assertDeclaration('.definition-results', 'overflow-y: auto');
assertDeclaration('.definition-results', 'scrollbar-width: thin');
assertDeclaration('.reference-results', 'overflow-y: auto');
assertDeclaration('.reference-results', 'scrollbar-width: thin');
assertDeclaration('.implementation-results', 'overflow-y: auto');
assertDeclaration('.implementation-results', 'scrollbar-width: thin');
assertDeclaration('.type-definition-results', 'overflow-y: auto');
assertDeclaration('.type-definition-results', 'scrollbar-width: thin');
assert.ok(
  tauriSource.includes('findSourceLspImplementationsFromTauri'),
  'Tauri source bridge should expose native LSP implementation lookup'
);
assert.ok(
  tauriSource.includes("'find_source_lsp_implementations'"),
  'Tauri source bridge should invoke native implementation lookup'
);
assert.ok(
  tauriSource.includes('findSourceLspTypeDefinitionsFromTauri'),
  'Tauri source bridge should expose native LSP type-definition lookup'
);
assert.ok(
  tauriSource.includes("'find_source_lsp_type_definitions'"),
  'Tauri source bridge should invoke native type-definition lookup'
);
assert.ok(
  tauriSource.includes('findSourceLspDocumentHighlightsFromTauri'),
  'Tauri source bridge should expose native LSP document highlights'
);
assert.ok(
  tauriSource.includes("'find_source_lsp_document_highlights'"),
  'Tauri source bridge should invoke native document highlights'
);
assert.ok(
  tauriSource.includes('formatSourceWithLspFromTauri'),
  'Tauri source bridge should expose native LSP formatting'
);
assert.ok(
  tauriSource.includes("'format_source_with_lsp'"),
  'Tauri source bridge should invoke native formatting'
);
assert.ok(
  tauriSource.includes('renameSourceWithLspFromTauri'),
  'Tauri source bridge should expose native LSP rename'
);
assert.ok(
  tauriSource.includes("'rename_source_with_lsp'"),
  'Tauri source bridge should invoke native rename'
);
assert.ok(
  tauriSource.includes('findSourceLspCodeActionsFromTauri'),
  'Tauri source bridge should expose native LSP code actions'
);
assert.ok(
  tauriSource.includes("'find_source_lsp_code_actions'"),
  'Tauri source bridge should invoke native code actions'
);
assert.ok(
  tauriSource.includes('findSourceLspSignatureHelpFromTauri'),
  'Tauri source bridge should expose native LSP signature help'
);
assert.ok(
  tauriSource.includes("'find_source_lsp_signature_help'"),
  'Tauri source bridge should invoke native signature help'
);
assert.ok(
  tauriSource.includes('findSourceLspSemanticTokensFromTauri'),
  'Tauri source bridge should expose native LSP semantic tokens'
);
assert.ok(
  tauriSource.includes("'find_source_lsp_semantic_tokens'"),
  'Tauri source bridge should invoke native semantic tokens'
);
assert.ok(
  tauriSource.includes('findSourceLspWorkspaceSymbolsFromTauri'),
  'Tauri source bridge should expose native LSP workspace symbols'
);
assert.ok(
  tauriSource.includes("'find_source_lsp_workspace_symbols'"),
  'Tauri source bridge should invoke native workspace symbols'
);
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
assert.ok(pageSource.includes('taskReferenceUrl(taskID, commandCenterTaskUrls)'), 'Task links should fall back to Notion search');
assert.ok(pageSource.includes('function gitCommitSummaryText'), 'Git history should create a copyable commit summary');
assert.ok(pageSource.includes('function copyGitCommitSha'), 'Git history should expose quick SHA copy');
assert.ok(pageSource.includes('function copyGitCommitSummary'), 'Git history should expose quick summary copy');
assert.ok(pageSource.includes('function copyGitTaskReference'), 'Git task links should expose quick task reference copy');
assert.ok(pageSource.includes('function gitWorkspaceBriefText'), 'Git view should build a copyable workspace brief');
assert.ok(pageSource.includes('function copyGitWorkspaceBrief'), 'Git view should expose a copyable workspace brief action');
assert.ok(pageSource.includes("id: 'git-copy-workspace-brief'"), 'Command palette should copy the Git workspace brief');
assert.ok(pageSource.includes('selectedProjectGitTaskIDs'), 'Git panel should derive a selected-project task trail');
assert.ok(
  pageSource.includes('uniqueTaskIDsFromGitMetadata'),
  'Git panel should dedupe task IDs from repo/worktree/history metadata'
);
assert.ok(
  pageSource.includes('gitCommitRefChips(entry)'),
  'Git history should split decorated refs into compact chips'
);
assert.ok(
  pageSource.includes('gitCommitGraphClass(entry, index)'),
  'Git history should classify commits for graph styling'
);
assert.ok(pageSource.includes('function gitCommitTopology'), 'Git history should label commit topology for graph markers');
assert.ok(
  pageSource.includes('gitCommitTaskSourceLabel(entry)'),
  'Git task links should explain whether task IDs came from refs or subjects'
);
assert.ok(pageSource.includes('class="git-task-trail"'), 'Git panel should render task trail links');
assert.ok(pageSource.includes('gitCommitMessage'), 'Source page should track a Git commit message draft');
assert.ok(pageSource.includes('let selectedGitCommitSha'), 'Git panel should track a selected commit');
assert.ok(pageSource.includes('selectedGitCommit'), 'Git panel should derive the selected commit from history');
assert.ok(pageSource.includes('function selectGitCommit'), 'Git history should allow selecting a commit row');
assert.ok(pageSource.includes('function gitCommitDetailText'), 'Git history should build a detailed selected-commit handoff');
assert.ok(pageSource.includes('function copySelectedGitCommitDetail'), 'Git history should expose selected commit detail copying');
assert.ok(pageSource.includes('function runGitPathAction'), 'Source page should expose reusable stage/unstage handling');
assert.ok(pageSource.includes('type GitStatusGroupID'), 'Git panel should define staged/unstaged/untracked groups');
assert.ok(pageSource.includes('function buildGitStatusFileGroups'), 'Git panel should group changed files by index state');
assert.ok(pageSource.includes('function runGitStatusGroupAction'), 'Git panel should expose group-level stage and unstage actions');
assert.ok(pageSource.includes('selectedProjectGitFileGroups'), 'Git panel should derive grouped changed files');
assert.ok(pageSource.includes('selectedProjectGitFileGroupSummary'), 'Git panel should summarize grouped changed files');
assert.ok(pageSource.includes('function runGitRemoteAction'), 'Source page should expose reusable fetch/pull/push handling');
assert.ok(pageSource.includes('function commitGitChanges'), 'Source page should expose a commit action');
assert.ok(pageSource.includes('selectedSourceGitDiff'), 'Source page should track selected source Git diff');
assert.ok(pageSource.includes('function loadSelectedSourceGitDiff'), 'Source page should expose a selected-file Git diff loader');
assert.ok(pageSource.includes('formatSourceContextIdentity'), 'Source page should format a visible context identity');
assert.ok(pageSource.includes('formatSourceContextGitSummary'), 'Source page should use the shared Git context summary');
assert.ok(pageSource.includes('sourceContextIdentity'), 'Source page should derive the current source context identity');
assert.ok(pageSource.includes('aria-label="Current source context"'), 'Workspace should expose the current context strip');
assert.ok(pageSource.includes('class="context-identity-strip"'), 'Workspace should render a context identity strip');
assert.ok(pageSource.includes('class="context-identity-item"'), 'Context strip should render compact labeled status items');
assert.ok(pageSource.includes('class="context-identity-key">Prj</span>'), 'Context strip should abbreviate the project label');
assert.ok(pageSource.includes('class="context-identity-key">Root</span>'), 'Context strip should label the root/worktree');
assert.ok(pageSource.includes('class="context-identity-key">Git</span>'), 'Context strip should abbreviate the branch/Git label');
assert.ok(pageSource.includes('class="context-identity-key">Run</span>'), 'Context strip should abbreviate the runtime label');
assert.ok(pageSource.includes('{sourceContextIdentity.gitSummary}'), 'Context strip should render the project Git branch/status summary');
assert.ok(pageSource.includes('class="git-status-badge"'), 'Tree and tabs should render file-level Git status badges');
assert.ok(pageSource.includes('<span>Git</span>'), 'Language panel should include a Git tab');
assert.ok(pageSource.includes('class="git-diff-panel"'), 'Source page should render a selected-file Git diff panel');
assert.ok(pageSource.includes('class="git-status-list"'), 'Git tab should render changed file rows');
assert.ok(pageSource.includes('class="git-status-overview"'), 'Git tab should render staged/unstaged summary');
assert.ok(pageSource.includes('class="git-status-group"'), 'Git tab should render grouped changed files');
assert.ok(pageSource.includes('class="git-status-group-heading"'), 'Git tab should render group-level actions');
assert.ok(pageSource.includes('aria-label="Stage selected source file"'), 'Git tab should expose staging for the selected file');
assert.ok(pageSource.includes('aria-label="Unstage selected source file"'), 'Git tab should expose unstaging for the selected file');
assert.ok(pageSource.includes('aria-label="Fetch selected repository"'), 'Git tab should expose repository fetch');
assert.ok(pageSource.includes('aria-label="Pull selected repository"'), 'Git tab should expose repository pull');
assert.ok(pageSource.includes('aria-label="Push selected repository"'), 'Git tab should expose repository push');
assert.ok(pageSource.includes('aria-label="Commit staged Git changes"'), 'Git tab should expose a guarded commit action');
assert.ok(pageSource.includes('aria-label="Git commit history"'), 'Git tab should expose commit history');
assert.ok(pageSource.includes("id: 'git-refresh-history'"), 'Command palette should refresh Git history directly');
assert.ok(pageSource.includes('id: `git-copy-task-${taskID}`'), 'Command palette should copy task links from Git metadata');
assert.ok(pageSource.includes('id: `git-copy-commit-${entry.sha}`'), 'Command palette should copy recent commit summaries');
assert.ok(pageSource.includes('class="git-history-list"'), 'Git tab should render commit history rows');
assert.ok(pageSource.includes('class="git-commit-detail"'), 'Git tab should render selected commit details');
assert.ok(pageSource.includes("id: 'git-copy-selected-commit-detail'"), 'Command palette should copy selected commit details');
assert.ok(
  pageSource.includes('git-history-row ${gitCommitGraphClass(entry, index)}'),
  'Git tab should render individual commit history rows with graph styling'
);
assert.ok(
  pageSource.includes('class:selected={selectedGitCommitSha === entry.sha}'),
  'Git history rows should show the selected commit'
);
assert.ok(pageSource.includes('class="git-task-link"'), 'Git history should render task links when task metadata is present');
assert.ok(pageSource.includes('class="activity-commit-meta"'), 'Activity Git commit rows should group task links with quick actions');
assert.ok(pageSource.includes('class="git-history-actions"'), 'Git history rows should render compact quick actions');
assert.ok(pageSource.includes('aria-label="Copy commit SHA"'), 'Git rows should expose copy SHA actions');
assert.ok(pageSource.includes('aria-label="Copy commit summary"'), 'Git rows should expose copy summary actions');
assert.ok(pageSource.includes('aria-label="Copy task reference"'), 'Git rows should expose copy task reference actions');
assert.ok(pageSource.includes('class="git-diff-block"'), 'Source page should render diff text in a monospace block');
assertDeclaration('.git-status-badge', 'font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace');
assertDeclaration('.git-diff-panel', 'overflow: hidden');
assertDeclaration('.git-status-list', 'overflow-y: auto');
assertDeclaration('.git-status-overview', 'text-overflow: ellipsis');
assertDeclaration('.git-status-group', 'display: grid');
assertDeclaration('.git-status-group-heading', 'grid-template-columns: minmax(0, 1fr) auto auto');
assertDeclaration('.git-history-list', 'overflow-y: auto');
assertDeclaration('.git-task-trail', 'display: flex');
assert.match(
  pageSource,
  /\.activity-commit-row\s*\{\s*grid-template-columns: auto minmax\(0, 1fr\) auto;/,
  'Activity commit rows should reserve a compact action column'
);
assertDeclaration('.activity-commit-meta', 'display: inline-flex');
assertDeclaration('.git-history-row.head', 'background: rgba(111, 223, 207, 0.07)');
assertDeclaration('.git-history-row.merge', 'border-color: rgba(216, 170, 85, 0.24)');
assertDeclaration('.git-history-actions', 'display: inline-flex');
assertDeclaration('.git-graph-marker.head::after', 'background: #6fdfcf');
assertDeclaration('.git-graph-marker.merge::after', 'border-radius: 3px');
assertDeclaration('.run-current-activity', 'grid-template-columns: auto minmax(0, 1fr)');
assert.ok(pageSource.includes('orchestrationAttentionQueue'), 'Runs mode should derive decision and blocker queues');
assert.ok(pageSource.includes('class="run-attention-queue"'), 'Runs mode should render decision and blocker queues');
assert.ok(pageSource.includes('aria-label="Run decisions and blockers"'), 'Runs mode should expose the attention queue');
assertDeclaration('.run-attention-queue', 'display: grid');
assertDeclaration('.run-attention-item', 'grid-template-columns: auto minmax(0, 1fr) auto');
assertDeclaration('.run-timeline-item', 'grid-template-columns: 18px minmax(0, 1fr) auto');
assertDeclaration('.run-artifact-row', 'display: flex');
assert.ok(
  pageSource.includes('.run-status-badge.attention,') &&
    pageSource.includes('.run-step-marker.attention') &&
    pageSource.includes('background: #d8aa55'),
  'Run attention status should style badges and markers'
);
assertDeclaration('.orchestration-context-row.attention', 'background: rgba(216, 170, 85, 0.09)');
assertDeclaration('.git-history-list', 'scrollbar-width: thin');
assertDeclaration('.git-history-row', 'min-width: 0');
assertDeclaration('.git-task-link', 'white-space: nowrap');
assertDeclaration('.git-commit-input', 'resize: none');
assertDeclaration('.git-remote-row', 'grid-template-columns: repeat(3, minmax(0, 1fr))');
assertDeclaration('.git-diff-block', 'overflow: auto');
assertDeclaration('.git-diff-block', 'font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace');
assertDeclaration('.context-identity-strip', 'overflow: hidden');
assertDeclaration('.context-identity-item', 'min-width: 0');
assertDeclaration('.context-identity-value', 'overflow: hidden');
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
assert.ok(pageSource.includes('function runtimeContextUrl'), 'Runtime contexts should expose local browser URLs');
assert.ok(pageSource.includes('aria-label="Refresh runtime contexts"'), 'Runtime context panel should expose a refresh action');
assert.ok(pageSource.includes('aria-label="Open runtime URL"'), 'Runtime context panel should expose URL launch actions');
assert.ok(pageSource.includes('aria-label="Open active session URL"'), 'Active session rows should expose URL launch actions');
assert.ok(pageSource.includes('href={runtimeContextUrl(context)}'), 'Runtime context URL links should use derived local URLs');
assert.ok(pageSource.includes('class="runtime-context-panel"'), 'Source page should render a runtime context panel');
assert.ok(pageSource.includes('class="runtime-context-list"'), 'Runtime context panel should render a scrollable list');
assert.ok(pageSource.includes('class="runtime-url-link"'), 'Runtime context rows should render URL badges');
assert.ok(pageSource.includes('class="runtime-port"'), 'Runtime context rows should show the listening port');
assert.ok(pageSource.includes('Runtime Contexts'), 'Runtime context panel should have a clear heading');
assertDeclaration('.runtime-context-list', 'overflow-y: auto');
assertDeclaration('.runtime-context-list', 'scrollbar-width: thin');
assertDeclaration('.runtime-url-link', 'white-space: nowrap');
assert.ok(pageSource.includes('listProjectWorktreesFromTauri'), 'Source page should load native project worktrees');
assert.ok(pageSource.includes('projectWorktrees'), 'Source page should track project worktrees');
assert.ok(pageSource.includes('function loadProjectWorktrees'), 'Source page should expose a worktree refresh action');
assert.ok(pageSource.includes('aria-label="Refresh worktrees"'), 'Worktree panel should expose a refresh action');
assert.ok(pageSource.includes('class="worktree-context-panel"'), 'Source page should render a worktree safety panel');
assert.ok(pageSource.includes('{projectWorktreeCleanupBrief.headline}'), 'Worktree panel should show the aggregate cleanup headline');
assert.ok(pageSource.includes('function copyProjectWorktreeCleanupBrief'), 'Worktree panel should copy the aggregate cleanup brief');
assert.ok(pageSource.includes('aria-label="Copy worktree cleanup script"'), 'Worktree panel should expose the guarded cleanup script');
assert.ok(pageSource.includes('class="worktree-context-list"'), 'Worktree panel should render a scrollable list');
assert.ok(pageSource.includes('worktree-status-badge ${safety.kind}'), 'Worktree rows should show safety status');
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
assert.ok(pageSource.includes('function agentSessionRowKey'), 'Agent session lists should use duplicate-safe row keys');
assert.ok(pageSource.includes("agentSessionRowKey(session, index, 'context')"), 'Context agent list should not key only by provider and id');
assert.ok(pageSource.includes('aria-label="Refresh agent sessions"'), 'Agent session panel should expose a refresh action');
assert.ok(pageSource.includes('class="agent-session-panel"'), 'Source page should render an agent session panel');
assert.ok(pageSource.includes('class="agent-session-list"'), 'Agent session panel should render a scrollable list');
assert.ok(pageSource.includes('class="agent-provider-badge"'), 'Agent session rows should show the provider');
assert.ok(pageSource.includes('Agent Sessions'), 'Agent session panel should have a clear heading');
assertDeclaration('.agent-session-list', 'overflow-y: auto');
assertDeclaration('.agent-session-list', 'scrollbar-width: thin');
