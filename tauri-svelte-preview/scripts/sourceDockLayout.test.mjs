import assert from 'node:assert/strict';
import {
  activateSourceDockPanel,
  createDefaultSourceDockLayout,
  hideSourceDockPanel,
  moveSourceDockPanel,
  normalizeSourceDockLayout,
  resizeSourceDockGroup,
  showSourceDockPanel,
  sourceDockGroupSize,
  sourceDockPanelDescriptors,
  visibleSourceDockPanelIDs
} from '../src/lib/sourceDockLayout.ts';

const defaultLayout = createDefaultSourceDockLayout();

assert.deepEqual(
  defaultLayout.groups.map((group) => [group.id, group.panelIDs]),
  [
    ['left', ['activity']],
    ['center', ['editor']],
    ['right', ['context', 'insights']],
    ['bottom', []]
  ],
  'default layout should match the compact IDE shell'
);
assert.deepEqual(defaultLayout.hiddenPanelIDs, ['terminal', 'browser']);
assert.equal(defaultLayout.activePanelByGroup.center, 'editor');

const movedContext = moveSourceDockPanel(defaultLayout, 'context', 'bottom');
assert.deepEqual(
  movedContext.groups.map((group) => [group.id, group.panelIDs]),
  [
    ['left', ['activity']],
    ['center', ['editor']],
    ['right', ['insights']],
    ['bottom', ['context']]
  ],
  'moving a panel should remove it from the old group and add it to the target group'
);
assert.equal(movedContext.activePanelByGroup.bottom, 'context');
assert.equal(movedContext.preset, 'custom');
assert.deepEqual(
  defaultLayout.groups.map((group) => [group.id, group.panelIDs]),
  [
    ['left', ['activity']],
    ['center', ['editor']],
    ['right', ['context', 'insights']],
    ['bottom', []]
  ],
  'layout transforms should not mutate the input'
);

const hiddenInsights = hideSourceDockPanel(defaultLayout, 'insights');
assert.deepEqual(
  hiddenInsights.groups.find((group) => group.id === 'right')?.panelIDs,
  ['context'],
  'hiding a panel should remove it from its visible group'
);
assert.deepEqual(hiddenInsights.hiddenPanelIDs, ['terminal', 'browser', 'insights']);

const restoredInsights = showSourceDockPanel(hiddenInsights, 'insights');
assert.deepEqual(
  restoredInsights.groups.find((group) => group.id === 'right')?.panelIDs,
  ['context', 'insights'],
  'restoring a panel should return it to its preferred group'
);
assert.deepEqual(restoredInsights.hiddenPanelIDs, ['terminal', 'browser']);
assert.equal(restoredInsights.activePanelByGroup.right, 'insights');

const attemptedEditorHide = hideSourceDockPanel(defaultLayout, 'editor');
assert.deepEqual(
  attemptedEditorHide.groups.find((group) => group.id === 'center')?.panelIDs,
  ['editor'],
  'the editor panel should remain visible'
);
assert.deepEqual(attemptedEditorHide.hiddenPanelIDs, ['terminal', 'browser']);

const normalized = normalizeSourceDockLayout({
  preset: 'custom',
  groups: [
    { id: 'left', panelIDs: ['editor', 'unknown', 'context'], size: 111 },
    { id: 'right', panelIDs: ['context', 'activity'], size: Number.NaN }
  ],
  hiddenPanelIDs: ['editor', 'browser', 'missing'],
  activePanelByGroup: {
    left: 'unknown',
    center: 'context',
    right: 'activity'
  }
});

assert.deepEqual(
  normalized.groups.map((group) => [group.id, group.panelIDs]),
  [
    ['left', ['context']],
    ['center', ['editor']],
    ['right', ['activity']],
    ['bottom', []]
  ],
  'normalizing persisted data should remove unknowns, remove duplicates, and keep editor centered'
);
assert.deepEqual(normalized.hiddenPanelIDs, ['browser']);
assert.equal(normalized.activePanelByGroup.left, 'context');
assert.equal(normalized.activePanelByGroup.center, 'editor');
assert.equal(normalized.activePanelByGroup.right, 'activity');

assert.deepEqual(visibleSourceDockPanelIDs(normalized), ['context', 'editor', 'activity']);

const resizedLayout = resizeSourceDockGroup(defaultLayout, 'left', 444.6);
assert.equal(
  sourceDockGroupSize(resizedLayout, 'left'),
  445,
  'resizing a dock group should persist a rounded pane size'
);
assert.equal(resizedLayout.preset, 'custom');
assert.equal(
  sourceDockGroupSize(defaultLayout, 'left'),
  360,
  'resizing a dock group should not mutate the input layout'
);

const invalidResizeLayout = resizeSourceDockGroup(defaultLayout, 'right', -1);
assert.equal(
  sourceDockGroupSize(invalidResizeLayout, 'right'),
  330,
  'invalid dock group sizes should fall back to the group default'
);

const activatedInsights = activateSourceDockPanel(defaultLayout, 'insights');
assert.equal(
  activatedInsights.activePanelByGroup.right,
  'insights',
  'activating a panel should select it within its current dock group'
);
assert.equal(activatedInsights.preset, 'custom');
assert.equal(
  activateSourceDockPanel(defaultLayout, 'terminal').activePanelByGroup.bottom,
  undefined,
  'activating a hidden panel should leave the layout unchanged'
);

assert.deepEqual(
  sourceDockPanelDescriptors.map((panel) => [panel.id, panel.defaultGroupID, panel.label]),
  [
    ['activity', 'left', 'Activity'],
    ['editor', 'center', 'Editor'],
    ['context', 'right', 'Context'],
    ['insights', 'right', 'Insights'],
    ['terminal', 'bottom', 'Terminal'],
    ['browser', 'bottom', 'Browser']
  ],
  'panel descriptors should expose stable labels and preferred docking groups'
);
