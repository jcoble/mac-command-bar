//! One authenticated WebSocket boundary for conversations owned by a remote machine.

use std::collections::{BTreeMap, HashMap};
use std::future::IntoFuture;
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
use mcb_core::session_store::SessionStore;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message as TungsteniteMessage};

use super::manager::AgentRuntimeManager;
use super::attachments::{self, DeleteConversationAttachmentRequest, SavedConversationAttachment};
use super::prompt_content::prompt_from_blocks;
use super::protocol::{
    AgentCapabilities, AgentConfigOption, AgentConversationConfigState,
    AgentConversationConnection, AgentConversationEvent, AgentConversationEventPage,
    AgentConversationProvider,
    AgentConversationSessionRecord, AgentConversationSnapshot,
    ChangeAgentConversationCheckoutRequest, EnsureAgentConversationRequest, ExecutionEnvironment,
    RespondAgentConversationApprovalRequest, RespondAgentConversationInputRequest,
    RespondAgentConversationPermissionRequest, SendAgentConversationMessageRequest,
    SetAgentConversationConfigRequest, StopAgentConversationTurnRequest,
    UpdateAgentConversationSessionMetaRequest,
};
use super::providers::ProviderRegistry;

const PROTOCOL_VERSION: u16 = 3;
const MAX_WIRE_FRAME_BYTES: usize = 1024 * 1024;
const ATTACHMENT_CHUNK_BYTES: usize = 128 * 1024;
const OUTBOUND_FRAME_CAPACITY: usize = 8;
const REPLAY_PAGE_BYTES: u32 = 1024 * 1024;
pub const REMOTE_ASSEMBLY_PROFILE_SETTING_KEY: &str = "remote-assembly.profile.v1";
pub const REMOTE_ASSEMBLY_PROFILES_SETTING_KEY: &str = "remote-assembly.profiles.v1";
const REMOTE_SESSION_PROJECTIONS_SETTING_KEY: &str = "remote-assembly.session-projections.v1";
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
    ProbeProviderConfig { provider: AgentConversationProvider, cwd: String },
    ListSessions,
    CheckProviderUpdates,
    InstallProviderUpdates,
    RestartForProviderUpdates,
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
    AttachmentChunk { upload_id: String, owned_id: String, mime_type: String, first: bool, last: bool, bytes: Vec<u8> },
    AbortAttachmentUpload { upload_id: String },
    ReadAttachments { owned_id: String },
    ReadAttachmentChunk { owned_id: String, attachment_id: String, thumbnail: bool, offset: u64 },
    DeleteAttachment(DeleteConversationAttachmentRequest),
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
    ProviderUpdates(super::providers::updates::ProviderUpdateStatus),
    Restarting,
    Connection(AgentConversationConnection),
    Snapshot(#[serde(deserialize_with = "deserialize_wire_payload")] Option<AgentConversationSnapshot>),
    EventPage(#[serde(deserialize_with = "deserialize_wire_payload")] AgentConversationEventPage),
    Capabilities(AgentCapabilities),
    Config(AgentConversationConfigState),
    ConfigOptions(Vec<AgentConfigOption>),
    Session(AgentConversationSessionRecord),
    Attachment(SavedConversationAttachment),
    Attachments(Vec<SavedConversationAttachment>),
    Bytes(Vec<u8>),
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
    data_dir: PathBuf,
    token: Arc<str>,
    maintenance: Arc<tokio::sync::RwLock<()>>,
    restarting: Arc<AtomicBool>,
    restart_requested: Arc<tokio::sync::Notify>,
    restart_flushed: Arc<tokio::sync::Notify>,
    events: broadcast::Sender<AgentConversationEvent>,
}

#[derive(Debug)]
struct PendingAttachment {
    upload_id: String,
    owned_id: String,
    mime_type: String,
    path: PathBuf,
    file: std::fs::File,
    bytes: usize,
    deadline: tokio::time::Instant,
}

impl Drop for PendingAttachment {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
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
    cached_sessions: Arc<Mutex<Vec<AgentConversationSessionRecord>>>,
    store: Arc<SessionStore>,
    next_request_id: Arc<AtomicU64>,
    event_sink: Arc<dyn Fn(AgentConversationEvent) + Send + Sync>,
    status_sink: Arc<dyn Fn(RemoteConnectionStatus) + Send + Sync>,
    connection_lock: Arc<tokio::sync::Mutex<()>>,
    attempts: Arc<Mutex<HashMap<String, tokio::task::AbortHandle>>>,
    /// The attempt token of each profile whose connection attempt is running
    /// before any client exists. Removing the profile drops its token, which
    /// both answers disconnected at once and tells the running attempt that it
    /// no longer owns the profile.
    connecting: Arc<Mutex<HashMap<String, u64>>>,
    next_attempt_token: Arc<AtomicU64>,
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
    replaced_profile_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteUninstallResult {
    environment: RemoteAssemblyEnvironment,
    replaced_profile_id: Option<String>,
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBackendProfileStatus {
    pub profile_id: String,
    #[serde(flatten)]
    pub backend: super::remote_install::BackendStatus,
}

/// What one saved machine's transport is doing. The manager derives it from the
/// live client only: a session row, a saved record, or a transcript read never
/// implies it.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RemoteConnectionState {
    /// The socket answered the readiness session list.
    Connected,
    /// A requested client or tunnel is attempting or retrying.
    Reconnecting,
    /// Stopped on request, or no retry remains and the client is gone.
    Disconnected,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteConnectionStatus {
    pub profile_id: String,
    pub state: RemoteConnectionState,
}

/// Holds one profile in Reconnecting for as long as its attempt runs, including
/// the cancelled and timed-out attempts, which drop it.
struct ConnectingAttempt {
    manager: RemoteConnectionManager,
    profile_id: String,
    token: u64,
}

impl ConnectingAttempt {
    fn start(manager: &RemoteConnectionManager, profile_id: &str) -> Self {
        let token = manager.next_attempt_token.fetch_add(1, Ordering::Relaxed);
        manager
            .connecting
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(profile_id.to_string(), token);
        manager.publish_status(profile_id);
        Self { manager: manager.clone(), profile_id: profile_id.to_string(), token }
    }
}

impl Drop for ConnectingAttempt {
    fn drop(&mut self) {
        {
            let mut connecting = self
                .manager
                .connecting
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            // The profile was removed, or a later attempt took it over: this
            // attempt owns nothing and must not report for it.
            if connecting.get(&self.profile_id) != Some(&self.token) {
                return;
            }
            connecting.remove(&self.profile_id);
        }
        self.manager.publish_status(&self.profile_id);
    }
}

/// Reports the end of a live connection once, however the actor ends: normal
/// exit, error return, or an aborted task.
struct ConnectionEnd {
    ready: Arc<AtomicBool>,
    status: Arc<dyn Fn() + Send + Sync>,
}

impl Drop for ConnectionEnd {
    fn drop(&mut self) {
        if self.ready.swap(false, Ordering::AcqRel) {
            (self.status)();
        }
    }
}

fn read_cached_sessions(store: &SessionStore) -> Result<Vec<AgentConversationSessionRecord>, String> {
    let Some(json) = store
        .get_app_setting(REMOTE_SESSION_PROJECTIONS_SETTING_KEY)
        .map_err(|error| error.to_string())?
    else {
        return Ok(Vec::new());
    };
    let sessions: Vec<AgentConversationSessionRecord> = serde_json::from_str(&json)
        .map_err(|error| format!("Invalid cached remote sessions: {error}"))?;
    Ok(sessions
        .into_iter()
        .filter(|session| {
            session.execution_environment == ExecutionEnvironment::Remote
                && session.remote_profile_id.as_deref().is_some_and(|id| !id.is_empty())
        })
        .collect())
}

impl RemoteConnectionManager {
    pub fn from_environment(
        event_sink: Arc<dyn Fn(AgentConversationEvent) + Send + Sync>,
        status_sink: Arc<dyn Fn(RemoteConnectionStatus) + Send + Sync>,
        store: Arc<SessionStore>,
    ) -> Result<Self, String> {
        let cached_sessions = read_cached_sessions(&store)?;
        let remote_sessions = cached_sessions
            .iter()
            .filter_map(|session| {
                session
                    .remote_profile_id
                    .as_ref()
                    .map(|profile_id| (session.owned_id.clone(), profile_id.clone()))
            })
            .collect();
        let manager = Self {
            client: Arc::new(Mutex::new(RemoteClientState::default())),
            tunnel_processes: Arc::new(Mutex::new(HashMap::new())),
            remote_sessions: Arc::new(Mutex::new(remote_sessions)),
            cached_sessions: Arc::new(Mutex::new(cached_sessions)),
            store,
            next_request_id: Arc::new(AtomicU64::new(1)),
            event_sink,
            status_sink,
            connection_lock: Arc::new(tokio::sync::Mutex::new(())),
            attempts: Arc::new(Mutex::new(HashMap::new())),
            connecting: Arc::new(Mutex::new(HashMap::new())),
            next_attempt_token: Arc::new(AtomicU64::new(1)),
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
            // The development environment client has no saved profile, so no rail
            // row maps to it and its transitions have nobody to tell.
            let status: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
            client_loop(url, token, &mut request_rx, event_sink, None, actor_ready, &mut None, status)
                .await;
        });
        Ok(manager)
    }

    /// The compact local rail projection. Conversation events and transcripts
    /// remain exclusively in the remote backend's database.
    pub fn cached_sessions(&self) -> Vec<AgentConversationSessionRecord> {
        self.cached_sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn save_cached_sessions(
        &self,
        next: Vec<AgentConversationSessionRecord>,
    ) -> Result<(), String> {
        let json = serde_json::to_string(&next)
            .map_err(|error| format!("Could not encode cached remote sessions: {error}"))?;
        self.store
            .upsert_app_setting(REMOTE_SESSION_PROJECTIONS_SETTING_KEY, &json)
            .map_err(|error| error.to_string())?;
        *self
            .cached_sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = next;
        Ok(())
    }

    fn replace_cached_profile_sessions(
        &self,
        profile_id: &str,
        sessions: &[AgentConversationSessionRecord],
    ) -> Result<(), String> {
        let incoming_ids = sessions
            .iter()
            .map(|session| session.owned_id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let mut next = self.cached_sessions();
        next.retain(|session| {
            session.remote_profile_id.as_deref() != Some(profile_id)
                && !incoming_ids.contains(session.owned_id.as_str())
        });
        next.extend(sessions.iter().cloned().map(|mut session| {
            session.execution_environment = ExecutionEnvironment::Remote;
            session.remote_profile_id = Some(profile_id.to_string());
            session
        }));
        next.sort_by(|left, right| {
            right
                .last_activity_at_ms
                .cmp(&left.last_activity_at_ms)
                .then_with(|| left.owned_id.cmp(&right.owned_id))
        });
        self.save_cached_sessions(next)?;
        let mut routing = self
            .remote_sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        routing.retain(|_, stored_profile_id| stored_profile_id != profile_id);
        routing.extend(
            sessions
                .iter()
                .map(|session| (session.owned_id.clone(), profile_id.to_string())),
        );
        Ok(())
    }

    fn reassign_cached_profile_sessions(
        &self,
        from_profile_id: &str,
        to_profile_id: &str,
    ) -> Result<(), String> {
        if from_profile_id == to_profile_id {
            return Ok(());
        }
        let mut next = self.cached_sessions();
        let mut changed = false;
        for session in &mut next {
            if session.remote_profile_id.as_deref() == Some(from_profile_id) {
                session.remote_profile_id = Some(to_profile_id.to_string());
                changed = true;
            }
        }
        if !changed {
            return Ok(());
        }
        self.save_cached_sessions(next)?;
        let mut routing = self
            .remote_sessions
            .lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        for profile_id in routing.values_mut() {
            if profile_id == from_profile_id {
                *profile_id = to_profile_id.to_string();
            }
        }
        Ok(())
    }

    fn cache_session(&self, mut session: AgentConversationSessionRecord) -> Result<(), String> {
        session.execution_environment = ExecutionEnvironment::Remote;
        let owned_id = session.owned_id.clone();
        let profile_id = session
            .remote_profile_id
            .clone()
            .ok_or_else(|| "Remote session did not identify its machine".to_string())?;
        let mut next = self.cached_sessions();
        next.retain(|stored| stored.owned_id != session.owned_id);
        next.push(session);
        next.sort_by(|left, right| {
            right
                .last_activity_at_ms
                .cmp(&left.last_activity_at_ms)
                .then_with(|| left.owned_id.cmp(&right.owned_id))
        });
        self.save_cached_sessions(next)?;
        self.remember(owned_id, profile_id);
        Ok(())
    }

    fn forget_cached_session(&self, owned_id: &str) -> Result<(), String> {
        let mut next = self.cached_sessions();
        next.retain(|session| session.owned_id != owned_id);
        self.save_cached_sessions(next)
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

    /// The one answer the rail paints, read from the live client each time so a
    /// stale actor's report cannot contradict it.
    fn connection_state(&self, profile_id: &str) -> RemoteConnectionState {
        if self
            .connecting
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains_key(profile_id)
        {
            return RemoteConnectionState::Reconnecting;
        }
        let state = self.client.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        match state.clients.get(profile_id) {
            Some(client)
                if client.requests.as_ref().is_some_and(|sender| !sender.is_closed()) =>
            {
                if client.ready.load(Ordering::Acquire) {
                    RemoteConnectionState::Connected
                } else {
                    RemoteConnectionState::Reconnecting
                }
            }
            _ => RemoteConnectionState::Disconnected,
        }
    }

    fn publish_status(&self, profile_id: &str) {
        (self.status_sink)(RemoteConnectionStatus {
            profile_id: profile_id.to_string(),
            state: self.connection_state(profile_id),
        });
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

    pub async fn install_and_connect_profile(&self, profile: RemoteAssemblyProfile, operation_id: String,
        status: tauri::ipc::Channel<String>) -> Result<RemoteConnectionResult, String> {
        validate_profile(&profile)?;
        let manager = self.clone();
        let progress = status.clone();
        let mut task = {
            let mut attempts = self.attempts.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if attempts.contains_key(&operation_id) { return Err("Connection attempt already exists".into()); }
            let task = tokio::spawn(async move {
                let _owner = manager.connection_lock.lock().await;
                let (profile, _, _) = manager.resolve_profile_identity(profile).await?;
                manager.disconnect_profile(&profile.id);
                let receipt = super::remote_install::install_latest(&profile.ssh_target, progress.clone()).await?;
                let _ = progress.send(format!("Installed backend {} from {}. Connecting…",
                    receipt.version, &receipt.commit[..12]));
                manager.connect_profile_inner_locked(profile, progress).await
            });
            attempts.insert(operation_id.clone(), task.abort_handle());
            task
        };
        let _ = status.send("Preparing installation…".into());
        let result = match tokio::time::timeout(Duration::from_secs(900), &mut task).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("Installation cancelled".into()),
            Err(_) => {
                task.abort();
                let _ = task.await;
                Err("Installation timed out after 15 minutes".into())
            }
        };
        self.attempts.lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&operation_id);
        result
    }

    pub async fn uninstall_profile(
        &self,
        profile: RemoteAssemblyProfile,
        delete_data: bool,
    ) -> Result<Option<String>, String> {
        validate_profile(&profile)?;
        let _owner = self.connection_lock.lock().await;
        let (profile, _, replaced_profile_id) = self.resolve_profile_identity(profile).await?;
        self.disconnect_profile(&profile.id);
        super::remote_install::uninstall(&profile.ssh_target, delete_data).await?;
        if delete_data {
            self.replace_cached_profile_sessions(&profile.id, &[])?;
        }
        Ok(replaced_profile_id)
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

    async fn connect_profile_inner(&self, profile: RemoteAssemblyProfile,
        status: tauri::ipc::Channel<String>) -> Result<RemoteConnectionResult, String> {
        let _owner = self.connection_lock.lock().await;
        self.connect_profile_inner_locked(profile, status).await
    }

    async fn connect_profile_inner_locked(&self, profile: RemoteAssemblyProfile,
        status: tauri::ipc::Channel<String>) -> Result<RemoteConnectionResult, String> {
        let _ = status.send("Resolving SSH configuration…".into());
        let (profile, target_key, replaced_profile_id) = self.resolve_profile_identity(profile).await?;
        let attempt = ConnectingAttempt::start(self, &profile.id);
        let reusable = self.client.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
            .clients.get(&profile.id).is_some_and(|client| client.target_key.as_ref() == Some(&target_key)
                && client.requests.as_ref().is_some_and(|sender| !sender.is_closed()));
        let (mut sessions, candidate) = if reusable {
            let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
            let RemoteResponse::Sessions(sessions) = self.request_with_id(&profile.id, id, RemoteCommand::ListSessions).await?
                else { return Err("Backend returned an invalid session list".into()); };
            (sessions, None)
        } else {
            let _ = status.send("Checking the installed backend…".into());
            let (requests, receiver) = mpsc::channel(32);
            let (ready_tx, ready_rx) = oneshot::channel();
            let ready = Arc::new(AtomicBool::new(false));
            let manager = self.clone();
            let reported_id = profile.id.clone();
            let report: Arc<dyn Fn() + Send + Sync> =
                Arc::new(move || manager.publish_status(&reported_id));
            let task = tokio::spawn(profile_client_loop(profile.clone(), receiver, self.event_sink.clone(),
                self.tunnel_processes.clone(), ready.clone(), Some(ready_tx), report));
            // Dropping a failed/cancelled candidate aborts only its own actor and tunnel.
            let candidate = RemoteClient { requests: Some(requests), profile: Some(profile.clone()),
                target_key: Some(target_key), task: Some(task.abort_handle()), ready };
            let sessions = ready_rx.await.map_err(|_| "Backend connection ended before readiness".to_string())??;
            (sessions, Some(candidate))
        };
        for session in &mut sessions {
            session.remote_profile_id = Some(profile.id.clone());
        }
        self.replace_cached_profile_sessions(&profile.id, &sessions)?;
        // A removal while this attempt was in flight wins: the profile neither
        // gets its client back nor any report or session from this attempt.
        self.adopt_client(&attempt, candidate)?;
        for session in &sessions {
            self.remember(session.owned_id.clone(), profile.id.clone());
        }
        if let Some(client) = self.client.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clients.get_mut(&profile.id) {
            client.profile = Some(profile.clone());
        }
        Ok(RemoteConnectionResult { profile, sessions, replaced_profile_id })
    }

    async fn resolve_profile_identity(&self, mut profile: RemoteAssemblyProfile)
        -> Result<(RemoteAssemblyProfile, String, Option<String>), String> {
        let target_key = resolve_ssh_target(&profile.ssh_target).await?;
        let mut replaced_profile_id = None;
        let saved = self.environment().profiles;
        for existing in saved {
            let key = if existing.ssh_target == profile.ssh_target { Some(target_key.clone()) }
                else { resolve_ssh_target(&existing.ssh_target).await.ok() };
            if key.as_ref() != Some(&target_key) { continue; }
            // Alias and address forms of the same host keep the original profile/session IDs.
            if existing.id != profile.id { profile = existing; }
            break;
        }
        if profile.id != "agent-workbox" {
            if let Some(json) = self.store.get_app_setting(REMOTE_ASSEMBLY_PROFILE_SETTING_KEY)
                .map_err(|error| error.to_string())?
            {
                if let Ok(legacy) = serde_json::from_str::<LegacyRemoteAssemblyProfile>(&json) {
                    let legacy_key = if legacy.ssh_target == profile.ssh_target {
                        Some(target_key.clone())
                    } else {
                        resolve_ssh_target(&legacy.ssh_target).await.ok()
                    };
                    if legacy_key.as_ref() == Some(&target_key) {
                        self.reassign_cached_profile_sessions("agent-workbox", &profile.id)?;
                        replaced_profile_id = Some("agent-workbox".to_string());
                    }
                }
            }
        }
        Ok((profile, target_key, replaced_profile_id))
    }

    pub fn disconnect_profile(&self, profile_id: &str) {
        {
            let mut state = self.client.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(client) = state.clients.get_mut(profile_id) {
                if let Some(task) = client.task.take() { task.abort(); }
                client.requests = None;
                client.target_key = None;
                client.ready.store(false, Ordering::Release);
            }
        }
        self.publish_status(profile_id);
    }

    pub fn remove_profile(&self, profile_id: &str) {
        // Dropping the attempt token first makes removal authoritative: the
        // state derived below is disconnected even mid-attempt, and the running
        // attempt can no longer install a client for this profile. A later
        // connect starts a fresh attempt with a new token, so nothing is
        // permanently blocked.
        self.connecting
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(profile_id);
        // Dropping the client aborts its own actor and tunnel.
        self.client
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clients
            .remove(profile_id);
        self.publish_status(profile_id);
    }

    /// Installs a freshly connected client only while this attempt still owns
    /// the profile, holding the attempt token while it writes so a concurrent
    /// removal cannot be overtaken. A dropped candidate aborts its own actor
    /// and tunnel.
    fn adopt_client(
        &self,
        attempt: &ConnectingAttempt,
        candidate: Option<RemoteClient>,
    ) -> Result<(), String> {
        let connecting = self
            .connecting
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if connecting.get(&attempt.profile_id) != Some(&attempt.token) {
            return Err(format!(
                "Remote machine {} was removed during the connection attempt",
                attempt.profile_id
            ));
        }
        if let Some(candidate) = candidate {
            self.client
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clients
                .insert(attempt.profile_id.clone(), candidate);
        }
        Ok(())
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
        // Metadata can take two 30-second reads. Installation additionally streams
        // seven payloads, each bounded to 180 seconds, then verifies them on disk.
        let timeout_seconds = match &command {
            RemoteCommand::CheckProviderUpdates => 75,
            RemoteCommand::InstallProviderUpdates => 25 * 60,
            _ => 15,
        };
        let (reply, answer) = oneshot::channel();
        sender
            .try_send(ClientRequest::Execute { id, command, reply })
            .map_err(|error| {
                format!("The Remote Assembly request queue is unavailable: {error}")
            })?;
        match tokio::time::timeout(Duration::from_secs(timeout_seconds), answer).await {
            Ok(answer) => answer.map_err(|_| {
                "The Remote Assembly connection closed before answering".to_string()
            })?,
            Err(_) => {
                let _ = sender.try_send(ClientRequest::Cancel { id });
                Err(format!("The remote machine did not answer within {timeout_seconds} seconds"))
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
            }
            self.replace_cached_profile_sessions(&profile_id, &sessions)?;
            for session in &sessions {
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
        let RemoteResponse::Sessions(mut sessions) = self
            .request_for_profile(&profile_id, RemoteCommand::ListSessions)
            .await?
        else {
            return Err("Remote Assembly returned the wrong list response".to_string());
        };
        for session in &mut sessions {
            session.remote_profile_id = Some(profile_id.clone());
        }
        if !sessions.iter().any(|session| session.owned_id == owned_id) {
            return Err(format!("Ensured session {owned_id} was not stored"));
        }
        self.replace_cached_profile_sessions(&profile_id, &sessions)?;
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

    pub async fn probe_provider_config(
        &self,
        profile_id: &str,
        provider: AgentConversationProvider,
        cwd: String,
        request_id: u64,
    ) -> Result<AgentConversationConfigState, String> {
        match self.request_with_id(
            profile_id,
            request_id,
            RemoteCommand::ProbeProviderConfig { provider, cwd },
        ).await? {
            RemoteResponse::Config(config) => Ok(config),
            _ => Err("Remote Assembly returned the wrong provider catalog response".into()),
        }
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
        self.cache_session(session.clone())?;
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
        self.cache_session(session.clone())?;
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
            self.forget_cached_session(&owned_id)?;
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

    pub async fn save_attachment(&self, owned_id: String, mime_type: String, bytes: Vec<u8>) -> Result<SavedConversationAttachment, String> {
        if bytes.is_empty() || bytes.len() > attachments::MAX_ATTACHMENT_BYTES {
            return Err("Image must be between 1 byte and 20 MB".into());
        }
        let upload_id = uuid::Uuid::new_v4().to_string();
        let mut saved = None;
        for (index, chunk) in bytes.chunks(ATTACHMENT_CHUNK_BYTES).enumerate() {
            let first = index == 0;
            let last = (index + 1) * ATTACHMENT_CHUNK_BYTES >= bytes.len();
            let result = self.request_for_owned(&owned_id, RemoteCommand::AttachmentChunk {
                upload_id: upload_id.clone(), owned_id: owned_id.clone(), mime_type: mime_type.clone(), first, last, bytes: chunk.to_vec(),
            }).await;
            match result {
                Ok(RemoteResponse::Attachment(attachment)) if last => saved = Some(attachment),
                Ok(RemoteResponse::Empty) if !last => {},
                Ok(_) => {
                    let _ = self.empty_for_owned(&owned_id, RemoteCommand::AbortAttachmentUpload { upload_id }).await;
                    return Err("Remote Assembly returned the wrong attachment response".into());
                }
                Err(error) => {
                    let _ = self.empty_for_owned(&owned_id, RemoteCommand::AbortAttachmentUpload { upload_id }).await;
                    return Err(error);
                }
            }
        }
        saved.ok_or_else(|| "Remote attachment was not saved".into())
    }

    pub async fn read_attachments(&self, owned_id: String) -> Result<Vec<SavedConversationAttachment>, String> {
        let RemoteResponse::Attachments(rows) = self.request_for_owned(&owned_id, RemoteCommand::ReadAttachments { owned_id: owned_id.clone() }).await? else {
            return Err("Remote Assembly returned the wrong attachment list".into());
        };
        Ok(rows)
    }

    pub async fn read_attachment_chunk(&self, owned_id: String, attachment_id: String, thumbnail: bool, offset: u64) -> Result<Vec<u8>, String> {
        let RemoteResponse::Bytes(bytes) = self.request_for_owned(&owned_id, RemoteCommand::ReadAttachmentChunk {
            owned_id: owned_id.clone(), attachment_id, thumbnail, offset,
        }).await? else {
            return Err("Remote Assembly returned the wrong attachment bytes".into());
        };
        Ok(bytes)
    }

    pub async fn delete_attachment(&self, request: DeleteConversationAttachmentRequest) -> Result<(), String> {
        let owned_id = request.owned_id.clone();
        self.empty_for_owned(&owned_id, RemoteCommand::DeleteAttachment(request)).await
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
pub async fn install_remote_assembly(
    manager: tauri::State<'_, AgentRuntimeManager>, remote: tauri::State<'_, RemoteConnectionManager>,
    profile: RemoteAssemblyProfile, operation_id: String, status: tauri::ipc::Channel<String>,
) -> Result<RemoteConnectionResult, super::protocol::CommandError> {
    let result = remote.install_and_connect_profile(profile, operation_id, status).await
        .map_err(super::protocol::CommandError::from)?;
    let profiles_json = serde_json::to_string(&remote.environment().profiles)
        .map_err(|error| super::protocol::CommandError::from(error.to_string()))?;
    manager.write_app_setting(REMOTE_ASSEMBLY_PROFILES_SETTING_KEY, &profiles_json)
        .map_err(super::protocol::CommandError::from)?;
    Ok(result)
}

#[tauri::command]
pub async fn read_remote_backend_statuses(
    profiles: Vec<RemoteAssemblyProfile>,
) -> Result<Vec<RemoteBackendProfileStatus>, super::protocol::CommandError> {
    for profile in &profiles {
        validate_profile(profile).map_err(super::protocol::CommandError::from)?;
    }
    let latest_version = super::remote_install::latest_version()
        .await
        .map_err(super::protocol::CommandError::from)?;
    let statuses = join_all(profiles.into_iter().map(|profile| {
        let latest_version = latest_version.clone();
        async move {
            let backend = super::remote_install::read_status(&profile.ssh_target, &latest_version)
                .await?;
            Ok::<_, String>(RemoteBackendProfileStatus {
                profile_id: profile.id,
                backend,
            })
        }
    }))
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .map_err(super::protocol::CommandError::from)?;
    Ok(statuses)
}

#[tauri::command]
pub async fn uninstall_remote_assembly(
    remote: tauri::State<'_, RemoteConnectionManager>,
    profile: RemoteAssemblyProfile,
    delete_data: bool,
) -> Result<RemoteUninstallResult, super::protocol::CommandError> {
    let replaced_profile_id = remote
        .uninstall_profile(profile, delete_data)
        .await
        .map_err(super::protocol::CommandError::from)?;
    Ok(RemoteUninstallResult {
        environment: remote.environment(),
        replaced_profile_id,
    })
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
    status: Arc<dyn Fn() + Send + Sync>,
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
        client_loop(url, token, &mut requests, event_sink.clone(), Some(4), ready.clone(),
            &mut initial_ready, status.clone()).await;
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
    status: Arc<dyn Fn() + Send + Sync>,
) {
    let _end = ConnectionEnd { ready: ready.clone(), status: status.clone() };
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
                Some(ServerFrame::Ready { protocol_version }) => Err(format!("Remote backend protocol {protocol_version} needs an update; this Assembly requires {PROTOCOL_VERSION}")),
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
                            status();
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
        if ready.swap(false, Ordering::AcqRel) { status(); }
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
    for entry in std::fs::read_dir(&data_dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        if entry.file_type().map_err(|error| error.to_string())?.is_file()
            && name.to_string_lossy().strip_prefix(".attachment-upload-")
                .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok()) {
            std::fs::remove_file(entry.path()).map_err(|error| error.to_string())?;
        }
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    runtime.block_on(async move {
        let manager = AgentRuntimeManager::open(
            ProviderRegistry::bundled_from_environment_at(Some(&data_dir))?,
            &data_dir.join("sessions.db"),
        )?;
        let (events, _) = broadcast::channel(256);
        let event_sender = events.clone();
        manager.set_emitter(Arc::new(move |event| {
            let _ = event_sender.send(event);
        }));
        let state = ServerState {
            manager,
            data_dir: data_dir.clone(),
            token: Arc::from(token),
            maintenance: Arc::new(tokio::sync::RwLock::new(())),
            restarting: Arc::new(AtomicBool::new(false)),
            restart_requested: Arc::new(tokio::sync::Notify::new()),
            restart_flushed: Arc::new(tokio::sync::Notify::new()),
            events,
        };
        let restart_requested = state.restart_requested.clone();
        let restart_flushed = state.restart_flushed.clone();
        let app = Router::new()
            .route("/assembly", get(upgrade_remote_socket))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind(&bind)
            .await
            .map_err(|error| error.to_string())?;
        println!("Assembly remote server listening on {bind}");
        use axum::serve::ListenerExt;
        tokio::select! {
            result = axum::serve(listener.tap_io(|stream| {
                if let Err(error) = stream.set_nodelay(true) {
                    eprintln!("Could not disable remote socket packet delay: {error}");
                }
            }), app).into_future() => result.map_err(|error| error.to_string()),
            _ = wait_for_provider_restart(&restart_requested, &restart_flushed) => {
                // The service uses Restart=on-failure. The server owns the
                // accepted restart even if its requesting socket disappears.
                std::process::exit(75);
            }
        }
    })
}

async fn wait_for_provider_restart(requested: &tokio::sync::Notify, flushed: &tokio::sync::Notify) {
    requested.notified().await;
    // Allow the acknowledgement to flush, but a dropped or stalled socket
    // must not strand the server in its no-new-requests state.
    let _ = tokio::time::timeout(Duration::from_secs(2), flushed.notified()).await;
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
    let restart_flushed = state.restart_flushed.clone();
    let writer_task = tokio::spawn(async move {
        while let Some(frame) = outgoing.recv().await {
            let Ok(json) = serde_json::to_string(&frame) else {
                continue;
            };
            if json.len() > MAX_WIRE_FRAME_BYTES {
                continue;
            }
            let restarting = matches!(frame, ServerFrame::Response { response: RemoteResponse::Restarting, .. });
            let sent = writer.send(AxumMessage::Text(json.into())).await;
            if restarting {
                restart_flushed.notify_one();
            }
            if sent.is_err() { break; }
        }
    });
    let _ = outbound
        .send(ServerFrame::Ready {
            protocol_version: PROTOCOL_VERSION,
        })
        .await;
    let mut live_events = state.events.subscribe();
    let (completed, mut completions) = mpsc::channel::<(u64, Result<RemoteResponse, String>)>(32);
    let upload = Arc::new(Mutex::new(None::<PendingAttachment>));
    let mut request_tasks = HashMap::new();
    loop {
        let upload_deadline = upload.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref().map(|pending| pending.deadline);
        let message = tokio::select! {
            _ = tokio::time::sleep_until(upload_deadline.unwrap_or_else(tokio::time::Instant::now)), if upload_deadline.is_some() => {
                let mut pending = upload.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                if pending.as_ref().is_some_and(|item| item.deadline <= tokio::time::Instant::now()) { *pending = None; }
                None
            }
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
                let request_state = state.clone();
                let request_upload = upload.clone();
                let completion_sink = completed.clone();
                let task = tokio::spawn(async move {
                    let response = execute_server_command(&request_state, &request_upload, command).await;
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

fn abort_attachment_upload(upload: &Mutex<Option<PendingAttachment>>, upload_id: &str) {
    let mut pending = upload.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if pending.as_ref().is_some_and(|current| current.upload_id == upload_id) { *pending = None; }
}

fn write_attachment_chunk(
    data_dir: &std::path::Path,
    upload: &Mutex<Option<PendingAttachment>>,
    upload_id: String,
    owned_id: String,
    mime_type: String,
    first: bool,
    last: bool,
    bytes: Vec<u8>,
) -> Result<Option<PendingAttachment>, String> {
    uuid::Uuid::parse_str(&upload_id).map_err(|_| "Attachment upload id is invalid")?;
    let mut pending = upload.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if pending.as_ref().is_some_and(|current| current.upload_id != upload_id) {
        return Err("Another attachment transfer is active".into());
    }
    if bytes.is_empty() || bytes.len() > ATTACHMENT_CHUNK_BYTES {
        if pending.as_ref().is_some_and(|current| current.upload_id == upload_id) { *pending = None; }
        return Err("Attachment chunk is outside the transfer limit".into());
    }
    if first {
        if pending.is_some() { return Err("Attachment transfer already started".into()); }
        let path = data_dir.join(format!(".attachment-upload-{upload_id}"));
        let file = std::fs::OpenOptions::new().write(true).create_new(true).open(&path)
            .map_err(|error| format!("Could not start attachment transfer: {error}"))?;
        *pending = Some(PendingAttachment { upload_id: upload_id.clone(), owned_id: owned_id.clone(), mime_type: mime_type.clone(), path, file, bytes: 0, deadline: tokio::time::Instant::now() + Duration::from_secs(30) });
    }
    let Some(current) = pending.as_mut() else { return Err("Attachment transfer has not started".into()); };
    if current.owned_id != owned_id || current.mime_type != mime_type || current.bytes + bytes.len() > attachments::MAX_ATTACHMENT_BYTES {
        *pending = None;
        return Err("Attachment transfer is invalid or exceeds 20 MB".into());
    }
    use std::io::Write;
    if let Err(error) = current.file.write_all(&bytes) {
        *pending = None;
        return Err(format!("Could not write attachment transfer: {error}"));
    }
    current.bytes += bytes.len();
    current.deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    Ok(if last { pending.take() } else { None })
}

async fn execute_server_command(state: &ServerState, upload: &Mutex<Option<PendingAttachment>>, command: RemoteCommand) -> Result<RemoteResponse, String> {
    let vault = state.data_dir.join("conversation-attachments");
    let command = match command {
        RemoteCommand::AttachmentChunk { upload_id, owned_id, mime_type, first, last, bytes } => {
            let complete = write_attachment_chunk(&state.data_dir, upload, upload_id, owned_id, mime_type, first, last, bytes)?;
            return match complete {
                Some(complete) => {
                    use std::io::Read;
                    let file = std::fs::File::open(&complete.path).map_err(|error| error.to_string())?;
                    let mut bytes = Vec::new();
                    file.take(attachments::MAX_ATTACHMENT_BYTES as u64 + 1).read_to_end(&mut bytes)
                        .map_err(|error| error.to_string())?;
                    if bytes.len() > attachments::MAX_ATTACHMENT_BYTES { return Err("Attachment exceeds 20 MB".into()); }
                    attachments::save_at(&vault, state.manager.store(), &complete.owned_id, &complete.mime_type, &bytes).map(RemoteResponse::Attachment)
                }
                None => Ok(RemoteResponse::Empty),
            };
        }
        RemoteCommand::AbortAttachmentUpload { upload_id } => {
            abort_attachment_upload(upload, &upload_id);
            return Ok(RemoteResponse::Empty);
        }
        RemoteCommand::ReadAttachments { owned_id } => {
            return attachments::read_at(&vault, state.manager.store(), &owned_id).map(RemoteResponse::Attachments);
        }
        RemoteCommand::ReadAttachmentChunk { owned_id, attachment_id, thumbnail, offset } => {
            return attachments::read_chunk_at(&vault, state.manager.store(), &owned_id, &attachment_id, thumbnail, offset, ATTACHMENT_CHUNK_BYTES).map(RemoteResponse::Bytes);
        }
        RemoteCommand::DeleteAttachment(request) => {
            return attachments::delete_at(&vault, state.manager.store(), request).map(|_| RemoteResponse::Empty);
        }
        other => other,
    };
    if matches!(command, RemoteCommand::RestartForProviderUpdates) {
        let _exclusive = state.maintenance.write().await;
        if state.manager.has_pending_provider_work() {
            return Err("Finish or stop remote conversations before restarting the remote server".into());
        }
        if state.restarting.swap(true, Ordering::AcqRel) {
            return Err("The remote server is already restarting".into());
        }
        state.restart_requested.notify_one();
        return Ok(RemoteResponse::Restarting);
    }
    let _request = state.maintenance.read().await;
    if state.restarting.load(Ordering::Acquire) {
        return Err("The remote server is restarting; reconnect before continuing".into());
    }
    execute_remote_command(&state.manager, &vault, command).await
}

async fn execute_remote_command(
    manager: &AgentRuntimeManager,
    vault: &std::path::Path,
    command: RemoteCommand,
) -> Result<RemoteResponse, String> {
    match command {
        RemoteCommand::RestartForProviderUpdates => Err("Remote restart requires exclusive request ownership".into()),
        RemoteCommand::Workspace { operation, args } => super::remote_workspace::execute(operation, args).await.map(RemoteResponse::Workspace),
        RemoteCommand::ProbeProviderConfig { provider, cwd } => manager
            .probe_provider_config(provider, &cwd)
            .await
            .map(RemoteResponse::Config),
        RemoteCommand::CheckProviderUpdates | RemoteCommand::InstallProviderUpdates => {
            let data_dir = std::env::var_os("ASSEMBLY_SERVER_DATA_DIR").map(PathBuf::from)
                .ok_or_else(|| "Remote server data directory is unavailable".to_string())?;
            let status = if matches!(command, RemoteCommand::InstallProviderUpdates) {
                if manager.has_pending_provider_work() {
                    return Err("Finish or stop remote conversations before updating their adapters".into());
                }
                super::providers::updates::install_at(&data_dir, manager.providers()).await?
            } else {
                super::providers::updates::check_at(&data_dir, manager.providers()).await?
            };
            Ok(RemoteResponse::ProviderUpdates(status))
        }
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
        RemoteCommand::AttachmentChunk { .. } | RemoteCommand::AbortAttachmentUpload { .. }
        | RemoteCommand::ReadAttachments { .. } | RemoteCommand::ReadAttachmentChunk { .. }
        | RemoteCommand::DeleteAttachment(_) => Err("Attachment command needs socket ownership".into()),
        RemoteCommand::Send(request) => {
            if request.content.iter().any(|block| matches!(block, super::prompt_content::AgentPromptContentBlock::Image { .. })) {
                return Err("Remote images must use saved attachment ids".into());
            }
            let mut prompt = if request.text.trim().is_empty() {
                super::providers::AgentPrompt { text: String::new(), images: Vec::new(), attachment_ids: Vec::new() }
            } else {
                prompt_from_blocks(request.text.trim(), request.content)?
            };
            if request.attachment_ids.is_empty() && prompt.text.is_empty() {
                return Err("Message cannot be empty".into());
            }
            let mut total_image_bytes = 0usize;
            if !request.attachment_ids.is_empty() {
                let rows = manager.store().get_attachments(&request.owned_id, &request.attachment_ids)
                    .map_err(|error| error.to_string())?;
                for id in &request.attachment_ids {
                    let row = rows.iter().find(|row| &row.id == id)
                        .ok_or_else(|| "Attachment does not belong to this session".to_string())?;
                    let bytes = attachments::read_original_at(vault, row)?;
                    total_image_bytes += bytes.len();
                    if total_image_bytes > attachments::MAX_ATTACHMENT_BYTES {
                        return Err("Remote prompt images exceed 20 MB".into());
                    }
                    let image = prompt_from_blocks("", vec![super::prompt_content::AgentPromptContentBlock::Image {
                        mime_type: row.mime_type.clone(), data: bytes, name: Some(row.file_name.clone()),
                    }])?.images.remove(0);
                    prompt.images.push(image);
                }
            }
            prompt.attachment_ids = request.attachment_ids;
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

pub(super) fn validate_ssh_target(target: &str) -> Result<(), String> {
    if target.is_empty() || target.starts_with('-')
        || !target.bytes().all(|value| value.is_ascii_alphanumeric() || b"._@-".contains(&value)) {
        return Err("SSH destination may contain only letters, numbers, '.', '_', '@', and '-'".into());
    }
    Ok(())
}

#[cfg(test)]
mod connection_tests {
    use super::*;

    #[test]
    fn attachment_upload_is_single_flight_and_cleans_only_its_owner() {
        let directory = std::env::temp_dir().join(format!("assembly-attachment-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let upload = Mutex::new(None);
        let first = uuid::Uuid::new_v4().to_string();
        let second = uuid::Uuid::new_v4().to_string();
        let chunk = |id: &str, start, end, data: Vec<u8>| {
            write_attachment_chunk(&directory, &upload, id.into(), "owned-a".into(), "image/png".into(), start, end, data)
        };
        assert!(chunk(&first, true, false, vec![1]).unwrap().is_none());
        let path = directory.join(format!(".attachment-upload-{first}"));
        assert!(chunk(&second, true, true, vec![2]).unwrap_err().contains("Another attachment"));
        abort_attachment_upload(&upload, &second);
        assert!(path.exists());
        let finished = chunk(&first, false, true, vec![3]).unwrap().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), vec![1, 3]);
        drop(finished);
        assert!(!path.exists());

        assert!(chunk(&first, true, false, vec![4]).unwrap().is_none());
        assert!(chunk(&first, false, false, Vec::new()).is_err());
        assert!(!path.exists());

        assert!(chunk(&first, true, false, vec![5]).unwrap().is_none());
        assert!(path.exists());
        let partial = upload.lock().unwrap().take().unwrap();
        drop(partial);
        assert!(!path.exists());

        assert!(chunk(&first, true, false, vec![6]).unwrap().is_none());
        let wrong_owner = write_attachment_chunk(&directory, &upload, first.clone(),
            "owned-b".into(), "image/png".into(), false, true, vec![7]).unwrap_err();
        assert!(wrong_owner.contains("invalid"));
        assert!(!path.exists());
        assert!(upload.lock().unwrap().is_none());
        assert!(chunk(&second, true, false, vec![8]).unwrap().is_none());
        let second_path = directory.join(format!(".attachment-upload-{second}"));
        let finished = chunk(&second, false, true, vec![9]).unwrap().unwrap();
        assert_eq!(std::fs::read(&second_path).unwrap(), vec![8, 9]);
        drop(finished);
        assert!(!second_path.exists());
        std::fs::remove_dir(&directory).unwrap();
    }

    fn store() -> Arc<SessionStore> {
        Arc::new(SessionStore::open_in_memory().unwrap())
    }

    fn profile(id: &str) -> RemoteAssemblyProfile {
        RemoteAssemblyProfile { id: id.into(), name: "Workbox".into(), ssh_target: "agent-workbox".into(), source_root: String::new(), default_cwd: String::new() }
    }

    fn session(owned_id: &str, profile_id: &str, last_activity_at_ms: i64) -> AgentConversationSessionRecord {
        AgentConversationSessionRecord {
            owned_id: owned_id.into(),
            execution_environment: ExecutionEnvironment::Remote,
            remote_profile_id: Some(profile_id.into()),
            provider: super::super::protocol::AgentConversationProvider::Codex,
            model: Some("gpt-test".into()),
            effort: None,
            cwd: "/work/project".into(),
            state: super::super::protocol::AgentRuntimeState::Suspended,
            suspended: true,
            created_at_ms: 1,
            last_activity_at_ms,
            active_turn_id: None,
            pending_permission: false,
            pending_input: false,
            native_session_id: Some(format!("native-{owned_id}")),
            meta: super::super::protocol::AgentConversationSessionMeta {
                title: Some(format!("Session {owned_id}")),
                ..Default::default()
            },
        }
    }

    #[test]
    fn cached_remote_rows_survive_restart_and_retain_routing() {
        let shared = store();
        let manager = RemoteConnectionManager::from_environment(
            Arc::new(|_| {}),
            Arc::new(|_| {}),
            shared.clone(),
        )
        .unwrap();
        manager
            .replace_cached_profile_sessions("workbox", &[session("remote-1", "workbox", 10)])
            .unwrap();

        let restarted = RemoteConnectionManager::from_environment(
            Arc::new(|_| {}),
            Arc::new(|_| {}),
            shared,
        )
        .unwrap();
        assert_eq!(restarted.cached_sessions(), vec![session("remote-1", "workbox", 10)]);
        assert!(restarted.owns("remote-1"));
        assert_eq!(restarted.connection_state("workbox"), RemoteConnectionState::Disconnected);
    }

    #[test]
    fn successful_profile_refresh_reconciles_only_that_machines_rows() {
        let manager = RemoteConnectionManager::from_environment(
            Arc::new(|_| {}),
            Arc::new(|_| {}),
            store(),
        )
        .unwrap();
        manager
            .replace_cached_profile_sessions("one", &[session("old", "one", 1)])
            .unwrap();
        manager
            .replace_cached_profile_sessions("two", &[session("keep", "two", 2)])
            .unwrap();
        manager
            .replace_cached_profile_sessions("one", &[session("new", "one", 3)])
            .unwrap();

        assert_eq!(
            manager
                .cached_sessions()
                .into_iter()
                .map(|session| session.owned_id)
                .collect::<Vec<_>>(),
            vec!["new", "keep"]
        );
        assert!(!manager.owns("old"));
        assert!(manager.owns("new"));
        assert!(manager.owns("keep"));

        manager.replace_cached_profile_sessions("one", &[]).unwrap();
        assert_eq!(
            manager
                .cached_sessions()
                .into_iter()
                .map(|session| session.owned_id)
                .collect::<Vec<_>>(),
            vec!["keep"]
        );
        assert!(!manager.owns("new"));
        assert!(manager.owns("keep"));
    }

    #[test]
    fn legacy_cached_rows_move_to_the_current_profile_before_reconciliation() {
        let manager = RemoteConnectionManager::from_environment(
            Arc::new(|_| {}), Arc::new(|_| {}), store(),
        ).unwrap();
        manager.replace_cached_profile_sessions(
            "agent-workbox", &[session("legacy", "agent-workbox", 2)],
        ).unwrap();
        manager.replace_cached_profile_sessions(
            "other", &[session("keep", "other", 1)],
        ).unwrap();

        manager.reassign_cached_profile_sessions("agent-workbox", "current").unwrap();

        let sessions = manager.cached_sessions();
        assert_eq!(sessions[0].remote_profile_id.as_deref(), Some("current"));
        assert_eq!(sessions[1].remote_profile_id.as_deref(), Some("other"));
        assert_eq!(manager.profile_for_owned_id("legacy").unwrap(), "current");

        manager.replace_cached_profile_sessions("current", &[]).unwrap();

        let sessions = manager.cached_sessions();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].owned_id, "keep");
        assert!(!manager.owns("legacy"));
        assert!(manager.owns("keep"));
    }

    #[tokio::test]
    async fn disconnect_keeps_profile_and_session_ownership_but_stops_transport() {
        let manager = RemoteConnectionManager::from_environment(Arc::new(|_| {}), Arc::new(|_| {}), store()).unwrap();
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
        let restarted = RemoteConnectionManager::from_environment(Arc::new(|_| {}), Arc::new(|_| {}), store()).unwrap();
        restarted.restore_profiles(&serde_json::to_string(&manager.environment().profiles).unwrap()).unwrap();
        assert_eq!(restarted.environment().profiles, vec![profile("saved")]);
        assert!(restarted.environment().ready_profile_ids.is_empty());
    }

    #[tokio::test]
    async fn restored_profiles_are_dormant_and_cancellation_releases_attempt() {
        let manager = RemoteConnectionManager::from_environment(Arc::new(|_| {}), Arc::new(|_| {}), store()).unwrap();
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
        let manager = RemoteConnectionManager::from_environment(Arc::new(|_| {}), Arc::new(|_| {}), store()).unwrap();
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

    #[tokio::test]
    async fn connection_state_reports_connected_reconnecting_and_disconnected() {
        let reports: Arc<Mutex<Vec<RemoteConnectionStatus>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = reports.clone();
        let manager = RemoteConnectionManager::from_environment(
            Arc::new(|_| {}),
            Arc::new(move |status| sink.lock().unwrap().push(status)),
            store(),
        )
        .unwrap();
        manager.register_profile(profile("saved")).unwrap();
        // A saved machine nobody connected is disconnected, not reconnecting.
        assert_eq!(manager.connection_state("saved"), RemoteConnectionState::Disconnected);
        assert_eq!(manager.connection_state("never-saved"), RemoteConnectionState::Disconnected);

        let (sender, _receiver) = mpsc::channel(1);
        {
            let mut state = manager.client.lock().unwrap();
            let client = state.clients.get_mut("saved").unwrap();
            client.requests = Some(sender);
        }
        // A live client that has not answered the readiness list is attempting.
        assert_eq!(manager.connection_state("saved"), RemoteConnectionState::Reconnecting);
        manager.client.lock().unwrap().clients["saved"].ready.store(true, Ordering::Release);
        assert_eq!(manager.connection_state("saved"), RemoteConnectionState::Connected);

        manager.disconnect_profile("saved");
        assert_eq!(manager.connection_state("saved"), RemoteConnectionState::Disconnected);
        {
            let attempt = ConnectingAttempt::start(&manager, "saved");
            assert_eq!(manager.connection_state("saved"), RemoteConnectionState::Reconnecting);
            drop(attempt);
        }
        assert_eq!(manager.connection_state("saved"), RemoteConnectionState::Disconnected);
        let sent = reports.lock().unwrap().clone();
        assert_eq!(
            sent,
            vec![
                RemoteConnectionStatus { profile_id: "saved".into(), state: RemoteConnectionState::Disconnected },
                RemoteConnectionStatus { profile_id: "saved".into(), state: RemoteConnectionState::Reconnecting },
                RemoteConnectionStatus { profile_id: "saved".into(), state: RemoteConnectionState::Disconnected },
            ]
        );
    }

    #[tokio::test]
    async fn removing_a_profile_mid_attempt_disconnects_and_refuses_the_stale_client() {
        let reports: Arc<Mutex<Vec<RemoteConnectionStatus>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = reports.clone();
        let manager = RemoteConnectionManager::from_environment(
            Arc::new(|_| {}),
            Arc::new(move |status| sink.lock().unwrap().push(status)),
            store(),
        )
        .unwrap();
        manager.register_profile(profile("saved")).unwrap();
        let attempt = ConnectingAttempt::start(&manager, "saved");
        assert_eq!(manager.connection_state("saved"), RemoteConnectionState::Reconnecting);

        // Removal during the attempt answers disconnected immediately.
        manager.remove_profile("saved");
        assert_eq!(manager.connection_state("saved"), RemoteConnectionState::Disconnected);

        // The pre-removal attempt finishes late: its client is refused, its own
        // actor is aborted, and the removed profile is not resurrected.
        let stale_actor = tokio::spawn(std::future::pending::<()>());
        let (stale_sender, _stale_receiver) = mpsc::channel(1);
        let stale = RemoteClient {
            requests: Some(stale_sender), profile: Some(profile("saved")),
            target_key: Some("owner@box:22".into()), task: Some(stale_actor.abort_handle()),
            ready: Arc::new(AtomicBool::new(true)),
        };
        let refused = manager.adopt_client(&attempt, Some(stale)).unwrap_err();
        assert!(refused.contains("was removed during the connection attempt"), "{refused}");
        assert!(stale_actor.await.unwrap_err().is_cancelled());
        assert!(!manager.client.lock().unwrap().clients.contains_key("saved"));
        assert_eq!(manager.connection_state("saved"), RemoteConnectionState::Disconnected);
        drop(attempt);
        assert_eq!(manager.connection_state("saved"), RemoteConnectionState::Disconnected);

        // The id is not tombstoned: a later add and connect works normally.
        manager.register_profile(profile("saved")).unwrap();
        let next = ConnectingAttempt::start(&manager, "saved");
        assert_eq!(manager.connection_state("saved"), RemoteConnectionState::Reconnecting);
        let (sender, _receiver) = mpsc::channel(1);
        manager
            .adopt_client(&next, Some(RemoteClient {
                requests: Some(sender), profile: Some(profile("saved")), target_key: None,
                task: None, ready: Arc::new(AtomicBool::new(true)),
            }))
            .unwrap();
        drop(next);
        assert_eq!(manager.connection_state("saved"), RemoteConnectionState::Connected);

        let status = |state| RemoteConnectionStatus { profile_id: "saved".into(), state };
        assert_eq!(
            reports.lock().unwrap().clone(),
            vec![
                status(RemoteConnectionState::Reconnecting),
                status(RemoteConnectionState::Disconnected),
                status(RemoteConnectionState::Reconnecting),
                status(RemoteConnectionState::Connected),
            ]
        );
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
            let actor = tokio::spawn(async move { client_loop(format!("ws://{address}"), "test-token".into(), &mut receiver, Arc::new(|_| {}), Some(1), actor_ready, &mut Some(reply), Arc::new(|| {})).await });
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
        let manager = RemoteConnectionManager::from_environment(Arc::new(|_| {}), Arc::new(|_| {}), store()).unwrap();
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

#[tauri::command]
pub async fn check_remote_provider_updates(
    remote: tauri::State<'_, RemoteConnectionManager>,
    profile_id: String,
) -> Result<super::providers::updates::ProviderUpdateStatus, String> {
    match remote.request_for_profile(&profile_id, RemoteCommand::CheckProviderUpdates).await? {
        RemoteResponse::ProviderUpdates(status) => Ok(status),
        _ => Err("The remote backend does not support provider updates; update its backend first".into()),
    }
}

#[tauri::command]
pub async fn install_remote_provider_updates(
    remote: tauri::State<'_, RemoteConnectionManager>,
    profile_id: String,
) -> Result<super::providers::updates::ProviderUpdateStatus, String> {
    match remote.request_for_profile(&profile_id, RemoteCommand::InstallProviderUpdates).await? {
        RemoteResponse::ProviderUpdates(status) => Ok(status),
        _ => Err("The remote backend does not support provider updates; update its backend first".into()),
    }
}

#[tauri::command]
pub async fn restart_remote_for_provider_updates(
    remote: tauri::State<'_, RemoteConnectionManager>,
    profile_id: String,
) -> Result<(), String> {
    match remote.request_for_profile(&profile_id, RemoteCommand::RestartForProviderUpdates).await? {
        RemoteResponse::Restarting => Ok(()),
        _ => Err("The remote backend returned an incompatible restart response".into()),
    }
}

#[cfg(test)]
mod provider_restart_tests {
    use super::*;

    #[tokio::test]
    async fn server_does_not_restart_without_an_accepted_request() {
        let requested = tokio::sync::Notify::new();
        let flushed = tokio::sync::Notify::new();
        assert!(tokio::time::timeout(Duration::from_millis(20),
            wait_for_provider_restart(&requested, &flushed)).await.is_err());
    }

    #[tokio::test]
    async fn accepted_restart_survives_missing_socket_acknowledgement() {
        let requested = tokio::sync::Notify::new();
        let flushed = tokio::sync::Notify::new();
        requested.notify_one();
        tokio::time::timeout(Duration::from_secs(3), wait_for_provider_restart(&requested, &flushed))
            .await.expect("a disconnected client must not prevent restart");
    }

    #[tokio::test]
    async fn flushed_restart_acknowledgement_does_not_wait_for_deadline() {
        let requested = tokio::sync::Notify::new();
        let flushed = tokio::sync::Notify::new();
        requested.notify_one();
        flushed.notify_one();
        tokio::time::timeout(Duration::from_millis(100), wait_for_provider_restart(&requested, &flushed))
            .await.expect("a flushed response should release restart immediately");
    }
}
