/**
 * sessionRailStore.svelte.ts — Svelte 5 runes state for the /next session rail.
 *
 * Holds the rail's STATE only: the owned sessions (CommandBar-minted records,
 * keyed by `ownedId`), the scanned-but-not-yet-adopted `available` sessions, the
 * active owned id, and scan status/error. Pattern mirrors
 * `src/lib/stores/projectStore.svelte.ts` and `src/lib/settingsStore.svelte.ts`.
 *
 * TWO RULES this module exists to enforce:
 *
 * 1. **No backend, ever.** The store performs no IO except localStorage. Every
 *    Tauri call (scan, start, close, re-attach) lives in the page/service layer
 *    and lands here as a plain mutation.
 * 2. **No `$effect`.** `$effect` is illegal in a `.svelte.ts` module (and against
 *    the constitution), so persistence is an EXPLICIT `persist()` call at the end
 *    of every mutator that changes `owned`. Miss one and a reload loses state —
 *    notably `updateOwnedSession`, which is how a freshly started session's
 *    `ptySessionId` reaches storage for reload re-attach.
 *
 * Quota/unavailable-storage errors are swallowed (matching `settingsStore`'s
 * `persist()`): a full disk must never break the rail's in-memory state. They
 * are NOT silent, though — see `persist()`.
 */
// NOTE: explicit `.ts` specifiers, matching `terminalService.ts` and the rest of
// `src/lib/shell`.
import type { AgentSession } from '../../tauriSource.ts';
import {
  parseStoredOwnedSessions,
  serializeOwnedSessions,
  type OwnedSession
} from '../ownedSessions.ts';

/** localStorage key holding the serialized owned-session list. */
export const OWNED_SESSIONS_STORAGE_KEY = 'mac-command-bar.next.owned-sessions';

// ── Reactive state ────────────────────────────────────────────────────────────

/**
 * The single reactive rail state object. Read fields directly in components
 * (e.g. `rail.owned`, `rail.activeOwnedId`); reads/writes are tracked by
 * Svelte's runes runtime. Mutate through the exported functions so persistence
 * stays in lockstep.
 */
export const rail = $state<{
  /** Sessions CommandBar owns, keyed by `ownedId`. Persisted. */
  owned: OwnedSession[];
  /** Scanned agent sessions not yet adopted. Never persisted (rescanned). */
  available: AgentSession[];
  /** `ownedId` of the session shown in the main pane, if any. */
  activeOwnedId: string | null;
  /** A session scan is in flight. */
  scanning: boolean;
  /** Last rail-level error message, if any. */
  error: string | null;
}>({
  owned: [],
  available: [],
  activeOwnedId: null,
  scanning: false,
  error: null
});

// ── Persistence ───────────────────────────────────────────────────────────────

/** What the user is told when the rail could not be written to localStorage. */
export const STORAGE_WRITE_FAILED_MESSAGE =
  'Session list could not be saved — browser storage is full; a reload may lose the session links';

/**
 * Write the current `owned` list to localStorage; `true` when it landed.
 *
 * No-op when storage is unavailable (SSR / private mode). A quota error is
 * still swallowed — a full store must never break the in-memory rail — but it
 * is NOT silent: the dropped write loses `ptySessionId`, which is exactly what
 * reload re-attach reads back, so the failure is surfaced on `rail.error`.
 * Nothing is evicted to make room: the other keys on this origin are not ours.
 */
function persist(): boolean {
  if (typeof localStorage === 'undefined') return false;
  try {
    localStorage.setItem(
      OWNED_SESSIONS_STORAGE_KEY,
      serializeOwnedSessions($state.snapshot(rail.owned) as OwnedSession[])
    );
    return true;
  } catch {
    rail.error = STORAGE_WRITE_FAILED_MESSAGE;
    return false;
  }
}

/**
 * Read the persisted owned sessions. Tolerant: returns `[]` when storage is
 * unavailable or the payload is missing/corrupt. Does NOT touch `rail` — the
 * caller reconciles against live PTYs first, then calls `hydrateOwned`.
 */
export function loadStoredOwned(): OwnedSession[] {
  if (typeof localStorage === 'undefined') return [];
  try {
    return parseStoredOwnedSessions(localStorage.getItem(OWNED_SESSIONS_STORAGE_KEY));
  } catch {
    return [];
  }
}

// ── Mutations ─────────────────────────────────────────────────────────────────

/**
 * Seed `owned` from a reconciled list at startup. Persists so a reconcile that
 * dropped dead PTY ids (`state: 'exited'`) is written back immediately.
 */
export function hydrateOwned(sessions: OwnedSession[]): void {
  rail.owned = sessions;
  persist();
}

/** Append a newly adopted or freshly created owned session. */
export function addOwnedSession(session: OwnedSession): void {
  rail.owned = [...rail.owned, session];
  persist();
}

/**
 * Patch one owned session by `ownedId`. MUST persist — this is the path that
 * stores `ptySessionId` after a start, and losing that write breaks reload
 * re-attach. Unknown ids are ignored.
 */
export function updateOwnedSession(ownedId: string, patch: Partial<OwnedSession>): void {
  let changed = false;
  rail.owned = rail.owned.map((session) => {
    if (session.ownedId !== ownedId) return session;
    changed = true;
    return { ...session, ...patch, ownedId: session.ownedId };
  });
  if (changed) {
    persist();
  }
}

/**
 * Mark a session done. The caller supplies the clock so this stays testable and
 * the store keeps no notion of "now".
 *
 * Being done is only ever the user saying so, and it costs the session nothing:
 * its terminal keeps running, and `reopenOwnedSession` puts it straight back.
 */
export function completeOwnedSession(ownedId: string, when: Date): void {
  updateOwnedSession(ownedId, { completedAt: when.toISOString() });
}

/** Put a done session back on the working list. */
export function reopenOwnedSession(ownedId: string): void {
  updateOwnedSession(ownedId, { completedAt: null });
}

/** Drop an owned session. Clears `activeOwnedId` when it was the active one. */
export function removeOwnedSession(ownedId: string): void {
  rail.owned = rail.owned.filter((session) => session.ownedId !== ownedId);
  if (rail.activeOwnedId === ownedId) {
    rail.activeOwnedId = null;
  }
  persist();
}

/**
 * Focus an owned session: the new active one becomes `'live'`, the previously
 * active one falls back to `'background'`. An `'exited'` session is NEVER
 * re-labelled — a dead PTY stays dead no matter what gets focused.
 */
export function setActiveOwned(ownedId: string | null): void {
  const previousId = rail.activeOwnedId;
  rail.activeOwnedId = ownedId;
  rail.owned = rail.owned.map((session) => {
    if (session.state === 'exited') return session;
    if (session.ownedId === ownedId) {
      return session.state === 'live' ? session : { ...session, state: 'live' };
    }
    if (session.ownedId === previousId) {
      return session.state === 'background' ? session : { ...session, state: 'background' };
    }
    return session;
  });
  persist();
}

/** Replace the scanned `available` list. Not persisted — rescanned on load. */
export function setAvailable(sessions: AgentSession[]): void {
  rail.available = sessions;
}
