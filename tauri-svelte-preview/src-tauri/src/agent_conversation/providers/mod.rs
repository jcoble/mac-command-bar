pub mod acp;
pub mod acp_client;
pub mod process;

pub use acp::{AcpRuntimeAdapter, StructuredRuntimeHandle};
pub use acp_client::AcpClient;

use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use sha2::{Digest, Sha256};
use serde::Serialize;
use serde_json::Value;

use super::capabilities::{
    validate_manifest, AGY_ACP_VERSION, CLAUDE_AGENT_ACP_VERSION, CODEX_ACP_VERSION,
};
use super::protocol::{
    AgentApprovalResponse, AgentCapabilities, AgentCommandDescriptor, AgentConfigOption,
    AgentConversationConfigState, AgentConversationProvider, AgentProviderManifest, ProviderSource,
    ProviderTransport,
};

#[derive(Clone, Debug)]
pub struct InitializeAgentInput {
    pub provider: AgentConversationProvider,
}

#[derive(Clone, Debug)]
pub struct NewAgentSession {
    pub cwd: PathBuf,
}

#[derive(Clone, Debug)]
pub struct LoadAgentSession {
    pub cwd: PathBuf,
    pub native_session_id: String,
}

pub type ResumeAgentSession = LoadAgentSession;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentPromptImage {
    pub data: String,
    pub mime_type: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentPrompt {
    pub text: String,
    pub images: Vec<AgentPromptImage>,
    /// Ids of the saved screenshots the composer sent with this prompt.
    pub attachment_ids: Vec<String>,
}

pub type AgentConfigValue = Value;
pub type PermissionResponse = AgentApprovalResponse;

#[derive(Clone, Debug, PartialEq)]
pub struct StartedAgentSession {
    pub native_session_id: String,
    pub config: AgentConversationConfigState,
    pub commands: Vec<AgentCommandDescriptor>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationConfigUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedText {
    pub turn_id: Option<String>,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentRuntimeError {
    pub code: &'static str,
    pub message: String,
}

impl AgentRuntimeError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AgentRuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AgentRuntimeError {}

#[allow(async_fn_in_trait)]
pub trait AgentRuntimeAdapter: Send + Sync {
    async fn initialize(
        &mut self,
        input: InitializeAgentInput,
    ) -> Result<AgentCapabilities, AgentRuntimeError>;
    async fn new_session(
        &mut self,
        input: NewAgentSession,
    ) -> Result<StartedAgentSession, AgentRuntimeError>;
    async fn load_session(
        &mut self,
        input: LoadAgentSession,
    ) -> Result<StartedAgentSession, AgentRuntimeError>;
    async fn resume_session(
        &mut self,
        input: ResumeAgentSession,
    ) -> Result<StartedAgentSession, AgentRuntimeError>;
    /// Stop the transport without sending the destructive session/close
    /// request. The native session id remains valid for a later resume.
    async fn detach_session(&mut self) -> Result<(), AgentRuntimeError>;
    async fn close_session(&mut self) -> Result<(), AgentRuntimeError>;
}

#[derive(Clone, Default)]
pub struct ProviderRegistry {
    manifests: Arc<Vec<(AgentConversationProvider, AgentProviderManifest)>>,
}

impl ProviderRegistry {
    pub fn bundled_from_environment() -> Result<Self, String> {
        let configured_pair = |path_name: &str, hash_name: &str| match (
            std::env::var_os(path_name),
            std::env::var(hash_name).ok(),
        ) {
            (None, None) => Ok(None),
            (Some(path), Some(hash)) => Ok(Some((path.into(), hash))),
            _ => Err(
                "Packaged ACP adapter paths and SHA-256 values must be configured together"
                    .to_string(),
            ),
        };
        let mut codex = configured_pair("MCB_CODEX_ACP_PATH", "MCB_CODEX_ACP_SHA256")?;
        let mut claude =
            configured_pair("MCB_CLAUDE_AGENT_ACP_PATH", "MCB_CLAUDE_AGENT_ACP_SHA256")?;
        let mut antigravity = configured_pair("MCB_AGY_ACP_PATH", "MCB_AGY_ACP_SHA256")?;
        if codex.is_none() || claude.is_none() || antigravity.is_none() {
            if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
                if codex.is_none() {
                    codex = discover_home_wrapper(
                        &home,
                        &["codex-acp-bridge.sh", "codex-acp-dev.sh"],
                    )?;
                }
                if claude.is_none() {
                    claude = discover_home_wrapper(&home, &["claude-acp-wrapper.sh"])?;
                }
                if antigravity.is_none() {
                    antigravity = discover_home_wrapper(&home, &["agy-acp-wrapper.sh"])?;
                }
            }
        }
        if codex.is_some() != claude.is_some() {
            return Err(
                "Packaged ACP adapter paths and SHA-256 values must be configured together"
                    .to_string(),
            );
        }
        Self::initial_manifests(codex, claude, antigravity)
    }

    pub fn new(
        manifests: impl IntoIterator<Item = (AgentConversationProvider, AgentProviderManifest)>,
    ) -> Result<Self, String> {
        let mut values = Vec::new();
        for (provider, manifest) in manifests {
            validate_manifest(&manifest)?;
            if values.iter().any(|(current, _)| *current == provider) {
                return Err("Provider manifests must have one entry per provider".to_string());
            }
            values.push((provider, manifest));
        }
        Ok(Self {
            manifests: Arc::new(values),
        })
    }

    pub fn initial_manifests(
        codex: Option<(PathBuf, String)>,
        claude: Option<(PathBuf, String)>,
        antigravity: Option<(PathBuf, String)>,
    ) -> Result<Self, String> {
        let mut manifests = Vec::new();
        if let Some((codex_executable, codex_hash)) = codex {
            manifests.push((
                AgentConversationProvider::Codex,
                AgentProviderManifest {
                    id: "codex-acp".into(),
                    display_name: "Codex".into(),
                    transport: ProviderTransport::AcpStdio,
                    executable: codex_executable,
                    args: Vec::new(),
                    version: CODEX_ACP_VERSION.into(),
                    content_hash: codex_hash,
                    trusted_source: ProviderSource::Bundled,
                },
            ));
        }
        if let Some((claude_executable, claude_hash)) = claude {
            manifests.push((
                AgentConversationProvider::Claude,
                AgentProviderManifest {
                    id: "claude-agent-acp".into(),
                    display_name: "Claude".into(),
                    transport: ProviderTransport::AcpStdio,
                    executable: claude_executable,
                    args: Vec::new(),
                    version: CLAUDE_AGENT_ACP_VERSION.into(),
                    content_hash: claude_hash,
                    trusted_source: ProviderSource::Bundled,
                },
            ));
        }
        if let Some((agy_executable, agy_hash)) = antigravity {
            manifests.push((
                AgentConversationProvider::Antigravity,
                AgentProviderManifest {
                    id: "agy-acp".into(),
                    display_name: "Antigravity".into(),
                    transport: ProviderTransport::AcpStdio,
                    executable: agy_executable,
                    args: Vec::new(),
                    version: AGY_ACP_VERSION.into(),
                    content_hash: agy_hash,
                    trusted_source: ProviderSource::Bundled,
                },
            ));
        }
        Self::new(manifests)
    }

    pub fn manifest(
        &self,
        provider: AgentConversationProvider,
    ) -> Result<AgentProviderManifest, String> {
        self.manifests
            .iter()
            .find(|(current, _)| *current == provider)
            .map(|(_, manifest)| manifest.clone())
            .ok_or_else(|| "No trusted adapter is registered for this provider".to_string())
    }
}

#[cfg(test)]
fn discover_home_wrappers(
    home: &Path,
) -> Result<
    (
        Option<(PathBuf, String)>,
        Option<(PathBuf, String)>,
        Option<(PathBuf, String)>,
    ),
    String,
> {
    let codex = discover_home_wrapper(home, &["codex-acp-bridge.sh", "codex-acp-dev.sh"])?;
    let claude = discover_home_wrapper(home, &["claude-acp-wrapper.sh"])?;
    let antigravity = discover_home_wrapper(home, &["agy-acp-wrapper.sh"])?;
    Ok((codex, claude, antigravity))
}

fn discover_home_wrapper(
    home: &Path,
    names: &[&str],
) -> Result<Option<(PathBuf, String)>, String> {
    let wrapper_dir = home.join(".mac-command-bar");
    executable_wrapper(&wrapper_dir, names)
        .map(|path| file_sha256(&path).map(|hash| (path, hash)))
        .transpose()
}

fn executable_wrapper(directory: &Path, names: &[&str]) -> Option<PathBuf> {
    names.iter().map(|name| directory.join(name)).find(|path| {
        path.is_file()
            && path
                .metadata()
                .map(|metadata| {
                    #[cfg(unix)]
                    {
                        metadata.permissions().mode() & 0o111 != 0
                    }
                    #[cfg(not(unix))]
                    {
                        true
                    }
                })
                .unwrap_or(false)
    })
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let contents = std::fs::read(path)
        .map_err(|error| format!("Could not read ACP adapter wrapper {}: {error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(contents)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn write_executable(path: &Path, contents: &[u8], mode: u32) {
        std::fs::write(path, contents).unwrap();
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_mode(mode);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn initial_provider_manifests_pin_both_official_adapters() {
        let hash = "0000000000000000000000000000000000000000000000000000000000000000".to_string();
        let registry = ProviderRegistry::initial_manifests(
            Some((PathBuf::from("/bin/sh"), hash.clone())),
            Some((PathBuf::from("/bin/sh"), hash)),
            None,
        )
        .unwrap();
        let codex = registry.manifest(AgentConversationProvider::Codex).unwrap();
        let claude = registry
            .manifest(AgentConversationProvider::Claude)
            .unwrap();
        assert_eq!(
            (codex.id.as_str(), codex.version.as_str()),
            ("codex-acp", CODEX_ACP_VERSION)
        );
        assert_eq!(
            (claude.id.as_str(), claude.version.as_str()),
            ("claude-agent-acp", CLAUDE_AGENT_ACP_VERSION)
        );
        assert!(registry
            .manifests
            .iter()
            .all(|(_, manifest)| !manifest.args.iter().any(|arg| arg.contains("@latest"))));
    }

    #[cfg(unix)]
    #[test]
    fn home_wrapper_resolver_prefers_codex_bridge_and_hashes_executable_files() {
        let home = std::env::temp_dir().join(format!("mcb-provider-wrappers-{}", uuid::Uuid::new_v4()));
        let wrapper_dir = home.join(".mac-command-bar");
        std::fs::create_dir_all(&wrapper_dir).unwrap();

        let bridge = wrapper_dir.join("codex-acp-bridge.sh");
        let legacy = wrapper_dir.join("codex-acp-dev.sh");
        let claude = wrapper_dir.join("claude-acp-wrapper.sh");
        let antigravity = wrapper_dir.join("agy-acp-wrapper.sh");
        write_executable(&bridge, b"bridge", 0o755);
        write_executable(&legacy, b"legacy", 0o755);
        write_executable(&claude, b"claude", 0o755);
        write_executable(&antigravity, b"agy", 0o644);

        let (codex, resolved_claude, resolved_antigravity) = discover_home_wrappers(&home).unwrap();
        let (codex_path, codex_hash) = codex.unwrap();
        assert_eq!(codex_path, bridge);
        assert_eq!(codex_hash, "17f29b073143d8cd97b5bbe492bdeffec1c5fee55cc1fe2112c8b9335f8b6121");
        assert_eq!(resolved_claude.unwrap().0, claude);
        assert!(resolved_antigravity.is_none());

        write_executable(&bridge, b"bridge", 0o644);
        let (codex_fallback, _, _) = discover_home_wrappers(&home).unwrap();
        assert_eq!(codex_fallback.unwrap().0, legacy);

        std::fs::remove_dir_all(home).unwrap();
    }
}
