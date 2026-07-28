import assert from 'node:assert/strict';

import {
  ACTIVE_VIEW_KEY,
  DEFAULT_SIDEBAR_VIEW,
  SIDEBAR_VIEWS,
  clearActiveView,
  isSidebarViewId,
  readActiveView,
  viewPanesKey,
  writeActiveView
} from '../src/lib/shell/layout/sidebarViews.ts';

/** A localStorage stand-in; `failWrites` makes every write throw like a full quota. */
function fakeStorage({ failWrites = false } = {}) {
  const map = new Map();
  return {
    map,
    getItem: (key) => (map.has(key) ? map.get(key) : null),
    setItem: (key, value) => {
      if (failWrites) throw new Error('QuotaExceededError');
      map.set(key, value);
    },
    removeItem: (key) => map.delete(key)
  };
}

// The roster is the four views the activity bar offers, and the one it opens on
// is one of them.
{
  assert.deepEqual(
    SIDEBAR_VIEWS.map((view) => view.id),
    ['sessions', 'explorer', 'source-control', 'worktrees']
  );
  assert.ok(
    SIDEBAR_VIEWS.every((view) => typeof view.title === 'string' && view.title.length > 0),
    'every view has a name to show in its tooltip'
  );
  assert.ok(isSidebarViewId(DEFAULT_SIDEBAR_VIEW));
}

// Only the four ids are ids. Anything else — including the shapes a corrupt
// stored value can take — is not.
{
  for (const id of ['sessions', 'explorer', 'source-control', 'worktrees']) {
    assert.ok(isSidebarViewId(id), `${id} is a view`);
  }
  for (const value of ['', 'files', 'git', 'SESSIONS', null, undefined, 7, {}, ['sessions']]) {
    assert.equal(isSidebarViewId(value), false, `${JSON.stringify(value)} is not a view`);
  }
}

// Each view keeps its panes under its own key, and no two views share one.
{
  const keys = SIDEBAR_VIEWS.map((view) => viewPanesKey(view.id));
  assert.deepEqual(keys, [
    'mac-command-bar.next.view-sessions-panes',
    'mac-command-bar.next.view-explorer-panes',
    'mac-command-bar.next.view-source-control-panes',
    'mac-command-bar.next.view-worktrees-panes'
  ]);
  assert.equal(new Set(keys).size, keys.length, 'no two views write to the same key');
  assert.ok(!keys.includes(ACTIVE_VIEW_KEY), 'and none of them is the active-view key');
}

// Round trip: what is written is what comes back.
{
  const storage = fakeStorage();
  for (const view of SIDEBAR_VIEWS) {
    assert.equal(writeActiveView(storage, view.id), true);
    assert.equal(readActiveView(storage), view.id);
  }
}

// Nothing stored yet — open on the default view.
{
  assert.equal(readActiveView(fakeStorage()), DEFAULT_SIDEBAR_VIEW);
}

// A stored id that is not a view any more falls back rather than opening a view
// that does not exist. Same for a value that is not JSON at all.
{
  const storage = fakeStorage();
  storage.map.set(ACTIVE_VIEW_KEY, JSON.stringify('a-view-we-removed'));
  assert.equal(readActiveView(storage), DEFAULT_SIDEBAR_VIEW);

  storage.map.set(ACTIVE_VIEW_KEY, '{not json');
  assert.equal(readActiveView(storage), DEFAULT_SIDEBAR_VIEW);

  storage.map.set(ACTIVE_VIEW_KEY, JSON.stringify({ id: 'sessions' }));
  assert.equal(readActiveView(storage), DEFAULT_SIDEBAR_VIEW, 'the wrong shape is not an id');
}

// Clearing takes the shell back to the default view.
{
  const storage = fakeStorage();
  writeActiveView(storage, 'worktrees');
  clearActiveView(storage);
  assert.equal(storage.getItem(ACTIVE_VIEW_KEY), null);
  assert.equal(readActiveView(storage), DEFAULT_SIDEBAR_VIEW);
}

// A storage that refuses to write says so instead of throwing into the shell,
// and reading it still answers with the default.
{
  const storage = fakeStorage({ failWrites: true });
  assert.equal(writeActiveView(storage, 'explorer'), false, 'a full storage reports the failure');
  assert.equal(readActiveView(storage), DEFAULT_SIDEBAR_VIEW);
}

// A storage that throws on every call at all is still survivable.
{
  const hostile = {
    getItem() {
      throw new Error('nope');
    },
    setItem() {
      throw new Error('nope');
    },
    removeItem() {
      throw new Error('nope');
    }
  };
  assert.equal(readActiveView(hostile), DEFAULT_SIDEBAR_VIEW);
  assert.equal(writeActiveView(hostile, 'sessions'), false);
  clearActiveView(hostile);
}

console.log('sidebarViews: all tests passed');
