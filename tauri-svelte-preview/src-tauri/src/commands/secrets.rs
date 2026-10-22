//! API keys and other secrets. They live in the operating system's keyring when
//! there is one. When there is none (headless Linux has no Secret Service), they
//! live in one private file, outside the app's data folder. Values never reach logs,
//! errors, `Debug` output or the frontend: the Tauri commands only ever say whether
//! a name is stored.

use serde::Serialize;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

const SERVICE: &str = "assembly";
const PROBE_NAME: &str = "backend-probe";

fn check_name(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let first_ok = chars.next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let rest_ok = chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
    if first_ok && rest_ok && name.len() <= 64 {
        Ok(())
    } else {
        Err("A secret name is 1 to 64 characters: lowercase letters, digits, '-' and '_'.".to_string())
    }
}

/// The names and values in the private file. `Debug` shows the names only.
#[derive(Default)]
struct FileSecrets(BTreeMap<String, String>);

impl std::fmt::Debug for FileSecrets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.0.keys()).finish()
    }
}

/// `$XDG_CONFIG_HOME/assembly/secrets.json`, else `~/.config/assembly/secrets.json`;
/// on macOS `~/Library/Application Support/assembly-secrets.json`. Never inside the app's data folder.
fn default_file_path(home: &Path, xdg_config_home: Option<&Path>) -> PathBuf {
    if cfg!(target_os = "macos") {
        return home.join("Library/Application Support/assembly-secrets.json");
    }
    let config = xdg_config_home.map(Path::to_path_buf).unwrap_or_else(|| home.join(".config"));
    config.join("assembly/secrets.json")
}

fn file_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").map(PathBuf::from).ok_or("The home folder is unavailable.")?;
    let xdg = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()).map(PathBuf::from);
    Ok(default_file_path(&home, xdg.as_deref()))
}

/// A file that other users can read or write is refused, not quietly fixed: it may already have been read.
fn load_file(path: &Path) -> Result<FileSecrets, String> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(FileSecrets::default()),
        Err(_) => return Err("The secrets file cannot be read.".to_string()),
    };
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(format!("The secrets file {} is readable by other users. Run chmod 600 on it, then try again.", path.display()));
    }
    let text = std::fs::read_to_string(path).map_err(|_| "The secrets file cannot be read.".to_string())?;
    let map = serde_json::from_str(&text).map_err(|_| "The secrets file is not valid JSON.".to_string())?;
    Ok(FileSecrets(map))
}

/// Written to a new 0600 file, then renamed over the old one, so the file is never visible with looser permissions.
fn save_file(path: &Path, secrets: &FileSecrets) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    let failed = |_| "The secrets file cannot be written.".to_string();
    let parent = path.parent().ok_or("The secrets file has no folder.")?;
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(parent).map_err(failed)?;
    let temp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let text = serde_json::to_string(&secrets.0).map_err(|_| "The secrets file cannot be written.".to_string())?;
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp)
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .and_then(|()| std::fs::rename(&temp, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    written.map_err(failed)
}

fn keyring_entry(name: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, name).map_err(|_| "The keyring is unavailable.".to_string())
}

fn keyring_set(name: &str, value: &str) -> Result<(), String> {
    keyring_entry(name)?.set_password(value).map_err(|_| "The keyring refused the secret.".to_string())
}

fn keyring_get(name: &str) -> Result<Option<String>, String> {
    match keyring_entry(name)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err("The keyring cannot be read.".to_string()),
    }
}

fn keyring_delete(name: &str) -> Result<(), String> {
    match keyring_entry(name)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err("The keyring refused the delete.".to_string()),
    }
}

fn set_with(
    file: &Path,
    name: &str,
    value: &str,
    keyring: impl FnOnce(&str, &str) -> Result<(), String>,
) -> Result<(), String> {
    check_name(name)?;
    if keyring(name, value).is_ok() {
        return Ok(());
    }
    let mut secrets = load_file(file)?;
    secrets.0.insert(name.to_string(), value.to_string());
    save_file(file, &secrets)
}

fn get_with(
    file: &Path,
    name: &str,
    keyring: impl FnOnce(&str) -> Result<Option<String>, String>,
) -> Result<Option<String>, String> {
    check_name(name)?;
    if let Ok(Some(value)) = keyring(name) {
        return Ok(Some(value));
    }
    Ok(load_file(file)?.0.remove(name))
}

fn delete_with(
    file: &Path,
    name: &str,
    keyring: impl FnOnce(&str) -> Result<(), String>,
) -> Result<(), String> {
    check_name(name)?;
    let _ = keyring(name);
    let mut secrets = load_file(file)?;
    if secrets.0.remove(name).is_some() {
        save_file(file, &secrets)?;
    }
    Ok(())
}

pub fn set(name: &str, value: &str) -> Result<(), String> {
    set_with(&file_path()?, name, value, keyring_set)
}

// Read when the API-model sidecar is launched with its keys; the commands never call it.
#[allow(dead_code)]
pub fn get(name: &str) -> Result<Option<String>, String> {
    get_with(&file_path()?, name, keyring_get)
}

pub fn delete(name: &str) -> Result<(), String> {
    delete_with(&file_path()?, name, keyring_delete)
}

/// `"keyring"` when the system keyring answers, else `"file"`.
pub fn backend() -> &'static str {
    if keyring_get(PROBE_NAME).is_ok() { "keyring" } else { "file" }
}

#[derive(Serialize)]
pub struct SecretStatus {
    name: String,
    stored: bool,
}

fn status_with(
    file: &Path,
    names: Vec<String>,
    keyring: impl Fn(&str) -> Result<Option<String>, String>,
) -> Result<Vec<SecretStatus>, String> {
    names
        .into_iter()
        .map(|name| {
            let stored = get_with(file, &name, &keyring)?.is_some();
            Ok(SecretStatus { name, stored })
        })
        .collect()
}

// The commands below never return a secret's value: the frontend can store, delete and ask "is it stored".

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tokio::task::spawn_blocking(work).await.map_err(|_| "The secrets task failed.".to_string())?
}

#[tauri::command]
pub async fn secret_set(name: String, value: String) -> Result<(), String> {
    blocking(move || set(&name, &value)).await
}

#[tauri::command]
pub async fn secret_delete(name: String) -> Result<(), String> {
    blocking(move || delete(&name)).await
}

#[tauri::command]
pub async fn secret_status(names: Vec<String>) -> Result<Vec<SecretStatus>, String> {
    blocking(move || status_with(&file_path()?, names, keyring_get)).await
}

#[tauri::command]
pub async fn secrets_backend() -> Result<&'static str, String> {
    blocking(|| Ok(backend())).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    const VALUE: &str = "sk-test-value-9f3a";

    fn scratch() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("assembly-secrets-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn no_keyring_set(_: &str, _: &str) -> Result<(), String> { Err("no keyring".to_string()) }
    fn no_keyring_get(_: &str) -> Result<Option<String>, String> { Err("no keyring".to_string()) }
    fn no_keyring_delete(_: &str) -> Result<(), String> { Err("no keyring".to_string()) }

    #[test]
    fn a_secret_round_trips_through_the_file_store() {
        let file = scratch().join("assembly/secrets.json");
        set_with(&file, "openrouter", VALUE, no_keyring_set).unwrap();
        assert_eq!(get_with(&file, "openrouter", no_keyring_get).unwrap().as_deref(), Some(VALUE));
        delete_with(&file, "openrouter", no_keyring_delete).unwrap();
        assert_eq!(get_with(&file, "openrouter", no_keyring_get).unwrap(), None);
    }

    #[test]
    fn the_file_is_created_0600_and_outside_the_data_folder() {
        let file = scratch().join("assembly/secrets.json");
        set_with(&file, "openrouter", VALUE, no_keyring_set).unwrap();
        assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(std::fs::metadata(file.parent().unwrap()).unwrap().permissions().mode() & 0o777, 0o700);
        let default = default_file_path(Path::new("/home/u"), None);
        assert!(!default.to_string_lossy().contains("dev.blackcolours"));
        assert!(default.to_string_lossy().contains("assembly"));
        let xdg = default_file_path(Path::new("/home/u"), Some(Path::new("/cfg")));
        assert!(cfg!(target_os = "macos") || xdg == Path::new("/cfg/assembly/secrets.json"));
    }

    #[test]
    fn a_file_other_users_can_read_is_refused() {
        let file = scratch().join("secrets.json");
        std::fs::write(&file, format!("{{\"openrouter\":\"{VALUE}\"}}")).unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
        let read = get_with(&file, "openrouter", no_keyring_get).unwrap_err();
        let write = set_with(&file, "other", "x", no_keyring_set).unwrap_err();
        assert!(read.contains("chmod 600") && write.contains("chmod 600"));
        assert!(!read.contains(VALUE));
    }

    #[test]
    fn a_bad_name_is_refused() {
        let file = scratch().join("secrets.json");
        for name in ["../x", "", "Upper", &"a".repeat(65), "-lead"] {
            assert!(set_with(&file, name, VALUE, no_keyring_set).is_err(), "{name:?}");
            assert!(get_with(&file, name, no_keyring_get).is_err(), "{name:?}");
            assert!(delete_with(&file, name, no_keyring_delete).is_err(), "{name:?}");
        }
        assert!(!file.exists());
        assert!(check_name(&"a".repeat(64)).is_ok());
    }

    #[test]
    fn delete_of_a_missing_secret_is_ok() {
        let file = scratch().join("secrets.json");
        delete_with(&file, "nothing-here", no_keyring_delete).unwrap();
    }

    #[test]
    fn a_keyring_error_falls_back_to_the_file() {
        let file = scratch().join("secrets.json");
        set_with(&file, "k", VALUE, no_keyring_set).unwrap();
        assert!(file.exists());
        // When the keyring works, the file is not touched.
        let other = scratch().join("secrets.json");
        set_with(&other, "k", VALUE, |_, _| Ok(())).unwrap();
        assert!(!other.exists());
        assert_eq!(get_with(&other, "k", |_| Ok(Some("from-keyring".to_string()))).unwrap().as_deref(), Some("from-keyring"));
    }

    #[test]
    fn no_error_or_debug_output_contains_the_value() {
        let dir = scratch();
        let file = dir.join("secrets.json");
        set_with(&file, "k", VALUE, no_keyring_set).unwrap();
        let debug = format!("{:?}", load_file(&file).unwrap());
        assert!(debug.contains('k') && !debug.contains(VALUE));
        // Every error path: a leaking value would show up in these.
        std::fs::write(&file, format!("{{broken {VALUE}")).unwrap();
        let broken = get_with(&file, "k", no_keyring_get).unwrap_err();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o666)).unwrap();
        let open = set_with(&file, "k", VALUE, no_keyring_set).unwrap_err();
        let bad_name = set_with(&file, VALUE, VALUE, no_keyring_set).unwrap_err();
        let unwritable = set_with(&dir.join("secrets.json/inside/x.json"), "k", VALUE, no_keyring_set).unwrap_err();
        for message in [broken, open, bad_name, unwritable] {
            assert!(!message.contains(VALUE), "{message}");
        }
    }

    #[test]
    fn secret_status_never_returns_values() {
        let file = scratch().join("secrets.json");
        set_with(&file, "have", VALUE, no_keyring_set).unwrap();
        let status = status_with(&file, vec!["have".to_string(), "lack".to_string()], no_keyring_get).unwrap();
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#"[{"name":"have","stored":true},{"name":"lack","stored":false}]"#);
        assert!(!json.contains(VALUE));
    }

    #[test]
    fn no_tauri_command_returns_a_secret_value() {
        let source = include_str!("secrets.rs").split("#[cfg(test)]").next().unwrap();
        let commands: Vec<&str> = source.lines().filter(|l| l.starts_with("pub async fn secret")).collect();
        assert_eq!(commands.len(), 4);
        for line in commands {
            let returns = line.split("->").nth(1).unwrap().trim();
            assert!(
                ["Result<(), String> {", "Result<Vec<SecretStatus>, String> {", "Result<&'static str, String> {"].contains(&returns),
                "{line}"
            );
        }
    }
}
