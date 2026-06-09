use std::path::Path;

const MAX_PREVIEW_BYTES: u64 = 512 * 1024;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourcePreview {
    path: String,
    relative_path: String,
    file_name: String,
    language: String,
    byte_count: u64,
    content: String,
    line_count: usize,
}

#[tauri::command]
fn read_source_file(path: String) -> Result<SourcePreview, String> {
    let path_ref = Path::new(&path);
    let metadata = std::fs::metadata(path_ref)
        .map_err(|error| format!("Could not read source metadata: {error}"))?;
    if !metadata.is_file() {
        return Err("Source path is not a file".to_string());
    }
    if metadata.len() > MAX_PREVIEW_BYTES {
        return Err(format!("Source file is too large: {} bytes", metadata.len()));
    }

    let content = std::fs::read_to_string(path_ref)
        .map_err(|error| format!("Could not read source file as UTF-8: {error}"))?;
    let file_name = path_ref
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("source")
        .to_string();

    Ok(SourcePreview {
        path: path_ref.display().to_string(),
        relative_path: file_name.clone(),
        file_name,
        language: detect_language(path_ref),
        byte_count: metadata.len(),
        line_count: content.lines().count(),
        content,
    })
}

fn detect_language(path: &Path) -> String {
    match path.extension().and_then(|value| value.to_str()) {
        Some("cs") => "csharp".to_string(),
        Some("swift") => "swift".to_string(),
        Some("rs") => "rust".to_string(),
        _ => "text".to_string(),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![read_source_file])
        .run(tauri::generate_context!())
        .expect("failed to run MacCommandBar webview preview");
}
