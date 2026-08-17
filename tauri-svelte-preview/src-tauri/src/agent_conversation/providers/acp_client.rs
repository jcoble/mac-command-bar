use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex, Weak,
};

use agent_client_protocol::schema::{v1 as acp, ProtocolVersion};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};

use super::super::protocol::{
    AgentCapabilities, AgentCommandDescriptor, AgentConfigOption, AgentConversationConfigState,
    AgentConversationProvider, AgentImplementation, AgentInteractionCapabilities,
    AgentPromptCapabilities, AgentProviderManifest, AgentSessionCapabilities,
};
use super::process::{
    SidecarEnvironment, SidecarProcess, SidecarProcessHandle, SidecarReadHalf, SidecarWriteHalf,
};
use super::{
    AgentConversationConfigUpdate, AgentPrompt, AgentRuntimeError, GeneratedText,
    StartedAgentSession,
};

#[derive(Clone, Debug)]
pub enum AcpInbound {
    /// params of a session/update notification
    SessionUpdate(Value),
    /// an agent->client JSON-RPC REQUEST (session/request_permission, etc.)
    AgentRequest {
        wire_id: Value,
        method: String,
        params: Value,
    },
    /// reader ended: sidecar exited or stdout closed
    TransportClosed { reason: String },
}

pub struct AcpTransport {
    pending: Mutex<HashMap<u64, oneshot::Sender<Result<Value, AgentRuntimeError>>>>,
    next_id: AtomicU64,
    closed: std::sync::atomic::AtomicBool,
    writer: tokio::sync::Mutex<SidecarWriteHalf>,
    process: Mutex<Option<SidecarProcessHandle>>,
    sessions: Arc<Mutex<HashMap<SessionId, AcpSessionState>>>,
}

pub type SessionId = String;

struct AcpSessionState {
    native_session_id: SessionId,
    commands: Vec<AgentCommandDescriptor>,
    config: AgentConversationConfigState,
    config_protocol: ConversationConfigProtocol,
    prompt_updates: PromptUpdateRoute,
}

#[derive(Default)]
struct PromptUpdateRoute {
    active: Option<ActivePromptUpdateRoute>,
    completed_turn_ids: VecDeque<String>,
}

struct ActivePromptUpdateRoute {
    turn_id: String,
    sender: mpsc::UnboundedSender<Value>,
    observed_turn_ids: Vec<String>,
}

impl AcpTransport {
    /// Splits the process pipes, spawns the reader task, returns the transport
    /// plus the single consumer end of the inbound channel.
    pub fn start(
        process: SidecarProcess,
    ) -> (Arc<AcpTransport>, mpsc::UnboundedReceiver<AcpInbound>) {
        let (mut reader, writer, process_handle) = process.split();
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let transport = Arc::new(Self {
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(0),
            closed: std::sync::atomic::AtomicBool::new(false),
            writer: tokio::sync::Mutex::new(writer),
            process: Mutex::new(Some(process_handle)),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        });
        let reader_transport = Arc::downgrade(&transport);
        tokio::spawn(async move {
            reader_loop(&mut reader, &inbound_tx, reader_transport).await;
        });
        (transport, inbound_rx)
    }

    /// JSON-RPC request: allocate id, register oneshot, write, await.
    pub async fn request(&self, method: &str, params: Value) -> Result<Value, AgentRuntimeError> {
        let id = self
            .next_id
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        let (sender, receiver) = oneshot::channel();
        {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| transport_error("pending request map is unavailable".to_string()))?;
            if self.closed.load(Ordering::Acquire) {
                return Err(transport_error("ACP transport is closed".to_string()));
            }
            pending.insert(id, sender);
        }
        let mut guard = PendingRequestGuard {
            transport: self,
            id,
            armed: true,
        };
        let frame = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        if let Err(error) = self.write(frame).await {
            if let Ok(mut pending) = self.pending.lock() {
                pending.remove(&id);
            }
            guard.armed = false;
            return Err(error);
        }
        let result = receiver
            .await
            .map_err(|_| transport_error("transport closed before response".to_string()))?;
        guard.armed = false;
        result
    }

    /// Fire-and-forget notification (e.g. session/cancel).
    pub async fn notify(&self, method: &str, params: Value) -> Result<(), AgentRuntimeError> {
        self.write(json!({"jsonrpc": "2.0", "method": method, "params": params}))
            .await
    }

    /// Answer an agent->client request by wire id.
    pub async fn respond(&self, wire_id: Value, result: Value) -> Result<(), AgentRuntimeError> {
        self.write(json!({"jsonrpc": "2.0", "id": wire_id, "result": result}))
            .await
    }

    pub async fn stop(&self) {
        self.fail_pending("ACP transport stopped");
        let process = self.process.lock().ok().and_then(|mut value| value.take());
        if let Some(mut process) = process {
            process.stop().await;
        }
    }

    pub fn process_id(&self) -> Option<u32> {
        self.process
            .lock()
            .ok()
            .and_then(|value| value.as_ref().and_then(SidecarProcessHandle::process_id))
    }

    fn sessions(&self) -> Arc<Mutex<HashMap<SessionId, AcpSessionState>>> {
        Arc::clone(&self.sessions)
    }

    fn register_prompt_updates(
        self: &Arc<Self>,
        session_id: &str,
        turn_id: String,
    ) -> Result<(mpsc::UnboundedReceiver<Value>, PromptUpdateRegistration), AgentRuntimeError> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| transport_error("prompt update queue is unavailable".to_string()))?;
        let prompt_updates = &mut sessions
            .get_mut(session_id)
            .ok_or_else(|| session_not_found(session_id))?
            .prompt_updates;
        if prompt_updates.active.is_some() {
            return Err(transport_error(
                "ACP prompt aggregation is already active for this session".to_string(),
            ));
        }
        prompt_updates.active = Some(ActivePromptUpdateRoute {
            turn_id: turn_id.clone(),
            sender,
            observed_turn_ids: Vec::new(),
        });
        Ok((
            receiver,
            PromptUpdateRegistration {
                transport: Arc::clone(self),
                session_id: session_id.to_string(),
                turn_id,
                active: true,
            },
        ))
    }

    fn unregister_prompt_updates(
        &self,
        session_id: &str,
        turn_id: &str,
        response_turn_id: Option<&str>,
    ) {
        if let Ok(mut sessions) = self.sessions.lock() {
            let Some(prompt_updates) = sessions
                .get_mut(session_id)
                .map(|state| &mut state.prompt_updates)
            else {
                return;
            };
            let Some(active) = prompt_updates.active.take() else {
                return;
            };
            if active.turn_id != turn_id {
                prompt_updates.active = Some(active);
                return;
            }
            Self::remember_completed_turn_id(prompt_updates, turn_id);
            if let Some(response_turn_id) = response_turn_id {
                Self::remember_completed_turn_id(prompt_updates, response_turn_id);
            }
            for observed_turn_id in active.observed_turn_ids {
                Self::remember_completed_turn_id(prompt_updates, &observed_turn_id);
            }
        }
    }

    /// Route every update to an active one-shot aggregation queue. Returning
    /// true tells the reader that this frame was consumed and must not enter
    /// the conversation pump. Completed one-shot turn identities are retained
    /// long enough to drop late updates after the queue is unregistered.
    fn send_prompt_update(&self, params: Value) -> bool {
        let turn_id = prompt_update_turn_id(&params).map(str::to_string);
        if let Ok(mut sessions) = self.sessions.lock() {
            let routed_session_id =
                resolve_update_session_id(&sessions, &params, turn_id.as_deref());
            let Some(prompt_updates) = routed_session_id
                .as_deref()
                .and_then(|session_id| sessions.get_mut(session_id))
                .map(|state| &mut state.prompt_updates)
            else {
                return false;
            };
            if let Some(active) = prompt_updates.active.as_mut() {
                if let Some(turn_id) = turn_id.as_deref() {
                    if !active
                        .observed_turn_ids
                        .iter()
                        .any(|observed| observed == turn_id)
                    {
                        active.observed_turn_ids.push(turn_id.to_string());
                    }
                }
                let _ = active.sender.send(params);
                return true;
            }
            if let Some(turn_id) = turn_id.as_deref() {
                if prompt_updates
                    .completed_turn_ids
                    .iter()
                    .any(|completed| completed == turn_id)
                {
                    crate::debug_log::stderr_log!(
                        "[debug] Dropping late ACP one-shot update for completed turn {turn_id}"
                    );
                    return true;
                }
            }
        }
        false
    }

    fn capture_session_update(&self, params: &Value) {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session_id =
            resolve_update_session_id(&sessions, params, prompt_update_turn_id(params));
        let Some(state) = session_id.and_then(|session_id| sessions.get_mut(&session_id)) else {
            return;
        };
        if let Some(mode_id) = current_mode_update(params) {
            state.config.approval_policy = Some(mode_id.to_string());
        }
        if let Some(commands) = available_commands_update(params) {
            state.commands = commands;
        }
    }

    fn remember_completed_turn_id(prompt_updates: &mut PromptUpdateRoute, turn_id: &str) {
        if prompt_updates
            .completed_turn_ids
            .iter()
            .any(|completed| completed == turn_id)
        {
            return;
        }
        prompt_updates
            .completed_turn_ids
            .push_back(turn_id.to_string());
        while prompt_updates.completed_turn_ids.len() > 32 {
            prompt_updates.completed_turn_ids.pop_front();
        }
    }

    async fn write(&self, frame: Value) -> Result<(), AgentRuntimeError> {
        self.writer
            .lock()
            .await
            .write_json(&frame)
            .await
            .map_err(transport_error)
    }

    fn fail_pending(&self, reason: &str) {
        if let Ok(mut map) = self.pending.lock() {
            self.closed.store(true, Ordering::Release);
            for (_, sender) in map.drain() {
                let _ = sender.send(Err(transport_error(reason.to_string())));
            }
        }
    }
}

struct PendingRequestGuard<'a> {
    transport: &'a AcpTransport,
    id: u64,
    armed: bool,
}

struct PromptUpdateRegistration {
    transport: Arc<AcpTransport>,
    session_id: SessionId,
    turn_id: String,
    active: bool,
}

impl PromptUpdateRegistration {
    fn finish(&mut self, response_turn_id: Option<&str>) {
        if self.active {
            self.transport.unregister_prompt_updates(
                &self.session_id,
                &self.turn_id,
                response_turn_id,
            );
            self.active = false;
        }
    }
}

impl Drop for PromptUpdateRegistration {
    fn drop(&mut self) {
        self.finish(None);
    }
}

impl Drop for PendingRequestGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            if let Ok(mut pending) = self.transport.pending.lock() {
                pending.remove(&self.id);
            }
        }
    }
}

async fn reader_loop(
    reader: &mut SidecarReadHalf,
    inbound_tx: &mpsc::UnboundedSender<AcpInbound>,
    transport: Weak<AcpTransport>,
) {
    let reason = loop {
        let frame = match reader.next_json().await {
            Ok(frame) => frame,
            Err(reason) => break reason,
        };
        let has_id = frame.get("id").is_some();
        let method = frame.get("method").and_then(Value::as_str);
        match (has_id, method) {
            (true, None) => {
                let Some(transport) = transport.upgrade() else {
                    return;
                };
                let Some(id) = frame.get("id").and_then(Value::as_u64) else {
                    continue;
                };
                let sender = transport
                    .pending
                    .lock()
                    .ok()
                    .and_then(|mut map| map.remove(&id));
                if let Some(sender) = sender {
                    let result = if let Some(error) = frame.get("error") {
                        Err(AgentRuntimeError::new("acp-error", error.to_string()))
                    } else {
                        Ok(frame.get("result").cloned().unwrap_or(Value::Null))
                    };
                    let _ = sender.send(result);
                } else {
                    crate::debug_log::stderr_log!("Unmatched ACP response id: {id}");
                }
            }
            (true, Some(method)) => {
                let inbound = AcpInbound::AgentRequest {
                    wire_id: frame.get("id").cloned().unwrap_or(Value::Null),
                    method: method.to_string(),
                    params: frame.get("params").cloned().unwrap_or(Value::Null),
                };
                let _ = inbound_tx.send(inbound);
            }
            (false, Some("session/update")) => {
                let params = frame.get("params").cloned().unwrap_or(Value::Null);
                if let Some(transport) = transport.upgrade() {
                    transport.capture_session_update(&params);
                    if transport.send_prompt_update(params.clone()) {
                        continue;
                    }
                }
                let _ = inbound_tx.send(AcpInbound::SessionUpdate(params));
            }
            (false, Some(method)) => {
                crate::debug_log::stderr_log!("Ignoring unsupported ACP notification: {method}");
            }
            (false, None) => {}
        }
    };

    if let Some(transport) = transport.upgrade() {
        transport.fail_pending(&reason);
    }
    let inbound = AcpInbound::TransportClosed {
        reason: reason.clone(),
    };
    let _ = inbound_tx.send(inbound);
}

pub struct AcpClient {
    transport: Arc<AcpTransport>,
    inbound: Option<mpsc::UnboundedReceiver<AcpInbound>>,
    sessions: Arc<Mutex<HashMap<SessionId, AcpSessionState>>>,
    primary_session_id: Option<SessionId>,
    provider: Option<AgentConversationProvider>,
}

const CLAUDE_VERIFIED_EXTRA_MODELS: [&str; 3] = ["opus", "claude-opus-5", "claude-fable-5"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConversationConfigProtocol {
    Metadata,
    Standard,
}

impl AcpClient {
    #[cfg(test)]
    pub fn spawn(
        manifest: &AgentProviderManifest,
        cwd: &Path,
        owned_id: &str,
    ) -> Result<Self, AgentRuntimeError> {
        Self::spawn_with_environment(manifest, cwd, owned_id, &SidecarEnvironment::default())
    }

    pub fn spawn_with_environment(
        manifest: &AgentProviderManifest,
        cwd: &Path,
        owned_id: &str,
        environment: &SidecarEnvironment,
    ) -> Result<Self, AgentRuntimeError> {
        let process = SidecarProcess::spawn_with_environment(manifest, cwd, owned_id, environment)
            .map_err(|message| AgentRuntimeError::new("sidecar-spawn", message))?;
        let (transport, inbound) = AcpTransport::start(process);
        let sessions = transport.sessions();
        Ok(Self {
            transport,
            inbound: Some(inbound),
            sessions,
            primary_session_id: None,
            provider: None,
        })
    }

    pub fn transport(&self) -> Arc<AcpTransport> {
        Arc::clone(&self.transport)
    }

    pub fn take_inbound(
        &mut self,
    ) -> Result<mpsc::UnboundedReceiver<AcpInbound>, AgentRuntimeError> {
        self.inbound.take().ok_or_else(|| {
            AgentRuntimeError::new(
                "inbound-already-taken",
                "ACP inbound events already have a consumer",
            )
        })
    }

    pub async fn initialize(
        &mut self,
        provider: AgentConversationProvider,
    ) -> Result<AgentCapabilities, AgentRuntimeError> {
        let request = acp::InitializeRequest::new(ProtocolVersion::V1);
        let result = self.request("initialize", &request).await?;
        let implementation = result.get("agentInfo").or_else(|| result.get("agent_info"));
        let capabilities = AgentCapabilities {
            revision: 1,
            provider,
            implementation: AgentImplementation {
                name: implementation
                    .and_then(|value| value.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("ACP adapter")
                    .to_string(),
                version: implementation
                    .and_then(|value| value.get("version"))
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
            },
            session: AgentSessionCapabilities {
                multi_session: capability_at(
                    &result,
                    &["agentCapabilities", "sessionCapabilities", "multiSession"],
                ),
                list: capability_at(
                    &result,
                    &["agentCapabilities", "sessionCapabilities", "list"],
                ),
                load: bool_at(&result, &["agentCapabilities", "loadSession"]),
                resume: capability_at(
                    &result,
                    &["agentCapabilities", "sessionCapabilities", "resume"],
                ),
                close: capability_at(
                    &result,
                    &["agentCapabilities", "sessionCapabilities", "close"],
                ),
                steering: capability_at(
                    &result,
                    &["agentCapabilities", "sessionCapabilities", "steering"],
                ),
                fork: capability_at(
                    &result,
                    &["agentCapabilities", "sessionCapabilities", "fork"],
                ) || capability_at(&result, &["agentCapabilities", "fork"]),
            },
            prompt: AgentPromptCapabilities {
                text: true,
                image: bool_at(
                    &result,
                    &["agentCapabilities", "promptCapabilities", "image"],
                ),
                embedded_context: bool_at(
                    &result,
                    &["agentCapabilities", "promptCapabilities", "embeddedContext"],
                ),
                resource_links: true,
            },
            interaction: AgentInteractionCapabilities {
                permissions: true,
                structured_user_input: true,
                tool_terminals: true,
                plans: true,
                tasks: true,
                subagents: true,
            },
            config_options: parse_config_options(result.get("sessionConfigOptions")),
            commands: parse_command_descriptors(
                result
                    .get("availableCommands")
                    .or_else(|| result.get("available_commands"))
                    .or_else(|| result.pointer("/agentCapabilities/commands")),
            ),
        };
        self.provider = Some(provider);
        Ok(capabilities)
    }

    pub async fn new_session(
        &mut self,
        cwd: &Path,
    ) -> Result<StartedAgentSession, AgentRuntimeError> {
        let request = acp::NewSessionRequest::new(cwd);
        self.start_session("session/new", &request, None).await
    }

    /// Start an additional native session without changing the primary session.
    #[allow(dead_code)] // Track B calls this after the manager multiplexing cutover.
    pub async fn new_session_multi(&mut self, cwd: &Path) -> Result<SessionId, AgentRuntimeError> {
        Ok(self.new_session(cwd).await?.native_session_id)
    }

    pub async fn resume_session(
        &mut self,
        cwd: &Path,
        native_session_id: &str,
    ) -> Result<StartedAgentSession, AgentRuntimeError> {
        let request = acp::ResumeSessionRequest::new(native_session_id.to_string(), cwd);
        self.start_session("session/resume", &request, Some(native_session_id))
            .await
    }

    /// Resume an additional native session without changing the primary session.
    #[allow(dead_code)] // Track B calls this after the manager multiplexing cutover.
    pub async fn resume_session_multi(
        &mut self,
        cwd: &Path,
        native_session_id: &str,
    ) -> Result<SessionId, AgentRuntimeError> {
        Ok(self
            .resume_session(cwd, native_session_id)
            .await?
            .native_session_id)
    }

    pub async fn load_session(
        &mut self,
        cwd: &Path,
        native_session_id: &str,
    ) -> Result<StartedAgentSession, AgentRuntimeError> {
        let request = acp::LoadSessionRequest::new(native_session_id.to_string(), cwd);
        self.start_session("session/load", &request, Some(native_session_id))
            .await
    }

    pub async fn load_session_multi(
        &mut self,
        cwd: &Path,
        native_session_id: &str,
    ) -> Result<SessionId, AgentRuntimeError> {
        Ok(self
            .load_session(cwd, native_session_id)
            .await?
            .native_session_id)
    }

    /// Send a one-shot prompt and retain only assistant text updates.
    ///
    /// The regular `prompt` call intentionally returns as soon as ACP accepts
    /// the request; the conversation event stream owns the rest of that turn.
    /// Git actions need a direct result, so this narrow path consumes the same
    /// ACP transport until the prompt response arrives and ignores tool,
    /// reasoning, and lifecycle updates.
    /// Send a one-shot prompt to a specific native session.
    pub async fn prompt_on(
        &self,
        session_id: &str,
        prompt: AgentPrompt,
    ) -> Result<GeneratedText, AgentRuntimeError> {
        self.require_session(session_id)?;
        let mut blocks = vec![acp::ContentBlock::Text(acp::TextContent::new(prompt.text))];
        blocks.extend(prompt.images.into_iter().map(|image| {
            acp::ContentBlock::Image(acp::ImageContent::new(image.data, image.mime_type))
        }));
        let params = serde_json::to_value(acp::PromptRequest::new(session_id.to_string(), blocks))
            .map_err(serialization_error)?;
        let one_shot_turn_id = format!("turn-{}", uuid::Uuid::new_v4());
        let mut params = params;
        insert_prompt_turn_id(&mut params, &one_shot_turn_id)?;
        let (mut updates, mut registration) = self
            .transport
            .register_prompt_updates(session_id, one_shot_turn_id.clone())?;
        let mut text = String::new();
        let mut turn_id = None;
        let response = self.transport.request("session/prompt", params).await;
        let response_turn_id = response
            .as_ref()
            .ok()
            .and_then(|response| response.get("turnId").or_else(|| response.get("turn_id")))
            .and_then(Value::as_str)
            .map(str::to_string);
        registration.finish(response_turn_id.as_deref());
        let response = response?;
        while let Some(params) = updates.recv().await {
            append_prompt_update(&params, &mut turn_id, &mut text);
        }
        if turn_id.is_none() {
            turn_id = response_turn_id.or_else(|| Some(one_shot_turn_id));
        }
        if let Some(result_text) = extract_text_value(&response) {
            text.push_str(&result_text);
        }
        let text = text.trim().to_string();
        if text.is_empty() {
            return Err(AgentRuntimeError::new(
                "empty-response",
                "The active agent returned no text",
            ));
        }
        Ok(GeneratedText { turn_id, text })
    }

    pub async fn set_config_on(
        &self,
        session_id: &str,
        option_id: &str,
        value: Value,
    ) -> Result<Vec<AgentConfigOption>, AgentRuntimeError> {
        self.require_session(session_id)?;
        let result = self
            .transport
            .request(
                "session/set_config_option",
                json!({ "sessionId": session_id, "configId": option_id, "value": value }),
            )
            .await?;
        Ok(parse_config_options(
            result
                .get("configOptions")
                .or_else(|| result.get("sessionConfigOptions")),
        ))
    }

    /// Update conversation configuration on a specific native session.
    pub async fn set_conversation_config_on(
        &self,
        session_id: &str,
        update: &AgentConversationConfigUpdate,
    ) -> Result<AgentConversationConfigState, AgentRuntimeError> {
        let protocol = self.require_session(session_id)?.config_protocol;
        if protocol == ConversationConfigProtocol::Standard {
            return self
                .set_standard_conversation_config_on(session_id, update)
                .await;
        }
        let mut params = serde_json::to_value(update)
            .map_err(serialization_error)?
            .as_object()
            .cloned()
            .ok_or_else(|| AgentRuntimeError::new("serialization", "Config update is invalid"))?;
        params.insert(
            "sessionId".to_string(),
            Value::String(session_id.to_string()),
        );
        let result = self
            .transport
            .request("session/set_config_option", Value::Object(params))
            .await?;
        let config = parse_conversation_config(Some(&result))?;
        self.replace_session_config(session_id, config.clone())?;
        Ok(config)
    }

    async fn set_standard_conversation_config_on(
        &self,
        session_id: &str,
        update: &AgentConversationConfigUpdate,
    ) -> Result<AgentConversationConfigState, AgentRuntimeError> {
        if update.reasoning_effort.is_some() {
            return Err(AgentRuntimeError::new(
                "unsupported-config",
                "This ACP session does not expose a reasoning-effort control",
            ));
        }
        if let Some(model_id) = &update.model {
            self.transport
                .request(
                    "session/set_model",
                    json!({ "sessionId": session_id, "modelId": model_id }),
                )
                .await?;
            self.update_session_config(session_id, |config| config.model = Some(model_id.clone()))?;
        }
        if let Some(mode_id) = &update.approval_policy {
            self.transport
                .request(
                    "session/set_mode",
                    json!({ "sessionId": session_id, "modeId": mode_id }),
                )
                .await?;
            self.update_session_config(session_id, |config| {
                config.approval_policy = Some(mode_id.clone());
            })?;
        }
        Ok(self.require_session(session_id)?.config)
    }

    /// Select a model on a specific native session using its negotiated config protocol.
    #[allow(dead_code)] // Track B calls this after the manager multiplexing cutover.
    pub async fn set_model_on(
        &self,
        session_id: &str,
        model_id: &str,
    ) -> Result<AgentConversationConfigState, AgentRuntimeError> {
        self.set_conversation_config_on(
            session_id,
            &AgentConversationConfigUpdate {
                model: Some(model_id.to_string()),
                reasoning_effort: None,
                approval_policy: None,
            },
        )
        .await
    }

    pub async fn close(&mut self) -> Result<(), AgentRuntimeError> {
        let session_ids = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for session_id in session_ids {
            let _ = self
                .request("session/close", &acp::CloseSessionRequest::new(session_id))
                .await;
        }
        self.transport.stop().await;
        self.sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
        self.primary_session_id = None;
        Ok(())
    }

    /// Close one native session while leaving the shared adapter process alive.
    #[allow(dead_code)] // Track B calls this after the manager multiplexing cutover.
    pub async fn close_session(&mut self, session_id: &str) -> Result<(), AgentRuntimeError> {
        self.require_session(session_id)?;
        self.request(
            "session/close",
            &acp::CloseSessionRequest::new(session_id.to_string()),
        )
        .await?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| transport_error("session state is unavailable".to_string()))?;
        sessions.remove(session_id);
        if self.primary_session_id.as_deref() == Some(session_id) {
            self.primary_session_id = sessions.keys().next().cloned();
        }
        Ok(())
    }

    /// Detach the local transport while preserving the provider-native
    /// session id. A later activation can resume the same session.
    pub async fn detach(&mut self) -> Result<(), AgentRuntimeError> {
        self.transport.stop().await;
        Ok(())
    }

    pub fn process_id(&self) -> Option<u32> {
        self.transport.process_id()
    }

    async fn start_session<T: Serialize>(
        &mut self,
        method: &str,
        request: &T,
        known_session_id: Option<&str>,
    ) -> Result<StartedAgentSession, AgentRuntimeError> {
        let result = self.request(method, request).await?;
        // session/new must return the id; session/load and session/resume
        // may return a null result because the caller already knows it.
        let native_session_id = result
            .get("sessionId")
            .or_else(|| result.get("session_id"))
            .and_then(Value::as_str)
            .or(known_session_id)
            .ok_or_else(|| {
                AgentRuntimeError::new(
                    "invalid-response",
                    "ACP session response did not include sessionId",
                )
            })?
            .to_string();
        let metadata_config = parse_conversation_config(result.get("_meta"))?;
        let (config, protocol) = if metadata_config == AgentConversationConfigState::default() {
            let standard_config = parse_standard_conversation_config(&result, self.provider);
            if standard_config == AgentConversationConfigState::default() {
                (metadata_config, ConversationConfigProtocol::Metadata)
            } else {
                (standard_config, ConversationConfigProtocol::Standard)
            }
        } else {
            (metadata_config, ConversationConfigProtocol::Metadata)
        };
        let commands = parse_command_descriptors(
            result
                .get("availableCommands")
                .or_else(|| result.get("available_commands"))
                .or_else(|| result.pointer("/_meta/availableCommands"))
                .or_else(|| result.pointer("/_meta/available_commands")),
        );
        self.sessions
            .lock()
            .map_err(|_| transport_error("session state is unavailable".to_string()))?
            .insert(
                native_session_id.clone(),
                AcpSessionState {
                    native_session_id: native_session_id.clone(),
                    commands: commands.clone(),
                    config: config.clone(),
                    config_protocol: protocol,
                    prompt_updates: PromptUpdateRoute::default(),
                },
            );
        if self.primary_session_id.is_none() {
            self.primary_session_id = Some(native_session_id.clone());
        }
        Ok(StartedAgentSession {
            native_session_id,
            config,
            commands,
        })
    }

    async fn request<T: Serialize>(
        &mut self,
        method: &str,
        params: &T,
    ) -> Result<Value, AgentRuntimeError> {
        let params = serde_json::to_value(params).map_err(serialization_error)?;
        self.transport.request(method, params).await
    }

    fn require_session(&self, session_id: &str) -> Result<AcpSessionSnapshot, AgentRuntimeError> {
        self.sessions
            .lock()
            .map_err(|_| transport_error("session state is unavailable".to_string()))?
            .get(session_id)
            .map(AcpSessionSnapshot::from)
            .ok_or_else(|| session_not_found(session_id))
    }

    fn replace_session_config(
        &self,
        session_id: &str,
        config: AgentConversationConfigState,
    ) -> Result<(), AgentRuntimeError> {
        self.update_session_config(session_id, |current| *current = config)
    }

    fn update_session_config(
        &self,
        session_id: &str,
        update: impl FnOnce(&mut AgentConversationConfigState),
    ) -> Result<(), AgentRuntimeError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| transport_error("session state is unavailable".to_string()))?;
        let state = sessions
            .get_mut(session_id)
            .ok_or_else(|| session_not_found(session_id))?;
        update(&mut state.config);
        Ok(())
    }

    #[allow(dead_code)] // Track B reads this after the manager multiplexing cutover.
    pub fn commands_on(
        &self,
        session_id: &str,
    ) -> Result<Vec<AgentCommandDescriptor>, AgentRuntimeError> {
        self.sessions
            .lock()
            .map_err(|_| transport_error("session state is unavailable".to_string()))?
            .get(session_id)
            .map(|state| state.commands.clone())
            .ok_or_else(|| session_not_found(session_id))
    }

    pub fn config_on(
        &self,
        session_id: &str,
    ) -> Result<AgentConversationConfigState, AgentRuntimeError> {
        Ok(self.require_session(session_id)?.config)
    }
}

struct AcpSessionSnapshot {
    config: AgentConversationConfigState,
    config_protocol: ConversationConfigProtocol,
}

impl From<&AcpSessionState> for AcpSessionSnapshot {
    fn from(state: &AcpSessionState) -> Self {
        debug_assert!(!state.native_session_id.is_empty());
        Self {
            config: state.config.clone(),
            config_protocol: state.config_protocol,
        }
    }
}

fn session_not_found(session_id: &str) -> AgentRuntimeError {
    AgentRuntimeError::new(
        "session-not-started",
        format!("ACP session {session_id} has not started"),
    )
}

fn insert_prompt_turn_id(params: &mut Value, turn_id: &str) -> Result<(), AgentRuntimeError> {
    params
        .as_object_mut()
        .ok_or_else(|| {
            AgentRuntimeError::new(
                "serialization",
                "ACP prompt request did not serialize to an object",
            )
        })?
        .insert("turnId".to_string(), Value::String(turn_id.to_string()));
    Ok(())
}

fn extract_text_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(values) => {
            let text = values
                .iter()
                .filter_map(extract_text_value)
                .collect::<String>();
            (!text.is_empty()).then_some(text)
        }
        Value::Object(object) => {
            if object.get("type").and_then(Value::as_str) == Some("text") {
                return object
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::to_string);
            }
            object
                .get("text")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| object.get("content").and_then(extract_text_value))
        }
        _ => None,
    }
}

fn append_prompt_update(params: &Value, turn_id: &mut Option<String>, text: &mut String) {
    let update = params
        .get("update")
        .or_else(|| params.get("sessionUpdate"))
        .unwrap_or(params);
    if let Some(id) = update
        .get("turnId")
        .or_else(|| update.get("turn_id"))
        .and_then(Value::as_str)
    {
        *turn_id = Some(id.to_string());
    }
    if update
        .get("sessionUpdate")
        .or_else(|| update.get("session_update"))
        .or_else(|| update.get("type"))
        .and_then(Value::as_str)
        .is_some_and(|kind| {
            matches!(
                kind,
                "agent_message_chunk"
                    | "agent-message-chunk"
                    | "agent_message"
                    | "assistant_message_delta"
            )
        })
    {
        if let Some(chunk) = update.get("content").and_then(extract_text_value) {
            text.push_str(&chunk);
        }
        if let Some(message) = update.get("message").and_then(extract_text_value) {
            text.push_str(&message);
        }
    }
}

fn prompt_update_turn_id(params: &Value) -> Option<&str> {
    params
        .get("turnId")
        .or_else(|| params.get("turn_id"))
        .and_then(Value::as_str)
        .or_else(|| {
            params
                .get("update")
                .or_else(|| params.get("sessionUpdate"))
                .and_then(|update| {
                    update
                        .get("turnId")
                        .or_else(|| update.get("turn_id"))
                        .and_then(Value::as_str)
                })
        })
}

fn update_session_id(params: &Value) -> Option<&str> {
    params
        .get("sessionId")
        .or_else(|| params.get("session_id"))
        .and_then(Value::as_str)
}

fn resolve_update_session_id(
    sessions: &HashMap<SessionId, AcpSessionState>,
    params: &Value,
    turn_id: Option<&str>,
) -> Option<SessionId> {
    if let Some(session_id) = update_session_id(params) {
        return Some(session_id.to_string());
    }
    if let Some(turn_id) = turn_id {
        let mut matching_sessions = sessions
            .iter()
            .filter(|(_, state)| {
                state.prompt_updates.active.as_ref().is_some_and(|active| {
                    active.turn_id == turn_id
                        || active
                            .observed_turn_ids
                            .iter()
                            .any(|observed| observed == turn_id)
                }) || state
                    .prompt_updates
                    .completed_turn_ids
                    .iter()
                    .any(|completed| completed == turn_id)
            })
            .map(|(session_id, _)| session_id.clone());
        if let Some(only_match) = matching_sessions.next() {
            if matching_sessions.next().is_none() {
                return Some(only_match);
            }
        }
    }
    let mut active_sessions = sessions
        .iter()
        .filter(|(_, state)| state.prompt_updates.active.is_some())
        .map(|(session_id, _)| session_id.clone());
    if let Some(only_active) = active_sessions.next() {
        if active_sessions.next().is_none() {
            return Some(only_active);
        }
    }
    (sessions.len() == 1)
        .then(|| sessions.keys().next().cloned())
        .flatten()
}

fn available_commands_update(params: &Value) -> Option<Vec<AgentCommandDescriptor>> {
    let update = params
        .get("update")
        .or_else(|| params.get("sessionUpdate"))
        .unwrap_or(params);
    update
        .get("sessionUpdate")
        .or_else(|| update.get("session_update"))
        .or_else(|| update.get("type"))
        .and_then(Value::as_str)
        .filter(|kind| *kind == "available_commands_update")?;
    Some(parse_command_descriptors(
        update
            .get("availableCommands")
            .or_else(|| update.get("available_commands")),
    ))
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

fn bool_at(value: &Value, path: &[&str]) -> bool {
    path.iter()
        .try_fold(value, |current, part| current.get(*part))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn capability_at(value: &Value, path: &[&str]) -> bool {
    match path
        .iter()
        .try_fold(value, |current, part| current.get(*part))
    {
        Some(Value::Bool(enabled)) => *enabled,
        Some(Value::Object(_)) => true,
        _ => false,
    }
}

fn parse_config_options(value: Option<&Value>) -> Vec<AgentConfigOption> {
    value
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default()
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

fn parse_conversation_config(
    value: Option<&Value>,
) -> Result<AgentConversationConfigState, AgentRuntimeError> {
    value
        .map(|value| {
            serde_json::from_value(value.clone()).map_err(|error| {
                AgentRuntimeError::new(
                    "invalid-response",
                    format!("ACP session configuration is invalid: {error}"),
                )
            })
        })
        .transpose()
        .map(Option::unwrap_or_default)
}

fn parse_standard_conversation_config(
    value: &Value,
    provider: Option<AgentConversationProvider>,
) -> AgentConversationConfigState {
    let models = value.get("models");
    let modes = value.get("modes");
    let mut config = AgentConversationConfigState {
        model: models
            .and_then(|models| models.get("currentModelId"))
            .and_then(Value::as_str)
            .map(str::to_string),
        available_models: models
            .and_then(|models| models.get("availableModels"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|model| model.get("modelId").and_then(Value::as_str))
            .map(str::to_string)
            .collect(),
        reasoning_effort: None,
        available_efforts: Vec::new(),
        approval_policy: modes
            .and_then(|modes| modes.get("currentModeId"))
            .and_then(Value::as_str)
            .map(str::to_string),
        available_approval_policies: modes
            .and_then(|modes| modes.get("availableModes"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|mode| mode.get("id").and_then(Value::as_str))
            .map(str::to_string)
            .collect(),
    };
    if provider == Some(AgentConversationProvider::Claude) {
        for model_id in CLAUDE_VERIFIED_EXTRA_MODELS {
            if !config
                .available_models
                .iter()
                .any(|available| available == model_id)
            {
                config.available_models.push(model_id.to_string());
            }
        }
    }
    config
}

fn serialization_error(error: serde_json::Error) -> AgentRuntimeError {
    AgentRuntimeError::new("serialization", error.to_string())
}
fn transport_error(message: String) -> AgentRuntimeError {
    AgentRuntimeError::new("transport", message)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use serde_json::json;

    use super::*;
    use crate::agent_conversation::protocol::{ProviderSource, ProviderTransport};
    use crate::agent_conversation::providers::AgentRuntimeAdapter;

    fn fixture_manifest(log_path: &Path) -> AgentProviderManifest {
        fixture_manifest_named(log_path, "default")
    }

    pub(crate) fn fixture_manifest_named(log_path: &Path, fixture: &str) -> AgentProviderManifest {
        let script = format!(
            r#"log={log}
fixture={fixture}
session_count=0
if [ "${{MAX_THINKING_TOKENS+x}}" = "x" ]; then
  printf 'MAX_THINKING_TOKENS=%s\n' "$MAX_THINKING_TOKENS" >> "$log"
else
  printf 'MAX_THINKING_TOKENS=<unset>\n' >> "$log"
fi
while IFS= read -r line; do
  printf '%s\n' "$line" >> "$log"
  id=$(printf '%s\n' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
	  *'"method":"initialize"'*)
	    if [ "$fixture" = "multiplex" ]; then
	      printf '{{"jsonrpc":"2.0","id":%s,"result":{{"agentInfo":{{"name":"fake-acp","version":"1"}},"agentCapabilities":{{"loadSession":true,"sessionCapabilities":{{"resume":true,"close":true,"multiSession":true}},"promptCapabilities":{{"image":true}}}}}}}}\n' "$id"
	    elif [ "${{fixture#suspend_}}" != "$fixture" ] && [ "$fixture" != "suspend_no_resume" ]; then
	      printf '{{"jsonrpc":"2.0","id":%s,"result":{{"agentInfo":{{"name":"fake-acp","version":"1"}},"agentCapabilities":{{"loadSession":true,"sessionCapabilities":{{"resume":{{}}}},"promptCapabilities":{{"image":true}}}}}}}}\n' "$id"
	    else
	      printf '{{"jsonrpc":"2.0","id":%s,"result":{{"agentInfo":{{"name":"fake-acp","version":"1"}},"agentCapabilities":{{"loadSession":true,"promptCapabilities":{{"image":true}}}}}}}}\n' "$id"
	    fi ;;
    *'"method":"session/new"'*)
      if [ "$fixture" = "multiplex" ]; then
        session_count=$((session_count + 1))
        session_id="session-$session_count"
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"%s","availableCommands":[{{"name":"initial-%s"}}]}}}}\n' "$id" "$session_id" "$session_count"
      elif [ "$fixture" = "command_capture" ]; then
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"new-session","availableCommands":[{{"name":"initial","description":"Initial command","input":{{"hint":"path"}}}}]}}}}\n' "$id"
        printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"available_commands_update","availableCommands":[{{"name":"updated","description":"Updated command"}}]}}}}}}\n'
        printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"usage_update","used":120,"size":4096}}}}}}\n'
      elif [ "$fixture" = "config_update_failure" ]; then
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"new-session","_meta":{{"availableEfforts":["low","medium","high","xhigh","max"]}}}}}}\n' "$id"
      elif [ "$fixture" = "standard_config" ]; then
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"new-session","models":{{"availableModels":[{{"modelId":"default","name":"Default (recommended)","description":"Opus 4.6 · Most capable for complex work"}},{{"modelId":"sonnet","name":"Sonnet","description":"Sonnet 4.5 · Best for everyday tasks"}},{{"modelId":"haiku","name":"Haiku","description":"Haiku 4.5 · Fastest for quick answers"}}],"currentModelId":"default"}},"modes":{{"currentModeId":"default","availableModes":[{{"id":"default","name":"Default","description":"Standard behavior, prompts for dangerous operations"}},{{"id":"acceptEdits","name":"Accept Edits","description":"Auto-accept file edit operations"}},{{"id":"plan","name":"Plan Mode","description":"Planning mode, no actual tool execution"}},{{"id":"dontAsk","name":"Dont Ask","description":"Deny operations that are not pre-approved"}},{{"id":"bypassPermissions","name":"Bypass Permissions","description":"Bypass all permission checks"}}]}}}}}}\n' "$id"
      else
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"new-session","_meta":{{"model":"gpt-5.6-sol","availableModels":["gpt-5.6-sol","gpt-5.6-terra","gpt-5.6-luna"],"reasoningEffort":"high","availableEfforts":["low","medium","high","xhigh","max"],"approvalPolicy":"on-request","availableApprovalPolicies":["untrusted","on-request","never"]}}}}}}\n' "$id"
      fi ;;
    *'"method":"session/load"'*)
      if [ "$fixture" = "replay_on_load" ]; then
        printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"user_message_chunk","messageId":"history-user-1","content":{{"type":"text","text":"First question"}},"turnId":"history-turn-1","_meta":{{"replay":true}}}}}}}}\n'
        printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"agent_message_chunk","messageId":"history-agent-1","content":{{"type":"text","text":"First answer"}},"turnId":"history-turn-1","_meta":{{"replay":true}}}}}}}}\n'
        printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"user_message_chunk","messageId":"history-user-2","content":{{"type":"text","text":"Second question"}},"turnId":"history-turn-2","_meta":{{"replay":true}}}}}}}}\n'
        printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"agent_message_chunk","messageId":"history-agent-2","content":{{"type":"text","text":"Second answer"}},"turnId":"history-turn-2","_meta":{{"replay":true}}}}}}}}\n'
        printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"agent_message_chunk","messageId":"unmarked-update","content":{{"type":"text","text":"Must stay dropped"}},"turnId":"history-turn-2"}}}}}}\n'
      fi
	      printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"loaded-session","_meta":{{"model":"gpt-5.6-sol","availableModels":["gpt-5.6-sol","gpt-5.6-terra","gpt-5.6-luna"],"reasoningEffort":"high","availableEfforts":["low","medium","high","xhigh","max"],"approvalPolicy":"on-request","availableApprovalPolicies":["untrusted","on-request","never"]}}}}}}\n' "$id" ;;
	    *'"method":"session/resume"'*)
	      if [ "$fixture" = "resume_failure" ]; then
	        printf '{{"jsonrpc":"2.0","id":%s,"error":{{"code":-32001,"message":"fixture resume failed"}}}}\n' "$id"
	        continue
	      elif [ "$fixture" = "replay_on_resume" ]; then
	        printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"agent_message_chunk","messageId":"historical-message","content":{{"type":"text","text":"historical answer"}},"turnId":"historical-turn","_meta":{{"replay":true}}}}}}}}\n'
	      elif [ "${{fixture#suspend_}}" != "$fixture" ]; then
	        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"new-session","_meta":{{"model":"gpt-5.6-sol","availableModels":["gpt-5.6-sol","gpt-5.6-terra","gpt-5.6-luna"],"reasoningEffort":"high","availableEfforts":["low","medium","high","xhigh","max"],"approvalPolicy":"on-request","availableApprovalPolicies":["untrusted","on-request","never"]}}}}}}\n' "$id"
	        continue
	      fi
		      printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"resumed-session","_meta":{{"model":"gpt-5.6-sol","availableModels":["gpt-5.6-sol","gpt-5.6-terra","gpt-5.6-luna"],"reasoningEffort":"high","availableEfforts":["low","medium","high","xhigh","max"],"approvalPolicy":"on-request","availableApprovalPolicies":["untrusted","on-request","never"]}}}}}}\n' "$id" ;;
    *'"method":"session/prompt"'*)
      turn=$(printf '%s\n' "$line" | sed -n 's/.*"turnId":"\([^"]*\)".*/\1/p')
      session_id=$(printf '%s\n' "$line" | sed -n 's/.*"sessionId":"\([^"]*\)".*/\1/p')
      if [ -z "$turn" ]; then turn="turn-1"; fi
      if [ "$fixture" = "multiplex" ]; then
        if [ "$session_id" = "session-1" ]; then
          (
            sleep 0.03
            printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"sessionId":"session-1","update":{{"sessionUpdate":"agent_message_chunk","content":{{"type":"text","text":"AL"}},"turnId":"%s"}}}}}}\n' "$turn"
            sleep 0.03
            printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"sessionId":"session-1","update":{{"sessionUpdate":"agent_message_chunk","content":{{"type":"text","text":"PHA"}},"turnId":"%s"}}}}}}\n' "$turn"
            printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"%s","stopReason":"end_turn"}}}}\n' "$id" "$turn"
          ) &
        else
          (
            sleep 0.01
            printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"sessionId":"session-2","update":{{"sessionUpdate":"agent_message_chunk","content":{{"type":"text","text":"BR"}},"turnId":"%s"}}}}}}\n' "$turn"
            printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"sessionId":"session-2","update":{{"sessionUpdate":"available_commands_update","availableCommands":[{{"name":"updated-2"}}]}}}}}}\n'
            sleep 0.01
            printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"sessionId":"session-2","update":{{"sessionUpdate":"agent_message_chunk","content":{{"type":"text","text":"AVO"}},"turnId":"%s"}}}}}}\n' "$turn"
            printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"%s","stopReason":"end_turn"}}}}\n' "$id" "$turn"
          ) &
        fi
      elif [ "$fixture" = "dies_midturn" ]; then
        exit 0
      elif [ "$fixture" = "pending_drop" ]; then
        sleep 1
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"turn-1","stopReason":"end_turn"}}}}\n' "$id"
      elif [ "$fixture" = "direct_result" ]; then
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"turn-direct","text":"direct response text"}}}}\n' "$id"
      elif [ "$fixture" = "current_mode_update" ]; then
        printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"sessionId":"new-session","update":{{"sessionUpdate":"current_mode_update","currentModeId":"plan"}}}}}}\n'
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"%s","stopReason":"end_turn"}}}}\n' "$id" "$turn"
      elif [ "$fixture" = "prompt_error" ]; then
        printf '{{"jsonrpc":"2.0","id":%s,"error":{{"code":-32001,"message":"fixture prompt failed"}}}}\n' "$id"
      elif [ "$fixture" = "many_updates" ]; then
        i=0
        while [ "$i" -lt 10000 ]; do
          printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"agent_message_chunk","content":{{"type":"text","text":"x"}},"turnId":"%s"}}}}}}\n' "$turn"
          i=$((i + 1))
        done
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"%s","stopReason":"end_turn"}}}}\n' "$id" "$turn"
      elif [ "$fixture" = "late_one_shot_update" ]; then
        agent_turn="agent-turn-9"
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"%s","text":"generated text"}}}}\n' "$id" "$agent_turn"
        sleep 0.05
        printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"agent_message_chunk","content":{{"type":"text","text":"late text"}},"turnId":"%s"}}}}}}\n' "$agent_turn"
      elif [ "$fixture" = "agent_minted_in_flight_update" ]; then
        printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"agent_message_chunk","content":{{"type":"text","text":"agent-minted text"}},"turnId":"agent-turn-9"}}}}}}\n'
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"%s","stopReason":"end_turn"}}}}\n' "$id" "$turn"
      elif [ "$fixture" = "permission_prompt_error" ]; then
        printf '{{"jsonrpc":"2.0","id":77,"method":"session/request_permission","params":{{"title":"Approval before failed prompt","options":[{{"optionId":"allow","name":"Allow","kind":"allow_once"}}]}}}}\n'
        printf '{{"jsonrpc":"2.0","id":%s,"error":{{"code":-32001,"message":"fixture prompt failed"}}}}\n' "$id"
        exit 0
      elif [ "$fixture" = "permission_midturn" ]; then
        printf '{{"jsonrpc":"2.0","id":77,"method":"session/request_permission","params":{{"options":[{{"optionId":"allow","name":"Allow","kind":"allow_once"}},{{"optionId":"reject","name":"Reject","kind":"reject_once"}}]}}}}\n'
        while IFS= read -r response; do
          printf '%s\n' "$response" >> "$log"
          case "$response" in
	            *'"id":77'*)
	              if printf '%s' "$response" | grep -q '\"outcome\":{{\"outcome\":\"selected\",\"optionId\":\"allow\"'; then
	                printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"agent_message_chunk","messageId":"after-approval","content":{{"type":"text","text":"continued after approval"}},"turnId":"%s"}}}}}}\n' "$turn"
	                printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"turn-1","stopReason":"end_turn"}}}}\n' "$id"
	              elif printf '%s' "$response" | grep -q '\"outcome\":{{\"outcome\":\"selected\",\"optionId\":\"reject\"'; then
	                printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"agent_message_chunk","messageId":"after-approval","content":{{"type":"text","text":"continued after approval"}},"turnId":"%s"}}}}}}\n' "$turn"
	                printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"turn-1","stopReason":"end_turn"}}}}\n' "$id"
              else
                printf '{{"jsonrpc":"2.0","id":%s,"error":{{"code":-32000,"message":"invalid permission option"}}}}\n' "$id"
              fi
              break ;;
          esac
        done
      elif [ "$fixture" = "permission_cancelled" ]; then
        printf '{{"jsonrpc":"2.0","id":77,"method":"session/request_permission","params":{{"options":[{{"optionId":"allow","name":"Allow","kind":"allow_once"}},{{"optionId":"reject","name":"Reject","kind":"reject_once"}}]}}}}\n'
        while IFS= read -r response; do
          printf '%s\n' "$response" >> "$log"
          case "$response" in
            *'"method":"session/cancel"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"turn-cancelled","stopReason":"cancelled"}}}}\n' "$id"; break ;;
          esac
        done
      elif [ "$fixture" = "permission_dies" ]; then
        printf '{{"jsonrpc":"2.0","id":77,"method":"session/request_permission","params":{{"title":"Approval before transport exit","options":[{{"optionId":"allow","name":"Allow","kind":"allow_once"}}]}}}}\n'
        exit 0
      elif [ "$fixture" = "cancelled_turn" ]; then
        while IFS= read -r response; do
          printf '%s\n' "$response" >> "$log"
          case "$response" in
            *'"method":"session/cancel"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"turn-cancelled","stopReason":"cancelled"}}}}\n' "$id"; break ;;
          esac
        done
      else
        printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"agent_message_chunk","content":{{"type":"text","text":"generated text"}},"turnId":"%s"}}}}}}\n' "$turn"
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"%s","stopReason":"end_turn"}}}}\n' "$id" "$turn"
      fi ;;
	    *'"method":"session/set_config_option"'*)
      if [ "$fixture" = "config_update_failure" ]; then
        printf '{{"jsonrpc":"2.0","id":%s,"error":{{"code":-32002,"message":"fixture config update failed"}}}}\n' "$id"
      else
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"model":"gpt-5.6-terra","availableModels":["gpt-5.6-sol","gpt-5.6-terra","gpt-5.6-luna"],"reasoningEffort":"xhigh","availableEfforts":["low","medium","high","xhigh","max"],"approvalPolicy":"never","availableApprovalPolicies":["untrusted","on-request","never"],"configOptions":[{{"id":"model","label":"Model","category":"model","value":"new"}}]}}}}\n' "$id"
      fi ;;
    *'"method":"session/set_model"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{}}}}\n' "$id" ;;
    *'"method":"session/set_mode"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{}}}}\n' "$id" ;;
    *'"method":"session/steer"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{}}}}\n' "$id" ;;
    *'"method":"session/close"'*)
      printf '{{"jsonrpc":"2.0","id":%s,"result":{{}}}}\n' "$id"
      if [ "$fixture" != "multiplex" ]; then exit 0; fi ;;
  esac
done"#,
            log = shell_quote(log_path),
            fixture = fixture
        );
        AgentProviderManifest {
            id: "fake-acp".into(),
            display_name: "Fake ACP".into(),
            transport: ProviderTransport::AcpStdio,
            executable: PathBuf::from("/bin/sh"),
            args: vec!["-c".into(), script],
            version: "1.0.0".into(),
            content_hash: "0000000000000000000000000000000000000000000000000000000000000000".into(),
            trusted_source: ProviderSource::Bundled,
        }
    }

    fn fixture_process(name: &str) -> SidecarProcess {
        let root = fixture_root();
        let log = root.join(format!("{name}.jsonl"));
        SidecarProcess::spawn(&fixture_manifest_named(&log, name), &root, name).unwrap()
    }

    fn shell_quote(path: &Path) -> String {
        format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
    }

    fn fixture_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("mcb-fake-acp-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn prompt_turn_id_insertion_rejects_non_object_params() {
        let mut params = Value::Null;

        let error = insert_prompt_turn_id(&mut params, "turn-1").unwrap_err();

        assert_eq!(error.code, "serialization");
        assert_eq!(params, Value::Null);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn transport_routes_updates_and_responses_concurrently() {
        let (transport, mut inbound) = AcpTransport::start(fixture_process("prompt_with_update"));
        let response = transport
            .request("session/prompt", json!({"sessionId": "s"}))
            .await
            .expect("prompt response");
        assert_eq!(
            response.get("stopReason").and_then(|v| v.as_str()),
            Some("end_turn")
        );
        match inbound.recv().await.expect("inbound") {
            AcpInbound::SessionUpdate(params) => {
                assert_eq!(params["update"]["sessionUpdate"], "agent_message_chunk");
            }
            other => panic!("expected SessionUpdate, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn agent_requests_surface_on_the_inbound_channel_and_can_be_answered() {
        let (transport, mut inbound) = AcpTransport::start(fixture_process("permission_midturn"));
        let prompt = tokio::spawn({
            let t = transport.clone();
            async move { t.request("session/prompt", json!({"sessionId": "s"})).await }
        });
        let (wire_id, method) = match inbound.recv().await.expect("inbound") {
            AcpInbound::AgentRequest {
                wire_id, method, ..
            } => (wire_id, method),
            other => panic!("expected AgentRequest, got {other:?}"),
        };
        assert_eq!(method, "session/request_permission");
        transport
            .respond(
                wire_id,
                json!({"outcome": {"outcome": "selected", "optionId": "allow"}}),
            )
            .await
            .expect("respond");
        prompt
            .await
            .expect("join")
            .expect("prompt completes after permission answered");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn transport_close_fails_pending_requests_and_notifies() {
        let (transport, mut inbound) = AcpTransport::start(fixture_process("dies_midturn"));
        let error = transport
            .request("session/prompt", json!({"sessionId": "s"}))
            .await
            .expect_err("must fail");
        assert!(error.to_string().contains("transport"), "{error}");
        assert!(matches!(
            inbound.recv().await,
            Some(AcpInbound::TransportClosed { .. })
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn dropped_request_future_removes_pending_entry() {
        let (transport, _inbound) = AcpTransport::start(fixture_process("pending_drop"));
        let request = tokio::spawn({
            let transport = transport.clone();
            async move {
                transport
                    .request("session/prompt", json!({"sessionId": "s"}))
                    .await
            }
        });
        let deadline = Instant::now() + Duration::from_secs(1);
        while transport.pending.lock().unwrap().is_empty() {
            assert!(Instant::now() < deadline, "request was not registered");
            tokio::task::yield_now().await;
        }
        request.abort();
        let _ = request.await;
        assert!(transport.pending.lock().unwrap().is_empty());
        transport.stop().await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn request_after_transport_eof_fails_immediately() {
        let (transport, mut inbound) = AcpTransport::start(fixture_process("dies_midturn"));
        let _ = transport
            .request("session/prompt", json!({"sessionId": "s"}))
            .await
            .expect_err("initial request must fail on EOF");
        assert!(matches!(
            inbound.recv().await,
            Some(AcpInbound::TransportClosed { .. })
        ));
        let result = tokio::time::timeout(
            Duration::from_millis(100),
            transport.request("session/prompt", json!({"sessionId": "s"})),
        )
        .await
        .expect("post-EOF request must not wait");
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn prompt_once_reads_text_directly_from_response_result() {
        let root = fixture_root();
        let log = root.join("direct.jsonl");
        let mut client = AcpClient::spawn(
            &fixture_manifest_named(&log, "direct_result"),
            &root,
            "direct",
        )
        .unwrap();
        client
            .initialize(AgentConversationProvider::Codex)
            .await
            .unwrap();
        let session = client.new_session(&root).await.unwrap();
        let generated = client
            .prompt_on(
                &session.native_session_id,
                AgentPrompt {
                    text: "write".into(),
                    images: Vec::new(),
                    attachment_ids: Vec::new(),
                },
            )
            .await
            .unwrap();
        assert_eq!(generated.turn_id.as_deref(), Some("turn-direct"));
        assert_eq!(generated.text, "direct response text");
        client.close().await.unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn prompt_once_aggregates_more_than_256_updates_without_loss() {
        let root = fixture_root();
        let log = root.join("many.jsonl");
        let mut client =
            AcpClient::spawn(&fixture_manifest_named(&log, "many_updates"), &root, "many").unwrap();
        client
            .initialize(AgentConversationProvider::Codex)
            .await
            .unwrap();
        let session = client.new_session(&root).await.unwrap();
        let generated = client
            .prompt_on(
                &session.native_session_id,
                AgentPrompt {
                    text: "write".into(),
                    images: Vec::new(),
                    attachment_ids: Vec::new(),
                },
            )
            .await
            .unwrap();
        assert!(
            generated
                .turn_id
                .as_deref()
                .is_some_and(|turn_id| turn_id.starts_with("turn-")),
            "one-shot turn id should be client-minted"
        );
        assert_eq!(generated.text, "x".repeat(10000));
        client.close().await.unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn acp_initialize_new_config_one_shot_and_close() {
        let root = fixture_root();
        let log = root.join("frames.jsonl");
        let mut adapter = super::super::AcpRuntimeAdapter::new(
            fixture_manifest(&log),
            "owned-a".into(),
            root.clone(),
        );
        let capabilities = adapter
            .initialize(super::super::InitializeAgentInput {
                provider: AgentConversationProvider::Codex,
            })
            .await
            .unwrap();
        assert!(capabilities.prompt.image);
        let started = adapter
            .new_session(super::super::NewAgentSession { cwd: root.clone() })
            .await
            .unwrap();
        assert_eq!(started.native_session_id, "new-session");
        let generated = adapter
            .prompt_once_on(
                &started.native_session_id,
                super::super::AgentPrompt {
                    text: "write a commit subject".into(),
                    images: Vec::new(),
                    attachment_ids: Vec::new(),
                },
            )
            .await
            .unwrap();
        assert!(
            generated
                .turn_id
                .as_deref()
                .is_some_and(|turn_id| turn_id.starts_with("turn-")),
            "one-shot turn id should be client-minted"
        );
        assert_eq!(generated.text, "generated text");
        let options = adapter
            .set_config_on(&started.native_session_id, "model", json!("new"))
            .await
            .unwrap();
        assert_eq!(options[0].value, json!("new"));
        let pid = adapter.process_id().unwrap();
        adapter.close_session().await.unwrap();
        assert_ne!(
            unsafe { libc::kill(pid as i32, 0) },
            0,
            "fake ACP parent process survived close"
        );

        let frames = std::fs::read_to_string(&log).unwrap();
        assert!(frames.contains("session/new"));
        assert!(frames.contains("session/prompt"));
        assert!(frames.contains("session/set_config_option"));
        assert!(frames.contains("session/close"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn acp_new_maps_and_sets_standard_session_config() {
        let root = fixture_root();
        let log = root.join("standard-config.jsonl");
        let mut client = AcpClient::spawn(
            &fixture_manifest_named(&log, "standard_config"),
            &root,
            "standard-config",
        )
        .unwrap();
        client
            .initialize(AgentConversationProvider::Claude)
            .await
            .unwrap();

        let started = client.new_session(&root).await.unwrap();

        assert_eq!(
            started.config,
            AgentConversationConfigState {
                model: Some("default".into()),
                available_models: vec![
                    "default".into(),
                    "sonnet".into(),
                    "haiku".into(),
                    "opus".into(),
                    "claude-opus-5".into(),
                    "claude-fable-5".into(),
                ],
                reasoning_effort: None,
                available_efforts: Vec::new(),
                approval_policy: Some("default".into()),
                available_approval_policies: vec![
                    "default".into(),
                    "acceptEdits".into(),
                    "plan".into(),
                    "dontAsk".into(),
                    "bypassPermissions".into(),
                ],
            }
        );

        let configured = client
            .set_conversation_config_on(
                &started.native_session_id,
                &AgentConversationConfigUpdate {
                    model: Some("claude-fable-5".into()),
                    reasoning_effort: None,
                    approval_policy: Some("bypassPermissions".into()),
                },
            )
            .await
            .unwrap();

        assert_eq!(configured.model.as_deref(), Some("claude-fable-5"));
        assert_eq!(
            configured.approval_policy.as_deref(),
            Some("bypassPermissions")
        );
        client.close().await.unwrap();
        let frames = std::fs::read_to_string(&log).unwrap();
        assert!(frames.contains(r#""method":"session/set_model""#));
        assert!(frames.contains(r#""modelId":"claude-fable-5""#));
        assert!(frames.contains(r#""method":"session/set_mode""#));
        assert!(frames.contains(r#""modeId":"bypassPermissions""#));
        assert!(!frames.contains("session/set_config_option"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parses_initial_command_descriptors_from_a_session_result() {
        let commands = parse_command_descriptors(Some(&json!([
            { "name": "review", "description": "Review the change", "input": { "hint": "path" } },
            { "id": "status", "label": "Status" }
        ])));
        assert_eq!(
            commands,
            vec![
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
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn captures_initial_commands_and_forwards_the_update_notification() {
        let root = fixture_root();
        let log = root.join("commands.jsonl");
        let mut client = AcpClient::spawn(
            &fixture_manifest_named(&log, "command_capture"),
            &root,
            "commands",
        )
        .unwrap();
        client
            .initialize(AgentConversationProvider::Codex)
            .await
            .unwrap();
        let started = client.new_session(&root).await.unwrap();
        assert_eq!(started.commands[0].id, "initial");
        let mut inbound = client.take_inbound().unwrap();
        let update = inbound.recv().await.expect("command update");
        match update {
            AcpInbound::SessionUpdate(params) => {
                assert_eq!(
                    params["update"]["sessionUpdate"],
                    "available_commands_update"
                );
                assert_eq!(params["update"]["availableCommands"][0]["name"], "updated");
            }
            other => panic!("expected command update, got {other:?}"),
        }
        client.close().await.unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn acp_resume_uses_resume_session_method() {
        let root = fixture_root();
        let log = root.join("resume.jsonl");
        let mut client = AcpClient::spawn(&fixture_manifest(&log), &root, "resume").unwrap();
        client
            .initialize(AgentConversationProvider::Claude)
            .await
            .unwrap();
        let started = client.resume_session(&root, "native-old").await.unwrap();
        assert_eq!(started.native_session_id, "resumed-session");
        let resumed = client
            .resume_session_multi(&root, "native-second")
            .await
            .unwrap();
        assert_eq!(resumed, "resumed-session");
        client.close().await.unwrap();
        let frames = std::fs::read_to_string(log).unwrap();
        assert!(frames.contains("session/resume"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn acp_load_uses_load_session_method() {
        let root = fixture_root();
        let log = root.join("load.jsonl");
        let mut client = AcpClient::spawn(&fixture_manifest(&log), &root, "load").unwrap();
        client
            .initialize(AgentConversationProvider::Codex)
            .await
            .unwrap();
        let started = client.load_session(&root, "native-old").await.unwrap();
        assert_eq!(started.native_session_id, "loaded-session");
        client.close().await.unwrap();
        let frames = std::fs::read_to_string(log).unwrap();
        assert!(frames.contains("session/load"));
        assert!(!frames.contains("session/resume"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn two_sessions_on_one_process_route_independently() {
        let root = fixture_root();
        let log = root.join("multiplex.jsonl");
        let mut client =
            AcpClient::spawn(&fixture_manifest_named(&log, "multiplex"), &root, "mux").unwrap();
        client
            .initialize(AgentConversationProvider::Codex)
            .await
            .unwrap();
        let session_a = client.new_session_multi(&root).await.unwrap();
        let session_b = client.new_session_multi(&root).await.unwrap();

        let (alpha, bravo) = tokio::join!(
            client.prompt_on(
                &session_a,
                AgentPrompt {
                    text: "ALPHA".into(),
                    images: Vec::new(),
                    attachment_ids: Vec::new(),
                }
            ),
            client.prompt_on(
                &session_b,
                AgentPrompt {
                    text: "BRAVO".into(),
                    images: Vec::new(),
                    attachment_ids: Vec::new(),
                }
            )
        );

        assert_eq!(alpha.unwrap().text, "ALPHA");
        assert_eq!(bravo.unwrap().text, "BRAVO");
        client.close().await.unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn second_session_new_does_not_clobber_primary() {
        let root = fixture_root();
        let log = root.join("primary.jsonl");
        let mut client =
            AcpClient::spawn(&fixture_manifest_named(&log, "multiplex"), &root, "primary").unwrap();
        client
            .initialize(AgentConversationProvider::Codex)
            .await
            .unwrap();
        let primary = client.new_session(&root).await.unwrap();
        let second = client.new_session_multi(&root).await.unwrap();
        assert_ne!(primary.native_session_id, second);
        let config = client.set_model_on(&second, "new").await.unwrap();
        assert_eq!(config.model.as_deref(), Some("gpt-5.6-terra"));

        let generated = client
            .prompt_on(
                &primary.native_session_id,
                AgentPrompt {
                    text: "primary".into(),
                    images: Vec::new(),
                    attachment_ids: Vec::new(),
                },
            )
            .await
            .unwrap();

        assert_eq!(generated.text, "ALPHA");
        client.close().await.unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn per_session_command_capture() {
        let root = fixture_root();
        let log = root.join("commands-multiplex.jsonl");
        let mut client = AcpClient::spawn(
            &fixture_manifest_named(&log, "multiplex"),
            &root,
            "commands-multiplex",
        )
        .unwrap();
        client
            .initialize(AgentConversationProvider::Codex)
            .await
            .unwrap();
        let session_a = client.new_session_multi(&root).await.unwrap();
        let session_b = client.new_session_multi(&root).await.unwrap();
        client
            .prompt_on(
                &session_b,
                AgentPrompt {
                    text: "commands".into(),
                    images: Vec::new(),
                    attachment_ids: Vec::new(),
                },
            )
            .await
            .unwrap();

        assert_eq!(client.commands_on(&session_a).unwrap()[0].id, "initial-1");
        assert_eq!(client.commands_on(&session_b).unwrap()[0].id, "updated-2");
        client.close().await.unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn close_one_session_keeps_the_other_alive() {
        let root = fixture_root();
        let log = root.join("close-one.jsonl");
        let mut client = AcpClient::spawn(
            &fixture_manifest_named(&log, "multiplex"),
            &root,
            "close-one",
        )
        .unwrap();
        client
            .initialize(AgentConversationProvider::Codex)
            .await
            .unwrap();
        let session_a = client.new_session_multi(&root).await.unwrap();
        let session_b = client.new_session_multi(&root).await.unwrap();

        client.close_session(&session_a).await.unwrap();
        let generated = client
            .prompt_on(
                &session_b,
                AgentPrompt {
                    text: "still alive".into(),
                    images: Vec::new(),
                    attachment_ids: Vec::new(),
                },
            )
            .await
            .unwrap();

        assert_eq!(generated.text, "BRAVO");
        client.close().await.unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
