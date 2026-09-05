use crate::projection_streams::ProjectionStreams;
use crate::terminal::{self, TerminalRegistry, TerminalSessionInfo, TerminalStartRequest};

#[tauri::command]
pub(crate) async fn start_terminal_session(
    terminal_registry: tauri::State<'_, TerminalRegistry>,
    projection_streams: tauri::State<'_, ProjectionStreams>,
    request: TerminalStartRequest,
) -> Result<TerminalSessionInfo, String> {
    terminal::start_terminal_session(
        &terminal_registry,
        projection_streams.inner().clone(),
        request,
    )
}

#[tauri::command]
pub(crate) async fn list_terminal_sessions(
    terminal_registry: tauri::State<'_, TerminalRegistry>,
) -> Result<Vec<TerminalSessionInfo>, String> {
    terminal::list_terminal_sessions(&terminal_registry)
}

#[tauri::command]
pub(crate) async fn read_terminal_session_scrollback(
    terminal_registry: tauri::State<'_, TerminalRegistry>,
    session_id: String,
    max_bytes: Option<usize>,
) -> Result<Option<String>, String> {
    terminal::read_terminal_session_scrollback(&terminal_registry, &session_id, max_bytes)
}

#[tauri::command]
pub(crate) async fn write_terminal_session(
    terminal_registry: tauri::State<'_, TerminalRegistry>,
    session_id: String,
    data: String,
) -> Result<bool, String> {
    terminal::write_terminal_session(&terminal_registry, &session_id, &data)
}

#[tauri::command]
pub(crate) async fn resize_terminal_session(
    terminal_registry: tauri::State<'_, TerminalRegistry>,
    session_id: String,
    cols: Option<u16>,
    rows: Option<u16>,
) -> Result<bool, String> {
    terminal::resize_terminal_session(&terminal_registry, &session_id, cols, rows)
}

#[tauri::command]
pub(crate) async fn close_terminal_session(
    terminal_registry: tauri::State<'_, TerminalRegistry>,
    session_id: String,
) -> Result<bool, String> {
    terminal::close_terminal_session(&terminal_registry, &session_id)
}
