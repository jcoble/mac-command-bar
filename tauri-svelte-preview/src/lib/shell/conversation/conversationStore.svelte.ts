/**
 * Reactive conversation state keyed by Command Bar's stable `ownedId`.
 *
 * This module performs no IO. The conversation service owns Tauri calls and
 * feeds normalized events into `applyAgentConversationEvent`.
 */
import { applyConversationEvent, createConversationState } from './conversationReducer.ts';
import type {
  AgentConversationConnection,
  AgentConversationEvent,
  AgentConversationProvider,
  AgentConversationSnapshot,
  AgentConfigValue,
  AgentWriterLease,
  AgentWriterLeaseTransition,
  ConversationAttachment,
  ConversationChildAgent,
  ConversationMetadata,
  ConversationTranscriptMessage,
  ConversationTranscriptSnapshot,
  ConversationSessionState,
  ConversationTimelineEntry
} from './conversationTypes.ts';
import type { AgentExecutionOwner } from '../ownedSessions.ts';
import {
  SESSION_CONVERSATION_WORKSPACE_VERSION,
  type SessionConversationWorkspace
} from '../sessionWorkspaces.ts';

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
  childScrollTopById: Record<string, number>;
  executionOwner: AgentExecutionOwner;
  writerLease: AgentWriterLease;
  writerLeaseTransition: AgentWriterLeaseTransition | null;
  attachmentIds: string[];
  config: Record<string, AgentConfigValue>;
  telemetry: Record<string, AgentConfigValue>;
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
    scrollTop: 0,
    childScrollTopById: {},
    executionOwner: 'stopped',
    writerLease: { ownedId, generation: 0, owner: 'none' },
    writerLeaseTransition: null,
    attachmentIds: [],
    config: {},
    telemetry: {}
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
    scrollTop: current.scrollTop,
    childScrollTopById: current.childScrollTopById,
    executionOwner: current.executionOwner,
    writerLease: { ...current.writerLease, generation: next.generation },
    writerLeaseTransition: current.writerLeaseTransition,
    attachmentIds: current.attachmentIds,
    config: current.config,
    telemetry: current.telemetry
  };
  return true;
}

export function applyAgentConversationSnapshot(snapshot: AgentConversationSnapshot): void {
  const current = ensureConversationSession(
    snapshot.connection.ownedId,
    snapshot.connection.provider
  );
  if (snapshot.connection.generation < current.generation) return;
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
    scrollTop: current.scrollTop,
    childScrollTopById: current.childScrollTopById,
    executionOwner: current.executionOwner,
    writerLease: { ...current.writerLease, generation: rebuilt.generation },
    writerLeaseTransition: current.writerLeaseTransition,
    attachmentIds: current.attachmentIds,
    config: current.config,
    telemetry: current.telemetry
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
  if (current) {
    current.attachments = attachments;
    current.attachmentIds = attachments.map((attachment) => attachment.id);
  }
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

export function setChildConversationScrollTop(
  ownedId: string,
  childId: string,
  scrollTop: number
): void {
  const current = conversationSessions[ownedId];
  if (current) current.childScrollTopById[childId] = Math.max(0, scrollTop);
}

export function setConversationWriterLeaseTransition(
  transition: AgentWriterLeaseTransition
): boolean {
  const current = conversationSessions[transition.ownedId];
  if (!current || transition.generation !== current.generation) return false;
  if (transition.from !== current.writerLease.owner) return false;
  current.writerLeaseTransition = transition;
  if (transition.state === 'committed') {
    current.writerLease = {
      ownedId: transition.ownedId,
      generation: transition.generation,
      owner: transition.to
    };
    current.executionOwner = transition.to === 'none' ? 'stopped' : transition.to;
    current.writerLeaseTransition = null;
  }
  return true;
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
  current.writerLease.generation = connection.generation;
  current.connectionState = connection.state;
  if (connection.nativeSessionId) current.nativeSessionId = connection.nativeSessionId;
}

export function captureConversationWorkspace(
  ownedId: string
): SessionConversationWorkspace | undefined {
  const current = conversationSessions[ownedId];
  if (!current) return undefined;
  return {
    version: SESSION_CONVERSATION_WORKSPACE_VERSION,
    mode: current.mode,
    draft: current.draft,
    generation: current.generation,
    owner: current.executionOwner,
    attachmentIds: current.attachmentIds,
    config: current.config,
    parentScrollTop: current.scrollTop,
    childScrollTopById: current.childScrollTopById,
    sequence: current.lastSequence,
    telemetry: current.telemetry,
    writerLease: current.writerLease,
    writerLeaseTransition: current.writerLeaseTransition,
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
  current.scrollTop = snapshot?.parentScrollTop ?? snapshot?.scrollTop ?? 0;
  current.childScrollTopById = snapshot?.childScrollTopById ?? {};
  current.executionOwner = snapshot?.owner ?? current.executionOwner;
  current.attachmentIds = snapshot?.attachmentIds ?? current.attachmentIds;
  current.config = snapshot?.config ?? current.config;
  current.telemetry = snapshot?.telemetry ?? current.telemetry;
  if (snapshot?.writerLease?.ownedId === ownedId) current.writerLease = snapshot.writerLease;
  if (snapshot?.writerLeaseTransition?.ownedId === ownedId) {
    current.writerLeaseTransition = snapshot.writerLeaseTransition;
  }
  return current;
}

export function removeConversationSession(ownedId: string): void {
  delete conversationSessions[ownedId];
}
