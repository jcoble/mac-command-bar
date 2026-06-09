use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionRecord {
    pub provider: String,
    pub id: String,
    pub title: String,
    pub project_path: Option<String>,
    pub last_activity: Option<String>,
    pub resume_commands: Vec<String>,
}

pub fn scan_sessions() -> Vec<AgentSessionRecord> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };

    let mut records = Vec::new();
    let codex_index = home.join(".codex/session_index.jsonl");
    if let Ok(contents) = fs::read_to_string(codex_index) {
        records.extend(parse_codex_index_jsonl(&contents));
    }

    let claude_projects = home.join(".claude/projects");
    for file in jsonl_files(&claude_projects) {
        let project_path = file
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .and_then(decode_claude_project_dir)
            .unwrap_or_default();
        if let Ok(contents) = fs::read_to_string(&file) {
            records.extend(parse_claude_jsonl(&contents, &project_path));
        }
    }

    records.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
    records.truncate(80);
    records
}

pub fn parse_codex_index_jsonl(input: &str) -> Vec<AgentSessionRecord> {
    input
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|value| {
            let id = value.get("id")?.as_str()?.to_string();
            let title = value
                .get("thread_name")
                .and_then(Value::as_str)
                .unwrap_or("Untitled Codex session")
                .to_string();
            let last_activity = value
                .get("updated_at")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);

            Some(AgentSessionRecord {
                provider: "codex".to_string(),
                id: id.clone(),
                title,
                project_path: None,
                last_activity,
                resume_commands: vec![format!("codex resume {id}")],
            })
        })
        .collect()
}

pub fn parse_claude_jsonl(input: &str, project_path: &str) -> Vec<AgentSessionRecord> {
    let mut latest: Option<AgentSessionRecord> = None;

    for value in input
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
    {
        let Some(id) = value
            .get("sessionId")
            .or_else(|| value.get("session_id"))
            .and_then(Value::as_str)
        else {
            continue;
        };

        let timestamp = value
            .get("timestamp")
            .or_else(|| value.get("created_at"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        let cwd = value
            .get("cwd")
            .and_then(Value::as_str)
            .unwrap_or(project_path)
            .to_string();

        let title =
            title_from_claude_message(&value).unwrap_or_else(|| "Claude session".to_string());
        latest = Some(AgentSessionRecord {
            provider: "claude".to_string(),
            id: id.to_string(),
            title,
            project_path: (!cwd.is_empty()).then_some(cwd.clone()),
            last_activity: timestamp,
            resume_commands: vec![
                format!("cd {} && claude --resume {id}", shell_quote(&cwd)),
                format!("claude --resume {id}"),
            ],
        });
    }

    latest.into_iter().collect()
}

fn title_from_claude_message(value: &Value) -> Option<String> {
    let content = value
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(value_to_text)
        .or_else(|| value.get("summary").and_then(value_to_text))?;

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.chars().take(80).collect())
}

fn value_to_text(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Array(values) => values
            .iter()
            .filter_map(|item| {
                item.get("text")
                    .and_then(Value::as_str)
                    .or_else(|| item.get("content").and_then(Value::as_str))
            })
            .collect::<Vec<_>>()
            .join(" ")
            .into(),
        _ => None,
    }
}

fn jsonl_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return files;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(jsonl_files(&path));
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }

    files
}

pub fn decode_claude_project_dir(name: &str) -> Option<String> {
    decode_claude_project_dir_with_users_root(name, Path::new("/Users"))
}

pub fn decode_claude_project_dir_with_users_root(name: &str, users_root: &Path) -> Option<String> {
    let rest = name.strip_prefix("-Users-")?;
    let segments: Vec<&str> = rest
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        return None;
    }

    let mut path = users_root.to_path_buf();
    let mut index = 0;
    while index < segments.len() {
        let mut chosen_len = 1;
        for len in (1..=(segments.len() - index)).rev() {
            let candidate = segments[index..index + len].join("-");
            if path.join(&candidate).exists() {
                chosen_len = len;
                break;
            }
        }

        path.push(segments[index..index + chosen_len].join("-"));
        index += chosen_len;
    }

    Some(path.display().to_string())
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}
