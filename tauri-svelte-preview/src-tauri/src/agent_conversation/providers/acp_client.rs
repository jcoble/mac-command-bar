use std::collections::{BTreeMap, HashMap, VecDeque};
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
    AgentCapabilities, AgentCommandDescriptor, AgentConfigOption, AgentConversationProvider,
    AgentImplementation, AgentInteractionCapabilities, AgentPromptCapabilities,
    AgentProviderManifest, AgentSessionCapabilities,
};
use super::process::{SidecarProcess, SidecarProcessHandle, SidecarReadHalf, SidecarWriteHalf};
use super::{AgentPrompt, AgentRuntimeError, GeneratedText, StartedAgentSession, StartedTurn};

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
    prompt_updates: Mutex<PromptUpdateRoute>,
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
            prompt_updates: Mutex::new(PromptUpdateRoute::default()),
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

    pub fn stderr_snapshot(&self) -> Vec<String> {
        self.process
            .lock()
            .ok()
            .and_then(|value| value.as_ref().map(SidecarProcessHandle::stderr_snapshot))
            .unwrap_or_default()
    }

    pub fn process_id(&self) -> Option<u32> {
        self.process
            .lock()
            .ok()
            .and_then(|value| value.as_ref().and_then(SidecarProcessHandle::process_id))
    }

    fn register_prompt_updates(
        self: &Arc<Self>,
        turn_id: String,
    ) -> Result<(mpsc::UnboundedReceiver<Value>, PromptUpdateRegistration), AgentRuntimeError> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut prompt_updates = self
            .prompt_updates
            .lock()
            .map_err(|_| transport_error("prompt update queue is unavailable".to_string()))?;
        if prompt_updates.active.is_some() {
            return Err(transport_error(
                "ACP prompt aggregation is already active".to_string(),
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
                turn_id,
                active: true,
            },
        ))
    }

    fn unregister_prompt_updates(&self, turn_id: &str, response_turn_id: Option<&str>) {
        if let Ok(mut prompt_updates) = self.prompt_updates.lock() {
            let Some(active) = prompt_updates.active.take() else {
                return;
            };
            if active.turn_id != turn_id {
                prompt_updates.active = Some(active);
                return;
            }
            Self::remember_completed_turn_id(&mut prompt_updates, turn_id);
            if let Some(response_turn_id) = response_turn_id {
                Self::remember_completed_turn_id(&mut prompt_updates, response_turn_id);
            }
            for observed_turn_id in active.observed_turn_ids {
                Self::remember_completed_turn_id(&mut prompt_updates, &observed_turn_id);
            }
        }
    }

    /// Route every update to an active one-shot aggregation queue. Returning
    /// true tells the reader that this frame was consumed and must not enter
    /// the conversation pump. Completed one-shot turn identities are retained
    /// long enough to drop late updates after the queue is unregistered.
    fn send_prompt_update(&self, params: Value) -> bool {
        let turn_id = prompt_update_turn_id(&params).map(str::to_string);
        if let Ok(mut prompt_updates) = self.prompt_updates.lock() {
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
                    eprintln!(
                        "[debug] Dropping late ACP one-shot update for completed turn {turn_id}"
                    );
                    return true;
                }
            }
        }
        false
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
    turn_id: String,
    active: bool,
}

impl PromptUpdateRegistration {
    fn finish(&mut self, response_turn_id: Option<&str>) {
        if self.active {
            self.transport
                .unregister_prompt_updates(&self.turn_id, response_turn_id);
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
                    eprintln!("Unmatched ACP response id: {id}");
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
                    if transport.send_prompt_update(params.clone()) {
                        continue;
                    }
                }
                let _ = inbound_tx.send(AcpInbound::SessionUpdate(params));
            }
            (false, Some(method)) => {
                eprintln!("Ignoring unsupported ACP notification: {method}");
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
    native_session_id: Option<String>,
}

impl AcpClient {
    pub fn spawn(
        manifest: &AgentProviderManifest,
        cwd: &Path,
        owned_id: &str,
    ) -> Result<Self, AgentRuntimeError> {
        let process = SidecarProcess::spawn(manifest, cwd, owned_id)
            .map_err(|message| AgentRuntimeError::new("sidecar-spawn", message))?;
        let (transport, inbound) = AcpTransport::start(process);
        Ok(Self {
            transport,
            inbound: Some(inbound),
            native_session_id: None,
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
        Ok(AgentCapabilities {
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
                list: bool_at(
                    &result,
                    &["agentCapabilities", "sessionCapabilities", "list"],
                ),
                load: bool_at(&result, &["agentCapabilities", "loadSession"]),
                resume: bool_at(
                    &result,
                    &["agentCapabilities", "sessionCapabilities", "resume"],
                ),
                close: bool_at(
                    &result,
                    &["agentCapabilities", "sessionCapabilities", "close"],
                ),
                steering: bool_at(
                    &result,
                    &["agentCapabilities", "sessionCapabilities", "steering"],
                ),
                fork: bool_at(
                    &result,
                    &["agentCapabilities", "sessionCapabilities", "fork"],
                ) || bool_at(&result, &["agentCapabilities", "fork"]),
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
            commands: Vec::<AgentCommandDescriptor>::new(),
        })
    }

    pub async fn new_session(
        &mut self,
        cwd: &Path,
    ) -> Result<StartedAgentSession, AgentRuntimeError> {
        let request = acp::NewSessionRequest::new(cwd);
        self.start_session("session/new", &request, None).await
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

    pub async fn resume_session(
        &mut self,
        cwd: &Path,
        native_session_id: &str,
    ) -> Result<StartedAgentSession, AgentRuntimeError> {
        let request = acp::ResumeSessionRequest::new(native_session_id.to_string(), cwd);
        self.start_session("session/resume", &request, Some(native_session_id))
            .await
    }

    pub async fn prompt(&mut self, prompt: AgentPrompt) -> Result<StartedTurn, AgentRuntimeError> {
        let session_id = self.session_id()?;
        let mut blocks = vec![acp::ContentBlock::Text(acp::TextContent::new(prompt.text))];
        blocks.extend(prompt.images.into_iter().map(|image| {
            acp::ContentBlock::Image(acp::ImageContent::new(image.data, image.mime_type))
        }));
        let result = self
            .request(
                "session/prompt",
                &acp::PromptRequest::new(session_id, blocks),
            )
            .await?;
        Ok(StartedTurn {
            turn_id: result
                .get("turnId")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }

    /// Send a one-shot prompt and retain only assistant text updates.
    ///
    /// The regular `prompt` call intentionally returns as soon as ACP accepts
    /// the request; the conversation event stream owns the rest of that turn.
    /// Git actions need a direct result, so this narrow path consumes the same
    /// ACP transport until the prompt response arrives and ignores tool,
    /// reasoning, and lifecycle updates.
    pub async fn prompt_once(
        &mut self,
        prompt: AgentPrompt,
    ) -> Result<GeneratedText, AgentRuntimeError> {
        let session_id = self.session_id()?;
        let mut blocks = vec![acp::ContentBlock::Text(acp::TextContent::new(prompt.text))];
        blocks.extend(prompt.images.into_iter().map(|image| {
            acp::ContentBlock::Image(acp::ImageContent::new(image.data, image.mime_type))
        }));
        let params = serde_json::to_value(acp::PromptRequest::new(session_id, blocks))
            .map_err(serialization_error)?;
        let one_shot_turn_id = format!("turn-{}", uuid::Uuid::new_v4());
        let mut params = params;
        params["turnId"] = Value::String(one_shot_turn_id.clone());
        let (mut updates, mut registration) = self
            .transport
            .register_prompt_updates(one_shot_turn_id.clone())?;
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

    pub async fn steer(&mut self, text: String) -> Result<(), AgentRuntimeError> {
        let session_id = self.session_id()?;
        self.request(
            "session/steer",
            &json!({ "sessionId": session_id, "prompt": [{ "type": "text", "text": text }] }),
        )
        .await?;
        Ok(())
    }

    pub async fn cancel(&mut self) -> Result<(), AgentRuntimeError> {
        let value = serde_json::to_value(acp::CancelNotification::new(self.session_id()?))
            .map_err(serialization_error)?;
        self.transport.notify("session/cancel", value).await
    }

    pub async fn set_config(
        &mut self,
        option_id: &str,
        value: Value,
    ) -> Result<Vec<AgentConfigOption>, AgentRuntimeError> {
        let result = self
            .request(
                "session/set_config_option",
                &json!({ "sessionId": self.session_id()?, "configId": option_id, "value": value }),
            )
            .await?;
        Ok(parse_config_options(
            result
                .get("configOptions")
                .or_else(|| result.get("sessionConfigOptions")),
        ))
    }

    pub async fn respond_permission(
        &mut self,
        request_id: &str,
        decision: &str,
    ) -> Result<(), AgentRuntimeError> {
        self.respond(request_id, json!({ "outcome": { "outcome": decision } }))
            .await
    }

    pub async fn respond_user_input(
        &mut self,
        request_id: &str,
        values: BTreeMap<String, Value>,
        cancelled: bool,
    ) -> Result<(), AgentRuntimeError> {
        self.respond(
            request_id,
            json!({ "values": values, "cancelled": cancelled }),
        )
        .await
    }

    pub async fn close(&mut self) -> Result<(), AgentRuntimeError> {
        if let Some(session_id) = self.native_session_id.clone() {
            let _ = self
                .request("session/close", &acp::CloseSessionRequest::new(session_id))
                .await;
        }
        self.transport.stop().await;
        self.native_session_id = None;
        Ok(())
    }

    /// Detach the local transport while preserving the provider-native
    /// session id. A later activation can resume the same session.
    pub async fn detach(&mut self) -> Result<(), AgentRuntimeError> {
        self.transport.stop().await;
        Ok(())
    }

    pub fn stderr_snapshot(&self) -> Vec<String> {
        self.transport.stderr_snapshot()
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
        self.native_session_id = Some(native_session_id.clone());
        Ok(StartedAgentSession { native_session_id })
    }

    async fn request<T: Serialize>(
        &mut self,
        method: &str,
        params: &T,
    ) -> Result<Value, AgentRuntimeError> {
        let params = serde_json::to_value(params).map_err(serialization_error)?;
        self.transport.request(method, params).await
    }

    async fn respond(&mut self, request_id: &str, result: Value) -> Result<(), AgentRuntimeError> {
        let id = serde_json::from_str::<Value>(request_id)
            .unwrap_or_else(|_| Value::String(request_id.to_string()));
        self.transport.respond(id, result).await
    }

    fn session_id(&self) -> Result<String, AgentRuntimeError> {
        self.native_session_id.clone().ok_or_else(|| {
            AgentRuntimeError::new("session-not-started", "ACP session has not started")
        })
    }
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

fn bool_at(value: &Value, path: &[&str]) -> bool {
    path.iter()
        .try_fold(value, |current, part| current.get(*part))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn parse_config_options(value: Option<&Value>) -> Vec<AgentConfigOption> {
    value
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default()
}

fn serialization_error(error: serde_json::Error) -> AgentRuntimeError {
    AgentRuntimeError::new("serialization", error.to_string())
}
fn transport_error(message: String) -> AgentRuntimeError {
    AgentRuntimeError::new("transport", message)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use serde_json::json;

    use super::*;
    use crate::agent_conversation::protocol::{
        AgentApprovalDecision, AgentApprovalResponse, AgentRequestIdentity, AgentUserInputResponse,
        ProviderSource, ProviderTransport,
    };
    use crate::agent_conversation::providers::AgentRuntimeAdapter;

    fn fixture_manifest(log_path: &Path) -> AgentProviderManifest {
        fixture_manifest_named(log_path, "default")
    }

    pub(crate) fn fixture_manifest_named(log_path: &Path, fixture: &str) -> AgentProviderManifest {
        let script = format!(
            r#"log={log}
fixture={fixture}
while IFS= read -r line; do
  printf '%s\n' "$line" >> "$log"
  id=$(printf '%s\n' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"agentInfo":{{"name":"fake-acp","version":"1"}},"agentCapabilities":{{"loadSession":true,"promptCapabilities":{{"image":true}}}}}}}}\n' "$id" ;;
    *'"method":"session/new"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"new-session"}}}}\n' "$id" ;;
    *'"method":"session/load"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"loaded-session"}}}}\n' "$id" ;;
    *'"method":"session/resume"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"resumed-session"}}}}\n' "$id" ;;
    *'"method":"session/prompt"'*)
      turn=$(printf '%s\n' "$line" | sed -n 's/.*"turnId":"\([^"]*\)".*/\1/p')
      if [ -z "$turn" ]; then turn="turn-1"; fi
      if [ "$fixture" = "dies_midturn" ]; then
        exit 0
      elif [ "$fixture" = "pending_drop" ]; then
        sleep 1
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"turn-1","stopReason":"end_turn"}}}}\n' "$id"
      elif [ "$fixture" = "direct_result" ]; then
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"turn-direct","text":"direct response text"}}}}\n' "$id"
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
                printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"turn-1","stopReason":"end_turn"}}}}\n' "$id"
              elif printf '%s' "$response" | grep -q '\"outcome\":{{\"outcome\":\"selected\",\"optionId\":\"reject\"'; then
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
    *'"method":"session/set_config_option"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"configOptions":[{{"id":"model","label":"Model","category":"model","value":"new"}}]}}}}\n' "$id" ;;
    *'"method":"session/steer"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{}}}}\n' "$id" ;;
    *'"method":"session/close"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{}}}}\n' "$id"; exit 0 ;;
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

    fn identity(request_id: &str) -> AgentRequestIdentity {
        AgentRequestIdentity {
            owned_id: "owned-a".into(),
            generation: 1,
            request_id: request_id.into(),
            turn_id: Some("turn-1".into()),
            item_id: None,
        }
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
        client.new_session(&root).await.unwrap();
        let generated = client
            .prompt_once(AgentPrompt {
                text: "write".into(),
                images: Vec::new(),
            })
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
        client.new_session(&root).await.unwrap();
        let generated = client
            .prompt_once(AgentPrompt {
                text: "write".into(),
                images: Vec::new(),
            })
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
    async fn acp_initialize_new_prompt_image_config_correlations_cancel_and_close() {
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
                provider_instance_id: "fake-1".into(),
            })
            .await
            .unwrap();
        assert!(capabilities.prompt.image);
        assert_eq!(
            adapter
                .new_session(super::super::NewAgentSession { cwd: root.clone() })
                .await
                .unwrap()
                .native_session_id,
            "new-session"
        );
        let turn = adapter
            .prompt(super::super::AgentPrompt {
                text: "hello".into(),
                images: vec![super::super::AgentPromptImage {
                    data: "aW1hZ2U=".into(),
                    mime_type: "image/png".into(),
                }],
            })
            .await
            .unwrap();
        assert_eq!(turn.turn_id.as_deref(), Some("turn-1"));
        let generated = adapter
            .prompt_once(super::super::AgentPrompt {
                text: "write a commit subject".into(),
                images: Vec::new(),
            })
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
        let options = adapter.set_config("model", json!("new")).await.unwrap();
        assert_eq!(options[0].value, json!("new"));
        adapter
            .respond_permission(AgentApprovalResponse {
                identity: identity("approval-7"),
                decision: AgentApprovalDecision::Accept,
            })
            .await
            .unwrap();
        adapter
            .respond_user_input(AgentUserInputResponse {
                identity: identity("input-8"),
                values: BTreeMap::from([("answer".into(), json!("yes"))]),
                cancelled: false,
            })
            .await
            .unwrap();
        adapter.cancel_turn(Some("turn-1")).await.unwrap();
        let pid = adapter.process_id().unwrap();
        adapter.close_session().await.unwrap();
        assert_ne!(
            unsafe { libc::kill(pid as i32, 0) },
            0,
            "fake ACP parent process survived close"
        );

        let deadline = Instant::now() + Duration::from_secs(2);
        let frames = loop {
            let frames = std::fs::read_to_string(&log).unwrap_or_default();
            if frames.contains("input-8") || Instant::now() >= deadline {
                break frames;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        assert!(frames.contains("\"type\":\"image\""));
        assert!(frames.contains("approval-7"));
        assert!(frames.contains("input-8"));
        assert!(frames.contains("session/cancel"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn acp_load_and_resume_use_distinct_session_methods() {
        let root = fixture_root();
        for (method, expected) in [("load", "loaded-session"), ("resume", "resumed-session")] {
            let log = root.join(format!("{method}.jsonl"));
            let mut client = AcpClient::spawn(&fixture_manifest(&log), &root, method).unwrap();
            client
                .initialize(AgentConversationProvider::Claude)
                .await
                .unwrap();
            let started = if method == "load" {
                client.load_session(&root, "native-old").await.unwrap()
            } else {
                client.resume_session(&root, "native-old").await.unwrap()
            };
            assert_eq!(started.native_session_id, expected);
            client.close().await.unwrap();
            let frames = std::fs::read_to_string(log).unwrap();
            assert!(frames.contains(&format!("session/{method}")));
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
