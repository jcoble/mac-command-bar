//! Synchronous Tauri commands run on the UI thread, so storage-backed commands are async to keep the interface responsive.

mod attachments;
mod broker_status;
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
pub mod transcript_import;

use manager::AgentRuntimeManager;
use mcb_core::session_store::AnnotationRow;
use prompt_content::prompt_from_blocks;
use protocol::{
    AgentCapabilities, AgentConversationConfigState, AgentConversationConnection,
    AgentConversationEvent, AgentConversationEventPage, AgentConversationSessionRecord,
    AgentConversationSnapshot,
    ChangeAgentConversationCheckoutRequest, CommandResult, EnsureAgentConversationRequest,
    RespondAgentConversationApprovalRequest,
    RespondAgentConversationInputRequest, RespondAgentConversationPermissionRequest,
    SendAgentConversationMessageRequest, SetAgentConversationConfigRequest,
    StopAgentConversationTurnRequest, UpdateAgentConversationSessionMetaRequest,
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
/// Adds one annotation and converts storage failures at the native boundary.
pub async fn agent_conversation_add_session_annotation(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
    url: String,
    rect_json: String,
    note: String,
) -> CommandResult<AgentConversationSessionAnnotation> {
    command_result(
        manager
            .add_session_annotation(&owned_id, &url, &rect_json, &note)
            .map(Into::into),
    )
}

#[tauri::command]
/// Lists annotations for one conversation with the shared command error shape.
pub async fn agent_conversation_list_session_annotations(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> CommandResult<Vec<AgentConversationSessionAnnotation>> {
    command_result(
        manager
            .list_session_annotations(&owned_id)
            .map(|annotations| annotations.into_iter().map(Into::into).collect()),
    )
}

#[tauri::command]
/// Deletes one annotation and converts storage failures at the native boundary.
pub async fn agent_conversation_delete_session_annotation(
    manager: tauri::State<'_, AgentRuntimeManager>,
    id: i64,
) -> CommandResult<()> {
    command_result(manager.delete_session_annotation(id))
}

#[tauri::command]
/// Ensures a conversation runtime and returns typed failures to the frontend.
pub async fn ensure_agent_conversation(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: EnsureAgentConversationRequest,
) -> CommandResult<AgentConversationConnection> {
    let owned_id = request.owned_id.clone();
    let ensured = log_command_error(
        "ensure_agent_conversation",
        &owned_id,
        manager.ensure_async(request).await,
    )?;

    // Ensuring does not start an adapter, which is why a resumed conversation
    // opens on "Agent settings unavailable": the model list, the effort levels
    // and the command roster all come from a handshake, and the adapter is
    // otherwise started by a turn.
    //
    // Starting one here was tried and taken out again. Suspending afterwards
    // does not stop the process — the pool detaches the session and keeps the
    // adapter warm for the next turn — and the config it fetched did not
    // survive into the next ensure, so the "have we got models yet" test stayed
    // false and every open started another one. Several adapters were left
    // running, hundreds of megabytes each.
    //
    // Whatever replaces it has to keep the config it fetched, and has to stop
    // the process rather than suspend it.
    Ok(ensured)
}

#[tauri::command]
/// Sends one structured message and returns typed failures without logging its content.
pub async fn send_agent_conversation_message(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: SendAgentConversationMessageRequest,
) -> CommandResult<()> {
    let owned_id = request.owned_id.clone();
    let result = async {
        let mut prompt = prompt_from_blocks(request.text.trim(), request.content)?;
        // The ids come from the composer, not from the image bytes, so they are
        // attached here rather than inside the content block conversion.
        prompt.attachment_ids = request.attachment_ids;
        manager
            .send_message(
                &request.owned_id,
                request.generation,
                prompt,
                request.model,
                request.approval_policy,
            )
            .await
    }
    .await;
    log_command_error("send_agent_conversation_message", &owned_id, result)
}

#[tauri::command]
/// Persists one draft and converts storage failures at the native boundary.
pub async fn agent_conversation_set_session_draft(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
    text: String,
) -> CommandResult<()> {
    command_result(manager.set_session_draft(&owned_id, &text))
}

#[tauri::command]
/// Reads one persisted draft with the shared command error shape.
pub async fn agent_conversation_get_session_draft(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> CommandResult<Option<String>> {
    command_result(manager.get_session_draft(&owned_id))
}

#[tauri::command]
/// Clears one persisted draft and converts storage failures at the native boundary.
pub async fn agent_conversation_clear_session_draft(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> CommandResult<()> {
    command_result(manager.clear_session_draft(&owned_id))
}

#[tauri::command]
pub async fn write_agent_conversation_workspace(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
    snapshot_json: String,
) -> CommandResult<()> {
    command_result(manager.write_workspace(&owned_id, &snapshot_json))
}

#[tauri::command]
pub async fn read_agent_conversation_workspace(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> CommandResult<Option<String>> {
    command_result(manager.read_workspace(&owned_id))
}

#[tauri::command]
pub async fn read_agent_conversation_workspace_expanded_paths(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
    root: String,
) -> CommandResult<Vec<String>> {
    command_result(manager.read_workspace_expanded_paths(&owned_id, &root))
}

#[tauri::command]
pub async fn write_agent_conversation_workspace_expanded_paths(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
    root: String,
    paths: Vec<String>,
) -> CommandResult<()> {
    command_result(manager.write_workspace_expanded_paths(&owned_id, &root, &paths))
}

#[tauri::command]
pub async fn delete_agent_conversation_workspace(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> CommandResult<()> {
    command_result(manager.delete_workspace(&owned_id))
}

#[tauri::command]
pub async fn clear_agent_conversation_workspace_editors(
    manager: tauri::State<'_, AgentRuntimeManager>,
) -> CommandResult<()> {
    command_result(manager.clear_workspace_editors())
}

#[tauri::command]
pub async fn clear_agent_conversation_workspace_tabs(
    manager: tauri::State<'_, AgentRuntimeManager>,
) -> CommandResult<()> {
    command_result(manager.clear_workspace_tabs())
}

#[tauri::command]
pub async fn write_assembly_setting(
    manager: tauri::State<'_, AgentRuntimeManager>,
    setting_key: String,
    value_json: String,
) -> CommandResult<()> {
    command_result(manager.write_app_setting(&setting_key, &value_json))
}

#[tauri::command]
pub async fn read_assembly_setting(
    manager: tauri::State<'_, AgentRuntimeManager>,
    setting_key: String,
) -> CommandResult<Option<String>> {
    command_result(manager.read_app_setting(&setting_key))
}

#[tauri::command]
/// Records a legacy approval decision after validating its request identity.
pub async fn respond_agent_conversation_approval(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: RespondAgentConversationApprovalRequest,
) -> CommandResult<()> {
    let request_id = required_id(&request.request_id, "Approval request id")?;
    command_result(
        manager
            .respond_legacy_approval(
                &request.owned_id,
                request.generation,
                request_id,
                request.decision,
            )
            .await,
    )
}

#[tauri::command]
/// Records a provider permission choice after validating its request identity.
pub async fn respond_agent_conversation_permission(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: RespondAgentConversationPermissionRequest,
) -> CommandResult<()> {
    let request_id = required_id(&request.request_id, "Permission request id")?;
    command_result(
        manager
            .respond_permission_option(
                &request.owned_id,
                request.generation,
                request_id,
                request.option_id,
            )
            .await,
    )
}

#[tauri::command]
/// Records structured user input after validating its request identity.
pub async fn respond_agent_conversation_input(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: RespondAgentConversationInputRequest,
) -> CommandResult<()> {
    let request_id = required_id(&request.request_id, "User input request id")?;
    command_result(
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
            .await,
    )
}

#[tauri::command]
/// Stops the active turn for the requested conversation generation.
pub async fn stop_agent_conversation_turn(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: StopAgentConversationTurnRequest,
) -> CommandResult<()> {
    command_result(
        manager
            .cancel_turn(&request.owned_id, request.generation)
            .await,
    )
}

#[tauri::command]
/// Changes a Codex session's durable checkout while it is quiescent.
pub async fn change_agent_conversation_checkout(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: ChangeAgentConversationCheckoutRequest,
) -> CommandResult<AgentConversationSessionRecord> {
    let owned_id = request.owned_id.clone();
    log_command_error(
        "change_agent_conversation_checkout",
        &owned_id,
        manager.change_checkout(request).await,
    )
}

#[tauri::command]
/// Applies the supported conversation configuration fields for one generation.
pub async fn set_agent_conversation_config(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: SetAgentConversationConfigRequest,
) -> CommandResult<AgentConversationConfigState> {
    command_result(manager.set_conversation_config(request).await)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAgentConversationConfigOptionRequest {
    owned_id: String,
    generation: u64,
    option_id: String,
    value: providers::AgentConfigValue,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAgentConversationConfigOptionResponse {
    config_options: Vec<protocol::AgentConfigOption>,
}

#[tauri::command]
/// Applies one advertised provider option and returns the refreshed choices.
pub async fn set_agent_conversation_config_option(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: SetAgentConversationConfigOptionRequest,
) -> CommandResult<SetAgentConversationConfigOptionResponse> {
    command_result(
        manager
            .set_config(
                &request.owned_id,
                request.generation,
                &request.option_id,
                request.value,
            )
            .await
            .map(|config_options| SetAgentConversationConfigOptionResponse { config_options }),
    )
}

#[tauri::command]
/// Asks a conversation's adapter what it offers, once, and stops it again.
///
/// For a conversation resumed from a past transcript, which has never run and so
/// has never been told what it offers. What comes back is stored, and every
/// later read goes to the database the same way an existing session's does.
pub async fn warm_agent_conversation_config(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
    generation: u64,
) -> CommandResult<AgentConversationConfigState> {
    command_result(
        manager
            .warm_conversation_config(&owned_id, generation)
            .await,
    )
}

#[tauri::command]
/// Reads the current provider configuration for one conversation.
pub fn read_agent_conversation_config(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> CommandResult<AgentConversationConfigState> {
    command_result(manager.conversation_config(&owned_id))
}

#[tauri::command]
/// Reads the current provider capabilities for one conversation.
pub fn read_agent_conversation_capabilities(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> CommandResult<AgentCapabilities> {
    command_result(manager.capabilities_for_owned_id(&owned_id))
}

#[tauri::command]
/// Closes one exact conversation generation so stale views cannot close a replacement.
pub async fn close_agent_conversation(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
    generation: u64,
) -> CommandResult<bool> {
    command_result(manager.close(&owned_id, generation).await)
}

#[tauri::command]
/// Takes one conversation out of the store for good, stopping it first if it
/// is running. Answers whether there was anything to delete.
pub async fn delete_agent_conversation_session(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> CommandResult<bool> {
    let result = manager.delete(&owned_id).await;
    log_command_error("delete_agent_conversation_session", &owned_id, result)
}

#[tauri::command]
/// Reads the durable snapshot for one conversation.
pub async fn read_agent_conversation_snapshot(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
    request_id: u64,
) -> CommandResult<Option<AgentConversationSnapshot>> {
    command_result(manager.latest_snapshot(&owned_id, request_id))
}

#[tauri::command]
/// Cancels a snapshot whose frontend projection owner has been released.
pub fn cancel_agent_conversation_snapshot(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request_id: u64,
) {
    manager.cancel_snapshot(request_id);
}

#[tauri::command]
/// Lists durable conversation sessions with typed storage failures.
pub async fn list_agent_conversation_sessions(
    manager: tauri::State<'_, AgentRuntimeManager>,
) -> CommandResult<Vec<AgentConversationSessionRecord>> {
    command_result(manager.list_sessions())
}

#[tauri::command]
/// Lists durable events after the requested sequence for one conversation.
pub async fn list_agent_conversation_events(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
    from_sequence: Option<i64>,
) -> CommandResult<Vec<AgentConversationEvent>> {
    command_result(manager.list_events(&owned_id, from_sequence.unwrap_or(0)))
}

#[tauri::command]
/// Lists the page of durable events just older than the requested sequence.
pub async fn list_agent_conversation_events_before(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
    before_sequence: i64,
    max_bytes: u32,
) -> CommandResult<AgentConversationEventPage> {
    command_result(manager.list_events_before(&owned_id, before_sequence, max_bytes))
}

#[tauri::command]
/// Lists the page of durable events just newer than the requested sequence.
pub async fn list_agent_conversation_events_after(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
    after_sequence: i64,
    max_bytes: u32,
) -> CommandResult<AgentConversationEventPage> {
    command_result(manager.list_events_after(&owned_id, after_sequence, max_bytes))
}

#[tauri::command]
/// Updates owner-scoped conversation metadata and returns the stored record.
pub async fn update_agent_conversation_session_meta(
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: UpdateAgentConversationSessionMetaRequest,
) -> CommandResult<AgentConversationSessionRecord> {
    command_result(manager.update_session_meta(request))
}

#[tauri::command]
/// Reads a provider transcript while keeping transcript failures in the shared shape.
pub async fn read_agent_conversation_transcript(
    provider: String,
    native_session_id: String,
    child_session_id: Option<String>,
) -> CommandResult<transcript::TranscriptSnapshot> {
    command_result(transcript::read(
        &provider,
        &native_session_id,
        child_session_id.as_deref(),
    ))
}

/// One import page reads at most this many transcript bytes.
/// How much of a past transcript one read pulls in — the initial import and
/// each Load More alike. Bytes bound this rather than records because a single
/// turn carrying a large tool dump can outweigh many ordinary ones.
const IMPORT_MAX_BYTES: u64 = 2 * 1024 * 1024;
/// One import page keeps at most this many transcript events.
const IMPORT_MAX_RECORDS: usize = 2_000;

#[tauri::command]
/// Names a past provider transcript as an owned conversation and returns its owned id, reading no records.
pub async fn begin_agent_conversation_import(
    manager: tauri::State<'_, AgentRuntimeManager>,
    provider: protocol::AgentConversationProvider,
    native_session_id: String,
    transcript_path: String,
    cwd: String,
    title: Option<String>,
) -> CommandResult<String> {
    command_result(manager.begin_import_transcript_session(
        provider,
        &native_session_id,
        std::path::Path::new(&transcript_path),
        &cwd,
        title,
    ))
}

#[tauri::command]
/// Reads the newest page of a named import and returns how many events it gained.
pub async fn finish_agent_conversation_import(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> CommandResult<usize> {
    command_result(manager.finish_import_transcript_session(
        &owned_id,
        IMPORT_MAX_BYTES,
        IMPORT_MAX_RECORDS,
    ))
}

#[tauri::command]
/// Adds one older page to an imported conversation and returns how many events it gained.
pub async fn extend_agent_conversation_import(
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> CommandResult<transcript_import::ExtendedImport> {
    command_result(manager.extend_imported_session(&owned_id, IMPORT_MAX_BYTES, IMPORT_MAX_RECORDS))
}

#[tauri::command]
/// Starts transcript projection for one native conversation session.
pub async fn start_agent_conversation_terminal_projection(
    manager: tauri::State<'_, AgentRuntimeManager>,
    registry: tauri::State<'_, terminal_projection::TerminalProjectionRegistry>,
    request: terminal_projection::StartTerminalProjectionRequest,
) -> CommandResult<terminal_projection::TerminalProjectionRegistration> {
    command_result(registry.start(manager.inner().clone(), request))
}

#[tauri::command]
/// Stops transcript projection for one owner-scoped conversation.
pub async fn stop_agent_conversation_terminal_projection(
    registry: tauri::State<'_, terminal_projection::TerminalProjectionRegistry>,
    owned_id: String,
) -> CommandResult<bool> {
    command_result(registry.stop(&owned_id))
}

#[tauri::command]
/// Saves one validated image in the conversation attachment vault.
pub async fn save_agent_conversation_attachment(
    app: tauri::AppHandle,
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
    mime_type: String,
    bytes: Vec<u8>,
) -> CommandResult<attachments::SavedConversationAttachment> {
    command_result(attachments::save(
        &app,
        manager.store(),
        &owned_id,
        &mime_type,
        &bytes,
    ))
}

#[tauri::command]
/// Lists validated images stored for one conversation owner.
pub async fn read_agent_conversation_attachments(
    app: tauri::AppHandle,
    manager: tauri::State<'_, AgentRuntimeManager>,
    owned_id: String,
) -> CommandResult<Vec<attachments::SavedConversationAttachment>> {
    command_result(attachments::read(&app, manager.store(), &owned_id))
}

#[tauri::command]
/// Deletes one validated image from the conversation attachment vault.
pub async fn delete_agent_conversation_attachment(
    app: tauri::AppHandle,
    manager: tauri::State<'_, AgentRuntimeManager>,
    request: attachments::DeleteConversationAttachmentRequest,
) -> CommandResult<()> {
    command_result(attachments::delete(&app, manager.store(), request))
}

/// Rejects blank request identifiers before manager work begins.
fn required_id(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label} is required"))
    } else {
        Ok(value.to_string())
    }
}

/// Converts an internal string failure only when it crosses the native command boundary.
fn command_result<T>(result: Result<T, String>) -> CommandResult<T> {
    result.map_err(Into::into)
}

/// Logs a sanitized command failure and returns it in the shared boundary shape.
fn log_command_error<T>(
    command: &str,
    owned_id: &str,
    result: Result<T, String>,
) -> CommandResult<T> {
    if let Err(error) = &result {
        crate::debug_log::stderr_log!("{command} [{owned_id}]: {error}");
    }
    command_result(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_rejection_serializes_as_a_typed_error() {
        let rejection = command_result(required_id("", "Request id"));
        let serialized = serde_json::to_value(rejection.unwrap_err()).unwrap();

        assert_eq!(
            serialized,
            serde_json::json!({
                "code": "agent-conversation-command-failed",
                "message": "Request id is required",
                "recoverable": false
            })
        );
    }
}
