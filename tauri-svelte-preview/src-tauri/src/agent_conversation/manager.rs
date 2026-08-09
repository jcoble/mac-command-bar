use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{Arc, Mutex, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tokio::sync::Mutex as AsyncMutex;

use super::handoff::{
    AgentConversationHandoffDirection, AgentConversationHandoffMode, AgentConversationHandoffPhase,
    AgentConversationHandoffReceipt, AgentConversationHandoffRequest,
};
use super::journal::AgentEventJournal;
use super::protocol::{
    AgentApprovalDecision, AgentApprovalResponse, AgentCapabilities, AgentConversationConnection,
    AgentConversationEvent, AgentConversationPayload, AgentConversationProvider,
    AgentConversationSnapshot, AgentEvent, AgentEventType, AgentExecutionOwner,
    AgentImplementation, AgentInteractionCapabilities, AgentPromptCapabilities,
    AgentRequestIdentity, AgentRuntimeState, AgentSessionCapabilities, AgentUserInputResponse,
    AgentWriterLease, AgentWriterLeaseOwner, AgentWriterLeaseTransition, ApprovalState,
    ConversationConnectionState, EnsureAgentConversationRequest, PlanItem, ToolState,
};
use super::providers::acp_client::{AcpInbound, AcpTransport};
use super::providers::process::validated_conversation_cwd;
use super::providers::{
    AcpRuntimeAdapter, AgentConfigValue, AgentPrompt, AgentRuntimeAdapter, AgentRuntimeError,
    GeneratedText, InitializeAgentInput, LoadAgentSession, NewAgentSession, PermissionResponse,
    ProviderRegistry, StructuredRuntimeHandle,
};
use tokio::sync::mpsc::{self, UnboundedSender};

const SNAPSHOT_EVENT_CAP: usize = 2_000;
const JOURNAL_EVENT_CAP: usize = 10_000;

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
}

pub struct ManagedAgentSession {
    pub owned_id: String,
    pub provider: AgentConversationProvider,
    pub provider_instance_id: String,
    pub native_session_id: Option<String>,
    pub generation: u64,
    pub owner: AgentExecutionOwner,
    pub state: AgentRuntimeState,
    pub capabilities: AgentCapabilities,
    pub next_sequence: u64,
    pub active_turn_id: Option<String>,
    prompt_once_active: bool,
    pub runtime: Option<Arc<AsyncMutex<StructuredRuntimeHandle>>>,
    transport: Option<Arc<AcpTransport>>,
    pub recent_events: VecDeque<AgentEvent>,
    pub writer_lease: AgentWriterLease,
    pub writer_lease_transition: Option<AgentWriterLeaseTransition>,
    permission_requests: HashMap<String, PendingPermission>,
    next_permission_id: u64,
    ordered_events: Option<UnboundedSender<OrderedSessionEvent>>,
    cwd: String,
    connection: AgentConversationConnection,
    frontend_events: VecDeque<AgentConversationEvent>,
    journal: AgentEventJournal,
}

pub(crate) struct HandoffContext {
    pub previous_owner: AgentWriterLeaseOwner,
    pub owner: AgentWriterLeaseOwner,
    pub native_session_id: Option<String>,
}

#[derive(Clone)]
pub struct AgentRuntimeManager {
    sessions: Arc<Mutex<HashMap<String, ManagedAgentSession>>>,
    providers: Arc<ProviderRegistry>,
    emitter: Arc<Mutex<Option<ConversationEmitter>>>,
    activation_locks: Arc<Mutex<HashMap<String, Arc<AsyncMutex<()>>>>>,
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
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            providers: Arc::new(providers),
            emitter: Arc::new(Mutex::new(None)),
            activation_locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn providers(&self) -> &ProviderRegistry {
        &self.providers
    }

    pub fn set_emitter(&self, emitter: ConversationEmitter) {
        if let Ok(mut current) = self.emitter.lock() {
            *current = Some(emitter);
        }
    }

    /// Snapshot the provider processes that this registry currently owns.
    /// The async runtime mutex is intentionally sampled with `try_lock`: a
    /// resource refresh must never block an agent turn. A session that is in a
    /// transition simply appears on the next refresh.
    pub fn resource_roots(&self) -> Vec<AgentResourceRoot> {
        let Ok(sessions) = self.sessions.lock() else {
            return Vec::new();
        };
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
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
        Ok(current_session(&sessions, owned_id, generation)?
            .capabilities
            .clone())
    }

    pub fn ensure(
        &self,
        request: EnsureAgentConversationRequest,
    ) -> Result<AgentConversationConnection, String> {
        self.ensure_inner(request).map(|(connection, _)| connection)
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
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
        if let Some(current) = sessions.get(&owned_id) {
            if current.provider == request.provider
                && current.cwd == cwd
                && current.connection.state != ConversationConnectionState::Failed
            {
                return Ok((current.connection.clone(), None));
            }
        }
        let prior = sessions.remove(&owned_id);
        let previous_runtime = prior
            .as_ref()
            .and_then(|session| Some((session.runtime.clone()?, session.transport.clone()?)));
        let generation = prior
            .as_ref()
            .map(|session| session.generation.saturating_add(1))
            .unwrap_or(1);
        let connection = AgentConversationConnection {
            owned_id: owned_id.clone(),
            provider: request.provider,
            generation,
            native_session_id: normalized_optional_id(request.native_session_id),
            state: ConversationConnectionState::Connecting,
        };
        let provider_instance_id = format!("{}-{generation}", provider_id(request.provider));
        sessions.insert(
            owned_id.clone(),
            ManagedAgentSession {
                owned_id: owned_id.clone(),
                provider: request.provider,
                provider_instance_id,
                native_session_id: connection.native_session_id.clone(),
                generation,
                owner: AgentExecutionOwner::Stopped,
                state: AgentRuntimeState::Closed,
                capabilities: empty_capabilities(request.provider),
                next_sequence: 1,
                active_turn_id: None,
                prompt_once_active: false,
                runtime: None,
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
                ordered_events: None,
                cwd,
                connection: connection.clone(),
                frontend_events: VecDeque::new(),
                journal: AgentEventJournal::new(JOURNAL_EVENT_CAP),
            },
        );
        Ok((connection, previous_runtime))
    }

    pub async fn activate(
        &self,
        owned_id: &str,
        generation: u64,
    ) -> Result<AgentConversationConnection, String> {
        let activation_lock = self.activation_lock(owned_id)?;
        let _activation = activation_lock.lock().await;
        let (provider, provider_instance_id, cwd, native_session_id) = {
            let sessions = self
                .sessions
                .lock()
                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
            let session = current_session(&sessions, owned_id, generation)?;
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
            (
                session.provider,
                session.provider_instance_id.clone(),
                std::path::PathBuf::from(&session.cwd),
                session.native_session_id.clone(),
            )
        };
        let manifest = self.providers.manifest(provider)?;
        let mut adapter = AcpRuntimeAdapter::new(manifest, owned_id.to_string(), cwd.clone());
        let capabilities = adapter
            .initialize(InitializeAgentInput {
                provider,
                provider_instance_id,
            })
            .await
            .map_err(|error| error.to_string())?;
        let expected_native_session_id = native_session_id.clone();
        let started = match native_session_id {
            Some(native_session_id) => {
                adapter
                    .resume_session(LoadAgentSession {
                        cwd,
                        native_session_id,
                    })
                    .await
            }
            None => adapter.new_session(NewAgentSession { cwd }).await,
        }
        .map_err(|error| error.to_string())?;
        if let Some(expected_native_session_id) = expected_native_session_id {
            if started.native_session_id != expected_native_session_id {
                let _ = adapter.detach_session().await;
                return Err(
                    "The provider resumed a different native session; handoff was rejected"
                        .to_string(),
                );
            }
        }
        let runtime = Arc::new(AsyncMutex::new(StructuredRuntimeHandle::Acp(adapter)));
        let (transport, inbound) = {
            let mut runtime = runtime.lock().await;
            let transport = runtime.transport().map_err(|error| error.to_string())?;
            let inbound = runtime.take_inbound().map_err(|error| error.to_string())?;
            (transport, inbound)
        };
        let (ordered_tx, ordered_rx) = mpsc::unbounded_channel();
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
        let session = current_session_mut(&mut sessions, owned_id, generation)?;
        session.capabilities = capabilities;
        session.native_session_id = Some(started.native_session_id.clone());
        session.connection.native_session_id = Some(started.native_session_id);
        session.connection.state = ConversationConnectionState::Connected;
        let restoring_terminal_transition = session.owner
            == AgentExecutionOwner::TransitioningToStructured
            && session
                .writer_lease_transition
                .as_ref()
                .map(|transition| transition.to == AgentWriterLeaseOwner::Structured)
                .unwrap_or(false);
        if !restoring_terminal_transition {
            session.owner = AgentExecutionOwner::Structured;
            session.writer_lease.owner = AgentWriterLeaseOwner::Structured;
        }
        session.state = AgentRuntimeState::Ready;
        session.runtime = Some(runtime);
        session.transport = Some(Arc::clone(&transport));
        session.ordered_events = Some(ordered_tx);
        let connection = session.connection.clone();
        drop(sessions);
        spawn_inbound_pump(
            Arc::downgrade(&self.sessions),
            Arc::downgrade(&self.emitter),
            Arc::downgrade(&transport),
            inbound,
            ordered_rx,
            owned_id.to_string(),
            generation,
        );
        Ok(connection)
    }

    pub fn emit_payload(
        &self,
        owned_id: &str,
        generation: u64,
        payload: AgentConversationPayload,
    ) -> Result<AgentConversationEvent, String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
        let session = current_session_mut(&mut sessions, owned_id, generation)?;
        record_payload_for_session_and_dispatch(session, &self.emitter, payload)
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
                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
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
            session.state = AgentRuntimeState::Working;
            record_payload_for_session_and_dispatch(
                session,
                &self.emitter,
                AgentConversationPayload::Turn {
                    turn_id: turn_id.clone(),
                    state: super::protocol::TurnState::Started,
                },
            )?;
            (native_session_id, ordered_events)
        };

        let params = prompt_params(native_session_id, input);
        spawn_prompt_completion(transport, ordered_events, turn_id, params);
        Ok(())
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
        {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
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
        }
        let result = runtime.lock().await.prompt_once(input).await;
        if let Ok(mut sessions) = self.sessions.lock() {
            if let Ok(session) = current_session_mut(&mut sessions, owned_id, generation) {
                session.prompt_once_active = false;
            }
        }
        result.map_err(|error| error.to_string())
    }

    pub async fn respond_permission(&self, input: PermissionResponse) -> Result<(), String> {
        let owned_id = input.identity.owned_id.clone();
        let generation = input.identity.generation;
        let request_id = input.identity.request_id.clone();
        let runtime = self.runtime(&owned_id, generation)?;
        let transport = {
            let runtime = runtime.lock().await;
            runtime.transport().map_err(|error| error.to_string())?
        };
        let (pending, ordered_events, turn_active) = {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
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
            match input.decision {
                AgentApprovalDecision::Accept => ApprovalState::Accepted,
                AgentApprovalDecision::Decline | AgentApprovalDecision::Cancel => {
                    ApprovalState::Declined
                }
            }
        } else {
            ApprovalState::Declined
        };
        let response = match if turn_active {
            permission_response(&pending.options, input.decision)
        } else {
            Ok(serde_json::json!({ "outcome": { "outcome": "cancelled" } }))
        } {
            Ok(response) => response,
            Err(error) => {
                if let Ok(mut sessions) = self.sessions.lock() {
                    if let Ok(session) = current_session_mut(&mut sessions, &owned_id, generation) {
                        session.permission_requests.insert(request_id, pending);
                    }
                }
                return Err(error);
            }
        };
        if let Err(error) = transport.respond(pending.wire_id.clone(), response).await {
            if let Ok(mut sessions) = self.sessions.lock() {
                if let Ok(session) = current_session_mut(&mut sessions, &owned_id, generation) {
                    session.permission_requests.insert(request_id, pending);
                }
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
        let runtime = self.runtime(&input.identity.owned_id, input.identity.generation)?;
        let result = runtime
            .lock()
            .await
            .respond_user_input(input)
            .await
            .map_err(|error| error.to_string());
        result
    }

    pub async fn steer(&self, owned_id: &str, generation: u64, text: String) -> Result<(), String> {
        let runtime = self.runtime(owned_id, generation)?;
        let transport = {
            let runtime = runtime.lock().await;
            runtime.transport().map_err(|error| error.to_string())?
        };
        let native_session_id = self.native_session_id(owned_id, generation)?;
        transport
            .request(
                "session/steer",
                serde_json::json!({
                    "sessionId": native_session_id,
                    "prompt": [{ "type": "text", "text": text }]
                }),
            )
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub async fn set_config(
        &self,
        owned_id: &str,
        generation: u64,
        option_id: &str,
        value: AgentConfigValue,
    ) -> Result<Vec<super::protocol::AgentConfigOption>, String> {
        {
            let sessions = self
                .sessions
                .lock()
                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
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
        }
        let runtime = self.runtime(owned_id, generation)?;
        let replacement = runtime
            .lock()
            .await
            .set_config(option_id, value)
            .await
            .map_err(|error| error.to_string())?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
        let session = current_session_mut(&mut sessions, owned_id, generation)?;
        super::capabilities::replace_config_options(
            &mut session.capabilities,
            replacement.clone(),
        )?;
        Ok(replacement)
    }

    pub fn request_tool_terminal<R: tauri::Runtime>(
        &self,
        app: tauri::AppHandle<R>,
        terminal_registry: &crate::terminal::TerminalRegistry,
        generation: u64,
        request: crate::terminal::TerminalStartRequest,
        identity: crate::terminal::ToolTerminalIdentity,
    ) -> Result<crate::terminal::TerminalSessionInfo, String> {
        {
            let sessions = self
                .sessions
                .lock()
                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
            let session = current_session(&sessions, &identity.owned_id, generation)?;
            if session.owner != AgentExecutionOwner::Structured || session.runtime.is_none() {
                return Err(
                    "Tool terminal request is not from a current structured runtime".to_string(),
                );
            }
            if identity.turn_id.trim().is_empty() || identity.tool_call_id.trim().is_empty() {
                return Err("Tool terminal request identity is incomplete".to_string());
            }
        }
        crate::terminal::start_tool_terminal_session(app, terminal_registry, request, identity)
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
                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
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
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
        Ok(sessions
            .get(owned_id)
            .map(|session| AgentConversationSnapshot {
                connection: session.connection.clone(),
                last_sequence: session.next_sequence.saturating_sub(1),
                events: session.frontend_events.iter().cloned().collect(),
            }))
    }

    pub async fn close(&self, owned_id: &str) -> Result<bool, String> {
        let runtime = {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
            let Some(session) = sessions.get_mut(owned_id) else {
                return Ok(false);
            };
            session.state = AgentRuntimeState::Closed;
            session.owner = AgentExecutionOwner::Stopped;
            session.writer_lease.owner = AgentWriterLeaseOwner::None;
            session.transport = None;
            session.runtime.take()
        };
        if let Some(runtime) = runtime {
            runtime
                .lock()
                .await
                .close_session()
                .await
                .map_err(|error| error.to_string())?;
        }
        self.sessions
            .lock()
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?
            .remove(owned_id);
        Ok(true)
    }

    pub(crate) fn handoff_prepare(
        &self,
        request: &AgentConversationHandoffRequest,
    ) -> Result<HandoffContext, String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
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
        session.owner = match request.direction {
            AgentConversationHandoffDirection::StructuredToTerminal => {
                AgentExecutionOwner::TransitioningToTerminal
            }
            AgentConversationHandoffDirection::TerminalToStructured => {
                AgentExecutionOwner::TransitioningToStructured
            }
        };
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
                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
            let session = current_session_mut(&mut sessions, owned_id, generation)?;
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
                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
            if let Ok(session) = current_session_mut(&mut sessions, owned_id, generation) {
                session.runtime = Some(runtime);
            }
            return Err(error.to_string());
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
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
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
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
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
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
        let session = current_session_mut(&mut sessions, &request.owned_id, request.generation)?;
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
        session.writer_lease.owner = owner;
        session.owner = match owner {
            AgentWriterLeaseOwner::Structured => AgentExecutionOwner::Structured,
            AgentWriterLeaseOwner::Terminal => AgentExecutionOwner::Terminal,
            AgentWriterLeaseOwner::None => AgentExecutionOwner::Stopped,
        };
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
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
        let session = current_session_mut(&mut sessions, &request.owned_id, request.generation)?;
        let transition = session
            .writer_lease_transition
            .take()
            .ok_or_else(|| "Handoff rollback has no prepared transition".to_string())?;
        session.writer_lease.owner = transition.from;
        session.owner = match transition.from {
            AgentWriterLeaseOwner::Structured => AgentExecutionOwner::Structured,
            AgentWriterLeaseOwner::Terminal => AgentExecutionOwner::Terminal,
            AgentWriterLeaseOwner::None => AgentExecutionOwner::Stopped,
        };
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

    pub fn canonical_snapshot(&self, owned_id: &str) -> Result<Vec<AgentEvent>, String> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
        Ok(sessions
            .get(owned_id)
            .map(|session| session.recent_events.iter().cloned().collect())
            .unwrap_or_default())
    }

    fn activation_lock(&self, owned_id: &str) -> Result<Arc<AsyncMutex<()>>, String> {
        let mut locks = self
            .activation_locks
            .lock()
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
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
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
        let session = current_session(&sessions, owned_id, generation)?;
        if session.owner != AgentExecutionOwner::Structured {
            return Err("The structured writer is not the current owner".to_string());
        }
        session
            .runtime
            .clone()
            .ok_or_else(|| "Structured provider is still connecting".to_string())
    }

    fn native_session_id(&self, owned_id: &str, generation: u64) -> Result<String, String> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
        current_session(&sessions, owned_id, generation)?
            .native_session_id
            .clone()
            .ok_or_else(|| "Structured provider session has not started".to_string())
    }

    #[cfg(test)]
    pub(crate) fn session_terminal_ownership(
        &self,
        owned_id: &str,
    ) -> Option<AgentWriterLeaseOwner> {
        self.sessions
            .lock()
            .ok()?
            .get(owned_id)
            .map(|session| session.writer_lease.owner)
    }
}

impl Drop for AgentRuntimeManager {
    fn drop(&mut self) {
        if Arc::strong_count(&self.sessions) == 1 {
            if let Ok(mut sessions) = self.sessions.lock() {
                sessions.clear();
            }
        }
    }
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

fn record_payload_for_session(
    session: &mut ManagedAgentSession,
    payload: AgentConversationPayload,
) -> Result<AgentConversationEvent, String> {
    let sequence = session.next_sequence;
    session.next_sequence = session.next_sequence.saturating_add(1);
    if let AgentConversationPayload::Connection {
        state,
        native_session_id,
    } = &payload
    {
        session.connection.state = *state;
        if native_session_id.is_some() {
            session.connection.native_session_id = native_session_id.clone();
            session.native_session_id = native_session_id.clone();
        }
    }
    let timestamp_ms = timestamp_millis();
    let frontend_event = AgentConversationEvent {
        owned_id: session.owned_id.clone(),
        provider: session.provider,
        generation: session.generation,
        sequence,
        timestamp_ms,
        payload: payload.clone(),
    };
    let canonical = canonical_event(session, sequence, timestamp_ms, &payload)?;
    session.journal.append(canonical.clone())?;
    session.recent_events.push_back(canonical);
    session.frontend_events.push_back(frontend_event.clone());
    while session.recent_events.len() > SNAPSHOT_EVENT_CAP {
        session.recent_events.pop_front();
    }
    while session.frontend_events.len() > SNAPSHOT_EVENT_CAP {
        session.frontend_events.pop_front();
    }
    Ok(frontend_event)
}

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

fn dispatch_event(
    emitter: &Arc<Mutex<Option<ConversationEmitter>>>,
    event: &AgentConversationEvent,
) {
    let callback = emitter.lock().ok().and_then(|current| current.clone());
    if let Some(callback) = callback {
        callback(event.clone());
    }
}

fn prompt_params(native_session_id: String, input: AgentPrompt) -> Value {
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
    sessions: Weak<Mutex<HashMap<String, ManagedAgentSession>>>,
    emitter: Weak<Mutex<Option<ConversationEmitter>>>,
    transport: Weak<AcpTransport>,
    inbound: mpsc::UnboundedReceiver<AcpInbound>,
    ordered_events: mpsc::UnboundedReceiver<OrderedSessionEvent>,
    owned_id: String,
    generation: u64,
) {
    tokio::spawn(pump_inbound(
        sessions,
        emitter,
        transport,
        inbound,
        ordered_events,
        owned_id,
        generation,
    ));
}

async fn pump_inbound(
    sessions: Weak<Mutex<HashMap<String, ManagedAgentSession>>>,
    emitter: Weak<Mutex<Option<ConversationEmitter>>>,
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
                                &sessions,
                                &emitter,
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
        if transport.upgrade().is_none() {
            return;
        }
        let (Some(sessions), Some(emitter)) = (sessions.upgrade(), emitter.upgrade()) else {
            return;
        };
        match inbound {
            AcpInbound::SessionUpdate(params) => {
                let Ok(mut sessions) = sessions.lock() else {
                    return;
                };
                let Ok(session) = current_session_mut(&mut sessions, &owned_id, generation) else {
                    return;
                };
                if session.writer_lease.owner != AgentWriterLeaseOwner::Structured {
                    continue;
                }
                let Some(payload) = payload_from_session_update_for_turn(
                    &params,
                    session.active_turn_id.as_deref(),
                ) else {
                    continue;
                };
                if let Err(error) =
                    record_payload_for_session_and_dispatch(session, &emitter, payload)
                {
                    eprintln!("Could not record ACP session update: {error}");
                }
            }
            AcpInbound::AgentRequest {
                wire_id,
                method,
                params,
            } => {
                if method != "session/request_permission" {
                    eprintln!("Ignoring unsupported ACP agent request: {method}");
                    continue;
                }
                let summary = permission_summary(&params);
                let options = permission_options(&params);
                let Ok(mut sessions) = sessions.lock() else {
                    return;
                };
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
                session.state = AgentRuntimeState::WaitingApproval;
                if let Err(error) = record_payload_for_session_and_dispatch(
                    session,
                    &emitter,
                    AgentConversationPayload::Approval {
                        request_id,
                        state: ApprovalState::Requested,
                        summary,
                    },
                ) {
                    eprintln!("Could not record ACP permission request: {error}");
                }
            }
            AcpInbound::TransportClosed { reason } => {
                let Ok(mut sessions) = sessions.lock() else {
                    return;
                };
                let Ok(session) = current_session_mut(&mut sessions, &owned_id, generation) else {
                    return;
                };
                if session.writer_lease.owner == AgentWriterLeaseOwner::Structured {
                    let pending_permissions =
                        session.permission_requests.drain().collect::<Vec<_>>();
                    for (request_id, pending) in pending_permissions {
                        if let Err(error) = record_payload_for_session_and_dispatch(
                            session,
                            &emitter,
                            AgentConversationPayload::Approval {
                                request_id,
                                state: ApprovalState::Expired,
                                summary: pending.summary,
                            },
                        ) {
                            eprintln!("Could not record expired ACP permission request: {error}");
                        }
                    }
                    if let Some(turn_id) = session.active_turn_id.clone() {
                        if let Err(error) = record_payload_for_session_and_dispatch(
                            session,
                            &emitter,
                            AgentConversationPayload::Turn {
                                turn_id,
                                state: super::protocol::TurnState::Failed,
                            },
                        ) {
                            eprintln!("Could not record failed ACP turn: {error}");
                        }
                    }
                    let native_session_id = session.native_session_id.clone();
                    if let Err(error) = record_payload_for_session_and_dispatch(
                        session,
                        &emitter,
                        AgentConversationPayload::Connection {
                            state: ConversationConnectionState::Failed,
                            native_session_id,
                        },
                    ) {
                        eprintln!("Could not record ACP transport failure: {error}");
                    }
                    if let Err(error) = record_payload_for_session_and_dispatch(
                        session,
                        &emitter,
                        AgentConversationPayload::Error {
                            code: "acp-transport".to_string(),
                            message: reason,
                            recoverable: true,
                        },
                    ) {
                        eprintln!("Could not record ACP transport error: {error}");
                    }
                } else {
                    session.connection.state = ConversationConnectionState::Failed;
                }
                session.active_turn_id = None;
                session.prompt_once_active = false;
                session.state = AgentRuntimeState::Failed;
                return;
            }
        }
    }
}

async fn handle_ordered_session_event(
    ordered: OrderedSessionEvent,
    sessions: &Weak<Mutex<HashMap<String, ManagedAgentSession>>>,
    emitter: &Weak<Mutex<Option<ConversationEmitter>>>,
    transport: &Weak<AcpTransport>,
    owned_id: &str,
    generation: u64,
) -> bool {
    let (Some(sessions), Some(emitter)) = (sessions.upgrade(), emitter.upgrade()) else {
        return false;
    };
    match ordered {
        OrderedSessionEvent::ApprovalResolved {
            request_id,
            state,
            summary,
        } => {
            let Ok(mut sessions) = sessions.lock() else {
                return false;
            };
            let Ok(session) = current_session_mut(&mut sessions, owned_id, generation) else {
                return false;
            };
            if session.writer_lease.owner != AgentWriterLeaseOwner::Structured {
                return true;
            }
            if let Err(error) = record_payload_for_session_and_dispatch(
                session,
                &emitter,
                AgentConversationPayload::Approval {
                    request_id,
                    state,
                    summary,
                },
            ) {
                eprintln!("Could not record ACP permission resolution: {error}");
                return false;
            }
            session.state = if session.active_turn_id.is_some() {
                AgentRuntimeState::Working
            } else {
                AgentRuntimeState::Ready
            };
            true
        }
        OrderedSessionEvent::PromptResult { turn_id, result } => {
            let cancelled = result
                .as_ref()
                .ok()
                .and_then(stop_reason)
                .is_some_and(|reason| reason == "cancelled");
            let pending_permissions =
                take_pending_permissions(&sessions, owned_id, generation, &turn_id)
                    .unwrap_or_default();
            if let Some(transport) = transport.upgrade() {
                for (_, pending) in &pending_permissions {
                    if let Err(error) = transport
                        .respond(
                            pending.wire_id.clone(),
                            serde_json::json!({ "outcome": { "outcome": "cancelled" } }),
                        )
                        .await
                    {
                        eprintln!("Could not expire ACP permission request: {error}");
                    }
                }
            }
            let error_details = result
                .as_ref()
                .err()
                .map(|error| (error.code.to_string(), error.message.clone()));

            let Ok(mut sessions) = sessions.lock() else {
                return false;
            };
            let Ok(session) = current_session_mut(&mut sessions, owned_id, generation) else {
                return false;
            };
            if session.active_turn_id.as_deref() != Some(turn_id.as_str())
                || session.writer_lease.owner != AgentWriterLeaseOwner::Structured
            {
                return true;
            }
            for (request_id, pending) in pending_permissions {
                if let Err(error) = record_payload_for_session_and_dispatch(
                    session,
                    &emitter,
                    AgentConversationPayload::Approval {
                        request_id,
                        state: if cancelled {
                            ApprovalState::Declined
                        } else {
                            ApprovalState::Expired
                        },
                        summary: pending.summary,
                    },
                ) {
                    eprintln!("Could not record cancelled ACP permission request: {error}");
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
            if let Err(error) = record_payload_for_session_and_dispatch(session, &emitter, payload)
            {
                eprintln!("Could not record ACP turn completion: {error}");
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
                    eprintln!("Could not record ACP prompt error: {error}");
                    return false;
                }
            }
            session.active_turn_id = None;
            session.prompt_once_active = false;
            session.state = AgentRuntimeState::Ready;
            true
        }
    }
}

fn take_pending_permissions(
    sessions: &Arc<Mutex<HashMap<String, ManagedAgentSession>>>,
    owned_id: &str,
    generation: u64,
    turn_id: &str,
) -> Option<Vec<(String, PendingPermission)>> {
    let mut sessions = sessions.lock().ok()?;
    let session = current_session_mut(&mut sessions, owned_id, generation).ok()?;
    if session.active_turn_id.as_deref() != Some(turn_id)
        || session.writer_lease.owner != AgentWriterLeaseOwner::Structured
    {
        return None;
    }
    Some(session.permission_requests.drain().collect())
}

fn stop_reason(response: &Value) -> Option<&str> {
    response
        .get("stopReason")
        .or_else(|| response.get("stop_reason"))
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

fn is_allow_kind(kind: &str) -> bool {
    kind.eq_ignore_ascii_case("allow") || kind.to_ascii_lowercase().starts_with("allow_")
}

fn is_reject_kind(kind: &str) -> bool {
    kind.eq_ignore_ascii_case("reject") || kind.to_ascii_lowercase().starts_with("reject_")
}

fn canonical_event(
    session: &ManagedAgentSession,
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
        AgentConversationPayload::Approval { .. } => AgentEventType::ApprovalRequested,
        AgentConversationPayload::Plan { .. } => AgentEventType::PlanUpdated,
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
        AgentConversationPayload::Error { .. } => AgentEventType::RuntimeError,
    };
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
        native_session_id: session.native_session_id.clone(),
        turn_id: session.active_turn_id.clone(),
        item_id: None,
        request_id: None,
        payload,
        provider_metadata: None,
        raw_frame_reference: None,
    })
}

pub fn payload_from_session_update(params: &Value) -> Option<AgentConversationPayload> {
    payload_from_session_update_for_turn(params, None)
}

fn payload_from_session_update_for_turn(
    params: &Value,
    active_turn_id: Option<&str>,
) -> Option<AgentConversationPayload> {
    let update = params.get("update").unwrap_or(params);
    let Some(kind) = update
        .get("sessionUpdate")
        .or_else(|| update.get("session_update"))
        .or_else(|| update.get("type"))
        .and_then(Value::as_str)
    else {
        eprintln!("Ignoring ACP session update without a kind");
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

    match kind {
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
            completed: false,
        }),
        "agent_thought_chunk" => {
            eprintln!("Ignoring ACP agent thought update");
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
                    eprintln!("Ignoring unknown ACP tool status while retaining update: {status}");
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
            eprintln!("Ignoring unknown ACP session update kind: {unknown}");
            None
        }
    }
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
        }
    }
    fn temp_root() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("mcb-runtime-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    struct FixtureManager {
        manager: AgentRuntimeManager,
        owned_id: String,
        generation: u64,
        root: std::path::PathBuf,
    }

    async fn fixture_manager_with_acp_session(fixture: &str) -> FixtureManager {
        let root = temp_root();
        let log = root.join(format!("{fixture}.jsonl"));
        let manifest =
            super::super::providers::acp_client::tests::fixture_manifest_named(&log, fixture);
        let providers = ProviderRegistry::new([(AgentConversationProvider::Codex, manifest)])
            .expect("fixture provider");
        let manager = AgentRuntimeManager::new(providers);
        let owned_id = format!("owned-{fixture}");
        let connection = manager
            .ensure(request(
                root.to_str().unwrap(),
                &owned_id,
                AgentConversationProvider::Codex,
            ))
            .expect("ensure");
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
            AgentConversationPayload::Approval { .. } => "approval",
            AgentConversationPayload::Plan { .. } => "plan",
            AgentConversationPayload::Turn { .. } => "turn",
            AgentConversationPayload::Usage { .. } => "usage",
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
            payload_from_session_update(&chunk),
            Some(AgentConversationPayload::AssistantDelta {
                item_id: "m1".into(),
                delta: "Hi".into(),
            })
        );
        let tool = json!({ "sessionId": "s", "update": {
            "sessionUpdate": "tool_call", "toolCallId": "t1", "title": "Read file",
            "status": "in_progress" } });
        match payload_from_session_update(&tool) {
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
        let plan = json!({ "sessionId": "s", "update": {
            "sessionUpdate": "plan", "entries": [
                { "content": "step one", "status": "pending" }
            ] } });
        match payload_from_session_update(&plan) {
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
            payload_from_session_update(&user),
            Some(AgentConversationPayload::UserMessage {
                item_id: "user-turn-2".into(),
                text: "Question".into(),
                completed: false,
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
            match payload_from_session_update(&update) {
                Some(AgentConversationPayload::Tool { state, summary, .. }) => {
                    assert_eq!(state, expected);
                    assert_eq!(summary.as_deref(), Some("detail\nsrc/main.rs"));
                }
                other => panic!("expected Tool update, got {other:?}"),
            }
        }

        assert_eq!(
            payload_from_session_update(&json!({ "update": {
                "sessionUpdate": "agent_thought_chunk",
                "content": { "type": "text", "text": "private" }
            } })),
            None
        );
        assert_eq!(
            payload_from_session_update(&json!({ "update": {
                "sessionUpdate": "future_update"
            } })),
            None
        );
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

        fixture.manager.close(&fixture.owned_id).await.unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
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

        fixture.manager.close(&fixture.owned_id).await.unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn prompt_once_updates_are_not_forwarded_to_the_conversation_emitter() {
        let fixture = fixture_manager_with_acp_session("prompt_with_update").await;
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
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            seen.lock().unwrap().is_empty(),
            "one-shot updates must stay in the aggregation queue"
        );

        fixture.manager.close(&fixture.owned_id).await.unwrap();
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

        fixture.manager.close(&fixture.owned_id).await.unwrap();
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

        fixture.manager.close(&fixture.owned_id).await.unwrap();
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

        fixture.manager.close(&fixture.owned_id).await.unwrap();
        wait_until(|| !process_is_alive(pid)).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            seen.lock().unwrap().is_empty(),
            "a closed session cannot emit from its stale pump"
        );
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn permission_requests_are_answerable_while_a_turn_is_pending() {
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
        let fixture_log = fs::read_to_string(fixture.root.join("permission_midturn.jsonl"))
            .expect("permission fixture log");
        assert!(
            fixture_log.contains(r#""outcome":{"outcome":"selected","optionId":"allow""#),
            "permission response must select a valid allow option: {fixture_log}"
        );

        fixture.manager.close(&fixture.owned_id).await.unwrap();
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
        fixture.manager.close(&fixture.owned_id).await.unwrap();
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
        fixture.manager.close(&fixture.owned_id).await.unwrap();
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

        fixture.manager.close(&fixture.owned_id).await.unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn transport_exit_expires_pending_approval_before_cleanup() {
        let fixture = fixture_manager_with_acp_session("permission_dies").await;
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
                        state: ApprovalState::Expired,
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
        drop(seen);

        fixture.manager.close(&fixture.owned_id).await.unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[test]
    fn ensure_is_idempotent_and_generation_rejects_stale_writers() {
        let root = temp_root();
        let manager = AgentRuntimeManager::default();
        let first = manager
            .ensure(request(
                root.to_str().unwrap(),
                "owned-a",
                AgentConversationProvider::Codex,
            ))
            .unwrap();
        assert_eq!(
            manager
                .ensure(request(
                    root.to_str().unwrap(),
                    "owned-a",
                    AgentConversationProvider::Codex
                ))
                .unwrap(),
            first
        );
        let changed = manager
            .ensure(request(
                root.to_str().unwrap(),
                "owned-a",
                AgentConversationProvider::Claude,
            ))
            .unwrap();
        assert_eq!(changed.generation, 2);
        assert!(manager
            .emit_payload(
                "owned-a",
                first.generation,
                AgentConversationPayload::Error {
                    code: "stale".into(),
                    message: "stale".into(),
                    recoverable: true
                }
            )
            .is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn canonical_sequence_and_snapshot_are_bounded_and_repairable() {
        let root = temp_root();
        let manager = AgentRuntimeManager::default();
        let connection = manager
            .ensure(request(
                root.to_str().unwrap(),
                "owned-a",
                AgentConversationProvider::Codex,
            ))
            .unwrap();
        for index in 0..(SNAPSHOT_EVENT_CAP + 3) {
            manager
                .emit_payload(
                    "owned-a",
                    connection.generation,
                    AgentConversationPayload::Error {
                        code: "fixture".into(),
                        message: index.to_string(),
                        recoverable: true,
                    },
                )
                .unwrap();
        }
        let events = manager.canonical_snapshot("owned-a").unwrap();
        assert_eq!(events.len(), SNAPSHOT_EVENT_CAP);
        assert!(events
            .windows(2)
            .all(|pair| pair[1].sequence == pair[0].sequence + 1));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn structured_session_does_not_claim_a_user_pty() {
        let root = temp_root();
        let manager = AgentRuntimeManager::default();
        manager
            .ensure(request(
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
