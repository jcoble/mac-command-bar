mod attachments;
mod process;
pub mod protocol;
#[allow(dead_code)]
mod provider;
mod transcript;

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use protocol::{
    AgentConversationConnection, AgentConversationEvent, AgentConversationPayload,
    AgentConversationSnapshot, ApprovalDecision, ConversationConnectionState,
    EnsureAgentConversationRequest, RespondAgentConversationApprovalRequest,
    SendAgentConversationMessageRequest, StopAgentConversationTurnRequest,
};
use tauri::Emitter;
use tokio::sync::mpsc;

const SNAPSHOT_EVENT_CAP: usize = 2_000;

#[derive(Debug)]
pub(crate) enum ProviderCommand {
    SendMessage(String),
    RespondApproval {
        request_id: String,
        decision: ApprovalDecision,
    },
    Stop,
    Close,
}

struct ConversationSession {
    connection: AgentConversationConnection,
    cwd: String,
    sequence: u64,
    events: VecDeque<AgentConversationEvent>,
    command_tx: Option<mpsc::UnboundedSender<ProviderCommand>>,
}

#[derive(Clone, Default)]
pub struct AgentConversationRegistry {
    inner: Arc<Mutex<HashMap<String, ConversationSession>>>,
}

impl AgentConversationRegistry {
    fn ensure(
        &self,
        request: EnsureAgentConversationRequest,
    ) -> Result<AgentConversationConnection, String> {
        let owned_id = required_id(&request.owned_id, "Owned session id")?;
        let cwd = process::validated_conversation_cwd(&request.cwd)?
            .display()
            .to_string();
        let mut sessions = self
            .inner
            .lock()
            .map_err(|_| "Conversation registry is unavailable".to_string())?;

        if let Some(current) = sessions.get(&owned_id) {
            if current.connection.provider == request.provider && current.cwd == cwd {
                return Ok(current.connection.clone());
            }
        }

        let prior = sessions.remove(&owned_id);
        let generation = prior
            .as_ref()
            .map(|session| session.connection.generation.saturating_add(1))
            .unwrap_or(1);
        if let Some(sender) = prior.and_then(|session| session.command_tx) {
            let _ = sender.send(ProviderCommand::Close);
        }

        let connection = AgentConversationConnection {
            owned_id: owned_id.clone(),
            provider: request.provider,
            generation,
            native_session_id: normalized_optional_id(request.native_session_id),
            state: ConversationConnectionState::Connecting,
        };
        sessions.insert(
            owned_id,
            ConversationSession {
                connection: connection.clone(),
                cwd,
                sequence: 0,
                events: VecDeque::new(),
                command_tx: None,
            },
        );
        Ok(connection)
    }

    #[cfg(test)]
    pub(crate) fn install_sender(
        &self,
        owned_id: &str,
        generation: u64,
        sender: mpsc::UnboundedSender<ProviderCommand>,
    ) -> Result<(), String> {
        let mut sessions = self
            .inner
            .lock()
            .map_err(|_| "Conversation registry is unavailable".to_string())?;
        let session = current_session_mut(&mut sessions, owned_id, generation)?;
        session.command_tx = Some(sender);
        Ok(())
    }

    #[allow(dead_code)]
    fn install_sender_if_missing(
        &self,
        owned_id: &str,
        generation: u64,
    ) -> Result<Option<mpsc::UnboundedReceiver<ProviderCommand>>, String> {
        let mut sessions = self
            .inner
            .lock()
            .map_err(|_| "Conversation registry is unavailable".to_string())?;
        let session = current_session_mut(&mut sessions, owned_id, generation)?;
        if session.command_tx.is_some() {
            return Ok(None);
        }
        let (tx, rx) = mpsc::unbounded_channel();
        session.command_tx = Some(tx);
        Ok(Some(rx))
    }

    pub(crate) fn emit_payload(
        &self,
        owned_id: &str,
        generation: u64,
        payload: AgentConversationPayload,
    ) -> Result<AgentConversationEvent, String> {
        let mut sessions = self
            .inner
            .lock()
            .map_err(|_| "Conversation registry is unavailable".to_string())?;
        let session = current_session_mut(&mut sessions, owned_id, generation)?;
        session.sequence = session.sequence.saturating_add(1);
        if let AgentConversationPayload::Connection {
            state,
            native_session_id,
        } = &payload
        {
            session.connection.state = *state;
            if native_session_id.is_some() {
                session.connection.native_session_id = native_session_id.clone();
            }
        }
        let event = AgentConversationEvent {
            owned_id: owned_id.to_string(),
            provider: session.connection.provider,
            generation,
            sequence: session.sequence,
            timestamp_ms: timestamp_millis(),
            payload,
        };
        session.events.push_back(event.clone());
        while session.events.len() > SNAPSHOT_EVENT_CAP {
            session.events.pop_front();
        }
        Ok(event)
    }

    fn send_command(
        &self,
        owned_id: &str,
        generation: u64,
        command: ProviderCommand,
    ) -> Result<(), String> {
        let sessions = self
            .inner
            .lock()
            .map_err(|_| "Conversation registry is unavailable".to_string())?;
        let session = current_session(&sessions, owned_id, generation)?;
        let sender = session
            .command_tx
            .as_ref()
            .ok_or_else(|| "Structured provider is still connecting".to_string())?;
        sender
            .send(command)
            .map_err(|_| "Structured provider is no longer running".to_string())
    }

    fn snapshot(&self, owned_id: &str) -> Result<Option<AgentConversationSnapshot>, String> {
        let sessions = self
            .inner
            .lock()
            .map_err(|_| "Conversation registry is unavailable".to_string())?;
        Ok(sessions
            .get(owned_id)
            .map(|session| AgentConversationSnapshot {
                connection: session.connection.clone(),
                last_sequence: session.sequence,
                events: session.events.iter().cloned().collect(),
            }))
    }

    fn close(&self, owned_id: &str) -> Result<bool, String> {
        let mut sessions = self
            .inner
            .lock()
            .map_err(|_| "Conversation registry is unavailable".to_string())?;
        let Some(session) = sessions.remove(owned_id) else {
            return Ok(false);
        };
        if let Some(sender) = session.command_tx {
            let _ = sender.send(ProviderCommand::Close);
        }
        Ok(true)
    }
}

impl Drop for AgentConversationRegistry {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) != 1 {
            return;
        }
        if let Ok(mut sessions) = self.inner.lock() {
            for (_, session) in sessions.drain() {
                if let Some(sender) = session.command_tx {
                    let _ = sender.send(ProviderCommand::Close);
                }
            }
        }
    }
}

fn current_session<'a>(
    sessions: &'a HashMap<String, ConversationSession>,
    owned_id: &str,
    generation: u64,
) -> Result<&'a ConversationSession, String> {
    let session = sessions
        .get(owned_id)
        .ok_or_else(|| "Conversation session was not found".to_string())?;
    if session.connection.generation != generation {
        return Err("Conversation connection changed; retry on the current session".to_string());
    }
    Ok(session)
}

fn current_session_mut<'a>(
    sessions: &'a mut HashMap<String, ConversationSession>,
    owned_id: &str,
    generation: u64,
) -> Result<&'a mut ConversationSession, String> {
    let session = sessions
        .get_mut(owned_id)
        .ok_or_else(|| "Conversation session was not found".to_string())?;
    if session.connection.generation != generation {
        return Err("Conversation connection changed; retry on the current session".to_string());
    }
    Ok(session)
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

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[tauri::command]
pub async fn ensure_agent_conversation(
    registry: tauri::State<'_, AgentConversationRegistry>,
    request: EnsureAgentConversationRequest,
) -> Result<AgentConversationConnection, String> {
    // Intentionally unwired: a selected session already has one authoritative
    // agent inside its PTY. Resuming it through app-server/stream-json creates
    // a second agent that can diverge and append competing transcript records.
    // Structured rendering now mirrors the real PTY's transcript instead.
    registry.ensure(request)
}

pub(crate) fn publish_payload(
    registry: &AgentConversationRegistry,
    app: &tauri::AppHandle,
    owned_id: &str,
    generation: u64,
    payload: AgentConversationPayload,
) {
    if let Ok(event) = registry.emit_payload(owned_id, generation, payload) {
        let _ = app.emit("agent-conversation-event", event);
    }
}

#[tauri::command]
pub async fn send_agent_conversation_message(
    registry: tauri::State<'_, AgentConversationRegistry>,
    request: SendAgentConversationMessageRequest,
) -> Result<(), String> {
    let text = request.text.trim();
    if text.is_empty() {
        return Err("Message cannot be empty".to_string());
    }
    registry.send_command(
        &request.owned_id,
        request.generation,
        ProviderCommand::SendMessage(text.to_string()),
    )
}

#[tauri::command]
pub async fn respond_agent_conversation_approval(
    registry: tauri::State<'_, AgentConversationRegistry>,
    request: RespondAgentConversationApprovalRequest,
) -> Result<(), String> {
    let request_id = required_id(&request.request_id, "Approval request id")?;
    registry.send_command(
        &request.owned_id,
        request.generation,
        ProviderCommand::RespondApproval {
            request_id,
            decision: request.decision,
        },
    )
}

#[tauri::command]
pub async fn stop_agent_conversation_turn(
    registry: tauri::State<'_, AgentConversationRegistry>,
    request: StopAgentConversationTurnRequest,
) -> Result<(), String> {
    registry.send_command(&request.owned_id, request.generation, ProviderCommand::Stop)
}

#[tauri::command]
pub async fn close_agent_conversation(
    registry: tauri::State<'_, AgentConversationRegistry>,
    owned_id: String,
) -> Result<bool, String> {
    registry.close(&owned_id)
}

#[tauri::command]
pub async fn read_agent_conversation_snapshot(
    registry: tauri::State<'_, AgentConversationRegistry>,
    owned_id: String,
) -> Result<Option<AgentConversationSnapshot>, String> {
    registry.snapshot(&owned_id)
}

#[tauri::command]
pub async fn read_agent_conversation_transcript(
    provider: String,
    native_session_id: String,
    child_session_id: Option<String>,
) -> Result<transcript::TranscriptSnapshot, String> {
    transcript::read(&provider, &native_session_id, child_session_id.as_deref())
}

#[tauri::command]
pub async fn save_agent_conversation_attachment(
    app: tauri::AppHandle,
    owned_id: String,
    mime_type: String,
    bytes: Vec<u8>,
) -> Result<attachments::SavedConversationAttachment, String> {
    attachments::save(&app, &owned_id, &mime_type, &bytes)
}

#[cfg(test)]
mod tests {
    use super::protocol::AgentConversationProvider;
    use super::*;
    use std::fs;

    fn request(
        root: &str,
        owned_id: &str,
        provider: AgentConversationProvider,
    ) -> EnsureAgentConversationRequest {
        EnsureAgentConversationRequest {
            owned_id: owned_id.to_string(),
            provider,
            cwd: root.to_string(),
            native_session_id: None,
        }
    }

    fn temp_root() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("mcb-conversation-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn ensure_is_idempotent_and_provider_change_increments_generation() {
        let root = temp_root();
        let registry = AgentConversationRegistry::default();
        let first = registry
            .ensure(request(
                root.to_str().unwrap(),
                "owned-a",
                AgentConversationProvider::Codex,
            ))
            .unwrap();
        let same = registry
            .ensure(request(
                root.to_str().unwrap(),
                "owned-a",
                AgentConversationProvider::Codex,
            ))
            .unwrap();
        let changed = registry
            .ensure(request(
                root.to_str().unwrap(),
                "owned-a",
                AgentConversationProvider::Claude,
            ))
            .unwrap();
        assert_eq!(first.generation, 1);
        assert_eq!(same, first);
        assert_eq!(changed.generation, 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn generation_guards_commands_and_sequence_is_registry_owned() {
        let root = temp_root();
        let registry = AgentConversationRegistry::default();
        let connection = registry
            .ensure(request(
                root.to_str().unwrap(),
                "owned-a",
                AgentConversationProvider::Codex,
            ))
            .unwrap();
        let (tx, mut rx) = mpsc::unbounded_channel();
        registry
            .install_sender("owned-a", connection.generation, tx)
            .unwrap();
        registry
            .send_command("owned-a", connection.generation, ProviderCommand::Stop)
            .unwrap();
        assert!(matches!(rx.try_recv(), Ok(ProviderCommand::Stop)));
        assert!(registry
            .send_command("owned-a", connection.generation + 1, ProviderCommand::Stop)
            .is_err());

        let first = registry
            .emit_payload(
                "owned-a",
                connection.generation,
                AgentConversationPayload::Connection {
                    state: ConversationConnectionState::Connected,
                    native_session_id: Some("thread-a".to_string()),
                },
            )
            .unwrap();
        let second = registry
            .emit_payload(
                "owned-a",
                connection.generation,
                AgentConversationPayload::Turn {
                    turn_id: "turn-a".to_string(),
                    state: protocol::TurnState::Started,
                },
            )
            .unwrap();
        assert_eq!((first.sequence, second.sequence), (1, 2));
        assert_eq!(
            registry.snapshot("owned-a").unwrap().unwrap().events.len(),
            2
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn closing_one_session_does_not_close_another() {
        let root = temp_root();
        let registry = AgentConversationRegistry::default();
        registry
            .ensure(request(
                root.to_str().unwrap(),
                "owned-a",
                AgentConversationProvider::Codex,
            ))
            .unwrap();
        registry
            .ensure(request(
                root.to_str().unwrap(),
                "owned-b",
                AgentConversationProvider::Claude,
            ))
            .unwrap();
        assert_eq!(registry.close("owned-a").unwrap(), true);
        assert!(registry.snapshot("owned-a").unwrap().is_none());
        assert!(registry.snapshot("owned-b").unwrap().is_some());
        fs::remove_dir_all(root).unwrap();
    }
}
