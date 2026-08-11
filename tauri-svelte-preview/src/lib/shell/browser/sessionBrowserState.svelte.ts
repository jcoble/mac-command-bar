/**
 * sessionBrowserState.svelte.ts — where each session's browsing lives.
 *
 * Holds STATE ONLY, keyed by session id: whether that session's overlay is
 * showing, the address it is on, whether notes are being drawn, and the notes
 * themselves. Every decision is a plain function in `sessionBrowserOps.ts`;
 * this module assigns what those functions return, exactly like the editor
 * store does.
 *
 * No backend call and no `$effect` here. Switching sessions needs no work at
 * all: the overlay reads the view for whichever session is active, so the
 * content swaps on its own.
 */
import {
  addSessionAnnotation,
  clearSessionAnnotations,
  closeSessionBrowser,
  createSessionBrowserView,
  openSessionBrowser,
  readSessionBrowserView,
  removeSessionAnnotation,
  setSessionBrowserAnnotating,
  setSessionBrowserUrl,
  type SessionAnnotationInput,
  type SessionBrowserMap,
  type SessionBrowserView
} from './sessionBrowserOps.ts';

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

export function setSessionBrowserAnnotateMode(sessionId: string | null, annotating: boolean): void {
  state.bySession = setSessionBrowserAnnotating(state.bySession, sessionId, annotating);
}

export function addSessionBrowserAnnotation(sessionId: string | null, input: SessionAnnotationInput): void {
  state.bySession = addSessionAnnotation(state.bySession, sessionId, input);
}

export function removeSessionBrowserAnnotation(sessionId: string | null, annotationId: string): void {
  state.bySession = removeSessionAnnotation(state.bySession, sessionId, annotationId);
}

export function clearSessionBrowserAnnotations(sessionId: string | null): void {
  state.bySession = clearSessionAnnotations(state.bySession, sessionId);
}

/** Used by tests and by a session being removed from the rail. */
export function resetSessionBrowserState(): void {
  state.bySession = {};
}

export { createSessionBrowserView };
