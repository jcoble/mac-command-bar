use mcb_core::scanners::sessions::{
    scan_session_details, scan_sessions, scan_sessions_for_project, AgentSessionRecord,
};
use mcb_core::session_store::SessionStore;

use crate::agent_conversation::manager::AgentRuntimeManager;
use crate::agent_conversation::remote::RemoteConnectionManager;

/// The scan with each session's project group on it, matched in SQL against the projects this Mac registered for `machine`.
fn grouped(store: &SessionStore, machine: &str, mut sessions: Vec<AgentSessionRecord>) -> Result<Vec<AgentSessionRecord>, String> {
    crate::project_folders::attach_scanned_project_groups(store, machine, &mut sessions)?;
    Ok(sessions)
}

/// This Mac's past sessions, or with `remote_profile_id` the ones that machine's own scanner finds.
#[tauri::command]
pub(crate) async fn list_agent_sessions(
    manager: tauri::State<'_, AgentRuntimeManager>,
    remote: tauri::State<'_, RemoteConnectionManager>,
    remote_profile_id: Option<String>,
) -> Result<Vec<AgentSessionRecord>, String> {
    let store = manager.store_handle();
    if let Some(profile) = remote_profile_id {
        return grouped(&store, &profile, remote.scan_transcripts(&profile, None).await?);
    }
    tauri::async_runtime::spawn_blocking(move || grouped(&store, "local", scan_sessions()))
        .await
        .map_err(|error| format!("Agent session scan task failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn list_agent_sessions_for_project(
    manager: tauri::State<'_, AgentRuntimeManager>,
    project_path: String,
) -> Result<Vec<AgentSessionRecord>, String> {
    let store = manager.store_handle();
    tauri::async_runtime::spawn_blocking(move || grouped(&store, "local", scan_sessions_for_project(&project_path)))
        .await
        .map_err(|error| format!("Agent session scan task failed: {error}"))?
}

/// One transcript's details, read on the machine that has the file.
#[tauri::command]
pub(crate) async fn read_agent_session_details(
    manager: tauri::State<'_, AgentRuntimeManager>,
    remote: tauri::State<'_, RemoteConnectionManager>,
    log_path: String,
    remote_profile_id: Option<String>,
) -> Result<Vec<AgentSessionRecord>, String> {
    let store = manager.store_handle();
    if let Some(profile) = remote_profile_id {
        return grouped(&store, &profile, remote.scan_transcripts(&profile, Some(log_path)).await?);
    }
    tauri::async_runtime::spawn_blocking(move || grouped(&store, "local", scan_session_details(&log_path)))
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

#[cfg(test)]
mod tests {
    use mcb_core::session_store::{ProjectRow, SessionStore};

    use super::grouped;

    fn project(id: &str, machine: &str) -> ProjectRow {
        ProjectRow { id: id.into(), machine: machine.into(), root_path: "/work/edi".into(), title: id.into(), repo_key: String::new(),
            created_at_ms: 1, group_key: String::new(), pinned_at_ms: None, last_used_ms: None }
    }

    fn scanned(folder: &str) -> mcb_core::scanners::sessions::AgentSessionRecord {
        serde_json::from_value(serde_json::json!({ "provider": "claude", "id": "native", "title": "Past", "model": null,
            "projectPath": folder, "lastActivity": null, "resumeCommands": [] })).unwrap()
    }

    #[test]
    fn remote_scans_group_under_that_machines_projects() {
        let store = SessionStore::open_in_memory().unwrap();
        store.insert_or_get_project(&project("on-mac", "local")).unwrap();
        store.insert_or_get_project(&project("on-box", "box")).unwrap();
        let key = |machine: &str, folder: &str| grouped(&store, machine, vec![scanned(folder)]).unwrap()[0].project_group_key.clone();
        assert_eq!(key("box", "/work/edi/sub"), "project:on-box");
        assert_eq!(key("local", "/work/edi/sub"), "project:on-mac");
        assert_eq!(key("box", "/elsewhere"), "none");
    }
}
