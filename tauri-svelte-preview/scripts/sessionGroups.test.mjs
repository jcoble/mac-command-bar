import assert from 'node:assert/strict';

import {
  groupSessions,
  groupToggleKey,
  isGroupExpanded,
  RAIL_GROUPS_STORAGE_KEY,
  readGroupExpansion,
  rememberGroupToggle,
  RESUME_GROUP_ROW_CAP,
  sessionGroupPath,
  visibleGroupItems,
  writeGroupExpansion
} from '../src/lib/shell/sessionGroups.ts';

/** An owned-session record with only the fields the grouping reads. */
function owned(title, { projectPath = null, cwd = '' } = {}) {
  return { ownedId: title, title, projectPath, cwd };
}

/** A scanned agent session with only the fields the grouping reads. */
function available(title, { projectPath = null, description = null } = {}) {
  return { provider: 'codex', id: title, title, projectPath, description };
}

/** Just the shape a test cares about: group name plus the titles under it. */
function shapeOf(groups) {
  return groups.map((group) => [group.name, group.items.map((item) => item.title)]);
}

// Sessions are grouped by the LAST folder of their path, and the full path is
// the key — two projects that end in the same folder name stay apart.
{
  const { owned: groups } = groupSessions(
    [
      owned('a', { projectPath: '/Users/me/dev/mac-command-bar' }),
      owned('b', { projectPath: '/Users/me/dev/mac-command-bar' }),
      owned('c', { projectPath: '/Users/me/other/mac-command-bar' })
    ],
    [],
    ''
  );
  assert.deepEqual(shapeOf(groups), [
    ['mac-command-bar', ['a', 'b']],
    ['mac-command-bar', ['c']]
  ]);
  assert.deepEqual(
    groups.map((group) => group.path),
    ['/Users/me/dev/mac-command-bar', '/Users/me/other/mac-command-bar'],
    'the full path is the key, so same-named folders do not merge'
  );
}

// An owned session with no project path falls back to the folder it is
// actually running in, and its project path wins when it has both.
{
  const { owned: groups } = groupSessions(
    [
      owned('worktree', { cwd: '/Users/me/dev/worktrees/fix-a' }),
      owned('also worktree', { cwd: '/Users/me/dev/worktrees/fix-a' }),
      owned('parent', { projectPath: '/Users/me/dev/project', cwd: '/Users/me/dev/worktrees/fix-a' })
    ],
    [],
    ''
  );
  assert.deepEqual(shapeOf(groups), [
    ['fix-a', ['worktree', 'also worktree']],
    ['project', ['parent']]
  ]);
}

// The biggest group comes first, and sessions with no folder at all collect
// under "Other" at the bottom whatever their count.
{
  const { owned: groups } = groupSessions(
    [
      owned('n1'),
      owned('n2'),
      owned('n3'),
      owned('small', { projectPath: '/repo/small' }),
      owned('big1', { projectPath: '/repo/big' }),
      owned('big2', { projectPath: '/repo/big' })
    ],
    [],
    ''
  );
  assert.deepEqual(shapeOf(groups), [
    ['big', ['big1', 'big2']],
    ['small', ['small']],
    ['Other', ['n1', 'n2', 'n3']]
  ]);
  assert.equal(groups.at(-1).path, '', 'the "Other" group keeps an empty path as its key');
}

// Whitespace is not a folder: a path of blanks is treated as no path at all.
{
  const { owned: groups } = groupSessions([owned('blank', { projectPath: '   ' })], [], '');
  assert.deepEqual(shapeOf(groups), [['Other', ['blank']]]);
}

// Items keep the order they arrived in — the stores already sort by recency.
{
  const { owned: groups } = groupSessions(
    [
      owned('third', { projectPath: '/repo/one' }),
      owned('first', { projectPath: '/repo/one' }),
      owned('second', { projectPath: '/repo/one' })
    ],
    [],
    ''
  );
  assert.deepEqual(shapeOf(groups), [['one', ['third', 'first', 'second']]]);
}

// An empty query keeps everything, on both lists.
{
  const result = groupSessions(
    [owned('kept', { projectPath: '/repo/one' })],
    [available('also kept', { projectPath: '/repo/two' })],
    '   '
  );
  assert.deepEqual(shapeOf(result.owned), [['one', ['kept']]]);
  assert.deepEqual(shapeOf(result.available), [['two', ['also kept']]]);
}

// A query matches a session's title, case-insensitively and on part of a word.
{
  const result = groupSessions(
    [
      owned('Fix the scanner', { projectPath: '/repo/one' }),
      owned('Write the docs', { projectPath: '/repo/one' })
    ],
    [],
    'SCAN'
  );
  assert.deepEqual(shapeOf(result.owned), [['one', ['Fix the scanner']]]);
}

// A query also matches the folder path, which is how you narrow to a project.
{
  const result = groupSessions(
    [
      owned('one', { projectPath: '/Users/me/dev/mac-command-bar' }),
      owned('two', { projectPath: '/Users/me/dev/other-thing' })
    ],
    [],
    'command-bar'
  );
  assert.deepEqual(shapeOf(result.owned), [['mac-command-bar', ['one']]]);
}

// For an owned session the cwd is searched too, so a worktree checkout is
// findable by the folder it is actually running in.
{
  const result = groupSessions([owned('worktree run', { cwd: '/Users/me/worktrees/fix-a' })], [], 'fix-a');
  assert.deepEqual(shapeOf(result.owned), [['fix-a', ['worktree run']]]);
}

// A scanned session is searched by its description as well as its title.
{
  const result = groupSessions(
    [],
    [
      available('opaque id', { projectPath: '/repo/one', description: 'reviewing the dock layout' }),
      available('other', { projectPath: '/repo/one', description: 'unrelated' })
    ],
    'dock layout'
  );
  assert.deepEqual(shapeOf(result.available), [['one', ['opaque id']]]);
}

// A group left with no matching items disappears entirely rather than showing
// an empty header.
{
  const result = groupSessions(
    [owned('keep', { projectPath: '/repo/one' }), owned('drop', { projectPath: '/repo/two' })],
    [],
    'keep'
  );
  assert.deepEqual(shapeOf(result.owned), [['one', ['keep']]]);
}

// A query that matches nothing produces no groups at all, on both lists.
{
  const result = groupSessions(
    [owned('one', { projectPath: '/repo/one' })],
    [available('two', { projectPath: '/repo/two' })],
    'nothing here'
  );
  assert.deepEqual(result.owned, []);
  assert.deepEqual(result.available, []);
}

// Two empty lists are not an error.
{
  const result = groupSessions([], [], '');
  assert.deepEqual(result, { owned: [], available: [] });
}

// The group key a session lands under is the same one the expansion rules take.
{
  assert.equal(sessionGroupPath('/Users/me/dev/mac-command-bar'), '/Users/me/dev/mac-command-bar');
  assert.equal(sessionGroupPath('  '), '');
  assert.equal(sessionGroupPath(null), '');
}

// Nothing chosen yet: everything the rail already owns is open, and everything
// offered for resume is closed apart from the project on screen.
{
  const base = { remembered: {}, activeProjectPath: '/repo/one', searching: false };
  assert.equal(isGroupExpanded({ ...base, list: 'owned', path: '/repo/one' }), true);
  assert.equal(isGroupExpanded({ ...base, list: 'owned', path: '/repo/two' }), true);
  assert.equal(isGroupExpanded({ ...base, list: 'resume', path: '/repo/one' }), true);
  assert.equal(isGroupExpanded({ ...base, list: 'resume', path: '/repo/two' }), false);
}

// With no session on screen, every Resume heading starts closed — including the
// one holding sessions with no folder, whose key is the empty string.
{
  const base = { remembered: {}, activeProjectPath: null, searching: false };
  assert.equal(isGroupExpanded({ ...base, list: 'resume', path: '/repo/one' }), false);
  assert.equal(isGroupExpanded({ ...base, list: 'resume', path: '' }), false);
  assert.equal(isGroupExpanded({ ...base, list: 'owned', path: '' }), true);
}

// The user's own choice beats both defaults, in both directions.
{
  const remembered = {
    [groupToggleKey('resume', '/repo/two')]: true,
    [groupToggleKey('owned', '/repo/one')]: false
  };
  const base = { remembered, activeProjectPath: '/repo/one', searching: false };
  assert.equal(isGroupExpanded({ ...base, list: 'resume', path: '/repo/two' }), true);
  assert.equal(isGroupExpanded({ ...base, list: 'owned', path: '/repo/one' }), false);
}

// The two lists remember the same project folder separately.
{
  assert.notEqual(groupToggleKey('owned', '/repo/one'), groupToggleKey('resume', '/repo/one'));
  const remembered = { [groupToggleKey('owned', '/repo/one')]: false };
  const base = { remembered, activeProjectPath: null, searching: false };
  assert.equal(isGroupExpanded({ ...base, list: 'owned', path: '/repo/one' }), false);
  assert.equal(isGroupExpanded({ ...base, list: 'resume', path: '/repo/one' }), false);
}

// A search opens everything: a match hidden under a closed heading would make
// the search box look broken.
{
  const remembered = { [groupToggleKey('resume', '/repo/two')]: false };
  const base = { remembered, activeProjectPath: null, searching: true };
  assert.equal(isGroupExpanded({ ...base, list: 'resume', path: '/repo/two' }), true);
  assert.equal(isGroupExpanded({ ...base, list: 'resume', path: '/repo/three' }), true);
}

// A choice that matches the default is forgotten rather than stored, so the
// remembered set only ever holds headings the user argued with.
{
  assert.deepEqual(rememberGroupToggle({}, 'resume:/repo/one', true, false), {
    'resume:/repo/one': true
  });
  assert.deepEqual(rememberGroupToggle({}, 'resume:/repo/one', false, false), {});
  assert.deepEqual(
    rememberGroupToggle({ 'resume:/repo/one': true }, 'resume:/repo/one', false, false),
    {}
  );

  // And it does not mutate what it was given.
  const before = { 'owned:/repo/one': false };
  assert.deepEqual(rememberGroupToggle(before, 'owned:/repo/one', true, true), {});
  assert.deepEqual(before, { 'owned:/repo/one': false });
}

// An open heading stops after the cap and reports how many it is holding back.
{
  const rows = ['a', 'b', 'c', 'd', 'e'];
  assert.deepEqual(visibleGroupItems(rows, 3, false), {
    shown: ['a', 'b', 'c'],
    hiddenCount: 2
  });
  assert.deepEqual(visibleGroupItems(rows, 3, true), { shown: rows, hiddenCount: 0 });
  assert.deepEqual(visibleGroupItems(rows, 5, false), { shown: rows, hiddenCount: 0 });
  assert.deepEqual(visibleGroupItems(rows, 9, false), { shown: rows, hiddenCount: 0 });
  assert.deepEqual(visibleGroupItems([], 3, false), { shown: [], hiddenCount: 0 });
  assert.deepEqual(visibleGroupItems(rows, 0, false), { shown: rows, hiddenCount: 0 });
  assert.equal(RESUME_GROUP_ROW_CAP, 8);
}

// Storage round-trips, and anything unreadable comes back as no choices made
// rather than as a throw into the rail.
{
  const store = new Map();
  const storage = {
    getItem: (key) => store.get(key) ?? null,
    setItem: (key, value) => store.set(key, value),
    removeItem: (key) => store.delete(key)
  };

  assert.deepEqual(readGroupExpansion(storage), {});
  assert.equal(writeGroupExpansion(storage, { 'resume:/repo/one': true }), true);
  assert.equal(store.get(RAIL_GROUPS_STORAGE_KEY), '{"resume:/repo/one":true}');
  assert.deepEqual(readGroupExpansion(storage), { 'resume:/repo/one': true });

  store.set(RAIL_GROUPS_STORAGE_KEY, 'not json at all');
  assert.deepEqual(readGroupExpansion(storage), {});

  store.set(RAIL_GROUPS_STORAGE_KEY, '["resume:/repo/one"]');
  assert.deepEqual(readGroupExpansion(storage), {});

  // Entries that are not booleans are dropped; the rest still load.
  store.set(RAIL_GROUPS_STORAGE_KEY, '{"a":true,"b":"yes","c":false}');
  assert.deepEqual(readGroupExpansion(storage), { a: true, c: false });

  // A full storage refuses the write and says so, instead of throwing.
  const fullStorage = {
    getItem: () => null,
    setItem: () => {
      throw new Error('QuotaExceededError');
    },
    removeItem: () => {}
  };
  assert.equal(writeGroupExpansion(fullStorage, { a: true }), false);
}

console.log('sessionGroups: all tests passed');
