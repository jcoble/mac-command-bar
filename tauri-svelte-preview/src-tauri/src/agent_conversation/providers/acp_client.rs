use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

use agent_client_protocol::schema::{v1 as acp, ProtocolVersion};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::{broadcast, mpsc, oneshot};

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
    writer: tokio::sync::Mutex<SidecarWriteHalf>,
    process: Mutex<Option<SidecarProcessHandle>>,
    inbound_copy: broadcast::Sender<AcpInbound>,
}

impl AcpTransport {
    /// Splits the process pipes, spawns the reader task, returns the transport
    /// plus the single consumer end of the inbound channel.
    pub fn start(
        process: SidecarProcess,
    ) -> (Arc<AcpTransport>, mpsc::UnboundedReceiver<AcpInbound>) {
        let (mut reader, writer, process_handle) = process.split();
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let (copy_tx, _) = broadcast::channel(256);
        let transport = Arc::new(Self {
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(0),
            writer: tokio::sync::Mutex::new(writer),
            process: Mutex::new(Some(process_handle)),
            inbound_copy: copy_tx.clone(),
        });
        let reader_transport = transport.clone();
        tokio::spawn(async move {
            reader_loop(&mut reader, &inbound_tx, &copy_tx, &reader_transport).await;
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
        self.pending
            .lock()
            .map_err(|_| transport_error("pending request map is unavailable".to_string()))?
            .insert(id, sender);
        let frame = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        if let Err(error) = self.write(frame).await {
            if let Ok(mut pending) = self.pending.lock() {
                pending.remove(&id);
            }
            return Err(error);
        }
        receiver
            .await
            .map_err(|_| transport_error("transport closed before response".to_string()))?
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

    fn subscribe(&self) -> broadcast::Receiver<AcpInbound> {
        self.inbound_copy.subscribe()
    }

    async fn write(&self, frame: Value) -> Result<(), AgentRuntimeError> {
        self.writer
            .lock()
            .await
            .write_json(&frame)
            .await
            .map_err(transport_error)
    }
}

async fn reader_loop(
    reader: &mut SidecarReadHalf,
    inbound_tx: &mpsc::UnboundedSender<AcpInbound>,
    copy_tx: &broadcast::Sender<AcpInbound>,
    transport: &AcpTransport,
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
                let inbound =
                    AcpInbound::SessionUpdate(frame.get("params").cloned().unwrap_or(Value::Null));
                let _ = inbound_tx.send(inbound);
                let _ = copy_tx.send(AcpInbound::SessionUpdate(
                    frame.get("params").cloned().unwrap_or(Value::Null),
                ));
            }
            (false, Some(method)) => {
                eprintln!("Ignoring unsupported ACP notification: {method}");
            }
            (false, None) => {}
        }
    };

    let error = transport_error(reason.clone());
    if let Ok(mut map) = transport.pending.lock() {
        for (_, sender) in map.drain() {
            let _ = sender.send(Err(AgentRuntimeError::new(
                "transport",
                error.message.clone(),
            )));
        }
    }
    let inbound = AcpInbound::TransportClosed { reason };
    let _ = inbound_tx.send(inbound);
    let _ = copy_tx.send(AcpInbound::TransportClosed {
        reason: error.message,
    });
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
        self.start_session("session/new", &request).await
    }

    pub async fn load_session(
        &mut self,
        cwd: &Path,
        native_session_id: &str,
    ) -> Result<StartedAgentSession, AgentRuntimeError> {
        let request = acp::LoadSessionRequest::new(native_session_id.to_string(), cwd);
        self.start_session("session/load", &request).await
    }

    pub async fn resume_session(
        &mut self,
        cwd: &Path,
        native_session_id: &str,
    ) -> Result<StartedAgentSession, AgentRuntimeError> {
        let request = acp::ResumeSessionRequest::new(native_session_id.to_string(), cwd);
        self.start_session("session/resume", &request).await
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

    /// Send a bounded one-shot prompt and retain only assistant text updates.
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
        let mut updates = self.transport.subscribe();
        let params = serde_json::to_value(acp::PromptRequest::new(session_id, blocks))
            .map_err(serialization_error)?;
        let response = self.transport.request("session/prompt", params).await?;
        let mut text = String::new();
        let mut turn_id = None;
        while let Ok(inbound) = updates.try_recv() {
            let AcpInbound::SessionUpdate(params) = inbound else {
                continue;
            };
            let update = params
                .get("update")
                .or_else(|| params.get("sessionUpdate"))
                .unwrap_or(&params);
            if let Some(id) = update
                .get("turnId")
                .or_else(|| update.get("turn_id"))
                .and_then(Value::as_str)
            {
                turn_id = Some(id.to_string());
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
        if let Some(result_text) = response.get("result").and_then(extract_text_value) {
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
    ) -> Result<StartedAgentSession, AgentRuntimeError> {
        let result = self.request(method, request).await?;
        let native_session_id = result
            .get("sessionId")
            .or_else(|| result.get("session_id"))
            .and_then(Value::as_str)
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
mod tests {
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

    fn fixture_manifest_named(log_path: &Path, fixture: &str) -> AgentProviderManifest {
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
      if [ "$fixture" = "dies_midturn" ]; then
        exit 0
      elif [ "$fixture" = "permission_midturn" ]; then
        printf '{{"jsonrpc":"2.0","id":77,"method":"session/request_permission","params":{{"options":[{{"optionId":"allow","name":"Allow"}}]}}}}\n'
        while IFS= read -r response; do
          printf '%s\n' "$response" >> "$log"
          case "$response" in
            *'"id":77'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"turn-1","stopReason":"end_turn"}}}}\n' "$id"; break ;;
          esac
        done
      else
        printf '{{"jsonrpc":"2.0","method":"session/update","params":{{"update":{{"sessionUpdate":"agent_message_chunk","content":{{"type":"text","text":"generated text"}},"turnId":"turn-1"}}}}}}\n'
        printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"turn-1","stopReason":"end_turn"}}}}\n' "$id"
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
        assert_eq!(generated.turn_id.as_deref(), Some("turn-1"));
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
