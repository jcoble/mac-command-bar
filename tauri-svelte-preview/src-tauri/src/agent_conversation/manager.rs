use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mcb_core::session_store::{AnnotationRow, EventRow, SessionRow, SessionStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

use super::broker_status::{status_for, AgentWorkStatus, WorkflowBrokerEvent};
use super::handoff::{
    AgentConversationHandoffDirection, AgentConversationHandoffMode, AgentConversationHandoffPhase,
    AgentConversationHandoffReceipt, AgentConversationHandoffRequest,
};
use super::protocol::{
    AgentApprovalDecision, AgentApprovalResponse, AgentCapabilities, AgentCommandDescriptor,
    AgentConversationConfigState, AgentConversationConnection, AgentConversationEvent,
    AgentConversationEventPage, AgentConversationPayload, AgentConversationProvider,
    AgentConversationSessionMeta, AgentConversationSessionRecord, AgentConversationSnapshot,
    AgentEvent, AgentEventType, AgentExecutionOwner, AgentImplementation,
    AgentInteractionCapabilities, AgentNativeSessionMode, AgentPromptCapabilities,
    AgentRequestIdentity, AgentRuntimeState, AgentSessionCapabilities, AgentUserInputResponse,
    AgentWriterLease, AgentWriterLeaseOwner, AgentWriterLeaseTransition, ApprovalState,
    ChangeAgentConversationCheckoutRequest, ConversationConnectionState,
    EnsureAgentConversationRequest, PlanItem, SetAgentConversationConfigRequest,
    TerminalProjectionPayload, ToolState, UpdateAgentConversationSessionMetaRequest,
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

/// How much of a conversation opening it hands over, in bytes of stored event.
///
/// Bytes rather than a count of events, because an event is anything from a
/// config record to a sixteen-kilobyte tool result: the same count of them is a
/// different amount of reading in every session, and it is the bytes that cost
/// the fetch, the hop to the front end and the parse at the other end.
const SNAPSHOT_WINDOW_BYTES: u32 = 512 * 1024;
/// The most events one catch-up read will hand back after a missed update.
///
/// A bound on a single fetch, not on what a session keeps. Nothing trims a
/// conversation's journal: history costs disk and nothing else, and every read
/// of it is already bounded.
const CATCH_UP_EVENT_CAP: u32 = 10_000;
const SESSION_TITLE_CHAR_CAP: usize = 64;

/// Where a session's name came from, as it is written to the row. A row with
/// none of these on it was written before the app recorded this, and is read as
/// having come from the prompt.
const TITLE_SOURCE_PROMPT: &str = "prompt";
const TITLE_SOURCE_HELPER: &str = "helper";
const TITLE_SOURCE_USER: &str = "user";

/// How much of the first prompt, and of the reply to it, the helper model is
/// given to name a session by. A name comes from the shape of an exchange, not
/// from all of it, and the call is billed by the token.
const TITLE_INPUT_CHAR_CAP: usize = 1_000;
const CHILD_ROLLOUT_SCAN_INTERVAL: Duration = Duration::from_secs(10);
const MAX_THINKING_TOKENS_ENV: &str = "MAX_THINKING_TOKENS";
/// The efforts a Claude conversation can offer before it has an adapter.
///
/// A new conversation picks its effort before anything is running, so the
/// picker needs something to show. Once the adapter has a session it reports
/// its own effort control, and that report replaces this list.
const CLAUDE_SESSION_EFFORTS: [&str; 4] = ["low", "medium", "high", "max"];

pub type ConversationEmitter = std::sync::Arc<dyn Fn(AgentConversationEvent) + Send + Sync>;
pub type BrokerEmitter = std::sync::Arc<dyn Fn(WorkflowBrokerEvent) + Send + Sync>;

/// Asks the helper model to name one session from its first exchange. Set once
/// at launch; with none set the helper is not involved and a session keeps the
/// name taken from its first prompt.
pub type SessionNamer =
    std::sync::Arc<dyn Fn(&str) -> Result<String, crate::helper::HelperError> + Send + Sync>;

/// Told which session has just been given a new name, once that name is saved,
/// so the rail can show it without asking for it.
pub type SessionRenamedListener = std::sync::Arc<dyn Fn(&str, &str) + Send + Sync>;

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
    pub next_sequence: i64,
    pub active_turn_id: Option<String>,
    prompt_once_active: bool,
    pub runtime: Option<Arc<AsyncMutex<StructuredRuntimeHandle>>>,
    pool_key: Option<AdapterPoolKey>,
    transport: Option<Arc<AcpTransport>>,
    pub writer_lease: AgentWriterLease,
    pub writer_lease_transition: Option<AgentWriterLeaseTransition>,
    permission_requests: HashMap<String, PendingPermission>,
    next_permission_id: u64,
    user_input_requests: HashMap<String, PendingUserInput>,
    next_user_input_id: u64,
    ordered_events: Option<UnboundedSender<OrderedSessionEvent>>,
    cwd: String,
    spawn_reasoning_effort: Option<String>,
    /// Whether the running adapter reported an effort control of its own.
    /// When it did, the effort can be changed at any time; when it did not,
    /// the session keeps the effort it was started with.
    adapter_offers_effort: bool,
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
    /// Where the current name came from: the first prompt, the helper model, or
    /// the person. A session recovered from a row written before this was
    /// recorded carries none, and counts as the first prompt.
    title_source: Option<String>,
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
    next_sequence: i64,
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
    broker_emitter: Arc<Mutex<Option<BrokerEmitter>>>,
    broker_statuses: Arc<Mutex<HashMap<String, AgentWorkStatus>>>,
    namer: Arc<Mutex<Option<SessionNamer>>>,
    renamed_listener: Arc<Mutex<Option<SessionRenamedListener>>>,
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

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRuntimeDiagnostics {
    pub durable_session_rows: usize,
    pub live_session_overlays: usize,
    pub live_runtime_handles: usize,
    pub sidecar_processes: usize,
    pub active_turns: usize,
    pub pending_permissions: usize,
    pub pending_inputs: usize,
    pub live_tool_calls: usize,
    pub background_work: usize,
    pub suspendable_sessions: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_pools: Option<usize>,
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
            broker_emitter: Arc::new(Mutex::new(None)),
            broker_statuses: Arc::new(Mutex::new(HashMap::new())),
            namer: Arc::new(Mutex::new(None)),
            renamed_listener: Arc::new(Mutex::new(None)),
            activation_locks: Arc::new(Mutex::new(HashMap::new())),
            store,
            adapter_pools: Arc::new(AsyncMutex::new(HashMap::new())),
        })
    }

    /// The durable store, for records that live beside a session but are written
    /// by the module that owns them, such as the conversation attachment index.
    pub(crate) fn store(&self) -> &SessionStore {
        &self.store
    }

    pub(crate) fn store_handle(&self) -> Arc<SessionStore> {
        Arc::clone(&self.store)
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

    pub fn write_workspace(&self, owned_id: &str, snapshot_json: &str) -> Result<(), String> {
        self.store
            .upsert_workspace_snapshot(owned_id, snapshot_json)
            .map_err(|error| error.to_string())
    }

    pub fn read_workspace(&self, owned_id: &str) -> Result<Option<String>, String> {
        self.store
            .get_workspace_snapshot(owned_id)
            .map_err(|error| error.to_string())
    }

    pub fn delete_workspace(&self, owned_id: &str) -> Result<(), String> {
        self.store
            .delete_workspace_snapshot(owned_id)
            .map_err(|error| error.to_string())
    }

    pub fn clear_workspace_editors(&self) -> Result<(), String> {
        self.store
            .clear_workspace_editor_tabs()
            .map_err(|error| error.to_string())
    }

    pub fn clear_workspace_tabs(&self) -> Result<(), String> {
        self.store
            .clear_workspace_selected_tabs()
            .map_err(|error| error.to_string())
    }

    pub fn write_app_setting(&self, setting_key: &str, value_json: &str) -> Result<(), String> {
        self.store
            .upsert_app_setting(setting_key, value_json)
            .map_err(|error| error.to_string())
    }

    pub fn read_app_setting(&self, setting_key: &str) -> Result<Option<String>, String> {
        self.store
            .get_app_setting(setting_key)
            .map_err(|error| error.to_string())
    }

    #[allow(clippy::too_many_arguments)]
    /// Names an imported session without reading a single record of it, so the
    /// rail has something to show while the transcript is still being read.
    pub fn begin_import_transcript_session(
        &self,
        provider: AgentConversationProvider,
        native_session_id: &str,
        path: &Path,
        cwd: &str,
        title: Option<String>,
    ) -> Result<String, String> {
        let owned_id = super::transcript_import::begin_import_session(
            &self.store,
            provider,
            native_session_id,
            path,
            cwd,
            title,
        )?;
        self.restore_overlay_from_store(&owned_id)?;
        Ok(owned_id)
    }

    /// Reads the newest page of an imported transcript into a session that has
    /// already been named, and returns how many records it gained.
    pub fn finish_import_transcript_session(
        &self,
        owned_id: &str,
        max_bytes: u64,
        max_records: usize,
    ) -> Result<usize, String> {
        let count = super::transcript_import::finish_import_session(
            &self.store,
            owned_id,
            max_bytes,
            max_records,
        )?;
        // Built again now that the records exist. The overlay carries the events
        // a snapshot is read from, and the one made when the session was named
        // was made from an empty row — leaving it in place showed the title over
        // an empty conversation until the next reload.
        self.restore_overlay_from_store(owned_id)?;
        Ok(count)
    }

    /// Rebuilds one session's live overlay from what is stored.
    ///
    /// Every stored row needs an overlay before anything lists the sessions or
    /// reads one — listing fails outright on a row without one, and reading
    /// reports an empty conversation. An import calls this because it has just
    /// written a row nothing has an overlay for yet, and reading calls it
    /// because the overlay is runtime state that comes and goes underneath a
    /// conversation that is durably on disk either way.
    /// A rebuild never moves a session's generation backwards. The stored row
    /// carries the generation the session had when it was last written, and an
    /// import writes zero; ensuring the same session meanwhile raises the live
    /// one. Replacing the live session with the stored one then published a
    /// conversation numbered below what the reader had already been told, and
    /// the reader discards those — it is how a resumed transcript arrived
    /// complete and was thrown away, in silence, until the next launch started
    /// the count again from nothing.
    fn restore_overlay_from_store(&self, owned_id: &str) -> Result<(), String> {
        let row = self
            .store
            .get_session(owned_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "The imported session was not stored".to_string())?;
        let mut session = recovered_session_from_row(&self.store, row)?;
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(live) = sessions.get(owned_id) {
            if live.generation > session.generation {
                session.generation = live.generation;
                session.connection.generation = live.generation;
            }
        }
        sessions.insert(owned_id.to_string(), session);
        Ok(())
    }

    pub fn extend_imported_session(
        &self,
        owned_id: &str,
        max_bytes: u64,
        max_records: usize,
    ) -> Result<super::transcript_import::ExtendedImport, String> {
        super::transcript_import::extend_session(&self.store, owned_id, max_bytes, max_records)
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
        let last_member = pool.members.len() == 1 && pool.members.contains(owned_id);
        if last_member {
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
        }
        pool.members.remove(owned_id);
        if pool.members.is_empty() {
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

    pub fn set_broker_emitter(&self, emitter: BrokerEmitter) {
        let mut current = self
            .broker_emitter
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *current = Some(emitter);
    }

    fn sync_broker_status(
        &self,
        owned_id: &str,
        generation: u64,
        runtime_error: bool,
    ) -> Result<(), String> {
        let (turn_active, awaiting_permission_or_input, session_closed) = {
            let sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = current_session(&sessions, owned_id, generation)?;
            (
                session.active_turn_id.is_some(),
                !session.permission_requests.is_empty() || !session.user_input_requests.is_empty(),
                session.state == AgentRuntimeState::Closed,
            )
        };
        let Some((group_id, _, orchestrator, _)) = self
            .store
            .list_open_groups()
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|(_, _, _, members)| members.iter().any(|member| member == owned_id))
        else {
            return Ok(());
        };
        let decision_pending = self
            .store
            .pending_for(&group_id, owned_id)
            .map_err(|error| error.to_string())?
            .iter()
            .any(|envelope| envelope.kind == mcb_core::broker::MessageKind::DecisionRequest);
        let status = status_for(
            turn_active,
            awaiting_permission_or_input,
            decision_pending,
            runtime_error,
            session_closed,
        );
        let mut statuses = self
            .broker_statuses
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if statuses.get(owned_id) == Some(&status) {
            return Ok(());
        }
        let envelope = self
            .store
            .append_message(
                &group_id,
                owned_id,
                orchestrator.as_deref().unwrap_or("orchestrator"),
                mcb_core::broker::MessageKind::Status,
                status.as_str(),
            )
            .map_err(|error| error.to_string())?;
        statuses.insert(owned_id.to_string(), status);
        let callback = self
            .broker_emitter
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if let Some(callback) = callback {
            callback(WorkflowBrokerEvent {
                group_id,
                envelope: envelope.into(),
            });
        }
        Ok(())
    }

    pub fn set_session_namer(&self, namer: SessionNamer) {
        let mut current = self
            .namer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *current = Some(namer);
    }

    pub fn set_session_renamed_listener(&self, listener: SessionRenamedListener) {
        let mut current = self
            .renamed_listener
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *current = Some(listener);
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

    pub fn resource_diagnostics(&self) -> Result<AgentRuntimeDiagnostics, String> {
        let durable_session_rows = self.store.count_sessions().map_err(|error| error.to_string())?;
        let sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let diagnostics = AgentRuntimeDiagnostics {
            durable_session_rows,
            live_session_overlays: sessions.len(),
            live_runtime_handles: sessions
                .values()
                .filter(|session| session.runtime.is_some())
                .count(),
            sidecar_processes: sessions
                .values()
                .filter_map(|session| session.runtime.as_ref())
                .filter_map(|runtime| runtime.try_lock().ok())
                .filter(|runtime| runtime.process_id().is_some())
                .count(),
            active_turns: sessions
                .values()
                .filter(|session| session.active_turn_id.is_some())
                .count(),
            pending_permissions: sessions
                .values()
                .map(|session| session.permission_requests.len())
                .sum(),
            pending_inputs: sessions
                .values()
                .map(|session| session.user_input_requests.len())
                .sum(),
            live_tool_calls: sessions
                .values()
                .map(|session| session.live_tool_calls.len())
                .sum(),
            background_work: sessions
                .values()
                .map(|session| session.background_work.len())
                .sum(),
            suspendable_sessions: sessions
                .values()
                .filter(|session| session_can_suspend(session))
                .count(),
            adapter_pools: None,
        };
        drop(sessions);
        Ok(AgentRuntimeDiagnostics {
            adapter_pools: self.adapter_pools.try_lock().ok().map(|pools| pools.len()),
            ..diagnostics
        })
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
        let (connection, mut prior) = self.ensure_inner(request)?;
        if let Some(previous) = prior.as_ref() {
            if let Some(pool_key) = previous.pool_key.as_ref() {
                let cleanup = self
                    .release_pool_scope(pool_key, &owned_id, None, false)
                    .await;
                if let Err(error) = cleanup {
                    let mut sessions = self
                        .sessions
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let previous = prior.take().expect("prior session disappeared");
                    sessions.insert(owned_id.clone(), previous);
                    let restored = sessions
                        .get(&owned_id)
                        .expect("prior session was not restored");
                    persist_session(restored)?;
                    return Err(error);
                }
            } else if let Some(transport) = previous.transport.as_ref() {
                transport.stop().await;
            }
            if let Some(task) = prior
                .as_mut()
                .and_then(|session| session.child_rollout_scan.take())
            {
                task.abort();
            }
        }
        Ok(connection)
    }

    /// Changes the durable checkout for one Codex session without replacing
    /// its generation or native session. A running adapter is detached only
    /// after the quiescent gate passes; the checkout marker and the resulting
    /// suspended session row are then committed in the same SQLite transaction.
    pub async fn change_checkout(
        &self,
        request: ChangeAgentConversationCheckoutRequest,
    ) -> Result<AgentConversationSessionRecord, String> {
        let owned_id = required_id(&request.owned_id, "Owned session id")?;
        let new_cwd = validated_conversation_cwd(&request.cwd)?
            .display()
            .to_string();
        let _lifecycle = self.lifecycle_guard(&owned_id).await?;
        let (
            runtime,
            transport,
            ordered_events,
            pool_key,
            native_session_id,
            child_rollout_scan,
            from_cwd,
        ) = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = lifecycle_session_mut(&mut sessions, &owned_id, request.generation)?;
            if session.provider != AgentConversationProvider::Codex {
                return Err("Only Codex sessions can change checkout".to_string());
            }
            if session.cwd == new_cwd {
                return Err("Session is already using this checkout".to_string());
            }
            if !session_can_change_checkout(session) {
                return Err(
                    "Checkout changes are available only while the Codex session is quiescent"
                        .to_string(),
                );
            }
            let from_cwd = session.cwd.clone();
            if session.runtime.is_some() {
                session.suspending = true;
            }
            (
                session.runtime.take(),
                session.transport.take(),
                session.ordered_events.take(),
                session.pool_key.take(),
                session.native_session_id.clone(),
                session.child_rollout_scan.take(),
                from_cwd,
            )
        };

        let had_runtime = runtime.is_some();
        if let Some(runtime) = runtime.as_ref() {
            let detach_result = if let Some(pool_key) = &pool_key {
                self.release_pool_scope(
                    pool_key,
                    &owned_id,
                    native_session_id.as_deref(),
                    false,
                )
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
                if let Ok(session) =
                    lifecycle_session_mut(&mut sessions, &owned_id, request.generation)
                {
                    session.suspending = false;
                    session.runtime = Some(Arc::clone(runtime));
                    session.transport = transport;
                    session.ordered_events = ordered_events;
                    session.pool_key = pool_key;
                    session.child_rollout_scan = child_rollout_scan;
                }
                return Err(error);
            }
        }
        if let Some(task) = child_rollout_scan {
            task.abort();
        }

        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = lifecycle_session_mut(&mut sessions, &owned_id, request.generation)?;
        if had_runtime && (!session.suspending || session.state != AgentRuntimeState::Ready) {
            session.suspending = false;
            session.state = AgentRuntimeState::Suspended;
            session.connection.state = ConversationConnectionState::Disconnected;
            session.owner = AgentExecutionOwner::Stopped;
            session.writer_lease.owner = AgentWriterLeaseOwner::None;
            session.native_session_mode = AgentNativeSessionMode::Resume;
            let _ = persist_session(session);
            return Err("Conversation lifecycle changed while checkout was preparing".to_string());
        }
        let previous_worktree = session.rail_meta.worktree.clone();
        session.suspending = false;
        session.cwd = new_cwd.clone();
        session.rail_meta.worktree = Some(new_cwd.clone());
        let payload = AgentConversationPayload::CheckoutChanged {
            from_cwd: from_cwd.clone(),
            to_cwd: new_cwd,
        };
        let event = if had_runtime {
            record_payload_for_session_and_dispatch_with_lifecycle(
                session,
                &self.emitter,
                payload,
                SessionLifecycleUpdate {
                    state: AgentRuntimeState::Suspended,
                    connection_state: ConversationConnectionState::Disconnected,
                    owner: AgentExecutionOwner::Stopped,
                    writer_owner: AgentWriterLeaseOwner::None,
                    native_session_mode: Some(AgentNativeSessionMode::Resume),
                },
            )
        } else {
            record_payload_for_session_and_dispatch(session, &self.emitter, payload)
        };
        if let Err(error) = event {
            session.cwd = from_cwd;
            session.rail_meta.worktree = previous_worktree;
            if had_runtime {
                session.state = AgentRuntimeState::Suspended;
                session.connection.state = ConversationConnectionState::Disconnected;
                session.owner = AgentExecutionOwner::Stopped;
                session.writer_lease.owner = AgentWriterLeaseOwner::None;
                session.native_session_mode = AgentNativeSessionMode::Resume;
                let _ = persist_session(session);
            }
            return Err(error);
        }
        drop(sessions);
        self.list_sessions()?
            .into_iter()
            .find(|session| session.owned_id == owned_id)
            .ok_or_else(|| "Conversation session disappeared after checkout change".to_string())
    }

    fn ensure_inner(
        &self,
        request: EnsureAgentConversationRequest,
    ) -> Result<(AgentConversationConnection, Option<ManagedAgentSession>), String> {
        let owned_id = required_id(&request.owned_id, "Owned session id")?;
        let cwd = validated_conversation_cwd(&request.cwd)?
            .display()
            .to_string();
        let native_session_id = normalized_optional_id(request.native_session_id);
        let reasoning_effort = normalized_optional_id(request.reasoning_effort);
        if request.native_session_mode == AgentNativeSessionMode::Load
            && native_session_id.is_none()
        {
            return Err("A native session id is required to load a stopped session".to_string());
        }
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // A session that has never run is not answered by its own record.
        //
        // Generation zero belongs to one thing only: a conversation imported
        // from a transcript, which has a stored row and an entry here so it can
        // be listed, but has never started an adapter and so has never been
        // told what models it offers, what effort levels it takes, or anything
        // else that arrives with the handshake. Handing that record back leaves
        // a session nothing can be chosen for. It falls through instead and is
        // started once, the same as any first run.
        //
        // A suspended session is different and keeps the behaviour it had: it
        // ran before, its capabilities are on disk, and its adapter is started
        // again only when a turn actually needs one.
        if let Some(current) = sessions.get(&owned_id) {
            if current.generation > 0
                && current.provider == request.provider
                && current.cwd == cwd
                && native_session_id
                    .as_ref()
                    .is_none_or(|native_id| current.native_session_id.as_ref() == Some(native_id))
                && current.native_session_mode == request.native_session_mode
                // An effort asked for here is matched against the one the
                // session is on now, not the one it was created with: a change
                // made while it was running has already written the first, and
                // the next send carries that value back here. Comparing it
                // against the spawn value read the same session as a different
                // one and threw its record away. A request that names no effort
                // asks for nothing and cannot disagree.
                && reasoning_effort.as_ref().is_none_or(|asked_for| {
                    current
                        .config
                        .reasoning_effort
                        .as_ref()
                        .or(current.spawn_reasoning_effort.as_ref())
                        == Some(asked_for)
                })
                && current.connection.state != ConversationConnectionState::Failed
            {
                return Ok((current.connection.clone(), None));
            }
        }
        let prior = sessions.remove(&owned_id);
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
            // Claude's efforts are a fixed list on this side, so the composer can
            // offer them before the adapter has ever run. Without this the picker
            // has nothing to show until the first turn, and the choice that is
            // supposed to be made before the first turn cannot be made at all.
            config: if request.provider == AgentConversationProvider::Claude {
                claude_session_config(
                    AgentConversationConfigState::default(),
                    reasoning_effort.as_deref(),
                )
            } else {
                AgentConversationConfigState::default()
            },
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
                adapter_offers_effort: false,
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
                // A replacement generation is the same conversation, so it keeps
                // the name it was given and where that name came from. Starting
                // these empty wrote a blank over the stored row.
                rail_meta: prior
                    .as_ref()
                    .map(|session| session.rail_meta.clone())
                    .unwrap_or_default(),
                title_source: prior
                    .as_ref()
                    .and_then(|session| session.title_source.clone()),
                suspending: false,
            },
        );
        let session = sessions
            .get(&connection.owned_id)
            .ok_or_else(|| "Conversation session was not inserted".to_string())?;
        persist_session(session)?;
        Ok((connection, prior))
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
            requested_model,
            requested_approval_policy,
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
                // What the session is on now, which a live change has already
                // written. Only a session that has never reported an effort
                // falls back to the one it was created with.
                session
                    .config
                    .reasoning_effort
                    .clone()
                    .or_else(|| session.spawn_reasoning_effort.clone()),
                session.config.model.clone(),
                // The policy the session is on now, which a live change has
                // already written. A restarted adapter comes back on its own
                // default, so the policy has to be asked for again.
                session.config.approval_policy.clone(),
                session.state == AgentRuntimeState::Suspended,
            )
        };
        let mut expected_native_session_id = native_session_id.clone();
        let never_had_a_turn = self
            .store
            .first_user_message_payload(owned_id)
            .map_err(|error| error.to_string())?
            .is_none();
        // A stored native session that cannot be resumed is started fresh when
        // there is nothing to lose by it: this store never recorded a turn for
        // it, or the provider no longer holds one. Asked only once a resume has
        // failed, because the second answer means reading the provider's files.
        let can_start_fresh = || {
            native_session_id
                .as_deref()
                .is_some_and(|native_session_id| {
                    never_had_a_turn || !provider_holds_session(provider, native_session_id)
                })
        };
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
                    if attempted.is_err() && can_start_fresh() {
                        crate::debug_log::stderr_log!(
                            "{owned_id}: stored native session holds nothing to resume; starting fresh"
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
                let environment = session_spawn_environment(provider);
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
                let (started, started_fresh) = if attempted.is_err() && can_start_fresh() {
                    crate::debug_log::stderr_log!(
                        "{owned_id}: stored native session holds nothing to resume; starting fresh"
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
        // The model and effort chosen for this conversation are asked of the
        // adapter now that it has a session, and its answer is what the session
        // reports from here on. Only what differs is sent: a session that
        // already starts on the wanted settings needs no call, and a request
        // the adapter resolves to something else must not be reported as if it
        // had been honoured.
        let requested = AgentConversationConfigUpdate {
            model: requested_model
                .filter(|model| Some(model.as_str()) != started.config.model.as_deref()),
            reasoning_effort: reasoning_effort.clone().filter(|effort| {
                Some(effort.as_str()) != started.config.reasoning_effort.as_deref()
            }),
            approval_policy: requested_approval_policy.filter(|policy| {
                Some(policy.as_str()) != started.config.approval_policy.as_deref()
            }),
        };
        if requested != AgentConversationConfigUpdate::default() {
            match runtime
                .lock()
                .await
                .set_conversation_config_on(&started.native_session_id, &requested)
                .await
            {
                Ok(config) => started.config = config,
                Err(error) => crate::debug_log::stderr_log!(
                    "{owned_id}: the settings chosen for this session were not applied: {error}"
                ),
            }
        }
        // An adapter that named an effort control owns the effort from now on.
        let adapter_offers_effort = !started.config.available_efforts.is_empty();
        if provider == AgentConversationProvider::Claude {
            started.config = claude_session_config(started.config, reasoning_effort.as_deref());
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
        session.adapter_offers_effort = adapter_offers_effort;
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

    /// Sends one message: the adapter is started if it is not running, the
    /// choices that came with the message are applied, and the prompt goes out.
    ///
    /// A message that is refused before its prompt — a model or approval
    /// policy the provider does not offer — leaves the session at rest. The
    /// adapter that activation just started has no turn to run, and nothing
    /// else would ever stop it: the teardown that keeps process trees from
    /// lingering runs when a turn ends, and this turn never began.
    pub async fn send_message(
        &self,
        owned_id: &str,
        generation: u64,
        input: AgentPrompt,
        model: Option<String>,
        approval_policy: Option<String>,
    ) -> Result<(), String> {
        // Activation, configuration, and durable prompt acceptance are one
        // lifecycle operation. Checkout switching uses this same guard, so it
        // cannot detach the runtime between any of those steps. The guard is
        // released as soon as `prompt` records the active turn; streaming keeps
        // running independently.
        let lifecycle = self.lifecycle_guard(owned_id).await?;
        let sent = async {
            self.activate_locked(owned_id, generation).await?;
            if model.is_some() || approval_policy.is_some() {
                self.apply_conversation_config(
                    SetAgentConversationConfigRequest {
                        owned_id: owned_id.to_string(),
                        generation,
                        model,
                        reasoning_effort: None,
                        approval_policy,
                    },
                    false,
                )
                .await?;
            }
            self.prompt(owned_id, generation, input).await
        }
        .await;
        drop(lifecycle);
        if sent.is_err() {
            if let Err(error) = self.suspend_if_quiescent(owned_id, generation).await {
                crate::debug_log::stderr_log!(
                    "{owned_id}: could not stop the adapter after a refused send: {error}"
                );
            }
        }
        sent
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
                    session.title_source = Some(TITLE_SOURCE_PROMPT.to_string());
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
                    attachment_ids: input.attachment_ids.clone(),
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
        let lifecycle = self.lifecycle_guard(owned_id).await?;
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
            let native_session_id = session
                .native_session_id
                .clone()
                .ok_or_else(|| "Structured provider session has not started".to_string())?;
            session.prompt_once_active = true;
            native_session_id
        };
        let result = runtime
            .lock()
            .await
            .prompt_once_on(&native_session_id, input)
            .await;
        {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Ok(session) = current_session_mut(&mut sessions, owned_id, generation) {
                session.prompt_once_active = false;
            }
        }
        drop(lifecycle);
        if let Err(error) = self.suspend_if_quiescent(owned_id, generation).await {
            crate::debug_log::stderr_log!(
                "{owned_id}: could not stop the adapter after one-shot generation: {error}"
            );
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
        let owned_id = request.owned_id.clone();
        let generation = request.generation;
        let config = self.apply_conversation_config(request, true).await?;
        // An accepted change ends the same way a refused one does: the adapter
        // was started only to be asked, no turn is running, and the teardown
        // that keeps process trees from lingering runs when a turn ends. A
        // send applies its settings through apply_conversation_config instead,
        // because its prompt is about to need the adapter it just started. A
        // session that is mid-turn is left alone — quiescence is the guard.
        if let Err(suspend_error) = self.suspend_if_quiescent(&owned_id, generation).await {
            crate::debug_log::stderr_log!(
                "{owned_id}: could not stop the adapter after a settings change: {suspend_error}"
            );
        }
        Ok(config)
    }

    async fn apply_conversation_config(
        &self,
        request: SetAgentConversationConfigRequest,
        suspend_on_error: bool,
    ) -> Result<AgentConversationConfigState, String> {
        let update = AgentConversationConfigUpdate {
            model: normalized_optional_id(request.model),
            reasoning_effort: normalized_optional_id(request.reasoning_effort),
            approval_policy: normalized_optional_id(request.approval_policy),
        };
        let native_session_id = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = lifecycle_session_mut(&mut sessions, &request.owned_id, request.generation)?;
            let started = session.native_session_id.is_some();
            // An adapter that named no effort control has no way to be
            // re-pointed: it took its effort when its session was created and
            // only a new session can carry a different one.
            if started
                && session.provider == AgentConversationProvider::Claude
                && update.reasoning_effort.is_some()
                && !session.adapter_offers_effort
            {
                return Err(
                    "Effort is set when the session starts. Start a new session to change it."
                        .to_string(),
                );
            }
            if update == AgentConversationConfigUpdate::default() {
                return Ok(session.config.clone());
            }
            match session.native_session_id.clone() {
                Some(native_session_id) => native_session_id,
                None => {
                    // The adapter only starts when the first turn is sent, so a
                    // choice made now has no session to carry it. Keep it on the
                    // record instead: the composer reads it back, and the start
                    // asks the adapter for it when there is finally one.
                    let effort = update.reasoning_effort.clone();
                    if let Some(model) = update.model.clone() {
                        session.config.model = Some(model);
                    }
                    if let Some(effort) = effort {
                        session.config.reasoning_effort = Some(effort.clone());
                        session.spawn_reasoning_effort = Some(effort);
                    }
                    if let Some(approval_policy) = update.approval_policy.clone() {
                        session.config.approval_policy = Some(approval_policy);
                    }
                    session.connection.config = session.config.clone();
                    persist_session(session)?;
                    return Ok(session.config.clone());
                }
            }
        };
        let runtime = self
            .runtime_or_activate(&request.owned_id, request.generation)
            .await?;
        let applied = async {
            // What a session offers is the running adapter's answer, and the
            // activation above has just refreshed it. Judging the request any
            // earlier weighs it against the snapshot an adapter that is no
            // longer running left behind, which refuses a choice the live one
            // offers.
            {
                let sessions = self
                    .sessions
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let session = current_session(&sessions, &request.owned_id, request.generation)?;
                validate_conversation_config_update(&session.config, &update)?;
            }
            runtime
                .lock()
                .await
                .set_conversation_config_on(&native_session_id, &update)
                .await
                .map_err(|error| error.to_string())
        }
        .await;
        // Asking the adapter meant starting one, and a refused change gives it
        // no turn to run: the teardown that keeps process trees from lingering
        // runs when a turn ends, and this session never began one.
        let config = match applied {
            Ok(config) => config,
            Err(error) => {
                if suspend_on_error {
                    if let Err(suspend_error) = self
                        .suspend_if_quiescent(&request.owned_id, request.generation)
                        .await
                    {
                        let owned_id = &request.owned_id;
                        crate::debug_log::stderr_log!(
                            "{owned_id}: could not stop the adapter after a refused settings change: {suspend_error}"
                        );
                    }
                }
                return Err(error);
            }
        };
        let config = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session =
                lifecycle_session_mut(&mut sessions, &request.owned_id, request.generation)?;
            // Claude's efforts are ours, not the provider's: standard ACP has no
            // such field, so the reply to any config change comes back with the
            // list empty. Assigning it whole erased the efforts every time the
            // model was changed, and the picker then offered only the current one.
            let config = if session.provider == AgentConversationProvider::Claude {
                claude_session_config(config, session.spawn_reasoning_effort.as_deref())
            } else {
                config
            };
            session.config = config.clone();
            session.connection.config = config.clone();
            persist_session(session)?;
            config
        };
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
        // A conversation that is stored can be read, whether or not anything is
        // running it. The live overlay is the runtime's business and it comes
        // and goes — ensuring a session takes it out of the map before putting
        // the replacement in, and a session whose project folder has since been
        // deleted never gets the replacement at all. Reporting nothing there
        // left a resumed transcript that was on disk, complete, and invisible,
        // until the next launch rebuilt the overlay and it appeared.
        let missing = {
            let sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            !sessions.contains_key(owned_id)
        };
        if missing {
            if self
                .store
                .get_session(owned_id)
                .map_err(|error| error.to_string())?
                .is_none()
            {
                return Ok(None);
            }
            self.restore_overlay_from_store(owned_id)?;
        }
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
        from_sequence: i64,
    ) -> Result<Vec<AgentConversationEvent>, String> {
        self.store
            .list_events(owned_id, from_sequence, CATCH_UP_EVENT_CAP)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(stored_event)
            .collect()
    }

    /// The page of transcript events just older than `before_sequence`.
    ///
    /// Opening a conversation ships one screen of history; scrolling up calls
    /// this for the previous page until `has_more` says there is nothing older.
    pub fn list_events_before(
        &self,
        owned_id: &str,
        before_sequence: i64,
        max_bytes: u32,
    ) -> Result<AgentConversationEventPage, String> {
        let page = self
            .store
            .list_events_before(owned_id, before_sequence, max_bytes)
            .map_err(|error| error.to_string())?;
        let events = page
            .events
            .into_iter()
            .map(stored_event)
            .collect::<Result<Vec<AgentConversationEvent>, String>>()?;
        Ok(AgentConversationEventPage {
            events,
            has_more: page.has_more,
        })
    }

    /// The page of transcript events just newer than `after_sequence`.
    pub fn list_events_after(
        &self,
        owned_id: &str,
        after_sequence: i64,
        max_bytes: u32,
    ) -> Result<AgentConversationEventPage, String> {
        let page = self
            .store
            .list_events_after(owned_id, after_sequence, max_bytes)
            .map_err(|error| error.to_string())?;
        let events = page
            .events
            .into_iter()
            .map(stored_event)
            .collect::<Result<Vec<AgentConversationEvent>, String>>()?;
        Ok(AgentConversationEventPage {
            events,
            has_more: page.has_more,
        })
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
        self.store
            .list_recent_events(owned_id, SNAPSHOT_WINDOW_BYTES)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(stored_event)
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
            // The rail row this comes from is a copy that a fresh session has
            // not filled in yet, and it is saved right after the first send.
            // Taking its empty model or effort wrote a blank over the choice
            // every time.
            if request.model.is_some() {
                session.config.model = request.model;
            }
            if request.effort.is_some() {
                session.spawn_reasoning_effort.clone_from(&request.effort);
                session.config.reasoning_effort.clone_from(&request.effort);
            }
            // Setting the value without the list left the menu showing exactly
            // one option: the picker falls back to the current value when the
            // available list is empty, so a session started with "medium" could
            // only ever offer "medium".
            if session.provider == AgentConversationProvider::Claude {
                session.config = claude_session_config(
                    session.config.clone(),
                    session.spawn_reasoning_effort.as_deref(),
                );
            }
            session.connection.config = session.config.clone();
            // A name typed here is the person's own, and nothing overwrites it
            // afterwards. Everything else this request carries — model, effort,
            // the rail's own copy of the row — arrives with the name unchanged,
            // so only a different one counts as a rename.
            let renamed = request
                .meta
                .title
                .as_deref()
                .is_some_and(|title| !title.trim().is_empty())
                && request.meta.title != session.rail_meta.title;
            session.rail_meta = request.meta;
            if renamed {
                session.title_source = Some(TITLE_SOURCE_USER.to_string());
            }
            persist_session(session)?;
        }
        self.list_sessions()?
            .into_iter()
            .find(|session| session.owned_id == owned_id)
            .ok_or_else(|| "Conversation session was not found after metadata update".to_string())
    }

    /// Replaces the provisional name of a session with one the helper model
    /// writes from the first exchange, in the background.
    ///
    /// The provisional name is the first line the person typed, which is rarely
    /// what the session turns out to be about. Once a turn has finished there
    /// is enough to summarise, so the helper is asked for a short one. Nothing
    /// waits on the answer: a send has already finished by the time this runs,
    /// and a helper that is off, slow, or unreachable simply leaves the name
    /// alone. A name the person typed is never touched, and neither is one the
    /// helper has already written, so this happens once per session.
    fn name_session_after_turn(&self, owned_id: &str, generation: u64, turn_id: &str) {
        let namer = {
            let namer = self
                .namer
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Some(namer) = namer.as_ref() else {
                return;
            };
            Arc::clone(namer)
        };
        let store = {
            let sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Ok(session) = current_session(&sessions, owned_id, generation) else {
                return;
            };
            if !title_can_be_replaced(session.title_source.as_deref()) {
                return;
            }
            Arc::clone(&session.store)
        };
        let sessions = Arc::clone(&self.sessions);
        let renamed_listener = Arc::clone(&self.renamed_listener);
        let owned_id = owned_id.to_string();
        let turn_id = turn_id.to_string();
        // The helper's transport blocks, and so do the two reads that gather
        // what it is given, so neither runs on the thread carrying the session's
        // events. This is the last thing a finished turn does, and the pump
        // moves on without it.
        tokio::task::spawn_blocking(move || {
            let Some(input) = title_input(&store, &owned_id, &turn_id) else {
                return;
            };
            let answer = match namer(&input) {
                Ok(answer) => answer,
                // No key stored means the helper is off, which is a choice
                // rather than a fault: the session keeps its prompt name.
                Err(crate::helper::HelperError::NoKey) => return,
                Err(error) => {
                    crate::debug_log::stderr_log!(
                        "Could not name the session: {}",
                        error.sentence()
                    );
                    return;
                }
            };
            let Some(title) = helper_title(&answer) else {
                return;
            };
            let mut sessions = sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Ok(session) = current_session_mut(&mut sessions, &owned_id, generation) else {
                return;
            };
            // The person may have renamed the session while the helper was
            // being asked, and their name wins.
            if !title_can_be_replaced(session.title_source.as_deref()) {
                return;
            }
            session.rail_meta.title = Some(title.clone());
            session.title_source = Some(TITLE_SOURCE_HELPER.to_string());
            if let Err(error) = persist_session(session) {
                crate::debug_log::stderr_log!("Could not save the session name: {error}");
                return;
            }
            drop(sessions);
            let listener = renamed_listener
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            if let Some(listener) = listener {
                listener(&owned_id, &title);
            }
        });
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

    /// Start this conversation's adapter, keep what its handshake answered, and
    /// stop the process again.
    ///
    /// A conversation imported from a past transcript has never run, so nothing
    /// has told it which models it offers, which effort levels it takes or which
    /// approval policies it understands. Those answers arrive only with an
    /// adapter's handshake, and the adapter is otherwise started by a turn —
    /// which is why a resumed conversation opened saying its agent had no
    /// settings. Asking once, here, is what fills the composer in.
    ///
    /// The process is STOPPED afterwards rather than suspended, and the
    /// difference is the whole reason an earlier attempt at this was taken out
    /// again: suspending detaches the session and keeps the adapter warm for the
    /// next turn, so one was left running per resume, hundreds of megabytes
    /// each. What activation learnt is on disk by then, so nothing is lost by
    /// stopping. The session is left Suspended, which is the state a session
    /// between turns is already in, and the next send resumes it the ordinary
    /// way.
    pub async fn warm_conversation_config(
        &self,
        owned_id: &str,
        generation: u64,
    ) -> Result<AgentConversationConfigState, String> {
        let _lifecycle = self.lifecycle_guard(owned_id).await?;
        self.activate_locked(owned_id, generation).await?;
        let (runtime, transport, ordered_events, pool_key, native_session_id) = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = lifecycle_session_mut(&mut sessions, owned_id, generation)?;
            // A send can reach `prompt` while this is running — that path takes
            // the sessions lock but not the lifecycle guard — and stopping an
            // adapter mid-turn leaves the turn id set against a process that has
            // been killed, which refuses every later send as "already has an
            // active turn". Someone talking to the conversation outranks warming
            // it: the answers are already stored, so leaving the adapter up is
            // the harmless half of the choice. Quiescence rather than
            // `session_can_suspend`, whose `capabilities.session.resume`
            // requirement would skip the very providers this exists for.
            if !session_is_quiescent(session) {
                return Ok(session.config.clone());
            }
            let Some(runtime) = session.runtime.take() else {
                // Activation started nothing, so there is nothing to stop and
                // whatever is known is already stored.
                return Ok(session.config.clone());
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

        // Both branches stop the process — `close` and `detach` alike end in
        // `transport.stop()` — and they differ only in whether `session/close`
        // is sent first. Warming asks for neither: the session has to stay
        // resumable, so closing it would be wrong even where it works, and it
        // does not always work. Claude's adapter answers `session/close` with
        // "Method not found" (-32601), which turned a successful warm into a
        // failed one. The leak this design guards against was never detach; it
        // was `suspend_if_quiescent` bailing before any teardown at all.
        let stop_result = if let Some(pool_key) = &pool_key {
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
        if let Err(error) = stop_result {
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
        // The same race again, on the other side of the stop: writing Suspended
        // over a session that has meanwhile started a turn would claim a state
        // that is not true. The adapter is already gone by here, so this reports
        // rather than pretends.
        if !session.suspending || session.state != AgentRuntimeState::Ready {
            session.suspending = false;
            return Err(
                "Conversation lifecycle changed while its settings were being read".to_string(),
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
        Ok(session.config.clone())
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

    /// Take a conversation out of the store for good.
    ///
    /// Stops it first if anything is running it, then drops the row — and with
    /// it, through the store's cascades, its events, draft, annotations and
    /// attachment records — and forgets it here, so nothing rebuilds it at the
    /// next launch. The provider's own transcript on disk is not touched.
    /// Answers whether there was anything to delete.
    pub async fn delete(&self, owned_id: &str) -> Result<bool, String> {
        let owned_id = required_id(owned_id, "Owned session id")?;
        let live = {
            let sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            sessions
                .get(&owned_id)
                .map(|session| (session.generation, session.state != AgentRuntimeState::Closed))
        };
        let Some((generation, open)) = live else {
            return Ok(false);
        };
        if open {
            self.close(&owned_id, generation).await?;
        }
        let _lifecycle = self.lifecycle_guard(&owned_id).await?;
        self.sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&owned_id);
        self.store
            .delete_session(&owned_id)
            .map_err(|error| error.to_string())?;
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

/// The stored metadata an imported conversation starts life with.
///
/// An import writes a session row without starting an adapter, so it has no
/// generation, no capabilities and no configuration of its own to record. It
/// cannot leave them out: every stored row is decoded together at launch, and
/// one row missing any of these fields fails that decode and takes the whole
/// launch with it. So an import writes what a session holds before its first
/// run — nothing is live, and the fields are all present.
pub(super) fn imported_session_extra(
    provider: AgentConversationProvider,
) -> Result<String, String> {
    serde_json::to_string(&StoredSessionExtra {
        generation: 0,
        native_session_mode: AgentNativeSessionMode::Resume,
        owner: AgentExecutionOwner::Stopped,
        config: AgentConversationConfigState::default(),
        capabilities: empty_capabilities(provider),
        rail_meta: AgentConversationSessionMeta::default(),
    })
    .map_err(|error| format!("Could not encode imported session metadata: {error}"))
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
        title_source: session.title_source.clone(),
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
        let owned_id = row.owned_id.clone();
        let session = recovered_session_from_row(store, row)?;
        persist_session(&session)?;
        sessions.insert(owned_id, session);
    }
    Ok(sessions)
}

/// Rebuilds what a stored session needs in memory, without starting anything.
///
/// `list_sessions` reads the stored rows and the live overlay together, and a
/// row with no overlay fails the whole list rather than only itself. Recovery
/// builds one of these for every row at launch; anything that writes a row
/// while the app is running has to build one too, or the next listing breaks
/// on the row it just created.
fn recovered_session_from_row(
    store: &Arc<SessionStore>,
    row: SessionRow,
) -> Result<ManagedAgentSession, String> {
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
                serde_json::from_str::<AgentConversationEvent>(&payload)
                    .map_err(|error| format!("Could not decode stored first user message: {error}"))
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
        .saturating_add(1);
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
        adapter_offers_effort: !stored.config.available_efforts.is_empty(),
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
        title_source: row.title_source,
        suspending: false,
    };
    Ok(session)
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

/// Whether a name may still be replaced by one the helper writes. A name the
/// person typed is theirs to keep, and one the helper has already written is
/// the name of the session.
fn title_can_be_replaced(source: Option<&str>) -> bool {
    !matches!(source, Some(TITLE_SOURCE_USER) | Some(TITLE_SOURCE_HELPER))
}

/// Reads the helper's answer as a name. It is asked for bare words and usually
/// gives them, but a model that wraps a name in quotation marks anyway must not
/// put those on the rail.
fn helper_title(answer: &str) -> Option<String> {
    let trimmed = answer
        .trim()
        .trim_matches(|character| matches!(character, '"' | '\'' | '\u{201c}' | '\u{201d}'))
        .trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(SESSION_TITLE_CHAR_CAP).collect())
}

/// What the helper is given to name a session by: what the person first asked
/// for, and what came back in the turn that just finished. On a live session
/// the reply is only ever stored as its deltas, so they are joined back
/// together here; a session read in from elsewhere holds whole messages, and
/// both are read the same way.
fn title_input(store: &SessionStore, owned_id: &str, turn_id: &str) -> Option<String> {
    let prompt = store
        .first_user_message_payload(owned_id)
        .ok()
        .flatten()
        .and_then(|payload| serde_json::from_str::<AgentConversationEvent>(&payload).ok())
        .and_then(|event| match event.payload {
            AgentConversationPayload::UserMessage { text, .. } => Some(text),
            _ => None,
        })?;
    let mut reply = String::new();
    for row in store
        .list_recent_events(owned_id, SNAPSHOT_WINDOW_BYTES)
        .ok()?
    {
        if row.turn_id.as_deref() != Some(turn_id) || reply.chars().count() >= TITLE_INPUT_CHAR_CAP
        {
            continue;
        }
        let Ok(event) = serde_json::from_str::<AgentConversationEvent>(&row.payload_json) else {
            continue;
        };
        match event.payload {
            AgentConversationPayload::AssistantDelta { delta, .. } => reply.push_str(&delta),
            AgentConversationPayload::AssistantMessage { text, .. } => reply.push_str(&text),
            _ => {}
        }
    }
    let prompt: String = prompt.chars().take(TITLE_INPUT_CHAR_CAP).collect();
    let reply: String = reply.chars().take(TITLE_INPUT_CHAR_CAP).collect();
    Some(format!("{prompt}\n\n{reply}"))
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

/// Whether the provider still holds anything for this session to resume.
///
/// Only Claude keeps a transcript this side can read. A Claude session whose
/// file holds no turn — its process was stopped before it wrote one — is gone
/// on Claude's side however many turns this store shows for it, and every
/// resume of it fails the same way. Other providers are taken at their word,
/// as is a transcript that cannot be read at all.
fn provider_holds_session(provider: AgentConversationProvider, native_session_id: &str) -> bool {
    match provider {
        AgentConversationProvider::Claude => {
            transcript::claude_transcript_holds_a_turn(native_session_id).unwrap_or(true)
        }
        AgentConversationProvider::Codex | AgentConversationProvider::Antigravity => true,
    }
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
    let result = if matches!(&payload, AgentConversationPayload::CheckoutChanged { .. }) {
        session
            .store
            .upsert_session_with_event_and_clear_workspace(&row, Some(&event_row))
    } else {
        session
            .store
            .upsert_session_with_event(&row, Some(&event_row))
    };
    result.map_err(|error| error.to_string())?;
    candidate.apply(session);
    Ok(frontend_event)
}

/// Reads one stored row back as the event it holds.
///
/// The row's `seq` column is the authority on where the event sits in the
/// journal; the copy inside the payload is only a copy, and it has been wrong.
/// Importing older history writes descending sequences, and a build that could
/// not represent a negative one wrote zero into every payload it touched. Those
/// rows are still in the database. Taking the position from the column repairs
/// them as they are read, and keeps the two from ever disagreeing again.
fn stored_event(row: EventRow) -> Result<AgentConversationEvent, String> {
    let mut event: AgentConversationEvent = serde_json::from_str(&row.payload_json)
        .map_err(|error| format!("Could not decode stored conversation event: {error}"))?;
    event.sequence = row.seq;
    Ok(event)
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
                let runtime_error = raw_update_failed(&params);
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
                if let Err(error) = manager.sync_broker_status(&owned_id, generation, runtime_error)
                {
                    crate::debug_log::stderr_log!("Could not publish broker status: {error}");
                }
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

fn raw_update_failed(params: &Value) -> bool {
    let update = params.get("update").unwrap_or(params);
    update
        .get("status")
        .or_else(|| update.get("state"))
        .and_then(Value::as_str)
        == Some("failed")
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
            let runtime_error = error_details.is_some();
            // The same reading of the result that decides the turn payload
            // below. A turn that was interrupted or that failed has nothing
            // worth naming a session after.
            let turn_completed = result.is_ok() && !cancelled;

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
            if turn_completed {
                manager.name_session_after_turn(owned_id, generation, &turn_id);
            }
            // A finished turn stops the adapter process as soon as no prompt,
            // approval, input request, or tool work remains. The stored native
            // session survives for resume on the next send; only the idle
            // process tree is removed because those trees otherwise retain
            // hundreds of megabytes between turns.
            if let Err(error) = manager.suspend_if_quiescent(owned_id, generation).await {
                crate::debug_log::stderr_log!("Could not tear down quiescent runtime: {error}");
            }
            if let Err(error) = manager.sync_broker_status(owned_id, generation, runtime_error) {
                crate::debug_log::stderr_log!("Could not publish broker status: {error}");
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
    sequence: i64,
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
        AgentConversationPayload::ContextCompaction { .. } => AgentEventType::ItemCompleted,
        AgentConversationPayload::CheckoutChanged { .. } => AgentEventType::RuntimeWarning,
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
    // What this number answers is "how full is the context window", so the last
    // request's total comes first. `total_token_usage` is every turn of the
    // session added together, cached reads included — it passes the window
    // inside an afternoon and reads as 100% full for the rest of the session.
    // It stays as a fallback for a report that carries nothing else.
    let used_tokens = first_u64(
        update,
        &[
            &["used"],
            &["usedTokens"],
            &["used_tokens"],
            &["totalTokens"],
            &["total_tokens"],
            &["info", "last_token_usage", "total_tokens"],
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
        "context_compaction" | "context-compaction" => {
            Some(AgentConversationPayload::ContextCompaction {
                trigger: update
                    .get("trigger")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                pre_tokens: first_u64(update, &[&["preTokens"], &["pre_tokens"]]),
                post_tokens: first_u64(update, &[&["postTokens"], &["post_tokens"]]),
            })
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
            attachment_ids: Vec::new(),
        }),
        "agent_thought_chunk" => {
            crate::debug_log::stderr_log!("Ignoring ACP agent thought update");
            None
        }
        "tool_call" => {
            let details = tool_details(update);
            Some(AgentConversationPayload::Tool {
                item_id: tool_item_id(update, turn_id),
                name: update
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("Tool")
                    .to_string(),
                state: ToolState::Started,
                summary: details.summary,
                output: details.output,
                path: details.path,
                diff: details.diff,
            })
        }
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
            let details = tool_details(update);
            Some(AgentConversationPayload::Tool {
                item_id: tool_item_id(update, turn_id),
                name: update
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("Tool")
                    .to_string(),
                state,
                summary: details.summary,
                output: details.output,
                path: details.path,
                diff: details.diff,
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
    let latest_activity = tool_details(update)
        .output
        .or_else(|| {
            update
                .get("title")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| label.clone())
        .map(|text: String| text.chars().take(160).collect());
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
    // A background agent writes its id into the body of the result, under the
    // line that says it started. Reading only the first line loses it, and the
    // child is then filed under the tool call rather than under the agent.
    let activity = tool_details(update).output;
    let async_child_id = activity.as_deref().and_then(async_agent_id);
    let is_async = async_child_id.is_some();
    let terminal = !is_async
        && update_kind == "tool_call_update"
        && matches!(
            update.get("status").and_then(Value::as_str),
            Some("completed" | "failed" | "cancelled" | "canceled" | "stopped" | "done")
        );
    let child_id = async_child_id.unwrap_or_else(|| tool_call_id.clone());
    let label = label.or_else(|| is_async.then(|| format!("Background agent {child_id}")));
    let latest_activity = activity
        .or_else(|| label.clone())
        .map(|text: String| text.chars().take(160).collect());

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

/// The parts of an ACP tool call a transcript row can draw.
#[derive(Debug, Default)]
struct ToolDetails {
    /// The one line a collapsed row shows.
    summary: Option<String>,
    /// What the call produced, kept whole.
    output: Option<String>,
    /// The file the call was about.
    path: Option<String>,
    /// A unified diff, when the call changed a file.
    diff: Option<String>,
}

/// Reads an ACP tool call into the parts a row draws.
///
/// This used to join `content` and `locations` into one string and keep only
/// that. The row then had a summary line and nothing else, so its expander
/// opened onto nothing: no command output, no file, no change. The structure is
/// in the update; it was being thrown away on the way to the database.
fn tool_details(update: &Value) -> ToolDetails {
    let mut output = Vec::new();
    let mut diff = Vec::new();
    let mut path = None;

    match update.get("content") {
        Some(Value::Array(blocks)) => {
            for block in blocks {
                if block.get("type").and_then(Value::as_str) == Some("diff") {
                    let block_path = block.get("path").and_then(Value::as_str);
                    if path.is_none() {
                        path = block_path.map(str::to_owned);
                    }
                    let patch = unified_diff(
                        block.get("oldText").and_then(Value::as_str).unwrap_or_default(),
                        block.get("newText").and_then(Value::as_str).unwrap_or_default(),
                    );
                    if !patch.is_empty() {
                        diff.push(patch);
                    }
                } else if let Some(text) = text_from_value(block) {
                    match patch_in_text(&text) {
                        Some(patch) => {
                            if path.is_none() {
                                path = patch_target_path(&text);
                            }
                            diff.push(patch);
                        }
                        None => output.push(text),
                    }
                }
            }
        }
        Some(content) => {
            if let Some(text) = text_from_value(content) {
                match patch_in_text(&text) {
                    Some(patch) => {
                        if path.is_none() {
                            path = patch_target_path(&text);
                        }
                        diff.push(patch);
                    }
                    None => output.push(text),
                }
            }
        }
        None => {}
    }

    if path.is_none() {
        path = update.get("locations").and_then(text_from_value);
    }

    let output = (!output.is_empty()).then(|| output.join("\n"));
    // The preview is the first line of what came back, which for a shell call
    // is the command itself. The body is everything.
    let summary = output
        .as_deref()
        .and_then(|text| text.lines().find(|line| !line.trim().is_empty()))
        .map(str::to_owned)
        .or_else(|| path.clone());

    ToolDetails {
        summary,
        output,
        path,
        diff: (!diff.is_empty()).then(|| diff.join("\n")),
    }
}

/// The patch inside a tool's text output, when the text is one.
///
/// Not every provider sends a file change the way the protocol describes it.
/// Codex's apply-file-changes call hands over a whole `git diff` as ordinary
/// text, which used to be drawn as tool output: a wall of raw `+` and `-`
/// lines with no gutters, no colour and no fold, in the middle of a
/// conversation. It is a diff, so it should be read as one.
///
/// Only the hunks are kept. The `diff --git`, `index` and `---`/`+++` lines
/// name the file, which the row already shows in its own header, and passing
/// them on would draw them as context lines inside the change.
fn patch_in_text(text: &str) -> Option<String> {
    let mut lines = text.lines();
    let first = lines.find(|line| !line.trim().is_empty())?;
    let named = first.starts_with("diff --git ");
    // A bare patch with no `diff --git` header still has to be a whole one:
    // both file lines and at least one hunk, or any prose quoting a `---` rule
    // would be swallowed.
    let bare = first.starts_with("--- ")
        && text.lines().any(|line| line.starts_with("+++ "))
        && text.lines().any(|line| line.starts_with("@@ "));
    if !named && !bare {
        return None;
    }
    let hunks: Vec<&str> = text
        .lines()
        .skip_while(|line| !line.starts_with("@@ "))
        .collect();
    (!hunks.is_empty()).then(|| hunks.join("\n"))
}

/// The file a text patch changes, read from its header.
///
/// The `+++` side names it, except for a deletion, where that side is
/// `/dev/null` and the `---` side is the file that went away. Both carry
/// git's `a/` and `b/` prefixes.
fn patch_target_path(text: &str) -> Option<String> {
    let named = |line: &str, marker: &str| -> Option<String> {
        let rest = line.strip_prefix(marker)?.trim();
        if rest == "/dev/null" {
            return None;
        }
        let rest = rest.split('\t').next().unwrap_or(rest);
        Some(
            rest.strip_prefix("a/")
                .or_else(|| rest.strip_prefix("b/"))
                .unwrap_or(rest)
                .to_owned(),
        )
    };
    text.lines()
        .find_map(|line| named(line, "+++ "))
        .or_else(|| text.lines().find_map(|line| named(line, "--- ")))
}

/// A unified diff of one whole-file replacement.
///
/// ACP hands over the file before and after rather than a patch, so the change
/// has to be found. Matching lines at each end are common ground and are left
/// out; what remains is the edit, with a hunk header carrying the line it
/// starts at. That is what the file-change row reads.
fn unified_diff(old_text: &str, new_text: &str) -> String {
    let old: Vec<&str> = old_text.split('\n').collect();
    let new: Vec<&str> = new_text.split('\n').collect();
    let mut prefix = 0;
    while prefix < old.len() && prefix < new.len() && old[prefix] == new[prefix] {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < old.len() - prefix
        && suffix < new.len() - prefix
        && old[old.len() - 1 - suffix] == new[new.len() - 1 - suffix]
    {
        suffix += 1;
    }
    let removed = &old[prefix..old.len() - suffix];
    let added = &new[prefix..new.len() - suffix];
    if removed.is_empty() && added.is_empty() {
        return String::new();
    }
    let mut patch = format!(
        "@@ -{},{} +{},{} @@",
        prefix + 1,
        removed.len(),
        prefix + 1,
        added.len()
    );
    for line in removed {
        patch.push_str("\n-");
        patch.push_str(line);
    }
    for line in added {
        patch.push_str("\n+");
        patch.push_str(line);
    }
    patch
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
        AgentConversationProvider::Antigravity => "antigravity",
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

fn session_can_change_checkout(session: &ManagedAgentSession) -> bool {
    matches!(
        session.owner,
        AgentExecutionOwner::Structured | AgentExecutionOwner::Stopped
    ) && !session.suspending
        && session.native_session_id.is_some()
        && session.capabilities.session.resume
        && (session.runtime.is_some() || session.state == AgentRuntimeState::Suspended)
        && session_is_checkout_quiescent(session)
}

fn session_is_checkout_quiescent(session: &ManagedAgentSession) -> bool {
    matches!(
        session.state,
        AgentRuntimeState::Ready | AgentRuntimeState::Suspended
    ) && session.active_turn_id.is_none()
        && !session.prompt_once_active
        && session.permission_requests.is_empty()
        && session.user_input_requests.is_empty()
        && session.writer_lease_transition.is_none()
        && session.live_tool_calls.is_empty()
        && session.background_work.is_empty()
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

/// How hard a session thinks is the adapter's own setting, asked for through
/// its effort control. An inherited thinking budget would speak over it, so the
/// adapter is started without one.
fn session_spawn_environment(provider: AgentConversationProvider) -> SidecarEnvironment {
    if provider != AgentConversationProvider::Claude {
        return SidecarEnvironment::default();
    }
    SidecarEnvironment::default().remove(MAX_THINKING_TOKENS_ENV)
}

/// Fills in the effort a Claude conversation shows while it has no adapter to
/// ask. An adapter that reports an effort control of its own has already said
/// what the session is on, and that answer is left alone.
fn claude_session_config(
    mut config: AgentConversationConfigState,
    reasoning_effort: Option<&str>,
) -> AgentConversationConfigState {
    if !config.available_efforts.is_empty() {
        return config;
    }
    config.reasoning_effort = reasoning_effort.map(str::to_string);
    config.available_efforts = CLAUDE_SESSION_EFFORTS
        .iter()
        .map(|effort| (*effort).to_string())
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

    /// A provider that hands over a whole `git diff` as text still gets a
    /// file-change row rather than a wall of raw patch lines.
    #[test]
    fn a_patch_sent_as_text_is_read_as_a_change() {
        let patch = "diff --git a/src/app.ts b/src/app.ts\n\
                     index 1111111..2222222 100644\n\
                     --- a/src/app.ts\n\
                     +++ b/src/app.ts\n\
                     @@ -1,2 +1,2 @@\n\
                     -let total = 1;\n\
                     +let total = 2;\n\
                      export { total };";
        let details = tool_details(&json!({
            "content": [{ "type": "content", "content": { "type": "text", "text": patch } }]
        }));
        let diff = details.diff.expect("the patch should be read as a change");
        assert!(diff.starts_with("@@ -1,2 +1,2 @@"), "hunks only, got: {diff}");
        assert!(!diff.contains("diff --git"), "the git header should be dropped");
        assert!(!diff.contains("index 1111111"), "the index line should be dropped");
        assert_eq!(details.path.as_deref(), Some("src/app.ts"));
        assert!(details.output.is_none(), "a patch is not also tool output");
    }

    /// A deletion names its file on the side that is not `/dev/null`.
    #[test]
    fn a_deletion_is_named_by_the_file_that_went_away() {
        let patch = "diff --git a/old.txt b/old.txt\n\
                     --- a/old.txt\n\
                     +++ /dev/null\n\
                     @@ -1 +0,0 @@\n\
                     -gone";
        let details = tool_details(&json!({
            "content": [{ "type": "content", "content": { "type": "text", "text": patch } }]
        }));
        assert_eq!(details.path.as_deref(), Some("old.txt"));
    }

    /// Ordinary output stays output. Prose that happens to quote a rule of
    /// dashes, or a command that printed one, must not become a file change.
    #[test]
    fn ordinary_output_is_not_mistaken_for_a_patch() {
        for text in [
            "Building...\n--- done ---\nok",
            "--- a/only-a-header",
            "@@ a hunk with no file lines @@",
            "",
        ] {
            let details = tool_details(&json!({
                "content": [{ "type": "content", "content": { "type": "text", "text": text } }]
            }));
            assert!(details.diff.is_none(), "{text:?} should not read as a change");
        }
    }

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

    /// The stored name of a fixture session, read back the way the rail reads it.
    fn stored_title(fixture: &FixtureManager) -> Option<String> {
        fixture
            .manager
            .store()
            .get_session(&fixture.owned_id)
            .expect("read the session row")
            .and_then(|row| row.title)
    }

    fn stored_title_source(fixture: &FixtureManager) -> Option<String> {
        fixture
            .manager
            .store()
            .get_session(&fixture.owned_id)
            .expect("read the session row")
            .and_then(|row| row.title_source)
    }

    /// A namer that answers from memory instead of from a helper model, and
    /// counts how many times it was asked.
    fn counting_namer(calls: &Arc<Mutex<usize>>, answer: &'static str) -> SessionNamer {
        let calls = Arc::clone(calls);
        Arc::new(move |_input: &str| {
            *calls.lock().unwrap() += 1;
            Ok(answer.to_string())
        })
    }

    fn renamed_meta(title: &str) -> AgentConversationSessionMeta {
        AgentConversationSessionMeta {
            title: Some(title.to_string()),
            ..AgentConversationSessionMeta::default()
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn a_helper_title_replaces_the_prompt_title_once() {
        let fixture = fixture_manager_with_acp_session("helper_title").await;
        let calls: Arc<Mutex<usize>> = Default::default();
        fixture
            .manager
            .set_session_namer(counting_namer(&calls, "Fix rail titles"));
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        fixture
            .manager
            .set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));

        fixture
            .manager
            .prompt(
                &fixture.owned_id,
                fixture.generation,
                test_prompt("make the rail titles readable"),
            )
            .await
            .expect("prompt starts");
        wait_until(|| stored_title(&fixture).as_deref() == Some("Fix rail titles")).await;

        assert_eq!(stored_title_source(&fixture).as_deref(), Some("helper"));
        assert_eq!(*calls.lock().unwrap(), 1);

        fixture
            .manager
            .prompt(
                &fixture.owned_id,
                fixture.generation,
                test_prompt("and one more thing"),
            )
            .await
            .expect("second prompt starts");
        wait_until(|| completed_turns(&seen) == 2).await;

        assert_eq!(*calls.lock().unwrap(), 1);
        assert_eq!(stored_title(&fixture).as_deref(), Some("Fix rail titles"));
        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    /// The rail saves a new session's row back within seconds of the send,
    /// carrying the same provisional name the prompt path has just written.
    /// That is not a rename, and reading it as one would stop the session from
    /// ever being given a better name after its first turn.
    #[tokio::test(flavor = "current_thread")]
    async fn saving_the_row_back_unchanged_is_not_a_rename() {
        let fixture = fixture_manager_with_acp_session("saved_back_title").await;
        let prompt = "n".repeat(100);

        fixture
            .manager
            .prompt(&fixture.owned_id, fixture.generation, test_prompt(&prompt))
            .await
            .expect("prompt starts");
        wait_until(|| stored_title(&fixture).is_some()).await;

        let provisional = stored_title(&fixture).expect("the prompt names the session");
        assert_eq!(provisional.chars().count(), SESSION_TITLE_CHAR_CAP);
        assert_eq!(stored_title_source(&fixture).as_deref(), Some("prompt"));

        fixture
            .manager
            .update_session_meta(UpdateAgentConversationSessionMetaRequest {
                owned_id: fixture.owned_id.clone(),
                model: None,
                effort: None,
                meta: renamed_meta(&provisional),
            })
            .expect("save the row back");

        assert_eq!(stored_title_source(&fixture).as_deref(), Some("prompt"));
        assert_eq!(stored_title(&fixture), Some(provisional));
        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn a_user_title_is_never_overwritten() {
        let fixture = fixture_manager_with_acp_session("user_title").await;
        let calls: Arc<Mutex<usize>> = Default::default();
        fixture
            .manager
            .set_session_namer(counting_namer(&calls, "Fix rail titles"));
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        fixture
            .manager
            .set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));
        fixture
            .manager
            .update_session_meta(UpdateAgentConversationSessionMetaRequest {
                owned_id: fixture.owned_id.clone(),
                model: None,
                effort: None,
                meta: renamed_meta("Rail work"),
            })
            .expect("rename the session");

        fixture
            .manager
            .prompt(
                &fixture.owned_id,
                fixture.generation,
                test_prompt("make the rail titles readable"),
            )
            .await
            .expect("prompt starts");
        wait_until(|| completed_turns(&seen) == 1).await;

        assert_eq!(stored_title(&fixture).as_deref(), Some("Rail work"));
        assert_eq!(stored_title_source(&fixture).as_deref(), Some("user"));
        assert_eq!(*calls.lock().unwrap(), 0);
        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    fn completed_turns(seen: &Arc<Mutex<Vec<AgentConversationEvent>>>) -> usize {
        seen.lock()
            .unwrap()
            .iter()
            .filter(|event| {
                matches!(
                    event.payload,
                    AgentConversationPayload::Turn {
                        state: super::super::protocol::TurnState::Completed,
                        ..
                    }
                )
            })
            .count()
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
            attachment_ids: Vec::new(),
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
            AgentConversationPayload::ContextCompaction { .. } => "contextCompaction",
            AgentConversationPayload::CheckoutChanged { .. } => "checkoutChanged",
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
                attachment_ids: Vec::new(),
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
                attachment_ids: Vec::new(),
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
                Some(AgentConversationPayload::Tool {
                    state,
                    summary,
                    output,
                    path,
                    ..
                }) => {
                    assert_eq!(state, expected);
                    // The row's one line and the body it opens onto are
                    // separate now, and the file is a field rather than a
                    // sentence appended to one.
                    assert_eq!(summary.as_deref(), Some("detail"));
                    assert_eq!(output.as_deref(), Some("detail"));
                    assert_eq!(path.as_deref(), Some("src/main.rs"));
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
        // How full the window is, not how much the session has spent getting
        // here. A report carrying both used to hand over the running total,
        // which passes the window early in a day's work and pins the meter at
        // full for everything after it.
        let session_totals = json!({ "update": {
            "sessionUpdate": "token_count",
            "info": {
                "model_context_window": 237_500,
                "total_token_usage": { "total_tokens": 15_908_466 },
                "last_token_usage": { "total_tokens": 41_233 }
            }
        }});
        assert_eq!(
            payload_from_session_update_for_turn(&session_totals, None),
            Some(AgentConversationPayload::Usage {
                input_tokens: None,
                output_tokens: None,
                used_tokens: Some(41_233),
                context_window: Some(237_500),
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
        // The bridge reports a compaction outright, because nothing else in a
        // Codex session shows one: no occupancy is reported at all, so the drop
        // that gives a compaction away elsewhere cannot be seen here.
        let compaction = json!({ "update": {
            "sessionUpdate": "context_compaction", "trigger": "auto",
            "preTokens": 351_238, "postTokens": 22_202
        }});
        assert_eq!(
            payload_from_session_update_for_turn(&compaction, None),
            Some(AgentConversationPayload::ContextCompaction {
                trigger: Some("auto".into()),
                pre_tokens: Some(351_238),
                post_tokens: Some(22_202),
            })
        );
        assert_eq!(
            payload_from_session_update_for_turn(
                &json!({ "update": { "sessionUpdate": "context_compaction" } }),
                None
            ),
            Some(AgentConversationPayload::ContextCompaction {
                trigger: None,
                pre_tokens: None,
                post_tokens: None,
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

    /// Antigravity's adapter fixes its working folder when its process starts
    /// and never reads the folder named in `session/new`, so every session has
    /// to keep a process of its own. It advertises no multi-session support,
    /// and the app pools sessions onto one process only when a provider does.
    /// Pinning the two together keeps anyone from turning pooling on later
    /// without also teaching the adapter about per-session folders — every
    /// pooled session would silently run in the first one's folder.
    #[tokio::test(flavor = "current_thread")]
    async fn antigravity_sessions_keep_a_process_each() {
        let fixture =
            fixture_manager_with_provider("agy", AgentConversationProvider::Antigravity, None)
                .await;
        let capabilities = fixture
            .manager
            .capabilities(&fixture.owned_id, fixture.generation)
            .expect("capabilities");
        assert!(!provider_is_multi_session_safe(&capabilities));
        fs::remove_dir_all(fixture.root).unwrap();
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

    /// A conversation keeps every event it was given.
    ///
    /// The journal used to be trimmed to its newest ten thousand on every
    /// write, which was reasonable when opening a conversation handed over all
    /// of them. Once a window became a bounded read that trim only did harm: it
    /// deleted the reading a person had just scrolled back to fetch, and it did
    /// it on their next message.
    #[test]
    fn a_conversation_keeps_every_event_it_was_given() {
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
            for index in 0..=CATCH_UP_EVENT_CAP {
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
        let written = i64::from(CATCH_UP_EVENT_CAP) + 1;
        assert_eq!(
            manager.store.latest_seq(&connection.owned_id).unwrap(),
            written,
            "nothing older is dropped to make room"
        );
        let events = manager
            .store
            .list_events(&connection.owned_id, 0, CATCH_UP_EVENT_CAP)
            .unwrap();
        // A catch-up read is bounded, but it starts at the first event there
        // ever was rather than at whatever survived a trim.
        assert_eq!(events.len(), CATCH_UP_EVENT_CAP as usize);
        assert_eq!(events.first().unwrap().seq, 1);
        let snapshot = manager.snapshot(&connection.owned_id).unwrap().unwrap();
        // Opening hands over the newest reading, bounded by bytes rather than
        // by a count of events, and ending on the newest event there is.
        assert!(
            snapshot.events.len() < events.len(),
            "a session larger than the window is not handed over whole"
        );
        assert_eq!(snapshot.events.last().unwrap().sequence, written);
        assert!(snapshot
            .events
            .windows(2)
            .all(|pair| pair[1].sequence == pair[0].sequence + 1));
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
                    attachment_ids: Vec::new(),
                },
            )
            .unwrap();
            record_payload_for_session(
                session,
                AgentConversationPayload::UserMessage {
                    item_id: "second-user".into(),
                    text: "Later title".into(),
                    completed: true,
                    attachment_ids: Vec::new(),
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

    // The rail row is saved right after the first send, before it has copied
    // the model in. Saving it must not blank the model the session was given.
    #[test]
    fn saving_the_rail_row_without_a_model_keeps_the_one_chosen() {
        let root = temp_root();
        let log = root.join("keep-model.jsonl");
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
                "owned-keep-model",
                AgentConversationProvider::Codex,
            ))
            .unwrap();
        let row = |model: Option<&str>| UpdateAgentConversationSessionMetaRequest {
            owned_id: connection.owned_id.clone(),
            model: model.map(str::to_owned),
            effort: None,
            meta: AgentConversationSessionMeta::default(),
        };
        manager
            .update_session_meta(row(Some("gpt-5.6-luna")))
            .unwrap();

        let saved = manager.update_session_meta(row(None)).unwrap();

        assert_eq!(saved.model.as_deref(), Some("gpt-5.6-luna"));
        assert_eq!(
            manager.list_sessions().unwrap()[0].model.as_deref(),
            Some("gpt-5.6-luna")
        );
        fs::remove_dir_all(root).unwrap();
    }

    // Removing a row on the rail used to remove nothing here, and the next
    // launch rebuilt the rail from this store — so every removed session came
    // back. Deleting takes the row, and everything hanging off it, out for good.
    #[tokio::test(flavor = "current_thread")]
    async fn deleting_a_conversation_takes_it_out_of_the_store() {
        let root = temp_root();
        let log = root.join("delete-session.jsonl");
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
                "owned-to-delete",
                AgentConversationProvider::Codex,
            ))
            .unwrap();
        {
            let mut sessions = manager.sessions.lock().unwrap();
            let session = sessions.get_mut(&connection.owned_id).unwrap();
            record_payload_for_session(
                session,
                AgentConversationPayload::UserMessage {
                    item_id: "message-to-delete".into(),
                    text: "stored".into(),
                    completed: true,
                    attachment_ids: Vec::new(),
                },
            )
            .unwrap();
        }
        assert_eq!(manager.list_sessions().unwrap().len(), 1);

        assert!(manager.delete(&connection.owned_id).await.unwrap());

        assert!(manager.list_sessions().unwrap().is_empty());
        assert!(manager
            .list_events(&connection.owned_id, 0)
            .unwrap()
            .is_empty());
        assert!(manager.snapshot(&connection.owned_id).unwrap().is_none());
        // Gone is gone: asking again is not an error, just nothing to do.
        assert!(!manager.delete(&connection.owned_id).await.unwrap());
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
                    attachment_ids: Vec::new(),
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
                    attachment_ids: Vec::new(),
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

    // Claude keeps a transcript this side can read. When it holds no turn — the
    // process was stopped before it wrote one — there is nothing on Claude's
    // side to resume however many turns this store shows, and every resume of
    // it fails the same way. Starting fresh is the only thing that works.
    #[tokio::test(flavor = "current_thread")]
    async fn resume_failure_of_a_claude_session_whose_transcript_holds_nothing_starts_fresh() {
        let root = temp_root();
        let log = root.join("resume-failure-claude.jsonl");
        let manifest = super::super::providers::acp_client::tests::fixture_manifest_named(
            &log,
            "resume_failure",
        );
        let providers = ProviderRegistry::new([(AgentConversationProvider::Claude, manifest)])
            .expect("fixture provider");
        let manager = AgentRuntimeManager::new(providers);
        let mut ensure = request(
            root.to_str().unwrap(),
            "owned-resume-failure-claude",
            AgentConversationProvider::Claude,
        );
        // No transcript by this name exists anywhere under ~/.claude/projects.
        ensure.native_session_id = Some("mcb-test-transcript-never-written".into());
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
                    attachment_ids: Vec::new(),
                },
            )
            .unwrap();
        }

        let resumed = manager
            .activate(&connection.owned_id, connection.generation)
            .await
            .expect("a session Claude no longer holds starts fresh");

        assert_eq!(resumed.native_session_id.as_deref(), Some("new-session"));
        let snapshot = manager.snapshot(&connection.owned_id).unwrap().unwrap();
        assert!(!snapshot.suspended);
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
    async fn changing_the_model_does_not_empty_claude_efforts() {
        // Standard ACP has no effort field, so the reply to a model change comes
        // back with the effort list empty. Assigning that whole erased the
        // efforts, and the picker then offered only whichever one was already
        // set — the list appeared to shrink each time anything was chosen.
        let root = temp_root();
        let log = root.join("standard_config.jsonl");
        let manifest = super::super::providers::acp_client::tests::fixture_manifest_named(
            &log,
            "standard_config",
        );
        let providers = ProviderRegistry::new([(AgentConversationProvider::Claude, manifest)])
            .expect("fixture provider");
        let manager = AgentRuntimeManager::new(providers);
        let owned_id = "owned-standard_config".to_string();
        let mut ensure_request = request(
            root.to_str().unwrap(),
            &owned_id,
            AgentConversationProvider::Claude,
        );
        ensure_request.reasoning_effort = Some("medium".to_string());
        let connection = manager.ensure_inner(ensure_request).expect("ensure").0;
        manager
            .activate(&owned_id, connection.generation)
            .await
            .expect("activate");

        let before = manager.conversation_config(&owned_id).expect("config");
        assert_eq!(before.available_efforts, ["low", "medium", "high", "max"]);
        assert_eq!(before.reasoning_effort.as_deref(), Some("medium"));

        let after = manager
            .set_conversation_config(SetAgentConversationConfigRequest {
                owned_id: owned_id.clone(),
                generation: connection.generation,
                model: Some("sonnet".to_string()),
                reasoning_effort: None,
                approval_policy: None,
            })
            .await
            .expect("changing the model is allowed on a live Claude session");
        assert_eq!(after.model.as_deref(), Some("sonnet"));
        assert_eq!(
            after.available_efforts,
            ["low", "medium", "high", "max"],
            "changing the model must not empty the effort list"
        );
        assert_eq!(after.reasoning_effort.as_deref(), Some("medium"));

        manager.close(&owned_id, connection.generation).await.unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn choices_made_before_the_adapter_starts_are_asked_of_it_when_it_starts() {
        // A session picks its model and effort before anything is running: the
        // adapter only starts when the first turn is sent. The choice has to
        // survive that gap and be asked of the adapter once there is one, or
        // the composer reverts it and the turn runs on the provider's default.
        let root = temp_root();
        let log = root.join("claude_preselect.jsonl");
        let manifest =
            super::super::providers::acp_client::tests::fixture_manifest_named(&log, "claude_preselect");
        let providers = ProviderRegistry::new([(AgentConversationProvider::Claude, manifest)])
            .expect("fixture provider");
        let manager = AgentRuntimeManager::new(providers);
        let owned_id = "owned-claude_preselect".to_string();
        let ensure_request = request(
            root.to_str().unwrap(),
            &owned_id,
            AgentConversationProvider::Claude,
        );
        let connection = manager.ensure_inner(ensure_request).expect("ensure").0;
        // The picker has to be able to offer efforts before the first turn,
        // which is the only time Claude's effort can be chosen at all.
        assert_eq!(
            connection.config.available_efforts,
            ["low", "medium", "high", "max"]
        );

        let config = manager
            .set_conversation_config(SetAgentConversationConfigRequest {
                owned_id: owned_id.clone(),
                generation: connection.generation,
                model: Some("gpt-5.6-luna".to_string()),
                reasoning_effort: Some("high".to_string()),
                approval_policy: None,
            })
            .await
            .expect("a choice made before the adapter starts is kept");
        assert_eq!(config.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(config.model.as_deref(), Some("gpt-5.6-luna"));

        manager
            .activate(&owned_id, connection.generation)
            .await
            .expect("activate");
        let spawn_log = fs::read_to_string(&log).expect("Claude spawn log");
        assert!(
            spawn_log.contains(r#""model":"gpt-5.6-luna""#),
            "the model chosen before the start must reach the adapter: {spawn_log}"
        );
        assert!(
            spawn_log.contains("MAX_THINKING_TOKENS=<unset>"),
            "nothing in the environment decides how hard a session thinks: {spawn_log}"
        );
        // The adapter answers with the session it actually has, and that answer
        // is what the conversation reports. This fixture resolves the request to
        // a different model, and the different model is what is shown.
        let after_start = manager.conversation_config(&owned_id).expect("config");
        assert_eq!(after_start.model.as_deref(), Some("gpt-5.6-terra"));
        assert_eq!(after_start.reasoning_effort.as_deref(), Some("xhigh"));

        manager.close(&owned_id, connection.generation).await.unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    /// The adapter resolves what it is asked for, so the model a session ends
    /// up on and the model that was requested can differ. What the session
    /// reports afterwards is the adapter's answer, never the request.
    #[tokio::test(flavor = "current_thread")]
    async fn acp_new_reports_the_model_the_adapter_chose_not_the_one_requested() {
        let root = temp_root();
        let log = root.join("config_options.jsonl");
        let manifest = super::super::providers::acp_client::tests::fixture_manifest_named(
            &log,
            "config_options",
        );
        let providers = ProviderRegistry::new([(AgentConversationProvider::Claude, manifest)])
            .expect("fixture provider");
        let manager = AgentRuntimeManager::new(providers);
        let owned_id = "owned-config_options_model".to_string();
        let ensure_request = request(
            root.to_str().unwrap(),
            &owned_id,
            AgentConversationProvider::Claude,
        );
        let connection = manager.ensure_inner(ensure_request).expect("ensure").0;

        // Picked before anything is running, which is when a new conversation's
        // model is chosen.
        manager
            .set_conversation_config(SetAgentConversationConfigRequest {
                owned_id: owned_id.clone(),
                generation: connection.generation,
                model: Some("haiku".into()),
                reasoning_effort: None,
                approval_policy: None,
            })
            .await
            .expect("a model chosen before the adapter starts is kept");

        manager
            .activate(&owned_id, connection.generation)
            .await
            .expect("activate");

        let frames = fs::read_to_string(&log).expect("fixture request log");
        assert!(
            frames.contains(r#""configId":"model","value":"haiku""#),
            "the chosen model must be asked of the adapter: {frames}"
        );
        // The fixture's session starts on one model, is asked for "haiku", and
        // answers "sonnet". The answer is what the session is on.
        let config = manager.conversation_config(&owned_id).expect("config");
        assert_eq!(config.model.as_deref(), Some("sonnet"));

        manager.close(&owned_id, connection.generation).await.unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    /// An adapter that offers an effort control can be re-pointed at any time:
    /// the change goes to the adapter, the session reports what the adapter
    /// says it is now, and nothing in the process environment has a say.
    #[tokio::test(flavor = "current_thread")]
    async fn effort_changes_live_when_the_adapter_offers_it() {
        let fixture = fixture_manager_with_provider(
            "config_options",
            AgentConversationProvider::Claude,
            None,
        )
        .await;

        let configured = fixture
            .manager
            .set_conversation_config(SetAgentConversationConfigRequest {
                owned_id: fixture.owned_id.clone(),
                generation: fixture.generation,
                model: None,
                reasoning_effort: Some("high".into()),
                approval_policy: None,
            })
            .await
            .expect("an adapter with an effort control takes a live change");
        assert_eq!(configured.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(
            fixture
                .manager
                .conversation_config(&fixture.owned_id)
                .expect("stored config")
                .reasoning_effort
                .as_deref(),
            Some("high")
        );

        let frames = fs::read_to_string(fixture.root.join("config_options.jsonl"))
            .expect("fixture request log");
        assert!(
            frames.contains(r#""configId":"effort","value":"high""#),
            "the effort change must go to the adapter: {frames}"
        );
        assert!(
            frames.contains("MAX_THINKING_TOKENS=<unset>"),
            "the thinking budget must not be decided by the environment: {frames}"
        );

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn effort_chosen_for_a_new_session_is_asked_of_the_adapter() {
        let fixture = fixture_manager_with_provider(
            "config_options",
            AgentConversationProvider::Claude,
            Some("medium"),
        )
        .await;

        let log = fs::read_to_string(fixture.root.join("config_options.jsonl"))
            .expect("Claude spawn log");
        assert!(
            log.contains(r#""configId":"effort","value":"medium""#),
            "the effort the session was started with must be asked of the adapter: {log}"
        );
        assert!(
            log.contains("MAX_THINKING_TOKENS=<unset>"),
            "nothing in the environment decides how hard a session thinks: {log}"
        );
        let config = fixture
            .manager
            .conversation_config(&fixture.owned_id)
            .expect("Claude config");
        // The adapter answers "high", and the answer is what the session is on.
        assert_eq!(config.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(
            config.available_efforts,
            ["default", "low", "medium", "high", "xhigh", "max"]
        );

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    /// An effort chosen while a session is running has to survive the adapter
    /// going away and coming back. Adapters stop when a turn ends, so the next
    /// message reactivates one, and reactivation asks for the effort the
    /// session is on now — not the one it was created with.
    #[tokio::test(flavor = "current_thread")]
    async fn reactivating_a_session_asks_for_the_effort_it_is_on_now() {
        let fixture = fixture_manager_with_provider(
            "suspend_effort_reactivation",
            AgentConversationProvider::Codex,
            Some("medium"),
        )
        .await;

        // The adapter answers the start-time effort with one of its own, so the
        // session is on "xhigh" while it was created with "medium".
        fixture
            .manager
            .set_conversation_config(SetAgentConversationConfigRequest {
                owned_id: fixture.owned_id.clone(),
                generation: fixture.generation,
                model: None,
                reasoning_effort: Some("high".into()),
                approval_policy: None,
            })
            .await
            .expect("a live effort change");
        assert_eq!(
            fixture
                .manager
                .conversation_config(&fixture.owned_id)
                .expect("config")
                .reasoning_effort
                .as_deref(),
            Some("xhigh")
        );

        // The settings change already put the session to rest; asking again
        // is a no-op. What matters is the state, not who got there first.
        fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .expect("suspend");
        assert!(fixture
            .manager
            .session_is_suspended(&fixture.owned_id, fixture.generation));
        fixture
            .manager
            .activate(&fixture.owned_id, fixture.generation)
            .await
            .expect("reactivate");

        let log = fs::read_to_string(fixture.root.join("suspend_effort_reactivation.jsonl"))
            .expect("fixture request log");
        assert!(
            log.contains(r#""reasoningEffort":"xhigh""#),
            "reactivation must ask for the effort the session is on now: {log}"
        );

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    /// A session's record — its name, where that name came from, the model it
    /// is on — belongs to the session, not to the effort it was started with.
    /// Every turn ends with the adapter stopped, so the next send ensures the
    /// session again and sends the effort it is on now. Reading that as a
    /// request for a different session threw the record away and started over.
    #[tokio::test(flavor = "current_thread")]
    async fn ensure_after_a_live_effort_change_keeps_the_session_record() {
        let fixture = fixture_manager_with_provider(
            "suspend_live_effort_ensure",
            AgentConversationProvider::Codex,
            Some("high"),
        )
        .await;
        let seen: Arc<Mutex<Vec<AgentConversationEvent>>> = Default::default();
        let sink = Arc::clone(&seen);
        fixture
            .manager
            .set_emitter(Arc::new(move |event| sink.lock().unwrap().push(event)));

        fixture
            .manager
            .prompt(
                &fixture.owned_id,
                fixture.generation,
                test_prompt("give this session a name"),
            )
            .await
            .expect("prompt starts");
        wait_until(|| completed_turns(&seen) == 1).await;
        let title = stored_title(&fixture).expect("the prompt names the session");
        assert_eq!(stored_title_source(&fixture).as_deref(), Some("prompt"));

        // A live change lands on the effort the adapter answers with, which is
        // not the one the session was started with.
        fixture
            .manager
            .set_conversation_config(SetAgentConversationConfigRequest {
                owned_id: fixture.owned_id.clone(),
                generation: fixture.generation,
                model: None,
                reasoning_effort: Some("medium".into()),
                approval_policy: None,
            })
            .await
            .expect("a live effort change");
        let live = fixture
            .manager
            .conversation_config(&fixture.owned_id)
            .expect("config");
        assert_eq!(live.reasoning_effort.as_deref(), Some("xhigh"));
        assert_eq!(live.model.as_deref(), Some("gpt-5.6-terra"));

        // The settings change already put the session to rest; asking again
        // is a no-op. What matters is the state, not who got there first.
        fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .expect("suspend");
        assert!(fixture
            .manager
            .session_is_suspended(&fixture.owned_id, fixture.generation));

        // The next send ensures the session again, carrying the effort the
        // composer is showing: the live one, not the spawn one.
        let mut request = request(
            fixture.root.to_str().unwrap(),
            &fixture.owned_id,
            AgentConversationProvider::Codex,
        );
        request.reasoning_effort = Some("xhigh".into());
        let connection = fixture
            .manager
            .ensure_inner(request)
            .expect("ensure the suspended session")
            .0;

        assert_eq!(connection.generation, fixture.generation);
        assert_eq!(stored_title(&fixture).as_deref(), Some(title.as_str()));
        assert_eq!(stored_title_source(&fixture).as_deref(), Some("prompt"));
        let after = fixture
            .manager
            .conversation_config(&fixture.owned_id)
            .expect("config after ensure");
        assert_eq!(after.reasoning_effort.as_deref(), Some("xhigh"));
        assert_eq!(after.model.as_deref(), Some("gpt-5.6-terra"));

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    /// The rail saves a session's row back carrying its own copy of the fields,
    /// and a rail action that is not about the effort sends none. Writing that
    /// empty answer over the session wiped the effort it was on.
    #[tokio::test(flavor = "current_thread")]
    async fn saving_the_row_back_without_an_effort_leaves_the_effort() {
        let fixture = fixture_manager_with_provider(
            "meta_keeps_effort",
            AgentConversationProvider::Codex,
            Some("medium"),
        )
        .await;
        // The adapter answers the start-time effort with one of its own, so the
        // session is on "xhigh" while it was created with "medium".
        assert_eq!(
            fixture
                .manager
                .conversation_config(&fixture.owned_id)
                .expect("config")
                .reasoning_effort
                .as_deref(),
            Some("xhigh")
        );

        fixture
            .manager
            .update_session_meta(UpdateAgentConversationSessionMetaRequest {
                owned_id: fixture.owned_id.clone(),
                model: None,
                effort: None,
                meta: renamed_meta("Rail work"),
            })
            .expect("save the row back");

        assert_eq!(
            fixture
                .manager
                .conversation_config(&fixture.owned_id)
                .expect("config after the save")
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

    /// An approval policy chosen while a session is running has to survive the
    /// adapter going away and coming back, just as the effort does. Adapters
    /// stop when a turn ends, so reactivation must ask for the policy the
    /// session is on now rather than let the restarted adapter's default win.
    #[tokio::test(flavor = "current_thread")]
    async fn reactivating_a_session_keeps_the_approval_policy_it_is_on() {
        let fixture = fixture_manager_with_provider(
            "suspend_approval_reactivation",
            AgentConversationProvider::Codex,
            None,
        )
        .await;

        // The adapter answers the requested policy with one of its own, so the
        // session lands on "never" having been asked for "untrusted".
        fixture
            .manager
            .set_conversation_config(SetAgentConversationConfigRequest {
                owned_id: fixture.owned_id.clone(),
                generation: fixture.generation,
                model: None,
                reasoning_effort: None,
                approval_policy: Some("untrusted".into()),
            })
            .await
            .expect("a live approval policy change");
        assert_eq!(
            fixture
                .manager
                .conversation_config(&fixture.owned_id)
                .expect("config")
                .approval_policy
                .as_deref(),
            Some("never")
        );

        // The settings change already put the session to rest; asking again
        // is a no-op. What matters is the state, not who got there first.
        fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .expect("suspend");
        assert!(fixture
            .manager
            .session_is_suspended(&fixture.owned_id, fixture.generation));
        fixture
            .manager
            .activate(&fixture.owned_id, fixture.generation)
            .await
            .expect("reactivate");

        let log = fs::read_to_string(fixture.root.join("suspend_approval_reactivation.jsonl"))
            .expect("fixture request log");
        assert!(
            log.contains(r#""approvalPolicy":"never""#),
            "reactivation must ask for the approval policy the session is on now: {log}"
        );
        assert_eq!(
            fixture
                .manager
                .conversation_config(&fixture.owned_id)
                .expect("config")
                .approval_policy
                .as_deref(),
            Some("never")
        );

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    /// Which approval policies a session offers is the running adapter's
    /// answer, and a suspended session carries only the snapshot the adapter
    /// that last ran left behind. A policy the adapter installed now offers
    /// must not be refused on the strength of that stale list.
    #[tokio::test(flavor = "current_thread")]
    async fn a_policy_the_running_adapter_offers_is_not_refused_by_a_stale_list() {
        let fixture = fixture_manager_with_provider(
            "suspend_approval_stale_list",
            AgentConversationProvider::Codex,
            None,
        )
        .await;

        assert!(fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .expect("suspend"));
        {
            let mut sessions = fixture.manager.sessions.lock().unwrap();
            let session = sessions.get_mut(&fixture.owned_id).unwrap();
            session
                .config
                .available_approval_policies
                .retain(|policy| policy.as_str() != "never");
        }

        let config = fixture
            .manager
            .set_conversation_config(SetAgentConversationConfigRequest {
                owned_id: fixture.owned_id.clone(),
                generation: fixture.generation,
                model: None,
                reasoning_effort: None,
                approval_policy: Some("never".into()),
            })
            .await
            .expect("a policy the running adapter offers is refused by a stale list");
        assert_eq!(config.approval_policy.as_deref(), Some("never"));

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    /// Judging a settings change against the running adapter means an adapter
    /// is started to ask. When the change is then refused it has no turn to
    /// run and nothing else would ever stop it, so the refusal has to put the
    /// session back at rest itself.
    #[tokio::test(flavor = "current_thread")]
    async fn a_refused_config_change_does_not_leave_an_adapter_running() {
        let fixture = fixture_manager_with_acp_session("suspend_refused_config").await;
        assert!(fixture
            .manager
            .suspend_if_quiescent(&fixture.owned_id, fixture.generation)
            .await
            .expect("suspend"));

        let refused = fixture
            .manager
            .set_conversation_config(SetAgentConversationConfigRequest {
                owned_id: fixture.owned_id.clone(),
                generation: fixture.generation,
                model: None,
                reasoning_effort: None,
                approval_policy: Some("acceptedits".into()),
            })
            .await
            .expect_err("a policy the adapter does not offer is refused");
        assert!(
            refused.contains("approval policy"),
            "the refusal names what was wrong: {refused}"
        );
        assert!(
            fixture
                .manager
                .session_is_suspended(&fixture.owned_id, fixture.generation),
            "a settings change that was refused leaves the session at rest"
        );

        fs::remove_dir_all(fixture.root).unwrap();
    }

    /// Whether the adapter has an effort control is a fact about the adapter,
    /// not about this run of the app. A conversation read back from the store
    /// after a restart still has one, and can still be re-pointed.
    #[tokio::test(flavor = "current_thread")]
    async fn a_session_read_back_after_a_restart_can_still_change_its_effort() {
        let root = temp_root();
        let database = root.join("sessions.db");
        let log = root.join("recovered-effort.jsonl");
        let manifest = super::super::providers::acp_client::tests::fixture_manifest_named(
            &log,
            "suspend_recovered_effort",
        );
        let providers =
            ProviderRegistry::new([(AgentConversationProvider::Claude, manifest.clone())]).unwrap();
        let manager = AgentRuntimeManager::open(providers, &database).unwrap();
        let connection = manager
            .ensure_async(request(
                root.to_str().unwrap(),
                "owned-recovered-effort",
                AgentConversationProvider::Claude,
            ))
            .await
            .unwrap();
        manager
            .activate(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        drop(manager);

        let providers =
            ProviderRegistry::new([(AgentConversationProvider::Claude, manifest)]).unwrap();
        let recovered = AgentRuntimeManager::open(providers, &database).unwrap();
        let config = recovered
            .set_conversation_config(SetAgentConversationConfigRequest {
                owned_id: connection.owned_id.clone(),
                generation: connection.generation,
                model: None,
                reasoning_effort: Some("xhigh".into()),
                approval_policy: None,
            })
            .await
            .expect("an adapter that offers an effort control still offers one after a restart");
        assert_eq!(config.reasoning_effort.as_deref(), Some("xhigh"));

        recovered
            .close(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    /// Which efforts exist is the adapter's answer, so a new session may be
    /// started on any of them. A conversation left on an effort outside the
    /// list this app can offer before an adapter exists must still be able to
    /// start one.
    #[tokio::test(flavor = "current_thread")]
    async fn a_new_session_may_start_on_any_effort_the_adapter_offers() {
        let root = temp_root();
        let log = root.join("ensure-effort.jsonl");
        let manifest = super::super::providers::acp_client::tests::fixture_manifest_named(
            &log,
            "config_options",
        );
        let providers = ProviderRegistry::new([(AgentConversationProvider::Claude, manifest)])
            .expect("fixture provider");
        let manager = AgentRuntimeManager::new(providers);
        let mut ensure_request = request(
            root.to_str().unwrap(),
            "owned-ensure-effort",
            AgentConversationProvider::Claude,
        );
        ensure_request.reasoning_effort = Some("xhigh".to_string());

        let connection = manager
            .ensure_inner(ensure_request)
            .expect("an effort the adapter offers is not refused before it runs")
            .0;
        assert_eq!(connection.config.reasoning_effort.as_deref(), Some("xhigh"));

        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn an_adapter_with_no_effort_control_refuses_a_live_effort_change() {
        let fixture = fixture_manager_with_provider(
            "standard_config",
            AgentConversationProvider::Claude,
            None,
        )
        .await;

        let log = fs::read_to_string(fixture.root.join("standard_config.jsonl"))
            .expect("Claude spawn log");
        assert!(
            log.contains("MAX_THINKING_TOKENS=<unset>"),
            "nothing in the environment decides how hard a session thinks: {log}"
        );
        let config = fixture
            .manager
            .conversation_config(&fixture.owned_id)
            .expect("Claude config");
        // This adapter named no effort control, so the conversation still shows
        // the efforts it can offer before a session exists.
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
            .expect_err("an adapter with no effort control cannot be re-pointed");
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
    async fn config_change_on_a_suspended_session_leaves_no_process_behind() {
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
        assert!(snapshot.suspended);
        assert!(fixture.manager.resource_roots().is_empty());

        fixture
            .manager
            .close(&fixture.owned_id, fixture.generation)
            .await
            .unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    /// Both halves of the bargain, in one test: a resumed conversation learns
    /// what its agent offers, and no adapter is left running for a turn that is
    /// not coming. An earlier attempt at this suspended instead of stopping and
    /// left one process per resume behind, so the second assertion is the one
    /// that matters most.
    #[tokio::test(flavor = "current_thread")]
    async fn warming_a_conversation_keeps_its_answers_and_leaves_no_adapter() {
        let fixture = fixture_manager_with_acp_session("warm_conversation_config").await;

        // Activated up front so the adapter's pid can be taken while it is still
        // running. Warming activates too; finding the session already up is the
        // ordinary case, since a reader who has sent anything has one.
        fixture
            .manager
            .activate(&fixture.owned_id, fixture.generation)
            .await
            .expect("activate");
        let (runtime, pool_key) = {
            let sessions = fixture
                .manager
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = sessions.get(&fixture.owned_id).expect("activated session");
            (
                session.runtime.clone().expect("a running adapter"),
                session.pool_key.clone().expect("a pooled adapter"),
            )
        };
        let adapter_pid = runtime
            .lock()
            .await
            .process_id()
            .expect("the adapter runs in a process of its own");
        drop(runtime);

        let warmed = fixture
            .manager
            .warm_conversation_config(&fixture.owned_id, fixture.generation)
            .await
            .expect("warm config");
        assert!(
            !warmed.available_models.is_empty(),
            "warming has to come back with what the agent offers"
        );

        assert_eq!(
            fixture
                .manager
                .conversation_config(&fixture.owned_id)
                .expect("stored config"),
            warmed,
            "what warming learnt is what an ordinary read gives back"
        );

        assert!(
            fixture
                .manager
                .session_is_suspended(&fixture.owned_id, fixture.generation),
            "the adapter is stopped, not left warm for a turn nobody asked for"
        );

        // The assertion above is bookkeeping: it is equally true of the detach
        // that leaked. These two are the ones that mean the process is gone —
        // its pool entry torn down rather than short-circuited, and its pid no
        // longer answering.
        assert!(
            !fixture.manager.adapter_pools.lock().await.contains_key(&pool_key),
            "the adapter pool entry was torn down"
        );
        let reaped = Instant::now() + Duration::from_secs(2);
        while unsafe { libc::kill(adapter_pid as i32, 0) } == 0 && Instant::now() < reaped {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_ne!(
            unsafe { libc::kill(adapter_pid as i32, 0) },
            0,
            "the adapter process outlived the warm that was supposed to stop it"
        );

        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn a_send_refused_before_its_prompt_leaves_no_process_behind() {
        // The first message of a new session carries the choices made in the
        // start pane. When one of them is refused, the adapter that activation
        // just started stayed up with no turn to run and nothing to stop it —
        // a Claude session that failed this way kept five processes alive
        // until the app quit.
        let fixture = fixture_manager_with_acp_session("suspend_refused_send").await;
        let (runtime, pool_key) = {
            let sessions = fixture
                .manager
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = sessions.get(&fixture.owned_id).expect("activated session");
            (
                session.runtime.clone().expect("a running adapter"),
                session.pool_key.clone().expect("a pooled adapter"),
            )
        };
        let adapter_pid = runtime
            .lock()
            .await
            .process_id()
            .expect("the adapter runs in a process of its own");
        drop(runtime);

        let refused = fixture
            .manager
            .send_message(
                &fixture.owned_id,
                fixture.generation,
                test_prompt("hello"),
                None,
                Some("acceptedits".into()),
            )
            .await
            .expect_err("a policy the provider does not offer is refused");
        assert!(
            refused.contains("approval policy"),
            "the refusal names what was wrong: {refused}"
        );

        assert!(
            fixture
                .manager
                .session_is_suspended(&fixture.owned_id, fixture.generation),
            "a send that never went out leaves the session at rest"
        );
        assert!(
            !fixture.manager.adapter_pools.lock().await.contains_key(&pool_key),
            "the adapter pool entry was torn down"
        );
        let reaped = Instant::now() + Duration::from_secs(2);
        while unsafe { libc::kill(adapter_pid as i32, 0) } == 0 && Instant::now() < reaped {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_ne!(
            unsafe { libc::kill(adapter_pid as i32, 0) },
            0,
            "the adapter process outlived the send that was refused"
        );

        // The refusal cost nothing: the next send, with a policy the provider
        // offers, starts the adapter again and goes out.
        fixture
            .manager
            .send_message(
                &fixture.owned_id,
                fixture.generation,
                test_prompt("hello again"),
                None,
                Some("never".into()),
            )
            .await
            .expect("a valid send after a refused one");

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

    /// A build that could not hold a negative sequence wrote zero into the
    /// payload of every event it imported backwards. Those rows are in the
    /// database now, and every one of them claims to sit at position zero. The
    /// column knows better, and reading has to believe it — otherwise the
    /// transcript reads 112 events as one, and draws nothing.
    /// A tool row is a line that opens onto what the call produced. It used to
    /// open onto nothing: `content` and `locations` were joined into one string
    /// kept as the summary, and the output, the file and the change were gone
    /// before anything was written down.
    #[test]
    fn a_finished_tool_call_carries_its_output_and_the_file_it_touched() {
        let update = json!({ "update": {
            "sessionUpdate": "tool_call_update",
            "toolCallId": "t9",
            "title": "Run tests",
            "status": "completed",
            "content": [
                { "type": "text", "text": "running 3 tests\nall passed" }
            ],
            "locations": [{ "path": "core/src/lib.rs" }]
        } });
        match payload_from_session_update_for_turn(&update, None) {
            Some(AgentConversationPayload::Tool {
                summary,
                output,
                path,
                diff,
                ..
            }) => {
                assert_eq!(summary.as_deref(), Some("running 3 tests"));
                assert_eq!(output.as_deref(), Some("running 3 tests\nall passed"));
                assert_eq!(path.as_deref(), Some("core/src/lib.rs"));
                assert_eq!(diff, None);
            }
            other => panic!("expected Tool, got {other:?}"),
        }
    }

    /// ACP hands over a file before and after rather than a patch, so the row
    /// had nothing a diff view could read.
    #[test]
    fn a_file_edit_becomes_a_diff_the_transcript_can_draw() {
        let update = json!({ "update": {
            "sessionUpdate": "tool_call_update",
            "toolCallId": "t10",
            "title": "Edit file",
            "status": "completed",
            "content": [{
                "type": "diff",
                "path": "src/main.rs",
                "oldText": "fn main() {\n    println!(\"one\");\n}",
                "newText": "fn main() {\n    println!(\"two\");\n}"
            }]
        } });
        match payload_from_session_update_for_turn(&update, None) {
            Some(AgentConversationPayload::Tool { path, diff, .. }) => {
                assert_eq!(path.as_deref(), Some("src/main.rs"));
                let diff = diff.expect("an edit carries its change");
                // Only the line that changed, with the line it sits on: the
                // matching lines at either end are common ground.
                assert_eq!(
                    diff,
                    "@@ -2,1 +2,1 @@\n-    println!(\"one\");\n+    println!(\"two\");"
                );
            }
            other => panic!("expected Tool, got {other:?}"),
        }
    }

    #[test]
    fn a_file_with_no_change_produces_no_diff() {
        assert_eq!(unified_diff("same\nlines", "same\nlines"), "");
    }

    #[test]
    fn a_stored_event_takes_its_position_from_the_column_not_the_payload() {
        let event = serde_json::json!({
            "ownedId": "owned-a",
            "provider": "codex",
            "generation": 0,
            "sequence": 0,
            "timestampMs": 1_000,
            "payload": { "kind": "assistantMessage", "itemId": "a", "text": "hi", "completed": true }
        });
        let decoded = stored_event(EventRow {
            owned_id: "owned-a".into(),
            seq: -42,
            turn_id: None,
            kind: "item.completed".into(),
            payload_json: event.to_string(),
            created_at_ms: 1_000,
        })
        .expect("a stored row decodes");
        assert_eq!(decoded.sequence, -42);
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
        const EVENTS_WRITTEN: usize = 1_003;
        for index in 0..EVENTS_WRITTEN {
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
        assert_eq!(stored.len(), EVENTS_WRITTEN);
        let snapshot = manager.snapshot("owned-a").unwrap().unwrap().events;
        // Every event here is small, so they all fit the window: what matters
        // is that the tail is contiguous and ends where the journal does.
        assert_eq!(
            snapshot.last().unwrap().sequence,
            stored.last().unwrap().sequence
        );
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
