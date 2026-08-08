use crate::usage_db::{UsageDb, UsageEvent};
use crate::usage_sources::{parse_usage_events, read_incremental_jsonl};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct UsageIndexer {
    pub database_path: std::path::PathBuf,
}

impl UsageIndexer {
    pub fn new(database_path: impl Into<std::path::PathBuf>) -> Self {
        Self { database_path: database_path.into() }
    }

    pub fn ingest_events(&self, events: &[UsageEvent]) -> Result<(), String> {
        UsageDb::open(&self.database_path)?.insert_events(events)
    }

    pub fn ingest_jsonl(
        &self,
        path: &Path,
        cursor: Option<&crate::usage_db::UsageSourceCursor>,
        provider: &str,
        provider_instance_id: &str,
        source_kind: &str,
        source_key: &str,
    ) -> Result<usize, String> {
        let source = read_incremental_jsonl(path, cursor, provider, provider_instance_id, source_kind, source_key)?;
        let events = parse_usage_events(&source.lines);
        let count = events.len();
        let db = UsageDb::open(&self.database_path)?;
        db.insert_events_with_cursor(&events, Some(&source.next_cursor))?;
        Ok(count)
    }
}
