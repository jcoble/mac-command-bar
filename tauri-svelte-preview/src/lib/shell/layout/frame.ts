/**
 * frame.ts — the /next shell's Gridview root (left rail / center / right
 * context / bottom dock). DOM-only: zero backend IO, zero Svelte imports.
 *
 * Teleport contract: every region's content is a Svelte-owned element that
 * this module MOVES into a dockview-owned host div. dockview never renders or
 * disposes app DOM; disposing the frame moves nothing (Svelte still owns the
 * nodes and the page is unmounting anyway).
 */
import {
  createGridview,
  GridviewPanel,
  Orientation,
  type GridviewApi,
  type IFrameworkPart
} from 'dockview-core';

import {
  clearLayout,
  GRID_LAYOUT_KEY,
  gridPanelIds,
  loadLayout,
  panelSetMatches,
  saveLayout,
  type LayoutStorage
} from './layoutStorage';

export type ShellRegionId = 'rail' | 'center' | 'context' | 'dock';

const REGION_IDS: readonly ShellRegionId[] = ['rail', 'center', 'context', 'dock'];
const COMPONENT = 'shell-region';
const PERSIST_DEBOUNCE_MS = 250;

export interface ShellFrameOptions {
  storage: LayoutStorage;
  regions: Record<ShellRegionId, HTMLElement>;
  onLayoutPersisted?: (ok: boolean) => void;
}

export interface ShellFrame {
  api: GridviewApi;
  resetLayout(): void;
  layout(width: number, height: number): void;
  dispose(): void;
}

/** Gridview panel that adopts a Svelte-owned element into `this.element`. */
class TeleportGridPanel extends GridviewPanel {
  private readonly adopt: (id: string, host: HTMLElement) => void;

  constructor(id: string, component: string, adopt: (id: string, host: HTMLElement) => void) {
    super(id, component);
    this.adopt = adopt;
  }

  protected getComponent(): IFrameworkPart {
    this.adopt(this.id, this.element);
    return {
      update: () => {},
      dispose: () => {}
    };
  }
}

export function createShellFrame(container: HTMLElement, options: ShellFrameOptions): ShellFrame {
  let synchronizing = false;
  let persistTimer: ReturnType<typeof setTimeout> | null = null;
  let disposed = false;

  const adopt = (id: string, host: HTMLElement): void => {
    const region = options.regions[id as ShellRegionId];
    if (!region) return;
    host.classList.add('shell-region-host', `shell-region-host-${id}`);
    if (region.parentElement !== host) host.replaceChildren(region);
  };

  const api = createGridview(container, {
    orientation: Orientation.HORIZONTAL,
    proportionalLayout: true,
    hideBorders: true,
    className: 'shell-grid',
    createComponent: ({ id, name }) => new TeleportGridPanel(id, name, adopt)
  });

  /** The default arrangement; also the fallback whenever restore is unusable. */
  const buildDefault = (): void => {
    api.addPanel({ id: 'center', component: COMPONENT });
    api.addPanel({
      id: 'rail',
      component: COMPONENT,
      position: { direction: 'left', referencePanel: 'center' },
      size: 264,
      minimumWidth: 200,
      maximumWidth: 480
    });
    api.addPanel({
      id: 'context',
      component: COMPONENT,
      position: { direction: 'right', referencePanel: 'center' },
      size: 300,
      minimumWidth: 220,
      maximumWidth: 560
    });
    // Below CENTER only: the dock spans the middle column, not the rails.
    api.addPanel({
      id: 'dock',
      component: COMPONENT,
      position: { direction: 'below', referencePanel: 'center' },
      size: 180,
      minimumHeight: 96
    });
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
    const stored = loadLayout<object>(options.storage, GRID_LAYOUT_KEY);
    if (stored && panelSetMatches(gridPanelIds(stored), REGION_IDS)) {
      try {
        api.fromJSON(stored as never);
        return;
      } catch {
        try {
          api.clear();
        } catch {
          // fall through to a plain rebuild on a fresh container
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
      const ok = saveLayout(options.storage, GRID_LAYOUT_KEY, api.toJSON());
      options.onLayoutPersisted?.(ok);
    }, PERSIST_DEBOUNCE_MS);
  };

  const changeListener = api.onDidLayoutChange(persistSoon);

  return {
    api,
    resetLayout(): void {
      clearLayout(options.storage, GRID_LAYOUT_KEY);
      runSynchronized(() => {
        api.clear();
        buildDefault();
      });
      persistSoon();
    },
    layout(width: number, height: number): void {
      api.layout(width, height);
    },
    dispose(): void {
      disposed = true;
      if (persistTimer !== null) clearTimeout(persistTimer);
      changeListener.dispose();
      api.dispose();
    }
  };
}
