use mcb_core::session_store::{AttachmentRow, SessionStore};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
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
    store: &SessionStore,
    owned_id: &str,
    mime_type: &str,
    bytes: &[u8],
) -> Result<SavedConversationAttachment, String> {
    let extension = validate_image(mime_type, bytes)?;
    let owned_id = safe_segment(owned_id, "Owned session id")?;
    let id = uuid::Uuid::new_v4().to_string();
    let name = format!("screenshot-{id}.{extension}");
    let root = vault_root(app)?.join(owned_id);
    fs::create_dir_all(&root)
        .map_err(|error| format!("Could not create attachment folder: {error}"))?;
    let path = root.join(&name);
    fs::write(&path, bytes).map_err(|error| format!("Could not save screenshot: {error}"))?;
    // The canonical form only proves the file landed inside the managed folder.
    // The reported path is the one the row rebuilds, so a save and a later read
    // name the same file the same way.
    if !path
        .canonicalize()
        .map_err(|error| format!("Could not verify screenshot path: {error}"))?
        .starts_with(canonical_root(&root)?)
    {
        return Err("Saved screenshot escaped the managed attachment folder".to_string());
    }
    let byte_length =
        i64::try_from(bytes.len()).map_err(|_| "Attachment is too large to record".to_string())?;
    if let Err(error) = store.add_attachment(&AttachmentRow {
        id: id.clone(),
        owned_id: owned_id.to_string(),
        file_name: name.clone(),
        mime_type: mime_type.to_string(),
        byte_length,
        relative_path: format!("{owned_id}/{name}"),
        created_at_ms: now_ms(),
    }) {
        // A file with no row is invisible to every later read, so it leaves with the failure.
        let _ = fs::remove_file(&path);
        return Err(error.to_string());
    }
    Ok(SavedConversationAttachment {
        id,
        name,
        mime_type: mime_type.to_string(),
        path: path.display().to_string(),
        byte_length: bytes.len(),
    })
}

pub fn read<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    store: &SessionStore,
    owned_id: &str,
) -> Result<Vec<SavedConversationAttachment>, String> {
    let root = vault_root(app)?;
    store
        .list_attachments(owned_id)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|row| {
            Ok(SavedConversationAttachment {
                path: root.join(&row.relative_path).display().to_string(),
                id: row.id,
                name: row.file_name,
                mime_type: row.mime_type,
                byte_length: usize::try_from(row.byte_length)
                    .map_err(|_| "Attachment is too large to describe".to_string())?,
            })
        })
        .collect()
}

pub fn delete<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    store: &SessionStore,
    request: DeleteConversationAttachmentRequest,
) -> Result<(), String> {
    let attachment_id = uuid::Uuid::parse_str(request.attachment_id.trim())
        .map_err(|_| "Attachment id is invalid".to_string())?;
    let root = attachment_root(app, &request.owned_id)?;
    let root = canonical_root(&root)?;
    let target = validate_delete_target(&root, attachment_id, Path::new(&request.path))?;
    store
        .delete_attachment(&attachment_id.to_string())
        .map_err(|error| error.to_string())?;
    fs::remove_file(&target).map_err(|error| format!("Could not delete screenshot: {error}"))
}

/// The folder that holds every session's attachments, and the root that stored
/// relative paths are measured against.
fn vault_root<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    #[cfg(test)]
    {
        let _ = app;
        Ok(std::env::temp_dir().join(format!(
            "mcb-conversation-attachments-test-{}",
            std::process::id()
        )))
    }
    #[cfg(not(test))]
    {
        Ok(app
            .path()
            .app_data_dir()
            .map_err(|error| format!("Application data directory is unavailable: {error}"))?
            .join("conversation-attachments"))
    }
}

fn attachment_root<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    owned_id: &str,
) -> Result<PathBuf, String> {
    let owned_id = safe_segment(owned_id, "Owned session id")?;
    Ok(vault_root(app)?.join(owned_id))
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

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since_epoch| i64::try_from(since_epoch.as_millis()).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{
        attachment_root, delete, read, safe_segment, save, validate_delete_target, validate_image,
        DeleteConversationAttachmentRequest, MAX_ATTACHMENT_BYTES,
    };
    use mcb_core::session_store::{SessionRow, SessionStore};
    use std::fs;

    /// Attachment rows point at their session, so every test owns a stored session.
    fn store_with_session(owned_id: &str) -> SessionStore {
        let store = SessionStore::open_in_memory().expect("open in-memory session store");
        store
            .upsert_session(&SessionRow {
                owned_id: owned_id.to_owned(),
                native_session_id: None,
                provider: "provider-a".to_owned(),
                model: None,
                effort: None,
                cwd: "/work/project".to_owned(),
                worktree: None,
                branch: None,
                title: None,
                title_source: None,
                project: None,
                state: "idle".to_owned(),
                suspended: false,
                created_at_ms: 1_000,
                last_activity_at_ms: 2_000,
                extra_json: "{}".to_owned(),
            })
            .expect("insert owning session");
        store
    }

    #[test]
    fn saved_attachments_read_back_and_reflect_deletion() {
        let app = tauri::test::mock_app();
        let owned_id = format!("owned-{}", uuid::Uuid::new_v4());
        let store = store_with_session(&owned_id);
        let first = save(
            app.handle(),
            &store,
            &owned_id,
            "image/png",
            b"\x89PNG\r\n\x1a\nfirst",
        )
        .unwrap();
        let second = save(
            app.handle(),
            &store,
            &owned_id,
            "image/gif",
            b"GIF89asecond",
        )
        .unwrap();
        let mut expected = vec![first.clone(), second.clone()];
        expected.sort_by(|left, right| left.name.cmp(&right.name));

        assert_eq!(read(app.handle(), &store, &owned_id).unwrap(), expected);

        delete(
            app.handle(),
            &store,
            DeleteConversationAttachmentRequest {
                owned_id: owned_id.clone(),
                attachment_id: first.id,
                path: first.path,
            },
        )
        .unwrap();
        assert_eq!(read(app.handle(), &store, &owned_id).unwrap(), vec![second]);

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
