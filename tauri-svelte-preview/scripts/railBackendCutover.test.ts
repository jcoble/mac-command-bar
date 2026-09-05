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
const rowVisual = read('../src/lib/shell/components/SessionRowVisual.svelte');
const rail = read('../src/lib/shell/components/SessionRail.svelte');
const filesPanel = read('../src/lib/shell/panels/files/FilesPanel.svelte');
const fileTreeWatch = read('../src/lib/shell/panels/files/fileTreeWatch.ts');
const composer = read('../src/lib/shell/components/conversation/ConversationComposer.svelte');
const conversationSurface = read('../src/lib/shell/components/ConversationSurface.svelte');
const toolRun = read('../src/lib/shell/components/conversation/ToolRunItem.svelte');
const elapsed = read('../src/lib/shell/components/railElapsedTicker.ts');
const editorSessions = read('../src/lib/shell/controllers/editorSessionController.svelte.ts');
const editorPanel = read('../src/lib/shell/components/EditorPanel.svelte');
const languageControls = read('../src/lib/shell/components/LanguageIntelligenceControls.svelte');
const centerTabs = read('../src/lib/shell/components/CenterCornerTabs.svelte');
const rightPanel = read('../src/lib/shell/components/RightPanel.svelte');
const browserPanel = read('../src/lib/shell/panels/browser/BrowserPanel.svelte');
const browserToolbar = read('../src/lib/shell/panels/browser/BrowserToolbar.svelte');
const frame = read('../src/lib/shell/layout/frame.ts');
const workbenchController = read('../src/lib/shell/controllers/workbenchController.svelte.ts');
const utilityStrip = read('../src/lib/shell/components/UtilityStrip.svelte');
const sourceControlPanel = read('../src/lib/shell/panels/sourceControl/SourceControlPanel.svelte');

assert.match(shellStartup, /listAgentConversationSessionsFromTauri\(\)/);
assert.match(shellStartup, /listRemoteAgentConversationSessionsFromTauri\(stopSignal\)/);
assert.match(shellStartup, /hydrateRemoteSessionsForOwner\(generation, controller\.signal\)/);
assert.match(selectionLayers, /loadConversationForRead\(session\.ownedId, false, owner\.signal\)/);
assert.match(selectionController, /private async drainSelections\(\): Promise<void>/);
assert.match(selectionController, /this\.selectionAbort\?\.abort\(\);[\s\S]*?await this\.selectionWork/);
assert.match(
  selectionController,
  /canonicalPath\(this\.sessionSelectionLayers\.treeRoot\) !== requestedRoot[\s\S]*?clearTreeView\(\)/,
  'a candidate selection tears down the file tree only when its checkout root changes'
);
assert.doesNotMatch(
  page.slice(page.indexOf('async function selectOwned'), page.indexOf('function handoffInput')),
  /ensureStructuredConversation/
);
// WIP: disabled. 65c2353b deleted the legacy localStorage adoption key; the rail is
// whatever SQLite holds, and nothing else.
// assert.equal((page.match(/mac-command-bar\.next\.owned-sessions/g) ?? []).length, 1);
assert.doesNotMatch(railStore, /localStorage|owned-sessions|persist/i);
assert.match(conversationService, /readAgentConversationSnapshotFromTauri\(ownedId, signal\)/);
assert.doesNotMatch(conversationService, /listAgentConversationEventsFromTauri/);
assert.match(conversationService, /model: startConfig\?\.model/);
assert.match(source, /invoke<AgentConversationSessionRecord\[]>\('list_agent_conversation_sessions'/);
assert.match(source, /invoke<AgentConversationEvent\[]>\('list_agent_conversation_events'/);
assert.match(source, /invoke<AgentConversationSessionRecord>\('update_agent_conversation_session_meta'/);
assert.match(row, /session\.runtimeState === ["']suspended["']/);
assert.match(row, /\{onOpenEditor\}/);
assert.match(row, /\{onOpenSourceControl\}/);
assert.match(rowVisual, /runShortcut\(event, onOpenEditor \?\? onSelect\)/);
assert.match(rowVisual, /runShortcut\(event, onOpenSourceControl \?\? onSelect\)/);
assert.match(rail, /onOpenEditor=\{\(\) => void jumpTo\(session, "editor"\)\}/);
assert.match(rail, /onOpenSourceControl=\{\(\) => void jumpTo\(session, "source-control"\)\}/);
assert.match(page, /registerSessionRowJumpTarget\(\{/);
assert.match(page, /showCenterPanel: \(_ownedId, id\) => selectCenterTab\(id\)/);
assert.match(page, /showSidebarView: \(_ownedId, id\) => selectRightTab\(id\)/);
assert.doesNotMatch(
  filesPanel,
  /const sessionKey = `\$\{ownedId/,
  'the file-tree owner is the checkout root, not each session id'
);
assert.match(filesPanel, /const rootChanged = nextRoot !== scopedRoot;/);
assert.match(
  filesPanel,
  /else if \(rootChanged \|\| !nextRoot\) \{\s*activateExplorer\(null\);/,
  'hiding Files parks its bounded tree; only a changed or missing root clears it'
);
assert.match(filesPanel, /watchFileTree\(directories, signal, refreshChangedPaths\)/);
assert.match(filesPanel, /listRepositoryCheckoutsFromTauri\(roots\)/);
assert.match(filesPanel, /data-testid="files-use-session-checkout"/);
assert.match(filesPanel, /if \(inspectionRoot === undefined\) return;/);
assert.match(sourceControlPanel, /if \(inspectionRoot === undefined\) return;/);
assert.match(page, /onUseSessionCheckout=\{[\s\S]*?selection\.useSessionCheckout\(root\)/);
assert.match(
  selectionController,
  /async useSessionCheckout\(requestedRoot: string\): Promise<boolean>[\s\S]*?validateProjectRootFromTauri\(root, owner\.signal\)[\s\S]*?changeStructuredConversationCheckout\(ownedId, root\)/,
  'checkout promotion is owned by the cancellable session-selection controller'
);
assert.match(editorSessions, /async resetForCheckoutChange\(stopSignal: AbortSignal\)/);
assert.match(fileTreeWatch, /signal\.addEventListener\("abort", stop, \{ once: true \}\)/);
assert.match(fileTreeWatch, /unwatch\?\.\(\)/);
assert.match(fileTreeWatch, /\{ recursive: false, delayMs: 350 \}/);
assert.doesNotMatch(fileTreeWatch, /recursive: true/);
assert.match(composer, /data-testid="conversation-working"/);
assert.match(composer, /aria-label=\{sending \? 'Steer current turn' : 'Send message'\}/);
assert.match(conversationSurface, /const steering = conversation\.sending/);
assert.doesNotMatch(
  conversationSurface.slice(conversationSurface.indexOf('async function send()'), conversationSurface.indexOf('async function selectChild')),
  /conversation\.sending \|\|/,
  'the composer can submit steering text during an active turn'
);
assert.match(toolRun, /if \(runWasActive\)[\s\S]*?runOpen = false/);
assert.match(elapsed, /if \(totalHours < 24\)/);
assert.match(rowVisual, /\.row-visual:hover :global\(\.line-title\)/);
assert.doesNotMatch(centerTabs, /LanguageIntelligenceControls/);
assert.match(centerTabs, /data-testid="toggle-right-panel"/);
assert.match(page, /visible=\{workbench\.rightPanelOpen\}/);
assert.match(frame, /setToolsPresent[\s\S]*?api\.removePanel\(panel\)/);
assert.match(workbenchController, /toggleRightPanel\(\)[\s\S]*?setToolsPresent\(open\)/);
assert.doesNotMatch(
  workbenchController,
  /releaseBrowserWorkspace/,
  'the right-panel controller must not release the Browser before Svelte hides its owner'
);
assert.match(
  browserPanel,
  /return \(\) => untrack\(\(\) => \{\s*releaseBrowserWorkspace\(\);/,
  'the Browser component releases its native workspace from its own lifecycle cleanup'
);
assert.equal(
  (browserToolbar.match(/tooltip=\{false\}/g) ?? []).length,
  7,
  'Browser toolbar icons must not mount the tooltip state loop over the native view'
);
assert.match(page, /onProblemsLocationChange=\{\(location\) => workbench\.applyProblemsLocation\(location\)\}/);
assert.match(workbenchController, /applyProblemsLocation\(location: ProblemsLocation\)[\s\S]*?setDockPresent\(atBottom\)/);
assert.match(workbenchController, /onFrameReady\([\s\S]*?applyProblemsLocation\(settings\.panels\.problemsLocation\)/);
assert.match(rightPanel, /visible && activeId === 'browser'/);
assert.match(rightPanel, /visible=\{visible && activeId === 'files'\}/);
assert.match(utilityStrip, /<LanguageIntelligenceControls \/>/);
assert.match(languageControls, /<Switch[\s\S]*?size="sm"/);
assert.match(languageControls, /data-tone=\{tone\}/);
assert.match(languageControls, /serverState === 'ready'[\s\S]*?'running'[\s\S]*?'waiting'/);
assert.match(page, /onCloseAllEditors=\{\(\) => selection\.editorSessions\.clearActiveEditors\(\)\}/);
assert.match(editorSessions, /async clearActiveEditors\(\): Promise<boolean>/);
assert.match(editorSessions, /openPaths: \[\],[\s\S]*?activePath: null/);
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
