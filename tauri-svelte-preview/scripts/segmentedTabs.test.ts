/**
 * Pins the one number the segmented tab strip has to get right: where the
 * selected pill sits. The segments touch, so the pill's left edge is the sum of
 * every width before it and its own width is that segment's. An index the strip
 * has not measured yet asks for nothing, so the pill stays hidden instead of
 * flashing at the wrong place on the first paint.
 *
 * Run: node --experimental-strip-types scripts/segmentedTabs.test.ts
 */
import assert from 'node:assert/strict';

import { indicatorFrame } from '../src/lib/shell/components/segmentedTabs.ts';

// --- the pill's frame ------------------------------------------------------

assert.deepEqual(
  indicatorFrame([80, 120, 100], 1),
  { left: 80, width: 120 },
  'the middle segment starts after the first and is as wide as itself'
);

assert.deepEqual(
  indicatorFrame([80, 120, 100], 0),
  { left: 0, width: 80 },
  'the first segment starts at the left edge of the track'
);

assert.deepEqual(
  indicatorFrame([80, 120, 100], 2),
  { left: 200, width: 100 },
  'the last segment starts after both of the others'
);

// --- before anything has been measured -------------------------------------

assert.deepEqual(indicatorFrame([], 0), { left: 0, width: 0 }, 'an unmeasured strip shows no pill');

assert.deepEqual(
  indicatorFrame([80, 120, 100], 3),
  { left: 0, width: 0 },
  'an index past the end shows no pill'
);

assert.deepEqual(
  indicatorFrame([80, 120, 100], -1),
  { left: 0, width: 0 },
  'no selection shows no pill'
);

console.log('segmentedTabs: ok');
