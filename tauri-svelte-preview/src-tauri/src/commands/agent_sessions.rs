use mcb_core::scanners::sessions::{
    scan_session_details, scan_sessions, scan_sessions_for_project, AgentSessionRecord,
};

#[tauri::command]
pub(crate) async fn list_agent_sessions() -> Result<Vec<AgentSessionRecord>, String> {
    tauri::async_runtime::spawn_blocking(scan_sessions)
        .await
        .map_err(|error| format!("Agent session scan task failed: {error}"))
}

#[tauri::command]
pub(crate) async fn list_agent_sessions_for_project(
    project_path: String,
) -> Result<Vec<AgentSessionRecord>, String> {
    tauri::async_runtime::spawn_blocking(move || scan_sessions_for_project(&project_path))
        .await
        .map_err(|error| format!("Agent session scan task failed: {error}"))
}

#[tauri::command]
pub(crate) async fn read_agent_session_details(
    log_path: String,
) -> Result<Vec<AgentSessionRecord>, String> {
    tauri::async_runtime::spawn_blocking(move || scan_session_details(&log_path))
        .await
        .map_err(|error| format!("Agent session detail task failed: {error}"))
}
