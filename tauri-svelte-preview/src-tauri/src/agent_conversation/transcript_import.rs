// Nothing outside the tests calls these yet: the command that imports a past session lands with the
// resume interface. Remove this allowance in that change.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use mcb_core::session_store::{EventRow, SessionRow, SessionStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::protocol::{
    AgentConversationProvider, AgentRawFrameReference, TerminalProjectionPayload,
};
use super::transcript::{complete_lines, parse_durable_line, read_range, ProjectedRecord};

/// Where an import stopped, stored inside the session's `extra_json`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCursor {
    pub transcript_path: String,
    pub cutoff_offset: u64,
    pub reached_start: bool,
}

pub struct ImportedTail {
    /// Records parsed from the tail, oldest first.
    pub records: Vec<ProjectedRecord>,
    /// Byte offset of the first line included. Pass this back as `end_offset` to read further back.
    pub cutoff_offset: u64,
    /// True when the tail reaches the start of the file and nothing older remains.
    pub reached_start: bool,
}

pub fn import_session(
    store: &SessionStore,
    provider: AgentConversationProvider,
    native_session_id: &str,
    path: &Path,
    cwd: &str,
    max_bytes: u64,
    max_records: usize,
) -> Result<String, String> {
    let file_len = path
        .metadata()
        .map_err(|error| format!("Could not inspect transcript: {error}"))?
        .len();
    let tail = read_tail(
        provider,
        native_session_id,
        path,
        file_len,
        max_bytes,
        max_records,
    )?;
    let owned_id = Uuid::new_v4().to_string();
    let fallback_timestamp = now_millis();
    let created_at_ms = tail
        .records
        .first()
        .and_then(record_timestamp)
        .unwrap_or(fallback_timestamp);
    let last_activity_at_ms = tail
        .records
        .last()
        .and_then(record_timestamp)
        .unwrap_or(created_at_ms);
    let cursor = ImportCursor {
        transcript_path: path.to_string_lossy().into_owned(),
        cutoff_offset: tail.cutoff_offset,
        reached_start: tail.reached_start,
    };
    let extra_json = merge_import_cursor("{}", &cursor)?;
    let session = SessionRow {
        owned_id: owned_id.clone(),
        native_session_id: Some(native_session_id.to_string()),
        provider: enum_storage_value(provider)?,
        model: None,
        effort: None,
        cwd: cwd.to_string(),
        worktree: None,
        branch: None,
        title: None,
        project: None,
        state: "ready".to_string(),
        suspended: false,
        created_at_ms,
        last_activity_at_ms,
        extra_json,
    };
    store
        .upsert_session(&session)
        .map_err(|error| error.to_string())?;
    for (sequence, record) in tail.records.into_iter().enumerate() {
        append_imported_record(
            store,
            &owned_id,
            native_session_id,
            i64::try_from(sequence)
                .map_err(|_| "Imported event sequence exceeded the store limit".to_string())?,
            record,
            created_at_ms,
        )?;
    }
    Ok(owned_id)
}

pub fn extend_session(
    store: &SessionStore,
    owned_id: &str,
    max_bytes: u64,
    max_records: usize,
) -> Result<usize, String> {
    let mut session = store
        .get_session(owned_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Imported session was not found".to_string())?;
    let cursor = import_cursor(&session.extra_json)?;
    if cursor.reached_start {
        return Ok(0);
    }
    let provider = super::transcript::parse_provider(&session.provider)?;
    let native_session_id = session
        .native_session_id
        .as_deref()
        .ok_or_else(|| "Imported session has no native session id".to_string())?;
    let tail = read_tail(
        provider,
        native_session_id,
        Path::new(&cursor.transcript_path),
        cursor.cutoff_offset,
        max_bytes,
        max_records,
    )?;
    let record_count = tail.records.len();
    let lowest_sequence = store
        .list_events(owned_id, i64::MIN, 1)
        .map_err(|error| error.to_string())?
        .first()
        .map_or(0, |event| event.seq);
    let first_sequence = lowest_sequence
        .checked_sub(
            i64::try_from(record_count)
                .map_err(|_| "Imported event count exceeded the store limit".to_string())?,
        )
        .ok_or_else(|| "Imported event sequence exceeded the store limit".to_string())?;
    for (index, record) in tail.records.into_iter().enumerate() {
        let sequence = first_sequence
            .checked_add(
                i64::try_from(index)
                    .map_err(|_| "Imported event sequence exceeded the store limit".to_string())?,
            )
            .ok_or_else(|| "Imported event sequence exceeded the store limit".to_string())?;
        append_imported_record(
            store,
            owned_id,
            native_session_id,
            sequence,
            record,
            session.created_at_ms,
        )?;
    }
    let next_cursor = ImportCursor {
        transcript_path: cursor.transcript_path,
        cutoff_offset: tail.cutoff_offset,
        reached_start: tail.reached_start,
    };
    session.extra_json = merge_import_cursor(&session.extra_json, &next_cursor)?;
    store
        .upsert_session(&session)
        .map_err(|error| error.to_string())?;
    Ok(record_count)
}

fn append_imported_record(
    store: &SessionStore,
    owned_id: &str,
    native_session_id: &str,
    sequence: i64,
    record: ProjectedRecord,
    fallback_timestamp: i64,
) -> Result<(), String> {
    let created_at_ms = record_timestamp(&record).unwrap_or(fallback_timestamp);
    let kind = enum_storage_value(record.event_type)?;
    let payload = projection_payload(native_session_id, record);
    let payload_json = serde_json::to_string(&payload)
        .map_err(|error| format!("Could not encode imported transcript event: {error}"))?;
    store
        .append_event(&EventRow {
            owned_id: owned_id.to_string(),
            seq: sequence,
            turn_id: None,
            kind,
            payload_json,
            created_at_ms,
        })
        .map_err(|error| error.to_string())
}

fn projection_payload(
    native_session_id: &str,
    record: ProjectedRecord,
) -> TerminalProjectionPayload {
    TerminalProjectionPayload {
        event_type: record.event_type,
        provider_instance_id: format!("imported-transcript:{native_session_id}"),
        timestamp_ms: u64::try_from(record.timestamp_ms)
            .ok()
            .filter(|value| *value != 0),
        native_session_id: native_session_id.to_string(),
        item_id: record.item_id.clone(),
        payload: record.payload,
        provider_metadata: BTreeMap::from([
            ("source".into(), Value::String("imported-transcript".into())),
            ("historical".into(), Value::Bool(true)),
        ]),
        raw_frame_reference: AgentRawFrameReference {
            id: record.key,
            redacted: true,
        },
    }
}

fn import_cursor(extra_json: &str) -> Result<ImportCursor, String> {
    let extra: Value = serde_json::from_str(extra_json)
        .map_err(|error| format!("Could not decode stored session metadata: {error}"))?;
    serde_json::from_value(
        extra
            .get("import")
            .cloned()
            .ok_or_else(|| "Session has no import cursor".to_string())?,
    )
    .map_err(|error| format!("Could not decode import cursor: {error}"))
}

fn merge_import_cursor(extra_json: &str, cursor: &ImportCursor) -> Result<String, String> {
    let mut extra: Value = serde_json::from_str(extra_json)
        .map_err(|error| format!("Could not decode stored session metadata: {error}"))?;
    let object = extra
        .as_object_mut()
        .ok_or_else(|| "Stored session metadata was not a JSON object".to_string())?;
    object.insert(
        "import".to_string(),
        serde_json::to_value(cursor)
            .map_err(|error| format!("Could not encode import cursor: {error}"))?,
    );
    serde_json::to_string(&extra)
        .map_err(|error| format!("Could not encode stored session metadata: {error}"))
}

fn enum_storage_value<T: Serialize>(value: T) -> Result<String, String> {
    serde_json::to_value(value)
        .map_err(|error| format!("Could not encode stored enum value: {error}"))?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "Stored enum value was not a string".to_string())
}

fn record_timestamp(record: &ProjectedRecord) -> Option<i64> {
    (record.timestamp_ms != 0).then(|| i64::try_from(record.timestamp_ms).unwrap_or(i64::MAX))
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use mcb_core::session_store::SessionStore;
    use serde_json::{json, Value};
    use uuid::Uuid;

    use super::*;

    struct TranscriptFixture {
        path: PathBuf,
        contents: String,
    }

    impl TranscriptFixture {
        fn new(record_count: usize) -> Self {
            let path = std::env::temp_dir().join(format!("mcb-tail-{}.jsonl", Uuid::new_v4()));
            let contents = (1..=record_count)
                .map(message_line)
                .collect::<Vec<_>>()
                .join("");
            fs::write(&path, &contents).expect("transcript fixture should be written");
            Self { path, contents }
        }

        fn line_len(&self) -> u64 {
            self.contents
                .find('\n')
                .map(|index| index as u64 + 1)
                .expect("fixture should contain a complete line")
        }

        fn len(&self) -> u64 {
            self.contents.len() as u64
        }
    }

    impl Drop for TranscriptFixture {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    fn message_line(index: usize) -> String {
        format!(
            "{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"id\":\"item-{index}\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"message\"}}]}}}}\n"
        )
    }

    fn item_ids(tail: &ImportedTail) -> Vec<&str> {
        tail.records
            .iter()
            .filter_map(|record| record.item_id.as_deref())
            .collect()
    }

    fn fixture_provider() -> AgentConversationProvider {
        super::super::transcript::parse_provider(&["co", "dex"].concat())
            .expect("fixture provider should be supported")
    }

    fn read_fixture(
        fixture: &TranscriptFixture,
        end_offset: u64,
        max_bytes: u64,
        max_records: usize,
    ) -> ImportedTail {
        read_tail(
            fixture_provider(),
            "session-1",
            Path::new(&fixture.path),
            end_offset,
            max_bytes,
            max_records,
        )
        .expect("tail should be read")
    }

    fn stored_item_ids(store: &SessionStore, owned_id: &str) -> Vec<String> {
        store
            .list_events(owned_id, i64::MIN, 100)
            .expect("events should be listed")
            .into_iter()
            .map(|event| {
                serde_json::from_str::<Value>(&event.payload_json)
                    .expect("event payload should be JSON")
                    .get("itemId")
                    .and_then(Value::as_str)
                    .expect("event should have an item id")
                    .to_string()
            })
            .collect()
    }

    fn stored_cursor(store: &SessionStore, owned_id: &str) -> ImportCursor {
        let session = store
            .get_session(owned_id)
            .expect("session should be read")
            .expect("session should exist");
        serde_json::from_str::<Value>(&session.extra_json)
            .expect("session extra should be JSON")
            .get("import")
            .cloned()
            .map(serde_json::from_value)
            .expect("session should have an import cursor")
            .expect("import cursor should be valid")
    }

    #[test]
    fn import_writes_the_tail_and_records_where_it_stopped() {
        let fixture = TranscriptFixture::new(6);
        let store = SessionStore::open_in_memory().expect("store should open");

        let owned_id = import_session(
            &store,
            fixture_provider(),
            "session-1",
            &fixture.path,
            "/tmp/project",
            fixture.line_len() * 4,
            usize::MAX,
        )
        .expect("session should import");

        let session = store
            .get_session(&owned_id)
            .expect("session should be read")
            .expect("session should exist");
        assert_eq!(session.native_session_id.as_deref(), Some("session-1"));
        assert_eq!(session.cwd, "/tmp/project");
        assert_eq!(
            stored_item_ids(&store, &owned_id),
            ["item-4", "item-5", "item-6"]
        );
        let cursor = stored_cursor(&store, &owned_id);
        assert!(cursor.cutoff_offset > 0);
        assert!(!cursor.reached_start);
    }

    #[test]
    fn import_of_a_short_transcript_reports_it_reached_the_start() {
        let fixture = TranscriptFixture::new(2);
        let store = SessionStore::open_in_memory().expect("store should open");

        let owned_id = import_session(
            &store,
            fixture_provider(),
            "session-1",
            &fixture.path,
            "/tmp/project",
            fixture.len() + 1,
            usize::MAX,
        )
        .expect("session should import");

        let cursor = stored_cursor(&store, &owned_id);
        assert_eq!(cursor.cutoff_offset, 0);
        assert!(cursor.reached_start);
    }

    #[test]
    fn extending_prepends_older_records_before_the_existing_ones() {
        let fixture = TranscriptFixture::new(6);
        let store = SessionStore::open_in_memory().expect("store should open");
        let owned_id = import_session(
            &store,
            fixture_provider(),
            "session-1",
            &fixture.path,
            "/tmp/project",
            fixture.line_len() * 4,
            usize::MAX,
        )
        .expect("session should import");

        let added = extend_session(&store, &owned_id, fixture.line_len() * 4, usize::MAX)
            .expect("session should extend");

        assert_eq!(added, 3);
        assert_eq!(
            stored_item_ids(&store, &owned_id),
            ["item-1", "item-2", "item-3", "item-4", "item-5", "item-6"]
        );
    }

    #[test]
    fn extending_a_session_that_reached_the_start_adds_nothing() {
        let fixture = TranscriptFixture::new(2);
        let store = SessionStore::open_in_memory().expect("store should open");
        let owned_id = import_session(
            &store,
            fixture_provider(),
            "session-1",
            &fixture.path,
            "/tmp/project",
            fixture.len() + 1,
            usize::MAX,
        )
        .expect("session should import");
        let events_before = store
            .list_events(&owned_id, i64::MIN, 100)
            .expect("events should be listed");
        let cursor_before = stored_cursor(&store, &owned_id);

        let added = extend_session(&store, &owned_id, fixture.len(), usize::MAX)
            .expect("extension should be checked");

        assert_eq!(added, 0);
        assert_eq!(
            store
                .list_events(&owned_id, i64::MIN, 100)
                .expect("events should be listed"),
            events_before
        );
        assert_eq!(stored_cursor(&store, &owned_id), cursor_before);
    }

    #[test]
    fn import_preserves_other_extra_json_content() {
        let fixture = TranscriptFixture::new(4);
        let store = SessionStore::open_in_memory().expect("store should open");
        let owned_id = import_session(
            &store,
            fixture_provider(),
            "session-1",
            &fixture.path,
            "/tmp/project",
            fixture.line_len() * 2,
            usize::MAX,
        )
        .expect("session should import");
        let mut session = store
            .get_session(&owned_id)
            .expect("session should be read")
            .expect("session should exist");
        let mut extra = serde_json::from_str::<Value>(&session.extra_json)
            .expect("session extra should be JSON");
        extra["other"] = json!({ "kept": true });
        session.extra_json = serde_json::to_string(&extra).expect("session extra should encode");
        store
            .upsert_session(&session)
            .expect("session should be updated");

        extend_session(&store, &owned_id, fixture.line_len() * 2, usize::MAX)
            .expect("session should extend");

        let session = store
            .get_session(&owned_id)
            .expect("session should be read")
            .expect("session should exist");
        let extra = serde_json::from_str::<Value>(&session.extra_json)
            .expect("session extra should be JSON");
        assert_eq!(extra["other"], json!({ "kept": true }));
        assert!(extra.get("import").is_some());
    }

    #[test]
    fn tail_stops_at_the_byte_budget() {
        let fixture = TranscriptFixture::new(5);

        let tail = read_fixture(&fixture, fixture.len(), fixture.line_len() * 3, usize::MAX);

        assert!(!tail.records.is_empty());
        assert!(tail.cutoff_offset > 0);
        assert!(!tail.reached_start);
    }

    #[test]
    fn tail_reaching_the_start_reports_it() {
        let fixture = TranscriptFixture::new(2);

        let tail = read_fixture(&fixture, fixture.len(), fixture.len() + 1, usize::MAX);

        assert_eq!(item_ids(&tail), ["item-1", "item-2"]);
        assert_eq!(tail.cutoff_offset, 0);
        assert!(tail.reached_start);
    }

    #[test]
    fn a_window_with_no_usable_line_still_moves_the_cutoff_back() {
        let path = std::env::temp_dir().join(format!("mcb-tail-noise-{}.jsonl", Uuid::new_v4()));
        let contents = "not json\nalso not json\n";
        fs::write(&path, contents).expect("noise fixture should be written");
        let len = contents.len() as u64;

        let tail = read_tail(
            AgentConversationProvider::Codex,
            "session",
            &path,
            len,
            len,
            usize::MAX,
        )
        .expect("reading the noise fixture should succeed");

        assert!(tail.records.is_empty());
        assert_eq!(tail.cutoff_offset, 0);
        assert!(tail.reached_start);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn tail_honours_the_record_cap() {
        let fixture = TranscriptFixture::new(4);

        let tail = read_fixture(&fixture, fixture.len(), fixture.len(), 2);

        assert_eq!(item_ids(&tail), ["item-3", "item-4"]);
        assert!(tail.records.len() <= 2);
        assert_eq!(tail.cutoff_offset, fixture.line_len() * 2);
        assert!(!tail.reached_start);
    }

    #[test]
    fn reading_again_from_the_cutoff_walks_backwards_without_gaps() {
        let fixture = TranscriptFixture::new(6);
        let max_bytes = fixture.line_len() * 4;

        let newer = read_fixture(&fixture, fixture.len(), max_bytes, usize::MAX);
        let older = read_fixture(&fixture, newer.cutoff_offset, max_bytes, usize::MAX);
        let ids = item_ids(&older)
            .into_iter()
            .chain(item_ids(&newer))
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            ["item-1", "item-2", "item-3", "item-4", "item-5", "item-6"]
        );
        assert!(older.reached_start);
    }
}

pub fn read_tail(
    provider: AgentConversationProvider,
    native_session_id: &str,
    path: &Path,
    end_offset: u64,
    max_bytes: u64,
    max_records: usize,
) -> Result<ImportedTail, String> {
    let start = end_offset.saturating_sub(max_bytes);
    let read_len = end_offset - start;
    if read_len == 0 {
        return Ok(ImportedTail {
            records: Vec::new(),
            cutoff_offset: end_offset,
            reached_start: end_offset == 0,
        });
    }

    let bytes = read_range(path, start, read_len)?;
    let (lines, _) = complete_lines(&bytes, start > 0);
    let bytes_start = bytes.as_ptr() as usize;
    let mut parsed_lines = lines
        .into_iter()
        .filter_map(|line| {
            let records = parse_durable_line(provider, native_session_id, line);
            if records.is_empty() {
                return None;
            }
            let line_offset = start + (line.as_ptr() as usize - bytes_start) as u64;
            Some((line_offset, records))
        })
        .collect::<Vec<_>>();

    let mut record_count = parsed_lines
        .iter()
        .map(|(_, records)| records.len())
        .sum::<usize>();
    let mut first_surviving_line = 0;
    while record_count > max_records && first_surviving_line < parsed_lines.len() {
        record_count -= parsed_lines[first_surviving_line].1.len();
        first_surviving_line += 1;
    }
    if first_surviving_line > 0 {
        parsed_lines.drain(..first_surviving_line);
    }

    // A window with no usable line still consumed everything back to `start`, so the cutoff moves
    // there. Reporting `end_offset` instead would hand back the same window forever.
    let cutoff_offset = parsed_lines.first().map_or(start, |(offset, _)| *offset);
    let records = parsed_lines
        .into_iter()
        .flat_map(|(_, records)| records)
        .collect();

    Ok(ImportedTail {
        records,
        cutoff_offset,
        reached_start: cutoff_offset == 0,
    })
}
