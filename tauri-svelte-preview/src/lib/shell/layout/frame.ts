/**
 * frame.ts — the /next shell's Gridview root. Left to right: the sessions
 * column, the center dock, the tool views, and the icon strip on the far right
 * edge; the bottom dock sits under the center only. DOM-only: zero backend IO,
 * zero Svelte imports.
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

export type ShellRegionId = 'sessions' | 'center' | 'tools' | 'activity' | 'dock';

const REGION_IDS: readonly ShellRegionId[] = ['sessions', 'center', 'tools', 'activity', 'dock'];
const COMPONENT = 'shell-region';
const PERSIST_DEBOUNCE_MS = 250;

/** Width of the icon strip, in px. Matches the strip's own CSS width — one
 * icon button wide and no wider. */
const ACTIVITY_STRIP_WIDTH = 44;
/** What the two side columns open at, in px, before anyone drags a divider. */
const SESSIONS_WIDTH = 300;
const TOOLS_WIDTH = 320;

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
  let synchronizingDepth = 0;
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

  /** Ask one region for a width. A region that will not take it keeps the width
   * it has: a default layout a few pixels off is not worth failing a launch. */
  const setRegionWidth = (id: ShellRegionId, width: number): void => {
    try {
      api.getPanel(id)?.api.setSize({ width });
    } catch {
      // nothing to do — the arrangement is still usable
    }
  };

  /** The default arrangement; also the fallback whenever restore is unusable. */
  const buildDefault = (): void => {
    api.addPanel({ id: 'center', component: COMPONENT });
    // The left side belongs to the sessions list and nothing else.
    api.addPanel({
      id: 'sessions',
      component: COMPONENT,
      position: { direction: 'left', referencePanel: 'center' },
      size: SESSIONS_WIDTH,
      minimumWidth: 220,
      maximumWidth: 560
    });
    // The icon strip that picks which tool view is open, hard against the right
    // edge of the window. Its width is fixed — the same number as its minimum
    // and its maximum — so the divider beside it cannot be dragged and a window
    // resize leaves it exactly one icon wide.
    //
    // It is added BEFORE the tool column even though it ends up outside it. A
    // new region takes its width out of the region it is added against, so
    // adding the strip against the tool column would have carved the strip out
    // of the 320px the column is meant to have. Adding both against the center
    // instead — the strip first, then the column, which lands between them —
    // takes both widths out of the middle, where there is room to spare.
    api.addPanel({
      id: 'activity',
      component: COMPONENT,
      position: { direction: 'right', referencePanel: 'center' },
      size: ACTIVITY_STRIP_WIDTH,
      minimumWidth: ACTIVITY_STRIP_WIDTH,
      maximumWidth: ACTIVITY_STRIP_WIDTH
    });
    // Every tool view — files, source control, worktrees, context — shares one
    // column on the right, and only one of them is open at a time.
    api.addPanel({
      id: 'tools',
      component: COMPONENT,
      position: { direction: 'right', referencePanel: 'center' },
      size: TOOLS_WIDTH,
      minimumWidth: 240,
      maximumWidth: 640
    });
    // Below CENTER only: the dock spans the middle column, not the side columns.
    api.addPanel({
      id: 'dock',
      component: COMPONENT,
      position: { direction: 'below', referencePanel: 'center' },
      size: 180,
      minimumHeight: 96
    });
    // Say the two side widths again, now that every region exists.
    //
    // The dock above is what makes this necessary: putting it under the middle
    // turns the middle from one region into a column of two, and dockview does
    // that by lifting the middle out of the row and putting it back. The width
    // it gave up goes round the other regions on the way out and comes back
    // unevenly — in practice the tool column ends up squashed to its minimum and
    // the sessions column keeps the difference. Asking for the two widths once
    // the arrangement is finished settles them where they were meant to be.
    setRegionWidth('sessions', SESSIONS_WIDTH);
    setRegionWidth('tools', TOOLS_WIDTH);
  };

  /**
   * Run a programmatic layout mutation with persistence suppressed.
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
   * Size the grid to its container BEFORE anything is restored or built.
   * dockview's constructor starts the grid at 0 x 0, and a restore into a
   * zero-width grid clamps every region to its minimum and remembers those
   * squashed proportions — which the first real layout then scales up wrong.
   * (This mirrors the old shell's `layoutDockviewApiToContainer`.)
   *
   * A container with no size yet — the route is hidden, say — is skipped rather
   * than forced: the first real layout does the job, and `persistSoon` refuses
   * to write a grid that has never had a real size.
   */
  const layoutToContainer = (): void => {
    const width = container.clientWidth;
    const height = container.clientHeight;
    if (width > 0 && height > 0) api.layout(width, height);
  };

  layoutToContainer();

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
    if (synchronizingDepth > 0 || disposed) return;
    if (persistTimer !== null) clearTimeout(persistTimer);
    persistTimer = setTimeout(() => {
      persistTimer = null;
      if (disposed) return;
      // Never store a grid measured at zero: every region in it sits at its
      // minimum, and the next launch scales those wrong sizes up to the window.
      if (api.width <= 0 || api.height <= 0) return;
      let ok = false;
      try {
        // `toJSON` runs inside the guard too: a grid in an unexpected state can
        // throw from it, and an unhandled throw in here kills the timer.
        ok = saveLayout(options.storage, GRID_LAYOUT_KEY, api.toJSON());
      } catch {
        ok = false;
      }
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
      // The guard above is still up — it releases on a timer — so ask for the
      // persist on the timer after it. Callbacks with the same delay run in the
      // order they were scheduled, and the release was scheduled first.
      setTimeout(persistSoon, 0);
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
