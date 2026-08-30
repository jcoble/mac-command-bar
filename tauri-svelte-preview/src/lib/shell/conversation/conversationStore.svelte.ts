/**
 * Reactive conversation state keyed by Command Bar's stable `ownedId`.
 *
 * This module performs no IO. The conversation service owns Tauri calls and
 * feeds normalized events into `applyAgentConversationEvent`.
 */
import type { SessionConversationWorkspace } from '../sessionWorkspaces.ts';
import { applyConversationEvent, createConversationState } from './conversationReducer.ts';
import type {
  AgentConversationConnection,
  AgentConversationEvent,
  AgentConversationProvider,
  AgentConversationSnapshot,
  ConversationAttachment,
  ConversationChildAgent,
  ConversationMetadata,
  ConversationSessionState,
  ConversationTimelineEntry,
  ConversationTranscriptMessage,
  ConversationTranscriptSnapshot
} from './conversationTypes.ts';

export type ConversationViewMode = 'structured' | 'raw';

export interface ConversationWorkspaceState extends ConversationSessionState {
  draft: string;
  mode: ConversationViewMode;
  sending: boolean;
  attachments: ConversationAttachment[];
  metadata: ConversationMetadata;
  children: ConversationChildAgent[];
  selectedChildId: string | null;
  childTimeline: ConversationTimelineEntry[];
  scrollTop: number;
}

const emptyMetadata = (): ConversationMetadata => ({
  model: null,
  effort: null,
  approvalPolicy: null,
  usedTokens: null,
  contextWindow: null
});

export const conversationSessions = $state<Record<string, ConversationWorkspaceState>>({});

function freshState(
  ownedId: string,
  provider: AgentConversationProvider
): ConversationWorkspaceState {
  return {
    ...createConversationState(ownedId, provider),
    draft: '',
    mode: 'structured',
    sending: false,
    attachments: [],
    metadata: emptyMetadata(),
    children: [],
    selectedChildId: null,
    childTimeline: [],
    scrollTop: 0
  };
}

export function getConversationSession(ownedId: string): ConversationWorkspaceState | null {
  return conversationSessions[ownedId] ?? null;
}

export function ensureConversationSession(
  ownedId: string,
  provider: AgentConversationProvider
): ConversationWorkspaceState {
  const current = conversationSessions[ownedId];
  if (current?.provider === provider) return current;
  const created = freshState(ownedId, provider);
  conversationSessions[ownedId] = created;
  return created;
}

export function applyAgentConversationEvent(event: AgentConversationEvent): boolean {
  const current = ensureConversationSession(event.ownedId, event.provider);
  const next = applyConversationEvent(current, event);
  if (next === current) return false;
  conversationSessions[event.ownedId] = {
    ...next,
    draft: current.draft,
    mode: current.mode,
    sending: current.sending,
    attachments: current.attachments,
    metadata: current.metadata,
    children: current.children,
    selectedChildId: current.selectedChildId,
    childTimeline: current.childTimeline,
    scrollTop: current.scrollTop
  };
  return true;
}

export function applyAgentConversationSnapshot(snapshot: AgentConversationSnapshot): void {
  const current = ensureConversationSession(
    snapshot.connection.ownedId,
    snapshot.connection.provider
  );
  let rebuilt = createConversationState(
    snapshot.connection.ownedId,
    snapshot.connection.provider
  );
  rebuilt.generation = snapshot.connection.generation;
  rebuilt.connectionState = snapshot.connection.state;
  rebuilt.nativeSessionId = snapshot.connection.nativeSessionId;
  for (const event of snapshot.events) rebuilt = applyConversationEvent(rebuilt, event);
  conversationSessions[snapshot.connection.ownedId] = {
    ...rebuilt,
    draft: current.draft,
    mode: current.mode,
    sending: current.sending,
    attachments: current.attachments,
    metadata: current.metadata,
    children: current.children,
    selectedChildId: current.selectedChildId,
    childTimeline: current.childTimeline,
    scrollTop: current.scrollTop
  };
}

export function applyConversationTranscript(
  ownedId: string,
  provider: AgentConversationProvider,
  snapshot: ConversationTranscriptSnapshot
): void {
  const current = ensureConversationSession(ownedId, provider);
  conversationSessions[ownedId] = {
    ...current,
    connectionState: 'connected',
    desynchronized: false,
    metadata: snapshot.metadata,
    children: snapshot.children,
    timeline: snapshot.messages.map((message) => ({
      kind: message.role,
      itemId: message.itemId,
      text: message.text,
      completed: true,
      timestampMs: message.timestampMs
    }))
  };
}

export function applyChildConversationTranscript(
  ownedId: string,
  childId: string,
  messages: readonly ConversationTranscriptMessage[]
): void {
  const current = conversationSessions[ownedId];
  if (!current || current.selectedChildId !== childId) return;
  current.childTimeline = messages.map((message) => ({
    kind: message.role,
    itemId: `child:${childId}:${message.itemId}`,
    text: message.text,
    completed: true,
    timestampMs: message.timestampMs
  }));
}

export function setConversationAttachments(ownedId: string, attachments: ConversationAttachment[]): void {
  const current = conversationSessions[ownedId];
  if (current) current.attachments = attachments;
}

export function setConversationSelectedChild(ownedId: string, childId: string | null): void {
  const current = conversationSessions[ownedId];
  if (!current || current.selectedChildId === childId) return;
  current.selectedChildId = childId;
  current.childTimeline = [];
}

export function setConversationScrollTop(ownedId: string, scrollTop: number): void {
  const current = conversationSessions[ownedId];
  if (current) current.scrollTop = Math.max(0, scrollTop);
}

export function setConversationDraft(ownedId: string, draft: string): void {
  const current = conversationSessions[ownedId];
  if (!current || current.draft === draft) return;
  current.draft = draft;
}

export function setConversationMode(ownedId: string, mode: ConversationViewMode): void {
  const current = conversationSessions[ownedId];
  if (!current || current.mode === mode) return;
  current.mode = mode;
}

export function setConversationSending(ownedId: string, sending: boolean): void {
  const current = conversationSessions[ownedId];
  if (!current || current.sending === sending) return;
  current.sending = sending;
}

export function setConversationConnection(connection: AgentConversationConnection): void {
  const current = ensureConversationSession(connection.ownedId, connection.provider);
  current.generation = connection.generation;
  current.connectionState = connection.state;
  if (connection.nativeSessionId) current.nativeSessionId = connection.nativeSessionId;
}

export function captureConversationWorkspace(
  ownedId: string
): SessionConversationWorkspace | undefined {
  const current = conversationSessions[ownedId];
  if (!current) return undefined;
  return {
    mode: current.mode,
    draft: current.draft,
    selectedChildId: current.selectedChildId,
    scrollTop: current.scrollTop,
    providerGeneration: current.generation,
    lastSequence: current.lastSequence
  };
}

export function restoreConversationWorkspace(
  ownedId: string,
  provider: AgentConversationProvider,
  snapshot: SessionConversationWorkspace | null | undefined
): ConversationWorkspaceState {
  const current = ensureConversationSession(ownedId, provider);
  current.mode = snapshot?.mode === 'raw' ? 'raw' : 'structured';
  current.draft = snapshot?.draft ?? '';
  current.selectedChildId = snapshot?.selectedChildId ?? null;
  current.scrollTop = snapshot?.scrollTop ?? 0;
  return current;
}

export function removeConversationSession(ownedId: string): void {
  delete conversationSessions[ownedId];
}
