/**
 * sessionBrowserOps.ts — the rules behind the per-session browser overlay.
 *
 * Browsing belongs to a session, not to the window: the address a session is
 * looking at and the notes drawn on top of it travel with that session, and
 * switching sessions swaps the whole overlay. Everything here is a plain
 * function over a plain record so the rules can be exercised under Node; the
 * reactive holder is `sessionBrowserState.svelte.ts`.
 *
 * Nothing in this file touches the DOM, the network, or the desktop.
 */

export interface SessionBrowserRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface SessionBrowserPoint {
  x: number;
  y: number;
}

export interface SessionBrowserAnnotation {
  id: string;
  /** The number shown on the page marker; always 1..n with no gaps. */
  marker: number;
  /** Where the note was drawn, in overlay coordinates. */
  rect: SessionBrowserRect;
  comment: string;
  /** The address the note was drawn on, so a later send says what it meant. */
  url: string;
  createdAt: string;
}

export interface SessionBrowserView {
  /** Whether the overlay is showing for this session right now. */
  open: boolean;
  /** Whether a drag over the page draws a note instead of scrolling. */
  annotating: boolean;
  url: string;
  annotations: SessionBrowserAnnotation[];
}

export type SessionBrowserMap = Record<string, SessionBrowserView>;

export interface SessionAnnotationInput {
  rect: SessionBrowserRect;
  comment: string;
  id?: string;
  createdAt?: string;
}

/** A drag shorter than this on either side is a stray click, not a region. */
export const MIN_ANNOTATION_SIDE = 8;

/** One page's worth of notes. Past this the batch stops being reviewable. */
export const MAX_SESSION_ANNOTATIONS = 24;

export function createSessionBrowserView(url = ''): SessionBrowserView {
  return { open: false, annotating: false, url, annotations: [] };
}

/**
 * The view for a session, without writing anything. A session nobody has
 * opened the browser for reads as a fresh, closed view.
 */
export function readSessionBrowserView(
  map: SessionBrowserMap,
  sessionId: string | null | undefined
): SessionBrowserView {
  if (!sessionId) return createSessionBrowserView();
  return map[sessionId] ?? createSessionBrowserView();
}

function withView(
  map: SessionBrowserMap,
  sessionId: string | null | undefined,
  patch: Partial<SessionBrowserView>
): SessionBrowserMap {
  if (!sessionId) return map;
  const current = readSessionBrowserView(map, sessionId);
  return { ...map, [sessionId]: { ...current, ...patch } };
}

export function openSessionBrowser(map: SessionBrowserMap, sessionId: string | null): SessionBrowserMap {
  return withView(map, sessionId, { open: true });
}

/** Hide the overlay. The address and the notes stay where they are. */
export function closeSessionBrowser(map: SessionBrowserMap, sessionId: string | null): SessionBrowserMap {
  return withView(map, sessionId, { open: false });
}

export function setSessionBrowserUrl(
  map: SessionBrowserMap,
  sessionId: string | null,
  url: string
): SessionBrowserMap {
  return withView(map, sessionId, { url: url.trim() });
}

export function setSessionBrowserAnnotating(
  map: SessionBrowserMap,
  sessionId: string | null,
  annotating: boolean
): SessionBrowserMap {
  return withView(map, sessionId, { annotating });
}

/** Two corners of a drag become a rectangle, whichever way it was drawn. */
export function rectFromDrag(start: SessionBrowserPoint, end: SessionBrowserPoint): SessionBrowserRect {
  return {
    x: Math.min(start.x, end.x),
    y: Math.min(start.y, end.y),
    width: Math.abs(end.x - start.x),
    height: Math.abs(end.y - start.y)
  };
}

export function isAnnotatableRect(rect: SessionBrowserRect): boolean {
  return rect.width >= MIN_ANNOTATION_SIDE && rect.height >= MIN_ANNOTATION_SIDE;
}

function renumber(annotations: SessionBrowserAnnotation[]): SessionBrowserAnnotation[] {
  return annotations.map((note, index) => ({ ...note, marker: index + 1 }));
}

/**
 * Add a note. A blank comment or a region too small to point at anything is
 * not a note, and the map comes back untouched.
 */
export function addSessionAnnotation(
  map: SessionBrowserMap,
  sessionId: string | null,
  input: SessionAnnotationInput
): SessionBrowserMap {
  if (!sessionId) return map;
  const comment = input.comment.trim();
  if (!comment || !isAnnotatableRect(input.rect)) return map;
  const current = readSessionBrowserView(map, sessionId);
  if (current.annotations.length >= MAX_SESSION_ANNOTATIONS) return map;
  const note: SessionBrowserAnnotation = {
    id: input.id ?? `annotation-${current.annotations.length + 1}-${Date.now()}`,
    marker: current.annotations.length + 1,
    rect: { ...input.rect },
    comment,
    url: current.url,
    createdAt: input.createdAt ?? new Date().toISOString()
  };
  return withView(map, sessionId, { annotations: renumber([...current.annotations, note]) });
}

export function removeSessionAnnotation(
  map: SessionBrowserMap,
  sessionId: string | null,
  annotationId: string
): SessionBrowserMap {
  if (!sessionId) return map;
  const current = readSessionBrowserView(map, sessionId);
  const kept = current.annotations.filter((note) => note.id !== annotationId);
  if (kept.length === current.annotations.length) return map;
  return withView(map, sessionId, { annotations: renumber(kept) });
}

export function clearSessionAnnotations(map: SessionBrowserMap, sessionId: string | null): SessionBrowserMap {
  return withView(map, sessionId, { annotations: [] });
}

export function annotationCountLabel(count: number): string {
  if (count <= 0) return 'No annotations';
  return count === 1 ? '1 annotation' : `${count} annotations`;
}

export function sendButtonLabel(count: number): string {
  return count > 0 ? `Send (${count})` : 'Send';
}

/** What the overlay header reads: the page, and whether notes are being drawn. */
export function overlayHeaderLabel(view: SessionBrowserView): string {
  const address = view.url || 'No address';
  return view.annotating ? `Annotating · ${address}` : address;
}

function describe(note: SessionBrowserAnnotation): string {
  const { x, y, width, height } = note.rect;
  const region = `${Math.round(x)}, ${Math.round(y)}, ${Math.round(width)}×${Math.round(height)}`;
  return `${note.marker}. (region ${region}) ${note.comment}`;
}

/**
 * One message carrying the typed prompt and every note. Regions are given as
 * coordinates rather than cropped pictures because the desktop side has no
 * page snapshot to crop — see `src-tauri/src/browser.rs`, where the capture
 * command answers "unsupported".
 */
export function composeAnnotationMessage(view: SessionBrowserView, prompt: string): string {
  const typed = prompt.trim();
  if (!view.annotations.length) return typed;
  const lines = [
    typed,
    '',
    `Page: ${view.url || 'No address'}`,
    ...view.annotations.map(describe)
  ];
  return lines.join('\n').trim();
}
