/**
 * The projections rebuilt when a person selects a session in the rail.
 *
 * Keep this sequence explicit. Each large surface gets one small loader here
 * so it can be added, measured, and removed without waking the other surfaces.
 */
import { validateProjectRootFromTauri } from '../tauriSource';
import {
  cancelConversationReadWork,
  loadConversationForRead,
  releaseConversationForRead
} from './conversation/conversationService';
import {
  ensureConversationSession,
  evictInactiveConversationSessions,
  setConversationMode
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
    await this.fillTreeView(session, owner);
    if (!this.isCurrent(owner)) return;
    await this.fillChatHistory(session, displayedChatOwnedId, owner);
  }

  async fillTreeView(
    session: OwnedSession,
    owner: SessionSelectionOwner
  ): Promise<void> {
    if (!this.isCurrent(owner)) return;
    if (session.executionEnvironment === 'remote') {
      if (!this.isCurrent(owner)) return;
      this.treeOwnedId = session.ownedId;
      this.treeRoot = '';
      this.hasTreeProjection = true;
      return;
    }
    const root = session.cwd.trim() || (session.projectPath ?? '').trim();

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
    await loadConversationForRead(session.ownedId, false, owner.signal);
    if (!this.isCurrent(owner)) {
      releaseConversationForRead(session.ownedId);
      return;
    }
    this.hasChatProjection = true;
    this.chatOwnedId = session.ownedId;
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
