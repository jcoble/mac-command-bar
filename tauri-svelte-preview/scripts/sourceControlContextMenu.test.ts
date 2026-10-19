import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import { test } from 'node:test';

import {
  snapshotSourceControlCommitFileMenu,
  snapshotSourceControlCommitMenu,
  sourceControlCommitFileContextMenuItems
} from '../src/lib/shell/components/git/sourceControlContextMenu.ts';

const anchor = {
  left: 400,
  right: 400,
  top: 300,
  bottom: 300,
  containingBlockLeft: 0,
  containingBlockRight: 800,
  containingBlockTop: 0,
  containingBlockBottom: 600
};

assert.deepEqual(
  sourceControlCommitFileContextMenuItems(true).map(({ id, label, enabled }) => [
    id,
    label,
    enabled
  ]),
  [
    ['open-commit-diff', 'Open diff', true],
    ['open-current-file', 'Open current file in editor', true],
    ['copy-commit-path', 'Copy path', true]
  ]
);
assert.deepEqual(
  sourceControlCommitFileContextMenuItems(false).map(({ id, enabled }) => [id, enabled]),
  [
    ['open-commit-diff', false],
    ['open-current-file', false],
    ['copy-commit-path', true]
  ]
);

const commit = snapshotSourceControlCommitMenu({
  sha: 'abc123',
  isMerge: false,
  expanded: false,
  anchor
});
assert.equal(commit.items[0]?.label, 'View changed files');

const originalFile = { relativePath: 'src/app.ts', status: 'modified', badge: 'M' };
const file = snapshotSourceControlCommitFileMenu({
  sha: 'abc123',
  file: originalFile,
  readable: true,
  anchor
});
originalFile.relativePath = 'src/changed-after-menu-open.ts';
assert.equal(file.target.file.relativePath, 'src/app.ts', 'the menu owns an immutable row snapshot');

for (const relativePath of [
  '../src/lib/shell/panels/sourceControl/CommitTimeline.svelte',
  '../src/lib/shell/components/git/GitHistoryView.svelte'
]) {
  const source = readFileSync(new URL(relativePath, import.meta.url), 'utf8');
  assert.match(source, /let contextMenu = \$state\.raw<SourceControlMenuSnapshot \| null>\(null\)/);
  assert.match(source, /<SourceControlContextMenu/);
  assert.doesNotMatch(source, /ContextMenu\.Root/);
}

console.log('sourceControlContextMenu: commit and commit-file menus passed');

test('checkout availability follows backend support and preserves session safety gates', () => {
  const source = readFileSync(new URL('../src/lib/shell/panels/sourceControl/SourceControlPanel.svelte', import.meta.url), 'utf8');
  const body = source.match(/const checkoutDisabledReason = \$derived.by\(\(\) => \{([\s\S]*?)\n  \}\);/)![1];
  const reason = new Function('readOnlyScope', 'onUseSessionCheckout', 'rail', 'ownedId', 'ownedSession', 'conversation', 'activeTool', body);
  const conversation = { sending: false, pendingApprovals: {}, pendingInputs: {} };
  const session = { origin: 'app', executionOwner: 'structured', runtimeState: 'ready', canChangeCheckout: true };
  const check = (ownedSession = session, chat = conversation) => reason(true, () => {}, { activeOwnedId: 's' }, 's', ownedSession, chat, false);
  for (const agent of ['codex', 'claude', 'antigravity']) {
    assert.equal(check({ ...session, agent } as typeof session), null);
    assert.equal(check({ ...session, agent, canChangeCheckout: false } as typeof session), "This session's agent can't keep its conversation in another checkout.");
  }
  assert.match(check({ ...session, origin: 'external' }), /Assembly/);
  assert.match(check(session, { ...conversation, sending: true }), /active turn/);
  assert.match(check(session, { ...conversation, pendingApprovals: { one: {} } }), /permission/);
  assert.match(check(session, { ...conversation, pendingInputs: { one: {} } }), /input/);
});

test('checkout service reports unsupported sessions and accepts supported providers', async () => {
  const source = readFileSync(new URL('../src/lib/shell/conversation/conversationService.ts', import.meta.url), 'utf8');
  const method = source.slice(source.indexOf('export async function changeStructuredConversationCheckout('), source.indexOf('const preparingSends'));
  const code = stripTypeScriptTypes(method.replace('export async function', 'async function'));
  for (const provider of ['codex', 'claude', 'antigravity']) {
    for (const canChangeCheckout of [false, true]) {
      let calls = 0;
      const state = { provider, generation: 1, connectionState: 'connected' };
      const record = { cwd: '/b', nativeSessionId: 'native', suspended: true };
      const change = new Function('getConversationSession', 'rail', 'changeAgentConversationCheckoutFromTauri', 'updateOwnedSession', 'setConversationConnection', `${code}; return changeStructuredConversationCheckout;`)(
        () => state, { owned: [{ ownedId: 's', origin: 'app', canChangeCheckout }] },
        async () => { calls++; return record; }, () => {}, () => {}
      );
      if (canChangeCheckout) assert.equal(await change('s', '/b'), record);
      else await assert.rejects(change('s', '/b'), /agent can't keep its conversation/);
      assert.equal(calls, canChangeCheckout ? 1 : 0);
    }
  }
});

test('fresh session capability loading refreshes checkout support from the backend record', async () => {
  const source = readFileSync(new URL('../src/lib/shell/conversation/conversationService.ts', import.meta.url), 'utf8');
  const start = source.indexOf('export async function loadConversationCapabilities(');
  const method = source.slice(start, source.indexOf('\n}\n', start) + 3);
  const code = stripTypeScriptTypes(method.replace('export async function', 'async function'));
  for (const aborted of [false, true]) {
    const controller = new AbortController();
    let support: boolean | undefined;
    const load = new Function('isTauri', 'getConversationSession', 'readAgentConversationCapabilitiesFromTauri',
      'conversationGenerationMatches', 'setConversationCapabilities', 'findSessionRecord', 'SESSION_READERS',
      'adoptController', 'updateOwnedSession', 'setConversationCapabilityError', `${code}; return loadConversationCapabilities;`)(
      () => true, () => ({ generation: 1 }), async () => ({ provider: 'claude' }), () => true, () => {},
      async () => { if (aborted) controller.abort(); return { canChangeCheckout: true }; }, {}, controller,
      (_id: string, patch: { canChangeCheckout: boolean }) => { support = patch.canChangeCheckout; }, () => {}
    );
    await load('s', 'claude', controller.signal);
    assert.equal(support, aborted ? undefined : true);
  }
});
