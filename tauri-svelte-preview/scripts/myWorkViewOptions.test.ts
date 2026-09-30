import assert from 'node:assert/strict';

import {
  buildMyWorkGroups,
  DEFAULT_MY_WORK_VIEW_OPTIONS,
  matchesMyWorkFilters,
  myWorkProject,
  normalizeMyWorkFilters,
  normalizeMyWorkViewOptions,
  prepareMyWorkSessions,
  type MyWorkGroup
} from '../src/lib/shell/components/myWorkViewOptions.ts';
import type { OwnedSession } from '../src/lib/shell/ownedSessions.ts';

function session(ownedId: string, extra: Record<string, unknown> = {}): OwnedSession {
  return {
    ownedId,
    title: ownedId,
    projectPath: '/work/mac-command-bar',
    cwd: '/work/mac-command-bar',
    agent: 'claude',
    executionEnvironment: 'local',
    completedAt: null,
    settledAt: null,
    lastActivity: null,
    ...extra
  } as unknown as OwnedSession;
}

function shape(groups: MyWorkGroup[]): unknown[] {
  return groups.map((group) => [
    group.key,
    group.label,
    group.sessions.map((row) => row.ownedId),
    ...(group.subgroups ? [shape(group.subgroups)] : [])
  ]);
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
    prepareMyWorkSessions(rows, { sortBy: 'name' }).map((row) => row.ownedId),
    ['newer', 'older', 'missing-b', 'missing-a']
  );
}

// Project sections use the full path as identity, so two same-named folders
// do not collapse into one section.
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
    { ...DEFAULT_MY_WORK_VIEW_OPTIONS, groupByProject: true, groupByStatus: false }
  );
  assert.deepEqual(shape(groups), [
    ['/one/shared', 'shared', ['settled', 'working']],
    ['/two/shared', 'shared', ['done']]
  ]);
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
    { ...DEFAULT_MY_WORK_VIEW_OPTIONS, sortBy: 'name', groupByProject: false, groupByStatus: true }
  );
  assert.deepEqual(groups.map((group) => [group.label, group.sessions.length]), [
    ['Working', 1],
    ['Done', 1],
    ['Settled', 1]
  ]);
}

// buildMyWorkGroups nests Working/Done/Settled under each project when both groupings are on.
{
  const groups = buildMyWorkGroups(
    [
      session('z-working', { projectPath: '/one/zeta', title: 'a' }),
      session('a-settled', { projectPath: '/two/alpha', title: 'b', settledAt: '2026-08-09T12:00:00.000Z' }),
      session('a-working', { projectPath: '/two/alpha', title: 'c' }),
      session('a-done', { projectPath: '/two/alpha', title: 'd', completedAt: '2026-08-09T10:00:00.000Z' })
    ],
    { ...DEFAULT_MY_WORK_VIEW_OPTIONS, sortBy: 'name', sortDirection: 'asc', groupByProject: true, groupByStatus: true }
  );
  assert.deepEqual(shape(groups), [
    ['/two/alpha', 'alpha', ['a-settled', 'a-working', 'a-done'], [
      ['/two/alpha::working', 'Working', ['a-working']],
      ['/two/alpha::done', 'Done', ['a-done']],
      ['/two/alpha::settled', 'Settled', ['a-settled']]
    ]],
    ['/one/zeta', 'zeta', ['z-working'], [
      ['/one/zeta::working', 'Working', ['z-working']]
    ]]
  ]);
}

// buildMyWorkGroups with neither grouping returns one unlabeled group.
{
  const groups = buildMyWorkGroups(
    [session('one'), session('two', { settledAt: '2026-08-09T12:00:00.000Z' })],
    { ...DEFAULT_MY_WORK_VIEW_OPTIONS, groupByProject: false, groupByStatus: false }
  );
  assert.deepEqual(shape(groups), [['all', '', ['two', 'one']]]);
}

// normalizeMyWorkViewOptions loads an old single-value groupBy.
{
  const grouping = (value: unknown) => {
    const options = normalizeMyWorkViewOptions(value);
    return [options.groupByProject, options.groupByStatus];
  };
  assert.deepEqual(grouping({ groupBy: 'project' }), [true, false]);
  assert.deepEqual(grouping({ groupBy: 'status' }), [false, true]);
  assert.deepEqual(grouping({ groupBy: 'none' }), [false, false]);
  assert.deepEqual(grouping({ groupBy: 'wrong' }), [false, true]);
  assert.deepEqual(grouping({}), [false, true]);
  assert.deepEqual(grouping({ groupBy: 'status', groupByProject: true, groupByStatus: false }), [true, false]);
  assert.deepEqual(normalizeMyWorkViewOptions({ groupBy: 'wrong', sortBy: 'name' }), {
    groupByProject: false,
    groupByStatus: true,
    sortBy: 'name',
    sortDirection: 'asc'
  });
  assert.deepEqual(normalizeMyWorkViewOptions(null), DEFAULT_MY_WORK_VIEW_OPTIONS);
}

// normalizeMyWorkViewOptions drops an old visibleStatuses.
{
  const options = normalizeMyWorkViewOptions({ groupBy: 'project', sortBy: 'recent', visibleStatuses: [] });
  assert.equal('visibleStatuses' in options, false);
  const rows = [
    session('working'),
    session('done', { completedAt: '2026-08-09T10:00:00.000Z' }),
    session('settled', { settledAt: '2026-08-09T12:00:00.000Z' })
  ];
  assert.deepEqual(
    prepareMyWorkSessions(rows, { ...options, sortBy: 'name', sortDirection: 'asc' }).map((row) => row.ownedId),
    ['done', 'settled', 'working']
  );
}

// matchesMyWorkFilters: OR inside a group, AND across groups, empty group = no filter.
{
  const rows = [
    session('claude-local-working', { agent: 'claude' }),
    session('codex-remote-done', { agent: 'codex', executionEnvironment: 'remote', completedAt: '2026-08-09T10:00:00.000Z' }),
    session('agy-local-settled', { agent: 'antigravity', settledAt: '2026-08-09T12:00:00.000Z' })
  ];
  const kept = (filters: Record<string, string[]>) =>
    rows.filter((row) => matchesMyWorkFilters(row, filters)).map((row) => row.ownedId);
  assert.deepEqual(kept({}), ['claude-local-working', 'codex-remote-done', 'agy-local-settled']);
  assert.deepEqual(kept({ provider: [], status: [], location: [] }), [
    'claude-local-working',
    'codex-remote-done',
    'agy-local-settled'
  ]);
  assert.deepEqual(kept({ provider: ['claude', 'codex'] }), ['claude-local-working', 'codex-remote-done']);
  assert.deepEqual(kept({ provider: ['claude', 'codex'], location: ['local'] }), ['claude-local-working']);
  assert.deepEqual(kept({ status: ['done', 'settled'] }), ['codex-remote-done', 'agy-local-settled']);
  assert.deepEqual(kept({ provider: ['antigravity'], status: ['working'] }), []);
}

// normalizeMyWorkFilters keeps only known group ids and option values.
{
  assert.deepEqual(normalizeMyWorkFilters(null), { provider: [], status: [], location: [] });
  assert.deepEqual(normalizeMyWorkFilters('junk'), { provider: [], status: [], location: [] });
  assert.deepEqual(
    normalizeMyWorkFilters({
      provider: ['codex', 'bogus', 'claude'],
      status: 'working',
      location: ['remote'],
      extra: ['x']
    }),
    { provider: ['claude', 'codex'], status: [], location: ['remote'] }
  );
}

console.log('myWorkViewOptions: grouping, sorting, filters, project identity, and normalization passed');
