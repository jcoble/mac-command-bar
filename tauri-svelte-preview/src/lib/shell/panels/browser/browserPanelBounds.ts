/**
 * browserPanelBounds.ts — turning the panel's own rectangle into the bounds the
 * native browser view is given.
 *
 * The page in the Browser tab is not a DOM element the panel can lay out. It is
 * a native child view the shell positions by window-space bounds, so the panel
 * has to measure the box it wants filled and hand those numbers over. Expanding
 * is the same idea with a different left edge: the view keeps its top and its
 * height and simply starts further left, over the center pane.
 *
 * PURE: no DOM, no backend call. The measuring happens in the component; the
 * arithmetic happens here so it can be tested without a window.
 */
import {
  browserBoundsEqual,
  clampBrowserFloatingBounds,
  type BrowserWindowSize
} from '../../browser/browserBounds.ts';
import type { BrowserFloatingBounds } from '../../browser/browserTypes.ts';

export type PanelRect = BrowserFloatingBounds;

/**
 * A view filling a panel has no minimum size. The floating browser has one — a
 * window too small to read is not worth showing — but a panel view held to that
 * floor is drawn wider than the panel and covers the pane beside it, which is
 * worse than being small. The panel is the authority on its own size.
 */
export const HOST_MIN_SIZE = { minWidth: 1, minHeight: 1 } as const;

/**
 * The bounds for a view that fills `rect`, kept inside the window.
 */
export function boundsForHost(rect: PanelRect, window: BrowserWindowSize): BrowserFloatingBounds {
  return clampBrowserFloatingBounds(rect, window, HOST_MIN_SIZE);
}

/** What the view is asked to do: fill a rectangle, or be off screen. */
export type HostPlacement =
  | { kind: 'hidden' }
  | { kind: 'bounds'; bounds: BrowserFloatingBounds };

export const HIDDEN_PLACEMENT: HostPlacement = { kind: 'hidden' };

/**
 * Whether two placements ask for the same thing. The panel re-measures on every
 * layout change; only a real difference is worth a call into the shell.
 */
export function samePlacement(a: HostPlacement, b: HostPlacement): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind !== 'bounds' || b.kind !== 'bounds') return true;
  return browserBoundsEqual(a.bounds, b.bounds);
}

/**
 * A rectangle worth handing over: on screen, and big enough to be a rectangle
 * at all. A panel that is hidden or mid-layout measures as an empty box, and a
 * view left where it was is a view drawn over something else.
 */
export function usableHostRect(rect: PanelRect | null): rect is PanelRect {
  return Boolean(
    rect &&
      Number.isFinite(rect.x) &&
      Number.isFinite(rect.y) &&
      rect.width >= 1 &&
      rect.height >= 1
  );
}

/**
 * The same box, stretched leftward to `leftEdge` — the session rail's right
 * edge. The origin moves left and the width grows by exactly that much, so the
 * right edge, the top and the height are all where they were. A `leftEdge` that
 * is already to the right of the box leaves it alone: there is nothing to
 * stretch into, and a negative width would be worse than no expansion at all.
 */
export function expandedBoundsForHost(
  rect: PanelRect,
  leftEdge: number,
  window: BrowserWindowSize
): BrowserFloatingBounds {
  const base = boundsForHost(rect, window);
  const edge = Number.isFinite(leftEdge) ? Math.max(0, Math.round(leftEdge)) : 0;
  const reach = base.x - edge;
  if (reach <= 0) return base;
  return clampBrowserFloatingBounds(
    { x: edge, y: base.y, width: base.width + reach, height: base.height },
    window
  );
}
