/**
 * The projections rebuilt when a person selects a session in the rail.
 *
 * Keep this sequence explicit. Each large surface gets one small loader here
 * so it can be added, measured, and removed without waking the other surfaces.
 */
import { validateProjectRootFromTauri } from '../tauriSource';
import {
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

export class SessionSelectionLayers {
  treeOwnedId = $state<string | null>(null);
  treeRoot = $state('');
  hasTreeProjection = $state(false);
  chatOwnedId = $state<string | null>(null);
  hasChatProjection = $state(false);
  private selectionGeneration = 0;

  /** Select only the filesystem projection. No chat ownership changes or reads. */
  async selectTreeOnly(session: OwnedSession): Promise<void> {
    const generation = ++this.selectionGeneration;
    await this.fillTreeView(session, generation);
  }

  async selectSession(session: OwnedSession, displayedChatOwnedId: string | null): Promise<void> {
    const generation = ++this.selectionGeneration;
    this.releaseDepartingChat(session.ownedId, displayedChatOwnedId);
    await Promise.all([
      this.fillTreeView(session, generation),
      this.fillChatHistory(session, generation)
    ]);
  }

  async fillTreeView(
    session: OwnedSession,
    generation = this.selectionGeneration
  ): Promise<void> {
    if (session.executionEnvironment === 'remote') {
      this.treeOwnedId = session.ownedId;
      this.treeRoot = '';
      this.hasTreeProjection = true;
      return;
    }
    const root = session.cwd.trim() || (session.projectPath ?? '').trim();
    this.hasTreeProjection = true;
    this.treeOwnedId = session.ownedId;

    if (!root) {
      this.treeRoot = '';
      return;
    }

    countInvoke('validate_project_root');
    const validation = await validateProjectRootFromTauri(root);
    if (generation !== this.selectionGeneration) return;
    if (validation && (!validation.exists || !validation.isDirectory)) {
      this.treeRoot = '';
      return;
    }

    this.treeRoot = root;
  }

  async fillChatHistory(
    session: OwnedSession,
    generation = this.selectionGeneration
  ): Promise<void> {
    if (this.chatOwnedId === session.ownedId && this.hasChatProjection) return;
    if (this.chatOwnedId) releaseConversationForRead(this.chatOwnedId);
    this.hasChatProjection = false;
    this.chatOwnedId = null;

    const provider = this.providerFor(session);
    if (!provider) {
      this.clearChatHistory();
      return;
    }

    ensureConversationSession(session.ownedId, provider);
    setConversationMode(session.ownedId, 'structured');
    this.hasChatProjection = true;
    this.chatOwnedId = session.ownedId;
    await loadConversationForRead(session.ownedId, false);
    if (generation !== this.selectionGeneration) {
      if (this.chatOwnedId === session.ownedId) {
        this.hasChatProjection = false;
        this.chatOwnedId = null;
      }
      releaseConversationForRead(session.ownedId);
      evictInactiveConversationSessions(this.chatOwnedId);
      return;
    }
    evictInactiveConversationSessions(session.ownedId);
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

  private releaseDepartingChat(nextOwnedId: string, displayedChatOwnedId: string | null): void {
    const departingOwnedId = this.chatOwnedId ?? displayedChatOwnedId;
    if (departingOwnedId && departingOwnedId !== nextOwnedId) {
      releaseConversationForRead(departingOwnedId);
    }
    evictInactiveConversationSessions(nextOwnedId);
    if (!departingOwnedId || (departingOwnedId === nextOwnedId && this.hasChatProjection)) return;
    this.hasChatProjection = false;
    this.chatOwnedId = null;
  }
}
