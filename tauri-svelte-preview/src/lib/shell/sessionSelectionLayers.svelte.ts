import { warmAgentConversationConfig } from './conversation/conversationConfig';
import { sessionWorkspaceRoot } from '../workspacePaths';
/**
 * The projections rebuilt when a person selects a session in the rail.
 *
 * Keep this sequence explicit. Each large surface gets one small loader here
 * so it can be added, measured, and removed without waking the other surfaces.
 */
import { validateProjectRootFromTauri } from '../tauriSource';
import {
  cancelConversationReadWork,
  loadConversationSessionDraft,
  loadConversationForRead,
  releaseConversationForRead
} from './conversation/conversationService';
import {
  ensureConversationSession,
  evictInactiveConversationSessions,
  setConversationMode,
  setConversationAgentConfigState,
  setConversationAgentConfigError,
  getConversationSession
} from './conversation/conversationStore.svelte';
import type { AgentConversationProvider } from './conversation/conversationTypes';
import { countInvoke } from './devInvokeCounter.svelte';
import type { OwnedSession } from './ownedSessions';

export type SessionSelectionOwner = {
  generation: number;
  signal: AbortSignal;
};

export class SessionSelectionLayers {
  treeOwnedId = $state<string | null>(null);
  treeRoot = $state('');
  hasTreeProjection = $state(false);
  chatOwnedId = $state<string | null>(null);
  hasChatProjection = $state(false);
  private selectionGeneration = 0;

  async selectSession(
    session: OwnedSession,
    displayedChatOwnedId: string | null,
    owner: SessionSelectionOwner
  ): Promise<void> {
    this.selectionGeneration = owner.generation;
    await Promise.all([
      this.fillTreeView(session, owner),
      this.fillChatHistory(session, displayedChatOwnedId, owner)
    ]);
  }

  async fillTreeView(
    session: OwnedSession,
    owner: SessionSelectionOwner
  ): Promise<void> {
    if (!this.isCurrent(owner)) return;
    const root = sessionWorkspaceRoot(session);

    if (!root) {
      if (!this.isCurrent(owner)) return;
      this.hasTreeProjection = true;
      this.treeOwnedId = session.ownedId;
      this.treeRoot = '';
      return;
    }

    countInvoke('validate_project_root');
    const validation = await validateProjectRootFromTauri(root, owner.signal);
    if (!this.isCurrent(owner)) return;
    if (validation && (!validation.exists || !validation.isDirectory)) {
      this.hasTreeProjection = true;
      this.treeOwnedId = session.ownedId;
      this.treeRoot = '';
      return;
    }

    this.hasTreeProjection = true;
    this.treeOwnedId = session.ownedId;
    this.treeRoot = root;
  }

  async fillChatHistory(
    session: OwnedSession,
    displayedChatOwnedId: string | null,
    owner: SessionSelectionOwner
  ): Promise<void> {
    if (!this.isCurrent(owner)) return;
    if (this.chatOwnedId === session.ownedId && this.hasChatProjection) return;
    const departingOwnedId = this.chatOwnedId ?? displayedChatOwnedId;

    const provider = this.providerFor(session);
    if (!provider) {
      if (!this.isCurrent(owner)) return;
      if (departingOwnedId) releaseConversationForRead(departingOwnedId);
      this.hasChatProjection = false;
      this.chatOwnedId = null;
      evictInactiveConversationSessions(null);
      return;
    }

    if (departingOwnedId && departingOwnedId !== session.ownedId) {
      cancelConversationReadWork(departingOwnedId);
    }

    ensureConversationSession(session.ownedId, provider);
    setConversationMode(session.ownedId, 'structured');
    await loadConversationForRead(session.ownedId, true, owner.signal);
    if (!this.isCurrent(owner)) {
      releaseConversationForRead(session.ownedId);
      return;
    }
    await loadConversationSessionDraft(session.ownedId);
    if (!this.isCurrent(owner)) {
      releaseConversationForRead(session.ownedId);
      return;
    }
    this.hasChatProjection = true;
    this.chatOwnedId = session.ownedId;
    if (session.executionEnvironment === 'remote' && session.nativeSessionId) {
      const generation = getConversationSession(session.ownedId)?.generation;
      if (generation !== undefined) void warmAgentConversationConfig(session.ownedId, generation).then((config) => {
        if (this.isCurrent(owner)) setConversationAgentConfigState(session.ownedId, config);
      }).catch((error: unknown) => {
        if (this.isCurrent(owner)) setConversationAgentConfigError(session.ownedId, error instanceof Error ? error.message : String(error));
      });
    }
    if (departingOwnedId && departingOwnedId !== session.ownedId) {
      releaseConversationForRead(departingOwnedId);
    }
    evictInactiveConversationSessions(session.ownedId);
  }

  abandonSelection(owner: SessionSelectionOwner): void {
    if (owner.generation === this.selectionGeneration) this.selectionGeneration += 1;
  }

  clearTreeView(): void {
    this.hasTreeProjection = false;
    this.treeOwnedId = null;
    this.treeRoot = '';
  }

  clearChatHistory(): void {
    this.selectionGeneration += 1;
    if (this.chatOwnedId) releaseConversationForRead(this.chatOwnedId);
    this.hasChatProjection = false;
    this.chatOwnedId = null;
    evictInactiveConversationSessions(null);
  }

  private providerFor(session: OwnedSession): AgentConversationProvider | null {
    const raw = (session.agent || '').toLowerCase();
    if (raw === 'claude' || raw.includes('claude') || raw.includes('anthropic')) return 'claude';
    if (raw === 'codex' || raw.includes('codex') || raw.includes('openai')) return 'codex';
    if (raw === 'antigravity' || raw.includes('antigravity') || raw.includes('agy') || raw.includes('gemini')) return 'antigravity';
    return null;
  }

  private isCurrent(owner: SessionSelectionOwner): boolean {
    return !owner.signal.aborted && owner.generation === this.selectionGeneration;
  }

}
