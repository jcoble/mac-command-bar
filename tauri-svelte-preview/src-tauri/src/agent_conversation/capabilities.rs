use std::collections::HashSet;
use std::path::Path;

use serde_json::Value;

use super::protocol::{
    AgentCapabilities, AgentConfigOption, AgentProviderManifest, ProviderTransport,
};

pub const CODEX_ACP_VERSION: &str = "1.10.0";
pub const CLAUDE_AGENT_ACP_VERSION: &str = "0.75.0";
pub const AGY_ACP_VERSION: &str = "0.1.0";

pub fn validate_manifest(manifest: &AgentProviderManifest) -> Result<(), String> {
    required(&manifest.id, "Provider id")?;
    required(&manifest.display_name, "Provider display name")?;
    required(&manifest.version, "Provider version")?;
    required(&manifest.content_hash, "Provider content hash")?;
    if manifest.content_hash.len() != 64
        || !manifest
            .content_hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("Provider content hash must be a SHA-256 hex digest".to_string());
    }
    if manifest.version.eq_ignore_ascii_case("latest")
        || manifest.args.iter().any(|arg| arg.contains("@latest"))
        || manifest.executable.to_string_lossy().contains("npx")
    {
        return Err("Provider manifests must use a pinned packaged adapter".to_string());
    }
    if manifest.transport != ProviderTransport::BuiltIn {
        validate_executable(&manifest.executable)?;
    }
    let _ = manifest.trusted_source;
    Ok(())
}

pub fn validate_capabilities(capabilities: &AgentCapabilities) -> Result<(), String> {
    if capabilities.revision == 0 {
        return Err("Capability revision must be positive".to_string());
    }
    required(
        &capabilities.implementation.name,
        "Provider implementation name",
    )?;
    required(
        &capabilities.implementation.version,
        "Provider implementation version",
    )?;
    validate_config_options(&capabilities.config_options)
}

pub fn validate_config_options(options: &[AgentConfigOption]) -> Result<(), String> {
    let mut ids = HashSet::new();
    for option in options {
        let id = required(&option.id, "Config option id")?;
        required(&option.label, "Config option label")?;
        required(&option.category, "Config option category")?;
        if !ids.insert(id) {
            return Err("Config option ids must be unique".to_string());
        }
        if let Some(choices) = &option.choices {
            if choices.is_empty() {
                return Err(format!("Config option {} has no choices", option.id));
            }
            if !choices.iter().any(|choice| choice.value == option.value) {
                return Err(format!(
                    "Config option {} has an invalid selected value",
                    option.id
                ));
            }
        }
    }
    Ok(())
}

pub fn replace_config_options(
    capabilities: &mut AgentCapabilities,
    replacement: Vec<AgentConfigOption>,
) -> Result<(), String> {
    validate_config_options(&replacement)?;
    capabilities.config_options = replacement;
    capabilities.revision = capabilities.revision.saturating_add(1);
    Ok(())
}

pub fn validate_config_value(option: &AgentConfigOption, value: &Value) -> Result<(), String> {
    if let Some(choices) = &option.choices {
        if !choices.iter().any(|choice| &choice.value == value) {
            return Err(format!("Unsupported value for config option {}", option.id));
        }
    }
    Ok(())
}

fn validate_executable(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("Provider executable is required".to_string());
    }
    if !path.is_absolute() {
        return Err("Provider executable must use an absolute packaged path".to_string());
    }
    if !path.is_file() {
        return Err("Provider executable does not exist".to_string());
    }
    Ok(())
}

fn required(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label} is required"))
    } else {
        Ok(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_conversation::protocol::{
        AgentConversationProvider, AgentImplementation, AgentInteractionCapabilities,
        AgentPromptCapabilities, AgentSessionCapabilities, ProviderSource, ProviderTransport,
    };

    fn capabilities() -> AgentCapabilities {
        AgentCapabilities {
            revision: 1,
            provider: AgentConversationProvider::Codex,
            implementation: AgentImplementation {
                name: "fake".into(),
                version: "1".into(),
            },
            session: AgentSessionCapabilities {
                multi_session: false,
                list: true,
                load: true,
                resume: true,
                close: true,
                steering: true,
                fork: false,
            },
            prompt: AgentPromptCapabilities {
                text: true,
                image: true,
                embedded_context: false,
                resource_links: false,
            },
            interaction: AgentInteractionCapabilities {
                permissions: true,
                structured_user_input: true,
                tool_terminals: true,
                plans: false,
                tasks: false,
                subagents: false,
            },
            config_options: Vec::new(),
            commands: Vec::new(),
        }
    }

    #[test]
    fn config_replacement_is_full_state_and_increments_revision() {
        let mut value = capabilities();
        value.config_options.push(AgentConfigOption {
            id: "old".into(),
            label: "Old".into(),
            category: "mode".into(),
            description: None,
            value: Value::String("a".into()),
            choices: None,
            provider_metadata: None,
        });
        replace_config_options(
            &mut value,
            vec![AgentConfigOption {
                id: "new".into(),
                label: "New".into(),
                category: "model".into(),
                description: None,
                value: Value::String("b".into()),
                choices: None,
                provider_metadata: None,
            }],
        )
        .unwrap();
        assert_eq!(value.revision, 2);
        assert_eq!(value.config_options.len(), 1);
        assert_eq!(value.config_options[0].id, "new");
    }

    #[test]
    fn manifest_validation_rejects_latest_and_unrecorded_hashes() {
        let manifest = AgentProviderManifest {
            id: "codex-acp".into(),
            display_name: "Codex".into(),
            transport: ProviderTransport::AcpStdio,
            executable: "/bin/sh".into(),
            args: vec!["npx @agentclientprotocol/codex-acp@latest".into()],
            version: "latest".into(),
            content_hash: "0000000000000000000000000000000000000000000000000000000000000000".into(),
            trusted_source: ProviderSource::Bundled,
        };
        assert!(validate_manifest(&manifest).unwrap_err().contains("pinned"));
        let mut missing_hash = manifest;
        missing_hash.args.clear();
        missing_hash.version = CODEX_ACP_VERSION.into();
        missing_hash.content_hash = "unverified".into();
        assert!(validate_manifest(&missing_hash)
            .unwrap_err()
            .contains("SHA-256"));
    }
}
