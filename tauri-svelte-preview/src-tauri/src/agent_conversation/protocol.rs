use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Carries one predictable error object across the conversation command boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub message: String,
    pub recoverable: bool,
}

impl From<String> for CommandError {
    /// Preserves the backend message while giving every command rejection one stable shape.
    fn from(message: String) -> Self {
        Self {
            code: "agent-conversation-command-failed",
            message,
            recoverable: false,
        }
    }
}

/// Keeps conversation command signatures concise while enforcing the typed error boundary.
pub type CommandResult<T> = Result<T, CommandError>;

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
    Suspended,
    Failed,
    Closed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentConversationProvider {
    Codex,
    Claude,
    Antigravity,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentNativeSessionMode {
    #[default]
    Resume,
    Load,
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
    #[serde(default, skip_serializing_if = "is_false")]
    pub multi_session: bool,
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
    #[serde(rename = "commands.updated")]
    CommandsUpdated,
    #[serde(rename = "usage.updated")]
    UsageUpdated,
    #[serde(rename = "rate-limits.updated")]
    RateLimitsUpdated,
    #[serde(rename = "runtime.warning")]
    RuntimeWarning,
    #[serde(rename = "runtime.error")]
    RuntimeError,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AgentRawFrameReference {
    pub id: String,
    pub redacted: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalProjectionPayload {
    pub event_type: AgentEventType,
    pub provider_instance_id: String,
    pub timestamp_ms: Option<u64>,
    pub native_session_id: String,
    pub item_id: Option<String>,
    pub payload: BTreeMap<String, Value>,
    pub provider_metadata: BTreeMap<String, Value>,
    pub raw_frame_reference: AgentRawFrameReference,
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
    /// Position in the session journal. Signed because importing older history
    /// writes it below what is already stored, counting down through zero.
    pub sequence: i64,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    pub text: String,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
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
        /// Ids of the saved screenshots this message went out with. Journal
        /// records written before this field existed simply carry none.
        #[serde(default)]
        attachment_ids: Vec<String>,
    },
    AssistantDelta {
        item_id: String,
        delta: String,
    },
    AssistantMessage {
        item_id: String,
        text: String,
        completed: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        blocks: Option<Vec<crate::agent_conversation::safe_markdown::SafeMarkdownBlock>>,
    },
    Tool {
        item_id: String,
        name: String,
        state: ToolState,
        /// The one line a collapsed row shows.
        #[serde(skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
        /// What the call produced, kept whole. A row with nothing here has
        /// nothing to open, which is what a tool call used to be: its content
        /// was flattened into the summary and the rest thrown away.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        /// The file the call was about.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        /// A unified diff, when the call changed a file.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        diff: Option<String>,
    },
    ChildUpdate {
        child_id: String,
        parent_tool_call_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        state: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        latest_activity: Option<String>,
    },
    Approval {
        request_id: String,
        state: ApprovalState,
        summary: String,
    },
    UserInputRequested {
        request_id: String,
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        fields: Vec<AgentUserInputField>,
    },
    UserInputResolved {
        request_id: String,
        cancelled: bool,
    },
    Plan {
        items: Vec<PlanItem>,
    },
    Turn {
        turn_id: String,
        state: TurnState,
    },
    AvailableCommandsUpdate {
        available_commands: Vec<AgentCommandDescriptor>,
    },
    Usage {
        #[serde(skip_serializing_if = "Option::is_none")]
        input_tokens: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output_tokens: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        used_tokens: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        context_window: Option<u64>,
    },
    /// The agent threw away the older part of the conversation to make room.
    /// The provider says so outright, or the reported occupancy falls far
    /// enough that nothing else explains it; either way the transcript says
    /// where it happened rather than leaving a silent gap.
    ContextCompaction {
        #[serde(skip_serializing_if = "Option::is_none")]
        trigger: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pre_tokens: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        post_tokens: Option<u64>,
    },
    /// Records the durable Codex checkout change separately from provider
    /// transcript content. The old root is retained so the timeline explains
    /// which projection was released and where the session now runs.
    CheckoutChanged {
        from_cwd: String,
        to_cwd: String,
    },
    TerminalProjection(TerminalProjectionPayload),
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
    /// Position in the session journal. Signed because importing older history
    /// writes it below what is already stored, counting down through zero.
    pub sequence: i64,
    pub timestamp_ms: u128,
    pub payload: AgentConversationPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnsureAgentConversationRequest {
    pub owned_id: String,
    #[serde(default)]
    pub execution_environment: ExecutionEnvironment,
    pub provider: AgentConversationProvider,
    pub cwd: String,
    pub native_session_id: Option<String>,
    #[serde(default)]
    pub native_session_mode: AgentNativeSessionMode,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
}

/// The machine that owns a conversation's complete runtime and durable state.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionEnvironment {
    #[default]
    Local,
    Remote,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAgentConversationMessageRequest {
    pub owned_id: String,
    pub generation: u64,
    pub text: String,
    /// Ordered prompt blocks from the composer, including pasted images.
    #[serde(default)]
    pub content: Vec<super::prompt_content::AgentPromptContentBlock>,
    /// Ids of the saved screenshots this send is delivering, so the recorded
    /// user message can find them again after a restart.
    #[serde(default)]
    pub attachment_ids: Vec<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub approval_policy: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalDecision {
    Accept,
    Decline,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespondAgentConversationApprovalRequest {
    pub owned_id: String,
    pub generation: u64,
    pub request_id: String,
    pub decision: ApprovalDecision,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespondAgentConversationPermissionRequest {
    pub owned_id: String,
    pub generation: u64,
    pub request_id: String,
    pub option_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespondAgentConversationInputRequest {
    pub owned_id: String,
    pub generation: u64,
    pub request_id: String,
    #[serde(default)]
    pub values: BTreeMap<String, Value>,
    pub cancelled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopAgentConversationTurnRequest {
    pub owned_id: String,
    pub generation: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeAgentConversationCheckoutRequest {
    pub owned_id: String,
    pub generation: u64,
    pub cwd: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AgentConversationConfigState {
    pub model: Option<String>,
    pub available_models: Vec<String>,
    pub reasoning_effort: Option<String>,
    pub available_efforts: Vec<String>,
    pub approval_policy: Option<String>,
    pub available_approval_policies: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAgentConversationConfigRequest {
    pub owned_id: String,
    pub generation: u64,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub approval_policy: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationConnection {
    pub owned_id: String,
    pub provider: AgentConversationProvider,
    pub generation: u64,
    pub native_session_id: Option<String>,
    pub state: ConversationConnectionState,
    pub config: AgentConversationConfigState,
}

/// One backward page of transcript events, with whether older history remains.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationEventPage {
    pub events: Vec<AgentConversationEvent>,
    pub has_more: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationSnapshot {
    pub connection: AgentConversationConnection,
    pub suspended: bool,
    pub last_sequence: i64,
    pub events: Vec<AgentConversationEvent>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationSessionMeta {
    pub worktree: Option<String>,
    pub branch: Option<String>,
    pub title: Option<String>,
    pub project: Option<String>,
    pub pty_session_id: Option<String>,
    pub origin: Option<String>,
    pub source: Option<String>,
    pub via_cmux: bool,
    pub resume_command: Option<String>,
    pub completed_at: Option<String>,
    pub settled_at: Option<String>,
    pub task_id: Option<String>,
    pub pull_request: Option<String>,
    pub message_count: Option<u64>,
    pub latest_turn_preview: Option<String>,
    pub scanned_last_activity: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAgentConversationSessionMetaRequest {
    pub owned_id: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub meta: AgentConversationSessionMeta,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationSessionRecord {
    pub owned_id: String,
    pub execution_environment: ExecutionEnvironment,
    pub provider: AgentConversationProvider,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub cwd: String,
    pub state: AgentRuntimeState,
    pub suspended: bool,
    pub created_at_ms: i64,
    pub last_activity_at_ms: i64,
    pub active_turn_id: Option<String>,
    pub pending_permission: bool,
    pub pending_input: bool,
    pub native_session_id: Option<String>,
    #[serde(flatten)]
    pub meta: AgentConversationSessionMeta,
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn omitted_execution_environment_means_this_mac() {
        let request: EnsureAgentConversationRequest = serde_json::from_value(json!({
            "ownedId": "owned-local",
            "provider": "codex",
            "cwd": "/tmp/project",
            "nativeSessionId": null
        }))
        .unwrap();

        assert_eq!(request.execution_environment, ExecutionEnvironment::Local);
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

        let plan = AgentConversationPayload::Plan {
            items: vec![PlanItem {
                text: "Inspect the change".to_string(),
                status: "pending".to_string(),
            }],
        };
        assert_eq!(
            serde_json::from_value::<AgentConversationPayload>(
                serde_json::to_value(&plan).unwrap()
            )
            .unwrap(),
            plan
        );
    }

    #[test]
    fn conversation_config_contract_uses_camel_case_fields() {
        let config = AgentConversationConfigState {
            model: Some("gpt-5.6-sol".into()),
            available_models: vec!["gpt-5.6-sol".into(), "gpt-5.6-terra".into()],
            reasoning_effort: Some("xhigh".into()),
            available_efforts: vec!["low".into(), "xhigh".into()],
            approval_policy: Some("on-request".into()),
            available_approval_policies: vec!["untrusted".into(), "on-request".into()],
        };
        assert_eq!(
            serde_json::to_value(config).unwrap(),
            json!({
                "model": "gpt-5.6-sol",
                "availableModels": ["gpt-5.6-sol", "gpt-5.6-terra"],
                "reasoningEffort": "xhigh",
                "availableEfforts": ["low", "xhigh"],
                "approvalPolicy": "on-request",
                "availableApprovalPolicies": ["untrusted", "on-request"]
            })
        );
    }
}
