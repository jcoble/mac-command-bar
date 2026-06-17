use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const CLAUDE_SESSION_FILE_LIMIT: usize = 512;
const CLAUDE_SESSION_TAIL_BYTES: usize = 256 * 1024;
const CODEX_SESSION_FILE_LIMIT: usize = 512;
const CODEX_SESSION_HEAD_BYTES: usize = 64 * 1024;
const CODEX_SESSION_TAIL_BYTES: usize = 256 * 1024;
const CMUX_SESSION_RESULT_HEADROOM: usize = 256;
const AGENT_SESSION_RESULT_LIMIT: usize =
    CODEX_SESSION_FILE_LIMIT + CLAUDE_SESSION_FILE_LIMIT + CMUX_SESSION_RESULT_HEADROOM;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionRecord {
    pub provider: String,
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub model: Option<String>,
    pub project_path: Option<String>,
    pub last_activity: Option<String>,
    pub resume_commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionDerivedMetadata {
    pub branch_hint: Option<String>,
    pub task_id: Option<String>,
    pub pull_request_hint: Option<String>,
    pub link_hint: Option<String>,
    pub source_label: String,
}

pub fn derive_agent_session_metadata(record: &AgentSessionRecord) -> AgentSessionDerivedMetadata {
    AgentSessionDerivedMetadata {
        branch_hint: first_agent_session_hint(record, branch_hint_from_text),
        task_id: first_agent_session_hint(record, task_id_from_text),
        pull_request_hint: first_agent_session_hint(record, pull_request_hint_from_text),
        link_hint: first_agent_session_hint(record, link_hint_from_text),
        source_label: agent_session_source_label(record),
    }
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
        if let Ok(contents) =
            read_head_and_tail_utf8(&file, CODEX_SESSION_HEAD_BYTES, CODEX_SESSION_TAIL_BYTES)
        {
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
    records.truncate(AGENT_SESSION_RESULT_LIMIT);
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
                description: None,
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
        match value.get("type").and_then(Value::as_str) {
            Some("session_meta") => {
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
                    description: None,
                    model: model_from_value(payload),
                    project_path: cwd,
                    last_activity,
                    resume_commands: vec![format!("codex resume {id}")],
                };

                if let Some(existing) = records.iter_mut().find(|candidate| {
                    candidate.provider == record.provider && candidate.id == record.id
                }) {
                    merge_codex_record(existing, record);
                } else {
                    records.push(record);
                }
            }
            Some("turn_context") => {
                let Some(payload) = value.get("payload") else {
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
                    .map(ToOwned::to_owned);
                update_latest_codex_record(
                    &mut records,
                    cwd,
                    None,
                    model_from_value(payload),
                    last_activity,
                );
            }
            Some("response_item") => {
                let cwd = codex_response_item_workdir(&value);
                let description = codex_response_item_description(&value);
                if cwd.is_none() && description.is_none() {
                    continue;
                };
                let last_activity = value
                    .get("timestamp")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                update_latest_codex_record(&mut records, cwd, description, None, last_activity);
            }
            _ => {}
        }
    }

    records
}

fn update_latest_codex_record(
    records: &mut [AgentSessionRecord],
    cwd: Option<String>,
    description: Option<String>,
    model: Option<String>,
    last_activity: Option<String>,
) {
    let Some(record) = records.last_mut() else {
        return;
    };

    let id = record.id.clone();
    let update = AgentSessionRecord {
        provider: "codex".to_string(),
        id: id.clone(),
        title: record.title.clone(),
        description,
        model,
        project_path: cwd,
        last_activity,
        resume_commands: vec![format!("codex resume {id}")],
    };
    merge_codex_record(record, update);
}

fn codex_response_item_workdir(value: &Value) -> Option<String> {
    let payload = value.get("payload")?;
    if payload.get("type").and_then(Value::as_str) != Some("function_call") {
        return None;
    }

    let arguments = payload.get("arguments").and_then(Value::as_str)?;
    let arguments = serde_json::from_str::<Value>(arguments).ok()?;
    optional_string(arguments.get("workdir"))
}

fn codex_response_item_description(value: &Value) -> Option<String> {
    let payload = value.get("payload")?;
    if payload.get("type").and_then(Value::as_str) != Some("message") {
        return None;
    }
    if payload.get("role").and_then(Value::as_str) != Some("user") {
        return None;
    }

    payload
        .get("content")
        .and_then(value_to_text)
        .map(|text| compact_text(&text, 140))
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
            let last_activity = timestampish_string(value.get("updatedAt"))
                .or_else(|| timestampish_string(value.get("startedAt")))
                .or_else(|| {
                    timestampish_string(
                        value
                            .get("launchCommand")
                            .and_then(|launch| launch.get("capturedAt")),
                    )
                });
            let status = value
                .get("runtimeStatus")
                .and_then(Value::as_str)
                .or_else(|| value.get("agentLifecycle").and_then(Value::as_str))
                .filter(|value| !value.trim().is_empty());
            let title = cmux_session_title(&agent, value, status, cwd.as_deref());

            Some(AgentSessionRecord {
                provider: format!("cmux-{agent}"),
                id: id.to_string(),
                title,
                description: cmux_session_description(value),
                model: model_from_value(value),
                project_path: cwd.clone(),
                last_activity,
                resume_commands: cmux_resume_commands(&agent, id, cwd.as_deref()),
            })
        })
        .collect()
}

pub fn parse_claude_jsonl(input: &str, project_path: &str) -> Vec<AgentSessionRecord> {
    let mut records: Vec<AgentSessionRecord> = Vec::new();

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
        let record = AgentSessionRecord {
            provider: "claude".to_string(),
            id: id.to_string(),
            title,
            description: claude_session_description(&value),
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
        };

        if let Some(existing) = records
            .iter_mut()
            .find(|candidate| candidate.provider == record.provider && candidate.id == record.id)
        {
            merge_agent_session_record(existing, record);
        } else {
            records.push(record);
        }
    }

    records
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
    if existing.description.is_none() {
        existing.description = candidate.description.clone();
    }

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
        existing.description = candidate.description.or(existing.description.take());
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

fn cmux_session_title(
    agent: &str,
    value: &Value,
    status: Option<&str>,
    cwd: Option<&str>,
) -> String {
    let agent_label = agent_display_label(agent);
    let detail = optional_string(value.get("title"))
        .or_else(|| optional_string(value.get("name")))
        .or_else(|| optional_string(value.get("threadName")))
        .or_else(|| optional_string(value.get("conversationTitle")))
        .or_else(|| optional_string(value.get("taskTitle")))
        .or_else(|| optional_string(value.get("lastSubtitle")))
        .or_else(|| {
            status
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .or_else(|| cwd.and_then(path_display_name));

    match detail {
        Some(detail) => format!("{agent_label} · {}", compact_text(&detail, 72)),
        None => format!("{agent_label} session"),
    }
}

fn cmux_session_description(value: &Value) -> Option<String> {
    optional_string(value.get("lastBody"))
        .or_else(|| optional_string(value.get("lastMessage")))
        .or_else(|| optional_string(value.get("summary")))
        .map(|text| compact_text(&text, 140))
}

fn claude_session_description(value: &Value) -> Option<String> {
    value
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(value_to_text)
        .or_else(|| value.get("summary").and_then(value_to_text))
        .map(|text| compact_text(&text, 140))
}

fn agent_display_label(agent: &str) -> String {
    match agent {
        "codex" => "Codex".to_string(),
        "claude" => "Claude".to_string(),
        "gemini" => "Gemini".to_string(),
        "opencode" => "OpenCode".to_string(),
        "cursor" | "cursor-agent" => "Cursor".to_string(),
        "antigravity" | "agy" => "Antigravity".to_string(),
        "rovo" | "acli" => "Rovo".to_string(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => "Agent".to_string(),
            }
        }
    }
}

fn compact_text(value: &str, max_chars: usize) -> String {
    let trimmed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.chars().count() <= max_chars {
        return trimmed;
    }

    let mut compacted = trimmed
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    compacted.push('…');
    compacted
}

fn path_display_name(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn first_agent_session_hint(
    record: &AgentSessionRecord,
    mut derive: impl FnMut(&str) -> Option<String>,
) -> Option<String> {
    derive(&record.title)
        .or_else(|| record.project_path.as_deref().and_then(&mut derive))
        .or_else(|| {
            record
                .resume_commands
                .iter()
                .find_map(|command| derive(command))
        })
}

fn agent_session_source_label(record: &AgentSessionRecord) -> String {
    let provider = agent_session_provider_label(&record.provider);
    match record
        .project_path
        .as_deref()
        .and_then(path_display_name)
        .or_else(|| {
            let title = record.title.trim();
            (!title.is_empty()).then(|| compact_text(title, 48))
        }) {
        Some(detail) => format!("{provider} · {detail}"),
        None => provider,
    }
}

fn agent_session_provider_label(provider: &str) -> String {
    let provider = provider.trim();
    if let Some(agent) = provider.strip_prefix("cmux-") {
        return format!("CMUX {}", agent_display_label(agent));
    }
    agent_display_label(provider)
}

fn branch_hint_from_text(text: &str) -> Option<String> {
    let lower_text = text.to_ascii_lowercase();
    for marker in ["branch:", "branch=", "branch "] {
        let mut search_start = 0;
        while let Some(offset) = lower_text[search_start..].find(marker) {
            let index = search_start + offset;
            if index > 0 {
                let previous = lower_text.as_bytes()[index - 1] as char;
                if previous.is_ascii_alphanumeric() {
                    search_start = index + marker.len();
                    continue;
                }
            }

            let suffix = &text[index + marker.len()..];
            if let Some(branch) = git_ref_token_from_text(suffix) {
                return Some(branch);
            }
            search_start = index + marker.len();
        }
    }

    None
}

fn git_ref_token_from_text(text: &str) -> Option<String> {
    let token: String = text
        .trim_start_matches(|character: char| {
            character.is_whitespace() || matches!(character, '`' | '"' | '\'')
        })
        .chars()
        .take_while(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | '/')
        })
        .collect();
    if token.is_empty()
        || token
            .chars()
            .all(|character| matches!(character, '.' | '_' | '-' | '/'))
    {
        return None;
    }

    Some(token)
}

fn task_id_from_text(text: &str) -> Option<String> {
    let lower_text = text.to_ascii_lowercase();
    for (index, _) in lower_text.match_indices("tsk") {
        if index > 0 {
            let previous = lower_text.as_bytes()[index - 1] as char;
            if previous.is_ascii_alphanumeric() {
                continue;
            }
        }

        let suffix =
            lower_text[index + 3..].trim_start_matches(['-', '_', '/', '#', '[', ' ', ':']);
        let digits: String = suffix
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            return Some(format!("TSK-{digits}"));
        }
    }

    None
}

fn pull_request_hint_from_text(text: &str) -> Option<String> {
    pull_request_number_after_marker(text, "pull request")
        .or_else(|| pull_request_number_after_marker(text, "pr"))
        .or_else(|| {
            link_hint_from_text(text)
                .as_deref()
                .and_then(github_pull_request_number_from_url)
        })
        .map(|number| format!("PR #{number}"))
}

fn pull_request_number_after_marker(text: &str, marker: &str) -> Option<String> {
    let lower_text = text.to_ascii_lowercase();
    let mut search_start = 0;
    while let Some(offset) = lower_text[search_start..].find(marker) {
        let index = search_start + offset;
        if index > 0 {
            let previous = lower_text.as_bytes()[index - 1] as char;
            if previous.is_ascii_alphanumeric() {
                search_start = index + marker.len();
                continue;
            }
        }

        let suffix = &text[index + marker.len()..];
        let suffix = suffix.trim_start_matches([' ', '#', '-', ':']);
        let digits: String = suffix
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            return Some(digits);
        }

        search_start = index + marker.len();
    }

    None
}

fn link_hint_from_text(text: &str) -> Option<String> {
    for scheme in ["https://", "http://"] {
        let mut search_start = 0;
        while let Some(offset) = text[search_start..].find(scheme) {
            let index = search_start + offset;
            let suffix = &text[index..];
            let end = suffix
                .char_indices()
                .find_map(|(index, character)| character.is_whitespace().then_some(index))
                .unwrap_or(suffix.len());
            let link = suffix[..end].trim_end_matches(['.', ',', ';', ':', ')', ']', '}', '"']);
            if !link.is_empty() {
                return Some(link.to_string());
            }

            search_start = index + scheme.len();
        }
    }

    None
}

fn github_pull_request_number_from_url(url: &str) -> Option<String> {
    if !url.to_ascii_lowercase().contains("github.com/") {
        return None;
    }

    let lower_url = url.to_ascii_lowercase();
    let index = lower_url.find("/pull/")? + "/pull/".len();
    let digits: String = url[index..]
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();
    (!digits.is_empty()).then_some(digits)
}

fn optional_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn timestampish_string(value: Option<&Value>) -> Option<String> {
    let value = value?;
    match value {
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else if let Ok(number) = trimmed.parse::<f64>() {
                unix_timestamp_number_to_iso(number).or_else(|| Some(trimmed.to_string()))
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Number(number) => number.as_f64().and_then(unix_timestamp_number_to_iso),
        _ => None,
    }
}

fn unix_timestamp_number_to_iso(value: f64) -> Option<String> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }

    let seconds_value = if value >= 1_000_000_000_000.0 {
        value / 1000.0
    } else {
        value
    };
    let mut seconds = seconds_value.floor() as i64;
    let mut millis = ((seconds_value - seconds as f64) * 1000.0).floor() as u32;
    if millis >= 1000 {
        seconds += 1;
        millis = 0;
    }
    Some(unix_seconds_to_iso8601(seconds, millis))
}

fn unix_seconds_to_iso8601(seconds: i64, millis: u32) -> String {
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;

    if millis > 0 {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
    } else {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
    }
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };

    (year, month as u32, day as u32)
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

fn read_head_and_tail_utf8(
    path: &Path,
    head_bytes: usize,
    tail_bytes: usize,
) -> std::io::Result<String> {
    let head = read_head_utf8(path, head_bytes)?;
    let tail = read_tail_utf8(path, tail_bytes)?;
    if tail.is_empty() || head.ends_with(&tail) {
        return Ok(head);
    }

    Ok(format!("{}\n{}", head.trim_end_matches('\n'), tail))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn session(title: &str, project_path: Option<&str>) -> AgentSessionRecord {
        AgentSessionRecord {
            provider: "codex".to_string(),
            id: "019e".to_string(),
            title: title.to_string(),
            description: None,
            model: None,
            project_path: project_path.map(ToOwned::to_owned),
            last_activity: None,
            resume_commands: vec!["codex resume 019e".to_string()],
        }
    }

    #[test]
    fn derived_metadata_extracts_explicit_agent_session_hints() {
        let record = session(
            "TSK-127 branch cdx/tsk-127-agent-session-metadata PR #42 https://github.com/acme/mac-command-bar/pull/42",
            Some("/Users/blackcolours/dev/work/mac-command-bar"),
        );

        let metadata = derive_agent_session_metadata(&record);

        assert_eq!(metadata.task_id.as_deref(), Some("TSK-127"));
        assert_eq!(
            metadata.branch_hint.as_deref(),
            Some("cdx/tsk-127-agent-session-metadata")
        );
        assert_eq!(metadata.pull_request_hint.as_deref(), Some("PR #42"));
        assert_eq!(
            metadata.link_hint.as_deref(),
            Some("https://github.com/acme/mac-command-bar/pull/42")
        );
        assert_eq!(metadata.source_label, "Codex · mac-command-bar");
    }

    #[test]
    fn derived_metadata_uses_path_task_ids_and_keeps_unclear_hints_empty() {
        let record = session(
            "Claude session",
            Some("/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-128-runtime-audit"),
        );

        let metadata = derive_agent_session_metadata(&record);

        assert_eq!(metadata.task_id.as_deref(), Some("TSK-128"));
        assert_eq!(metadata.branch_hint, None);
        assert_eq!(metadata.pull_request_hint, None);
        assert_eq!(metadata.link_hint, None);
        assert_eq!(metadata.source_label, "Codex · tsk-128-runtime-audit");
    }

    #[test]
    fn agent_session_record_json_shape_stays_unchanged() {
        let record = session("TSK-127 metadata", Some("/repo"));

        let value = serde_json::to_value(&record).unwrap();

        assert_eq!(value.get("taskId"), None);
        assert_eq!(value.get("branchHint"), None);
        assert_eq!(value.get("pullRequestHint"), None);
        assert_eq!(value.get("linkHint"), None);
        assert_eq!(value.get("sourceLabel"), None);
        assert_eq!(
            value.get("projectPath").and_then(Value::as_str),
            Some("/repo")
        );
    }
}
