export type AgentConversationProvider = 'codex' | 'claude';

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
