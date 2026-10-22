//! Usage rows from ACP. When a session's turn ends, the `session/prompt` result
//! says what the turn used, per model, and the last `usage_update` before it
//! carries the cost the provider reports. Rows are written already attributed to
//! the Assembly session and the workflow step attempt (`node_run_id`).
//!
//! Every input count is uncached input; cache reads and writes are their own
//! columns. Claude and Codex both report `inputTokens` that way over ACP.

use super::UsageDb;
use rusqlite::{params, Transaction, TransactionBehavior};
use serde_json::Value;

/// Rows written from ACP turns. Their `source_key` is the native session id, so
/// the transcript reader can tell which sessions it must not count again.
pub const ACP_SOURCE_KIND: &str = "acp";

/// Model, then US dollars per million tokens: input, output, cache read, cache
/// write. A stand-in until the provider descriptors carry rates (TSK-1594);
/// `rebuild_rates` copies it into `usage_rates` at startup.
pub const SEEDED_RATES: &[(&str, f64, f64, f64, f64)] = &[
    ("gpt-5.6-sol", 5.0, 30.0, 0.5, 6.25), ("codex", 5.0, 30.0, 0.5, 6.25),
    ("gpt-5.6-terra", 2.5, 15.0, 0.25, 3.125), ("gpt-5.6-luna", 1.0, 6.0, 0.1, 1.25),
    ("gpt-5.5", 5.0, 30.0, 0.5, 6.25), ("gpt-5.4", 2.5, 15.0, 0.25, 2.5),
    ("gpt-5.3-codex", 1.75, 14.0, 0.175, 1.75), ("gpt-5.2-codex", 1.75, 14.0, 0.175, 1.75),
    ("claude-opus-5", 5.0, 25.0, 0.5, 6.25), ("claude-opus-4-8", 5.0, 25.0, 0.5, 6.25),
    ("claude-opus-4.8", 5.0, 25.0, 0.5, 6.25), ("claude-opus-4-7", 5.0, 25.0, 0.5, 6.25),
    ("claude-opus-4.7", 5.0, 25.0, 0.5, 6.25), ("claude-opus-4-6", 5.0, 25.0, 0.5, 6.25),
    ("claude-opus-4.6", 5.0, 25.0, 0.5, 6.25), ("claude-opus-4-5", 5.0, 25.0, 0.5, 6.25),
    ("claude-opus-4.5", 5.0, 25.0, 0.5, 6.25), ("claude-sonnet-5", 2.0, 10.0, 0.2, 2.5),
    ("claude-sonnet-4-6", 3.0, 15.0, 0.3, 3.75), ("claude-sonnet-4.6", 3.0, 15.0, 0.3, 3.75),
    ("claude-haiku-4-5", 1.0, 5.0, 0.1, 1.25), ("claude-haiku-4.5", 1.0, 5.0, 0.1, 1.25),
];

/// One finished ACP turn, as the agent manager hands it over.
#[derive(Debug, Clone, Default)]
pub struct AcpTurn {
    pub owned_id: String,
    pub native_session_id: Option<String>,
    pub provider: String,
    pub turn_id: String,
    /// The session's model, for a result that has no per-model breakdown.
    pub model: Option<String>,
    /// The `session/prompt` result.
    pub result: Value,
    /// The latest `usage_update` cost: the session's running total in USD.
    pub cost_total_usd: Option<f64>,
    pub node_run_id: Option<String>,
    /// When the turn's prompt went out. Transcript rows between then and when
    /// the turn's rows are written belong to this turn, and ACP counts them.
    pub occurred_at_micros: i64,
    /// The session's working folder; its last part is the project, as for transcript rows.
    pub cwd: Option<String>,
}

struct Tokens { model: String, input: u64, output: u64, cache_read: u64, cache_write: u64, reasoning: u64 }

fn count(value: &Value, keys: &[&str]) -> u64 {
    keys.iter().find_map(|key| value.get(*key).and_then(Value::as_u64)).unwrap_or(0)
}

/// One entry per model from `_meta.quota.model_usage`, or the standard ACP
/// `usage` under the session's model when the agent gives no breakdown.
fn turn_tokens(turn: &AcpTurn) -> Vec<Tokens> {
    let read = |model: &str, usage: &Value| Tokens {
        model: model.to_string(),
        input: count(usage, &["inputTokens"]),
        output: count(usage, &["outputTokens"]),
        cache_read: count(usage, &["cachedReadTokens", "cachedInputTokens"]),
        cache_write: count(usage, &["cachedWriteTokens", "cacheWriteInputTokens"]),
        reasoning: count(usage, &["thoughtTokens", "reasoningOutputTokens"]),
    };
    let models = turn.result.pointer("/_meta/quota/model_usage").and_then(Value::as_array);
    let tokens: Vec<Tokens> = match models {
        Some(models) => models.iter().filter_map(|entry| {
            Some(read(entry.get("model")?.as_str()?, entry.get("token_count")?))
        }).collect(),
        None => turn.result.get("usage").filter(|usage| usage.is_object())
            .map(|usage| read(turn.model.as_deref().unwrap_or("unknown"), usage)).into_iter().collect(),
    };
    tokens.into_iter().filter(|t| t.input + t.output + t.cache_read + t.cache_write > 0).collect()
}

impl UsageDb {
    /// Copies the rates into `usage_rates`, replacing what was there.
    pub fn rebuild_rates(&self, rates: &[(&str, f64, f64, f64, f64)]) -> Result<(), String> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error("could not start rates rebuild", error))?;
        transaction.execute("DELETE FROM usage_rates", [])
            .map_err(|error| self.database_error("could not clear usage rates", error))?;
        for (model, input, output, cache_read, cache_write) in rates {
            transaction.execute("INSERT INTO usage_rates (model, input_rate, output_rate, cache_read_rate, cache_write_rate) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![model, input, output, cache_read, cache_write])
                .map_err(|error| self.database_error("could not write usage rate", error))?;
        }
        transaction.commit().map_err(|error| self.database_error("could not finish rates rebuild", error))
    }

    /// Writes one row per model the turn used. The reported cost is a running
    /// total, so the turn's share is that total less the session's previous
    /// total; both go on the turn's largest row. A total below the previous one
    /// means the provider restarted its count, so the whole total is the share.
    /// Transcript rows for the same native session written during the turn are
    /// dropped, since ACP counts it; rows of any other turn stay.
    pub fn insert_acp_turn(&self, turn: &AcpTurn) -> Result<usize, String> {
        let tokens = turn_tokens(turn);
        if tokens.is_empty() {
            return Ok(0);
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error("could not start ACP usage write", error))?;
        let cost = turn.cost_total_usd.map(|total| (total * 1_000_000.0).round() as i64)
            .map(|total| -> rusqlite::Result<(i64, i64)> {
                let previous: i64 = transaction.query_row(
                    "SELECT COALESCE((SELECT reported_cost_total_micros FROM usage_events WHERE owned_id = ?1 AND source_kind = ?2 AND reported_cost_total_micros IS NOT NULL ORDER BY id DESC LIMIT 1), 0)",
                    params![turn.owned_id, ACP_SOURCE_KIND], |row| row.get(0))?;
                Ok((if total >= previous { total - previous } else { total }, total))
            }).transpose().map_err(|error| self.database_error("could not read reported cost", error))?;
        let largest = (0..tokens.len()).max_by_key(|&i| {
            let t = &tokens[i];
            t.input + t.output + t.cache_read + t.cache_write
        });
        let source_key = turn.native_session_id.clone().unwrap_or_else(|| turn.owned_id.clone());
        let project_id = turn.cwd.as_deref().and_then(|cwd| std::path::Path::new(cwd).file_name()).map(|name| name.to_string_lossy().to_string());
        let written_at = super::now_micros();
        for (index, t) in tokens.iter().enumerate() {
            transaction.execute(
                "INSERT INTO usage_events (provider, provider_instance_id, owned_id, turn_id, project_id, workspace_id, node_run_id, occurred_at_micros, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, reasoning_tokens, model, reported_cost_micros, reported_cost_total_micros, source_kind, source_event_id, source_key, inserted_at_micros) VALUES (?1, 'local', ?2, ?3, ?17, ?18, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?16, ?13, ?14, ?15, ?19) ON CONFLICT(provider, provider_instance_id, source_kind, source_event_id) DO NOTHING",
                params![turn.provider, turn.owned_id, turn.turn_id, turn.node_run_id, turn.occurred_at_micros,
                    t.input as i64, t.output as i64, t.cache_read as i64, t.cache_write as i64, t.reasoning as i64, t.model,
                    cost.filter(|_| Some(index) == largest).map(|(share, _)| share), ACP_SOURCE_KIND,
                    format!("{}:{}:{}", turn.owned_id, turn.turn_id, t.model), source_key,
                    cost.filter(|_| Some(index) == largest).map(|(_, total)| total), project_id, turn.cwd, written_at],
            ).map_err(|error| self.database_error("could not write ACP usage", error))?;
        }
        transaction.execute("DELETE FROM usage_events WHERE owned_id = ?1 AND source_kind <> ?2 AND occurred_at_micros BETWEEN ?3 AND ?4", params![source_key, ACP_SOURCE_KIND, turn.occurred_at_micros, written_at])
            .map_err(|error| self.database_error("could not drop transcript usage", error))?;
        transaction.commit().map_err(|error| self.database_error("could not finish ACP usage write", error))?;
        Ok(tokens.len())
    }
}

/// Brings a database made before ACP rows up to date: the new columns, and
/// Codex transcript rows, whose input used to include cached input, stored as
/// uncached input like every other row.
pub(super) fn upgrade(transaction: &Transaction) -> rusqlite::Result<()> {
    let missing = |column: &str| -> rusqlite::Result<bool> {
        transaction.query_row("SELECT COUNT(*) = 0 FROM pragma_table_info('usage_events') WHERE name = ?1", [column], |row| row.get(0))
    };
    let first_upgrade = missing("node_run_id")?;
    for (column, kind) in [("node_run_id", "TEXT"), ("reported_cost_micros", "INTEGER"), ("reported_cost_total_micros", "INTEGER")] {
        if missing(column)? {
            transaction.execute_batch(&format!("ALTER TABLE usage_events ADD COLUMN {column} {kind};"))?;
        }
    }
    if first_upgrade {
        transaction.execute(
            "UPDATE usage_events SET input_tokens = MAX(input_tokens - cache_read_tokens - cache_write_tokens, 0) WHERE source_kind = ?1",
            ["codex-jsonl"])?;
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::super::{UsageEvent, UsageFilter, USAGE_SCHEMA};
    use super::*;
    use serde_json::json;

    fn open() -> (std::path::PathBuf, UsageDb) {
        let path = std::env::temp_dir().join(format!("mcb-usage-acp-{}.sqlite3", uuid::Uuid::new_v4()));
        let db = UsageDb::open(&path).expect("usage database should open");
        (path, db)
    }

    fn model(name: &str, input: u64, cache_read: u64, cache_write: u64, output: u64) -> Value {
        json!({"model": name, "token_count": {"inputTokens": input, "cachedInputTokens": cache_read, "cachedWriteTokens": cache_write, "outputTokens": output, "reasoningOutputTokens": 0}})
    }

    fn turn(turn_id: &str, models: Vec<Value>, cost: Option<f64>) -> AcpTurn {
        AcpTurn {
            owned_id: "assembly-1".into(), native_session_id: Some("native-1".into()), provider: "claude".into(),
            turn_id: turn_id.into(), result: json!({"stopReason": "end_turn", "_meta": {"quota": {"model_usage": models}}}),
            cost_total_usd: cost, node_run_id: Some("run-1:build:1".into()), occurred_at_micros: 1_700_000_000_000_000,
            cwd: Some("/work/alpha".into()),
            ..AcpTurn::default()
        }
    }

    /// (turn, model, input, cache read, cache write, reported cost, node run) for every row, in order.
    fn rows(db: &UsageDb) -> Vec<(String, String, i64, i64, i64, Option<i64>, Option<String>)> {
        let connection = db.connection().unwrap();
        let mut statement = connection.prepare("SELECT turn_id, model, input_tokens, cache_read_tokens, cache_write_tokens, reported_cost_micros, node_run_id FROM usage_events ORDER BY id").unwrap();
        statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)))
            .unwrap().collect::<rusqlite::Result<_>>().unwrap()
    }

    // Recorded from claude-agent-acp 0.94.0 (TSK-1595 check 4): the cost is the session's running
    // total, and it carries on after the adapter restarts and loads the session for turn 3.
    #[test]
    fn a_running_cost_total_becomes_each_turns_share() {
        let (path, db) = open();
        db.insert_acp_turn(&turn("turn-1", vec![model("claude-haiku-5-5", 1175, 0, 0, 19), model("claude-opus-5-5", 2, 10482, 10191, 5)], Some(0.0838594))).unwrap();
        db.insert_acp_turn(&turn("turn-2", vec![model("claude-opus-5-5", 2, 12823, 12793, 5)], Some(0.188876))).unwrap();
        db.insert_acp_turn(&turn("turn-3", vec![model("claude-opus-5-5", 2, 20690, 5863, 5)], Some(0.240026))).unwrap();
        let run = Some("run-1:build:1".to_string());
        assert_eq!(rows(&db), vec![
            ("turn-1".into(), "claude-haiku-5-5".into(), 1175, 0, 0, None, run.clone()),
            ("turn-1".into(), "claude-opus-5-5".into(), 2, 10482, 10191, Some(83_859), run.clone()),
            ("turn-2".into(), "claude-opus-5-5".into(), 2, 12823, 12793, Some(105_017), run.clone()),
            ("turn-3".into(), "claude-opus-5-5".into(), 2, 20690, 5863, Some(51_150), run),
        ]);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn after_the_provider_restarts_its_count_each_turn_is_measured_from_the_new_count() {
        let (path, db) = open();
        for (turn_id, total) in [("turn-1", 0.2), ("turn-2", 0.05), ("turn-3", 0.08)] {
            db.insert_acp_turn(&turn(turn_id, vec![model("claude-opus-5-5", 2, 10, 10, 5)], Some(total))).unwrap();
        }
        assert_eq!(rows(&db).iter().map(|row| row.5).collect::<Vec<_>>(), [Some(200_000), Some(50_000), Some(30_000)]);
        let _ = std::fs::remove_file(path);
    }

    // Recorded from codex-acp 2.2.2: per-turn counts, its own name for cache writes, no cost.
    #[test]
    fn per_turn_usage_without_a_cost_is_stored_unpriced() {
        let (path, db) = open();
        let mut codex = turn("turn-1", vec![], None);
        codex.provider = "codex".into();
        codex.result = json!({"stopReason": "end_turn", "usage": {"totalTokens": 16045, "inputTokens": 3752, "cachedReadTokens": 12288, "cachedWriteTokens": 0, "outputTokens": 5, "thoughtTokens": 0},
            "_meta": {"quota": {"model_usage": [{"model": "gpt-6.1-sol", "token_count": {"totalTokens": 16045, "inputTokens": 3752, "cachedInputTokens": 12288, "cacheWriteInputTokens": 7, "outputTokens": 5, "reasoningOutputTokens": 0}}]}}});
        assert_eq!(db.insert_acp_turn(&codex).unwrap(), 1);
        assert_eq!(rows(&db), vec![("turn-1".into(), "gpt-6.1-sol".into(), 3752, 12288, 7, None, Some("run-1:build:1".into()))]);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn standard_usage_is_used_when_there_is_no_breakdown_and_no_usage_writes_nothing() {
        let (path, db) = open();
        let mut plain = turn("turn-1", vec![], None);
        plain.model = Some("some-model".into());
        plain.result = json!({"stopReason": "end_turn", "usage": {"inputTokens": 9, "outputTokens": 3, "cachedReadTokens": 4}});
        assert_eq!(db.insert_acp_turn(&plain).unwrap(), 1);
        // agy-acp 1.4.1 answers a prompt with the stop reason alone.
        plain.turn_id = "turn-2".into();
        plain.result = json!({"stopReason": "end_turn"});
        assert_eq!(db.insert_acp_turn(&plain).unwrap(), 0);
        assert_eq!(rows(&db), vec![("turn-1".into(), "some-model".into(), 9, 4, 0, None, Some("run-1:build:1".into()))]);
        let _ = std::fs::remove_file(path);
    }

    fn transcript_row(id: &str, owned_id: &str, occurred_at_micros: i64) -> UsageEvent {
        UsageEvent {
            provider: "claude".into(), provider_instance_id: "local".into(), owned_id: Some(owned_id.into()),
            workflow_id: None, turn_id: None, project_id: None, workspace_id: None, occurred_at_micros,
            input_tokens: 5, output_tokens: 5, cache_read_tokens: 0, cache_write_tokens: 0, reasoning_tokens: 0,
            model: "claude-opus-5-5".into(), estimated_cost_micros: None, estimate_rate_version: None,
            source_kind: "claude-jsonl".into(), source_event_id: id.into(), source_key: "transcript-file".into(),
        }
    }

    // The turn starts at 1_700_000_000_000_000 (see `turn`); its transcript lines come after that.
    #[test]
    fn only_transcript_rows_written_during_an_acp_turn_are_replaced() {
        let (path, db) = open();
        let start = 1_700_000_000_000_000;
        // A later turn with no ACP row (one that ended in an error) keeps its transcript usage.
        let later = super::super::now_micros() + 3_600_000_000;
        db.insert_events_with_cursor(&[transcript_row("old-1", "native-1", start - 2_000_000), transcript_row("old-2", "native-1", start - 1_000_000),
            transcript_row("other", "native-2", start + 1), transcript_row("this-turn-early", "native-1", start + 1)], None).unwrap();
        db.insert_acp_turn(&turn("turn-1", vec![model("claude-opus-5-5", 2, 10, 10, 5)], None)).unwrap();
        db.insert_events_with_cursor(&[transcript_row("this-turn-late", "native-1", start + 2), transcript_row("old-3", "native-1", start - 500_000),
            transcript_row("later-turn", "native-1", later)], None).unwrap();
        let connection = db.connection().unwrap();
        let mut statement = connection.prepare("SELECT source_event_id FROM usage_events WHERE source_kind = 'claude-jsonl' ORDER BY id").unwrap();
        let kept: Vec<String> = statement.query_map([], |row| row.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        assert_eq!(kept, ["old-1", "old-2", "other", "old-3", "later-turn"]);
        // Five transcript rows (5 in, 5 out each), plus the ACP turn once, under its project.
        let summary = db.read_usage_summary(&UsageFilter { project_id: Some("alpha".into()), ..UsageFilter::default() }).unwrap();
        assert_eq!((summary.input_tokens, summary.output_tokens, summary.cache_read_tokens), (2, 5, 10));
        let summary = db.read_usage_summary(&UsageFilter::default()).unwrap();
        assert_eq!((summary.input_tokens, summary.output_tokens, summary.cache_read_tokens), (5 * 5 + 2, 5 * 5 + 5, 10));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn estimated_cost_joins_the_rates_table() {
        let (path, db) = open();
        db.rebuild_rates(SEEDED_RATES).unwrap();
        let mut priced = transcript_row("priced", "native-9", 1);
        (priced.model, priced.input_tokens, priced.output_tokens) = ("gpt-5.6-sol".into(), 10, 4);
        db.insert_events_with_cursor(&[priced], None).unwrap();
        let rows = db.read_usage_provider_summary(&UsageFilter::default()).unwrap();
        assert_eq!((rows[0].estimated_cost_micros, rows[0].unpriced_percent), (Some(170), 0));
        db.rebuild_rates(&[]).unwrap();
        let rows = db.read_usage_provider_summary(&UsageFilter::default()).unwrap();
        assert_eq!((rows[0].estimated_cost_micros, rows[0].unpriced_percent), (None, 100));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn an_older_database_gets_the_columns_and_uncached_codex_input() {
        let path = std::env::temp_dir().join(format!("mcb-usage-acp-{}.sqlite3", uuid::Uuid::new_v4()));
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection.execute_batch(&USAGE_SCHEMA.replace("    node_run_id TEXT,\n    reported_cost_micros INTEGER,\n    -- The provider's running cost total the share was taken from.\n    reported_cost_total_micros INTEGER,\n", "")).unwrap();
        connection.execute("INSERT INTO usage_events (provider, provider_instance_id, occurred_at_micros, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, reasoning_tokens, model, source_kind, source_event_id, source_key, inserted_at_micros) VALUES ('codex', 'local', 1, 100, 1, 80, 5, 0, 'gpt-5.6-sol', 'codex-jsonl', 'e', 'k', 1)", []).unwrap();
        drop(connection);
        let db = UsageDb::open(&path).expect("an older database should upgrade");
        UsageDb::open(&path).expect("an upgraded database should open again");
        let input: i64 = db.connection().unwrap().query_row("SELECT input_tokens FROM usage_events", [], |row| row.get(0)).unwrap();
        assert_eq!(input, 15);
        db.insert_acp_turn(&turn("turn-1", vec![model("claude-opus-5-5", 2, 10, 10, 5)], Some(0.1))).unwrap();
        let _ = std::fs::remove_file(path);
    }
}
