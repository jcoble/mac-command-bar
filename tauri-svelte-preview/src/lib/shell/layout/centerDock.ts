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
  let synchronizing = false;
  let persistTimer: ReturnType<typeof setTimeout> | null = null;
  let disposed = false;

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

  const buildDefault = (): void => {
    for (const panel of options.panels) {
      api.addPanel({
        id: panel.id,
        title: panel.title,
        component: COMPONENT,
        params: { panelId: panel.id }
      });
    }
    const first = options.panels[0];
    if (first) api.getPanel(first.id)?.api.setActive();
  };

  const runSynchronized = (fn: () => void): void => {
    synchronizing = true;
    try {
      fn();
    } finally {
      synchronizing = false;
    }
  };

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
    if (synchronizing || disposed) return;
    if (persistTimer !== null) clearTimeout(persistTimer);
    persistTimer = setTimeout(() => {
      persistTimer = null;
      if (disposed) return;
      const ok = saveLayout(options.storage, CENTER_LAYOUT_KEY, api.toJSON());
      options.onLayoutPersisted?.(ok);
    }, PERSIST_DEBOUNCE_MS);
  };

  const listeners = [api.onDidLayoutChange(persistSoon)];

  return {
    api,
    activatePanel(id: string): void {
      api.getPanel(id)?.api.setActive();
    },
    resetLayout(): void {
      clearLayout(options.storage, CENTER_LAYOUT_KEY);
      runSynchronized(() => {
        api.clear();
        buildDefault();
      });
      persistSoon();
    },
    dispose(): void {
      disposed = true; // renderer dispose() no-ops: page teardown owns the DOM now
      if (persistTimer !== null) clearTimeout(persistTimer);
      for (const listener of listeners) listener.dispose();
      api.dispose();
    }
  };
}
