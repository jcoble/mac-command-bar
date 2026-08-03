use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const TRANSCRIPT_TAIL_BYTES: u64 = 4 * 1024 * 1024;
static TRANSCRIPT_PATHS: OnceLock<Mutex<HashMap<String, PathBuf>>> = OnceLock::new();

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptMessage {
    pub item_id: String,
    pub role: String,
    pub text: String,
    pub timestamp_ms: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMetadata {
    pub model: Option<String>,
    pub effort: Option<String>,
    pub approval_policy: Option<String>,
    pub used_tokens: Option<u64>,
    pub context_window: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildAgentDescriptor {
    pub child_id: String,
    pub parent_id: String,
    pub provider: String,
    pub label: String,
    pub state: String,
    pub updated_at_ms: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSnapshot {
    pub messages: Vec<TranscriptMessage>,
    pub metadata: ConversationMetadata,
    pub children: Vec<ChildAgentDescriptor>,
}

pub fn read(
    provider: &str,
    native_session_id: &str,
    child_session_id: Option<&str>,
) -> Result<TranscriptSnapshot, String> {
    match provider.trim().to_ascii_lowercase().as_str() {
        "claude" => read_claude(native_session_id, child_session_id),
        "codex" => read_codex(native_session_id, child_session_id),
        _ => Ok(TranscriptSnapshot::default()),
    }
}

fn read_codex(
    native_session_id: &str,
    child_session_id: Option<&str>,
) -> Result<TranscriptSnapshot, String> {
    let parent_id = safe_session_id(native_session_id)?;
    let target_id = child_session_id
        .map(safe_session_id)
        .transpose()?
        .unwrap_or_else(|| parent_id.clone());
    let home = home_dir()?;
    let roots = [
        home.join(".codex/sessions"),
        home.join(".codex/archived_sessions"),
    ];
    let path = cached_transcript_path(&format!("codex:{target_id}"), || {
        roots
            .iter()
            .find_map(|root| find_codex_transcript(root, &target_id))
    })
    .ok_or_else(|| format!("Codex transcript {target_id} was not found"))?;
    let contents = read_tail(&path)?;
    Ok(TranscriptSnapshot {
        messages: parse_codex(&contents),
        metadata: parse_codex_metadata(&contents),
        children: if target_id == parent_id {
            parse_codex_children(&contents, &parent_id)
        } else {
            Vec::new()
        },
    })
}

fn read_claude(
    native_session_id: &str,
    child_session_id: Option<&str>,
) -> Result<TranscriptSnapshot, String> {
    let parent_id = safe_session_id(native_session_id)?;
    let child_id = child_session_id.map(safe_session_id).transpose()?;
    let root = home_dir()?.join(".claude/projects");
    let parent_path = cached_transcript_path(&format!("claude:{parent_id}"), || {
        find_claude_transcript(&root, &parent_id).ok().flatten()
    })
    .ok_or_else(|| format!("Claude transcript {parent_id} was not found"))?;
    if let Some(child_id) = child_id {
        let path = find_claude_child(&parent_path, &parent_id, &child_id)?
            .ok_or_else(|| format!("Claude child transcript {child_id} was not found"))?;
        let contents = read_tail(&path)?;
        return Ok(TranscriptSnapshot {
            messages: parse_claude(&contents, &parent_id, Some(&child_id)),
            metadata: parse_claude_metadata(&contents),
            children: Vec::new(),
        });
    }
    let contents = read_tail(&parent_path)?;
    Ok(TranscriptSnapshot {
        messages: parse_claude(&contents, &parent_id, None),
        metadata: parse_claude_metadata(&contents),
        children: discover_claude_children(&parent_path, &parent_id)?,
    })
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is unavailable".to_string())
}

fn cached_transcript_path(
    key: &str,
    discover: impl FnOnce() -> Option<PathBuf>,
) -> Option<PathBuf> {
    let cache = TRANSCRIPT_PATHS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(paths) = cache.lock() {
        if let Some(path) = paths.get(key).filter(|path| path.is_file()) {
            return Some(path.clone());
        }
    }
    let path = discover()?;
    if let Ok(mut paths) = cache.lock() {
        paths.insert(key.to_string(), path.clone());
    }
    Some(path)
}

fn safe_session_id(value: &str) -> Result<String, String> {
    let id = value.trim();
    if id.is_empty()
        || id.contains('/')
        || id.contains('\\')
        || !id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err("Session id is invalid".to_string());
    }
    Ok(id.to_string())
}

fn find_claude_transcript(root: &Path, id: &str) -> Result<Option<PathBuf>, String> {
    let filename = format!("{id}.jsonl");
    let projects =
        fs::read_dir(root).map_err(|error| format!("Could not read transcripts: {error}"))?;
    for project in projects.flatten() {
        let candidate = project.path().join(&filename);
        if candidate.is_file() {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn claude_children_dir(parent_path: &Path, parent_id: &str) -> Option<PathBuf> {
    Some(parent_path.parent()?.join(parent_id).join("subagents"))
}

fn find_claude_child(
    parent_path: &Path,
    parent_id: &str,
    child_id: &str,
) -> Result<Option<PathBuf>, String> {
    let Some(directory) = claude_children_dir(parent_path, parent_id) else {
        return Ok(None);
    };
    let Ok(entries) = fs::read_dir(directory) else {
        return Ok(None);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        let contents = read_tail(&path)?;
        if contents
            .lines()
            .filter_map(parse_json)
            .any(|value| value.get("agentId").and_then(Value::as_str) == Some(child_id))
        {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn discover_claude_children(
    parent_path: &Path,
    parent_id: &str,
) -> Result<Vec<ChildAgentDescriptor>, String> {
    let Some(directory) = claude_children_dir(parent_path, parent_id) else {
        return Ok(Vec::new());
    };
    let Ok(entries) = fs::read_dir(directory) else {
        return Ok(Vec::new());
    };
    let mut children = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        let contents = read_tail(&path)?;
        let values = contents.lines().filter_map(parse_json).collect::<Vec<_>>();
        let Some(child_id) = values
            .iter()
            .find_map(|value| value.get("agentId").and_then(Value::as_str))
        else {
            continue;
        };
        let completed = values.iter().rev().any(|value| {
            value.get("type").and_then(Value::as_str) == Some("assistant")
                && value
                    .pointer("/message/stop_reason")
                    .and_then(Value::as_str)
                    .is_some()
        });
        let label = child_id
            .strip_prefix("agent-")
            .unwrap_or(child_id)
            .to_string();
        children.push(ChildAgentDescriptor {
            child_id: child_id.to_string(),
            parent_id: parent_id.to_string(),
            provider: "claude".to_string(),
            label,
            state: if completed { "completed" } else { "available" }.to_string(),
            updated_at_ms: modified_millis(&path),
        });
    }
    children.sort_by(|left, right| right.updated_at_ms.cmp(&left.updated_at_ms));
    Ok(children)
}

fn find_codex_transcript(root: &Path, id: &str) -> Option<PathBuf> {
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

fn read_tail(path: &Path) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|error| format!("Could not open transcript: {error}"))?;
    let len = file.metadata().map_err(|error| error.to_string())?.len();
    let start = len.saturating_sub(TRANSCRIPT_TAIL_BYTES);
    file.seek(SeekFrom::Start(start))
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let text = String::from_utf8_lossy(&bytes);
    Ok(if start == 0 {
        text.into_owned()
    } else {
        text.split_once('\n')
            .map(|(_, rest)| rest)
            .unwrap_or("")
            .to_string()
    })
}

fn parse_json(line: &str) -> Option<Value> {
    serde_json::from_str(line).ok()
}

fn parse_claude(input: &str, session_id: &str, child_id: Option<&str>) -> Vec<TranscriptMessage> {
    input
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let value = parse_json(line)?;
            if value.get("sessionId").and_then(Value::as_str) != Some(session_id)
                || value.get("isMeta").and_then(Value::as_bool) == Some(true)
            {
                return None;
            }
            match child_id {
                Some(id) if value.get("agentId").and_then(Value::as_str) != Some(id) => {
                    return None
                }
                None if value.get("isSidechain").and_then(Value::as_bool) == Some(true) => {
                    return None
                }
                _ => {}
            }
            let role = value.get("type")?.as_str()?;
            if role != "user" && role != "assistant" {
                return None;
            }
            let content = value.pointer("/message/content")?;
            let text = match content {
                Value::String(text) if role == "user" => text.trim().to_string(),
                Value::Array(items) => items
                    .iter()
                    .filter(|item| item.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|item| item.get("text").and_then(Value::as_str))
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n\n"),
                _ => return None,
            };
            if text.is_empty() {
                return None;
            }
            let item_id = value
                .get("uuid")
                .and_then(Value::as_str)
                .or_else(|| value.pointer("/message/id").and_then(Value::as_str))
                .map(str::to_string)
                .unwrap_or_else(|| format!("transcript-{index}"));
            Some(TranscriptMessage {
                item_id,
                role: role.to_string(),
                text,
                timestamp_ms: value
                    .get("timestamp")
                    .and_then(Value::as_str)
                    .and_then(parse_timestamp_millis)
                    .unwrap_or(index as u64),
            })
        })
        .collect()
}

fn parse_claude_metadata(input: &str) -> ConversationMetadata {
    let mut result = ConversationMetadata::default();
    for value in input.lines().filter_map(parse_json) {
        if let Some(model) = value.pointer("/message/model").and_then(Value::as_str) {
            result.model = Some(model.to_string());
        }
        if let Some(effort) = value.get("effort").and_then(Value::as_str) {
            result.effort = Some(effort.to_string());
        }
        if let Some(policy) = value.get("permissionMode").and_then(Value::as_str) {
            result.approval_policy = Some(policy.to_string());
        }
        if let Some(usage) = value.pointer("/message/usage") {
            let used = [
                "input_tokens",
                "cache_creation_input_tokens",
                "cache_read_input_tokens",
            ]
            .iter()
            .filter_map(|key| usage.get(key).and_then(Value::as_u64))
            .sum::<u64>();
            if used > 0 {
                result.used_tokens = Some(used);
            }
        }
    }
    result
}

fn parse_codex(input: &str) -> Vec<TranscriptMessage> {
    input
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let value = parse_json(line)?;
            if value.get("type").and_then(Value::as_str) != Some("response_item")
                || value.pointer("/payload/type").and_then(Value::as_str) != Some("message")
            {
                return None;
            }
            let role = value.pointer("/payload/role").and_then(Value::as_str)?;
            if role != "user" && role != "assistant" {
                return None;
            }
            let text = value
                .pointer("/payload/content")
                .and_then(Value::as_array)?
                .iter()
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
                return None;
            }
            let item_id = value
                .pointer("/payload/id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("codex-transcript-{index}"));
            Some(TranscriptMessage {
                item_id,
                role: role.to_string(),
                text,
                timestamp_ms: index as u64,
            })
        })
        .collect()
}

fn parse_codex_metadata(input: &str) -> ConversationMetadata {
    let mut result = ConversationMetadata::default();
    for value in input.lines().filter_map(parse_json) {
        if value.get("type").and_then(Value::as_str) == Some("turn_context") {
            if let Some(model) = value.pointer("/payload/model").and_then(Value::as_str) {
                result.model = Some(model.to_string());
            }
            if let Some(effort) = value.pointer("/payload/effort").and_then(Value::as_str) {
                result.effort = Some(effort.to_string());
            }
            if let Some(policy) = value
                .pointer("/payload/approval_policy")
                .and_then(Value::as_str)
            {
                result.approval_policy = Some(policy.to_string());
            }
        }
        if value.get("type").and_then(Value::as_str) == Some("event_msg")
            && value.pointer("/payload/type").and_then(Value::as_str) == Some("token_count")
        {
            if let Some(tokens) = value
                .pointer("/payload/info/total_token_usage/total_tokens")
                .and_then(Value::as_u64)
            {
                result.used_tokens = Some(tokens);
            }
            if let Some(window) = value
                .pointer("/payload/info/model_context_window")
                .and_then(Value::as_u64)
            {
                result.context_window = Some(window);
            }
        }
    }
    result
}

fn parse_codex_children(input: &str, parent_id: &str) -> Vec<ChildAgentDescriptor> {
    let mut children = BTreeMap::<String, ChildAgentDescriptor>::new();
    for (index, value) in input.lines().filter_map(parse_json).enumerate() {
        if value.get("type").and_then(Value::as_str) != Some("event_msg")
            || value.pointer("/payload/type").and_then(Value::as_str) != Some("sub_agent_activity")
        {
            continue;
        }
        let Some(child_id) = value
            .pointer("/payload/agent_thread_id")
            .and_then(Value::as_str)
        else {
            continue;
        };
        let label = value
            .pointer("/payload/agent_path")
            .and_then(Value::as_str)
            .map(|path| path.trim_start_matches("/root/").replace('_', " "))
            .filter(|label| !label.is_empty())
            .unwrap_or_else(|| "Sub-agent".to_string());
        let kind = value
            .pointer("/payload/kind")
            .and_then(Value::as_str)
            .unwrap_or("available");
        let state = match kind {
            "started" | "running" | "interacted" => "active",
            "completed" | "finished" => "completed",
            "failed" | "errored" | "interrupted" => "failed",
            _ => "available",
        };
        children.insert(
            child_id.to_string(),
            ChildAgentDescriptor {
                child_id: child_id.to_string(),
                parent_id: parent_id.to_string(),
                provider: "codex".to_string(),
                label,
                state: state.to_string(),
                updated_at_ms: value
                    .pointer("/payload/occurred_at_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or(index as u64),
            },
        );
    }
    let mut values = children.into_values().collect::<Vec<_>>();
    values.sort_by(|left, right| right.updated_at_ms.cmp(&left.updated_at_ms));
    values
}

fn modified_millis(path: &Path) -> u64 {
    path.metadata()
        .ok()
        .and_then(|value| value.modified().ok())
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

fn parse_timestamp_millis(value: &str) -> Option<u64> {
    value.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_claude_parent_and_metadata_without_sidechain_messages() {
        let input = r#"{"type":"user","uuid":"u1","sessionId":"s1","isSidechain":false,"permissionMode":"acceptEdits","message":{"content":"Hello"}}
{"type":"assistant","uuid":"a1","sessionId":"s1","isSidechain":false,"effort":"high","message":{"model":"claude-opus-5","usage":{"input_tokens":2,"cache_read_input_tokens":120},"content":[{"type":"text","text":"Answer"}]}}
{"type":"assistant","uuid":"child","sessionId":"s1","isSidechain":true,"agentId":"agent-a","message":{"content":[{"type":"text","text":"Hidden"}]}}"#;
        let messages = parse_claude(input, "s1", None);
        let child_messages = parse_claude(input, "s1", Some("agent-a"));
        let metadata = parse_claude_metadata(input);
        assert_eq!(messages.len(), 2);
        assert_eq!(child_messages.len(), 1);
        assert_eq!(child_messages[0].text, "Hidden");
        assert_eq!(metadata.model.as_deref(), Some("claude-opus-5"));
        assert_eq!(metadata.effort.as_deref(), Some("high"));
        assert_eq!(metadata.approval_policy.as_deref(), Some("acceptEdits"));
        assert_eq!(metadata.used_tokens, Some(122));
    }

    #[test]
    fn parses_codex_metadata_messages_and_child_activity() {
        let input = r#"{"type":"turn_context","payload":{"model":"gpt-5.6-sol","effort":"medium","approval_policy":"never"}}
{"type":"response_item","payload":{"type":"message","id":"u1","role":"user","content":[{"type":"input_text","text":"Review this"}]}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":1000},"model_context_window":258400}}}
{"type":"event_msg","payload":{"type":"sub_agent_activity","agent_thread_id":"child-1","agent_path":"/root/reviewer","kind":"started","occurred_at_ms":50}}
{"type":"event_msg","payload":{"type":"sub_agent_activity","agent_thread_id":"child-1","agent_path":"/root/reviewer","kind":"interacted","occurred_at_ms":70}}"#;
        let metadata = parse_codex_metadata(input);
        let children = parse_codex_children(input, "parent");
        assert_eq!(parse_codex(input).len(), 1);
        assert_eq!(metadata.model.as_deref(), Some("gpt-5.6-sol"));
        assert_eq!(metadata.used_tokens, Some(1000));
        assert_eq!(metadata.context_window, Some(258400));
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].state, "active");
    }

    #[test]
    fn rejects_unsafe_child_session_ids() {
        assert!(safe_session_id("../secret").is_err());
        assert!(safe_session_id("child-ok_1").is_ok());
    }
}
