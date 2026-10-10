import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (relativePath: string): string => readFileSync(new URL(relativePath, import.meta.url), 'utf8');
const page = read('../src/routes/+page.svelte');
const shellStartup = read('../src/lib/shell/controllers/shellStartup.ts');
const selectionController = read('../src/lib/shell/controllers/sessionSelectionController.svelte.ts');
const selectionLayers = read('../src/lib/shell/sessionSelectionLayers.svelte.ts');
const railStore = read('../src/lib/shell/stores/sessionRailStore.svelte.ts');
const conversationService = read('../src/lib/shell/conversation/conversationService.ts');
const source = read('../src/lib/tauriSource.ts');
const row = read('../src/lib/shell/components/WorktreeAgentRow.svelte');
const rail = read('../src/lib/shell/components/SessionRail.svelte');
const filesPanel = read('../src/lib/shell/panels/files/FilesPanel.svelte');
const fileTreeWatch = read('../src/lib/shell/panels/files/fileTreeWatch.ts');
const composer = read('../src/lib/shell/components/conversation/ConversationComposer.svelte');
const conversationSurface = read('../src/lib/shell/components/ConversationSurface.svelte');
const elapsed = read('../src/lib/shell/components/railElapsedTicker.ts');
const editorSessions = read('../src/lib/shell/controllers/editorSessionController.svelte.ts');
const editorPanel = read('../src/lib/shell/components/EditorPanel.svelte');
const languageControls = read('../src/lib/shell/components/LanguageIntelligenceControls.svelte');
const settingsDialog = read('../src/lib/shell/components/SettingsDialog.svelte');
const settingsStore = read('../src/lib/settingsStore.svelte.ts');
const codeMirrorTheme = read('../src/lib/shell/editor/codeMirrorTheme.ts');
const codeMirrorSourceEditor = read('../src/lib/CodeMirrorSourceEditor.svelte');
const xtermFactory = read('../src/lib/shell/xtermFactory.ts');
const rightPanel = read('../src/lib/shell/components/RightPanel.svelte');
const browserPanel = read('../src/lib/shell/panels/browser/BrowserPanel.svelte');
const browserToolbar = read('../src/lib/shell/panels/browser/BrowserToolbar.svelte');
const frame = read('../src/lib/shell/layout/frame.ts');
const workbenchController = read('../src/lib/shell/controllers/workbenchController.svelte.ts');
const utilityStrip = read('../src/lib/shell/components/UtilityStrip.svelte');
const dockPanel = read('../src/lib/shell/components/DockPanel.svelte');
const workspaceTerminal = read('../src/lib/shell/components/WorkspaceTerminal.svelte');
const palettePanel = read('../src/lib/shell/components/PalettePanel.svelte');
const sourceControlPanel = read('../src/lib/shell/panels/sourceControl/SourceControlPanel.svelte');

assert.match(shellStartup, /listAgentConversationSessionsFromTauri\(\)/);
// a0b9ca6d0 folded the remote list into the start-up Promise.all; it keeps the shell stop signal and generation guard.
assert.match(shellStartup, /listRemoteAgentConversationSessionsFromTauri\(controller\.signal\)/);
assert.match(
  shellStartup,
  /listRemoteAgentConversationSessionsFromTauri\(controller\.signal\)[\s\S]*?\]\);\s*if \(!shellActive\(generation, controller\.signal\)\) return;[\s\S]*?hydrateOwned\(combined\)/
);
assert.match(selectionController, /private async drainSelections\(\): Promise<void>/);
assert.match(selectionController, /this\.selectionAbort\?\.abort\(\);[\s\S]*?await this\.selectionWork/);
// 03ddf7023 removed the same-root tree reuse on purpose: each session now restores its own file tree.
assert.doesNotMatch(
  page.slice(page.indexOf('async function selectOwned'), page.indexOf('function handoffInput')),
  /ensureStructuredConversation/
);
// WIP: disabled. 65c2353b deleted the legacy localStorage adoption key; the rail is
// whatever SQLite holds, and nothing else.
// assert.equal((page.match(/mac-command-bar\.next\.owned-sessions/g) ?? []).length, 1);
assert.doesNotMatch(railStore, /localStorage|owned-sessions|persist/i);
assert.doesNotMatch(conversationService, /listAgentConversationEventsFromTauri/);
assert.match(conversationService, /const requestedModel = startConfig\?\.model \?\? null/);
assert.match(source, /invoke<AgentConversationSessionRecord\[]>\('list_agent_conversation_sessions'/);
assert.match(source, /invoke<AgentConversationEvent\[]>\('list_agent_conversation_events'/);
assert.match(source, /invoke<AgentConversationSessionRecord>\('update_agent_conversation_session_meta'/);
assert.match(row, /session\.runtimeState === ["']suspended["']/);
assert.match(rail, /action === "open-in-editor"\) void jumpTo\(session, "editor"\)/);
assert.match(rail, /action === "open-source-control"\) void jumpTo\(session, "source-control"\)/);
assert.match(page, /registerSessionRowJumpTarget\(\{/);
assert.match(page, /showCenterPanel: \(_ownedId, id\) => selectCenterTab\(id\)/);
assert.match(page, /showSidebarView: \(_ownedId, id\) => selectRightTab\(id\)/);
assert.doesNotMatch(
  filesPanel,
  /const sessionKey = `\$\{ownedId/,
  'the file-tree owner is the checkout root, not each session id'
);
// 03ddf7023 scoped the tree to the session as well as the root; 50b05288e stopped a missing root from clearing it.
assert.match(filesPanel, /const rootChanged = nextRoot !== scopedRoot \|\| nextOwnedId !== scopedOwnedId;/);
assert.match(
  filesPanel,
  /else if \(rootChanged\) \{[\s\S]*?activateExplorer\(null\);/,
  'hiding Files parks its bounded tree; only a changed session or root clears it'
);
// bc7a3dcda routes watcher changes through the workspace file-change bus.
assert.match(filesPanel, /watchFileTree\(directories, signal, \(paths\) =>/);
assert.match(filesPanel, /listRepositoryCheckoutsFromTauri\(roots\)/);
assert.match(filesPanel, /data-testid="files-use-session-checkout"/);
assert.match(filesPanel, /if \(inspectionRoot === undefined\) return;/);
assert.match(sourceControlPanel, /if \(inspectionRoot === undefined\) return;/);
assert.match(page, /onUseSessionCheckout=\{[\s\S]*?selection\.useSessionCheckout\(root\)/);
// 725896a73 passes the remote checkout's own path when the root is a remote workspace path.
assert.match(
  selectionController,
  /async useSessionCheckout\(requestedRoot: string\): Promise<boolean>[\s\S]*?validateProjectRootFromTauri\(root, owner\.signal\)[\s\S]*?changeStructuredConversationCheckout\(ownedId, parseRemoteWorkspacePath\(root\)\?\.path \?\? root\)/,
  'checkout promotion is owned by the cancellable session-selection controller'
);
assert.match(editorSessions, /async resetForCheckoutChange\(stopSignal: AbortSignal, checkoutRoot: string\)/);
assert.match(fileTreeWatch, /signal\.addEventListener\("abort", stop, \{ once: true \}\)/);
assert.match(fileTreeWatch, /unwatch\?\.\(\)/);
assert.match(fileTreeWatch, /\{ recursive: false, delayMs: 350 \}/);
assert.doesNotMatch(fileTreeWatch, /recursive: true/);
assert.match(composer, /aria-label=\{sending \? 'Steer current turn' : 'Send message'\}/);
// aec869436 derives steering from turnActive, which still includes conversation.sending.
assert.match(conversationSurface, /const turnActive = \$derived\(Boolean\(\s*conversation\?\.sending[\s\S]*?const steering = turnActive/);
assert.doesNotMatch(
  conversationSurface.slice(conversationSurface.indexOf('async function send()'), conversationSurface.indexOf('async function selectChild')),
  /conversation\.sending \|\|/,
  'the composer can submit steering text during an active turn'
);
assert.match(elapsed, /if \(totalHours < 24\)/);
assert.match(page, /visible=\{workbench\.rightPanelOpen\}/);
assert.match(frame, /setToolsPresent[\s\S]*?api\.removePanel\(panel\)/);
// 2eb707b75 added a deliberate release at the session switch; the right-panel paths must still not release it.
assert.doesNotMatch(
  workbenchController.replace(/beginSessionSwitch\(\): void \{[\s\S]*?\n\t\}/, ''),
  /releaseBrowserWorkspace\(/,
  'the right-panel controller must not release the Browser before Svelte hides its owner'
);
assert.match(
  browserPanel,
  /if \(owningResources\) \{\s*releaseBrowserWorkspace\(\);/,
  'the Browser component releases its native workspace in an effect body; a teardown reads pre-change state'
);
assert.match(
  browserPanel,
  // 81b08dbf0 wraps the listener to also persist the workspace.
  /untrack\(\(\) => subscribeToBrowserNavigation\(\(event\) => \{\s*syncBrowserNavigation\(event\)/,
  'Browser navigation diagnostics cannot become a dependency of their subscribing effect'
);
assert.doesNotMatch(
  browserToolbar,
  /Tooltip\.|IconButton/,
  'Browser toolbar icons must not mount the tooltip state loop over the native view'
);
assert.match(page, /onProblemsLocationChange=\{\(location\) => workbench\.applyProblemsLocation\(location\)\}/);
assert.match(page, /onShowBottomDock=\{\(\) => workbench\.showBottomDock\(\)\}/);
assert.match(workbenchController, /applyProblemsLocation\(location: ProblemsLocation\)[\s\S]*?setDockPresent\(atBottom\)/);
assert.match(workbenchController, /showBottomDock\(\)[\s\S]*?problemsLocation = 'bottom'/);
assert.match(workbenchController, /onFrameReady\([\s\S]*?applyProblemsLocation\(settings\.panels\.problemsLocation\)/);
assert.match(dockPanel, /\{#if terminalOpened\}[\s\S]*?<WorkspaceTerminal/);
assert.match(dockPanel, /await terminal\?\.close\(\)[\s\S]*?problemsLocation = location/);
assert.match(workspaceTerminal, /new AbortController\(\)/);
assert.match(workspaceTerminal, /stop\.abort\(\)[\s\S]*?closeTerminalSessionFromTauri/);
assert.match(palettePanel, /id: 'show-bottom-dock'/);
assert.match(rightPanel, /visible=\{visible && activeId === 'files'\}/);
assert.doesNotMatch(utilityStrip, /LanguageIntelligenceControls/);
assert.match(editorPanel, /<div class="editor-status">[\s\S]*?<LanguageIntelligenceControls \/>/);
assert.match(languageControls, /<Switch[\s\S]*?size="sm"/);
assert.match(languageControls, /data-tone=\{tone\}/);
assert.match(settingsStore, /fontLigatures: false/);
assert.match(settingsStore, /cursorBlink: true/);
assert.match(settingsDialog, /bind:checked=\{settings\.editor\.fontLigatures\}/);
assert.match(settingsDialog, /bind:checked=\{settings\.terminal\.cursorBlink\}/);
assert.doesNotMatch(settingsDialog, /id: 'terminal-theme'/);
assert.match(
  settingsDialog,
  // 725896a73 moved the reset into resetSection(section) behind a confirmation dialog.
  /section === 'general'[\s\S]*?resetSettings\('general'\);[\s\S]*?resetSettings\('panels'\);[\s\S]*?resetSettings\('intelligence'\);[\s\S]*?onProblemsLocationChange\?\.\(settings\.panels\.problemsLocation\)[\s\S]*?await switchLanguageServers\(defaults\.languageServers\)[\s\S]*?await switchLanguageServer\(id, defaults\.languageServerEnabled\[id\]\)/,
  'resetting General must reset every settings-store section represented on that screen and apply the Problems layout immediately'
);
assert.match(
  settingsDialog,
  /shownSection\.id !== 'helper' && shownSection\.id !== 'updates'/,
  'sections without settings-store state must not offer a Reset section action'
);
assert.match(codeMirrorTheme, /fontVariantLigatures: appearance\.fontLigatures \? 'normal' : 'none'/);
assert.match(codeMirrorSourceEditor, /<ContextMenu\.Content side="left"[^>]+aria-label="Editor actions"/);
assert.match(
  codeMirrorSourceEditor,
  /contextmenu: \(event, editor\)[\s\S]*?posAtCoords\([\s\S]*?editor\.dispatch\(\{ selection: \{ anchor: position \} \}\)/,
  'editor context actions must target the symbol that was right-clicked'
);
assert.match(codeMirrorSourceEditor, />Go to Definition<\/ContextMenu\.Item>/);
assert.match(codeMirrorSourceEditor, />Change All Occurrences<\/ContextMenu\.Item>/);
assert.match(codeMirrorSourceEditor, />Format Document<\/ContextMenu\.Item>/);
assert.match(codeMirrorSourceEditor, />Rename Symbol<\/ContextMenu\.Item>/);
assert.match(codeMirrorSourceEditor, />Peek References<\/ContextMenu\.Item>/);
assert.match(xtermFactory, /terminal\.options\.cursorBlink = appearance\.cursorBlink/);
assert.match(languageControls, /serverState === 'ready'[\s\S]*?'running'[\s\S]*?'waiting'/);
assert.match(page, /onCloseAllEditors=\{\(\) => selection\.editorSessions\.clearActiveEditors\(\)\}/);
// 193547d08 lets a checkout change keep unsaved drafts; with no kept files it still persists an empty editor.
assert.match(editorSessions, /async clearActiveEditors\(keptFiles: [^)]*= \[\]\): Promise<boolean>/);
assert.match(editorSessions, /openFiles: keptFiles,[\s\S]*?openPaths: fallback\.openPaths,\s*activePath: null/);
assert.match(
  sourceControlPanel,
  /function releaseHistorySurface\(\): void \{\s*if \(historyRoot === ''\) return;/,
  'an already released source-control surface cannot retrigger its own reactive cleanup'
);
const closeAll = editorPanel.slice(
  editorPanel.indexOf('async function closeAllOpenEditorsNow'),
  editorPanel.indexOf('function closeAllOpenEditors()')
);
assert.ok(
  closeAll.indexOf('resetEditorState();') < closeAll.indexOf('await onCloseAllEditors?.();'),
  'close all releases the visible editor before session persistence can race it'
);

console.log('rail backend cutover tests passed');
