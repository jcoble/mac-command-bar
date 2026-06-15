import type {
  DockviewApi,
  DockviewIDisposable,
  IDockviewPanel,
  GroupPanelPartInitParameters,
  IContentRenderer,
  SerializedDockview
} from 'dockview-core';
import {
  activateSourceDockPanel,
  hideSourceDockPanel,
  normalizeSourceDockLayout,
  sourceDockPanelDescriptors,
  type SourceDockGroupID,
  type SourceDockLayout,
  type SourceDockPanelID
} from './sourceDockLayout.ts';

export const sourceDockviewStorageKey = 'mac-command-bar.source-browser.dockview-layout';
export const sourceDockviewComponentID = 'source-panel';

export type SourceDockviewMigrationSliceID = 'insights-only' | 'context-insights' | 'bottom-runtime';

export type SourceDockviewMigrationSlice = {
  id: SourceDockviewMigrationSliceID;
  rootPanelID: SourceDockPanelID;
  panelIDs: SourceDockPanelID[];
};

export type SourceDockviewPanelDescriptor = {
  id: SourceDockPanelID;
  title: string;
  component: typeof sourceDockviewComponentID;
};

export type SourceDockviewPanelPlan = {
  id: SourceDockPanelID;
  title: string;
  component: typeof sourceDockviewComponentID;
  params: {
    panelID: SourceDockPanelID;
  };
  renderer: 'always';
  initialWidth?: number;
  initialHeight?: number;
  position?: {
    referencePanel: SourceDockPanelID;
    direction: 'left' | 'right' | 'above' | 'below' | 'within';
  };
};

export type SourceDockviewWorkspace = {
  api: DockviewApi;
  setPanelElement(panelID: SourceDockPanelID, element: HTMLElement | null): void;
  syncLayout(layout: SourceDockLayout): void;
  toJSON(): SerializedDockview;
  dispose(): void;
};

export type SourceDockviewWorkspaceOptions = {
  layout: SourceDockLayout;
  panelIDs?: SourceDockPanelID[];
  rootPanelID?: SourceDockPanelID;
  storedLayout?: SerializedDockview | null;
  onDidLayoutChange?: (layout: SerializedDockview) => void;
  onDidPanelClose?: (panelID: SourceDockPanelID, layout: SerializedDockview) => void;
  onDidActivePanelChange?: (panelID: SourceDockPanelID | null, layout: SerializedDockview) => void;
  onDidPanelMove?: (panelID: SourceDockPanelID, layout: SerializedDockview) => void;
};

export const sourceDockviewPanelDescriptors: SourceDockviewPanelDescriptor[] =
  sourceDockPanelDescriptors.map((panel) => ({
    id: panel.id,
    title: panel.label,
    component: sourceDockviewComponentID
  }));

export const sourceDockviewMigrationSlices: SourceDockviewMigrationSlice[] = [
  { id: 'insights-only', rootPanelID: 'insights', panelIDs: ['insights'] },
  { id: 'context-insights', rootPanelID: 'context', panelIDs: ['context', 'insights'] },
  { id: 'bottom-runtime', rootPanelID: 'terminal', panelIDs: ['terminal', 'browser'] }
];

export type SourceDockviewPanelPlanOptions = {
  panelIDs?: SourceDockPanelID[];
  rootPanelID?: SourceDockPanelID;
};

type SourceDockviewPanelPlanRequestOptions = {
  panelIDs?: unknown;
  rootPanelID?: unknown;
};

export function sourceDockviewMigrationSliceStorageKey(
  sliceID: SourceDockviewMigrationSliceID
): string {
  return `${sourceDockviewStorageKey}.${sliceID}`;
}

export function sourceDockviewMigrationSlicePlanOptions(
  sliceID: SourceDockviewMigrationSliceID
): SourceDockviewPanelPlanOptions {
  const slice = sourceDockviewMigrationSlice(sliceID);
  return normalizeSourceDockviewPanelPlanOptions({
    panelIDs: [...slice.panelIDs],
    rootPanelID: slice.rootPanelID
  });
}

export function normalizeSourceDockviewPanelPlanOptions(
  options: SourceDockviewPanelPlanRequestOptions = {}
): SourceDockviewPanelPlanOptions {
  const hasRootPanelID = options.rootPanelID !== undefined;
  const rootPanelID = normalizeSourceDockviewPanelID(
    hasRootPanelID ? options.rootPanelID : 'editor',
    'rootPanelID'
  );

  if (options.panelIDs === undefined) {
    return hasRootPanelID ? { rootPanelID } : {};
  }
  if (!Array.isArray(options.panelIDs)) {
    throw new Error('Dockview panelIDs must be an array');
  }

  const panelIDs: SourceDockPanelID[] = [];
  for (const [index, panelID] of options.panelIDs.entries()) {
    const normalizedPanelID = normalizeSourceDockviewPanelID(panelID, `panelIDs[${index}]`);
    if (!panelIDs.includes(normalizedPanelID)) {
      panelIDs.push(normalizedPanelID);
    }
  }

  if (!panelIDs.includes(rootPanelID)) {
    panelIDs.unshift(rootPanelID);
  }

  return {
    panelIDs,
    rootPanelID
  };
}

function sourceDockviewMigrationSlice(sliceID: SourceDockviewMigrationSliceID): SourceDockviewMigrationSlice {
  const slice = sourceDockviewMigrationSlices.find((candidate) => candidate.id === sliceID);
  if (!slice) {
    throw new Error(`Unknown Dockview migration slice: ${sliceID}`);
  }
  return slice;
}

export function createSourceDockviewPanelPlans(
  layout: SourceDockLayout,
  options: SourceDockviewPanelPlanOptions = {}
): SourceDockviewPanelPlan[] {
  const normalizedLayout = normalizeSourceDockLayout(layout);
  const normalizedOptions = normalizeSourceDockviewPanelPlanOptions(options);
  const plans: SourceDockviewPanelPlan[] = [];
  const plannedPanelIDs = new Set<SourceDockPanelID>();
  const includedPanelIDs = normalizedOptions.panelIDs ? new Set(normalizedOptions.panelIDs) : null;
  const rootPanelID = normalizedOptions.rootPanelID ?? 'editor';
  const rootPanelVisible = normalizedLayout.groups.some((group) => group.panelIDs.includes(rootPanelID));

  if (includedPanelIDs && !rootPanelVisible) {
    return [];
  }

  if (includedPanelIDs && !includedPanelIDs.has(rootPanelID)) {
    includedPanelIDs.add(rootPanelID);
  }

  plans.push(createSourceDockviewPanelPlan(rootPanelID, sourceDockviewPanelTitle(rootPanelID)));
  plannedPanelIDs.add(rootPanelID);

  for (const group of normalizedLayout.groups) {
    const visiblePanelIDs = group.panelIDs.filter(
      (panelID) => panelID !== rootPanelID && (!includedPanelIDs || includedPanelIDs.has(panelID))
    );
    if (visiblePanelIDs.length === 0) continue;

    let previousPanelID: SourceDockPanelID | null =
      rootPanelID !== 'editor' && group.panelIDs.includes(rootPanelID) ? rootPanelID : null;
    for (const panelID of visiblePanelIDs) {
      if (plannedPanelIDs.has(panelID)) continue;

      const position = previousPanelID
        ? { referencePanel: previousPanelID, direction: 'within' as const }
        : firstPanelPositionForGroup(group.id, rootPanelID);
      const plan = createSourceDockviewPanelPlan(panelID, sourceDockviewPanelTitle(panelID), position);

      if (!previousPanelID || previousPanelID === rootPanelID) {
        if (group.id === 'left' || group.id === 'right') {
          plan.initialWidth = group.size;
        }
        if (group.id === 'bottom') {
          plan.initialHeight = group.size;
        }
      }

      plans.push(plan);
      plannedPanelIDs.add(panelID);
      previousPanelID = panelID;
    }
  }

  return plans;
}

export function applySourceDockviewPanelClose(
  layout: SourceDockLayout,
  panelID: SourceDockPanelID
): SourceDockLayout {
  if (panelID === 'editor') return normalizeSourceDockLayout(layout);
  return hideSourceDockPanel(layout, panelID);
}

export function applySourceDockviewActivePanel(
  layout: SourceDockLayout,
  panelID: SourceDockPanelID | null
): SourceDockLayout {
  if (!panelID) return normalizeSourceDockLayout(layout);
  return activateSourceDockPanel(layout, panelID);
}

export async function createSourceDockviewWorkspace(
  container: HTMLElement,
  options: SourceDockviewWorkspaceOptions
): Promise<SourceDockviewWorkspace> {
  const { createDockview } = await import('dockview-core');
  const panelHosts = new Map<SourceDockPanelID, HTMLElement>();
  const panelElements = new Map<SourceDockPanelID, HTMLElement>();
  const disposables: DockviewIDisposable[] = [];
  let synchronizingDockview = false;

  const attachPanelElement = (panelID: SourceDockPanelID) => {
    const host = panelHosts.get(panelID);
    const element = panelElements.get(panelID);
    if (!host || !element || element.parentElement === host) return;

    host.replaceChildren(element);
    element.classList.add('source-dockview-attached-panel');
  };

  const api = createDockview(container, {
    className: 'source-dockview-core',
    defaultRenderer: 'always',
    dndStrategy: 'pointer',
    scrollbars: 'native',
    singleTabMode: 'default',
    getTabContextMenuItems: () => ['close', 'closeOthers', 'separator', 'closeAll'],
    createComponent: () => createSourceDockviewContentRenderer(panelHosts, attachPanelElement)
  });

  container.classList.add('dockview-theme-dark', 'source-dockview-host');

  const runDockviewSync = (sync: () => void) => {
    synchronizingDockview = true;
    try {
      sync();
    } finally {
      synchronizingDockview = false;
    }
  };

  try {
    const storedLayout = options.storedLayout;
    const restorePlans = createSourceDockviewPanelPlans(options.layout, options);
    if (storedLayout && restorePlans.length > 0) {
      runDockviewSync(() => api.fromJSON(storedLayout, { reuseExistingPanels: true }));
    } else {
      runDockviewSync(() => addSourceDockviewPanels(api, options.layout, options));
    }
  } catch {
    runDockviewSync(() => {
      api.clear();
      addSourceDockviewPanels(api, options.layout, options);
    });
  }

  if (options.onDidLayoutChange) {
    disposables.push(
      api.onDidLayoutChange(() => {
        if (synchronizingDockview) return;
        options.onDidLayoutChange?.(api.toJSON());
      })
    );
  }

  disposables.push(
    api.onDidRemovePanel((panel) => {
      const panelID = sourceDockviewPanelID(panel);
      if (synchronizingDockview || !panelID) return;
      options.onDidPanelClose?.(panelID, api.toJSON());
    }),
    api.onDidActivePanelChange((panel) => {
      const panelID = sourceDockviewPanelID(panel);
      if (synchronizingDockview) return;
      options.onDidActivePanelChange?.(panelID, api.toJSON());
    }),
    api.onDidMovePanel((event) => {
      const panelID = sourceDockviewPanelID(event.panel);
      if (synchronizingDockview || !panelID) return;
      options.onDidPanelMove?.(panelID, api.toJSON());
    })
  );

  return {
    api,
    setPanelElement(panelID, element) {
      if (element) {
        panelElements.set(panelID, element);
        attachPanelElement(panelID);
        return;
      }
      panelElements.delete(panelID);
    },
    syncLayout(layout) {
      runDockviewSync(() => addSourceDockviewPanels(api, layout, options));
    },
    toJSON() {
      return api.toJSON();
    },
    dispose() {
      for (const disposable of disposables) {
        disposable.dispose();
      }
      api.dispose();
    }
  };
}

export function addSourceDockviewPanels(
  api: DockviewApi,
  layout: SourceDockLayout,
  options: SourceDockviewPanelPlanOptions = {}
) {
  for (const panel of api.panels) {
    api.removePanel(panel);
  }

  for (const plan of createSourceDockviewPanelPlans(layout, options)) {
    api.addPanel(plan);
  }
}

function createSourceDockviewContentRenderer(
  panelHosts: Map<SourceDockPanelID, HTMLElement>,
  attachPanelElement: (panelID: SourceDockPanelID) => void
): IContentRenderer {
  const element = document.createElement('div');
  element.className = 'source-dockview-panel-host';
  let panelID: SourceDockPanelID | null = null;

  return {
    element,
    init(params: GroupPanelPartInitParameters) {
      const candidatePanelID = params.params?.panelID;
      if (!isSourceDockviewPanelID(candidatePanelID)) {
        element.textContent = 'Unknown panel';
        return;
      }

      panelID = candidatePanelID;
      panelHosts.set(panelID, element);
      attachPanelElement(panelID);
    },
    dispose() {
      if (panelID) {
        panelHosts.delete(panelID);
      }
    },
    layout() {
      element.dispatchEvent(new CustomEvent('source-dockview-layout', { bubbles: true }));
    }
  };
}

function createSourceDockviewPanelPlan(
  panelID: SourceDockPanelID,
  title: string,
  position?: SourceDockviewPanelPlan['position']
): SourceDockviewPanelPlan {
  return {
    id: panelID,
    title,
    component: sourceDockviewComponentID,
    params: { panelID },
    renderer: 'always',
    position
  };
}

function sourceDockviewPanelTitle(panelID: SourceDockPanelID) {
  return sourceDockPanelDescriptors.find((panel) => panel.id === panelID)?.label ?? panelID;
}

function firstPanelPositionForGroup(
  groupID: SourceDockGroupID,
  rootPanelID: SourceDockPanelID = 'editor'
): SourceDockviewPanelPlan['position'] {
  if (rootPanelID !== 'editor') {
    return { referencePanel: rootPanelID, direction: 'within' };
  }

  switch (groupID) {
    case 'left':
      return { referencePanel: 'editor', direction: 'left' };
    case 'right':
      return { referencePanel: 'editor', direction: 'right' };
    case 'bottom':
      return { referencePanel: 'editor', direction: 'below' };
    case 'center':
      return { referencePanel: 'editor', direction: 'above' };
  }
}

function isSourceDockviewPanelID(value: unknown): value is SourceDockPanelID {
  return sourceDockviewPanelDescriptors.some((panel) => panel.id === value);
}

function normalizeSourceDockviewPanelID(value: unknown, fieldName: string): SourceDockPanelID {
  if (isSourceDockviewPanelID(value)) return value;
  throw new Error(`Unknown Dockview panel ID for ${fieldName}: ${String(value)}`);
}

function sourceDockviewPanelID(panel: IDockviewPanel | undefined): SourceDockPanelID | null {
  return isSourceDockviewPanelID(panel?.id) ? panel.id : null;
}
