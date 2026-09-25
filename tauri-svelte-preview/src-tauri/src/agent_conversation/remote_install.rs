//! Signed release discovery and one-shot installation for a remote Assembly backend.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use base64::Engine;
use flate2::read::GzDecoder;
use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::ipc::Channel;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const RELEASES_ENDPOINT: &str = "repos/jcoble/mac-command-bar/releases?per_page=30";
const RELEASE_REPOSITORY: &str = "jcoble/mac-command-bar";
const RELEASE_TAG_PREFIX: &str = "assembly-backend-v";
const TARGET: &str = "x86_64-unknown-linux-gnu";
const SIGNING_PUBLIC_KEY: &str = "RWQZvQJuc5RPnp9xO8+V9ppE3cCiodEFHPYqJpIMMVRhAnaxo0udp8xh";
const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_SIGNATURE_BYTES: u64 = 16 * 1024;
const PAYLOAD_FILES: [&str; 10] = [
    "payload/assembly-remote-server",
    "payload/adapters/manifest.json",
    "payload/adapters/codex-acp",
    "payload/adapters/codex-acp-runtime",
    "payload/adapters/claude-agent-acp",
    "payload/adapters/claude-agent-acp-runtime",
    "payload/adapters/agy-acp",
    "payload/adapters/agy_acp_server.par",
    "payload/adapters/localharness_external",
    "install.sh",
];

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    id: u64,
    name: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackendManifest {
    schema_version: u32,
    package_type: String,
    version: String,
    commit: String,
    target: String,
    files: Vec<ManifestFile>,
}

#[derive(Debug, Deserialize)]
struct ManifestFile {
    path: String,
    sha256: String,
}

pub struct InstallReceipt {
    pub version: String,
    pub commit: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendStatus {
    pub installed: bool,
    pub installed_version: Option<String>,
    pub latest_version: String,
    pub update_available: Option<bool>,
}

struct LocalStage(PathBuf);

impl LocalStage {
    fn create() -> Result<Self, String> {
        let path =
            std::env::temp_dir().join(format!("assembly-remote-install-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path)
            .map_err(|error| format!("Could not create local install staging: {error}"))?;
        Ok(Self(path))
    }
}

impl Drop for LocalStage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct RemoteStage {
    target: String,
    path: String,
    armed: bool,
}

impl RemoteStage {
    fn new(target: &str, path: String) -> Self {
        Self {
            target: target.to_string(),
            path,
            armed: true,
        }
    }

    async fn cleanup(&mut self) -> Result<(), String> {
        ssh_output(
            &self.target,
            &format!("find '{}' -depth -delete 2>/dev/null || true", self.path),
        )
        .await?;
        self.armed = false;
        Ok(())
    }
}

impl Drop for RemoteStage {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let target = self.target.clone();
        let command = format!("find '{}' -depth -delete 2>/dev/null || true", self.path);
        std::thread::spawn(move || {
            let _ = std::process::Command::new("ssh")
                .args([
                    "-o",
                    "BatchMode=yes",
                    "-o",
                    "ConnectTimeout=10",
                    "-o",
                    "ControlMaster=no",
                    "-o",
                    "ControlPath=none",
                    "--",
                    &target,
                    &command,
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        });
    }
}

pub async fn install_latest(
    ssh_target: &str,
    status: Channel<String>,
) -> Result<InstallReceipt, String> {
    super::remote::validate_ssh_target(ssh_target)?;
    let _ = status.send("Finding the latest signed backend…".into());
    let (archive_asset, signature_asset) = latest_assets().await?;
    let stage = LocalStage::create()?;
    let archive_path = stage.0.join(&archive_asset.name);
    let signature_path = stage.0.join(&signature_asset.name);
    let _ = status.send("Downloading the signed Linux backend…".into());
    download(&archive_asset, &archive_path, MAX_ARCHIVE_BYTES).await?;
    download(&signature_asset, &signature_path, MAX_SIGNATURE_BYTES).await?;

    let _ = status.send("Verifying the package signature and contents…".into());
    let archive = archive_path.clone();
    let signature = signature_path.clone();
    let inspected = tokio::task::spawn_blocking(move || inspect_package(&archive, &signature))
        .await
        .map_err(|error| format!("Backend package verification task failed: {error}"))??;

    let _ = status.send("Checking the remote operating system…".into());
    let platform = ssh_output(ssh_target, "printf '%s %s' \"$(uname -s)\" \"$(uname -m)\"").await?;
    if platform != "Linux x86_64" {
        return Err(format!("Unsupported remote platform: {platform}"));
    }

    let staging_path = format!(
        "/tmp/.assembly-install-{}-{}",
        &inspected.commit[..12],
        uuid::Uuid::new_v4()
    );
    let mut remote_stage = RemoteStage::new(ssh_target, staging_path.clone());
    let remote_archive = format!("{staging_path}/package.tar.gz");
    ssh_output(ssh_target, &format!("install -d -m 700 '{staging_path}'")).await?;
    let result: Result<(), String> = async {
        let _ = status.send("Uploading the prebuilt backend package…".into());
        command_output(tokio::process::Command::new("scp")
            .args(["-q", "-o", "BatchMode=yes", "-o", "ConnectTimeout=10", "-o", "ControlMaster=no", "-o", "ControlPath=none", "--"])
            .arg(&archive_path)
            .arg(format!("{ssh_target}:{remote_archive}")), "Backend upload").await?;
        let _ = status.send("Installing and starting the backend…".into());
        ssh_output(ssh_target, &format!("set -eu; cd '{staging_path}'; tar -xzf package.tar.gz; sh install.sh")).await?;
        let _ = status.send("Verifying the service, database, and loopback listener…".into());
        let receipt = ssh_output(ssh_target, "set -eu; systemctl --user is-active --quiet assembly-remote.service; test -f \"$HOME/.local/share/assembly/sessions.db\"; listeners=$(ss -ltnH 'sport = :7777'); test -n \"$listeners\"; printf '%s\\n' \"$listeners\" | awk '$4 != \"127.0.0.1:7777\" { exit 1 } END { if (NR == 0) exit 1 }'; sha256sum \"$HOME/.local/bin/assembly-remote-server\" | awk '{print $1}'").await?;
        if receipt != inspected.server_sha256 {
            return Err("Installed backend checksum does not match the signed package".into());
        }
        Ok(())
    }.await;
    let cleanup = remote_stage.cleanup().await;
    result?;
    cleanup?;
    Ok(InstallReceipt {
        version: inspected.version,
        commit: inspected.commit,
    })
}

pub async fn latest_version() -> Result<String, String> {
    let release = latest_release().await?;
    release_version(&release.tag_name)
}

pub async fn read_status(
    ssh_target: &str,
    latest_version: &str,
) -> Result<BackendStatus, String> {
    super::remote::validate_ssh_target(ssh_target)?;
    let home = remote_home(ssh_target).await?;
    let binary = shell_quote(&format!("{home}/.local/bin/assembly-remote-server"));
    let manifest = shell_quote(&format!(
        "{home}/.local/share/assembly/backend-manifest.json"
    ));
    let output = ssh_output(
        ssh_target,
        &format!(
            "if test -x {binary}; then printf 'installed\\n'; test ! -f {manifest} || sed -n 's/.*\"version\": \"\\([^\"]*\\)\".*/\\1/p' {manifest} | head -n 1; else printf 'not-installed\\n'; fi"
        ),
    )
    .await?;
    let mut lines = output.lines();
    let installed = lines.next() == Some("installed");
    let installed_version = installed
        .then(|| lines.next().unwrap_or_default().trim().to_string())
        .filter(|value| !value.is_empty());
    let latest = semver::Version::parse(latest_version)
        .map_err(|_| format!("Invalid latest backend version: {latest_version}"))?;
    let update_available = installed_version.as_deref().and_then(|version| {
        semver::Version::parse(version)
            .ok()
            .map(|installed| installed < latest)
    });
    Ok(BackendStatus {
        installed,
        installed_version,
        latest_version: latest_version.to_string(),
        update_available,
    })
}

pub async fn uninstall(ssh_target: &str, delete_data: bool) -> Result<(), String> {
    super::remote::validate_ssh_target(ssh_target)?;
    let home = remote_home(ssh_target).await?;
    let service = shell_quote(&format!(
        "{home}/.config/systemd/user/assembly-remote.service"
    ));
    let binary = shell_quote(&format!("{home}/.local/bin/assembly-remote-server"));
    let adapters = shell_quote(&format!("{home}/.local/bin/assembly-adapters"));
    let manifest = shell_quote(&format!(
        "{home}/.local/share/assembly/backend-manifest.json"
    ));
    let mut command = format!(
        "set -eu; systemctl --user disable --now assembly-remote.service >/dev/null 2>&1 || true; rm -f {service} {binary} {manifest}; rm -rf {adapters}; systemctl --user daemon-reload"
    );
    if delete_data {
        let config = shell_quote(&format!("{home}/.config/assembly"));
        let data = shell_quote(&format!("{home}/.local/share/assembly"));
        command.push_str(&format!("; rm -rf {config} {data}"));
    }
    command.push_str(&format!(
        "; test ! -e {service}; test ! -e {binary}; test ! -e {adapters}; test -z \"$(ss -ltnH 'sport = :7777')\""
    ));
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        ssh_output(ssh_target, &command),
    )
    .await
    .map_err(|_| "Remote backend uninstall timed out after 30 seconds".to_string())??;
    Ok(())
}

async fn latest_assets() -> Result<(GithubAsset, GithubAsset), String> {
    release_assets(latest_release().await?)
}

async fn latest_release() -> Result<GithubRelease, String> {
    let metadata = command_output(
        tokio::process::Command::new("gh").args(["api", RELEASES_ENDPOINT]),
        "GitHub release lookup",
    )
    .await
    .map_err(|error| {
        format!(
            "Remote installation requires GitHub CLI access to {RELEASE_REPOSITORY}. Run `gh auth login`, then try again. {error}"
        )
    })?;
    let releases = serde_json::from_str::<Vec<GithubRelease>>(&metadata)
        .map_err(|error| format!("Backend release metadata is invalid: {error}"))?;
    releases
        .into_iter()
        .find(|release| {
            !release.draft
                && !release.prerelease
                && release.tag_name.starts_with(RELEASE_TAG_PREFIX)
        })
        .ok_or_else(|| "No published Assembly backend release is available".to_string())
}

fn release_version(tag_name: &str) -> Result<String, String> {
    let version = tag_name
        .strip_prefix(RELEASE_TAG_PREFIX)
        .ok_or_else(|| format!("Invalid Assembly backend release tag: {tag_name}"))?;
    semver::Version::parse(version)
        .map_err(|_| format!("Invalid Assembly backend release tag: {tag_name}"))?;
    Ok(version.to_string())
}

fn release_assets(mut release: GithubRelease) -> Result<(GithubAsset, GithubAsset), String> {
    let archive_suffix = format!("-{TARGET}.tar.gz");
    let archive_index = release
        .assets
        .iter()
        .position(|asset| asset.name.ends_with(&archive_suffix))
        .ok_or_else(|| format!("{} has no Linux x86-64 backend package", release.tag_name))?;
    let archive = release.assets.remove(archive_index);
    let signature_name = format!("{}.sig", archive.name);
    let signature_index = release
        .assets
        .iter()
        .position(|asset| asset.name == signature_name)
        .ok_or_else(|| format!("{} has no signature for {}", release.tag_name, archive.name))?;
    Ok((archive, release.assets.remove(signature_index)))
}

async fn download(asset: &GithubAsset, destination: &Path, limit: u64) -> Result<(), String> {
    if asset.size == 0 || asset.size > limit {
        return Err(format!(
            "Backend release asset has an invalid size: {}",
            asset.name
        ));
    }
    let endpoint = format!("repos/{RELEASE_REPOSITORY}/releases/assets/{}", asset.id);
    let mut child = tokio::process::Command::new("gh")
        .args([
            "api",
            "--header",
            "Accept: application/octet-stream",
            &endpoint,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("Could not download {}: {error}", asset.name))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("Could not read backend release asset: {}", asset.name))?;
    let mut file = tokio::fs::File::create(destination)
        .await
        .map_err(|error| format!("Could not stage {}: {error}", asset.name))?;
    let mut written = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = stdout
            .read(&mut buffer)
            .await
            .map_err(|error| format!("Could not read {}: {error}", asset.name))?;
        if read == 0 {
            break;
        }
        written = written.saturating_add(read as u64);
        if written > limit || written > asset.size {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(format!(
                "Backend release asset exceeded its declared size: {}",
                asset.name
            ));
        }
        file.write_all(&buffer[..read])
            .await
            .map_err(|error| format!("Could not write {}: {error}", asset.name))?;
    }
    file.flush()
        .await
        .map_err(|error| format!("Could not finish {}: {error}", asset.name))?;
    let output = child
        .wait_with_output()
        .await
        .map_err(|error| format!("Backend release download failed: {error}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if message.is_empty() {
            format!("Backend release asset is unavailable: {}", asset.name)
        } else {
            format!("Backend release asset is unavailable: {message}")
        });
    }
    if written != asset.size {
        return Err(format!(
            "Backend release asset was incomplete: {}",
            asset.name
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct InspectedPackage {
    version: String,
    commit: String,
    server_sha256: String,
}

fn inspect_package(archive_path: &Path, signature_path: &Path) -> Result<InspectedPackage, String> {
    verify_signature(archive_path, signature_path)?;
    inspect_package_contents(archive_path)
}

fn inspect_package_contents(archive_path: &Path) -> Result<InspectedPackage, String> {
    let extracted = LocalStage::create()?;
    let archive_file = std::fs::File::open(archive_path)
        .map_err(|error| format!("Could not open backend package: {error}"))?;
    let mut archive = tar::Archive::new(GzDecoder::new(archive_file));
    let expected_files = std::iter::once("manifest.json")
        .chain(std::iter::once("SHA256SUMS"))
        .chain(PAYLOAD_FILES)
        .map(str::to_string)
        .collect::<HashSet<_>>();
    let expected_directories = HashSet::from(["", "payload", "payload/adapters"]);
    let mut found_files = HashSet::new();
    for entry in archive
        .entries()
        .map_err(|error| format!("Could not inspect backend package: {error}"))?
    {
        let mut entry =
            entry.map_err(|error| format!("Backend package entry is invalid: {error}"))?;
        let path = entry
            .path()
            .map_err(|error| format!("Backend package path is invalid: {error}"))?;
        let normalized = path
            .to_string_lossy()
            .trim_start_matches("./")
            .trim_end_matches('/')
            .to_string();
        if entry.header().entry_type().is_dir() {
            if !expected_directories.contains(normalized.as_str()) {
                return Err(format!(
                    "Backend package contains unexpected directory: {normalized}"
                ));
            }
            continue;
        }
        if !entry.header().entry_type().is_file()
            || !expected_files.contains(normalized.as_str())
            || !found_files.insert(normalized.clone())
        {
            return Err(format!(
                "Backend package contains an unexpected entry: {normalized}"
            ));
        }
        let destination = extracted.0.join(&normalized);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("Could not prepare {normalized}: {error}"))?;
        }
        entry
            .unpack(destination)
            .map_err(|error| format!("Could not inspect {normalized}: {error}"))?;
    }
    if found_files != expected_files {
        return Err("Backend package is missing required files".into());
    }
    let manifest: BackendManifest = serde_json::from_slice(
        &std::fs::read(extracted.0.join("manifest.json"))
            .map_err(|error| format!("Could not read backend manifest: {error}"))?,
    )
    .map_err(|error| format!("Backend manifest is invalid: {error}"))?;
    validate_manifest(&manifest)?;
    for file in &manifest.files {
        let actual = file_sha256(&extracted.0.join(&file.path))?;
        if actual != file.sha256 {
            return Err(format!("Backend package checksum failed: {}", file.path));
        }
    }
    let server_sha256 = manifest
        .files
        .iter()
        .find(|file| file.path == "payload/assembly-remote-server")
        .map(|file| file.sha256.clone())
        .ok_or_else(|| "Backend manifest is missing the server binary".to_string())?;
    Ok(InspectedPackage {
        version: manifest.version,
        commit: manifest.commit,
        server_sha256,
    })
}

fn verify_signature(archive_path: &Path, signature_path: &Path) -> Result<(), String> {
    let encoded = std::fs::read_to_string(signature_path)
        .map_err(|error| format!("Could not read backend signature: {error}"))?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|error| format!("Backend signature encoding is invalid: {error}"))?;
    let signature = Signature::decode(
        std::str::from_utf8(&decoded).map_err(|_| "Backend signature is not UTF-8".to_string())?,
    )
    .map_err(|error| format!("Backend signature is invalid: {error}"))?;
    let key = PublicKey::from_base64(SIGNING_PUBLIC_KEY)
        .map_err(|error| format!("Backend signing key is invalid: {error}"))?;
    let mut verifier = key
        .verify_stream(&signature)
        .map_err(|error| format!("Could not verify backend signature: {error}"))?;
    let mut file = std::fs::File::open(archive_path)
        .map_err(|error| format!("Could not open backend package: {error}"))?;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Could not read backend package: {error}"))?;
        if read == 0 {
            break;
        }
        verifier.update(&buffer[..read]);
    }
    verifier
        .finalize()
        .map_err(|error| format!("Backend package signature verification failed: {error}"))
}

fn validate_manifest(manifest: &BackendManifest) -> Result<(), String> {
    if manifest.schema_version != 1
        || manifest.package_type != "assembly-remote-backend"
        || manifest.target != TARGET
    {
        return Err("Backend manifest is not supported".into());
    }
    semver::Version::parse(&manifest.version)
        .map_err(|_| "Backend manifest version is invalid".to_string())?;
    if !(7..=40).contains(&manifest.commit.len())
        || !manifest.commit.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("Backend manifest commit is invalid".into());
    }
    let expected = HashSet::from(PAYLOAD_FILES);
    let actual = manifest
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<HashSet<_>>();
    if actual != expected || manifest.files.len() != expected.len() {
        return Err("Backend manifest file list is invalid".into());
    }
    if manifest.files.iter().any(|file| {
        file.sha256.len() != 64 || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        return Err("Backend manifest contains an invalid checksum".into());
    }
    Ok(())
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Could not hash {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

async fn remote_home(target: &str) -> Result<String, String> {
    let home = ssh_output(target, "printf '%s' \"$HOME\"").await?;
    let path = Path::new(&home);
    if !path.is_absolute() || home == "/" || home.contains('\n') || home.contains('\r') {
        return Err("Remote home directory is invalid".to_string());
    }
    Ok(home)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

async fn ssh_output(target: &str, remote_command: &str) -> Result<String, String> {
    command_output(
        tokio::process::Command::new("ssh").args([
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=10",
            "-o",
            "ControlMaster=no",
            "-o",
            "ControlPath=none",
            "--",
            target,
            remote_command,
        ]),
        "Remote backend command",
    )
    .await
}

async fn command_output(
    command: &mut tokio::process::Command,
    label: &str,
) -> Result<String, String> {
    let output = command
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|error| format!("{label} could not start: {error}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if message.is_empty() {
            format!("{label} failed")
        } else {
            format!("{label} failed: {message}")
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires the published signed backend release and GitHub CLI access"]
    async fn published_backend_signature_and_payloads_verify() {
        let release = latest_release().await.unwrap();
        let expected_version = release_version(&release.tag_name).unwrap();
        let (archive_asset, signature_asset) = release_assets(release).unwrap();
        let stage = LocalStage::create().unwrap();
        let archive = stage.0.join("backend.tar.gz");
        let signature = stage.0.join("backend.tar.gz.sig");
        download(&archive_asset, &archive, MAX_ARCHIVE_BYTES).await.unwrap();
        download(&signature_asset, &signature, MAX_SIGNATURE_BYTES).await.unwrap();
        let receipt = tokio::task::spawn_blocking(move || inspect_package(&archive, &signature))
            .await.unwrap().unwrap();
        assert_eq!(receipt.version, expected_version);
        println!("Verified published backend {} at {}", receipt.version, receipt.commit);
    }

    fn write_release_archive(
        archive_path: &Path,
        bad_checksum: bool,
        extra_path: Option<&str>,
        duplicate_manifest: bool,
    ) {
        let stage = LocalStage::create().unwrap();
        let contents = stage.0.join("contents");
        std::fs::create_dir_all(contents.join("payload/adapters")).unwrap();
        let mut files = Vec::new();
        for path in PAYLOAD_FILES {
            let destination = contents.join(path);
            std::fs::write(&destination, format!("{path}-bytes")).unwrap();
            let sha256 = if bad_checksum && path == "payload/assembly-remote-server" {
                "0".repeat(64)
            } else {
                file_sha256(&destination).unwrap()
            };
            files.push(serde_json::json!({
                "path": path,
                "sha256": sha256,
                "mode": if path.ends_with("manifest.json") { "0644" } else { "0755" }
            }));
        }
        let manifest = serde_json::json!({
            "schemaVersion": 1,
            "packageType": "assembly-remote-backend",
            "version": "1.2.3",
            "commit": "0123456789abcdef",
            "target": TARGET,
            "files": files
        });
        std::fs::write(
            contents.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        std::fs::write(contents.join("SHA256SUMS"), "fixture\n").unwrap();

        let file = std::fs::File::create(archive_path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        archive.append_dir_all(".", &contents).unwrap();
        if let Some(path) = extra_path {
            let bytes = b"unexpected";
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            archive.append_data(&mut header, path, &bytes[..]).unwrap();
        }
        if duplicate_manifest {
            let bytes = std::fs::read(contents.join("manifest.json")).unwrap();
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            archive
                .append_data(&mut header, "manifest.json", &bytes[..])
                .unwrap();
        }
        archive.into_inner().unwrap().finish().unwrap();
    }

    #[test]
    fn ssh_targets_accept_hosts_but_reject_options_and_shell_syntax() {
        assert!(super::super::remote::validate_ssh_target("agent-workbox").is_ok());
        assert!(super::super::remote::validate_ssh_target("person@server.example").is_ok());
        for target in [
            "",
            "-oProxyCommand=false",
            "host name",
            "host;false",
            "$(false)",
        ] {
            assert!(super::super::remote::validate_ssh_target(target).is_err());
        }
    }

    #[test]
    fn release_assets_require_one_matching_archive_and_signature() {
        let release = GithubRelease {
            tag_name: "assembly-backend-v1.2.3".into(),
            draft: false,
            prerelease: false,
            assets: vec![
                GithubAsset {
                    id: 1,
                    name: format!("assembly-remote-backend-1.2.3-{TARGET}.tar.gz"),
                    size: 1,
                },
                GithubAsset {
                    id: 2,
                    name: format!("assembly-remote-backend-1.2.3-{TARGET}.tar.gz.sig"),
                    size: 1,
                },
            ],
        };
        let (archive, signature) = release_assets(release).unwrap();
        assert_eq!(archive.id, 1);
        assert_eq!(signature.id, 2);
    }

    #[test]
    fn release_version_requires_the_backend_tag_prefix_and_semver() {
        assert_eq!(release_version("assembly-backend-v1.2.3").unwrap(), "1.2.3");
        assert!(release_version("assembly-v1.2.3").is_err());
        assert!(release_version("assembly-backend-vnext").is_err());
    }

    #[test]
    fn shell_quote_keeps_remote_paths_as_one_argument() {
        assert_eq!(shell_quote("/home/person/app"), "'/home/person/app'");
        assert_eq!(shell_quote("/home/o'neil/app"), "'/home/o'\\''neil/app'");
    }

    #[test]
    fn manifest_requires_the_exact_package_contract() {
        let files = PAYLOAD_FILES
            .iter()
            .map(|path| ManifestFile {
                path: (*path).into(),
                sha256: "0".repeat(64),
            })
            .collect();
        let manifest = BackendManifest {
            schema_version: 1,
            package_type: "assembly-remote-backend".into(),
            version: "1.2.3".into(),
            commit: "0123456789abcdef".into(),
            target: TARGET.into(),
            files,
        };
        assert!(validate_manifest(&manifest).is_ok());
    }

    #[test]
    fn package_inspection_rejects_non_file_entries() {
        let stage = LocalStage::create().unwrap();
        let archive_path = stage.0.join("linked.tar.gz");
        let file = std::fs::File::create(&archive_path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o777);
        header.set_cksum();
        header.set_link_name("manifest.json").unwrap();
        archive
            .append_data(&mut header, "install.sh", std::io::empty())
            .unwrap();
        archive.into_inner().unwrap().finish().unwrap();

        assert!(inspect_package_contents(&archive_path)
            .unwrap_err()
            .contains("unexpected entry: install.sh"));
    }

    #[test]
    fn package_inspection_accepts_the_release_archive_layout() {
        let stage = LocalStage::create().unwrap();
        let archive_path = stage.0.join("package.tar.gz");
        write_release_archive(&archive_path, false, None, false);

        let inspected = inspect_package_contents(&archive_path).unwrap();
        assert_eq!(inspected.version, "1.2.3");
        assert_eq!(inspected.commit, "0123456789abcdef");
    }

    #[test]
    fn package_inspection_rejects_tampered_payload() {
        let stage = LocalStage::create().unwrap();
        let archive_path = stage.0.join("tampered.tar.gz");
        write_release_archive(&archive_path, true, None, false);

        assert!(inspect_package_contents(&archive_path)
            .unwrap_err()
            .contains("checksum failed: payload/assembly-remote-server"));
    }

    #[test]
    fn package_inspection_rejects_unexpected_and_duplicate_files() {
        let stage = LocalStage::create().unwrap();
        let unexpected = stage.0.join("unexpected.tar.gz");
        write_release_archive(&unexpected, false, Some("extra.txt"), false);
        assert!(inspect_package_contents(&unexpected)
            .unwrap_err()
            .contains("unexpected entry: extra.txt"));

        let duplicate = stage.0.join("duplicate.tar.gz");
        write_release_archive(&duplicate, false, None, true);
        assert!(inspect_package_contents(&duplicate)
            .unwrap_err()
            .contains("unexpected entry: manifest.json"));
    }

    #[test]
    fn signature_verification_rejects_invalid_artifacts() {
        let stage = LocalStage::create().unwrap();
        let archive_path = stage.0.join("package.tar.gz");
        let signature_path = stage.0.join("package.tar.gz.sig");
        std::fs::write(&archive_path, b"package").unwrap();
        std::fs::write(&signature_path, b"not-base64").unwrap();

        assert!(verify_signature(&archive_path, &signature_path)
            .unwrap_err()
            .contains("signature encoding is invalid"));
    }
}
