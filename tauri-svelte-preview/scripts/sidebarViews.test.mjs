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
import { LEFT_SIDE_PANE_IDS } from '../src/lib/shell/layout/sidePaneRegistry.ts';

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

// The roster is the seven views the tool column can hold, and the one it opens on
// is one of them. Sessions is NOT among them any more: the sessions list has a
// column of its own on the left of the shell. Problems IS among them, even
// though its icon is usually not offered — the column keeps a container for it
// at all times and the activity bar decides whether to draw the icon, so that
// changing where the Problems list lives never tears a container down.
{
  assert.deepEqual(
    SIDEBAR_VIEWS.map((view) => view.id),
    ['explorer', 'source-control', 'worktrees', 'stacks', 'context', 'agents', 'problems']
  );
  assert.ok(
    SIDEBAR_VIEWS.every((view) => typeof view.title === 'string' && view.title.length > 0),
    'every view has a name to show in its tooltip'
  );
  assert.ok(isSidebarViewId(DEFAULT_SIDEBAR_VIEW));
}

// A8's left roster has three stable Paneview ids. They are not aliases for the
// older sessions view or the right-column view ids.
{
  assert.deepEqual([...LEFT_SIDE_PANE_IDS], ['working', 'done', 'settled']);
  assert.ok(!SIDEBAR_VIEWS.some((view) => LEFT_SIDE_PANE_IDS.includes(view.id)));
}

// Only the seven ids are ids. Anything else — including the shapes a corrupt
// stored value can take, and the sessions view that used to be one — is not.
{
  for (const id of [
    'explorer',
    'source-control',
    'worktrees',
    'stacks',
    'context',
    'agents',
    'problems'
  ]) {
    assert.ok(isSidebarViewId(id), `${id} is a view`);
  }
  for (const value of [
    '',
    'sessions',
    'files',
    'git',
    'CONTEXT',
    null,
    undefined,
    7,
    {},
    ['explorer']
  ]) {
    assert.equal(isSidebarViewId(value), false, `${JSON.stringify(value)} is not a view`);
  }
}

// Each view keeps its panes under its own key, and no two views share one.
{
  const keys = SIDEBAR_VIEWS.map((view) => viewPanesKey(view.id));
  assert.deepEqual(keys, [
    'mac-command-bar.next.view-explorer-panes',
    'mac-command-bar.next.view-source-control-panes',
    'mac-command-bar.next.view-worktrees-panes',
    'mac-command-bar.next.view-stacks-panes',
    'mac-command-bar.next.view-context-panes',
    'mac-command-bar.next.view-agents-panes',
    'mac-command-bar.next.view-problems-panes'
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

  // The real case of that today: a shell that was last left on the Sessions
  // view, which moved out of this column entirely.
  storage.map.set(ACTIVE_VIEW_KEY, JSON.stringify('sessions'));
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
  assert.equal(writeActiveView(hostile, 'explorer'), false);
  clearActiveView(hostile);
}

console.log('sidebarViews: all tests passed');
