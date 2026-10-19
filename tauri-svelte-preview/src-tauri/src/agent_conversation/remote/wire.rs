use std::collections::BTreeMap;
use std::io::{Read, Write};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use futures_util::SinkExt;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

use super::{MAX_SERVER_FRAME_BYTES, MAX_WIRE_FRAME_BYTES};
use crate::agent_conversation::attachments::{DeleteConversationAttachmentRequest, SavedConversationAttachment};
use crate::agent_conversation::protocol::{
    AgentCapabilities, AgentConfigOption, AgentConversationConfigState, AgentConversationConnection,
    AgentConversationEvent, AgentConversationEventPage, AgentConversationProvider,
    AgentConversationSendReceipt, AgentConversationSessionRecord, AgentConversationSnapshot,
    ChangeAgentConversationCheckoutRequest, EnsureAgentConversationRequest,
    RespondAgentConversationApprovalRequest, RespondAgentConversationInputRequest,
    RespondAgentConversationPermissionRequest, SendAgentConversationMessageRequest,
    SetAgentConversationConfigRequest, StopAgentConversationTurnRequest,
    UpdateAgentConversationSessionMetaRequest,
};
use crate::terminal::{TerminalOutputFrame, TerminalScrollbackSnapshot, TerminalSessionInfo, TerminalStartRequest};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(super) enum ClientFrame {
    Request { id: u64, command: RemoteCommand },
    Cancel { id: u64 },
    Resume { cursors: BTreeMap<String, i64> },
    /// The desktop's answer to a `RelayRequest`.
    RelayReply { request_id: u64, result: Result<serde_json::Value, String> },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command", content = "input", rename_all = "camelCase")]
pub(in crate::agent_conversation) enum RemoteCommand {
    Workspace { operation: String, args: serde_json::Value },
    ProbeProviderConfig { provider: AgentConversationProvider, cwd: String },
    ListSessions,
    CheckProviderUpdates,
    InstallProviderUpdates { provider: String },
    RestartForProviderUpdates,
    Ensure(EnsureAgentConversationRequest),
    Snapshot {
        owned_id: String,
        request_id: u64,
        after_sequence: Option<i64>,
    },
    ExtendImport { owned_id: String },
    SelectChildHistory {
        parent_owned_id: String,
        child_session_id: String,
        request_id: u64,
    },
    StopChildHistory {
        parent_owned_id: String,
        request_id: u64,
    },
    DownloadHistory { owned_id: String, before_sequence: Option<i64> },
    EventsAfter {
        owned_id: String,
        after_sequence: i64,
        max_bytes: u32,
    },
    Capabilities {
        owned_id: String,
    },
    Config {
        owned_id: String,
    },
    WarmConfig {
        owned_id: String,
        generation: u64,
    },
    SetConfig(SetAgentConversationConfigRequest),
    SetConfigOption(crate::agent_conversation::SetAgentConversationConfigOptionRequest),
    SetDraft {
        owned_id: String,
        text: String,
    },
    GetDraft {
        owned_id: String,
    },
    ClearDraft {
        owned_id: String,
    },
    WriteWorkspace {
        owned_id: String,
        snapshot_json: String,
    },
    ReadWorkspace {
        owned_id: String,
    },
    ReadExpandedPaths {
        owned_id: String,
        root: String,
    },
    WriteExpandedPaths {
        owned_id: String,
        root: String,
        paths: Vec<String>,
    },
    DeleteWorkspace {
        owned_id: String,
    },
    ChangeCheckout(ChangeAgentConversationCheckoutRequest),
    UpdateMeta(UpdateAgentConversationSessionMetaRequest),
    Close {
        owned_id: String,
        generation: u64,
    },
    Delete {
        owned_id: String,
    },
    AttachmentChunk { upload_id: String, owned_id: String, mime_type: String, first: bool, last: bool, bytes: String },
    AbortAttachmentUpload { upload_id: String },
    ReadAttachments { owned_id: String },
    ReadSelectedAttachments { owned_id: String, attachment_ids: Vec<String> },
    ReadAttachmentChunk { owned_id: String, attachment_id: String, thumbnail: bool, offset: u64 },
    DeleteAttachment(DeleteConversationAttachmentRequest),
    Send(SendAgentConversationMessageRequest),
    RespondApproval(RespondAgentConversationApprovalRequest),
    RespondPermission(RespondAgentConversationPermissionRequest),
    RespondInput(RespondAgentConversationInputRequest),
    Stop(StopAgentConversationTurnRequest),
    /// One `workflow::execute_operation` on this machine's workflow engine.
    Workflow { operation: String, args: serde_json::Value },
    /// One `messaging::answer` on this machine's sessions, for a message from another machine.
    Messaging { op: String, args: serde_json::Value },
    TerminalStart(TerminalStartRequest),
    TerminalWrite { session_id: String, data: String },
    TerminalResize { session_id: String, cols: Option<u16>, rows: Option<u16> },
    TerminalScrollback { session_id: String, max_bytes: Option<usize> },
    TerminalClose { session_id: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "result", content = "value", rename_all = "camelCase")]
pub(in crate::agent_conversation) enum RemoteResponse {
    Workspace(serde_json::Value),
    Sessions(Vec<AgentConversationSessionRecord>),
    ProviderUpdates(crate::agent_conversation::providers::updates::ProviderUpdateStatus),
    Restarting,
    Connection(AgentConversationConnection),
    Snapshot(#[serde(deserialize_with = "deserialize_wire_payload")] Option<AgentConversationSnapshot>),
    ChildHistoryId(String),
    EventPage(#[serde(deserialize_with = "deserialize_wire_payload")] AgentConversationEventPage),
    Capabilities(AgentCapabilities),
    Config(AgentConversationConfigState),
    ConfigOptions(Vec<AgentConfigOption>),
    Session(AgentConversationSessionRecord),
    Attachment(SavedConversationAttachment),
    Attachments(Vec<SavedConversationAttachment>),
    Bytes(String),
    OptionalString(Option<String>),
    Strings(Vec<String>),
    Bool(bool),
    ImportProgress { added: usize, reached_start: bool },
    SendReceipt(AgentConversationSendReceipt),
    Workflow(serde_json::Value),
    Messaging(serde_json::Value),
    Empty,
    TerminalSession(#[serde(deserialize_with = "deserialize_wire_payload")] TerminalSessionInfo),
    TerminalScrollback(Option<TerminalScrollbackSnapshot>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(super) enum ServerFrame {
    HistoryDownloadStart { id: u64, owned_id: String, low_sequence: Option<i64>, high_sequence: Option<i64>, row_count: u64, total_bytes: u64, covered_bytes: u64,
        #[serde(deserialize_with = "deserialize_wire_payload")] snapshot: AgentConversationSnapshot },
    HistoryDownloadChunk { id: u64, owned_id: String, oldest_sequence: i64, covered_bytes: u64, gzip_base64: String },
    HistoryDownloadDone { id: u64, owned_id: String },
    Ready { protocol_version: u16 },
    MessagesChanged,
    ReplayStart { owned_id: String },
    ReplayComplete { owned_id: String, head: i64, #[serde(deserialize_with = "deserialize_wire_payload")] snapshot: AgentConversationSnapshot },
    Response { id: u64, response: RemoteResponse },
    Error { id: u64, message: String },
    ProviderDownloadProgress { id: u64, progress: crate::agent_conversation::providers::updates::ProviderDownloadProgress },
    Event { #[serde(deserialize_with = "deserialize_wire_payload")] event: AgentConversationEvent },
    TerminalOutput { frame: TerminalOutputFrame },
    /// A session here messaging another machine; the desktop routes it and answers with `RelayReply`.
    RelayRequest { request_id: u64, op: String, args: serde_json::Value },
}

pub(super) fn encode_history_chunk(events: &[AgentConversationEvent]) -> Result<String, String> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&serde_json::to_vec(events).map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
    Ok(BASE64.encode(encoder.finish().map_err(|error| error.to_string())?))
}

pub(super) fn decode_history_chunk(gzip_base64: &str) -> Result<Vec<AgentConversationEvent>, String> {
    let bytes = BASE64.decode(gzip_base64).map_err(|error| error.to_string())?;
    let mut json = Vec::new();
    flate2::read::GzDecoder::new(bytes.as_slice()).read_to_end(&mut json).map_err(|error| error.to_string())?;
    serde_json::from_slice(&json).map_err(|error| error.to_string())
}

// Serde's buffered tagged-enum decoder cannot deserialize u128 timestamps.
// Decode these existing payloads through JSON's numeric decoder; wire shape stays unchanged.
fn deserialize_wire_payload<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where D: serde::Deserializer<'de>, T: serde::de::DeserializeOwned {
    let value = serde_json::Value::deserialize(deserializer)?;
    serde_json::from_value(value).map_err(serde::de::Error::custom)
}

pub(super) async fn send_client_frame<S>(socket: &mut S, frame: &ClientFrame) -> Result<(), String>
where
    S: futures_util::Sink<TungsteniteMessage> + Unpin,
    S::Error: std::fmt::Display,
{
    let json = serde_json::to_string(frame).map_err(|error| error.to_string())?;
    if json.len() > MAX_WIRE_FRAME_BYTES {
        return Err("Remote Assembly frame exceeds one MiB".to_string());
    }
    socket
        .send(TungsteniteMessage::Text(json.into()))
        .await
        .map_err(|error| error.to_string())
}

pub(super) fn parse_server_frame(message: TungsteniteMessage) -> Result<Option<ServerFrame>, String> {
    let TungsteniteMessage::Text(text) = message else { return Ok(None); };
    serde_json::from_str(text.as_str()).map(Some).map_err(|error| {
        format!("Backend response is incompatible with this client: {error}")
    })
}

pub(super) fn encode_server_frame(frame: &ServerFrame) -> Result<String, String> {
    let json = serde_json::to_string(frame).map_err(|error| error.to_string())?;
    if json.len() <= MAX_SERVER_FRAME_BYTES {
        return Ok(json);
    }
    let message = format!("Remote response exceeds the {} MiB transport limit", MAX_SERVER_FRAME_BYTES / 1024 / 1024);
    match frame {
        ServerFrame::Response { id, .. } | ServerFrame::HistoryDownloadStart { id, .. } | ServerFrame::HistoryDownloadChunk { id, .. } => serde_json::to_string(&ServerFrame::Error {
            id: *id,
            message,
        }).map_err(|error| error.to_string()),
        _ => Err(message),
    }
}
