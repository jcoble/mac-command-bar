use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const CLAUDE_GENERIC_SESSION_TITLE: &str = "Claude session";
const CLAUDE_SESSION_FILE_LIMIT: usize = 512;
const CLAUDE_SESSION_TAIL_BYTES: usize = 256 * 1024;
const CODEX_GENERIC_SESSION_TITLE: &str = "Codex session";
const CODEX_UNTITLED_INDEX_TITLE: &str = "Untitled Codex session";
const CODEX_SESSION_FILE_LIMIT: usize = 512;
/// The first user message of a Codex session sits behind the opening metadata
/// record, which carries the whole system prompt and can run past 40 KB. On this
/// machine 256 KB reaches the first typed prompt in 59 of 67 top-level sessions;
/// the rest fall back to whatever the tail window offers.
const CODEX_SESSION_HEAD_BYTES: usize = 256 * 1024;
const CODEX_SESSION_TAIL_BYTES: usize = 256 * 1024;
/// Enough to hold the opening `session_meta` line whole. Measured across 687
/// rollout files: median 27 KB, largest 44 KB.
const CODEX_SESSION_META_PROBE_BYTES: usize = 64 * 1024;
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
    // Drop sub-agent threads BEFORE the file budget, for the same reason the
    // Claude path drops its subagents first: they outnumber the user's own
    // sessions nine to one here, so a budget applied first would evict the
    // sessions the rail exists to show.
    codex_files.retain(|file| !codex_rollout_file_is_subagent_thread(file));
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
    // Drop subagent transcripts BEFORE the file budget is applied: they
    // outnumber real sessions on a busy machine and would otherwise evict them.
    files.retain(|file| !is_claude_subagent_transcript_path(file));
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
                .unwrap_or(CODEX_UNTITLED_INDEX_TITLE)
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
    let mut first_prompts: HashMap<String, String> = HashMap::new();

    for value in input
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
    {
        match value.get("type").and_then(Value::as_str) {
            Some("session_meta") => {
                let Some(payload) = value.get("payload") else {
                    continue;
                };
                if is_codex_subagent_meta(payload) {
                    return Vec::new();
                }
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
                    title: CODEX_GENERIC_SESSION_TITLE.to_string(),
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
                if let (Some(record), Some(prompt)) =
                    (records.last(), codex_user_prompt_text(&value))
                {
                    first_prompts.entry(record.id.clone()).or_insert(prompt);
                }

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

    for record in records.iter_mut() {
        record.title = codex_display_title(
            &record.title,
            first_prompts.get(&record.id).map(String::as_str),
        );
    }

    records
}

/// Codex spawns its own helper threads and writes each one as a rollout file
/// next to the user's, so nine of every ten files under `~/.codex/sessions` are
/// threads nobody opened. The opening `session_meta` record says which it is:
/// a helper carries `thread_source: "subagent"`, and/or a `source` object whose
/// only key is `subagent` (a session the user started has `source` as a plain
/// string — `cli`, `vscode`, `exec`).
///
/// Verified 2026-07-28 across 687 rollout files on this machine: 619 helper
/// threads, 68 sessions the user started, and the two markers disagreed on a
/// single file — so both are checked and either one is enough.
///
/// A file with neither marker is kept. Older Codex versions predate the field
/// (15 files here), and losing one of the user's sessions is worse than listing
/// a helper thread.
fn is_codex_subagent_meta(payload: &Value) -> bool {
    if payload.get("thread_source").and_then(Value::as_str) == Some("subagent") {
        return true;
    }

    payload
        .get("source")
        .and_then(Value::as_object)
        .is_some_and(|source| source.contains_key("subagent"))
}

/// Same question asked of a file instead of a parsed record, so the scan can
/// skip helper threads before it spends its read budget on them. The opening
/// line is the metadata record; anything we cannot read or parse is kept.
fn codex_rollout_file_is_subagent_thread(path: &Path) -> bool {
    let Ok(head) = read_head_utf8(path, CODEX_SESSION_META_PROBE_BYTES) else {
        return false;
    };

    codex_rollout_head_is_subagent_thread(&head)
}

pub fn codex_rollout_head_is_subagent_thread(head: &str) -> bool {
    let Some(line) = head.lines().next() else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return false;
    };
    if value.get("type").and_then(Value::as_str) != Some("session_meta") {
        return false;
    }

    value.get("payload").is_some_and(is_codex_subagent_meta)
}

/// The first thing the user actually typed. Codex opens every session with
/// machine-written turns sent as the user — the repository's `AGENTS.md`, the
/// environment block, the plugin list, the file list — and each has a shape we
/// can recognise, so the title comes from the first turn that is none of them.
fn codex_user_prompt_text(value: &Value) -> Option<String> {
    let payload = value.get("payload")?;
    if payload.get("type").and_then(Value::as_str) != Some("message") {
        return None;
    }
    if payload.get("role").and_then(Value::as_str) != Some("user") {
        return None;
    }

    let text = payload.get("content").and_then(value_to_text)?;
    let text = text.trim();
    if text.is_empty() || CODEX_INJECTED_PROMPT_PREFIXES.iter().any(|prefix| text.starts_with(prefix)) {
        return None;
    }

    Some(compact_text(text, 140))
}

/// Openings that mean Codex wrote this turn, not the user. `<` covers every
/// tagged block it injects (`<environment_context>`, `<recommended_plugins>`,
/// `<user_shell_command>`); the two headings are the repository instructions
/// and the attached-file list.
const CODEX_INJECTED_PROMPT_PREFIXES: &[&str] = &[
    "<",
    "# AGENTS.md instructions",
    "# Files mentioned by the user",
];

/// A Codex rollout file carries no title of its own, so a rail built from these
/// files alone reads as hundreds of rows all saying "Codex session". The first
/// typed prompt is the only description of the session in the file, so it
/// becomes the title; the generic label survives only when the scanned window
/// held no typed prompt at all.
fn codex_display_title(derived: &str, first_prompt: Option<&str>) -> String {
    if derived != CODEX_GENERIC_SESSION_TITLE {
        return derived.to_string();
    }

    match first_prompt {
        Some(prompt) => compact_text(prompt, 60),
        None => CODEX_GENERIC_SESSION_TITLE.to_string(),
    }
}

fn is_generic_codex_title(title: &str) -> bool {
    title == CODEX_GENERIC_SESSION_TITLE || title == CODEX_UNTITLED_INDEX_TITLE
}

/// One Codex session can be described twice: the index file may name it, and
/// the rollout file offers the opening prompt. Whichever record looks newer is
/// beside the point — a placeholder must never displace a real title, and the
/// index name describes the whole session where the prompt only opens it, so
/// the title already in hand wins when both are real.
fn better_codex_title(existing: String, candidate: String) -> String {
    if !is_generic_codex_title(&existing) {
        return existing;
    }
    if !is_generic_codex_title(&candidate) {
        return candidate;
    }

    existing
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
    let mut ai_titles: HashMap<String, String> = HashMap::new();
    let mut first_prompts: HashMap<String, String> = HashMap::new();

    for value in input
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
    {
        // One such entry condemns the whole transcript: these files are what
        // they are end to end, so anything already collected from one is an
        // agent's turn, not a session the user can resume. Both checks answer
        // the same question — "is this a top-level session?" — from different
        // evidence: a subagent's sidechain flag, or a helper's entrypoint.
        if is_claude_sidechain_entry(&value) || is_claude_agent_launched_entry(&value) {
            return Vec::new();
        }

        if let Some((session, title)) = claude_ai_title(&value) {
            ai_titles.insert(session, title); // a later line carries the newer title
        }
        if let Some((session, prompt)) = claude_user_prompt_text(&value) {
            first_prompts.entry(session).or_insert(prompt);
        }

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

        let title = title_from_claude_message(&value)
            .unwrap_or_else(|| CLAUDE_GENERIC_SESSION_TITLE.to_string());
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

    for record in records.iter_mut() {
        record.title = claude_display_title(
            &record.title,
            ai_titles.get(&record.id).map(String::as_str),
            first_prompts.get(&record.id).map(String::as_str),
            record.project_path.as_deref(),
        );
    }

    records
}

/// A Claude Code SUBAGENT transcript is stored as its own `.jsonl` and is never
/// resumable — `claude --resume <id>` on one is meaningless — so it must never
/// reach the rail. Two independent discriminators, because each covers what the
/// other cannot: the path is free and also keeps subagents out of the scan's
/// file budget, the content is authoritative and still catches a flat layout.
///
/// Today Claude Code writes them under `<project>/<session>/subagents/`.
fn is_claude_subagent_transcript_path(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "subagents")
}

/// Every entry of a subagent transcript carries `"isSidechain": true`; every
/// entry of a real session carries `false`. Verified across 2180 transcripts on
/// a live machine: 1177 sidechain files, all of them pure, zero mixed files.
fn is_claude_sidechain_entry(value: &Value) -> bool {
    value.get("isSidechain").and_then(Value::as_bool) == Some(true)
}

/// Entrypoint values that mean "a program started this run, not the user".
/// Prefixes, so a future `sdk-node` is covered without a code change; adding a
/// new family is a one-line edit here.
const AGENT_LAUNCH_ENTRYPOINT_PREFIXES: &[&str] = &["sdk"];

/// Helper-agent transcripts — a team lead's dispatched teammates, and any other
/// SDK-driven run — land flat in the same project directory as the user's own
/// sessions, with `isSidechain: false`, `userType: "external"` and no agent name
/// anywhere, so neither discriminator above sees them. What they do carry is how
/// they were launched: every `user`, `assistant` and `attachment` record repeats
/// an `entrypoint`, and a programmatic run is always `sdk-…` (`sdk-cli`,
/// `sdk-py`) where a session the user typed into is `cli` or `claude-vscode`.
///
/// Verified 2026-07-28 across 1073 transcripts on this machine: every one of the
/// 888 dispatched helper runs was `sdk-…`, every human session was `cli` or
/// `claude-vscode`, and no transcript ever mixed the two families. 1072 of the
/// 1073 carry the field within the 256 KB tail this scanner reads.
///
/// This deliberately replaces the "does the first message read like a dispatch
/// prompt?" idea: the user's real sessions often open with pasted logs and
/// instruction-shaped text, and the scanner reads a tail window that usually
/// does not even contain the first message.
fn is_agent_launch_entrypoint(entrypoint: &str) -> bool {
    AGENT_LAUNCH_ENTRYPOINT_PREFIXES
        .iter()
        .any(|prefix| entrypoint.starts_with(prefix))
}

/// Reads that entrypoint off one transcript line. A line without the field says
/// nothing either way — older transcripts predate it — so it is not evidence of
/// a helper and the transcript is kept.
fn is_claude_agent_launched_entry(value: &Value) -> bool {
    value
        .get("entrypoint")
        .and_then(Value::as_str)
        .is_some_and(is_agent_launch_entrypoint)
}

/// Claude Code records its own generated session title on a `type: "ai-title"`
/// line. It is the best title available — it describes the whole session rather
/// than whichever message happened to be last — so it wins outright.
fn claude_ai_title(value: &Value) -> Option<(String, String)> {
    if optional_string(value.get("type"))? != "ai-title" {
        return None;
    }

    Some((
        optional_string(value.get("sessionId"))?,
        optional_string(value.get("aiTitle"))?,
    ))
}

/// The first genuine user prompt in the scanned window. Tool results are
/// `type: "user"` too, so plain `content` text is required and `tool_result`
/// items are dropped; slash-command wrappers (`<command-name>…`) and the resume
/// caveat are skipped because neither says what the session is about.
fn claude_user_prompt_text(value: &Value) -> Option<(String, String)> {
    if optional_string(value.get("type"))? != "user" {
        return None;
    }
    if value.get("isMeta").and_then(Value::as_bool) == Some(true) {
        return None;
    }

    let id = optional_string(value.get("sessionId"))?;
    let text = match value.get("message")?.get("content")? {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .filter(|item| optional_string(item.get("type")).as_deref() == Some("text"))
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" "),
        _ => return None,
    };

    let text = text.trim();
    if text.is_empty() || text.starts_with('<') || text.starts_with("Caveat:") {
        return None;
    }

    Some((id, text.to_string()))
}

/// Title preference for a Claude row: the session's own AI title, then whatever
/// a message yielded, then `<project folder> — <first user prompt>`. The generic
/// label survives only when nothing else exists — a rail full of "Claude
/// session" rows tells the user nothing about which session to resume.
fn claude_display_title(
    derived: &str,
    ai_title: Option<&str>,
    first_prompt: Option<&str>,
    project_path: Option<&str>,
) -> String {
    if let Some(title) = ai_title {
        return compact_text(title, 80);
    }
    if derived != CLAUDE_GENERIC_SESSION_TITLE {
        return derived.to_string();
    }

    match (first_prompt, project_path.and_then(path_display_name)) {
        (Some(prompt), Some(folder)) => compact_text(&format!("{folder} — {prompt}"), 60),
        (Some(prompt), None) => compact_text(prompt, 60),
        (None, _) => CLAUDE_GENERIC_SESSION_TITLE.to_string(),
    }
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
    let previous_title = existing.title.clone();
    let candidate_title = candidate.title.clone();
    merge_agent_session_record(existing, candidate);
    existing.title = better_codex_title(previous_title, candidate_title);
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

    /// A real Claude session: a user prompt, then an assistant turn that is
    /// nothing but a `tool_use`. The tool_use line is NEWER, so the merge takes
    /// its (empty) title — which is exactly why real rails filled up with
    /// "Claude session".
    const REAL_SESSION_JSONL: &str = concat!(
        r#"{"type":"user","isSidechain":false,"sessionId":"S1","cwd":"/Users/dev/work/mac-command-bar","timestamp":"2026-07-28T09:00:00Z","message":{"role":"user","content":"Fix the resume rail"}}"#,
        "\n",
        r#"{"type":"user","isSidechain":false,"sessionId":"S1","timestamp":"2026-07-28T09:01:00Z","message":{"role":"user","content":[{"type":"tool_result","content":"File does not exist."}]}}"#,
        "\n",
        r#"{"type":"assistant","isSidechain":false,"sessionId":"S1","timestamp":"2026-07-28T09:02:00Z","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Read","input":{}}]}}"#,
    );

    /// A subagent transcript: the Task tool's prompt and its structured reply,
    /// every line `isSidechain: true`, and a `sessionId` that would otherwise
    /// mint a resumable-looking row.
    const SUBAGENT_JSONL: &str = concat!(
        r#"{"parentUuid":null,"isSidechain":true,"agentId":"a0a5","type":"user","sessionId":"S2","cwd":"/Users/dev/work/mac-command-bar","timestamp":"2026-07-28T10:00:00Z","message":{"role":"user","content":"You are implementing Task 6 of the Slice 1 plan."}}"#,
        "\n",
        r#"{"parentUuid":"1","isSidechain":true,"agentId":"a0a5","type":"assistant","sessionId":"S2","timestamp":"2026-07-28T10:01:00Z","message":{"role":"assistant","content":[{"type":"text","text":"Structured output provided successfully"}]}}"#,
    );

    /// A helper-agent transcript as Claude Code actually writes it: flat in the
    /// project directory, `isSidechain: false`, `userType: "external"`, no agent
    /// name anywhere — identical to a real session except for the entrypoint
    /// that launched it.
    const HELPER_AGENT_JSONL: &str = concat!(
        r#"{"type":"user","isSidechain":false,"userType":"external","entrypoint":"sdk-cli","promptSource":"sdk","sessionId":"S4","cwd":"/Users/dev/work/mac-command-bar","timestamp":"2026-07-28T11:00:00Z","message":{"role":"user","content":"You are implementing Task 1 of the v2-base plan."}}"#,
        "\n",
        r#"{"type":"assistant","isSidechain":false,"userType":"external","entrypoint":"sdk-cli","sessionId":"S4","timestamp":"2026-07-28T11:01:00Z","message":{"role":"assistant","content":[{"type":"text","text":"Reading the brief."}]}}"#,
    );

    /// A REAL session that opens with pasted log output. The user does this
    /// often, and it is why the filter reads the entrypoint rather than
    /// guessing from the shape of the first message: nothing about this text
    /// distinguishes it from a machine-authored dispatch prompt.
    const REAL_SESSION_STARTING_WITH_A_PASTED_LOG_JSONL: &str = concat!(
        r#"{"type":"user","isSidechain":false,"userType":"external","entrypoint":"cli","promptSource":"typed","sessionId":"S5","cwd":"/Users/dev/work/mac-command-bar","timestamp":"2026-07-28T12:00:00Z","message":{"role":"user","content":"You are seeing this in the console:\n[vite] hmr update /src/routes/next/+page.svelte\nERROR  Cannot read properties of undefined (reading 'api')\n    at mount (chunk-QK6X.js:14:9)\nReview this change and tell me why. Your final message should name the file."}}"#,
        "\n",
        r#"{"type":"assistant","isSidechain":false,"entrypoint":"cli","sessionId":"S5","timestamp":"2026-07-28T12:01:00Z","message":{"role":"assistant","content":[{"type":"text","text":"That mount is running before the store exists."}]}}"#,
    );

    /// A thread Codex spawned for itself. Both markers are present, as they are
    /// on 618 of the 619 helper threads on this machine.
    const CODEX_SUBAGENT_ROLLOUT_JSONL: &str = concat!(
        r#"{"timestamp":"2026-07-28T17:24:36.972Z","type":"session_meta","payload":{"id":"019fa9c1","parent_thread_id":"019fa964","cwd":"/Users/dev/work/rental-management","originator":"codex-tui","thread_source":"subagent","agent_role":"executor","source":{"subagent":{"parent_thread_id":"019fa964","depth":1}}}}"#,
        "\n",
        r#"{"timestamp":"2026-07-28T17:25:00.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Implement the year simulation runner."}]}}"#,
    );

    /// A session the user started in the terminal. `source` is a plain string,
    /// and the first two turns are the ones Codex writes for itself before the
    /// user has typed anything.
    const CODEX_TOP_LEVEL_ROLLOUT_JSONL: &str = concat!(
        r#"{"timestamp":"2026-07-28T16:42:17.000Z","type":"session_meta","payload":{"id":"019fa964","cwd":"/Users/dev/work/rental-management","originator":"codex-tui","thread_source":"user","source":"cli"}}"#,
        "\n",
        r##"{"timestamp":"2026-07-28T16:42:18.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"# AGENTS.md instructions for /Users/dev/work/rental-management\n\n<INSTRUCTIONS>\nEvery reply MUST open with a plain-English summary.\n</INSTRUCTIONS>"}]}}"##,
        "\n",
        r#"{"timestamp":"2026-07-28T16:42:19.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"<environment_context>\n  <cwd>/Users/dev/work/rental-management</cwd>\n</environment_context>"}]}}"#,
        "\n",
        r#"{"timestamp":"2026-07-28T16:42:30.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Please plan out an entire year of scans and entries for the 2027 simulation."}]}}"#,
    );

    /// An older Codex build that wrote no thread marker at all.
    const CODEX_UNMARKED_ROLLOUT_JSONL: &str = concat!(
        r#"{"timestamp":"2026-04-14T13:53:07.000Z","type":"session_meta","payload":{"id":"019d8d20","cwd":"/Users/dev/work/EdiPlatform","originator":"codex_cli_rs","source":"unknown"}}"#,
        "\n",
        r#"{"timestamp":"2026-04-14T13:54:00.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Help me trace where the 810 mapping loses the invoice date."}]}}"#,
    );

    #[test]
    fn codex_scan_excludes_threads_codex_spawned_for_itself() {
        // Either marker on its own is enough — they disagreed on one real file.
        assert!(codex_rollout_head_is_subagent_thread(
            r#"{"type":"session_meta","payload":{"thread_source":"subagent","source":"vscode"}}"#
        ));
        assert!(codex_rollout_head_is_subagent_thread(
            r#"{"type":"session_meta","payload":{"source":{"subagent":{"depth":1}}}}"#
        ));

        // A session the user started: `source` is a plain string, not an object.
        assert!(!codex_rollout_head_is_subagent_thread(
            r#"{"type":"session_meta","payload":{"thread_source":"user","source":"cli"}}"#
        ));
        // No marker, unreadable, or not a metadata line at all: keep the file.
        assert!(!codex_rollout_head_is_subagent_thread(
            r#"{"type":"session_meta","payload":{"source":"unknown"}}"#
        ));
        assert!(!codex_rollout_head_is_subagent_thread("{\"type\":\"sessi"));
        assert!(!codex_rollout_head_is_subagent_thread(""));

        assert_eq!(
            parse_codex_rollout_jsonl(CODEX_SUBAGENT_ROLLOUT_JSONL),
            Vec::new()
        );

        let records = parse_codex_rollout_jsonl(CODEX_TOP_LEVEL_ROLLOUT_JSONL);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "019fa964");

        // Older rollouts predate the marker. Keep them: losing a session the
        // user wants back is worse than listing a helper thread.
        let records = parse_codex_rollout_jsonl(CODEX_UNMARKED_ROLLOUT_JSONL);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "019d8d20");
    }

    #[test]
    fn codex_titles_come_from_the_first_prompt_the_user_typed() {
        let records = parse_codex_rollout_jsonl(CODEX_TOP_LEVEL_ROLLOUT_JSONL);
        assert_eq!(
            records[0].title,
            "Please plan out an entire year of scans and entries for the…"
        );

        // The repository instructions and the environment block are written by
        // Codex, not the user, so neither may become the title.
        assert!(!records[0].title.contains("AGENTS.md"));
        assert!(!records[0].title.contains("environment_context"));

        // Nothing typed inside the scanned window leaves the generic label.
        let meta_only = r#"{"type":"session_meta","payload":{"id":"019fa964","cwd":"/Users/dev","source":"cli"}}"#;
        assert_eq!(
            parse_codex_rollout_jsonl(meta_only)[0].title,
            CODEX_GENERIC_SESSION_TITLE
        );

        // The index file names some sessions. That name must survive a merge
        // with the rollout file whichever record carries the later timestamp.
        let index = parse_codex_index_jsonl(
            r#"{"id":"019fa964","thread_name":"Year simulation planning","updated_at":"2026-07-28T16:00:00.000000Z"}"#,
        );
        let merged =
            merge_codex_session_metadata(index, parse_codex_rollout_jsonl(CODEX_TOP_LEVEL_ROLLOUT_JSONL));
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].title, "Year simulation planning");

        // And an unnamed index row must not overwrite a real rollout title.
        let index = parse_codex_index_jsonl(
            r#"{"id":"019fa964","updated_at":"2026-07-29T16:00:00.000000Z"}"#,
        );
        let merged =
            merge_codex_session_metadata(index, parse_codex_rollout_jsonl(CODEX_TOP_LEVEL_ROLLOUT_JSONL));
        assert_eq!(
            merged[0].title,
            "Please plan out an entire year of scans and entries for the…"
        );
    }

    #[test]
    fn claude_scan_excludes_helper_agent_transcripts() {
        // The entrypoint is the whole rule: `sdk-*` launched it programmatically.
        assert!(is_agent_launch_entrypoint("sdk-cli"));
        assert!(is_agent_launch_entrypoint("sdk-py"));
        assert!(!is_agent_launch_entrypoint("cli"));
        assert!(!is_agent_launch_entrypoint("claude-vscode"));
        assert!(!is_agent_launch_entrypoint(""));

        assert_eq!(
            parse_claude_jsonl(HELPER_AGENT_JSONL, "/Users/dev/work/mac-command-bar"),
            Vec::new()
        );

        // A real session that opens with pasted log output and the very phrases
        // a dispatch prompt uses ("You are ", "Review this change", "Your final
        // message") must survive. Excluding one of the user's own sessions is
        // strictly worse than listing a helper.
        let records = parse_claude_jsonl(
            REAL_SESSION_STARTING_WITH_A_PASTED_LOG_JSONL,
            "/Users/dev/work/mac-command-bar",
        );
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "S5");

        // Older transcripts predate the field. Keep them: a missing entrypoint
        // is not evidence of a helper, and the cost of guessing wrong is losing
        // a session the user wanted.
        let records = parse_claude_jsonl(REAL_SESSION_JSONL, "/Users/dev/work/mac-command-bar");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "S1");
    }

    #[test]
    fn claude_scan_excludes_subagent_sidechain_transcripts() {
        // Path discriminator — free, and it also keeps subagents from evicting
        // real sessions from the scan's file budget.
        assert!(is_claude_subagent_transcript_path(Path::new(
            "/Users/dev/.claude/projects/-Users-dev/71dd/subagents/agent-a0a5.jsonl"
        )));
        assert!(is_claude_subagent_transcript_path(Path::new(
            "/Users/dev/.claude/projects/-Users-dev/71dd/subagents/workflows/wf_02/agent-a2.jsonl"
        )));
        assert!(!is_claude_subagent_transcript_path(Path::new(
            "/Users/dev/.claude/projects/-Users-dev/71dd4d8f.jsonl"
        )));

        // Content discriminator — authoritative, and it still catches a
        // subagent transcript written flat into the project directory.
        assert_eq!(
            parse_claude_jsonl(SUBAGENT_JSONL, "/Users/dev/work/mac-command-bar"),
            Vec::new()
        );

        let records = parse_claude_jsonl(REAL_SESSION_JSONL, "/Users/dev/work/mac-command-bar");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "S1");
    }

    #[test]
    fn claude_titles_fall_back_to_project_folder_and_first_prompt() {
        let records = parse_claude_jsonl(REAL_SESSION_JSONL, "/Users/dev/work/mac-command-bar");

        // Without the fallback this row reads "Claude session": the newest line
        // is a tool_use with no text, and a tool_result is not a user prompt.
        assert_eq!(records[0].title, "mac-command-bar — Fix the resume rail");
    }

    #[test]
    fn claude_ai_title_wins_and_long_fallbacks_are_truncated() {
        let with_ai_title = format!(
            "{REAL_SESSION_JSONL}\n{}",
            r#"{"type":"ai-title","sessionId":"S1","aiTitle":"Resume rail subagent filter"}"#
        );
        let records = parse_claude_jsonl(&with_ai_title, "/Users/dev/work/mac-command-bar");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].title, "Resume rail subagent filter");

        let long_prompt = REAL_SESSION_JSONL.replace(
            "Fix the resume rail",
            "Fix the resume rail so it stops listing subagent transcripts and generic titles",
        );
        let records = parse_claude_jsonl(&long_prompt, "/Users/dev/work/mac-command-bar");
        assert_eq!(records[0].title.chars().count(), 60);
        assert!(records[0].title.starts_with("mac-command-bar — Fix the resume rail"));
        assert!(records[0].title.ends_with('…'));

        // Nothing usable anywhere: the generic label is still the last resort.
        let bare = r#"{"type":"file-history-snapshot","sessionId":"S3","timestamp":"2026-07-28T09:00:00Z"}"#;
        let records = parse_claude_jsonl(bare, "");
        assert_eq!(records[0].title, "Claude session");
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
