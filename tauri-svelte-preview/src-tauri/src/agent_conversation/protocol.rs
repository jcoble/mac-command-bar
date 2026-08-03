use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentConversationProvider {
    Codex,
    Claude,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConversationConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
    Closed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolState {
    Started,
    Updated,
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ApprovalState {
    Requested,
    Accepted,
    Declined,
    Expired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TurnState {
    Started,
    Completed,
    Interrupted,
    Failed,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AgentConversationPayload {
    Connection {
        state: ConversationConnectionState,
        #[serde(skip_serializing_if = "Option::is_none")]
        native_session_id: Option<String>,
    },
    UserMessage {
        item_id: String,
        text: String,
        completed: bool,
    },
    AssistantDelta {
        item_id: String,
        delta: String,
    },
    AssistantMessage {
        item_id: String,
        text: String,
        completed: bool,
    },
    Tool {
        item_id: String,
        name: String,
        state: ToolState,
        #[serde(skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
    },
    Approval {
        request_id: String,
        state: ApprovalState,
        summary: String,
    },
    Turn {
        turn_id: String,
        state: TurnState,
    },
    Usage {
        #[serde(skip_serializing_if = "Option::is_none")]
        input_tokens: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output_tokens: Option<u64>,
    },
    Error {
        code: String,
        message: String,
        recoverable: bool,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationEvent {
    pub owned_id: String,
    pub provider: AgentConversationProvider,
    pub generation: u64,
    pub sequence: u64,
    pub timestamp_ms: u128,
    pub payload: AgentConversationPayload,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnsureAgentConversationRequest {
    pub owned_id: String,
    pub provider: AgentConversationProvider,
    pub cwd: String,
    pub native_session_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAgentConversationMessageRequest {
    pub owned_id: String,
    pub generation: u64,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalDecision {
    Accept,
    Decline,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespondAgentConversationApprovalRequest {
    pub owned_id: String,
    pub generation: u64,
    pub request_id: String,
    pub decision: ApprovalDecision,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopAgentConversationTurnRequest {
    pub owned_id: String,
    pub generation: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationConnection {
    pub owned_id: String,
    pub provider: AgentConversationProvider,
    pub generation: u64,
    pub native_session_id: Option<String>,
    pub state: ConversationConnectionState,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationSnapshot {
    pub connection: AgentConversationConnection,
    pub last_sequence: u64,
    pub events: Vec<AgentConversationEvent>,
}
