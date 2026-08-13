/**
 * sessionBrowserState.test.ts — the rules behind the per-session browser
 * overlay.
 *
 * The rules live in `sessionBrowserOps.ts` as plain functions so this test can
 * run under Node. The runes module `sessionBrowserState.svelte.ts` is a thin
 * wrapper that holds the state and assigns whatever these functions return.
 *
 * Run: node --experimental-strip-types scripts/sessionBrowserState.test.ts
 */
import assert from 'node:assert/strict';
import {
  MAX_SESSION_ANNOTATIONS,
  MIN_ANNOTATION_SIDE,
  appendSessionAnnotation,
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
  replaceSessionAnnotations,
  sendButtonLabel,
  setSessionBrowserAnnotating,
  setSessionBrowserUrl,
  stepSessionBrowserHistory
} from '../src/lib/shell/browser/sessionBrowserOps.ts';
import {
  addStoredSessionAnnotation,
  deleteStoredSessionAnnotation,
  listStoredSessionAnnotations,
  presentStoredSessionAnnotation,
  type SessionAnnotationCommandInvoker,
  type StoredSessionAnnotation
} from '../src/lib/shell/browser/sessionAnnotationPersistence.ts';

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

map = appendSessionAnnotation(map, 'session-b', {
  marker: 1,
  rect,
  comment: 'The heading wraps',
  id: 1,
  url: 'https://example.com/b',
  createdAt: '2026-08-11T10:00:00.000Z'
});
map = appendSessionAnnotation(map, 'session-b', {
  marker: 2,
  rect: { x: 200, y: 200, width: 40, height: 40 },
  comment: 'This button is dead',
  id: 2,
  url: 'https://example.com/b',
  createdAt: '2026-08-11T10:01:00.000Z'
});
let view = readSessionBrowserView(map, 'session-b');
assert.deepEqual(view.annotations.map((note) => note.marker), [1, 2]);
assert.equal(view.annotations[0].url, 'https://example.com/b', 'a note remembers the page it was drawn on');

// ── Removing renumbers the markers so the page never shows a gap ───────────
const removed = removeSessionAnnotation(map, 'session-b', 1);
view = readSessionBrowserView(removed, 'session-b');
assert.deepEqual(view.annotations.map((note) => note.marker), [1]);
assert.equal(view.annotations[0].id, 2);

// ── The cap keeps one page from collecting an unbounded pile ───────────────
let full = openSessionBrowser({}, 'session-c');
full = setSessionBrowserUrl(full, 'session-c', 'https://example.com/c');
for (let index = 0; index < MAX_SESSION_ANNOTATIONS + 4; index += 1) {
  full = appendSessionAnnotation(full, 'session-c', {
    id: index + 10,
    marker: index + 1,
    rect,
    comment: `note ${index}`,
    url: 'https://example.com/c',
    createdAt: '2026-08-11T10:00:00.000Z'
  });
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

// ── A fresh frontend state reloads the rows left in the backend store ──────
const persistedRows: StoredSessionAnnotation[] = [];
const invoked: string[] = [];
const invokeCommand: SessionAnnotationCommandInvoker = async <T>(
  command: string,
  args: Record<string, unknown>
): Promise<T> => {
  invoked.push(command);
  if (command === 'agent_conversation_add_session_annotation') {
    const row: StoredSessionAnnotation = {
      id: 91,
      ownedId: String(args.ownedId),
      url: String(args.url),
      rectJson: String(args.rectJson),
      note: String(args.note),
      createdAtMs: Date.parse('2026-08-11T10:05:00.000Z')
    };
    persistedRows.push(row);
    return row as T;
  }
  if (command === 'agent_conversation_list_session_annotations') {
    return persistedRows.filter((row) => row.ownedId === args.ownedId) as T;
  }
  if (command === 'agent_conversation_delete_session_annotation') {
    const index = persistedRows.findIndex((row) => row.id === args.id);
    if (index >= 0) persistedRows.splice(index, 1);
    return undefined as T;
  }
  throw new Error(`unexpected command: ${command}`);
};

await addStoredSessionAnnotation(
  'session-restart',
  'https://example.com/restart',
  rect,
  'Still here',
  invokeCommand
);
const restartedRows = await listStoredSessionAnnotations('session-restart', invokeCommand);
const restartedMap = replaceSessionAnnotations(
  {},
  'session-restart',
  restartedRows.map((row, index) => presentStoredSessionAnnotation(row, index + 1))
);
assert.equal(readSessionBrowserView(restartedMap, 'session-restart').annotations[0].comment, 'Still here');
await deleteStoredSessionAnnotation(91, invokeCommand);
assert.deepEqual(await listStoredSessionAnnotations('session-restart', invokeCommand), []);
assert.deepEqual(invoked, [
  'agent_conversation_add_session_annotation',
  'agent_conversation_list_session_annotations',
  'agent_conversation_delete_session_annotation',
  'agent_conversation_list_session_annotations'
]);

console.log('sessionBrowserState: all checks passed');
