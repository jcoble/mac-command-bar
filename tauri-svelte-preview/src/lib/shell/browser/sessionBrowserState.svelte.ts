/**
 * sessionBrowserState.svelte.ts — where each session's browsing lives.
 *
 * Holds transient render state keyed by session id. Annotation persistence is
 * exclusively the backend session store: activation loads it and mutations
 * write it. The in-memory list only paints the current backend result.
 *
 * No polling, timers, local storage, or `$effect` performs persistence here.
 */
import {
  appendSessionAnnotation,
  closeSessionBrowser,
  createSessionBrowserView,
  isAnnotatableRect,
  MAX_SESSION_ANNOTATIONS,
  openSessionBrowser,
  readSessionBrowserView,
  removeSessionAnnotation,
  replaceSessionAnnotations,
  setSessionBrowserAnnotating,
  setSessionBrowserUrl,
  stepSessionBrowserHistory,
  type SessionAnnotationInput,
  type SessionBrowserMap,
  type SessionBrowserView
} from './sessionBrowserOps.ts';
import {
  addStoredSessionAnnotation,
  deleteStoredSessionAnnotation,
  listStoredSessionAnnotations,
  presentStoredSessionAnnotation
} from './sessionAnnotationPersistence.ts';

export type {
  SessionAnnotationInput,
  SessionBrowserAnnotation,
  SessionBrowserMap,
  SessionBrowserPoint,
  SessionBrowserRect,
  SessionBrowserView
} from './sessionBrowserOps.ts';

const state = $state<{ bySession: SessionBrowserMap }>({ bySession: {} });

/** The overlay state for one session, or a fresh closed view. */
export function sessionBrowserView(sessionId: string | null | undefined): SessionBrowserView {
  return readSessionBrowserView(state.bySession, sessionId);
}

export function openSessionBrowserOverlay(sessionId: string | null): void {
  state.bySession = openSessionBrowser(state.bySession, sessionId);
}

export function closeSessionBrowserOverlay(sessionId: string | null): void {
  state.bySession = closeSessionBrowser(state.bySession, sessionId);
}

export function toggleSessionBrowserOverlay(sessionId: string | null): void {
  if (!sessionId) return;
  const view = sessionBrowserView(sessionId);
  state.bySession = view.open
    ? closeSessionBrowser(state.bySession, sessionId)
    : openSessionBrowser(state.bySession, sessionId);
}

export function setSessionBrowserAddress(sessionId: string | null, url: string): void {
  state.bySession = setSessionBrowserUrl(state.bySession, sessionId, url);
}

export function stepSessionBrowserAddress(
  sessionId: string | null,
  direction: 'back' | 'forward'
): void {
  state.bySession = stepSessionBrowserHistory(state.bySession, sessionId, direction);
}

export function setSessionBrowserAnnotateMode(sessionId: string | null, annotating: boolean): void {
  state.bySession = setSessionBrowserAnnotating(state.bySession, sessionId, annotating);
}

export async function loadSessionBrowserAnnotations(sessionId: string): Promise<void> {
  const rows = await listStoredSessionAnnotations(sessionId);
  state.bySession = replaceSessionAnnotations(
    state.bySession,
    sessionId,
    rows.map((row, index) => presentStoredSessionAnnotation(row, index + 1))
  );
}

export async function addSessionBrowserAnnotation(
  sessionId: string | null,
  input: SessionAnnotationInput
): Promise<void> {
  if (!sessionId) return;
  const comment = input.comment.trim();
  const view = sessionBrowserView(sessionId);
  if (
    !comment ||
    !isAnnotatableRect(input.rect) ||
    view.annotations.length >= MAX_SESSION_ANNOTATIONS
  ) return;
  const row = await addStoredSessionAnnotation(sessionId, view.url, input.rect, comment);
  state.bySession = appendSessionAnnotation(
    state.bySession,
    sessionId,
    presentStoredSessionAnnotation(row, view.annotations.length + 1)
  );
}

export async function removeSessionBrowserAnnotation(
  sessionId: string | null,
  annotationId: number
): Promise<void> {
  if (!sessionId) return;
  await deleteStoredSessionAnnotation(annotationId);
  state.bySession = removeSessionAnnotation(state.bySession, sessionId, annotationId);
}

export async function clearSessionBrowserAnnotations(sessionId: string | null): Promise<void> {
  if (!sessionId) return;
  for (const annotation of sessionBrowserView(sessionId).annotations) {
    await deleteStoredSessionAnnotation(annotation.id);
    state.bySession = removeSessionAnnotation(state.bySession, sessionId, annotation.id);
  }
}

/** Used by tests and by a session being removed from the rail. */
export function resetSessionBrowserState(): void {
  state.bySession = {};
}

export { createSessionBrowserView };
