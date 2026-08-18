export type AgentConversationProvider = 'codex' | 'claude' | 'antigravity';

export interface ConversationCommandError {
  code: string;
  message: string;
  recoverable: boolean;
}

export type AgentConfigValue = string | number | boolean | null | AgentConfigValue[] | {
  [key: string]: AgentConfigValue;
};

export interface AgentConfigOptionChoice {
  value: AgentConfigValue;
  label: string;
  description?: string;
}

export interface AgentConfigOption {
  id: string;
  label: string;
  category: string;
  description?: string;
  value: AgentConfigValue;
  choices?: AgentConfigOptionChoice[];
  providerMetadata?: Record<string, AgentConfigValue>;
}

export interface AgentCommandDescriptor {
  id: string;
  label: string;
  description?: string;
  inputHint?: string;
  providerMetadata?: Record<string, AgentConfigValue>;
}

export type ProviderTransport = 'acp-stdio' | 'acp-websocket' | 'built-in';
export type ProviderSource = 'bundled' | 'trusted' | 'configured';

export interface AgentProviderManifest {
  id: string;
  displayName: string;
  transport: ProviderTransport;
  executable: string;
  args: string[];
  version: string;
  contentHash: string;
  trustedSource: ProviderSource;
}

export interface AgentCapabilities {
  revision: number;
  provider: AgentConversationProvider;
  implementation: { name: string; version: string };
  session: {
    list: boolean;
    load: boolean;
    resume: boolean;
    close: boolean;
    steering: boolean;
    fork?: boolean;
  };
  prompt: {
    text: boolean;
    image: boolean;
    embeddedContext: boolean;
    resourceLinks: boolean;
  };
  interaction: {
    permissions: boolean;
    structuredUserInput: boolean;
    toolTerminals: boolean;
    plans: boolean;
    tasks: boolean;
    subagents: boolean;
  };
  configOptions: AgentConfigOption[];
  commands: AgentCommandDescriptor[];
}

export type AgentConversationHandoffDirection = 'structured-to-terminal' | 'terminal-to-structured';
export type AgentConversationHandoffMode = 'same-session' | 'fork';
export type AgentConversationHandoffPhase = 'prepare' | 'commit' | 'rollback';
export type AgentConversationHandoffReceiptPhase = 'prepared' | 'committed' | 'rolled-back' | AgentConversationHandoffPhase;

export interface AgentConversationHistoryBoundary {
  nativeSessionId?: string | null;
  firstSequence: number;
  lastSequence: number;
  reconciledSequence?: number | null;
}

export interface AgentConversationProcessTreeAssertion {
  checked: boolean;
  tuiLive?: boolean;
  tuiReleased?: boolean;
  writerCount?: number;
  ptyCount?: number;
  userPtyCount?: number;
  sidecarCount?: number;
  ptySessionId?: string | null;
}

export interface AgentConversationHandoffRequest {
  ownedId: string;
  generation: number;
  direction: AgentConversationHandoffDirection;
  mode: AgentConversationHandoffMode;
  phase: AgentConversationHandoffPhase;
  expectedOwner?: AgentWriterLeaseOwner;
  targetOwnedId?: string | null;
  nativeSessionId?: string | null;
  ptySessionId?: string | null;
  historyBoundary: AgentConversationHistoryBoundary;
  processTree: AgentConversationProcessTreeAssertion;
}

export interface AgentConversationHandoffReceipt {
  ownedId: string;
  generation: number;
  direction: AgentConversationHandoffDirection;
  mode: AgentConversationHandoffMode;
  phase: AgentConversationHandoffReceiptPhase;
  previousOwner: AgentWriterLeaseOwner;
  owner: AgentWriterLeaseOwner;
  nativeSessionId?: string | null;
  ptySessionId?: string | null;
  historyBoundary: AgentConversationHistoryBoundary;
  processTree: AgentConversationProcessTreeAssertion;
  rollbackAvailable: boolean;
  message: string;
}

export function createConversationHandoffRequest(
  input: Partial<AgentConversationHandoffRequest> & Pick<AgentConversationHandoffRequest, 'ownedId' | 'generation' | 'direction' | 'mode' | 'phase'>
): AgentConversationHandoffRequest {
  return {
    ownedId: input.ownedId,
    generation: input.generation,
    direction: input.direction,
    mode: input.mode,
    phase: input.phase,
    expectedOwner: input.expectedOwner,
    targetOwnedId: input.targetOwnedId ?? null,
    nativeSessionId: input.nativeSessionId ?? null,
    ptySessionId: input.ptySessionId ?? null,
    historyBoundary: {
      nativeSessionId: input.historyBoundary?.nativeSessionId ?? input.nativeSessionId ?? null,
      firstSequence: input.historyBoundary?.firstSequence ?? 0,
      lastSequence: input.historyBoundary?.lastSequence ?? 0,
      reconciledSequence: input.historyBoundary?.reconciledSequence ?? null
    },
    processTree: {
      checked: input.processTree?.checked ?? false,
      tuiLive: input.processTree?.tuiLive ?? false,
      tuiReleased: input.processTree?.tuiReleased ?? false,
      writerCount: input.processTree?.writerCount ?? 0,
      ptyCount: input.processTree?.ptyCount ?? input.processTree?.userPtyCount ?? 0,
      sidecarCount: input.processTree?.sidecarCount ?? 0,
      ptySessionId: input.processTree?.ptySessionId ?? input.ptySessionId ?? null
    }
  };
}

export function handoffGenerationMatches(
  receipt: Pick<AgentConversationHandoffReceipt, 'ownedId' | 'generation'>,
  ownedId: string,
  generation: number
): boolean {
  return receipt.ownedId === ownedId && receipt.generation === generation;
}

export function handoffActionLabel(
  direction: AgentConversationHandoffDirection,
  mode: AgentConversationHandoffMode
): string {
  if (direction === 'terminal-to-structured') return 'Return to structured';
  return mode === 'fork' ? 'Fork to native CLI' : 'Open in native CLI';
}

export function applyConversationHandoffReceipt<T extends Record<string, any>>(
  workspace: T,
  receipt: AgentConversationHandoffReceipt
): T {
  const terminal = receipt.owner === 'terminal';
  const owner = receipt.owner;
  const writerLease = {
    ...(workspace.writerLease ?? {}),
    ownedId: receipt.ownedId,
    generation: receipt.generation,
    owner
  };
  return {
    ...workspace,
    mode: terminal ? 'raw' : 'structured',
    owner,
    executionOwner: owner,
    generation: receipt.generation,
    lastSequence: receipt.historyBoundary.lastSequence ?? workspace.lastSequence,
    nativeSessionId: receipt.nativeSessionId ?? workspace.nativeSessionId,
    writerLease,
    writerLeaseTransition: null
  };
}

export function restoreConversationHandoffAfterFailure<T extends Record<string, any>>(
  workspace: T
): T {
  return {
    ...workspace,
    writerLeaseTransition: null,
    draft: workspace.draft
  };
}

export type AgentConfigOptionPlacement =
  | 'model-picker'
  | 'reasoning-picker'
  | 'mode-picker'
  | 'model-popover'
  | 'more-options';

export function configOptionPlacement(category: string): AgentConfigOptionPlacement {
  if (category === 'model') return 'model-picker';
  if (category === 'thought_level') return 'reasoning-picker';
  if (category === 'mode') return 'mode-picker';
  if (category === 'model_config' || category.startsWith('model_')) return 'model-popover';
  return 'more-options';
}

export type AgentEventType =
  | 'session.started'
  | 'session.config.updated'
  | 'session.state.changed'
  | 'session.closed'
  | 'turn.started'
  | 'turn.completed'
  | 'turn.interrupted'
  | 'item.started'
  | 'item.updated'
  | 'item.completed'
  | 'content.delta'
  | 'approval.requested'
  | 'approval.resolved'
  | 'user-input.requested'
  | 'user-input.resolved'
  | 'plan.updated'
  | 'tasks.updated'
  | 'children.updated'
  | 'commands.updated'
  | 'usage.updated'
  | 'rate-limits.updated'
  | 'runtime.warning'
  | 'runtime.error';

export type AgentItemType =
  | 'user-message'
  | 'assistant-message'
  | 'reasoning'
  | 'plan'
  | 'task-list'
  | 'command'
  | 'file-change'
  | 'mcp-tool'
  | 'web-search'
  | 'image-view'
  | 'image-generation'
  | 'subagent'
  | 'review'
  | 'context-compaction'
  | 'error'
  | 'unknown';

export type AgentContentChannel =
  | 'assistant'
  | 'reasoning'
  | 'reasoning-summary'
  | 'plan'
  | 'command-output'
  | 'file-change-output';

export interface AgentRawFrameReference {
  id: string;
  redacted: true;
}

export interface AgentEvent {
  type: AgentEventType;
  ownedId: string;
  provider: AgentConversationProvider;
  providerInstanceId: string;
  generation: number;
  sequence: number;
  timestampMs: number;
  nativeSessionId?: string;
  turnId?: string;
  itemId?: string;
  requestId?: string;
  payload: Record<string, AgentConfigValue>;
  providerMetadata?: Record<string, AgentConfigValue>;
  rawFrameReference?: AgentRawFrameReference;
}

export interface AgentContent {
  channel: AgentContentChannel;
  text: string;
  mimeType?: string;
}

export interface AgentItem {
  id: string;
  type: AgentItemType;
  turnId?: string;
  content: AgentContent[];
  providerMetadata?: Record<string, AgentConfigValue>;
}

export interface AgentRequestIdentity {
  ownedId: string;
  generation: number;
  requestId: string;
  turnId?: string;
  itemId?: string;
}

export type AgentApprovalDecision = 'accept' | 'decline' | 'cancel';

export interface AgentPermissionOption {
  optionId: string;
  name: string;
  kind?: string;
  /**
   * True for the Allow/Deny pair the client invents when a permission request
   * arrives carrying no options of its own. Their ids are ours, not the
   * agent's, so the backend will refuse them by id — these are answered by
   * decision instead.
   */
  synthetic?: boolean;
}

export interface AgentApprovalRequest extends AgentRequestIdentity {
  title: string;
  description?: string;
  options: AgentApprovalDecision[];
}

export interface AgentPermissionRequest extends AgentRequestIdentity {
  title: string;
  toolTitle: string;
  description?: string;
  options: AgentPermissionOption[];
  state: string;
}

export interface AgentApprovalResponse extends AgentRequestIdentity {
  decision: AgentApprovalDecision;
}

export interface AgentUserInputField {
  id: string;
  label: string;
  description?: string;
  required: boolean;
  kind: 'text' | 'password' | 'select' | 'boolean';
  choices?: AgentConfigOptionChoice[];
}

export interface AgentUserInputRequest extends AgentRequestIdentity {
  title: string;
  description?: string;
  fields: AgentUserInputField[];
}

export interface AgentUserInputResponse extends AgentRequestIdentity {
  values: Record<string, AgentConfigValue>;
  cancelled: boolean;
}

export type AgentWriterLeaseOwner = 'structured' | 'terminal' | 'none';
export type AgentWriterLeaseTransitionState = 'requested' | 'committed' | 'failed';

export interface AgentWriterLease {
  ownedId: string;
  generation: number;
  owner: AgentWriterLeaseOwner;
}

export interface AgentWriterLeaseTransition {
  ownedId: string;
  generation: number;
  from: AgentWriterLeaseOwner;
  to: AgentWriterLeaseOwner;
  state: AgentWriterLeaseTransitionState;
  error?: string;
}

export interface ToolTerminalIdentity {
  ownedId: string;
  turnId: string;
  toolCallId: string;
  terminalId: string;
}

export type ConversationConnectionState =
  | 'disconnected'
  | 'connecting'
  | 'connected'
  | 'reconnecting'
  | 'failed'
  | 'closed';

export type ToolState = 'started' | 'updated' | 'completed' | 'failed';
export type ApprovalState = 'requested' | 'accepted' | 'declined' | 'expired';
export type TurnState = 'started' | 'completed' | 'interrupted' | 'failed';

export type AgentConversationPayload =
  | {
      kind: 'connection';
      state: ConversationConnectionState;
      nativeSessionId?: string;
    }
  /** `attachmentIds` names the saved screenshots the message went out with.
   * Journal records written before the field existed carry none. */
  | { kind: 'userMessage'; itemId: string; text: string; completed: boolean; attachmentIds?: string[] }
  | { kind: 'assistantDelta'; itemId: string; delta: string }
  | { kind: 'assistantMessage'; itemId: string; text: string; completed: true }
  | { kind: 'tool'; itemId: string; name: string; state: ToolState; summary?: string }
  | {
      kind: 'childUpdate';
      childId: string;
      parentToolCallId: string;
      label?: string;
      state: string;
      latestActivity?: string;
    }
  | { kind: 'approval'; requestId: string; state: ApprovalState; summary: string }
  | {
      kind: 'plan';
      items?: { text: string; status: string; id?: string; title?: string; detail?: string }[];
      entries?: { content?: string; text?: string; title?: string; status?: string; id?: string; detail?: string }[];
      planId?: string;
      turnId?: string;
      _meta?: Record<string, AgentConfigValue>;
    }
  | { kind: 'turn'; turnId: string; state: TurnState }
  | {
      kind: 'availableCommandsUpdate';
      availableCommands: AgentCommandDescriptor[];
    }
  | {
      kind: 'usage';
      inputTokens?: number;
      outputTokens?: number;
      usedTokens?: number;
      contextWindow?: number;
    }
  | {
      kind: 'terminalProjection';
      eventType: AgentEventType;
      providerInstanceId: string;
      timestampMs: number | null;
      nativeSessionId: string;
      itemId: string | null;
      payload: Record<string, AgentConfigValue>;
      providerMetadata: Record<string, AgentConfigValue>;
      rawFrameReference: AgentRawFrameReference;
    }
  | { kind: 'error'; code: string; message: string; recoverable: boolean };

/**
 * Rich ACP bridge payloads are retained until the timeline has normalized
 * them. The Rust controller serializes the enum tag and fields as camelCase;
 * optional aliases keep replay snapshots from older controller builds usable.
 */
export type AgentConversationRichPayload =
  | {
      kind: 'agentThoughtChunk';
      messageId?: string;
      itemId?: string;
      turnId?: string;
      content?: AgentConfigValue;
      text?: string;
      delta?: string;
      _meta?: Record<string, AgentConfigValue>;
    }
  | {
      kind: 'toolCall' | 'toolCallUpdate';
      toolCallId?: string;
      itemId?: string;
      turnId?: string;
      title?: string;
      name?: string;
      toolKind?: string;
      status?: string;
      state?: string;
      content?: AgentConfigValue;
      locations?: AgentConfigValue;
      command?: string;
      path?: string;
      diff?: string;
      _meta?: Record<string, AgentConfigValue>;
    }
  | {
      kind: 'turnDiff';
      turnId?: string;
      itemId?: string;
      path?: string;
      diff: string;
      status?: string;
      _meta?: Record<string, AgentConfigValue>;
    }
  | {
      kind: 'permissionRequest';
      requestId: string;
      turnId?: string;
      itemId?: string;
      toolCallId?: string;
      title?: string;
      toolTitle?: string;
      description?: string;
      options?: AgentPermissionOption[];
      state?: string;
      _meta?: Record<string, AgentConfigValue>;
    }
  | {
      kind: 'availableCommandsUpdate';
      availableCommands: AgentCommandDescriptor[];
      _meta?: Record<string, AgentConfigValue>;
    }
  | {
      kind: 'agentMessageChunk' | 'userMessageChunk';
      messageId?: string;
      itemId?: string;
      turnId?: string;
      content?: AgentConfigValue;
      text?: string;
      delta?: string;
      _meta?: Record<string, AgentConfigValue>;
    };

export interface AgentConversationEvent {
  ownedId: string;
  provider: AgentConversationProvider;
  generation: number;
  sequence: number;
  timestampMs: number;
  payload: AgentConversationPayload | AgentConversationRichPayload;
}

interface ConversationTextEntry {
  itemId: string;
  text: string;
  completed: boolean;
  timestampMs: number;
}

export type ConversationTimelineEntry =
  | (ConversationTextEntry & { kind: 'user' | 'assistant' })
  | {
      kind: 'tool';
      itemId: string;
      name: string;
      state: ToolState;
      summary?: string;
      timestampMs: number;
    }
  | {
      kind: 'approval';
      itemId: string;
      requestId: string;
      state: ApprovalState;
      summary: string;
      timestampMs: number;
    }
  | {
      kind: 'plan';
      itemId: string;
      items: { text: string; status: string }[];
      timestampMs: number;
    }
  | {
      kind: 'turn';
      itemId: string;
      turnId: string;
      state: TurnState;
      timestampMs: number;
    }
  | {
      kind: 'error';
      itemId: string;
      code: string;
      message: string;
      recoverable: boolean;
      timestampMs: number;
    };

export interface ConversationUsage {
  inputTokens?: number;
  outputTokens?: number;
  usedTokens?: number;
  contextWindow?: number;
}

export interface ConversationMetadata {
  model: string | null;
  effort: string | null;
  approvalPolicy: string | null;
  usedTokens: number | null;
  contextWindow: number | null;
}

export interface ConversationChildAgent {
  childId: string;
  parentId: string;
  parentToolCallId?: string;
  provider: AgentConversationProvider;
  label: string;
  state: string;
  latestActivity?: string;
  updatedAtMs: number;
}

export interface ConversationTranscriptMessage {
  itemId: string;
  role: 'user' | 'assistant';
  text: string;
  timestampMs: number;
}

export interface ConversationTranscriptSnapshot {
  messages: ConversationTranscriptMessage[];
  metadata: ConversationMetadata;
  children: ConversationChildAgent[];
}

export interface ConversationAttachment {
  id: string;
  name: string;
  mimeType: string;
  path: string;
  previewUrl: string;
}

export interface ConversationSessionState {
  ownedId: string;
  provider: AgentConversationProvider;
  generation: number;
  lastSequence: number;
  desynchronized: boolean;
  connectionState: ConversationConnectionState;
  suspended: boolean;
  /** Monotonic display-content version used by scroll effects. */
  timelineRevision: number;
  nativeSessionId?: string;
  activeTurnId?: string;
  usage?: ConversationUsage;
  timeline: ConversationTimelineEntry[];
}

export interface AgentConversationConnection {
  ownedId: string;
  provider: AgentConversationProvider;
  generation: number;
  nativeSessionId?: string;
  state: ConversationConnectionState;
  config?: import('./conversationConfig').AgentConversationConfigState;
}

/** One backward page of transcript events, with whether older history remains. */
export interface AgentConversationEventPage {
  events: AgentConversationEvent[];
  hasMore: boolean;
}

export interface AgentConversationSnapshot {
  connection: AgentConversationConnection;
  suspended?: boolean;
  lastSequence: number;
  events: AgentConversationEvent[];
}
