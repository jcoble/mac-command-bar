import assert from 'node:assert/strict';

import { groupSessions } from '../src/lib/shell/sessionGroups.ts';

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

console.log('sessionGroups: all tests passed');
