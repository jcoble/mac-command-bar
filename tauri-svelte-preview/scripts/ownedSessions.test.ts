import assert from 'node:assert/strict';
import {
  normalizeProvider, adoptAgentSession, createFreshSession,
  ownedSessionFromBackend, ownedSessionMetaForBackend, parseStoredOwnedSessions, reconcileOwnedSessions,
  resolveOwnedSessionProject,
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
{ // project labels never expose the old placeholder
  assert.deepEqual(
    resolveOwnedSessionProject({
      projectPath: '/workspaces/mac-command-bar',
      cwd: '/worktrees/other',
      agent: 'codex',
      viaCmux: false
    }),
    { path: '/workspaces/mac-command-bar', label: 'mac-command-bar' }
  );
  assert.deepEqual(
    resolveOwnedSessionProject({
      projectPath: null,
      cwd: '/worktrees/rail-redesign',
      agent: 'claude',
      viaCmux: false
    }),
    { path: '/worktrees/rail-redesign', label: 'rail-redesign' }
  );
  assert.deepEqual(
    resolveOwnedSessionProject({
      projectPath: 'No Project recorded',
      cwd: '   ',
      agent: 'opencode',
      viaCmux: true
    }),
    { path: '', label: 'opencode session' }
  );
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
  assert.equal(owned.executionOwner, 'stopped');
  assert.equal(owned.runtimeState, 'closed');
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
{ // when the session was last busy travels with it, so a working row can say so
  const seen = '2026-07-28T09:00:00.000Z';
  const owned = adoptAgentSession({ ...scanRecord, lastActivity: seen }, mint);
  assert.equal(owned.lastActivity, seen);
  // A scan with nothing to say about it leaves the row with no stamp at all.
  assert.equal(adoptAgentSession(scanRecord, mint).lastActivity, null);
  for (const junk of [123, true, {}, [], '', undefined]) {
    assert.equal(
      adoptAgentSession({ ...scanRecord, lastActivity: junk }, mint).lastActivity,
      null,
      `a last-activity stamp of ${JSON.stringify(junk)}`
    );
  }
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
  // Including when it was last busy: that is the scanner's reading of a
  // conversation on disk, and a shell started here has not had one yet.
  assert.equal(fresh.lastActivity, null);
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
  const parsed = parseStoredOwnedSessions(JSON.stringify([owned]));
  assert.deepEqual(parsed, [owned]);
  assert.deepEqual(parseStoredOwnedSessions('not json'), []);
  assert.deepEqual(parseStoredOwnedSessions('{"a":1}'), []);
  assert.deepEqual(parseStoredOwnedSessions(JSON.stringify([{ ownedId: '', cwd: '/x' }])), []);
  const weird = { ...owned, state: 'zombie' };
  assert.equal(parseStoredOwnedSessions(JSON.stringify([weird]))[0].state, 'exited');
}
{ // when the user marked a session done survives a save and a reload
  const done = { ...adoptAgentSession(scanRecord, mint), completedAt: '2026-07-28T10:00:00.000Z' };
  const parsed = parseStoredOwnedSessions(JSON.stringify([done]));
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
{ // Settled is a separate explicit migration field, never inferred from age/state.
  const settled = {
    ...adoptAgentSession(scanRecord, mint),
    state: 'live',
    settledAt: '2026-08-01T10:00:00.000Z'
  };
  assert.equal(parseStoredOwnedSessions(JSON.stringify([settled]))[0].settledAt, settled.settledAt);
  const { settledAt, ...older } = settled;
  assert.equal(parseStoredOwnedSessions(JSON.stringify([older]))[0].settledAt, null);
}
{ // the branch, task and pull request survive a save and a reload
  const owned = adoptAgentSession(scanRecord, mint);
  const parsed = parseStoredOwnedSessions(JSON.stringify([owned]));
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
  const parsed = parseStoredOwnedSessions(JSON.stringify([owned]));
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
{ // when the session was last busy survives a save and a reload
  const seen = '2026-07-28T09:00:00.000Z';
  const owned = adoptAgentSession({ ...scanRecord, lastActivity: seen }, mint);
  const parsed = parseStoredOwnedSessions(JSON.stringify([owned]));
  assert.deepEqual(parsed, [owned]);
  // Sessions saved before this field existed come back with no stamp.
  const { lastActivity, ...older } = owned;
  assert.equal(parseStoredOwnedSessions(JSON.stringify([older]))[0].lastActivity, null);
  // Anything that is not text is not a stamp.
  for (const junk of [123, true, {}, [], '']) {
    const record = parseStoredOwnedSessions(JSON.stringify([{ ...owned, lastActivity: junk }]))[0];
    assert.equal(record.lastActivity, null, `a last-activity stamp of ${JSON.stringify(junk)}`);
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
  assert.equal(owned[0].executionOwner, 'terminal');
  assert.equal(owned[0].runtimeState, 'ready');
  assert.equal(owned[1].state, 'exited');
  assert.equal(owned[1].executionOwner, 'stopped');
  assert.equal(owned[2].state, 'exited');
  assert.equal(owned[2].ptySessionId, 'term-3');
  assert.equal(owned[2].executionOwner, 'stopped');
  assert.deepEqual(reattachable.map((s) => s.ownedId), ['a']);
}
{ // a structured session without a PTY stays resumable after reload
  const structured = {
    ...adoptAgentSession(scanRecord, () => 'structured-app'),
    origin: 'app',
    ptySessionId: null,
    state: 'live',
    executionOwner: 'structured',
    runtimeState: 'ready'
  };
  const { owned, reattachable } = reconcileOwnedSessions([structured], []);
  assert.equal(owned[0].state, 'background');
  assert.equal(owned[0].executionOwner, 'structured');
  assert.equal(owned[0].runtimeState, 'closed');
  assert.deepEqual(reattachable, []);

  const staleStopped = {
    ...structured,
    state: 'exited',
    executionOwner: 'stopped',
    runtimeState: 'closed'
  };
  const recovered = reconcileOwnedSessions([staleStopped], []).owned[0];
  assert.equal(recovered.state, 'background');
  assert.equal(recovered.executionOwner, 'structured');

  const failed = {
    ...staleStopped,
    runtimeState: 'failed',
    lastError: 'structured connection failed'
  };
  const retainedFailure = reconcileOwnedSessions([failed], []).owned[0];
  assert.equal(retainedFailure.state, 'exited');
  assert.equal(retainedFailure.executionOwner, 'stopped');
  assert.equal(retainedFailure.runtimeState, 'failed');
}
{ // a pre-runtime record migrates without claiming structured ownership
  const current = adoptAgentSession(scanRecord, mint);
  const {
    executionOwner, runtimeState, providerInstanceId, activeTurnId,
    capabilityRevision, lastRuntimeError, ...legacy
  } = { ...current, ptySessionId: 'term-legacy', state: 'background' };
  const migrated = parseStoredOwnedSessions(JSON.stringify([legacy]))[0];
  assert.equal(migrated.executionOwner, 'terminal');
  assert.equal(migrated.runtimeState, 'ready');
  assert.equal(migrated.ptySessionId, 'term-legacy');
  assert.equal(migrated.nativeSessionId, 'native-9');
  assert.deepEqual(parseStoredOwnedSessions(JSON.stringify([migrated])), [migrated]);
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
{ // SQLite list rows project every persisted rail field and the live overlay
  const record = {
    ownedId: 'owned-db', provider: 'codex' as const, model: 'model-a', effort: 'high',
    cwd: '/tmp/project/worktree', state: 'waiting-approval' as const, suspended: false,
    createdAtMs: 10, lastActivityAtMs: 20, activeTurnId: 'turn-a',
    pendingPermission: true, pendingInput: false, nativeSessionId: 'native-a',
    worktree: '/tmp/project/worktree', branch: 'tsk-872-db-rail', title: 'DB rail',
    project: '/tmp/project', ptySessionId: null, origin: 'app' as const,
    source: 'fresh' as const, viaCmux: false, resumeCommand: null,
    completedAt: null, settledAt: null, taskId: 'TSK-872', pullRequest: null,
    messageCount: 7, latestTurnPreview: 'Agent: retained',
    scannedLastActivity: '2026-08-13T10:00:00.000Z'
  };
  const projected = ownedSessionFromBackend(record);
  assert.equal(projected.runtimeState, 'waiting-approval');
  assert.equal(projected.pendingPermission, true);
  assert.equal(projected.branch, record.branch);
  assert.equal(projected.title, record.title);
  assert.equal(projected.projectPath, record.project);
  assert.equal(ownedSessionMetaForBackend(projected).messageCount, 7);
  assert.equal(ownedSessionFromBackend({ ...record, suspended: true }).runtimeState, 'suspended');
}
console.log('ownedSessions tests passed');
