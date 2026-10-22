use super::*;
use crate::usage_sources::{read_incremental_jsonl, UsageSourceContext};

fn token_count(second: u32, total: u64, last: u64) -> String {
    format!(
        r#"{{"type":"event_msg","timestamp":"2026-08-08T12:34:{second:02}Z","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":{total}}},"last_token_usage":{{"input_tokens":{last}}}}}}}}}"#
    )
}

fn ingest_codex(indexer: &UsageIndexer, path: &Path, cursor: Option<&UsageSourceCursor>) -> usize {
    indexer
        .ingest_jsonl(
            path,
            cursor,
            "codex",
            "local",
            "codex-jsonl",
            &path.to_string_lossy(),
        )
        .unwrap()
}

fn stored_cursor(db: &UsageDb) -> UsageSourceCursor {
    db.read_source_cursors()
        .unwrap()
        .into_values()
        .next()
        .unwrap()
}

#[test]
fn codex_repeat_split_across_indexing_passes_counts_once() {
    let root = claude_test_root("codex-split-repeat");
    let source_path = root.join("rollout.jsonl");
    let indexer = UsageIndexer::new(root.join("usage.sqlite3"));
    let mut source = fs::File::create(&source_path).unwrap();
    writeln!(source, "{}", token_count(1, 100, 100)).unwrap();
    source.flush().unwrap();
    assert_eq!(ingest_codex(&indexer, &source_path, None), 1);

    let db = UsageDb::open(&indexer.database_path).unwrap();
    let cursor = stored_cursor(&db);
    writeln!(source, "{}", token_count(2, 100, 100)).unwrap();
    writeln!(source, "{}", token_count(3, 150, 50)).unwrap();
    source.flush().unwrap();
    assert_eq!(ingest_codex(&indexer, &source_path, Some(&cursor)), 1);

    let summary = db.read_usage_summary(&Default::default()).unwrap();
    assert_eq!(summary.event_count, 2);
    assert_eq!(summary.input_tokens, 150);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn codex_history_rebuild_recounts_repeated_totals_once() {
    let root = claude_test_root("codex-rebuild");
    let source_path = root.join("rollout.jsonl");
    let source_key = source_path.to_string_lossy().to_string();
    let lines = vec![
        token_count(1, 100, 100),
        token_count(2, 100, 100),
        token_count(3, 150, 50),
    ];
    fs::write(&source_path, lines.join("\n") + "\n").unwrap();
    let indexer = UsageIndexer::new(root.join("usage.sqlite3"));
    let db = UsageDb::open(&indexer.database_path).unwrap();
    // What the old indexer stored: every line counted, Claude already rebuilt.
    let old_rows = lines
        .iter()
        .flat_map(|line| {
            crate::usage_sources::parse_usage_events_for_source_with_context(
                &[line.clone()],
                "codex",
                "local",
                "codex-jsonl",
                &source_key,
                &UsageSourceContext::default(),
                &mut None,
            )
        })
        .collect::<Vec<_>>();
    let cursor = read_incremental_jsonl(
        &source_path,
        None,
        "codex",
        "local",
        "codex-jsonl",
        &source_key,
    )
    .unwrap()
    .next_cursor;
    db.insert_events_with_cursor(&old_rows, Some(&cursor))
        .unwrap();
    db.connection()
        .unwrap()
        .execute_batch("PRAGMA user_version = 1;")
        .unwrap();
    assert_eq!(
        db.read_usage_summary(&Default::default())
            .unwrap()
            .input_tokens,
        250
    );

    // The stored cursor is ignored once: the transcript is re-read and replaces its rows.
    assert_eq!(ingest_codex(&indexer, &source_path, Some(&cursor)), 2);
    let summary = db.read_usage_summary(&Default::default()).unwrap();
    assert_eq!((summary.event_count, summary.input_tokens), (2, 150));

    // After that, passes read only new lines again.
    assert_eq!(
        ingest_codex(&indexer, &source_path, Some(&stored_cursor(&db))),
        0
    );
    assert_eq!(
        db.read_usage_summary(&Default::default())
            .unwrap()
            .event_count,
        2
    );
    let _ = fs::remove_dir_all(root);
}
