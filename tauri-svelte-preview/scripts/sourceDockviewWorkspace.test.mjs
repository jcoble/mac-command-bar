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
  normalizeSourceDockviewPanelPlanOptions,
  sourceDockviewMigrationSlicePlanOptions,
  sourceDockviewMigrationSlices,
  sourceDockviewMigrationSliceStorageKey,
  syncSourceDockviewTabStackPanels,
  sourceDockviewStoredLayoutMatchesPanelPlans,
  sourceDockviewPanelDescriptors,
  sourceDockviewStorageKey
} from '../src/lib/sourceDockviewWorkspace.ts';

const pageSource = await readFile(new URL('../src/routes/+page.svelte', import.meta.url), 'utf8');
const workspaceSource = await readFile(new URL('../src/lib/sourceDockviewWorkspace.ts', import.meta.url), 'utf8');

assert.equal(
  sourceDockviewStorageKey,
  'mac-command-bar.source-browser.dockview-layout',
  'Dockview layouts should use a dedicated storage key separate from the legacy custom dock JSON'
);

assert.ok(
  workspaceSource.includes('hideBorders: true') &&
    workspaceSource.includes('theme: { ...themeDracula, gap: 0 }') &&
    workspaceSource.includes('dockview-theme-dracula') &&
    workspaceSource.includes('source-dockview-attached-panel'),
  'Dockview workspaces should hide borders, use Dracula with zero gap, and mark attached panels for full-tab layout'
);

assert.deepEqual(
  sourceDockviewPanelDescriptors.map((panel) => [panel.id, panel.title, panel.component]),
  [
    ['activity', 'Activity', 'source-panel'],
    ['editor', 'Editor', 'source-panel'],
    ['context', 'Context', 'source-panel'],
    ['insights', 'Insights', 'source-panel'],
    ['terminal', 'Terminal', 'source-panel'],
    ['browser', 'Browser', 'source-panel'],
    ['markdown', 'Markdown', 'source-panel']
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

const contextTopPlans = createSourceDockviewPanelPlans(
  moveSourceDockPanel(createDefaultSourceDockLayout(), 'context', 'center')
);
assert.deepEqual(
  contextTopPlans.find((plan) => plan.id === 'context')?.position,
  { referencePanel: 'editor', direction: 'above' },
  'A center-group context panel should map to Dockview above-editor placement'
);

const centerRuntimePlans = createSourceDockviewPanelPlans(
  showSourceDockPanel(showSourceDockPanel(createDefaultSourceDockLayout(), 'terminal'), 'browser')
);
assert.deepEqual(
  centerRuntimePlans
    .filter((plan) => plan.id === 'terminal' || plan.id === 'browser')
    .map((plan) => [plan.id, plan.position]),
  [
    ['terminal', { referencePanel: 'editor', direction: 'within' }],
    ['browser', { referencePanel: 'terminal', direction: 'within' }]
  ],
  'Center runtime panels should stack as editor tabs instead of splitting the editor group'
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
    { id: 'activity-only', rootPanelID: 'activity', panelIDs: ['activity'] },
    { id: 'insights-only', rootPanelID: 'insights', panelIDs: ['insights'] },
    { id: 'context-insights', rootPanelID: 'context', panelIDs: ['context', 'insights'] },
    {
      id: 'center-runtime',
      rootPanelID: 'editor',
      panelIDs: ['editor', 'terminal', 'browser'],
      groupID: 'center'
    },
    {
      id: 'bottom-runtime',
      rootPanelID: 'terminal',
      panelIDs: ['terminal', 'browser'],
      groupID: 'bottom'
    }
  ],
  'Dockview migration slices should expose stable roots and panel membership'
);

assert.deepEqual(
  [
    sourceDockviewMigrationSliceStorageKey('activity-only'),
    sourceDockviewMigrationSliceStorageKey('insights-only'),
    sourceDockviewMigrationSliceStorageKey('context-insights'),
    sourceDockviewMigrationSliceStorageKey('center-runtime'),
    sourceDockviewMigrationSliceStorageKey('bottom-runtime')
  ],
  [
    'mac-command-bar.source-browser.dockview-layout.activity-only',
    'mac-command-bar.source-browser.dockview-layout.insights-only',
    'mac-command-bar.source-browser.dockview-layout.context-insights',
    'mac-command-bar.source-browser.dockview-layout.center-runtime',
    'mac-command-bar.source-browser.dockview-layout.bottom-runtime'
  ],
  'Dockview migration slices should use stable layout storage keys'
);

assert.deepEqual(
  [
    sourceDockviewMigrationSlicePlanOptions('activity-only'),
    sourceDockviewMigrationSlicePlanOptions('insights-only'),
    sourceDockviewMigrationSlicePlanOptions('context-insights'),
    sourceDockviewMigrationSlicePlanOptions('center-runtime'),
    sourceDockviewMigrationSlicePlanOptions('bottom-runtime')
  ],
  [
    { panelIDs: ['activity'], rootPanelID: 'activity' },
    { panelIDs: ['insights'], rootPanelID: 'insights' },
    { panelIDs: ['context', 'insights'], rootPanelID: 'context' },
    { panelIDs: ['editor', 'terminal', 'browser'], rootPanelID: 'editor', groupID: 'center' },
    { panelIDs: ['terminal', 'browser'], rootPanelID: 'terminal', groupID: 'bottom' }
  ],
  'Dockview migration slices should expose reusable panel plan options'
);

assert.deepEqual(
  normalizeSourceDockviewPanelPlanOptions({
    panelIDs: ['context', 'insights', 'context', 'insights'],
    rootPanelID: 'context'
  }),
  { panelIDs: ['context', 'insights'], rootPanelID: 'context' },
  'Dockview migration panel plan options should de-duplicate requested panel IDs'
);

assert.deepEqual(
  normalizeSourceDockviewPanelPlanOptions({
    panelIDs: ['insights'],
    rootPanelID: 'context'
  }),
  { panelIDs: ['context', 'insights'], rootPanelID: 'context' },
  'Dockview migration panel plan options should include the requested root panel'
);

assert.throws(
  () =>
    normalizeSourceDockviewPanelPlanOptions({
      panelIDs: ['context', 'missing-panel'],
      rootPanelID: 'context'
    }),
  /Unknown Dockview panel ID for panelIDs\[1\]: missing-panel/,
  'Dockview migration panel plan options should reject unknown panel IDs'
);

assert.throws(
  () =>
    normalizeSourceDockviewPanelPlanOptions({
      panelIDs: ['context'],
      rootPanelID: 'missing-root'
    }),
  /Unknown Dockview panel ID for rootPanelID: missing-root/,
  'Dockview migration panel plan options should reject unknown root panel IDs'
);

assert.throws(
  () => sourceDockviewMigrationSlicePlanOptions('missing-slice'),
  /Unknown Dockview migration slice: missing-slice/,
  'Dockview migration slice helpers should reject unknown slice IDs'
);

assert.deepEqual(
  createSourceDockviewPanelPlans(
    createDefaultSourceDockLayout(),
    sourceDockviewMigrationSlicePlanOptions('activity-only')
  ).map((plan) => [plan.id, plan.position]),
  [['activity', undefined]],
  'The activity-only migration slice should create only the activity panel plan'
);

assert.deepEqual(
  createSourceDockviewPanelPlans(
    createDefaultSourceDockLayout(),
    sourceDockviewMigrationSlicePlanOptions('insights-only')
  ).map((plan) => [plan.id, plan.position]),
  [['insights', undefined]],
  'The insights-only migration slice should create only the insights panel plan'
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

assert.deepEqual(
  createSourceDockviewPanelPlans(
    hideSourceDockPanel(createDefaultSourceDockLayout(), 'insights'),
    sourceDockviewMigrationSlicePlanOptions('insights-only')
  ),
  [],
  'A migration slice should create no panels while its root panel is hidden'
);

const runtimeBottomLayout = showSourceDockPanel(
  showSourceDockPanel(createDefaultSourceDockLayout(), 'terminal'),
  'browser'
);
assert.deepEqual(
  createSourceDockviewPanelPlans(
    runtimeBottomLayout,
    sourceDockviewMigrationSlicePlanOptions('center-runtime')
  ).map((plan) => [plan.id, plan.position]),
  [
    ['editor', undefined],
    ['terminal', { referencePanel: 'editor', direction: 'within' }],
    ['browser', { referencePanel: 'terminal', direction: 'within' }]
  ],
  'The center-runtime migration slice should stack editor, terminal, and browser as center tabs'
);
assert.deepEqual(
  createSourceDockviewPanelPlans(
    runtimeBottomLayout,
    sourceDockviewMigrationSlicePlanOptions('bottom-runtime')
  ),
  [],
  'The retired bottom-runtime migration slice should not create visible runtime panels'
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

assert.match(
  pageSource,
  /import\s+["']dockview-core\/dist\/styles\/dockview\.css["'];/,
  'Source page should load Dockview base CSS'
);
assert.match(
  pageSource,
  /from\s+["']\$lib\/sourceDockviewWorkspace["']/,
  'Source page should import the Dockview workspace bridge'
);

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
  workspaceSource.includes('queueMicrotask(() => dispatchSourceDockviewLayout(element))') &&
    workspaceSource.includes('function dispatchSourceDockviewLayout(element: HTMLElement)') &&
    workspaceSource.includes('element.firstElementChild'),
  'Dockview workspace should forward layout events to attached Svelte panel elements after reparenting'
);
assert.ok(
  workspaceSource.includes('sourceDockviewStoredLayoutMatchesPanelPlans(storedLayout, restorePlans)'),
  'Dockview workspace should restore stored JSON only when it matches the current visible panel plans'
);

const fakeDockviewPanels = [
  {
    id: 'file:a.cs',
    title: 'A.cs',
    api: {
      setTitle(title) {
        fakeDockviewPanels[0].title = title;
      }
    }
  },
  {
    id: 'file:old.cs',
    title: 'Old.cs',
    api: {
      setTitle() {}
    }
  }
];
const fakeAddedPanelPlans = [];
const fakeDockviewApi = {
  get panels() {
    return fakeDockviewPanels;
  },
  getPanel(id) {
    return fakeDockviewPanels.find((panel) => panel.id === id);
  },
  removePanel(panel) {
    const index = fakeDockviewPanels.indexOf(panel);
    if (index !== -1) fakeDockviewPanels.splice(index, 1);
  },
  addPanel(plan) {
    fakeAddedPanelPlans.push(plan);
    const panel = {
      id: plan.id,
      title: plan.title,
      api: {
        setTitle(title) {
          panel.title = title;
        }
      }
    };
    fakeDockviewPanels.push(panel);
    return panel;
  }
};
const validFilePanelIDs = new Set(['file:a.cs', 'file:old.cs']);
syncSourceDockviewTabStackPanels(
  fakeDockviewApi,
  [
    { id: 'file:a.cs', title: 'A.cs *' },
    { id: 'file:b.cs', title: 'B.cs' }
  ],
  validFilePanelIDs,
  'file:a.cs'
);
assert.deepEqual(
  fakeDockviewPanels.map((panel) => [panel.id, panel.title]),
  [
    ['file:a.cs', 'A.cs *'],
    ['file:b.cs', 'B.cs']
  ],
  'Dynamic Dockview tab stacks should remove closed panels, add new panels, and update titles without clearing existing panels'
);
assert.deepEqual(
  [...validFilePanelIDs],
  ['file:a.cs', 'file:b.cs'],
  'Dynamic Dockview tab stacks should refresh the valid panel ID set used by content renderers'
);
assert.deepEqual(
  fakeAddedPanelPlans.map((plan) => [plan.id, plan.position]),
  [['file:b.cs', { referencePanel: 'file:a.cs', direction: 'within' }]],
  'Dynamic Dockview tab stacks should add new file tabs into the existing active tab group'
);
