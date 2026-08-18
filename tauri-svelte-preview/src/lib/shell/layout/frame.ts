/**
 * frame.ts — the /next shell's Gridview root. Left to right: the sessions
 * column, the center dock, and the right panel; the bottom dock sits under the
 * center only. DOM-only: zero backend IO, zero Svelte imports.
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
  saveLayout,
  type LayoutStorage
} from './layoutStorage';

export type ShellRegionId = 'sessions' | 'center' | 'tools' | 'dock';

const REGION_IDS: readonly ShellRegionId[] = ['sessions', 'center', 'tools', 'dock'];

/**
 * The regions a stored layout must carry to be worth restoring.
 *
 * The dock is absent from this list because it is absent from the grid
 * whenever the Problems list is not at the bottom — a layout saved in that
 * state has three regions, and demanding four would throw it away and rebuild
 * the default arrangement on every launch, losing both column widths.
 */
const REQUIRED_REGION_IDS: readonly ShellRegionId[] = ['sessions', 'center', 'tools'];

/** Every required region is there, and nothing this frame does not own is. */
function storedRegionsUsable(ids: Iterable<string>): boolean {
  const present = new Set(ids);
  const known: readonly string[] = REGION_IDS;
  return (
    REQUIRED_REGION_IDS.every((id) => present.has(id)) &&
    [...present].every((id) => known.includes(id))
  );
}
const COMPONENT = 'shell-region';
const PERSIST_DEBOUNCE_MS = 250;

/** What the two side columns open at, in px, before anyone drags a divider.
 * The sessions column opens wider than the tool column because its rows carry
 * a provider mark, three lines of text and a reserved column for their hover
 * actions; at 300 the title had 68px to live in and truncated to a word. */
export const SESSIONS_WIDTH = 360;
export const TOOLS_WIDTH = 320;

/**
 * How narrow and how wide the tool column may be dragged. It stops short of
 * the window's edge because the center pane keeps `CENTER_MIN_WIDTH` whatever
 * else happens. A browser page that wants more than this fills the window
 * from inside its own panel rather than by dragging the seam.
 */
export const TOOLS_MIN_WIDTH = 240;
export const TOOLS_MAX_WIDTH = 960;

/** What the middle keeps. A conversation narrower than this is unreadable. */
export const CENTER_MIN_WIDTH = 420;

/** How narrow and how wide the sessions column may be dragged while it is
 * open. Exported because the page puts these back when the column is unfolded
 * — folding it replaces them with a fixed width. */
export const SESSIONS_MIN_WIDTH = 220;
export const SESSIONS_MAX_WIDTH = 560;

/** What the bottom strip opens at. */
export const DOCK_HEIGHT = 180;

/** The width of the sessions column folded up: one icon-sized cell per
 * session and nothing else. Fixed the same way the surface rail is, so the
 * divider beside a folded column cannot be dragged. */
export const SESSIONS_STRIP_WIDTH = 52;

export interface ShellFrameOptions {
  storage: LayoutStorage;
  regions: Record<ShellRegionId, HTMLElement>;
  onLayoutPersisted?: (ok: boolean) => void;
}

/** How narrow and how wide a region may be dragged. Both optional: leaving one
 * out keeps whatever limit the region already has. */
export interface RegionWidthLimits {
  minimumWidth?: number;
  maximumWidth?: number;
}

/** How short and how tall a region may be dragged. The dock is the only region
 * laid out vertically, and this is what lets it be closed entirely. */
export interface RegionHeightLimits {
  minimumHeight?: number;
  maximumHeight?: number;
}

export interface ShellFrame {
  api: GridviewApi;
  resetLayout(): void;
  /**
   * Give one region a width, optionally changing what it may be dragged to.
   *
   * This is how a column asks to be folded up or opened out: the width a
   * region takes is the grid's business, not the column's, so nothing hides
   * content behind a CSS width of its own. The limits are applied first —
   * asking for 52px while the region's own minimum is still 220 would simply
   * be clamped back to 220.
   */
  setRegionWidth(id: ShellRegionId, width: number, limits?: RegionWidthLimits): void;
  /**
   * Give one region a height, optionally changing what it may be dragged to.
   * The mirror of `setRegionWidth`, for the bottom dock — the one region this
   * frame lays out vertically. Closing the dock means asking for a height of
   * zero, which a minimum height would otherwise clamp back up, so the limits
   * are applied first here for the same reason they are there.
   */
  setRegionHeight(id: ShellRegionId, height: number, limits?: RegionHeightLimits): void;
  /**
   * Put the bottom dock in the grid, or take it out.
   *
   * Closing the dock is not the same as making it short. A region of zero
   * height is still a region: the grid keeps its divider, and the middle
   * column keeps the room the divider and the empty view sit in — measured at
   * 16px above the status bar with the strip shut. Removing the region is what
   * gives that room back, and adding it again is what a reopened strip means.
   *
   * The dock's content is Svelte-owned and only borrowed by the grid, so it
   * survives the round trip: removal detaches it, and the next add moves it
   * back into the new host.
   */
  setDockPresent(present: boolean): void;
  /**
   * Say what a region may be dragged to, without touching the width it has.
   *
   * A stored layout carries each region's limits as well as its width, so a
   * layout written while a column was folded restores the column locked at
   * strip width — minimum and maximum both 52px, which is also a divider that
   * cannot be dragged. That is right while the column is meant to be folded and
   * wrong the moment it is not, and the two facts are stored separately and can
   * disagree. This is how the page says "open, and draggable again" on restore
   * while leaving a width the user chose alone.
   */
  setRegionLimits(id: ShellRegionId, limits: RegionWidthLimits): void;
  /** How wide a region is right now, or null if there is no such region. Used
   * to tell a width the user dragged from one restored below its own minimum. */
  regionWidth(id: ShellRegionId): number | null;
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

  /** Ask one region for a width, and for what it may be dragged to. A region
   * that will not take it keeps the width it has: a default layout a few pixels
   * off is not worth failing a launch. */
  const setRegionWidth = (
    id: ShellRegionId,
    width: number,
    limits?: RegionWidthLimits
  ): void => {
    try {
      const panel = api.getPanel(id);
      if (!panel) return;
      if (limits) panel.api.setConstraints(limits);
      panel.api.setSize({ width });
    } catch {
      // nothing to do — the arrangement is still usable
    }
  };

  /** Ask one region for a height, and for what it may be dragged to. Same
   * tolerance as `setRegionWidth`. */
  const setRegionHeight = (
    id: ShellRegionId,
    height: number,
    limits?: RegionHeightLimits
  ): void => {
    try {
      const panel = api.getPanel(id);
      if (!panel) return;
      if (limits) panel.api.setConstraints(limits);
      panel.api.setSize({ height });
    } catch {
      // nothing to do — the arrangement is still usable
    }
  };

  /** Say what a region may be dragged to and nothing else. Same tolerance as
   * `setRegionWidth`: a region that will not take it keeps what it has. */
  const setRegionLimits = (id: ShellRegionId, limits: RegionWidthLimits): void => {
    try {
      api.getPanel(id)?.api.setConstraints(limits);
    } catch {
      // nothing to do — the arrangement is still usable
    }
  };

  /** How wide a region is right now, or null if it is not there. */
  const regionWidth = (id: ShellRegionId): number | null => {
    try {
      return api.getPanel(id)?.api.width ?? null;
    } catch {
      return null;
    }
  };

  /**
   * Below CENTER only: the dock spans the middle column, not the side columns.
   *
   * No minimum height, on purpose. The Problems list can be moved to the tool
   * column or hidden altogether, and both answers close this strip completely —
   * a minimum of 96px would clamp that back to a 96px strip of nothing.
   */
  const addDock = (): void => {
    api.addPanel({
      id: 'dock',
      component: COMPONENT,
      position: { direction: 'below', referencePanel: 'center' },
      size: DOCK_HEIGHT,
      minimumHeight: 0
    });
  };

  /** The default arrangement; also the fallback whenever restore is unusable. */
  const buildDefault = (): void => {
    api.addPanel({ id: 'center', component: COMPONENT, minimumWidth: CENTER_MIN_WIDTH });
    // The left side belongs to the sessions list and nothing else.
    api.addPanel({
      id: 'sessions',
      component: COMPONENT,
      position: { direction: 'left', referencePanel: 'center' },
      size: SESSIONS_WIDTH,
      minimumWidth: SESSIONS_MIN_WIDTH,
      maximumWidth: SESSIONS_MAX_WIDTH
    });
    // Every panel — files, source control, worktrees, run, context, agents,
    // browser, history — shares one column on the right, and only one of them
    // is open at a time.
    api.addPanel({
      id: 'tools',
      component: COMPONENT,
      position: { direction: 'right', referencePanel: 'center' },
      size: TOOLS_WIDTH,
      minimumWidth: TOOLS_MIN_WIDTH,
      maximumWidth: TOOLS_MAX_WIDTH
    });
    addDock();
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
    if (stored && storedRegionsUsable(gridPanelIds(stored))) {
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

  const setDockPresent = (present: boolean): void => {
    try {
      const panel = api.getPanel('dock');
      if (present === Boolean(panel)) return;
      // Both side widths are read before the change and asked for again after,
      // for the reason `buildDefault` gives: adding the dock lifts the middle
      // out of the row and puts it back as a column, and the width it gives up
      // comes back unevenly. Removing it does the same in reverse.
      const sessions = regionWidth('sessions');
      const tools = regionWidth('tools');
      runSynchronized(() => {
        if (present) addDock();
        else if (panel) api.removePanel(panel);
        if (sessions !== null) setRegionWidth('sessions', sessions);
        if (tools !== null) setRegionWidth('tools', tools);
      });
      // Same ordering as `resetLayout`: the guard releases on a timer, so the
      // persist is asked for on the timer scheduled after it.
      setTimeout(persistSoon, 0);
    } catch {
      // nothing to do — the arrangement is still usable
    }
  };

  return {
    api,
    setRegionWidth,
    setRegionHeight,
    setDockPresent,
    setRegionLimits,
    regionWidth,
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
