use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use base64::Engine;
use minisign_verify::{PublicKey, Signature};
use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::Manager;
use tokio::io::AsyncWriteExt;

use super::packaged::{self, AdapterPairs, PackagedAdapter};
use super::ProviderRegistry;
use crate::agent_conversation::protocol::AgentConversationProvider;

const RELEASE_BASE_URL: &str = "https://github.com/jcoble/mac-command-bar/releases/download/assembly-providers-stable";
const SIGNING_PUBLIC_KEY: &str = "RWQZvQJuc5RPnp9xO8+V9ppE3cCiodEFHPYqJpIMMVRhAnaxo0udp8xh";
static INSTALL_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
// Google’s official Linux server is 919,951,920 bytes in the signed 1.2.1 release.
const MAX_ADAPTER_FILE_BYTES: u64 = 1024 * 1024 * 1024;

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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUpdateVersion {
    provider: String,
    current_version: String,
    available_version: String,
    update_available: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
pub async fn check_provider_updates(
    app: tauri::AppHandle,
    manager: tauri::State<'_, crate::agent_conversation::manager::AgentRuntimeManager>,
) -> Result<ProviderUpdateStatus, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Application data directory is unavailable: {error}"))?;
    check_at(&app_data_dir, manager.providers()).await
}

#[tauri::command]
pub async fn install_provider_updates(
    app: tauri::AppHandle,
    manager: tauri::State<'_, crate::agent_conversation::manager::AgentRuntimeManager>,
) -> Result<ProviderUpdateStatus, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Application data directory is unavailable: {error}"))?;
    install_at(&app_data_dir, manager.providers()).await
}

pub(crate) async fn check_at(
    app_data_dir: &Path,
    providers: &ProviderRegistry,
) -> Result<ProviderUpdateStatus, String> {
    let (_, manifest) = fetch_signed_manifest().await?;
    status_for(app_data_dir, &manifest, &running_versions(providers)?)
}

pub(crate) async fn install_at(
    app_data_dir: &Path,
    providers: &ProviderRegistry,
) -> Result<ProviderUpdateStatus, String> {
    // Native commands and remote clients share the same installation owner.
    // Reject overlap instead of queueing an obsolete second bundle activation.
    let _installation = INSTALL_LOCK.try_lock()
        .map_err(|_| "A provider adapter installation is already running".to_string())?;
    let (manifest_bytes, manifest) = fetch_signed_manifest().await?;
    let running = running_versions(providers)?;
    let before = status_for(app_data_dir, &manifest, &running)?;
    if !before.update_available {
        return Ok(before);
    }
    ensure_no_downgrades(&before)?;
    install_manifest(app_data_dir, &manifest_bytes, &manifest).await?;
    status_for(app_data_dir, &manifest, &running)
}

#[tauri::command]
pub fn restart_for_provider_updates(
    app: tauri::AppHandle,
    manager: tauri::State<'_, crate::agent_conversation::manager::AgentRuntimeManager>,
) -> Result<(), String> {
    if manager.has_pending_provider_work() {
        return Err("Provider adapters are installed. Finish or stop active conversations before restarting Assembly.".into());
    }
    app.restart();
}

fn ensure_no_downgrades(status: &ProviderUpdateStatus) -> Result<(), String> {
    for provider in &status.providers {
        if Version::parse(&provider.available_version).map_err(|error| error.to_string())?
            < Version::parse(&provider.current_version).map_err(|error| error.to_string())?
        {
            return Err(format!(
                "Provider update would downgrade {}; the installed adapters were kept",
                provider.provider
            ));
        }
    }
    Ok(())
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
    verify_manifest_signature(&manifest_bytes, &signature_bytes, SIGNING_PUBLIC_KEY)?;
    let manifest: RemoteManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("Provider update manifest is invalid: {error}"))?;
    if manifest.schema_version != 1 || manifest.target != target {
        return Err("Provider update manifest does not match this build".to_string());
    }
    validate_remote_manifest(&manifest)?;
    Ok((manifest_bytes, manifest))
}

fn verify_manifest_signature(manifest_bytes: &[u8], signature_bytes: &[u8], public_key: &str) -> Result<(), String> {
    let public_key = PublicKey::from_base64(public_key)
        .map_err(|error| format!("Provider update public key is invalid: {error}"))?;
    // Tauri signer emits a base64 envelope around the Minisign text.
    let encoded = std::str::from_utf8(signature_bytes)
        .map_err(|_| "Provider update signature is not UTF-8".to_string())?;
    let decoded = base64::engine::general_purpose::STANDARD.decode(encoded.trim())
        .map_err(|error| format!("Provider update signature encoding is invalid: {error}"))?;
    let signature = Signature::decode(
        std::str::from_utf8(&decoded)
            .map_err(|_| "Provider update signature is not UTF-8".to_string())?,
    )
    .map_err(|error| format!("Provider update signature is invalid: {error}"))?;
    public_key
        .verify(&manifest_bytes, &signature, false)
        .map_err(|error| format!("Provider update signature verification failed: {error}"))?;
    Ok(())
}

async fn download_small(client: &Client, url: &str) -> Result<Vec<u8>, String> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Could not download provider update metadata: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Provider update metadata is unavailable: {error}"))?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("Could not read provider update metadata: {error}"))?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_MANIFEST_BYTES {
            return Err("Provider update metadata is too large".to_string());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
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
    running: &[(String, String)],
) -> Result<ProviderUpdateStatus, String> {
    let current = active_versions(app_data_dir)?;
    let restart_required = current.iter().any(|(provider, version)| {
        running.iter().find(|(name, _)| name == provider).map(|(_, active)| active) != Some(version)
    });
    let mut providers = Vec::with_capacity(manifest.adapters.len());
    for adapter in &manifest.adapters {
        let running_version = running.iter().find(|(provider, _)| provider == &adapter.provider)
            .map(|(_, version)| version.as_str())
            .ok_or_else(|| format!("No running adapter version for {}", adapter.provider))?;
        let current_version = current
            .iter()
            .find(|(provider, _)| provider == &adapter.provider)
            .map(|(_, version)| version.as_str())
            .unwrap_or(running_version);
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

// Own only this installation's files. Dropping an interrupted download must
// clean them without touching the directory selected by the active pointer.
struct PendingInstallation(Option<PathBuf>);

impl Drop for PendingInstallation {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

async fn install_manifest(
    app_data_dir: &Path,
    manifest_bytes: &[u8],
    manifest: &RemoteManifest,
) -> Result<(), String> {
    let root = app_data_dir.join("provider-adapters");
    std::fs::create_dir_all(&root)
        .map_err(|error| format!("Could not create provider update directory: {error}"))?;
    // Each verified install owns its directory; never reuse a damaged or
    // interrupted prior installation of the same manifest.
    let directory_name = format!("{}-{}", manifest.target, uuid::Uuid::new_v4());
    let destination = root.join(&directory_name);
    let temporary = root.join(format!("install-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&temporary)
        .map_err(|error| format!("Could not stage provider update: {error}"))?;
    let mut pending = PendingInstallation(Some(temporary.clone()));

    download_adapter_files(&temporary, manifest).await?;
    std::fs::write(temporary.join("manifest.json"), manifest_bytes)
        .map_err(|error| format!("Could not stage provider manifest: {error}"))?;
    packaged::discover_in(&temporary, false)?;

    // No await between promotion and pointer activation: cancellation cannot
    // race filesystem work and accidentally remove the newly active bundle.
    std::fs::rename(&temporary, &destination)
        .map_err(|error| format!("Could not install provider update: {error}"))?;
    pending.0 = Some(destination);
    let pointer = serde_json::to_vec(&ActiveAdapterDirectory {
        directory: directory_name,
    })
    .map_err(|error| format!("Could not encode provider update pointer: {error}"))?;
    let pointer_temporary = root.join(format!("active-{}.tmp", uuid::Uuid::new_v4()));
    let activation_result = (|| {
        std::fs::write(&pointer_temporary, pointer)
            .map_err(|error| format!("Could not stage provider update pointer: {error}"))?;
        std::fs::rename(&pointer_temporary, root.join("active.json"))
            .map_err(|error| format!("Could not activate provider update: {error}"))?;
        Ok::<(), String>(())
    })();
    if let Err(error) = activation_result {
        let _ = std::fs::remove_file(&pointer_temporary);
        return Err(error);
    }
    pending.0 = None;
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

fn running_versions(registry: &ProviderRegistry) -> Result<Vec<(String, String)>, String> {
    [
        ("codex", AgentConversationProvider::Codex),
        ("claude", AgentConversationProvider::Claude),
        ("antigravity", AgentConversationProvider::Antigravity),
    ].into_iter().map(|(name, provider)| {
        registry.manifest(provider).map(|manifest| (name.to_string(), manifest.version))
    }).collect()
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
        "linux" => "unknown-linux-gnu",
        other => return Err(format!("Provider updates do not support {other}")),
    };
    Ok(format!("{architecture}-{platform}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires the published GitHub provider release"]
    async fn published_provider_manifest_verifies_with_production_key() {
        let (_, manifest) = fetch_signed_manifest().await.unwrap();
        assert_eq!(manifest.adapters.len(), 3);
        // Linux ships Google’s largest payload; verify our shared download limit admits it.
        let linux = Client::new().head(format!("{RELEASE_BASE_URL}/provider-x86_64-unknown-linux-gnu-agy_acp_server.par"))
            .timeout(Duration::from_secs(30)).send().await.unwrap().error_for_status().unwrap();
        let size: u64 = linux.headers().get(reqwest::header::CONTENT_LENGTH).unwrap().to_str().unwrap().parse().unwrap();
        assert!(size > 0 && size <= MAX_ADAPTER_FILE_BYTES, "Published Linux adapter is {size} bytes");
        for adapter in manifest.adapters {
            println!("Published {} {}", adapter.provider, adapter.version);
        }
    }

    #[test]
    fn provider_feed_is_independent_of_app_and_backend_releases() {
        assert!(RELEASE_BASE_URL.ends_with("/releases/download/assembly-providers-stable"));
    }

    #[test]
    fn mixed_bundle_cannot_downgrade_an_installed_adapter() {
        let mut status = ProviderUpdateStatus {
            target: release_target().unwrap(),
            update_available: true,
            restart_required: false,
            providers: vec![ProviderUpdateVersion {
                provider: "claude".into(),
                current_version: "0.81.2".into(),
                available_version: "0.76.0".into(),
                update_available: false,
            }],
        };
        assert!(ensure_no_downgrades(&status).is_err());
        status.providers[0].available_version = "0.81.2".into();
        assert!(ensure_no_downgrades(&status).is_ok());
        status.providers[0].available_version = "0.82.0".into();
        assert!(ensure_no_downgrades(&status).is_ok());
    }

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
        let running = vec![("codex".into(), "1.0.0".into())];
        let status = status_for(&directory, &manifest, &running).unwrap();
        assert!(status.update_available);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn installed_update_keeps_restart_required_until_runtime_uses_it() {
        let directory = std::env::temp_dir().join(format!("mcb-provider-restart-{}", uuid::Uuid::new_v4()));
        let root = directory.join("provider-adapters");
        std::fs::create_dir_all(root.join("installed")).unwrap();
        std::fs::write(root.join("active.json"), r#"{"directory":"installed"}"#).unwrap();
        let bytes = serde_json::json!({"schemaVersion":1,"target":release_target().unwrap(),"adapters":[{
            "provider":"codex","id":"codex-acp","version":"2.0.0","executable":"codex-acp","files":[]
        }]}).to_string();
        std::fs::write(root.join("installed/manifest.json"), &bytes).unwrap();
        let manifest = serde_json::from_str(&bytes).unwrap();
        let before = status_for(&directory, &manifest, &[("codex".into(), "1.0.0".into())]).unwrap();
        assert!(before.restart_required);
        assert!(!before.update_available);
        assert_eq!(before.providers[0].current_version, "2.0.0");
        let after = status_for(&directory, &manifest, &[("codex".into(), "2.0.0".into())]).unwrap();
        assert!(!after.restart_required);
        assert!(!after.update_available);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn tauri_signer_output_verifies_and_tampering_is_rejected() {
        // Public fixture generated by the same Tauri signer command used in CI.
        // Its disposable private key was discarded; this is not a release key.
        let public_key = "RWQaQV+jk36Ok4Nq0PBBqubOdKFz09ZM8mQggZWxaJSGmy1tlCTi7Ts3";
        let payload = "{\"fixture\":\"TSK-936 signed provider metadata\"}\n".as_bytes();
        let signature = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVRYVFWK2prMzZPazRkbnF3VW9LQTlwVUw3ZmZnQzNZU3hQekFrT1dpYjRQR096WXZ6ZWpzVnFLVmMreE1naFlhNFEzM3EyYjB0UDFTa3ZnWWtXMENWY2VmZGliV0g5NEFNPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkwMzEzMDI0CWZpbGU6bWFuaWZlc3QuanNvbgowMHFXWWQ1QXBPMGhId0ZHMThEVHNXWjdDVmhpQzZxcDVFTnFYNWd5THA3SWY1aDJxSW13WXVIYUpoeU5TTjNXRFdTZXRKclc1SUhrRXJuTEt5MEhEdz09Cg==".as_bytes();
        verify_manifest_signature(payload, signature, public_key).unwrap();
        assert!(verify_manifest_signature(b"tampered manifest", signature, public_key).is_err());
        assert!(verify_manifest_signature(payload, b"invalid signature", public_key).is_err());
        assert!(verify_manifest_signature(payload, signature, SIGNING_PUBLIC_KEY).is_err());
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
    async fn overlapping_native_install_is_rejected_before_network_or_disk_work() {
        let _owner = INSTALL_LOCK.lock().await;
        let directory = std::env::temp_dir().join(format!("mcb-provider-overlap-{}", uuid::Uuid::new_v4()));
        let registry = ProviderRegistry::new([]).unwrap();
        let result = install_at(&directory, &registry).await;
        assert!(result.unwrap_err().contains("already running"));
        assert!(!directory.exists());
    }

    #[tokio::test]
    async fn cancelled_installation_removes_only_its_owned_files() {
        let directory = std::env::temp_dir().join(format!("mcb-provider-abort-{}", uuid::Uuid::new_v4()));
        let active = directory.join("installed");
        let staging = directory.join("install-interrupted");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir(&staging).unwrap();
        std::fs::write(active.join("adapter"), b"usable installation").unwrap();
        std::fs::write(directory.join("active.json"), b"original pointer").unwrap();
        let owned = staging.clone();
        let (ready, started) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _pending = PendingInstallation(Some(owned.clone()));
            std::fs::write(owned.join("partial-download"), b"partial").unwrap();
            ready.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        started.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(!staging.exists());
        assert_eq!(std::fs::read(active.join("adapter")).unwrap(), b"usable installation");
        assert_eq!(std::fs::read(directory.join("active.json")).unwrap(), b"original pointer");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn payload_download_accepts_valid_bytes_and_rejects_corruption_or_truncation() {
        use tokio::io::AsyncReadExt;
        let directory = std::env::temp_dir().join(format!("mcb-provider-http-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let payload = b"verified provider payload";
        let hash = format!("{:x}", Sha256::digest(payload));
        let client = Client::builder().no_proxy().timeout(Duration::from_secs(3)).build().unwrap();
        for (name, expected_hash, extra_length) in [
            ("valid", hash.clone(), 0),
            ("corrupt", "0".repeat(64), 0),
            ("truncated", hash.clone(), 10),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0_u8; 2048];
                socket.read(&mut request).await.unwrap();
                let header = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", payload.len() + extra_length);
                socket.write_all(header.as_bytes()).await.unwrap();
                socket.write_all(payload).await.unwrap();
                socket.shutdown().await.unwrap();
            });
            let result = download_file(&client, &format!("http://{address}/adapter"), &directory.join(name), &expected_hash).await;
            server.await.unwrap();
            if name == "valid" {
                result.unwrap();
                assert_eq!(std::fs::read(directory.join(name)).unwrap(), payload);
            } else if name == "corrupt" {
                assert!(result.unwrap_err().contains("SHA-256"));
            } else {
                assert!(result.unwrap_err().contains("Could not read provider adapter"));
            }
        }
        std::fs::remove_dir_all(directory).unwrap();
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
