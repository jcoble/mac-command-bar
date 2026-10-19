use mcb_core::scanners::sessions::{
    scan_session_details, scan_sessions, scan_sessions_for_project, AgentSessionRecord,
};
use mcb_core::session_store::SessionStore;

use crate::agent_conversation::manager::AgentRuntimeManager;

/// The scan with each session's project group on it, matched in SQL against this Mac's projects.
fn grouped(store: &SessionStore, mut sessions: Vec<AgentSessionRecord>) -> Result<Vec<AgentSessionRecord>, String> {
    crate::project_folders::attach_scanned_project_groups(store, &mut sessions)?;
    Ok(sessions)
}

#[tauri::command]
pub(crate) async fn list_agent_sessions(
    manager: tauri::State<'_, AgentRuntimeManager>,
) -> Result<Vec<AgentSessionRecord>, String> {
    let store = manager.store_handle();
    tauri::async_runtime::spawn_blocking(move || grouped(&store, scan_sessions()))
        .await
        .map_err(|error| format!("Agent session scan task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn list_agent_sessions_for_project(
    manager: tauri::State<'_, AgentRuntimeManager>,
    project_path: String,
) -> Result<Vec<AgentSessionRecord>, String> {
    let store = manager.store_handle();
    tauri::async_runtime::spawn_blocking(move || grouped(&store, scan_sessions_for_project(&project_path)))
        .await
        .map_err(|error| format!("Agent session scan task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn read_agent_session_details(
    manager: tauri::State<'_, AgentRuntimeManager>,
    log_path: String,
) -> Result<Vec<AgentSessionRecord>, String> {
    let store = manager.store_handle();
    tauri::async_runtime::spawn_blocking(move || grouped(&store, scan_session_details(&log_path)))
        .await
        .map_err(|error| format!("Agent session detail task failed: {error}"))?
}

/// Reads the displaying desktop's saved copy only. Never selects or contacts a runtime.
#[tauri::command]
pub(crate) async fn read_remote_saved_session_details(
    manager: tauri::State<'_, AgentRuntimeManager>,
    profile_id: String,
    owned_id: String,
) -> Result<mcb_core::session_store::RemoteSavedSessionDetails, String> {
    manager.store_handle().remote_saved_session_details(&profile_id, &owned_id)
        .map_err(|error| error.to_string())
}
