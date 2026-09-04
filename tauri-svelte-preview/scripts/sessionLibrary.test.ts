import assert from 'node:assert/strict';

import {
  buildSessionLibrary,
  canonicalCwd,
  deriveOwnedLibraryState,
  filterSessionLibrary,
  filterSessionHistory,
  groupSessionHistory,
  groupSessionLibrary,
  sessionIdentityKey,
  toggleSessionLibraryExpansion
} from '../src/lib/shell/sessionLibrary/sessionLibraryModel.ts';
import { sessionContextMenuRoster } from '../src/lib/shell/sessionLibrary/sessionLibraryContextMenu.ts';
import {
  createSessionLibraryMountCoordinator,
  createSessionLibraryService,
  registerSessionLibraryHost,
  sessionLibraryHost
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
  assert.deepEqual(groupSessionLibrary(rows).map((group) => group.state), [
    'working',
    'resumable',
    'done'
  ]);

  // The history view groups by project, while its segmented scopes narrow by
  // exact canonical workspace/project paths before the text search runs.
  assert.deepEqual(groupSessionHistory(rows).map((group) => [group.name, group.items.length]), [
    ['one', 1],
    ['two', 1],
    ['three', 1]
  ]);
  assert.equal(filterSessionHistory(rows, { scope: 'workspace', workspacePath: '/repo/one' }).length, 1);
  assert.equal(filterSessionHistory(rows, { scope: 'project', projectPath: '/repo/two' }).length, 1);
  assert.equal(filterSessionHistory(rows, { scope: 'all', query: 'three' }).length, 1);
  assert.equal(toggleSessionLibraryExpansion(null, rows[0].key), rows[0].key);
  assert.equal(toggleSessionLibraryExpansion(rows[0].key, rows[0].key), null);
}

// Every context action stays visible. The current rail supports resume/archive/
// delete for owned rows, while missing native actions remain explicitly disabled.
{
  const row = buildSessionLibrary([owned('menu', '/repo/menu')], [])[0];
  const roster = sessionContextMenuRoster({ target: 'session', record: row });
  assert.deepEqual(roster.map((item) => item.label), [
    'Resume in Worktree',
    'Continue in New Session',
    'View Log',
    'Copy ID',
    'Archive',
    'Delete'
  ]);
  assert.equal(roster.find((item) => item.id === 'resume-worktree').enabled, true);
  assert.equal(roster.find((item) => item.id === 'continue-new-session').enabled, false);
  assert.match(roster.find((item) => item.id === 'view-log').disabledReason, /Future-native/);
  assert.equal(sessionContextMenuRoster({ target: 'center-tab' }).find((item) => item.id === 'copy-id').enabled, true);
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

// History holds only the rows belonging to projects the reader opened. Closing
// one project releases its rows; closing the panel releases every held row.
{
  const first = available('first', '/repo/first', { projectRoot: '/repo' });
  const firstSibling = available('first-sibling', '/repo/second', { projectRoot: '/repo' });
  const second = available('second', '/other/second', { projectRoot: '/other' });
  const allRows = buildSessionLibrary([], [first, firstSibling, second]);
  const firstKeys = new Set(allRows.slice(0, 2).map((row) => row.key));
  const secondKeys = new Set([allRows[2].key]);
  const service = createSessionLibraryService({
    listProviderSessions: async () => [first, firstSibling, second]
  });

  assert.equal(service.records.length, 0);
  await service.refresh(firstKeys);
  assert.deepEqual(service.records.map((row) => row.key), [...firstKeys]);
  await service.refresh(secondKeys);
  assert.deepEqual(new Set(service.records.map((row) => row.key)), new Set([...firstKeys, ...secondKeys]));
  service.release(firstKeys);
  assert.deepEqual(service.records.map((row) => row.key), [...secondKeys]);
  service.release();
  assert.equal(service.records.length, 0);
}

// Opening a project reuses the summary records already in the rail. Opening one
// card reads only that transcript for details instead of rescanning the project.
{
  const cached = available('cached', '/repo', {
    logPath: '/logs/cached.jsonl',
    latestTurns: undefined
  });
  let projectScans = 0;
  let detailReads = 0;
  const service = createSessionLibraryService({
    getAvailableSessions: () => [cached],
    listProviderSessions: async () => { projectScans += 1; return []; },
    readProviderSessionDetails: async () => {
      detailReads += 1;
      return [available('cached', '/repo', {
        logPath: '/logs/cached.jsonl',
        latestTurns: [{ speaker: 'agent', text: 'Finished.' }]
      })];
    }
  });
  const key = buildSessionLibrary([], [cached])[0].key;
  assert.equal((await service.refresh(new Set([key]))).length, 1);
  assert.equal(projectScans, 0);
  const details = await service.refresh(new Set([key]), { projectPath: '/repo' }, { includeDetails: true });
  assert.equal(details[0].latestTurns[0].text, 'Finished.');
  assert.equal(detailReads, 1);
  assert.equal(projectScans, 0);
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

// A panel can be constructed before the route registers its live host. The
// captured handle must still begin forwarding once registration lands.
{
  const captured = sessionLibraryHost();
  const live = createSessionLibraryService({
    listProviderSessions: async () => [available('late-host', '/repo')]
  });
  const release = registerSessionLibraryHost({ service: live });
  assert.equal((await captured.service.refresh()).length, 1);
  release();
  assert.equal((await captured.service.refresh()).length, 0);
}

console.log('sessionLibrary: all tests passed');
