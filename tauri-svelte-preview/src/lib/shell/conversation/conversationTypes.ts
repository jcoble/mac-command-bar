export type AgentConversationProvider = 'codex' | 'claude';

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

export interface AgentApprovalRequest extends AgentRequestIdentity {
  title: string;
  description?: string;
  options: AgentApprovalDecision[];
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
  | { kind: 'userMessage'; itemId: string; text: string; completed: true }
  | { kind: 'assistantDelta'; itemId: string; delta: string }
  | { kind: 'assistantMessage'; itemId: string; text: string; completed: true }
  | { kind: 'tool'; itemId: string; name: string; state: ToolState; summary?: string }
  | { kind: 'approval'; requestId: string; state: ApprovalState; summary: string }
  | { kind: 'turn'; turnId: string; state: TurnState }
  | { kind: 'usage'; inputTokens?: number; outputTokens?: number }
  | { kind: 'error'; code: string; message: string; recoverable: boolean };

export interface AgentConversationEvent {
  ownedId: string;
  provider: AgentConversationProvider;
  generation: number;
  sequence: number;
  timestampMs: number;
  payload: AgentConversationPayload;
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
  provider: AgentConversationProvider;
  label: string;
  state: string;
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
}

export interface AgentConversationSnapshot {
  connection: AgentConversationConnection;
  lastSequence: number;
  events: AgentConversationEvent[];
}
