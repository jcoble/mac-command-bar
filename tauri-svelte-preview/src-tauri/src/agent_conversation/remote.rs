//! One authenticated WebSocket boundary for conversations owned by a remote machine.

use std::collections::{BTreeMap, HashMap};
use std::net::{TcpListener as StdTcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::{future::join_all, SinkExt, StreamExt};
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
pub const REMOTE_ASSEMBLY_PROFILE_SETTING_KEY: &str = "remote-assembly.profile.v1";
pub const REMOTE_ASSEMBLY_PROFILES_SETTING_KEY: &str = "remote-assembly.profiles.v1";
const REMOTE_SERVER_PORT: u16 = 7777;
const READINESS_REQUEST_ID: u64 = u64::MAX;

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
    Workspace { operation: String, args: serde_json::Value },
    ListSessions,
    Ensure(EnsureAgentConversationRequest),
    Snapshot {
        owned_id: String,
        request_id: u64,
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
    Workspace(serde_json::Value),
    Sessions(Vec<AgentConversationSessionRecord>),
    Connection(AgentConversationConnection),
    Snapshot(#[serde(deserialize_with = "deserialize_wire_payload")] Option<AgentConversationSnapshot>),
    EventPage(#[serde(deserialize_with = "deserialize_wire_payload")] AgentConversationEventPage),
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
    Event { #[serde(deserialize_with = "deserialize_wire_payload")] event: AgentConversationEvent },
}

// Serde's buffered tagged-enum decoder cannot deserialize u128 timestamps.
// Decode these existing payloads through JSON's numeric decoder; wire shape stays unchanged.
fn deserialize_wire_payload<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where D: serde::Deserializer<'de>, T: serde::de::DeserializeOwned {
    let value = serde_json::Value::deserialize(deserializer)?;
    serde_json::from_value(value).map_err(serde::de::Error::custom)
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

/// Owns the remote socket, SSH tunnels, and compact routing data needed to reconnect.
#[derive(Clone)]
pub struct RemoteConnectionManager {
    client: Arc<Mutex<RemoteClientState>>,
    tunnel_processes: Arc<Mutex<HashMap<u32, Child>>>,
    remote_sessions: Arc<Mutex<HashMap<String, String>>>,
    next_request_id: Arc<AtomicU64>,
    event_sink: Arc<dyn Fn(AgentConversationEvent) + Send + Sync>,
    connection_lock: Arc<tokio::sync::Mutex<()>>,
    attempts: Arc<Mutex<HashMap<String, tokio::task::AbortHandle>>>,
}

#[derive(Default)]
struct RemoteClientState {
    clients: HashMap<String, RemoteClient>,
}

struct RemoteClient {
    requests: Option<mpsc::Sender<ClientRequest>>,
    profile: Option<RemoteAssemblyProfile>,
    target_key: Option<String>,
    task: Option<tokio::task::AbortHandle>,
    ready: Arc<AtomicBool>,
}

impl Drop for RemoteClient {
    fn drop(&mut self) {
        if let Some(task) = &self.task { task.abort(); }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteConnectionResult {
    profile: RemoteAssemblyProfile,
    sessions: Vec<AgentConversationSessionRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAssemblyProfile {
    pub id: String,
    pub name: String,
    pub ssh_target: String,
    pub source_root: String,
    pub default_cwd: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyRemoteAssemblyProfile {
    ssh_target: String,
    source_root: String,
    default_cwd: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAssemblyEnvironment {
    pub profiles: Vec<RemoteAssemblyProfile>,
    pub ready_profile_ids: Vec<String>,
}

impl RemoteConnectionManager {
    pub fn from_environment(
        event_sink: Arc<dyn Fn(AgentConversationEvent) + Send + Sync>,
    ) -> Result<Self, String> {
        let manager = Self {
            client: Arc::new(Mutex::new(RemoteClientState::default())),
            tunnel_processes: Arc::new(Mutex::new(HashMap::new())),
            remote_sessions: Arc::new(Mutex::new(HashMap::new())),
            next_request_id: Arc::new(AtomicU64::new(1)),
            event_sink,
            connection_lock: Arc::new(tokio::sync::Mutex::new(())),
            attempts: Arc::new(Mutex::new(HashMap::new())),
        };
        let Some(url) = std::env::var("ASSEMBLY_REMOTE_WS_URL").ok() else {
            return Ok(manager);
        };
        let token = std::env::var("ASSEMBLY_REMOTE_TOKEN").map_err(|_| {
            "ASSEMBLY_REMOTE_TOKEN is required when a remote URL is configured".to_string()
        })?;
        let default_cwd = std::env::var("ASSEMBLY_REMOTE_DEFAULT_CWD").map_err(|_| {
            "ASSEMBLY_REMOTE_DEFAULT_CWD is required when a remote URL is configured".to_string()
        })?;
        if !PathBuf::from(&default_cwd).is_absolute() {
            return Err("ASSEMBLY_REMOTE_DEFAULT_CWD must be an absolute path".to_string());
        }
        let (request_tx, mut request_rx) = mpsc::channel(32);
        let event_sink = manager.event_sink.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let actor_ready = ready.clone();
        manager
            .client
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clients
            .insert(
                "environment".to_string(),
                RemoteClient {
                    requests: Some(request_tx),
                    profile: None,
                    target_key: None,
                    task: None,
                    ready,
                },
            );
        tauri::async_runtime::spawn(async move {
            client_loop(url, token, &mut request_rx, event_sink, None, actor_ready, &mut None).await;
        });
        Ok(manager)
    }

    pub fn is_configured(&self) -> bool {
        self.client
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clients
            .values()
            .any(|client| client.requests.is_some())
    }

    pub fn environment(&self) -> RemoteAssemblyEnvironment {
        let mut profiles = self
            .client
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clients
            .values()
            .filter_map(|client| client.profile.clone())
            .collect::<Vec<_>>();
        profiles.sort_by(|left, right| left.name.cmp(&right.name));
        let ready_profile_ids = self.client.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
            .clients.iter().filter(|(_, client)| client.ready.load(Ordering::Acquire)
                && client.requests.as_ref().is_some_and(|sender| !sender.is_closed()))
            .map(|(id, _)| id.clone()).collect();
        RemoteAssemblyEnvironment { profiles, ready_profile_ids }
    }

    pub fn shutdown(&self) {
        for (_, attempt) in self.attempts.lock().unwrap_or_else(std::sync::PoisonError::into_inner).drain() {
            attempt.abort();
        }
        self.client
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clients
            .clear();
        let mut tunnels = self
            .tunnel_processes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for (_, mut child) in tunnels.drain() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn restore_profiles(&self, profiles_json: &str) -> Result<(), String> {
        let profiles: Vec<RemoteAssemblyProfile> = serde_json::from_str(profiles_json)
            .map_err(|error| format!("Invalid Remote Assembly profiles: {error}"))?;
        for profile in profiles {
            self.register_profile(profile)?;
        }
        Ok(())
    }

    pub fn migrate_legacy_profile(
        &self,
        profile_json: &str,
    ) -> Result<Vec<RemoteAssemblyProfile>, String> {
        let legacy: LegacyRemoteAssemblyProfile = serde_json::from_str(profile_json)
            .map_err(|error| format!("Invalid legacy Remote Assembly profile: {error}"))?;
        let profile = RemoteAssemblyProfile {
            id: "agent-workbox".to_string(),
            name: "Agent Workbox".to_string(),
            ssh_target: legacy.ssh_target,
            source_root: legacy.source_root,
            default_cwd: legacy.default_cwd,
        };
        self.register_profile(profile.clone())?;
        Ok(vec![profile])
    }

    fn register_profile(&self, profile: RemoteAssemblyProfile) -> Result<(), String> {
        validate_profile(&profile)?;
        self.client.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clients.insert(
            profile.id.clone(), RemoteClient {
                requests: None, profile: Some(profile), target_key: None, task: None,
                ready: Arc::new(AtomicBool::new(false)),
            });
        Ok(())
    }

    pub fn cancel_connection(&self, operation_id: &str) {
        if let Some(task) = self.attempts.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(operation_id) {
            task.abort();
        }
    }

    pub async fn connect_profile(&self, profile: RemoteAssemblyProfile, operation_id: String,
        status: tauri::ipc::Channel<String>) -> Result<RemoteConnectionResult, String> {
        validate_profile(&profile)?;
        let manager = self.clone();
        let progress = status.clone();
        let mut task = {
            let mut attempts = self.attempts.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if attempts.contains_key(&operation_id) { return Err("Connection attempt already exists".into()); }
            let task = tokio::spawn(async move { manager.connect_profile_inner(profile, progress).await });
            attempts.insert(operation_id.clone(), task.abort_handle());
            task
        };
        // The UI enables cancellation only after this registration acknowledgment.
        let _ = status.send("Connecting…".into());
        let result = match tokio::time::timeout(Duration::from_secs(30), &mut task).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("Connection cancelled".into()),
            Err(_) => {
                task.abort();
                let _ = task.await;
                Err("Connection timed out after 30 seconds".into())
            }
        };
        self.attempts.lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&operation_id);
        result
    }

    async fn connect_profile_inner(&self, mut profile: RemoteAssemblyProfile,
        status: tauri::ipc::Channel<String>) -> Result<RemoteConnectionResult, String> {
        let _owner = self.connection_lock.lock().await;
        let _ = status.send("Resolving SSH configuration…".into());
        let target_key = resolve_ssh_target(&profile.ssh_target).await?;
        let saved = self.environment().profiles;
        for existing in saved {
            let key = if existing.ssh_target == profile.ssh_target { Some(target_key.clone()) }
                else { resolve_ssh_target(&existing.ssh_target).await.ok() };
            if key.as_ref() != Some(&target_key) { continue; }
            // Alias and address forms of the same host keep the original profile/session IDs.
            if existing.id != profile.id { profile = existing; }
            break;
        }
        let reusable = self.client.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
            .clients.get(&profile.id).is_some_and(|client| client.target_key.as_ref() == Some(&target_key)
                && client.requests.as_ref().is_some_and(|sender| !sender.is_closed()));
        let mut sessions = if reusable {
            let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
            let RemoteResponse::Sessions(sessions) = self.request_with_id(&profile.id, id, RemoteCommand::ListSessions).await?
                else { return Err("Backend returned an invalid session list".into()); };
            sessions
        } else {
            let _ = status.send("Checking the installed backend…".into());
            let (requests, receiver) = mpsc::channel(32);
            let (ready_tx, ready_rx) = oneshot::channel();
            let ready = Arc::new(AtomicBool::new(false));
            let task = tokio::spawn(profile_client_loop(profile.clone(), receiver, self.event_sink.clone(),
                self.tunnel_processes.clone(), ready.clone(), Some(ready_tx)));
            // Dropping a failed/cancelled candidate aborts only its own actor and tunnel.
            let candidate = RemoteClient { requests: Some(requests), profile: Some(profile.clone()),
                target_key: Some(target_key), task: Some(task.abort_handle()), ready };
            let sessions = ready_rx.await.map_err(|_| "Backend connection ended before readiness".to_string())??;
            self.client.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clients.insert(profile.id.clone(), candidate);
            sessions
        };
        for session in &mut sessions {
            session.remote_profile_id = Some(profile.id.clone());
            self.remember(session.owned_id.clone(), profile.id.clone());
        }
        if let Some(client) = self.client.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clients.get_mut(&profile.id) {
            client.profile = Some(profile.clone());
        }
        Ok(RemoteConnectionResult { profile, sessions })
    }

    pub fn disconnect_profile(&self, profile_id: &str) {
        let mut state = self.client.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(client) = state.clients.get_mut(profile_id) {
            if let Some(task) = client.task.take() { task.abort(); }
            client.requests = None;
            client.target_key = None;
            client.ready.store(false, Ordering::Release);
        }
    }

    pub fn remove_profile(&self, profile_id: &str) {
        self.client
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clients
            .remove(profile_id);
    }

    pub fn owns(&self, owned_id: &str) -> bool {
        self.remote_sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains_key(owned_id)
    }

    fn remember(&self, owned_id: impl Into<String>, profile_id: impl Into<String>) {
        self.remote_sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(owned_id.into(), profile_id.into());
    }

    fn profile_for_owned_id(&self, owned_id: &str) -> Result<String, String> {
        self.remote_sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(owned_id)
            .cloned()
            .ok_or_else(|| format!("No remote machine owns session {owned_id}"))
    }

    async fn request_for_owned(
        &self,
        owned_id: &str,
        command: RemoteCommand,
    ) -> Result<RemoteResponse, String> {
        let profile_id = self.profile_for_owned_id(owned_id)?;
        self.request_for_profile(&profile_id, command).await
    }

    async fn request_for_profile(
        &self,
        profile_id: &str,
        command: RemoteCommand,
    ) -> Result<RemoteResponse, String> {
        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        self.request_with_id(profile_id, id, command).await
    }

    async fn request_with_id(
        &self,
        profile_id: &str,
        id: u64,
        command: RemoteCommand,
    ) -> Result<RemoteResponse, String> {
        let sender = self
            .client
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clients
            .get(profile_id)
            .and_then(|client| client.requests.as_ref())
            .cloned()
            .ok_or_else(|| format!("Remote machine {profile_id} is not connected"))?;
        let (reply, answer) = oneshot::channel();
        sender
            .try_send(ClientRequest::Execute { id, command, reply })
            .map_err(|error| {
                format!("The Remote Assembly request queue is unavailable: {error}")
            })?;
        match tokio::time::timeout(Duration::from_secs(15), answer).await {
            Ok(answer) => answer.map_err(|_| {
                "The Remote Assembly connection closed before answering".to_string()
            })?,
            Err(_) => {
                let _ = sender.try_send(ClientRequest::Cancel { id });
                Err("The remote machine did not answer within 15 seconds".to_string())
            }
        }
    }

    pub async fn list_sessions(
        &self,
        request_id: u64,
    ) -> Result<Vec<AgentConversationSessionRecord>, String> {
        let profile_ids = self
            .client
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clients
            .iter()
            .filter(|(_, client)| client.requests.as_ref().is_some_and(|sender| !sender.is_closed()))
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        let responses = join_all(profile_ids.into_iter().map(|profile_id| async move {
            let response = self
                .request_with_id(&profile_id, request_id, RemoteCommand::ListSessions)
                .await;
            (profile_id, response)
        }))
        .await;
        let mut all_sessions = Vec::new();
        let mut successful_profiles = 0;
        let mut first_error = None;
        for (profile_id, response) in responses {
            let mut sessions = match response {
                Ok(RemoteResponse::Sessions(sessions)) => {
                    successful_profiles += 1;
                    sessions
                }
                Ok(_) => {
                    first_error.get_or_insert_with(|| {
                        "Remote Assembly returned the wrong list response".to_string()
                    });
                    continue;
                }
                Err(error) => {
                    first_error.get_or_insert(error);
                    continue;
                }
            };
            for session in &mut sessions {
                session.remote_profile_id = Some(profile_id.clone());
                self.remember(session.owned_id.clone(), profile_id.clone());
            }
            all_sessions.extend(sessions);
        }
        if successful_profiles > 0 || first_error.is_none() {
            Ok(all_sessions)
        } else {
            Err(first_error.unwrap_or_else(|| "No remote machines are connected".to_string()))
        }
    }

    pub async fn ensure(
        &self,
        request: EnsureAgentConversationRequest,
    ) -> Result<AgentConversationConnection, String> {
        let owned_id = request.owned_id.clone();
        let profile_id = request
            .remote_profile_id
            .clone()
            .or_else(|| {
                self.remote_sessions
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get(&owned_id)
                    .cloned()
            })
            .ok_or_else(|| "Choose a remote machine for this session".to_string())?;
        let RemoteResponse::Connection(connection) = self
            .request_for_profile(&profile_id, RemoteCommand::Ensure(request))
            .await?
        else {
            return Err("Remote Assembly returned the wrong ensure response".to_string());
        };
        self.remember(owned_id, profile_id);
        Ok(connection)
    }

    pub async fn snapshot(
        &self,
        owned_id: String,
        request_id: u64,
    ) -> Result<Option<AgentConversationSnapshot>, String> {
        let RemoteResponse::Snapshot(snapshot) = self
            .request_with_id(
                &self.profile_for_owned_id(&owned_id)?,
                request_id,
                RemoteCommand::Snapshot {
                    owned_id,
                    request_id,
                },
            )
            .await?
        else {
            return Err("Remote Assembly returned the wrong snapshot response".to_string());
        };
        Ok(snapshot)
    }

    pub async fn cancel_request(&self, request_id: u64) {
        let senders = self
            .client
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clients
            .values()
            .filter_map(|client| client.requests.clone())
            .collect::<Vec<_>>();
        for sender in senders {
            let _ = sender.try_send(ClientRequest::Cancel { id: request_id });
        }
    }

    pub async fn events_before(
        &self,
        owned_id: String,
        before_sequence: i64,
        max_bytes: u32,
        request_id: u64,
    ) -> Result<AgentConversationEventPage, String> {
        let RemoteResponse::EventPage(page) = self
            .request_with_id(
                &self.profile_for_owned_id(&owned_id)?,
                request_id,
                RemoteCommand::EventsBefore {
                    owned_id,
                    before_sequence,
                    max_bytes,
                },
            )
            .await?
        else {
            return Err("Remote Assembly returned the wrong event-page response".to_string());
        };
        Ok(page)
    }

    pub async fn events_after(
        &self,
        owned_id: String,
        after_sequence: i64,
        max_bytes: u32,
        request_id: u64,
    ) -> Result<AgentConversationEventPage, String> {
        let RemoteResponse::EventPage(page) = self
            .request_with_id(
                &self.profile_for_owned_id(&owned_id)?,
                request_id,
                RemoteCommand::EventsAfter {
                    owned_id,
                    after_sequence,
                    max_bytes,
                },
            )
            .await?
        else {
            return Err("Remote Assembly returned the wrong event-page response".to_string());
        };
        Ok(page)
    }

    pub async fn capabilities(
        &self,
        owned_id: String,
        request_id: u64,
    ) -> Result<AgentCapabilities, String> {
        let RemoteResponse::Capabilities(capabilities) = self
            .request_with_id(
                &self.profile_for_owned_id(&owned_id)?,
                request_id,
                RemoteCommand::Capabilities { owned_id },
            )
            .await?
        else {
            return Err("Remote Assembly returned the wrong capabilities response".to_string());
        };
        Ok(capabilities)
    }

    pub async fn config(
        &self,
        owned_id: String,
        request_id: u64,
    ) -> Result<AgentConversationConfigState, String> {
        let RemoteResponse::Config(config) = self
            .request_with_id(
                &self.profile_for_owned_id(&owned_id)?,
                request_id,
                RemoteCommand::Config { owned_id },
            )
            .await?
        else {
            return Err("Remote Assembly returned the wrong config response".to_string());
        };
        Ok(config)
    }

    pub async fn warm_config(
        &self,
        owned_id: String,
        generation: u64,
    ) -> Result<AgentConversationConfigState, String> {
        let route = owned_id.clone();
        let RemoteResponse::Config(config) = self
            .request_for_owned(
                &route,
                RemoteCommand::WarmConfig {
                    owned_id,
                    generation,
                },
            )
            .await?
        else {
            return Err("Remote Assembly returned the wrong config response".to_string());
        };
        Ok(config)
    }

    pub async fn set_config(
        &self,
        request: SetAgentConversationConfigRequest,
    ) -> Result<AgentConversationConfigState, String> {
        let owned_id = request.owned_id.clone();
        let RemoteResponse::Config(config) = self
            .request_for_owned(&owned_id, RemoteCommand::SetConfig(request))
            .await?
        else {
            return Err("Remote Assembly returned the wrong config response".to_string());
        };
        Ok(config)
    }

    pub async fn set_config_option(
        &self,
        request: super::SetAgentConversationConfigOptionRequest,
    ) -> Result<Vec<AgentConfigOption>, String> {
        let owned_id = request.owned_id.clone();
        let RemoteResponse::ConfigOptions(options) = self
            .request_for_owned(&owned_id, RemoteCommand::SetConfigOption(request))
            .await?
        else {
            return Err("Remote Assembly returned the wrong config-option response".to_string());
        };
        Ok(options)
    }

    pub async fn set_draft(&self, owned_id: String, text: String) -> Result<(), String> {
        let route = owned_id.clone();
        self.empty_for_owned(&route, RemoteCommand::SetDraft { owned_id, text })
            .await
    }

    pub async fn get_draft(&self, owned_id: String) -> Result<Option<String>, String> {
        let route = owned_id.clone();
        let RemoteResponse::OptionalString(draft) = self
            .request_for_owned(&route, RemoteCommand::GetDraft { owned_id })
            .await?
        else {
            return Err("Remote Assembly returned the wrong draft response".to_string());
        };
        Ok(draft)
    }

    pub async fn clear_draft(&self, owned_id: String) -> Result<(), String> {
        let route = owned_id.clone();
        self.empty_for_owned(&route, RemoteCommand::ClearDraft { owned_id })
            .await
    }

    pub async fn write_workspace(
        &self,
        owned_id: String,
        snapshot_json: String,
    ) -> Result<(), String> {
        let route = owned_id.clone();
        self.empty_for_owned(
            &route,
            RemoteCommand::WriteWorkspace {
                owned_id,
                snapshot_json,
            },
        )
        .await
    }

    pub async fn read_workspace(&self, owned_id: String) -> Result<Option<String>, String> {
        let route = owned_id.clone();
        let RemoteResponse::OptionalString(snapshot) = self
            .request_for_owned(&route, RemoteCommand::ReadWorkspace { owned_id })
            .await?
        else {
            return Err("Remote Assembly returned the wrong workspace response".to_string());
        };
        Ok(snapshot)
    }

    pub async fn read_expanded_paths(
        &self,
        owned_id: String,
        root: String,
        request_id: u64,
    ) -> Result<Vec<String>, String> {
        let RemoteResponse::Strings(paths) = self
            .request_with_id(
                &self.profile_for_owned_id(&owned_id)?,
                request_id,
                RemoteCommand::ReadExpandedPaths { owned_id, root },
            )
            .await?
        else {
            return Err("Remote Assembly returned the wrong expanded-path response".to_string());
        };
        Ok(paths)
    }

    pub async fn write_expanded_paths(
        &self,
        owned_id: String,
        root: String,
        paths: Vec<String>,
    ) -> Result<(), String> {
        let route = owned_id.clone();
        self.empty_for_owned(
            &route,
            RemoteCommand::WriteExpandedPaths {
                owned_id,
                root,
                paths,
            },
        )
        .await
    }

    pub async fn delete_workspace(&self, owned_id: String) -> Result<(), String> {
        let route = owned_id.clone();
        self.empty_for_owned(&route, RemoteCommand::DeleteWorkspace { owned_id })
            .await
    }

    pub async fn change_checkout(
        &self,
        request: ChangeAgentConversationCheckoutRequest,
    ) -> Result<AgentConversationSessionRecord, String> {
        let owned_id = request.owned_id.clone();
        let profile_id = self.profile_for_owned_id(&owned_id)?;
        let RemoteResponse::Session(mut session) = self
            .request_for_owned(&owned_id, RemoteCommand::ChangeCheckout(request))
            .await?
        else {
            return Err("Remote Assembly returned the wrong session response".to_string());
        };
        session.remote_profile_id = Some(profile_id);
        Ok(session)
    }

    pub async fn update_meta(
        &self,
        request: UpdateAgentConversationSessionMetaRequest,
    ) -> Result<AgentConversationSessionRecord, String> {
        let owned_id = request.owned_id.clone();
        let profile_id = self.profile_for_owned_id(&owned_id)?;
        let RemoteResponse::Session(mut session) = self
            .request_for_owned(&owned_id, RemoteCommand::UpdateMeta(request))
            .await?
        else {
            return Err("Remote Assembly returned the wrong session response".to_string());
        };
        session.remote_profile_id = Some(profile_id);
        Ok(session)
    }

    pub async fn close(&self, owned_id: String, generation: u64) -> Result<bool, String> {
        let route = owned_id.clone();
        let RemoteResponse::Bool(closed) = self
            .request_for_owned(
                &route,
                RemoteCommand::Close {
                    owned_id,
                    generation,
                },
            )
            .await?
        else {
            return Err("Remote Assembly returned the wrong close response".to_string());
        };
        Ok(closed)
    }

    pub async fn delete(&self, owned_id: String) -> Result<bool, String> {
        let route = owned_id.clone();
        let RemoteResponse::Bool(deleted) = self
            .request_for_owned(
                &route,
                RemoteCommand::Delete {
                    owned_id: owned_id.clone(),
                },
            )
            .await?
        else {
            return Err("Remote Assembly returned the wrong delete response".to_string());
        };
        if deleted {
            self.remote_sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&owned_id);
        }
        Ok(deleted)
    }

    async fn empty_for_owned(&self, owned_id: &str, command: RemoteCommand) -> Result<(), String> {
        match self.request_for_owned(owned_id, command).await? {
            RemoteResponse::Empty => Ok(()),
            _ => Err("Remote Assembly returned the wrong command response".to_string()),
        }
    }

    pub async fn send(&self, request: SendAgentConversationMessageRequest) -> Result<(), String> {
        let owned_id = request.owned_id.clone();
        self.empty_for_owned(&owned_id, RemoteCommand::Send(request))
            .await
    }

    pub async fn respond_approval(
        &self,
        request: RespondAgentConversationApprovalRequest,
    ) -> Result<(), String> {
        let owned_id = request.owned_id.clone();
        self.empty_for_owned(&owned_id, RemoteCommand::RespondApproval(request))
            .await
    }

    pub async fn respond_permission(
        &self,
        request: RespondAgentConversationPermissionRequest,
    ) -> Result<(), String> {
        let owned_id = request.owned_id.clone();
        self.empty_for_owned(&owned_id, RemoteCommand::RespondPermission(request))
            .await
    }

    pub async fn respond_input(
        &self,
        request: RespondAgentConversationInputRequest,
    ) -> Result<(), String> {
        let owned_id = request.owned_id.clone();
        self.empty_for_owned(&owned_id, RemoteCommand::RespondInput(request))
            .await
    }

    pub async fn stop(&self, request: StopAgentConversationTurnRequest) -> Result<(), String> {
        let owned_id = request.owned_id.clone();
        self.empty_for_owned(&owned_id, RemoteCommand::Stop(request))
            .await
    }
}

#[tauri::command]
pub fn read_remote_assembly_environment(
    remote: tauri::State<'_, RemoteConnectionManager>,
) -> RemoteAssemblyEnvironment {
    remote.environment()
}

#[tauri::command]
pub async fn connect_remote_assembly(
    manager: tauri::State<'_, AgentRuntimeManager>, remote: tauri::State<'_, RemoteConnectionManager>,
    profile: RemoteAssemblyProfile, operation_id: String, status: tauri::ipc::Channel<String>,
) -> Result<RemoteConnectionResult, super::protocol::CommandError> {
    let result = remote.connect_profile(profile, operation_id, status).await
        .map_err(super::protocol::CommandError::from)?;
    let profiles_json = serde_json::to_string(&remote.environment().profiles)
        .map_err(|error| super::protocol::CommandError::from(error.to_string()))?;
    manager.write_app_setting(REMOTE_ASSEMBLY_PROFILES_SETTING_KEY, &profiles_json)
        .map_err(super::protocol::CommandError::from)?;
    Ok(result)
}

#[tauri::command]
pub fn cancel_remote_connection(remote: tauri::State<'_, RemoteConnectionManager>, operation_id: String) {
    remote.cancel_connection(&operation_id);
}

#[tauri::command]
pub fn disconnect_remote_assembly(
    remote: tauri::State<'_, RemoteConnectionManager>, profile_id: String,
) -> RemoteAssemblyEnvironment {
    remote.disconnect_profile(&profile_id);
    remote.environment()
}

#[tauri::command]
pub fn remove_remote_assembly_profile(
    manager: tauri::State<'_, AgentRuntimeManager>,
    remote: tauri::State<'_, RemoteConnectionManager>,
    profile_id: String,
) -> Result<RemoteAssemblyEnvironment, super::protocol::CommandError> {
    remote.remove_profile(&profile_id);
    let environment = remote.environment();
    let profiles_json = serde_json::to_string(&environment.profiles)
        .map_err(|error| super::protocol::CommandError::from(error.to_string()))?;
    manager
        .write_app_setting(REMOTE_ASSEMBLY_PROFILES_SETTING_KEY, &profiles_json)
        .map_err(super::protocol::CommandError::from)?;
    Ok(environment)
}

fn validate_profile(profile: &RemoteAssemblyProfile) -> Result<(), String> {
    if profile.id.trim().is_empty()
        || !profile
            .id
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || b"_-".contains(&value))
    {
        return Err(
            "Remote machine id may contain only letters, numbers, '_', and '-'".to_string(),
        );
    }
    if profile.name.trim().is_empty() {
        return Err("Remote machine name is required".to_string());
    }
    validate_ssh_target(&profile.ssh_target)?;
    Ok(())
}

async fn profile_client_loop(
    profile: RemoteAssemblyProfile,
    mut requests: mpsc::Receiver<ClientRequest>,
    event_sink: Arc<dyn Fn(AgentConversationEvent) + Send + Sync>,
    tunnel_processes: Arc<Mutex<HashMap<u32, Child>>>,
    ready: Arc<AtomicBool>,
    mut initial_ready: Option<oneshot::Sender<Result<Vec<AgentConversationSessionRecord>, String>>>,
) {
    while !requests.is_closed() {
        let target = profile.ssh_target.clone();
        let processes = tunnel_processes.clone();
        let tunnel = match open_remote_tunnel(&target, processes).await {
            Ok(tunnel) => tunnel,
            Err(error) => {
                if let Some(reply) = initial_ready.take() { let _ = reply.send(Err(error)); return; }
                fail_queued_requests(&mut requests, error);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };
        let url = format!("ws://127.0.0.1:{}/assembly", tunnel.local_port);
        let token = tunnel.token.clone();
        client_loop(url, token, &mut requests, event_sink.clone(), Some(4), ready.clone(), &mut initial_ready).await;
        drop(tunnel);
    }
}

struct RemoteTunnel {
    pid: u32,
    processes: Arc<Mutex<HashMap<u32, Child>>>,
    local_port: u16,
    token: String,
}

impl Drop for RemoteTunnel {
    fn drop(&mut self) {
        stop_remote_tunnel(&self.processes, self.pid);
    }
}

fn stop_remote_tunnel(processes: &Mutex<HashMap<u32, Child>>, pid: u32) {
    let child = processes
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&pid);
    if let Some(mut child) = child {
        let _ = child.kill();
        let _ = child.wait();
    }
}

async fn open_remote_tunnel(
    target: &str,
    processes: Arc<Mutex<HashMap<u32, Child>>>,
) -> Result<RemoteTunnel, String> {
    let token = read_remote_token(target).await?;
    let local_port = StdTcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map_err(|error| format!("Could not reserve a local Remote Assembly port: {error}"))?
        .port();
    let forwarding = format!("127.0.0.1:{local_port}:127.0.0.1:{REMOTE_SERVER_PORT}");
    let mut command = Command::new("ssh");
    command.args([
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=10",
        "-o",
        "ExitOnForwardFailure=yes",
        "-o", "ControlMaster=no", "-o", "ControlPath=none",
        "-o",
        "ServerAliveInterval=15",
        "-C",
        "-T",
        "-L",
        &forwarding,
        "--",
        target,
        "cat >/dev/null",
    ]);
    let child = command
        // Keep the remote command alive through this parent-owned pipe. If
        // Assembly is terminated before its shutdown hook runs, EOF closes
        // the remote command and the SSH forward exits instead of orphaning.
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not start the Remote Assembly tunnel: {error}"))?;
    let pid = child.id();
    processes
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(pid, child);
    let tunnel = RemoteTunnel { pid, processes: processes.clone(), local_port, token };
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", local_port)).is_ok() { return Ok(tunnel); }
        let status = {
            let mut tunnels = processes
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Some(child) = tunnels.get_mut(&pid) else {
                return Err("Remote Assembly tunnel was stopped".to_string());
            };
            child
                .try_wait()
                .map_err(|error| format!("Could not inspect the Remote Assembly tunnel: {error}"))?
        };
        if let Some(status) = status {
            processes
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&pid);
            return Err(format!("Remote Assembly tunnel exited with {status}"));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    stop_remote_tunnel(&processes, pid);
    Err("Remote Assembly tunnel did not become ready".to_string())
}

async fn ssh_output(target: &str, command: &str) -> Result<std::process::Output, String> {
    tokio::process::Command::new("ssh")
        .args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=10", "-o", "ControlMaster=no",
            "-o", "ControlPath=none", "--", target, command])
        .stdin(Stdio::null()).kill_on_drop(true).output().await
        .map_err(|error| format!("Could not start SSH: {error}"))
}

async fn read_remote_token(target: &str) -> Result<String, String> {
    let output = ssh_output(target, "test -f \"$HOME/.config/assembly/server.env\" || { echo Backend-not-installed >&2; exit 44; }; sed -n 's/^ASSEMBLY_SERVER_TOKEN=//p' \"$HOME/.config/assembly/server.env\"").await?;
    if !output.status.success() {
        if output.status.code() == Some(44) { return Err("Backend not installed on this SSH account".into()); }
        return Err(format!("SSH connection failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }
    let token = String::from_utf8_lossy(&output.stdout).lines().last().unwrap_or_default().trim().to_string();
    if token.len() < 32 { return Err("Installed backend authentication is missing or invalid".into()); }
    Ok(token)
}

fn ssh_target_key(output: &str) -> Result<String, String> {
    let value = |name: &str| output.lines().find_map(|line| line.strip_prefix(name));
    let (Some(host), Some(user), Some(port)) = (value("hostname "), value("user "), value("port "))
        else { return Err("SSH configuration did not resolve hostname, user and port".into()); };
    Ok(format!("{}@{}:{}", user.trim(), host.trim().to_lowercase(), port.trim()))
}

async fn resolve_ssh_target(target: &str) -> Result<String, String> {
    validate_ssh_target(target)?;
    let output = tokio::process::Command::new("ssh").args(["-G", "--", target])
        .stdin(Stdio::null()).kill_on_drop(true).output().await.map_err(|error| error.to_string())?;
    if !output.status.success() { return Err(format!("SSH configuration failed: {}", String::from_utf8_lossy(&output.stderr).trim())); }
    ssh_target_key(&String::from_utf8_lossy(&output.stdout))
}

async fn client_loop(
    url: String,
    token: String,
    requests: &mut mpsc::Receiver<ClientRequest>,
    event_sink: Arc<dyn Fn(AgentConversationEvent) + Send + Sync>,
    reconnect_tunnel_after_failures: Option<u8>,
    ready: Arc<AtomicBool>,
    initial_ready: &mut Option<oneshot::Sender<Result<Vec<AgentConversationSessionRecord>, String>>>,
) {
    let cursors = Arc::new(Mutex::new(BTreeMap::<String, i64>::new()));
    let mut failed_connections = 0_u8;
    loop {
        ready.store(false, Ordering::Release);
        let mut request = match url.as_str().into_client_request() {
            Ok(request) => request,
            Err(error) => {
                fail_queued_requests(requests, format!("Invalid remote URL: {error}"));
                return;
            }
        };
        let auth = format!("Bearer {token}");
        match auth.parse() {
            Ok(value) => {
                request.headers_mut().insert("authorization", value);
            }
            Err(error) => {
                fail_queued_requests(requests, format!("Invalid remote token: {error}"));
                return;
            }
        }
        let connected = tokio_tungstenite::connect_async_with_config(request, None, true).await;
        let (mut socket, _) = match connected {
            Ok(socket) => socket,
            Err(error) => {
                if let Some(reply) = initial_ready.take() {
                    let _ = reply.send(Err(format!("Backend connection/authentication failed: {error}")));
                    return;
                }
            failed_connections = failed_connections.saturating_add(1);
            if reconnect_tunnel_after_failures.is_some_and(|limit| failed_connections >= limit) {
                return;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
            }
        };
        failed_connections = 0;
        let handshake = tokio::time::timeout(Duration::from_secs(15), async {
            let hello = socket.next().await.ok_or("Backend closed before protocol readiness")?
                .map_err(|error| error.to_string())?;
            match parse_server_frame(hello)? {
                Some(ServerFrame::Ready { protocol_version }) if protocol_version == PROTOCOL_VERSION => Ok(()),
                Some(ServerFrame::Ready { protocol_version }) => Err(format!("Unsupported remote protocol {protocol_version}; expected {PROTOCOL_VERSION}")),
                _ => Err("Backend did not announce protocol readiness".into()),
            }
        }).await.unwrap_or_else(|_| Err("Backend readiness timed out".into()));
        if let Err(error) = handshake {
            if let Some(reply) = initial_ready.take() { let _ = reply.send(Err(error.clone())); }
            fail_queued_requests(requests, error);
            return;
        }
        let resume = ClientFrame::Resume {
            cursors: cursors
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
        };
        if send_client_frame(&mut socket, &resume).await.is_err() {
            continue;
        }
        if send_client_frame(&mut socket, &ClientFrame::Request { id: READINESS_REQUEST_ID, command: RemoteCommand::ListSessions }).await.is_err() { return; }
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
                    let frame = match parse_server_frame(message) {
                        Ok(Some(frame)) => frame,
                        Ok(None) => continue,
                        Err(error) => {
                            if let Some(reply) = initial_ready.take() { let _ = reply.send(Err(error.clone())); }
                            for (_, reply) in pending.drain() { let _ = reply.send(Err(error.clone())); }
                            return;
                        }
                    };
                    match frame {
                        ServerFrame::Ready { protocol_version } if protocol_version != PROTOCOL_VERSION => {
                            for (_, reply) in pending.drain() {
                                let _ = reply.send(Err(format!("Unsupported remote protocol {protocol_version}")));
                            }
                            return;
                        }
                        ServerFrame::Ready { .. } => {}
                        ServerFrame::Response { id: READINESS_REQUEST_ID, response } => {
                            let RemoteResponse::Sessions(sessions) = response else {
                                if let Some(reply) = initial_ready.take() { let _ = reply.send(Err("Backend returned an invalid readiness response".into())); }
                                return;
                            };
                            ready.store(true, Ordering::Release);
                            if let Some(reply) = initial_ready.take() { let _ = reply.send(Ok(sessions)); }
                        }
                        ServerFrame::Response { id, response } => {
                            if let Some(reply) = pending.remove(&id) {
                                let _ = reply.send(Ok(response));
                            }
                        }
                        ServerFrame::Error { id: READINESS_REQUEST_ID, message } => {
                            if let Some(reply) = initial_ready.take() { let _ = reply.send(Err(message)); }
                            return;
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
        ready.store(false, Ordering::Release);
        if let Some(reply) = initial_ready.take() {
            let _ = reply.send(Err("Backend disconnected before readiness".into()));
            return;
        }
        for (_, reply) in pending.drain() {
            let _ = reply.send(Err(
                "The Remote Assembly connection was interrupted".to_string()
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
        return Err("Remote Assembly frame exceeds one MiB".to_string());
    }
    socket
        .send(TungsteniteMessage::Text(json.into()))
        .await
        .map_err(|error| error.to_string())
}

fn parse_server_frame(message: TungsteniteMessage) -> Result<Option<ServerFrame>, String> {
    let TungsteniteMessage::Text(text) = message else { return Ok(None); };
    serde_json::from_str(text.as_str()).map(Some).map_err(|error| {
        format!("Backend response is incompatible with this client: {error}")
    })
}

/// Runs the resident remote server instead of starting a Tauri window.
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
            .route("/assembly", get(upgrade_remote_socket))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind(&bind)
            .await
            .map_err(|error| error.to_string())?;
        println!("Assembly remote server listening on {bind}");
        use axum::serve::ListenerExt;
        axum::serve(listener.tap_io(|stream| {
            if let Err(error) = stream.set_nodelay(true) {
                eprintln!("Could not disable remote socket packet delay: {error}");
            }
        }), app)
            .await
            .map_err(|error| error.to_string())
    })
}

async fn upgrade_remote_socket(
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
        .on_upgrade(move |socket| serve_remote_socket(socket, state))
        .into_response()
}

async fn serve_remote_socket(socket: WebSocket, state: ServerState) {
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
        RemoteCommand::Workspace { operation, args } => super::remote_workspace::execute(operation, args).await.map(RemoteResponse::Workspace),
        RemoteCommand::ListSessions => {
            let mut sessions = manager.list_sessions()?;
            for session in &mut sessions {
                session.execution_environment = ExecutionEnvironment::Remote;
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
        // Request ownership is per WebSocket, not shared across Mac clients.
        RemoteCommand::Snapshot {
            owned_id,
            request_id: _,
        } => manager
            .snapshot(&owned_id)
            .map(RemoteResponse::Snapshot),
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
                session.execution_environment = ExecutionEnvironment::Remote;
                RemoteResponse::Session(session)
            })
        }
        RemoteCommand::UpdateMeta(request) => {
            manager.update_session_meta(request).map(|mut session| {
                session.execution_environment = ExecutionEnvironment::Remote;
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

fn validate_ssh_target(target: &str) -> Result<(), String> {
    if target.is_empty() || target.starts_with('-')
        || !target.bytes().all(|value| value.is_ascii_alphanumeric() || b"._@-".contains(&value)) {
        return Err("SSH destination may contain only letters, numbers, '.', '_', '@', and '-'".into());
    }
    Ok(())
}

#[cfg(test)]
mod connection_tests {
    use super::*;

    fn profile(id: &str) -> RemoteAssemblyProfile {
        RemoteAssemblyProfile { id: id.into(), name: "Workbox".into(), ssh_target: "agent-workbox".into(), source_root: String::new(), default_cwd: String::new() }
    }

    #[tokio::test]
    async fn disconnect_keeps_profile_and_session_ownership_but_stops_transport() {
        let manager = RemoteConnectionManager::from_environment(Arc::new(|_| {})).unwrap();
        manager.register_profile(profile("saved")).unwrap();
        manager.remember("conversation", "saved");
        let task = tokio::spawn(std::future::pending::<()>());
        let (sender, _receiver) = mpsc::channel(1);
        {
            let mut state = manager.client.lock().unwrap();
            let client = state.clients.get_mut("saved").unwrap();
            client.requests = Some(sender);
            client.task = Some(task.abort_handle());
            client.ready.store(true, Ordering::Release);
        }
        assert_eq!(manager.environment().ready_profile_ids, vec!["saved"]);
        manager.disconnect_profile("saved");
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(manager.environment().profiles, vec![profile("saved")]);
        assert!(manager.environment().ready_profile_ids.is_empty());
        assert!(manager.owns("conversation"));
        assert!(manager.request_for_profile("saved", RemoteCommand::ListSessions).await.unwrap_err().contains("not connected"));
        let restarted = RemoteConnectionManager::from_environment(Arc::new(|_| {})).unwrap();
        restarted.restore_profiles(&serde_json::to_string(&manager.environment().profiles).unwrap()).unwrap();
        assert_eq!(restarted.environment().profiles, vec![profile("saved")]);
        assert!(restarted.environment().ready_profile_ids.is_empty());
    }

    #[tokio::test]
    async fn restored_profiles_are_dormant_and_cancellation_releases_attempt() {
        let manager = RemoteConnectionManager::from_environment(Arc::new(|_| {})).unwrap();
        manager.restore_profiles(&serde_json::to_string(&vec![profile("original")]).unwrap()).unwrap();
        assert!(!manager.is_configured());
        assert!(manager.environment().ready_profile_ids.is_empty());
        assert!(manager.tunnel_processes.lock().unwrap().is_empty());
        let guard = manager.connection_lock.lock().await;
        let owner = manager.clone();
        let (registered, registration) = oneshot::channel();
        let registered = Mutex::new(Some(registered));
        let task = tokio::spawn(async move {
            owner.connect_profile(profile("original"), "cancel-test".into(), tauri::ipc::Channel::new(move |_| {
                if let Some(reply) = registered.lock().unwrap().take() { let _ = reply.send(()); }
                Ok(())
            })).await
        });
        registration.await.unwrap();
        manager.cancel_connection("cancel-test");
        assert_eq!(task.await.unwrap().err().unwrap(), "Connection cancelled");
        drop(guard);
        assert!(manager.attempts.lock().unwrap().is_empty());
        assert!(manager.tunnel_processes.lock().unwrap().is_empty());
        assert_eq!(manager.environment().profiles[0].id, "original");
    }

    #[tokio::test]
    async fn dropping_candidate_reaps_its_tunnel() {
        let processes = Arc::new(Mutex::new(HashMap::new()));
        let child = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = child.id();
        processes.lock().unwrap().insert(pid, child);
        let tunnel = RemoteTunnel { pid, processes: processes.clone(), local_port: 0, token: String::new() };
        let (started, running) = oneshot::channel();
        let task = tokio::spawn(async move {
            let _tunnel = tunnel;
            let _ = started.send(());
            std::future::pending::<()>().await;
        });
        running.await.unwrap();
        let candidate = RemoteClient { requests: None, profile: None, target_key: None,
            task: Some(task.abort_handle()), ready: Arc::new(AtomicBool::new(false)) };
        drop(candidate);
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(processes.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn connection_timeout_releases_attempt() {
        let manager = RemoteConnectionManager::from_environment(Arc::new(|_| {})).unwrap();
        let _guard = manager.connection_lock.lock().await;
        let result = manager.connect_profile(profile("timeout"), "timeout-test".into(), tauri::ipc::Channel::new(|_| Ok(()))).await;
        assert_eq!(result.err().unwrap(), "Connection timed out after 30 seconds");
        assert!(manager.attempts.lock().unwrap().is_empty());
        assert!(manager.tunnel_processes.lock().unwrap().is_empty());
    }

    #[test]
    fn timestamp_payloads_decode_in_snapshots_pages_and_live_events() {
        let event = serde_json::json!({"ownedId":"test", "provider":"codex", "generation":1,
            "sequence":7, "timestampMs":1800000000000_u64,
            "payload":{"kind":"connection", "state":"connected"}});
        let snapshot = serde_json::json!({"connection":{"ownedId":"test", "provider":"codex",
            "generation":1, "state":"connected", "config":{}}, "suspended":true,
            "lastSequence":7, "events":[event.clone()]});
        for frame in [serde_json::json!({"type":"event", "event":event.clone()}),
            serde_json::json!({"type":"response", "id":1, "response":{"result":"snapshot", "value":snapshot}}),
            serde_json::json!({"type":"response", "id":2, "response":{"result":"eventPage", "value":{"events":[event], "hasMore":false}}})] {
            assert!(parse_server_frame(TungsteniteMessage::Text(frame.to_string().into())).unwrap().is_some());
        }
        assert!(parse_server_frame(TungsteniteMessage::Text("{invalid".into())).unwrap_err().contains("incompatible"));
    }

    #[test]
    fn resolved_identity_uses_user_host_and_port() {
        assert_eq!(ssh_target_key("host alias\nuser owner\nhostname BOX.Example\nport 2222\n").unwrap(), "owner@box.example:2222");
        assert!(ssh_target_key("hostname box\n").is_err());
    }

    #[tokio::test]
    async fn readiness_requires_protocol_and_session_response() {
        for version in [PROTOCOL_VERSION, PROTOCOL_VERSION + 1] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let (probe_tx, probe_rx) = oneshot::channel();
            let (release_tx, release_rx) = oneshot::channel();
            let server = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.unwrap();
                let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
                socket.send(TungsteniteMessage::Text(serde_json::to_string(&ServerFrame::Ready { protocol_version: version }).unwrap().into())).await.unwrap();
                if version != PROTOCOL_VERSION { return; }
                let _resume = socket.next().await.unwrap().unwrap();
                let request = socket.next().await.unwrap().unwrap();
                let frame: ClientFrame = serde_json::from_str(request.to_text().unwrap()).unwrap();
                assert!(matches!(frame, ClientFrame::Request { id: READINESS_REQUEST_ID, command: RemoteCommand::ListSessions }));
                probe_tx.send(()).unwrap();
                release_rx.await.unwrap();
                socket.send(TungsteniteMessage::Text(serde_json::to_string(&ServerFrame::Response { id: READINESS_REQUEST_ID, response: RemoteResponse::Sessions(vec![]) }).unwrap().into())).await.unwrap();
                let _ = socket.next().await;
            });
            let (requests, mut receiver) = mpsc::channel(1);
            let (reply, mut result) = oneshot::channel();
            let ready = Arc::new(AtomicBool::new(false));
            let actor_ready = ready.clone();
            let actor = tokio::spawn(async move { client_loop(format!("ws://{address}"), "test-token".into(), &mut receiver, Arc::new(|_| {}), Some(1), actor_ready, &mut Some(reply)).await });
            if version == PROTOCOL_VERSION {
                probe_rx.await.unwrap();
                assert!(!ready.load(Ordering::Acquire));
                assert!(matches!(result.try_recv(), Err(oneshot::error::TryRecvError::Empty)));
                release_tx.send(()).unwrap();
                assert!(result.await.unwrap().unwrap().is_empty());
                assert!(ready.load(Ordering::Acquire));
            } else {
                assert!(result.await.unwrap().unwrap_err().contains("Unsupported remote protocol"));
                assert!(!ready.load(Ordering::Acquire));
            }
            drop(requests);
            actor.abort();
            let _ = actor.await;
            server.await.unwrap();
        }
    }

    #[tokio::test]
    #[ignore = "read-only proof against the owner's installed backend"]
    async fn live_existing_workbox_connection() {
        let manager = RemoteConnectionManager::from_environment(Arc::new(|_| {})).unwrap();
        let result = manager.connect_profile(profile("stage1-proof"), "live-proof".into(), tauri::ipc::Channel::new(|_| Ok(()))).await.unwrap();
        for session in &result.sessions {
            let workspace = manager.request_for_profile(&result.profile.id, RemoteCommand::Workspace {
                operation: "validate_project_root".into(), args: serde_json::json!({"path": session.cwd})
            }).await.unwrap();
            eprintln!("Remote workspace validation: {:?}", workspace);
            assert!(manager.snapshot(session.owned_id.clone(), 100).await.unwrap().is_some());
            let capabilities = manager.capabilities(session.owned_id.clone(), 101).await.unwrap();
            eprintln!("Remote steering advertised: {}", capabilities.session.steering);
        }
        let again = manager.connect_profile(profile("duplicate"), "live-again".into(), tauri::ipc::Channel::new(|_| Ok(()))).await.unwrap();
        assert_eq!(again.profile.id, "stage1-proof");
        assert_eq!(manager.tunnel_processes.lock().unwrap().len(), 1);
        eprintln!("Read-only connection verified: {} session snapshots; duplicate reused one tunnel", result.sessions.len());
        manager.shutdown();
        tokio::task::yield_now().await;
        assert!(manager.tunnel_processes.lock().unwrap().is_empty());
    }
}

#[tauri::command]
pub async fn remote_workspace(
    remote: tauri::State<'_, RemoteConnectionManager>,
    profile_id: String,
    operation: String,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    match remote.request_for_profile(&profile_id, RemoteCommand::Workspace { operation, args }).await? {
        RemoteResponse::Workspace(value) => Ok(value),
        _ => Err("The remote backend returned an incompatible workspace response".into()),
    }
}
