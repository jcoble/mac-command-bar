use std::collections::HashSet;
use std::path::{Component, Path};
use std::time::Duration;

use minisign_verify::{PublicKey, Signature};
use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::Manager;
use tokio::io::AsyncWriteExt;

use super::packaged::{self, AdapterPairs, PackagedAdapter};
use crate::agent_conversation::capabilities::{
    AGY_ACP_VERSION, CLAUDE_AGENT_ACP_VERSION, CODEX_ACP_VERSION,
};

const RELEASE_BASE_URL: &str = "https://github.com/jcoble/mac-command-bar/releases/latest/download";
const SIGNING_PUBLIC_KEY: &str = "RWQZvQJuc5RPnp9xO8+V9ppE3cCiodEFHPYqJpIMMVRhAnaxo0udp8xh";
const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_ADAPTER_FILE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteManifest {
    schema_version: u32,
    target: String,
    adapters: Vec<PackagedAdapter>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ActiveAdapterDirectory {
    directory: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUpdateVersion {
    provider: String,
    current_version: String,
    available_version: String,
    update_available: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUpdateStatus {
    target: String,
    update_available: bool,
    restart_required: bool,
    providers: Vec<ProviderUpdateVersion>,
}

pub(super) fn discover_active(app_data_dir: &Path) -> Result<AdapterPairs, String> {
    let root = app_data_dir.join("provider-adapters");
    let pointer_path = root.join("active.json");
    if !pointer_path.is_file() {
        return Ok((None, None, None));
    }
    let pointer: ActiveAdapterDirectory = serde_json::from_slice(
        &std::fs::read(&pointer_path)
            .map_err(|error| format!("Could not read active provider adapters: {error}"))?,
    )
    .map_err(|error| format!("Active provider adapter pointer is invalid: {error}"))?;
    if !safe_file_name(&pointer.directory) {
        return Err("Active provider adapter directory is invalid".to_string());
    }
    packaged::discover_in(&root.join(pointer.directory), false)
}

#[tauri::command]
pub async fn check_provider_updates(app: tauri::AppHandle) -> Result<ProviderUpdateStatus, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Application data directory is unavailable: {error}"))?;
    let (_, manifest) = fetch_signed_manifest().await?;
    status_for(&app_data_dir, &manifest, false)
}

#[tauri::command]
pub async fn install_provider_updates(
    app: tauri::AppHandle,
) -> Result<ProviderUpdateStatus, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Application data directory is unavailable: {error}"))?;
    let (manifest_bytes, manifest) = fetch_signed_manifest().await?;
    let before = status_for(&app_data_dir, &manifest, false)?;
    if !before.update_available {
        return Ok(before);
    }
    install_manifest(&app_data_dir, &manifest_bytes, &manifest).await?;
    status_for(&app_data_dir, &manifest, true)
}

async fn fetch_signed_manifest() -> Result<(Vec<u8>, RemoteManifest), String> {
    let target = release_target()?;
    let manifest_name = format!("provider-manifest-{target}.json");
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent("Assembly provider updater")
        .build()
        .map_err(|error| format!("Could not create provider update client: {error}"))?;
    let manifest_bytes =
        download_small(&client, &format!("{RELEASE_BASE_URL}/{manifest_name}")).await?;
    let signature_bytes =
        download_small(&client, &format!("{RELEASE_BASE_URL}/{manifest_name}.sig")).await?;
    let public_key = PublicKey::from_base64(SIGNING_PUBLIC_KEY)
        .map_err(|error| format!("Provider update public key is invalid: {error}"))?;
    let signature = Signature::decode(
        std::str::from_utf8(&signature_bytes)
            .map_err(|_| "Provider update signature is not UTF-8".to_string())?,
    )
    .map_err(|error| format!("Provider update signature is invalid: {error}"))?;
    public_key
        .verify(&manifest_bytes, &signature, false)
        .map_err(|error| format!("Provider update signature verification failed: {error}"))?;
    let manifest: RemoteManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("Provider update manifest is invalid: {error}"))?;
    if manifest.schema_version != 1 || manifest.target != target {
        return Err("Provider update manifest does not match this build".to_string());
    }
    validate_remote_manifest(&manifest)?;
    Ok((manifest_bytes, manifest))
}

async fn download_small(client: &Client, url: &str) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Could not download provider update metadata: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Provider update metadata is unavailable: {error}"))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("Could not read provider update metadata: {error}"))?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err("Provider update metadata is too large".to_string());
    }
    Ok(bytes.to_vec())
}

fn validate_remote_manifest(manifest: &RemoteManifest) -> Result<(), String> {
    if manifest.adapters.len() != 3 {
        return Err("Provider update manifest must contain all three adapters".to_string());
    }
    let expected = [
        ("codex", "codex-acp"),
        ("claude", "claude-agent-acp"),
        ("antigravity", "agy-acp"),
    ];
    let mut providers = HashSet::new();
    for adapter in &manifest.adapters {
        if !expected
            .iter()
            .any(|(provider, id)| adapter.provider == *provider && adapter.id == *id)
            || !providers.insert(adapter.provider.as_str())
            || adapter.files.is_empty()
            || !safe_file_name(&adapter.executable)
            || !adapter
                .files
                .iter()
                .any(|file| file.path == adapter.executable)
        {
            return Err(format!(
                "Provider update for {} is incomplete",
                adapter.provider
            ));
        }
        Version::parse(&adapter.version).map_err(|_| {
            format!(
                "Provider update for {} has an invalid version",
                adapter.provider
            )
        })?;
        for file in &adapter.files {
            if !safe_file_name(&file.path)
                || file.sha256.len() != 64
                || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(format!(
                    "Provider update file for {} is invalid",
                    adapter.provider
                ));
            }
        }
    }
    Ok(())
}

fn status_for(
    app_data_dir: &Path,
    manifest: &RemoteManifest,
    restart_required: bool,
) -> Result<ProviderUpdateStatus, String> {
    let current = active_versions(app_data_dir)?;
    let mut providers = Vec::with_capacity(manifest.adapters.len());
    for adapter in &manifest.adapters {
        let bundled_version = bundled_version(&adapter.provider)?;
        let current_version = current
            .iter()
            .find(|(provider, _)| provider == &adapter.provider)
            .map(|(_, version)| version.as_str())
            .unwrap_or(bundled_version);
        let update_available = Version::parse(&adapter.version)
            .map_err(|error| error.to_string())?
            > Version::parse(current_version).map_err(|error| error.to_string())?;
        providers.push(ProviderUpdateVersion {
            provider: adapter.provider.clone(),
            current_version: current_version.to_string(),
            available_version: adapter.version.clone(),
            update_available,
        });
    }
    Ok(ProviderUpdateStatus {
        target: manifest.target.clone(),
        update_available: providers.iter().any(|provider| provider.update_available),
        restart_required,
        providers,
    })
}

fn active_versions(app_data_dir: &Path) -> Result<Vec<(String, String)>, String> {
    let root = app_data_dir.join("provider-adapters");
    let pointer_path = root.join("active.json");
    if !pointer_path.is_file() {
        return Ok(Vec::new());
    }
    let pointer: ActiveAdapterDirectory = serde_json::from_slice(
        &std::fs::read(pointer_path)
            .map_err(|error| format!("Could not read active provider adapters: {error}"))?,
    )
    .map_err(|error| format!("Active provider adapter pointer is invalid: {error}"))?;
    if !safe_file_name(&pointer.directory) {
        return Err("Active provider adapter directory is invalid".to_string());
    }
    let manifest: packaged::PackagedManifest = serde_json::from_slice(
        &std::fs::read(root.join(pointer.directory).join("manifest.json"))
            .map_err(|error| format!("Could not read active provider manifest: {error}"))?,
    )
    .map_err(|error| format!("Active provider manifest is invalid: {error}"))?;
    Ok(manifest
        .adapters
        .into_iter()
        .map(|adapter| (adapter.provider, adapter.version))
        .collect())
}

async fn install_manifest(
    app_data_dir: &Path,
    manifest_bytes: &[u8],
    manifest: &RemoteManifest,
) -> Result<(), String> {
    let root = app_data_dir.join("provider-adapters");
    tokio::fs::create_dir_all(&root)
        .await
        .map_err(|error| format!("Could not create provider update directory: {error}"))?;
    let manifest_hash = format!("{:x}", Sha256::digest(manifest_bytes));
    let directory_name = format!("{}-{}", manifest.target, &manifest_hash[..16]);
    let destination = root.join(&directory_name);
    let temporary = root.join(format!("install-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir(&temporary)
        .await
        .map_err(|error| format!("Could not stage provider update: {error}"))?;

    let staging_result = async {
        download_adapter_files(&temporary, manifest).await?;
        tokio::fs::write(temporary.join("manifest.json"), manifest_bytes)
            .await
            .map_err(|error| format!("Could not stage provider manifest: {error}"))?;
        packaged::discover_in(&temporary, false)?;

        if destination.exists() {
            tokio::fs::remove_dir_all(&temporary)
                .await
                .map_err(|error| format!("Could not discard duplicate provider update: {error}"))?;
        } else {
            tokio::fs::rename(&temporary, &destination)
                .await
                .map_err(|error| format!("Could not install provider update: {error}"))?;
        }
        Ok::<(), String>(())
    }
    .await;
    if let Err(error) = staging_result {
        let _ = tokio::fs::remove_dir_all(&temporary).await;
        return Err(error);
    }
    let pointer = serde_json::to_vec(&ActiveAdapterDirectory {
        directory: directory_name,
    })
    .map_err(|error| format!("Could not encode provider update pointer: {error}"))?;
    let pointer_temporary = root.join(format!("active-{}.tmp", uuid::Uuid::new_v4()));
    let activation_result = async {
        tokio::fs::write(&pointer_temporary, pointer)
            .await
            .map_err(|error| format!("Could not stage provider update pointer: {error}"))?;
        tokio::fs::rename(&pointer_temporary, root.join("active.json"))
            .await
            .map_err(|error| format!("Could not activate provider update: {error}"))?;
        Ok::<(), String>(())
    }
    .await;
    if let Err(error) = activation_result {
        let _ = tokio::fs::remove_file(&pointer_temporary).await;
        return Err(error);
    }
    Ok(())
}

async fn download_adapter_files(directory: &Path, manifest: &RemoteManifest) -> Result<(), String> {
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(180))
        .user_agent("Assembly provider updater")
        .build()
        .map_err(|error| format!("Could not create provider download client: {error}"))?;
    for adapter in &manifest.adapters {
        for file in &adapter.files {
            let asset_name = format!("provider-{}-{}", manifest.target, file.path);
            download_file(
                &client,
                &format!("{RELEASE_BASE_URL}/{asset_name}"),
                &directory.join(&file.path),
                &file.sha256,
            )
            .await?;
        }
    }
    Ok(())
}

async fn download_file(
    client: &Client,
    url: &str,
    destination: &Path,
    expected_hash: &str,
) -> Result<(), String> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Could not download provider adapter: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Provider adapter is unavailable: {error}"))?;
    let mut file = tokio::fs::File::create(destination)
        .await
        .map_err(|error| format!("Could not stage provider adapter: {error}"))?;
    let mut hasher = Sha256::new();
    let mut written = 0_u64;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("Could not read provider adapter: {error}"))?
    {
        written = written.saturating_add(chunk.len() as u64);
        if written > MAX_ADAPTER_FILE_BYTES {
            return Err("Provider adapter exceeds the download limit".to_string());
        }
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|error| format!("Could not write provider adapter: {error}"))?;
    }
    file.flush()
        .await
        .map_err(|error| format!("Could not finish provider adapter: {error}"))?;
    if format!("{:x}", hasher.finalize()) != expected_hash.to_ascii_lowercase() {
        return Err("Provider adapter failed SHA-256 verification".to_string());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = tokio::fs::metadata(destination)
            .await
            .map_err(|error| format!("Could not read provider adapter permissions: {error}"))?
            .permissions();
        permissions.set_mode(0o755);
        tokio::fs::set_permissions(destination, permissions)
            .await
            .map_err(|error| format!("Could not make provider adapter executable: {error}"))?;
    }
    Ok(())
}

fn bundled_version(provider: &str) -> Result<&'static str, String> {
    match provider {
        "codex" => Ok(CODEX_ACP_VERSION),
        "claude" => Ok(CLAUDE_AGENT_ACP_VERSION),
        "antigravity" => Ok(AGY_ACP_VERSION),
        other => Err(format!("Unknown provider in update manifest: {other}")),
    }
}

fn safe_file_name(value: &str) -> bool {
    !value.is_empty()
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && Path::new(value).components().count() == 1
}

fn release_target() -> Result<String, String> {
    let architecture = match std::env::consts::ARCH {
        "aarch64" => "aarch64",
        "x86_64" => "x86_64",
        other => return Err(format!("Provider updates do not support {other}")),
    };
    let platform = match std::env::consts::OS {
        "macos" => "apple-darwin",
        other => return Err(format!("Provider updates do not support {other}")),
    };
    Ok(format!("{architecture}-{platform}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_pointer_cannot_escape_provider_directory() {
        assert!(safe_file_name("aarch64-apple-darwin-1234"));
        assert!(!safe_file_name("../outside"));
        assert!(!safe_file_name("nested/value"));
    }

    #[test]
    fn update_status_only_accepts_newer_versions() {
        let manifest = RemoteManifest {
            schema_version: 1,
            target: release_target().unwrap(),
            adapters: vec![PackagedAdapter {
                provider: "codex".into(),
                id: "codex-acp".into(),
                version: "99.0.0".into(),
                executable: "codex-acp".into(),
                files: vec![packaged::PackagedFile {
                    path: "codex-acp".into(),
                    sha256: "0".repeat(64),
                }],
            }],
        };
        let directory = std::env::temp_dir().join(format!(
            "mcb-provider-update-status-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let status = status_for(&directory, &manifest, false).unwrap();
        assert!(status.update_available);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn provider_manifest_public_key_is_valid() {
        PublicKey::from_base64(SIGNING_PUBLIC_KEY).unwrap();
    }

    #[test]
    fn provider_manifest_requires_each_expected_executable() {
        let adapters = [
            ("codex", "codex-acp"),
            ("claude", "claude-agent-acp"),
            ("antigravity", "agy-acp"),
        ]
        .map(|(provider, id)| PackagedAdapter {
            provider: provider.into(),
            id: id.into(),
            version: "1.0.0".into(),
            executable: id.into(),
            files: vec![packaged::PackagedFile {
                path: id.into(),
                sha256: "0".repeat(64),
            }],
        });
        let valid = RemoteManifest {
            schema_version: 1,
            target: release_target().unwrap(),
            adapters: adapters.into_iter().collect(),
        };
        assert!(validate_remote_manifest(&valid).is_ok());

        let mut missing_executable = valid;
        missing_executable.adapters[0].files[0].path = "different-file".into();
        assert!(validate_remote_manifest(&missing_executable).is_err());
    }

    #[tokio::test]
    async fn failed_install_removes_staging_directory() {
        let directory = std::env::temp_dir().join(format!(
            "mcb-provider-update-cleanup-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let manifest = RemoteManifest {
            schema_version: 1,
            target: release_target().unwrap(),
            adapters: Vec::new(),
        };

        assert!(install_manifest(&directory, b"invalid manifest", &manifest)
            .await
            .is_err());
        let provider_root = directory.join("provider-adapters");
        assert!(std::fs::read_dir(provider_root).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("install-")));

        std::fs::remove_dir_all(directory).unwrap();
    }
}
