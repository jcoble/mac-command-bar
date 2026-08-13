mod attachments;
pub mod capabilities;
pub mod handoff;
mod legacy_import;
pub mod manager;
pub mod prompt_content;
pub mod protocol;
pub mod providers;
pub mod reaper;
pub mod terminal_projection;
mod transcript;

use manager::AgentRuntimeManager;
use mcb_core::session_store::AnnotationRow;
use prompt_content::prompt_from_blocks;
use protocol::{
    AgentCapabilities, AgentConversationConfigState, AgentConversationConnection,
    AgentConversationSnapshot, EnsureAgentConversationRequest,
    RespondAgentConversationApprovalRequest, RespondAgentConversationInputRequest,
    RespondAgentConversationPermissionRequest, SendAgentConversationMessageRequest,
    SetAgentConversationConfigRequest, StopAgentConversationTurnRequest,
};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationSessionAnnotation {
    id: i64,
    owned_id: String,
    url: String,
    rect_json: String,
    note: String,
    created_at_ms: i64,
}

impl From<AnnotationRow> for AgentConversationSessionAnnotation {
    fn from(row: AnnotationRow) -> Self {
        Self {
            id: row.id,
            owned_id: row.owned_id,
            url: row.url,
            rect_json: row.rect_json,
            note: row.note,
            created_at_ms: row.created_at_ms,
        }
    }
}

#[tauri::command]
pub fn agent_conversation_add_session_annotation(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
    url: String,
    rect_json: String,
    note: String,
) -> Result<AgentConversationSessionAnnotation, String> {
    manager
        .add_session_annotation(&owned_id, &url, &rect_json, &note)
        .map(Into::into)
}

#[tauri::command]
pub fn agent_conversation_list_session_annotations(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> Result<Vec<AgentConversationSessionAnnotation>, String> {
    manager
        .list_session_annotations(&owned_id)
        .map(|annotations| annotations.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub fn agent_conversation_delete_session_annotation(
    manager: tauri::State<'_, AgentRuntimeManager>,
    id: i64,
) -> Result<(), String> {
    manager.delete_session_annotation(id)
}

#[tauri::command]
pub async fn ensure_agent_conversation(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: EnsureAgentConversationRequest,
) -> Result<AgentConversationConnection, String> {
    let connection = manager.ensure_async(request).await?;
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
    let prompt = prompt_from_blocks(request.text.trim(), request.content)?;
    manager
        .prompt(&request.owned_id, request.generation, prompt)
        .await
}

#[tauri::command]
pub fn agent_conversation_set_session_draft(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
    text: String,
) -> Result<(), String> {
    manager.set_session_draft(&owned_id, &text)
}

#[tauri::command]
pub fn agent_conversation_get_session_draft(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> Result<Option<String>, String> {
    manager.get_session_draft(&owned_id)
}

#[tauri::command]
pub fn agent_conversation_clear_session_draft(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> Result<(), String> {
    manager.clear_session_draft(&owned_id)
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
pub async fn respond_agent_conversation_permission(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: RespondAgentConversationPermissionRequest,
) -> Result<(), String> {
    let request_id = required_id(&request.request_id, "Permission request id")?;
    manager
        .respond_permission_option(
            &request.owned_id,
            request.generation,
            request_id,
            request.option_id,
        )
        .await
}

#[tauri::command]
pub async fn respond_agent_conversation_input(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: RespondAgentConversationInputRequest,
) -> Result<(), String> {
    let request_id = required_id(&request.request_id, "User input request id")?;
    manager
        .respond_user_input(protocol::AgentUserInputResponse {
            identity: protocol::AgentRequestIdentity {
                owned_id: request.owned_id,
                generation: request.generation,
                request_id,
                turn_id: None,
                item_id: None,
            },
            values: request.values,
            cancelled: request.cancelled,
        })
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
pub async fn set_agent_conversation_config(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: SetAgentConversationConfigRequest,
) -> Result<AgentConversationConfigState, String> {
    manager.set_conversation_config(request).await
}

#[tauri::command]
pub fn read_agent_conversation_config(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> Result<AgentConversationConfigState, String> {
    manager.conversation_config(&owned_id)
}

#[tauri::command]
pub fn read_agent_conversation_capabilities(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> Result<AgentCapabilities, String> {
    manager.capabilities_for_owned_id(&owned_id)
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

#[tauri::command]
pub fn delete_agent_conversation_attachment(
    app: tauri::AppHandle,
    request: attachments::DeleteConversationAttachmentRequest,
) -> Result<(), String> {
    attachments::delete(&app, request)
}

fn required_id(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label} is required"))
    } else {
        Ok(value.to_string())
    }
}
