//! Providers are data: each one Assembly can run is a descriptor in `descriptors.json`, and code
//! asks the provider's descriptor instead of matching on its name (workflow redesign plan 0.2).
//! Adding an ACP agent is a new descriptor plus its release-bundle entry.
//!
//! Evidence for today's values:
//! - `initialize_meta`: Codex reads `recommendedValue` for its default effort; Claude ACP 0.88/0.93
//!   read `asyncTasks`, `nativeSubagentSessions` and `backgroundSubagents` (l9/y3, runtime bytes
//!   82788482/92449895). Claude answers forms through `elicitation`.
//! - `effort_in_model_id`: Codex ACP 1.13.1/2.2.2 emit `model[effort]` (codex-1.13.1-isolated.jsonl:6).
//! - `canonical_cwd`: Antigravity 1.2.1 server.py:3264-3268 binds restored tool permissions to
//!   realpath(cwd).
//! - `unattended`: the auto-review modes, Claude 0.88/0.93 `auto` and Codex 1.13.1/2.2.2 `agent`;
//!   Antigravity has none (its modes are default, auto_edit and yolo).
//! - `pre_approval` lets unattended workers use Assembly's own tools without asking: Claude's
//!   `allowedTools` meta, Codex's `default_tools_approval_mode` config, and for Antigravity, which
//!   has neither, Assembly's permission handler answers them (TSK-1522; its MCP requests carry
//!   `_meta.mcp.server`, Antigravity 1.2.1 tools.py:386-391, server.py:4400).
//! - `login`: Claude reads an empty refresh token as expired, so it is deleted; Codex can't read
//!   `auth.json` without the field (token_data.rs:22), so it is blanked. Antigravity's saved login
//!   holds only a refresh token, so it has no proof home and can't prove (plan 3.11).

use std::sync::LazyLock;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::agent_conversation::protocol::AgentConversationProvider;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderDescriptor {
    pub id: String,
    /// The adapter manifest id it launches (`AgentProviderManifest::id`).
    pub adapter_id: String,
    pub display_name: String,
    #[serde(default)]
    pub initialize_meta: Option<Map<String, Value>>,
    #[serde(default)]
    pub elicitation: bool,
    #[serde(default)]
    pub effort_in_model_id: bool,
    #[serde(default)]
    pub canonical_cwd: bool,
    #[serde(default)]
    pub unattended: LaunchSetting,
    #[serde(default)]
    pub pre_approval: LaunchSetting,
    /// What moves its config folder into a proof run's provider home (`{home}`); missing means
    /// the provider is unavailable in proof mode.
    #[serde(default)]
    pub proof_home: Option<LaunchSetting>,
    /// Missing means it can't run read-only, so it can't prove.
    #[serde(default)]
    pub read_only: Option<LaunchSetting>,
    /// Read by proof mode (plan 3.11), a later piece.
    #[allow(dead_code)]
    #[serde(default)]
    pub login: Option<LoginEntry>,
}

/// One launch setting: a session mode, `session/new` `_meta`, adapter arguments and environment.
/// A JSON object in `env` is merged into the variable's inherited JSON value.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct LaunchSetting {
    pub mode: Option<String>,
    pub meta: Map<String, Value>,
    pub args: Vec<String>,
    pub env: Map<String, Value>,
    /// Assembly's permission handler approves the agent's requests for `assembly` MCP tools.
    pub permission_handler: bool,
}

/// Which launch settings a session starts with; they merge in this order, `read_only` last.
#[derive(Clone, Copy, Debug, Default)]
pub struct LaunchFor {
    pub unattended: bool,
    pub pre_approval: bool,
    pub proof: bool,
    pub read_only: bool,
}

/// How a copied login loses its refresh token, so a proof session can't renew the real login.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LoginEntry {
    /// The login file, relative to the provider's config folder.
    pub file: String,
    /// JSON pointers into that file.
    pub refresh_field: String,
    pub action: LoginAction,
    #[serde(default)]
    pub expiry_field: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LoginAction {
    Delete,
    Blank,
}

static DESCRIPTORS: LazyLock<Vec<ProviderDescriptor>> = LazyLock::new(|| {
    parse_manifest(include_str!("descriptors.json")).expect("descriptors.json is a valid manifest")
});

pub fn parse_manifest(text: &str) -> Result<Vec<ProviderDescriptor>, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Manifest {
        providers: Vec<ProviderDescriptor>,
    }
    let providers = serde_json::from_str::<Manifest>(text).map_err(|error| format!("Invalid provider manifest: {error}"))?.providers;
    for (index, descriptor) in providers.iter().enumerate() {
        if providers[..index].iter().any(|earlier| earlier.id == descriptor.id || earlier.adapter_id == descriptor.adapter_id) {
            return Err(format!("Provider manifest lists {} twice", descriptor.id));
        }
    }
    Ok(providers)
}

pub fn all() -> &'static [ProviderDescriptor] {
    &DESCRIPTORS
}

/// The descriptor that launches this adapter manifest.
pub fn for_adapter(adapter_id: &str) -> Option<&'static ProviderDescriptor> {
    all().iter().find(|descriptor| descriptor.adapter_id == adapter_id)
}

/// Whether Assembly's permission handler approves this ACP permission request itself: the
/// provider's pre-approval says so and the request is for one of Assembly's own MCP tools.
pub fn handler_approves(provider: AgentConversationProvider, params: &Value) -> bool {
    provider.descriptor().pre_approval.permission_handler
        && params.pointer("/toolCall/_meta/mcp/server").and_then(Value::as_str) == Some("assembly")
}

/// The value to set for `key`, given what it would inherit.
pub fn env_value(key: &str, inherited: Option<&str>, value: &Value) -> Result<String, String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Object(value) => {
            let mut merged = match inherited {
                Some(inherited) => serde_json::from_str(inherited).map_err(|error| format!("Invalid adapter {key}: {error}"))?,
                None => Map::new(),
            };
            deep_merge(&mut merged, value);
            Ok(Value::Object(merged).to_string())
        }
        value => Ok(value.to_string()),
    }
}

/// Objects merge key by key; anything else from `from` replaces what is there.
fn deep_merge(into: &mut Map<String, Value>, from: &Map<String, Value>) {
    for (key, value) in from {
        match (into.get_mut(key), value) {
            (Some(Value::Object(current)), Value::Object(value)) => deep_merge(current, value),
            _ => {
                into.insert(key.clone(), value.clone());
            }
        }
    }
}

impl ProviderDescriptor {
    pub fn provider(&'static self) -> AgentConversationProvider {
        AgentConversationProvider(self.id.as_str())
    }

    /// The merged launch settings, or `None` when a wanted one is missing.
    pub fn launch(&self, wanted: LaunchFor) -> Option<LaunchSetting> {
        let mut merged = LaunchSetting::default();
        for (on, setting) in [
            (wanted.unattended, Some(&self.unattended)),
            (wanted.pre_approval, Some(&self.pre_approval)),
            (wanted.proof, self.proof_home.as_ref()),
            (wanted.read_only, self.read_only.as_ref()),
        ] {
            if on {
                let setting = setting?;
                merged.mode = setting.mode.clone().or(merged.mode);
                deep_merge(&mut merged.meta, &setting.meta);
                merged.args.extend(setting.args.iter().cloned());
                deep_merge(&mut merged.env, &setting.env);
                merged.permission_handler |= setting.permission_handler;
            }
        }
        Some(merged)
    }
}

#[allow(dead_code)] // Used by proof mode (plan 3.11), a later piece.
impl LoginEntry {
    /// A copy of the login with its refresh token removed.
    pub fn stripped(&self, login: &Value) -> Value {
        let mut copy = login.clone();
        let (parent, field) = self.refresh_field.rsplit_once('/').unwrap_or(("", &self.refresh_field));
        if let Some(Value::Object(parent)) = copy.pointer_mut(parent) {
            match self.action {
                LoginAction::Delete => {
                    parent.remove(field);
                }
                LoginAction::Blank => {
                    if let Some(value) = parent.get_mut(field) {
                        *value = Value::String(String::new());
                    }
                }
            }
        }
        copy
    }
}

#[allow(non_upper_case_globals)]
impl AgentConversationProvider {
    pub const Codex: Self = Self("codex");
    pub const Claude: Self = Self("claude");
    pub const Antigravity: Self = Self("antigravity");

    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim().to_ascii_lowercase();
        all().iter().find(|descriptor| descriptor.id == value).map(ProviderDescriptor::provider)
    }

    pub fn id(self) -> &'static str {
        self.0
    }

    pub fn descriptor(self) -> &'static ProviderDescriptor {
        all().iter().find(|descriptor| descriptor.id == self.0).expect("provider ids come from the manifest")
    }
}

impl Serialize for AgentConversationProvider {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

impl<'de> Deserialize<'de> for AgentConversationProvider {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).ok_or_else(|| serde::de::Error::custom(format!("unknown provider {value}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn launch(id: &str, wanted: LaunchFor) -> Option<LaunchSetting> {
        AgentConversationProvider::parse(id).unwrap().descriptor().launch(wanted)
    }

    #[test]
    fn the_manifest_covers_todays_providers() {
        let ids: Vec<_> = all().iter().map(|descriptor| descriptor.id.as_str()).collect();
        assert_eq!(ids, ["codex", "claude", "antigravity"]);
        for provider in [AgentConversationProvider::Codex, AgentConversationProvider::Claude, AgentConversationProvider::Antigravity] {
            assert_eq!(AgentConversationProvider::parse(provider.id()), Some(provider));
            assert_eq!(serde_json::to_value(provider).unwrap(), json!(provider.id()));
            assert_eq!(serde_json::from_value::<AgentConversationProvider>(json!(provider.id())).unwrap(), provider);
        }
        assert!(serde_json::from_value::<AgentConversationProvider>(json!("grok")).is_err());
        assert_eq!(for_adapter("codex-acp").unwrap().id, "codex");
        assert!(parse_manifest(r#"{"providers":[{"id":"a","adapter_id":"x","display_name":"A"},{"id":"a","adapter_id":"y","display_name":"B"}]}"#).is_err());
    }

    #[test]
    fn launch_settings_merge_in_order_with_read_only_last() {
        let all_on = LaunchFor { unattended: true, pre_approval: true, proof: true, read_only: true };
        let claude = launch("claude", all_on).unwrap();
        assert_eq!(claude.mode.as_deref(), Some("auto"));
        assert_eq!(Value::Object(claude.meta), json!({"claudeCode": {"options": {"allowedTools": ["mcp__assembly"], "tools": []}}}));
        assert_eq!(claude.env, Map::from_iter([("CLAUDE_CONFIG_DIR".to_string(), json!("{home}"))]));
        let codex = launch("codex", all_on).unwrap();
        assert_eq!(codex.env["CODEX_CONFIG"], json!({"mcp_servers.assembly.default_tools_approval_mode": "approve", "sandbox_mode": "read-only"}));
        let pre_approval = LaunchFor { pre_approval: true, ..Default::default() };
        assert!(launch("antigravity", pre_approval).unwrap().permission_handler);
        assert_eq!(launch("antigravity", LaunchFor { unattended: true, ..Default::default() }).unwrap().mode, None);
        assert!(launch("antigravity", LaunchFor { proof: true, ..Default::default() }).is_none());
        assert!(launch("antigravity", LaunchFor { read_only: true, ..Default::default() }).is_none());
    }

    #[test]
    fn a_descriptor_added_only_in_the_manifest_launches() {
        let manifest = r#"{"providers":[{"id":"grok","adapter_id":"grok-acp","display_name":"Grok Build",
            "unattended":{"mode":"auto","args":["--unattended"],"env":{"GROK_CONFIG":{"approval":"auto"}}},
            "pre_approval":{"args":["--allow-mcp","assembly"]},
            "read_only":{"mode":"plan","args":["--read-only"],"env":{"GROK_CONFIG":{"approval":"never","sandbox":"read-only"}}}}]}"#;
        let grok = &parse_manifest(manifest).unwrap()[0];
        let merged = grok.launch(LaunchFor { unattended: true, pre_approval: true, read_only: true, ..Default::default() }).unwrap();
        assert_eq!(merged.mode.as_deref(), Some("plan"));
        assert_eq!(merged.args, ["--unattended", "--allow-mcp", "assembly", "--read-only"]);
        let config = env_value("GROK_CONFIG", Some(r#"{"theme":"dark"}"#), &merged.env["GROK_CONFIG"]).unwrap();
        assert_eq!(serde_json::from_str::<Value>(&config).unwrap(), json!({"approval": "never", "sandbox": "read-only", "theme": "dark"}));
        assert!(env_value("GROK_CONFIG", Some("not json"), &merged.env["GROK_CONFIG"]).is_err());
        assert!(grok.launch(LaunchFor { proof: true, ..Default::default() }).is_none());
    }

    #[test]
    fn logins_are_copied_without_their_refresh_token() {
        let claude = AgentConversationProvider::Claude.descriptor().login.as_ref().unwrap();
        let login = json!({"claudeAiOauth": {"accessToken": "a", "refreshToken": "r", "expiresAt": 1}});
        assert_eq!(claude.stripped(&login), json!({"claudeAiOauth": {"accessToken": "a", "expiresAt": 1}}));
        assert_eq!(claude.expiry_field.as_deref(), Some("/claudeAiOauth/expiresAt"));
        let codex = AgentConversationProvider::Codex.descriptor().login.as_ref().unwrap();
        let login = json!({"tokens": {"access_token": "a", "refresh_token": "r"}, "last_refresh": "t"});
        assert_eq!(codex.stripped(&login), json!({"tokens": {"access_token": "a", "refresh_token": ""}, "last_refresh": "t"}));
        let antigravity = AgentConversationProvider::Antigravity.descriptor();
        assert!(antigravity.login.is_none() && antigravity.proof_home.is_none());
    }

    #[test]
    fn the_permission_handler_approves_only_assembly_tools_for_providers_that_ask_it() {
        // Antigravity 1.2.1 sends the MCP server in the tool call's _meta (tools.py:386-391).
        let request = |server: &str| json!({"toolCall": {"_meta": {"mcp": {"tool": "report_result", "server": server}, "is_mcp_tool_call": true}}});
        assert!(handler_approves(AgentConversationProvider::Antigravity, &request("assembly")));
        assert!(!handler_approves(AgentConversationProvider::Antigravity, &request("notion")));
        assert!(!handler_approves(AgentConversationProvider::Antigravity, &json!({"toolCall": {"title": "assembly_report_result"}})));
        // Claude and Codex pre-approve in the adapter, so their requests still reach the owner.
        assert!(!handler_approves(AgentConversationProvider::Claude, &request("assembly")));
        assert!(!handler_approves(AgentConversationProvider::Codex, &request("assembly")));
    }
}
