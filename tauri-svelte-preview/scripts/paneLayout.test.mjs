import assert from 'node:assert/strict';

import {
  canRestorePaneLayout,
  migrateLegacyPaneLayout,
  migrateViewPaneLayouts
} from '../src/lib/shell/layout/paneStack.ts';
import { viewPanesKey } from '../src/lib/shell/layout/sidebarViews.ts';

/**
 * A stored layout in the exact shape dockview-core 6.6.1 writes — see
 * `PaneviewComponent.toJSON` (paneviewComponent.js:281-307). `panes` names the
 * sections in stack order; pass `false` for one to store it collapsed.
 */
function storedLayout(panes) {
  return {
    size: 800,
    views: panes.map(([id, expanded = true]) => ({
      size: expanded ? 240 : 22,
      data: {
        id,
        component: 'pane',
        params: { paneId: id },
        title: id
      },
      headerSize: 22,
      expanded
    }))
  };
}

const SECTIONS = ['sessions', 'files', 'source-control'];

function memoryStorage() {
  const map = new Map();
  return {
    map,
    getItem: (key) => map.get(key) ?? null,
    setItem: (key, value) => map.set(key, value),
    removeItem: (key) => map.delete(key)
  };
}

// The layout the shell last wrote is restored, in any order, open or collapsed.
{
  assert.equal(
    canRestorePaneLayout(storedLayout([['sessions'], ['files'], ['source-control']]), SECTIONS),
    true,
    'the same three sections restore'
  );
  assert.equal(
    canRestorePaneLayout(storedLayout([['source-control'], ['sessions'], ['files']]), SECTIONS),
    true,
    'the user having dragged them into a different order still restores'
  );
  assert.equal(
    canRestorePaneLayout(
      storedLayout([
        ['sessions', false],
        ['files', false],
        ['source-control', false]
      ]),
      SECTIONS
    ),
    true,
    'a stack the user left fully collapsed still restores'
  );
}

// A valid legacy view-panes payload moves into the new exact pane id without
// changing the user's size/order/expanded choice. A partial or extra roster is
// refused rather than silently losing a pane.
{
  const legacy = storedLayout([['run-configurations', false]]);
  const migrated = migrateLegacyPaneLayout(legacy, 'run-configurations', 'run');
  assert.equal(canRestorePaneLayout(migrated, ['run']), true);
  assert.equal(migrated.views[0].size, 22);
  assert.equal(migrated.views[0].expanded, false);
  assert.equal(migrated.views[0].data.id, 'run');

  const storage = memoryStorage();
  storage.setItem(viewPanesKey('explorer'), JSON.stringify(storedLayout([['files', false]])));
  storage.setItem(viewPanesKey('source-control'), JSON.stringify(storedLayout([['source-control']])));
  const combined = migrateViewPaneLayouts(storage, ['files', 'source-control']);
  assert.equal(canRestorePaneLayout(combined, ['files', 'source-control']), true);
  assert.equal(migrateViewPaneLayouts(storage, ['files', 'missing']), null);
}

// A stored layout from a different set of sections is thrown away, so the
// shell rebuilds from defaults instead of coming up missing a section.
{
  assert.equal(
    canRestorePaneLayout(storedLayout([['sessions'], ['files']]), SECTIONS),
    false,
    'a section added since the layout was saved -> rebuild'
  );
  assert.equal(
    canRestorePaneLayout(
      storedLayout([['sessions'], ['files'], ['source-control'], ['worktrees']]),
      SECTIONS
    ),
    false,
    'a section removed since the layout was saved -> rebuild'
  );
  assert.equal(
    canRestorePaneLayout(storedLayout([['sessions'], ['files'], ['git']]), SECTIONS),
    false,
    'a renamed section -> rebuild'
  );
  assert.equal(
    canRestorePaneLayout(storedLayout([['sessions'], ['files'], ['source-control']]), []),
    false,
    'a stored layout with no sections asked for -> rebuild'
  );
}

// Anything that is not a real layout is refused rather than handed to dockview,
// which disposes the live stack before it discovers the layout is unusable.
{
  for (const junk of [
    null,
    undefined,
    'a string',
    42,
    [],
    {},
    { size: 800 },
    { size: 800, views: null },
    { size: 800, views: { sessions: {} } },
    { views: 'sessions,files' }
  ]) {
    assert.equal(
      canRestorePaneLayout(junk, SECTIONS),
      false,
      `refused: ${JSON.stringify(junk) ?? String(junk)}`
    );
  }
}

// Views the walker cannot read an id from count as a mismatch, not as a crash.
{
  assert.equal(
    canRestorePaneLayout({ size: 800, views: [{ size: 240 }, { size: 240, data: null }] }, SECTIONS),
    false,
    'views with no data -> rebuild, no throw'
  );
  assert.equal(
    canRestorePaneLayout({ size: 800, views: [null, { data: { id: 'sessions' } }] }, SECTIONS),
    false,
    'a null view among real ones -> rebuild, no throw'
  );
}

// An empty stack of sections and an empty stored layout do agree — but the
// `views` key still has to be there, because dockview reads it unconditionally.
{
  assert.equal(canRestorePaneLayout({ size: 0, views: [] }, []), true);
  assert.equal(canRestorePaneLayout({ size: 0 }, []), false, 'no views key -> refused');
}

console.log('paneLayout: all tests passed');
