/**
 * In-memory Svelte 5 projection for the /next session rail.
 *
 * SQLite owns durable session identity and metadata. The page hydrates this
 * projection from backend records and refreshes conversation state from stored
 * snapshots/events; this module performs no I/O.
 */
import type { AgentSession } from '../../tauriSource.ts';
import type { ConversationWorkspaceState } from '../conversation/conversationStore.svelte.ts';
import type { OwnedSession } from '../ownedSessions.ts';
import type { SessionWorkspaceSnapshot } from '../sessionWorkspaces.ts';

/** SQLite setting naming the rail row the owner last had selected. */
export const ACTIVE_OWNED_SESSION_SETTING_KEY = 'shell.active-owned-session';

/**
 * What one saved remote machine's transport is doing, as the backend reports it.
 * Rows start disconnected and only the native lifecycle event changes it.
 */
export type RemoteConnectionState = 'connected' | 'disconnected' | 'reconnecting';

/** The rail stays compact while SQLite hydrates only the selected session. */
export interface SessionProjection {
  activeOwnedId: string | null;
  rail: readonly OwnedSession[];
  activeConversation: ConversationWorkspaceState | null;
  activeWorkspace: SessionWorkspaceSnapshot | null;
}

export const rail = $state<{
  owned: OwnedSession[];
  available: AgentSession[];
  activeOwnedId: string | null;
  scanning: boolean;
  error: string | null;
  remoteConnections: Record<string, RemoteConnectionState>;
}>({
  owned: [],
  available: [],
  activeOwnedId: null,
  scanning: false,
  error: null,
  remoteConnections: {}
});

/** One machine's state, keyed by the profile id a remote session carries. */
export function setRemoteConnection(profileId: string, state: RemoteConnectionState): void {
  if (rail.remoteConnections[profileId] === state) return;
  rail.remoteConnections[profileId] = state;
}

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
  rail.activeOwnedId = ownedId;
}

export function setAvailable(sessions: AgentSession[]): void {
  // The rail needs resumable-session identity and row metadata, not the full
  // turns History shows after a project is expanded. Keeping those turns here
  // made closing History release nothing because the rail still owned them.
  rail.available = sessions.map((session) => ({ ...session, latestTurns: undefined }));
}
