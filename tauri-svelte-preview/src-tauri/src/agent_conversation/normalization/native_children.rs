//! Both installed ACP sub-agent drafts describe the same parent/child lifecycle.
use serde_json::Value;

pub(in crate::agent_conversation) struct ChildUpdate<'a> {
    pub child_id: &'a str,
    pub parent_id: &'a str,
    pub label: Option<&'a str>,
    pub activity: Option<&'a str>,
    pub state: Option<&'a str>,
}

/// Claude0.99.1 and Codex2.5.0 emit AIR for AIR clients and RFD for root subagents
/// clients (TSK-1574 real adapter frames, 2026-10-31). Older installed adapters
/// also emit AIR. In RFD the outer sessionId is the parent, the inner one the child.
pub(in crate::agent_conversation) fn read(params: &Value) -> Option<ChildUpdate<'_>> {
    let update = params.get("update").unwrap_or(params);
    let kind = super::session_update_kind(params)?;
    let rfd = kind == "subagent_update";
    if !rfd && !matches!(kind, "subagent_spawned" | "subagent_state_update") {
        return None;
    }
    let string = |key: &str, alias: &str| update.get(key).or_else(|| update.get(alias)).and_then(Value::as_str);
    let child_id = if rfd { string("sessionId", "session_id") } else { string("subagentSessionId", "subagent_session_id") }?;
    let parent_id = params.get("sessionId").or_else(|| params.get("session_id")).and_then(Value::as_str)?;
    let state = if kind == "subagent_spawned" {
        Some("running")
    } else {
        let value = update.get("state");
        match value.and_then(|state| state.as_str().or_else(|| state.get("state").and_then(Value::as_str))) {
            Some("idle") => Some(match value.and_then(|state| state.get("stopReason")).and_then(Value::as_str) {
                Some("error") => "failed",
                Some("cancelled") => "cancelled",
                _ => "finished",
            }),
            Some("completed") => Some("finished"),
            other => other,
        }
    };
    Some(ChildUpdate {
        child_id, parent_id, state,
        label: if rfd { string("title", "name") } else { string("name", "title") }.map(str::trim).filter(|label| !label.is_empty()),
        activity: if rfd { string("description", "task") } else { string("task", "description") },
    })
}
