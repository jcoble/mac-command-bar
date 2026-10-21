use crate::usage_db::{cursor_lookup_key, UsageDb, REBUILD_ROLLUPS_SQL};
use crate::usage_sources::{
    discover_local_usage_sources, opaque_source_key, parse_usage_events_for_source_with_context,
    read_incremental_jsonl, read_source_context,
};
use rusqlite::TransactionBehavior;
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
        let sources = discover_local_usage_sources();
        db.rebuild_claude_history_once(
            &sources
                .iter()
                .map(|source| opaque_source_key(&source.path.to_string_lossy()))
                .collect::<Vec<_>>(),
        )?;
        let cursors = db.read_source_cursors()?;
        let mut count = 0_usize;
        let mut rollups_dirty = false;
        let ingest_result = (|| -> Result<(), String> {
            for source in sources {
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
                if events.is_empty() && cursor == Some(&incremental.next_cursor) {
                    continue;
                }
                db.insert_events_with_cursor_deferred_rollup(
                    &events,
                    Some(&incremental.next_cursor),
                )?;
                if !events.is_empty() {
                    count = count.saturating_add(events.len());
                    rollups_dirty = true;
                }
            }
            Ok(())
        })();
        let rollup_result = if rollups_dirty {
            db.rebuild_daily_rollups()
        } else {
            Ok(())
        };
        if let Err(error) = ingest_result {
            if let Err(rollup_error) = rollup_result {
                return Err(format!("{error}; {rollup_error}"));
            }
            return Err(error);
        }
        rollup_result?;
        Ok(count)
    }
}

impl UsageDb {
    /// Claude rows used to be keyed by transcript line, so a message written
    /// over several lines was counted once per line. Once per database, drop
    /// the rows and cursors of the transcripts still on disk so the indexer
    /// re-reads them under the message key. Rows whose transcript is gone
    /// cannot be re-read and are kept. The delete is scoped to Claude rows in
    /// SQL, so the caller passes every discovered source key.
    pub fn rebuild_claude_history_once(&self, source_keys: &[String]) -> Result<(), String> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                self.database_error("could not start Claude history rebuild", error)
            })?;
        let version: i64 = transaction
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| self.database_error("could not read usage schema version", error))?;
        if version >= 1 {
            return Ok(());
        }
        // One statement per table: source_key has no index, so a delete per
        // transcript would scan the table once per file.
        let source_keys = serde_json::to_string(source_keys)
            .map_err(|error| format!("Could not encode Claude source keys: {error}"))?;
        for sql in [
            "DELETE FROM usage_events WHERE provider = 'claude' AND source_kind = 'claude-jsonl' AND source_key IN (SELECT value FROM json_each(?))",
            "DELETE FROM usage_source_cursors WHERE provider = 'claude' AND source_kind = 'claude-jsonl' AND source_key IN (SELECT value FROM json_each(?))",
        ] {
            transaction.execute(sql, [&source_keys]).map_err(|error| {
                self.database_error("could not clear Claude usage history", error)
            })?;
        }
        transaction
            .execute_batch(&format!("{REBUILD_ROLLUPS_SQL};\nPRAGMA user_version = 1;"))
            .map_err(|error| {
                self.database_error("could not finish Claude history rebuild", error)
            })?;
        transaction
            .commit()
            .map_err(|error| self.database_error("could not commit Claude history rebuild", error))
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

    fn claude_test_root(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "mcb-usage-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn claude_line(uuid: &str, message_id: &str, output_tokens: u64) -> String {
        format!(
            r#"{{"type":"assistant","uuid":"{uuid}","sessionId":"session-1","timestamp":"2026-08-08T12:34:56Z","message":{{"id":"{message_id}","model":"model-a","usage":{{"input_tokens":2,"output_tokens":{output_tokens},"cache_read_input_tokens":31538}}}}}}"#
        )
    }

    fn ingest_claude(
        indexer: &UsageIndexer,
        path: &Path,
        cursor: Option<&crate::usage_db::UsageSourceCursor>,
    ) -> usize {
        indexer
            .ingest_jsonl(
                path,
                cursor,
                "claude",
                "local",
                "claude-jsonl",
                &path.to_string_lossy(),
            )
            .unwrap()
    }

    #[test]
    fn claude_usage_second_line_in_a_later_pass_updates_the_stored_event() {
        let root = claude_test_root("claude-incremental");
        let source_path = root.join("session.jsonl");
        let indexer = UsageIndexer::new(root.join("usage.sqlite3"));
        let mut source = fs::File::create(&source_path).unwrap();
        writeln!(source, "{}", claude_line("line-1", "msg-1", 16)).unwrap();
        source.flush().unwrap();
        assert_eq!(ingest_claude(&indexer, &source_path, None), 1);

        let db = UsageDb::open(&indexer.database_path).unwrap();
        let cursor = db
            .read_source_cursors()
            .unwrap()
            .into_values()
            .next()
            .unwrap();
        writeln!(source, "{}", claude_line("line-2", "msg-1", 214)).unwrap();
        source.flush().unwrap();
        assert_eq!(ingest_claude(&indexer, &source_path, Some(&cursor)), 1);

        let summary = db.read_usage_summary(&Default::default()).unwrap();
        assert_eq!(summary.event_count, 1);
        assert_eq!(summary.output_tokens, 214);
        assert_eq!(summary.cache_read_tokens, 31538);
        let _ = fs::remove_dir_all(root);
    }

    /// A resumed or forked Claude session copies earlier messages into a new
    /// transcript under the same message id. It is one API response, so it is
    /// counted once across files.
    #[test]
    fn claude_usage_same_message_in_two_session_files_counts_once() {
        let root = claude_test_root("claude-two-files");
        let indexer = UsageIndexer::new(root.join("usage.sqlite3"));
        let first = root.join("first.jsonl");
        let resumed = root.join("resumed.jsonl");
        fs::write(&first, format!("{}\n", claude_line("line-1", "msg-1", 214))).unwrap();
        fs::write(
            &resumed,
            format!("{}\n", claude_line("line-9", "msg-1", 214)),
        )
        .unwrap();
        ingest_claude(&indexer, &first, None);
        ingest_claude(&indexer, &resumed, None);

        let db = UsageDb::open(&indexer.database_path).unwrap();
        let summary = db.read_usage_summary(&Default::default()).unwrap();
        assert_eq!(summary.event_count, 1);
        assert_eq!(summary.output_tokens, 214);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn claude_history_rebuild_replaces_old_line_keyed_rows_once() {
        let root = claude_test_root("claude-rebuild");
        let source_path = root.join("session.jsonl");
        let source_key = opaque_source_key(&source_path.to_string_lossy());
        let indexer = UsageIndexer::new(root.join("usage.sqlite3"));
        fs::write(
            &source_path,
            format!(
                "{}\n{}\n",
                claude_line("line-1", "msg-1", 16),
                claude_line("line-2", "msg-1", 214)
            ),
        )
        .unwrap();
        let db = UsageDb::open(&indexer.database_path).unwrap();
        // Seed what the old indexer stored: one row per transcript line, plus
        // a row whose transcript is gone and cannot be re-read.
        let old_row = |id: &str, key: &str, output_tokens| crate::usage_db::UsageEvent {
            provider: "claude".into(),
            provider_instance_id: "local".into(),
            owned_id: Some("session-1".into()),
            workflow_id: None,
            turn_id: None,
            project_id: None,
            workspace_id: None,
            occurred_at_micros: 1_786_192_496_000_000,
            input_tokens: 2,
            output_tokens,
            cache_read_tokens: 31538,
            cache_write_tokens: 0,
            reasoning_tokens: 0,
            model: "model-a".into(),
            estimated_cost_micros: None,
            estimate_rate_version: None,
            source_kind: "claude-jsonl".into(),
            source_event_id: id.into(),
            source_key: key.into(),
        };
        db.insert_events_with_cursor(
            &[
                old_row("line-1", &source_key, 16),
                old_row("line-2", &source_key, 214),
                old_row("gone-line", "source-0000000000000000", 5),
            ],
            None,
        )
        .unwrap();
        let mut cursor = crate::usage_sources::read_incremental_jsonl(
            &source_path,
            None,
            "claude",
            "local",
            "claude-jsonl",
            &source_path.to_string_lossy(),
        )
        .unwrap()
        .next_cursor;
        db.insert_events_with_cursor(&[], Some(&cursor)).unwrap();

        db.rebuild_claude_history_once(&[source_key.clone()])
            .unwrap();
        assert!(db.read_source_cursors().unwrap().is_empty());
        assert_eq!(ingest_claude(&indexer, &source_path, None), 1);

        let summary = db.read_usage_summary(&Default::default()).unwrap();
        assert_eq!(summary.event_count, 2);
        assert_eq!(summary.output_tokens, 219);
        assert_eq!(db.read_daily(&Default::default()).unwrap().len(), 1);

        // A second run is a no-op: the re-read rows and cursor stay.
        cursor = db
            .read_source_cursors()
            .unwrap()
            .into_values()
            .next()
            .unwrap();
        db.rebuild_claude_history_once(&[source_key]).unwrap();
        assert_eq!(
            db.read_source_cursors().unwrap().into_values().next(),
            Some(cursor)
        );
        assert_eq!(
            db.read_usage_summary(&Default::default())
                .unwrap()
                .event_count,
            2
        );
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
