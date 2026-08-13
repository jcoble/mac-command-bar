/**
 * In-memory Svelte 5 projection for the /next session rail.
 *
 * SQLite owns durable session identity and metadata. The page hydrates this
 * projection from backend records and refreshes conversation state from stored
 * snapshots/events; this module performs no I/O.
 */
import type { AgentSession } from '../../tauriSource.ts';
import type { OwnedSession } from '../ownedSessions.ts';

export const rail = $state<{
  owned: OwnedSession[];
  available: AgentSession[];
  activeOwnedId: string | null;
  scanning: boolean;
  error: string | null;
}>({
  owned: [],
  available: [],
  activeOwnedId: null,
  scanning: false,
  error: null
});

export function hydrateOwned(sessions: OwnedSession[]): void {
  rail.owned = sessions;
}

export function addOwnedSession(session: OwnedSession): void {
  rail.owned = [...rail.owned, session];
}

export function updateOwnedSession(ownedId: string, patch: Partial<OwnedSession>): void {
  rail.owned = rail.owned.map((session) =>
    session.ownedId === ownedId ? { ...session, ...patch, ownedId: session.ownedId } : session
  );
}

export function completeOwnedSession(ownedId: string, when: Date): void {
  updateOwnedSession(ownedId, { completedAt: when.toISOString() });
}

export function reopenOwnedSession(ownedId: string): void {
  updateOwnedSession(ownedId, { completedAt: null });
}

export function removeOwnedSession(ownedId: string): void {
  rail.owned = rail.owned.filter((session) => session.ownedId !== ownedId);
  if (rail.activeOwnedId === ownedId) rail.activeOwnedId = null;
}

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
}

export function setAvailable(sessions: AgentSession[]): void {
  rail.available = sessions;
}
