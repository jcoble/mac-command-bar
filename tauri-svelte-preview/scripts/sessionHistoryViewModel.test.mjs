import assert from 'node:assert/strict';

import {
  buildSessionHistoryViewModel,
  createSessionHistoryCollapseState,
  isSessionHistoryGroupOpen,
  toggleSessionHistoryGroup,
  visibleSessionHistoryRows
} from '../src/lib/shell/history/sessionHistoryViewModel.ts';

function record(key, input = {}) {
  return {
    key,
    source: 'provider',
    ownedId: null,
    provider: 'alpha',
    nativeSessionId: key,
    canonicalCwd: '/Users/dev/work/atlas',
    title: `Session ${key}`,
    description: null,
    projectPath: '/Users/dev/work/atlas',
    model: null,
    state: 'resumable',
    runtimeState: null,
    lastActivity: null,
    updatedAt: null,
    messageCount: null,
    firstPrompt: null,
    latestTurns: [],
    owned: null,
    available: null,
    ...input
  };
}

const rows = [
  record('atlas-main', {
    title: 'Review the navigation',
    updatedAt: '2026-08-08T12:00:00Z',
    messageCount: 4
  }),
  record('atlas-lane-old', {
    canonicalCwd: '/Users/dev/work/worktrees/atlas/feature-one',
    projectPath: '/Users/dev/work/worktrees/atlas/feature-one',
    title: 'Investigate the cache',
    firstPrompt: 'Investigate cache invalidation in the editor',
    updatedAt: '2026-08-09T12:00:00Z',
    messageCount: 8
  }),
  record('atlas-lane-new', {
    canonicalCwd: '/Users/dev/work/worktrees/atlas/feature-one',
    projectPath: '/Users/dev/work/worktrees/atlas/feature-one',
    title: 'Finish the cache repair',
    updatedAt: '2026-08-10T12:00:00Z',
    messageCount: 11
  }),
  record('beacon', {
    provider: 'beta',
    canonicalCwd: '/Users/dev/work/beacon',
    projectPath: '/Users/dev/work/beacon',
    title: '',
    nativeSessionId: 'beacon',
    firstPrompt: 'Trace the first visible message',
    updatedAt: '2026-08-07T12:00:00Z',
    messageCount: 2
  }),
  record('cedar-live', {
    source: 'owned',
    ownedId: 'cedar-live',
    provider: 'beta',
    canonicalCwd: '/Users/dev/work/cedar',
    projectPath: '/Users/dev/work/cedar',
    title: 'Keep the live session visible',
    state: 'working',
    runtimeState: 'working',
    updatedAt: '2026-08-06T12:00:00Z',
    owned: { state: 'live' }
  })
];

// Projects and worktrees are counted from the filtered rows. Shared-root
// worktrees resolve back to their repository, and activity orders every level.
{
  const view = buildSessionHistoryViewModel(rows);
  assert.equal(view.totalCount, 5);
  assert.deepEqual(view.projects.map((project) => [project.name, project.count]), [
    ['atlas', 3],
    ['beacon', 1],
    ['cedar', 1]
  ]);

  const atlas = view.projects[0];
  assert.equal(atlas.singleCheckout, false);
  assert.deepEqual(atlas.worktrees.map((worktree) => [worktree.name, worktree.count]), [
    ['feature-one', 2],
    ['atlas', 1]
  ]);
  assert.deepEqual(atlas.worktrees[0].rows.map((row) => row.record.key), [
    'atlas-lane-new',
    'atlas-lane-old'
  ]);

  const beacon = view.projects.find((project) => project.name === 'beacon');
  assert.equal(beacon.singleCheckout, true);
  assert.equal(beacon.worktrees[0].rows[0].displayTitle, 'Trace the first visible message');
  assert.equal(view.projects.find((project) => project.name === 'cedar').worktrees[0].rows[0].statusHint, 'Live');
}

// Search covers the displayed content plus project and worktree identity.
{
  assert.equal(buildSessionHistoryViewModel(rows, { query: 'cache invalidation' }).totalCount, 1);
  assert.equal(buildSessionHistoryViewModel(rows, { query: 'feature-one' }).totalCount, 2);
  assert.equal(buildSessionHistoryViewModel(rows, { query: 'beacon' }).totalCount, 1);
  assert.equal(buildSessionHistoryViewModel(rows, { query: 'atlas' }).totalCount, 3);
}

// Provider filtering is exact, keeps a stable provider roster, and recomputes
// every project/worktree badge from only the rows that remain.
{
  const view = buildSessionHistoryViewModel(rows, { provider: 'beta' });
  assert.deepEqual(view.providers.map((provider) => provider.value), ['alpha', 'beta']);
  assert.equal(view.totalCount, 2);
  assert.deepEqual(view.projects.map((project) => [project.name, project.count]), [
    ['beacon', 1],
    ['cedar', 1]
  ]);
}

// Collapse state is immutable and applies independently at project and
// worktree level. A collapsed project hides all of its worktrees.
{
  const view = buildSessionHistoryViewModel(rows);
  const atlas = view.projects.find((project) => project.name === 'atlas');
  const feature = atlas.worktrees.find((worktree) => worktree.name === 'feature-one');
  const initial = createSessionHistoryCollapseState();
  const worktreeCollapsed = toggleSessionHistoryGroup(initial, 'worktree', feature.key);
  assert.equal(isSessionHistoryGroupOpen(initial, 'worktree', feature.key), true);
  assert.equal(isSessionHistoryGroupOpen(worktreeCollapsed, 'worktree', feature.key), false);
  assert.equal(visibleSessionHistoryRows(view.projects, worktreeCollapsed).length, 3);

  const projectCollapsed = toggleSessionHistoryGroup(initial, 'project', atlas.key);
  assert.equal(isSessionHistoryGroupOpen(projectCollapsed, 'project', atlas.key), false);
  assert.equal(visibleSessionHistoryRows(view.projects, projectCollapsed).length, 2);
  assert.equal(
    isSessionHistoryGroupOpen(toggleSessionHistoryGroup(projectCollapsed, 'project', atlas.key), 'project', atlas.key),
    true
  );
}

console.log('sessionHistoryViewModel: all tests passed');
