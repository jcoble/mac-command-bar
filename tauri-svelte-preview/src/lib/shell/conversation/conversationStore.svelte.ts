import { displayEventFrom } from './conversationMessages.ts';
export { displayEventFrom } from './conversationMessages.ts';
/**
 * Reactive conversation state keyed by Command Bar's stable `ownedId`.
 *
 * This module performs no IO. The conversation service owns Tauri calls and
 * holds controls and view state beside the selected TanStack message graph.
 */
import { createConversationState, reduceConversationEvent, shouldClearConversationSending } from './conversationReducer.ts';
import type {
  AgentApprovalRequest,
  AgentCapabilities,
  AgentCommandDescriptor,
  AgentEvent,
  AgentConversationConnection,
  AgentConversationEvent,
  AgentConversationProvider,
  AgentConversationItemPage,
  AgentConversationSelectionSnapshot,
  AgentConversationTurnFacts,
  AgentConfigValue,
  AgentPermissionOption,
  AgentPermissionRequest,
  AgentUserInputRequest,
  AgentWriterLease,
  AgentWriterLeaseTransition,
  ConversationAttachment,
  ConversationChildAgent,
  ConversationMetadata,
  ConversationSessionState
} from './conversationTypes.ts';
import {
  agentItemFromEvent,
  availableCommandsFromEvent,
  displayItemFromAgentItem,
  displayItemFromApproval,
  permissionRequestFromEvent
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
  type ConversationViewState,
  type SessionConversationWorkspace
} from '../sessionWorkspaces.ts';
import {
  revokeTrackedObjectUrl,
  setConversationProjectionDiagnostics,
  setSentAttachmentDiagnostics
} from '../resourceDiagnostics.svelte.ts';

export type ConversationViewMode = 'structured' | 'raw';

export interface ConversationRecentEvent {
  sequence: number;
  kind: string;
  summary: string;
  timestampMs: number;
}

export const CONVERSATION_RECENT_EVENT_CAP = 200;
export const ACTIVE_EVENT_WINDOW_EVENTS = 20_000;
export const ACTIVE_EVENT_WINDOW_BYTES = 4 * 1024 * 1024; // 4 MiB stream race-buffer budget

export interface ConversationWorkspaceState extends ConversationSessionState {
  draft: string;
  /** Why this session's last send did not go out, held beside its own draft:
   * the surface is one component for every session, so a failure kept there
   * showed up under every conversation and outlived the one it belonged to. */
  sendError: string;
  /** Why this session could not take on an attachment or open a file link,
   * held beside its own draft for the same reason as `sendError`. */
  attachmentError: string;
  /** What this session's provider will not do, said once beside the box and
   * dismissed for good. Held per session for the same reason as `sendError`. */
  providerNotice: string;
  mode: ConversationViewMode;
  sending: boolean;
  attachments: ConversationAttachment[];
  metadata: ConversationMetadata;
  children: ConversationChildAgent[];
  selectedChildId: string | null;
  selectedChildHistoryOwnedId: string | null;
  childTranscriptTruncated: boolean;
  childTranscriptError: string | null;
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
  availableCommands: AgentCommandDescriptor[];
  pendingApprovals: Record<string, AgentPermissionRequest>;
  pendingInputs: Record<string, AgentUserInputRequest>;
  pendingConfig: Record<string, AgentConfigValue>;
  configErrors: Record<string, string>;
  recentEvents: ConversationRecentEvent[];
  selectedHistoryOwnedId: string;
  selectedTurns: AgentConversationTurnFacts[];
  selectedBeforeCursor?: number;
  selectedAfterCursor?: number;
  selectedHasBefore: boolean;
  selectedHasAfter: boolean;
  selectedLoadingOlder: boolean;
  selectedLoadingNewer: boolean;
  selectedPageError: string;
  selectedWatermark: number;
  selectedTransferBytes: number;
  selectedOversized: boolean;
  viewByHistoryId: Record<string, ConversationViewState>;
}

const emptyMetadata = (): ConversationMetadata => ({
  model: null,
  effort: null,
  approvalPolicy: null,
  usedTokens: null,
  contextWindow: null
});

export const conversationSessions = $state<Record<string, ConversationWorkspaceState>>({});

function publishConversationProjectionDiagnostics(): void {
  if (!import.meta.env.DEV) return;
  const projections = Object.values(conversationSessions);
  let sentAttachmentMapEntries = 0;
  let sentAttachmentCount = 0;
  for (const projection of projections) {
    const entries = Object.values(projection.sentAttachments);
    sentAttachmentMapEntries += entries.length;
    for (const attachments of entries) sentAttachmentCount += attachments.length;
  }
  setConversationProjectionDiagnostics(
    projections.length,
    0,
    0
  );
  setSentAttachmentDiagnostics(sentAttachmentMapEntries, sentAttachmentCount);
}

function revokeUnretainedPreviewUrls(
  candidates: readonly ConversationAttachment[],
  retained: readonly ConversationAttachment[]
): void {
  const retainedUrls = new Set(retained.map((attachment) => attachment.previewUrl));
  for (const attachment of candidates) {
    if (attachment.previewUrl.startsWith('blob:') && !retainedUrls.has(attachment.previewUrl)) {
      revokeTrackedObjectUrl(attachment.previewUrl);
    }
  }
}

function freshState(
  ownedId: string,
  provider: AgentConversationProvider
): ConversationWorkspaceState {
  return {
    ...createConversationState(ownedId, provider),
    draft: '',
    sendError: '',
    attachmentError: '',
    providerNotice: '',
    mode: 'structured',
    sending: false,
    attachments: [],
    metadata: emptyMetadata(),
    children: [],
    selectedChildId: null,
    selectedChildHistoryOwnedId: null,
    childTranscriptTruncated: false,
    childTranscriptError: null,
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
    availableCommands: [],
    pendingApprovals: {},
    pendingInputs: {},
    pendingConfig: {},
    configErrors: {},
    recentEvents: [],
    selectedHistoryOwnedId: ownedId,
    selectedTurns: [],
    selectedHasBefore: false,
    selectedHasAfter: false,
    selectedLoadingOlder: false,
    selectedLoadingNewer: false,
    selectedPageError: '',
    selectedWatermark: 0,
    selectedTransferBytes: 0,
    selectedOversized: false,
    viewByHistoryId: {}
  };
}

function applySelectedPageState(
  current: ConversationWorkspaceState,
  page: AgentConversationItemPage,
  direction: 'snapshot' | 'older' | 'newer',
  window?: {
    beforeCursor?: number;
    afterCursor?: number;
    hasBefore: boolean;
    hasAfter: boolean;
    retainedTurnIds: readonly string[];
    transferBytes: number;
  }
): void {
  const turns = new Map((direction === 'snapshot' ? [] : current.selectedTurns)
    .map((turn) => [turn.turnId, turn]));
  for (const turn of page.turns) turns.set(turn.turnId, turn);
  const retainedTurns = window ? new Set(window.retainedTurnIds) : null;
  current.selectedTurns = [...turns.values()].filter((turn) =>
    !retainedTurns || retainedTurns.has(turn.turnId)
  );
  if (direction !== 'newer' || window) current.selectedBeforeCursor = window?.beforeCursor ?? page.beforeCursor;
  if (direction !== 'older' || window) current.selectedAfterCursor = window?.afterCursor ?? page.afterCursor;
  current.selectedHasBefore = window?.hasBefore
    ?? (page.hasBefore || page.hasEarlierTranscript || !!(page.coverage && !page.coverage.startComplete));
  current.selectedHasAfter = window?.hasAfter ?? page.hasAfter;
  current.selectedWatermark = Math.max(current.selectedWatermark, page.watermark);
  current.selectedTransferBytes = window?.transferBytes ?? page.transferBytes;
  current.selectedOversized = window ? window.transferBytes > ACTIVE_EVENT_WINDOW_BYTES : page.oversized;
  if (direction === 'snapshot') {
    current.selectedLoadingOlder = false;
    current.selectedLoadingNewer = false;
  }
  current.timelineRevision += 1;
}

function applyConversationSnapshotControls(
  current: ConversationWorkspaceState,
  snapshot: AgentConversationSelectionSnapshot
): void {
  current.generation = snapshot.connection.generation;
  current.lastSequence = snapshot.pendingSequence;
  current.desynchronized = false;
  current.connectionState = snapshot.connection.state;
  current.suspended = snapshot.suspended;
  current.activeTurnId = snapshot.suspended ? undefined : snapshot.activeTurnId;
  current.pendingApprovals = {};
  current.pendingInputs = {};
  current.children = [];
  for (const event of snapshot.pendingEvents) applyTypedEventPayload(current, displayEventFrom(event));
}

export function applySelectedConversationSnapshotState(
  workspaceOwnedId: string,
  historyOwnedId: string,
  snapshot: AgentConversationSelectionSnapshot,
  applyControls = true,
  window?: Parameters<typeof applySelectedPageState>[3]
): void {
  const current = !applyControls && conversationSessions[workspaceOwnedId]
    ? conversationSessions[workspaceOwnedId]
    : ensureConversationSession(workspaceOwnedId, snapshot.connection.provider);
  current.selectedHistoryOwnedId = historyOwnedId;
  if (applyControls) applyConversationSnapshotControls(current, snapshot);
  else {
    current.generation = snapshot.connection.generation;
    current.activeTurnId = snapshot.suspended ? undefined : snapshot.activeTurnId;
  }
  current.selectedPageError = '';
  applySelectedPageState(current, snapshot.page, 'snapshot', window);
}

function applyConversationEventControls(
  workspaceOwnedId: string,
  event: AgentConversationEvent
): { current: ConversationWorkspaceState; displayEvent: AgentConversationEvent | AgentEvent } | null {
  const current = conversationSessions[workspaceOwnedId];
  if (!current || event.generation < current.generation) return null;
  const next = reduceConversationEvent(current, event);
  if (next === current) return null;
  appendRecentEvent(current, event);
  Object.assign(current, next);
  const displayEvent = displayEventFrom(event);
  if (event.payload.kind === 'userMessage' && current.unclaimedSentAttachments.length) {
    current.sentAttachments[event.payload.itemId] = current.unclaimedSentAttachments;
    current.unclaimedSentAttachments = [];
  }
  applyTypedEventPayload(current, displayEvent);
  return { current, displayEvent };
}

function applySelectedConversationTurnState(
  current: ConversationWorkspaceState,
  event: AgentConversationEvent,
  displayEvent: AgentConversationEvent | AgentEvent
): void {
  const item = agentItemFromEvent(displayEvent);
  const row = item ? displayItemFromAgentItem(item, displayEvent.timestampMs) : null;
  const payload = displayEvent.payload as Record<string, unknown>;
  const turnId = ('turnId' in displayEvent ? displayEvent.turnId : undefined)
    ?? asString(payload.turnId)
    ?? item?.turnId;
  if (turnId) {
    const prior = current.selectedTurns.find((turn) => turn.turnId === turnId);
    const eventType = 'type' in displayEvent ? displayEvent.type : '';
    const turnState = event.payload.kind === 'turn' ? event.payload.state : null;
    const next: AgentConversationTurnFacts = {
      turnId,
      ...prior,
      ...((turnState === 'started' || eventType === 'turn.started') && !prior?.startedAtMs
        ? { startedAtMs: displayEvent.timestampMs } : {}),
      ...((turnState && turnState !== 'started') || ['turn.completed', 'turn.interrupted'].includes(eventType)
        ? {
            endedAtMs: displayEvent.timestampMs,
            terminalState: turnState ?? eventType.slice('turn.'.length)
          } : {}),
      ...(row?.kind === 'assistant' && row.completed
        ? { finalAssistantItemId: row.itemId } : {})
    };
    current.selectedTurns = [
      ...current.selectedTurns.filter((turn) => turn.turnId !== turnId),
      next
    ];
  }
}

export function applySelectedConversationEventState(
  workspaceOwnedId: string,
  event: AgentConversationEvent
): void {
  const result = applyConversationEventControls(workspaceOwnedId, event);
  if (result) applySelectedConversationTurnState(result.current, event, result.displayEvent);
}

export function applySelectedConversationHistoryEventState(
  workspaceOwnedId: string,
  event: AgentConversationEvent
): void {
  const current = conversationSessions[workspaceOwnedId];
  if (!current) return;
  const displayEvent = displayEventFrom(event);
  const payload = displayEvent.payload as Record<string, unknown>;
  const eventType = 'type' in displayEvent ? displayEvent.type : '';
  const turnId = asString(payload.turnId) ?? ('turnId' in displayEvent ? displayEvent.turnId : undefined);
  if ((payload.kind === 'turn' && payload.state === 'started') || eventType === 'turn.started') {
    current.activeTurnId = turnId;
  } else if (shouldClearConversationSending(displayEvent) && (!turnId || turnId === current.activeTurnId)) {
    current.activeTurnId = undefined;
  }
  applySelectedConversationTurnState(current, event, displayEvent);
}

export function applySelectedConversationLiveWindow(
  ownedId: string,
  retainedTurnIds: readonly string[],
  transferBytes: number
): void {
  const current = conversationSessions[ownedId];
  if (!current) return;
  const retained = new Set(retainedTurnIds);
  if (current.activeTurnId) retained.add(current.activeTurnId);
  current.selectedTurns = current.selectedTurns.filter((turn) => retained.has(turn.turnId));
  current.selectedTransferBytes = transferBytes;
  current.selectedOversized = transferBytes > ACTIVE_EVENT_WINDOW_BYTES;
}

export function applySelectedConversationPageState(
  ownedId: string,
  page: AgentConversationItemPage,
  direction: 'older' | 'newer',
  window: {
    beforeCursor?: number;
    afterCursor?: number;
    hasBefore: boolean;
    hasAfter: boolean;
    retainedTurnIds: readonly string[];
    transferBytes: number;
  }
): void {
  const current = conversationSessions[ownedId];
  if (current) applySelectedPageState(current, page, direction, window);
}

export function setSelectedConversationPageError(ownedId: string, message: string): void {
  const current = conversationSessions[ownedId];
  if (current) current.selectedPageError = message;
}

export function setSelectedConversationPageLoading(
  ownedId: string,
  direction: 'older' | 'newer',
  loading: boolean
): boolean {
  const current = conversationSessions[ownedId];
  if (!current) return false;
  if (loading && (current.selectedLoadingOlder || current.selectedLoadingNewer)) return false;
  if (direction === 'older') {
    if (loading && !current.selectedHasBefore) return false;
    current.selectedLoadingOlder = loading;
  } else {
    if (loading && !current.selectedHasAfter) return false;
    current.selectedLoadingNewer = loading;
  }
  if (loading) current.selectedPageError = '';
  return true;
}

export function selectedConversationViewState(
  ownedId: string,
  historyOwnedId: string
): ConversationViewState {
  return conversationSessions[ownedId]?.viewByHistoryId[historyOwnedId]
    ?? { followLatest: true, expandedTurns: {} };
}

export function setSelectedConversationViewState(
  ownedId: string,
  historyOwnedId: string,
  view: ConversationViewState
): void {
  const current = conversationSessions[ownedId];
  if (current) current.viewByHistoryId[historyOwnedId] = view;
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
  publishConversationProjectionDiagnostics();
  return created;
}

/** Update rail attention for an inactive session without retaining its transcript. */
export function recordAgentConversationPresenceEvent(event: AgentConversationEvent): void {
  recordConversationPresenceEvent(displayEventFrom(event));
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

function asString(value: unknown): string | null {
  return typeof value === 'string' && value.length > 0 ? value : null;
}

function normalizeChildProvider(
  rawProvider: unknown,
  fallback: AgentConversationProvider
): AgentConversationProvider {
  const provider = asString(rawProvider)?.toLowerCase().replaceAll(/[\s_-]+/g, '');
  if (provider === 'codex') return 'codex';
  if (provider === 'claude' || provider === 'claudecode') return 'claude';
  if (provider === 'agy' || provider === 'antigravity') return 'antigravity';
  return fallback;
}

function childTranscriptAvailable(provider: AgentConversationProvider): boolean {
  return provider !== 'antigravity';
}

function isWorkflowChildRecord(value: Record<string, unknown>): boolean {
  return ['kind', 'type', 'category'].some((key) => asString(value[key])?.toLowerCase() === 'workflow');
}

function childParentWouldCycle(
  children: readonly ConversationChildAgent[],
  childId: string,
  parentId: string
): boolean {
  const seen = new Set<string>([childId]);
  let nextParentId: string | undefined = parentId;
  while (nextParentId) {
    if (seen.has(nextParentId)) return true;
    seen.add(nextParentId);
    nextParentId = children.find((child) => child.childId === nextParentId)?.parentId;
  }
  return false;
}

function normalizedConversationChild(
  value: unknown,
  existing: ConversationChildAgent | null,
  provider: AgentConversationProvider,
  parentOwnedId: string,
  parentGeneration: number,
  timestampMs: number,
  siblings: readonly ConversationChildAgent[],
  rootNativeSessionId?: string
): ConversationChildAgent | null {
  if (!isRecord(value) || isWorkflowChildRecord(value)) return null;
  const childId = asString(value.childId) ?? asString(value.childSessionId) ?? asString(value.id);
  if (!childId) return null;
  const parentToolCallId = asString(value.parentToolCallId) ?? existing?.parentToolCallId;
  const rawParentId = asString(value.parentId) ?? parentToolCallId ?? existing?.parentId;
  const parentId = rawParentId === rootNativeSessionId ? parentOwnedId : rawParentId;
  if (!parentId || childParentWouldCycle(siblings, childId, parentId)) return null;
  const childProvider = normalizeChildProvider(value.provider, existing?.provider ?? provider);
  return {
    childId,
    parentOwnedId,
    parentGeneration,
    parentId,
    ...(parentToolCallId ? { parentToolCallId } : {}),
    ...(asString(value.transcriptId) || existing?.transcriptId
      ? { transcriptId: asString(value.transcriptId) ?? existing?.transcriptId }
      : {}),
    provider: childProvider,
    title: asString(value.title) ?? asString(value.label) ?? existing?.title ?? 'Sub-agent',
    state: asString(value.state) ?? existing?.state ?? 'finished',
    ...(asString(value.latestActivity) || existing?.latestActivity
      ? { latestActivity: asString(value.latestActivity) ?? existing?.latestActivity }
      : {}),
    updatedAtMs: typeof value.updatedAtMs === 'number' && Number.isFinite(value.updatedAtMs)
      ? value.updatedAtMs
      : timestampMs,
    transcriptAvailable: childTranscriptAvailable(childProvider)
  };
}

function normalizedConversationChildren(
  values: readonly unknown[],
  existingChildren: readonly ConversationChildAgent[],
  provider: AgentConversationProvider,
  parentOwnedId: string,
  parentGeneration: number,
  timestampMs: number,
  rootNativeSessionId?: string
): ConversationChildAgent[] {
  const children: ConversationChildAgent[] = [];
  for (const value of values) {
    if (!isRecord(value)) continue;
    const childId = asString(value.childId) ?? asString(value.childSessionId) ?? asString(value.id);
    const existing = childId
      ? children.find((child) => child.childId === childId)
        ?? existingChildren.find((child) => child.childId === childId)
        ?? null
      : null;
    const child = normalizedConversationChild(
      value,
      existing,
      provider,
      parentOwnedId,
      parentGeneration,
      timestampMs,
      children,
      rootNativeSessionId
    );
    if (!child) continue;
    const index = children.findIndex((entry) => entry.childId === child.childId);
    if (index >= 0) children[index] = child;
    else children.push(child);
  }
  return children;
}

function mergeConversationChild(
  current: ConversationWorkspaceState,
  value: unknown,
  provider: AgentConversationProvider,
  parentGeneration: number,
  timestampMs: number
): void {
  if (!isRecord(value)) return;
  const childId = asString(value.childId) ?? asString(value.childSessionId) ?? asString(value.id);
  if (!childId) return;
  const index = current.children.findIndex((child) => child.childId === childId);
  const existing = index >= 0 ? current.children[index] : null;
  const child = normalizedConversationChild(
    value,
    existing,
    provider,
    current.ownedId,
    parentGeneration,
    timestampMs,
    current.children,
    current.nativeSessionId
  );
  if (!child) return;
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

function applyTypedEventPayload(current: ConversationWorkspaceState, event: AgentConversationEvent | AgentEvent): void {
  const payload = event.payload as Record<string, unknown>;
  const raw = isRecord(payload);
  if (!raw) return;
  const eventType: string = 'type' in event
    ? event.type
    : payload.kind === 'approval' ? 'approval.requested'
      : payload.kind === 'userInputRequested' ? 'user-input.requested'
      : payload.kind === 'userInputResolved' ? 'user-input.resolved'
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
    const totalTokens = typeof payload.totalTokens === 'number' && Number.isFinite(payload.totalTokens)
      ? payload.totalTokens : undefined;
    current.usage = {
      inputTokens: inputTokens ?? current.usage?.inputTokens,
      outputTokens: outputTokens ?? current.usage?.outputTokens,
      usedTokens: usedTokens ?? current.usage?.usedTokens,
      contextWindow: contextWindow ?? current.usage?.contextWindow,
      totalTokens: totalTokens ?? current.usage?.totalTokens
    };
    if (usedTokens !== undefined || contextWindow !== undefined) {
      current.metadata = {
        ...current.metadata,
        usedTokens: usedTokens ?? current.metadata.usedTokens,
        contextWindow: contextWindow ?? current.metadata.contextWindow
      };
    }
  }
  if (payload.kind === 'childUpdate') {
    mergeConversationChild(current, payload, event.provider, event.generation, event.timestampMs);
  }
  if (eventType === 'children.updated' && Array.isArray(payload.children)) {
    for (const child of payload.children) {
      mergeConversationChild(current, child, event.provider, event.generation, event.timestampMs);
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
        fields: payload.fields as AgentUserInputRequest['fields'],
        canDecline: payload.canDecline === true
      };
    }
  }
  if (eventType === 'user-input.resolved') {
    const requestId = asString(payload.requestId) ?? requestIdFromEvent;
    if (requestId) delete current.pendingInputs[requestId];
  }
}

export function applyChildConversationHistoryStatus(
  ownedId: string,
  childId: string,
  truncated: boolean
): void {
  const current = conversationSessions[ownedId];
  if (!current || current.selectedChildId !== childId) return;
  current.childTranscriptTruncated = truncated;
  current.childTranscriptError = null;
}

export function failChildConversationTranscript(
  ownedId: string,
  childId: string,
  message: string
): void {
  const current = conversationSessions[ownedId];
  if (!current || current.selectedChildId !== childId) return;
  current.childTranscriptError = message;
}

export function setConversationAttachments(ownedId: string, attachments: ConversationAttachment[]): void {
  const current = conversationSessions[ownedId];
  if (current) {
    const previous = current.attachments;
    const previousVisible = new Set(current.attachments.map((attachment) => attachment.id));
    const nextVisible = new Set(attachments.map((attachment) => attachment.id));
    current.attachmentIds = [
      ...current.attachmentIds.filter((id) => !previousVisible.has(id) && !nextVisible.has(id)),
      ...nextVisible
    ];
    current.attachments = attachments;
    revokeUnretainedPreviewUrls(previous, [
      ...attachments,
      ...current.unclaimedSentAttachments,
      ...Object.values(current.sentAttachments).flat()
    ]);
  }
}

export function setConversationAttachmentIds(ownedId: string, ids: readonly string[]): void {
  const current = conversationSessions[ownedId];
  if (current) current.attachmentIds = [...ids];
}

export function restoreConversationAttachmentIds(ownedId: string, ids: readonly string[]): void {
  const current = conversationSessions[ownedId];
  if (current && current.attachments.length === 0) current.attachmentIds = [...ids];
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
/** Keep saved attachments only for user items still in the selected TanStack graph. */
export function restoreSelectedConversationAttachments(
  ownedId: string,
  byItemId: Record<string, ConversationAttachment[]>,
  generation: number,
  retainedItemIds: readonly string[]
): void {
  const current = conversationSessions[ownedId];
  const retained = new Set(retainedItemIds);
  const incoming = Object.values(byItemId).flat();
  if (!current || current.generation !== generation) {
    revokeUnretainedPreviewUrls(incoming, current ? Object.values(current.sentAttachments).flat() : []);
    return;
  }
  const next: Record<string, ConversationAttachment[]> = {};
  for (const [itemId, attachments] of Object.entries(current.sentAttachments)) {
    if (retained.has(itemId)) next[itemId] = attachments;
  }
  for (const [itemId, attachments] of Object.entries(byItemId)) {
    if (retained.has(itemId)) next[itemId] = attachments;
  }
  revokeUnretainedPreviewUrls(
    [...Object.values(current.sentAttachments).flat(), ...incoming],
    [...Object.values(next).flat(), ...current.attachments]
  );
  current.sentAttachments = next;
  publishConversationProjectionDiagnostics();
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
  if (!current) return;
  if (current.selectedChildId === childId) return;
  current.selectedChildId = childId;
  current.childTranscriptTruncated = false;
  current.childTranscriptError = null;
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

/** Record or clear what this session's provider will not do. */
export function setConversationProviderNotice(ownedId: string, message: string): void {
  const current = conversationSessions[ownedId];
  if (!current || current.providerNotice === message) return;
  current.providerNotice = message;
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
  current.suspended = connection.suspended ?? false;
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
    viewByHistoryId: current.viewByHistoryId,
    sequence: current.lastSequence,
    telemetry: current.telemetry,
    writerLease: current.writerLease,
    writerLeaseTransition: current.writerLeaseTransition,
    selectedChildId: null,
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
  current.selectedChildId = null;
  current.selectedChildHistoryOwnedId = null;
  current.childTranscriptTruncated = false;
  current.childTranscriptError = null;
  current.executionOwner = snapshot?.owner ?? current.executionOwner;
  current.attachmentIds = snapshot?.attachmentIds ?? current.attachmentIds;
  current.config = snapshot?.config ?? current.config;
  current.viewByHistoryId = snapshot?.viewByHistoryId ?? current.viewByHistoryId;
  current.telemetry = snapshot?.telemetry ?? current.telemetry;
  if (snapshot?.writerLease?.ownedId === ownedId) current.writerLease = snapshot.writerLease;
  if (snapshot?.writerLeaseTransition?.ownedId === ownedId) {
    current.writerLeaseTransition = snapshot.writerLeaseTransition;
  }
  return current;
}

/** Release one materialized transcript while preserving its lightweight rail presence. */
export function evictConversationSession(ownedId: string): void {
  const current = conversationSessions[ownedId];
  if (!current) return;
  const previewUrls = new Set([
    ...current.attachments,
    ...current.unclaimedSentAttachments,
    ...Object.values(current.sentAttachments).flat()
  ].map((attachment) => attachment.previewUrl));
  for (const previewUrl of previewUrls) {
    if (previewUrl.startsWith('blob:')) revokeTrackedObjectUrl(previewUrl);
  }
  delete conversationSessions[ownedId];
  publishConversationProjectionDiagnostics();
}

/** Retain the active parent and its one displayed child history. */
export function evictInactiveConversationSessions(activeOwnedId: string | null): void {
  const childHistoryOwnedId = activeOwnedId ? conversationSessions[activeOwnedId]?.selectedChildHistoryOwnedId : null;
  for (const ownedId of Object.keys(conversationSessions)) {
    if (ownedId !== activeOwnedId && ownedId !== childHistoryOwnedId) {
      const current = conversationSessions[ownedId];
      if (current && !current.sending) {
        evictConversationSession(ownedId);
      }
    }
  }
}

export function removeConversationSession(ownedId: string): void {
  evictConversationSession(ownedId);
  clearSessionPresence(ownedId);
}
