/**
 * The projections rebuilt when a person selects a session in the rail.
 *
 * Keep this sequence explicit. Each large surface gets one small loader here
 * so it can be added, measured, and removed without waking the other surfaces.
 */
import type { OwnedSession } from './ownedSessions';
import type { AgentConversationProvider } from './conversation/conversationTypes';
import {
  ensureConversationSession,
  setConversationMode
} from './conversation/conversationStore.svelte';
import {
  loadConversationForRead,
  releaseConversationForRead
} from './conversation/conversationService';
import { countInvoke } from './devInvokeCounter.svelte';
import { activate as activateExplorer } from './explorer/explorerService';
import { resetExplorer } from './explorer/explorerStore.svelte';
import { validateProjectRootFromTauri } from '../tauriSource';

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
    this.releaseDepartingTree(session.ownedId);
    await this.fillTreeView(session, generation);
  }

  async selectSession(session: OwnedSession, displayedChatOwnedId: string | null): Promise<void> {
    const generation = ++this.selectionGeneration;
    this.releaseDepartingChat(session.ownedId, displayedChatOwnedId);
    this.releaseDepartingTree(session.ownedId);
    await this.fillTreeView(session, generation);
    await this.fillChatHistory(session, generation);
  }

  async fillTreeView(
    session: OwnedSession,
    generation = this.selectionGeneration
  ): Promise<void> {
    const root = session.cwd.trim() || (session.projectPath ?? '').trim();
    this.hasTreeProjection = true;

    if (!root) {
      this.treeOwnedId = session.ownedId;
      this.treeRoot = '';
      activateExplorer(null, true);
      return;
    }

    countInvoke('validate_project_root');
    const validation = await validateProjectRootFromTauri(root);
    if (generation !== this.selectionGeneration) return;
    if (validation && (!validation.exists || !validation.isDirectory)) {
      this.treeOwnedId = session.ownedId;
      this.treeRoot = '';
      activateExplorer(null, true);
      return;
    }

    this.treeOwnedId = session.ownedId;
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
    if (generation !== this.selectionGeneration) return;
  }

  clearTreeView(): void {
    this.hasTreeProjection = false;
    this.treeOwnedId = null;
    this.treeRoot = '';
    resetExplorer();
  }

  clearChatHistory(): void {
    this.selectionGeneration += 1;
    if (this.chatOwnedId) releaseConversationForRead(this.chatOwnedId);
    this.hasChatProjection = false;
    this.chatOwnedId = null;
  }

  private providerFor(session: OwnedSession): AgentConversationProvider | null {
    return session.agent === 'codex' || session.agent === 'claude' || session.agent === 'antigravity'
      ? session.agent
      : null;
  }

  private releaseDepartingTree(nextOwnedId: string): void {
    if (this.treeOwnedId === nextOwnedId) return;
    this.hasTreeProjection = false;
    this.treeOwnedId = null;
    this.treeRoot = '';
    resetExplorer();
  }

  private releaseDepartingChat(nextOwnedId: string, displayedChatOwnedId: string | null): void {
    const departingOwnedId = this.chatOwnedId ?? displayedChatOwnedId;
    if (!departingOwnedId || (departingOwnedId === nextOwnedId && this.hasChatProjection)) return;
    releaseConversationForRead(departingOwnedId);
    this.hasChatProjection = false;
    this.chatOwnedId = null;
  }
}
