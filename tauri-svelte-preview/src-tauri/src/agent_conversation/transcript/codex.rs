use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{
    home_dir, object, parse_json_lines, read_snapshot_text, stable_key, timestamp,
    ChildAgentDescriptor, CodexChildRollout, ConversationMetadata, ProjectedRecord,
    TranscriptMessage, TranscriptSnapshot,
};
use crate::agent_conversation::protocol::AgentEventType;

pub(super) fn discover_path(id: &str) -> Option<PathBuf> {
    let home = home_dir().ok()?;
    [
        home.join(".codex/sessions"),
        home.join(".codex/archived_sessions"),
    ]
    .into_iter()
    .find_map(|root| find(&root, id))
}

pub(super) fn scan_child_rollouts(
    parent_path: &Path,
    parent_id: &str,
    active_after: SystemTime,
) -> Result<Vec<CodexChildRollout>, String> {
    let directory = parent_path
        .parent()
        .ok_or_else(|| "The parent transcript has no dated directory".to_string())?;
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("Could not scan child transcripts: {error}"))?;
    let mut children = entries
        .flatten()
        .filter_map(|entry| child_rollout_from_first_line(&entry.path(), parent_id, active_after))
        .collect::<Vec<_>>();
    children.sort_by(|left, right| left.child_id.cmp(&right.child_id));
    Ok(children)
}

fn child_rollout_from_first_line(
    path: &Path,
    parent_id: &str,
    active_after: SystemTime,
) -> Option<CodexChildRollout> {
    if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
        return None;
    }
    let metadata = path.metadata().ok()?;
    let modified = metadata.modified().ok()?;
    let mut first_line = String::new();
    BufReader::new(File::open(path).ok()?)
        .read_line(&mut first_line)
        .ok()?;
    let value: Value = serde_json::from_str(&first_line).ok()?;
    let payload = value
        .get("type")
        .and_then(Value::as_str)
        .filter(|kind| *kind == "session_meta")
        .and_then(|_| value.get("payload"))?;
    if payload.get("parent_thread_id").and_then(Value::as_str) != Some(parent_id)
        || payload.get("thread_source").and_then(Value::as_str) != Some("subagent")
    {
        return None;
    }
    let child_id = payload.get("id").and_then(Value::as_str)?.to_string();
    let spawn = payload.pointer("/source/subagent/thread_spawn");
    let role = spawn
        .and_then(|value| value.get("agent_role"))
        .or_else(|| payload.get("agent_role"))
        .and_then(Value::as_str);
    let name = spawn
        .and_then(|value| value.get("agent_path"))
        .or_else(|| payload.get("agent_path"))
        .and_then(Value::as_str)
        .and_then(|path| path.rsplit('/').find(|part| !part.is_empty()))
        .or_else(|| {
            spawn
                .and_then(|value| value.get("agent_nickname"))
                .or_else(|| payload.get("agent_nickname"))
                .and_then(Value::as_str)
        })
        .unwrap_or("Sub-agent");
    let label = role.map_or_else(|| name.to_string(), |role| format!("{name} ({role})"));
    let running = modified >= active_after;
    Some(CodexChildRollout {
        child_id,
        label,
        state: if running { "running" } else { "finished" }.to_string(),
        latest_activity: if running { "Running" } else { "Finished" }.to_string(),
    })
}

fn find(root: &Path, id: &str) -> Option<PathBuf> {
    let suffix = format!("{id}.jsonl");
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(&suffix))
            {
                return Some(path);
            }
        }
    }
    None
}

pub(super) fn project(value: &Value, line: &[u8]) -> Vec<ProjectedRecord> {
    let mut records = Vec::new();
    match (
        value.get("type").and_then(Value::as_str),
        value.pointer("/payload/type").and_then(Value::as_str),
    ) {
        (Some("response_item"), Some("message")) => {
            let Some(role) = value
                .pointer("/payload/role")
                .and_then(Value::as_str)
                .filter(|role| *role == "user" || *role == "assistant")
            else {
                return records;
            };
            let text = value
                .pointer("/payload/content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|item| {
                    let kind = item.get("type").and_then(Value::as_str)?;
                    ((role == "user" && kind == "input_text")
                        || (role == "assistant" && kind == "output_text"))
                        .then(|| item.get("text").and_then(Value::as_str))
                        .flatten()
                })
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");
            if text.is_empty() {
                return records;
            }
            let item_id = value
                .pointer("/payload/id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| stable_key("codex-message", line));
            let item_type = if role == "user" {
                "user-message"
            } else {
                "assistant-message"
            };
            let item = object([
                ("id", json!(item_id)),
                ("type", json!(item_type)),
                (
                    "content",
                    json!([{"channel": if role == "user" { "assistant" } else { "assistant" }, "text": text}]),
                ),
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
                payload: BTreeMap::from([
                    ("historical".into(), json!(true)),
                    ("item".into(), item),
                ]),
            });
        }
        (Some("turn_context"), _) => {
            let config = json!({
                "model": value.pointer("/payload/model").and_then(Value::as_str),
                "effort": value.pointer("/payload/effort").and_then(Value::as_str),
                "approvalPolicy": value.pointer("/payload/approval_policy").and_then(Value::as_str)
            });
            records.push(ProjectedRecord {
                key: stable_key("codex-config", line),
                event_type: AgentEventType::SessionConfigUpdated,
                timestamp_ms: timestamp(value),
                item_id: None,
                payload: BTreeMap::from([
                    ("historical".into(), json!(true)),
                    ("config".into(), config),
                ]),
            });
        }
        (Some("event_msg"), Some("token_count")) => {
            records.push(ProjectedRecord {
                key: stable_key("codex-usage", line),
                event_type: AgentEventType::UsageUpdated,
                timestamp_ms: timestamp(value),
                item_id: None,
                payload: BTreeMap::from([
                    ("historical".into(), json!(true)),
                    (
                        "usedTokens".into(),
                        value
                            .pointer("/payload/info/total_token_usage/total_tokens")
                            .cloned()
                            .unwrap_or(Value::Null),
                    ),
                    (
                        "contextWindow".into(),
                        value
                            .pointer("/payload/info/model_context_window")
                            .cloned()
                            .unwrap_or(Value::Null),
                    ),
                ]),
            });
        }
        (Some("event_msg"), Some("sub_agent_activity")) => {
            let Some(child_id) = value
                .pointer("/payload/agent_thread_id")
                .and_then(Value::as_str)
            else {
                return records;
            };
            let kind = value
                .pointer("/payload/kind")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let state = if matches!(kind, "completed" | "finished") {
                "completed"
            } else if matches!(kind, "failed" | "errored" | "interrupted") {
                "failed"
            } else {
                "historical"
            };
            let child = json!({"childId": child_id, "label": value.pointer("/payload/agent_path").and_then(Value::as_str).unwrap_or("Sub-agent"), "state": state, "historical": true});
            records.push(ProjectedRecord {
                key: stable_key("codex-child", line),
                event_type: AgentEventType::ChildrenUpdated,
                timestamp_ms: timestamp(value),
                item_id: None,
                payload: BTreeMap::from([
                    ("historical".into(), json!(true)),
                    ("children".into(), json!([child])),
                ]),
            });
        }
        _ => {}
    }
    records
}

pub(super) fn read_snapshot(
    id: &str,
    child_id: Option<&str>,
) -> Result<TranscriptSnapshot, String> {
    let target = child_id.unwrap_or(id);
    let path =
        discover_path(target).ok_or_else(|| format!("Codex transcript {target} was not found"))?;
    let input = read_snapshot_text(&path)?;
    let mut messages = Vec::new();
    let mut metadata = ConversationMetadata::default();
    let mut children = HashMap::new();
    for value in parse_json_lines(&input) {
        if value.get("type").and_then(Value::as_str) == Some("turn_context") {
            metadata.model = value
                .pointer("/payload/model")
                .and_then(Value::as_str)
                .map(str::to_string);
            metadata.effort = value
                .pointer("/payload/effort")
                .and_then(Value::as_str)
                .map(str::to_string);
            metadata.approval_policy = value
                .pointer("/payload/approval_policy")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        if value.pointer("/payload/type").and_then(Value::as_str) == Some("token_count") {
            metadata.used_tokens = value
                .pointer("/payload/info/total_token_usage/total_tokens")
                .and_then(Value::as_u64);
            metadata.context_window = value
                .pointer("/payload/info/model_context_window")
                .and_then(Value::as_u64);
        }
        for record in project(
            &value,
            serde_json::to_string(&value).unwrap_or_default().as_bytes(),
        ) {
            if record.event_type == AgentEventType::ItemCompleted {
                let Some(item) = record.payload.get("item") else {
                    continue;
                };
                let role = item
                    .pointer("/providerMetadata/transcriptRole")
                    .and_then(Value::as_str)
                    .unwrap_or("assistant");
                let text = item
                    .pointer("/content/0/text")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                messages.push(TranscriptMessage {
                    item_id: record.item_id.unwrap_or(record.key),
                    role: role.into(),
                    text: text.into(),
                    timestamp_ms: record.timestamp_ms as u64,
                });
            } else if record.event_type == AgentEventType::ChildrenUpdated && child_id.is_none() {
                if let Some(child) = record
                    .payload
                    .get("children")
                    .and_then(Value::as_array)
                    .and_then(|items| items.first())
                {
                    if let Some(child_id) = child.get("childId").and_then(Value::as_str) {
                        children.insert(
                            child_id.to_string(),
                            ChildAgentDescriptor {
                                child_id: child_id.into(),
                                parent_id: id.into(),
                                provider: "codex".into(),
                                label: child
                                    .get("label")
                                    .and_then(Value::as_str)
                                    .unwrap_or("Sub-agent")
                                    .into(),
                                state: child
                                    .get("state")
                                    .and_then(Value::as_str)
                                    .unwrap_or("historical")
                                    .into(),
                                updated_at_ms: record.timestamp_ms as u64,
                            },
                        );
                    }
                }
            }
        }
    }
    Ok(TranscriptSnapshot {
        messages,
        metadata,
        children: children.into_values().collect(),
    })
}
