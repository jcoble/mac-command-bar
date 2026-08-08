use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentExecutionOwner {
    Structured,
    Terminal,
    TransitioningToStructured,
    TransitioningToTerminal,
    Stopped,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentRuntimeState {
    Starting,
    Ready,
    Working,
    WaitingApproval,
    WaitingInput,
    Interrupting,
    Failed,
    Closed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnedAgentRuntimeFields {
    pub execution_owner: AgentExecutionOwner,
    pub runtime_state: AgentRuntimeState,
    pub provider_instance_id: Option<String>,
    pub native_session_id: Option<String>,
    pub active_turn_id: Option<String>,
    pub capability_revision: u64,
    pub last_runtime_error: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentConversationProvider {
    Codex,
    Claude,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfigOptionChoice {
    pub value: Value,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfigOption {
    pub id: String,
    pub label: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<AgentConfigOptionChoice>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<BTreeMap<String, Value>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCommandDescriptor {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<BTreeMap<String, Value>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderTransport {
    AcpStdio,
    AcpWebSocket,
    BuiltIn,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderSource {
    Bundled,
    Trusted,
    Configured,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProviderManifest {
    pub id: String,
    pub display_name: String,
    pub transport: ProviderTransport,
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub version: String,
    pub content_hash: String,
    pub trusted_source: ProviderSource,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    pub revision: u64,
    pub provider: AgentConversationProvider,
    pub implementation: AgentImplementation,
    pub session: AgentSessionCapabilities,
    pub prompt: AgentPromptCapabilities,
    pub interaction: AgentInteractionCapabilities,
    pub config_options: Vec<AgentConfigOption>,
    pub commands: Vec<AgentCommandDescriptor>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AgentImplementation {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AgentSessionCapabilities {
    pub list: bool,
    pub load: bool,
    pub resume: bool,
    pub close: bool,
    pub steering: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub fork: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPromptCapabilities {
    pub text: bool,
    pub image: bool,
    pub embedded_context: bool,
    pub resource_links: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInteractionCapabilities {
    pub permissions: bool,
    pub structured_user_input: bool,
    pub tool_terminals: bool,
    pub plans: bool,
    pub tasks: bool,
    pub subagents: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AgentEventType {
    #[serde(rename = "session.started")]
    SessionStarted,
    #[serde(rename = "session.config.updated")]
    SessionConfigUpdated,
    #[serde(rename = "session.state.changed")]
    SessionStateChanged,
    #[serde(rename = "session.closed")]
    SessionClosed,
    #[serde(rename = "turn.started")]
    TurnStarted,
    #[serde(rename = "turn.completed")]
    TurnCompleted,
    #[serde(rename = "turn.interrupted")]
    TurnInterrupted,
    #[serde(rename = "item.started")]
    ItemStarted,
    #[serde(rename = "item.updated")]
    ItemUpdated,
    #[serde(rename = "item.completed")]
    ItemCompleted,
    #[serde(rename = "content.delta")]
    ContentDelta,
    #[serde(rename = "approval.requested")]
    ApprovalRequested,
    #[serde(rename = "approval.resolved")]
    ApprovalResolved,
    #[serde(rename = "user-input.requested")]
    UserInputRequested,
    #[serde(rename = "user-input.resolved")]
    UserInputResolved,
    #[serde(rename = "plan.updated")]
    PlanUpdated,
    #[serde(rename = "tasks.updated")]
    TasksUpdated,
    #[serde(rename = "children.updated")]
    ChildrenUpdated,
    #[serde(rename = "usage.updated")]
    UsageUpdated,
    #[serde(rename = "rate-limits.updated")]
    RateLimitsUpdated,
    #[serde(rename = "runtime.warning")]
    RuntimeWarning,
    #[serde(rename = "runtime.error")]
    RuntimeError,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentItemType {
    UserMessage,
    AssistantMessage,
    Reasoning,
    Plan,
    TaskList,
    Command,
    FileChange,
    McpTool,
    WebSearch,
    ImageView,
    ImageGeneration,
    Subagent,
    Review,
    ContextCompaction,
    Error,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentContentChannel {
    Assistant,
    Reasoning,
    ReasoningSummary,
    Plan,
    CommandOutput,
    FileChangeOutput,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AgentRawFrameReference {
    pub id: String,
    pub redacted: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEvent {
    #[serde(rename = "type")]
    pub event_type: AgentEventType,
    pub owned_id: String,
    pub provider: AgentConversationProvider,
    pub provider_instance_id: String,
    pub generation: u64,
    pub sequence: u64,
    pub timestamp_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub payload: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_frame_reference: Option<AgentRawFrameReference>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentContent {
    pub channel: AgentContentChannel,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentItem {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: AgentItemType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    pub content: Vec<AgentContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<BTreeMap<String, Value>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRequestIdentity {
    pub owned_id: String,
    pub generation: u64,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentApprovalDecision {
    Accept,
    Decline,
    Cancel,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentApprovalRequest {
    #[serde(flatten)]
    pub identity: AgentRequestIdentity,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub options: Vec<AgentApprovalDecision>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentApprovalResponse {
    #[serde(flatten)]
    pub identity: AgentRequestIdentity,
    pub decision: AgentApprovalDecision,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentUserInputKind {
    Text,
    Password,
    Select,
    Boolean,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUserInputField {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub required: bool,
    pub kind: AgentUserInputKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<AgentConfigOptionChoice>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUserInputRequest {
    #[serde(flatten)]
    pub identity: AgentRequestIdentity,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub fields: Vec<AgentUserInputField>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUserInputResponse {
    #[serde(flatten)]
    pub identity: AgentRequestIdentity,
    pub values: BTreeMap<String, Value>,
    pub cancelled: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentWriterLeaseOwner {
    Structured,
    Terminal,
    None,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentWriterLeaseTransitionState {
    Requested,
    Committed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWriterLease {
    pub owned_id: String,
    pub generation: u64,
    pub owner: AgentWriterLeaseOwner,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWriterLeaseTransition {
    pub owned_id: String,
    pub generation: u64,
    pub from: AgentWriterLeaseOwner,
    pub to: AgentWriterLeaseOwner,
    pub state: AgentWriterLeaseTransitionState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolTerminalIdentity {
    pub owned_id: String,
    pub turn_id: String,
    pub tool_call_id: String,
    pub terminal_id: String,
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

#[cfg(test)]
mod contract_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn runtime_owner_and_state_use_the_public_wire_names() {
        let fields = OwnedAgentRuntimeFields {
            execution_owner: AgentExecutionOwner::TransitioningToStructured,
            runtime_state: AgentRuntimeState::WaitingApproval,
            provider_instance_id: Some("provider-a".to_string()),
            native_session_id: Some("native-a".to_string()),
            active_turn_id: None,
            capability_revision: 4,
            last_runtime_error: None,
        };
        let value = serde_json::to_value(&fields).unwrap();
        assert_eq!(value["executionOwner"], "transitioning-to-structured");
        assert_eq!(value["runtimeState"], "waiting-approval");
        assert_eq!(
            serde_json::from_value::<OwnedAgentRuntimeFields>(value).unwrap(),
            fields
        );
    }

    #[test]
    fn unknown_config_categories_round_trip_losslessly() {
        let value = json!({
            "revision": 9,
            "provider": "codex",
            "implementation": { "name": "fixture", "version": "1" },
            "session": { "list": true, "load": true, "resume": true, "close": true, "steering": true },
            "prompt": { "text": true, "image": true, "embeddedContext": true, "resourceLinks": true },
            "interaction": {
                "permissions": true, "structuredUserInput": true, "toolTerminals": true,
                "plans": true, "tasks": true, "subagents": true
            },
            "configOptions": [{
                "id": "future", "label": "Future", "category": "provider.future/category",
                "value": { "nested": ["kept", 3] },
                "providerMetadata": { "provider_field": "kept" }
            }],
            "commands": []
        });
        let capabilities: AgentCapabilities = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            capabilities.config_options[0].category,
            "provider.future/category"
        );
        assert_eq!(serde_json::to_value(capabilities).unwrap(), value);
    }

    #[test]
    fn canonical_event_round_trips_without_acp_frames() {
        let value = json!({
            "type": "content.delta",
            "ownedId": "owned-a",
            "provider": "claude",
            "providerInstanceId": "provider-a",
            "generation": 3,
            "sequence": 12,
            "timestampMs": 1000,
            "nativeSessionId": "native-a",
            "turnId": "turn-a",
            "itemId": "item-a",
            "payload": { "channel": "assistant", "text": "hello" },
            "providerMetadata": { "claude.example": "retained" },
            "rawFrameReference": { "id": "frame-a", "redacted": true }
        });
        let event: AgentEvent = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(event.event_type, AgentEventType::ContentDelta);
        assert_eq!(serde_json::to_value(event).unwrap(), value);
    }
}
