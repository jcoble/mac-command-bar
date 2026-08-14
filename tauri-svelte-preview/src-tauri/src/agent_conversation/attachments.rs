use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(not(test))]
use tauri::Manager;

const MAX_ATTACHMENT_BYTES: usize = 20 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedConversationAttachment {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub path: String,
    pub byte_length: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteConversationAttachmentRequest {
    pub owned_id: String,
    pub attachment_id: String,
    pub path: String,
}

pub fn save<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    owned_id: &str,
    mime_type: &str,
    bytes: &[u8],
) -> Result<SavedConversationAttachment, String> {
    let extension = validate_image(mime_type, bytes)?;
    let id = uuid::Uuid::new_v4().to_string();
    let name = format!("screenshot-{id}.{extension}");
    let root = attachment_root(app, owned_id)?;
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

pub fn read<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    owned_id: &str,
) -> Result<Vec<SavedConversationAttachment>, String> {
    let root = attachment_root(app, owned_id)?;
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not read attachment folder: {error}")),
    };
    let root = canonical_root(&root)?;
    let mut attachments = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("Could not read attachment entry: {error}"))?;
        if !entry
            .file_type()
            .map_err(|error| format!("Could not inspect attachment entry: {error}"))?
            .is_file()
        {
            continue;
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Attachment file name is invalid".to_string())?;
        let Some((id, extension)) = name
            .strip_prefix("screenshot-")
            .and_then(|name| name.rsplit_once('.'))
        else {
            continue;
        };
        if uuid::Uuid::parse_str(id).is_err() {
            continue;
        }
        let mime_type = match extension {
            "png" => "image/png",
            "jpg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            _ => continue,
        };
        let path = entry
            .path()
            .canonicalize()
            .map_err(|error| format!("Could not verify screenshot path: {error}"))?;
        if !path.starts_with(&root) || path.parent() != Some(root.as_path()) {
            return Err("Attachment path is outside the managed attachment folder".to_string());
        }
        let byte_length = usize::try_from(
            entry
                .metadata()
                .map_err(|error| format!("Could not inspect attachment: {error}"))?
                .len(),
        )
        .map_err(|_| "Attachment is too large to describe".to_string())?;
        attachments.push(SavedConversationAttachment {
            id: id.to_string(),
            name,
            mime_type: mime_type.to_string(),
            path: path.display().to_string(),
            byte_length,
        });
    }
    attachments.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(attachments)
}

pub fn delete<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    request: DeleteConversationAttachmentRequest,
) -> Result<(), String> {
    let attachment_id = uuid::Uuid::parse_str(request.attachment_id.trim())
        .map_err(|_| "Attachment id is invalid".to_string())?;
    let root = attachment_root(app, &request.owned_id)?;
    let root = canonical_root(&root)?;
    let target = validate_delete_target(&root, attachment_id, Path::new(&request.path))?;
    fs::remove_file(&target).map_err(|error| format!("Could not delete screenshot: {error}"))
}

fn attachment_root<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    owned_id: &str,
) -> Result<PathBuf, String> {
    let owned_id = safe_segment(owned_id, "Owned session id")?;
    #[cfg(test)]
    {
        let _ = app;
        Ok(std::env::temp_dir()
            .join(format!(
                "mcb-conversation-attachments-test-{}",
                std::process::id()
            ))
            .join(owned_id))
    }
    #[cfg(not(test))]
    {
        Ok(app
            .path()
            .app_data_dir()
            .map_err(|error| format!("Application data directory is unavailable: {error}"))?
            .join("conversation-attachments")
            .join(owned_id))
    }
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

fn validate_delete_target(
    root: &Path,
    attachment_id: uuid::Uuid,
    path: &Path,
) -> Result<PathBuf, String> {
    let target = path
        .canonicalize()
        .map_err(|error| format!("Could not verify screenshot path: {error}"))?;
    if !target.starts_with(root) || target.parent() != Some(root) {
        return Err("Attachment path is outside the managed attachment folder".to_string());
    }
    let expected_prefix = format!("screenshot-{attachment_id}.");
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Attachment path has no valid file name".to_string())?;
    if !file_name.starts_with(&expected_prefix) {
        return Err("Attachment id does not match the managed screenshot path".to_string());
    }
    Ok(target)
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
    use super::{
        attachment_root, delete, read, safe_segment, save, validate_delete_target, validate_image,
        DeleteConversationAttachmentRequest, MAX_ATTACHMENT_BYTES,
    };
    use std::fs;

    #[test]
    fn saved_attachments_read_back_and_reflect_deletion() {
        let app = tauri::test::mock_app();
        let owned_id = format!("owned-{}", uuid::Uuid::new_v4());
        let first = save(
            app.handle(),
            &owned_id,
            "image/png",
            b"\x89PNG\r\n\x1a\nfirst",
        )
        .unwrap();
        let second = save(app.handle(), &owned_id, "image/gif", b"GIF89asecond").unwrap();
        let mut expected = vec![first.clone(), second.clone()];
        expected.sort_by(|left, right| left.name.cmp(&right.name));

        assert_eq!(read(app.handle(), &owned_id).unwrap(), expected);

        delete(
            app.handle(),
            DeleteConversationAttachmentRequest {
                owned_id: owned_id.clone(),
                attachment_id: first.id,
                path: first.path,
            },
        )
        .unwrap();
        assert_eq!(read(app.handle(), &owned_id).unwrap(), vec![second]);

        let root = attachment_root(app.handle(), &owned_id).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

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

    #[test]
    fn attachment_delete_target_stays_inside_the_owned_folder() {
        let root =
            std::env::temp_dir().join(format!("mcb-attachment-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let id = uuid::Uuid::new_v4();
        let target = root.join(format!("screenshot-{id}.png"));
        fs::write(&target, b"png").unwrap();
        assert!(validate_delete_target(&root, id, &target).is_ok());
        assert!(validate_delete_target(&root, id, &root.join("../outside.png")).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
