use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const USAGE_SCHEMA: &str = include_str!("../../migrations/0001_usage_history.sql");

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UsageEvent {
    pub provider: String,
    pub provider_instance_id: String,
    pub owned_id: Option<String>,
    pub workflow_id: Option<String>,
    pub turn_id: Option<String>,
    pub project_id: Option<String>,
    pub workspace_id: Option<String>,
    pub occurred_at_micros: i64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub reasoning_tokens: u64,
    pub model: String,
    pub estimated_cost_micros: Option<i64>,
    pub estimate_rate_version: Option<String>,
    pub source_kind: String,
    pub source_event_id: String,
    pub source_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UsageSourceCursor {
    pub provider: String,
    pub provider_instance_id: String,
    pub source_kind: String,
    pub source_key: String,
    pub file_identity: String,
    pub offset: u64,
    pub size: u64,
    pub modified_at_micros: i64,
    pub last_event_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UsageRateVersion {
    pub provider: String,
    pub model: String,
    pub effective_from_micros: i64,
    pub effective_to_micros: Option<i64>,
    pub input_micros_per_million: i64,
    pub output_micros_per_million: i64,
    pub cache_read_micros_per_million: i64,
    pub cache_write_micros_per_million: i64,
    pub reasoning_micros_per_million: i64,
    pub version: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UsageFilter {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
    pub workflow_id: Option<String>,
    pub start_micros: Option<i64>,
    pub end_micros: Option<i64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub event_count: u64,
    pub provider_count: u64,
    pub active_days: u64,
    pub session_count: u64,
    pub turn_count: u64,
    pub workflow_count: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub reasoning_tokens: u64,
    pub estimated_cost_micros: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UsageBreakdownRow {
    pub provider: String,
    pub model: String,
    pub project_id: Option<String>,
    pub event_count: u64,
    pub session_count: u64,
    pub turn_count: u64,
    pub workflow_count: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_count: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UsageDailyRow {
    pub day: String,
    pub provider: String,
    pub model: String,
    pub project_id: Option<String>,
    pub event_count: u64,
    pub session_count: u64,
    pub turn_count: u64,
    pub workflow_count: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub estimated_cost_micros: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct UsageDb {
    path: PathBuf,
}

impl UsageDb {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, String> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("Could not create usage database folder: {error}"))?;
        }
        let db = Self { path };
        db.execute(&format!("BEGIN IMMEDIATE;\n{USAGE_SCHEMA}\nCOMMIT;"))?;
        Ok(db)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn insert_events(&self, events: &[UsageEvent]) -> Result<(), String> {
        self.insert_events_with_cursor(events, None)
    }

    pub fn insert_events_with_cursor(
        &self,
        events: &[UsageEvent],
        cursor: Option<&UsageSourceCursor>,
    ) -> Result<(), String> {
        let mut sql = String::from("BEGIN IMMEDIATE;\n");
        for event in events {
            validate_event(event)?;
            sql.push_str(&format!(
                "INSERT OR IGNORE INTO usage_events (provider, provider_instance_id, owned_id, workflow_id, turn_id, project_id, workspace_id, occurred_at_micros, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, reasoning_tokens, model, estimated_cost_micros, estimate_rate_version, source_kind, source_event_id, source_key, inserted_at_micros) VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {});\n",
                quote(&event.provider),
                quote(&event.provider_instance_id),
                quote_option(event.owned_id.as_deref()),
                quote_option(event.workflow_id.as_deref()),
                quote_option(event.turn_id.as_deref()),
                quote_option(event.project_id.as_deref()),
                quote_option(event.workspace_id.as_deref()),
                event.occurred_at_micros,
                event.input_tokens,
                event.output_tokens,
                event.cache_read_tokens,
                event.cache_write_tokens,
                event.reasoning_tokens,
                quote(&event.model),
                event.estimated_cost_micros.map_or_else(|| "NULL".to_string(), |value| value.to_string()),
                quote_option(event.estimate_rate_version.as_deref()),
                quote(&event.source_kind),
                quote(&event.source_event_id),
                quote(&event.source_key),
                now_micros(),
            ));
        }
        if let Some(cursor) = cursor {
            sql.push_str(&format!(
                "INSERT INTO usage_source_cursors (provider, provider_instance_id, source_kind, source_key, file_identity, offset_bytes, size_bytes, modified_at_micros, last_event_id, updated_at_micros) VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}) ON CONFLICT(provider, provider_instance_id, source_kind, source_key) DO UPDATE SET file_identity=excluded.file_identity, offset_bytes=excluded.offset_bytes, size_bytes=excluded.size_bytes, modified_at_micros=excluded.modified_at_micros, last_event_id=excluded.last_event_id, updated_at_micros=excluded.updated_at_micros;\n",
                quote(&cursor.provider),
                quote(&cursor.provider_instance_id),
                quote(&cursor.source_kind),
                quote(&cursor.source_key),
                quote(&cursor.file_identity),
                cursor.offset,
                cursor.size,
                cursor.modified_at_micros,
                quote_option(cursor.last_event_id.as_deref()),
                now_micros()
            ));
        }
        if events.is_empty() {
            sql.push_str("COMMIT;\n");
            return self.execute(&sql);
        }
        // The rollup is rebuilt with SQL, not by loading events into Rust.
        sql.push_str("DELETE FROM usage_daily_rollups;\nINSERT INTO usage_daily_rollups (day, provider, model, project_id, event_count, session_count, turn_count, workflow_count, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, reasoning_tokens, estimated_cost_micros) SELECT date(occurred_at_micros / 1000000, 'unixepoch'), provider, model, project_id, COUNT(*), COUNT(DISTINCT owned_id), COUNT(DISTINCT turn_id), COUNT(DISTINCT workflow_id), SUM(input_tokens), SUM(output_tokens), SUM(cache_read_tokens), SUM(cache_write_tokens), SUM(reasoning_tokens), SUM(estimated_cost_micros) FROM usage_events GROUP BY date(occurred_at_micros / 1000000, 'unixepoch'), provider, model, project_id;\nCOMMIT;\n");
        self.execute(&sql)
    }

    pub fn upsert_cursor(&self, cursor: &UsageSourceCursor) -> Result<(), String> {
        let sql = format!(
            "INSERT INTO usage_source_cursors (provider, provider_instance_id, source_kind, source_key, file_identity, offset_bytes, size_bytes, modified_at_micros, last_event_id, updated_at_micros) VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}) ON CONFLICT(provider, provider_instance_id, source_kind, source_key) DO UPDATE SET file_identity=excluded.file_identity, offset_bytes=excluded.offset_bytes, size_bytes=excluded.size_bytes, modified_at_micros=excluded.modified_at_micros, last_event_id=excluded.last_event_id, updated_at_micros=excluded.updated_at_micros;",
            quote(&cursor.provider), quote(&cursor.provider_instance_id), quote(&cursor.source_kind), quote(&cursor.source_key), quote(&cursor.file_identity), cursor.offset, cursor.size, cursor.modified_at_micros, quote_option(cursor.last_event_id.as_deref()), now_micros()
        );
        self.execute(&sql)
    }

    pub fn insert_rate_version(&self, rate: &UsageRateVersion) -> Result<(), String> {
        if rate.provider.trim().is_empty() || rate.model.trim().is_empty() || rate.version.trim().is_empty() {
            return Err("Usage rates need provider, model, and version".to_string());
        }
        let sql = format!(
            "INSERT OR REPLACE INTO usage_rate_versions (provider, model, effective_from_micros, effective_to_micros, input_micros_per_million, output_micros_per_million, cache_read_micros_per_million, cache_write_micros_per_million, reasoning_micros_per_million, version) VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {});",
            quote(&rate.provider), quote(&rate.model), rate.effective_from_micros,
            rate.effective_to_micros.map_or_else(|| "NULL".to_string(), |value| value.to_string()),
            rate.input_micros_per_million, rate.output_micros_per_million,
            rate.cache_read_micros_per_million, rate.cache_write_micros_per_million,
            rate.reasoning_micros_per_million, quote(&rate.version)
        );
        self.execute(&sql)
    }

    /// Estimate only when one versioned rate covers the event timestamp. The
    /// caller labels the result with the returned rate version.
    pub fn estimate_event_cost_micros(&self, event: &UsageEvent) -> Result<Option<(i64, String)>, String> {
        let sql = format!(
            "SELECT input_micros_per_million, output_micros_per_million, cache_read_micros_per_million, cache_write_micros_per_million, reasoning_micros_per_million, version FROM usage_rate_versions WHERE provider = {} AND model = {} AND effective_from_micros <= {} AND (effective_to_micros IS NULL OR effective_to_micros > {}) ORDER BY effective_from_micros DESC LIMIT 1",
            quote(&event.provider), quote(&event.model), event.occurred_at_micros, event.occurred_at_micros
        );
        let Some(row) = self.query(&sql)?.first().cloned() else { return Ok(None); };
        let total = |tokens: u64, key: &str| -> Result<i64, String> {
            let rate = row.get(key).and_then(Value::as_i64).unwrap_or_default();
            let value = (tokens as i128).checked_mul(rate as i128).ok_or_else(|| "Usage cost overflow".to_string())? / 1_000_000;
            i64::try_from(value).map_err(|_| "Usage cost overflow".to_string())
        };
        let cost = total(event.input_tokens, "input_micros_per_million")?
            .saturating_add(total(event.output_tokens, "output_micros_per_million")?)
            .saturating_add(total(event.cache_read_tokens, "cache_read_micros_per_million")?)
            .saturating_add(total(event.cache_write_tokens, "cache_write_micros_per_million")?)
            .saturating_add(total(event.reasoning_tokens, "reasoning_micros_per_million")?);
        Ok(Some((cost, value_string(&row, "version"))))
    }

    pub fn read_usage_summary(&self, filter: &UsageFilter) -> Result<UsageSummary, String> {
        let rows = self.query(&summary_sql(filter))?;
        let row = rows.first().cloned().unwrap_or(Value::Null);
        Ok(UsageSummary {
            event_count: value_u64(&row, "event_count"),
            provider_count: value_u64(&row, "provider_count"),
            active_days: value_u64(&row, "active_days"),
            session_count: value_u64(&row, "session_count"),
            turn_count: value_u64(&row, "turn_count"),
            workflow_count: value_u64(&row, "workflow_count"),
            input_tokens: value_u64(&row, "input_tokens"),
            output_tokens: value_u64(&row, "output_tokens"),
            cache_read_tokens: value_u64(&row, "cache_read_tokens"),
            cache_write_tokens: value_u64(&row, "cache_write_tokens"),
            reasoning_tokens: value_u64(&row, "reasoning_tokens"),
            estimated_cost_micros: value_i64(&row, "estimated_cost_micros"),
        })
    }

    pub fn read_usage_breakdown(&self, filter: &UsageFilter) -> Result<Vec<UsageBreakdownRow>, String> {
        self.query(&breakdown_sql(filter))?.into_iter().map(|row| {
            Ok(UsageBreakdownRow {
                provider: value_string(&row, "provider"),
                model: value_string(&row, "model"),
                project_id: value_optional_string(&row, "project_id"),
                event_count: value_u64(&row, "event_count"),
                session_count: value_u64(&row, "session_count"),
                turn_count: value_u64(&row, "turn_count"),
                workflow_count: value_u64(&row, "workflow_count"),
                input_tokens: value_u64(&row, "input_tokens"),
                output_tokens: value_u64(&row, "output_tokens"),
                total_count: value_u64(&row, "total_count"),
            })
        }).collect()
    }

    pub fn read_daily(&self, filter: &UsageFilter) -> Result<Vec<UsageDailyRow>, String> {
        self.query(&daily_sql(filter))?.into_iter().map(|row| {
            Ok(UsageDailyRow {
                day: value_string(&row, "day"),
                provider: value_string(&row, "provider"),
                model: value_string(&row, "model"),
                project_id: value_optional_string(&row, "project_id"),
                event_count: value_u64(&row, "event_count"),
                session_count: value_u64(&row, "session_count"),
                turn_count: value_u64(&row, "turn_count"),
                workflow_count: value_u64(&row, "workflow_count"),
                input_tokens: value_u64(&row, "input_tokens"),
                output_tokens: value_u64(&row, "output_tokens"),
                estimated_cost_micros: value_i64(&row, "estimated_cost_micros"),
            })
        }).collect()
    }

    pub fn explain_breakdown(&self, filter: &UsageFilter) -> Result<Vec<Value>, String> {
        let sql = format!("EXPLAIN QUERY PLAN {}", breakdown_sql(filter));
        trace_sql(&sql);
        let output = sqlite_command()
            .arg("-batch")
            .arg(&self.path)
            .arg(&sql)
            .output()
            .map_err(|error| format!("SQLite is unavailable: {error}"))?;
        if !output.status.success() {
            return Err(format!("SQLite query plan failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| Value::String(line.to_string()))
            .collect())
    }

    fn execute(&self, sql: &str) -> Result<(), String> {
        trace_sql(sql);
        let output = sqlite_command().arg("-batch").arg(&self.path).arg(sql).output().map_err(|error| format!("SQLite is unavailable: {error}"))?;
        if output.status.success() { Ok(()) } else { Err(format!("SQLite command failed: {}", String::from_utf8_lossy(&output.stderr).trim())) }
    }

    fn query(&self, sql: &str) -> Result<Vec<Value>, String> {
        trace_sql(sql);
        let output = sqlite_command().arg("-batch").arg("-json").arg(&self.path).arg(sql).output().map_err(|error| format!("SQLite is unavailable: {error}"))?;
        if !output.status.success() { return Err(format!("SQLite query failed: {}", String::from_utf8_lossy(&output.stderr).trim())); }
        let text = String::from_utf8_lossy(&output.stdout);
        if text.trim().is_empty() { return Ok(Vec::new()); }
        serde_json::from_str(text.trim()).map_err(|error| format!("SQLite JSON output was invalid: {error}"))
    }
}

fn sqlite_command() -> Command {
    let binary = std::env::var_os("MCB_SQLITE_BIN").unwrap_or_else(|| "sqlite3".into());
    let mut command = Command::new(binary);
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    command
}

fn validate_event(event: &UsageEvent) -> Result<(), String> {
    if event.provider.trim().is_empty() || event.provider_instance_id.trim().is_empty() || event.model.trim().is_empty() || event.source_kind.trim().is_empty() || event.source_event_id.trim().is_empty() {
        return Err("Usage events need provider, instance, model, source kind, and source event id".to_string());
    }
    Ok(())
}

fn summary_sql(filter: &UsageFilter) -> String {
    format!("SELECT COUNT(*) AS event_count, COUNT(DISTINCT provider) AS provider_count, COUNT(DISTINCT date(occurred_at_micros / 1000000, 'unixepoch')) AS active_days, COUNT(DISTINCT owned_id) AS session_count, COUNT(DISTINCT turn_id) AS turn_count, COUNT(DISTINCT workflow_id) AS workflow_count, COALESCE(SUM(input_tokens), 0) AS input_tokens, COALESCE(SUM(output_tokens), 0) AS output_tokens, COALESCE(SUM(cache_read_tokens), 0) AS cache_read_tokens, COALESCE(SUM(cache_write_tokens), 0) AS cache_write_tokens, COALESCE(SUM(reasoning_tokens), 0) AS reasoning_tokens, SUM(estimated_cost_micros) AS estimated_cost_micros FROM usage_events WHERE {}", where_sql(filter))
}

fn breakdown_sql(filter: &UsageFilter) -> String {
    let limit = filter.limit.unwrap_or(20).clamp(1, 200);
    let offset = filter.offset.unwrap_or(0).min(100_000);
    format!("WITH filtered AS (SELECT provider, model, project_id, owned_id, turn_id, workflow_id, input_tokens, output_tokens FROM usage_events WHERE {where_sql}), grouped AS (SELECT provider, model, project_id, COUNT(*) AS event_count, COUNT(DISTINCT owned_id) AS session_count, COUNT(DISTINCT turn_id) AS turn_count, COUNT(DISTINCT workflow_id) AS workflow_count, SUM(input_tokens) AS input_tokens, SUM(output_tokens) AS output_tokens FROM filtered GROUP BY provider, model, project_id) SELECT provider, model, project_id, event_count, session_count, turn_count, workflow_count, input_tokens, output_tokens, COUNT(*) OVER () AS total_count FROM grouped ORDER BY provider ASC, model ASC, project_id ASC LIMIT {limit} OFFSET {offset}", where_sql = where_sql(filter), limit = limit, offset = offset)
}

fn daily_sql(filter: &UsageFilter) -> String {
    let limit = filter.limit.unwrap_or(31).clamp(1, 200);
    let offset = filter.offset.unwrap_or(0).min(100_000);
    format!("SELECT day, provider, model, project_id, event_count, session_count, turn_count, workflow_count, input_tokens, output_tokens, estimated_cost_micros FROM usage_daily_provider_model WHERE {} ORDER BY day DESC, provider ASC, model ASC, project_id ASC LIMIT {} OFFSET {}", daily_where_sql(filter), limit, offset)
}

fn where_sql(filter: &UsageFilter) -> String {
    let mut parts = vec!["1 = 1".to_string()];
    if let Some(provider) = &filter.provider { parts.push(format!("provider = {}", quote(provider))); }
    if let Some(model) = &filter.model { parts.push(format!("model = {}", quote(model))); }
    if let Some(project_id) = &filter.project_id { parts.push(format!("project_id = {}", quote(project_id))); }
    if let Some(workflow_id) = &filter.workflow_id { parts.push(format!("workflow_id = {}", quote(workflow_id))); }
    if let Some(start) = filter.start_micros { parts.push(format!("occurred_at_micros >= {}", start)); }
    if let Some(end) = filter.end_micros { parts.push(format!("occurred_at_micros < {}", end)); }
    parts.join(" AND ")
}

fn daily_where_sql(filter: &UsageFilter) -> String {
    let mut parts = vec!["1 = 1".to_string()];
    if let Some(provider) = &filter.provider { parts.push(format!("provider = {}", quote(provider))); }
    if let Some(model) = &filter.model { parts.push(format!("model = {}", quote(model))); }
    if let Some(project_id) = &filter.project_id { parts.push(format!("project_id = {}", quote(project_id))); }
    if let Some(start) = filter.start_micros { parts.push(format!("day >= date({} / 1000000, 'unixepoch')", start)); }
    if let Some(end) = filter.end_micros { parts.push(format!("day < date({} / 1000000, 'unixepoch')", end)); }
    parts.join(" AND ")
}

fn quote(value: &str) -> String { format!("'{}'", value.replace('\'', "''")) }
fn quote_option(value: Option<&str>) -> String { value.map(quote).unwrap_or_else(|| "NULL".to_string()) }
fn trace_sql(sql: &str) {
    if std::env::var("MCB_SQL_TRACE").as_deref() == Ok("1") {
        eprintln!("MCB_SQL_TRACE {}", redact_sql_literals(sql).replace('\n', " "));
    }
}

fn redact_sql_literals(sql: &str) -> String {
    let mut redacted = String::with_capacity(sql.len());
    let mut in_literal = false;
    let mut chars = sql.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '\'' {
            if !in_literal {
                redacted.push(character);
            }
            continue;
        }
        if !in_literal {
            in_literal = true;
            redacted.push_str("'?'");
        } else if chars.peek() == Some(&'\'') {
            let _ = chars.next();
        } else {
            in_literal = false;
        }
    }
    redacted
}
fn now_micros() -> i64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|value| value.as_micros().min(i64::MAX as u128) as i64).unwrap_or_default() }
fn value_u64(row: &Value, key: &str) -> u64 { row.get(key).and_then(Value::as_u64).or_else(|| row.get(key).and_then(Value::as_i64).map(|value| value.max(0) as u64)).unwrap_or_default() }
fn value_i64(row: &Value, key: &str) -> Option<i64> { row.get(key).and_then(Value::as_i64) }
fn value_string(row: &Value, key: &str) -> String { row.get(key).and_then(Value::as_str).unwrap_or_default().to_string() }
fn value_optional_string(row: &Value, key: &str) -> Option<String> { row.get(key).and_then(Value::as_str).map(ToString::to_string) }

pub fn summary_sql_for_test(filter: &UsageFilter) -> String { summary_sql(filter) }
pub fn breakdown_sql_for_test(filter: &UsageFilter) -> String { breakdown_sql(filter) }

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_db_path() -> PathBuf {
        std::env::temp_dir().join(format!("mcb-usage-test-{}.sqlite3", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()))
    }

    fn event(id: &str) -> UsageEvent {
        UsageEvent { provider: "provider-a".into(), provider_instance_id: "instance-a".into(), owned_id: None, workflow_id: None, turn_id: None, project_id: Some("project-a".into()), workspace_id: None, occurred_at_micros: 1_700_000_000_000_000, input_tokens: 10, output_tokens: 4, cache_read_tokens: 0, cache_write_tokens: 0, reasoning_tokens: 0, model: "model-a".into(), estimated_cost_micros: None, estimate_rate_version: None, source_kind: "acp".into(), source_event_id: id.into(), source_key: "session-a".into() }
    }

    #[test]
    fn usage_history_deduplicates_events_and_rebuilds_rollups() {
        let path = test_db_path();
        let db = UsageDb::open(&path).expect("sqlite database should open");
        db.insert_events(&[event("event-1"), event("event-1")]).expect("events should insert");
        let summary = db.read_usage_summary(&UsageFilter::default()).expect("summary should query");
        assert_eq!(summary.event_count, 1);
        assert_eq!(db.read_daily(&UsageFilter::default()).unwrap().len(), 1);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn usage_queries_are_db_side_and_bounded() {
        let sql = breakdown_sql_for_test(&UsageFilter { limit: Some(20), offset: Some(40), ..UsageFilter::default() });
        assert!(sql.contains("GROUP BY"));
        assert!(sql.contains("COUNT(*) OVER ()"));
        assert!(sql.contains("LIMIT 20 OFFSET 40"));
    }

    #[test]
    fn usage_breakdown_plan_uses_covering_provider_index() {
        let path = test_db_path();
        let db = UsageDb::open(&path).expect("sqlite database should open");
        let plan = db
            .explain_breakdown(&UsageFilter { provider: Some("provider-a".into()), ..UsageFilter::default() })
            .expect("sqlite should explain the bounded breakdown query");
        let plan_text = serde_json::to_string(&plan).expect("query plan should serialize");
        assert!(plan_text.contains("usage_events_provider_model_time_idx"), "query plan did not use the provider index: {plan_text}");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn usage_rates_estimate_only_with_a_matching_version() {
        let path = test_db_path();
        let db = UsageDb::open(&path).expect("sqlite database should open");
        db.insert_rate_version(&UsageRateVersion { provider: "provider-a".into(), model: "model-a".into(), effective_from_micros: 1_600_000_000_000_000, effective_to_micros: None, input_micros_per_million: 2_000_000, output_micros_per_million: 4_000_000, cache_read_micros_per_million: 0, cache_write_micros_per_million: 0, reasoning_micros_per_million: 0, version: "v1".into() }).unwrap();
        let event = event("rate-event");
        assert_eq!(db.estimate_event_cost_micros(&event).unwrap(), Some((36, "v1".into())));
        let mut unmatched = event;
        unmatched.occurred_at_micros = 1;
        assert_eq!(db.estimate_event_cost_micros(&unmatched).unwrap(), None);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn sql_trace_redacts_string_literals() {
        assert_eq!(redact_sql_literals("SELECT * FROM usage_events WHERE provider = 'private-provider'"), "SELECT * FROM usage_events WHERE provider = '?'" );
    }
}
