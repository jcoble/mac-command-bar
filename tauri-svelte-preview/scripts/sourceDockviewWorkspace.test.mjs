import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import {
  createDefaultSourceDockLayout,
  hideSourceDockPanel,
  moveSourceDockPanel,
  showSourceDockPanel
} from '../src/lib/sourceDockLayout.ts';
import {
  applySourceDockviewActivePanel,
  applySourceDockviewPanelClose,
  createSourceDockviewPanelPlans,
  sourceDockviewMigrationSlicePlanOptions,
  sourceDockviewMigrationSlices,
  sourceDockviewMigrationSliceStorageKey,
  sourceDockviewPanelDescriptors,
  sourceDockviewStorageKey
} from '../src/lib/sourceDockviewWorkspace.ts';

const pageSource = await readFile(new URL('../src/routes/+page.svelte', import.meta.url), 'utf8');

assert.equal(
  sourceDockviewStorageKey,
  'mac-command-bar.source-browser.dockview-layout',
  'Dockview layouts should use a dedicated storage key separate from the legacy custom dock JSON'
);

assert.deepEqual(
  sourceDockviewPanelDescriptors.map((panel) => [panel.id, panel.title, panel.component]),
  [
    ['activity', 'Activity', 'source-panel'],
    ['editor', 'Editor', 'source-panel'],
    ['context', 'Context', 'source-panel'],
    ['insights', 'Insights', 'source-panel'],
    ['terminal', 'Terminal', 'source-panel'],
    ['browser', 'Browser', 'source-panel']
  ],
  'Dockview panel descriptors should expose all stable workspace panels'
);

const defaultPlans = createSourceDockviewPanelPlans(createDefaultSourceDockLayout());

assert.deepEqual(
  defaultPlans.map((plan) => [plan.id, plan.title, plan.position]),
  [
    ['editor', 'Editor', undefined],
    ['activity', 'Activity', { referencePanel: 'editor', direction: 'left' }],
    ['context', 'Context', { referencePanel: 'editor', direction: 'right' }],
    ['insights', 'Insights', { referencePanel: 'context', direction: 'within' }]
  ],
  'Default Dockview plan should mirror the IDE shell with editor centered and context stacked on the right'
);

assert.equal(defaultPlans.find((plan) => plan.id === 'activity')?.initialWidth, 360);
assert.equal(defaultPlans.find((plan) => plan.id === 'context')?.initialWidth, 330);
assert.equal(
  defaultPlans.some((plan) => plan.id === 'terminal' || plan.id === 'browser'),
  false,
  'Hidden bottom panels should not be added to Dockview until restored'
);

const terminalBottomLayout = moveSourceDockPanel(
  showSourceDockPanel(createDefaultSourceDockLayout(), 'terminal'),
  'context',
  'bottom'
);
const bottomPlans = createSourceDockviewPanelPlans(terminalBottomLayout);

assert.deepEqual(
  bottomPlans
    .filter((plan) => plan.id === 'context' || plan.id === 'terminal')
    .map((plan) => [plan.id, plan.position, plan.initialHeight]),
  [
    ['terminal', { referencePanel: 'editor', direction: 'below' }, 260],
    ['context', { referencePanel: 'terminal', direction: 'within' }, undefined]
  ],
  'Bottom dock panels should preserve layout order while becoming a below-editor Dockview tab stack'
);

const contextTopPlans = createSourceDockviewPanelPlans(
  moveSourceDockPanel(createDefaultSourceDockLayout(), 'context', 'center')
);
assert.deepEqual(
  contextTopPlans.find((plan) => plan.id === 'context')?.position,
  { referencePanel: 'editor', direction: 'above' },
  'A center-group context panel should map to Dockview above-editor placement'
);

assert.deepEqual(
  createSourceDockviewPanelPlans(createDefaultSourceDockLayout(), {
    panelIDs: ['insights'],
    rootPanelID: 'insights'
  }).map((plan) => [plan.id, plan.position]),
  [['insights', undefined]],
  'A visible migration slice should be able to host only the insights panel without creating editor placeholders'
);

assert.deepEqual(
  createSourceDockviewPanelPlans(hideSourceDockPanel(createDefaultSourceDockLayout(), 'insights'), {
    panelIDs: ['insights'],
    rootPanelID: 'insights'
  }),
  [],
  'A subset Dockview host should create no panels while its root panel is hidden'
);

assert.deepEqual(
  createSourceDockviewPanelPlans(createDefaultSourceDockLayout(), {
    panelIDs: ['context', 'insights'],
    rootPanelID: 'context'
  }).map((plan) => [plan.id, plan.position]),
  [
    ['context', undefined],
    ['insights', { referencePanel: 'context', direction: 'within' }]
  ],
  'A subset Dockview host should stack sibling panels inside the chosen root panel'
);

assert.deepEqual(
  sourceDockviewMigrationSlices,
  [
    { id: 'insights-only', rootPanelID: 'insights', panelIDs: ['insights'] },
    { id: 'context-insights', rootPanelID: 'context', panelIDs: ['context', 'insights'] },
    { id: 'bottom-runtime', rootPanelID: 'terminal', panelIDs: ['terminal', 'browser'] }
  ],
  'Dockview migration slices should expose stable roots and panel membership'
);

assert.deepEqual(
  [
    sourceDockviewMigrationSliceStorageKey('insights-only'),
    sourceDockviewMigrationSliceStorageKey('context-insights'),
    sourceDockviewMigrationSliceStorageKey('bottom-runtime')
  ],
  [
    'mac-command-bar.source-browser.dockview-layout.insights-only',
    'mac-command-bar.source-browser.dockview-layout.context-insights',
    'mac-command-bar.source-browser.dockview-layout.bottom-runtime'
  ],
  'Dockview migration slices should use stable layout storage keys'
);

assert.deepEqual(
  [
    sourceDockviewMigrationSlicePlanOptions('insights-only'),
    sourceDockviewMigrationSlicePlanOptions('context-insights'),
    sourceDockviewMigrationSlicePlanOptions('bottom-runtime')
  ],
  [
    { panelIDs: ['insights'], rootPanelID: 'insights' },
    { panelIDs: ['context', 'insights'], rootPanelID: 'context' },
    { panelIDs: ['terminal', 'browser'], rootPanelID: 'terminal' }
  ],
  'Dockview migration slices should expose reusable panel plan options'
);

assert.deepEqual(
  createSourceDockviewPanelPlans(
    createDefaultSourceDockLayout(),
    sourceDockviewMigrationSlicePlanOptions('context-insights')
  ).map((plan) => [plan.id, plan.position]),
  [
    ['context', undefined],
    ['insights', { referencePanel: 'context', direction: 'within' }]
  ],
  'The context-insights migration slice should create only context and insights panel plans'
);

const runtimeBottomLayout = showSourceDockPanel(
  showSourceDockPanel(createDefaultSourceDockLayout(), 'terminal'),
  'browser'
);
assert.deepEqual(
  createSourceDockviewPanelPlans(
    runtimeBottomLayout,
    sourceDockviewMigrationSlicePlanOptions('bottom-runtime')
  ).map((plan) => [plan.id, plan.position]),
  [
    ['terminal', undefined],
    ['browser', { referencePanel: 'terminal', direction: 'within' }]
  ],
  'The bottom-runtime migration slice should create only terminal and browser panel plans'
);

const closedContextLayout = applySourceDockviewPanelClose(createDefaultSourceDockLayout(), 'context');
assert.equal(
  closedContextLayout.hiddenPanelIDs.includes('context'),
  true,
  'Closing a Dockview tab should hide the matching app panel'
);
assert.equal(
  closedContextLayout.groups.some((group) => group.panelIDs.includes('context')),
  false,
  'Closed Dockview panels should be removed from visible app groups'
);
assert.deepEqual(
  applySourceDockviewPanelClose(createDefaultSourceDockLayout(), 'editor'),
  createDefaultSourceDockLayout(),
  'Dockview should not be able to hide the required editor panel through the app adapter'
);

const activeInsightsLayout = applySourceDockviewActivePanel(createDefaultSourceDockLayout(), 'insights');
assert.equal(
  activeInsightsLayout.activePanelByGroup.right,
  'insights',
  'Activating a Dockview tab should activate the matching app dock panel'
);
assert.deepEqual(
  applySourceDockviewActivePanel(createDefaultSourceDockLayout(), null),
  createDefaultSourceDockLayout(),
  'A missing Dockview active panel should leave app layout state unchanged'
);

assert.ok(
  pageSource.includes("import 'dockview-core/dist/styles/dockview.css';"),
  'Source page should load Dockview base CSS'
);
assert.ok(
  pageSource.includes("from '$lib/sourceDockviewWorkspace'"),
  'Source page should import the Dockview workspace bridge'
);

const workspaceSource = await readFile(new URL('../src/lib/sourceDockviewWorkspace.ts', import.meta.url), 'utf8');
assert.ok(
  workspaceSource.includes('api.onDidRemovePanel'),
  'Dockview workspace should observe tab close events before visible pane wiring'
);
assert.ok(
  workspaceSource.includes('api.onDidActivePanelChange'),
  'Dockview workspace should observe active tab changes before visible pane wiring'
);
assert.ok(
  workspaceSource.includes('api.onDidMovePanel'),
  'Dockview workspace should observe panel moves before visible pane wiring'
);
assert.ok(
  workspaceSource.includes('synchronizingDockview'),
  'Dockview workspace should suppress callback churn during app-driven sync'
);
assert.ok(
  workspaceSource.includes('storedLayout && restorePlans.length > 0'),
  'Dockview workspace should not restore stored JSON when the current app layout hides the migrated root panel'
);
