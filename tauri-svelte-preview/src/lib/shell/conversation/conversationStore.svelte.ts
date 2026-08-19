/**
 * Reactive conversation state keyed by Command Bar's stable `ownedId`.
 *
 * This module performs no IO. The conversation service owns Tauri calls and
 * feeds normalized events into `applyAgentConversationEvent`.
 */
import { applyConversationEvent, createConversationState, usageDropIsCompaction } from './conversationReducer.ts';
import type {
  AgentApprovalRequest,
  AgentCapabilities,
  AgentCommandDescriptor,
  AgentEvent,
  AgentConversationConnection,
  AgentConversationEvent,
  AgentConversationEventPage,
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
  hasAgentConversationConfig,
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
  /** Why this session's last send did not go out, held beside its own draft:
   * the surface is one component for every session, so a failure kept there
   * showed up under every conversation and outlived the one it belonged to. */
  sendError: string;
  /** Why this session could not take on an attachment or open a file link,
   * held beside its own draft for the same reason as `sendError`. */
  attachmentError: string;
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
  capabilitiesGeneration: number;
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
  /** The lowest stored sequence currently on screen, and where scrolling up
   * asks from. Zero until a session has been opened. */
  oldestLoadedSequence: number;
  /** A backward page is in flight; the transcript must not ask for another. */
  loadingOlder: boolean;
  /** Nothing older than what is on screen exists, so stop asking. */
  reachedTranscriptStart: boolean;
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
    sendError: '',
    attachmentError: '',
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
    capabilitiesGeneration: 0,
    capabilityError: null,
    agentItems: [],
    planSteps: [],
    tasks: [],
    availableCommands: [],
    pendingApprovals: {},
    pendingInputs: {},
    pendingConfig: {},
    configErrors: {},
    recentEvents: [],
    oldestLoadedSequence: 0,
    loadingOlder: false,
    reachedTranscriptStart: false
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

/**
 * A session picked up from a past transcript stores its events wrapped in a
 * terminal projection, with the real event one level further down. Typing an
 * item reads the inner event, so the wrapper has to come off first — a wrapper
 * read as an event has none of the fields that give an item its type or text,
 * and becomes an empty `unknown` row.
 *
 * Both the live path and snapshot restore go through here. They did not always:
 * restore typed the wrapper, so a resumed transcript painted one empty row per
 * stored event. An event that is not a projection is returned untouched, which
 * is every event a session of this app's own produces.
 */
function displayEventFrom(event: AgentConversationEvent): AgentConversationEvent | AgentEvent {
  if (event.payload.kind !== 'terminalProjection') return event;
  return {
    type: event.payload.eventType,
    ownedId: event.ownedId,
    provider: event.provider,
    providerInstanceId: event.payload.providerInstanceId,
    generation: event.generation,
    sequence: event.sequence,
    timestampMs: event.payload.timestampMs ?? event.timestampMs,
    nativeSessionId: event.payload.nativeSessionId,
    itemId: event.payload.itemId ?? undefined,
    payload: event.payload.payload,
    providerMetadata: event.payload.providerMetadata,
    rawFrameReference: event.payload.rawFrameReference
  };
}

export function applyAgentConversationEvent(event: AgentConversationEvent): boolean {
  const existing = conversationSessions[event.ownedId];
  if (existing && event.provider !== existing.provider) return false;
  const current = ensureConversationSession(event.ownedId, event.provider);
  appendRecentEvent(current, event);
  const applied = applyLegacyEventInPlace(current, event);
  if (!applied) return false;
  const displayEvent = displayEventFrom(event);
  const typedItem = agentItemFromEvent(displayEvent);
  if (typedItem) {
    if (mergeAgentItemInPlace(current, typedItem, conversationEventAppendsItemContent(displayEvent))
      && !['userMessage', 'assistantDelta', 'assistantMessage', 'tool'].includes(event.payload.kind)) {
      current.timelineRevision += 1;
    }
  }
  applyTypedEventPayload(current, displayEvent);
  if (!current.desynchronized) recordConversationPresenceEvent(displayEvent);
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
        displayChanged = existing.name !== payload.name
          || existing.state !== payload.state
          || existing.summary !== payload.summary
          || existing.output !== payload.output
          || existing.diff !== payload.diff;
        // A call and its result are the same row arriving twice: the call names
        // the tool and says what was asked, the result says what came back and
        // names nothing. Each only writes what it actually carries, or the
        // result would blank out the row it belongs to.
        if (payload.name) existing.name = payload.name;
        existing.state = payload.state;
        if (payload.summary !== undefined) existing.summary = payload.summary;
        if (payload.output !== undefined) existing.output = payload.output;
        if (payload.path !== undefined) existing.path = payload.path;
        if (payload.diff !== undefined) existing.diff = payload.diff;
      } else {
        appendTimelineEntry(current, {
          kind: 'tool', itemId: payload.itemId,
          // A result names no tool, and its call is what would have. When the
          // two are separated — a page boundary can fall between them — the row
          // is still worth drawing, so it takes a plain name until the call
          // turns up and gives it the real one.
          name: payload.name || 'Tool',
          state: payload.state, summary: payload.summary, output: payload.output,
          path: payload.path, diff: payload.diff, timestampMs: event.timestampMs
        });
        displayChanged = true;
      }
      break;
    }
    case 'childUpdate':
      break;
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
    case 'contextCompaction': {
      appendTimelineEntry(current, {
        kind: 'compaction', itemId: `compaction:${event.sequence}`,
        trigger: payload.trigger ?? undefined,
        preTokens: payload.preTokens ?? undefined,
        postTokens: payload.postTokens ?? undefined,
        timestampMs: event.timestampMs
      });
      displayChanged = true;
      break;
    }
    case 'usage': {
      // Claude says nothing when it compacts; the only sign is the reported
      // occupancy falling off a cliff. A full window dropping to a fraction of
      // itself has no other cause, so the transcript says so where it happened
      // rather than leaving the reader to guess. A small decline is ordinary —
      // a report of the last request rather than the session — and is ignored.
      const previous = current.usage?.usedTokens;
      const alreadySaid = current.timeline[current.timeline.length - 1]?.kind === 'compaction';
      if (!alreadySaid && usageDropIsCompaction(previous, payload.usedTokens)) {
        appendTimelineEntry(current, {
          kind: 'compaction', itemId: `compaction:${event.sequence}`,
          preTokens: previous, postTokens: payload.usedTokens, timestampMs: event.timestampMs
        });
        displayChanged = true;
      }
      current.usage = {
        inputTokens: payload.inputTokens ?? current.usage?.inputTokens,
        outputTokens: payload.outputTokens ?? current.usage?.outputTokens,
        usedTokens: payload.usedTokens ?? current.usage?.usedTokens,
        contextWindow: payload.contextWindow ?? current.usage?.contextWindow
      };
      break;
    }
    case 'terminalProjection':
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
  // Re-selecting a session hands us a snapshot we have usually already applied.
  // When it is the same generation, holds no event newer than what is on
  // screen, and changes no connection fact, rebuilding would redo the whole
  // replay and re-render for nothing — on long sessions that work is
  // user-visible. Skip it outright. A desynchronized session never skips:
  // its snapshot is the repair.
  const newestSnapshotSequence = snapshot.events.length
    ? snapshot.events[snapshot.events.length - 1].sequence
    : 0;
  if (
    snapshot.connection.generation === current.generation
    && current.lastSequence > 0
    && newestSnapshotSequence <= current.lastSequence
    && !current.desynchronized
    && current.suspended === snapshot.suspended
    && current.connectionState === snapshot.connection.state
  ) return;
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
  // Not clamped at zero: importing older history writes it at descending
  // sequences that run through zero into negatives, and a clamp here would make
  // the reducer read every one of those as already seen and drop it.
  rebuilt.lastSequence = firstEvent ? firstEvent.sequence - 1 : 0;
  rebuilt.connectionState = snapshot.connection.state;
  rebuilt.nativeSessionId = snapshot.connection.nativeSessionId;
  for (const event of events) {
    rebuilt = applyConversationEvent(rebuilt, event);
  }
  if (
    snapshot.connection.generation === current.generation
    && rebuilt.lastSequence < current.lastSequence
  ) return;
  // Build the complete snapshot off the reactive graph. Publishing this object
  // before replay made every event traverse Svelte's deep proxy machinery and
  // invalidated subscribers 2,000 times during a read-only load.
  // Replay is seeded from the first retained event so no history is skipped,
  // which leaves the rebuilt generation at whatever the window ended on. A
  // session re-ensured after a suspend has a newer adapter incarnation and no
  // events in it yet, so the connection's generation is the current one: keep
  // it, or the next send is addressed to an incarnation the backend has
  // already replaced and is refused.
  const generation = Math.max(rebuilt.generation, snapshot.connection.generation);
  const restored: ConversationWorkspaceState = {
    ...rebuilt,
    generation,
    suspended: snapshot.suspended === true,
    timelineRevision: current.timelineRevision + 1,
    draft: current.draft,
    sendError: current.sendError,
    attachmentError: current.attachmentError,
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
    writerLease: { ...current.writerLease, generation },
    writerLeaseTransition: current.writerLeaseTransition,
    attachmentIds: current.attachmentIds,
    sentAttachments: current.sentAttachments,
    unclaimedSentAttachments: current.unclaimedSentAttachments,
    config: current.config,
    telemetry: current.telemetry,
    capabilities: current.capabilitiesGeneration === generation ? current.capabilities : null,
    capabilitiesGeneration: current.capabilitiesGeneration === generation
      ? current.capabilitiesGeneration
      : 0,
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
    recentEvents: [],
    // A snapshot is the newest window of a longer journal. Scrolling up asks
    // for what came before its first event.
    oldestLoadedSequence: firstEvent?.sequence ?? 0,
    loadingOlder: false,
    reachedTranscriptStart: false
  };
  for (const event of events) {
    const displayEvent = displayEventFrom(event);
    const typedItem = agentItemFromEvent(displayEvent);
    if (typedItem) {
      mergeAgentItemInPlace(restored, typedItem, conversationEventAppendsItemContent(displayEvent));
    }
    applyTypedEventPayload(restored, displayEvent);
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

/**
 * Puts the page of stored events just older than the transcript in front of it.
 *
 * Opening a conversation ships one screen of history, so reaching further back
 * means reading a page and prepending it. The page is replayed into a throwaway
 * session first: it is built by the same reducer the live path uses, but its
 * metadata, plan, tasks and usage are stale by definition and must never
 * overwrite what is on screen. Only the rows are taken.
 *
 * An item whose start is in the older page and whose completion is already on
 * screen appears in both, so anything already drawn is dropped rather than
 * duplicated.
 */
export function prependOlderConversationEvents(
  ownedId: string,
  page: AgentConversationEventPage
): void {
  const current = conversationSessions[ownedId];
  if (!current) return;
  const events = idempotentSnapshotEvents(page.events);
  const firstEvent = events[0];
  let rebuilt = createConversationState(current.ownedId, current.provider);
  rebuilt.generation = firstEvent?.generation ?? current.generation;
  // Not clamped at zero: importing older history writes it at descending
  // sequences that run through zero into negatives, and a clamp here would make
  // the reducer read every one of those as already seen and drop it.
  rebuilt.lastSequence = firstEvent ? firstEvent.sequence - 1 : 0;
  for (const event of events) {
    rebuilt = applyConversationEvent(rebuilt, event);
  }
  const older: ConversationWorkspaceState = {
    ...freshState(current.ownedId, current.provider),
    ...rebuilt
  };
  for (const event of events) {
    const displayEvent = displayEventFrom(event);
    const typedItem = agentItemFromEvent(displayEvent);
    if (typedItem) {
      mergeAgentItemInPlace(older, typedItem, conversationEventAppendsItemContent(displayEvent));
    }
  }

  const drawnEntries = new Set(current.timeline.map((entry) => entry.itemId));
  const entries = older.timeline.filter((entry) => !drawnEntries.has(entry.itemId));
  const drawnItems = new Set(current.agentItems.map((item) => item.id));
  const items = older.agentItems.filter((item) => !drawnItems.has(item.id));
  if (entries.length) current.timeline.unshift(...entries);
  if (items.length) current.agentItems.unshift(...items);
  // Both caches map an item id to its position, and every position just moved.
  timelineIndexBySession.delete(current);
  agentItemIndexBySession.delete(current);

  if (firstEvent) current.oldestLoadedSequence = firstEvent.sequence;
  current.reachedTranscriptStart = !page.hasMore;
  current.loadingOlder = false;
  current.timelineRevision += 1;
}

/** Marks a backward page as in flight so only one is ever asked for. */
export function beginLoadingOlderConversationEvents(ownedId: string): boolean {
  const current = conversationSessions[ownedId];
  if (!current) return false;
  if (current.loadingOlder || current.reachedTranscriptStart) return false;
  // Nothing is on screen yet, so there is no cursor to read backwards from.
  // Sequence numbers themselves say nothing here: extending an import writes
  // older events at descending sequences, which run through 1 and past it.
  if (current.timeline.length === 0) return false;
  current.loadingOlder = true;
  return true;
}

/** Releases the in-flight guard when a backward page could not be read. */
export function failLoadingOlderConversationEvents(ownedId: string): void {
  const current = conversationSessions[ownedId];
  if (current) current.loadingOlder = false;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

function asString(value: unknown): string | null {
  return typeof value === 'string' && value.length > 0 ? value : null;
}

function mergeConversationChild(
  current: ConversationWorkspaceState,
  value: unknown,
  provider: AgentConversationProvider,
  timestampMs: number
): void {
  if (!isRecord(value)) return;
  const childId = asString(value.childId);
  if (!childId) return;
  const index = current.children.findIndex((child) => child.childId === childId);
  const existing = index >= 0 ? current.children[index] : null;
  const parentToolCallId = asString(value.parentToolCallId) ?? existing?.parentToolCallId;
  const parentId = asString(value.parentId) ?? parentToolCallId ?? existing?.parentId;
  if (!parentId) return;
  const rawProvider = asString(value.provider);
  const childProvider = rawProvider === 'codex' || rawProvider === 'claude'
    ? rawProvider
    : existing?.provider ?? provider;
  const child: ConversationChildAgent = {
    childId,
    parentId,
    ...(parentToolCallId ? { parentToolCallId } : {}),
    provider: childProvider,
    label: asString(value.label) ?? existing?.label ?? 'Sub-agent',
    state: asString(value.state) ?? existing?.state ?? 'finished',
    ...(asString(value.latestActivity) || existing?.latestActivity
      ? { latestActivity: asString(value.latestActivity) ?? existing?.latestActivity }
      : {}),
    updatedAtMs: typeof value.updatedAtMs === 'number' && Number.isFinite(value.updatedAtMs)
      ? value.updatedAtMs
      : timestampMs
  };
  if (index >= 0) current.children[index] = child;
  else current.children.push(child);
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
    current.capabilitiesGeneration = current.generation;
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
  if (payload.kind === 'childUpdate') {
    mergeConversationChild(current, payload, event.provider, event.timestampMs);
  }
  if (eventType === 'children.updated' && Array.isArray(payload.children)) {
    for (const child of payload.children) {
      mergeConversationChild(current, child, event.provider, event.timestampMs);
    }
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
          { optionId: 'accept', name: 'Allow', kind: 'allow_once', synthetic: true },
          { optionId: 'decline', name: 'Deny', kind: 'reject_once', synthetic: true }
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

/** Restore the screenshots of messages replayed from the journal. The live
 * hold above only covers sends this window made, so a restarted app has to
 * hang the saved files back on the user messages that named them. A message
 * that already carries its screenshots keeps them. */
export function restoreSentConversationAttachments(
  ownedId: string,
  byItemId: Record<string, ConversationAttachment[]>
): void {
  const current = conversationSessions[ownedId];
  if (!current) return;
  for (const [itemId, attachments] of Object.entries(byItemId)) {
    if (!current.sentAttachments[itemId]?.length) current.sentAttachments[itemId] = attachments;
  }
}

export function setConversationCapabilities(
  ownedId: string,
  generation: number,
  capabilities: AgentCapabilities
): void {
  const current = conversationSessions[ownedId];
  if (
    !current
    || generation !== current.generation
    || capabilities.provider !== current.provider
  ) return;
  current.capabilities = capabilities;
  current.capabilitiesGeneration = generation;
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
  if (capabilities && capabilities.provider === current.provider) {
    current.capabilities = capabilities;
    current.capabilitiesGeneration = current.generation;
  }
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

/** Record or clear why this session's last send did not go out. */
export function setConversationSendError(ownedId: string, message: string): void {
  const current = conversationSessions[ownedId];
  if (!current || current.sendError === message) return;
  current.sendError = message;
}

/** Record or clear this session's attachment or file-link failure. */
export function setConversationAttachmentError(ownedId: string, message: string): void {
  const current = conversationSessions[ownedId];
  if (!current || current.attachmentError === message) return;
  current.attachmentError = message;
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
  const existing = conversationSessions[connection.ownedId];
  if (existing && connection.generation < existing.generation) return;
  const current = ensureConversationSession(connection.ownedId, connection.provider);
  if (connection.generation > current.generation) {
    current.capabilities = null;
    current.capabilitiesGeneration = 0;
  }
  current.generation = connection.generation;
  current.writerLease.generation = connection.generation;
  current.connectionState = connection.state;
  current.suspended = false;
  if (connection.nativeSessionId) current.nativeSessionId = connection.nativeSessionId;
  // A connection always carries a config object, even when nobody has asked an
  // adapter anything — every field null, every list empty. Applying that on its
  // mere existence wiped the answers the composer had just read back from the
  // database, which is what left a resumed conversation saying its agent had no
  // settings. Only a config with something in it replaces what is on screen.
  if (connection.config && hasAgentConversationConfig(connection.config)) {
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
