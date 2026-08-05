use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::Mutex as AsyncMutex;

use super::journal::AgentEventJournal;
use super::protocol::{
    AgentApprovalDecision, AgentApprovalResponse, AgentCapabilities, AgentConversationConnection,
    AgentConversationEvent, AgentConversationPayload, AgentConversationProvider,
    AgentConversationSnapshot, AgentEvent, AgentEventType, AgentExecutionOwner,
    AgentImplementation, AgentInteractionCapabilities, AgentPromptCapabilities,
    AgentRequestIdentity, AgentRuntimeState, AgentSessionCapabilities, AgentUserInputResponse,
    AgentWriterLease, AgentWriterLeaseOwner, AgentWriterLeaseTransition,
    ConversationConnectionState, EnsureAgentConversationRequest,
};
use super::providers::process::validated_conversation_cwd;
use super::providers::{
    AcpRuntimeAdapter, AgentConfigValue, AgentPrompt, AgentRuntimeAdapter, AgentSteeringInput,
    InitializeAgentInput, LoadAgentSession, NewAgentSession, PermissionResponse, ProviderRegistry,
    StructuredRuntimeHandle,
};

const SNAPSHOT_EVENT_CAP: usize = 2_000;
const JOURNAL_EVENT_CAP: usize = 10_000;

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
    pub runtime: Option<Arc<AsyncMutex<StructuredRuntimeHandle>>>,
    pub recent_events: VecDeque<AgentEvent>,
    pub writer_lease: AgentWriterLease,
    pub writer_lease_transition: Option<AgentWriterLeaseTransition>,
    cwd: String,
    connection: AgentConversationConnection,
    frontend_events: VecDeque<AgentConversationEvent>,
    journal: AgentEventJournal,
}

#[derive(Clone)]
pub struct AgentRuntimeManager {
    sessions: Arc<Mutex<HashMap<String, ManagedAgentSession>>>,
    providers: Arc<ProviderRegistry>,
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
        }
    }

    pub fn providers(&self) -> &ProviderRegistry {
        &self.providers
    }

    pub fn ensure(
        &self,
        request: EnsureAgentConversationRequest,
    ) -> Result<AgentConversationConnection, String> {
        let owned_id = required_id(&request.owned_id, "Owned session id")?;
        let cwd = validated_conversation_cwd(&request.cwd)?
            .display()
            .to_string();
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
        if let Some(current) = sessions.get(&owned_id) {
            if current.provider == request.provider && current.cwd == cwd {
                return Ok(current.connection.clone());
            }
        }
        let prior = sessions.remove(&owned_id);
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
                runtime: None,
                recent_events: VecDeque::new(),
                writer_lease: AgentWriterLease {
                    owned_id,
                    generation,
                    owner: AgentWriterLeaseOwner::None,
                },
                writer_lease_transition: None,
                cwd,
                connection: connection.clone(),
                frontend_events: VecDeque::new(),
                journal: AgentEventJournal::new(JOURNAL_EVENT_CAP),
            },
        );
        Ok(connection)
    }

    pub async fn activate(
        &self,
        owned_id: &str,
        generation: u64,
    ) -> Result<AgentConversationConnection, String> {
        let (provider, provider_instance_id, cwd, native_session_id) = {
            let sessions = self
                .sessions
                .lock()
                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
            let session = current_session(&sessions, owned_id, generation)?;
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
        let runtime = Arc::new(AsyncMutex::new(StructuredRuntimeHandle::Acp(adapter)));
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
        let session = current_session_mut(&mut sessions, owned_id, generation)?;
        session.capabilities = capabilities;
        session.native_session_id = Some(started.native_session_id.clone());
        session.connection.native_session_id = Some(started.native_session_id);
        session.connection.state = ConversationConnectionState::Connected;
        session.owner = AgentExecutionOwner::Structured;
        session.state = AgentRuntimeState::Ready;
        session.writer_lease.owner = AgentWriterLeaseOwner::Structured;
        session.runtime = Some(runtime);
        Ok(session.connection.clone())
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
            owned_id: owned_id.to_string(),
            provider: session.provider,
            generation,
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

    pub async fn prompt(
        &self,
        owned_id: &str,
        generation: u64,
        input: AgentPrompt,
    ) -> Result<(), String> {
        let runtime = self.runtime(owned_id, generation)?;
        let started = runtime
            .lock()
            .await
            .prompt(input)
            .await
            .map_err(|error| error.to_string())?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
        let session = current_session_mut(&mut sessions, owned_id, generation)?;
        session.active_turn_id = started.turn_id;
        session.state = AgentRuntimeState::Working;
        Ok(())
    }

    pub async fn respond_permission(&self, input: PermissionResponse) -> Result<(), String> {
        let runtime = self.runtime(&input.identity.owned_id, input.identity.generation)?;
        let result = runtime
            .lock()
            .await
            .respond_permission(input)
            .await
            .map_err(|error| error.to_string());
        result
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
        let result = runtime
            .lock()
            .await
            .steer(AgentSteeringInput { text })
            .await
            .map_err(|error| error.to_string());
        result
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
        let turn_id = {
            let sessions = self
                .sessions
                .lock()
                .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
            current_session(&sessions, owned_id, generation)?
                .active_turn_id
                .clone()
        };
        let result = runtime
            .lock()
            .await
            .cancel_turn(turn_id.as_deref())
            .await
            .map_err(|error| error.to_string());
        result
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

    fn runtime(
        &self,
        owned_id: &str,
        generation: u64,
    ) -> Result<Arc<AsyncMutex<StructuredRuntimeHandle>>, String> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| "Agent runtime manager is unavailable".to_string())?;
        current_session(&sessions, owned_id, generation)?
            .runtime
            .clone()
            .ok_or_else(|| "Structured provider is still connecting".to_string())
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
    use std::fs;

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
