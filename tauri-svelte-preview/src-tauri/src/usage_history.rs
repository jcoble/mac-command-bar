use crate::usage_db::{
    UsageBreakdownRow, UsageDailyRow, UsageDailyTotalsRow, UsageDb, UsageEvent, UsageFilter,
    UsageProviderSummaryRow, UsageSummary,
};
use crate::usage_indexer::UsageIndexer;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Default)]
pub struct UsageHistoryState;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageHistoryQuery {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
    pub workflow_id: Option<String>,
    pub start_micros: Option<i64>,
    pub end_micros: Option<i64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl From<UsageHistoryQuery> for UsageFilter {
    fn from(value: UsageHistoryQuery) -> Self {
        Self {
            provider: value.provider,
            model: value.model,
            project_id: value.project_id,
            workflow_id: value.workflow_id,
            start_micros: value.start_micros,
            end_micros: value.end_micros,
            limit: value.limit,
            offset: value.offset,
        }
    }
}

#[tauri::command]
pub async fn read_usage_summary(
    app: AppHandle,
    query: UsageHistoryQuery,
) -> Result<UsageSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        UsageDb::open(usage_db_path(&app)?)?.read_usage_summary(&query.into())
    })
    .await
    .map_err(|error| format!("usage task failed: {error}"))?
}

#[tauri::command]
pub async fn read_usage_breakdown(
    app: AppHandle,
    query: UsageHistoryQuery,
) -> Result<Vec<UsageBreakdownRow>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        UsageDb::open(usage_db_path(&app)?)?.read_usage_breakdown(&query.into())
    })
    .await
    .map_err(|error| format!("usage task failed: {error}"))?
}

#[tauri::command]
pub async fn read_usage_provider_summary(
    app: AppHandle,
    query: UsageHistoryQuery,
) -> Result<Vec<UsageProviderSummaryRow>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        UsageDb::open(usage_db_path(&app)?)?.read_usage_provider_summary(&query.into())
    })
    .await
    .map_err(|error| format!("usage task failed: {error}"))?
}

#[tauri::command]
pub async fn read_usage_daily(
    app: AppHandle,
    query: UsageHistoryQuery,
) -> Result<Vec<UsageDailyRow>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        UsageDb::open(usage_db_path(&app)?)?.read_daily(&query.into())
    })
    .await
    .map_err(|error| format!("usage task failed: {error}"))?
}

#[tauri::command]
pub async fn read_usage_daily_totals(
    app: AppHandle,
    query: UsageHistoryQuery,
) -> Result<Vec<UsageDailyTotalsRow>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        UsageDb::open(usage_db_path(&app)?)?.read_usage_daily_totals(&query.into())
    })
    .await
    .map_err(|error| format!("usage task failed: {error}"))?
}

#[tauri::command]
pub async fn refresh_usage_history(app: AppHandle) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = usage_db_path(&app)?;
        UsageIndexer::new(path).ingest_local_sources()
    })
    .await
    .map_err(|error| format!("usage task failed: {error}"))?
}

/// Files one helper-model call in the same store the agents' own token counts
/// go to, so the Usage popover counts it without knowing the helper exists.
/// The vendor is the provider ("openai", "anthropic"), the job is the source
/// key, and the identifier is fresh every time so nothing is ever collapsed
/// into an earlier row.
///
/// The daily rollup table is left alone. Nothing reads it — every usage query
/// aggregates `usage_events` directly — and the indexer rebuilds it wholesale
/// on its next pass, so rebuilding it here would only put a full-table
/// aggregate in front of every title the app writes.
pub fn record_helper_call(
    app: &AppHandle,
    job: &str,
    vendor: &str,
    model: &str,
    input_tokens: u64,
    output_tokens: u64,
) -> Result<(), String> {
    let event = UsageEvent {
        provider: vendor.to_string(),
        provider_instance_id: "local".to_string(),
        owned_id: None,
        workflow_id: None,
        turn_id: None,
        project_id: None,
        workspace_id: None,
        occurred_at_micros: now_micros(),
        input_tokens,
        output_tokens,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        reasoning_tokens: 0,
        model: model.to_string(),
        estimated_cost_micros: None,
        estimate_rate_version: None,
        source_kind: "helper".to_string(),
        source_event_id: uuid::Uuid::new_v4().to_string(),
        source_key: format!("helper/{job}"),
    };
    UsageDb::open(usage_db_path(app)?)?.insert_events_with_cursor_deferred_rollup(&[event], None)
}

fn now_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_micros() as i64)
        .unwrap_or_default()
}

pub fn usage_db_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    if let Some(path) = std::env::var_os("MAC_COMMAND_BAR_USAGE_DB") {
        return Ok(path.into());
    }
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Application data directory is unavailable: {error}"))?
        .join("usage-history.sqlite3"))
}
