import './svelteRuneTestSetup.ts';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import ts from 'typescript';

import {
  buildMyWorkGroups,
  DEFAULT_MY_WORK_VIEW_OPTIONS,
  matchesMyWorkFilters,
  myWorkProject,
  normalizeMyWorkFilters,
  normalizeMyWorkViewOptions,
  prepareMyWorkSessions,
  reorderMyWorkSessions,
  type MyWorkGroup
} from '../src/lib/shell/components/myWorkViewOptions.ts';
import { sessionRowMenuItems } from '../src/lib/shell/components/sessionRowMenu.ts';
import type { OwnedSession } from '../src/lib/shell/ownedSessions.ts';
import { ownedSessionMetaForBackend } from '../src/lib/shell/ownedSessions.ts';
import { hydrateOwned, rail, setOwnedSessionStatus, updateOwnedSession } from '../src/lib/shell/stores/sessionRailStore.svelte.ts';

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

// buildMyWorkGroups puts Working/Done/Settled outside and splits each by project
// when both groupings are on, so marking a session done moves it down to Done.
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
    ['working', 'Working', ['z-working', 'a-working'], [
      ['working::/two/alpha', 'alpha', ['a-working']],
      ['working::/one/zeta', 'zeta', ['z-working']]
    ]],
    ['done', 'Done', ['a-done'], [
      ['done::/two/alpha', 'alpha', ['a-done']]
    ]],
    ['settled', 'Settled', ['a-settled'], [
      ['settled::/two/alpha', 'alpha', ['a-settled']]
    ]]
  ]);
}

// Pinned sessions leave their own section for a Pinned section at the top,
// whatever the grouping and whatever their status, until they are unpinned.
{
  const rows = [
    session('working', { title: 'a' }),
    session('pinned-done', { title: 'b', completedAt: '2026-08-09T10:00:00.000Z', pinnedAt: '2026-10-07T10:00:00.000Z' }),
    session('pinned-working', { title: 'c', pinnedAt: '2026-10-07T11:00:00.000Z' })
  ];
  const byStatus = buildMyWorkGroups(rows, { ...DEFAULT_MY_WORK_VIEW_OPTIONS, sortBy: 'name', sortDirection: 'asc' });
  assert.deepEqual(shape(byStatus), [
    ['pinned', 'Pinned', ['pinned-done', 'pinned-working']],
    ['working', 'Working', ['working']]
  ]);
  const ungrouped = buildMyWorkGroups(rows, {
    ...DEFAULT_MY_WORK_VIEW_OPTIONS, sortBy: 'name', sortDirection: 'asc', groupByStatus: false
  });
  assert.deepEqual(shape(ungrouped), [
    ['pinned', 'Pinned', ['pinned-done', 'pinned-working']],
    ['all', '', ['working']]
  ]);
}

// The row menu offers Pin to top, or Unpin once pinned.
{
  const pinItem = (pinned: boolean) => sessionRowMenuItems({ status: 'working', sessionId: 'x', worktreePath: null, pinned })
    .find((item) => item.id === 'pin' || item.id === 'unpin');
  assert.deepEqual([pinItem(false)?.id, pinItem(false)?.label, pinItem(false)?.enabled], ['pin', 'Pin to top', true]);
  assert.deepEqual([pinItem(true)?.id, pinItem(true)?.label, pinItem(true)?.enabled], ['unpin', 'Unpin', true]);
}

// Rename is a working item on the row menu.
{
  const rename = sessionRowMenuItems({ status: 'done', sessionId: null, worktreePath: null, pinned: false })
    .find((item) => item.id === 'rename');
  assert.deepEqual([rename?.label, rename?.enabled, rename?.disabledReason], ['Rename', true, undefined]);
}

// Custom order follows the saved order inside each group; a session with no
// saved place yet goes to the top, newest first.
{
  const rows = [
    session('a', { lastActivity: '2026-08-01T10:00:00.000Z' }),
    session('b', { lastActivity: '2026-08-02T10:00:00.000Z' }),
    session('c', { lastActivity: '2026-08-03T10:00:00.000Z', completedAt: '2026-08-03T10:00:00.000Z' }),
    session('new-old', { lastActivity: '2026-08-04T10:00:00.000Z' }),
    session('new-new', { lastActivity: '2026-08-05T10:00:00.000Z' })
  ];
  const groups = buildMyWorkGroups(rows, { ...DEFAULT_MY_WORK_VIEW_OPTIONS, sortBy: 'manual' }, ['c', 'a', 'b']);
  assert.deepEqual(shape(groups), [
    ['working', 'Working', ['new-new', 'new-old', 'a', 'b']],
    ['done', 'Done', ['c']]
  ]);
  assert.equal(normalizeMyWorkViewOptions({ sortBy: 'manual' }).sortBy, 'manual');
}

// Moving a row reorders its own group and keeps every other saved place.
{
  assert.deepEqual(reorderMyWorkSessions(['x', 'a', 'y'], ['a', 'b', 'c'], 'c', 'a', 'before'), ['c', 'a', 'b', 'x', 'y']);
  assert.deepEqual(reorderMyWorkSessions([], ['a', 'b', 'c'], 'a', 'c', 'after'), ['b', 'c', 'a']);
  assert.deepEqual(reorderMyWorkSessions([], ['a', 'b', 'c'], 'a', 'a', 'after'), ['a', 'b', 'c']);
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

// Run the route's status callbacks through the real rail projection and filter.
{
  const route = process.env.STATUS_TEST_BASE
    ? execFileSync('git', ['show', `${process.env.STATUS_TEST_BASE}:tauri-svelte-preview/src/routes/+page.svelte`], { encoding: 'utf8' })
    : readFileSync(new URL('../src/routes/+page.svelte', import.meta.url), 'utf8');
  const ast = parse(route, { modern: true });
  const action = ast.instance?.content.body.find((node) => node.type === 'FunctionDeclaration'
    && node.id?.name === 'changeSessionStatus');
  assert.ok(action, 'route status action exists');
  const saves: Array<{ ownedId: string; completedAt: string | null; settledAt: string | null }> = [];
  const actionCode = ts.transpileModule(route.slice(action.start, action.end), {
    compilerOptions: { target: ts.ScriptTarget.ES2022 }
  }).outputText;
  const changeSessionStatus = new Function('selection', 'setOwnedSessionStatus',
    'updateOwnedSession', 'ownedSessionMetaForBackend', 'updateAgentConversationSessionMetaFromTauri',
    `${actionCode}; return changeSessionStatus;`)(
      { get railOwned() { return rail.owned; } }, setOwnedSessionStatus, updateOwnedSession,
      ownedSessionMetaForBackend,
      async ({ ownedId, meta }: { ownedId: string; meta: { completedAt: string | null; settledAt: string | null } }) => {
        saves.push({ ownedId, completedAt: meta.completedAt, settledAt: meta.settledAt });
      }
    ) as (ownedId: string, status: string) => Promise<void>;
  let column: { attributes: Array<{ name?: string; value?: { expression?: { start: number; end: number } } }> } | undefined;
  function findColumn(node: unknown): void {
    if (!node || typeof node !== 'object') return;
    const current = node as { type?: string; name?: string; [key: string]: unknown };
    if (current.type === 'Component' && current.name === 'SessionsColumn') column = current as typeof column;
    for (const value of Object.values(current)) {
      if (Array.isArray(value)) value.forEach(findColumn);
      else if (value && typeof value === 'object') findColumn(value);
    }
  }
  findColumn(ast.fragment);
  assert.ok(column, 'route renders the sessions column');
  const invoke = async (name: string, ownedId: string) => {
    const expression = column.attributes.find((attribute) => attribute.name === name)?.value?.expression;
    assert.ok(expression, `${name} status callback exists`);
    const callback = new Function('changeSessionStatus', `return (${route.slice(expression.start, expression.end)});`)(
      changeSessionStatus
    ) as (id: string) => void;
    callback(ownedId);
    await Promise.resolve();
  };
  hydrateOwned([session('working'), session('done'), session('settled')]);
  await invoke('onComplete', 'done');
  await invoke('onSettle', 'settled');
  const kept = (status: string[]) => rail.owned
    .filter((row) => matchesMyWorkFilters(row, { status }))
    .map((row) => row.ownedId);
  assert.deepEqual(kept(['working']), ['working']);
  assert.deepEqual(kept(['done']), ['done']);
  assert.deepEqual(kept(['settled']), ['settled']);
  assert.deepEqual(kept(['done', 'settled']), ['done', 'settled']);
  assert.deepEqual(saves.map(({ ownedId, completedAt, settledAt }) => [ownedId, !!completedAt, !!settledAt]), [
    ['done', true, false], ['settled', true, true]
  ]);
  await invoke('onUnsettle', 'settled');
  assert.deepEqual(kept(['done']), ['done', 'settled']);
  await invoke('onReopen', 'done');
  assert.deepEqual(kept(['working']), ['working', 'done']);
  assert.deepEqual(saves.map(({ ownedId, completedAt, settledAt }) => [ownedId, !!completedAt, !!settledAt]), [
    ['done', true, false], ['settled', true, true], ['settled', true, false], ['done', false, false]
  ]);
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
