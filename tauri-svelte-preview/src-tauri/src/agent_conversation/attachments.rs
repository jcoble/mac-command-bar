use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

const MAX_ATTACHMENT_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedConversationAttachment {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub path: String,
    pub byte_length: usize,
}

pub fn save(
    app: &tauri::AppHandle,
    owned_id: &str,
    mime_type: &str,
    bytes: &[u8],
) -> Result<SavedConversationAttachment, String> {
    let owned_id = safe_segment(owned_id, "Owned session id")?;
    let extension = validate_image(mime_type, bytes)?;
    let id = uuid::Uuid::new_v4().to_string();
    let name = format!("screenshot-{id}.{extension}");
    let root = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Application data directory is unavailable: {error}"))?
        .join("conversation-attachments")
        .join(owned_id);
    fs::create_dir_all(&root)
        .map_err(|error| format!("Could not create attachment folder: {error}"))?;
    let path = root.join(&name);
    fs::write(&path, bytes).map_err(|error| format!("Could not save screenshot: {error}"))?;
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("Could not verify screenshot path: {error}"))?;
    if !canonical.starts_with(canonical_root(&root)?) {
        return Err("Saved screenshot escaped the managed attachment folder".to_string());
    }
    Ok(SavedConversationAttachment {
        id,
        name,
        mime_type: mime_type.to_string(),
        path: canonical.display().to_string(),
        byte_length: bytes.len(),
    })
}

fn validate_image<'a>(mime_type: &str, bytes: &'a [u8]) -> Result<&'static str, String> {
    let (extension, expected_magic): (&str, &[u8]) = match mime_type {
        "image/png" => ("png", b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => ("jpg", b"\xff\xd8\xff"),
        "image/gif" => ("gif", b"GIF8"),
        "image/webp" => ("webp", b"RIFF"),
        _ => return Err("Only PNG, JPEG, GIF, and WebP images are supported".to_string()),
    };
    if bytes.is_empty() || bytes.len() > MAX_ATTACHMENT_BYTES {
        return Err("Image must be between 1 byte and 20 MB".to_string());
    }
    if !bytes.starts_with(expected_magic)
        || (mime_type == "image/webp" && bytes.get(8..12) != Some(b"WEBP"))
    {
        return Err("Clipboard image data does not match its declared type".to_string());
    }
    Ok(extension)
}

fn canonical_root(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|error| format!("Could not verify attachment folder: {error}"))
}

fn safe_segment<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty()
        || value.contains('/')
        || value.contains('\\')
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(format!("{label} is invalid"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{safe_segment, validate_image, MAX_ATTACHMENT_BYTES};

    #[test]
    fn attachment_session_folder_rejects_path_traversal() {
        assert!(safe_segment("owned-good_1", "Owned session id").is_ok());
        assert!(safe_segment("../outside", "Owned session id").is_err());
    }

    #[test]
    fn attachment_validation_accepts_supported_image_signatures() {
        assert_eq!(
            validate_image("image/png", b"\x89PNG\r\n\x1a\nrest").unwrap(),
            "png"
        );
        assert_eq!(
            validate_image("image/jpeg", b"\xff\xd8\xffrest").unwrap(),
            "jpg"
        );
        assert_eq!(validate_image("image/gif", b"GIF89a").unwrap(), "gif");
        assert_eq!(
            validate_image("image/webp", b"RIFF0000WEBPrest").unwrap(),
            "webp"
        );
    }

    #[test]
    fn attachment_validation_rejects_spoofed_unsupported_and_oversized_images() {
        assert!(validate_image("image/png", b"not-a-png").is_err());
        assert!(validate_image("image/svg+xml", b"<svg></svg>").is_err());
        assert!(validate_image("image/png", &vec![0; MAX_ATTACHMENT_BYTES + 1]).is_err());
    }
}
