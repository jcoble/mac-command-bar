import assert from 'node:assert/strict';

import {
  SESSIONS_COLLAPSED_KEY,
  readSessionsCollapsed,
  sessionLabel,
  splitOwnedSessions,
  stripCells,
  writeSessionsCollapsed
} from '../src/lib/shell/sessionStrip.ts';

/** An owned session with only the fields these functions read. */
function owned(ownedId, extra = {}) {
  return {
    ownedId,
    title: ownedId,
    agent: 'claude',
    viaCmux: false,
    state: 'live',
    completedAt: null,
    ...extra
  };
}

/** A localStorage stand-in. `refuse` makes every write throw, the way a full
 * storage does. */
function fakeStorage({ refuse = false, unreadable = false } = {}) {
  const data = new Map();
  return {
    data,
    getItem(key) {
      if (unreadable) throw new Error('storage is unavailable');
      return data.has(key) ? data.get(key) : null;
    },
    setItem(key, value) {
      if (refuse) throw new Error('quota exceeded');
      data.set(key, value);
    },
    removeItem(key) {
      data.delete(key);
    }
  };
}

// ── Working and Done ──────────────────────────────────────────────────────

// Which side a session sits on is the user's answer (`completedAt`) and
// nothing else: a session whose process has ended is still work in progress
// until it is marked done.
{
  const sessions = [
    owned('a'),
    owned('b', { state: 'exited' }),
    owned('c', { completedAt: '2026-07-29T10:00:00.000Z' })
  ];
  const { working, done } = splitOwnedSessions(sessions);
  assert.deepEqual(
    working.map((session) => session.ownedId),
    ['a', 'b'],
    'a finished process is still working until the user says otherwise'
  );
  assert.deepEqual(
    done.map((session) => session.ownedId),
    ['c']
  );
}

// Done reads most recently finished first; Working keeps the order it arrived
// in, which is the order the store already put the sessions in.
{
  const sessions = [
    owned('older', { completedAt: '2026-07-27T09:00:00.000Z' }),
    owned('first-working'),
    owned('newest', { completedAt: '2026-07-29T09:00:00.000Z' }),
    owned('second-working'),
    owned('middle', { completedAt: '2026-07-28T09:00:00.000Z' })
  ];
  const { working, done } = splitOwnedSessions(sessions);
  assert.deepEqual(
    working.map((session) => session.ownedId),
    ['first-working', 'second-working']
  );
  assert.deepEqual(
    done.map((session) => session.ownedId),
    ['newest', 'middle', 'older']
  );
  assert.deepEqual(
    sessions.map((session) => session.ownedId),
    ['older', 'first-working', 'newest', 'second-working', 'middle'],
    'the list handed in is left exactly as it was'
  );
}

// ── What a row is called ──────────────────────────────────────────────────

{
  assert.equal(sessionLabel(owned('x', { title: 'Fix the scanner' })), 'Fix the scanner');
  assert.equal(
    sessionLabel(owned('0f9c1d2e-7a4b-4c3d-9e8f-112233445566', { title: '' })),
    '0f9c1d2e',
    'a session with no title is called by the front of its id'
  );
}

// ── The collapsed strip ───────────────────────────────────────────────────

// One cell per session the user owns, in the same order the open column draws
// them: everything being worked on, then everything finished.
{
  const cells = stripCells(
    [
      owned('work-1', { title: 'Scanner enrichment', agent: 'codex', state: 'background' }),
      owned('done-1', { title: 'Old thing', completedAt: '2026-07-27T09:00:00.000Z' }),
      owned('work-2', { title: 'Layout flip', viaCmux: true, state: 'exited' }),
      owned('done-2', { title: 'Newer thing', completedAt: '2026-07-29T09:00:00.000Z' })
    ],
    'work-2'
  );
  assert.deepEqual(
    cells.map((cell) => cell.ownedId),
    ['work-1', 'work-2', 'done-2', 'done-1']
  );
  assert.deepEqual(cells[0], {
    ownedId: 'work-1',
    label: 'Scanner enrichment',
    agent: 'codex',
    viaCmux: false,
    state: 'background',
    done: false,
    active: false
  });
  assert.deepEqual(cells[1], {
    ownedId: 'work-2',
    label: 'Layout flip',
    agent: 'claude',
    viaCmux: true,
    state: 'exited',
    done: false,
    active: true,
    });
  assert.equal(cells[2].done, true, 'a finished session still gets a cell, marked as done');
  assert.equal(
    cells.filter((cell) => cell.active).length,
    1,
    'only the session on screen is the active one'
  );
}

// Nothing owned means nothing to draw, and an active id that names no session
// simply marks nothing.
{
  assert.deepEqual(stripCells([], null), []);
  assert.deepEqual(
    stripCells([owned('a')], 'gone').map((cell) => cell.active),
    [false]
  );
}

// ── Remembering that the column is collapsed ──────────────────────────────

{
  const storage = fakeStorage();
  assert.equal(readSessionsCollapsed(storage), false, 'the column starts open');

  assert.equal(writeSessionsCollapsed(storage, true), true);
  assert.equal(storage.data.get(SESSIONS_COLLAPSED_KEY), 'true');
  assert.equal(readSessionsCollapsed(storage), true);

  assert.equal(writeSessionsCollapsed(storage, false), true);
  assert.equal(readSessionsCollapsed(storage), false);
}

// Anything unreadable — corrupt text, a value of the wrong shape, a storage
// that throws — reads as "not collapsed". The worst that costs is a column
// that opens when it was left closed.
{
  const storage = fakeStorage();
  storage.data.set(SESSIONS_COLLAPSED_KEY, 'not json at all');
  assert.equal(readSessionsCollapsed(storage), false);

  storage.data.set(SESSIONS_COLLAPSED_KEY, '"true"');
  assert.equal(readSessionsCollapsed(storage), false, 'the text "true" is not the answer true');

  assert.equal(readSessionsCollapsed(fakeStorage({ unreadable: true })), false);
}

// A storage that refuses the write says so rather than throwing into the shell.
{
  assert.equal(writeSessionsCollapsed(fakeStorage({ refuse: true }), true), false);
}

console.log('sessionStrip: all assertions passed');
