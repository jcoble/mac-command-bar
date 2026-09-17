use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::file_sha256;
use crate::agent_conversation::capabilities::{
    AGY_ACP_VERSION, CLAUDE_AGENT_ACP_VERSION, CODEX_ACP_VERSION,
};

#[derive(Clone, Debug)]
pub(super) struct AdapterPackage {
    pub executable: PathBuf,
    pub content_hash: String,
    pub version: String,
}

pub(super) type AdapterPair = AdapterPackage;
pub(super) type AdapterPairs = (
    Option<AdapterPair>,
    Option<AdapterPair>,
    Option<AdapterPair>,
);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PackagedManifest {
    pub schema_version: u32,
    pub adapters: Vec<PackagedAdapter>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PackagedAdapter {
    pub provider: String,
    pub id: String,
    pub version: String,
    pub executable: String,
    pub files: Vec<PackagedFile>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PackagedFile {
    pub path: String,
    pub sha256: String,
}

pub(super) fn discover() -> Result<AdapterPairs, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("Could not locate the Assembly executable: {error}"))?;
    let Some(executable_dir) = executable.parent() else {
        return Ok((None, None, None));
    };
    let candidates = [
        executable_dir.join("../Resources/adapters"),
        executable_dir.join("../resources/adapters"),
        executable_dir.join("assembly-adapters"),
        executable_dir.join("adapters"),
    ];
    let Some(directory) = candidates
        .into_iter()
        .find(|path| path.join("manifest.json").is_file())
    else {
        return Ok((None, None, None));
    };
    discover_in(&directory, true)
}

pub(super) fn discover_in(
    directory: &Path,
    require_bundled_versions: bool,
) -> Result<AdapterPairs, String> {
    let manifest_path = directory.join("manifest.json");
    let manifest: PackagedManifest =
        serde_json::from_slice(&std::fs::read(&manifest_path).map_err(|error| {
            format!(
                "Could not read packaged adapter manifest {}: {error}",
                manifest_path.display()
            )
        })?)
        .map_err(|error| format!("Packaged adapter manifest is invalid: {error}"))?;
    if manifest.schema_version != 1 {
        return Err(format!(
            "Unsupported packaged adapter manifest version {}",
            manifest.schema_version
        ));
    }

    let mut codex = None;
    let mut claude = None;
    let mut antigravity = None;
    for adapter in manifest.adapters {
        let expected = match adapter.provider.as_str() {
            "codex" => ("codex-acp", CODEX_ACP_VERSION),
            "claude" => ("claude-agent-acp", CLAUDE_AGENT_ACP_VERSION),
            "antigravity" => ("agy-acp", AGY_ACP_VERSION),
            other => return Err(format!("Unknown packaged adapter provider {other}")),
        };
        if adapter.id != expected.0 || (require_bundled_versions && adapter.version != expected.1) {
            return Err(format!(
                "Packaged {} adapter is {}, expected {} {}",
                adapter.provider, adapter.version, expected.0, expected.1
            ));
        }
        if adapter.files.is_empty() {
            return Err(format!(
                "Packaged {} adapter has no verified files",
                adapter.provider
            ));
        }
        for file in &adapter.files {
            let actual_hash = file_sha256(&directory.join(&file.path))?;
            if !actual_hash.eq_ignore_ascii_case(&file.sha256) {
                return Err(format!(
                    "Packaged {} adapter failed SHA-256 verification",
                    adapter.provider
                ));
            }
        }
        let path = directory.join(&adapter.executable);
        let actual_hash = file_sha256(&path)?;
        let pair = Some(AdapterPackage {
            executable: path,
            content_hash: actual_hash,
            version: adapter.version,
        });
        match adapter.provider.as_str() {
            "codex" => codex = pair,
            "claude" => claude = pair,
            "antigravity" => antigravity = pair,
            _ => unreachable!(),
        }
    }
    Ok((codex, claude, antigravity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn packaged_manifest_verifies_every_adapter_hash() {
        let directory =
            std::env::temp_dir().join(format!("mcb-packaged-adapters-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let entries = [
            ("codex", "codex-acp", CODEX_ACP_VERSION),
            ("claude", "claude-agent-acp", CLAUDE_AGENT_ACP_VERSION),
            ("antigravity", "agy-acp", AGY_ACP_VERSION),
        ];
        let adapters = entries.map(|(provider, id, version)| {
            let path = directory.join(id);
            std::fs::write(&path, provider.as_bytes()).unwrap();
            let mut permissions = std::fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&path, permissions).unwrap();
            let hash = file_sha256(&path).unwrap();
            serde_json::json!({
                "provider": provider,
                "id": id,
                "version": version,
                "executable": id,
                "files": [{ "path": id, "sha256": hash }]
            })
        });
        std::fs::write(
            directory.join("manifest.json"),
            serde_json::to_vec(&serde_json::json!({ "schemaVersion": 1, "adapters": adapters }))
                .unwrap(),
        )
        .unwrap();

        let (codex, claude, antigravity) = discover_in(&directory, true).unwrap();
        assert!(codex.is_some() && claude.is_some() && antigravity.is_some());

        std::fs::write(directory.join("codex-acp"), b"tampered").unwrap();
        assert!(discover_in(&directory, true)
            .unwrap_err()
            .contains("SHA-256"));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
