/**
 * sessionStrip.ts — how the sessions column splits your own work in two, and
 * what is left of it when the column is folded up into a narrow strip.
 *
 * PURE: no store, no DOM, no backend call. Callers hand in the sessions and a
 * Storage-like object, so all of it runs in a node test with no browser.
 *
 * The split lives here rather than in the component because it is a rule, not a
 * drawing: which side a session sits on is the USER's answer (`completedAt`),
 * never the process's. An agent that stopped running is still work in progress
 * until it is marked done, and a session marked done may still have a terminal
 * running.
 */
import { loadLayout, saveLayout, type LayoutStorage } from './layout/layoutStorage.ts';
import type { AgentKind, OwnedSession, OwnedSessionState } from './ownedSessions';

/** Where the column remembers that it was folded up. Its own key: the width it
 * takes in the frame is stored with the rest of the layout, but WHY it is that
 * width is this column's business and outlives any rearranging. */
export const SESSIONS_COLLAPSED_KEY = 'mac-command-bar.next.sessions-collapsed';

/** Your own sessions, in the two lists the column draws. */
export interface OwnedSplit {
  /** Not marked done yet, in the order the store already put them in. */
  working: OwnedSession[];
  /** Marked done, most recently finished first. */
  done: OwnedSession[];
}

/** One session as the folded-up strip shows it: an icon, a state, a name for
 * the tooltip, and the id a click sends back. */
export interface StripCell {
  ownedId: string;
  /** What the session is called — the tooltip, and the label a reader hears. */
  label: string;
  agent: AgentKind;
  viaCmux: boolean;
  state: OwnedSessionState;
  /** Marked done. The strip dims these rather than hiding them: a folded column
   * that quietly drops half your sessions is a column you cannot trust. */
  done: boolean;
  /** The session showing in the main pane. */
  active: boolean;
}

/** What a session is called on screen. A session that arrived with no title —
 * a fresh shell, or a scanned record whose transcript said nothing — is called
 * by the front of its id, which is at least stable and unique. */
export function sessionLabel(session: Pick<OwnedSession, 'title' | 'ownedId'>): string {
  return session.title || session.ownedId.slice(0, 8);
}

/** Mirror the native first-prompt title rule for the already-flowing live user
 * message event. SQLite remains authoritative; this updates only the rail's
 * in-memory projection without adding another event kind or backend read. */
export function sessionTitleFromPrompt(prompt: string): string | null {
  const firstLine = prompt.split(/\r?\n/, 1)[0]?.trim() ?? '';
  return firstLine ? [...firstLine].slice(0, 64).join('') : null;
}

/** Split your sessions into Working and Done. The list handed in is left
 * exactly as it was; both lists come back new. */
export function splitOwnedSessions(owned: OwnedSession[]): OwnedSplit {
  const working = owned.filter((session) => session.completedAt === null);
  const done = owned
    .filter((session) => session.completedAt !== null)
    .sort((a, b) => (b.completedAt ?? '').localeCompare(a.completedAt ?? ''));
  return { working, done };
}

/**
 * The cells the folded-up strip draws, in the same order the open column draws
 * its cards: everything being worked on, then everything finished. Same order
 * either way is the whole point — folding the column must not shuffle the list
 * you had just learned to read.
 */
export function stripCells(owned: OwnedSession[], activeOwnedId: string | null): StripCell[] {
  const { working, done } = splitOwnedSessions(owned);
  const cellFor = (session: OwnedSession, isDone: boolean): StripCell => ({
    ownedId: session.ownedId,
    label: sessionLabel(session),
    agent: session.agent,
    viaCmux: session.viaCmux,
    state: session.state,
    done: isDone,
    active: session.ownedId === activeOwnedId
  });
  return [
    ...working.map((session) => cellFor(session, false)),
    ...done.map((session) => cellFor(session, true))
  ];
}

/** Was the column left folded up? Anything unreadable — corrupt text, a value
 * of the wrong shape, a storage that throws — reads as "no", and the worst that
 * costs is a column that opens when it was left closed. */
export function readSessionsCollapsed(storage: LayoutStorage): boolean {
  return loadLayout<unknown>(storage, SESSIONS_COLLAPSED_KEY) === true;
}

/** False means the write was refused (a full storage). The column carries on;
 * the next launch simply reads the column as open. That is only the whole story
 * because the page re-states the open column's width limits on restore — the
 * stored grid layout remembers the folded 52px lock as well, and without that
 * re-statement a refused write here came back as a column stuck at strip width
 * with the expanded markup crammed into it. See `onReady` in
 * `src/routes/next/+page.svelte`. */
export function writeSessionsCollapsed(storage: LayoutStorage, collapsed: boolean): boolean {
  return saveLayout(storage, SESSIONS_COLLAPSED_KEY, collapsed);
}
