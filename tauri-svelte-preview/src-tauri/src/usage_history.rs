use crate::usage_db::{
    UsageBreakdownRow, UsageDailyRow, UsageDailyTotalsRow, UsageDb, UsageFilter,
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
