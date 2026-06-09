use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CoreRequest {
    pub id: String,
    pub action: String,
    #[serde(rename = "dryRun")]
    pub dry_run: bool,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CoreResponse {
    pub id: String,
    pub ok: bool,
    pub summary: String,
    #[serde(default)]
    pub data: Value,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposed_command: Option<String>,
}

impl CoreResponse {
    pub fn ok(id: impl Into<String>, summary: impl Into<String>, data: Value) -> Self {
        Self {
            id: id.into(),
            ok: true,
            summary: summary.into(),
            data,
            warnings: Vec::new(),
            proposed_command: None,
        }
    }

    pub fn error(id: impl Into<String>, summary: impl Into<String>, warnings: Vec<String>) -> Self {
        Self {
            id: id.into(),
            ok: false,
            summary: summary.into(),
            data: Value::Object(Default::default()),
            warnings,
            proposed_command: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmableAction {
    pub action_id: String,
    pub kind: String,
    pub target_label: String,
    pub risk: String,
    pub command_preview: String,
    pub requires_confirmation: bool,
}
