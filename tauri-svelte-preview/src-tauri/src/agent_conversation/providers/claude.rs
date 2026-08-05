//! Claude Agent ACP frame adapter.
//!
//! Claude's pinned Agent ACP package speaks the same ACP JSON-RPC session/update contract as
//! Codex.  The adapter uses the shared transport-free projector, with a Claude-specific metadata
//! namespace.  It does not install a package per launch, invoke `claude -p`, attach to a native
//! TUI, or create a PTY.

use serde_json::Value;

use super::super::protocol::{AgentCapabilities, AgentConversationProvider, AgentItem};
use super::adapter_support::AcpProjectionState;
pub use super::adapter_support::{AdapterContext, ProjectionBatch};

pub const CLAUDE_METADATA_NAMESPACE: &str = "claude.acp";

pub struct ClaudeFrameAdapter {
    state: AcpProjectionState,
}

impl ClaudeFrameAdapter {
    pub fn new(context: AdapterContext) -> Self {
        Self {
            state: AcpProjectionState::new(
                context,
                AgentConversationProvider::Claude,
                CLAUDE_METADATA_NAMESPACE,
            ),
        }
    }

    pub fn from_ids(
        owned_id: impl Into<String>,
        provider_instance_id: impl Into<String>,
        generation: u64,
        native_session_id: Option<impl Into<String>>,
    ) -> Self {
        Self::new(AdapterContext::new(
            owned_id,
            provider_instance_id,
            generation,
            native_session_id,
        ))
    }

    pub fn map_frame(&mut self, frame: &Value) -> ProjectionBatch {
        self.state.map_frame(frame)
    }

    pub fn map_jsonl(&mut self, input: &str) -> Vec<ProjectionBatch> {
        self.state.map_jsonl(input)
    }

    pub fn capabilities(&self) -> AgentCapabilities {
        self.state.capabilities()
    }

    pub fn ordered_items(&self) -> Vec<AgentItem> {
        self.state.ordered_items()
    }

    pub fn uses_pty(&self) -> bool {
        self.state.uses_pty()
    }
}

pub type ClaudeAdapter = ClaudeFrameAdapter;
pub type ClaudeProviderAdapter = ClaudeFrameAdapter;

pub fn map_claude_frame(context: AdapterContext, frame: &Value) -> ProjectionBatch {
    let mut adapter = ClaudeFrameAdapter::new(context);
    adapter.map_frame(frame)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::Value;

    use super::*;
    use crate::agent_conversation::protocol::{AgentEventType, AgentItemType};

    fn fixture(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/agent_conversation/claude")
            .join(name);
        std::fs::read_to_string(path).expect("Claude fixture must be present")
    }

    fn adapter() -> ClaudeFrameAdapter {
        ClaudeFrameAdapter::new(
            AdapterContext::new("owned-claude", "claude-provider-1", 8, Some("session-old"))
                .with_timestamp(20_000),
        )
    }

    #[test]
    fn claude_fixture_maps_models_images_permissions_tools_todos_review_terminals_mcp_and_children()
    {
        let mut adapter = adapter();
        let batches = adapter.map_jsonl(&fixture("all-categories.jsonl"));
        let events: Vec<_> = batches
            .iter()
            .flat_map(|batch| batch.events.iter())
            .collect();
        let event_types: Vec<_> = events.iter().map(|event| event.event_type).collect();
        for expected in [
            AgentEventType::SessionStarted,
            AgentEventType::SessionConfigUpdated,
            AgentEventType::TurnStarted,
            AgentEventType::ItemStarted,
            AgentEventType::ContentDelta,
            AgentEventType::ApprovalRequested,
            AgentEventType::UserInputRequested,
            AgentEventType::PlanUpdated,
            AgentEventType::TasksUpdated,
            AgentEventType::ChildrenUpdated,
            AgentEventType::UsageUpdated,
            AgentEventType::RateLimitsUpdated,
            AgentEventType::TurnCompleted,
            AgentEventType::SessionClosed,
        ] {
            assert!(event_types.contains(&expected), "missing {expected:?}");
        }
        assert!(events.iter().any(|event| {
            event.event_type == AgentEventType::ApprovalRequested
                && event.request_id.as_deref() == Some("4")
        }));
        assert!(events.iter().any(|event| {
            event.event_type == AgentEventType::UserInputRequested
                && event.request_id.as_deref() == Some("5")
        }));
        let ordered_items = adapter.ordered_items();
        let item_ids: Vec<_> = ordered_items.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(
            item_ids,
            vec![
                "user-claude-1",
                "assistant-claude-1",
                "terminal-claude-1",
                "reasoning-claude-1",
                "review-claude-1",
                "mcp-claude-1",
                "child-claude-1",
                "plan-claude-1",
                "todo-claude-1",
            ]
        );
        let prompt = adapter.ordered_items()[0]
            .provider_metadata
            .as_ref()
            .unwrap()["claude.acp.item"]["prompt"]
            .clone();
        assert_eq!(prompt[1]["type"], "image");
        assert_eq!(prompt[1]["mimeType"], "image/png");
        let terminal = ordered_items
            .iter()
            .find(|item| item.id == "terminal-claude-1")
            .unwrap();
        assert!(terminal
            .content
            .iter()
            .any(|content| content.text.contains("PASS fixture test")));
        assert!(adapter
            .ordered_items()
            .iter()
            .any(|item| item.item_type == AgentItemType::Subagent));
        assert!(adapter
            .ordered_items()
            .iter()
            .any(|item| item.item_type == AgentItemType::Review));
        assert!(!adapter.uses_pty());
    }

    #[test]
    fn claude_unknown_categories_and_future_options_are_retained() {
        let mut adapter = adapter();
        let batches = adapter.map_jsonl(&fixture("unknown-and-lifecycle.jsonl"));
        let unknown = batches
            .iter()
            .flat_map(|batch| batch.events.iter())
            .find(|event| event.event_type == AgentEventType::RuntimeWarning)
            .expect("unknown Claude ACP frame should be retained");
        assert_eq!(
            unknown.payload["frame"]["params"]["update"]["future"],
            "kept"
        );
        assert_eq!(
            unknown.provider_metadata.as_ref().unwrap()["claude.acp.unknown"]["params"]["update"]
                ["future"],
            "kept"
        );
        let capabilities = adapter.capabilities();
        let option = capabilities
            .config_options
            .iter()
            .find(|option| option.id == "future-claude-option")
            .unwrap();
        assert_eq!(option.category, "claude.future");
        assert_eq!(option.value["nested"][0], "preserved");
        assert_eq!(
            option.provider_metadata.as_ref().unwrap()["claude.acp.config"]["vendorField"],
            "kept"
        );
    }

    #[test]
    fn claude_malformed_jsonl_does_not_launch_legacy_cli_or_a_pty() {
        let mut adapter = adapter();
        let batches =
            adapter.map_jsonl("{bad json\n{\"method\":\"session/cancel\",\"params\":{}}\n");
        assert!(batches
            .iter()
            .flat_map(|batch| batch.events.iter())
            .any(|event| { event.event_type == AgentEventType::RuntimeWarning }));
        assert!(!adapter.uses_pty());
    }

    #[test]
    fn claude_capabilities_are_snapshot_driven_without_a_model_catalog() {
        let mut adapter = adapter();
        adapter.map_frame(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
        }));
        adapter.map_frame(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "result": {
                "agentInfo": { "name": "claude-agent-acp", "version": "0.23.1" },
                "agentCapabilities": {
                    "loadSession": true,
                    "sessionCapabilities": { "resume": true, "close": true },
                    "promptCapabilities": { "image": true },
                    "permissions": true, "structuredUserInput": true,
                    "toolTerminals": true, "plans": true, "tasks": true, "subagents": true
                },
                "sessionConfigOptions": [
                    { "id": "claude-model", "label": "Model", "category": "model", "value": "fixture-model" },
                    { "id": "thought", "label": "Thought level", "category": "thought_level", "value": "medium" },
                    { "id": "permissions", "label": "Permissions", "category": "mode", "value": "ask" }
                ]
            }
        }));
        let capabilities = adapter.capabilities();
        assert_eq!(capabilities.implementation.name, "claude-agent-acp");
        assert!(capabilities.prompt.image);
        assert!(capabilities.interaction.structured_user_input);
        assert_eq!(capabilities.config_options[0].value, "fixture-model");
        assert!(!capabilities
            .config_options
            .iter()
            .any(|option| option.id == "claude-opus"));
    }

    #[test]
    fn claude_fixture_contains_only_redacted_jsonl() {
        let text = fixture("all-categories.jsonl");
        assert!(text
            .lines()
            .all(|line| serde_json::from_str::<Value>(line).is_ok()));
        assert!(!text.contains("claude -p"));
        assert!(!text.contains("portable-pty"));
        assert!(!text.contains("sk-ant-"));
    }
}
