pub mod acp;
pub mod acp_client;
pub mod process;

pub use acp::{AcpRuntimeAdapter, StructuredRuntimeHandle};
pub use acp_client::AcpClient;

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use super::capabilities::{validate_manifest, CLAUDE_AGENT_ACP_VERSION, CODEX_ACP_VERSION};
use super::protocol::{
    AgentApprovalResponse, AgentCapabilities, AgentConfigOption, AgentConversationConfigState,
    AgentConversationProvider, AgentProviderManifest, ProviderSource, ProviderTransport,
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
}

pub type AgentConfigValue = Value;
pub type PermissionResponse = AgentApprovalResponse;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartedAgentSession {
    pub native_session_id: String,
    pub config: AgentConversationConfigState,
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
    async fn prompt_once(&mut self, input: AgentPrompt)
        -> Result<GeneratedText, AgentRuntimeError>;
    async fn set_config(
        &mut self,
        option_id: &str,
        value: AgentConfigValue,
    ) -> Result<Vec<AgentConfigOption>, AgentRuntimeError>;
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
        let codex_path = std::env::var_os("MCB_CODEX_ACP_PATH");
        let codex_hash = std::env::var("MCB_CODEX_ACP_SHA256").ok();
        let claude_path = std::env::var_os("MCB_CLAUDE_AGENT_ACP_PATH");
        let claude_hash = std::env::var("MCB_CLAUDE_AGENT_ACP_SHA256").ok();
        match (codex_path, codex_hash, claude_path, claude_hash) {
            (None, None, None, None) => Ok(Self::default()),
            (Some(codex_path), Some(codex_hash), Some(claude_path), Some(claude_hash)) => {
                Self::initial_manifests(
                    codex_path.into(),
                    codex_hash,
                    claude_path.into(),
                    claude_hash,
                )
            }
            _ => Err(
                "Packaged ACP adapter paths and SHA-256 values must be configured together"
                    .to_string(),
            ),
        }
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
        codex_executable: PathBuf,
        codex_hash: String,
        claude_executable: PathBuf,
        claude_hash: String,
    ) -> Result<Self, String> {
        Self::new([
            (
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
            ),
            (
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
            ),
        ])
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
mod tests {
    use super::*;

    #[test]
    fn initial_provider_manifests_pin_both_official_adapters() {
        let hash = "0000000000000000000000000000000000000000000000000000000000000000".to_string();
        let registry = ProviderRegistry::initial_manifests(
            PathBuf::from("/bin/sh"),
            hash.clone(),
            PathBuf::from("/bin/sh"),
            hash,
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
}
