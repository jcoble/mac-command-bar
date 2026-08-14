mod claude;
mod codex;

use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File, Metadata};
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::protocol::{AgentConversationProvider, AgentEventType};

pub const RECONCILIATION_BYTES: u64 = 4 * 1024 * 1024;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CodexChildRollout {
    pub child_id: String,
    pub label: String,
    pub state: String,
    pub latest_activity: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSnapshot {
    pub messages: Vec<TranscriptMessage>,
    pub metadata: ConversationMetadata,
    pub children: Vec<ChildAgentDescriptor>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileIdentity {
    pub device: u64,
    pub inode: u64,
}

#[derive(Clone, Debug)]
pub struct TranscriptLocation {
    pub canonical_path: PathBuf,
    pub identity: FileIdentity,
    pub len: u64,
}

#[derive(Clone, Debug)]
pub struct ProjectedRecord {
    pub key: String,
    pub event_type: AgentEventType,
    pub timestamp_ms: u128,
    pub item_id: Option<String>,
    pub payload: std::collections::BTreeMap<String, Value>,
}

pub fn read(
    provider: &str,
    native_session_id: &str,
    child_session_id: Option<&str>,
) -> Result<TranscriptSnapshot, String> {
    let provider = parse_provider(provider)?;
    let native_session_id = safe_session_id(native_session_id)?;
    let child_session_id = child_session_id.map(safe_session_id).transpose()?;
    match provider {
        AgentConversationProvider::Codex => {
            codex::read_snapshot(&native_session_id, child_session_id.as_deref())
        }
        AgentConversationProvider::Claude => {
            claude::read_snapshot(&native_session_id, child_session_id.as_deref())
        }
    }
}

pub fn parse_provider(value: &str) -> Result<AgentConversationProvider, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "codex" => Ok(AgentConversationProvider::Codex),
        "claude" => Ok(AgentConversationProvider::Claude),
        _ => Err("Transcript provider is unsupported".to_string()),
    }
}

pub fn discover(
    provider: AgentConversationProvider,
    native_session_id: &str,
) -> Result<Option<TranscriptLocation>, String> {
    let id = safe_session_id(native_session_id)?;
    let path = match provider {
        AgentConversationProvider::Codex => codex::discover_path(&id),
        AgentConversationProvider::Claude => claude::discover_path(&id)?,
    };
    path.map(location).transpose()
}

pub(crate) fn scan_codex_child_rollouts(
    parent_path: &Path,
    parent_id: &str,
    active_after: SystemTime,
) -> Result<Vec<CodexChildRollout>, String> {
    codex::scan_child_rollouts(parent_path, parent_id, active_after)
}

pub fn inspect(path: &Path) -> Result<Option<TranscriptLocation>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    location(path.to_path_buf()).map(Some)
}

pub fn parse_durable_line(
    provider: AgentConversationProvider,
    native_session_id: &str,
    line: &[u8],
) -> Vec<ProjectedRecord> {
    let Ok(value) = serde_json::from_slice::<Value>(line) else {
        return Vec::new();
    };
    match provider {
        AgentConversationProvider::Codex => codex::project(&value, line),
        AgentConversationProvider::Claude => claude::project(&value, line, native_session_id),
    }
}

pub fn read_range(path: &Path, start: u64, max_bytes: u64) -> Result<Vec<u8>, String> {
    let mut file =
        File::open(path).map_err(|error| format!("Could not open transcript: {error}"))?;
    file.seek(SeekFrom::Start(start))
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.take(max_bytes)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

pub fn read_bounded(path: &Path, len: u64) -> Result<(u64, Vec<u8>), String> {
    let start = len.saturating_sub(RECONCILIATION_BYTES);
    Ok((start, read_range(path, start, RECONCILIATION_BYTES)?))
}

pub fn complete_lines(bytes: &[u8], skip_first_partial: bool) -> (Vec<&[u8]>, Vec<u8>) {
    let mut start = 0;
    if skip_first_partial {
        start = bytes
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(bytes.len(), |index| index + 1);
    }
    let mut lines = Vec::new();
    let mut search = start;
    while let Some(relative) = bytes[search..].iter().position(|byte| *byte == b'\n') {
        let end = search + relative;
        if end >= start {
            let line = bytes[start..end]
                .strip_suffix(b"\r")
                .unwrap_or(&bytes[start..end]);
            if !line.is_empty() {
                lines.push(line);
            }
            start = end + 1;
        }
        search = end + 1;
    }
    (lines, bytes[start..].to_vec())
}

pub fn safe_session_id(value: &str) -> Result<String, String> {
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

pub fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is unavailable".to_string())
}

pub fn read_snapshot_text(path: &Path) -> Result<String, String> {
    let len = path.metadata().map_err(|error| error.to_string())?.len();
    let (start, bytes) = read_bounded(path, len)?;
    let (lines, _) = complete_lines(&bytes, start > 0);
    Ok(lines
        .into_iter()
        .map(String::from_utf8_lossy)
        .collect::<Vec<_>>()
        .join("\n"))
}

pub fn parse_json_lines(input: &str) -> impl Iterator<Item = Value> + '_ {
    input
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
}

pub fn object(entries: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Value {
    Value::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect::<Map<_, _>>(),
    )
}

pub fn stable_key(prefix: &str, line: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    line.hash(&mut hasher);
    format!("{prefix}:{:016x}", hasher.finish())
}

pub fn timestamp(value: &Value) -> u128 {
    value
        .get("timestamp")
        .and_then(Value::as_u64)
        .map(u128::from)
        .or_else(|| {
            value
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(|value| value.parse().ok())
        })
        .or_else(|| {
            value
                .pointer("/payload/occurred_at_ms")
                .and_then(Value::as_u64)
                .map(u128::from)
        })
        .unwrap_or(0)
}

pub fn modified_millis(path: &Path) -> u64 {
    path.metadata()
        .ok()
        .and_then(|value| value.modified().ok())
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

fn location(path: PathBuf) -> Result<TranscriptLocation, String> {
    let canonical_path = fs::canonicalize(&path)
        .map_err(|error| format!("Could not resolve transcript: {error}"))?;
    let metadata = canonical_path
        .metadata()
        .map_err(|error| error.to_string())?;
    Ok(TranscriptLocation {
        identity: file_identity(&metadata),
        len: metadata.len(),
        canonical_path,
    })
}

#[cfg(unix)]
fn file_identity(metadata: &Metadata) -> FileIdentity {
    use std::os::unix::fs::MetadataExt;
    FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

#[cfg(not(unix))]
fn file_identity(metadata: &Metadata) -> FileIdentity {
    FileIdentity {
        device: 0,
        inode: metadata.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_claude_parent_and_metadata_without_sidechain_messages() {
        let input = r#"{"type":"user","uuid":"u1","sessionId":"s1","isSidechain":false,"permissionMode":"acceptEdits","message":{"content":"Hello"}}
{"type":"assistant","uuid":"a1","sessionId":"s1","isSidechain":false,"effort":"high","message":{"model":"claude-opus-5","usage":{"input_tokens":2,"cache_read_input_tokens":120},"content":[{"type":"text","text":"Answer"}]}}
{"type":"assistant","uuid":"child","sessionId":"s1","isSidechain":true,"agentId":"agent-a","message":{"content":[{"type":"text","text":"Hidden"}]}}"#;
        let records = input
            .lines()
            .flat_map(|line| {
                parse_durable_line(AgentConversationProvider::Claude, "s1", line.as_bytes())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            records
                .iter()
                .filter(|record| record.event_type == AgentEventType::ItemCompleted)
                .count(),
            2
        );
        assert!(!records
            .iter()
            .any(|record| record.item_id.as_deref() == Some("child")));
        let config = records
            .iter()
            .find(|record| {
                record.event_type == AgentEventType::SessionConfigUpdated
                    && record.payload["config"]["model"] == "claude-opus-5"
            })
            .unwrap();
        assert_eq!(config.payload["config"]["model"], "claude-opus-5");
        assert_eq!(config.payload["config"]["effort"], "high");
        let usage = records
            .iter()
            .find(|record| record.event_type == AgentEventType::UsageUpdated)
            .unwrap();
        assert_eq!(usage.payload["usedTokens"], 122);
    }

    #[test]
    fn parses_codex_metadata_messages_and_child_activity() {
        let input = r#"{"type":"turn_context","payload":{"model":"gpt-5.6-sol","effort":"medium","approval_policy":"never"}}
{"type":"response_item","payload":{"type":"message","id":"u1","role":"user","content":[{"type":"input_text","text":"Review this"}]}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":1000},"model_context_window":258400}}}
{"type":"event_msg","payload":{"type":"sub_agent_activity","agent_thread_id":"child-1","agent_path":"/root/reviewer","kind":"started","occurred_at_ms":50}}"#;
        let records = input
            .lines()
            .flat_map(|line| {
                parse_durable_line(AgentConversationProvider::Codex, "s1", line.as_bytes())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            records
                .iter()
                .filter(|record| record.event_type == AgentEventType::ItemCompleted)
                .count(),
            1
        );
        let config = records
            .iter()
            .find(|record| record.event_type == AgentEventType::SessionConfigUpdated)
            .unwrap();
        assert_eq!(config.payload["config"]["model"], "gpt-5.6-sol");
        let usage = records
            .iter()
            .find(|record| record.event_type == AgentEventType::UsageUpdated)
            .unwrap();
        assert_eq!(usage.payload["usedTokens"], 1000);
        let children = records
            .iter()
            .find(|record| record.event_type == AgentEventType::ChildrenUpdated)
            .unwrap();
        assert_eq!(children.payload["children"][0]["state"], "historical");
    }

    #[test]
    fn rejects_unsafe_child_session_ids() {
        assert!(safe_session_id("../secret").is_err());
        assert!(safe_session_id("child-ok_1").is_ok());
    }
}
