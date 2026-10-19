use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use sha2::{Digest, Sha256};
use crate::files::discovery::paths::{
    detect_language,
};
use crate::files::types::{
    SourcePreview,
};

static SOURCE_WRITE_LOCK: Mutex<()> = Mutex::new(());

pub(crate) const MAX_PREVIEW_BYTES: u64 = 4 * 1024 * 1024;
pub(crate) const MAX_SOURCE_IMAGE_BYTES: u64 = 16 * 1024 * 1024;
#[derive(Default)]
pub(crate) struct SourceFileReadOwner {
    generation: Arc<AtomicU64>,
}

impl SourceFileReadOwner {
    pub(crate) fn advance(&self, generation: u64) -> bool {
        generation >= self.generation.fetch_max(generation, Ordering::AcqRel)
    }

    pub(crate) fn generation(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.generation)
    }
}

pub(crate) fn read_source_file_sync(path: PathBuf) -> Result<SourcePreview, String> {
    read_source_file_sync_while(path, || true)?
        .ok_or_else(|| "Source read was cancelled".to_string())
}

pub(crate) fn read_source_image_sync(path: PathBuf) -> Result<Vec<u8>, String> {
    let metadata = std::fs::metadata(&path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err("Source image path is not a file".to_string());
    }
    if metadata.len() > MAX_SOURCE_IMAGE_BYTES {
        return Err(format!("Image is too large to preview: {} bytes", metadata.len()));
    }
    std::fs::read(&path)
        .map_err(|error| format!("Could not read source image {}: {error}", path.display()))
}

pub(crate) fn read_source_file_sync_while(
    path: PathBuf,
    keep_reading: impl Fn() -> bool,
) -> Result<Option<SourcePreview>, String> {
    let path_ref = path.as_path();
    let metadata = std::fs::metadata(path_ref)
        .map_err(|error| format!("Could not read {}: {error}", path_ref.display()))?;
    if !metadata.is_file() {
        return Err("Source path is not a file".to_string());
    }
    if metadata.len() > MAX_PREVIEW_BYTES {
        return Err(format!(
            "Source file is too large: {} bytes",
            metadata.len()
        ));
    }

    if !keep_reading() {
        return Ok(None);
    }
    let mut file = std::fs::File::open(path_ref)
        .map_err(|error| format!("Could not read source file as UTF-8: {error}"))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let mut chunk = [0_u8; 64 * 1024];
    loop {
        if !keep_reading() {
            return Ok(None);
        }
        let read = file
            .read(&mut chunk)
            .map_err(|error| format!("Could not read source file as UTF-8: {error}"))?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.len() as u64 > MAX_PREVIEW_BYTES {
            return Err(format!("Source file is too large: {} bytes", bytes.len()));
        }
    }
    if !keep_reading() {
        return Ok(None);
    }
    let revision = format!("{:x}", Sha256::digest(&bytes));
    let content = String::from_utf8(bytes)
        .map_err(|error| format!("Could not read source file as UTF-8: {error}"))?;
    let file_name = path_ref
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("source")
        .to_string();

    let preview = SourcePreview {
        path: path_ref.display().to_string(),
        relative_path: file_name.clone(),
        file_name,
        language: detect_language(path_ref),
        byte_count: metadata.len(),
        revision,
        line_count: content.lines().count(),
        content,
    };
    #[cfg(debug_assertions)]
    crate::debug_log::stderr_log!(
        "mcb tauri source.preview path={} language={} lines={}",
        preview.path,
        preview.language,
        preview.line_count
    );
    Ok(Some(preview))
}

pub(crate) fn write_source_file_sync(
    path: PathBuf,
    content: String,
    expected_revision: String,
) -> Result<SourcePreview, String> {
    // Assembly saves are serialized, then checked again immediately before the
    // atomic replacement so a competing save or ordinary external edit is not
    // silently replaced while the new bytes are prepared.
    let _write_guard = SOURCE_WRITE_LOCK
        .lock()
        .map_err(|_| "Source write lock is unavailable".to_string())?;
    let path_ref = path.as_path();
    let current = read_source_file_sync(path.clone())?;
    if current.revision != expected_revision {
        return Err("Source file changed on disk since it was opened.".to_string());
    }

    let byte_count = content.as_bytes().len() as u64;
    if byte_count > MAX_PREVIEW_BYTES {
        return Err(format!("Source content is too large: {byte_count} bytes"));
    }

    let parent = path_ref
        .parent()
        .ok_or_else(|| "Source path has no parent directory".to_string())?;
    let file_name = path_ref
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("source");
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let temp_path = parent.join(format!(
        ".{file_name}.mcb-write-{}-{unique}",
        std::process::id()
    ));

    std::fs::write(&temp_path, content.as_bytes())
        .and_then(|()| std::fs::set_permissions(&temp_path, std::fs::metadata(path_ref)?.permissions()))
        .map_err(|error| {
            let _ = std::fs::remove_file(&temp_path);
            format!("Could not write source temp file: {error}")
        })?;
    let latest = match read_source_file_sync(path.clone()) {
        Ok(preview) => preview,
        Err(error) => {
            let _ = std::fs::remove_file(&temp_path);
            return Err(error);
        }
    };
    if latest.revision != expected_revision {
        let _ = std::fs::remove_file(&temp_path);
        return Err("Source file changed on disk since it was opened.".to_string());
    }
    if let Err(error) = std::fs::rename(&temp_path, path_ref) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!("Could not replace source file: {error}"));
    }

    read_source_file_sync(path)
}
