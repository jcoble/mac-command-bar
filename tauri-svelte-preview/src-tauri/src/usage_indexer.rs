use crate::usage_db::{cursor_lookup_key, UsageDb};
use crate::usage_sources::{
    discover_local_usage_sources, opaque_source_key, parse_usage_events_for_source_with_context,
    read_incremental_jsonl, read_source_context,
};
#[cfg(test)]
use std::path::Path;

#[derive(Debug, Clone)]
pub struct UsageIndexer {
    pub database_path: std::path::PathBuf,
}

impl UsageIndexer {
    pub fn new(database_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            database_path: database_path.into(),
        }
    }

    #[cfg(test)]
    pub fn ingest_jsonl(
        &self,
        path: &Path,
        cursor: Option<&crate::usage_db::UsageSourceCursor>,
        provider: &str,
        provider_instance_id: &str,
        source_kind: &str,
        source_key: &str,
    ) -> Result<usize, String> {
        let source = read_incremental_jsonl(
            path,
            cursor,
            provider,
            provider_instance_id,
            source_kind,
            source_key,
        )?;
        let context = read_source_context(path, provider);
        let events = parse_usage_events_for_source_with_context(
            &source.lines,
            provider,
            provider_instance_id,
            source_kind,
            source_key,
            &context,
        );
        let count = events.len();
        let db = UsageDb::open(&self.database_path)?;
        db.insert_events_with_cursor(&events, Some(&source.next_cursor))?;
        Ok(count)
    }

    /// Ingest all locally available provider transcripts. Discovery and
    /// cursor lookup happen once; each source tail is committed atomically so
    /// a very large transcript cannot turn into an oversized SQLite command.
    pub fn ingest_local_sources(&self) -> Result<usize, String> {
        let db = UsageDb::open(&self.database_path)?;
        let cursors = db.read_source_cursors()?;
        let mut count = 0_usize;
        for source in discover_local_usage_sources() {
            let source_key = source.path.to_string_lossy().to_string();
            let opaque_key = opaque_source_key(&source_key);
            let lookup = cursor_lookup_key(
                &source.provider,
                &source.provider_instance_id,
                &source.source_kind,
                &opaque_key,
            );
            let cursor = cursors.get(&lookup);
            let incremental = read_incremental_jsonl(
                &source.path,
                cursor,
                &source.provider,
                &source.provider_instance_id,
                &source.source_kind,
                &source_key,
            )?;
            let context = read_source_context(&source.path, &source.provider);
            let events = parse_usage_events_for_source_with_context(
                &incremental.lines,
                &source.provider,
                &source.provider_instance_id,
                &source.source_kind,
                &source_key,
                &context,
            );
            count = count.saturating_add(events.len());
            db.insert_events_with_cursor(&events, Some(&incremental.next_cursor))?;
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn fixture_jsonl_is_indexed_incrementally_with_non_zero_totals() {
        let root = std::env::temp_dir().join(format!(
            "mcb-usage-indexer-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let source_path = root.join("session.jsonl");
        let database_path = root.join("usage.sqlite3");
        let mut source = fs::File::create(&source_path).unwrap();
        writeln!(source, r#"{{"type":"assistant","uuid":"assistant-1","sessionId":"session-1","timestamp":"2026-08-08T12:34:56Z","cwd":"/tmp/project/workspace","message":{{"model":"model-a","usage":{{"input_tokens":12,"output_tokens":7}}}}}}"#).unwrap();
        let indexer = UsageIndexer::new(&database_path);
        let first = indexer
            .ingest_jsonl(
                &source_path,
                None,
                "claude",
                "local",
                "claude-jsonl",
                &source_path.to_string_lossy(),
            )
            .unwrap();
        assert_eq!(first, 1);
        let db = UsageDb::open(&database_path).unwrap();
        let cursor = db
            .read_source_cursors()
            .unwrap()
            .into_values()
            .next()
            .unwrap();
        let second = indexer
            .ingest_jsonl(
                &source_path,
                Some(&cursor),
                "claude",
                "local",
                "claude-jsonl",
                &source_path.to_string_lossy(),
            )
            .unwrap();
        assert_eq!(second, 0);
        let summary = db.read_usage_summary(&Default::default()).unwrap();
        assert_eq!(summary.event_count, 1);
        assert_eq!(summary.input_tokens, 12);
        assert_eq!(summary.output_tokens, 7);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn codex_incremental_tail_retains_session_context_and_unique_events() {
        let root = std::env::temp_dir().join(format!(
            "mcb-usage-codex-indexer-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let source_path = root.join("rollout.jsonl");
        let database_path = root.join("usage.sqlite3");
        let mut source = fs::File::create(&source_path).unwrap();
        writeln!(source, r#"{{"type":"session_meta","payload":{{"id":"session-2","cwd":"/tmp/project/workspace"}}}}"#).unwrap();
        writeln!(source, r#"{{"type":"event_msg","timestamp":"2026-08-08T12:34:56Z","payload":{{"type":"token_count","info":{{"last_token_usage":{{"input_tokens":20,"output_tokens":9}}}}}}}}"#).unwrap();
        source.flush().unwrap();

        let indexer = UsageIndexer::new(&database_path);
        assert_eq!(
            indexer
                .ingest_jsonl(
                    &source_path,
                    None,
                    "codex",
                    "local",
                    "codex-jsonl",
                    &source_path.to_string_lossy(),
                )
                .unwrap(),
            1
        );
        let db = UsageDb::open(&database_path).unwrap();
        let cursor = db
            .read_source_cursors()
            .unwrap()
            .into_values()
            .next()
            .unwrap();
        writeln!(source, r#"{{"type":"event_msg","timestamp":"2026-08-08T12:35:56Z","payload":{{"type":"token_count","info":{{"last_token_usage":{{"input_tokens":21,"output_tokens":10}}}}}}}}"#).unwrap();
        source.flush().unwrap();

        assert_eq!(
            indexer
                .ingest_jsonl(
                    &source_path,
                    Some(&cursor),
                    "codex",
                    "local",
                    "codex-jsonl",
                    &source_path.to_string_lossy(),
                )
                .unwrap(),
            1
        );
        let summary = db.read_usage_summary(&Default::default()).unwrap();
        assert_eq!(summary.event_count, 2);
        assert_eq!(summary.session_count, 1);
        assert_eq!(summary.input_tokens, 41);
        assert_eq!(summary.output_tokens, 19);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[ignore = "reads a bounded sample of the machine's real local transcript stores"]
    fn live_local_usage_sample_has_non_zero_totals() {
        let root = std::env::temp_dir().join(format!("mcb-live-usage-{}", std::process::id()));
        let indexer = UsageIndexer::new(root.join("usage.sqlite3"));
        let sources = crate::usage_sources::discover_local_usage_sources();
        assert!(
            !sources.is_empty(),
            "the machine should expose at least one local usage source"
        );
        let mut indexed_files = 0_usize;
        for source in sources.into_iter().take(32) {
            let key = source.path.to_string_lossy().to_string();
            let count = indexer
                .ingest_jsonl(
                    &source.path,
                    None,
                    &source.provider,
                    &source.provider_instance_id,
                    &source.source_kind,
                    &key,
                )
                .unwrap();
            indexed_files += 1;
            if count > 0 {
                break;
            }
        }
        let db = UsageDb::open(indexer.database_path.clone()).unwrap();
        let summary = db.read_usage_summary(&Default::default()).unwrap();
        crate::debug_log::stderr_log!("live usage sample indexed_files={indexed_files} events={} input={} output={} cache_read={} cache_write={} reasoning={}", summary.event_count, summary.input_tokens, summary.output_tokens, summary.cache_read_tokens, summary.cache_write_tokens, summary.reasoning_tokens);
        assert!(summary.event_count > 0);
        assert!(
            summary.input_tokens
                + summary.output_tokens
                + summary.cache_read_tokens
                + summary.cache_write_tokens
                + summary.reasoning_tokens
                > 0
        );
        let _ = fs::remove_dir_all(root);
    }
}
