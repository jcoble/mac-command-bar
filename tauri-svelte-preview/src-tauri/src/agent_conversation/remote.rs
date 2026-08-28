//! One authenticated WebSocket boundary for conversations owned by the agent workbox.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message as TungsteniteMessage};

use super::manager::AgentRuntimeManager;
use super::prompt_content::prompt_from_blocks;
use super::protocol::{
    AgentCapabilities, AgentConfigOption, AgentConversationConfigState,
    AgentConversationConnection, AgentConversationEvent, AgentConversationEventPage,
    AgentConversationSessionRecord, AgentConversationSnapshot,
    ChangeAgentConversationCheckoutRequest, EnsureAgentConversationRequest, ExecutionEnvironment,
    RespondAgentConversationApprovalRequest, RespondAgentConversationInputRequest,
    RespondAgentConversationPermissionRequest, SendAgentConversationMessageRequest,
    SetAgentConversationConfigRequest, StopAgentConversationTurnRequest,
    UpdateAgentConversationSessionMetaRequest,
};
use super::providers::ProviderRegistry;

const PROTOCOL_VERSION: u16 = 1;
const MAX_WIRE_FRAME_BYTES: usize = 1024 * 1024;
const OUTBOUND_FRAME_CAPACITY: usize = 8;
const REPLAY_PAGE_BYTES: u32 = 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum ClientFrame {
    Request { id: u64, command: RemoteCommand },
    Cancel { id: u64 },
    Resume { cursors: BTreeMap<String, i64> },
    Ack { owned_id: String, sequence: i64 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command", content = "input", rename_all = "camelCase")]
enum RemoteCommand {
    ListSessions,
    Ensure(EnsureAgentConversationRequest),
    Snapshot {
        owned_id: String,
    },
    EventsBefore {
        owned_id: String,
        before_sequence: i64,
        max_bytes: u32,
    },
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
    SetConfigOption(super::SetAgentConversationConfigOptionRequest),
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
    Send(SendAgentConversationMessageRequest),
    RespondApproval(RespondAgentConversationApprovalRequest),
    RespondPermission(RespondAgentConversationPermissionRequest),
    RespondInput(RespondAgentConversationInputRequest),
    Stop(StopAgentConversationTurnRequest),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "result", content = "value", rename_all = "camelCase")]
enum RemoteResponse {
    Sessions(Vec<AgentConversationSessionRecord>),
    Connection(AgentConversationConnection),
    Snapshot(Option<AgentConversationSnapshot>),
    EventPage(AgentConversationEventPage),
    Capabilities(AgentCapabilities),
    Config(AgentConversationConfigState),
    ConfigOptions(Vec<AgentConfigOption>),
    Session(AgentConversationSessionRecord),
    OptionalString(Option<String>),
    Strings(Vec<String>),
    Bool(bool),
    Empty,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum ServerFrame {
    Ready { protocol_version: u16 },
    Response { id: u64, response: RemoteResponse },
    Error { id: u64, message: String },
    Event { event: AgentConversationEvent },
}

#[derive(Clone)]
struct ServerState {
    manager: AgentRuntimeManager,
    token: Arc<str>,
    events: broadcast::Sender<AgentConversationEvent>,
}

type ClientReply = oneshot::Sender<Result<RemoteResponse, String>>;

enum ClientRequest {
    Execute {
        id: u64,
        command: RemoteCommand,
        reply: ClientReply,
    },
    Cancel {
        id: u64,
    },
}

/// Owns the one workbox socket and the compact routing/cursor data needed to reconnect it.
#[derive(Clone, Default)]
pub struct RemoteConnectionManager {
    requests: Option<mpsc::Sender<ClientRequest>>,
    remote_sessions: Arc<Mutex<HashSet<String>>>,
    default_cwd: Option<String>,
    next_request_id: Arc<AtomicU64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkboxEnvironment {
    pub configured: bool,
    pub default_cwd: Option<String>,
}

impl RemoteConnectionManager {
    pub fn from_environment(
        event_sink: Arc<dyn Fn(AgentConversationEvent) + Send + Sync>,
    ) -> Result<Self, String> {
        let Some(url) = std::env::var("ASSEMBLY_AGENT_WORKBOX_WS_URL").ok() else {
            return Ok(Self::default());
        };
        let token = std::env::var("ASSEMBLY_AGENT_WORKBOX_TOKEN").map_err(|_| {
            "ASSEMBLY_AGENT_WORKBOX_TOKEN is required when a workbox URL is configured".to_string()
        })?;
        let default_cwd = std::env::var("ASSEMBLY_AGENT_WORKBOX_DEFAULT_CWD").map_err(|_| {
            "ASSEMBLY_AGENT_WORKBOX_DEFAULT_CWD is required when a workbox URL is configured"
                .to_string()
        })?;
        if !PathBuf::from(&default_cwd).is_absolute() {
            return Err("ASSEMBLY_AGENT_WORKBOX_DEFAULT_CWD must be an absolute path".to_string());
        }
        let (request_tx, request_rx) = mpsc::channel(32);
        let remote_sessions = Arc::new(Mutex::new(HashSet::new()));
        tauri::async_runtime::spawn(client_loop(url, token, request_rx, event_sink));
        Ok(Self {
            requests: Some(request_tx),
            remote_sessions,
            default_cwd: Some(default_cwd),
            next_request_id: Arc::new(AtomicU64::new(1)),
        })
    }

    pub fn is_configured(&self) -> bool {
        self.requests.is_some()
    }

    pub fn environment(&self) -> AgentWorkboxEnvironment {
        AgentWorkboxEnvironment {
            configured: self.is_configured(),
            default_cwd: self.default_cwd.clone(),
        }
    }

    pub fn owns(&self, owned_id: &str) -> bool {
        self.remote_sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains(owned_id)
    }

    fn remember(&self, owned_id: impl Into<String>) {
        self.remote_sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(owned_id.into());
    }

    async fn request(&self, command: RemoteCommand) -> Result<RemoteResponse, String> {
        let sender = self.requests.as_ref().ok_or_else(|| {
            "The Agent Workbox connection is not configured on this Mac".to_string()
        })?;
        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let (reply, answer) = oneshot::channel();
        sender
            .send(ClientRequest::Execute { id, command, reply })
            .await
            .map_err(|_| "The Agent Workbox connection task stopped".to_string())?;
        match tokio::time::timeout(Duration::from_secs(15), answer).await {
            Ok(answer) => {
                answer.map_err(|_| "The Agent Workbox disconnected before answering".to_string())?
            }
            Err(_) => {
                let _ = sender.send(ClientRequest::Cancel { id }).await;
                Err("The Agent Workbox did not answer within 15 seconds".to_string())
            }
        }
    }

    pub async fn list_sessions(&self) -> Result<Vec<AgentConversationSessionRecord>, String> {
        let RemoteResponse::Sessions(sessions) = self.request(RemoteCommand::ListSessions).await?
        else {
            return Err("The Agent Workbox returned the wrong list response".to_string());
        };
        for session in &sessions {
            self.remember(session.owned_id.clone());
        }
        Ok(sessions)
    }

    pub async fn ensure(
        &self,
        request: EnsureAgentConversationRequest,
    ) -> Result<AgentConversationConnection, String> {
        let owned_id = request.owned_id.clone();
        let RemoteResponse::Connection(connection) =
            self.request(RemoteCommand::Ensure(request)).await?
        else {
            return Err("The Agent Workbox returned the wrong ensure response".to_string());
        };
        self.remember(owned_id);
        Ok(connection)
    }

    pub async fn snapshot(
        &self,
        owned_id: String,
    ) -> Result<Option<AgentConversationSnapshot>, String> {
        let RemoteResponse::Snapshot(snapshot) =
            self.request(RemoteCommand::Snapshot { owned_id }).await?
        else {
            return Err("The Agent Workbox returned the wrong snapshot response".to_string());
        };
        Ok(snapshot)
    }

    pub async fn events_before(
        &self,
        owned_id: String,
        before_sequence: i64,
        max_bytes: u32,
    ) -> Result<AgentConversationEventPage, String> {
        let RemoteResponse::EventPage(page) = self
            .request(RemoteCommand::EventsBefore {
                owned_id,
                before_sequence,
                max_bytes,
            })
            .await?
        else {
            return Err("The Agent Workbox returned the wrong event-page response".to_string());
        };
        Ok(page)
    }

    pub async fn events_after(
        &self,
        owned_id: String,
        after_sequence: i64,
        max_bytes: u32,
    ) -> Result<AgentConversationEventPage, String> {
        let RemoteResponse::EventPage(page) = self
            .request(RemoteCommand::EventsAfter {
                owned_id,
                after_sequence,
                max_bytes,
            })
            .await?
        else {
            return Err("The Agent Workbox returned the wrong event-page response".to_string());
        };
        Ok(page)
    }

    pub async fn capabilities(&self, owned_id: String) -> Result<AgentCapabilities, String> {
        let RemoteResponse::Capabilities(capabilities) = self
            .request(RemoteCommand::Capabilities { owned_id })
            .await?
        else {
            return Err("The Agent Workbox returned the wrong capabilities response".to_string());
        };
        Ok(capabilities)
    }

    pub async fn config(&self, owned_id: String) -> Result<AgentConversationConfigState, String> {
        let RemoteResponse::Config(config) =
            self.request(RemoteCommand::Config { owned_id }).await?
        else {
            return Err("The Agent Workbox returned the wrong config response".to_string());
        };
        Ok(config)
    }

    pub async fn warm_config(
        &self,
        owned_id: String,
        generation: u64,
    ) -> Result<AgentConversationConfigState, String> {
        let RemoteResponse::Config(config) = self
            .request(RemoteCommand::WarmConfig {
                owned_id,
                generation,
            })
            .await?
        else {
            return Err("The Agent Workbox returned the wrong config response".to_string());
        };
        Ok(config)
    }

    pub async fn set_config(
        &self,
        request: SetAgentConversationConfigRequest,
    ) -> Result<AgentConversationConfigState, String> {
        let RemoteResponse::Config(config) =
            self.request(RemoteCommand::SetConfig(request)).await?
        else {
            return Err("The Agent Workbox returned the wrong config response".to_string());
        };
        Ok(config)
    }

    pub async fn set_config_option(
        &self,
        request: super::SetAgentConversationConfigOptionRequest,
    ) -> Result<Vec<AgentConfigOption>, String> {
        let RemoteResponse::ConfigOptions(options) = self
            .request(RemoteCommand::SetConfigOption(request))
            .await?
        else {
            return Err("The Agent Workbox returned the wrong config-option response".to_string());
        };
        Ok(options)
    }

    pub async fn set_draft(&self, owned_id: String, text: String) -> Result<(), String> {
        self.empty(RemoteCommand::SetDraft { owned_id, text }).await
    }

    pub async fn get_draft(&self, owned_id: String) -> Result<Option<String>, String> {
        let RemoteResponse::OptionalString(draft) =
            self.request(RemoteCommand::GetDraft { owned_id }).await?
        else {
            return Err("The Agent Workbox returned the wrong draft response".to_string());
        };
        Ok(draft)
    }

    pub async fn clear_draft(&self, owned_id: String) -> Result<(), String> {
        self.empty(RemoteCommand::ClearDraft { owned_id }).await
    }

    pub async fn write_workspace(
        &self,
        owned_id: String,
        snapshot_json: String,
    ) -> Result<(), String> {
        self.empty(RemoteCommand::WriteWorkspace {
            owned_id,
            snapshot_json,
        })
        .await
    }

    pub async fn read_workspace(&self, owned_id: String) -> Result<Option<String>, String> {
        let RemoteResponse::OptionalString(snapshot) = self
            .request(RemoteCommand::ReadWorkspace { owned_id })
            .await?
        else {
            return Err("The Agent Workbox returned the wrong workspace response".to_string());
        };
        Ok(snapshot)
    }

    pub async fn read_expanded_paths(
        &self,
        owned_id: String,
        root: String,
    ) -> Result<Vec<String>, String> {
        let RemoteResponse::Strings(paths) = self
            .request(RemoteCommand::ReadExpandedPaths { owned_id, root })
            .await?
        else {
            return Err("The Agent Workbox returned the wrong expanded-path response".to_string());
        };
        Ok(paths)
    }

    pub async fn write_expanded_paths(
        &self,
        owned_id: String,
        root: String,
        paths: Vec<String>,
    ) -> Result<(), String> {
        self.empty(RemoteCommand::WriteExpandedPaths {
            owned_id,
            root,
            paths,
        })
        .await
    }

    pub async fn delete_workspace(&self, owned_id: String) -> Result<(), String> {
        self.empty(RemoteCommand::DeleteWorkspace { owned_id })
            .await
    }

    pub async fn change_checkout(
        &self,
        request: ChangeAgentConversationCheckoutRequest,
    ) -> Result<AgentConversationSessionRecord, String> {
        let RemoteResponse::Session(session) =
            self.request(RemoteCommand::ChangeCheckout(request)).await?
        else {
            return Err("The Agent Workbox returned the wrong session response".to_string());
        };
        Ok(session)
    }

    pub async fn update_meta(
        &self,
        request: UpdateAgentConversationSessionMetaRequest,
    ) -> Result<AgentConversationSessionRecord, String> {
        let RemoteResponse::Session(session) =
            self.request(RemoteCommand::UpdateMeta(request)).await?
        else {
            return Err("The Agent Workbox returned the wrong session response".to_string());
        };
        Ok(session)
    }

    pub async fn close(&self, owned_id: String, generation: u64) -> Result<bool, String> {
        let RemoteResponse::Bool(closed) = self
            .request(RemoteCommand::Close {
                owned_id,
                generation,
            })
            .await?
        else {
            return Err("The Agent Workbox returned the wrong close response".to_string());
        };
        Ok(closed)
    }

    pub async fn delete(&self, owned_id: String) -> Result<bool, String> {
        let RemoteResponse::Bool(deleted) = self
            .request(RemoteCommand::Delete {
                owned_id: owned_id.clone(),
            })
            .await?
        else {
            return Err("The Agent Workbox returned the wrong delete response".to_string());
        };
        if deleted {
            self.remote_sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&owned_id);
        }
        Ok(deleted)
    }

    async fn empty(&self, command: RemoteCommand) -> Result<(), String> {
        match self.request(command).await? {
            RemoteResponse::Empty => Ok(()),
            _ => Err("The Agent Workbox returned the wrong command response".to_string()),
        }
    }

    pub async fn send(&self, request: SendAgentConversationMessageRequest) -> Result<(), String> {
        self.empty(RemoteCommand::Send(request)).await
    }

    pub async fn respond_approval(
        &self,
        request: RespondAgentConversationApprovalRequest,
    ) -> Result<(), String> {
        self.empty(RemoteCommand::RespondApproval(request)).await
    }

    pub async fn respond_permission(
        &self,
        request: RespondAgentConversationPermissionRequest,
    ) -> Result<(), String> {
        self.empty(RemoteCommand::RespondPermission(request)).await
    }

    pub async fn respond_input(
        &self,
        request: RespondAgentConversationInputRequest,
    ) -> Result<(), String> {
        self.empty(RemoteCommand::RespondInput(request)).await
    }

    pub async fn stop(&self, request: StopAgentConversationTurnRequest) -> Result<(), String> {
        self.empty(RemoteCommand::Stop(request)).await
    }
}

#[tauri::command]
pub fn read_agent_workbox_environment(
    remote: tauri::State<'_, RemoteConnectionManager>,
) -> AgentWorkboxEnvironment {
    remote.environment()
}

async fn client_loop(
    url: String,
    token: String,
    mut requests: mpsc::Receiver<ClientRequest>,
    event_sink: Arc<dyn Fn(AgentConversationEvent) + Send + Sync>,
) {
    let cursors = Arc::new(Mutex::new(BTreeMap::<String, i64>::new()));
    loop {
        let mut request = match url.as_str().into_client_request() {
            Ok(request) => request,
            Err(error) => {
                fail_queued_requests(&mut requests, format!("Invalid workbox URL: {error}"));
                return;
            }
        };
        let auth = format!("Bearer {token}");
        match auth.parse() {
            Ok(value) => {
                request.headers_mut().insert("authorization", value);
            }
            Err(error) => {
                fail_queued_requests(&mut requests, format!("Invalid workbox token: {error}"));
                return;
            }
        }
        let connected = tokio_tungstenite::connect_async(request).await;
        let Ok((mut socket, _)) = connected else {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        };
        let resume = ClientFrame::Resume {
            cursors: cursors
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
        };
        if send_client_frame(&mut socket, &resume).await.is_err() {
            continue;
        }
        let mut pending = HashMap::<u64, ClientReply>::new();
        loop {
            tokio::select! {
                request = requests.recv() => {
                    let Some(request) = request else {
                        return;
                    };
                    match request {
                        ClientRequest::Execute { id, command, reply } => {
                            let frame = ClientFrame::Request { id, command };
                            pending.insert(id, reply);
                            if let Err(error) = send_client_frame(&mut socket, &frame).await {
                                if let Some(reply) = pending.remove(&id) {
                                    let _ = reply.send(Err(error));
                                }
                                break;
                            }
                        }
                        ClientRequest::Cancel { id } => {
                            pending.remove(&id);
                            if send_client_frame(&mut socket, &ClientFrame::Cancel { id }).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                message = socket.next() => {
                    let Some(Ok(message)) = message else { break; };
                    let Some(frame) = parse_server_frame(message) else { continue; };
                    match frame {
                        ServerFrame::Ready { protocol_version } if protocol_version != PROTOCOL_VERSION => {
                            for (_, reply) in pending.drain() {
                                let _ = reply.send(Err(format!("Unsupported workbox protocol {protocol_version}")));
                            }
                            return;
                        }
                        ServerFrame::Ready { .. } => {}
                        ServerFrame::Response { id, response } => {
                            if let Some(reply) = pending.remove(&id) {
                                let _ = reply.send(Ok(response));
                            }
                        }
                        ServerFrame::Error { id, message } => {
                            if let Some(reply) = pending.remove(&id) {
                                let _ = reply.send(Err(message));
                            }
                        }
                        ServerFrame::Event { event } => {
                            let should_apply = {
                                let mut values = cursors.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                                let cursor = values.entry(event.owned_id.clone()).or_insert(i64::MIN);
                                if event.sequence <= *cursor { false } else { *cursor = event.sequence; true }
                            };
                            if should_apply {
                                event_sink(event.clone());
                                let _ = send_client_frame(&mut socket, &ClientFrame::Ack {
                                    owned_id: event.owned_id,
                                    sequence: event.sequence,
                                }).await;
                            }
                        }
                    }
                }
            }
        }
        for (_, reply) in pending.drain() {
            let _ = reply.send(Err(
                "The Agent Workbox connection was interrupted".to_string()
            ));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn fail_queued_requests(requests: &mut mpsc::Receiver<ClientRequest>, message: String) {
    while let Ok(request) = requests.try_recv() {
        if let ClientRequest::Execute { reply, .. } = request {
            let _ = reply.send(Err(message.clone()));
        }
    }
}

async fn send_client_frame<S>(socket: &mut S, frame: &ClientFrame) -> Result<(), String>
where
    S: futures_util::Sink<TungsteniteMessage> + Unpin,
    S::Error: std::fmt::Display,
{
    let json = serde_json::to_string(frame).map_err(|error| error.to_string())?;
    if json.len() > MAX_WIRE_FRAME_BYTES {
        return Err("Agent Workbox frame exceeds one MiB".to_string());
    }
    socket
        .send(TungsteniteMessage::Text(json.into()))
        .await
        .map_err(|error| error.to_string())
}

fn parse_server_frame(message: TungsteniteMessage) -> Option<ServerFrame> {
    let TungsteniteMessage::Text(text) = message else {
        return None;
    };
    serde_json::from_str(text.as_str()).ok()
}

/// Runs the resident workbox server instead of starting a Tauri window.
pub fn run_server_from_environment() -> Result<(), String> {
    let bind =
        std::env::var("ASSEMBLY_SERVER_BIND").unwrap_or_else(|_| "127.0.0.1:7777".to_string());
    let token = std::env::var("ASSEMBLY_SERVER_TOKEN")
        .map_err(|_| "ASSEMBLY_SERVER_TOKEN is required".to_string())?;
    if token.len() < 32 {
        return Err("ASSEMBLY_SERVER_TOKEN must contain at least 32 characters".to_string());
    }
    let data_dir = std::env::var_os("ASSEMBLY_SERVER_DATA_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| "ASSEMBLY_SERVER_DATA_DIR is required".to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|error| error.to_string())?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    runtime.block_on(async move {
        let manager = AgentRuntimeManager::open(
            ProviderRegistry::bundled_from_environment()?,
            &data_dir.join("sessions.db"),
        )?;
        let (events, _) = broadcast::channel(256);
        let event_sender = events.clone();
        manager.set_emitter(Arc::new(move |event| {
            let _ = event_sender.send(event);
        }));
        let state = ServerState {
            manager,
            token: Arc::from(token),
            events,
        };
        let app = Router::new()
            .route("/assembly", get(upgrade_workbox_socket))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind(&bind)
            .await
            .map_err(|error| error.to_string())?;
        println!("Assembly workbox server listening on {bind}");
        axum::serve(listener, app)
            .await
            .map_err(|error| error.to_string())
    })
}

async fn upgrade_workbox_socket(
    State(state): State<ServerState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    let expected = format!("Bearer {}", state.token);
    if headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        != Some(expected.as_str())
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    upgrade
        .max_message_size(MAX_WIRE_FRAME_BYTES)
        .on_upgrade(move |socket| serve_workbox_socket(socket, state))
        .into_response()
}

async fn serve_workbox_socket(socket: WebSocket, state: ServerState) {
    let (mut writer, mut reader) = socket.split();
    let (outbound, mut outgoing) = mpsc::channel::<ServerFrame>(OUTBOUND_FRAME_CAPACITY);
    let writer_task = tokio::spawn(async move {
        while let Some(frame) = outgoing.recv().await {
            let Ok(json) = serde_json::to_string(&frame) else {
                continue;
            };
            if json.len() > MAX_WIRE_FRAME_BYTES {
                continue;
            }
            if writer.send(AxumMessage::Text(json.into())).await.is_err() {
                break;
            }
        }
    });
    let _ = outbound
        .send(ServerFrame::Ready {
            protocol_version: PROTOCOL_VERSION,
        })
        .await;
    let mut live_events = state.events.subscribe();
    let (completed, mut completions) = mpsc::channel::<(u64, Result<RemoteResponse, String>)>(32);
    let mut request_tasks = HashMap::new();
    loop {
        let message = tokio::select! {
            message = reader.next() => {
                let Some(Ok(message)) = message else { break; };
                Some(message)
            }
            event = live_events.recv() => {
                match event {
                    Ok(event) => {
                        if outbound.send(ServerFrame::Event { event }).await.is_err() { break; }
                    }
                    // Dropped broadcast frames are recoverable from SQLite, but only
                    // through the ordered resume handshake. Closing this socket makes
                    // the client reconnect with its last acknowledged sequence.
                    Err(broadcast::error::RecvError::Lagged(_)) => break,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
                None
            }
            completion = completions.recv() => {
                let Some((id, response)) = completion else { break; };
                request_tasks.remove(&id);
                let frame = match response {
                    Ok(response) => ServerFrame::Response { id, response },
                    Err(message) => ServerFrame::Error { id, message },
                };
                if outbound.send(frame).await.is_err() { break; }
                None
            }
        };
        let Some(message) = message else {
            continue;
        };
        let AxumMessage::Text(text) = message else {
            continue;
        };
        let Ok(frame) = serde_json::from_str::<ClientFrame>(text.as_str()) else {
            continue;
        };
        match frame {
            ClientFrame::Request { id, command } => {
                let manager = state.manager.clone();
                let completion_sink = completed.clone();
                let task = tokio::spawn(async move {
                    let response = execute_remote_command(&manager, command).await;
                    let _ = completion_sink.send((id, response)).await;
                });
                if let Some(previous) = request_tasks.insert(id, task.abort_handle()) {
                    previous.abort();
                }
            }
            ClientFrame::Cancel { id } => {
                if let Some(task) = request_tasks.remove(&id) {
                    task.abort();
                }
            }
            ClientFrame::Resume { cursors } => {
                for (owned_id, mut sequence) in cursors {
                    loop {
                        let Ok(page) =
                            state
                                .manager
                                .list_events_after(&owned_id, sequence, REPLAY_PAGE_BYTES)
                        else {
                            break;
                        };
                        if page.events.is_empty() {
                            break;
                        }
                        for event in page.events {
                            sequence = sequence.max(event.sequence);
                            if outbound.send(ServerFrame::Event { event }).await.is_err() {
                                break;
                            }
                        }
                        if !page.has_more {
                            break;
                        }
                    }
                }
            }
            ClientFrame::Ack { .. } => {}
        }
    }
    for (_, task) in request_tasks.drain() {
        task.abort();
    }
    drop(outbound);
    let _ = writer_task.await;
}

async fn execute_remote_command(
    manager: &AgentRuntimeManager,
    command: RemoteCommand,
) -> Result<RemoteResponse, String> {
    match command {
        RemoteCommand::ListSessions => {
            let mut sessions = manager.list_sessions()?;
            for session in &mut sessions {
                session.execution_environment = ExecutionEnvironment::AgentWorkbox;
            }
            Ok(RemoteResponse::Sessions(sessions))
        }
        RemoteCommand::Ensure(mut request) => {
            request.execution_environment = ExecutionEnvironment::Local;
            manager
                .ensure_async(request)
                .await
                .map(RemoteResponse::Connection)
        }
        RemoteCommand::Snapshot { owned_id } => {
            manager.snapshot(&owned_id).map(RemoteResponse::Snapshot)
        }
        RemoteCommand::EventsBefore {
            owned_id,
            before_sequence,
            max_bytes,
        } => manager
            .list_events_before(&owned_id, before_sequence, max_bytes)
            .map(RemoteResponse::EventPage),
        RemoteCommand::EventsAfter {
            owned_id,
            after_sequence,
            max_bytes,
        } => manager
            .list_events_after(&owned_id, after_sequence, max_bytes)
            .map(RemoteResponse::EventPage),
        RemoteCommand::Capabilities { owned_id } => manager
            .capabilities_for_owned_id(&owned_id)
            .map(RemoteResponse::Capabilities),
        RemoteCommand::Config { owned_id } => manager
            .conversation_config(&owned_id)
            .map(RemoteResponse::Config),
        RemoteCommand::WarmConfig {
            owned_id,
            generation,
        } => manager
            .warm_conversation_config(&owned_id, generation)
            .await
            .map(RemoteResponse::Config),
        RemoteCommand::SetConfig(request) => manager
            .set_conversation_config(request)
            .await
            .map(RemoteResponse::Config),
        RemoteCommand::SetConfigOption(request) => manager
            .set_config(
                &request.owned_id,
                request.generation,
                &request.option_id,
                request.value,
            )
            .await
            .map(RemoteResponse::ConfigOptions),
        RemoteCommand::SetDraft { owned_id, text } => {
            manager.set_session_draft(&owned_id, &text)?;
            Ok(RemoteResponse::Empty)
        }
        RemoteCommand::GetDraft { owned_id } => manager
            .get_session_draft(&owned_id)
            .map(RemoteResponse::OptionalString),
        RemoteCommand::ClearDraft { owned_id } => {
            manager.clear_session_draft(&owned_id)?;
            Ok(RemoteResponse::Empty)
        }
        RemoteCommand::WriteWorkspace {
            owned_id,
            snapshot_json,
        } => {
            manager.write_workspace(&owned_id, &snapshot_json)?;
            Ok(RemoteResponse::Empty)
        }
        RemoteCommand::ReadWorkspace { owned_id } => manager
            .read_workspace(&owned_id)
            .map(RemoteResponse::OptionalString),
        RemoteCommand::ReadExpandedPaths { owned_id, root } => manager
            .read_workspace_expanded_paths(&owned_id, &root)
            .map(RemoteResponse::Strings),
        RemoteCommand::WriteExpandedPaths {
            owned_id,
            root,
            paths,
        } => {
            manager.write_workspace_expanded_paths(&owned_id, &root, &paths)?;
            Ok(RemoteResponse::Empty)
        }
        RemoteCommand::DeleteWorkspace { owned_id } => {
            manager.delete_workspace(&owned_id)?;
            Ok(RemoteResponse::Empty)
        }
        RemoteCommand::ChangeCheckout(request) => {
            manager.change_checkout(request).await.map(|mut session| {
                session.execution_environment = ExecutionEnvironment::AgentWorkbox;
                RemoteResponse::Session(session)
            })
        }
        RemoteCommand::UpdateMeta(request) => {
            manager.update_session_meta(request).map(|mut session| {
                session.execution_environment = ExecutionEnvironment::AgentWorkbox;
                RemoteResponse::Session(session)
            })
        }
        RemoteCommand::Close {
            owned_id,
            generation,
        } => manager
            .close(&owned_id, generation)
            .await
            .map(RemoteResponse::Bool),
        RemoteCommand::Delete { owned_id } => {
            manager.delete(&owned_id).await.map(RemoteResponse::Bool)
        }
        RemoteCommand::Send(request) => {
            if !request.attachment_ids.is_empty() {
                return Err("Remote attachments are not available in this milestone".to_string());
            }
            let prompt = prompt_from_blocks(request.text.trim(), request.content)?;
            manager
                .send_message(
                    &request.owned_id,
                    request.generation,
                    prompt,
                    request.model,
                    request.approval_policy,
                )
                .await?;
            Ok(RemoteResponse::Empty)
        }
        RemoteCommand::RespondApproval(request) => {
            manager
                .respond_legacy_approval(
                    &request.owned_id,
                    request.generation,
                    request.request_id,
                    request.decision,
                )
                .await?;
            Ok(RemoteResponse::Empty)
        }
        RemoteCommand::RespondPermission(request) => {
            manager
                .respond_permission_option(
                    &request.owned_id,
                    request.generation,
                    request.request_id,
                    request.option_id,
                )
                .await?;
            Ok(RemoteResponse::Empty)
        }
        RemoteCommand::RespondInput(request) => {
            manager
                .respond_user_input(super::protocol::AgentUserInputResponse {
                    identity: super::protocol::AgentRequestIdentity {
                        owned_id: request.owned_id,
                        generation: request.generation,
                        request_id: request.request_id,
                        turn_id: None,
                        item_id: None,
                    },
                    values: request.values,
                    cancelled: request.cancelled,
                })
                .await?;
            Ok(RemoteResponse::Empty)
        }
        RemoteCommand::Stop(request) => {
            manager
                .cancel_turn(&request.owned_id, request.generation)
                .await?;
            Ok(RemoteResponse::Empty)
        }
    }
}
