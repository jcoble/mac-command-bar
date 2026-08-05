use std::collections::BTreeMap;
use std::path::Path;

use agent_client_protocol::schema::{v1 as acp, ProtocolVersion};
use serde::Serialize;
use serde_json::{json, Value};

use super::super::protocol::{
    AgentCapabilities, AgentCommandDescriptor, AgentConfigOption, AgentConversationProvider,
    AgentImplementation, AgentInteractionCapabilities, AgentPromptCapabilities,
    AgentProviderManifest, AgentSessionCapabilities,
};
use super::process::SidecarProcess;
use super::{AgentPrompt, AgentRuntimeError, StartedAgentSession, StartedTurn};

pub struct AcpClient {
    process: SidecarProcess,
    next_request_id: u64,
    native_session_id: Option<String>,
}

impl AcpClient {
    pub fn spawn(
        manifest: &AgentProviderManifest,
        cwd: &Path,
        owned_id: &str,
    ) -> Result<Self, AgentRuntimeError> {
        Ok(Self {
            process: SidecarProcess::spawn(manifest, cwd, owned_id)
                .map_err(|message| AgentRuntimeError::new("sidecar-spawn", message))?,
            next_request_id: 0,
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
        self.process
            .write_json(&json!({ "jsonrpc": "2.0", "method": "session/cancel", "params": value }))
            .await
            .map_err(transport_error)
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
        self.process.stop().await;
        self.native_session_id = None;
        Ok(())
    }

    pub fn stderr_snapshot(&self) -> Vec<String> {
        self.process.stderr_snapshot()
    }
    pub fn process_id(&self) -> Option<u32> {
        self.process.process_id()
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
        self.next_request_id = self.next_request_id.saturating_add(1);
        let id = self.next_request_id;
        let params = serde_json::to_value(params).map_err(serialization_error)?;
        self.process
            .write_json(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
            .await
            .map_err(transport_error)?;
        loop {
            let frame = self.process.next_json().await.map_err(transport_error)?;
            if frame.get("id") != Some(&json!(id)) {
                continue;
            }
            if let Some(error) = frame.get("error") {
                return Err(AgentRuntimeError::new("acp-error", error.to_string()));
            }
            return Ok(frame.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    async fn respond(&mut self, request_id: &str, result: Value) -> Result<(), AgentRuntimeError> {
        let id = serde_json::from_str::<Value>(request_id)
            .unwrap_or_else(|_| Value::String(request_id.to_string()));
        self.process
            .write_json(&json!({ "jsonrpc": "2.0", "id": id, "result": result }))
            .await
            .map_err(transport_error)
    }

    fn session_id(&self) -> Result<String, AgentRuntimeError> {
        self.native_session_id.clone().ok_or_else(|| {
            AgentRuntimeError::new("session-not-started", "ACP session has not started")
        })
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
        let script = format!(
            r#"log={log}
while IFS= read -r line; do
  printf '%s\n' "$line" >> "$log"
  id=$(printf '%s\n' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"agentInfo":{{"name":"fake-acp","version":"1"}},"agentCapabilities":{{"loadSession":true,"promptCapabilities":{{"image":true}}}}}}}}\n' "$id" ;;
    *'"method":"session/new"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"new-session"}}}}\n' "$id" ;;
    *'"method":"session/load"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"loaded-session"}}}}\n' "$id" ;;
    *'"method":"session/resume"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"sessionId":"resumed-session"}}}}\n' "$id" ;;
    *'"method":"session/prompt"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"turnId":"turn-1","stopReason":"end_turn"}}}}\n' "$id" ;;
    *'"method":"session/set_config_option"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"configOptions":[{{"id":"model","label":"Model","category":"model","value":"new"}}]}}}}\n' "$id" ;;
    *'"method":"session/steer"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{}}}}\n' "$id" ;;
    *'"method":"session/close"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{}}}}\n' "$id"; exit 0 ;;
  esac
done"#,
            log = shell_quote(log_path)
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
