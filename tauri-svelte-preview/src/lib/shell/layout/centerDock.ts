/**
 * centerDock.ts — the /next center tab area (one Dockview). DOM-only: zero
 * backend IO, zero Svelte imports. Same teleport contract as frame.ts, plus
 * an explicit "return to parking" on panel close so Svelte-owned content
 * (the terminal surface!) is never destroyed with a dockview renderer.
 */
import {
  createDockview,
  themeDracula,
  type DockviewApi,
  type GroupPanelPartInitParameters,
  type IContentRenderer
} from 'dockview-core';

import {
  CENTER_LAYOUT_KEY,
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
}

export interface CenterDockOptions {
  storage: LayoutStorage;
  panels: CenterPanelSpec[];
  onPanelLayout?: (id: string) => void;
  onLayoutPersisted?: (ok: boolean) => void;
}

export interface CenterDock {
  api: DockviewApi;
  activatePanel(id: string): void;
  resetLayout(): void;
  dispose(): void;
}

const COMPONENT = 'center-panel';
const PERSIST_DEBOUNCE_MS = 250;

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

  const addPanelFor = (panel: CenterPanelSpec): void => {
    api.addPanel({
      id: panel.id,
      title: panel.title,
      component: COMPONENT,
      params: { panelId: panel.id }
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

  const buildDefault = (): void => {
    for (const panel of options.panels) addPanelFor(panel);
    const first = options.panels[0];
    if (first) panelById(first.id)?.api.setActive();
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
   * The three roster tabs are permanent for now. dockview puts a close button on
   * every tab, and closing one hands its content back to the parking stage with
   * no way left in the UI to bring it back — for the Session tab that is a LIVE
   * terminal. Put the tab straight back instead, so the close button is a no-op.
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
    buildDefault();
  });

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

  const listeners = [
    api.onDidLayoutChange(persistSoon),
    api.onDidRemovePanel((panel) => keepRosterPanel(panel.id))
  ];

  return {
    api,
    activatePanel(id: string): void {
      panelById(id)?.api.setActive();
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
