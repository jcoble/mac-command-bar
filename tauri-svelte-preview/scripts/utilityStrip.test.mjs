import assert from 'node:assert/strict';

import {
  utilityAnchorFor,
  utilityAnchorStyle
} from '../src/lib/shell/components/utilityStrip.ts';

// The strip measures the button that was pressed and hands the rectangle over;
// the overlay layer turns it into a `position: fixed` box. There is no window
// event and no untrusted payload between the two any more — the strip calls the
// overlay layer directly — so the only thing left to pin is the arithmetic.
const anchor = utilityAnchorFor({ left: 1184, top: 692, width: 32, height: 28 });

assert.deepEqual(anchor, { left: 1184, top: 692, width: 32, height: 28 });
assert.equal(
  utilityAnchorStyle(anchor),
  'left: 1184px; top: 692px; width: 32px; height: 28px;'
);

// A DOMRect carries more than these four numbers; only these four travel.
assert.deepEqual(
  utilityAnchorFor({ left: 0, top: 0, width: 0, height: 0, right: 9, bottom: 9 }),
  { left: 0, top: 0, width: 0, height: 0 }
);

console.log('utilityStrip: the anchor a bottom-strip button hands the overlay layer is verified');
