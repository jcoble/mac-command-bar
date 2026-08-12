import assert from 'node:assert/strict';

import {
  buildMyWorkGroups,
  DEFAULT_MY_WORK_VIEW_OPTIONS,
  MY_WORK_VIEW_OPTIONS_KEY,
  myWorkProject,
  normalizeMyWorkViewOptions,
  prepareMyWorkSessions,
  readMyWorkViewOptions,
  writeMyWorkViewOptions
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

function storage() {
  const rows = new Map();
  return {
    rows,
    getItem(key) { return rows.get(key) ?? null; },
    setItem(key, value) { rows.set(key, value); },
    removeItem(key) { rows.delete(key); }
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

// Persistence is tolerant of corrupt fields, saves one normalized payload, and
// preserves an intentional empty filter set.
{
  const fake = storage();
  fake.setItem(MY_WORK_VIEW_OPTIONS_KEY, JSON.stringify({ groupBy: 'wrong', sortBy: 'name' }));
  assert.deepEqual(readMyWorkViewOptions(fake), {
    groupBy: 'status',
    sortBy: 'name',
    visibleStatuses: ['working', 'done', 'settled']
  });
  assert.equal(
    writeMyWorkViewOptions(fake, { groupBy: 'project', sortBy: 'recent', visibleStatuses: [] }),
    true
  );
  assert.deepEqual(readMyWorkViewOptions(fake), {
    groupBy: 'project',
    sortBy: 'recent',
    visibleStatuses: []
  });
  assert.deepEqual(normalizeMyWorkViewOptions(null), DEFAULT_MY_WORK_VIEW_OPTIONS);
}

console.log('myWorkViewOptions: grouping, sorting, filters, project identity, and persistence passed');
