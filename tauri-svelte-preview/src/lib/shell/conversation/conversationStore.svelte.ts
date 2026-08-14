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
  AgentCommandDescriptor,
  AgentEvent,
  AgentConversationConnection,
  AgentConversationEvent,
  AgentConversationProvider,
  AgentConversationSnapshot,
  AgentConfigValue,
  AgentItem,
  AgentPermissionOption,
  AgentPermissionRequest,
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
import {
  agentItemFromEvent,
  availableCommandsFromEvent,
  conversationEventAppendsItemContent,
  mergeAgentItem,
  permissionRequestFromEvent,
  type AgentPlanStep,
  type ConversationTask
} from './conversationTimeline.ts';
import {
  emptyAgentConversationConfigState,
  type AgentConversationConfigField,
  type AgentConversationConfigState
} from './conversationConfig.ts';
import type { AgentExecutionOwner } from '../ownedSessions.ts';
import {
  clearSessionPresence,
  synchronizeSessionPresenceWork
} from './sessionPresence.ts';
import { recordConversationPresenceEvent } from './sessionNotifications.ts';
import {
  SESSION_CONVERSATION_WORKSPACE_VERSION,
  type SessionConversationWorkspace
} from '../sessionWorkspaces.ts';

export type ConversationViewMode = 'structured' | 'raw';

export interface ConversationRecentEvent {
  sequence: number;
  kind: string;
  summary: string;
  timestampMs: number;
}

export const CONVERSATION_RECENT_EVENT_CAP = 200;

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
  /** Screenshots that went out with a send, keyed by the user message they
   * produced. No provider echoes an image back, so this local copy is the only
   * way the transcript can show what the reader actually sent. */
  sentAttachments: Record<string, ConversationAttachment[]>;
  /** Sent but not yet claimed by a user message event. */
  unclaimedSentAttachments: ConversationAttachment[];
  config: Record<string, AgentConfigValue>;
  agentConfig: AgentConversationConfigState;
  pendingAgentConfig: Partial<Record<AgentConversationConfigField, string>>;
  agentConfigError: string | null;
  telemetry: Record<string, AgentConfigValue>;
  /** The provider's authoritative capability snapshot for this owned session. */
  capabilities: AgentCapabilities | null;
  capabilityError: string | null;
  /** Typed ACP items are kept beside the legacy reducer projection. */
  agentItems: AgentItem[];
  planSteps: AgentPlanStep[];
  tasks: ConversationTask[];
  availableCommands: AgentCommandDescriptor[];
  pendingApprovals: Record<string, AgentPermissionRequest>;
  pendingInputs: Record<string, AgentUserInputRequest>;
  pendingConfig: Record<string, AgentConfigValue>;
  configErrors: Record<string, string>;
  recentEvents: ConversationRecentEvent[];
}

const emptyMetadata = (): ConversationMetadata => ({
  model: null,
  effort: null,
  approvalPolicy: null,
  usedTokens: null,
  contextWindow: null
});

export const conversationSessions = $state<Record<string, ConversationWorkspaceState>>({});

const timelineIndexBySession = new WeakMap<ConversationWorkspaceState, Map<string, number>>();
const agentItemIndexBySession = new WeakMap<ConversationWorkspaceState, Map<string, number>>();
const activeReasoningBySession = new WeakMap<ConversationWorkspaceState, Set<AgentItem>>();

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
    sentAttachments: {},
    unclaimedSentAttachments: [],
    config: {},
    agentConfig: emptyAgentConversationConfigState(),
    pendingAgentConfig: {},
    agentConfigError: null,
    telemetry: {},
    capabilities: null,
    capabilityError: null,
    agentItems: [],
    planSteps: [],
    tasks: [],
    availableCommands: [],
    pendingApprovals: {},
    pendingInputs: {},
    pendingConfig: {},
    configErrors: {},
    recentEvents: []
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
  const current = ensureConversationSession(event.ownedId, event.provider);
  appendRecentEvent(current, event);
  if ('type' in event) {
    const applied = applyCanonicalAgentEvent(event);
    if (applied && !conversationSessions[event.ownedId]?.desynchronized) {
      recordConversationPresenceEvent(event);
    }
    return applied;
  }
  const applied = applyLegacyEventInPlace(current, event);
  if (!applied) return false;
  const typedItem = agentItemFromEvent(event);
  if (typedItem) {
    if (mergeAgentItemInPlace(current, typedItem, conversationEventAppendsItemContent(event))
      && !['userMessage', 'assistantDelta', 'assistantMessage', 'tool'].includes(event.payload.kind)) {
      current.timelineRevision += 1;
    }
  }
  applyTypedEventPayload(current, event);
  if (!current.desynchronized) recordConversationPresenceEvent(event);
  return true;
}

function timelineIndex(current: ConversationWorkspaceState): Map<string, number> {
  let index = timelineIndexBySession.get(current);
  if (!index) {
    index = new Map(current.timeline.map((entry, entryIndex) => [entry.itemId, entryIndex]));
    timelineIndexBySession.set(current, index);
  }
  return index;
}

function timelineEntry(current: ConversationWorkspaceState, itemId: string): ConversationTimelineEntry | undefined {
  const index = timelineIndex(current).get(itemId);
  return index === undefined ? undefined : current.timeline[index];
}

function appendTimelineEntry(current: ConversationWorkspaceState, entry: ConversationTimelineEntry): void {
  timelineIndex(current).set(entry.itemId, current.timeline.length);
  current.timeline.push(entry);
}

function applyLegacyEventInPlace(current: ConversationWorkspaceState, event: AgentConversationEvent): boolean {
  if (event.ownedId !== current.ownedId || event.provider !== current.provider) return false;
  if (event.generation < current.generation) return false;
  const newGeneration = event.generation > current.generation;
  const previousSequence = newGeneration ? 0 : current.lastSequence;
  if (!newGeneration && event.sequence <= previousSequence) return false;
  const hasGap = event.sequence !== previousSequence + 1;
  current.generation = event.generation;
  current.lastSequence = event.sequence;
  current.desynchronized = newGeneration ? hasGap : current.desynchronized || hasGap;
  current.writerLease.generation = event.generation;
  if (hasGap) return true;

  const { payload } = event;
  let displayChanged = false;
  switch (payload.kind) {
    case 'connection':
      current.connectionState = payload.state;
      current.nativeSessionId = payload.nativeSessionId ?? current.nativeSessionId;
      break;
    case 'userMessage': {
      const existing = timelineEntry(current, payload.itemId);
      if (existing?.kind === 'user') {
        displayChanged = existing.text !== payload.text || existing.completed !== payload.completed;
        existing.text = payload.text;
        existing.completed = payload.completed;
      } else {
        appendTimelineEntry(current, {
          kind: 'user', itemId: payload.itemId, text: payload.text,
          completed: payload.completed, timestampMs: event.timestampMs
        });
        if (current.unclaimedSentAttachments.length) {
          current.sentAttachments[payload.itemId] = current.unclaimedSentAttachments;
          current.unclaimedSentAttachments = [];
        }
        displayChanged = true;
      }
      break;
    }
    case 'assistantDelta': {
      const existing = timelineEntry(current, payload.itemId);
      if (existing?.kind === 'assistant') {
        if (payload.delta) {
          existing.text += payload.delta;
          displayChanged = true;
        }
        existing.completed = false;
      } else {
        appendTimelineEntry(current, {
          kind: 'assistant', itemId: payload.itemId, text: payload.delta,
          completed: false, timestampMs: event.timestampMs
        });
        displayChanged = payload.delta.length > 0;
      }
      break;
    }
    case 'assistantMessage': {
      const existing = timelineEntry(current, payload.itemId);
      if (existing?.kind === 'assistant') {
        displayChanged = existing.text !== payload.text || !existing.completed;
        existing.text = payload.text;
        existing.completed = true;
      } else {
        appendTimelineEntry(current, {
          kind: 'assistant', itemId: payload.itemId, text: payload.text,
          completed: true, timestampMs: event.timestampMs
        });
        displayChanged = true;
      }
      break;
    }
    case 'tool': {
      const existing = timelineEntry(current, payload.itemId);
      if (existing?.kind === 'tool') {
        displayChanged = existing.name !== payload.name || existing.state !== payload.state || existing.summary !== payload.summary;
        existing.name = payload.name;
        existing.state = payload.state;
        existing.summary = payload.summary;
      } else {
        appendTimelineEntry(current, {
          kind: 'tool', itemId: payload.itemId, name: payload.name,
          state: payload.state, summary: payload.summary, timestampMs: event.timestampMs
        });
        displayChanged = true;
      }
      break;
    }
    case 'approval': {
      const itemId = `approval:${payload.requestId}`;
      const existing = timelineEntry(current, itemId);
      if (existing?.kind === 'approval') {
        displayChanged = existing.state !== payload.state || existing.summary !== payload.summary;
        existing.state = payload.state;
        existing.summary = payload.summary;
      } else {
        appendTimelineEntry(current, {
          kind: 'approval', itemId, requestId: payload.requestId,
          state: payload.state, summary: payload.summary, timestampMs: event.timestampMs
        });
        displayChanged = true;
      }
      break;
    }
    case 'plan': {
      const itemId = `plan:${event.generation}`;
      const items = payload.items ?? (payload.entries ?? []).map((entry) => ({
        text: entry.title ?? entry.text ?? entry.content ?? '',
        status: entry.status ?? 'pending'
      }));
      const existing = timelineEntry(current, itemId);
      if (existing?.kind === 'plan') {
        existing.items = items;
      } else {
        appendTimelineEntry(current, { kind: 'plan', itemId, items, timestampMs: event.timestampMs });
      }
      displayChanged = true;
      break;
    }
    case 'error': {
      appendTimelineEntry(current, {
        kind: 'error', itemId: `error:${event.generation}:${event.sequence}`,
        code: payload.code, message: payload.message, recoverable: payload.recoverable,
        timestampMs: event.timestampMs
      });
      current.activeTurnId = undefined;
      if (!payload.recoverable) current.connectionState = 'failed';
      displayChanged = true;
      break;
    }
    case 'turn':
      current.activeTurnId = payload.state === 'started' ? payload.turnId : undefined;
      break;
    case 'usage':
      current.usage = {
        inputTokens: payload.inputTokens ?? current.usage?.inputTokens,
        outputTokens: payload.outputTokens ?? current.usage?.outputTokens,
        usedTokens: payload.usedTokens ?? current.usage?.usedTokens,
        contextWindow: payload.contextWindow ?? current.usage?.contextWindow
      };
      break;
    case 'agentThoughtChunk':
    case 'toolCall':
    case 'toolCallUpdate':
    case 'turnDiff':
    case 'permissionRequest':
    case 'availableCommandsUpdate':
    case 'agentMessageChunk':
    case 'userMessageChunk':
      break;
  }
  if (displayChanged) current.timelineRevision += 1;
  return true;
}

function agentItemIndex(current: ConversationWorkspaceState): Map<string, number> {
  let index = agentItemIndexBySession.get(current);
  if (!index) {
    index = new Map(current.agentItems.map((item, itemIndex) => [item.id, itemIndex]));
    agentItemIndexBySession.set(current, index);
  }
  return index;
}

function mergeAgentItemInPlace(current: ConversationWorkspaceState, incoming: AgentItem, append: boolean): boolean {
  const index = agentItemIndex(current);
  const itemIndex = index.get(incoming.id);
  if (itemIndex === undefined) {
    index.set(incoming.id, current.agentItems.length);
    current.agentItems.push(incoming);
    if (incoming.type === 'reasoning' && incoming.providerMetadata?.completed !== true) {
      const active = activeReasoningBySession.get(current) ?? new Set<AgentItem>();
      active.add(current.agentItems[current.agentItems.length - 1]);
      activeReasoningBySession.set(current, active);
    }
    return true;
  }
  const existing = current.agentItems[itemIndex];
  const merged = mergeAgentItem([existing], incoming, append)[0];
  if (merged === existing) return false;
  existing.type = merged.type;
  existing.turnId = merged.turnId;
  existing.content = merged.content;
  existing.providerMetadata = merged.providerMetadata;
  return true;
}

export function conversationRecentEvents(ownedId: string): ReadonlyArray<ConversationRecentEvent> {
  return conversationSessions[ownedId]?.recentEvents ?? [];
}

function appendRecentEvent(
  current: ConversationWorkspaceState,
  event: AgentConversationEvent | AgentEvent
): void {
  const recent = current.recentEvents ?? [];
  current.recentEvents = [
    ...recent,
    {
      sequence: event.sequence,
      kind: 'type' in event ? event.type : event.payload.kind,
      summary: summarizeRecentEvent(event),
      timestampMs: event.timestampMs
    }
  ].slice(-CONVERSATION_RECENT_EVENT_CAP);
}

function summarizeRecentEvent(event: AgentConversationEvent | AgentEvent): string {
  const payload = event.payload as Record<string, unknown>;
  for (const key of ['summary', 'text', 'delta', 'message', 'name', 'state', 'code']) {
    const value = payload[key];
    if (typeof value === 'string' && value.trim()) return value.trim().slice(0, 240);
  }
  if (Array.isArray(payload.items)) return `${payload.items.length} items`;
  if (Array.isArray(payload.tasks)) return `${payload.tasks.length} tasks`;
  return 'Event received';
}

function applyCanonicalAgentEvent(event: AgentEvent): boolean {
  const current = ensureConversationSession(event.ownedId, event.provider);
  if (event.generation < current.generation) return false;
  const newGeneration = event.generation > current.generation;
  const previousSequence = newGeneration ? 0 : current.lastSequence;
  if (!newGeneration && event.sequence <= previousSequence) return false;
  const hasGap = event.sequence !== previousSequence + 1;
  if (hasGap) {
    current.generation = event.generation;
    current.lastSequence = event.sequence;
    current.desynchronized = true;
    current.writerLease.generation = event.generation;
    return true;
  }
  const payload = event.payload as Record<string, unknown>;
  current.generation = event.generation;
  current.lastSequence = event.sequence;
  current.desynchronized = newGeneration ? false : current.desynchronized;
  current.connectionState = event.type === 'session.closed' ? 'closed'
      : event.type === 'session.started' ? 'connected'
        : event.type === 'runtime.error' && payload.recoverable === false ? 'failed' : current.connectionState;
  current.activeTurnId = event.type === 'turn.started'
      ? event.turnId ?? asString(payload.turnId) ?? current.activeTurnId
      : event.type === 'turn.completed'
        || event.type === 'turn.interrupted'
        || event.type === 'session.closed'
        || event.type === 'runtime.error'
        ? undefined
        : current.activeTurnId;
  current.nativeSessionId = event.nativeSessionId ?? current.nativeSessionId;
  current.writerLease.generation = event.generation;
  const typedItem = agentItemFromEvent(event);
  if (typedItem) {
    if (mergeAgentItemInPlace(current, typedItem, conversationEventAppendsItemContent(event))) {
      current.timelineRevision += 1;
    }
  }
  applyTypedEventPayload(current, event);
  return true;
}

/**
 * Historical replay from some ACP adapters can repeat a complete assistant
 * chunk with the same item id. Keep the event sequence intact for snapshot gap
 * checks, but make only the repeated delta empty while rebuilding the view.
 */
function idempotentSnapshotEvents(events: AgentConversationEvent[]): AgentConversationEvent[] {
  let previous: { itemId: string; delta: string } | null = null;
  const replayedChunksByItem = new Map<string, Set<string>>();
  const completedItemIds = new Set<string>();
  let insideCompletedItemReplay = false;
  return events.map((event) => {
    if (event.payload.kind === 'userMessage' || event.payload.kind === 'assistantMessage') {
      if (completedItemIds.has(event.payload.itemId)) insideCompletedItemReplay = true;
      completedItemIds.add(event.payload.itemId);
      previous = null;
      // A resumed provider can replay its whole completed history as one
      // contiguous block. The first repeated stable id identifies that block;
      // suppress it and its following completed items while preserving journal
      // sequence continuity for the snapshot reducer.
      return insideCompletedItemReplay
        ? { ...event, payload: { kind: 'usage' } }
        : event;
    }
    insideCompletedItemReplay = false;
    if (event.payload.kind !== 'assistantDelta') {
      previous = null;
      return event;
    }
    const current = { itemId: event.payload.itemId, delta: event.payload.delta };
    const metadata = (event.payload as unknown as { _meta?: { replay?: boolean } })._meta;
    const seen = replayedChunksByItem.get(current.itemId) ?? new Set<string>();
    const duplicateReplay = metadata?.replay === true && seen.has(current.delta);
    if (metadata?.replay === true && current.delta.length > 0) {
      seen.add(current.delta);
      replayedChunksByItem.set(current.itemId, seen);
    }
    const duplicate = current.delta.length > 0 && (
      duplicateReplay
      || (previous?.itemId === current.itemId && previous.delta === current.delta)
    );
    previous = current;
    return duplicate
      ? { ...event, payload: { ...event.payload, delta: '' } }
      : event;
  });
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
  const events = idempotentSnapshotEvents(snapshot.events);
  const firstEvent = events[0];
  // A read snapshot is deliberately a bounded tail window. Seed the reducer
  // immediately before that window so the first retained event is contiguous
  // without pretending the omitted older journal was materialized.
  rebuilt.generation = firstEvent?.generation ?? snapshot.connection.generation;
  rebuilt.lastSequence = firstEvent ? Math.max(0, firstEvent.sequence - 1) : 0;
  rebuilt.connectionState = snapshot.connection.state;
  rebuilt.nativeSessionId = snapshot.connection.nativeSessionId;
  for (const event of events) {
    rebuilt = applyConversationEvent(rebuilt, event);
  }
  // Build the complete snapshot off the reactive graph. Publishing this object
  // before replay made every event traverse Svelte's deep proxy machinery and
  // invalidated subscribers 2,000 times during a read-only load.
  const restored: ConversationWorkspaceState = {
    ...rebuilt,
    suspended: snapshot.suspended === true,
    timelineRevision: current.timelineRevision + 1,
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
    sentAttachments: current.sentAttachments,
    unclaimedSentAttachments: current.unclaimedSentAttachments,
    config: current.config,
    telemetry: current.telemetry,
    capabilities: current.capabilities,
    capabilityError: current.capabilityError,
    agentItems: [],
    planSteps: [],
    tasks: [],
    availableCommands: current.availableCommands,
    pendingApprovals: {},
    pendingInputs: {},
    pendingConfig: current.pendingConfig,
    configErrors: current.configErrors,
    agentConfig: current.agentConfig,
    pendingAgentConfig: current.pendingAgentConfig,
    agentConfigError: current.agentConfigError,
    recentEvents: []
  };
  for (const event of events) {
    const typedItem = agentItemFromEvent(event);
    if (typedItem) {
      mergeAgentItemInPlace(restored, typedItem, conversationEventAppendsItemContent(event));
    }
    applyTypedEventPayload(restored, event);
  }
  restored.recentEvents = events.slice(-CONVERSATION_RECENT_EVENT_CAP).map((event) => ({
    sequence: event.sequence,
    kind: String('type' in event ? event.type : event.payload.kind),
    summary: summarizeRecentEvent(event),
    timestampMs: event.timestampMs
  }));
  // One reactive publication: subscribers see only the finished snapshot.
  conversationSessions[snapshot.connection.ownedId] = restored;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

function asString(value: unknown): string | null {
  return typeof value === 'string' && value.length > 0 ? value : null;
}

function permissionOptions(value: unknown): AgentPermissionOption[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((entry) => {
    if (typeof entry === 'string') {
      return [{
        optionId: entry,
        name: entry === 'accept' ? 'Allow' : entry === 'decline' || entry === 'cancel' ? 'Deny' : entry
      }];
    }
    if (!isRecord(entry)) return [];
    const optionId = asString(entry.optionId) ?? asString(entry.id);
    if (!optionId) return [];
    return [{
      optionId,
      name: asString(entry.name) ?? asString(entry.label) ?? optionId,
      kind: asString(entry.kind) ?? undefined
    }];
  });
}

function finishReasoningItems(current: ConversationWorkspaceState, event: AgentConversationEvent | AgentEvent): void {
  const payload = event.payload as Record<string, unknown>;
  const rawKind = asString(payload.kind)?.replace(/_([a-z])/g, (_, letter: string) => letter.toUpperCase()) ?? '';
  const turnFinished = rawKind === 'turn' && payload.state !== 'started';
  const assistantStarted = rawKind === 'assistantDelta' || rawKind === 'assistantMessage' || rawKind === 'agentMessageChunk';
  const canonicalFinished = 'type' in event && (event.type === 'turn.completed' || event.type === 'turn.interrupted');
  const canonicalAssistant = 'type' in event && event.type === 'content.delta' && payload.channel === 'assistant';
  if (!turnFinished && !assistantStarted && !canonicalFinished && !canonicalAssistant) return;
  const turnId = 'turnId' in event ? event.turnId : asString(payload.turnId) ?? undefined;
  const active = activeReasoningBySession.get(current);
  if (!active?.size) return;
  for (const item of active) {
    if (turnId && item.turnId && item.turnId !== turnId) continue;
    item.providerMetadata = { ...(item.providerMetadata ?? {}), completed: true, streaming: false };
    active.delete(item);
  }
}

function applyTypedEventPayload(current: ConversationWorkspaceState, event: AgentConversationEvent | AgentEvent): void {
  const payload = event.payload as Record<string, unknown>;
  const raw = isRecord(payload);
  if (!raw) return;
  const eventType: string = 'type' in event
    ? event.type
    : payload.kind === 'approval' ? 'approval.requested'
      : payload.kind === 'error' ? 'runtime.error'
        : payload.kind === 'usage' ? 'usage.updated' : '';
  const requestIdFromEvent = 'requestId' in event ? event.requestId : undefined;
  const turnIdFromEvent = 'turnId' in event ? event.turnId : undefined;
  const itemIdFromEvent = 'itemId' in event ? event.itemId : undefined;
  const capabilities = isRecord(payload.capabilities) ? payload.capabilities as unknown as AgentCapabilities : null;
  if (capabilities && Array.isArray(capabilities.configOptions) && Array.isArray(capabilities.commands)) {
    current.capabilities = capabilities;
    current.availableCommands = capabilities.commands;
    current.capabilityError = null;
  }
  const commands = availableCommandsFromEvent(event);
  if (commands) {
    current.availableCommands = commands;
    if (current.capabilities) current.capabilities = { ...current.capabilities, commands };
  }
  if (eventType === 'usage.updated' || payload.kind === 'usage') {
    const inputTokens = typeof payload.inputTokens === 'number' && Number.isFinite(payload.inputTokens)
      ? payload.inputTokens : undefined;
    const outputTokens = typeof payload.outputTokens === 'number' && Number.isFinite(payload.outputTokens)
      ? payload.outputTokens : undefined;
    const usedTokens = typeof payload.usedTokens === 'number' && Number.isFinite(payload.usedTokens)
      ? payload.usedTokens : undefined;
    const contextWindow = typeof payload.contextWindow === 'number' && Number.isFinite(payload.contextWindow)
      ? payload.contextWindow : undefined;
    current.usage = {
      inputTokens: inputTokens ?? current.usage?.inputTokens,
      outputTokens: outputTokens ?? current.usage?.outputTokens,
      usedTokens: usedTokens ?? current.usage?.usedTokens,
      contextWindow: contextWindow ?? current.usage?.contextWindow
    };
    if (usedTokens !== undefined || contextWindow !== undefined) {
      current.metadata = {
        ...current.metadata,
        usedTokens: usedTokens ?? current.metadata.usedTokens,
        contextWindow: contextWindow ?? current.metadata.contextWindow
      };
    }
  }
  if (eventType === 'plan.updated' || payload.kind === 'plan') {
    const entries = Array.isArray(payload.items) ? payload.items : payload.entries;
    if (Array.isArray(entries)) current.planSteps = parsePlanSteps(entries);
  }
  if (eventType === 'tasks.updated' || payload.kind === 'tasks') {
    if (Array.isArray(payload.tasks)) current.tasks = parseTasks(payload.tasks);
  }
  const richPermission = permissionRequestFromEvent(event);
  if (richPermission) {
    if (richPermission.state === 'requested') current.pendingApprovals[richPermission.requestId] = richPermission;
    else delete current.pendingApprovals[richPermission.requestId];
  } else if (eventType === 'approval.requested') {
    const requestId = asString(payload.requestId) ?? requestIdFromEvent;
    if (requestId) {
      const tool = isRecord(payload.toolCall) ? payload.toolCall : null;
      const toolTitle = asString(payload.toolTitle) ?? asString(tool?.title) ?? asString(payload.summary) ?? asString(payload.title) ?? 'Permission requested';
      const options = permissionOptions(payload.options);
      current.pendingApprovals[requestId] = {
        ownedId: current.ownedId,
        generation: current.generation,
        requestId,
        turnId: turnIdFromEvent,
        itemId: itemIdFromEvent,
        title: asString(payload.title) ?? 'Approval needed',
        toolTitle,
        description: asString(payload.description) ?? asString(payload.summary) ?? undefined,
        options: options.length ? options : [
          { optionId: 'accept', name: 'Allow', kind: 'allow_once' },
          { optionId: 'decline', name: 'Deny', kind: 'reject_once' }
        ],
        state: 'requested'
      };
    }
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
  finishReasoningItems(current, event);
}

function parsePlanSteps(value: unknown): AgentPlanStep[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((entry, index) => {
    if (!isRecord(entry)) return [];
    const state = asString(entry.state);
    return [{
      id: asString(entry.id) ?? `step-${index + 1}`,
      title: asString(entry.title) ?? asString(entry.text) ?? `Step ${index + 1}`,
      detail: asString(entry.detail),
      state: state === 'in-progress' || state === 'completed' || state === 'failed' || state === 'blocked'
        ? state
        : (() => {
          const status = asString(entry.status)?.replaceAll('_', '-');
          return status === 'in-progress' || status === 'completed' || status === 'failed' || status === 'blocked'
            ? status
            : 'pending';
        })(),
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

/** Hold the screenshots a send is delivering until the user message they
 * produce arrives, so the transcript can show them beside the typed text.
 * An empty list clears the hold when a send fails. */
export function recordSentConversationAttachments(
  ownedId: string,
  attachments: ConversationAttachment[]
): void {
  const current = conversationSessions[ownedId];
  if (current) current.unclaimedSentAttachments = attachments;
}

export function setConversationCapabilities(ownedId: string, capabilities: AgentCapabilities): void {
  const current = conversationSessions[ownedId];
  if (!current || capabilities.provider !== current.provider) return;
  current.capabilities = capabilities;
  current.availableCommands = capabilities.commands;
  current.capabilityError = null;
}

export function setConversationCapabilityError(ownedId: string, message: string | null): void {
  const current = conversationSessions[ownedId];
  if (current) current.capabilityError = message;
}

export function setConversationAgentConfigState(
  ownedId: string,
  state: AgentConversationConfigState
): boolean {
  const current = conversationSessions[ownedId];
  if (!current) return false;
  current.agentConfig = state;
  current.agentConfigError = null;
  current.metadata = {
    ...current.metadata,
    model: state.model,
    effort: state.reasoningEffort,
    approvalPolicy: state.approvalPolicy
  };
  return true;
}

export function beginConversationAgentConfigChange(
  ownedId: string,
  field: AgentConversationConfigField,
  value: string
): AgentConversationConfigState | null {
  const current = conversationSessions[ownedId];
  if (!current) return null;
  const available = field === 'model'
    ? current.agentConfig.availableModels
    : field === 'reasoningEffort'
      ? current.agentConfig.availableEfforts
      : current.agentConfig.availableApprovalPolicies;
  if (!available.includes(value)) return null;
  const previous = current.agentConfig;
  current.agentConfig = { ...current.agentConfig, [field]: value };
  current.pendingAgentConfig[field] = value;
  current.agentConfigError = null;
  return previous;
}

export function confirmConversationAgentConfigChange(
  ownedId: string,
  field: AgentConversationConfigField,
  state: AgentConversationConfigState
): boolean {
  const current = conversationSessions[ownedId];
  if (!current || !(field in current.pendingAgentConfig)) return false;
  delete current.pendingAgentConfig[field];
  const optimistic = { ...current.pendingAgentConfig };
  current.agentConfig = { ...state, ...optimistic };
  current.agentConfigError = null;
  current.metadata = {
    ...current.metadata,
    model: current.agentConfig.model,
    effort: current.agentConfig.reasoningEffort,
    approvalPolicy: current.agentConfig.approvalPolicy
  };
  return true;
}

export function failConversationAgentConfigChange(
  ownedId: string,
  field: AgentConversationConfigField,
  previous: AgentConversationConfigState,
  message: string
): void {
  const current = conversationSessions[ownedId];
  if (!current || !(field in current.pendingAgentConfig)) return;
  delete current.pendingAgentConfig[field];
  current.agentConfig = { ...current.agentConfig, [field]: previous[field] };
  current.metadata = {
    ...current.metadata,
    model: current.agentConfig.model,
    effort: current.agentConfig.reasoningEffort,
    approvalPolicy: current.agentConfig.approvalPolicy
  };
  current.agentConfigError = message;
}

export function setConversationAgentConfigError(ownedId: string, message: string | null): void {
  const current = conversationSessions[ownedId];
  if (current) current.agentConfigError = message;
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
    current.pendingApprovals[request.requestId] = {
      ...request,
      toolTitle: request.title,
      options: permissionOptions(request.options),
      state: request.state
    };
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
  synchronizeSessionPresenceWork(ownedId, current.activeTurnId, sending);
}

export function setConversationConnection(connection: AgentConversationConnection): void {
  const current = ensureConversationSession(connection.ownedId, connection.provider);
  current.generation = connection.generation;
  current.writerLease.generation = connection.generation;
  current.connectionState = connection.state;
  current.suspended = false;
  if (connection.nativeSessionId) current.nativeSessionId = connection.nativeSessionId;
  if (connection.config) {
    current.agentConfig = connection.config;
    current.agentConfigError = null;
  }
}

export function captureConversationWorkspace(
  ownedId: string
): SessionConversationWorkspace | undefined {
  const current = conversationSessions[ownedId];
  if (!current) return undefined;
  return {
    version: SESSION_CONVERSATION_WORKSPACE_VERSION,
    mode: current.mode,
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
  clearSessionPresence(ownedId);
}
