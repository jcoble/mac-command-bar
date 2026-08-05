//! Codex ACP frame adapter.
//!
//! Codex ACP is consumed as structured JSON-RPC over the A2 piped transport.  This adapter is
//! intentionally a pure frame projector: it accepts fixture/transport frames and returns the
//! canonical Assembly events and ordered item snapshots.  It never starts `codex`, attaches to a
//! TUI, or creates a PTY.

use serde_json::Value;

use super::super::protocol::{AgentCapabilities, AgentConversationProvider, AgentItem};
use super::adapter_support::AcpProjectionState;
pub use super::adapter_support::{AdapterContext, ProjectionBatch};

pub const CODEX_METADATA_NAMESPACE: &str = "codex.acp";

pub struct CodexFrameAdapter {
    state: AcpProjectionState,
}

impl CodexFrameAdapter {
    pub fn new(context: AdapterContext) -> Self {
        Self {
            state: AcpProjectionState::new(
                context,
                AgentConversationProvider::Codex,
                CODEX_METADATA_NAMESPACE,
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

pub type CodexAdapter = CodexFrameAdapter;
pub type CodexProviderAdapter = CodexFrameAdapter;

pub fn map_codex_frame(context: AdapterContext, frame: &Value) -> ProjectionBatch {
    let mut adapter = CodexFrameAdapter::new(context);
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
            .join("fixtures/agent_conversation/codex")
            .join(name);
        std::fs::read_to_string(path).expect("Codex fixture must be present")
    }

    fn adapter() -> CodexFrameAdapter {
        CodexFrameAdapter::new(
            AdapterContext::new("owned-codex", "codex-provider-1", 4, Some("session-old"))
                .with_timestamp(10_000),
        )
    }

    #[test]
    fn codex_fixture_maps_every_canonical_category_and_keeps_interleaved_ids() {
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
            AgentEventType::PlanUpdated,
            AgentEventType::TasksUpdated,
            AgentEventType::ChildrenUpdated,
            AgentEventType::ApprovalRequested,
            AgentEventType::UserInputRequested,
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
        let sequences: Vec<_> = events.iter().map(|event| event.sequence).collect();
        let mut sorted = sequences.clone();
        sorted.sort_unstable();
        assert_eq!(sequences, sorted);
        assert!(sequences.windows(2).all(|window| window[0] < window[1]));

        let item_ids: Vec<_> = adapter
            .ordered_items()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(
            item_ids,
            vec![
                "user-1",
                "assistant-1",
                "command-1",
                "reasoning-1",
                "file-1",
                "assistant-2",
                "plan-1",
                "tasks-1",
                "child-tool-1",
            ]
        );
        let prompt = adapter.ordered_items()[0]
            .provider_metadata
            .as_ref()
            .unwrap()["codex.acp.item"]["prompt"]
            .clone();
        assert_eq!(prompt[1]["type"], "image");
        assert_eq!(prompt[1]["mimeType"], "image/png");
        let ordered_items = adapter.ordered_items();
        let command = ordered_items
            .iter()
            .find(|item| item.id == "command-1")
            .unwrap();
        assert!(command
            .content
            .iter()
            .any(|content| content.text.contains("src/main.rs")));
        assert!(ordered_items
            .iter()
            .find(|item| item.id == "file-1")
            .and_then(|item| item.provider_metadata.as_ref())
            .is_some());
        assert!(adapter
            .ordered_items()
            .iter()
            .any(|item| item.item_type == AgentItemType::Subagent));
        assert!(!adapter.uses_pty());
    }

    #[test]
    fn codex_unknown_frame_and_config_category_round_trip_losslessly() {
        let mut adapter = adapter();
        let batches = adapter.map_jsonl(&fixture("unknown-and-lifecycle.jsonl"));
        let unknown = batches
            .iter()
            .flat_map(|batch| batch.events.iter())
            .find(|event| event.event_type == AgentEventType::RuntimeWarning)
            .expect("unknown ACP frame should be retained");
        assert_eq!(
            unknown.payload["frame"]["params"]["update"]["futureField"],
            "kept"
        );
        assert_eq!(
            unknown.provider_metadata.as_ref().unwrap()["codex.acp.unknown"]["params"]["update"]
                ["futureField"],
            "kept"
        );

        let capabilities = adapter.capabilities();
        let option = capabilities
            .config_options
            .iter()
            .find(|option| option.id == "future-option")
            .expect("unknown config option should be advertised");
        assert_eq!(option.category, "provider.future/category");
        assert_eq!(
            option.provider_metadata.as_ref().unwrap()["codex.acp.config"]["custom"],
            true
        );
        let round_trip = serde_json::to_value(option.provider_metadata.as_ref().unwrap()).unwrap();
        assert_eq!(round_trip["codex.acp.config"]["custom"], true);
    }

    #[test]
    fn codex_malformed_jsonl_is_non_fatal_and_real_processes_are_never_started() {
        let mut adapter = adapter();
        let batches =
            adapter.map_jsonl("not json\n\n{\"method\":\"session/cancel\",\"params\":{}}\n");
        assert!(batches
            .iter()
            .flat_map(|batch| batch.events.iter())
            .any(|event| { event.event_type == AgentEventType::RuntimeWarning }));
        assert!(!adapter.uses_pty());
    }

    #[test]
    fn codex_capability_snapshot_uses_advertised_models_images_effort_and_mode_only() {
        let mut adapter = adapter();
        let initialize = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "result": {
                "agentInfo": { "name": "codex-acp", "version": "1.1.9" },
                "agentCapabilities": {
                    "loadSession": true,
                    "sessionCapabilities": { "resume": true, "close": true },
                    "promptCapabilities": { "image": true },
                    "permissions": true, "plans": true, "tasks": true, "subagents": true
                },
                "sessionConfigOptions": [
                    { "id": "model", "label": "Model", "category": "model", "value": "gpt-fixture", "options": [{"value":"gpt-fixture","label":"Fixture"}] },
                    { "id": "effort", "label": "Reasoning", "category": "thought_level", "value": "high" },
                    { "id": "mode", "label": "Mode", "category": "mode", "value": "workspace" }
                ]
            }
        });
        adapter.map_frame(
            &serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        );
        adapter.map_frame(&initialize);
        let capabilities = adapter.capabilities();
        assert!(capabilities.prompt.image);
        assert!(capabilities.interaction.permissions);
        assert_eq!(capabilities.config_options.len(), 3);
        assert_eq!(capabilities.config_options[1].category, "thought_level");
        assert_eq!(capabilities.config_options[2].category, "mode");
        assert!(!capabilities
            .config_options
            .iter()
            .any(|option| option.id == "gpt-5"));
    }

    #[test]
    fn codex_fixture_is_jsonl_not_a_pty_script() {
        let text = fixture("all-categories.jsonl");
        assert!(text
            .lines()
            .all(|line| serde_json::from_str::<Value>(line).is_ok()));
        assert!(!text.contains("portable-pty"));
    }
}
