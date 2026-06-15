import assert from 'node:assert/strict';
import {
  clampSourcePaneSize,
  deriveSourcePaneState,
  finishSourcePanePointerSize,
  resolveSourcePaneSize,
  restoreSourcePaneExpandedSize
} from '../src/lib/sourcePaneSizing.ts';

const sidePaneConfig = {
  defaultSize: 407,
  minSize: 40,
  maxSize: 1600,
  collapseThreshold: 48,
  railThreshold: 260
};
const sidePaneRailSizeConfig = {
  ...sidePaneConfig,
  railSize: 40
};

const contextPaneConfig = {
  defaultSize: 330,
  minSize: 34,
  maxSize: 1600,
  collapseThreshold: 30,
  railThreshold: 220
};
const contextPaneRailSizeConfig = {
  ...contextPaneConfig,
  railSize: 34
};

const bottomPaneConfig = {
  defaultSize: 300,
  minSize: 96,
  maxSize: 1100,
  collapseThreshold: 84
};

assert.equal(clampSourcePaneSize(22, sidePaneConfig), 40, 'side panes clamp below the minimum');
assert.equal(clampSourcePaneSize(1700, sidePaneConfig), 1600, 'side panes clamp above the maximum');
assert.equal(clampSourcePaneSize(407.6, sidePaneConfig), 408, 'pane sizes should be rounded');
assert.equal(clampSourcePaneSize(Number.NaN, sidePaneConfig), 407, 'invalid sizes fall back to default');

assert.equal(
  deriveSourcePaneState({ visible: false, size: 407 }, sidePaneConfig),
  'collapsed',
  'hidden panes should derive collapsed state independent of size'
);
assert.equal(
  deriveSourcePaneState({ visible: true, size: 40 }, sidePaneConfig),
  'rail',
  'visible horizontal panes at or below the rail threshold should derive rail state'
);
assert.equal(
  deriveSourcePaneState({ visible: true, size: 261 }, sidePaneConfig),
  'expanded',
  'visible horizontal panes above the rail threshold should derive expanded state'
);

assert.deepEqual(
  finishSourcePanePointerSize(44, sidePaneConfig, { previousExpandedSize: 512 }),
  { state: 'collapsed', size: 0, persistedSize: 512 },
  'pointer finish below the collapse threshold should collapse without overwriting the prior expanded size'
);
assert.deepEqual(
  finishSourcePanePointerSize(96.2, sidePaneConfig),
  { state: 'rail', size: 96, persistedSize: 96 },
  'pointer finish below the rail threshold should keep raw rail width when no rail size is configured'
);
assert.deepEqual(
  finishSourcePanePointerSize(96.2, sidePaneRailSizeConfig),
  { state: 'rail', size: 40, persistedSize: 40 },
  'pointer finish below the side rail threshold should snap to the configured rail size'
);
assert.deepEqual(
  finishSourcePanePointerSize(70, contextPaneRailSizeConfig),
  { state: 'rail', size: 34, persistedSize: 34 },
  'pointer finish below the context rail threshold should snap to the configured rail size'
);
assert.deepEqual(
  finishSourcePanePointerSize(120, contextPaneRailSizeConfig),
  { state: 'rail', size: 34, persistedSize: 34 },
  'context pointer finish below the compact-card minimum should snap to the rail size'
);
assert.deepEqual(
  finishSourcePanePointerSize(220, contextPaneRailSizeConfig),
  { state: 'rail', size: 34, persistedSize: 34 },
  'context pointer finish at the compact-card minimum should still snap to the rail size'
);
assert.deepEqual(
  finishSourcePanePointerSize(221, contextPaneRailSizeConfig),
  { state: 'expanded', size: 221, persistedSize: 221 },
  'context pointer finish above the compact-card minimum should expand without a middle band'
);
assert.deepEqual(
  finishSourcePanePointerSize(340.6, contextPaneConfig),
  { state: 'expanded', size: 341, persistedSize: 341 },
  'context panes above thresholds should finish expanded with a rounded persisted width'
);

assert.equal(
  restoreSourcePaneExpandedSize(40, sidePaneRailSizeConfig, { previousExpandedSize: 512.4 }),
  512,
  'expanding from a rail should restore the prior expanded size when available'
);
assert.deepEqual(
  resolveSourcePaneSize({ visible: false, size: 40 }, sidePaneRailSizeConfig, {
    previousExpandedSize: 512.4
  }),
  { state: 'collapsed', size: 0, persistedSize: 512 },
  'collapsed resolution should prefer previous expanded size over a rail-sized persisted value'
);
assert.equal(
  restoreSourcePaneExpandedSize(40, sidePaneConfig),
  407,
  'expanding from a rail without prior size should restore the default size'
);

assert.deepEqual(
  resolveSourcePaneSize({ visible: true, size: 82 }, contextPaneConfig),
  { state: 'rail', size: 82, persistedSize: 82 },
  'side context panes should resolve rail state from their width config'
);
assert.deepEqual(
  resolveSourcePaneSize({ visible: false, size: 330 }, contextPaneConfig),
  { state: 'collapsed', size: 0, persistedSize: 330 },
  'collapsed context panes should keep an expanded restore width'
);

assert.deepEqual(
  finishSourcePanePointerSize(72, bottomPaneConfig, { previousExpandedSize: 420 }),
  { state: 'collapsed', size: 0, persistedSize: 420 },
  'bottom panes should collapse from raw heights below the bottom collapse threshold'
);
assert.deepEqual(
  finishSourcePanePointerSize(95, bottomPaneConfig),
  { state: 'expanded', size: 96, persistedSize: 96 },
  'bottom panes have no rail state and clamp visible heights to the bottom minimum'
);
assert.deepEqual(
  finishSourcePanePointerSize(1400, bottomPaneConfig),
  { state: 'expanded', size: 1100, persistedSize: 1100 },
  'bottom panes should clamp finished heights to their maximum'
);
