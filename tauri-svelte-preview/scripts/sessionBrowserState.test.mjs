/**
 * sessionBrowserState.test.mjs — the rules behind the per-session browser
 * overlay.
 *
 * The rules live in `sessionBrowserOps.ts` as plain functions so this test can
 * run under Node. The runes module `sessionBrowserState.svelte.ts` is a thin
 * wrapper that holds the state and assigns whatever these functions return.
 *
 * Run: node --experimental-strip-types scripts/sessionBrowserState.test.mjs
 */
import assert from 'node:assert/strict';
import {
  MAX_SESSION_ANNOTATIONS,
  MIN_ANNOTATION_SIDE,
  addSessionAnnotation,
  annotationCountLabel,
  clearSessionAnnotations,
  closeSessionBrowser,
  composeAnnotationMessage,
  createSessionBrowserView,
  isAnnotatableRect,
  openSessionBrowser,
  overlayHeaderLabel,
  rectFromDrag,
  readSessionBrowserView,
  removeSessionAnnotation,
  sendButtonLabel,
  setSessionBrowserAnnotating,
  setSessionBrowserUrl,
  stepSessionBrowserHistory
} from '../src/lib/shell/browser/sessionBrowserOps.ts';

// ── A fresh view, and reading a session nobody has opened yet ──────────────
const blank = createSessionBrowserView();
assert.equal(blank.open, false);
assert.equal(blank.annotating, false);
assert.equal(blank.url, '');
assert.deepEqual(blank.annotations, []);

const empty = {};
assert.deepEqual(readSessionBrowserView(empty, 'session-a'), blank);
assert.deepEqual(empty, {}, 'reading must not create an entry');
assert.deepEqual(readSessionBrowserView(empty, null), blank);

// ── Opening and closing belong to one session at a time ────────────────────
let map = openSessionBrowser({}, 'session-a');
assert.equal(readSessionBrowserView(map, 'session-a').open, true);
assert.equal(readSessionBrowserView(map, 'session-b').open, false);

map = setSessionBrowserUrl(map, 'session-a', 'https://example.com/a');
map = openSessionBrowser(map, 'session-b');
map = setSessionBrowserUrl(map, 'session-b', 'https://example.com/b');
assert.equal(readSessionBrowserView(map, 'session-a').url, 'https://example.com/a');
assert.equal(readSessionBrowserView(map, 'session-b').url, 'https://example.com/b');

// Back and forward stay inside the session, and a new address replaces only
// the forward branch just like a normal browser history.
map = setSessionBrowserUrl(map, 'session-a', 'https://example.com/a/details');
map = stepSessionBrowserHistory(map, 'session-a', 'back');
assert.equal(readSessionBrowserView(map, 'session-a').url, 'https://example.com/a');
map = stepSessionBrowserHistory(map, 'session-a', 'forward');
assert.equal(readSessionBrowserView(map, 'session-a').url, 'https://example.com/a/details');
map = stepSessionBrowserHistory(map, 'session-a', 'back');
map = setSessionBrowserUrl(map, 'session-a', 'https://example.com/a/alternate');
assert.deepEqual(readSessionBrowserView(map, 'session-a').history, [
  'https://example.com/a',
  'https://example.com/a/alternate'
]);
assert.equal(stepSessionBrowserHistory(map, 'session-a', 'forward'), map);

// Closing hides the overlay but keeps everything the session had.
map = closeSessionBrowser(map, 'session-a');
assert.equal(readSessionBrowserView(map, 'session-a').open, false);
assert.equal(readSessionBrowserView(map, 'session-a').url, 'https://example.com/a/alternate');
assert.equal(readSessionBrowserView(map, 'session-b').open, true, 'the other session is untouched');

// ── Drag rectangles ────────────────────────────────────────────────────────
assert.deepEqual(rectFromDrag({ x: 120, y: 90 }, { x: 40, y: 30 }), {
  x: 40,
  y: 30,
  width: 80,
  height: 60
});
assert.equal(isAnnotatableRect({ x: 0, y: 0, width: MIN_ANNOTATION_SIDE, height: MIN_ANNOTATION_SIDE }), true);
assert.equal(isAnnotatableRect({ x: 0, y: 0, width: MIN_ANNOTATION_SIDE - 1, height: 40 }), false);

// ── Saving annotations numbers them in order ───────────────────────────────
const rect = { x: 10, y: 20, width: 100, height: 50 };
map = setSessionBrowserAnnotating(map, 'session-b', true);
assert.equal(readSessionBrowserView(map, 'session-b').annotating, true);

map = addSessionAnnotation(map, 'session-b', {
  rect,
  comment: 'The heading wraps',
  id: 'note-1',
  createdAt: '2026-08-11T10:00:00.000Z'
});
map = addSessionAnnotation(map, 'session-b', {
  rect: { x: 200, y: 200, width: 40, height: 40 },
  comment: 'This button is dead',
  id: 'note-2',
  createdAt: '2026-08-11T10:01:00.000Z'
});
let view = readSessionBrowserView(map, 'session-b');
assert.deepEqual(view.annotations.map((note) => note.marker), [1, 2]);
assert.equal(view.annotations[0].url, 'https://example.com/b', 'a note remembers the page it was drawn on');

// A blank comment is not an annotation.
const unchanged = addSessionAnnotation(map, 'session-b', { rect, comment: '   ', id: 'note-3' });
assert.equal(readSessionBrowserView(unchanged, 'session-b').annotations.length, 2);

// A rectangle too small to point at anything is not an annotation either.
const tooSmall = addSessionAnnotation(map, 'session-b', {
  rect: { x: 0, y: 0, width: 2, height: 2 },
  comment: 'Here',
  id: 'note-4'
});
assert.equal(readSessionBrowserView(tooSmall, 'session-b').annotations.length, 2);

// ── Removing renumbers the markers so the page never shows a gap ───────────
const removed = removeSessionAnnotation(map, 'session-b', 'note-1');
view = readSessionBrowserView(removed, 'session-b');
assert.deepEqual(view.annotations.map((note) => note.marker), [1]);
assert.equal(view.annotations[0].id, 'note-2');

// ── The cap keeps one page from collecting an unbounded pile ───────────────
let full = openSessionBrowser({}, 'session-c');
full = setSessionBrowserUrl(full, 'session-c', 'https://example.com/c');
for (let index = 0; index < MAX_SESSION_ANNOTATIONS + 4; index += 1) {
  full = addSessionAnnotation(full, 'session-c', { rect, comment: `note ${index}`, id: `full-${index}` });
}
assert.equal(readSessionBrowserView(full, 'session-c').annotations.length, MAX_SESSION_ANNOTATIONS);

// ── Labels ─────────────────────────────────────────────────────────────────
assert.equal(annotationCountLabel(0), 'No annotations');
assert.equal(annotationCountLabel(1), '1 annotation');
assert.equal(annotationCountLabel(4), '4 annotations');
assert.equal(sendButtonLabel(0), 'Send');
assert.equal(sendButtonLabel(3), 'Send (3)');
assert.equal(
  overlayHeaderLabel(readSessionBrowserView(map, 'session-b')),
  'Annotating · https://example.com/b'
);
assert.equal(
  overlayHeaderLabel(readSessionBrowserView(closeSessionBrowser(map, 'session-b'), 'session-b')),
  'Annotating · https://example.com/b',
  'the header follows annotate mode, not whether the overlay is showing'
);
assert.equal(
  overlayHeaderLabel(readSessionBrowserView(setSessionBrowserAnnotating(map, 'session-b', false), 'session-b')),
  'https://example.com/b'
);
assert.equal(overlayHeaderLabel(createSessionBrowserView()), 'No address');

// ── One message carries the prompt and every annotation ────────────────────
const message = composeAnnotationMessage(readSessionBrowserView(map, 'session-b'), 'Fix these two things');
assert.match(message, /^Fix these two things/);
assert.match(message, /https:\/\/example\.com\/b/);
assert.match(message, /1\. .*The heading wraps/);
assert.match(message, /2\. .*This button is dead/);
assert.match(message, /10, 20, 100×50/, 'the region is carried as coordinates');

// The prompt alone is fine when nothing was annotated.
assert.equal(composeAnnotationMessage(createSessionBrowserView(), 'Just asking'), 'Just asking');
assert.equal(composeAnnotationMessage(createSessionBrowserView(), '   '), '');

// ── Clearing throws the pile away and leaves the page alone ────────────────
const cleared = clearSessionAnnotations(map, 'session-b');
assert.deepEqual(readSessionBrowserView(cleared, 'session-b').annotations, []);
assert.equal(readSessionBrowserView(cleared, 'session-b').url, 'https://example.com/b');

console.log('sessionBrowserState: all checks passed');
