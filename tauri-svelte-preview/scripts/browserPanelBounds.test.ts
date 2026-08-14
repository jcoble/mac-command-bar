/**
 * browserPanelBounds.test.ts — where the native browser view is put.
 *
 * The browser is a native child view positioned by window-space bounds, not a
 * DOM element, so the panel has to translate its own host rectangle into those
 * bounds itself. These are the rules behind that translation, kept as plain
 * functions so they can be exercised without a window.
 *
 * Run: node --experimental-strip-types scripts/browserPanelBounds.test.ts
 */
import assert from 'node:assert/strict';

import {
  boundsForHost,
  expandedBoundsForHost,
  samePlacement,
  usableHostRect
} from '../src/lib/shell/panels/browser/browserPanelBounds.ts';

const window = { width: 1710, height: 990 };

// A rect that is already inside the window and above both minimums is used as
// it is: the common case must not be nudged.
assert.deepEqual(
  boundsForHost({ x: 1100, y: 120, width: 560, height: 700 }, window),
  { x: 1100, y: 120, width: 560, height: 700 }
);

// A narrow panel gets a narrow view. The floating browser has a minimum size;
// a view filling a panel must not, or it is drawn over the pane beside it.
assert.deepEqual(
  boundsForHost({ x: 1400, y: 120, width: 200, height: 100 }, window),
  { x: 1400, y: 120, width: 200, height: 100 }
);

// A rect that would hang off the right edge is pulled back inside.
const overflowing = boundsForHost({ x: 1500, y: 120, width: 500, height: 700 }, window);
assert.equal(overflowing.x + overflowing.width <= window.width, true);
assert.equal(overflowing.x, window.width - overflowing.width);

// Expanding stretches the view leftward to the session rail's right edge. The
// width grows by exactly what the origin moved; the vertical stays put.
const rect = { x: 1100, y: 120, width: 560, height: 700 };
const expanded = expandedBoundsForHost(rect, 320, window);
assert.deepEqual(expanded, { x: 320, y: 120, width: 560 + (1100 - 320), height: 700 });

// A left edge that is already to the right of the rect cannot pull it anywhere:
// the rect comes back unchanged rather than as a negative width.
assert.deepEqual(expandedBoundsForHost(rect, 1300, window), rect);
assert.deepEqual(expandedBoundsForHost(rect, 1100, window), rect);

// A panel that is off screen, mid-layout or collapsed measures as an empty box.
// Those measurements are refused, so the view is hidden rather than left where
// it was, over whatever replaced the panel.
assert.equal(usableHostRect(null), false);
assert.equal(usableHostRect({ x: 0, y: 0, width: 0, height: 0 }), false);
assert.equal(usableHostRect({ x: 1100, y: 120, width: 560, height: 0 }), false);
assert.equal(usableHostRect({ x: Number.NaN, y: 120, width: 560, height: 700 }), false);
assert.equal(usableHostRect({ x: 1100, y: 120, width: 560, height: 700 }), true);

// Re-measuring the same panel asks for the same thing, so nothing is sent. Only
// a moved or resized rectangle, or a change of visibility, is worth a call.
const placed = { kind: 'bounds' as const, bounds: boundsForHost(rect, window) };
assert.equal(samePlacement(placed, { kind: 'bounds', bounds: boundsForHost(rect, window) }), true);
assert.equal(samePlacement(placed, { kind: 'hidden' }), false);
assert.equal(samePlacement({ kind: 'hidden' }, { kind: 'hidden' }), true);
assert.equal(
  samePlacement(placed, {
    kind: 'bounds',
    bounds: boundsForHost({ ...rect, width: rect.width - 40 }, window)
  }),
  false
);

console.log('browserPanelBounds: all checks passed');
