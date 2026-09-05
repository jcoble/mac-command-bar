use crate::backend_capabilities;

#[tauri::command]
pub(crate) async fn read_backend_capabilities() -> Result<Vec<String>, String> {
    Ok(backend_capabilities())
}

/// Open the web inspector on the shell's own window.
///
/// The embedded browser's tabs have had this for a while; the window the app
/// itself is drawn in did not, so looking at the shell meant reaching for the
/// context menu. WebKit offers its own shortcut, but only where the inspector is
/// compiled in, which before the `devtools` feature meant debug builds alone.
#[tauri::command]
pub(crate) fn open_main_devtools(window: tauri::WebviewWindow) -> Result<(), String> {
    window.open_devtools();
    Ok(())
}
