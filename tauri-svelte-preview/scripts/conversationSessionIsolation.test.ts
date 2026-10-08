import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';

import {
  captureWorkspace,
  normalizeWorkspaceSnapshot
} from '../src/lib/shell/sessionWorkspaces.ts';

function sessionSnapshot(name, mode, activeKey) {
  return captureWorkspace({
    openFiles: [{ path: `/repo/${name}.cs` }],
    activePath: `/repo/${name}.cs`,
    selectedPath: `/repo/${name}.cs`,
    scrollTop: name.length,
    diffPath: `${name}.cs`,
    diffRoot: '/repo',
    conversation: {
      mode,
      draft: `draft-${name}`,
      selectedChildId: `child-${name}`,
      providerGeneration: 2,
      lastSequence: 8,
      version: 1,
      generation: 2,
      owner: 'terminal',
      attachmentIds: [`attachment-${name}`],
      config: { future_category: name },
      viewByHistoryId: { [name]: { followLatest: false, anchor: { itemId: `item-${name}`, firstSequence: 1, offsetPx: 20 }, expandedTurns: {} } },
      sequence: 8,
      telemetry: { providerLatencyMs: name.length },
      futureWorkspaceField: { owner: name }
    },
    browser: {
      tabs: [{
        id: `browser-${name}`,
        url: `http://${name}.localhost:5177/`,
        inputUrl: `${name}.localhost:5177`,
        title: name
      }],
      activeTabId: `browser-${name}`
    },
    topTabs: {
      order: ['diff', `editor:/repo/${name}.cs`, `browser:browser-${name}`],
      activeKey
    }
  });
}

const sessions = {
  'owned-a': sessionSnapshot('alpha', 'structured', 'editor:/repo/alpha.cs'),
  'owned-b': sessionSnapshot('bravo', 'raw', 'browser:browser-bravo'),
  'owned-c': sessionSnapshot('charlie', 'structured', 'diff')
};

const restored = Object.fromEntries(
  Object.entries(sessions).map(([ownedId, snapshot]) => [
    ownedId,
    normalizeWorkspaceSnapshot(snapshot)!
  ])
);

assert.equal('draft' in restored['owned-a'].conversation, false);
assert.equal(restored['owned-b'].conversation.mode, 'raw');
assert.equal(restored['owned-a'].conversation.selectedChildId, 'child-alpha');
assert.equal(restored['owned-b'].conversation.viewByHistoryId.bravo.anchor.itemId, 'item-bravo');
assert.equal(restored['owned-c'].browser.tabs[0].url, 'http://charlie.localhost:5177/');
assert.equal(restored['owned-c'].browser.activeTabId, 'browser-charlie');
assert.equal(restored['owned-a'].topTabs.activeKey, 'editor:/repo/alpha.cs');
assert.equal(restored['owned-b'].topTabs.activeKey, 'browser:browser-bravo');
assert.equal(restored['owned-c'].topTabs.activeKey, 'diff');
assert.notDeepEqual(restored['owned-a'].topTabs.order, restored['owned-b'].topTabs.order);
assert.deepEqual(restored['owned-a'].conversation.attachmentIds, ['attachment-alpha']);
assert.equal(restored['owned-b'].conversation.config.future_category, 'bravo');
assert.equal(restored['owned-a'].conversation.futureWorkspaceField.owner, 'alpha');
assert.notDeepEqual(
  restored['owned-a'].conversation.viewByHistoryId,
  restored['owned-b'].conversation.viewByHistoryId
);

// A pre-conversation snapshot migrates without inventing another session's UI.
const legacy = normalizeWorkspaceSnapshot({
  openPaths: ['/repo/legacy.cs'],
  activePath: '/repo/legacy.cs',
  selectedPath: null,
  scrollTop: 0,
  diffPath: null,
  diffRoot: null
});
assert.ok(legacy);
assert.equal(legacy.conversation, undefined);
assert.equal(legacy.browser, undefined);
assert.equal(legacy.topTabs, undefined);

// Exercise the production fixed client slots with a transport-free chat fixture.
{
  const source = readFileSync(new URL('../src/lib/shell/conversation/conversationConnection.ts', import.meta.url), 'utf8');
  const current = source.slice(source.indexOf('function isCurrent('), source.indexOf('function messageBytes('));
  const bind = source.slice(source.indexOf('function bindSelectionSignal('), source.indexOf('async function restorePageAttachments('));
  const selection = source.slice(source.indexOf('export function selectConversationChat('), source.indexOf('export function sendSelectedConversationMessage(')).replaceAll('export ', '');
  const states = new Map<string, { provider: string; generation: number; selectedChildHistoryOwnedId: string | null }>();
  const clearedAttachments: string[] = [];
  const disposed: string[] = [];
  const dependencies = {
    getConversationSession: (id: string) => states.get(id),
    ensureConversationSession: (id: string, provider: string) => {
      if (!states.has(id)) states.set(id, { provider, generation: 1, selectedChildHistoryOwnedId: null });
      return states.get(id);
    },
    evictConversationSession: (id: string) => states.delete(id),
    restoreSelectedConversationAttachments: (id: string) => clearedAttachments.push(id),
    rejectPendingAdmission: (value: { pendingAdmission: unknown }) => { value.pendingAdmission = null; },
    createConnection: () => ({}),
    createChat: ({ threadId }: { threadId: string }) => ({
      messages: [], dispose: () => disposed.push(threadId)
    })
  };
  const api = Function(...Object.keys(dependencies), stripTypeScriptTypes(`
    let active = null;
    let childActive = null;
    ${current}${bind}${selection}
  `, { mode: 'strip' }) + `
    return { selectConversationChat, selectedConversationChat, disposeChildConversationChat,
      disposeSelectedConversationChat, parent: () => active };
  `)(...Object.values(dependencies));
  dependencies.ensureConversationSession('parent-a', 'codex');
  const parentOwner = new AbortController();
  const parent = api.selectConversationChat('parent-a', 'parent-a', parentOwner.signal);
  const admission = { waiting: true };
  api.parent().pendingAdmission = admission;
  const childOwner = new AbortController();
  const child = api.selectConversationChat('parent-a', 'history-a', childOwner.signal);
  assert.equal(api.selectedConversationChat('parent-a'), parent);
  assert.equal(api.selectedConversationChat('parent-a', 'history-a'), child);
  assert.equal(api.selectConversationChat('parent-a', 'history-a', childOwner.signal), child);
  assert.deepEqual(disposed, []);
  api.disposeChildConversationChat('parent-a');
  assert.equal(api.selectedConversationChat('parent-a'), parent);
  assert.equal(api.parent().pendingAdmission, admission, 'Back preserves an admitted parent send');
  assert.equal(states.has('history-a'), false);
  assert.equal(states.get('parent-a')!.selectedChildHistoryOwnedId, null);
  assert.deepEqual(clearedAttachments, ['history-a']);
  const obsoleteOwner = new AbortController();
  api.selectConversationChat('parent-a', 'history-b', obsoleteOwner.signal);
  const next = api.selectConversationChat('parent-a', 'history-c');
  obsoleteOwner.abort();
  assert.equal(api.selectedConversationChat('parent-a', 'history-c'), next, 'obsolete owner cannot dispose the replacement');
  assert.equal(states.has('history-b'), false);
  parentOwner.abort();
  assert.equal(api.selectedConversationChat('parent-a'), null);
  assert.equal(api.selectedConversationChat('parent-a', 'history-c'), null);
  assert.equal(states.has('history-c'), false);
  assert.deepEqual(disposed, ['history-a', 'history-b', 'history-c', 'parent-a']);
}

console.log('conversation session isolation tests passed');
