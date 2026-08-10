/**
 * centerDock.ts — the /next center surface area (one Dockview). DOM-only: zero
 * backend IO, zero Svelte imports. Same teleport contract as frame.ts, plus
 * an explicit "return to parking" on panel close so Svelte-owned content
 * (the terminal surface!) is never destroyed with a dockview renderer.
 */
import {
  createDockview,
  themeDracula,
  type AddPanelPositionOptions,
  type DockviewPanelRenderer,
  type DockviewApi,
  type GroupPanelPartInitParameters,
  type IContentRenderer
} from 'dockview-core';

import {
  CENTER_LAYOUT_KEY,
  CENTER_LAYOUT_KEY_V4,
  clearLayout,
  dockPanelIds,
  loadLayout,
  panelSetMatches,
  saveLayout,
  type LayoutStorage
} from './layoutStorage';

export interface CenterPanelSpec {
  id: string;
  title: string;
  element: HTMLElement;
  /** Override the dock-wide attachment policy for surfaces that cannot remain
   * in the overlay container while another tab is active. */
  renderer?: DockviewPanelRenderer;
  /**
   * Which side of the default layout this panel opens on: `'conversation'` (the
   * left group, where the user talks to the session) or `'display'` (the right
   * group, where results are shown). Only read when the dock is built from
   * scratch — once the user has dragged tabs around, the stored layout wins.
   * Defaults to `'conversation'`.
   */
  group?: 'conversation' | 'display';
}

export interface CenterDockOptions {
  storage: LayoutStorage;
  panels: CenterPanelSpec[];
  onPanelLayout?: (id: string) => void;
  /**
   * The tab the user is looking at changed. Dockview delivers this through a
   * microtask, so the events caused by building or restoring the dock arrive
   * shortly after `createCenterDock` returns — a listener that loads data must
   * ignore those and start honouring the signal one timer tick later. See the
   * note on `runSynchronized` below for why a microtask cannot be told apart
   * from a real click any other way.
   */
  onPanelActivated?: (id: string) => void;
  onLayoutPersisted?: (ok: boolean) => void;
}

export interface CenterDock {
  api: DockviewApi;
  activatePanel(id: string): void;
  captureLayout(): CenterDockSnapshot | null;
  restoreLayout(snapshot: CenterDockSnapshot | null | undefined): void;
  resetLayout(): void;
  dispose(): void;
}

export interface CenterDockSnapshot {
  activePanelId: string | null;
  layout: object;
}

const COMPONENT = 'center-panel';
const PERSIST_DEBOUNCE_MS = 250;
export const CENTER_PANEL_IDS = [
  'session',
  'editor',
  'browser',
  'diff',
  'session-library',
  'agents'
] as const;
export type CenterPanelId = (typeof CENTER_PANEL_IDS)[number];

const CENTER_PANEL_IDS_V4 = CENTER_PANEL_IDS.filter((id) => id !== 'agents');

export function isCenterPanelId(id: string): id is CenterPanelId {
  return CENTER_PANEL_IDS.some((candidate) => candidate === id);
}

export function createCenterDock(container: HTMLElement, options: CenterDockOptions): CenterDock {
  const specs = new Map(options.panels.map((panel) => [panel.id, panel]));
  /** Where each element goes back to when its dockview panel dies. */
  const parking = new Map(
    options.panels.map((panel) => [panel.id, panel.element.parentElement as HTMLElement | null])
  );
  let synchronizingDepth = 0;
  let persistTimer: ReturnType<typeof setTimeout> | null = null;
  let disposed = false;
  /** Roster tabs whose re-add is already queued, so a burst cannot double-add. */
  const readding = new Set<string>();

  const createRenderer = (): IContentRenderer => {
    const element = document.createElement('div');
    element.className = 'center-panel-host';
    let panelId: string | null = null;
    return {
      element,
      init(parameters: GroupPanelPartInitParameters): void {
        const candidate = parameters.params?.panelId;
        if (typeof candidate !== 'string' || !specs.has(candidate)) {
          element.textContent = `Unknown panel: ${String(candidate)}`;
          return;
        }
        panelId = candidate;
        const content = specs.get(candidate)!.element;
        if (content.parentElement !== element) element.replaceChildren(content);
      },
      layout(): void {
        if (panelId) options.onPanelLayout?.(panelId);
      },
      dispose(): void {
        if (disposed || !panelId) return;
        // Panel closed while the shell lives on: hand the content back to its
        // Svelte-owned parking node so nothing app-owned dies with dockview.
        const content = specs.get(panelId)?.element;
        const home = parking.get(panelId) ?? null;
        if (content && home && content.parentElement !== home) home.appendChild(content);
      }
    };
  };

  const api = createDockview(container, {
    className: 'shell-center-dock',
    defaultRenderer: 'always',
    dndStrategy: 'pointer',
    hideBorders: true,
    scrollbars: 'native',
    theme: { ...themeDracula, gap: 0 },
    createComponent: createRenderer
  });

  /**
   * Add one roster panel. With no `position` dockview puts it in whichever group
   * is active (and makes a group if the dock is empty) — that is what a re-add
   * after an accidental close wants, and it is the fallback whenever there is no
   * existing panel to anchor against.
   */
  const addPanelFor = (panel: CenterPanelSpec, position?: AddPanelPositionOptions): void => {
    api.addPanel({
      id: panel.id,
      // Keep the persisted panel id/registration stable while giving the
      // dedicated history experience the title users see in the tab.
      title: panel.id === 'session-library' ? 'Session History' : panel.title,
      component: COMPONENT,
      params: { panelId: panel.id },
      renderer: panel.renderer,
      position
    });
  };

  /** Panel-id lookup. `api.getPanel(id)` also does this correctly — but the
   * naming is INVERTED between dockview's layers (`DockviewApi.getPanel` calls
   * the component's `getGroupPanel`, while `DockviewApi.getGroup` calls the
   * component's `getPanel`, which looks up groups), and that trap has already
   * produced one confident misreading in each direction during review. The
   * explicit form below is the exact body of `getGroupPanel` and of
   * `addPanel`'s own duplicate guard, so what we test is what dockview does. */
  const panelById = (id: string) => api.panels.find((panel) => panel.id === id);

  /** Existing serialized layouts may still carry the former tab label. */
  const normalizeSessionLibraryTitle = (): void => {
    panelById('session-library')?.api.setTitle('Session History');
  };

  /** Old saved layouts predate per-panel renderers, so enforce the current
   * roster contract after either restore or build. */
  const normalizePanelRenderers = (): void => {
    for (const panel of options.panels) {
      if (panel.renderer) panelById(panel.id)?.api.setRenderer(panel.renderer);
    }
  };

  /**
   * The layout a fresh dock opens with: the conversation panels on the left, the
   * display panels stacked as tabs in a second group to their right. Both sides
   * may be empty — a roster that is all conversation just never opens the second
   * group, and one that is all display opens a single group with no anchor to
   * sit beside (asking dockview to position against a panel that does not exist
   * yet throws, so that case adds the first panel with no position at all).
   */
  const buildDefault = (): void => {
    const conversation = options.panels.filter((panel) => panel.group !== 'display');
    const display = options.panels.filter((panel) => panel.group === 'display');
    for (const panel of conversation) addPanelFor(panel);

    const anchor = conversation[0];
    const [leadDisplay, ...stackedDisplay] = display;
    if (leadDisplay) {
      addPanelFor(
        leadDisplay,
        anchor ? { referencePanel: anchor.id, direction: 'right' } : undefined
      );
      for (const panel of stackedDisplay) {
        addPanelFor(panel, { referencePanel: leadDisplay.id, direction: 'within' });
      }
    }

    // Every add above made its own panel the active one, so the last display tab
    // is showing — which is now Diff, an empty pane until a file is picked. Put
    // the display group back on its first tab, then hand focus to the front of
    // the roster.
    if (leadDisplay) panelById(leadDisplay.id)?.api.setActive();
    const opening = anchor ?? leadDisplay;
    if (opening) panelById(opening.id)?.api.setActive();
  };

  /**
   * Run a programmatic layout mutation with persistence and the roster rebuild
   * below both suppressed.
   *
   * dockview reports layout changes through `queueMicrotask` (its `AsapEvent`),
   * so the events this block causes are delivered AFTER it returns — a flag
   * cleared synchronously is already down when they land, which is why the
   * previous version of this guard never suppressed anything. Releasing it on a
   * timer instead is what makes it real: the whole microtask queue (including
   * microtasks queued by other microtasks) drains before any timer callback
   * runs, so every event delivered that way lands while the guard is still up.
   *
   * That covers the microtask channel only, which is the one a programmatic
   * mutation uses. Resize-driven changes are delivered separately, through a
   * `requestAnimationFrame` inside dockview's own resize watcher, and those
   * deliberately fall outside the guard: they report a finished layout the user
   * asked for, which is exactly what we want written.
   */
  const runSynchronized = (fn: () => void): void => {
    synchronizingDepth += 1;
    try {
      fn();
    } finally {
      setTimeout(() => {
        if (synchronizingDepth > 0) synchronizingDepth -= 1;
      }, 0);
    }
  };

  /**
   * The six roster surfaces are permanent for now. Dockview still owns their
   * lifecycle even though its horizontal headers are visually replaced by the
   * shell's top surface strip. If a panel is removed through a restored layout
   * or API call, put it straight back so the strip never points at an
   * unavailable surface — especially the live Session terminal.
   *
   * Deferred by a timer on purpose: dockview fires this event from
   * `doRemovePanel`, BEFORE it disposes the panel, and that dispose is what
   * returns the content to parking. Re-adding synchronously would be undone a
   * moment later, stranding the content for good.
   */
  const keepRosterPanel = (id: string): void => {
    if (disposed || synchronizingDepth > 0 || readding.has(id)) return;
    if (!specs.has(id)) return;
    readding.add(id);
    setTimeout(() => {
      readding.delete(id);
      const spec = specs.get(id);
      // Duplicate-add guard: tests the exact condition `addPanel` itself
      // throws on (see the `panelById` note above).
      if (disposed || synchronizingDepth > 0 || !spec || panelById(id)) return;
      try {
        addPanelFor(spec);
      } catch {
        // Nothing here can recover a dock that refuses the panel, and an
        // uncaught throw in a timer callback goes nowhere useful. The tab stays
        // shut; "Reset layout" rebuilds the roster.
      }
    }, 0);
  };

  /**
   * Size the dock to its container BEFORE anything is restored or built, for the
   * same reason as the grid in `frame.ts`: dockview starts at 0 x 0, and a
   * restore at that size records squashed proportions that the first real layout
   * then scales up wrong. A container with no size yet is skipped — dockview's
   * own resize observer delivers the first real layout, and `persistSoon`
   * refuses to write a dock that has never had a real size.
   */
  const layoutToContainer = (): void => {
    const width = container.clientWidth;
    const height = container.clientHeight;
    if (width > 0 && height > 0) api.layout(width, height);
  };

  layoutToContainer();

  let migratedFromV4 = false;

  runSynchronized(() => {
    const stored = loadLayout<object>(options.storage, CENTER_LAYOUT_KEY);
    if (stored && panelSetMatches(dockPanelIds(stored), specs.keys())) {
      try {
        api.fromJSON(stored as never);
        return;
      } catch {
        try {
          api.clear();
        } catch {
          // fall through
        }
      }
    }

    /**
     * v4 has the same arrangement as the current roster minus Agents. Restore
     * that exact set first, then add the new tab beside Session Library through
     * the live Dockview API. This preserves the serialized groups/sizes/order
     * and avoids hand-editing Dockview's private grid tree. Any mismatch or
     * Dockview rejection falls through to the safe current defaults.
     */
    const previous = loadLayout<object>(options.storage, CENTER_LAYOUT_KEY_V4);
    if (previous && panelSetMatches(dockPanelIds(previous), CENTER_PANEL_IDS_V4)) {
      try {
        api.fromJSON(previous as never);
        const activePanelId = api.activePanel?.id ?? null;
        const library = specs.get('session-library');
        const agents = specs.get('agents');
        if (!library || !agents || !panelById('session-library')) {
          throw new Error('Agents migration roster is incomplete');
        }
        addPanelFor(agents, { referencePanel: 'session-library', direction: 'within' });
        if (activePanelId) panelById(activePanelId)?.api.setActive();
        migratedFromV4 = true;
        return;
      } catch {
        try {
          api.clear();
        } catch {
          // fall through
        }
      }
    }

    buildDefault();
  });
  normalizeSessionLibraryTitle();
  normalizePanelRenderers();

  const persistSoon = (): void => {
    if (synchronizingDepth > 0 || disposed) return;
    if (persistTimer !== null) clearTimeout(persistTimer);
    persistTimer = setTimeout(() => {
      persistTimer = null;
      if (disposed) return;
      // Never store a dock measured at zero: the sizes in it are meaningless and
      // the next launch would restore from them.
      if (api.width <= 0 || api.height <= 0) return;
      let ok = false;
      try {
        // `toJSON` runs inside the guard too: a dock in an unexpected state can
        // throw from it, and an unhandled throw in here kills the timer.
        ok = saveLayout(options.storage, CENTER_LAYOUT_KEY, api.toJSON());
      } catch {
        ok = false;
      }
      options.onLayoutPersisted?.(ok);
    }, PERSIST_DEBOUNCE_MS);
  };

  // The migrated layout is already a complete current-roster Dockview tree.
  // Persist it under v5 immediately so a reload does not repeat the migration;
  // the old v4 payload remains untouched as a harmless fallback record.
  if (migratedFromV4) {
    try {
      saveLayout(options.storage, CENTER_LAYOUT_KEY, api.toJSON());
    } catch {
      // A quota/serialization failure is benign; the live layout remains usable
      // and the next launch safely retries from the old v3 record.
    }
  }

  const listeners = [
    api.onDidLayoutChange(persistSoon),
    api.onDidRemovePanel((panel) => keepRosterPanel(panel.id)),
    api.onDidActivePanelChange((panel) => {
      if (disposed || !panel) return;
      options.onPanelActivated?.(panel.id);
    })
  ];

  return {
    api,
    activatePanel(id: string): void {
      panelById(id)?.api.setActive();
    },
    captureLayout(): CenterDockSnapshot | null {
      try {
        return {
          activePanelId: api.activePanel?.id ?? null,
          layout: api.toJSON() as object
        };
      } catch {
        return null;
      }
    },
    restoreLayout(snapshot: CenterDockSnapshot | null | undefined): void {
      // Sessions saved before per-session layouts existed have no layout to
      // restore. Leaving the live dock alone keeps every roster tab visible;
      // the next capture gives that session its own starting arrangement.
      if (!snapshot) return;
      // Keep the one live six-tab roster mounted. Replaying a serialized
      // Dockview tree during a session switch intermittently retained the
      // panel bodies but dropped their tab renderers. The active tab is the
      // session-specific state the reader needs; editor/browser/diff content is
      // restored by their own stores without replacing this navigation tree.
      if (snapshot.activePanelId) panelById(snapshot.activePanelId)?.api.setActive();
    },
    resetLayout(): void {
      clearLayout(options.storage, CENTER_LAYOUT_KEY);
      runSynchronized(() => {
        api.clear();
        buildDefault();
      });
      // The guard above is still up — it releases on a timer — so ask for the
      // persist on the timer after it. Callbacks with the same delay run in the
      // order they were scheduled, and the release was scheduled first.
      setTimeout(persistSoon, 0);
    },
    dispose(): void {
      disposed = true; // renderer dispose() no-ops: page teardown owns the DOM now
      if (persistTimer !== null) clearTimeout(persistTimer);
      for (const listener of listeners) listener.dispose();
      api.dispose();
    }
  };
}
