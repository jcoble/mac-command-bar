mod attachments;
pub mod capabilities;
pub mod journal;
pub mod manager;
pub mod protocol;
pub mod providers;
pub mod terminal_projection;
mod transcript;

use manager::AgentRuntimeManager;
use protocol::{
    AgentConversationConnection, AgentConversationSnapshot, EnsureAgentConversationRequest,
    RespondAgentConversationApprovalRequest, SendAgentConversationMessageRequest,
    StopAgentConversationTurnRequest,
};
use providers::AgentPrompt;

#[tauri::command]
pub async fn ensure_agent_conversation(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: EnsureAgentConversationRequest,
) -> Result<AgentConversationConnection, String> {
    let connection = manager.ensure(request)?;
    if manager.providers().manifest(connection.provider).is_ok() {
        manager
            .activate(&connection.owned_id, connection.generation)
            .await
    } else {
        // A0 installs the packaged, hash-recorded adapters. Until then the
        // existing connecting response remains available without a fallback PTY.
        Ok(connection)
    }
}

#[tauri::command]
pub async fn send_agent_conversation_message(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: SendAgentConversationMessageRequest,
) -> Result<(), String> {
    let text = request.text.trim();
    if text.is_empty() {
        return Err("Message cannot be empty".to_string());
    }
    manager
        .prompt(
            &request.owned_id,
            request.generation,
            AgentPrompt {
                text: text.to_string(),
                images: Vec::new(),
            },
        )
        .await
}

#[tauri::command]
pub async fn respond_agent_conversation_approval(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: RespondAgentConversationApprovalRequest,
) -> Result<(), String> {
    let request_id = required_id(&request.request_id, "Approval request id")?;
    manager
        .respond_legacy_approval(
            &request.owned_id,
            request.generation,
            request_id,
            request.decision,
        )
        .await
}

#[tauri::command]
pub async fn stop_agent_conversation_turn(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: StopAgentConversationTurnRequest,
) -> Result<(), String> {
    manager
        .cancel_turn(&request.owned_id, request.generation)
        .await
}

#[tauri::command]
pub async fn close_agent_conversation(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> Result<bool, String> {
    manager.close(&owned_id).await
}

#[tauri::command]
pub async fn read_agent_conversation_snapshot(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> Result<Option<AgentConversationSnapshot>, String> {
    manager.snapshot(&owned_id)
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
pub async fn start_agent_conversation_terminal_projection(
    app: tauri::AppHandle,
    registry: tauri::State<'_, terminal_projection::TerminalProjectionRegistry>,
    request: terminal_projection::StartTerminalProjectionRequest,
) -> Result<terminal_projection::TerminalProjectionRegistration, String> {
    registry.start(app, request)
}

#[tauri::command]
pub async fn stop_agent_conversation_terminal_projection(
    registry: tauri::State<'_, terminal_projection::TerminalProjectionRegistry>,
    owned_id: String,
) -> Result<bool, String> {
    registry.stop(&owned_id)
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

fn required_id(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label} is required"))
    } else {
        Ok(value.to_string())
    }
}
