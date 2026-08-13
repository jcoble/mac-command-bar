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
import { clampBrowserFloatingBounds, type BrowserWindowSize } from '../../browser/browserBounds.ts';
import type { BrowserFloatingBounds } from '../../browser/browserTypes.ts';

export type PanelRect = BrowserFloatingBounds;

/**
 * The bounds for a view that fills `rect`, kept inside the window and above the
 * browser's own minimum size by the existing clamping rules.
 */
export function boundsForHost(rect: PanelRect, window: BrowserWindowSize): BrowserFloatingBounds {
  return clampBrowserFloatingBounds(rect, window);
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
