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
const filesPanel = read('../src/lib/shell/panels/files/FilesPanel.svelte');

assert.match(shellStartup, /listAgentConversationSessionsFromTauri\(\)/);
assert.match(shellStartup, /listRemoteAgentConversationSessionsFromTauri\(signal\)/);
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
assert.doesNotMatch(
  filesPanel,
  /const sessionKey = `\$\{ownedId/,
  'the file-tree owner is the checkout root, not each session id'
);
assert.match(filesPanel, /if \(nextRoot !== scopedRoot\)/);

console.log('rail backend cutover tests passed');
