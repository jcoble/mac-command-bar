use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const CLAUDE_SESSION_FILE_LIMIT: usize = 120;
const CLAUDE_SESSION_TAIL_BYTES: usize = 256 * 1024;
const CODEX_SESSION_FILE_LIMIT: usize = 160;
const CODEX_SESSION_HEAD_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionRecord {
    pub provider: String,
    pub id: String,
    pub title: String,
    pub model: Option<String>,
    pub project_path: Option<String>,
    pub last_activity: Option<String>,
    pub resume_commands: Vec<String>,
}

pub fn scan_sessions() -> Vec<AgentSessionRecord> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };

    let mut records = Vec::new();
    let mut codex_records = Vec::new();
    let codex_index = home.join(".codex/session_index.jsonl");
    if let Ok(contents) = fs::read_to_string(codex_index) {
        codex_records.extend(parse_codex_index_jsonl(&contents));
    }

    let codex_sessions = home.join(".codex/sessions");
    let mut codex_files = jsonl_files(&codex_sessions);
    codex_files.sort_by(|a, b| modified_time(b).cmp(&modified_time(a)));
    let mut codex_metadata = Vec::new();
    for file in codex_files.into_iter().take(CODEX_SESSION_FILE_LIMIT) {
        if let Ok(contents) = read_head_utf8(&file, CODEX_SESSION_HEAD_BYTES) {
            codex_metadata.extend(parse_codex_rollout_jsonl(&contents));
        }
    }
    records.extend(merge_codex_session_metadata(codex_records, codex_metadata));

    let cmux_term = home.join(".cmuxterm");
    for (agent, file) in cmux_hook_session_files(&cmux_term) {
        if let Ok(contents) = fs::read_to_string(file) {
            records.extend(parse_cmux_hook_sessions_json(&agent, &contents));
        }
    }

    let claude_projects = home.join(".claude/projects");
    let mut files = jsonl_files(&claude_projects);
    files.sort_by(|a, b| modified_time(b).cmp(&modified_time(a)));
    for file in files.into_iter().take(CLAUDE_SESSION_FILE_LIMIT) {
        let project_path = file
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .and_then(decode_claude_project_dir)
            .unwrap_or_default();
        if let Ok(contents) = read_tail_utf8(&file, CLAUDE_SESSION_TAIL_BYTES) {
            records.extend(parse_claude_jsonl(&contents, &project_path));
        }
    }

    records = merge_agent_session_records(records);
    records.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
    records.truncate(80);
    records
}

pub fn merge_agent_session_records(records: Vec<AgentSessionRecord>) -> Vec<AgentSessionRecord> {
    let mut merged = Vec::<AgentSessionRecord>::new();

    for record in records {
        if let Some(existing) = merged
            .iter_mut()
            .find(|candidate| candidate.provider == record.provider && candidate.id == record.id)
        {
            merge_agent_session_record(existing, record);
        } else {
            merged.push(record);
        }
    }

    merged
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
                model: model_from_value(&value),
                project_path: None,
                last_activity,
                resume_commands: vec![format!("codex resume {id}")],
            })
        })
        .collect()
}

pub fn parse_codex_rollout_jsonl(input: &str) -> Vec<AgentSessionRecord> {
    let mut records: Vec<AgentSessionRecord> = Vec::new();

    for value in input
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
    {
        if value.get("type").and_then(Value::as_str) != Some("session_meta") {
            continue;
        }

        let Some(payload) = value.get("payload") else {
            continue;
        };
        let Some(id) = payload.get("id").and_then(Value::as_str) else {
            continue;
        };

        let cwd = payload
            .get("cwd")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned);
        let last_activity = value
            .get("timestamp")
            .and_then(Value::as_str)
            .or_else(|| payload.get("timestamp").and_then(Value::as_str))
            .map(ToOwned::to_owned);

        let record = AgentSessionRecord {
            provider: "codex".to_string(),
            id: id.to_string(),
            title: "Codex session".to_string(),
            model: model_from_value(payload),
            project_path: cwd,
            last_activity,
            resume_commands: vec![format!("codex resume {id}")],
        };

        if let Some(existing) = records
            .iter_mut()
            .find(|candidate| candidate.provider == record.provider && candidate.id == record.id)
        {
            merge_codex_record(existing, record);
        } else {
            records.push(record);
        }
    }

    records
}

pub fn merge_codex_session_metadata(
    mut indexed: Vec<AgentSessionRecord>,
    metadata: Vec<AgentSessionRecord>,
) -> Vec<AgentSessionRecord> {
    for record in metadata {
        if let Some(existing) = indexed
            .iter_mut()
            .find(|candidate| candidate.provider == record.provider && candidate.id == record.id)
        {
            merge_codex_record(existing, record);
        } else {
            indexed.push(record);
        }
    }

    indexed
}

pub fn parse_cmux_hook_sessions_json(agent: &str, input: &str) -> Vec<AgentSessionRecord> {
    let Ok(value) = serde_json::from_str::<Value>(input) else {
        return Vec::new();
    };
    let Some(sessions) = value.get("sessions").and_then(Value::as_object) else {
        return Vec::new();
    };

    let agent = agent.trim().to_lowercase();
    if agent.is_empty() {
        return Vec::new();
    }

    sessions
        .iter()
        .filter_map(|(key, value)| {
            let id = value
                .get("sessionId")
                .and_then(Value::as_str)
                .unwrap_or(key)
                .trim();
            if id.is_empty() {
                return None;
            }

            let cwd = value
                .get("cwd")
                .and_then(Value::as_str)
                .or_else(|| {
                    value
                        .get("launchCommand")
                        .and_then(|launch| launch.get("workingDirectory"))
                        .and_then(Value::as_str)
                })
                .filter(|value| !value.trim().is_empty())
                .map(ToOwned::to_owned);
            let last_activity = value
                .get("updatedAt")
                .and_then(Value::as_str)
                .or_else(|| value.get("startedAt").and_then(Value::as_str))
                .or_else(|| {
                    value
                        .get("launchCommand")
                        .and_then(|launch| launch.get("capturedAt"))
                        .and_then(Value::as_str)
                })
                .map(ToOwned::to_owned);
            let status = value
                .get("runtimeStatus")
                .and_then(Value::as_str)
                .or_else(|| value.get("agentLifecycle").and_then(Value::as_str))
                .filter(|value| !value.trim().is_empty());
            let title = status
                .map(|status| format!("cmux {agent} · {status}"))
                .unwrap_or_else(|| format!("cmux {agent} session"));

            Some(AgentSessionRecord {
                provider: format!("cmux-{agent}"),
                id: id.to_string(),
                title,
                model: model_from_value(value),
                project_path: cwd.clone(),
                last_activity,
                resume_commands: cmux_resume_commands(&agent, id, cwd.as_deref()),
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
            model: value
                .get("message")
                .and_then(model_from_value)
                .or_else(|| model_from_value(&value)),
            project_path: (!cwd.is_empty()).then_some(cwd.clone()),
            last_activity: timestamp,
            resume_commands: vec![
                format!("claude --resume {id}"),
                format!("cd {} && claude --resume {id}", shell_quote(&cwd)),
            ],
        });
    }

    latest.into_iter().collect()
}

fn cmux_resume_commands(agent: &str, id: &str, cwd: Option<&str>) -> Vec<String> {
    let command = match agent {
        "codex" => format!("codex resume {id}"),
        "claude" => format!("claude --resume {id}"),
        "gemini" => format!("gemini --resume {id}"),
        "opencode" => format!("opencode --session {id}"),
        "omp" | "pi" => format!("{agent} --session {id}"),
        "amp" => format!("amp threads continue {id}"),
        "antigravity" | "agy" => format!("agy --conversation {id}"),
        "rovo" | "acli" => format!("acli rovodev run --restore {id}"),
        "cursor" | "cursor-agent" => format!("cursor-agent --resume {id}"),
        other => format!("{other} --resume {id}"),
    };

    match cwd.filter(|value| !value.trim().is_empty()) {
        Some(cwd) => vec![
            command.clone(),
            format!("cd {} && {command}", shell_quote(cwd)),
        ],
        None => vec![command],
    }
}

fn merge_codex_record(existing: &mut AgentSessionRecord, candidate: AgentSessionRecord) {
    merge_agent_session_record(existing, candidate);
}

fn merge_agent_session_record(existing: &mut AgentSessionRecord, candidate: AgentSessionRecord) {
    if existing.model.is_none() {
        existing.model = candidate.model.clone();
    }

    if existing.project_path.is_none() {
        existing.project_path = candidate.project_path.clone();
    }

    let candidate_is_newer = candidate
        .last_activity
        .as_ref()
        .is_some_and(|candidate_activity| {
            existing
                .last_activity
                .as_ref()
                .map_or(true, |existing_activity| {
                    candidate_activity > existing_activity
                })
        });

    if candidate_is_newer {
        existing.title = candidate.title;
        existing.model = candidate.model.or(existing.model.take());
        existing.project_path = candidate.project_path.or(existing.project_path.take());
        existing.last_activity = candidate.last_activity;
    }

    for command in candidate.resume_commands {
        if !existing.resume_commands.contains(&command) {
            existing.resume_commands.push(command);
        }
    }
}

fn model_from_value(value: &Value) -> Option<String> {
    optional_string(value.get("model"))
        .or_else(|| optional_string(value.get("model_slug")))
        .or_else(|| optional_string(value.get("modelSlug")))
        .or_else(|| optional_string(value.get("modelName")))
        .or_else(|| optional_string(value.get("model_name")))
        .or_else(|| optional_string(value.get("modelId")))
        .or_else(|| optional_string(value.get("model_id")))
}

fn optional_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
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

fn cmux_hook_session_files(root: &Path) -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return files;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(agent) = name.strip_suffix("-hook-sessions.json") else {
            continue;
        };
        if agent.is_empty() {
            continue;
        }

        files.push((agent.to_string(), path));
    }

    files
}

pub fn read_tail_utf8(path: &Path, max_bytes: usize) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(max_bytes as u64);
    file.seek(SeekFrom::Start(start))?;

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    if start == 0 {
        return Ok(text.into_owned());
    }

    Ok(text
        .split_once('\n')
        .map(|(_, tail)| tail.to_string())
        .unwrap_or_default())
}

pub fn read_head_utf8(path: &Path, max_bytes: usize) -> std::io::Result<String> {
    let file = fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(max_bytes as u64).read_to_end(&mut bytes)?;

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn modified_time(path: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
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
