/**
 * browserAnnotationComposite.test.ts — the rules behind marking up a page.
 *
 * The drawing surface is a canvas and the burn-in step needs one, so the parts
 * that decide *what* is drawn — which marks are still live, which one an erase
 * click removes, how marks land on the captured image's pixel grid, and what
 * the note beside the picture says — are plain functions. Those are what this
 * test covers; `compositeAnnotations` itself needs a real canvas and is left to
 * the app.
 *
 * Run: node --experimental-strip-types scripts/browserAnnotationComposite.test.ts
 */
import assert from 'node:assert/strict';

import {
  liveShapes,
  scaleShapes,
  shapeAtPoint,
  type AnnotationShape
} from '../src/lib/shell/panels/browser/annotationComposite.ts';
import { formatAttachmentNote } from '../src/lib/shell/panels/browser/browserAttachmentNote.ts';

// ── liveShapes ───────────────────────────────────────────────────────────────

const drawn: { id: string; shape: AnnotationShape }[] = [
  { id: 'a', shape: { kind: 'region', x: 10, y: 10, width: 100, height: 60 } },
  { id: 'b', shape: { kind: 'region', x: 200, y: 10, width: 0.4, height: 60 } },
  { id: 'c', shape: { kind: 'stroke', points: [{ x: 5, y: 5 }] } },
  { id: 'd', shape: { kind: 'stroke', points: [{ x: 5, y: 5 }, { x: 5, y: 90 }] } },
  { id: 'e', shape: { kind: 'element', x: 300, y: 40, width: 80, height: 30, tag: 'svg' } }
];

assert.deepEqual(
  liveShapes(drawn, new Set(['e'])).map((item) => item.id),
  ['a', 'd'],
  'erased ids and sub-pixel marks both drop out'
);
assert.deepEqual(
  liveShapes(drawn, new Set()).map((item) => item.id),
  ['a', 'd', 'e']
);
// A stroke straight down has no width at all and must still survive.
assert.equal(liveShapes([drawn[3]], new Set()).length, 1);

// ── shapeAtPoint ─────────────────────────────────────────────────────────────

const stacked: { id: string; shape: AnnotationShape }[] = [
  { id: 'under', shape: { kind: 'region', x: 0, y: 0, width: 200, height: 200 } },
  { id: 'over', shape: { kind: 'region', x: 50, y: 50, width: 50, height: 50 } },
  { id: 'line', shape: { kind: 'stroke', points: [{ x: 300, y: 20 }, { x: 380, y: 20 }] } }
];

assert.equal(shapeAtPoint(stacked, 60, 60), 'over', 'the topmost mark is the one erased');
assert.equal(shapeAtPoint(stacked, 10, 10), 'under');
assert.equal(shapeAtPoint(stacked, 900, 900), null, 'a click on bare page erases nothing');
assert.equal(shapeAtPoint(stacked, 340, 22), 'line', 'a stroke is hit within its tolerance');
assert.equal(shapeAtPoint(stacked, 340, 90), null, 'and missed outside it');

// ── scaleShapes ──────────────────────────────────────────────────────────────

const layer = { width: 400, height: 300 };

assert.deepEqual(
  scaleShapes(
    [
      { kind: 'region', x: 20, y: 30, width: 100, height: 60 },
      { kind: 'stroke', points: [{ x: 10, y: 10 }, { x: 40, y: 80 }] }
    ],
    layer,
    { width: 800, height: 600 }
  ),
  [
    { kind: 'region', x: 40, y: 60, width: 200, height: 120 },
    { kind: 'stroke', points: [{ x: 20, y: 20 }, { x: 80, y: 160 }] }
  ],
  'a doubled image doubles both axes'
);

// The captured image need not share the layer's aspect ratio — a device pixel
// ratio only on one axis is normal. Each axis takes its own factor.
assert.deepEqual(
  scaleShapes([{ kind: 'element', x: 20, y: 30, width: 100, height: 60, tag: 'svg' }], layer, {
    width: 800,
    height: 300
  }),
  [{ kind: 'element', x: 40, y: 30, width: 200, height: 60, tag: 'svg' }]
);

// ── formatAttachmentNote ─────────────────────────────────────────────────────

const full = formatAttachmentNote({
  url: 'https://example.com/pricing',
  selector: 'main > section:nth-child(2) h1',
  tag: 'h1',
  description: 'Make this heading two lines on mobile.'
});
assert.equal(full.includes('https://example.com/pricing'), true);
assert.equal(full.includes('Make this heading two lines on mobile.'), true);
assert.equal(full.includes('main > section:nth-child(2) h1'), true);
assert.equal(full.includes('h1'), true);

const sparse = formatAttachmentNote({
  url: 'https://example.com/pricing',
  selector: null,
  tag: null,
  description: 'The spacing under the header is too tight.'
});
assert.equal(sparse.includes('Element'), false, 'no element line without an element');
assert.equal(sparse.includes('Selector'), false);
assert.equal(sparse.includes('https://example.com/pricing'), true);

// No line may be a label with nothing after it, in any combination.
for (const note of [
  full,
  sparse,
  formatAttachmentNote({ url: '', selector: null, tag: null, description: '' }),
  formatAttachmentNote({ url: 'https://example.com', selector: '  ', tag: '', description: '   ' })
]) {
  for (const line of note.split('\n')) {
    assert.equal(/:\s*$/.test(line), false, `empty labeled line: ${JSON.stringify(line)}`);
  }
}

console.log('browserAnnotationComposite: all checks passed');
