import assert from 'node:assert/strict';

import {
  buildMyWorkGroups,
  DEFAULT_MY_WORK_VIEW_OPTIONS,
  myWorkProject,
  normalizeMyWorkViewOptions,
  prepareMyWorkSessions
} from '../src/lib/shell/components/myWorkViewOptions.ts';

function session(ownedId, extra = {}) {
  return {
    ownedId,
    title: ownedId,
    projectPath: '/work/mac-command-bar',
    cwd: '/work/mac-command-bar',
    completedAt: null,
    settledAt: null,
    lastActivity: null,
    ...extra
  };
}

// Project identity comes from the workspace root basename, preferring the
// explicit project path and tolerating a trailing slash.
{
  assert.deepEqual(
    myWorkProject(session('one', { projectPath: '/work/projects/Assembly/', cwd: '/wrong' })),
    { key: '/work/projects/Assembly', label: 'Assembly' }
  );
  assert.deepEqual(
    myWorkProject(session('two', { projectPath: null, cwd: '/work/fallback' })),
    { key: '/work/fallback', label: 'fallback' }
  );
  assert.deepEqual(
    myWorkProject(session('three', { projectPath: 'No Project recorded', cwd: ' ', agent: 'codex', viaCmux: false })),
    { key: 'provider:codex', label: 'codex' }
  );
}

// Recent activity uses the transcript stamp first, then lifecycle stamps, and
// keeps incoming order when no reliable clock exists. Name sorting is case-insensitive.
{
  const rows = [
    session('missing-a', { title: 'Zulu' }),
    session('older', { lastActivity: '2026-08-08T10:00:00.000Z', title: 'beta' }),
    session('newer', { completedAt: '2026-08-09T10:00:00.000Z', title: 'Alpha' }),
    session('missing-b', { title: 'gamma' })
  ];
  assert.deepEqual(
    prepareMyWorkSessions(rows, DEFAULT_MY_WORK_VIEW_OPTIONS).map((row) => row.ownedId),
    ['newer', 'older', 'missing-a', 'missing-b']
  );
  assert.deepEqual(
    prepareMyWorkSessions(rows, {
      sortBy: 'name',
      visibleStatuses: ['working', 'done', 'settled']
    }).map((row) => row.ownedId),
    ['newer', 'older', 'missing-b', 'missing-a']
  );
}

// Status filtering happens before grouping. Project sections use the full path
// as identity, so two same-named folders do not collapse into one section.
{
  const groups = buildMyWorkGroups(
    [
      session('working', { projectPath: '/one/shared' }),
      session('done', { projectPath: '/two/shared', completedAt: '2026-08-09T10:00:00.000Z' }),
      session('settled', {
        projectPath: '/one/shared',
        completedAt: '2026-08-08T10:00:00.000Z',
        settledAt: '2026-08-09T12:00:00.000Z'
      })
    ],
    { groupBy: 'project', sortBy: 'recent', visibleStatuses: ['working', 'settled'] }
  );
  assert.deepEqual(
    groups.map((group) => [group.key, group.label, group.sessions.map((row) => row.ownedId)]),
    [['/one/shared', 'shared', ['settled', 'working']]]
  );
}

// Status groups have a stable Working / Done / Settled order and carry counts
// through their session arrays.
{
  const groups = buildMyWorkGroups(
    [
      session('settled', { settledAt: '2026-08-09T12:00:00.000Z' }),
      session('working'),
      session('done', { completedAt: '2026-08-09T10:00:00.000Z' })
    ],
    { groupBy: 'status', sortBy: 'name', visibleStatuses: ['working', 'done', 'settled'] }
  );
  assert.deepEqual(groups.map((group) => [group.label, group.sessions.length]), [
    ['Working', 1],
    ['Done', 1],
    ['Settled', 1]
  ]);
}

// Normalization tolerates corrupt fields and preserves an intentional empty filter set.
{
  assert.deepEqual(normalizeMyWorkViewOptions({ groupBy: 'wrong', sortBy: 'name' }), {
    groupBy: 'status',
    sortBy: 'name',
    sortDirection: 'asc',
    visibleStatuses: ['working', 'done', 'settled']
  });
  assert.deepEqual(normalizeMyWorkViewOptions({
    groupBy: 'project',
    sortBy: 'recent',
    sortDirection: 'desc',
    visibleStatuses: []
  }), {
    groupBy: 'project',
    sortBy: 'recent',
    sortDirection: 'desc',
    visibleStatuses: []
  });
  assert.deepEqual(normalizeMyWorkViewOptions(null), DEFAULT_MY_WORK_VIEW_OPTIONS);
}

console.log('myWorkViewOptions: grouping, sorting, filters, project identity, and normalization passed');
