import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (relativePath: string): string => readFileSync(new URL(relativePath, import.meta.url), 'utf8');
const page = read('../src/routes/next/+page.svelte');
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
const utilityStrip = read('../src/lib/shell/components/UtilityStrip.svelte');

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
assert.match(page, /showCenterPanel: \(_ownedId, id\) => workbench\.selectCenterTab\(id\)/);
assert.match(page, /showSidebarView: \(_ownedId, id\) => workbench\.selectRightTab\(id\)/);
assert.doesNotMatch(
  filesPanel,
  /const sessionKey = `\$\{ownedId/,
  'the file-tree owner is the checkout root, not each session id'
);
assert.match(filesPanel, /if \(nextRoot !== scopedRoot\)/);
assert.match(filesPanel, /watchFileTree\(target, signal, refresh\)/);
assert.match(fileTreeWatch, /signal\.addEventListener\("abort", stop, \{ once: true \}\)/);
assert.match(fileTreeWatch, /unwatch\?\.\(\)/);
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
assert.match(utilityStrip, /<LanguageIntelligenceControls \/>/);
assert.match(languageControls, /<Switch[\s\S]*?size="sm"/);
assert.match(languageControls, /data-tone=\{tone\}/);
assert.match(languageControls, /serverState === 'ready'[\s\S]*?'running'[\s\S]*?'waiting'/);
assert.match(page, /onCloseAllEditors=\{\(\) => selection\.editorSessions\.clearActiveEditors\(\)\}/);
assert.match(editorSessions, /async clearActiveEditors\(\): Promise<boolean>/);
assert.match(editorSessions, /openPaths: \[\],[\s\S]*?activePath: null/);
const closeAll = editorPanel.slice(
  editorPanel.indexOf('async function closeAllOpenEditorsNow'),
  editorPanel.indexOf('function closeAllOpenEditors()')
);
assert.ok(
  closeAll.indexOf('resetEditorState();') < closeAll.indexOf('await onCloseAllEditors?.();'),
  'close all releases the visible editor before session persistence can race it'
);

console.log('rail backend cutover tests passed');
