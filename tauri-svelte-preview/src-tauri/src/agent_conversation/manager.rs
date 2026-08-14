use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mcb_core::session_store::{AnnotationRow, EventRow, SessionRow, SessionStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

use super::handoff::{
    AgentConversationHandoffDirection, AgentConversationHandoffMode, AgentConversationHandoffPhase,
    AgentConversationHandoffReceipt, AgentConversationHandoffRequest,
};
use super::protocol::{
    AgentApprovalDecision, AgentApprovalResponse, AgentCapabilities, AgentCommandDescriptor,
    AgentConversationConfigState, AgentConversationConnection, AgentConversationEvent,
    AgentConversationPayload, AgentConversationProvider, AgentConversationSessionMeta,
    AgentConversationSessionRecord, AgentConversationSnapshot, AgentEvent, AgentEventType,
    AgentExecutionOwner, AgentImplementation, AgentInteractionCapabilities, AgentNativeSessionMode,
    AgentPromptCapabilities, AgentRequestIdentity, AgentRuntimeState, AgentSessionCapabilities,
    AgentUserInputResponse, AgentWriterLease, AgentWriterLeaseOwner, AgentWriterLeaseTransition,
    ApprovalState, ConversationConnectionState, EnsureAgentConversationRequest, PlanItem,
    SetAgentConversationConfigRequest, TerminalProjectionPayload, ToolState,
    UpdateAgentConversationSessionMetaRequest,
};
use super::providers::acp_client::{AcpInbound, AcpTransport};
use super::providers::process::validated_conversation_cwd;
use super::providers::process::SidecarEnvironment;
use super::providers::{
    AcpRuntimeAdapter, AgentConfigValue, AgentConversationConfigUpdate, AgentPrompt,
    AgentRuntimeAdapter, AgentRuntimeError, GeneratedText, InitializeAgentInput, LoadAgentSession,
    NewAgentSession, PermissionResponse, ProviderRegistry, StructuredRuntimeHandle,
};
use super::transcript::{self, CodexChildRollout};
use tokio::sync::mpsc::{self, UnboundedSender};

const SNAPSHOT_EVENT_CAP: usize = 2_000;
const STORE_EVENT_CAP: u32 = 10_000;
const SESSION_TITLE_CHAR_CAP: usize = 64;
const CHILD_ROLLOUT_SCAN_INTERVAL: Duration = Duration::from_secs(10);
const MAX_THINKING_TOKENS_ENV: &str = "MAX_THINKING_TOKENS";
const CLAUDE_SESSION_EFFORTS: [(&str, &str); 4] = [
    ("low", "4000"),
    ("medium", "12000"),
    ("high", "32000"),
    ("max", "63999"),
];

pub type ConversationEmitter = std::sync::Arc<dyn Fn(AgentConversationEvent) + Send + Sync>;

#[derive(Clone, Debug)]
struct PermissionOption {
    option_id: String,
    kind: String,
}

#[derive(Clone, Debug)]
struct PendingPermission {
    wire_id: Value,
    options: Vec<PermissionOption>,
    summary: String,
}

#[derive(Clone, Debug)]
struct PendingUserInput {
    wire_id: Value,
}

enum PermissionSelection {
    Decision(AgentApprovalDecision),
    Option(String),
}

enum OrderedSessionEvent {
    PromptResult {
        turn_id: String,
        result: Result<Value, AgentRuntimeError>,
    },
    ApprovalResolved {
        request_id: String,
        state: ApprovalState,
        summary: String,
    },
    UserInputResolved {
        request_id: String,
        cancelled: bool,
    },
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum AdapterPoolKey {
    Shared(AgentConversationProvider),
    Isolated(AgentConversationProvider, String),
}

struct AdapterPoolEntry {
    runtime: Arc<AsyncMutex<StructuredRuntimeHandle>>,
    transport: Arc<AcpTransport>,
    capabilities: AgentCapabilities,
    members: HashSet<String>,
}

pub struct ManagedAgentSession {
    pub owned_id: String,
    pub provider: AgentConversationProvider,
    pub provider_instance_id: String,
    pub native_session_id: Option<String>,
    native_session_mode: AgentNativeSessionMode,
    pub generation: u64,
    pub owner: AgentExecutionOwner,
    pub state: AgentRuntimeState,
    pub capabilities: AgentCapabilities,
    pub next_sequence: u64,
    pub active_turn_id: Option<String>,
    prompt_once_active: bool,
    pub runtime: Option<Arc<AsyncMutex<StructuredRuntimeHandle>>>,
    pool_key: Option<AdapterPoolKey>,
    transport: Option<Arc<AcpTransport>>,
    pub recent_events: VecDeque<AgentEvent>,
    pub writer_lease: AgentWriterLease,
    pub writer_lease_transition: Option<AgentWriterLeaseTransition>,
    permission_requests: HashMap<String, PendingPermission>,
    next_permission_id: u64,
    user_input_requests: HashMap<String, PendingUserInput>,
    next_user_input_id: u64,
    ordered_events: Option<UnboundedSender<OrderedSessionEvent>>,
    cwd: String,
    spawn_reasoning_effort: Option<String>,
    config: AgentConversationConfigState,
    connection: AgentConversationConnection,
    store: Arc<SessionStore>,
    created_at_ms: u128,
    last_activity_ms: u128,
    live_tool_calls: HashSet<String>,
    background_work: HashSet<String>,
    child_rollout_scan: Option<tokio::task::JoinHandle<()>>,
    child_rollout_parent_path: Option<PathBuf>,
    codex_children: HashMap<String, CodexChildRollout>,
    rail_meta: AgentConversationSessionMeta,
    suspending: bool,
}

/// Holds only the in-memory fields that an event is allowed to advance.
///
/// Building this copy first keeps the live session unchanged until SQLite has
/// accepted the matching session row and event row.
struct SessionEventCandidate {
    native_session_id: Option<String>,
    native_session_mode: AgentNativeSessionMode,
    state: AgentRuntimeState,
    owner: AgentExecutionOwner,
    capabilities: AgentCapabilities,
    next_sequence: u64,
    connection: AgentConversationConnection,
    writer_owner: AgentWriterLeaseOwner,
    last_activity_ms: u128,
    live_tool_calls: HashSet<String>,
}

#[derive(Clone, Copy)]
struct SessionLifecycleUpdate {
    state: AgentRuntimeState,
    connection_state: ConversationConnectionState,
    owner: AgentExecutionOwner,
    writer_owner: AgentWriterLeaseOwner,
    native_session_mode: Option<AgentNativeSessionMode>,
}

impl SessionEventCandidate {
    /// Copies the small event-owned portion of a live session for a pending write.
    fn from_session(session: &ManagedAgentSession) -> Self {
        Self {
            native_session_id: session.native_session_id.clone(),
            native_session_mode: session.native_session_mode,
            state: session.state,
            owner: session.owner,
            capabilities: session.capabilities.clone(),
            next_sequence: session.next_sequence,
            connection: session.connection.clone(),
            writer_owner: session.writer_lease.owner,
            last_activity_ms: session.last_activity_ms,
            live_tool_calls: session.live_tool_calls.clone(),
        }
    }

    /// Validates and stages a lifecycle transition without changing live memory.
    fn transition_lifecycle(
        &mut self,
        state: AgentRuntimeState,
        connection_state: ConversationConnectionState,
        owner: AgentExecutionOwner,
        writer_owner: AgentWriterLeaseOwner,
    ) -> Result<(), String> {
        let terminal = self.state == AgentRuntimeState::Closed
            || self.connection.state == ConversationConnectionState::Closed;
        if terminal
            && (state != AgentRuntimeState::Closed
                || connection_state != ConversationConnectionState::Closed)
        {
            return Err("Closed conversation sessions cannot be revived".to_string());
        }
        self.state = state;
        self.connection.state = connection_state;
        self.owner = owner;
        self.writer_owner = writer_owner;
        Ok(())
    }

    /// Publishes a committed candidate to the live session in one replacement step.
    /// The live session owns the next unused sequence and advances it only here,
    /// after SQLite accepts the event that used the previous value.
    fn apply(self, session: &mut ManagedAgentSession) {
        session.native_session_id = self.native_session_id;
        session.native_session_mode = self.native_session_mode;
        session.state = self.state;
        session.owner = self.owner;
        session.capabilities = self.capabilities;
        session.next_sequence = self.next_sequence;
        session.connection = self.connection;
        session.writer_lease.owner = self.writer_owner;
        session.last_activity_ms = self.last_activity_ms;
        session.live_tool_calls = self.live_tool_calls;
    }
}

impl ManagedAgentSession {
    fn transition_lifecycle(
        &mut self,
        state: AgentRuntimeState,
        connection_state: ConversationConnectionState,
        owner: AgentExecutionOwner,
        writer_owner: AgentWriterLeaseOwner,
    ) -> Result<(), String> {
        let terminal = self.state == AgentRuntimeState::Closed
            || self.connection.state == ConversationConnectionState::Closed;
        if terminal
            && (state != AgentRuntimeState::Closed
                || connection_state != ConversationConnectionState::Closed)
        {
            return Err("Closed conversation sessions cannot be revived".to_string());
        }
        self.state = state;
        self.connection.state = connection_state;
        self.owner = owner;
        self.writer_lease.owner = writer_owner;
        Ok(())
    }
}

pub(crate) struct HandoffContext {
    pub previous_owner: AgentWriterLeaseOwner,
    pub owner: AgentWriterLeaseOwner,
    pub native_session_id: Option<String>,
}

#[derive(Clone)]
pub struct AgentRuntimeManager {
    /// Live process and request overlay. Durable identity, lifecycle state, and
    /// conversation events are always reconstructed from `store` at startup.
    sessions: Arc<Mutex<HashMap<String, ManagedAgentSession>>>,
    providers: Arc<ProviderRegistry>,
    emitter: Arc<Mutex<Option<ConversationEmitter>>>,
    activation_locks: Arc<Mutex<HashMap<String, Arc<AsyncMutex<()>>>>>,
    store: Arc<SessionStore>,
    adapter_pools: Arc<AsyncMutex<HashMap<AdapterPoolKey, AdapterPoolEntry>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentResourceRoot {
    pub pid: u32,
    pub owned_id: String,
    pub provider: AgentConversationProvider,
    pub cwd: String,
}

impl Default for AgentRuntimeManager {
    fn default() -> Self {
        Self::new(ProviderRegistry::default())
    }
}

impl AgentRuntimeManager {
    pub fn new(providers: ProviderRegistry) -> Self {
        let store = SessionStore::open_in_memory()
            .expect("the in-memory session store must initialize for an unmanaged manager");
        Self::with_store(providers, store).expect("the empty session store must recover")
    }

    pub fn open(providers: ProviderRegistry, database_path: &Path) -> Result<Self, String> {
        let store = SessionStore::open(database_path).map_err(|error| error.to_string())?;
        if let Some(directory) = database_path.parent() {
            super::legacy_import::import_if_store_empty(&store, directory)?;
        }
        Self::with_store(providers, store)
    }

    pub fn with_store(providers: ProviderRegistry, store: SessionStore) -> Result<Self, String> {
        let store = Arc::new(store);
        let sessions = recover_sessions_from_store(&store)?;
        Ok(Self {
            sessions: Arc::new(Mutex::new(sessions)),
            providers: Arc::new(providers),
            emitter: Arc::new(Mutex::new(None)),
            activation_locks: Arc::new(Mutex::new(HashMap::new())),
            store,
            adapter_pools: Arc::new(AsyncMutex::new(HashMap::new())),
        })
    }

    pub fn set_session_draft(&self, owned_id: &str, text: &str) -> Result<(), String> {
        self.store
            .set_draft(owned_id, text)
            .map_err(|error| error.to_string())
    }

    pub fn get_session_draft(&self, owned_id: &str) -> Result<Option<String>, String> {
        self.store
            .get_draft(owned_id)
            .map_err(|error| error.to_string())
    }

    pub fn clear_session_draft(&self, owned_id: &str) -> Result<(), String> {
        self.store
            .clear_draft(owned_id)
            .map_err(|error| error.to_string())
    }

    async fn release_pool_scope(
        &self,
        pool_key: &AdapterPoolKey,
        owned_id: &str,
        native_session_id: Option<&str>,
        close_native_session: bool,
    ) -> Result<(), String> {
        let mut pools = self.adapter_pools.lock().await;
        let Some(pool) = pools.get_mut(pool_key) else {
            return Ok(());
        };
        pool.members.remove(owned_id);
        if close_native_session {
            if let Some(native_session_id) = native_session_id {
                pool.runtime
                    .lock()
                    .await
                    .close_native_session(native_session_id)
                    .await
                    .map_err(|error| error.to_string())?;
            }
        }
        if pool.members.is_empty() {
            let runtime = Arc::clone(&pool.runtime);
            if close_native_session {
                runtime
                    .lock()
                    .await
                    .close_session()
                    .await
                    .map_err(|error| error.to_string())?;
            } else {
                runtime
                    .lock()
                    .await
                    .detach_session()
                    .await
                    .map_err(|error| error.to_string())?;
            }
            pools.remove(pool_key);
        }
        Ok(())
    }

    pub fn providers(&self) -> &ProviderRegistry {
        &self.providers
    }

    pub fn add_session_annotation(
        &self,
        owned_id: &str,
        url: &str,
        rect_json: &str,
        note: &str,
    ) -> Result<AnnotationRow, String> {
        let id = self
            .store
            .add_annotation(owned_id, url, rect_json, note)
            .map_err(|error| error.to_string())?;
        self.store
            .list_annotations(owned_id)
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|annotation| annotation.id == id)
            .ok_or_else(|| "The saved session annotation was not found".to_string())
    }

    pub fn list_session_annotations(&self, owned_id: &str) -> Result<Vec<AnnotationRow>, String> {
        self.store
            .list_annotations(owned_id)
            .map_err(|error| error.to_string())
    }

    pub fn delete_session_annotation(&self, id: i64) -> Result<(), String> {
        self.store
            .delete_annotation(id)
            .map_err(|error| error.to_string())
    }

    pub fn set_emitter(&self, emitter: ConversationEmitter) {
        let mut current = self
            .emitter
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *current = Some(emitter);
    }

    /// Snapshot the provider processes that this registry currently owns.
    /// The async runtime mutex is intentionally sampled with `try_lock`: a
    /// resource refresh must never block an agent turn. A session that is in a
    /// transition simply appears on the next refresh.
    pub fn resource_roots(&self) -> Vec<AgentResourceRoot> {
        let sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        sessions
            .values()
            .filter_map(|session| {
                let runtime = session.runtime.as_ref()?;
                let runtime = runtime.try_lock().ok()?;
                let pid = runtime.process_id()?;
                Some(AgentResourceRoot {
                    pid,
                    owned_id: session.owned_id.clone(),
                    provider: session.provider,
                    cwd: session.cwd.clone(),
                })
            })
            .collect()
    }

    /// Returns the capability snapshot advertised by the active ACP session. WorkflowEngine uses
    /// this to map role policy onto provider-owned config options without guessing option ids.
    pub fn capabilities(
        &self,
        owned_id: &str,
        generation: u64,
    ) -> Result<AgentCapabilities, String> {
        let sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(current_session(&sessions, owned_id, generation)?
            .capabilities
            .clone())
    }

    /// Return the current capability snapshot for a session-owned identifier.
    /// The native read command is intentionally generation-free because it is a
    /// snapshot read, not a mutating operation; the map contains only the
    /// current generation for each owned session.
    pub fn capabilities_for_owned_id(&self, owned_id: &str) -> Result<AgentCapabilities, String> {
        let sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        sessions
            .get(owned_id)
            .map(|session| session.capabilities.clone())
            .ok_or_else(|| "Conversation session was not found".to_string())
    }

    /// Ensure an app-owned ACP session while serializing it with activation.
    /// Replacing a failed or changed generation detaches the previous transport
    /// before the caller can activate the new one.
    pub async fn ensure_async(
        &self,
        request: EnsureAgentConversationRequest,
    ) -> Result<AgentConversationConnection, String> {
        let owned_id = required_id(&request.owned_id, "Owned session id")?;
        let activation_lock = self.activation_lock(&owned_id)?;
        let _activation = activation_lock.lock().await;
        let (connection, previous_runtime) = self.ensure_inner(request)?;
        if let Some((runtime, transport)) = previous_runtime {
            // Stop through the transport directly so a one-shot prompt holding
            // the runtime mutex cannot prevent the old generation from being
            // shut down before the replacement activates.
            transport.stop().await;
            drop(runtime);
        }
        Ok(connection)
    }

    fn ensure_inner(
        &self,
        request: EnsureAgentConversationRequest,
    ) -> Result<
        (
            AgentConversationConnection,
            Option<(Arc<AsyncMutex<StructuredRuntimeHandle>>, Arc<AcpTransport>)>,
        ),
        String,
    > {
        let owned_id = required_id(&request.owned_id, "Owned session id")?;
        let cwd = validated_conversation_cwd(&request.cwd)?
            .display()
            .to_string();
        let native_session_id = normalized_optional_id(request.native_session_id);
        let reasoning_effort =
            normalized_session_start_effort(request.provider, request.reasoning_effort)?;
        if request.native_session_mode == AgentNativeSessionMode::Load
            && native_session_id.is_none()
        {
            return Err("A native session id is required to load a stopped session".to_string());
        }
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(current) = sessions.get(&owned_id) {
            if current.provider == request.provider
                && current.cwd == cwd
                && native_session_id
                    .as_ref()
                    .is_none_or(|native_id| current.native_session_id.as_ref() == Some(native_id))
                && current.native_session_mode == request.native_session_mode
                && current.spawn_reasoning_effort == reasoning_effort
                && current.connection.state != ConversationConnectionState::Failed
            {
                return Ok((current.connection.clone(), None));
            }
        }
        let mut prior = sessions.remove(&owned_id);
        if let Some(task) = prior
            .as_mut()
            .and_then(|session| session.child_rollout_scan.take())
        {
            task.abort();
        }
        let previous_runtime = prior
            .as_ref()
            .and_then(|session| Some((session.runtime.clone()?, session.transport.clone()?)));
        let generation = prior
            .as_ref()
            .map(|session| session.generation.saturating_add(1))
            .unwrap_or(1);
        // A replacement generation continues the same owned event journal, so
        // it inherits the prior session's next unused sequence instead of
        // starting again at one.
        let next_sequence = prior.as_ref().map_or(1, |session| session.next_sequence);
        let connection = AgentConversationConnection {
            owned_id: owned_id.clone(),
            provider: request.provider,
            generation,
            native_session_id,
            state: ConversationConnectionState::Connecting,
            config: AgentConversationConfigState::default(),
        };
        let provider_instance_id = format!("{}-{generation}", provider_id(request.provider));
        let created_at_ms = timestamp_millis();
        sessions.insert(
            owned_id.clone(),
            ManagedAgentSession {
                owned_id: owned_id.clone(),
                provider: request.provider,
                provider_instance_id,
                native_session_id: connection.native_session_id.clone(),
                native_session_mode: request.native_session_mode,
                generation,
                owner: AgentExecutionOwner::Stopped,
                state: AgentRuntimeState::Starting,
                capabilities: empty_capabilities(request.provider),
                next_sequence,
                active_turn_id: None,
                prompt_once_active: false,
                runtime: None,
                pool_key: None,
                transport: None,
                recent_events: VecDeque::new(),
                writer_lease: AgentWriterLease {
                    owned_id,
                    generation,
                    owner: AgentWriterLeaseOwner::None,
                },
                writer_lease_transition: None,
                permission_requests: HashMap::new(),
                next_permission_id: 0,
                user_input_requests: HashMap::new(),
                next_user_input_id: 0,
                ordered_events: None,
                cwd,
                spawn_reasoning_effort: reasoning_effort,
                config: connection.config.clone(),
                connection: connection.clone(),
                store: Arc::clone(&self.store),
                created_at_ms,
                last_activity_ms: created_at_ms,
                live_tool_calls: HashSet::new(),
                background_work: HashSet::new(),
                child_rollout_scan: None,
                child_rollout_parent_path: None,
                codex_children: HashMap::new(),
                rail_meta: AgentConversationSessionMeta::default(),
                suspending: false,
            },
        );
        let session = sessions
            .get(&connection.owned_id)
            .ok_or_else(|| "Conversation session was not inserted".to_string())?;
        persist_session(session)?;
        Ok((connection, previous_runtime))
    }

    pub async fn activate(
        &self,
        owned_id: &str,
        generation: u64,
    ) -> Result<AgentConversationConnection, String> {
        let _lifecycle = self.lifecycle_guard(owned_id).await?;
        self.activate_locked(owned_id, generation).await
    }

    pub(crate) async fn activate_locked(
        &self,
        owned_id: &str,
        generation: u64,
    ) -> Result<AgentConversationConnection, String> {
        let (
            provider,
            cwd,
            native_session_id,
            native_session_mode,
            reasoning_effort,
            was_suspended,
        ) = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = lifecycle_session_mut(&mut sessions, owned_id, generation)?;
            session.last_activity_ms = timestamp_millis();
            if session.runtime.is_some() {
                if session.connection.state == ConversationConnectionState::Connected
                    && session.state != AgentRuntimeState::Failed
                {
                    return Ok(session.connection.clone());
                }
                return Err(
                    "The structured session must be ensured again before it can reactivate"
                        .to_string(),
                );
            }
            if !matches!(
                session.state,
                AgentRuntimeState::Starting | AgentRuntimeState::Suspended
            ) {
                return Err("Conversation is not ready to activate".to_string());
            }
            (
                session.provider,
                std::path::PathBuf::from(&session.cwd),
                session.native_session_id.clone(),
                session.native_session_mode,
                session.spawn_reasoning_effort.clone(),
                session.state == AgentRuntimeState::Suspended,
            )
        };
        let mut expected_native_session_id = native_session_id.clone();
        let can_start_fresh = expected_native_session_id.is_some()
            && self
                .store
                .first_user_message_payload(owned_id)
                .map_err(|error| error.to_string())?
                .is_none();
        let shared_key = AdapterPoolKey::Shared(provider);
        let mut pools = self.adapter_pools.lock().await;
        let (capabilities, started_result, runtime, transport, inbound, pool_key, started_fresh) =
            if let Some(pool) = pools.get_mut(&shared_key) {
                let runtime = Arc::clone(&pool.runtime);
                let mut started_fresh = false;
                let started = {
                    let mut runtime = runtime.lock().await;
                    let attempted = match native_session_id.as_deref() {
                        Some(native_session_id) => match native_session_mode {
                            AgentNativeSessionMode::Resume => {
                                runtime.resume_session_multi(&cwd, native_session_id).await
                            }
                            AgentNativeSessionMode::Load => {
                                runtime.load_session_multi(&cwd, native_session_id).await
                            }
                        },
                        None => runtime.new_session_multi(&cwd).await,
                    };
                    if attempted.is_err() && can_start_fresh {
                        crate::debug_log::stderr_log!(
                            "{owned_id}: stored native session never started; starting fresh"
                        );
                        started_fresh = true;
                        runtime.new_session_multi(&cwd).await
                    } else {
                        attempted
                    }
                };
                if started.is_ok() {
                    pool.members.insert(owned_id.to_string());
                }
                (
                    pool.capabilities.clone(),
                    started,
                    runtime,
                    Arc::clone(&pool.transport),
                    None,
                    shared_key,
                    started_fresh,
                )
            } else {
                let manifest = self.providers.manifest(provider)?;
                let environment = session_spawn_environment(provider, reasoning_effort.as_deref());
                let mut adapter = AcpRuntimeAdapter::with_environment(
                    manifest,
                    format!("provider:{}", provider_id(provider)),
                    cwd.clone(),
                    environment,
                );
                let capabilities = adapter
                    .initialize(InitializeAgentInput { provider })
                    .await
                    .map_err(|error| error.to_string())?;
                let attempted = match native_session_id.as_ref() {
                    Some(native_session_id) => {
                        let input = LoadAgentSession {
                            cwd: cwd.clone(),
                            native_session_id: native_session_id.clone(),
                        };
                        match native_session_mode {
                            AgentNativeSessionMode::Resume => adapter.resume_session(input).await,
                            AgentNativeSessionMode::Load => adapter.load_session(input).await,
                        }
                    }
                    None => {
                        adapter
                            .new_session(NewAgentSession { cwd: cwd.clone() })
                            .await
                    }
                };
                let (started, started_fresh) = if attempted.is_err() && can_start_fresh {
                    crate::debug_log::stderr_log!(
                        "{owned_id}: stored native session never started; starting fresh"
                    );
                    (
                        adapter
                            .new_session(NewAgentSession { cwd: cwd.clone() })
                            .await,
                        true,
                    )
                } else {
                    (attempted, false)
                };
                let runtime = Arc::new(AsyncMutex::new(StructuredRuntimeHandle::Acp(adapter)));
                let (transport, inbound) = {
                    let mut runtime = runtime.lock().await;
                    let transport = runtime.transport().map_err(|error| error.to_string())?;
                    let inbound = runtime.take_inbound().map_err(|error| error.to_string())?;
                    (transport, inbound)
                };
                let pool_key = if provider_is_multi_session_safe(&capabilities) {
                    shared_key
                } else {
                    AdapterPoolKey::Isolated(provider, owned_id.to_string())
                };
                let members = HashSet::from([owned_id.to_string()]);
                pools.insert(
                    pool_key.clone(),
                    AdapterPoolEntry {
                        runtime: Arc::clone(&runtime),
                        transport: Arc::clone(&transport),
                        capabilities: capabilities.clone(),
                        members,
                    },
                );
                (
                    capabilities,
                    started,
                    runtime,
                    transport,
                    Some(inbound),
                    pool_key,
                    started_fresh,
                )
            };
        drop(pools);
        if started_fresh {
            expected_native_session_id = None;
        }
        let mut started = match started_result {
            Ok(started) => started,
            Err(error) if expected_native_session_id.is_some() => {
                let _ = self
                    .release_pool_scope(&pool_key, owned_id, None, false)
                    .await;
                let mut sessions = self
                    .sessions
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let session = lifecycle_session_mut(&mut sessions, owned_id, generation)?;
                record_payload_for_session_and_dispatch_with_lifecycle(
                    session,
                    &self.emitter,
                    AgentConversationPayload::Error {
                        code: "session-resume-failed".to_string(),
                        // The card names the underlying cause. Without it the
                        // reader is told the resume failed and nothing about
                        // why, which is the one thing they need to act on.
                        message: format!(
                            "The stored provider session could not be resumed: {error}"
                        ),
                        recoverable: true,
                    },
                    SessionLifecycleUpdate {
                        state: AgentRuntimeState::Suspended,
                        connection_state: ConversationConnectionState::Disconnected,
                        owner: AgentExecutionOwner::Stopped,
                        writer_owner: AgentWriterLeaseOwner::None,
                        native_session_mode: None,
                    },
                )?;
                return Err(format!("Stored session could not be resumed: {error}"));
            }
            Err(error) => {
                let _ = self
                    .release_pool_scope(&pool_key, owned_id, None, false)
                    .await;
                return Err(error.to_string());
            }
        };
        if provider == AgentConversationProvider::Claude {
            started.config = claude_session_config(started.config, reasoning_effort.as_deref());
        } else if let Some(reasoning_effort) = reasoning_effort {
            let update = AgentConversationConfigUpdate {
                reasoning_effort: Some(reasoning_effort),
                ..AgentConversationConfigUpdate::default()
            };
            match runtime
                .lock()
                .await
                .set_conversation_config_on(&started.native_session_id, &update)
                .await
            {
                Ok(config) => started.config = config,
                Err(error) => crate::debug_log::stderr_log!(
                    "{owned_id}: session-start effort was not applied: {error}"
                ),
            }
        }
        if let Some(expected_native_session_id) = expected_native_session_id {
            if started.native_session_id != expected_native_session_id {
                let _ = self
                    .release_pool_scope(&pool_key, owned_id, Some(&started.native_session_id), true)
                    .await;
                return Err(
                    "The provider resumed a different native session; handoff was rejected"
                        .to_string(),
                );
            }
        }
        let (ordered_tx, ordered_rx) = mpsc::unbounded_channel();
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = lifecycle_session_mut(&mut sessions, owned_id, generation)?;
        if session.state
            != if was_suspended {
                AgentRuntimeState::Suspended
            } else {
                AgentRuntimeState::Starting
            }
        {
            return Err(
                "Conversation lifecycle changed while activation was in progress".to_string(),
            );
        }
        session.capabilities = capabilities;
        if !started.commands.is_empty() {
            session.capabilities.commands = started.commands.clone();
        }
        session.native_session_id = Some(started.native_session_id.clone());
        session.connection.native_session_id = Some(started.native_session_id);
        session.config = started.config;
        session.connection.config = session.config.clone();
        let restoring_terminal_transition = session.owner
            == AgentExecutionOwner::TransitioningToStructured
            && session
                .writer_lease_transition
                .as_ref()
                .map(|transition| transition.to == AgentWriterLeaseOwner::Structured)
                .unwrap_or(false);
        let (owner, writer_owner) = if restoring_terminal_transition {
            (session.owner, session.writer_lease.owner)
        } else {
            (
                AgentExecutionOwner::Structured,
                AgentWriterLeaseOwner::Structured,
            )
        };
        session.runtime = Some(runtime);
        session.pool_key = Some(pool_key);
        session.transport = Some(Arc::clone(&transport));
        session.ordered_events = Some(ordered_tx);
        if was_suspended {
            record_payload_for_session_and_dispatch_with_lifecycle(
                session,
                &self.emitter,
                AgentConversationPayload::Connection {
                    state: ConversationConnectionState::Connected,
                    native_session_id: session.native_session_id.clone(),
                },
                SessionLifecycleUpdate {
                    state: AgentRuntimeState::Ready,
                    connection_state: ConversationConnectionState::Connected,
                    owner,
                    writer_owner,
                    native_session_mode: None,
                },
            )?;
        } else {
            session.transition_lifecycle(
                AgentRuntimeState::Ready,
                ConversationConnectionState::Connected,
                owner,
                writer_owner,
            )?;
            persist_session(session)?;
        }
        let connection = session.connection.clone();
        drop(sessions);
        if let Some(inbound) = inbound {
            spawn_inbound_pump(
                self.clone(),
                Arc::downgrade(&transport),
                inbound,
                ordered_rx,
                owned_id.to_string(),
                generation,
            );
        } else {
            spawn_ordered_pump(
                self.clone(),
                Arc::downgrade(&transport),
                ordered_rx,
                owned_id.to_string(),
                generation,
            );
        }
        Ok(connection)
    }

    pub async fn prompt(
        &self,
        owned_id: &str,
        generation: u64,
        input: AgentPrompt,
    ) -> Result<(), String> {
        let runtime = self.runtime(owned_id, generation)?;
        let transport = {
            let runtime = runtime.lock().await;
            runtime.transport().map_err(|error| error.to_string())?
        };
        let turn_id = format!("turn-{}", uuid::Uuid::new_v4());
        let (native_session_id, ordered_events) = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = current_session_mut(&mut sessions, owned_id, generation)?;
            if session.active_turn_id.is_some() {
                return Err("The structured session already has an active turn".to_string());
            }
            if session.prompt_once_active {
                return Err(
                    "The structured session is busy generating a one-shot result".to_string(),
                );
            }
            if session.writer_lease.owner != AgentWriterLeaseOwner::Structured {
                return Err("The structured writer is not the current owner".to_string());
            }
            let native_session_id = session
                .native_session_id
                .clone()
                .ok_or_else(|| "Structured provider session has not started".to_string())?;
            let ordered_events = session
                .ordered_events
                .clone()
                .ok_or_else(|| "Structured event pump has not started".to_string())?;
            session.active_turn_id = Some(turn_id.clone());
            if session_title_is_empty(session.rail_meta.title.as_deref()) {
                let is_first_user_prompt = session
                    .store
                    .first_user_message_payload(&session.owned_id)
                    .map_err(|error| error.to_string())?
                    .is_none();
                if is_first_user_prompt {
                    session.rail_meta.title = prompt_title(&input.text);
                }
            }
            let lifecycle = lifecycle_update_for_state(
                session,
                AgentRuntimeState::Working,
                session.connection.state,
            );
            record_payload_for_session_and_dispatch_with_lifecycle(
                session,
                &self.emitter,
                AgentConversationPayload::Turn {
                    turn_id: turn_id.clone(),
                    state: super::protocol::TurnState::Started,
                },
                lifecycle,
            )?;
            // Agents are not required to echo the prompt back as a
            // user_message_chunk, so the app records its own copy.
            record_payload_for_session_and_dispatch(
                session,
                &self.emitter,
                AgentConversationPayload::UserMessage {
                    item_id: format!("user-{turn_id}"),
                    text: input.text.clone(),
                    completed: true,
                },
            )?;
            (native_session_id, ordered_events)
        };

        self.start_child_rollout_scan(owned_id, generation)?;
        let params = prompt_params(native_session_id, input);
        spawn_prompt_completion(transport, ordered_events, turn_id, params);
        Ok(())
    }

    fn start_child_rollout_scan(&self, owned_id: &str, generation: u64) -> Result<(), String> {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = current_session_mut(&mut sessions, owned_id, generation)?;
        if session.provider != AgentConversationProvider::Codex {
            return Ok(());
        }
        if let Some(task) = session.child_rollout_scan.take() {
            task.abort();
        }
        let native_session_id = session
            .native_session_id
            .clone()
            .ok_or_else(|| "Structured provider session has not started".to_string())?;
        let sessions = Arc::downgrade(&self.sessions);
        let emitter = Arc::clone(&self.emitter);
        let owned_id = owned_id.to_string();
        session.child_rollout_scan = Some(tokio::spawn(async move {
            run_child_rollout_scan(sessions, emitter, owned_id, generation, native_session_id)
                .await;
        }));
        Ok(())
    }

    async fn finish_child_rollout_scan(&self, owned_id: &str, generation: u64, turn_id: &str) {
        let scan = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Ok(session) = current_session_mut(&mut sessions, owned_id, generation) else {
                return;
            };
            if session.active_turn_id.as_deref() != Some(turn_id) {
                return;
            }
            session.native_session_id.clone().map(|native_session_id| {
                (native_session_id, session.child_rollout_parent_path.clone())
            })
        };
        let Some((native_session_id, parent_path)) = scan else {
            return;
        };
        let outcome = scan_codex_children_once(
            &Arc::downgrade(&self.sessions),
            &self.emitter,
            owned_id,
            generation,
            &native_session_id,
            parent_path,
        )
        .await;
        if outcome.is_some_and(|outcome| outcome.has_running_children) {
            return;
        }
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Ok(session) = current_session_mut(&mut sessions, owned_id, generation) else {
            return;
        };
        if session.active_turn_id.as_deref() == Some(turn_id) {
            if let Some(task) = session.child_rollout_scan.take() {
                task.abort();
            }
        }
    }

    /// Run one product action through the current structured agent without
    /// writing through the conversation UI. The runtime and generation checks
    /// remain the same as an ordinary conversation prompt.
    pub async fn prompt_once(
        &self,
        owned_id: &str,
        generation: u64,
        input: AgentPrompt,
    ) -> Result<GeneratedText, String> {
        let runtime = self.runtime(owned_id, generation)?;
        let native_session_id = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = current_session_mut(&mut sessions, owned_id, generation)?;
            if session.active_turn_id.is_some() {
                return Err(
                    "The structured session already has an active turn; one-shot generation is unavailable"
                        .to_string(),
                );
            }
            if session.prompt_once_active {
                return Err("A one-shot generation is already active".to_string());
            }
            if session.writer_lease.owner != AgentWriterLeaseOwner::Structured {
                return Err("The structured writer is not the current owner".to_string());
            }
            session.prompt_once_active = true;
            session
                .native_session_id
                .clone()
                .ok_or_else(|| "Structured provider session has not started".to_string())?
        };
        let result = runtime
            .lock()
            .await
            .prompt_once_on(&native_session_id, input)
            .await;
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Ok(session) = current_session_mut(&mut sessions, owned_id, generation) {
            session.prompt_once_active = false;
        }
        result.map_err(|error| error.to_string())
    }

    pub async fn respond_permission(&self, input: PermissionResponse) -> Result<(), String> {
        self.respond_permission_selection(
            &input.identity.owned_id,
            input.identity.generation,
            input.identity.request_id,
            PermissionSelection::Decision(input.decision),
        )
        .await
    }

    pub async fn respond_permission_option(
        &self,
        owned_id: &str,
        generation: u64,
        request_id: String,
        option_id: String,
    ) -> Result<(), String> {
        let option_id = required_id(&option_id, "Permission option id")?;
        self.respond_permission_selection(
            owned_id,
            generation,
            request_id,
            PermissionSelection::Option(option_id),
        )
        .await
    }

    async fn respond_permission_selection(
        &self,
        owned_id: &str,
        generation: u64,
        request_id: String,
        selection: PermissionSelection,
    ) -> Result<(), String> {
        let _runtime_scope_is_live = {
            let sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = current_session(&sessions, owned_id, generation)?;
            if session.runtime.is_none()
                || matches!(
                    session.state,
                    AgentRuntimeState::Suspended
                        | AgentRuntimeState::Failed
                        | AgentRuntimeState::Closed
                )
            {
                return Err(
                    "Stale approval request: the original runtime scope is closed".to_string(),
                );
            }
            true
        };
        let runtime = self.runtime(&owned_id, generation)?;
        let transport = {
            let runtime = runtime.lock().await;
            runtime.transport().map_err(|error| error.to_string())?
        };
        let (pending, ordered_events, turn_active) = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = current_session_mut(&mut sessions, &owned_id, generation)?;
            let ordered_events = session
                .ordered_events
                .clone()
                .ok_or_else(|| "Structured event pump has not started".to_string())?;
            let pending = session
                .permission_requests
                .remove(&request_id)
                .ok_or_else(|| "Approval request is no longer pending".to_string())?;
            (pending, ordered_events, session.active_turn_id.is_some())
        };
        let terminal_state = if turn_active {
            permission_state(&pending.options, &selection)
        } else {
            ApprovalState::Declined
        };
        let response = match if turn_active {
            permission_response_for_selection(&pending.options, &selection)
        } else {
            Ok(serde_json::json!({ "outcome": { "outcome": "cancelled" } }))
        } {
            Ok(response) => response,
            Err(error) => {
                let mut sessions = self
                    .sessions
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Ok(session) = current_session_mut(&mut sessions, &owned_id, generation) {
                    session.permission_requests.insert(request_id, pending);
                }
                return Err(error);
            }
        };
        if let Err(error) = transport.respond(pending.wire_id.clone(), response).await {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Ok(session) = current_session_mut(&mut sessions, &owned_id, generation) {
                session.permission_requests.insert(request_id, pending);
            }
            return Err(error.to_string());
        }
        ordered_events
            .send(OrderedSessionEvent::ApprovalResolved {
                request_id,
                state: terminal_state,
                summary: pending.summary,
            })
            .map_err(|_| "Structured event pump is no longer running".to_string())?;
        Ok(())
    }

    pub async fn respond_user_input(&self, input: AgentUserInputResponse) -> Result<(), String> {
        let owned_id = input.identity.owned_id.clone();
        let generation = input.identity.generation;
        let request_id = input.identity.request_id.clone();
        {
            let sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = current_session(&sessions, &owned_id, generation)?;
            if session.runtime.is_none()
                || matches!(
                    session.state,
                    AgentRuntimeState::Suspended
                        | AgentRuntimeState::Failed
                        | AgentRuntimeState::Closed
                )
            {
                return Err("Stale input request: the original runtime scope is closed".to_string());
            }
        }
        let runtime = self.runtime(&owned_id, generation)?;
        let transport = {
            let runtime = runtime.lock().await;
            runtime.transport().map_err(|error| error.to_string())?
        };
        let (pending, ordered_events) = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = current_session_mut(&mut sessions, &owned_id, generation)?;
            let ordered_events = session
                .ordered_events
                .clone()
                .ok_or_else(|| "Structured event pump has not started".to_string())?;
            let pending = session
                .user_input_requests
                .remove(&request_id)
                .ok_or_else(|| "User input request is no longer pending".to_string())?;
            (pending, ordered_events)
        };
        let response = serde_json::json!({
            "values": input.values,
            "cancelled": input.cancelled,
        });
        if let Err(error) = transport.respond(pending.wire_id.clone(), response).await {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Ok(session) = current_session_mut(&mut sessions, &owned_id, generation) {
                session.user_input_requests.insert(request_id, pending);
            }
            return Err(error.to_string());
        }
        ordered_events
            .send(OrderedSessionEvent::UserInputResolved {
                request_id,
                cancelled: input.cancelled,
            })
            .map_err(|_| "Structured event pump is no longer running".to_string())?;
        Ok(())
    }

    pub async fn set_config(
        &self,
        owned_id: &str,
        generation: u64,
        option_id: &str,
        value: AgentConfigValue,
    ) -> Result<Vec<super::protocol::AgentConfigOption>, String> {
        let native_session_id = {
            let sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = current_session(&sessions, owned_id, generation)?;
            let option = session
                .capabilities
                .config_options
                .iter()
                .find(|option| option.id == option_id)
                .ok_or_else(|| {
                    "Config option was not advertised by the current provider".to_string()
                })?;
            super::capabilities::validate_config_value(option, &value)?;
            session
                .native_session_id
                .clone()
                .ok_or_else(|| "Structured provider session has not started".to_string())?
        };
        let runtime = self.runtime(owned_id, generation)?;
        let replacement = runtime
            .lock()
            .await
            .set_config_on(&native_session_id, option_id, value)
            .await
            .map_err(|error| error.to_string())?;
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = current_session_mut(&mut sessions, owned_id, generation)?;
        super::capabilities::replace_config_options(
            &mut session.capabilities,
            replacement.clone(),
        )?;
        persist_session(session)?;
        Ok(replacement)
    }

    pub fn conversation_config(
        &self,
        owned_id: &str,
    ) -> Result<AgentConversationConfigState, String> {
        let sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        sessions
            .get(owned_id)
            .map(|session| session.config.clone())
            .ok_or_else(|| "Conversation session was not found".to_string())
    }

    pub async fn set_conversation_config(
        &self,
        request: SetAgentConversationConfigRequest,
    ) -> Result<AgentConversationConfigState, String> {
        let update = AgentConversationConfigUpdate {
            model: normalized_optional_id(request.model),
            reasoning_effort: normalized_optional_id(request.reasoning_effort),
            approval_policy: normalized_optional_id(request.approval_policy),
        };
        let native_session_id = {
            let sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = current_session(&sessions, &request.owned_id, request.generation)?;
            if session.provider == AgentConversationProvider::Claude
                && update.reasoning_effort.is_some()
            {
                return Err(
                    "Effort is set when the session starts. Start a new session to change it."
                        .to_string(),
                );
            }
            validate_conversation_config_update(&session.config, &update)?;
            if update == AgentConversationConfigUpdate::default() {
                return Ok(session.config.clone());
            }
            session
                .native_session_id
                .clone()
                .ok_or_else(|| "Structured provider session has not started".to_string())?
        };
        let runtime = self
            .runtime_or_activate(&request.owned_id, request.generation)
            .await?;
        let config = runtime
            .lock()
            .await
            .set_conversation_config_on(&native_session_id, &update)
            .await
            .map_err(|error| error.to_string())?;
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = lifecycle_session_mut(&mut sessions, &request.owned_id, request.generation)?;
        session.config = config.clone();
        session.connection.config = config.clone();
        persist_session(session)?;
        Ok(config)
    }

    pub async fn respond_legacy_approval(
        &self,
        owned_id: &str,
        generation: u64,
        request_id: String,
        decision: super::protocol::ApprovalDecision,
    ) -> Result<(), String> {
        let decision = match decision {
            super::protocol::ApprovalDecision::Accept => AgentApprovalDecision::Accept,
            super::protocol::ApprovalDecision::Decline => AgentApprovalDecision::Decline,
        };
        self.respond_permission(AgentApprovalResponse {
            identity: AgentRequestIdentity {
                owned_id: owned_id.to_string(),
                generation,
                request_id,
                turn_id: None,
                item_id: None,
            },
            decision,
        })
        .await
    }

    pub async fn cancel_turn(&self, owned_id: &str, generation: u64) -> Result<(), String> {
        let runtime = self.runtime(owned_id, generation)?;
        let transport = {
            let runtime = runtime.lock().await;
            runtime.transport().map_err(|error| error.to_string())?
        };
        let native_session_id = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = current_session_mut(&mut sessions, owned_id, generation)?;
            session.state = AgentRuntimeState::Interrupting;
            session
                .native_session_id
                .clone()
                .ok_or_else(|| "Structured provider session has not started".to_string())?
        };
        transport
            .notify(
                "session/cancel",
                serde_json::json!({ "sessionId": native_session_id }),
            )
            .await
            .map_err(|error| error.to_string())
    }

    pub fn snapshot(&self, owned_id: &str) -> Result<Option<AgentConversationSnapshot>, String> {
        let sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(session) = sessions.get(owned_id) else {
            return Ok(None);
        };
        Ok(Some(AgentConversationSnapshot {
            connection: session.connection.clone(),
            suspended: session.state == AgentRuntimeState::Suspended,
            last_sequence: session.next_sequence.saturating_sub(1),
            events: self.list_recent_events(owned_id)?,
        }))
    }

    pub fn list_sessions(&self) -> Result<Vec<AgentConversationSessionRecord>, String> {
        let rows = self
            .store
            .list_sessions()
            .map_err(|error| error.to_string())?;
        let sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        rows.into_iter()
            .map(|row| {
                let session = sessions.get(&row.owned_id).ok_or_else(|| {
                    "The stored session is missing its live runtime overlay".to_string()
                })?;
                let mut meta = session.rail_meta.clone();
                meta.worktree.clone_from(&row.worktree);
                meta.branch.clone_from(&row.branch);
                meta.title.clone_from(&row.title);
                meta.project.clone_from(&row.project);
                Ok(AgentConversationSessionRecord {
                    owned_id: row.owned_id,
                    provider: session.provider,
                    model: row.model,
                    effort: row.effort,
                    cwd: row.cwd,
                    state: session.state,
                    suspended: session.state == AgentRuntimeState::Suspended,
                    created_at_ms: row.created_at_ms,
                    last_activity_at_ms: row.last_activity_at_ms,
                    active_turn_id: session.active_turn_id.clone(),
                    pending_permission: !session.permission_requests.is_empty(),
                    pending_input: !session.user_input_requests.is_empty(),
                    native_session_id: session.native_session_id.clone(),
                    meta,
                })
            })
            .collect()
    }

    pub fn list_events(
        &self,
        owned_id: &str,
        from_sequence: u64,
    ) -> Result<Vec<AgentConversationEvent>, String> {
        let from_sequence = i64::try_from(from_sequence)
            .map_err(|_| "Conversation event sequence exceeded the store limit".to_string())?;
        self.store
            .list_events(owned_id, from_sequence, STORE_EVENT_CAP)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|row| {
                serde_json::from_str(&row.payload_json)
                    .map_err(|error| format!("Could not decode stored conversation event: {error}"))
            })
            .collect()
    }

    pub fn submit_terminal_projection(
        &self,
        owned_id: &str,
        projection: TerminalProjectionPayload,
    ) -> Result<AgentConversationEvent, String> {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = sessions
            .get_mut(owned_id)
            .ok_or_else(|| "Agent conversation session was not found".to_string())?;
        record_payload_for_session_and_dispatch(
            session,
            &self.emitter,
            AgentConversationPayload::TerminalProjection(projection),
        )
    }

    fn list_recent_events(&self, owned_id: &str) -> Result<Vec<AgentConversationEvent>, String> {
        let limit = u32::try_from(SNAPSHOT_EVENT_CAP)
            .map_err(|_| "Conversation snapshot event cap is invalid".to_string())?;
        self.store
            .list_recent_events(owned_id, limit)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|row| {
                serde_json::from_str(&row.payload_json)
                    .map_err(|error| format!("Could not decode stored conversation event: {error}"))
            })
            .collect()
    }

    pub fn update_session_meta(
        &self,
        request: UpdateAgentConversationSessionMetaRequest,
    ) -> Result<AgentConversationSessionRecord, String> {
        let owned_id = required_id(&request.owned_id, "Owned session id")?;
        {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = sessions
                .get_mut(&owned_id)
                .ok_or_else(|| "Conversation session was not found".to_string())?;
            session.spawn_reasoning_effort.clone_from(&request.effort);
            session.config.model.clone_from(&request.model);
            session.config.reasoning_effort.clone_from(&request.effort);
            session.connection.config = session.config.clone();
            session.rail_meta = request.meta;
            persist_session(session)?;
        }
        self.list_sessions()?
            .into_iter()
            .find(|session| session.owned_id == owned_id)
            .ok_or_else(|| "Conversation session was not found after metadata update".to_string())
    }

    pub async fn suspend_if_quiescent(
        &self,
        owned_id: &str,
        generation: u64,
    ) -> Result<bool, String> {
        let _lifecycle = self.lifecycle_guard(owned_id).await?;
        let (runtime, transport, ordered_events, pool_key, native_session_id) = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = lifecycle_session_mut(&mut sessions, owned_id, generation)?;
            if !session_can_suspend(session) {
                return Ok(false);
            }
            let Some(runtime) = session.runtime.take() else {
                return Ok(false);
            };
            session.suspending = true;
            (
                runtime,
                session.transport.take(),
                session.ordered_events.take(),
                session.pool_key.take(),
                session.native_session_id.clone(),
            )
        };

        let detach_result = if let Some(pool_key) = &pool_key {
            self.release_pool_scope(pool_key, owned_id, native_session_id.as_deref(), false)
                .await
        } else {
            runtime
                .lock()
                .await
                .detach_session()
                .await
                .map_err(|error| error.to_string())
        };
        if let Err(error) = detach_result {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Ok(session) = lifecycle_session_mut(&mut sessions, owned_id, generation) {
                session.suspending = false;
                session.runtime = Some(runtime);
                session.transport = transport;
                session.ordered_events = ordered_events;
                session.pool_key = pool_key;
            }
            return Err(error);
        }

        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = lifecycle_session_mut(&mut sessions, owned_id, generation)?;
        if !session.suspending || session.state != AgentRuntimeState::Ready {
            return Err(
                "Conversation lifecycle changed while suspension was in progress".to_string(),
            );
        }
        session.suspending = false;
        record_payload_for_session_and_dispatch_with_lifecycle(
            session,
            &self.emitter,
            AgentConversationPayload::Connection {
                state: ConversationConnectionState::Disconnected,
                native_session_id: session.native_session_id.clone(),
            },
            SessionLifecycleUpdate {
                state: AgentRuntimeState::Suspended,
                connection_state: ConversationConnectionState::Disconnected,
                owner: AgentExecutionOwner::Stopped,
                writer_owner: AgentWriterLeaseOwner::None,
                native_session_mode: Some(AgentNativeSessionMode::Resume),
            },
        )?;
        Ok(true)
    }

    pub async fn close(&self, owned_id: &str, generation: u64) -> Result<bool, String> {
        let _lifecycle = self.lifecycle_guard(owned_id).await?;
        let (runtime, transport, pool_key, native_session_id, pending_permissions, pending_inputs) = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Some(session) = sessions.get_mut(owned_id) else {
                return Ok(false);
            };
            if session.generation != generation {
                return Err(
                    "Conversation connection changed; retry on the current session".to_string(),
                );
            }
            let transport = session.transport.take();
            if let Some(task) = session.child_rollout_scan.take() {
                task.abort();
            }
            session.ordered_events = None;
            let pending_permission_rows = session.permission_requests.drain().collect::<Vec<_>>();
            let mut pending_permissions = Vec::with_capacity(pending_permission_rows.len());
            for (request_id, pending) in pending_permission_rows {
                let _ = record_payload_for_session_and_dispatch(
                    session,
                    &self.emitter,
                    AgentConversationPayload::Approval {
                        request_id,
                        state: ApprovalState::Expired,
                        summary: pending.summary,
                    },
                );
                pending_permissions.push(pending.wire_id);
            }
            let pending_input_rows = session.user_input_requests.drain().collect::<Vec<_>>();
            let mut pending_inputs = Vec::with_capacity(pending_input_rows.len());
            for (request_id, pending) in pending_input_rows {
                let _ = record_payload_for_session_and_dispatch(
                    session,
                    &self.emitter,
                    AgentConversationPayload::UserInputResolved {
                        request_id,
                        cancelled: true,
                    },
                );
                pending_inputs.push(pending.wire_id);
            }
            record_payload_for_session_and_dispatch_with_lifecycle(
                session,
                &self.emitter,
                AgentConversationPayload::Connection {
                    state: ConversationConnectionState::Closed,
                    native_session_id: session.native_session_id.clone(),
                },
                SessionLifecycleUpdate {
                    state: AgentRuntimeState::Closed,
                    connection_state: ConversationConnectionState::Closed,
                    owner: AgentExecutionOwner::Stopped,
                    writer_owner: AgentWriterLeaseOwner::None,
                    native_session_mode: None,
                },
            )?;
            (
                session.runtime.take(),
                transport,
                session.pool_key.take(),
                session.native_session_id.clone(),
                pending_permissions,
                pending_inputs,
            )
        };
        if let Some(transport) = transport {
            for wire_id in pending_permissions {
                let _ = transport
                    .respond(
                        wire_id,
                        serde_json::json!({ "outcome": { "outcome": "cancelled" } }),
                    )
                    .await;
            }
            for wire_id in pending_inputs {
                let _ = transport
                    .respond(
                        wire_id,
                        serde_json::json!({ "values": {}, "cancelled": true }),
                    )
                    .await;
            }
        }
        if let Some(pool_key) = pool_key {
            self.release_pool_scope(&pool_key, owned_id, native_session_id.as_deref(), true)
                .await?;
        } else if let Some(runtime) = runtime {
            runtime
                .lock()
                .await
                .close_session()
                .await
                .map_err(|error| error.to_string())?;
        }
        let sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = current_session(&sessions, owned_id, generation)?;
        if session.state != AgentRuntimeState::Closed
            || session.connection.state != ConversationConnectionState::Closed
        {
            return Err("Conversation lifecycle changed while close was in progress".to_string());
        }
        Ok(true)
    }

    pub(crate) fn handoff_prepare(
        &self,
        request: &AgentConversationHandoffRequest,
    ) -> Result<HandoffContext, String> {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = current_session_mut(&mut sessions, &request.owned_id, request.generation)?;
        let expected = request
            .expected_owner
            .unwrap_or_else(|| match request.direction {
                AgentConversationHandoffDirection::StructuredToTerminal => {
                    AgentWriterLeaseOwner::Structured
                }
                AgentConversationHandoffDirection::TerminalToStructured => {
                    AgentWriterLeaseOwner::Terminal
                }
            });
        if session.writer_lease.owner != expected {
            return Err("The current writer owner does not match the handoff request".to_string());
        }
        if session.writer_lease_transition.is_some() {
            return Err("Another handoff is already in progress".to_string());
        }
        if request.mode == AgentConversationHandoffMode::Fork {
            if request.direction != AgentConversationHandoffDirection::StructuredToTerminal {
                return Err("Only structured conversations can fork to a terminal".to_string());
            }
            if !session.capabilities.session.fork {
                return Err("The current provider did not advertise fork support".to_string());
            }
            let target = request
                .target_owned_id
                .as_deref()
                .ok_or_else(|| "Fork handoff requires a distinct target owned id".to_string())?;
            if target.trim().is_empty() || target == request.owned_id {
                return Err("Fork handoff requires a distinct target owned id".to_string());
            }
        }
        let to = match request.direction {
            AgentConversationHandoffDirection::StructuredToTerminal => {
                AgentWriterLeaseOwner::Terminal
            }
            AgentConversationHandoffDirection::TerminalToStructured => {
                AgentWriterLeaseOwner::Structured
            }
        };
        let previous_owner = session.writer_lease.owner;
        let owner = match request.direction {
            AgentConversationHandoffDirection::StructuredToTerminal => {
                AgentExecutionOwner::TransitioningToTerminal
            }
            AgentConversationHandoffDirection::TerminalToStructured => {
                AgentExecutionOwner::TransitioningToStructured
            }
        };
        session.transition_lifecycle(
            session.state,
            session.connection.state,
            owner,
            session.writer_lease.owner,
        )?;
        session.writer_lease_transition = Some(AgentWriterLeaseTransition {
            owned_id: request.owned_id.clone(),
            generation: request.generation,
            from: previous_owner,
            to,
            state: super::protocol::AgentWriterLeaseTransitionState::Requested,
            error: None,
        });
        Ok(HandoffContext {
            previous_owner,
            owner: previous_owner,
            native_session_id: session.native_session_id.clone(),
        })
    }

    pub(crate) async fn detach_structured_runtime(
        &self,
        owned_id: &str,
        generation: u64,
    ) -> Result<(), String> {
        let runtime = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = lifecycle_session_mut(&mut sessions, owned_id, generation)?;
            if session.owner != AgentExecutionOwner::TransitioningToTerminal {
                return Err("Structured runtime is not in a terminal handoff".to_string());
            }
            session.runtime.take()
        };
        let Some(runtime) = runtime else {
            return Ok(());
        };
        let result = runtime.lock().await.detach_session().await;
        if let Err(error) = result {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Ok(session) = lifecycle_session_mut(&mut sessions, owned_id, generation) {
                session.runtime = Some(runtime);
            }
            return Err(error.to_string());
        }
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = lifecycle_session_mut(&mut sessions, owned_id, generation)?;
        if session.owner != AgentExecutionOwner::TransitioningToTerminal {
            return Err("Conversation lifecycle changed while handoff was in progress".to_string());
        }
        Ok(())
    }

    pub(crate) fn validate_handoff_history(
        &self,
        request: &AgentConversationHandoffRequest,
    ) -> Result<(), String> {
        let sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = current_session(&sessions, &request.owned_id, request.generation)?;
        let boundary = &request.history_boundary;
        if request.native_session_id != session.native_session_id {
            return Err("Handoff native session does not match the current session".to_string());
        }
        if boundary.native_session_id != session.native_session_id {
            return Err(
                "Handoff history native session does not match the current session".to_string(),
            );
        }
        if session.native_session_id.is_none() {
            return Err(
                "A native session id is required to resume the structured session".to_string(),
            );
        }
        let last_sequence = session.next_sequence.saturating_sub(1);
        if boundary.first_sequence > boundary.last_sequence
            || boundary.last_sequence > last_sequence
        {
            return Err("Handoff history boundary is outside the stored conversation".to_string());
        }
        if boundary.reconciled_sequence != Some(last_sequence) {
            return Err("Handoff history was not reconciled to the current sequence".to_string());
        }
        Ok(())
    }

    pub(crate) fn validate_handoff_identity(
        &self,
        request: &AgentConversationHandoffRequest,
    ) -> Result<(), String> {
        let sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = current_session(&sessions, &request.owned_id, request.generation)?;
        if request.native_session_id != session.native_session_id {
            return Err("Handoff native session does not match the current session".to_string());
        }
        if request.history_boundary.native_session_id != request.native_session_id {
            return Err("Handoff history native session does not match the request".to_string());
        }
        Ok(())
    }

    pub(crate) async fn handoff_commit(
        &self,
        request: &AgentConversationHandoffRequest,
    ) -> Result<AgentConversationHandoffReceipt, String> {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = lifecycle_session_mut(&mut sessions, &request.owned_id, request.generation)?;
        let transition = session
            .writer_lease_transition
            .clone()
            .ok_or_else(|| "Handoff commit has no prepared transition".to_string())?;
        let expected_to = match request.direction {
            AgentConversationHandoffDirection::StructuredToTerminal => {
                AgentWriterLeaseOwner::Terminal
            }
            AgentConversationHandoffDirection::TerminalToStructured => {
                AgentWriterLeaseOwner::Structured
            }
        };
        if transition.to != expected_to || transition.from != session.writer_lease.owner {
            return Err("Handoff commit does not match the prepared transition".to_string());
        }
        let previous_owner = transition.from;
        let owner = if request.mode == AgentConversationHandoffMode::Fork {
            previous_owner
        } else {
            expected_to
        };
        session.writer_lease_transition = None;
        let execution_owner = match owner {
            AgentWriterLeaseOwner::Structured => AgentExecutionOwner::Structured,
            AgentWriterLeaseOwner::Terminal => AgentExecutionOwner::Terminal,
            AgentWriterLeaseOwner::None => AgentExecutionOwner::Stopped,
        };
        session.transition_lifecycle(
            session.state,
            session.connection.state,
            execution_owner,
            owner,
        )?;
        Ok(AgentConversationHandoffReceipt {
            owned_id: request
                .target_owned_id
                .clone()
                .filter(|_| request.mode == AgentConversationHandoffMode::Fork)
                .unwrap_or_else(|| request.owned_id.clone()),
            generation: request.generation,
            direction: request.direction,
            mode: request.mode,
            phase: AgentConversationHandoffPhase::Commit,
            previous_owner,
            owner,
            native_session_id: session.native_session_id.clone(),
            pty_session_id: request
                .pty_session_id
                .clone()
                .or_else(|| request.process_tree.pty_session_id.clone()),
            history_boundary: request.history_boundary.clone(),
            process_tree: request.process_tree.clone(),
            rollback_available: false,
            message: "Handoff committed without closing the terminal scrollback".to_string(),
        })
    }

    pub(crate) fn handoff_rollback(
        &self,
        request: &AgentConversationHandoffRequest,
    ) -> Result<AgentConversationHandoffReceipt, String> {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = lifecycle_session_mut(&mut sessions, &request.owned_id, request.generation)?;
        let transition = session
            .writer_lease_transition
            .take()
            .ok_or_else(|| "Handoff rollback has no prepared transition".to_string())?;
        let owner = match transition.from {
            AgentWriterLeaseOwner::Structured => AgentExecutionOwner::Structured,
            AgentWriterLeaseOwner::Terminal => AgentExecutionOwner::Terminal,
            AgentWriterLeaseOwner::None => AgentExecutionOwner::Stopped,
        };
        session.transition_lifecycle(
            session.state,
            session.connection.state,
            owner,
            transition.from,
        )?;
        Ok(AgentConversationHandoffReceipt {
            owned_id: request.owned_id.clone(),
            generation: request.generation,
            direction: request.direction,
            mode: request.mode,
            phase: AgentConversationHandoffPhase::Rollback,
            previous_owner: transition.from,
            owner: transition.from,
            native_session_id: session.native_session_id.clone(),
            pty_session_id: request
                .pty_session_id
                .clone()
                .or_else(|| request.process_tree.pty_session_id.clone()),
            history_boundary: request.history_boundary.clone(),
            process_tree: request.process_tree.clone(),
            rollback_available: false,
            message: "Handoff rolled back; existing terminal scrollback was preserved".to_string(),
        })
    }

    pub(crate) async fn lifecycle_guard(
        &self,
        owned_id: &str,
    ) -> Result<OwnedMutexGuard<()>, String> {
        Ok(self.activation_lock(owned_id)?.lock_owned().await)
    }

    fn activation_lock(&self, owned_id: &str) -> Result<Arc<AsyncMutex<()>>, String> {
        let mut locks = self
            .activation_locks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(locks
            .entry(owned_id.to_string())
            .or_insert_with(|| Arc::new(AsyncMutex::new(())))
            .clone())
    }

    fn runtime(
        &self,
        owned_id: &str,
        generation: u64,
    ) -> Result<Arc<AsyncMutex<StructuredRuntimeHandle>>, String> {
        let sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = current_session(&sessions, owned_id, generation)?;
        if session.owner != AgentExecutionOwner::Structured {
            return Err("The structured writer is not the current owner".to_string());
        }
        session
            .runtime
            .clone()
            .ok_or_else(|| "Structured provider is still connecting".to_string())
    }

    /// The runtime for a session, waking a suspended one first.
    ///
    /// A session restored from the database has no runtime until something
    /// activates it. Sending a message does that on the way through, so a
    /// session a person has been talking to is awake; changing a setting used
    /// to be refused outright instead, which read as "still connecting" on a
    /// session that was demonstrably live.
    async fn runtime_or_activate(
        &self,
        owned_id: &str,
        generation: u64,
    ) -> Result<Arc<AsyncMutex<StructuredRuntimeHandle>>, String> {
        match self.runtime(owned_id, generation) {
            Ok(runtime) => Ok(runtime),
            Err(error) => {
                if !self.session_is_suspended(owned_id, generation) {
                    return Err(error);
                }
                self.activate(owned_id, generation).await?;
                self.runtime(owned_id, generation)
            }
        }
    }

    /// True when a session is stored and recoverable but has no runtime yet.
    fn session_is_suspended(&self, owned_id: &str, generation: u64) -> bool {
        let sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        current_session(&sessions, owned_id, generation)
            .map(|session| {
                session.runtime.is_none() && session.state == AgentRuntimeState::Suspended
            })
            .unwrap_or(false)
    }

    #[cfg(test)]
    pub(crate) fn session_terminal_ownership(
        &self,
        owned_id: &str,
    ) -> Option<AgentWriterLeaseOwner> {
        self.sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(owned_id)
            .map(|session| session.writer_lease.owner)
    }
}

impl Drop for AgentRuntimeManager {
    fn drop(&mut self) {
        if Arc::strong_count(&self.sessions) == 1 {
            self.sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clear();
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredSessionExtra {
    generation: u64,
    native_session_mode: AgentNativeSessionMode,
    owner: AgentExecutionOwner,
    config: AgentConversationConfigState,
    capabilities: AgentCapabilities,
    #[serde(default)]
    rail_meta: AgentConversationSessionMeta,
}

/// Builds the durable row from the current live session for metadata-only writes.
fn persisted_session_row(session: &ManagedAgentSession) -> Result<SessionRow, String> {
    let candidate = SessionEventCandidate::from_session(session);
    persisted_session_row_for_candidate(session, &candidate)
}

/// Builds the session row that must become visible with a pending event.
fn persisted_session_row_for_candidate(
    session: &ManagedAgentSession,
    candidate: &SessionEventCandidate,
) -> Result<SessionRow, String> {
    let extra = StoredSessionExtra {
        generation: session.generation,
        native_session_mode: candidate.native_session_mode,
        owner: candidate.owner,
        config: session.config.clone(),
        capabilities: candidate.capabilities.clone(),
        rail_meta: session.rail_meta.clone(),
    };
    Ok(SessionRow {
        owned_id: session.owned_id.clone(),
        native_session_id: candidate.native_session_id.clone(),
        provider: enum_storage_value(session.provider)?,
        model: session.config.model.clone(),
        effort: session
            .config
            .reasoning_effort
            .clone()
            .or_else(|| session.spawn_reasoning_effort.clone()),
        cwd: session.cwd.clone(),
        worktree: session
            .rail_meta
            .worktree
            .clone()
            .or_else(|| Some(session.cwd.clone())),
        branch: session.rail_meta.branch.clone(),
        title: session.rail_meta.title.clone(),
        project: session.rail_meta.project.clone(),
        state: enum_storage_value(candidate.state)?,
        suspended: candidate.state == AgentRuntimeState::Suspended,
        created_at_ms: store_timestamp(session.created_at_ms),
        last_activity_at_ms: store_timestamp(candidate.last_activity_ms),
        extra_json: serde_json::to_string(&extra)
            .map_err(|error| format!("Could not encode stored session metadata: {error}"))?,
    })
}

/// Persists rail-only metadata when no event needs to share the transaction.
fn persist_session(session: &ManagedAgentSession) -> Result<(), String> {
    let row = persisted_session_row(session)?;
    session
        .store
        .upsert_session(&row)
        .map_err(|error| error.to_string())
}

fn recover_sessions_from_store(
    store: &Arc<SessionStore>,
) -> Result<HashMap<String, ManagedAgentSession>, String> {
    let mut sessions = HashMap::new();
    for row in store.list_sessions().map_err(|error| error.to_string())? {
        let provider: AgentConversationProvider = enum_from_storage(&row.provider)?;
        let mut stored: StoredSessionExtra = serde_json::from_str(&row.extra_json)
            .map_err(|error| format!("Could not decode stored session metadata: {error}"))?;
        if stored.rail_meta.worktree.is_none() {
            stored.rail_meta.worktree.clone_from(&row.worktree);
        }
        if stored.rail_meta.branch.is_none() {
            stored.rail_meta.branch.clone_from(&row.branch);
        }
        if session_title_is_empty(stored.rail_meta.title.as_deref())
            && !session_title_is_empty(row.title.as_deref())
        {
            stored.rail_meta.title.clone_from(&row.title);
        }
        if session_title_is_empty(stored.rail_meta.title.as_deref()) {
            stored.rail_meta.title = store
                .first_user_message_payload(&row.owned_id)
                .map_err(|error| error.to_string())?
                .map(|payload| {
                    serde_json::from_str::<AgentConversationEvent>(&payload).map_err(|error| {
                        format!("Could not decode stored first user message: {error}")
                    })
                })
                .transpose()?
                .and_then(|event| match event.payload {
                    AgentConversationPayload::UserMessage { text, .. } => prompt_title(&text),
                    _ => None,
                });
        }
        if stored.rail_meta.project.is_none() {
            stored.rail_meta.project.clone_from(&row.project);
        }
        let persisted_state: AgentRuntimeState = enum_from_storage(&row.state)?;
        let recoverable =
            row.native_session_id.is_some() && persisted_state != AgentRuntimeState::Closed;
        let state = if recoverable {
            AgentRuntimeState::Suspended
        } else {
            persisted_state
        };
        let connection_state = match state {
            AgentRuntimeState::Suspended | AgentRuntimeState::Closed => {
                ConversationConnectionState::Disconnected
            }
            AgentRuntimeState::Failed => ConversationConnectionState::Failed,
            _ => ConversationConnectionState::Disconnected,
        };
        let next_sequence = store
            .latest_seq(&row.owned_id)
            .map_err(|error| error.to_string())?
            .saturating_add(1) as u64;
        let connection = AgentConversationConnection {
            owned_id: row.owned_id.clone(),
            provider,
            generation: stored.generation,
            native_session_id: row.native_session_id.clone(),
            state: connection_state,
            config: stored.config.clone(),
        };
        let session = ManagedAgentSession {
            owned_id: row.owned_id.clone(),
            provider,
            provider_instance_id: format!("{}-{}", provider_id(provider), stored.generation),
            native_session_id: row.native_session_id,
            native_session_mode: AgentNativeSessionMode::Resume,
            generation: stored.generation,
            owner: stored.owner,
            state,
            capabilities: stored.capabilities,
            next_sequence,
            active_turn_id: None,
            prompt_once_active: false,
            runtime: None,
            pool_key: None,
            transport: None,
            recent_events: VecDeque::new(),
            writer_lease: AgentWriterLease {
                owned_id: row.owned_id.clone(),
                generation: stored.generation,
                owner: AgentWriterLeaseOwner::Structured,
            },
            writer_lease_transition: None,
            permission_requests: HashMap::new(),
            next_permission_id: 0,
            user_input_requests: HashMap::new(),
            next_user_input_id: 0,
            ordered_events: None,
            cwd: row.cwd,
            spawn_reasoning_effort: row.effort,
            config: stored.config,
            connection,
            store: Arc::clone(store),
            created_at_ms: row.created_at_ms.max(0) as u128,
            last_activity_ms: row.last_activity_at_ms.max(0) as u128,
            live_tool_calls: HashSet::new(),
            background_work: HashSet::new(),
            child_rollout_scan: None,
            child_rollout_parent_path: None,
            codex_children: HashMap::new(),
            rail_meta: stored.rail_meta,
            suspending: false,
        };
        persist_session(&session)?;
        sessions.insert(row.owned_id, session);
    }
    Ok(sessions)
}

fn enum_storage_value<T: Serialize>(value: T) -> Result<String, String> {
    let encoded = serde_json::to_value(value)
        .map_err(|error| format!("Could not encode stored enum value: {error}"))?;
    encoded
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "Stored enum value was not a string".to_string())
}

fn enum_from_storage<T: for<'de> Deserialize<'de>>(value: &str) -> Result<T, String> {
    serde_json::from_value(Value::String(value.to_string()))
        .map_err(|error| format!("Could not decode stored enum value: {error}"))
}

fn store_timestamp(value: u128) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn session_title_is_empty(title: Option<&str>) -> bool {
    title.is_none_or(|title| title.trim().is_empty())
}

fn prompt_title(prompt: &str) -> Option<String> {
    prompt
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.chars().take(SESSION_TITLE_CHAR_CAP).collect())
}

fn current_session<'a>(
    sessions: &'a HashMap<String, ManagedAgentSession>,
    owned_id: &str,
    generation: u64,
) -> Result<&'a ManagedAgentSession, String> {
    let session = sessions
        .get(owned_id)
        .ok_or_else(|| "Conversation session was not found".to_string())?;
    if session.generation != generation {
        return Err("Conversation connection changed; retry on the current session".to_string());
    }
    Ok(session)
}

fn current_session_mut<'a>(
    sessions: &'a mut HashMap<String, ManagedAgentSession>,
    owned_id: &str,
    generation: u64,
) -> Result<&'a mut ManagedAgentSession, String> {
    let session = sessions
        .get_mut(owned_id)
        .ok_or_else(|| "Conversation session was not found".to_string())?;
    if session.generation != generation {
        return Err("Conversation connection changed; retry on the current session".to_string());
    }
    Ok(session)
}

fn lifecycle_session_mut<'a>(
    sessions: &'a mut HashMap<String, ManagedAgentSession>,
    owned_id: &str,
    generation: u64,
) -> Result<&'a mut ManagedAgentSession, String> {
    let session = current_session_mut(sessions, owned_id, generation)?;
    if session.state == AgentRuntimeState::Closed
        || session.connection.state == ConversationConnectionState::Closed
    {
        return Err("Closed conversation sessions cannot be changed".to_string());
    }
    Ok(session)
}

/// Records one payload while leaving live memory untouched if SQLite rejects it.
fn record_payload_for_session(
    session: &mut ManagedAgentSession,
    payload: AgentConversationPayload,
) -> Result<AgentConversationEvent, String> {
    record_payload_for_session_with_lifecycle(session, payload, None)
}

/// Commits an event and an optional lifecycle change before publishing either in memory.
fn record_payload_for_session_with_lifecycle(
    session: &mut ManagedAgentSession,
    payload: AgentConversationPayload,
    lifecycle: Option<SessionLifecycleUpdate>,
) -> Result<AgentConversationEvent, String> {
    let mut candidate = SessionEventCandidate::from_session(session);
    if let Some(lifecycle) = lifecycle {
        candidate.transition_lifecycle(
            lifecycle.state,
            lifecycle.connection_state,
            lifecycle.owner,
            lifecycle.writer_owner,
        )?;
        if let Some(native_session_mode) = lifecycle.native_session_mode {
            candidate.native_session_mode = native_session_mode;
        }
    }
    let sequence = candidate.next_sequence;
    candidate.next_sequence = candidate.next_sequence.saturating_add(1);
    if let AgentConversationPayload::Connection {
        state,
        native_session_id,
    } = &payload
    {
        candidate.connection.state = *state;
        if native_session_id.is_some() {
            candidate.connection.native_session_id = native_session_id.clone();
            candidate.native_session_id = native_session_id.clone();
        }
    }
    if let AgentConversationPayload::AvailableCommandsUpdate { available_commands } = &payload {
        candidate.capabilities.commands = available_commands.clone();
    }
    let timestamp_ms = timestamp_millis();
    candidate.last_activity_ms = timestamp_ms;
    let frontend_event = AgentConversationEvent {
        owned_id: session.owned_id.clone(),
        provider: session.provider,
        generation: session.generation,
        sequence,
        timestamp_ms,
        payload: payload.clone(),
    };
    let canonical = canonical_event(
        session,
        candidate.native_session_id.clone(),
        sequence,
        timestamp_ms,
        &payload,
    )?;
    match &payload {
        AgentConversationPayload::Tool { item_id, state, .. } => match state {
            ToolState::Started | ToolState::Updated => {
                candidate.live_tool_calls.insert(item_id.clone());
            }
            ToolState::Completed | ToolState::Failed => {
                candidate.live_tool_calls.remove(item_id);
            }
        },
        AgentConversationPayload::Turn { state, .. }
            if matches!(
                state,
                super::protocol::TurnState::Completed
                    | super::protocol::TurnState::Interrupted
                    | super::protocol::TurnState::Failed
            ) =>
        {
            candidate.live_tool_calls.clear();
        }
        _ => {}
    }
    let payload_json = serde_json::to_string(&frontend_event)
        .map_err(|error| format!("Could not encode the stored conversation event: {error}"))?;
    let kind = enum_storage_value(canonical.event_type)?;
    let row = persisted_session_row_for_candidate(session, &candidate)?;
    let event_row = EventRow {
        owned_id: session.owned_id.clone(),
        seq: i64::try_from(sequence)
            .map_err(|_| "Conversation event sequence exceeded the store limit".to_string())?,
        turn_id: canonical.turn_id.clone(),
        kind,
        payload_json,
        created_at_ms: store_timestamp(timestamp_ms),
    };
    session
        .store
        .upsert_session_with_event(&row, Some(&event_row), STORE_EVENT_CAP)
        .map_err(|error| error.to_string())?;
    candidate.apply(session);
    session.recent_events.push_back(canonical);
    while session.recent_events.len() > SNAPSHOT_EVENT_CAP {
        session.recent_events.pop_front();
    }
    Ok(frontend_event)
}

/// Records durably, then emits while the session lock still preserves event order.
fn record_payload_for_session_and_dispatch(
    session: &mut ManagedAgentSession,
    emitter: &Arc<Mutex<Option<ConversationEmitter>>>,
    payload: AgentConversationPayload,
) -> Result<AgentConversationEvent, String> {
    let event = record_payload_for_session(session, payload)?;
    // The session mutex is the emit-order lock. Keep it held through the
    // callback so sequence allocation and frontend dispatch are one atomic
    // operation for this session.
    dispatch_event(emitter, &event);
    Ok(event)
}

/// Records a lifecycle event transactionally, then emits the committed event.
fn record_payload_for_session_and_dispatch_with_lifecycle(
    session: &mut ManagedAgentSession,
    emitter: &Arc<Mutex<Option<ConversationEmitter>>>,
    payload: AgentConversationPayload,
    lifecycle: SessionLifecycleUpdate,
) -> Result<AgentConversationEvent, String> {
    let event = record_payload_for_session_with_lifecycle(session, payload, Some(lifecycle))?;
    dispatch_event(emitter, &event);
    Ok(event)
}

/// Describes a state-only lifecycle change while preserving current ownership.
fn lifecycle_update_for_state(
    session: &ManagedAgentSession,
    state: AgentRuntimeState,
    connection_state: ConversationConnectionState,
) -> SessionLifecycleUpdate {
    SessionLifecycleUpdate {
        state,
        connection_state,
        owner: session.owner,
        writer_owner: session.writer_lease.owner,
        native_session_mode: None,
    }
}

fn dispatch_event(
    emitter: &Arc<Mutex<Option<ConversationEmitter>>>,
    event: &AgentConversationEvent,
) {
    let callback = emitter
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    if let Some(callback) = callback {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(event.clone())))
            .is_err()
        {
            crate::debug_log::stderr_log!(
                "Agent conversation event emitter panicked; sequence {} remains available in the session snapshot",
                event.sequence
            );
        }
    }
}

pub(crate) fn prompt_params(native_session_id: String, input: AgentPrompt) -> Value {
    let mut prompt = Vec::new();
    if !input.text.is_empty() {
        prompt.push(serde_json::json!({ "type": "text", "text": input.text }));
    }
    prompt.extend(input.images.into_iter().map(|image| {
        serde_json::json!({
            "type": "image",
            "data": image.data,
            "mimeType": image.mime_type
        })
    }));
    serde_json::json!({ "sessionId": native_session_id, "prompt": prompt })
}

fn spawn_prompt_completion(
    transport: Arc<AcpTransport>,
    ordered_events: UnboundedSender<OrderedSessionEvent>,
    turn_id: String,
    params: Value,
) {
    tokio::spawn(async move {
        let result = transport.request("session/prompt", params).await;
        let _ = ordered_events.send(OrderedSessionEvent::PromptResult { turn_id, result });
    });
}

fn spawn_inbound_pump(
    manager: AgentRuntimeManager,
    transport: Weak<AcpTransport>,
    inbound: mpsc::UnboundedReceiver<AcpInbound>,
    ordered_events: mpsc::UnboundedReceiver<OrderedSessionEvent>,
    owned_id: String,
    generation: u64,
) {
    tokio::spawn(pump_inbound(
        manager,
        transport,
        inbound,
        ordered_events,
        owned_id,
        generation,
    ));
}

fn spawn_ordered_pump(
    manager: AgentRuntimeManager,
    transport: Weak<AcpTransport>,
    mut ordered_events: mpsc::UnboundedReceiver<OrderedSessionEvent>,
    owned_id: String,
    generation: u64,
) {
    tokio::spawn(async move {
        while let Some(ordered) = ordered_events.recv().await {
            if !handle_ordered_session_event(ordered, &manager, &transport, &owned_id, generation)
                .await
            {
                return;
            }
        }
    });
}

async fn pump_inbound(
    manager: AgentRuntimeManager,
    transport: Weak<AcpTransport>,
    mut inbound: mpsc::UnboundedReceiver<AcpInbound>,
    mut ordered_events: mpsc::UnboundedReceiver<OrderedSessionEvent>,
    owned_id: String,
    generation: u64,
) {
    let mut ordered_open = true;
    loop {
        let inbound = if ordered_open {
            tokio::select! {
                biased;
                inbound = inbound.recv() => inbound,
                ordered = ordered_events.recv() => {
                    match ordered {
                        Some(ordered) => {
                            if !handle_ordered_session_event(
                                ordered,
                                &manager,
                                &transport,
                                &owned_id,
                                generation,
                            ).await {
                                return;
                            }
                        }
                        None => ordered_open = false,
                    }
                    continue;
                }
            }
        } else {
            inbound.recv().await
        };

        let Some(inbound) = inbound else {
            return;
        };
        let Some(transport_runtime) = transport.upgrade() else {
            return;
        };
        if let AcpInbound::TransportClosed { reason } = &inbound {
            settle_closed_transport(&manager, &transport_runtime, reason).await;
            return;
        }
        let target = {
            let sessions = manager
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            routed_session_for_inbound(&sessions, &transport_runtime, &inbound)
        };
        let Some((target_owned_id, target_generation)) = target else {
            crate::debug_log::stderr_log!(
                "Dropping an adapter event that did not identify one live session"
            );
            continue;
        };
        let owned_id = target_owned_id;
        let generation = target_generation;
        let sessions = Arc::clone(&manager.sessions);
        let emitter = Arc::clone(&manager.emitter);
        match inbound {
            AcpInbound::SessionUpdate(params) => {
                let reached_quiescence = 'update: {
                    let mut sessions = sessions
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let Ok(session) = current_session_mut(&mut sessions, &owned_id, generation)
                    else {
                        return;
                    };
                    let reached_quiescence = update_raw_liveness(session, &params);
                    if let Err(error) = persist_session(session) {
                        crate::debug_log::stderr_log!(
                            "Could not persist adapter liveness state: {error}"
                        );
                    }
                    if let Some(mode_id) = current_mode_update(&params) {
                        session.config.approval_policy = Some(mode_id.to_string());
                        session.connection.config = session.config.clone();
                        if let Err(error) = persist_session(session) {
                            crate::debug_log::stderr_log!(
                                "Could not persist adapter configuration: {error}"
                            );
                        }
                        break 'update reached_quiescence;
                    }
                    if is_session_state_update(&params) {
                        if session.writer_lease.owner != AgentWriterLeaseOwner::Structured {
                            break 'update reached_quiescence;
                        }
                        let Some(payload) = payload_from_session_update_for_turn(
                            &params,
                            session.active_turn_id.as_deref(),
                        ) else {
                            break 'update reached_quiescence;
                        };
                        if let Err(error) =
                            record_payload_for_session_and_dispatch(session, &emitter, payload)
                        {
                            crate::debug_log::stderr_log!(
                                "Could not record ACP session state update: {error}"
                            );
                        }
                        break 'update reached_quiescence;
                    }
                    let replay = is_replay_session_update(&params);
                    if replay
                        && session.native_session_mode == AgentNativeSessionMode::Resume
                        && session
                            .store
                            .has_display_events(&session.owned_id)
                            .unwrap_or_else(|error| {
                                crate::debug_log::stderr_log!(
                                    "Could not inspect existing conversation history: {error}"
                                );
                                false
                            })
                    {
                        break 'update reached_quiescence;
                    }
                    if session.active_turn_id.is_none() && !replay {
                        crate::debug_log::stderr_log!(
                            "[debug] Dropping ACP session update without an active conversation turn"
                        );
                        break 'update reached_quiescence;
                    }
                    if session.writer_lease.owner != AgentWriterLeaseOwner::Structured {
                        break 'update reached_quiescence;
                    }
                    let Some(payload) = payload_from_session_update_for_turn(
                        &params,
                        session.active_turn_id.as_deref(),
                    ) else {
                        break 'update reached_quiescence;
                    };
                    if let Err(error) =
                        record_payload_for_session_and_dispatch(session, &emitter, payload)
                    {
                        crate::debug_log::stderr_log!(
                            "Could not record ACP session update: {error}"
                        );
                    }
                    reached_quiescence
                };
                if reached_quiescence {
                    if let Err(error) = manager.suspend_if_quiescent(&owned_id, generation).await {
                        crate::debug_log::stderr_log!(
                            "Could not tear down quiescent runtime: {error}"
                        );
                    }
                }
            }
            AcpInbound::AgentRequest {
                wire_id,
                method,
                params,
            } => {
                if method != "session/request_permission"
                    && method != "session/request_user_input"
                    && method != "session/request_input"
                {
                    crate::debug_log::stderr_log!(
                        "Ignoring unsupported ACP agent request: {method}"
                    );
                    continue;
                }
                if method == "session/request_user_input" || method == "session/request_input" {
                    let title = params
                        .get("title")
                        .and_then(Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or("Input requested")
                        .to_string();
                    let description = params
                        .get("description")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let fields = params
                        .get("fields")
                        .cloned()
                        .and_then(|value| serde_json::from_value(value).ok())
                        .unwrap_or_default();
                    let mut sessions = sessions
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let Ok(session) = current_session_mut(&mut sessions, &owned_id, generation)
                    else {
                        return;
                    };
                    if session.writer_lease.owner != AgentWriterLeaseOwner::Structured {
                        continue;
                    }
                    session.next_user_input_id = session.next_user_input_id.saturating_add(1);
                    let request_id = format!("input-{}", session.next_user_input_id);
                    session
                        .user_input_requests
                        .insert(request_id.clone(), PendingUserInput { wire_id });
                    let lifecycle = lifecycle_update_for_state(
                        session,
                        AgentRuntimeState::WaitingInput,
                        session.connection.state,
                    );
                    if let Err(error) = record_payload_for_session_and_dispatch_with_lifecycle(
                        session,
                        &emitter,
                        AgentConversationPayload::UserInputRequested {
                            request_id,
                            title,
                            description,
                            fields,
                        },
                        lifecycle,
                    ) {
                        crate::debug_log::stderr_log!(
                            "Could not record ACP user input request: {error}"
                        );
                    }
                    continue;
                }
                let summary = permission_summary(&params);
                let options = permission_options(&params);
                let mut sessions = sessions
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let Ok(session) = current_session_mut(&mut sessions, &owned_id, generation) else {
                    return;
                };
                if session.writer_lease.owner != AgentWriterLeaseOwner::Structured {
                    continue;
                }
                session.next_permission_id = session.next_permission_id.saturating_add(1);
                let request_id = format!("perm-{}", session.next_permission_id);
                session.permission_requests.insert(
                    request_id.clone(),
                    PendingPermission {
                        wire_id,
                        options,
                        summary: summary.clone(),
                    },
                );
                let lifecycle = lifecycle_update_for_state(
                    session,
                    AgentRuntimeState::WaitingApproval,
                    session.connection.state,
                );
                if let Err(error) = record_payload_for_session_and_dispatch_with_lifecycle(
                    session,
                    &emitter,
                    AgentConversationPayload::Approval {
                        request_id,
                        state: ApprovalState::Requested,
                        summary,
                    },
                    lifecycle,
                ) {
                    crate::debug_log::stderr_log!(
                        "Could not record ACP permission request: {error}"
                    );
                }
            }
            AcpInbound::TransportClosed { .. } => unreachable!("handled before session routing"),
        }
    }
}

async fn settle_closed_transport(
    manager: &AgentRuntimeManager,
    transport: &Arc<AcpTransport>,
    reason: &str,
) {
    let emitter = Arc::clone(&manager.emitter);
    {
        let mut sessions = manager
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for session in sessions.values_mut().filter(|session| {
            session
                .transport
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, transport))
        }) {
            if let Some(task) = session.child_rollout_scan.take() {
                task.abort();
            }
            session.runtime = None;
            session.transport = None;
            session.ordered_events = None;
            session.pool_key = None;
            if session.suspending {
                continue;
            }
            for (request_id, pending) in session.permission_requests.drain().collect::<Vec<_>>() {
                let _ = record_payload_for_session_and_dispatch(
                    session,
                    &emitter,
                    AgentConversationPayload::Approval {
                        request_id,
                        state: ApprovalState::Expired,
                        summary: pending.summary,
                    },
                );
            }
            for (request_id, _) in session.user_input_requests.drain().collect::<Vec<_>>() {
                let _ = record_payload_for_session_and_dispatch(
                    session,
                    &emitter,
                    AgentConversationPayload::UserInputResolved {
                        request_id,
                        cancelled: true,
                    },
                );
            }
            if let Some(turn_id) = session.active_turn_id.take() {
                let _ = record_payload_for_session_and_dispatch(
                    session,
                    &emitter,
                    AgentConversationPayload::Turn {
                        turn_id,
                        state: super::protocol::TurnState::Failed,
                    },
                );
            }
            session.prompt_once_active = false;
            let lifecycle = lifecycle_update_for_state(
                session,
                AgentRuntimeState::Failed,
                ConversationConnectionState::Failed,
            );
            let _ = record_payload_for_session_and_dispatch_with_lifecycle(
                session,
                &emitter,
                AgentConversationPayload::Connection {
                    state: ConversationConnectionState::Failed,
                    native_session_id: session.native_session_id.clone(),
                },
                lifecycle,
            );
            let _ = record_payload_for_session_and_dispatch(
                session,
                &emitter,
                AgentConversationPayload::Error {
                    code: "acp-transport".to_string(),
                    message: reason.to_string(),
                    recoverable: true,
                },
            );
        }
    }
    manager
        .adapter_pools
        .lock()
        .await
        .retain(|_, pool| !Arc::ptr_eq(&pool.transport, transport));
}

fn routed_session_for_inbound(
    sessions: &HashMap<String, ManagedAgentSession>,
    transport: &Arc<AcpTransport>,
    inbound: &AcpInbound,
) -> Option<(String, u64)> {
    let params = match inbound {
        AcpInbound::SessionUpdate(params) | AcpInbound::AgentRequest { params, .. } => Some(params),
        AcpInbound::TransportClosed { .. } => None,
    };
    let native_session_id = params.and_then(|params| {
        params
            .get("sessionId")
            .or_else(|| params.get("session_id"))
            .or_else(|| params.pointer("/update/sessionId"))
            .or_else(|| params.pointer("/update/session_id"))
            .and_then(Value::as_str)
    });
    let matching_transport = |session: &&ManagedAgentSession| {
        session
            .transport
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, transport))
    };
    if let Some(native_session_id) = native_session_id {
        return sessions
            .values()
            .filter(matching_transport)
            .find(|session| session.native_session_id.as_deref() == Some(native_session_id))
            .map(|session| (session.owned_id.clone(), session.generation));
    }
    let matching = sessions
        .values()
        .filter(matching_transport)
        .collect::<Vec<_>>();
    if let [session] = matching.as_slice() {
        return Some((session.owned_id.clone(), session.generation));
    }
    let mut active = matching
        .into_iter()
        .filter(|session| session.active_turn_id.is_some());
    let session = active.next()?;
    active
        .next()
        .is_none()
        .then(|| (session.owned_id.clone(), session.generation))
}

fn update_raw_liveness(session: &mut ManagedAgentSession, params: &Value) -> bool {
    let update = params.get("update").unwrap_or(params);
    let kind = update
        .get("sessionUpdate")
        .or_else(|| update.get("session_update"))
        .or_else(|| update.get("type"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let state = update
        .get("status")
        .or_else(|| update.get("state"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let terminal = matches!(
        state.as_str(),
        "completed" | "failed" | "cancelled" | "canceled" | "stopped" | "done"
    );
    let identifier = update
        .get("toolCallId")
        .or_else(|| update.get("tool_call_id"))
        .or_else(|| update.get("taskId"))
        .or_else(|| update.get("task_id"))
        .or_else(|| update.get("childSessionId"))
        .or_else(|| update.get("child_session_id"))
        .or_else(|| update.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let Some(identifier) = identifier else {
        return false;
    };
    let target = if kind.contains("tool") {
        &mut session.live_tool_calls
    } else if kind.contains("task")
        || kind.contains("child")
        || kind.contains("subagent")
        || kind.contains("background")
    {
        &mut session.background_work
    } else {
        return false;
    };
    if terminal {
        target.remove(&identifier);
    } else {
        target.insert(identifier);
    }
    terminal && session_is_quiescent(session)
}

async fn run_child_rollout_scan(
    sessions: Weak<Mutex<HashMap<String, ManagedAgentSession>>>,
    emitter: Arc<Mutex<Option<ConversationEmitter>>>,
    owned_id: String,
    generation: u64,
    native_session_id: String,
) {
    let mut parent_path = None;
    loop {
        let Some(outcome) = scan_codex_children_once(
            &sessions,
            &emitter,
            &owned_id,
            generation,
            &native_session_id,
            parent_path,
        )
        .await
        else {
            return;
        };
        parent_path = Some(outcome.parent_path);
        // After the turn ends, keep polling only while a child is still working.
        if !outcome.keep_scanning {
            return;
        }
        tokio::time::sleep(CHILD_ROLLOUT_SCAN_INTERVAL).await;
    }
}

struct ChildRolloutScanOutcome {
    parent_path: PathBuf,
    has_running_children: bool,
    keep_scanning: bool,
}

async fn scan_codex_children_once(
    sessions: &Weak<Mutex<HashMap<String, ManagedAgentSession>>>,
    emitter: &Arc<Mutex<Option<ConversationEmitter>>>,
    owned_id: &str,
    generation: u64,
    native_session_id: &str,
    parent_path: Option<PathBuf>,
) -> Option<ChildRolloutScanOutcome> {
    let native_id = native_session_id.to_string();
    let scan_parent_id = native_id.clone();
    let scan = tokio::task::spawn_blocking(move || {
        let path = match parent_path {
            Some(path) => path,
            None => {
                transcript::discover(AgentConversationProvider::Codex, &native_id)
                    .ok()
                    .flatten()?
                    .canonical_path
            }
        };
        let active_after = SystemTime::now()
            .checked_sub(CHILD_ROLLOUT_SCAN_INTERVAL.saturating_mul(2))
            .unwrap_or(UNIX_EPOCH);
        transcript::scan_codex_child_rollouts(&path, &scan_parent_id, active_after)
            .ok()
            .map(|children| (path, children))
    })
    .await
    .ok()
    .flatten()?;
    let (parent_path, children) = scan;
    let sessions = sessions.upgrade()?;
    let mut sessions = sessions
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let session = current_session_mut(&mut sessions, owned_id, generation).ok()?;
    let turn_active = session.active_turn_id.is_some();
    let had_running_children = session
        .codex_children
        .values()
        .any(|child| child.state == "running");
    if session.provider != AgentConversationProvider::Codex
        || session.state == AgentRuntimeState::Closed
        // Once the turn ends, only a previously running child earns another scan.
        || (!turn_active && !had_running_children)
    {
        return None;
    }
    session.child_rollout_parent_path = Some(parent_path.clone());
    let (updates, has_running_children, keep_scanning) = child_rollout_scan_updates(
        native_session_id,
        &mut session.codex_children,
        children,
        turn_active,
    );
    for payload in updates {
        if let Err(error) = record_payload_for_session_and_dispatch(session, emitter, payload) {
            crate::debug_log::stderr_log!("Could not record child session update: {error}");
            break;
        }
    }
    Some(ChildRolloutScanOutcome {
        parent_path,
        has_running_children,
        keep_scanning,
    })
}

fn child_rollout_scan_updates(
    parent_id: &str,
    known: &mut HashMap<String, CodexChildRollout>,
    children: Vec<CodexChildRollout>,
    turn_active: bool,
) -> (Vec<AgentConversationPayload>, bool, bool) {
    let has_running_children = children.iter().any(|child| child.state == "running");
    (
        changed_child_updates(parent_id, known, children),
        has_running_children,
        turn_active || has_running_children,
    )
}

fn changed_child_updates(
    parent_id: &str,
    known: &mut HashMap<String, CodexChildRollout>,
    children: Vec<CodexChildRollout>,
) -> Vec<AgentConversationPayload> {
    let mut updates = Vec::new();
    for child in children {
        if known.get(&child.child_id) == Some(&child) {
            continue;
        }
        updates.push(AgentConversationPayload::ChildUpdate {
            child_id: child.child_id.clone(),
            parent_tool_call_id: parent_id.to_string(),
            label: Some(child.label.clone()),
            state: child.state.clone(),
            latest_activity: Some(child.latest_activity.clone()),
        });
        known.insert(child.child_id.clone(), child);
    }
    updates
}

async fn handle_ordered_session_event(
    ordered: OrderedSessionEvent,
    manager: &AgentRuntimeManager,
    transport: &Weak<AcpTransport>,
    owned_id: &str,
    generation: u64,
) -> bool {
    let sessions = Arc::clone(&manager.sessions);
    let emitter = Arc::clone(&manager.emitter);
    match ordered {
        OrderedSessionEvent::ApprovalResolved {
            request_id,
            state,
            summary,
        } => {
            let mut sessions = sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Ok(session) = current_session_mut(&mut sessions, owned_id, generation) else {
                return false;
            };
            if session.writer_lease.owner != AgentWriterLeaseOwner::Structured {
                return true;
            }
            let next_state = if session.active_turn_id.is_some() {
                AgentRuntimeState::Working
            } else {
                AgentRuntimeState::Ready
            };
            let lifecycle =
                lifecycle_update_for_state(session, next_state, session.connection.state);
            if let Err(error) = record_payload_for_session_and_dispatch_with_lifecycle(
                session,
                &emitter,
                AgentConversationPayload::Approval {
                    request_id,
                    state,
                    summary,
                },
                lifecycle,
            ) {
                crate::debug_log::stderr_log!(
                    "Could not record ACP permission resolution: {error}"
                );
                return false;
            }
            true
        }
        OrderedSessionEvent::UserInputResolved {
            request_id,
            cancelled,
        } => {
            let mut sessions = sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Ok(session) = current_session_mut(&mut sessions, owned_id, generation) else {
                return false;
            };
            if session.writer_lease.owner != AgentWriterLeaseOwner::Structured {
                return true;
            }
            let next_state = if session.active_turn_id.is_some() {
                AgentRuntimeState::Working
            } else {
                AgentRuntimeState::Ready
            };
            let lifecycle =
                lifecycle_update_for_state(session, next_state, session.connection.state);
            if let Err(error) = record_payload_for_session_and_dispatch_with_lifecycle(
                session,
                &emitter,
                AgentConversationPayload::UserInputResolved {
                    request_id,
                    cancelled,
                },
                lifecycle,
            ) {
                crate::debug_log::stderr_log!(
                    "Could not record ACP user input resolution: {error}"
                );
                return false;
            }
            true
        }
        OrderedSessionEvent::PromptResult { turn_id, result } => {
            manager
                .finish_child_rollout_scan(owned_id, generation, &turn_id)
                .await;
            let cancelled = result
                .as_ref()
                .ok()
                .and_then(stop_reason)
                .is_some_and(|reason| reason == "cancelled");
            let error_details = result
                .as_ref()
                .err()
                .map(|error| (error.code.to_string(), error.message.clone()));

            // Approval draining and terminal event emission are one atomic
            // session-lock operation. If transport closure wins this lock,
            // it emits the same terminal approval/turn events and this handler
            // returns without duplicating them.
            let (pending_permissions, pending_inputs) = {
                let mut sessions = sessions
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let Ok(session) = current_session_mut(&mut sessions, owned_id, generation) else {
                    return false;
                };
                if session.active_turn_id.as_deref() != Some(turn_id.as_str())
                    || session.writer_lease.owner != AgentWriterLeaseOwner::Structured
                {
                    return true;
                }
                let pending_permissions = session.permission_requests.drain().collect::<Vec<_>>();
                for (request_id, pending) in &pending_permissions {
                    if let Err(error) = record_payload_for_session_and_dispatch(
                        session,
                        &emitter,
                        AgentConversationPayload::Approval {
                            request_id: request_id.clone(),
                            state: if cancelled {
                                ApprovalState::Declined
                            } else {
                                ApprovalState::Expired
                            },
                            summary: pending.summary.clone(),
                        },
                    ) {
                        crate::debug_log::stderr_log!(
                            "Could not expire ACP permission request: {error}"
                        );
                        return false;
                    }
                }
                let pending_inputs = session.user_input_requests.drain().collect::<Vec<_>>();
                for (request_id, _pending) in &pending_inputs {
                    if let Err(error) = record_payload_for_session_and_dispatch(
                        session,
                        &emitter,
                        AgentConversationPayload::UserInputResolved {
                            request_id: request_id.clone(),
                            cancelled: true,
                        },
                    ) {
                        crate::debug_log::stderr_log!(
                            "Could not expire ACP user input request: {error}"
                        );
                        return false;
                    }
                }
                let payload = match result {
                    Ok(_) if cancelled => AgentConversationPayload::Turn {
                        turn_id: turn_id.clone(),
                        state: super::protocol::TurnState::Interrupted,
                    },
                    Ok(_) => AgentConversationPayload::Turn {
                        turn_id: turn_id.clone(),
                        state: super::protocol::TurnState::Completed,
                    },
                    Err(_) => AgentConversationPayload::Turn {
                        turn_id: turn_id.clone(),
                        state: super::protocol::TurnState::Failed,
                    },
                };
                let lifecycle = lifecycle_update_for_state(
                    session,
                    AgentRuntimeState::Ready,
                    session.connection.state,
                );
                if let Err(error) = record_payload_for_session_and_dispatch_with_lifecycle(
                    session, &emitter, payload, lifecycle,
                ) {
                    crate::debug_log::stderr_log!("Could not record ACP turn completion: {error}");
                    return false;
                }
                if let Some((code, message)) = error_details {
                    if let Err(error) = record_payload_for_session_and_dispatch(
                        session,
                        &emitter,
                        AgentConversationPayload::Error {
                            code,
                            message,
                            recoverable: true,
                        },
                    ) {
                        crate::debug_log::stderr_log!("Could not record ACP prompt error: {error}");
                        return false;
                    }
                }
                session.active_turn_id = None;
                session.prompt_once_active = false;
                (pending_permissions, pending_inputs)
            };

            // The wire responses are transport I/O and must not hold the
            // session lock. Lifecycle emission above is complete regardless of
            // whether the transport is still able to accept these responses.
            if let Some(transport) = transport.upgrade() {
                for (_, pending) in pending_permissions {
                    if let Err(error) = transport
                        .respond(
                            pending.wire_id,
                            serde_json::json!({ "outcome": { "outcome": "cancelled" } }),
                        )
                        .await
                    {
                        crate::debug_log::stderr_log!(
                            "Could not expire ACP permission request: {error}"
                        );
                    }
                }
                for (_, pending) in pending_inputs {
                    if let Err(error) = transport
                        .respond(
                            pending.wire_id,
                            serde_json::json!({ "values": {}, "cancelled": true }),
                        )
                        .await
                    {
                        crate::debug_log::stderr_log!(
                            "Could not expire ACP user input request: {error}"
                        );
                    }
                }
            }
            // A finished turn stops the adapter process as soon as no prompt,
            // approval, input request, or tool work remains. The stored native
            // session survives for resume on the next send; only the idle
            // process tree is removed because those trees otherwise retain
            // hundreds of megabytes between turns.
            if let Err(error) = manager.suspend_if_quiescent(owned_id, generation).await {
                crate::debug_log::stderr_log!("Could not tear down quiescent runtime: {error}");
            }
            true
        }
    }
}

fn stop_reason(response: &Value) -> Option<&str> {
    response
        .get("stopReason")
        .or_else(|| response.get("stop_reason"))
        .and_then(Value::as_str)
}

fn current_mode_update(params: &Value) -> Option<&str> {
    let update = params
        .get("update")
        .or_else(|| params.get("sessionUpdate"))
        .unwrap_or(params);
    update
        .get("sessionUpdate")
        .or_else(|| update.get("session_update"))
        .or_else(|| update.get("type"))
        .and_then(Value::as_str)
        .filter(|kind| *kind == "current_mode_update")?;
    update
        .get("currentModeId")
        .or_else(|| update.get("current_mode_id"))
        .and_then(Value::as_str)
}

fn permission_summary(params: &Value) -> String {
    params
        .get("toolCall")
        .and_then(|tool| tool.get("title").or_else(|| tool.get("name")))
        .or_else(|| params.get("title"))
        .and_then(Value::as_str)
        .filter(|summary| !summary.is_empty())
        .unwrap_or("Permission requested")
        .to_string()
}

fn permission_options(params: &Value) -> Vec<PermissionOption> {
    params
        .get("options")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|option| {
            let option_id = option
                .get("optionId")
                .or_else(|| option.get("option_id"))
                .and_then(Value::as_str)
                .filter(|option_id| !option_id.is_empty())?
                .to_string();
            Some(PermissionOption {
                option_id,
                kind: option
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            })
        })
        .collect()
}

fn permission_response(
    options: &[PermissionOption],
    decision: AgentApprovalDecision,
) -> Result<Value, String> {
    match decision {
        AgentApprovalDecision::Cancel => Ok(serde_json::json!({
            "outcome": { "outcome": "cancelled" }
        })),
        AgentApprovalDecision::Accept => {
            let option_id = options
                .iter()
                .find(|option| is_allow_kind(&option.kind))
                .or_else(|| options.first())
                .map(|option| option.option_id.clone())
                .ok_or_else(|| "ACP permission request did not provide an option".to_string())?;
            Ok(serde_json::json!({
                "outcome": { "outcome": "selected", "optionId": option_id }
            }))
        }
        AgentApprovalDecision::Decline => {
            let option_id = options
                .iter()
                .find(|option| is_reject_kind(&option.kind))
                .map(|option| option.option_id.clone())
                .ok_or_else(|| {
                    "ACP permission request did not provide a reject option".to_string()
                })?;
            Ok(serde_json::json!({
                "outcome": { "outcome": "selected", "optionId": option_id }
            }))
        }
    }
}

fn permission_response_for_selection(
    options: &[PermissionOption],
    selection: &PermissionSelection,
) -> Result<Value, String> {
    match selection {
        PermissionSelection::Decision(decision) => permission_response(options, *decision),
        PermissionSelection::Option(option_id) => {
            if options.iter().any(|option| option.option_id == *option_id) {
                Ok(serde_json::json!({
                    "outcome": { "outcome": "selected", "optionId": option_id }
                }))
            } else {
                Err("ACP permission option is not part of the pending request".to_string())
            }
        }
    }
}

fn permission_state(
    options: &[PermissionOption],
    selection: &PermissionSelection,
) -> ApprovalState {
    match selection {
        PermissionSelection::Decision(AgentApprovalDecision::Accept) => ApprovalState::Accepted,
        PermissionSelection::Decision(
            AgentApprovalDecision::Decline | AgentApprovalDecision::Cancel,
        ) => ApprovalState::Declined,
        PermissionSelection::Option(option_id) => options
            .iter()
            .find(|option| option.option_id == *option_id)
            .filter(|option| !is_reject_kind(&option.kind))
            .map(|_| ApprovalState::Accepted)
            .unwrap_or(ApprovalState::Declined),
    }
}

fn is_allow_kind(kind: &str) -> bool {
    kind.eq_ignore_ascii_case("allow") || kind.to_ascii_lowercase().starts_with("allow_")
}

fn is_reject_kind(kind: &str) -> bool {
    kind.eq_ignore_ascii_case("reject") || kind.to_ascii_lowercase().starts_with("reject_")
}

fn canonical_event(
    session: &ManagedAgentSession,
    native_session_id: Option<String>,
    sequence: u64,
    timestamp_ms: u128,
    payload: &AgentConversationPayload,
) -> Result<AgentEvent, String> {
    let event_type = match payload {
        AgentConversationPayload::Connection {
            state: ConversationConnectionState::Closed,
            ..
        } => AgentEventType::SessionClosed,
        AgentConversationPayload::Connection { .. } => AgentEventType::SessionStateChanged,
        AgentConversationPayload::UserMessage { .. } => AgentEventType::ItemCompleted,
        AgentConversationPayload::AssistantDelta { .. } => AgentEventType::ContentDelta,
        AgentConversationPayload::AssistantMessage { .. } => AgentEventType::ItemCompleted,
        AgentConversationPayload::Tool { .. } => AgentEventType::ItemUpdated,
        AgentConversationPayload::ChildUpdate { .. } => AgentEventType::ChildrenUpdated,
        AgentConversationPayload::Approval { .. } => AgentEventType::ApprovalRequested,
        AgentConversationPayload::UserInputRequested { .. } => AgentEventType::UserInputRequested,
        AgentConversationPayload::UserInputResolved { .. } => AgentEventType::UserInputResolved,
        AgentConversationPayload::Plan { .. } => AgentEventType::PlanUpdated,
        AgentConversationPayload::AvailableCommandsUpdate { .. } => AgentEventType::CommandsUpdated,
        AgentConversationPayload::Turn {
            state: super::protocol::TurnState::Started,
            ..
        } => AgentEventType::TurnStarted,
        AgentConversationPayload::Turn {
            state: super::protocol::TurnState::Interrupted,
            ..
        } => AgentEventType::TurnInterrupted,
        AgentConversationPayload::Turn { .. } => AgentEventType::TurnCompleted,
        AgentConversationPayload::Usage { .. } => AgentEventType::UsageUpdated,
        AgentConversationPayload::TerminalProjection(projection) => projection.event_type,
        AgentConversationPayload::Error { .. } => AgentEventType::RuntimeError,
    };
    if let AgentConversationPayload::TerminalProjection(projection) = payload {
        return Ok(AgentEvent {
            event_type,
            owned_id: session.owned_id.clone(),
            provider: session.provider,
            provider_instance_id: projection.provider_instance_id.clone(),
            generation: session.generation,
            sequence,
            timestamp_ms: projection
                .timestamp_ms
                .map(u128::from)
                .unwrap_or(timestamp_ms),
            native_session_id: Some(projection.native_session_id.clone()),
            turn_id: None,
            item_id: projection.item_id.clone(),
            request_id: None,
            payload: projection.payload.clone(),
            provider_metadata: Some(projection.provider_metadata.clone()),
            raw_frame_reference: Some(projection.raw_frame_reference.clone()),
        });
    }
    let payload_value = serde_json::to_value(payload).map_err(|error| error.to_string())?;
    let payload = payload_value
        .as_object()
        .map(|object| {
            object
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_else(BTreeMap::new);
    Ok(AgentEvent {
        event_type,
        owned_id: session.owned_id.clone(),
        provider: session.provider,
        provider_instance_id: session.provider_instance_id.clone(),
        generation: session.generation,
        sequence,
        timestamp_ms,
        native_session_id,
        turn_id: session.active_turn_id.clone(),
        item_id: None,
        request_id: None,
        payload,
        provider_metadata: None,
        raw_frame_reference: None,
    })
}

pub(crate) fn frontend_payload_from_canonical(
    event: &AgentEvent,
) -> Result<AgentConversationPayload, String> {
    let object = event
        .payload
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    serde_json::from_value(Value::Object(object))
        .map_err(|error| format!("Could not decode legacy conversation payload: {error}"))
}

fn session_update_kind(params: &Value) -> Option<&str> {
    let update = params.get("update").unwrap_or(params);
    update
        .get("sessionUpdate")
        .or_else(|| update.get("session_update"))
        .or_else(|| update.get("type"))
        .and_then(Value::as_str)
}

fn is_session_state_update(params: &Value) -> bool {
    matches!(
        session_update_kind(params),
        Some(
            "available_commands_update"
                | "available-commands-update"
                | "usage_update"
                | "usage-update"
                | "token_count"
                | "token-count"
        )
    )
}

fn parse_command_descriptors(value: Option<&Value>) -> Vec<AgentCommandDescriptor> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let id = entry
                .get("id")
                .or_else(|| entry.get("name"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())?
                .to_string();
            let label = entry
                .get("label")
                .or_else(|| entry.get("name"))
                .or_else(|| entry.get("id"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(&id)
                .to_string();
            let description = entry
                .get("description")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let input = entry.get("input");
            let input_hint = entry
                .get("inputHint")
                .or_else(|| entry.get("input_hint"))
                .and_then(Value::as_str)
                .or_else(|| {
                    input
                        .and_then(|input| input.get("hint"))
                        .and_then(Value::as_str)
                })
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let provider_metadata = entry
                .get("providerMetadata")
                .or_else(|| entry.get("provider_metadata"))
                .and_then(|metadata| serde_json::from_value(metadata.clone()).ok());
            Some(AgentCommandDescriptor {
                id,
                label,
                description,
                input_hint,
                provider_metadata,
            })
        })
        .collect()
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
}

fn first_u64(value: &Value, paths: &[&[&str]]) -> Option<u64> {
    paths
        .iter()
        .find_map(|path| value_at(value, path).and_then(Value::as_u64))
}

fn parse_usage_payload(update: &Value) -> Option<AgentConversationPayload> {
    let usage = update.get("usage").unwrap_or(update);
    let used_tokens = first_u64(
        update,
        &[
            &["used"],
            &["usedTokens"],
            &["used_tokens"],
            &["totalTokens"],
            &["total_tokens"],
            &["info", "total_token_usage", "total_tokens"],
        ],
    )
    .or_else(|| {
        first_u64(
            usage,
            &[
                &["used"],
                &["usedTokens"],
                &["used_tokens"],
                &["totalTokens"],
                &["total_tokens"],
            ],
        )
    });
    let context_window = first_u64(
        update,
        &[
            &["size"],
            &["contextWindow"],
            &["context_window"],
            &["modelContextWindow"],
            &["model_context_window"],
            &["info", "model_context_window"],
        ],
    )
    .or_else(|| {
        first_u64(
            usage,
            &[
                &["size"],
                &["contextWindow"],
                &["context_window"],
                &["modelContextWindow"],
                &["model_context_window"],
            ],
        )
    });
    let input_tokens = first_u64(update, &[&["inputTokens"], &["input_tokens"]])
        .or_else(|| first_u64(usage, &[&["inputTokens"], &["input_tokens"]]));
    let output_tokens = first_u64(update, &[&["outputTokens"], &["output_tokens"]])
        .or_else(|| first_u64(usage, &[&["outputTokens"], &["output_tokens"]]));
    (used_tokens.is_some()
        || context_window.is_some()
        || input_tokens.is_some()
        || output_tokens.is_some())
    .then_some(AgentConversationPayload::Usage {
        input_tokens,
        output_tokens,
        used_tokens,
        context_window,
    })
}

fn payload_from_session_update_for_turn(
    params: &Value,
    active_turn_id: Option<&str>,
) -> Option<AgentConversationPayload> {
    let update = params.get("update").unwrap_or(params);
    let replay = is_replay_session_update(params);
    let Some(kind) = session_update_kind(params) else {
        crate::debug_log::stderr_log!("Ignoring ACP session update without a kind");
        return None;
    };
    let turn_id = active_turn_id.unwrap_or_else(|| {
        update
            .get("turnId")
            .or_else(|| update.get("turn_id"))
            .or_else(|| params.get("turnId"))
            .or_else(|| params.get("turn_id"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    });

    if matches!(kind, "tool_call" | "tool_call_update")
        && update.get("kind").and_then(Value::as_str) == Some("subagent")
    {
        return child_update_payload(update, kind);
    }
    if matches!(kind, "tool_call" | "tool_call_update")
        && update
            .pointer("/_meta/claudeCode/toolName")
            .and_then(Value::as_str)
            == Some("Task")
    {
        return task_child_update_payload(update, kind);
    }

    match kind {
        "available_commands_update" | "available-commands-update" => {
            Some(AgentConversationPayload::AvailableCommandsUpdate {
                available_commands: parse_command_descriptors(
                    update
                        .get("availableCommands")
                        .or_else(|| update.get("available_commands")),
                ),
            })
        }
        "usage_update" | "usage-update" | "token_count" | "token-count" => {
            parse_usage_payload(update)
        }
        "agent_message_chunk" if replay => Some(AgentConversationPayload::AssistantMessage {
            item_id: update
                .get("messageId")
                .or_else(|| update.get("message_id"))
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("assistant-{turn_id}")),
            text: update
                .get("content")
                .and_then(text_from_value)
                .unwrap_or_default(),
            completed: true,
        }),
        "agent_message_chunk" => Some(AgentConversationPayload::AssistantDelta {
            item_id: update
                .get("messageId")
                .or_else(|| update.get("message_id"))
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("assistant-{turn_id}")),
            delta: update
                .get("content")
                .and_then(text_from_value)
                .unwrap_or_default(),
        }),
        "user_message_chunk" => Some(AgentConversationPayload::UserMessage {
            item_id: update
                .get("messageId")
                .or_else(|| update.get("message_id"))
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("user-{turn_id}")),
            text: update
                .get("content")
                .and_then(text_from_value)
                .unwrap_or_default(),
            completed: replay,
        }),
        "agent_thought_chunk" => {
            crate::debug_log::stderr_log!("Ignoring ACP agent thought update");
            None
        }
        "tool_call" => Some(AgentConversationPayload::Tool {
            item_id: tool_item_id(update, turn_id),
            name: update
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Tool")
                .to_string(),
            state: ToolState::Started,
            summary: tool_summary(update),
        }),
        "tool_call_update" => {
            let state = match update.get("status").and_then(Value::as_str) {
                Some("completed") => ToolState::Completed,
                Some("failed") => ToolState::Failed,
                Some("in_progress") | None => ToolState::Updated,
                Some(status) => {
                    crate::debug_log::stderr_log!(
                        "Ignoring unknown ACP tool status while retaining update: {status}"
                    );
                    ToolState::Updated
                }
            };
            Some(AgentConversationPayload::Tool {
                item_id: tool_item_id(update, turn_id),
                name: update
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("Tool")
                    .to_string(),
                state,
                summary: tool_summary(update),
            })
        }
        "plan" => Some(AgentConversationPayload::Plan {
            items: update
                .get("entries")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|entry| PlanItem {
                    text: entry
                        .get("content")
                        .and_then(text_from_value)
                        .unwrap_or_default(),
                    status: entry
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("pending")
                        .to_string(),
                })
                .collect(),
        }),
        unknown => {
            crate::debug_log::stderr_log!("Ignoring unknown ACP session update kind: {unknown}");
            None
        }
    }
}

fn child_update_payload(update: &Value, update_kind: &str) -> Option<AgentConversationPayload> {
    let child_id = update
        .get("childSessionId")
        .or_else(|| update.get("child_session_id"))
        .and_then(Value::as_str)?
        .to_string();
    let parent_tool_call_id = update
        .get("parentToolCallId")
        .or_else(|| update.get("parent_tool_call_id"))
        .and_then(Value::as_str)?
        .to_string();
    let label = update
        .get("label")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .map(str::to_string);
    let terminal = update_kind == "tool_call_update"
        && matches!(
            update.get("status").and_then(Value::as_str),
            Some("completed" | "failed" | "cancelled" | "canceled" | "stopped" | "done")
        );
    // Keep this line small because it is persisted and rendered in a compact row.
    let latest_activity = tool_summary(update)
        .or_else(|| {
            update
                .get("title")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| label.clone())
        .map(|text| text.chars().take(160).collect());
    Some(AgentConversationPayload::ChildUpdate {
        child_id,
        parent_tool_call_id,
        label,
        state: if terminal { "finished" } else { "running" }.into(),
        latest_activity,
    })
}

fn task_child_update_payload(
    update: &Value,
    update_kind: &str,
) -> Option<AgentConversationPayload> {
    let tool_call_id = update
        .get("toolCallId")
        .or_else(|| update.get("tool_call_id"))
        .and_then(Value::as_str)?
        .to_string();
    let label = update
        .get("title")
        .and_then(Value::as_str)
        .and_then(|title| title.lines().next())
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_string);
    let summary = tool_summary(update);
    let async_child_id = summary.as_deref().and_then(async_agent_id);
    let is_async = async_child_id.is_some();
    let terminal = !is_async
        && update_kind == "tool_call_update"
        && matches!(
            update.get("status").and_then(Value::as_str),
            Some("completed" | "failed" | "cancelled" | "canceled" | "stopped" | "done")
        );
    let child_id = async_child_id.unwrap_or_else(|| tool_call_id.clone());
    let label = label.or_else(|| is_async.then(|| format!("Background agent {child_id}")));
    let latest_activity = summary
        .or_else(|| label.clone())
        .map(|text| text.chars().take(160).collect());

    Some(AgentConversationPayload::ChildUpdate {
        child_id,
        parent_tool_call_id: tool_call_id,
        label,
        state: if terminal { "finished" } else { "running" }.into(),
        latest_activity,
    })
}

fn async_agent_id(text: &str) -> Option<String> {
    let mut lines = text.lines();
    if lines.next()? != "Async agent launched successfully." {
        return None;
    }
    lines
        .find_map(|line| line.strip_prefix("agentId:"))
        .and_then(|value| value.split_whitespace().next())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn is_replay_session_update(params: &Value) -> bool {
    fn replay_flag(value: &Value) -> bool {
        value
            .get("_meta")
            .and_then(|metadata| metadata.get("replay"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    replay_flag(params)
        || params
            .get("update")
            .or_else(|| params.get("sessionUpdate"))
            .is_some_and(replay_flag)
}

fn tool_item_id(update: &Value, turn_id: &str) -> String {
    update
        .get("toolCallId")
        .or_else(|| update.get("tool_call_id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("tool-{turn_id}"))
}

fn tool_summary(update: &Value) -> Option<String> {
    let summary = ["content", "locations"]
        .into_iter()
        .filter_map(|field| update.get(field).and_then(text_from_value))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    (!summary.is_empty()).then_some(summary)
}

fn text_from_value(value: &Value) -> Option<String> {
    let text = match value {
        Value::String(text) => text.clone(),
        Value::Array(values) => values
            .iter()
            .filter_map(text_from_value)
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(object) => {
            for field in ["text", "output", "path", "uri", "content"] {
                if let Some(text) = object.get(field).and_then(text_from_value) {
                    if !text.is_empty() {
                        return Some(text);
                    }
                }
            }
            String::new()
        }
        _ => String::new(),
    };
    (!text.is_empty()).then_some(text)
}

fn empty_capabilities(provider: AgentConversationProvider) -> AgentCapabilities {
    AgentCapabilities {
        revision: 1,
        provider,
        implementation: AgentImplementation {
            name: "not-initialized".into(),
            version: "0".into(),
        },
        session: AgentSessionCapabilities {
            multi_session: false,
            list: false,
            load: false,
            resume: false,
            close: false,
            steering: false,
            fork: false,
        },
        prompt: AgentPromptCapabilities {
            text: true,
            image: false,
            embedded_context: false,
            resource_links: false,
        },
        interaction: AgentInteractionCapabilities {
            permissions: false,
            structured_user_input: false,
            tool_terminals: false,
            plans: false,
            tasks: false,
            subagents: false,
        },
        config_options: Vec::new(),
        commands: Vec::new(),
    }
}

fn provider_is_multi_session_safe(capabilities: &AgentCapabilities) -> bool {
    capabilities.session.multi_session
}

fn provider_id(provider: AgentConversationProvider) -> &'static str {
    match provider {
        AgentConversationProvider::Codex => "codex",
        AgentConversationProvider::Claude => "claude",
    }
}
fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn session_can_suspend(session: &ManagedAgentSession) -> bool {
    session.runtime.is_some()
        && !session.suspending
        && session.native_session_id.is_some()
        && session.capabilities.session.resume
        && session_is_quiescent(session)
}

fn session_is_quiescent(session: &ManagedAgentSession) -> bool {
    session.state == AgentRuntimeState::Ready
        && session.active_turn_id.is_none()
        && !session.prompt_once_active
        && session.permission_requests.is_empty()
        && session.user_input_requests.is_empty()
        && session.writer_lease_transition.is_none()
        && session.live_tool_calls.is_empty()
        && session.background_work.is_empty()
}
fn required_id(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label} is required"))
    } else {
        Ok(value.to_string())
    }
}
fn normalized_optional_id(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn normalized_session_start_effort(
    provider: AgentConversationProvider,
    value: Option<String>,
) -> Result<Option<String>, String> {
    let value = normalized_optional_id(value);
    let Some(value) = value else {
        return Ok(None);
    };
    if provider != AgentConversationProvider::Claude
        || CLAUDE_SESSION_EFFORTS
            .iter()
            .any(|(effort, _)| *effort == value)
    {
        Ok(Some(value))
    } else {
        Err("Claude session effort must be low, medium, high, or max".to_string())
    }
}

fn session_spawn_environment(
    provider: AgentConversationProvider,
    reasoning_effort: Option<&str>,
) -> SidecarEnvironment {
    if provider != AgentConversationProvider::Claude {
        return SidecarEnvironment::default();
    }
    let mut environment = SidecarEnvironment::default().remove(MAX_THINKING_TOKENS_ENV);
    if let Some((_, tokens)) = CLAUDE_SESSION_EFFORTS
        .iter()
        .find(|(effort, _)| Some(*effort) == reasoning_effort)
    {
        environment = environment.set(MAX_THINKING_TOKENS_ENV, *tokens);
    }
    environment
}

fn claude_session_config(
    mut config: AgentConversationConfigState,
    reasoning_effort: Option<&str>,
) -> AgentConversationConfigState {
    config.reasoning_effort = reasoning_effort.map(str::to_string);
    config.available_efforts = CLAUDE_SESSION_EFFORTS
        .iter()
        .map(|(effort, _)| (*effort).to_string())
        .collect();
    config
}

fn validate_conversation_config_update(
    current: &AgentConversationConfigState,
    update: &AgentConversationConfigUpdate,
) -> Result<(), String> {
    for (value, available, label) in [
        (&update.model, &current.available_models, "model"),
        (
            &update.reasoning_effort,
            &current.available_efforts,
            "reasoning effort",
        ),
        (
            &update.approval_policy,
            &current.available_approval_policies,
            "approval policy",
        ),
    ] {
        if value
            .as_ref()
            .is_some_and(|value| !available.contains(value))
        {
            return Err(format!(
                "The requested {label} is not available for this session"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::time::{Duration, Instant};

    use crate::agent_conversation::handoff::{
        AgentConversationHistoryBoundary, AgentConversationProcessTreeAssertion,
    };

    fn request(
        root: &str,
        owned_id: &str,
        provider: AgentConversationProvider,
    ) -> EnsureAgentConversationRequest {
        EnsureAgentConversationRequest {
            owned_id: owned_id.into(),
            provider,
            cwd: root.into(),
            native_session_id: None,
            native_session_mode: AgentNativeSessionMode::Resume,
            reasoning_effort: None,
        }
    }
    fn temp_root() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("mcb-runtime-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn child_rollout_scan_emits_once_until_metadata_changes() {
        let root = temp_root();
        let parent_directory = root.join("2026/08/13");
        let today_directory = root.join(chrono::Local::now().format("%Y/%m/%d").to_string());
        fs::create_dir_all(&parent_directory).unwrap();
        fs::create_dir_all(&today_directory).unwrap();
        let parent = parent_directory.join("rollout-parent.jsonl");
        fs::write(
            &parent,
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"parent\"}}\n",
        )
        .unwrap();
        fs::write(
            today_directory.join("rollout-child.jsonl"),
            concat!(
                "{\"type\":\"session_meta\",\"payload\":{",
                "\"id\":\"child-1\",\"parent_thread_id\":\"parent\",",
                "\"thread_source\":\"subagent\",",
                "\"source\":{\"subagent\":{\"thread_spawn\":{",
                "\"agent_path\":\"/root/probe\",\"agent_role\":\"explore\"}}}}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"text\":\"ignored\"}}\n"
            ),
        )
        .unwrap();
        let active_after = UNIX_EPOCH;
        let children =
            transcript::scan_codex_child_rollouts(&parent, "parent", active_after).unwrap();
        let mut known = HashMap::new();

        assert_eq!(
            changed_child_updates("parent", &mut known, children.clone()),
            vec![AgentConversationPayload::ChildUpdate {
                child_id: "child-1".into(),
                parent_tool_call_id: "parent".into(),
                label: Some("probe (explore)".into()),
                state: "running".into(),
                latest_activity: Some("Running".into()),
            }]
        );
        assert!(changed_child_updates("parent", &mut known, children).is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn child_rollout_scan_outlives_turn_until_running_children_finish() {
        let running = CodexChildRollout {
            child_id: "child-1".into(),
            label: "probe (explore)".into(),
            state: "running".into(),
            latest_activity: "Running".into(),
        };
        let finished = CodexChildRollout {
            state: "finished".into(),
            latest_activity: "Finished".into(),
            ..running.clone()
        };
        let mut known = HashMap::new();

        let (updates, _, keep_scanning) =
            child_rollout_scan_updates("parent", &mut known, vec![running], true);
        assert!(keep_scanning);
        assert_eq!(
            updates,
            vec![AgentConversationPayload::ChildUpdate {
                child_id: "child-1".into(),
                parent_tool_call_id: "parent".into(),
                label: Some("probe (explore)".into()),
                state: "running".into(),
                latest_activity: Some("Running".into()),
            }]
        );

        let (updates, _, keep_scanning) =
            child_rollout_scan_updates("parent", &mut known, vec![finished], false);
        assert!(!keep_scanning);
        assert_eq!(
            updates,
            vec![AgentConversationPayload::ChildUpdate {
                child_id: "child-1".into(),
                parent_tool_call_id: "parent".into(),
                label: Some("probe (explore)".into()),
                state: "finished".into(),
                latest_activity: Some("Finished".into()),
            }]
        );
    }

    struct FixtureManager {
        manager: AgentRuntimeManager,
        owned_id: String,
        generation: u64,
        root: std::path::PathBuf,
    }

    async fn fixture_manager_with_acp_session(fixture: &str) -> FixtureManager {
        fixture_manager_with_provider(fixture, AgentConversationProvider::Codex, None).await
    }

    async fn fixture_manager_with_provider(
        fixture: &str,
        provider: AgentConversationProvider,
        reasoning_effort: Option<&str>,
    ) -> FixtureManager {
        let root = temp_root();
        let log = root.join(format!("{fixture}.jsonl"));
        let manifest =
            super::super::providers::acp_client::tests::fixture_manifest_named(&log, fixture);
        let providers = ProviderRegistry::new([(provider, manifest)]).expect("fixture provider");
        let manager = AgentRuntimeManager::new(providers);
        let owned_id = format!("owned-{fixture}");
        let mut ensure_request = request(root.to_str().unwrap(), &owned_id, provider);
        ensure_request.reasoning_effort = reasoning_effort.map(str::to_string);
        let connection = manager.ensure_inner(ensure_request).expect("ensure").0;
        manager
            .activate(&owned_id, connection.generation)
            .await
            .expect("activate");
        FixtureManager {
            manager,
            owned_id,
            generation: connection.generation,
            root,
        }
    }

    fn test_prompt(text: &str) -> AgentPrompt {
        AgentPrompt {
            text: text.to_string(),
            images: Vec::new(),
        }
    }

    async fn wait_until(mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !condition() {
            assert!(Instant::now() < deadline, "condition timed out");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    fn payload_kind(payload: &AgentConversationPayload) -> &'static str {
        match payload {
            AgentConversationPayload::Connection { .. } => "connection",
            AgentConversationPayload::UserMessage { .. } => "userMessage",
            AgentConversationPayload::AssistantDelta { .. } => "assistantDelta",
            AgentConversationPayload::AssistantMessage { .. } => "assistantMessage",
            AgentConversationPayload::Tool { .. } => "tool",
            AgentConversationPayload::ChildUpdate { .. } => "childUpdate",
            AgentConversationPayload::Approval { .. } => "approval",
            AgentConversationPayload::UserInputRequested { .. } => "userInputRequested",
            AgentConversationPayload::UserInputResolved { .. } => "userInputResolved",
            AgentConversationPayload::Plan { .. } => "plan",
            AgentConversationPayload::Turn { .. } => "turn",
            AgentConversationPayload::AvailableCommandsUpdate { .. } => "availableCommandsUpdate",
            AgentConversationPayload::Usage { .. } => "usage",
            AgentConversationPayload::TerminalProjection(_) => "terminalProjection",
            AgentConversationPayload::Error { .. } => "error",
        }
    }

    fn terminal_handoff(
        owned_id: &str,
        generation: u64,
        native_session_id: String,
    ) -> AgentConversationHandoffRequest {
        AgentConversationHandoffRequest {
            owned_id: owned_id.to_string(),
            generation,
            direction: AgentConversationHandoffDirection::StructuredToTerminal,
            mode: AgentConversationHandoffMode::SameSession,
            phase: AgentConversationHandoffPhase::Commit,
            expected_owner: Some(AgentWriterLeaseOwner::Structured),
            target_owned_id: None,
            native_session_id: Some(native_session_id.clone()),
            pty_session_id: Some("fixture-pty".to_string()),
            history_boundary: AgentConversationHistoryBoundary {
                native_session_id: Some(native_session_id),
                first_sequence: 0,
                last_sequence: 0,
                reconciled_sequence: Some(0),
            },
            process_tree: AgentConversationProcessTreeAssertion {
                checked: true,
                tui_live: true,
                tui_released: false,
                writer_count: 1,
                user_pty_count: 1,
                sidecar_count: 1,
                pty_session_id: Some("fixture-pty".to_string()),
            },
        }
    }

    fn process_is_alive(pid: u32) -> bool {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    #[test]
    fn session_updates_map_to_conversation_payloads() {
        let chunk = json!({ "sessionId": "s", "update": {
            "sessionUpdate": "agent_message_chunk",
            "content": { "type": "text", "text": "Hi" }, "messageId": "m1" } });
        assert_eq!(
            payload_from_session_update_for_turn(&chunk, None),
            Some(AgentConversationPayload::AssistantDelta {
                item_id: "m1".into(),
                delta: "Hi".into(),
            })
        );
        let replayed_chunk = json!({ "sessionId": "s", "update": {
            "sessionUpdate": "agent_message_chunk",
            "content": { "type": "text", "text": "Restored" },
            "messageId": "m-replay",
            "_meta": { "replay": true }
        } });
        assert_eq!(
            payload_from_session_update_for_turn(&replayed_chunk, None),
            Some(AgentConversationPayload::AssistantMessage {
                item_id: "m-replay".into(),
                text: "Restored".into(),
                completed: true,
            })
        );
        let tool = json!({ "sessionId": "s", "update": {
            "sessionUpdate": "tool_call", "toolCallId": "t1", "title": "Read file",
            "status": "in_progress" } });
        match payload_from_session_update_for_turn(&tool, None) {
            Some(AgentConversationPayload::Tool {
                item_id,
                name,
                state,
                ..
            }) => {
                assert_eq!(item_id, "t1");
                assert_eq!(name, "Read file");
                assert_eq!(state, super::super::protocol::ToolState::Started);
            }
            other => panic!("expected Tool, got {other:?}"),
        }
        let child_started = json!({ "sessionId": "s", "update": {
            "sessionUpdate": "tool_call", "toolCallId": "child-tool-1", "kind": "subagent",
            "childSessionId": "child-session-1", "parentToolCallId": "parent-tool-1",
            "label": "Review the change" } });
        assert_eq!(
            payload_from_session_update_for_turn(&child_started, None),
            Some(AgentConversationPayload::ChildUpdate {
                child_id: "child-session-1".into(),
                parent_tool_call_id: "parent-tool-1".into(),
                label: Some("Review the change".into()),
                state: "running".into(),
                latest_activity: Some("Review the change".into()),
            })
        );
        let child_finished = json!({ "sessionId": "s", "update": {
            "sessionUpdate": "tool_call_update", "toolCallId": "child-tool-1", "kind": "subagent",
            "status": "completed", "childSessionId": "child-session-1",
            "parentToolCallId": "parent-tool-1",
            "content": [{ "type": "text", "text": "Review complete" }] } });
        assert_eq!(
            payload_from_session_update_for_turn(&child_finished, None),
            Some(AgentConversationPayload::ChildUpdate {
                child_id: "child-session-1".into(),
                parent_tool_call_id: "parent-tool-1".into(),
                label: None,
                state: "finished".into(),
                latest_activity: Some("Review complete".into()),
            })
        );
        let plan = json!({ "sessionId": "s", "update": {
            "sessionUpdate": "plan", "entries": [
                { "content": "step one", "status": "pending" }
            ] } });
        match payload_from_session_update_for_turn(&plan, None) {
            Some(AgentConversationPayload::Plan { items }) => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].text, "step one");
            }
            other => panic!("expected Plan, got {other:?}"),
        }

        let user = json!({ "update": {
            "sessionUpdate": "user_message_chunk",
            "content": { "type": "text", "text": "Question" },
            "turnId": "turn-2"
        } });
        assert_eq!(
            payload_from_session_update_for_turn(&user, None),
            Some(AgentConversationPayload::UserMessage {
                item_id: "user-turn-2".into(),
                text: "Question".into(),
                completed: false,
            })
        );
        let replayed_user = json!({ "update": {
            "sessionUpdate": "user_message_chunk",
            "content": { "type": "text", "text": "Restored question" },
            "messageId": "user-replay",
            "turnId": "turn-2",
            "_meta": { "replay": true }
        } });
        assert_eq!(
            payload_from_session_update_for_turn(&replayed_user, None),
            Some(AgentConversationPayload::UserMessage {
                item_id: "user-replay".into(),
                text: "Restored question".into(),
                completed: true,
            })
        );

        for (status, expected) in [
            ("in_progress", ToolState::Updated),
            ("completed", ToolState::Completed),
            ("failed", ToolState::Failed),
        ] {
            let update = json!({ "update": {
                "sessionUpdate": "tool_call_update",
                "toolCallId": "t1",
                "title": "Read file",
                "status": status,
                "content": [{ "type": "text", "text": "detail" }],
                "locations": [{ "path": "src/main.rs" }]
            } });
            match payload_from_session_update_for_turn(&update, None) {
                Some(AgentConversationPayload::Tool { state, summary, .. }) => {
                    assert_eq!(state, expected);
                    assert_eq!(summary.as_deref(), Some("detail\nsrc/main.rs"));
                }
                other => panic!("expected Tool update, got {other:?}"),
            }
        }

        assert_eq!(
            payload_from_session_update_for_turn(
                &json!({ "update": {
                "sessionUpdate": "agent_thought_chunk",
                "content": { "type": "text", "text": "private" }
            } }),
                None
            ),
            None
        );
        let commands = json!({ "update": {
            "sessionUpdate": "available_commands_update",
            "availableCommands": [
                { "name": "review", "description": "Review the change", "input": { "hint": "path" } },
                { "id": "status", "label": "Status" }
            ]
        }});
        assert_eq!(
            payload_from_session_update_for_turn(&commands, None),
            Some(AgentConversationPayload::AvailableCommandsUpdate {
                available_commands: vec![
                    AgentCommandDescriptor {
                        id: "review".into(),
                        label: "review".into(),
                        description: Some("Review the change".into()),
                        input_hint: Some("path".into()),
                        provider_metadata: None,
                    },
                    AgentCommandDescriptor {
                        id: "status".into(),
                        label: "Status".into(),
                        description: None,
                        input_hint: None,
                        provider_metadata: None,
                    }
                ]
            })
        );
        let usage = json!({ "update": {
            "sessionUpdate": "usage_update", "used": 120, "size": 4096,
            "inputTokens": 100, "outputTokens": 20
        }});
        assert_eq!(
            payload_from_session_update_for_turn(&usage, None),
            Some(AgentConversationPayload::Usage {
                input_tokens: Some(100),
                output_tokens: Some(20),
                used_tokens: Some(120),
                context_window: Some(4096),
            })
        );
        assert_eq!(
            payload_from_session_update_for_turn(
                &json!({ "update": {
                "sessionUpdate": "future_update"
            } }),
                None
            ),
            None
        );
    }

    #[test]
    fn task_tool_call_maps_to_running_child() {
        let task = json!({ "sessionId": "s", "update": {
            "sessionUpdate": "tool_call",
            "toolCallId": "task-tool-1",
            "title": "Review the change",
            "kind": "think",
            "status": "pending",
            "content": [{ "type": "content", "content": {
                "type": "text", "text": "Inspect the implementation"
            }}],
            "rawInput": { "description": "Review the change", "subagent_type": "Explore" },
            "_meta": { "claudeCode": { "toolName": "Task" } }
        }});

        assert_eq!(
            payload_from_session_update_for_turn(&task, None),
            Some(AgentConversationPayload::ChildUpdate {
                child_id: "task-tool-1".into(),
                parent_tool_call_id: "task-tool-1".into(),
                label: Some("Review the change".into()),
                state: "running".into(),
                latest_activity: Some("Inspect the implementation".into()),
            })
        );
    }

    #[test]
    fn task_terminal_update_maps_to_finished_child() {
        for (status, activity) in [
            ("completed", "Review complete"),
            ("failed", "Review failed"),
        ] {
            let task = json!({ "sessionId": "s", "update": {
                "sessionUpdate": "tool_call_update",
                "toolCallId": "task-tool-1",
                "status": status,
                "content": [{ "type": "content", "content": {
                    "type": "text", "text": activity
                }}],
                "_meta": { "claudeCode": { "toolName": "Task" } }
            }});

            assert_eq!(
                payload_from_session_update_for_turn(&task, None),
                Some(AgentConversationPayload::ChildUpdate {
                    child_id: "task-tool-1".into(),
                    parent_tool_call_id: "task-tool-1".into(),
                    label: None,
                    state: "finished".into(),
                    latest_activity: Some(activity.into()),
                })
            );
        }
    }

    #[test]
    fn async_task_launch_maps_agent_id_to_running_child() {
        let task = json!({ "sessionId": "s", "update": {
            "sessionUpdate": "tool_call_update",
            "toolCallId": "task-tool-2",
            "title": "Investigate startup",
            "status": "completed",
            "content": [{ "type": "content", "content": {
                "type": "text",
                "text": "Async agent launched successfully.\nagentId: ad9a1e2 (internal ID 123)\noutput_file: /private/tmp/task.output"
            }}],
            "_meta": { "claudeCode": { "toolName": "Task" } }
        }});

        assert_eq!(
            payload_from_session_update_for_turn(&task, None),
            Some(AgentConversationPayload::ChildUpdate {
                child_id: "ad9a1e2".into(),
                parent_tool_call_id: "task-tool-2".into(),
                label: Some("Investigate startup".into()),
                state: "running".into(),
                latest_activity: Some(
                    "Async agent launched successfully.\nagentId: ad9a1e2 (internal ID 123)\noutput_file: /private/tmp/task.output".into()
                ),
            })
        );
    }

    #[test]
    fn ordinary_think_tool_call_does_not_map_to_child() {
        let thought = json!({ "sessionId": "s", "update": {
            "sessionUpdate": "tool_call",
            "toolCallId": "think-tool-1",
            "title": "Think",
            "kind": "think",
            "status": "pending"
        }});

        assert!(matches!(
            payload_from_session_update_for_turn(&thought, None),
            Some(AgentConversationPayload::Tool { .. })
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn captures_session_commands_and_usage_without_an_active_turn() {
        let fixture = fixture_manager_with_acp_session("command_capture").await;
        wait_until(|| {
            fixture
                .manager
                .capabilities(&fixture.owned_id, fixture.generation)
                .map(|capabilities| {
                    capabilities
                        .commands
                        .iter()
                        .any(|command| command.id == "updated")
                })
                .unwrap_or(false)
        })
        .await;
        let capabilities = fixture
            .manager
            .capabilities(&fixture.owned_id, fixture.generation)
            .expect("capabilities");
        assert_eq!(capabilities.commands[0].id, "updated");
        let snapshot = fixture
            .manager
            .snapshot(&fixture.owned_id)
            .expect("snapshot")
            .expect("active snapshot");
        assert!(snapshot.events.iter().any(|event| matches!(
            event.payload,
            AgentConversationPayload::AvailableCommandsUpdate { .. }
        )));
        assert!(snapshot.events.iter().any(|event| matches!(
            event.payload,
            AgentConversationPayload::Usage {
                used_tokens: Some(120),
                context_window: Some(4096),
                ..
            }
        )));
        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .expect("close");
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn a_prompt_streams_updates_and_lifecycle_through_the_emitter() {
        let fixture = fixture_manager_with_acp_session("prompt_with_update").await;
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        fixture
            .manager
            .set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));

        fixture
            .manager
            .prompt(&fixture.owned_id, fixture.generation, test_prompt("hello"))
            .await
            .expect("prompt");
        wait_until(|| {
            seen.lock().unwrap().iter().any(|event| {
                matches!(
                    event.payload,
                    AgentConversationPayload::Turn {
                        state: super::super::protocol::TurnState::Completed,
                        ..
                    }
                )
            })
        })
        .await;

        {
            let seen = seen.lock().unwrap();
            let kinds = seen
                .iter()
                .map(|event| payload_kind(&event.payload))
                .collect::<Vec<_>>();
            assert!(
                kinds.starts_with(&["turn"]),
                "Turn Started must be first, got {kinds:?}"
            );
            assert!(
                kinds.contains(&"assistantDelta"),
                "streamed delta missing: {kinds:?}"
            );
            assert_eq!(seen[0].sequence, 1, "manager sequences start at 1");
            assert!(seen
                .windows(2)
                .all(|events| events[1].sequence == events[0].sequence + 1));
            let turn_ids = seen
                .iter()
                .filter_map(|event| match &event.payload {
                    AgentConversationPayload::Turn { turn_id, .. } => Some(turn_id.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(turn_ids.len(), 2);
            assert_eq!(
                turn_ids[0], turn_ids[1],
                "app-minted id correlates Started and Completed"
            );
            assert!(turn_ids[0].starts_with("turn-"));
            let assistant_item_id = seen.iter().find_map(|event| match &event.payload {
                AgentConversationPayload::AssistantDelta { item_id, .. } => Some(item_id),
                _ => None,
            });
            assert_eq!(
                assistant_item_id,
                Some(&format!("assistant-{}", turn_ids[0])),
                "fallback item identity must use the app-minted turn id"
            );
        }

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn prompt_after_failed_session_replacement_continues_the_event_sequence() {
        let fixture = fixture_manager_with_acp_session("prompt_with_update").await;

        fixture
            .manager
            .prompt(&fixture.owned_id, fixture.generation, test_prompt("first"))
            .await
            .expect("first prompt");
        wait_until(|| {
            fixture
                .manager
                .sessions
                .lock()
                .unwrap()
                .get(&fixture.owned_id)
                .is_some_and(|session| session.active_turn_id.is_none())
        })
        .await;
        let first_last_sequence = fixture
            .manager
            .snapshot(&fixture.owned_id)
            .unwrap()
            .unwrap()
            .last_sequence;

        {
            let mut sessions = fixture.manager.sessions.lock().unwrap();
            let session = sessions.get_mut(&fixture.owned_id).unwrap();
            session.state = AgentRuntimeState::Failed;
            session.connection.state = ConversationConnectionState::Failed;
        }
        let replacement = fixture
            .manager
            .ensure_async(request(
                fixture.root.to_str().unwrap(),
                &fixture.owned_id,
                AgentConversationProvider::Codex,
            ))
            .await
            .expect("replace failed session");
        fixture
            .manager
            .activate(&fixture.owned_id, replacement.generation)
            .await
            .expect("activate replacement");
        fixture
            .manager
            .prompt(
                &fixture.owned_id,
                replacement.generation,
                test_prompt("second"),
            )
            .await
            .expect("second prompt after replacement");

        let events = fixture.manager.list_events(&fixture.owned_id, 0).unwrap();
        let second_start = events
            .iter()
            .find(|event| event.sequence > first_last_sequence)
            .expect("replacement prompt event");
        assert_eq!(second_start.sequence, first_last_sequence + 1);

        let _ = fixture
            .manager
            .close(&fixture.owned_id, replacement.generation)
            .await;
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn first_prompt_titles_once_and_explicit_metadata_wins() {
        let fixture = fixture_manager_with_acp_session("prompt_with_update").await;

        fixture
            .manager
            .prompt(
                &fixture.owned_id,
                fixture.generation,
                test_prompt("  First prompt title  \nignored second line"),
            )
            .await
            .expect("first prompt");
        wait_until(|| {
            fixture
                .manager
                .list_sessions()
                .expect("list after first prompt")[0]
                .active_turn_id
                .is_none()
        })
        .await;
        assert_eq!(
            fixture.manager.list_sessions().unwrap()[0]
                .meta
                .title
                .as_deref(),
            Some("First prompt title")
        );

        fixture
            .manager
            .prompt(
                &fixture.owned_id,
                fixture.generation,
                test_prompt("A later prompt must not replace the title"),
            )
            .await
            .expect("second prompt");
        wait_until(|| {
            fixture
                .manager
                .list_sessions()
                .expect("list after second prompt")[0]
                .active_turn_id
                .is_none()
        })
        .await;
        assert_eq!(
            fixture.manager.list_sessions().unwrap()[0]
                .meta
                .title
                .as_deref(),
            Some("First prompt title")
        );

        fixture
            .manager
            .update_session_meta(UpdateAgentConversationSessionMetaRequest {
                owned_id: fixture.owned_id.clone(),
                model: None,
                effort: None,
                meta: AgentConversationSessionMeta {
                    title: Some("Chosen title".into()),
                    ..AgentConversationSessionMeta::default()
                },
            })
            .expect("set explicit title");
        fixture
            .manager
            .prompt(
                &fixture.owned_id,
                fixture.generation,
                test_prompt("Another prompt must preserve the chosen title"),
            )
            .await
            .expect("third prompt");
        wait_until(|| {
            fixture
                .manager
                .list_sessions()
                .expect("list after third prompt")[0]
                .active_turn_id
                .is_none()
        })
        .await;
        assert_eq!(
            fixture.manager.list_sessions().unwrap()[0]
                .meta
                .title
                .as_deref(),
            Some("Chosen title")
        );

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[test]
    fn prompt_title_is_char_boundary_safe_and_capped() {
        let long_title = format!("  {} trailing", "é".repeat(64));
        assert_eq!(prompt_title(&long_title), Some("é".repeat(64)));
        assert_eq!(prompt_title("   \nsecond line"), None);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn activating_the_same_generation_reuses_one_runtime_and_pump() {
        let fixture = fixture_manager_with_acp_session("prompt_with_update").await;
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        fixture
            .manager
            .set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));

        fixture
            .manager
            .activate(&fixture.owned_id, fixture.generation)
            .await
            .expect("second activation is idempotent");
        assert_eq!(fixture.manager.resource_roots().len(), 1);
        let log = fs::read_to_string(fixture.root.join("prompt_with_update.jsonl"))
            .expect("activation fixture log");
        assert_eq!(log.matches(r#""method":"initialize""#).count(), 1);
        assert_eq!(log.matches(r#""method":"session/new""#).count(), 1);
        assert!(
            seen.lock().unwrap().is_empty(),
            "idempotent activation emits no event"
        );

        fixture
            .manager
            .prompt(&fixture.owned_id, fixture.generation, test_prompt("hello"))
            .await
            .expect("prompt starts");
        wait_until(|| {
            seen.lock().unwrap().iter().any(|event| {
                matches!(
                    event.payload,
                    AgentConversationPayload::Turn {
                        state: super::super::protocol::TurnState::Completed,
                        ..
                    }
                )
            })
        })
        .await;
        assert_eq!(
            seen.lock()
                .unwrap()
                .iter()
                .filter(|event| matches!(
                    event.payload,
                    AgentConversationPayload::AssistantDelta { .. }
                ))
                .count(),
            1,
            "one pump must emit one update"
        );

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn store_row_updated_on_every_lifecycle_transition() {
        let root = temp_root();
        let database = root.join("sessions.db");
        let log = root.join("store-lifecycle.jsonl");
        let manifest = super::super::providers::acp_client::tests::fixture_manifest_named(
            &log,
            "suspend_store_lifecycle",
        );
        let providers = ProviderRegistry::new([(AgentConversationProvider::Codex, manifest)])
            .expect("fixture provider");
        let manager = AgentRuntimeManager::open(providers, &database).unwrap();
        let connection = manager
            .ensure_async(request(
                root.to_str().unwrap(),
                "owned-store-lifecycle",
                AgentConversationProvider::Codex,
            ))
            .await
            .unwrap();
        assert_eq!(
            manager
                .store
                .get_session(&connection.owned_id)
                .unwrap()
                .unwrap()
                .state,
            "starting"
        );
        manager
            .activate(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        assert_eq!(
            manager
                .store
                .get_session(&connection.owned_id)
                .unwrap()
                .unwrap()
                .state,
            "ready"
        );
        manager
            .suspend_if_quiescent(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        let suspended = manager
            .store
            .get_session(&connection.owned_id)
            .unwrap()
            .unwrap();
        assert_eq!(suspended.state, "suspended");
        assert!(suspended.suspended);

        manager
            .activate(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        assert_eq!(
            manager
                .store
                .get_session(&connection.owned_id)
                .unwrap()
                .unwrap()
                .state,
            "ready"
        );
        manager
            .close(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        assert_eq!(
            manager
                .store
                .get_session(&connection.owned_id)
                .unwrap()
                .unwrap()
                .state,
            "closed"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lifecycle_transition_updates_all_fields_and_rejects_terminal_revival() {
        let root = temp_root();
        let manager = AgentRuntimeManager::new(ProviderRegistry::default());
        let connection = manager
            .ensure_inner(request(
                root.to_str().unwrap(),
                "owned-lifecycle-transition",
                AgentConversationProvider::Codex,
            ))
            .unwrap()
            .0;
        let mut sessions = manager.sessions.lock().unwrap();
        let session = sessions.get_mut(&connection.owned_id).unwrap();

        session
            .transition_lifecycle(
                AgentRuntimeState::Ready,
                ConversationConnectionState::Connected,
                AgentExecutionOwner::Structured,
                AgentWriterLeaseOwner::Structured,
            )
            .unwrap();
        assert_eq!(session.state, AgentRuntimeState::Ready);
        assert_eq!(
            session.connection.state,
            ConversationConnectionState::Connected
        );
        assert_eq!(session.owner, AgentExecutionOwner::Structured);
        assert_eq!(
            session.writer_lease.owner,
            AgentWriterLeaseOwner::Structured
        );

        session
            .transition_lifecycle(
                AgentRuntimeState::Closed,
                ConversationConnectionState::Closed,
                AgentExecutionOwner::Stopped,
                AgentWriterLeaseOwner::None,
            )
            .unwrap();
        let error = session
            .transition_lifecycle(
                AgentRuntimeState::Ready,
                ConversationConnectionState::Connected,
                AgentExecutionOwner::Structured,
                AgentWriterLeaseOwner::Structured,
            )
            .unwrap_err();
        assert_eq!(error, "Closed conversation sessions cannot be revived");
        assert_eq!(session.state, AgentRuntimeState::Closed);
        assert_eq!(
            session.connection.state,
            ConversationConnectionState::Closed
        );
        assert_eq!(session.owner, AgentExecutionOwner::Stopped);
        assert_eq!(session.writer_lease.owner, AgentWriterLeaseOwner::None);
        drop(sessions);
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn stale_generation_close_is_refused_without_changing_current_session() {
        let root = temp_root();
        let manager = AgentRuntimeManager::new(ProviderRegistry::default());
        let connection = manager
            .ensure_inner(request(
                root.to_str().unwrap(),
                "owned-stale-close",
                AgentConversationProvider::Codex,
            ))
            .unwrap()
            .0;

        let error = manager
            .close(&connection.owned_id, connection.generation + 1)
            .await
            .unwrap_err();
        assert_eq!(
            error,
            "Conversation connection changed; retry on the current session"
        );
        {
            let sessions = manager.sessions.lock().unwrap();
            let session = sessions.get(&connection.owned_id).unwrap();
            assert_eq!(session.generation, connection.generation);
            assert_eq!(session.state, AgentRuntimeState::Starting);
            assert_eq!(
                session.connection.state,
                ConversationConnectionState::Connecting
            );
            assert_eq!(session.owner, AgentExecutionOwner::Stopped);
            assert_eq!(session.writer_lease.owner, AgentWriterLeaseOwner::None);
        }

        manager
            .close(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn close_during_activation_cannot_resurrect_the_session() {
        let root = temp_root();
        let log = root.join("blocked-activation.jsonl");
        let mut manifest =
            super::super::providers::acp_client::tests::fixture_manifest_named(&log, "default");
        let script = manifest.args.get_mut(1).expect("fixture script");
        *script = script.replace(
            "*'\"method\":\"session/new\"'*)",
            "*'\"method\":\"session/new\"'*)\n      sleep 0.15",
        );
        let providers = ProviderRegistry::new([(AgentConversationProvider::Codex, manifest)])
            .expect("fixture provider");
        let manager = AgentRuntimeManager::new(providers);
        let connection = manager
            .ensure_inner(request(
                root.to_str().unwrap(),
                "owned-blocked-activation",
                AgentConversationProvider::Codex,
            ))
            .expect("ensure")
            .0;

        let activating_manager = manager.clone();
        let activating_owned_id = connection.owned_id.clone();
        let activation = tokio::spawn(async move {
            activating_manager
                .activate(&activating_owned_id, connection.generation)
                .await
        });
        wait_until(|| {
            fs::read_to_string(&log)
                .is_ok_and(|requests| requests.contains(r#""method":"session/new""#))
        })
        .await;

        manager
            .close("owned-blocked-activation", connection.generation)
            .await
            .unwrap();
        activation.await.unwrap().unwrap();

        let sessions = manager.sessions.lock().unwrap();
        let session = sessions.get("owned-blocked-activation").unwrap();
        assert_eq!(session.state, AgentRuntimeState::Closed);
        assert_eq!(
            session.connection.state,
            ConversationConnectionState::Closed
        );
        assert!(session.runtime.is_none());
        drop(sessions);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn events_persisted_and_capped() {
        let root = temp_root();
        let database = root.join("sessions.db");
        let manager = AgentRuntimeManager::open(ProviderRegistry::default(), &database).unwrap();
        let connection = manager
            .ensure_inner(request(
                root.to_str().unwrap(),
                "owned-event-cap",
                AgentConversationProvider::Codex,
            ))
            .unwrap()
            .0;
        {
            let mut sessions = manager.sessions.lock().unwrap();
            let session = sessions.get_mut(&connection.owned_id).unwrap();
            for index in 0..=STORE_EVENT_CAP {
                record_payload_for_session(
                    session,
                    AgentConversationPayload::Error {
                        code: format!("event-{index}"),
                        message: "fixture".into(),
                        recoverable: true,
                    },
                )
                .unwrap();
            }
        }
        let events = manager
            .store
            .list_events(&connection.owned_id, 0, STORE_EVENT_CAP)
            .unwrap();
        assert_eq!(events.len(), STORE_EVENT_CAP as usize);
        assert_eq!(events.first().unwrap().seq, 2);
        assert_eq!(events.last().unwrap().seq, i64::from(STORE_EVENT_CAP) + 1);
        let snapshot = manager.snapshot(&connection.owned_id).unwrap().unwrap();
        assert_eq!(snapshot.events.len(), SNAPSHOT_EVENT_CAP);
        assert_eq!(
            snapshot.events.first().unwrap().sequence,
            u64::from(STORE_EVENT_CAP) + 2 - SNAPSHOT_EVENT_CAP as u64
        );
        assert_eq!(
            snapshot.events.last().unwrap().sequence,
            u64::from(STORE_EVENT_CAP) + 1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn restart_recovers_sessions_from_store() {
        let root = temp_root();
        let database = root.join("sessions.db");
        let log = root.join("restart.jsonl");
        let manifest = super::super::providers::acp_client::tests::fixture_manifest_named(
            &log,
            "suspend_restart",
        );
        let providers =
            ProviderRegistry::new([(AgentConversationProvider::Codex, manifest.clone())]).unwrap();
        let manager = AgentRuntimeManager::open(providers, &database).unwrap();
        let connection = manager
            .ensure_async(request(
                root.to_str().unwrap(),
                "owned-restart",
                AgentConversationProvider::Codex,
            ))
            .await
            .unwrap();
        manager
            .activate(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        drop(manager);

        let providers =
            ProviderRegistry::new([(AgentConversationProvider::Codex, manifest)]).unwrap();
        let recovered = AgentRuntimeManager::open(providers, &database).unwrap();
        let sessions = recovered.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(sessions[0].suspended);
        recovered
            .activate(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        assert!(
            !recovered
                .snapshot(&connection.owned_id)
                .unwrap()
                .unwrap()
                .suspended
        );
        recovered
            .close(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn startup_recovery_backfills_untitled_session_from_first_user_message() {
        let root = temp_root();
        let database = root.join("sessions.db");
        let manager = AgentRuntimeManager::open(ProviderRegistry::default(), &database).unwrap();
        let connection = manager
            .ensure_inner(request(
                root.to_str().unwrap(),
                "owned-title-backfill",
                AgentConversationProvider::Codex,
            ))
            .unwrap()
            .0;
        {
            let mut sessions = manager.sessions.lock().unwrap();
            let session = sessions.get_mut(&connection.owned_id).unwrap();
            record_payload_for_session(
                session,
                AgentConversationPayload::AssistantMessage {
                    item_id: "earlier-assistant".into(),
                    text: "not a title".into(),
                    completed: true,
                },
            )
            .unwrap();
            record_payload_for_session(
                session,
                AgentConversationPayload::UserMessage {
                    item_id: "first-user".into(),
                    text: "  Recovered title  \nignored second line".into(),
                    completed: true,
                },
            )
            .unwrap();
            record_payload_for_session(
                session,
                AgentConversationPayload::UserMessage {
                    item_id: "second-user".into(),
                    text: "Later title".into(),
                    completed: true,
                },
            )
            .unwrap();
        }
        drop(manager);

        let recovered = AgentRuntimeManager::open(ProviderRegistry::default(), &database).unwrap();
        assert_eq!(
            recovered.list_sessions().unwrap()[0].meta.title.as_deref(),
            Some("Recovered title")
        );
        assert_eq!(
            recovered
                .store
                .get_session(&connection.owned_id)
                .unwrap()
                .unwrap()
                .title
                .as_deref(),
            Some("Recovered title")
        );
        drop(recovered);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn session_draft_survives_manager_restart_and_clears() {
        let root = temp_root();
        let database = root.join("sessions.db");
        let manager = AgentRuntimeManager::open(ProviderRegistry::default(), &database).unwrap();
        let connection = manager
            .ensure_inner(request(
                root.to_str().unwrap(),
                "owned-draft-restart",
                AgentConversationProvider::Codex,
            ))
            .unwrap()
            .0;
        manager
            .set_session_draft(&connection.owned_id, "unfinished message")
            .unwrap();
        drop(manager);

        let recovered = AgentRuntimeManager::open(ProviderRegistry::default(), &database).unwrap();
        assert_eq!(
            recovered.get_session_draft(&connection.owned_id).unwrap(),
            Some("unfinished message".to_owned())
        );
        recovered.clear_session_draft(&connection.owned_id).unwrap();
        assert_eq!(
            recovered.get_session_draft(&connection.owned_id).unwrap(),
            None
        );
        drop(recovered);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn selecting_and_reading_session_never_spawns_runtime() {
        let root = temp_root();
        let log = root.join("read-only-session.jsonl");
        let manifest = super::super::providers::acp_client::tests::fixture_manifest_named(
            &log,
            "prompt_with_update",
        );
        let providers = ProviderRegistry::new([(AgentConversationProvider::Codex, manifest)])
            .expect("fixture provider");
        let manager = AgentRuntimeManager::new(providers);
        let (connection, _) = manager
            .ensure_inner(request(
                root.to_str().unwrap(),
                "owned-read-only",
                AgentConversationProvider::Codex,
            ))
            .unwrap();
        {
            let mut sessions = manager.sessions.lock().unwrap();
            let session = sessions.get_mut(&connection.owned_id).unwrap();
            record_payload_for_session(
                session,
                AgentConversationPayload::UserMessage {
                    item_id: "message-read-only".into(),
                    text: "stored".into(),
                    completed: true,
                },
            )
            .unwrap();
        }
        manager
            .update_session_meta(UpdateAgentConversationSessionMetaRequest {
                owned_id: connection.owned_id.clone(),
                model: Some("model-a".into()),
                effort: Some("medium".into()),
                meta: AgentConversationSessionMeta {
                    worktree: Some(root.display().to_string()),
                    branch: Some("lane/read-only".into()),
                    title: Some("Read only".into()),
                    project: Some("Project".into()),
                    pty_session_id: Some("pty-1".into()),
                    origin: Some("app".into()),
                    source: Some("fresh".into()),
                    ..AgentConversationSessionMeta::default()
                },
            })
            .unwrap();

        let listed = manager.list_sessions().unwrap();
        let snapshot = manager.snapshot(&connection.owned_id).unwrap().unwrap();
        let events = manager.list_events(&connection.owned_id, 0).unwrap();

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].meta.title.as_deref(), Some("Read only"));
        assert_eq!(listed[0].meta.pty_session_id.as_deref(), Some("pty-1"));
        assert_eq!(snapshot.events, events);
        assert_eq!(events.len(), 1);
        assert!(manager.resource_roots().is_empty());
        assert!(
            !log.exists(),
            "read commands must not create a transport process"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn session_annotations_survive_manager_restart() {
        let root = temp_root();
        let database = root.join("sessions.db");
        let owned_id = "owned-annotation-restart";
        let manager = AgentRuntimeManager::open(ProviderRegistry::default(), &database).unwrap();
        manager
            .ensure_inner(request(
                root.to_str().unwrap(),
                owned_id,
                AgentConversationProvider::Codex,
            ))
            .unwrap();
        let saved = manager
            .add_session_annotation(
                owned_id,
                "https://example.test/restart",
                r#"{"x":10,"y":20,"width":100,"height":50}"#,
                "Keep this note",
            )
            .unwrap();
        drop(manager);

        let restarted = AgentRuntimeManager::open(ProviderRegistry::default(), &database).unwrap();
        let annotations = restarted.list_session_annotations(owned_id).unwrap();
        assert_eq!(annotations, [saved.clone()]);

        restarted.delete_session_annotation(saved.id).unwrap();
        assert!(restarted
            .list_session_annotations(owned_id)
            .unwrap()
            .is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn turn_completion_suspends_and_stops_the_runtime() {
        let fixture = fixture_manager_with_acp_session("suspend_turn_completion").await;
        let pid = fixture.manager.resource_roots()[0].pid;
        let native_session_id = fixture
            .manager
            .snapshot(&fixture.owned_id)
            .unwrap()
            .unwrap()
            .connection
            .native_session_id
            .unwrap();

        fixture
            .manager
            .prompt(
                &fixture.owned_id,
                fixture.generation,
                test_prompt("finish this turn"),
            )
            .await
            .expect("prompt starts");
        wait_until(|| {
            fixture
                .manager
                .snapshot(&fixture.owned_id)
                .unwrap()
                .is_some_and(|snapshot| snapshot.suspended)
        })
        .await;
        wait_until(|| !process_is_alive(pid)).await;

        let snapshot = fixture
            .manager
            .snapshot(&fixture.owned_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            snapshot.connection.native_session_id.as_deref(),
            Some(native_session_id.as_str())
        );
        assert!(fixture.manager.resource_roots().is_empty());

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn teardown_at_quiescence() {
        let fixture = fixture_manager_with_acp_session("suspend_active").await;

        assert!(fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .unwrap());

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn suspend_skips_running_turn() {
        let fixture = fixture_manager_with_acp_session("suspend_running").await;
        {
            let mut sessions = fixture.manager.sessions.lock().unwrap();
            let session = sessions.get_mut(&fixture.owned_id).unwrap();
            session.active_turn_id = Some("turn-running".into());
            session.state = AgentRuntimeState::Working;
        }

        assert!(!fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .unwrap());
        assert_eq!(fixture.manager.resource_roots().len(), 1);

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn suspend_skips_pending_permission() {
        let fixture = fixture_manager_with_acp_session("suspend_permission").await;
        fixture
            .manager
            .sessions
            .lock()
            .unwrap()
            .get_mut(&fixture.owned_id)
            .unwrap()
            .permission_requests
            .insert(
                "permission-1".into(),
                PendingPermission {
                    wire_id: json!(1),
                    options: Vec::new(),
                    summary: "Pending permission".into(),
                },
            );

        assert!(!fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .unwrap());
        assert_eq!(fixture.manager.resource_roots().len(), 1);

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn suspend_skips_pending_user_input() {
        let fixture = fixture_manager_with_acp_session("suspend_input").await;
        fixture
            .manager
            .sessions
            .lock()
            .unwrap()
            .get_mut(&fixture.owned_id)
            .unwrap()
            .user_input_requests
            .insert("input-1".into(), PendingUserInput { wire_id: json!(1) });

        assert!(!fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .unwrap());
        assert_eq!(fixture.manager.resource_roots().len(), 1);

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn suspend_closes_runtime_but_keeps_record_and_native_id() {
        let fixture = fixture_manager_with_acp_session("suspend_runtime").await;
        let pid = fixture.manager.resource_roots()[0].pid;
        let native_session_id = fixture
            .manager
            .snapshot(&fixture.owned_id)
            .unwrap()
            .unwrap()
            .connection
            .native_session_id
            .unwrap();

        assert!(fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .unwrap());
        wait_until(|| !process_is_alive(pid)).await;
        assert!(fixture.manager.resource_roots().is_empty());
        let snapshot = fixture
            .manager
            .snapshot(&fixture.owned_id)
            .unwrap()
            .unwrap();
        assert!(snapshot.suspended);
        assert_eq!(
            snapshot.connection.native_session_id.as_deref(),
            Some(native_session_id.as_str())
        );
        assert_eq!(
            snapshot.connection.state,
            ConversationConnectionState::Disconnected
        );
        {
            let sessions = fixture.manager.sessions.lock().unwrap();
            let session = sessions.get(&fixture.owned_id).unwrap();
            assert_eq!(session.state, AgentRuntimeState::Suspended);
            assert_eq!(session.owner, AgentExecutionOwner::Stopped);
            assert_eq!(session.writer_lease.owner, AgentWriterLeaseOwner::None);
        }

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn activation_refreshes_and_persists_a_stale_capability_snapshot() {
        let fixture = fixture_manager_with_acp_session("suspend_stale_capabilities").await;
        assert!(fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .unwrap());
        // A session started before the provider gained image prompts keeps that
        // answer in its stored row, so the row is rewritten the same way here.
        {
            let mut sessions = fixture.manager.sessions.lock().unwrap();
            let session = sessions.get_mut(&fixture.owned_id).unwrap();
            session.capabilities.prompt.image = false;
            persist_session(session).unwrap();
        }
        assert!(!stored_capabilities(&fixture).prompt.image);

        let connection = fixture
            .manager
            .ensure_async(request(
                fixture.root.to_str().unwrap(),
                &fixture.owned_id,
                AgentConversationProvider::Codex,
            ))
            .await
            .expect("ensure suspended session");
        fixture
            .manager
            .activate(&fixture.owned_id, connection.generation)
            .await
            .expect("activate suspended session");

        assert!(
            fixture
                .manager
                .capabilities_for_owned_id(&fixture.owned_id)
                .unwrap()
                .prompt
                .image
        );
        assert!(stored_capabilities(&fixture).prompt.image);

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    fn stored_capabilities(fixture: &FixtureManager) -> AgentCapabilities {
        let row = fixture
            .manager
            .store
            .get_session(&fixture.owned_id)
            .unwrap()
            .expect("stored session row");
        serde_json::from_str::<StoredSessionExtra>(&row.extra_json)
            .unwrap()
            .capabilities
    }

    #[tokio::test(flavor = "current_thread")]
    async fn ensure_after_suspend_resumes_with_same_native_session_id() {
        let fixture = fixture_manager_with_acp_session("suspend_resume").await;
        let native_session_id = fixture
            .manager
            .snapshot(&fixture.owned_id)
            .unwrap()
            .unwrap()
            .connection
            .native_session_id
            .unwrap();
        fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        let sequence_before_resume = fixture
            .manager
            .snapshot(&fixture.owned_id)
            .unwrap()
            .unwrap()
            .last_sequence;

        let connection = fixture
            .manager
            .ensure_async(request(
                fixture.root.to_str().unwrap(),
                &fixture.owned_id,
                AgentConversationProvider::Codex,
            ))
            .await
            .expect("ensure suspended session");
        let resumed = fixture
            .manager
            .activate(&fixture.owned_id, connection.generation)
            .await
            .expect("resume suspended session");
        assert_eq!(
            resumed.native_session_id.as_deref(),
            Some(native_session_id.as_str())
        );
        assert_eq!(resumed.generation, fixture.generation);
        fixture
            .manager
            .prompt(
                &fixture.owned_id,
                fixture.generation,
                test_prompt("after suspend"),
            )
            .await
            .expect("prompt after resume");
        wait_until(|| {
            fixture
                .manager
                .sessions
                .lock()
                .unwrap()
                .get(&fixture.owned_id)
                .is_some_and(|session| session.active_turn_id.is_none())
        })
        .await;
        let snapshot = fixture
            .manager
            .snapshot(&fixture.owned_id)
            .unwrap()
            .unwrap();
        assert!(snapshot.suspended);
        assert_eq!(
            snapshot.connection.state,
            ConversationConnectionState::Disconnected
        );
        let connection_states = snapshot
            .events
            .iter()
            .filter_map(|event| match &event.payload {
                AgentConversationPayload::Connection { state, .. } => Some(*state),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            connection_states,
            [
                ConversationConnectionState::Disconnected,
                ConversationConnectionState::Connected,
                ConversationConnectionState::Disconnected
            ]
        );
        let resumed_events = fixture
            .manager
            .list_events(&fixture.owned_id, sequence_before_resume)
            .unwrap()
            .into_iter()
            .filter(|event| event.sequence > sequence_before_resume)
            .collect::<Vec<_>>();
        assert_eq!(resumed_events[0].sequence, sequence_before_resume + 1);
        assert!(resumed_events
            .windows(2)
            .all(|events| events[1].sequence == events[0].sequence + 1));
        assert!(fixture.manager.resource_roots().is_empty());
        let log = fs::read_to_string(fixture.root.join("suspend_resume.jsonl")).unwrap();
        assert_eq!(log.matches(r#""method":"session/new""#).count(), 1);
        assert_eq!(log.matches(r#""method":"session/resume""#).count(), 1);
        assert_eq!(log.matches(r#""method":"session/prompt""#).count(), 1);

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn resume_does_not_append_provider_history_to_an_existing_journal() {
        let fixture = fixture_manager_with_acp_session("replay_on_resume").await;
        {
            let mut sessions = fixture
                .manager
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = sessions.get_mut(&fixture.owned_id).unwrap();
            record_payload_for_session(
                session,
                AgentConversationPayload::AssistantMessage {
                    item_id: "live-message".into(),
                    text: "canonical answer".into(),
                    completed: true,
                },
            )
            .unwrap();
        }
        fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();

        let connection = fixture
            .manager
            .ensure_async(request(
                fixture.root.to_str().unwrap(),
                &fixture.owned_id,
                AgentConversationProvider::Codex,
            ))
            .await
            .expect("ensure suspended session");
        fixture
            .manager
            .activate(&fixture.owned_id, connection.generation)
            .await
            .expect("resume suspended session");
        tokio::time::sleep(Duration::from_millis(50)).await;

        let snapshot = fixture
            .manager
            .snapshot(&fixture.owned_id)
            .unwrap()
            .unwrap();
        let item_ids = snapshot
            .events
            .iter()
            .filter_map(|event| match &event.payload {
                AgentConversationPayload::UserMessage { item_id, .. }
                | AgentConversationPayload::AssistantDelta { item_id, .. }
                | AgentConversationPayload::AssistantMessage { item_id, .. }
                | AgentConversationPayload::Tool { item_id, .. } => Some(item_id.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(item_ids, ["live-message"]);

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn suspend_skips_provider_without_resume_capability() {
        let fixture = fixture_manager_with_acp_session("suspend_no_resume").await;
        assert!(
            !fixture
                .manager
                .capabilities(&fixture.owned_id, fixture.generation)
                .unwrap()
                .session
                .resume
        );

        assert!(!fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .unwrap());
        assert_eq!(fixture.manager.resource_roots().len(), 1);

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn quiescence_gates_teardown() {
        let fixture = fixture_manager_with_acp_session("suspend_quiescence").await;
        {
            let mut sessions = fixture.manager.sessions.lock().unwrap();
            let session = sessions.get_mut(&fixture.owned_id).unwrap();
            session.state = AgentRuntimeState::Working;
            session.active_turn_id = Some("turn-live".into());
            session.live_tool_calls.insert("tool-live".into());
        }
        assert!(!fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .unwrap());
        assert_eq!(fixture.manager.resource_roots().len(), 1);

        {
            let mut sessions = fixture.manager.sessions.lock().unwrap();
            let session = sessions.get_mut(&fixture.owned_id).unwrap();
            session.state = AgentRuntimeState::Ready;
            session.active_turn_id = None;
            session.prompt_once_active = false;
            session.permission_requests.clear();
            session.user_input_requests.clear();
            session.writer_lease_transition = None;
            session.live_tool_calls.clear();
            session.background_work.clear();
        }
        assert!(fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .unwrap());
        assert!(fixture.manager.resource_roots().is_empty());

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn pool_shares_one_process_across_two_sessions() {
        let root = temp_root();
        let log = root.join("pool-multiplex.jsonl");
        let manifest =
            super::super::providers::acp_client::tests::fixture_manifest_named(&log, "multiplex");
        let providers = ProviderRegistry::new([(AgentConversationProvider::Codex, manifest)])
            .expect("fixture provider");
        let manager = AgentRuntimeManager::new(providers);
        let first = manager
            .ensure_async(request(
                root.to_str().unwrap(),
                "owned-pool-a",
                AgentConversationProvider::Codex,
            ))
            .await
            .unwrap();
        let second = manager
            .ensure_async(request(
                root.to_str().unwrap(),
                "owned-pool-b",
                AgentConversationProvider::Codex,
            ))
            .await
            .unwrap();
        manager
            .activate(&first.owned_id, first.generation)
            .await
            .unwrap();
        manager
            .activate(&second.owned_id, second.generation)
            .await
            .unwrap();

        let roots = manager.resource_roots();
        assert_eq!(roots.len(), 2, "each session overlays the shared root");
        assert_eq!(roots[0].pid, roots[1].pid);
        assert_eq!(
            fs::read_to_string(&log)
                .unwrap()
                .matches(r#""method":"initialize""#)
                .count(),
            1
        );

        manager
            .close(&first.owned_id, first.generation)
            .await
            .unwrap();
        assert!(process_is_alive(roots[0].pid));
        manager
            .prompt(
                &second.owned_id,
                second.generation,
                test_prompt("still live"),
            )
            .await
            .unwrap();
        manager
            .close(&second.owned_id, second.generation)
            .await
            .unwrap();
        wait_until(|| !process_is_alive(roots[0].pid)).await;
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn last_session_close_kills_process() {
        let fixture =
            fixture_manager_with_provider("multiplex", AgentConversationProvider::Codex, None)
                .await;
        let pid = fixture.manager.resource_roots()[0].pid;
        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        wait_until(|| !process_is_alive(pid)).await;
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn resume_failure_without_user_history_starts_fresh() {
        let root = temp_root();
        let log = root.join("resume-failure.jsonl");
        let manifest = super::super::providers::acp_client::tests::fixture_manifest_named(
            &log,
            "resume_failure",
        );
        let providers = ProviderRegistry::new([(AgentConversationProvider::Codex, manifest)])
            .expect("fixture provider");
        let manager = AgentRuntimeManager::new(providers);
        let mut ensure = request(
            root.to_str().unwrap(),
            "owned-resume-failure",
            AgentConversationProvider::Codex,
        );
        ensure.native_session_id = Some("missing-native".into());
        let connection = manager.ensure_inner(ensure).unwrap().0;
        let resumed = manager
            .activate(&connection.owned_id, connection.generation)
            .await
            .expect("empty session starts fresh");
        let snapshot = manager.snapshot(&connection.owned_id).unwrap().unwrap();
        assert!(!snapshot.suspended);
        assert_eq!(resumed.native_session_id.as_deref(), Some("new-session"));
        assert_eq!(
            snapshot.connection.native_session_id.as_deref(),
            Some("new-session")
        );
        assert!(!snapshot.events.iter().any(|event| matches!(
            &event.payload,
            AgentConversationPayload::Error { code, .. } if code == "session-resume-failed"
        )));
        let requests = fs::read_to_string(&log).unwrap();
        assert!(requests.contains(r#""method":"session/resume""#));
        assert!(requests.contains(r#""method":"session/new""#));
        manager
            .close(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn resume_failure_with_user_history_stays_suspended() {
        let root = temp_root();
        let log = root.join("resume-failure-with-history.jsonl");
        let manifest = super::super::providers::acp_client::tests::fixture_manifest_named(
            &log,
            "resume_failure",
        );
        let providers = ProviderRegistry::new([(AgentConversationProvider::Codex, manifest)])
            .expect("fixture provider");
        let manager = AgentRuntimeManager::new(providers);
        let mut ensure = request(
            root.to_str().unwrap(),
            "owned-resume-failure-with-history",
            AgentConversationProvider::Codex,
        );
        ensure.native_session_id = Some("missing-native".into());
        let connection = manager.ensure_inner(ensure).unwrap().0;
        {
            let mut sessions = manager.sessions.lock().unwrap();
            let session = sessions.get_mut(&connection.owned_id).unwrap();
            record_payload_for_session(
                session,
                AgentConversationPayload::UserMessage {
                    item_id: "stored-user-message".into(),
                    text: "Earlier message".into(),
                    completed: true,
                },
            )
            .unwrap();
        }

        assert!(manager
            .activate(&connection.owned_id, connection.generation)
            .await
            .unwrap_err()
            .contains("could not be resumed"));
        let snapshot = manager.snapshot(&connection.owned_id).unwrap().unwrap();
        assert!(snapshot.suspended);
        // The card has to name the cause, not just report that a resume failed.
        let resume_error = snapshot
            .events
            .iter()
            .find_map(|event| match &event.payload {
                AgentConversationPayload::Error { code, message, .. }
                    if code == "session-resume-failed" =>
                {
                    Some(message.clone())
                }
                _ => None,
            })
            .expect("resume failure is recorded");
        assert!(resume_error.starts_with("The stored provider session could not be resumed: "));
        assert!(resume_error.len() > "The stored provider session could not be resumed: ".len());
        let requests = fs::read_to_string(&log).unwrap();
        assert!(requests.contains(r#""method":"session/resume""#));
        assert!(!requests.contains(r#""method":"session/new""#));
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn late_approval_gets_stale_result() {
        let fixture = fixture_manager_with_acp_session("late_approval").await;
        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        let error = fixture
            .manager
            .respond_permission(PermissionResponse {
                identity: AgentRequestIdentity {
                    owned_id: fixture.owned_id.clone(),
                    generation: fixture.generation,
                    request_id: "expired-request".into(),
                    turn_id: None,
                    item_id: None,
                },
                decision: AgentApprovalDecision::Accept,
            })
            .await
            .unwrap_err();
        assert!(error.starts_with("Stale approval request:"));
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn load_replay_populates_two_turns_once_across_repeated_ensure() {
        let root = temp_root();
        let log = root.join("replay_on_load.jsonl");
        let manifest = super::super::providers::acp_client::tests::fixture_manifest_named(
            &log,
            "replay_on_load",
        );
        let providers = ProviderRegistry::new([(AgentConversationProvider::Codex, manifest)])
            .expect("fixture provider");
        let manager = AgentRuntimeManager::new(providers);
        let owned_id = "owned-load-replay";
        let mut load_request = request(
            root.to_str().unwrap(),
            owned_id,
            AgentConversationProvider::Codex,
        );
        load_request.native_session_id = Some("loaded-session".into());
        load_request.native_session_mode = AgentNativeSessionMode::Load;

        let connection = manager
            .ensure_async(load_request.clone())
            .await
            .expect("ensure loaded session");
        manager
            .activate(owned_id, connection.generation)
            .await
            .expect("initial load");
        wait_until(|| {
            manager
                .snapshot(owned_id)
                .unwrap()
                .is_some_and(|snapshot| snapshot.events.len() == 4)
        })
        .await;

        for _ in 0..3 {
            let same = manager
                .ensure_async(load_request.clone())
                .await
                .expect("connected ensure is idempotent");
            manager
                .activate(owned_id, same.generation)
                .await
                .expect("connected activation is idempotent");
        }

        let frames = fs::read_to_string(&log).expect("load fixture log");
        assert_eq!(frames.matches(r#""method":"session/load""#).count(), 1);
        let snapshot = manager.snapshot(owned_id).unwrap().unwrap();
        assert_eq!(snapshot.events.len(), 4);
        let item_ids = snapshot
            .events
            .iter()
            .filter_map(|event| match &event.payload {
                AgentConversationPayload::UserMessage { item_id, .. }
                | AgentConversationPayload::AssistantMessage { item_id, .. } => {
                    Some(item_id.as_str())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            item_ids,
            [
                "history-user-1",
                "history-agent-1",
                "history-user-2",
                "history-agent-2"
            ]
        );
        let recent_item_ids = manager
            .sessions
            .lock()
            .unwrap()
            .get(owned_id)
            .unwrap()
            .recent_events
            .iter()
            .filter_map(|event| {
                event
                    .payload
                    .get("itemId")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            recent_item_ids, item_ids,
            "the canonical journal feed and frontend snapshot must contain one copy per item"
        );

        manager
            .close(owned_id, connection.generation)
            .await
            .unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn late_prompt_once_updates_are_dropped_after_registration_ends() {
        let fixture = fixture_manager_with_acp_session("late_one_shot_update").await;
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        fixture
            .manager
            .set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));

        let generated = fixture
            .manager
            .prompt_once(
                &fixture.owned_id,
                fixture.generation,
                test_prompt("write a subject"),
            )
            .await
            .expect("one-shot prompt");
        assert_eq!(generated.text, "generated text");
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            seen.lock().unwrap().is_empty(),
            "a late update for the completed one-shot turn must be dropped"
        );

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn active_prompt_once_consumes_agent_minted_updates_without_emitting() {
        let fixture = fixture_manager_with_acp_session("agent_minted_in_flight_update").await;
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        fixture
            .manager
            .set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));

        let generated = fixture
            .manager
            .prompt_once(
                &fixture.owned_id,
                fixture.generation,
                test_prompt("write a subject"),
            )
            .await
            .expect("one-shot prompt");
        assert_eq!(generated.text, "agent-minted text");
        assert_eq!(generated.turn_id.as_deref(), Some("agent-turn-9"));
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            seen.lock().unwrap().is_empty(),
            "an in-flight one-shot update must not reach the conversation emitter"
        );

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn failed_prompt_emits_a_terminal_turn_before_clearing_it() {
        let fixture = fixture_manager_with_acp_session("prompt_error").await;
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        fixture
            .manager
            .set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));

        fixture
            .manager
            .prompt(&fixture.owned_id, fixture.generation, test_prompt("hello"))
            .await
            .expect("prompt starts");
        wait_until(|| {
            seen.lock().unwrap().iter().any(|event| {
                matches!(
                    event.payload,
                    AgentConversationPayload::Turn {
                        state: super::super::protocol::TurnState::Failed,
                        ..
                    }
                )
            })
        })
        .await;
        let seen = seen.lock().unwrap();
        let failed_turn = seen.iter().position(|event| {
            matches!(
                event.payload,
                AgentConversationPayload::Turn {
                    state: super::super::protocol::TurnState::Failed,
                    ..
                }
            )
        });
        let error = seen.iter().position(|event| {
            matches!(
                &event.payload,
                AgentConversationPayload::Error { code, .. } if code == "acp-error"
            )
        });
        assert!(failed_turn.is_some(), "failed prompt must close its turn");
        assert!(error.is_some(), "failed prompt must retain the error");
        assert!(failed_turn.unwrap() < error.unwrap());
        drop(seen);
        assert!(fixture
            .manager
            .sessions
            .lock()
            .unwrap()
            .get(&fixture.owned_id)
            .unwrap()
            .active_turn_id
            .is_none());

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn the_pump_is_silent_while_the_writer_lease_is_terminal() {
        let fixture = fixture_manager_with_acp_session("prompt_with_update").await;
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        fixture
            .manager
            .set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));
        let runtime = fixture
            .manager
            .runtime(&fixture.owned_id, fixture.generation)
            .unwrap();
        let transport = runtime.lock().await.transport().expect("transport");
        let native_session_id = fixture
            .manager
            .snapshot(&fixture.owned_id)
            .unwrap()
            .unwrap()
            .connection
            .native_session_id
            .unwrap();
        let handoff = terminal_handoff(&fixture.owned_id, fixture.generation, native_session_id);
        fixture.manager.handoff_prepare(&handoff).unwrap();
        fixture.manager.handoff_commit(&handoff).await.unwrap();

        transport
            .request(
                "session/prompt",
                json!({ "sessionId": "new-session", "prompt": [
                    { "type": "text", "text": "hello" }
                ] }),
            )
            .await
            .expect("fixture prompt");
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            seen.lock().unwrap().is_empty(),
            "terminal writer must be the only live event source"
        );

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn dropping_a_session_releases_the_pump_and_kills_the_sidecar() {
        let fixture = fixture_manager_with_acp_session("prompt_with_update").await;
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        fixture
            .manager
            .set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));
        let pid = fixture.manager.resource_roots()[0].pid;
        assert!(process_is_alive(pid));

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        wait_until(|| !process_is_alive(pid)).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        let events = seen.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0].payload,
            AgentConversationPayload::Connection {
                state: ConversationConnectionState::Closed,
                ..
            }
        ));
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn respond_agent_conversation_permission_round_trips_through_fake_agent() {
        let fixture = fixture_manager_with_acp_session("permission_midturn").await;
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        fixture
            .manager
            .set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));
        fixture
            .manager
            .prompt(&fixture.owned_id, fixture.generation, test_prompt("hello"))
            .await
            .expect("prompt starts");
        wait_until(|| {
            seen.lock().unwrap().iter().any(|event| {
                matches!(
                    event.payload,
                    AgentConversationPayload::Approval {
                        state: super::super::protocol::ApprovalState::Requested,
                        ..
                    }
                )
            })
        })
        .await;
        let request_id = seen
            .lock()
            .unwrap()
            .iter()
            .find_map(|event| match &event.payload {
                AgentConversationPayload::Approval { request_id, .. } => Some(request_id.clone()),
                _ => None,
            })
            .unwrap();
        fixture
            .manager
            .respond_legacy_approval(
                &fixture.owned_id,
                fixture.generation,
                request_id,
                super::super::protocol::ApprovalDecision::Accept,
            )
            .await
            .expect("permission response");
        wait_until(|| {
            seen.lock().unwrap().iter().any(|event| {
                matches!(
                    event.payload,
                    AgentConversationPayload::Turn {
                        state: super::super::protocol::TurnState::Completed,
                        ..
                    }
                )
            })
        })
        .await;

        assert!(seen.lock().unwrap().iter().any(|event| {
            matches!(
                event.payload,
                AgentConversationPayload::Approval {
                    state: ApprovalState::Accepted,
                    ..
                }
            )
        }));
        assert!(seen.lock().unwrap().iter().any(|event| {
            matches!(
                &event.payload,
                AgentConversationPayload::AssistantDelta { delta, .. }
                    if delta == "continued after approval"
            )
        }));
        let fixture_log = fs::read_to_string(fixture.root.join("permission_midturn.jsonl"))
            .expect("permission fixture log");
        assert!(
            fixture_log.contains(r#""outcome":{"outcome":"selected","optionId":"allow""#),
            "permission response must select a valid allow option: {fixture_log}"
        );

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn current_mode_update_refreshes_the_stored_config() {
        let fixture = fixture_manager_with_acp_session("current_mode_update").await;

        fixture
            .manager
            .prompt(
                &fixture.owned_id,
                fixture.generation,
                test_prompt("change mode"),
            )
            .await
            .expect("prompt starts");
        wait_until(|| {
            fixture
                .manager
                .conversation_config(&fixture.owned_id)
                .is_ok_and(|config| config.approval_policy.as_deref() == Some("plan"))
        })
        .await;

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn claude_medium_effort_is_injected_at_spawn_and_exposed_as_session_config() {
        let fixture = fixture_manager_with_provider(
            "claude_medium_effort",
            AgentConversationProvider::Claude,
            Some("medium"),
        )
        .await;

        let log = fs::read_to_string(fixture.root.join("claude_medium_effort.jsonl"))
            .expect("Claude spawn log");
        assert!(log.contains("MAX_THINKING_TOKENS=12000"));
        let config = fixture
            .manager
            .conversation_config(&fixture.owned_id)
            .expect("Claude config");
        assert_eq!(config.reasoning_effort.as_deref(), Some("medium"));
        assert_eq!(config.available_efforts, ["low", "medium", "high", "max"]);

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn claude_default_effort_removes_spawn_env_and_rejects_live_changes() {
        let fixture = fixture_manager_with_provider(
            "claude_default_effort",
            AgentConversationProvider::Claude,
            None,
        )
        .await;

        let log = fs::read_to_string(fixture.root.join("claude_default_effort.jsonl"))
            .expect("Claude spawn log");
        assert!(log.contains("MAX_THINKING_TOKENS=<unset>"));
        let config = fixture
            .manager
            .conversation_config(&fixture.owned_id)
            .expect("Claude config");
        assert_eq!(config.reasoning_effort, None);
        assert_eq!(config.available_efforts, ["low", "medium", "high", "max"]);

        let error = fixture
            .manager
            .set_conversation_config(SetAgentConversationConfigRequest {
                owned_id: fixture.owned_id.clone(),
                generation: fixture.generation,
                model: None,
                reasoning_effort: Some("high".into()),
                approval_policy: None,
            })
            .await
            .expect_err("Claude effort cannot change after spawn");
        assert_eq!(
            error,
            "Effort is set when the session starts. Start a new session to change it."
        );

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn session_start_effort_is_applied_after_start() {
        let fixture = fixture_manager_with_provider(
            "session_start_effort",
            AgentConversationProvider::Codex,
            Some("xhigh"),
        )
        .await;

        let log = fs::read_to_string(fixture.root.join("session_start_effort.jsonl"))
            .expect("fixture request log");
        assert!(log.contains(r#""method":"session/set_config_option""#));
        assert!(log.contains(r#""reasoningEffort":"xhigh""#));
        assert_eq!(
            fixture
                .manager
                .conversation_config(&fixture.owned_id)
                .expect("stored config")
                .reasoning_effort
                .as_deref(),
            Some("xhigh")
        );

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn stored_session_effort_is_reapplied_on_resume() {
        let root = temp_root();
        let database = root.join("sessions.db");
        let log = root.join("stored-session-effort.jsonl");
        let manifest = super::super::providers::acp_client::tests::fixture_manifest_named(
            &log,
            "suspend_stored_session_effort",
        );
        let providers =
            ProviderRegistry::new([(AgentConversationProvider::Codex, manifest.clone())]).unwrap();
        let manager = AgentRuntimeManager::open(providers, &database).unwrap();
        let mut ensure = request(
            root.to_str().unwrap(),
            "owned-stored-session-effort",
            AgentConversationProvider::Codex,
        );
        ensure.reasoning_effort = Some("xhigh".into());
        let connection = manager.ensure_async(ensure).await.unwrap();
        manager
            .activate(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        drop(manager);

        let providers =
            ProviderRegistry::new([(AgentConversationProvider::Codex, manifest)]).unwrap();
        let recovered = AgentRuntimeManager::open(providers, &database).unwrap();
        recovered
            .activate(&connection.owned_id, connection.generation)
            .await
            .expect("stored session resumes");
        let snapshot = recovered.snapshot(&connection.owned_id).unwrap().unwrap();
        assert!(!snapshot.suspended);
        assert_eq!(
            snapshot.connection.config.reasoning_effort.as_deref(),
            Some("xhigh")
        );
        let requests = fs::read_to_string(&log).expect("fixture request log");
        assert!(requests.contains(r#""method":"session/resume""#));
        assert_eq!(requests.matches("session/set_config_option").count(), 2);

        recovered
            .close(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn session_start_effort_failure_keeps_session_ready() {
        let fixture = fixture_manager_with_provider(
            "config_update_failure",
            AgentConversationProvider::Codex,
            Some("xhigh"),
        )
        .await;

        let snapshot = fixture
            .manager
            .snapshot(&fixture.owned_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            snapshot.connection.state,
            ConversationConnectionState::Connected
        );
        assert_eq!(snapshot.connection.config.reasoning_effort, None);
        assert!(!snapshot
            .events
            .iter()
            .any(|event| matches!(event.payload, AgentConversationPayload::Error { .. })));
        let requests = fs::read_to_string(fixture.root.join("config_update_failure.jsonl"))
            .expect("fixture request log");
        assert!(requests.contains(r#""method":"session/set_config_option""#));

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn config_change_wakes_a_suspended_session() {
        let fixture = fixture_manager_with_acp_session("suspend_config_wake").await;
        assert!(fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .unwrap());

        let configured = fixture
            .manager
            .set_conversation_config(SetAgentConversationConfigRequest {
                owned_id: fixture.owned_id.clone(),
                generation: fixture.generation,
                model: Some("gpt-5.6-terra".into()),
                reasoning_effort: None,
                approval_policy: None,
            })
            .await
            .expect("a suspended session wakes for a settings change");
        assert_eq!(configured.model.as_deref(), Some("gpt-5.6-terra"));
        let snapshot = fixture
            .manager
            .snapshot(&fixture.owned_id)
            .unwrap()
            .unwrap();
        assert!(!snapshot.suspended);

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn conversation_config_commands_read_set_forward_and_reject_stale_generation() {
        let fixture = fixture_manager_with_acp_session("conversation_config").await;
        let initial = fixture
            .manager
            .conversation_config(&fixture.owned_id)
            .expect("initial config");
        assert_eq!(initial.model.as_deref(), Some("gpt-5.6-sol"));
        assert_eq!(initial.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(initial.approval_policy.as_deref(), Some("on-request"));

        let configured = fixture
            .manager
            .set_conversation_config(SetAgentConversationConfigRequest {
                owned_id: fixture.owned_id.clone(),
                generation: fixture.generation,
                model: Some("gpt-5.6-terra".into()),
                reasoning_effort: Some("xhigh".into()),
                approval_policy: Some("never".into()),
            })
            .await
            .expect("set config");
        assert_eq!(configured.model.as_deref(), Some("gpt-5.6-terra"));
        assert_eq!(configured.reasoning_effort.as_deref(), Some("xhigh"));
        assert_eq!(configured.approval_policy.as_deref(), Some("never"));
        assert_eq!(
            fixture
                .manager
                .conversation_config(&fixture.owned_id)
                .expect("stored config"),
            configured
        );

        let stale = fixture
            .manager
            .set_conversation_config(SetAgentConversationConfigRequest {
                owned_id: fixture.owned_id.clone(),
                generation: fixture.generation.saturating_add(1),
                model: Some("gpt-5.6-sol".into()),
                reasoning_effort: None,
                approval_policy: None,
            })
            .await
            .expect_err("stale generation must fail");
        assert_eq!(
            stale,
            "Conversation connection changed; retry on the current session"
        );

        let fixture_log = fs::read_to_string(fixture.root.join("conversation_config.jsonl"))
            .expect("config fixture log");
        assert_eq!(fixture_log.matches("session/set_config_option").count(), 1);
        assert!(fixture_log.contains(r#""model":"gpt-5.6-terra""#));
        assert!(fixture_log.contains(r#""reasoningEffort":"xhigh""#));
        assert!(fixture_log.contains(r#""approvalPolicy":"never""#));

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cancelled_prompt_emits_interrupted_turn() {
        let fixture = fixture_manager_with_acp_session("cancelled_turn").await;
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        fixture
            .manager
            .set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));
        fixture
            .manager
            .prompt(&fixture.owned_id, fixture.generation, test_prompt("hello"))
            .await
            .expect("prompt starts");
        let fixture_log_path = fixture.root.join("cancelled_turn.jsonl");
        wait_until(|| {
            fs::read_to_string(&fixture_log_path)
                .map(|log| log.contains(r#""method":"session/prompt""#))
                .unwrap_or(false)
        })
        .await;
        fixture
            .manager
            .cancel_turn(&fixture.owned_id, fixture.generation)
            .await
            .expect("cancel notification");
        wait_until(|| {
            seen.lock().unwrap().iter().any(|event| {
                matches!(
                    event.payload,
                    AgentConversationPayload::Turn {
                        state: super::super::protocol::TurnState::Interrupted,
                        ..
                    }
                )
            })
        })
        .await;
        let seen = seen.lock().unwrap();
        assert!(!seen.iter().any(|event| {
            matches!(
                event.payload,
                AgentConversationPayload::Turn {
                    state: super::super::protocol::TurnState::Completed,
                    ..
                }
            )
        }));
        assert!(seen
            .windows(2)
            .all(|events| events[1].sequence == events[0].sequence + 1));
        drop(seen);
        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cancelling_a_prompt_answers_pending_permissions() {
        let fixture = fixture_manager_with_acp_session("permission_cancelled").await;
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        fixture
            .manager
            .set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));
        fixture
            .manager
            .prompt(&fixture.owned_id, fixture.generation, test_prompt("hello"))
            .await
            .expect("prompt starts");
        wait_until(|| {
            seen.lock().unwrap().iter().any(|event| {
                matches!(
                    event.payload,
                    AgentConversationPayload::Approval {
                        state: ApprovalState::Requested,
                        ..
                    }
                )
            })
        })
        .await;
        fixture
            .manager
            .cancel_turn(&fixture.owned_id, fixture.generation)
            .await
            .expect("cancel notification");
        wait_until(|| {
            seen.lock().unwrap().iter().any(|event| {
                matches!(
                    event.payload,
                    AgentConversationPayload::Turn {
                        state: super::super::protocol::TurnState::Interrupted,
                        ..
                    }
                )
            })
        })
        .await;
        let seen = seen.lock().unwrap();
        assert!(seen.iter().any(|event| {
            matches!(
                event.payload,
                AgentConversationPayload::Approval {
                    state: ApprovalState::Declined,
                    ..
                }
            )
        }));
        drop(seen);
        let fixture_log = fs::read_to_string(fixture.root.join("permission_cancelled.jsonl"))
            .expect("permission cancellation fixture log");
        assert!(
            fixture_log.contains(r#""outcome":{"outcome":"cancelled"}"#),
            "cancellation must answer the wire permission request: {fixture_log}"
        );
        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn transport_exit_emits_connection_failure_and_recoverable_error() {
        let fixture = fixture_manager_with_acp_session("dies_midturn").await;
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        fixture
            .manager
            .set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));
        fixture
            .manager
            .prompt(&fixture.owned_id, fixture.generation, test_prompt("hello"))
            .await
            .expect("prompt starts");
        wait_until(|| {
            seen.lock().unwrap().iter().any(|event| {
                matches!(
                    &event.payload,
                    AgentConversationPayload::Error {
                        code,
                        recoverable: true,
                        ..
                    } if code == "acp-transport"
                )
            })
        })
        .await;
        let seen = seen.lock().unwrap();
        assert!(seen.iter().any(|event| {
            matches!(
                event.payload,
                AgentConversationPayload::Connection {
                    state: ConversationConnectionState::Failed,
                    ..
                }
            )
        }));
        assert!(seen
            .windows(2)
            .all(|events| events[1].sequence == events[0].sequence + 1));
        assert!(seen.iter().any(|event| {
            matches!(
                event.payload,
                AgentConversationPayload::Turn {
                    state: super::super::protocol::TurnState::Failed,
                    ..
                }
            )
        }));
        drop(seen);
        assert_eq!(
            fixture
                .manager
                .sessions
                .lock()
                .unwrap()
                .get(&fixture.owned_id)
                .unwrap()
                .state,
            AgentRuntimeState::Failed
        );

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn failed_prompt_racing_transport_exit_expires_pending_approval_once() {
        let fixture = fixture_manager_with_acp_session("permission_prompt_error").await;
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        fixture
            .manager
            .set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));
        fixture
            .manager
            .prompt(&fixture.owned_id, fixture.generation, test_prompt("hello"))
            .await
            .expect("prompt starts");
        wait_until(|| {
            let seen = seen.lock().unwrap();
            seen.iter().any(|event| {
                matches!(
                    event.payload,
                    AgentConversationPayload::Approval {
                        state: ApprovalState::Expired,
                        ..
                    }
                )
            }) && seen.iter().any(|event| {
                matches!(
                    event.payload,
                    AgentConversationPayload::Turn {
                        state: super::super::protocol::TurnState::Failed,
                        ..
                    }
                )
            })
        })
        .await;
        let seen = seen.lock().unwrap();
        assert_eq!(
            seen.iter()
                .filter(|event| {
                    matches!(
                        event.payload,
                        AgentConversationPayload::Approval {
                            state: ApprovalState::Expired,
                            ..
                        }
                    )
                })
                .count(),
            1,
            "the failed prompt must expire its approval exactly once"
        );
        assert_eq!(
            seen.iter()
                .filter(|event| {
                    matches!(
                        event.payload,
                        AgentConversationPayload::Turn {
                            state: super::super::protocol::TurnState::Failed,
                            ..
                        }
                    )
                })
                .count(),
            1,
            "the failed prompt must emit one terminal turn"
        );
        assert!(seen.iter().any(|event| {
            matches!(
                event.payload,
                AgentConversationPayload::Approval {
                    state: ApprovalState::Requested,
                    ..
                }
            )
        }));
        assert!(seen.iter().any(|event| {
            matches!(
                event.payload,
                AgentConversationPayload::Turn {
                    state: super::super::protocol::TurnState::Failed,
                    ..
                }
            )
        }));
        assert!(
            seen.windows(2)
                .all(|events| events[1].sequence == events[0].sequence + 1),
            "approval/turn race must preserve contiguous event sequences"
        );
        drop(seen);

        let _ = fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await;
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[test]
    fn ensure_is_idempotent_and_generation_rejects_stale_writers() {
        let root = temp_root();
        let manager = AgentRuntimeManager::default();
        let first = manager
            .ensure_inner(request(
                root.to_str().unwrap(),
                "owned-a",
                AgentConversationProvider::Codex,
            ))
            .unwrap()
            .0;
        assert_eq!(
            manager
                .ensure_inner(request(
                    root.to_str().unwrap(),
                    "owned-a",
                    AgentConversationProvider::Codex
                ))
                .unwrap()
                .0,
            first
        );
        let changed = manager
            .ensure_inner(request(
                root.to_str().unwrap(),
                "owned-a",
                AgentConversationProvider::Claude,
            ))
            .unwrap()
            .0;
        assert_eq!(changed.generation, 2);
        let mut sessions = manager.sessions.lock().unwrap();
        assert!(current_session_mut(&mut sessions, "owned-a", first.generation).is_err());
        drop(sessions);
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn ensure_and_resource_listing_recover_after_sessions_mutex_is_poisoned() {
        let root = temp_root();
        let manager = AgentRuntimeManager::default();
        let poisoned_sessions = Arc::clone(&manager.sessions);

        let panic = std::thread::spawn(move || {
            let _sessions = poisoned_sessions.lock().unwrap();
            panic!("deliberately poison the agent sessions mutex");
        })
        .join();
        assert!(panic.is_err());
        assert!(manager.sessions.is_poisoned());

        let connection = manager
            .ensure_async(request(
                root.to_str().unwrap(),
                "owned-after-poison",
                AgentConversationProvider::Codex,
            ))
            .await
            .expect("ensure must recover the poisoned sessions map");
        let snapshot = manager
            .snapshot("owned-after-poison")
            .expect("snapshot listing must recover the poisoned sessions map")
            .expect("ensured session must be listed");

        assert_eq!(snapshot.connection, connection);
        assert!(manager.resource_roots().is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn emitter_panic_is_contained_before_it_can_poison_sessions() {
        let root = temp_root();
        let manager = AgentRuntimeManager::default();
        let connection = manager
            .ensure_inner(request(
                root.to_str().unwrap(),
                "owned-emitter-panic",
                AgentConversationProvider::Codex,
            ))
            .unwrap()
            .0;
        manager.set_emitter(Arc::new(|_| panic!("fixture emitter panic")));

        {
            let mut sessions = manager
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session =
                current_session_mut(&mut sessions, "owned-emitter-panic", connection.generation)
                    .unwrap();
            record_payload_for_session_and_dispatch(
                session,
                &manager.emitter,
                AgentConversationPayload::Error {
                    code: "fixture".into(),
                    message: "retained after emitter panic".into(),
                    recoverable: true,
                },
            )
            .unwrap();
        }

        assert!(!manager.sessions.is_poisoned());
        assert_eq!(
            manager
                .snapshot("owned-emitter-panic")
                .unwrap()
                .unwrap()
                .events
                .len(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_event_write_leaves_memory_and_dispatch_unchanged() {
        let root = temp_root();
        let manager = AgentRuntimeManager::default();
        let connection = manager
            .ensure_inner(request(
                root.to_str().unwrap(),
                "owned-failed-event",
                AgentConversationProvider::Codex,
            ))
            .unwrap()
            .0;
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        manager.set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));

        let mut sessions = manager.sessions.lock().unwrap();
        let session =
            current_session_mut(&mut sessions, "owned-failed-event", connection.generation)
                .unwrap();
        let sequence = session.next_sequence;
        let last_activity_ms = session.last_activity_ms;
        session
            .store
            .append_event(&EventRow {
                owned_id: session.owned_id.clone(),
                seq: i64::try_from(sequence).unwrap(),
                turn_id: None,
                kind: "error".into(),
                payload_json: "{}".into(),
                created_at_ms: store_timestamp(last_activity_ms),
            })
            .unwrap();

        let result = record_payload_for_session_and_dispatch(
            session,
            &manager.emitter,
            AgentConversationPayload::Error {
                code: "duplicate-sequence".into(),
                message: "fixture".into(),
                recoverable: true,
            },
        );

        assert!(result.is_err());
        assert_eq!(session.next_sequence, sequence);
        assert_eq!(session.last_activity_ms, last_activity_ms);
        assert!(seen.lock().unwrap().is_empty());
        drop(sessions);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn canonical_sequence_and_bounded_snapshot_tail_are_repairable() {
        let root = temp_root();
        let manager = AgentRuntimeManager::default();
        let connection = manager
            .ensure_inner(request(
                root.to_str().unwrap(),
                "owned-a",
                AgentConversationProvider::Codex,
            ))
            .unwrap()
            .0;
        for index in 0..(SNAPSHOT_EVENT_CAP + 3) {
            let mut sessions = manager.sessions.lock().unwrap();
            let session =
                current_session_mut(&mut sessions, "owned-a", connection.generation).unwrap();
            record_payload_for_session_and_dispatch(
                session,
                &manager.emitter,
                AgentConversationPayload::Error {
                    code: "fixture".into(),
                    message: index.to_string(),
                    recoverable: true,
                },
            )
            .unwrap();
        }
        let stored = manager.list_events("owned-a", 0).unwrap();
        assert_eq!(stored.len(), SNAPSHOT_EVENT_CAP + 3);
        let snapshot = manager.snapshot("owned-a").unwrap().unwrap().events;
        assert_eq!(snapshot.len(), SNAPSHOT_EVENT_CAP);
        assert_eq!(snapshot.first().unwrap().sequence, 4);
        assert_eq!(snapshot.last().unwrap().sequence, 2_003);
        assert!(snapshot
            .windows(2)
            .all(|pair| pair[1].sequence == pair[0].sequence + 1));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn projected_terminal_payload_continues_the_journal_sequence() {
        let root = temp_root();
        let manager = AgentRuntimeManager::default();
        let connection = manager
            .ensure_inner(request(
                root.to_str().unwrap(),
                "owned-projection",
                AgentConversationProvider::Codex,
            ))
            .unwrap()
            .0;
        {
            let mut sessions = manager.sessions.lock().unwrap();
            let session =
                current_session_mut(&mut sessions, "owned-projection", connection.generation)
                    .unwrap();
            record_payload_for_session_and_dispatch(
                session,
                &manager.emitter,
                AgentConversationPayload::Error {
                    code: "fixture".into(),
                    message: "first journal event".into(),
                    recoverable: true,
                },
            )
            .unwrap();
        }

        manager
            .submit_terminal_projection(
                "owned-projection",
                super::super::protocol::TerminalProjectionPayload {
                    event_type: AgentEventType::ItemCompleted,
                    provider_instance_id: "terminal-transcript:native-projection".into(),
                    timestamp_ms: Some(42),
                    native_session_id: "native-projection".into(),
                    item_id: Some("projected-item".into()),
                    payload: BTreeMap::from([(
                        "text".into(),
                        Value::String("projected answer".into()),
                    )]),
                    provider_metadata: BTreeMap::from([(
                        "source".into(),
                        Value::String("terminal-transcript".into()),
                    )]),
                    raw_frame_reference: super::super::protocol::AgentRawFrameReference {
                        id: "frame-projection".into(),
                        redacted: true,
                    },
                },
            )
            .unwrap();

        let stored = manager.list_events("owned-projection", 0).unwrap();
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[1].generation, connection.generation);
        assert_eq!(stored[1].sequence, 2);
        assert!(matches!(
            stored[1].payload,
            AgentConversationPayload::TerminalProjection(_)
        ));
        let snapshot = manager.snapshot("owned-projection").unwrap().unwrap();
        assert_eq!(snapshot.last_sequence, 2);
        assert_eq!(snapshot.events, stored);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn structured_session_does_not_claim_a_user_pty() {
        let root = temp_root();
        let manager = AgentRuntimeManager::default();
        manager
            .ensure_inner(request(
                root.to_str().unwrap(),
                "owned-a",
                AgentConversationProvider::Codex,
            ))
            .unwrap();
        assert_eq!(
            manager.session_terminal_ownership("owned-a"),
            Some(AgentWriterLeaseOwner::None)
        );
        let terminal_registry = crate::terminal::TerminalRegistry::default();
        assert!(crate::terminal::list_terminal_sessions(&terminal_registry)
            .unwrap()
            .is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn distinct_tool_terminal_identity_is_not_a_user_pty_identity() {
        let first = crate::terminal::ToolTerminalIdentity {
            owned_id: "owned-a".into(),
            turn_id: "turn-a".into(),
            tool_call_id: "tool-a".into(),
            terminal_id: "terminal-a".into(),
        };
        let second = crate::terminal::ToolTerminalIdentity {
            owned_id: "owned-a".into(),
            turn_id: "turn-a".into(),
            tool_call_id: "tool-b".into(),
            terminal_id: "terminal-b".into(),
        };
        assert_ne!(first, second);
        assert_ne!(
            crate::terminal::TerminalKind::AgentTool,
            crate::terminal::TerminalKind::UserPty
        );
    }
}
