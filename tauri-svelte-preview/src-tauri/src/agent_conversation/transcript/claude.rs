use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::{
    home_dir, object, parse_json_lines, read_snapshot_text, stable_key, timestamp,
    ChildAgentDescriptor, ConversationMetadata, ProjectedRecord, TranscriptMessage,
    TranscriptSnapshot,
};
use crate::agent_conversation::protocol::AgentEventType;

pub(super) fn discover_path(id: &str) -> Result<Option<PathBuf>, String> {
    find(&home_dir()?.join(".claude/projects"), id)
}

fn find(root: &Path, id: &str) -> Result<Option<PathBuf>, String> {
    let filename = format!("{id}.jsonl");
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.file_name().and_then(|name| name.to_str()) == Some(filename.as_str()) {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
}

pub(super) fn project(value: &Value, line: &[u8], session_id: &str) -> Vec<ProjectedRecord> {
    if value.get("sessionId").and_then(Value::as_str) != Some(session_id)
        || value.get("isMeta").and_then(Value::as_bool) == Some(true)
        || value.get("isSidechain").and_then(Value::as_bool) == Some(true)
    {
        return Vec::new();
    }
    let Some(role) = value
        .get("type")
        .and_then(Value::as_str)
        .filter(|role| *role == "user" || *role == "assistant")
    else {
        return Vec::new();
    };
    let text = match value.pointer("/message/content") {
        Some(Value::String(text)) if role == "user" => text.trim().to_string(),
        Some(Value::Array(items)) => items
            .iter()
            .filter(|item| item.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => String::new(),
    };
    let mut records = Vec::new();
    if !text.is_empty() {
        let item_id = value
            .get("uuid")
            .and_then(Value::as_str)
            .or_else(|| value.pointer("/message/id").and_then(Value::as_str))
            .map(str::to_string)
            .unwrap_or_else(|| stable_key("claude-message", line));
        let item_type = if role == "user" {
            "user-message"
        } else {
            "assistant-message"
        };
        let item = object([
            ("id", json!(item_id)),
            ("type", json!(item_type)),
            ("content", json!([{"channel": "assistant", "text": text}])),
            (
                "providerMetadata",
                json!({"historical": true, "transcriptRole": role}),
            ),
        ]);
        records.push(ProjectedRecord {
            key: format!("item:{item_id}"),
            event_type: AgentEventType::ItemCompleted,
            timestamp_ms: timestamp(value),
            item_id: Some(item_id),
            payload: BTreeMap::from([("historical".into(), json!(true)), ("item".into(), item)]),
        });
    }
    let model = value.pointer("/message/model").and_then(Value::as_str);
    let effort = value.get("effort").and_then(Value::as_str);
    let approval = value.get("permissionMode").and_then(Value::as_str);
    if model.is_some() || effort.is_some() || approval.is_some() {
        records.push(ProjectedRecord {
            key: stable_key("claude-config", line),
            event_type: AgentEventType::SessionConfigUpdated,
            timestamp_ms: timestamp(value),
            item_id: None,
            payload: BTreeMap::from([
                ("historical".into(), json!(true)),
                (
                    "config".into(),
                    json!({"model": model, "effort": effort, "approvalPolicy": approval}),
                ),
            ]),
        });
    }
    if let Some(usage) = value.pointer("/message/usage") {
        let used = [
            "input_tokens",
            "cache_creation_input_tokens",
            "cache_read_input_tokens",
        ]
        .into_iter()
        .filter_map(|key| usage.get(key).and_then(Value::as_u64))
        .sum::<u64>();
        if used > 0 {
            records.push(ProjectedRecord {
                key: stable_key("claude-usage", line),
                event_type: AgentEventType::UsageUpdated,
                timestamp_ms: timestamp(value),
                item_id: None,
                payload: BTreeMap::from([
                    ("historical".into(), json!(true)),
                    ("usedTokens".into(), json!(used)),
                ]),
            });
        }
    }
    records
}

pub(super) fn read_snapshot(
    id: &str,
    child_id: Option<&str>,
) -> Result<TranscriptSnapshot, String> {
    let parent =
        discover_path(id)?.ok_or_else(|| format!("Claude transcript {id} was not found"))?;
    let path = if let Some(child_id) = child_id {
        find_child(&parent, id, child_id)?
            .ok_or_else(|| format!("Claude child transcript {child_id} was not found"))?
    } else {
        parent.clone()
    };
    let input = read_snapshot_text(&path)?;
    let mut messages = Vec::new();
    let mut metadata = ConversationMetadata::default();
    for value in parse_json_lines(&input) {
        if child_id.is_some() {
            if value.get("agentId").and_then(Value::as_str) != child_id {
                continue;
            }
        }
        if let Some(model) = value.pointer("/message/model").and_then(Value::as_str) {
            metadata.model = Some(model.into());
        }
        if let Some(effort) = value.get("effort").and_then(Value::as_str) {
            metadata.effort = Some(effort.into());
        }
        if let Some(policy) = value.get("permissionMode").and_then(Value::as_str) {
            metadata.approval_policy = Some(policy.into());
        }
        if let Some(usage) = value.pointer("/message/usage") {
            let used = [
                "input_tokens",
                "cache_creation_input_tokens",
                "cache_read_input_tokens",
            ]
            .into_iter()
            .filter_map(|key| usage.get(key).and_then(Value::as_u64))
            .sum();
            if used > 0 {
                metadata.used_tokens = Some(used);
            }
        }
        let mut projected = value.clone();
        if child_id.is_some() {
            projected["isSidechain"] = Value::Bool(false);
        }
        for record in project(
            &projected,
            serde_json::to_string(&projected)
                .unwrap_or_default()
                .as_bytes(),
            id,
        ) {
            if record.event_type != AgentEventType::ItemCompleted {
                continue;
            }
            let item = &record.payload["item"];
            messages.push(TranscriptMessage {
                item_id: record.item_id.unwrap_or(record.key),
                role: item
                    .pointer("/providerMetadata/transcriptRole")
                    .and_then(Value::as_str)
                    .unwrap_or("assistant")
                    .into(),
                text: item
                    .pointer("/content/0/text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .into(),
                timestamp_ms: record.timestamp_ms as u64,
            });
        }
    }
    Ok(TranscriptSnapshot {
        messages,
        metadata,
        children: if child_id.is_none() {
            discover_children(&parent, id)?
        } else {
            Vec::new()
        },
    })
}

fn children_dir(parent: &Path, id: &str) -> Option<PathBuf> {
    Some(parent.parent()?.join(id).join("subagents"))
}
fn find_child(parent: &Path, id: &str, child_id: &str) -> Result<Option<PathBuf>, String> {
    let Some(dir) = children_dir(parent, id) else {
        return Ok(None);
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(None);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        let input = read_snapshot_text(&path)?;
        if parse_json_lines(&input)
            .any(|value| value.get("agentId").and_then(Value::as_str) == Some(child_id))
        {
            return Ok(Some(path));
        }
    }
    Ok(None)
}
fn discover_children(parent: &Path, id: &str) -> Result<Vec<ChildAgentDescriptor>, String> {
    let Some(dir) = children_dir(parent, id) else {
        return Ok(Vec::new());
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(Vec::new());
    };
    let mut result = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        let input = read_snapshot_text(&path)?;
        let values = parse_json_lines(&input).collect::<Vec<_>>();
        let Some(child_id) = values
            .iter()
            .find_map(|value| value.get("agentId").and_then(Value::as_str))
        else {
            continue;
        };
        let completed = values.iter().any(|value| {
            value
                .pointer("/message/stop_reason")
                .and_then(Value::as_str)
                .is_some()
        });
        result.push(ChildAgentDescriptor {
            child_id: child_id.into(),
            parent_id: id.into(),
            provider: "claude".into(),
            label: child_id.trim_start_matches("agent-").into(),
            state: if completed { "completed" } else { "historical" }.into(),
            updated_at_ms: super::modified_millis(&path),
        });
    }
    Ok(result)
}
