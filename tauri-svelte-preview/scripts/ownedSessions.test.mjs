import assert from 'node:assert/strict';
import {
  normalizeProvider, adoptAgentSession, createFreshSession,
  serializeOwnedSessions, parseStoredOwnedSessions, reconcileOwnedSessions,
} from '../src/lib/shell/ownedSessions.ts';

const mint = () => 'owned-1';
const scanRecord = {
  provider: 'cmux-claude', id: 'native-9', title: 'Fix rail', description: null,
  model: null, projectPath: '/tmp/proj', lastActivity: null,
  resumeCommands: ['claude --resume native-9'],
  branchHint: 'tsk-788-session-workspaces', taskId: 'TSK-788', pullRequestHint: 'PR #12',
  sourceLabel: 'CMUX Claude · proj',
  messageCount: 12, latestTurnPreview: 'Agent: fixed the reference race',
};

{ // normalizeProvider
  assert.deepEqual(normalizeProvider('codex'), { agent: 'codex', viaCmux: false });
  assert.deepEqual(normalizeProvider('CMUX-Claude '), { agent: 'claude', viaCmux: true });
  assert.deepEqual(normalizeProvider('cmux-rovo'), { agent: 'other', viaCmux: true });
  assert.deepEqual(normalizeProvider(''), { agent: 'other', viaCmux: false });
}
{ // adoptAgentSession
  const owned = adoptAgentSession(scanRecord, mint);
  assert.equal(owned.ownedId, 'owned-1');
  assert.equal(owned.agent, 'claude');
  assert.equal(owned.viaCmux, true);
  assert.equal(owned.source, 'scanned');
  assert.equal(owned.nativeSessionId, 'native-9');
  assert.equal(owned.cwd, '/tmp/proj');
  assert.equal(owned.resumeCommand, 'claude --resume native-9');
  assert.equal(owned.state, 'background');
  assert.equal(owned.ptySessionId, null);
  // Adopting is not completing: only the user's "Mark done" sets this.
  assert.equal(owned.completedAt, null);
  // What the scanner worked out travels with the session, so a row keeps its
  // branch, task and pull request after it has been adopted.
  assert.equal(owned.branch, 'tsk-788-session-workspaces');
  assert.equal(owned.taskId, 'TSK-788');
  assert.equal(owned.pullRequest, 'PR #12');
  // So do how much was said and the last thing said, so the row keeps them.
  assert.equal(owned.messageCount, 12);
  assert.equal(owned.latestTurnPreview, 'Agent: fixed the reference race');
}
{ // a scan that found none of it leaves the fields empty rather than blank chips
  const { branchHint, taskId, pullRequestHint, messageCount, latestTurnPreview, ...bare } =
    scanRecord;
  const owned = adoptAgentSession(bare, mint);
  assert.equal(owned.branch, null);
  assert.equal(owned.taskId, null);
  assert.equal(owned.pullRequest, null);
  assert.equal(owned.messageCount, null);
  assert.equal(owned.latestTurnPreview, null);
}
{ // a count of zero turns is nothing to show, not "0 messages"
  const owned = adoptAgentSession({ ...scanRecord, messageCount: 0 }, mint);
  assert.equal(owned.messageCount, null);
  // Anything that is not a whole count of turns is not a count.
  for (const junk of [-3, 2.5, '12', true, {}, null]) {
    assert.equal(adoptAgentSession({ ...scanRecord, messageCount: junk }, mint).messageCount, null);
  }
}
{ // a session started here has nothing scanned about it yet
  const fresh = createFreshSession({ cwd: '/tmp/deep/proj' }, mint);
  assert.equal(fresh.branch, null);
  assert.equal(fresh.taskId, null);
  assert.equal(fresh.pullRequest, null);
  assert.equal(fresh.messageCount, null);
  assert.equal(fresh.latestTurnPreview, null);
}
{ // createFreshSession defaults
  const fresh = createFreshSession({ cwd: '/tmp/deep/proj' }, mint);
  assert.equal(fresh.title, 'proj');
  assert.equal(fresh.source, 'fresh');
  assert.equal(fresh.resumeCommand, null);
  assert.equal(fresh.completedAt, null);
}
{ // persistence round-trip + tolerance
  const owned = adoptAgentSession(scanRecord, mint);
  const parsed = parseStoredOwnedSessions(serializeOwnedSessions([owned]));
  assert.deepEqual(parsed, [owned]);
  assert.deepEqual(parseStoredOwnedSessions('not json'), []);
  assert.deepEqual(parseStoredOwnedSessions('{"a":1}'), []);
  assert.deepEqual(parseStoredOwnedSessions(JSON.stringify([{ ownedId: '', cwd: '/x' }])), []);
  const weird = { ...owned, state: 'zombie' };
  assert.equal(parseStoredOwnedSessions(JSON.stringify([weird]))[0].state, 'exited');
}
{ // when the user marked a session done survives a save and a reload
  const done = { ...adoptAgentSession(scanRecord, mint), completedAt: '2026-07-28T10:00:00.000Z' };
  const parsed = parseStoredOwnedSessions(serializeOwnedSessions([done]));
  assert.deepEqual(parsed, [done]);
  // Sessions saved before this field existed come back as "not done".
  const { completedAt, ...older } = done;
  assert.equal(parseStoredOwnedSessions(JSON.stringify([older]))[0].completedAt, null);
  // Anything that is not a stamp is not a stamp.
  for (const junk of [123, true, {}, [], '']) {
    const record = { ...done, completedAt: junk };
    assert.equal(parseStoredOwnedSessions(JSON.stringify([record]))[0].completedAt, null);
  }
}
{ // the branch, task and pull request survive a save and a reload
  const owned = adoptAgentSession(scanRecord, mint);
  const parsed = parseStoredOwnedSessions(serializeOwnedSessions([owned]));
  assert.deepEqual(parsed, [owned]);
  // Sessions saved before these fields existed come back with nothing to show.
  const { branch, taskId, pullRequest, ...older } = owned;
  const restored = parseStoredOwnedSessions(JSON.stringify([older]))[0];
  assert.equal(restored.branch, null);
  assert.equal(restored.taskId, null);
  assert.equal(restored.pullRequest, null);
  // Anything that is not text is not a branch name.
  for (const junk of [123, true, {}, [], '']) {
    const record = parseStoredOwnedSessions(
      JSON.stringify([{ ...owned, branch: junk, taskId: junk, pullRequest: junk }])
    )[0];
    assert.deepEqual([record.branch, record.taskId, record.pullRequest], [null, null, null]);
  }
}
{ // how much was said and the last thing said survive a save and a reload
  const owned = adoptAgentSession(scanRecord, mint);
  const parsed = parseStoredOwnedSessions(serializeOwnedSessions([owned]));
  assert.equal(parsed[0].messageCount, 12);
  assert.equal(parsed[0].latestTurnPreview, 'Agent: fixed the reference race');
  // Sessions saved before these fields existed come back with nothing to show.
  const { messageCount, latestTurnPreview, ...older } = owned;
  const restored = parseStoredOwnedSessions(JSON.stringify([older]))[0];
  assert.equal(restored.messageCount, null);
  assert.equal(restored.latestTurnPreview, null);
  // And neither is anything a count or a line of text is not.
  for (const junk of [-3, 2.5, '12', true, {}, [], '', 0]) {
    const record = parseStoredOwnedSessions(JSON.stringify([{ ...owned, messageCount: junk }]))[0];
    assert.equal(record.messageCount, null, `a message count of ${JSON.stringify(junk)}`);
  }
  for (const junk of [123, true, {}, [], '']) {
    const record = parseStoredOwnedSessions(
      JSON.stringify([{ ...owned, latestTurnPreview: junk }])
    )[0];
    assert.equal(record.latestTurnPreview, null, `a turn preview of ${JSON.stringify(junk)}`);
  }
}
{ // reconcile after reload
  const a = { ...adoptAgentSession(scanRecord, () => 'a'), ptySessionId: 'term-1' };
  const b = { ...adoptAgentSession(scanRecord, () => 'b'), ptySessionId: 'term-2' };
  const c = { ...adoptAgentSession(scanRecord, () => 'c'), ptySessionId: 'term-3' };
  const { owned, reattachable } = reconcileOwnedSessions([a, b, c], [
    { sessionId: 'term-1', exited: false },
    { sessionId: 'term-2', exited: true },
  ]);
  assert.equal(owned.length, 3);
  assert.equal(owned[0].state, 'background');
  assert.equal(owned[1].state, 'exited');
  assert.equal(owned[2].state, 'exited');
  assert.equal(owned[2].ptySessionId, null);
  assert.deepEqual(reattachable.map((s) => s.ownedId), ['a']);
}
{ // a reload never marks a session done and never un-marks one
  const stamp = '2026-07-28T10:00:00.000Z';
  const mark = (id, ptySessionId) => ({
    ...adoptAgentSession(scanRecord, () => id), ptySessionId, completedAt: stamp,
  });
  const { owned } = reconcileOwnedSessions(
    [mark('a', 'term-1'), mark('b', 'term-2'), mark('c', 'term-3')],
    [{ sessionId: 'term-1', exited: false }, { sessionId: 'term-2', exited: true }]
  );
  // still running / its terminal died / its terminal is gone entirely
  assert.deepEqual(owned.map((s) => s.completedAt), [stamp, stamp, stamp]);
  const notDone = reconcileOwnedSessions(
    [{ ...mark('d', 'term-4'), completedAt: null }],
    [{ sessionId: 'term-4', exited: false }]
  );
  assert.equal(notDone.owned[0].completedAt, null);
}
console.log('ownedSessions tests passed');
