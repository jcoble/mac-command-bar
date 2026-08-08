/**
 * Reactive conversation state keyed by Command Bar's stable `ownedId`.
 *
 * This module performs no IO. The conversation service owns Tauri calls and
 * feeds normalized events into `applyAgentConversationEvent`.
 */
import { applyConversationEvent, createConversationState } from './conversationReducer.ts';
import type {
  AgentApprovalRequest,
  AgentCapabilities,
  AgentEvent,
  AgentConversationConnection,
  AgentConversationEvent,
  AgentConversationProvider,
  AgentConversationSnapshot,
  AgentConfigValue,
  AgentItem,
  AgentUserInputRequest,
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
import { agentItemFromEvent, type AgentPlanStep, type ConversationTask } from './conversationTimeline.ts';
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
  /** The provider's authoritative capability snapshot for this owned session. */
  capabilities: AgentCapabilities | null;
  capabilityError: string | null;
  /** Typed ACP items are kept beside the legacy reducer projection. */
  agentItems: AgentItem[];
  planSteps: AgentPlanStep[];
  tasks: ConversationTask[];
  pendingApprovals: Record<string, AgentApprovalRequest & { state: string }>;
  pendingInputs: Record<string, AgentUserInputRequest>;
  pendingConfig: Record<string, AgentConfigValue>;
  configErrors: Record<string, string>;
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
    telemetry: {},
    capabilities: null,
    capabilityError: null,
    agentItems: [],
    planSteps: [],
    tasks: [],
    pendingApprovals: {},
    pendingInputs: {},
    pendingConfig: {},
    configErrors: {}
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

export function applyAgentConversationEvent(event: AgentConversationEvent | AgentEvent): boolean {
  if ('type' in event) return applyCanonicalAgentEvent(event);
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
    telemetry: current.telemetry,
    capabilities: current.capabilities,
    capabilityError: current.capabilityError,
    agentItems: current.agentItems,
    planSteps: current.planSteps,
    tasks: current.tasks,
    pendingApprovals: current.pendingApprovals,
    pendingInputs: current.pendingInputs,
    pendingConfig: current.pendingConfig,
    configErrors: current.configErrors
  };
  const typedItem = agentItemFromEvent(event);
  if (typedItem) {
    const delta = event.payload.kind === 'assistantDelta';
    conversationSessions[event.ownedId].agentItems = upsertAgentItem(
      conversationSessions[event.ownedId].agentItems,
      typedItem,
      delta
    );
  }
  applyTypedEventPayload(conversationSessions[event.ownedId], event);
  return true;
}

function applyCanonicalAgentEvent(event: AgentEvent): boolean {
  const current = ensureConversationSession(event.ownedId, event.provider);
  if (event.generation < current.generation) return false;
  const newGeneration = event.generation > current.generation;
  const previousSequence = newGeneration ? 0 : current.lastSequence;
  if (!newGeneration && event.sequence <= previousSequence) return false;
  const hasGap = event.sequence !== previousSequence + 1;
  if (hasGap) {
    conversationSessions[event.ownedId] = { ...current, generation: event.generation, lastSequence: event.sequence, desynchronized: true };
    return true;
  }
  const payload = event.payload as Record<string, unknown>;
  const nextState: ConversationWorkspaceState = {
    ...current,
    generation: event.generation,
    lastSequence: event.sequence,
    desynchronized: newGeneration ? false : current.desynchronized,
    connectionState: event.type === 'session.closed' ? 'closed'
      : event.type === 'session.started' ? 'connected'
        : event.type === 'runtime.error' && payload.recoverable === false ? 'failed' : current.connectionState,
    nativeSessionId: event.nativeSessionId ?? current.nativeSessionId,
    writerLease: { ...current.writerLease, generation: event.generation }
  };
  conversationSessions[event.ownedId] = nextState;
  const target = conversationSessions[event.ownedId];
  const typedItem = agentItemFromEvent(event);
  if (typedItem) {
    target.agentItems = upsertAgentItem(target.agentItems, typedItem, event.type === 'content.delta');
  }
  applyTypedEventPayload(target, event);
  return true;
}

function upsertAgentItem(items: AgentItem[], incoming: AgentItem, delta: boolean): AgentItem[] {
  const index = items.findIndex((item) => item.id === incoming.id);
  if (index < 0) return [...items, incoming];
  const existing = items[index];
  const content = delta && existing.content.length && incoming.content.length
    && existing.content[existing.content.length - 1].channel === incoming.content[0].channel
    ? [
      ...existing.content.slice(0, -1),
      {
        ...existing.content[existing.content.length - 1],
        text: `${existing.content[existing.content.length - 1].text}${incoming.content[0].text}`
      },
      ...incoming.content.slice(1)
    ]
    : incoming.content.length ? incoming.content : existing.content;
  return items.map((item, itemIndex) => itemIndex === index ? { ...item, ...incoming, content } : item);
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
    telemetry: current.telemetry,
    capabilities: current.capabilities,
    capabilityError: current.capabilityError,
    agentItems: current.agentItems,
    planSteps: current.planSteps,
    tasks: current.tasks,
    pendingApprovals: current.pendingApprovals,
    pendingInputs: current.pendingInputs,
    pendingConfig: current.pendingConfig,
    configErrors: current.configErrors
  };
  const restored = conversationSessions[snapshot.connection.ownedId];
  for (const event of snapshot.events) {
    const typedItem = agentItemFromEvent(event);
    if (typedItem) {
      restored.agentItems = upsertAgentItem(
        restored.agentItems,
        typedItem,
        event.payload.kind === 'assistantDelta'
      );
    }
    applyTypedEventPayload(restored, event);
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

function asString(value: unknown): string | null {
  return typeof value === 'string' && value.length > 0 ? value : null;
}

function applyTypedEventPayload(current: ConversationWorkspaceState, event: AgentConversationEvent | AgentEvent): void {
  const payload = event.payload as Record<string, unknown>;
  const raw = isRecord(payload);
  if (!raw) return;
  const eventType: string = 'type' in event
    ? event.type
    : payload.kind === 'approval' ? 'approval.requested' : payload.kind === 'error' ? 'runtime.error' : '';
  const requestIdFromEvent = 'requestId' in event ? event.requestId : undefined;
  const turnIdFromEvent = 'turnId' in event ? event.turnId : undefined;
  const itemIdFromEvent = 'itemId' in event ? event.itemId : undefined;
  const capabilities = isRecord(payload.capabilities) ? payload.capabilities as unknown as AgentCapabilities : null;
  if (capabilities && Array.isArray(capabilities.configOptions) && Array.isArray(capabilities.commands)) {
    current.capabilities = capabilities;
    current.capabilityError = null;
  }
  if (eventType === 'plan.updated' || payload.kind === 'plan') {
    if (Array.isArray(payload.steps)) current.planSteps = parsePlanSteps(payload.steps);
  }
  if (eventType === 'tasks.updated' || payload.kind === 'tasks') {
    if (Array.isArray(payload.tasks)) current.tasks = parseTasks(payload.tasks);
  }
  if (eventType === 'approval.requested') {
    const requestId = asString(payload.requestId) ?? requestIdFromEvent;
    const legacyState = payload.kind === 'approval' ? asString(payload.state) : null;
    if (requestId && legacyState && legacyState !== 'requested') delete current.pendingApprovals[requestId];
    else if (requestId) current.pendingApprovals[requestId] = {
        ownedId: current.ownedId,
        generation: current.generation,
        requestId,
        turnId: turnIdFromEvent,
        itemId: itemIdFromEvent,
        title: asString(payload.title) ?? 'Approval requested',
        description: asString(payload.description) ?? undefined,
        options: Array.isArray(payload.options) ? payload.options.filter((value): value is 'accept' | 'decline' | 'cancel' => value === 'accept' || value === 'decline' || value === 'cancel') : ['accept', 'decline'],
        state: 'requested'
      };
  }
  if (eventType === 'approval.resolved') {
    const requestId = asString(payload.requestId) ?? requestIdFromEvent;
    if (requestId) delete current.pendingApprovals[requestId];
  }
  if (eventType === 'user-input.requested') {
    const requestId = asString(payload.requestId) ?? requestIdFromEvent;
    if (requestId && Array.isArray(payload.fields)) {
      current.pendingInputs[requestId] = {
        ownedId: current.ownedId,
        generation: current.generation,
        requestId,
        turnId: turnIdFromEvent,
        itemId: itemIdFromEvent,
        title: asString(payload.title) ?? 'Input requested',
        description: asString(payload.description) ?? undefined,
        fields: payload.fields as AgentUserInputRequest['fields']
      };
    }
  }
  if (eventType === 'user-input.resolved') {
    const requestId = asString(payload.requestId) ?? requestIdFromEvent;
    if (requestId) delete current.pendingInputs[requestId];
  }
}

function parsePlanSteps(value: unknown): AgentPlanStep[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((entry, index) => {
    if (!isRecord(entry)) return [];
    const state = asString(entry.state);
    return [{
      id: asString(entry.id) ?? `step-${index + 1}`,
      title: asString(entry.title) ?? `Step ${index + 1}`,
      detail: asString(entry.detail),
      state: state === 'in-progress' || state === 'completed' || state === 'failed' || state === 'blocked' ? state : 'pending',
      ownerAgentId: asString(entry.ownerAgentId),
      startedAt: asString(entry.startedAt),
      completedAt: asString(entry.completedAt)
    }];
  });
}

function parseTasks(value: unknown): ConversationTask[] {
  return parsePlanSteps(value).map((step) => ({
    id: step.id,
    title: step.title,
    detail: step.detail,
    state: step.state,
    ownerAgentId: step.ownerAgentId
  }));
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

export function setConversationCapabilities(ownedId: string, capabilities: AgentCapabilities): void {
  const current = conversationSessions[ownedId];
  if (!current || capabilities.provider !== current.provider) return;
  current.capabilities = capabilities;
  current.capabilityError = null;
}

export function setConversationCapabilityError(ownedId: string, message: string | null): void {
  const current = conversationSessions[ownedId];
  if (current) current.capabilityError = message;
}

export function beginConversationConfigChange(
  ownedId: string,
  optionId: string,
  value: AgentConfigValue
): boolean {
  const current = conversationSessions[ownedId];
  const option = current?.capabilities?.configOptions.find((candidate) => candidate.id === optionId);
  if (!current || !option) return false;
  if (option.choices && !option.choices.some((choice) => JSON.stringify(choice.value) === JSON.stringify(value))) return false;
  current.pendingConfig[optionId] = value;
  delete current.configErrors[optionId];
  return true;
}

export function confirmConversationConfigChange(
  ownedId: string,
  optionId: string,
  value: AgentConfigValue,
  capabilities?: AgentCapabilities | null
): boolean {
  const current = conversationSessions[ownedId];
  if (!current || !(optionId in current.pendingConfig)) return false;
  current.config[optionId] = value;
  delete current.pendingConfig[optionId];
  delete current.configErrors[optionId];
  if (capabilities && capabilities.provider === current.provider) current.capabilities = capabilities;
  return true;
}

export function failConversationConfigChange(ownedId: string, optionId: string, message: string): void {
  const current = conversationSessions[ownedId];
  if (!current) return;
  delete current.pendingConfig[optionId];
  current.configErrors[optionId] = message;
}

export function setConversationAgentItems(ownedId: string, items: AgentItem[]): void {
  const current = conversationSessions[ownedId];
  if (current) current.agentItems = [...items];
}

export function setConversationPlanSteps(ownedId: string, steps: AgentPlanStep[]): void {
  const current = conversationSessions[ownedId];
  if (current) current.planSteps = [...steps];
}

export function setConversationTasks(ownedId: string, tasks: ConversationTask[]): void {
  const current = conversationSessions[ownedId];
  if (current) current.tasks = [...tasks];
}

export function setConversationPendingApproval(
  ownedId: string,
  request: AgentApprovalRequest & { state: string }
): void {
  const current = conversationSessions[ownedId];
  if (current && request.ownedId === ownedId && request.generation === current.generation) {
    current.pendingApprovals[request.requestId] = request;
  }
}

export function setConversationPendingInput(ownedId: string, request: AgentUserInputRequest): void {
  const current = conversationSessions[ownedId];
  if (current && request.ownedId === ownedId && request.generation === current.generation) {
    current.pendingInputs[request.requestId] = request;
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

export function clearConversationWriterLeaseTransition(ownedId: string, generation: number): void {
  const current = conversationSessions[ownedId];
  if (current?.generation === generation) current.writerLeaseTransition = null;
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
