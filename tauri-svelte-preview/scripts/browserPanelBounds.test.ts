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
  expandedBoundsForHost
} from '../src/lib/shell/panels/browser/browserPanelBounds.ts';

const window = { width: 1710, height: 990 };

// A rect that is already inside the window and above both minimums is used as
// it is: the common case must not be nudged.
assert.deepEqual(
  boundsForHost({ x: 1100, y: 120, width: 560, height: 700 }, window),
  { x: 1100, y: 120, width: 560, height: 700 }
);

// Below the 360px floor the view comes back at the floor, otherwise the page
// inside it is unusable.
assert.equal(boundsForHost({ x: 1400, y: 120, width: 200, height: 700 }, window).width, 360);

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

console.log('browserPanelBounds: all checks passed');
