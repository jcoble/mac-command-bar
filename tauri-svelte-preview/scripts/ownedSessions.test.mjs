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
}
{ // createFreshSession defaults
  const fresh = createFreshSession({ cwd: '/tmp/deep/proj' }, mint);
  assert.equal(fresh.title, 'proj');
  assert.equal(fresh.source, 'fresh');
  assert.equal(fresh.resumeCommand, null);
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
console.log('ownedSessions tests passed');
