use crate::backend_capabilities;

#[tauri::command]
pub(crate) async fn read_backend_capabilities() -> Result<Vec<String>, String> {
    Ok(backend_capabilities())
}
