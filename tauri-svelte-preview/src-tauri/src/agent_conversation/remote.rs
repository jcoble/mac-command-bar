//! One authenticated WebSocket boundary for conversations owned by a remote machine.

use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::net::{TcpListener as StdTcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
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

/// Owns the remote socket, SSH tunnels, and compact routing data needed to reconnect.
#[derive(Clone)]
pub struct RemoteConnectionManager {
    client: Arc<Mutex<RemoteClientState>>,
    tunnel_processes: Arc<Mutex<HashMap<u32, Child>>>,
    remote_sessions: Arc<Mutex<HashMap<String, String>>>,
    next_request_id: Arc<AtomicU64>,
    event_sink: Arc<dyn Fn(AgentConversationEvent) + Send + Sync>,
}

#[derive(Default)]
struct RemoteClientState {
    clients: HashMap<String, RemoteClient>,
}

struct RemoteClient {
    requests: Option<mpsc::Sender<ClientRequest>>,
    profile: Option<RemoteAssemblyProfile>,
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
                },
            );
        let event_sink = manager.event_sink.clone();
        tauri::async_runtime::spawn(async move {
            client_loop(url, token, &mut request_rx, event_sink, None).await;
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
        RemoteAssemblyEnvironment { profiles }
    }

    pub fn shutdown(&self) {
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
            self.connect_profile(profile)?;
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
        self.connect_profile(profile.clone())?;
        Ok(vec![profile])
    }

    pub fn connect_profile(&self, profile: RemoteAssemblyProfile) -> Result<(), String> {
        validate_profile(&profile)?;
        let (request_tx, request_rx) = mpsc::channel(32);
        {
            let mut state = self
                .client
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.clients.insert(
                profile.id.clone(),
                RemoteClient {
                    requests: Some(request_tx),
                    profile: Some(profile.clone()),
                },
            );
        }
        let event_sink = self.event_sink.clone();
        let tunnel_processes = self.tunnel_processes.clone();
        tauri::async_runtime::spawn(profile_client_loop(
            profile,
            request_rx,
            event_sink,
            tunnel_processes,
        ));
        Ok(())
    }

    pub async fn deploy_profile(&self, profile: RemoteAssemblyProfile) -> Result<(), String> {
        validate_profile(&profile)?;
        let deployment = profile.clone();
        tauri::async_runtime::spawn_blocking(move || deploy_remote_service(&deployment))
            .await
            .map_err(|error| format!("Remote Assembly deployment task failed: {error}"))??;
        self.connect_profile(profile)
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
            .keys()
            .cloned()
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
pub async fn deploy_remote_assembly(
    manager: tauri::State<'_, AgentRuntimeManager>,
    remote: tauri::State<'_, RemoteConnectionManager>,
    profile: RemoteAssemblyProfile,
) -> Result<RemoteAssemblyEnvironment, super::protocol::CommandError> {
    remote
        .deploy_profile(profile.clone())
        .await
        .map_err(super::protocol::CommandError::from)?;
    let profiles_json = serde_json::to_string(&remote.environment().profiles)
        .map_err(|error| super::protocol::CommandError::from(error.to_string()))?;
    manager
        .write_app_setting(REMOTE_ASSEMBLY_PROFILES_SETTING_KEY, &profiles_json)
        .map_err(super::protocol::CommandError::from)?;
    Ok(remote.environment())
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
    for (label, value) in [
        ("Remote Assembly checkout", profile.source_root.as_str()),
        ("Remote working directory", profile.default_cwd.as_str()),
    ] {
        if !value.starts_with('/') || value.contains('\0') || value.len() > 4096 {
            return Err(format!("{label} must be an absolute path of at most 4096 bytes"));
        }
    }
    Ok(())
}

fn deploy_remote_service(profile: &RemoteAssemblyProfile) -> Result<(), String> {
    let mut child = ssh_command(&profile.ssh_target)
        .args(["sh", "-s", "--", &shell_quote(&profile.source_root)])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not start SSH: {error}"))?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| "Could not open SSH input".to_string())?
        .write_all(REMOTE_INSTALL_SCRIPT.as_bytes())
        .map_err(|error| format!("Could not send the Remote Assembly installer: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("Could not wait for SSH deployment: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if message.is_empty() {
        "Remote Assembly deployment failed".to_string()
    } else {
        format!("Remote Assembly deployment failed: {message}")
    })
}

async fn profile_client_loop(
    profile: RemoteAssemblyProfile,
    mut requests: mpsc::Receiver<ClientRequest>,
    event_sink: Arc<dyn Fn(AgentConversationEvent) + Send + Sync>,
    tunnel_processes: Arc<Mutex<HashMap<u32, Child>>>,
) {
    while !requests.is_closed() {
        let target = profile.ssh_target.clone();
        let processes = tunnel_processes.clone();
        let connection =
            tauri::async_runtime::spawn_blocking(move || open_remote_tunnel(&target, processes))
                .await;
        let tunnel = match connection {
            Ok(Ok(tunnel)) => tunnel,
            Ok(Err(error)) => {
                fail_queued_requests(&mut requests, error);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
            Err(error) => {
                fail_queued_requests(
                    &mut requests,
                    format!("Remote Assembly connection task failed: {error}"),
                );
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };
        let url = format!("ws://127.0.0.1:{}/assembly", tunnel.local_port);
        let token = tunnel.token.clone();
        client_loop(url, token, &mut requests, event_sink.clone(), Some(4)).await;
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

fn open_remote_tunnel(
    target: &str,
    processes: Arc<Mutex<HashMap<u32, Child>>>,
) -> Result<RemoteTunnel, String> {
    let token = read_remote_token(target)?;
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
        "-o",
        "ServerAliveInterval=15",
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
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", local_port)).is_ok() {
            return Ok(RemoteTunnel {
                pid,
                processes,
                local_port,
                token,
            });
        }
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
        std::thread::sleep(Duration::from_millis(100));
    }
    stop_remote_tunnel(&processes, pid);
    Err("Remote Assembly tunnel did not become ready".to_string())
}

fn read_remote_token(target: &str) -> Result<String, String> {
    let mut child = ssh_command(target)
        .args(["sh", "-s"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not start SSH: {error}"))?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| "Could not open SSH input".to_string())?
        .write_all(b"set -eu\nsed -n 's/^ASSEMBLY_SERVER_TOKEN=//p' \"$HOME/.config/assembly/server.env\"\n")
        .map_err(|error| format!("Could not request the Remote Assembly token: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("Could not read the Remote Assembly token: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Could not read the Remote Assembly token: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let token = String::from_utf8_lossy(&output.stdout)
        .lines()
        .last()
        .unwrap_or_default()
        .trim()
        .to_string();
    if token.len() < 32 {
        return Err(
            "The Remote Assembly token is missing or invalid; deploy the server again".to_string(),
        );
    }
    Ok(token)
}

fn ssh_command(target: &str) -> Command {
    let mut command = Command::new("ssh");
    command.args([
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=10",
        "--",
        target,
    ]);
    command
}

const REMOTE_INSTALL_SCRIPT: &str = r#"set -eu
source_root=$1
crate_dir="$source_root/tauri-svelte-preview/src-tauri"
bridge_source="$source_root/tauri-svelte-preview/tools/codex-acp-bridge/bridge.mjs"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
for command in cargo node codex sha256sum systemctl openssl; do
  command -v "$command" >/dev/null 2>&1 || { echo "Required command is missing: $command" >&2; exit 1; }
done
[ -f "$crate_dir/Cargo.toml" ] || { echo "Assembly Rust source was not found at $crate_dir" >&2; exit 1; }
[ -f "$bridge_source" ] || { echo "Codex ACP bridge was not found at $bridge_source" >&2; exit 1; }
(cd "$crate_dir" && cargo build --release --bin mac-command-bar-webview-preview)
install -d "$HOME/.local/bin" "$HOME/.local/share/assembly" "$HOME/.config/assembly" "$HOME/.config/systemd/user"
install -m 755 "$crate_dir/target/release/mac-command-bar-webview-preview" "$HOME/.local/bin/assembly-remote-server"
install -m 644 "$bridge_source" "$HOME/.local/share/assembly/codex-acp-bridge.mjs"
codex_path=$(command -v codex)
node_path=$(command -v node)
claude_path=$(command -v claude-agent-acp 2>/dev/null || find /opt -maxdepth 4 -path '*/bin/claude-agent-acp' -print -quit 2>/dev/null || true)
[ -n "$claude_path" ] || { echo "Required command is missing: claude-agent-acp" >&2; exit 1; }
cat >"$HOME/.local/bin/assembly-codex-acp" <<EOF
#!/bin/sh
export CODEX_BIN="$codex_path"
exec "$node_path" "$HOME/.local/share/assembly/codex-acp-bridge.mjs" "\$@"
EOF
cat >"$HOME/.local/bin/assembly-claude-acp" <<EOF
#!/bin/sh
exec "$claude_path" "\$@"
EOF
chmod 755 "$HOME/.local/bin/assembly-codex-acp" "$HOME/.local/bin/assembly-claude-acp"
token=$(sed -n 's/^ASSEMBLY_SERVER_TOKEN=//p' "$HOME/.config/assembly/server.env" 2>/dev/null || true)
[ "${#token}" -ge 32 ] || token=$(openssl rand -hex 32)
codex_hash=$(sha256sum "$HOME/.local/bin/assembly-codex-acp" | awk '{print $1}')
claude_hash=$(sha256sum "$HOME/.local/bin/assembly-claude-acp" | awk '{print $1}')
cat >"$HOME/.config/assembly/server.env" <<EOF
ASSEMBLY_SERVER_BIND=127.0.0.1:7777
ASSEMBLY_SERVER_TOKEN=$token
ASSEMBLY_SERVER_DATA_DIR=$HOME/.local/share/assembly
MCB_CODEX_ACP_PATH=$HOME/.local/bin/assembly-codex-acp
MCB_CODEX_ACP_SHA256=$codex_hash
MCB_CLAUDE_AGENT_ACP_PATH=$HOME/.local/bin/assembly-claude-acp
MCB_CLAUDE_AGENT_ACP_SHA256=$claude_hash
PATH=$(dirname "$node_path"):$(dirname "$codex_path"):$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin
EOF
cat >"$HOME/.config/systemd/user/assembly-remote.service" <<EOF
[Unit]
Description=Assembly Remote Service
After=network.target

[Service]
Type=simple
EnvironmentFile=$HOME/.config/assembly/server.env
ExecStart=$HOME/.local/bin/assembly-remote-server --assembly-server
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
EOF
systemctl --user daemon-reload
systemctl --user enable --now assembly-remote.service
systemctl --user restart assembly-remote.service
systemctl --user is-active --quiet assembly-remote.service
"#;

async fn client_loop(
    url: String,
    token: String,
    requests: &mut mpsc::Receiver<ClientRequest>,
    event_sink: Arc<dyn Fn(AgentConversationEvent) + Send + Sync>,
    reconnect_tunnel_after_failures: Option<u8>,
) {
    let cursors = Arc::new(Mutex::new(BTreeMap::<String, i64>::new()));
    let mut failed_connections = 0_u8;
    loop {
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
        let connected = tokio_tungstenite::connect_async(request).await;
        let Ok((mut socket, _)) = connected else {
            failed_connections = failed_connections.saturating_add(1);
            if reconnect_tunnel_after_failures.is_some_and(|limit| failed_connections >= limit) {
                return;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        };
        failed_connections = 0;
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
                                let _ = reply.send(Err(format!("Unsupported remote protocol {protocol_version}")));
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

fn parse_server_frame(message: TungsteniteMessage) -> Option<ServerFrame> {
    let TungsteniteMessage::Text(text) = message else {
        return None;
    };
    serde_json::from_str(text.as_str()).ok()
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
        axum::serve(listener, app)
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
                state.manager.cancel_snapshot(id);
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
        RemoteCommand::Snapshot {
            owned_id,
            request_id,
        } => manager
            .latest_snapshot(&owned_id, request_id)
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteDirectoryListing {
    path: String,
    directories: Vec<String>,
    truncated: bool,
}

#[tauri::command]
pub async fn list_remote_directories(
    ssh_target: String,
    path: String,
) -> Result<RemoteDirectoryListing, super::protocol::CommandError> {
    tauri::async_runtime::spawn_blocking(move || read_remote_directories(&ssh_target, &path))
        .await
        .map_err(|error| super::protocol::CommandError::from(error.to_string()))?
        .map_err(super::protocol::CommandError::from)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn directory_listing_script(path: &str) -> Result<String, String> {
    if path.contains('\0') || path.len() > 4096 {
        return Err("Folder path is invalid or too long".into());
    }
    let destination = if path.is_empty() || path == "~" {
        "\"$HOME\"".to_string()
    } else if let Some(rest) = path.strip_prefix("~/") {
        format!("\"$HOME\"/{}", shell_quote(rest))
    } else if path.starts_with('/') {
        shell_quote(path)
    } else {
        return Err("Enter an absolute path or a path beginning with ~/".into());
    };
    Ok(format!(r#"set -eu
LC_ALL=C; export LC_ALL
cd -- {destination}
[ -r . ] && [ -x . ] || {{ echo 'Permission denied reading this folder' >&2; exit 1; }}
printf '%s\0' "$(pwd -P)"
count=0
for entry in ./* ./.[!.]* ./..?*; do
  [ -d "$entry" ] || continue
  printf '%s\0' "${{entry#./}}"
  count=$((count + 1))
  [ "$count" -lt 501 ] || break
done
"#))
}

fn parse_directory_listing(bytes: &[u8]) -> Result<RemoteDirectoryListing, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "Folder names are not valid UTF-8")?;
    let mut parts = text.split_terminator('\0');
    let path = parts.next().filter(|path| path.starts_with('/'))
        .ok_or("SSH returned an invalid folder listing")?.to_string();
    let mut directories = parts.map(str::to_string).collect::<Vec<_>>();
    let truncated = directories.len() > 500;
    directories.truncate(500);
    directories.sort();
    Ok(RemoteDirectoryListing { path, directories, truncated })
}

fn read_remote_directories(target: &str, path: &str) -> Result<RemoteDirectoryListing, String> {
    validate_ssh_target(target)?;
    let script = directory_listing_script(path)?;
    // One request owns one SSH process; never create a persistent control master.
    let mut command = Command::new("ssh");
    command.args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=10",
        "-o", "ControlMaster=no", "-o", "ControlPath=none", "--", target, &script]);
    let output = crate::bounded_process::output(&mut command, "Remote folder listing", Duration::from_secs(15))
        .map_err(|error| format!("Could not browse remote folders: {error}"))?;
    if !output.status.success() {
        return Err(format!("Could not browse remote folders. Check the SSH destination and your SSH key/configuration. {}",
            String::from_utf8_lossy(&output.stderr).trim()));
    }
    parse_directory_listing(&output.stdout)
}

fn validate_ssh_target(target: &str) -> Result<(), String> {
    if target.is_empty() || target.starts_with('-')
        || !target.bytes().all(|value| value.is_ascii_alphanumeric() || b"._@-".contains(&value)) {
        return Err("SSH destination may contain only letters, numbers, '.', '_', '@', and '-'".into());
    }
    Ok(())
}

#[cfg(test)]
mod directory_tests {
    use super::*;

    #[test]
    fn lists_only_directories_and_quotes_paths() {
        let root = std::env::temp_dir().join(format!("assembly-directory-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let folder = root.join("space ' quote $(false)");
        std::fs::create_dir(&folder).unwrap();
        for name in ["visible", ".hidden", "unicode-é", "line\nbreak"] {
            std::fs::create_dir(folder.join(name)).unwrap();
        }
        std::fs::write(folder.join("file"), "not a folder").unwrap();
        std::os::unix::fs::symlink(folder.join("visible"), folder.join("link")).unwrap();
        let output = Command::new("sh").args(["-c", &directory_listing_script(folder.to_str().unwrap()).unwrap()]).output().unwrap();
        assert!(output.status.success());
        let listing = parse_directory_listing(&output.stdout).unwrap();
        assert_eq!(listing.directories, [".hidden", "line\nbreak", "link", "unicode-é", "visible"]);
        assert!(!listing.truncated);
        assert_eq!(listing.path, folder.canonicalize().unwrap().to_str().unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn limits_output_and_rejects_invalid_inputs() {
        let data = format!("/tmp\0{}", (0..501).map(|n| format!("folder-{n}\0")).collect::<String>());
        let listing = parse_directory_listing(data.as_bytes()).unwrap();
        assert!(listing.truncated);
        assert_eq!(listing.directories.len(), 500);
        for target in ["-oProxyCommand=x", "host;false", "", "a b"] {
            assert!(validate_ssh_target(target).is_err());
        }
        assert!(validate_ssh_target("user@agent-workbox").is_ok());
        assert!(directory_listing_script("relative").is_err());
        assert!(directory_listing_script("/bad\0path").is_err());
        assert!(parse_directory_listing(b"banner\0").is_err());
        let output = Command::new("sh").args(["-c", &directory_listing_script("/nonexistent-assembly-directory-test").unwrap()]).output().unwrap();
        assert!(!output.status.success());
    }

    #[test]
    #[ignore = "requires the owner's SSH-accessible workbox"]
    fn live_workbox_directory_listing() {
        let listing = read_remote_directories("agent-workbox", "~").unwrap();
        assert!(listing.path.starts_with("/home/"));
        assert!(listing.directories.iter().any(|name| name == "dev"));
        let nested = read_remote_directories("agent-workbox", &format!("{}/dev/work", listing.path)).unwrap();
        assert!(nested.directories.iter().any(|name| name == "worktrees"));
        eprintln!("SSH browse verified: {} then {}", listing.path, nested.path);
    }
}
