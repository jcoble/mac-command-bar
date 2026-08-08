use crate::usage_db::{UsageEvent, UsageSourceCursor};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct IncrementalJsonl {
    pub lines: Vec<String>,
    pub next_cursor: UsageSourceCursor,
    pub reset: bool,
}

/// Read only the bytes after the durable cursor. A truncated or replaced file
/// starts from zero and is allowed to deduplicate through `source_event_id`.
pub fn read_incremental_jsonl(
    path: &Path,
    cursor: Option<&UsageSourceCursor>,
    provider: &str,
    provider_instance_id: &str,
    source_kind: &str,
    source_key: &str,
) -> Result<IncrementalJsonl, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("Usage source is unavailable: {error}"))?;
    let file_identity = file_identity(path, &metadata)?;
    let cursor_offset = cursor
        .filter(|cursor| cursor.file_identity == file_identity && cursor.offset <= metadata.len())
        .map(|cursor| cursor.offset as usize)
        .unwrap_or(0);
    let reset = cursor.is_some_and(|cursor| cursor_offset == 0 && cursor.offset != 0);
    let mut file = fs::File::open(path).map_err(|error| format!("Usage source could not be opened: {error}"))?;
    file.seek(SeekFrom::Start(cursor_offset as u64))
        .map_err(|error| format!("Usage source cursor could not seek: {error}"))?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail)
        .map_err(|error| format!("Usage source tail could not be read: {error}"))?;
    let lines = String::from_utf8_lossy(&tail)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let next_cursor = UsageSourceCursor {
        provider: provider.to_string(),
        provider_instance_id: provider_instance_id.to_string(),
        source_kind: source_kind.to_string(),
        source_key: opaque_source_key(source_key),
        file_identity,
        offset: metadata.len(),
        size: metadata.len(),
        modified_at_micros: metadata_modified_micros(&metadata),
        last_event_id: None,
    };
    Ok(IncrementalJsonl { lines, next_cursor, reset })
}

pub fn parse_usage_events(lines: &[String]) -> Vec<UsageEvent> {
    lines
        .iter()
        .filter_map(|line| {
            let mut event = serde_json::from_str::<UsageEvent>(line).ok()?;
            // Source keys identify a cursor, not a user path. Keep only the
            // stable opaque form in the normalized index.
            event.source_key = opaque_source_key(&event.source_key);
            Some(event)
        })
        .collect()
}

pub fn opaque_source_key(source_key: &str) -> String {
    if source_key.len() == 23
        && source_key.starts_with("source-")
        && source_key[7..].chars().all(|character| character.is_ascii_hexdigit())
    {
        return source_key.to_string();
    }
    // FNV-1a is small, deterministic, and does not retain any portion of a
    // path or provider payload in the database cursor key.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in source_key.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("source-{hash:016x}")
}

fn file_identity(path: &Path, metadata: &fs::Metadata) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|error| format!("Usage source could not be opened: {error}"))?;
    let mut prefix = [0_u8; 512];
    let prefix_len = file
        .read(&mut prefix)
        .map_err(|error| format!("Usage source identity could not be read: {error}"))?;
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in metadata
        .len()
        .to_le_bytes()
        .iter()
        .chain(metadata_modified_micros(metadata).to_le_bytes().iter())
        .chain(prefix[..prefix_len].iter())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Ok(format!("file-{hash:016x}"))
}

fn metadata_modified_micros(metadata: &fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_micros().min(i64::MAX as u128) as i64)
        .unwrap_or_else(|| now_micros())
}

fn now_micros() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_micros().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn usage_source_cursor_resets_after_truncation_without_replaying_same_suffix() {
        let path = std::env::temp_dir().join(format!("mcb-usage-source-{}.jsonl", std::process::id()));
        let mut file = fs::File::create(&path).unwrap();
        writeln!(file, "{{\"provider\":\"p\"}}").unwrap();
        let first = read_incremental_jsonl(&path, None, "p", "i", "jsonl", "source").unwrap();
        assert_eq!(first.lines.len(), 1);
        fs::write(&path, b"{\"provider\":\"p\"}\n").unwrap();
        let second = read_incremental_jsonl(&path, Some(&UsageSourceCursor { offset: 999, ..first.next_cursor.clone() }), "p", "i", "jsonl", "source").unwrap();
        assert!(second.reset);
        assert_eq!(first.next_cursor.source_key, opaque_source_key("source"));
        assert!(!first.next_cursor.source_key.contains("/"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn normalized_events_keep_source_keys_opaque() {
        let events = parse_usage_events(&[r#"{"provider":"p","providerInstanceId":"i","model":"m","sourceKind":"jsonl","sourceEventId":"e","sourceKey":"/private/worktree/session.jsonl","occurredAtMicros":1,"inputTokens":1,"outputTokens":1,"cacheReadTokens":0,"cacheWriteTokens":0,"reasoningTokens":0}"#.to_string()]);
        assert_eq!(events.len(), 1);
        assert!(events[0].source_key.starts_with("source-"));
        assert!(!events[0].source_key.contains("worktree"));
    }
}
