import assert from 'node:assert/strict';
import {
  actionsForContext,
  layoutActionFan,
  reduceActionSurface
} from '../src/lib/shell/overlay/actionSurfaceModel.ts';

const context = {
  kind: 'browser',
  centerPanelId: 'browser',
  ownedId: 'owner-1',
  workspaceId: 'browser-workspace',
  targetId: 'tab-1',
  generation: 3
};

const browserAction = {
  id: 'browser-expand',
  label: 'Expand browser',
  icon: null,
  contexts: ['browser'],
  shortcut: null,
  confirmation: 'none',
  enabled: () => ({ enabled: true, reason: null }),
  run: async () => {}
};
const editorAction = { ...browserAction, id: 'editor-open', contexts: ['editor'] };

assert.deepEqual(actionsForContext([browserAction, editorAction], context).map((action) => action.id), ['browser-expand']);

const collapsed = { mode: 'collapsed', pendingActionId: null, error: null };
assert.equal(reduceActionSurface(collapsed, { type: 'toggle' }).mode, 'fan-open');
assert.equal(reduceActionSurface(collapsed, { type: 'start-action', actionId: 'browser-expand' }).mode, 'action-pending');
assert.equal(reduceActionSurface(collapsed, { type: 'fail-action', message: 'not available' }).error, 'not available');

const fan = layoutActionFan(
  { width: 480, height: 360 },
  { x: 420, y: 320 },
  [
    { id: 'one', width: 48, height: 48 },
    { id: 'two', width: 48, height: 48 }
  ],
  []
);
assert.equal(fan.kind, 'fan');
assert.equal(fan.items.length, 2);
assert.ok(fan.items.every((item) => item.x >= 16 && item.x <= 464 && item.y >= 16 && item.y <= 344));

const fallback = layoutActionFan(
  { width: 180, height: 140 },
  { x: 140, y: 110 },
  Array.from({ length: 6 }, (_, index) => ({ id: `item-${index}`, width: 48, height: 48 })),
  [{ x: 0, y: 0, width: 180, height: 100 }]
);
assert.equal(fallback.kind, 'bottom-sheet');
assert.equal(fallback.items.length, 6);

console.log('actionSurfaceModel: all tests passed');
