import assert from 'node:assert/strict';

import {
  buildSessionLibrary,
  canonicalCwd,
  deriveOwnedLibraryState,
  filterSessionLibrary,
  groupSessionLibrary,
  paginateSessionLibrary,
  sessionIdentityKey
} from '../src/lib/shell/sessionLibrary/sessionLibraryModel.ts';
import {
  createSessionLibraryMountCoordinator,
  createSessionLibraryService
} from '../src/lib/shell/sessionLibrary/sessionLibraryService.ts';

function owned(ownedId, cwd, extra = {}) {
  return {
    ownedId,
    agent: 'codex',
    viaCmux: false,
    source: 'fresh',
    title: 'Same title',
    projectPath: cwd,
    cwd,
    resumeCommand: null,
    nativeSessionId: 'native-1',
    ptySessionId: null,
    state: 'background',
    completedAt: null,
    settledAt: null,
    branch: null,
    taskId: null,
    pullRequest: null,
    messageCount: null,
    latestTurnPreview: null,
    lastActivity: null,
    ...extra
  };
}

function available(id, cwd, extra = {}) {
  return {
    provider: 'codex',
    id,
    title: 'Same title',
    description: null,
    model: 'o4-mini',
    projectPath: cwd,
    lastActivity: null,
    resumeCommands: [],
    ...extra
  };
}

// Canonical identity never uses title: same-title sessions in two worktrees
// remain distinct, while separator spelling and trailing slashes normalize.
{
  assert.equal(canonicalCwd('/repo/worktree/'), '/repo/worktree');
  assert.equal(canonicalCwd('C:\\Repo\\Worktree\\'), 'c:/Repo/Worktree');
  assert.notEqual(
    sessionIdentityKey({ provider: 'codex', nativeSessionId: 'n', cwd: '/one' }),
    sessionIdentityKey({ provider: 'codex', nativeSessionId: 'n', cwd: '/two' })
  );
}

// OwnedId wins first; then the provider/native/cwd tuple deduplicates provider
// rows. A same-title different-worktree row is not dropped.
{
  const first = owned('owned-1', '/one');
  const duplicateOwned = owned('owned-1', '/duplicate');
  const second = owned('owned-2', '/two');
  const duplicateIdentity = owned('owned-3', '/two');
  const rows = buildSessionLibrary(
    [first, duplicateOwned, second, duplicateIdentity],
    [available('native-1', '/one'), available('native-2', '/three')]
  );
  assert.deepEqual(rows.filter((row) => row.source === 'owned').map((row) => row.ownedId), [
    'owned-1',
    'owned-2'
  ]);
  assert.equal(rows.filter((row) => row.source === 'provider').length, 1);
  assert.equal(rows.filter((row) => row.source === 'owned').length, 2, 'compound identity dedupes owned rows too');
  assert.equal(rows.at(-1).canonicalCwd, '/three');
}

// Settled is explicit settledAt semantics; age/title/process fields do not move
// a record between shelves.
{
  assert.equal(deriveOwnedLibraryState(owned('working', '/one')), 'working');
  assert.equal(
    deriveOwnedLibraryState(owned('done', '/one', { completedAt: '2000-01-01T00:00:00Z' })),
    'done'
  );
  assert.equal(
    deriveOwnedLibraryState(
      owned('settled', '/one', { state: 'live', settledAt: '2020-01-01T00:00:00Z' })
    ),
    'settled'
  );
  assert.equal(deriveOwnedLibraryState(owned('legacy', '/one', { settledAt: undefined })), 'working');
  assert.equal(deriveOwnedLibraryState(owned('invalid', '/one', { settledAt: '' })), 'working');
}

// Search/filter, grouping and paging remain pure projections.
{
  const rows = buildSessionLibrary(
    [owned('one', '/repo/one'), owned('two', '/repo/two', { completedAt: '2026-01-01T00:00:00Z' })],
    [available('three', '/repo/three')]
  );
  assert.equal(filterSessionLibrary(rows, { worktree: 'two' }).length, 1);
  assert.equal(filterSessionLibrary(rows, { state: 'resumable' }).length, 1);
  assert.equal(paginateSessionLibrary(rows, 2, 2).items.length, 1);
  assert.deepEqual(groupSessionLibrary(rows).map((group) => group.state), [
    'working',
    'resumable',
    'done'
  ]);
}

// Service construction is inert. Refresh/actions call injected effects only
// when explicitly requested, so center/right movement cannot duplicate IO.
{
  let listed = 0;
  const actions = [];
  const service = createSessionLibraryService(
    { listProviderSessions: async () => { listed += 1; return [available('native', '/repo')]; } },
    {
      onOpen: (record) => actions.push(`open:${record.key}`),
      onResume: (record) => actions.push(`resume:${record.key}`)
    }
  );
  assert.equal(listed, 0);
  const rows = await service.refresh();
  assert.equal(listed, 1);
  await service.open(rows[0]);
  await service.resume(rows[0]);
  assert.deepEqual(actions, [`open:${rows[0].key}`, `resume:${rows[0].key}`]);
}

// Center/right placement is one runtime host lease over the same service/store;
// construction and movement never call the provider adapter or create a second
// lease while the first host is live.
{
  let listed = 0;
  const service = createSessionLibraryService({
    listProviderSessions: async () => { listed += 1; return []; }
  });
  const mounts = createSessionLibraryMountCoordinator();
  const center = mounts.mount('center');
  assert.equal(mounts.mountedPlacement, 'center');
  assert.equal(center.active, true);
  assert.throws(() => mounts.mount('right'), /already mounted/);
  const right = mounts.move('right');
  assert.equal(center.active, false);
  assert.equal(right.active, true);
  assert.equal(mounts.mountedPlacement, 'right');
  right.release();
  assert.equal(mounts.mountedPlacement, null);
  assert.equal(listed, 0);
  assert.equal(typeof service.refresh, 'function');
}

console.log('sessionLibrary: all tests passed');
