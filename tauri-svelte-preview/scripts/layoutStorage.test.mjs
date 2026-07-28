import assert from 'node:assert/strict';
import {
  loadLayout,
  saveLayout,
  clearLayout,
  gridPanelIds,
  dockPanelIds,
  panelSetMatches
} from '../src/lib/shell/layout/layoutStorage.ts';

function memoryStorage(initial = {}) {
  const map = new Map(Object.entries(initial));
  return {
    getItem: (k) => (map.has(k) ? map.get(k) : null),
    setItem: (k, v) => map.set(k, v),
    removeItem: (k) => map.delete(k),
    _map: map
  };
}

// loadLayout: absent, corrupt, and round-trip
{
  const storage = memoryStorage();
  assert.equal(loadLayout(storage, 'k'), null, 'absent key -> null');
  storage.setItem('k', '{not json');
  assert.equal(loadLayout(storage, 'k'), null, 'corrupt json -> null, no throw');
  storage.setItem('k', JSON.stringify({ a: 1 }));
  assert.deepEqual(loadLayout(storage, 'k'), { a: 1 }, 'round-trip');
}

// saveLayout: happy path true, quota error false (never throws)
{
  const storage = memoryStorage();
  assert.equal(saveLayout(storage, 'k', { a: 1 }), true);
  assert.deepEqual(JSON.parse(storage.getItem('k')), { a: 1 });
  const failing = {
    getItem: () => null,
    setItem: () => {
      throw new DOMException('quota', 'QuotaExceededError');
    },
    removeItem: () => {}
  };
  assert.equal(saveLayout(failing, 'k', { a: 1 }), false, 'quota -> false, no throw');
}

// clearLayout tolerates a throwing storage
{
  const throwing = {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {
      throw new Error('nope');
    }
  };
  assert.doesNotThrow(() => clearLayout(throwing, 'k'));
}

// gridPanelIds walks the SerializedGridviewComponent tree (branch/leaf shape)
{
  const grid = {
    grid: {
      root: {
        type: 'branch',
        data: [
          { type: 'leaf', data: { id: 'rail' } },
          {
            type: 'branch',
            data: [
              { type: 'leaf', data: { id: 'center' } },
              { type: 'leaf', data: { id: 'dock' } }
            ]
          },
          { type: 'leaf', data: { id: 'context' } }
        ]
      },
      width: 1200,
      height: 800,
      orientation: 'HORIZONTAL'
    }
  };
  assert.deepEqual([...gridPanelIds(grid)].sort(), ['center', 'context', 'dock', 'rail']);
  assert.deepEqual([...gridPanelIds({})], [], 'malformed -> empty, no throw');
}

// dockPanelIds reads SerializedDockview.panels keys
{
  const dock = { panels: { session: {}, editor: {}, browser: {} }, grid: {} };
  assert.deepEqual([...dockPanelIds(dock)].sort(), ['browser', 'editor', 'session']);
  assert.deepEqual([...dockPanelIds({ grid: {} })], [], 'missing panels -> empty');
}

// panelSetMatches: exact set equality, order-independent
{
  assert.equal(panelSetMatches(['a', 'b'], ['b', 'a']), true);
  assert.equal(panelSetMatches(['a'], ['a', 'b']), false);
  assert.equal(panelSetMatches(['a', 'b', 'c'], ['a', 'b']), false);
  assert.equal(panelSetMatches([], []), true);
}

console.log('layoutStorage: all tests passed');
