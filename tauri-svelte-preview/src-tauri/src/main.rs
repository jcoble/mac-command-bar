use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use mcb_core::scanners::sessions::{scan_sessions, AgentSessionRecord};
use orchestration::{
    list_orchestration_runs_sync, record_orchestration_event_sync, OrchestrationEvent,
    OrchestrationRun,
};
use tauri::Emitter;

mod lsp;
mod orchestration;
mod terminal;

const MAX_PREVIEW_BYTES: u64 = 512 * 1024;
const DEFAULT_SOURCE_LIST_LIMIT: usize = 10_000;
const MAX_SOURCE_LIST_LIMIT: usize = 25_000;
const DEFAULT_SOURCE_SEARCH_LIMIT: usize = 50;
const MAX_SOURCE_SEARCH_LIMIT: usize = 200;
const DEFAULT_SOURCE_DEFINITION_LIMIT: usize = 20;
const MAX_SOURCE_DEFINITION_LIMIT: usize = 100;
const DEFAULT_SOURCE_REFERENCE_LIMIT: usize = 50;
const MAX_SOURCE_REFERENCE_LIMIT: usize = 200;
const DEFAULT_GIT_HISTORY_LIMIT: usize = 24;
const MAX_GIT_HISTORY_LIMIT: usize = 80;
const SOURCE_SCAN_PROGRESS_EVENT: &str = "source_scan_progress";
const SOURCE_SCAN_PROGRESS_INTERVAL: usize = 64;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceRecord {
    path: String,
    relative_path: String,
    file_name: String,
    language: String,
    byte_count: u64,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceScanResult {
    records: Vec<SourceRecord>,
    limit: usize,
    truncated: bool,
}

#[derive(Clone, Copy)]
struct SourceScanProgressSnapshot {
    visited_entries: usize,
    matched_files: usize,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceScanProgressEvent {
    scan_id: String,
    visited_entries: usize,
    matched_files: usize,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourcePreview {
    path: String,
    relative_path: String,
    file_name: String,
    language: String,
    byte_count: u64,
    content: String,
    line_count: usize,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceSearchMatch {
    path: String,
    relative_path: String,
    file_name: String,
    language: String,
    byte_count: u64,
    line: usize,
    column: usize,
    excerpt: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceDefinitionTarget {
    path: String,
    relative_path: String,
    file_name: String,
    language: String,
    byte_count: u64,
    symbol_name: String,
    kind: String,
    line: usize,
    column: usize,
    detail: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceReferenceTarget {
    path: String,
    relative_path: String,
    file_name: String,
    language: String,
    byte_count: u64,
    symbol_name: String,
    line: usize,
    column: usize,
    excerpt: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectGitStatus {
    branch: Option<String>,
    ahead: usize,
    behind: usize,
    files: Vec<GitFileStatus>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GitActionResult {
    message: String,
    status: ProjectGitStatus,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GitFileStatus {
    relative_path: String,
    index_status: String,
    worktree_status: String,
    status: String,
    badge: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceGitDiff {
    relative_path: String,
    status: String,
    diff: String,
    is_binary: bool,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ProjectWorktree {
    repo: String,
    path: String,
    branch: String,
    task_id: Option<String>,
    is_dirty: bool,
    has_unmerged_commits: bool,
    last_activity: Option<String>,
    delete_eligibility: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeContextProject {
    id: String,
    name: String,
    path: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeContext {
    pid: u32,
    command: String,
    port: u16,
    cwd: String,
    #[serde(rename = "projectID")]
    project_id: Option<String>,
    project_name: String,
    root_label: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct GitRepositorySummary {
    #[serde(rename = "projectID")]
    project_id: String,
    project_name: String,
    repo: String,
    path: String,
    root_label: String,
    branch: String,
    #[serde(rename = "taskID")]
    task_id: Option<String>,
    is_worktree: bool,
    is_dirty: bool,
    staged_count: usize,
    unstaged_count: usize,
    untracked_count: usize,
    dirty_count: usize,
    ahead: usize,
    behind: usize,
    last_commit_sha: Option<String>,
    last_commit_subject: Option<String>,
    last_commit_at: Option<String>,
    dirty_since_epoch_ms: Option<u64>,
    error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct GitCommitHistoryEntry {
    short_sha: String,
    sha: String,
    subject: String,
    author: String,
    committed_at: String,
    refs: String,
    parent_shas: Vec<String>,
    parent_count: usize,
    #[serde(rename = "taskID")]
    task_id: Option<String>,
    task_source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessListener {
    pid: u32,
    command: String,
    port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeContextProjectMatch {
    project_id: Option<String>,
    project_name: String,
    root_label: String,
}

#[derive(Clone, Copy)]
enum SourceFileAction {
    Open,
    Reveal,
}

#[derive(Clone, Copy)]
enum PathAction {
    Open,
    Reveal,
}

#[derive(Debug, PartialEq, Eq)]
struct SourceFileActionCommand {
    program: String,
    args: Vec<String>,
}

#[derive(Default)]
struct SourceScanRegistry {
    scans: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl SourceScanRegistry {
    fn register(&self, scan_id: &str) -> Arc<AtomicBool> {
        let cancelled = Arc::new(AtomicBool::new(false));
        self.scans
            .lock()
            .expect("source scan registry lock poisoned")
            .insert(scan_id.to_string(), Arc::clone(&cancelled));
        cancelled
    }

    fn cancel(&self, scan_id: &str) -> bool {
        let Some(cancelled) = self
            .scans
            .lock()
            .expect("source scan registry lock poisoned")
            .get(scan_id)
            .cloned()
        else {
            return false;
        };
        cancelled.store(true, Ordering::Relaxed);
        true
    }

    fn unregister(&self, scan_id: &str) {
        self.scans
            .lock()
            .expect("source scan registry lock poisoned")
            .remove(scan_id);
    }
}

struct SourceScanCancellation {
    cancelled: Arc<AtomicBool>,
    progress: Option<Arc<dyn Fn(SourceScanProgressSnapshot) + Send + Sync>>,
}

impl SourceScanCancellation {
    fn new(
        cancelled: Arc<AtomicBool>,
        progress: Option<Arc<dyn Fn(SourceScanProgressSnapshot) + Send + Sync>>,
    ) -> Self {
        Self {
            cancelled,
            progress,
        }
    }

    fn none() -> Self {
        Self::new(Arc::new(AtomicBool::new(false)), None)
    }

    #[cfg(test)]
    fn cancelled_for_test() -> Self {
        Self::new(Arc::new(AtomicBool::new(true)), None)
    }

    #[cfg(test)]
    fn active_for_test(
        progress: impl Fn(SourceScanProgressSnapshot) + Send + Sync + 'static,
    ) -> Self {
        Self::new(Arc::new(AtomicBool::new(false)), Some(Arc::new(progress)))
    }

    fn ensure_active(&self) -> Result<(), String> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err("Source scan cancelled".to_string())
        } else {
            Ok(())
        }
    }

    fn report_progress(&self, progress: SourceScanProgressSnapshot) {
        if let Some(callback) = &self.progress {
            callback(progress);
        }
    }
}

#[derive(Default)]
struct SourceScanWalkProgress {
    visited_entries: usize,
    matched_files: usize,
    next_report_at: usize,
}

impl SourceScanWalkProgress {
    fn visit_entry(&mut self, cancellation: &SourceScanCancellation) {
        self.visited_entries += 1;
        if self.visited_entries == 1 || self.visited_entries >= self.next_report_at {
            self.next_report_at = self.visited_entries + SOURCE_SCAN_PROGRESS_INTERVAL;
            self.report(cancellation);
        }
    }

    fn match_file(&mut self, cancellation: &SourceScanCancellation) {
        self.matched_files += 1;
        self.report(cancellation);
    }

    fn report(&self, cancellation: &SourceScanCancellation) {
        cancellation.report_progress(SourceScanProgressSnapshot {
            visited_entries: self.visited_entries,
            matched_files: self.matched_files,
        });
    }
}

#[tauri::command]
async fn list_source_files(
    app: tauri::AppHandle,
    scan_registry: tauri::State<'_, SourceScanRegistry>,
    root: String,
    limit: Option<usize>,
    query: Option<String>,
    scan_id: Option<String>,
) -> Result<SourceScanResult, String> {
    let cancellation =
        source_scan_cancellation_for_command(&app, &scan_registry, scan_id.as_deref());
    let scan_id_for_cleanup = scan_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        list_source_files_sync_with_cancellation(
            PathBuf::from(root),
            limit.unwrap_or(DEFAULT_SOURCE_LIST_LIMIT),
            query,
            cancellation,
        )
    })
    .await
    .map_err(|error| format!("Source scan task failed: {error}"))?;

    if let Some(scan_id) = &scan_id_for_cleanup {
        scan_registry.unregister(scan_id);
    }

    result
}

#[tauri::command]
async fn cancel_source_scan(
    scan_registry: tauri::State<'_, SourceScanRegistry>,
    scan_id: String,
) -> Result<bool, String> {
    Ok(scan_registry.cancel(&scan_id))
}

fn source_scan_cancellation_for_command(
    app: &tauri::AppHandle,
    scan_registry: &SourceScanRegistry,
    scan_id: Option<&str>,
) -> SourceScanCancellation {
    let Some(scan_id) = scan_id.filter(|value| !value.trim().is_empty()) else {
        return SourceScanCancellation::none();
    };

    let cancelled = scan_registry.register(scan_id);
    let app = app.clone();
    let scan_id = scan_id.to_string();
    let progress = Arc::new(move |snapshot: SourceScanProgressSnapshot| {
        let _ = app.emit(
            SOURCE_SCAN_PROGRESS_EVENT,
            SourceScanProgressEvent {
                scan_id: scan_id.clone(),
                visited_entries: snapshot.visited_entries,
                matched_files: snapshot.matched_files,
            },
        );
    });

    SourceScanCancellation::new(cancelled, Some(progress))
}

#[tauri::command]
async fn read_source_file(path: String) -> Result<SourcePreview, String> {
    tauri::async_runtime::spawn_blocking(move || read_source_file_sync(PathBuf::from(path)))
        .await
        .map_err(|error| format!("Source preview task failed: {error}"))?
}

#[tauri::command]
async fn write_source_file(path: String, content: String) -> Result<SourcePreview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        write_source_file_sync(PathBuf::from(path), content)
    })
    .await
    .map_err(|error| format!("Source write task failed: {error}"))?
}

#[tauri::command]
async fn open_source_file(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_source_file_action(PathBuf::from(path), SourceFileAction::Open)
    })
    .await
    .map_err(|error| format!("Source open task failed: {error}"))?
}

#[tauri::command]
async fn reveal_source_file(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_source_file_action(PathBuf::from(path), SourceFileAction::Reveal)
    })
    .await
    .map_err(|error| format!("Source reveal task failed: {error}"))?
}

#[tauri::command]
async fn open_path(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_path_action(PathBuf::from(path), PathAction::Open)
    })
    .await
    .map_err(|error| format!("Path open task failed: {error}"))?
}

#[tauri::command]
async fn reveal_path(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_path_action(PathBuf::from(path), PathAction::Reveal)
    })
    .await
    .map_err(|error| format!("Path reveal task failed: {error}"))?
}

#[tauri::command]
async fn open_terminal_path(path: String, terminal: Option<String>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_terminal_path_action(PathBuf::from(path), terminal)
    })
    .await
    .map_err(|error| format!("Terminal open task failed: {error}"))?
}

#[tauri::command]
async fn open_terminal_command(
    path: String,
    command: String,
    terminal: Option<String>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_terminal_command_action(PathBuf::from(path), command, terminal)
    })
    .await
    .map_err(|error| format!("Terminal command task failed: {error}"))?
}

#[tauri::command]
async fn search_source_files(
    records: Vec<SourceRecord>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SourceSearchMatch>, String> {
    tauri::async_runtime::spawn_blocking(move || search_source_files_sync(records, query, limit))
        .await
        .map_err(|error| format!("Source search task failed: {error}"))?
}

#[tauri::command]
async fn find_source_definitions(
    records: Vec<SourceRecord>,
    symbol_name: String,
    limit: Option<usize>,
) -> Result<Vec<SourceDefinitionTarget>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        find_source_definitions_sync(records, symbol_name, limit)
    })
    .await
    .map_err(|error| format!("Source definition task failed: {error}"))?
}

#[tauri::command]
async fn find_source_references(
    records: Vec<SourceRecord>,
    symbol_name: String,
    limit: Option<usize>,
) -> Result<Vec<SourceReferenceTarget>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        find_source_references_sync(records, symbol_name, limit)
    })
    .await
    .map_err(|error| format!("Source reference task failed: {error}"))?
}

#[tauri::command]
async fn read_source_lsp_status(
    root: String,
    language: String,
) -> Result<lsp::SourceLspStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        lsp::read_source_lsp_status_sync(PathBuf::from(root), language)
    })
    .await
    .map_err(|error| format!("Source LSP status task failed: {error}"))?
}

#[tauri::command]
async fn find_source_lsp_definitions(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspDefinitionTarget>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_definitions(preview, request))
        .await
        .map_err(|error| format!("Source LSP definition task failed: {error}"))?
}

#[tauri::command]
async fn find_source_lsp_completions(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspCompletionItem>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_completions(preview, request))
        .await
        .map_err(|error| format!("Source LSP completion task failed: {error}"))?
}

#[tauri::command]
async fn find_source_lsp_implementations(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspDefinitionTarget>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_implementations(preview, request))
        .await
        .map_err(|error| format!("Source LSP implementation task failed: {error}"))?
}

#[tauri::command]
async fn find_source_lsp_type_definitions(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspDefinitionTarget>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_type_definitions(preview, request))
        .await
        .map_err(|error| format!("Source LSP type-definition task failed: {error}"))?
}

#[tauri::command]
async fn find_source_lsp_document_highlights(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspDocumentHighlight>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        registry.find_document_highlights(preview, request)
    })
    .await
    .map_err(|error| format!("Source LSP document-highlight task failed: {error}"))?
}

#[tauri::command]
async fn find_source_lsp_signature_help(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Option<lsp::SourceLspSignatureHelp>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_signature_help(preview, request))
        .await
        .map_err(|error| format!("Source LSP signature-help task failed: {error}"))?
}

#[tauri::command]
async fn find_source_lsp_inlay_hints(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspInlayHint>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_inlay_hints(preview, request))
        .await
        .map_err(|error| format!("Source LSP inlay-hint task failed: {error}"))?
}

#[tauri::command]
async fn find_source_lsp_semantic_tokens(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspSemanticToken>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_semantic_tokens(preview, request))
        .await
        .map_err(|error| format!("Source LSP semantic-token task failed: {error}"))?
}

#[tauri::command]
async fn format_source_with_lsp(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspTextEdit>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.format_document(preview, request))
        .await
        .map_err(|error| format!("Source LSP formatting task failed: {error}"))?
}

#[tauri::command]
async fn rename_source_with_lsp(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspRenameRequest,
) -> Result<lsp::SourceLspRenameResult, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.rename(preview, request))
        .await
        .map_err(|error| format!("Source LSP rename task failed: {error}"))?
}

#[tauri::command]
async fn find_source_lsp_code_actions(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspCodeActionRequest,
) -> Result<Vec<lsp::SourceLspCodeAction>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_code_actions(preview, request))
        .await
        .map_err(|error| format!("Source LSP code action task failed: {error}"))?
}

#[tauri::command]
async fn find_source_lsp_references(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspReferenceTarget>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_references(preview, request))
        .await
        .map_err(|error| format!("Source LSP reference task failed: {error}"))?
}

#[tauri::command]
async fn find_source_lsp_hover(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Option<lsp::SourceLspHover>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_hover(preview, request))
        .await
        .map_err(|error| format!("Source LSP hover task failed: {error}"))?
}

#[tauri::command]
async fn find_source_lsp_symbols(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspSymbol>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_symbols(preview, request))
        .await
        .map_err(|error| format!("Source LSP symbol task failed: {error}"))?
}

#[tauri::command]
async fn read_source_lsp_diagnostics(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspLookupRequest,
) -> Result<Vec<lsp::SourceLspDiagnostic>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.read_diagnostics(preview, request))
        .await
        .map_err(|error| format!("Source LSP diagnostics task failed: {error}"))?
}

#[tauri::command]
async fn project_git_status(root: String) -> Result<ProjectGitStatus, String> {
    tauri::async_runtime::spawn_blocking(move || project_git_status_sync(PathBuf::from(root)))
        .await
        .map_err(|error| format!("Git status task failed: {error}"))?
}

#[tauri::command]
async fn read_source_git_diff(root: String, path: String) -> Result<SourceGitDiff, String> {
    tauri::async_runtime::spawn_blocking(move || {
        read_source_git_diff_sync(PathBuf::from(root), PathBuf::from(path))
    })
    .await
    .map_err(|error| format!("Git diff task failed: {error}"))?
}

#[tauri::command]
async fn stage_git_paths(root: String, paths: Vec<String>) -> Result<GitActionResult, String> {
    tauri::async_runtime::spawn_blocking(move || stage_git_paths_sync(PathBuf::from(root), paths))
        .await
        .map_err(|error| format!("Git stage task failed: {error}"))?
}

#[tauri::command]
async fn unstage_git_paths(root: String, paths: Vec<String>) -> Result<GitActionResult, String> {
    tauri::async_runtime::spawn_blocking(move || unstage_git_paths_sync(PathBuf::from(root), paths))
        .await
        .map_err(|error| format!("Git unstage task failed: {error}"))?
}

#[tauri::command]
async fn commit_git_repository(root: String, message: String) -> Result<GitActionResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commit_git_repository_sync(PathBuf::from(root), message)
    })
    .await
    .map_err(|error| format!("Git commit task failed: {error}"))?
}

#[tauri::command]
async fn fetch_git_repository(root: String) -> Result<GitActionResult, String> {
    tauri::async_runtime::spawn_blocking(move || fetch_git_repository_sync(PathBuf::from(root)))
        .await
        .map_err(|error| format!("Git fetch task failed: {error}"))?
}

#[tauri::command]
async fn pull_git_repository(root: String) -> Result<GitActionResult, String> {
    tauri::async_runtime::spawn_blocking(move || pull_git_repository_sync(PathBuf::from(root)))
        .await
        .map_err(|error| format!("Git pull task failed: {error}"))?
}

#[tauri::command]
async fn push_git_repository(root: String) -> Result<GitActionResult, String> {
    tauri::async_runtime::spawn_blocking(move || push_git_repository_sync(PathBuf::from(root)))
        .await
        .map_err(|error| format!("Git push task failed: {error}"))?
}

#[tauri::command]
async fn read_git_commit_history(
    root: String,
    limit: Option<usize>,
) -> Result<Vec<GitCommitHistoryEntry>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        read_git_commit_history_sync(PathBuf::from(root), limit)
    })
    .await
    .map_err(|error| format!("Git history task failed: {error}"))?
}

#[tauri::command]
async fn list_project_worktrees(root: String) -> Result<Vec<ProjectWorktree>, String> {
    tauri::async_runtime::spawn_blocking(move || list_project_worktrees_sync(PathBuf::from(root)))
        .await
        .map_err(|error| format!("Worktree scan task failed: {error}"))?
}

#[tauri::command]
async fn list_git_repository_summaries(
    projects: Vec<RuntimeContextProject>,
) -> Result<Vec<GitRepositorySummary>, String> {
    tauri::async_runtime::spawn_blocking(move || list_git_repository_summaries_sync(projects))
        .await
        .map_err(|error| format!("Repository dashboard task failed: {error}"))?
}

#[tauri::command]
async fn list_agent_sessions() -> Result<Vec<AgentSessionRecord>, String> {
    tauri::async_runtime::spawn_blocking(scan_sessions)
        .await
        .map_err(|error| format!("Agent session scan task failed: {error}"))
}

#[tauri::command]
async fn list_runtime_contexts(
    projects: Vec<RuntimeContextProject>,
) -> Result<Vec<RuntimeContext>, String> {
    tauri::async_runtime::spawn_blocking(move || list_runtime_contexts_sync(projects))
        .await
        .map_err(|error| format!("Runtime context task failed: {error}"))?
}

#[tauri::command]
async fn list_orchestration_runs(
    projects: Vec<RuntimeContextProject>,
) -> Result<Vec<OrchestrationRun>, String> {
    tauri::async_runtime::spawn_blocking(move || list_orchestration_runs_sync(projects))
        .await
        .map_err(|error| format!("Orchestration run task failed: {error}"))?
}

#[tauri::command]
async fn record_orchestration_event(event: OrchestrationEvent) -> Result<OrchestrationRun, String> {
    tauri::async_runtime::spawn_blocking(move || record_orchestration_event_sync(event))
        .await
        .map_err(|error| format!("Orchestration event task failed: {error}"))?
}

#[tauri::command]
async fn start_terminal_session(
    app: tauri::AppHandle,
    terminal_registry: tauri::State<'_, terminal::TerminalRegistry>,
    request: terminal::TerminalStartRequest,
) -> Result<terminal::TerminalSessionInfo, String> {
    terminal::start_terminal_session(app, &terminal_registry, request)
}

#[tauri::command]
async fn list_terminal_sessions(
    terminal_registry: tauri::State<'_, terminal::TerminalRegistry>,
) -> Result<Vec<terminal::TerminalSessionInfo>, String> {
    terminal::list_terminal_sessions(&terminal_registry)
}

#[tauri::command]
async fn read_terminal_session_scrollback(
    terminal_registry: tauri::State<'_, terminal::TerminalRegistry>,
    session_id: String,
) -> Result<Option<String>, String> {
    terminal::read_terminal_session_scrollback(&terminal_registry, &session_id)
}

#[tauri::command]
async fn write_terminal_session(
    terminal_registry: tauri::State<'_, terminal::TerminalRegistry>,
    session_id: String,
    data: String,
) -> Result<bool, String> {
    terminal::write_terminal_session(&terminal_registry, &session_id, &data)
}

#[tauri::command]
async fn resize_terminal_session(
    terminal_registry: tauri::State<'_, terminal::TerminalRegistry>,
    session_id: String,
    cols: Option<u16>,
    rows: Option<u16>,
) -> Result<bool, String> {
    terminal::resize_terminal_session(&terminal_registry, &session_id, cols, rows)
}

#[tauri::command]
async fn close_terminal_session(
    terminal_registry: tauri::State<'_, terminal::TerminalRegistry>,
    session_id: String,
) -> Result<bool, String> {
    terminal::close_terminal_session(&terminal_registry, &session_id)
}

#[cfg(test)]
fn list_source_files_sync(
    root: PathBuf,
    limit: usize,
    query: Option<String>,
) -> Result<SourceScanResult, String> {
    list_source_files_sync_with_cancellation(root, limit, query, SourceScanCancellation::none())
}

fn list_source_files_sync_with_cancellation(
    root: PathBuf,
    limit: usize,
    query: Option<String>,
    cancellation: SourceScanCancellation,
) -> Result<SourceScanResult, String> {
    cancellation.ensure_active()?;
    let metadata = std::fs::metadata(&root)
        .map_err(|error| format!("Could not read source root metadata: {error}"))?;
    if !metadata.is_dir() {
        return Err("Source root is not a directory".to_string());
    }

    let limit = if limit == 0 {
        DEFAULT_SOURCE_LIST_LIMIT
    } else {
        limit.min(MAX_SOURCE_LIST_LIMIT)
    };
    let normalized_query = query
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty());
    let collect_limit = limit.saturating_add(1);
    let mut records = Vec::new();
    let mut progress = SourceScanWalkProgress::default();
    collect_source_files(
        &root,
        &root,
        collect_limit,
        normalized_query.as_deref(),
        &mut records,
        &cancellation,
        &mut progress,
    )?;
    progress.report(&cancellation);
    cancellation.ensure_active()?;
    records.sort_by(compare_source_records);
    let truncated = records.len() > limit;
    records.truncate(limit);
    #[cfg(debug_assertions)]
    eprintln!(
        "mcb tauri source.list root={} count={} truncated={}",
        root.display(),
        records.len(),
        truncated
    );
    Ok(SourceScanResult {
        records,
        limit,
        truncated,
    })
}

fn collect_source_files(
    root: &Path,
    current: &Path,
    limit: usize,
    query: Option<&str>,
    records: &mut Vec<SourceRecord>,
    cancellation: &SourceScanCancellation,
    progress: &mut SourceScanWalkProgress,
) -> Result<(), String> {
    cancellation.ensure_active()?;
    if records.len() >= limit {
        return Ok(());
    }

    let mut entries = std::fs::read_dir(current)
        .map_err(|error| format!("Could not read source directory: {error}"))?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| compare_source_walk_entries(root, left, right));

    for entry in entries {
        cancellation.ensure_active()?;
        progress.visit_entry(cancellation);

        if records.len() >= limit {
            break;
        }

        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };

        if metadata.is_dir() {
            if should_skip_dir(&file_name) {
                continue;
            }
            collect_source_files(root, &path, limit, query, records, cancellation, progress)?;
            continue;
        }

        if !metadata.is_file() || !is_source_file(&path) {
            continue;
        }

        let relative_path = normalized_relative_source_path(root, &path);
        if !source_file_matches_query(&relative_path, &file_name, query) {
            continue;
        }

        records.push(SourceRecord {
            path: path.display().to_string(),
            relative_path,
            file_name,
            language: detect_language(&path),
            byte_count: metadata.len(),
        });
        progress.match_file(cancellation);
    }

    Ok(())
}

fn read_source_file_sync(path: PathBuf) -> Result<SourcePreview, String> {
    let path_ref = path.as_path();
    let metadata = std::fs::metadata(path_ref)
        .map_err(|error| format!("Could not read source metadata: {error}"))?;
    if !metadata.is_file() {
        return Err("Source path is not a file".to_string());
    }
    if metadata.len() > MAX_PREVIEW_BYTES {
        return Err(format!(
            "Source file is too large: {} bytes",
            metadata.len()
        ));
    }

    let content = std::fs::read_to_string(path_ref)
        .map_err(|error| format!("Could not read source file as UTF-8: {error}"))?;
    let file_name = path_ref
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("source")
        .to_string();

    let preview = SourcePreview {
        path: path_ref.display().to_string(),
        relative_path: file_name.clone(),
        file_name,
        language: detect_language(path_ref),
        byte_count: metadata.len(),
        line_count: content.lines().count(),
        content,
    };
    #[cfg(debug_assertions)]
    eprintln!(
        "mcb tauri source.preview path={} language={} lines={}",
        preview.path, preview.language, preview.line_count
    );
    Ok(preview)
}

fn write_source_file_sync(path: PathBuf, content: String) -> Result<SourcePreview, String> {
    let path_ref = path.as_path();
    let metadata = std::fs::metadata(path_ref)
        .map_err(|error| format!("Could not read source metadata: {error}"))?;
    if !metadata.is_file() {
        return Err("Source path is not a file".to_string());
    }
    if !is_source_file(path_ref) {
        return Err("Source path is not a supported source file".to_string());
    }

    let byte_count = content.as_bytes().len() as u64;
    if byte_count > MAX_PREVIEW_BYTES {
        return Err(format!("Source content is too large: {byte_count} bytes"));
    }

    let parent = path_ref
        .parent()
        .ok_or_else(|| "Source path has no parent directory".to_string())?;
    let file_name = path_ref
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("source");
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let temp_path = parent.join(format!(
        ".{file_name}.mcb-write-{}-{unique}",
        std::process::id()
    ));

    std::fs::write(&temp_path, content.as_bytes())
        .map_err(|error| format!("Could not write source temp file: {error}"))?;
    if let Err(error) = std::fs::rename(&temp_path, path_ref) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!("Could not replace source file: {error}"));
    }

    read_source_file_sync(path)
}

fn search_source_files_sync(
    records: Vec<SourceRecord>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SourceSearchMatch>, String> {
    let normalized_query = query.trim().to_lowercase();
    if normalized_query.is_empty() {
        return Ok(Vec::new());
    }

    let capped_limit = limit
        .unwrap_or(DEFAULT_SOURCE_SEARCH_LIMIT)
        .min(MAX_SOURCE_SEARCH_LIMIT);
    if capped_limit == 0 {
        return Ok(Vec::new());
    }

    let mut matches = Vec::new();
    for record in records {
        if matches.len() >= capped_limit {
            break;
        }

        let path = PathBuf::from(&record.path);
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        if !metadata.is_file() || metadata.len() > MAX_PREVIEW_BYTES {
            continue;
        }

        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let content = String::from_utf8_lossy(&bytes);
        append_source_search_matches(
            &record,
            &content,
            &normalized_query,
            capped_limit,
            &mut matches,
        );
    }

    Ok(matches)
}

fn append_source_search_matches(
    record: &SourceRecord,
    content: &str,
    normalized_query: &str,
    limit: usize,
    matches: &mut Vec<SourceSearchMatch>,
) {
    for (line_index, line) in content.lines().enumerate() {
        let Some(column_index) = line.to_lowercase().find(normalized_query) else {
            continue;
        };

        matches.push(SourceSearchMatch {
            path: record.path.clone(),
            relative_path: record.relative_path.clone(),
            file_name: record.file_name.clone(),
            language: record.language.clone(),
            byte_count: record.byte_count,
            line: line_index + 1,
            column: column_index + 1,
            excerpt: compact_source_line_excerpt(line),
        });

        if matches.len() >= limit {
            break;
        }
    }
}

fn find_source_definitions_sync(
    records: Vec<SourceRecord>,
    symbol_name: String,
    limit: Option<usize>,
) -> Result<Vec<SourceDefinitionTarget>, String> {
    let normalized_symbol_name = symbol_name.trim().to_lowercase();
    if normalized_symbol_name.is_empty() {
        return Ok(Vec::new());
    }

    let capped_limit = limit
        .unwrap_or(DEFAULT_SOURCE_DEFINITION_LIMIT)
        .min(MAX_SOURCE_DEFINITION_LIMIT);
    if capped_limit == 0 {
        return Ok(Vec::new());
    }

    let mut targets = Vec::new();
    for record in records {
        if targets.len() >= capped_limit {
            break;
        }

        let path = PathBuf::from(&record.path);
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        if !metadata.is_file() || metadata.len() > MAX_PREVIEW_BYTES {
            continue;
        }

        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let content = String::from_utf8_lossy(&bytes);
        append_source_definition_targets(
            &record,
            &content,
            &normalized_symbol_name,
            capped_limit,
            &mut targets,
        );
    }

    Ok(targets)
}

fn append_source_definition_targets(
    record: &SourceRecord,
    content: &str,
    normalized_symbol_name: &str,
    limit: usize,
    targets: &mut Vec<SourceDefinitionTarget>,
) {
    for (line_index, line) in content.lines().enumerate() {
        let Some(symbol) = parse_source_symbol_line(&record.language, line) else {
            continue;
        };
        if symbol.name.to_lowercase() != normalized_symbol_name {
            continue;
        }

        targets.push(SourceDefinitionTarget {
            path: record.path.clone(),
            relative_path: record.relative_path.clone(),
            file_name: record.file_name.clone(),
            language: record.language.clone(),
            byte_count: record.byte_count,
            symbol_name: symbol.name,
            kind: symbol.kind,
            line: line_index + 1,
            column: symbol.column,
            detail: line.trim().to_string(),
        });

        if targets.len() >= limit {
            break;
        }
    }
}

fn find_source_references_sync(
    records: Vec<SourceRecord>,
    symbol_name: String,
    limit: Option<usize>,
) -> Result<Vec<SourceReferenceTarget>, String> {
    let normalized_symbol_name = symbol_name.trim().to_lowercase();
    if normalized_symbol_name.is_empty() {
        return Ok(Vec::new());
    }

    let capped_limit = limit
        .unwrap_or(DEFAULT_SOURCE_REFERENCE_LIMIT)
        .min(MAX_SOURCE_REFERENCE_LIMIT);
    if capped_limit == 0 {
        return Ok(Vec::new());
    }

    let mut targets = Vec::new();
    for record in records {
        if targets.len() >= capped_limit {
            break;
        }

        let path = PathBuf::from(&record.path);
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        if !metadata.is_file() || metadata.len() > MAX_PREVIEW_BYTES {
            continue;
        }

        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let content = String::from_utf8_lossy(&bytes);
        append_source_reference_targets(
            &record,
            &content,
            &normalized_symbol_name,
            capped_limit,
            &mut targets,
        );
    }

    Ok(targets)
}

fn append_source_reference_targets(
    record: &SourceRecord,
    content: &str,
    normalized_symbol_name: &str,
    limit: usize,
    targets: &mut Vec<SourceReferenceTarget>,
) {
    for (line_index, line) in content.lines().enumerate() {
        let Some(column_index) = find_source_reference_column(line, normalized_symbol_name) else {
            continue;
        };

        let symbol_name = line
            .get(column_index..column_index + normalized_symbol_name.len())
            .unwrap_or(normalized_symbol_name)
            .to_string();
        targets.push(SourceReferenceTarget {
            path: record.path.clone(),
            relative_path: record.relative_path.clone(),
            file_name: record.file_name.clone(),
            language: record.language.clone(),
            byte_count: record.byte_count,
            symbol_name,
            line: line_index + 1,
            column: column_index + 1,
            excerpt: compact_source_line_excerpt(line),
        });

        if targets.len() >= limit {
            break;
        }
    }
}

fn find_source_reference_column(line: &str, normalized_symbol_name: &str) -> Option<usize> {
    let normalized_line = line.to_lowercase();
    let mut search_start = 0;

    while search_start < normalized_line.len() {
        let relative_index = normalized_line[search_start..].find(normalized_symbol_name)?;
        let index = search_start + relative_index;
        let end_index = index + normalized_symbol_name.len();
        if is_source_token_boundary(line, index, end_index) {
            return Some(index);
        }
        search_start = end_index;
    }

    None
}

fn is_source_token_boundary(line: &str, start: usize, end: usize) -> bool {
    let before = if start == 0 {
        None
    } else {
        line[..start].chars().next_back()
    };
    let after = line[end..].chars().next();

    !before.is_some_and(is_source_identifier_character)
        && !after.is_some_and(is_source_identifier_character)
}

fn is_source_identifier_character(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedSourceSymbol {
    name: String,
    kind: String,
    column: usize,
}

fn parse_source_symbol_line(language: &str, line: &str) -> Option<ParsedSourceSymbol> {
    match language {
        "csharp" => parse_csharp_symbol_line(line),
        "typescript" | "tsx" | "javascript" | "jsx" => parse_typescript_symbol_line(line),
        _ => None,
    }
}

fn parse_csharp_symbol_line(line: &str) -> Option<ParsedSourceSymbol> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return None;
    }

    if let Some(namespace_name) = trimmed.strip_prefix("namespace ") {
        let name = trim_symbol_name(namespace_name, &[';', '{']);
        return source_symbol_from_name(line, name, "namespace");
    }

    let tokens = source_line_tokens(trimmed);
    for (index, token) in tokens.iter().enumerate() {
        if !matches!(*token, "class" | "interface" | "record" | "enum" | "struct") {
            continue;
        }

        let name = tokens.get(index + 1)?;
        return source_symbol_from_name(line, name, token);
    }

    let before_params = trimmed.split_once('(')?.0.trim_end();
    let method_name = before_params.split_whitespace().last()?;
    if method_name.is_empty() || !starts_with_identifier(method_name) {
        return None;
    }

    let first_token = tokens.first().copied().unwrap_or_default();
    if !matches!(
        first_token,
        "public"
            | "private"
            | "protected"
            | "internal"
            | "static"
            | "async"
            | "virtual"
            | "override"
    ) {
        return None;
    }

    source_symbol_from_name(line, method_name, "method")
}

fn parse_typescript_symbol_line(line: &str) -> Option<ParsedSourceSymbol> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return None;
    }

    let tokens = source_line_tokens(trimmed);
    let mut index = 0;
    while matches!(
        tokens.get(index).copied(),
        Some("export" | "default" | "abstract" | "async" | "declare")
    ) {
        index += 1;
    }

    match tokens.get(index).copied()? {
        "class" | "interface" | "type" | "enum" => {
            source_symbol_from_name(line, tokens.get(index + 1)?, tokens[index])
        }
        "function" => source_symbol_from_name(line, tokens.get(index + 1)?, "function"),
        "const" | "let" | "var" => {
            source_symbol_from_name(line, tokens.get(index + 1)?, "constant")
        }
        _ => None,
    }
}

fn source_line_tokens(line: &str) -> Vec<&str> {
    line.split_whitespace()
        .map(|token| trim_symbol_name(token, &[';', '{', '(', ')', ':', ',', '=', '<']))
        .filter(|token| !token.is_empty())
        .collect()
}

fn trim_symbol_name<'a>(value: &'a str, separators: &[char]) -> &'a str {
    value
        .trim()
        .trim_matches(|character: char| separators.contains(&character))
        .split(|character: char| separators.contains(&character))
        .next()
        .unwrap_or("")
}

fn source_symbol_from_name(line: &str, name: &str, kind: &str) -> Option<ParsedSourceSymbol> {
    if name.is_empty() || !starts_with_identifier(name) {
        return None;
    }

    Some(ParsedSourceSymbol {
        name: name.to_string(),
        kind: kind.to_string(),
        column: line.find(name).map(|index| index + 1).unwrap_or(1),
    })
}

fn starts_with_identifier(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
}

fn compact_source_line_excerpt(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.chars().count() <= 180 {
        return trimmed.to_string();
    }

    format!("{}...", trimmed.chars().take(177).collect::<String>())
}

fn run_source_file_action(path: PathBuf, action: SourceFileAction) -> Result<(), String> {
    let command = source_file_action_command(&path, action)?;
    let status = Command::new(&command.program)
        .args(&command.args)
        .status()
        .map_err(|error| format!("Could not run source file action: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Source file action exited with {status}"))
    }
}

fn run_path_action(path: PathBuf, action: PathAction) -> Result<(), String> {
    let command = path_action_command(&path, action)?;
    let status = Command::new(&command.program)
        .args(&command.args)
        .status()
        .map_err(|error| format!("Could not run path action: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Path action exited with {status}"))
    }
}

fn run_terminal_path_action(path: PathBuf, terminal: Option<String>) -> Result<(), String> {
    let command = terminal_path_action_command(&path, terminal)?;
    let status = Command::new(&command.program)
        .args(&command.args)
        .status()
        .map_err(|error| format!("Could not run terminal action: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Terminal action exited with {status}"))
    }
}

fn run_terminal_command_action(
    path: PathBuf,
    command: String,
    terminal: Option<String>,
) -> Result<(), String> {
    let command = terminal_command_action_command(&path, &command, terminal)?;
    let status = Command::new(&command.program)
        .args(&command.args)
        .status()
        .map_err(|error| format!("Could not run terminal command action: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Terminal command action exited with {status}"))
    }
}

fn source_file_action_command(
    path: &Path,
    action: SourceFileAction,
) -> Result<SourceFileActionCommand, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("Could not read source metadata: {error}"))?;
    if !metadata.is_file() {
        return Err("Source path is not a file".to_string());
    }

    let path_arg = path.display().to_string();
    let args = match action {
        SourceFileAction::Open => vec![path_arg],
        SourceFileAction::Reveal => vec!["-R".to_string(), path_arg],
    };

    Ok(SourceFileActionCommand {
        program: "open".to_string(),
        args,
    })
}

fn path_action_command(path: &Path, action: PathAction) -> Result<SourceFileActionCommand, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("Could not read path metadata: {error}"))?;
    if !metadata.is_file() && !metadata.is_dir() {
        return Err("Path is not a file or directory".to_string());
    }

    let path_arg = path.display().to_string();
    let args = match action {
        PathAction::Open => vec![path_arg],
        PathAction::Reveal => vec!["-R".to_string(), path_arg],
    };

    Ok(SourceFileActionCommand {
        program: "open".to_string(),
        args,
    })
}

fn terminal_path_action_command(
    path: &Path,
    terminal: Option<String>,
) -> Result<SourceFileActionCommand, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("Could not read terminal path metadata: {error}"))?;
    if !metadata.is_dir() {
        return Err("Terminal path is not a directory".to_string());
    }

    let terminal_app = normalize_terminal_app(terminal.as_deref())?;
    if terminal_app == "Warp" {
        return Ok(SourceFileActionCommand {
            program: "open".to_string(),
            args: vec![format!(
                "warp://action/new_tab?path={}",
                uri_query_encode(&path.display().to_string())
            )],
        });
    }

    Ok(SourceFileActionCommand {
        program: "open".to_string(),
        args: vec!["-a".to_string(), terminal_app, path.display().to_string()],
    })
}

fn terminal_command_action_command(
    path: &Path,
    command: &str,
    terminal: Option<String>,
) -> Result<SourceFileActionCommand, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("Could not read terminal path metadata: {error}"))?;
    if !metadata.is_dir() {
        return Err("Terminal path is not a directory".to_string());
    }

    let command = normalize_terminal_command(command)?;
    let terminal_app = normalize_terminal_app(terminal.as_deref())?;
    let shell_command = format!(
        "cd {} && {}",
        shell_quote(&path.display().to_string()),
        command
    );
    let escaped_shell_command = applescript_string_escape(&shell_command);
    let script = match terminal_app.as_str() {
        "Terminal" => format!(
            "tell application \"Terminal\"\nactivate\ndo script \"{}\"\nend tell",
            escaped_shell_command
        ),
        "iTerm" | "iTerm2" => format!(
            "tell application \"{}\"\nactivate\ncreate window with default profile\ntell current session of current window\nwrite text \"{}\"\nend tell\nend tell",
            terminal_app,
            escaped_shell_command
        ),
        _ => {
            return Err(format!(
                "Terminal command execution is not supported for {terminal_app}"
            ))
        }
    };

    Ok(SourceFileActionCommand {
        program: "osascript".to_string(),
        args: vec!["-e".to_string(), script],
    })
}

fn normalize_terminal_app(terminal: Option<&str>) -> Result<String, String> {
    let requested = terminal
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Terminal");
    let allowlisted = [
        ("terminal", "Terminal"),
        ("terminal.app", "Terminal"),
        ("warp", "Warp"),
        ("warp.app", "Warp"),
        ("iterm", "iTerm"),
        ("iterm.app", "iTerm"),
        ("iterm2", "iTerm2"),
        ("iterm2.app", "iTerm2"),
        ("wezterm", "WezTerm"),
        ("wezterm.app", "WezTerm"),
        ("ghostty", "Ghostty"),
        ("ghostty.app", "Ghostty"),
        ("alacritty", "Alacritty"),
        ("alacritty.app", "Alacritty"),
    ];

    allowlisted
        .iter()
        .find(|(alias, _)| alias.eq_ignore_ascii_case(requested))
        .map(|(_, app)| (*app).to_string())
        .ok_or_else(|| format!("Unsupported terminal app: {requested}"))
}

fn normalize_terminal_command(command: &str) -> Result<String, String> {
    let command = command.trim();
    if command.is_empty() {
        return Err("Terminal command is empty".to_string());
    }
    if command.contains('\0') || command.contains('\n') || command.contains('\r') {
        return Err("Terminal command must be a single line".to_string());
    }

    Ok(command.to_string())
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn uri_query_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn applescript_string_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn project_git_status_sync(root: PathBuf) -> Result<ProjectGitStatus, String> {
    validate_git_root(&root)?;

    let root_arg = root.display().to_string();
    let output = Command::new("git")
        .args([
            "-C",
            root_arg.as_str(),
            "status",
            "--porcelain=v1",
            "--branch",
            "--untracked-files=normal",
        ])
        .output()
        .map_err(|error| format!("Could not run git status: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("git status exited with {}", output.status)
        } else {
            stderr
        });
    }

    parse_project_git_status(&String::from_utf8_lossy(&output.stdout))
}

fn stage_git_paths_sync(root: PathBuf, paths: Vec<String>) -> Result<GitActionResult, String> {
    validate_git_root(&root)?;
    let validated_paths = validate_git_relative_paths(&paths)?;
    run_git_with_paths(&root, &["add"], &validated_paths)?;
    Ok(GitActionResult {
        message: format_git_path_action_message("Staged", validated_paths.len()),
        status: project_git_status_sync(root)?,
    })
}

fn unstage_git_paths_sync(root: PathBuf, paths: Vec<String>) -> Result<GitActionResult, String> {
    validate_git_root(&root)?;
    let validated_paths = validate_git_relative_paths(&paths)?;
    run_git_with_paths(&root, &["restore", "--staged"], &validated_paths)?;
    Ok(GitActionResult {
        message: format_git_path_action_message("Unstaged", validated_paths.len()),
        status: project_git_status_sync(root)?,
    })
}

fn commit_git_repository_sync(root: PathBuf, message: String) -> Result<GitActionResult, String> {
    validate_git_root(&root)?;
    let message = message.trim();
    if message.is_empty() {
        return Err("Commit message is required".to_string());
    }
    if message.contains('\0') {
        return Err("Commit message cannot contain null bytes".to_string());
    }
    if !git_has_staged_changes(&root)? {
        return Err("No staged changes to commit".to_string());
    }

    run_git_text(&root, &["commit", "-m", message])?;
    Ok(GitActionResult {
        message: "Committed staged changes".to_string(),
        status: project_git_status_sync(root)?,
    })
}

fn fetch_git_repository_sync(root: PathBuf) -> Result<GitActionResult, String> {
    validate_git_root(&root)?;
    run_git_text(&root, &["fetch", "--prune"])?;
    Ok(GitActionResult {
        message: "Fetched repository remotes".to_string(),
        status: project_git_status_sync(root)?,
    })
}

fn pull_git_repository_sync(root: PathBuf) -> Result<GitActionResult, String> {
    validate_git_root(&root)?;
    run_git_text(&root, &["pull", "--ff-only"])?;
    Ok(GitActionResult {
        message: "Pulled fast-forward updates".to_string(),
        status: project_git_status_sync(root)?,
    })
}

fn push_git_repository_sync(root: PathBuf) -> Result<GitActionResult, String> {
    validate_git_root(&root)?;
    run_git_text(&root, &["push"])?;
    Ok(GitActionResult {
        message: "Pushed current branch".to_string(),
        status: project_git_status_sync(root)?,
    })
}

fn read_git_commit_history_sync(
    root: PathBuf,
    limit: Option<usize>,
) -> Result<Vec<GitCommitHistoryEntry>, String> {
    validate_git_root(&root)?;

    let limit = limit
        .unwrap_or(DEFAULT_GIT_HISTORY_LIMIT)
        .clamp(1, MAX_GIT_HISTORY_LIMIT);
    let limit_arg = format!("-n{limit}");
    let history_output = run_git_text(
        &root,
        &[
            "log",
            "--decorate=short",
            "--date=iso-strict",
            "--format=%h%x1f%H%x1f%s%x1f%an%x1f%cI%x1f%D%x1f%P",
            limit_arg.as_str(),
        ],
    );

    match history_output {
        Ok(output) => parse_git_commit_history(&output),
        Err(error) if error.contains("does not have any commits") => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

fn validate_git_root(root: &Path) -> Result<(), String> {
    let metadata = std::fs::metadata(root)
        .map_err(|error| format!("Could not read Git root metadata: {error}"))?;
    if metadata.is_dir() {
        Ok(())
    } else {
        Err("Git root is not a directory".to_string())
    }
}

fn validate_git_relative_paths(paths: &[String]) -> Result<Vec<String>, String> {
    if paths.is_empty() {
        return Err("At least one relative repo path is required".to_string());
    }

    let mut validated_paths = Vec::with_capacity(paths.len());
    for path in paths {
        let trimmed = path.trim();
        let relative_path = Path::new(trimmed);
        let is_safe_relative_path = !trimmed.is_empty()
            && !trimmed.contains('\0')
            && !relative_path.is_absolute()
            && relative_path
                .components()
                .all(|component| matches!(component, std::path::Component::Normal(_)));

        if !is_safe_relative_path {
            return Err("Git actions only accept relative repo paths".to_string());
        }

        validated_paths.push(trimmed.to_string());
    }

    Ok(validated_paths)
}

fn run_git_with_paths(root: &Path, args: &[&str], paths: &[String]) -> Result<String, String> {
    let mut git_args = args.to_vec();
    git_args.push("--");
    for path in paths {
        git_args.push(path.as_str());
    }

    run_git_text(root, &git_args)
}

fn git_has_staged_changes(root: &Path) -> Result<bool, String> {
    let root_arg = root.display().to_string();
    let output = Command::new("git")
        .arg("-C")
        .arg(root_arg)
        .args(["diff", "--cached", "--quiet", "--exit-code"])
        .output()
        .map_err(|error| format!("Could not run git diff --cached: {error}"))?;

    if output.status.success() {
        return Ok(false);
    }

    if output.status.code() == Some(1) {
        return Ok(true);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if stderr.is_empty() {
        format!("git diff --cached exited with {}", output.status)
    } else {
        stderr
    })
}

fn format_git_path_action_message(action: &str, count: usize) -> String {
    let noun = if count == 1 { "path" } else { "paths" };
    format!("{action} {count} {noun}")
}

fn read_source_git_diff_sync(root: PathBuf, path: PathBuf) -> Result<SourceGitDiff, String> {
    validate_git_root(&root)?;

    let path_metadata = std::fs::metadata(&path)
        .map_err(|error| format!("Could not read source path metadata: {error}"))?;
    if !path_metadata.is_file() {
        return Err("Source path is not a file".to_string());
    }

    let canonical_root = std::fs::canonicalize(&root)
        .map_err(|error| format!("Could not resolve Git root: {error}"))?;
    let canonical_path = std::fs::canonicalize(&path)
        .map_err(|error| format!("Could not resolve source path: {error}"))?;
    let root_string = normalized_path_string(&canonical_root);
    let path_string = normalized_path_string(&canonical_path);
    if !path_is_within(&path_string, &root_string) {
        return Err("Source path is outside Git root".to_string());
    }

    let relative_path = canonical_path
        .strip_prefix(&canonical_root)
        .map_err(|error| format!("Could not derive source relative path: {error}"))?
        .to_string_lossy()
        .replace('\\', "/");
    let status = read_source_git_status(&canonical_root, &relative_path)?;
    let staged_diff = run_git_text(
        &canonical_root,
        &[
            "diff",
            "--no-ext-diff",
            "--cached",
            "--",
            relative_path.as_str(),
        ],
    )?;
    let working_diff = run_git_text(
        &canonical_root,
        &["diff", "--no-ext-diff", "--", relative_path.as_str()],
    )?;
    let diff = combine_source_git_diffs(&staged_diff, &working_diff);
    let is_binary = diff.contains("Binary files ") || diff.contains("GIT binary patch");
    let status = if status.is_empty() && !diff.is_empty() {
        "modified".to_string()
    } else if status.is_empty() {
        "clean".to_string()
    } else {
        status
    };

    Ok(SourceGitDiff {
        relative_path,
        status,
        diff,
        is_binary,
    })
}

fn read_source_git_status(root: &Path, relative_path: &str) -> Result<String, String> {
    let output = run_git_text(
        root,
        &[
            "status",
            "--porcelain=v1",
            "--untracked-files=normal",
            "--",
            relative_path,
        ],
    )?;
    Ok(parse_project_git_status(&output)?
        .files
        .into_iter()
        .next()
        .map(|file| file.status)
        .unwrap_or_default())
}

fn run_git_text(root: &Path, args: &[&str]) -> Result<String, String> {
    let root_arg = root.display().to_string();
    let output = Command::new("git")
        .arg("-C")
        .arg(root_arg)
        .args(args)
        .output()
        .map_err(|error| format!("Could not run git {}: {error}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("git {} exited with {}", args.join(" "), output.status)
        } else {
            stderr
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn combine_source_git_diffs(staged_diff: &str, working_diff: &str) -> String {
    let staged_diff = staged_diff.trim_end();
    let working_diff = working_diff.trim_end();

    match (staged_diff.is_empty(), working_diff.is_empty()) {
        (true, true) => String::new(),
        (false, true) => staged_diff.to_string(),
        (true, false) => working_diff.to_string(),
        (false, false) => {
            format!("## Staged\n{staged_diff}\n\n## Working tree\n{working_diff}")
        }
    }
}

fn list_project_worktrees_sync(root: PathBuf) -> Result<Vec<ProjectWorktree>, String> {
    let metadata = std::fs::metadata(&root)
        .map_err(|error| format!("Could not read worktree root metadata: {error}"))?;
    if !metadata.is_dir() {
        return Err("Worktree root is not a directory".to_string());
    }

    let root_arg = root.display().to_string();
    let output = Command::new("git")
        .args(["-C", root_arg.as_str(), "worktree", "list", "--porcelain"])
        .output()
        .map_err(|error| format!("Could not run git worktree list: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("git worktree list exited with {}", output.status)
        } else {
            stderr
        });
    }

    let repo = path_last_segment(&root).unwrap_or_else(|| "repo".to_string());
    Ok(
        parse_project_worktree_porcelain(&String::from_utf8_lossy(&output.stdout))
            .into_iter()
            .map(|mut record| {
                record.repo = repo.clone();
                record.is_dirty = project_worktree_is_dirty(&record.path);
                record.has_unmerged_commits = project_worktree_has_unmerged_commits(&record.path);
                record.last_activity = project_worktree_last_activity(&record.path);
                record.delete_eligibility = project_worktree_delete_eligibility(
                    record.is_dirty,
                    record.has_unmerged_commits,
                );
                record
            })
            .collect(),
    )
}

fn parse_project_worktree_porcelain(output: &str) -> Vec<ProjectWorktree> {
    output
        .split("\n\n")
        .filter_map(|block| {
            let mut path = None;
            let mut branch = None;

            for line in block.lines() {
                if let Some(value) = line.strip_prefix("worktree ") {
                    path = Some(value.to_string());
                } else if let Some(value) = line.strip_prefix("branch ") {
                    branch = Some(value.trim_start_matches("refs/heads/").to_string());
                } else if line == "detached" {
                    branch = Some("detached".to_string());
                }
            }

            path.map(|path| {
                let branch = branch.unwrap_or_else(|| "unknown".to_string());
                let task_id = branch_task_id(&branch);
                ProjectWorktree {
                    repo: String::new(),
                    path,
                    branch,
                    task_id,
                    is_dirty: false,
                    has_unmerged_commits: false,
                    last_activity: None,
                    delete_eligibility: "unknown".to_string(),
                }
            })
        })
        .collect()
}

fn project_worktree_delete_eligibility(is_dirty: bool, has_unmerged_commits: bool) -> String {
    if is_dirty {
        "blocked: dirty worktree".to_string()
    } else if has_unmerged_commits {
        "blocked: unmerged commits".to_string()
    } else {
        "requires-confirmation".to_string()
    }
}

fn project_worktree_is_dirty(path: &str) -> bool {
    let output = Command::new("git")
        .args(["-C", path, "status", "--short"])
        .output();
    output
        .ok()
        .filter(|output| output.status.success())
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false)
}

fn project_worktree_has_unmerged_commits(path: &str) -> bool {
    let output = Command::new("git")
        .args([
            "-C",
            path,
            "log",
            "--branches",
            "--not",
            "--remotes",
            "--oneline",
        ])
        .output();
    output
        .ok()
        .filter(|output| output.status.success())
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false)
}

fn project_worktree_last_activity(path: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", path, "log", "-1", "--format=%cI"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn list_git_repository_summaries_sync(
    projects: Vec<RuntimeContextProject>,
) -> Result<Vec<GitRepositorySummary>, String> {
    let mut seen_paths = HashSet::new();
    let mut summaries = Vec::new();

    for project in projects {
        for (path, is_worktree) in git_repository_paths_for_project(&project) {
            let normalized_path = normalized_path_string(&path);
            if !seen_paths.insert(normalized_path) {
                continue;
            }

            summaries.push(git_repository_summary_for_path(&project, path, is_worktree));
        }
    }

    summaries.sort_by(|left, right| {
        left.project_name
            .cmp(&right.project_name)
            .then(left.root_label.cmp(&right.root_label))
            .then(left.branch.cmp(&right.branch))
    });
    Ok(summaries)
}

fn git_repository_paths_for_project(project: &RuntimeContextProject) -> Vec<(PathBuf, bool)> {
    let project_path = PathBuf::from(&project.path);
    let project_path_string = normalized_path_string(&project_path);
    let mut seen = HashSet::from([project_path_string]);
    let mut paths = vec![(project_path.clone(), false)];

    if let Ok(worktrees) = list_project_worktree_paths(&project_path) {
        for worktree_path in worktrees {
            let normalized_path = normalized_path_string(&worktree_path);
            if seen.insert(normalized_path) {
                paths.push((worktree_path, true));
            }
        }
    }

    paths
}

fn list_project_worktree_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    let output = run_git_text(root, &["worktree", "list", "--porcelain"])?;
    Ok(parse_project_worktree_porcelain(&output)
        .into_iter()
        .map(|worktree| PathBuf::from(worktree.path))
        .collect())
}

fn git_repository_summary_for_path(
    project: &RuntimeContextProject,
    path: PathBuf,
    is_worktree: bool,
) -> GitRepositorySummary {
    git_repository_summary_for_path_result(project, &path, is_worktree).unwrap_or_else(|error| {
        empty_git_repository_summary(project, &path, is_worktree, Some(error))
    })
}

fn git_repository_summary_for_path_result(
    project: &RuntimeContextProject,
    path: &Path,
    is_worktree: bool,
) -> Result<GitRepositorySummary, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("Could not read repository metadata: {error}"))?;
    if !metadata.is_dir() {
        return Err("Repository path is not a directory".to_string());
    }

    let status = project_git_status_sync(path.to_path_buf())?;
    let counts = git_repository_status_counts(&status.files);
    let (last_commit_sha, last_commit_subject, last_commit_at) = git_last_commit(path);
    let branch = status.branch.unwrap_or_else(|| "unknown".to_string());
    let task_id = branch_task_id(&branch)
        .or_else(|| last_commit_subject.as_deref().and_then(task_id_from_text));
    let dirty_count = counts.staged_count + counts.unstaged_count + counts.untracked_count;

    Ok(GitRepositorySummary {
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        repo: path_last_segment(Path::new(&project.path)).unwrap_or_else(|| project.name.clone()),
        path: path.display().to_string(),
        root_label: runtime_context_root_label(path),
        branch: branch.clone(),
        task_id,
        is_worktree,
        is_dirty: dirty_count > 0,
        staged_count: counts.staged_count,
        unstaged_count: counts.unstaged_count,
        untracked_count: counts.untracked_count,
        dirty_count,
        ahead: status.ahead,
        behind: status.behind,
        last_commit_sha,
        last_commit_subject,
        last_commit_at,
        dirty_since_epoch_ms: git_dirty_since_epoch_ms(path, &status.files),
        error: None,
    })
}

fn empty_git_repository_summary(
    project: &RuntimeContextProject,
    path: &Path,
    is_worktree: bool,
    error: Option<String>,
) -> GitRepositorySummary {
    GitRepositorySummary {
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        repo: path_last_segment(Path::new(&project.path)).unwrap_or_else(|| project.name.clone()),
        path: path.display().to_string(),
        root_label: runtime_context_root_label(path),
        branch: "unknown".to_string(),
        task_id: None,
        is_worktree,
        is_dirty: false,
        staged_count: 0,
        unstaged_count: 0,
        untracked_count: 0,
        dirty_count: 0,
        ahead: 0,
        behind: 0,
        last_commit_sha: None,
        last_commit_subject: None,
        last_commit_at: None,
        dirty_since_epoch_ms: None,
        error,
    }
}

#[derive(Default)]
struct GitRepositoryStatusCounts {
    staged_count: usize,
    unstaged_count: usize,
    untracked_count: usize,
}

fn git_repository_status_counts(files: &[GitFileStatus]) -> GitRepositoryStatusCounts {
    let mut counts = GitRepositoryStatusCounts::default();

    for file in files {
        if file.status == "untracked" {
            counts.untracked_count += 1;
            continue;
        }

        if !file.index_status.is_empty() {
            counts.staged_count += 1;
        }
        if !file.worktree_status.is_empty() {
            counts.unstaged_count += 1;
        }
    }

    counts
}

fn git_last_commit(root: &Path) -> (Option<String>, Option<String>, Option<String>) {
    let Ok(output) = run_git_text(root, &["log", "-1", "--format=%h%x1f%s%x1f%cI"]) else {
        return (None, None, None);
    };

    let mut parts = output.trim().split('\x1f');
    let sha = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let subject = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let committed_at = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    (sha, subject, committed_at)
}

fn git_dirty_since_epoch_ms(root: &Path, files: &[GitFileStatus]) -> Option<u64> {
    files
        .iter()
        .filter_map(|file| {
            std::fs::metadata(root.join(&file.relative_path))
                .ok()?
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        })
        .min()
}

fn parse_git_commit_history(output: &str) -> Result<Vec<GitCommitHistoryEntry>, String> {
    let mut entries = Vec::new();

    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let mut parts = line.split('\x1f');
        let short_sha = parts.next().unwrap_or_default().trim().to_string();
        let sha = parts.next().unwrap_or_default().trim().to_string();
        let subject = parts.next().unwrap_or_default().trim().to_string();
        let author = parts.next().unwrap_or_default().trim().to_string();
        let committed_at = parts.next().unwrap_or_default().trim().to_string();
        let refs = parts.next().unwrap_or_default().trim().to_string();
        let parent_shas = parts
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .map(str::trim)
            .filter(|parent| !parent.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let parent_count = parent_shas.len();

        if short_sha.is_empty() || sha.is_empty() {
            return Err("Could not parse Git history entry".to_string());
        }

        let (task_id, task_source) = match task_id_from_text(&refs) {
            Some(task_id) => (Some(task_id), Some("refs".to_string())),
            None => match task_id_from_text(&subject) {
                Some(task_id) => (Some(task_id), Some("subject".to_string())),
                None => (None, None),
            },
        };
        entries.push(GitCommitHistoryEntry {
            short_sha,
            sha,
            subject,
            author,
            committed_at,
            refs,
            parent_shas,
            parent_count,
            task_id,
            task_source,
        });
    }

    Ok(entries)
}

fn branch_task_id(branch: &str) -> Option<String> {
    task_id_from_text(branch)
}

fn task_id_from_text(text: &str) -> Option<String> {
    let lower_text = text.to_ascii_lowercase();
    for (index, _) in lower_text.match_indices("tsk") {
        let suffix = lower_text[index + 3..].trim_start_matches(['-', '_', '/', '#', '[', ' ']);
        let digits: String = suffix
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            return Some(format!("TSK-{digits}"));
        }
    }

    None
}

fn list_runtime_contexts_sync(
    projects: Vec<RuntimeContextProject>,
) -> Result<Vec<RuntimeContext>, String> {
    let output = Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-Fpcn"])
        .output()
        .map_err(|error| format!("Could not run lsof: {error}"))?;

    if !output.status.success() && output.stdout.is_empty() {
        return Err(format!(
            "Could not list listening processes: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let listeners = parse_lsof_tcp_listeners(&String::from_utf8_lossy(&output.stdout));
    let mut cwd_by_pid: HashMap<u32, Option<PathBuf>> = HashMap::new();
    let mut seen = HashSet::new();
    let mut contexts = Vec::new();

    for listener in listeners {
        if !seen.insert((listener.pid, listener.port)) {
            continue;
        }

        let cwd = cwd_by_pid
            .entry(listener.pid)
            .or_insert_with(|| process_cwd(listener.pid));
        let Some(cwd) = cwd.as_ref() else {
            continue;
        };
        let Some(project_match) = runtime_context_project_for_cwd(cwd, &projects) else {
            continue;
        };

        contexts.push(RuntimeContext {
            pid: listener.pid,
            command: listener.command,
            port: listener.port,
            cwd: cwd.display().to_string(),
            project_id: project_match.project_id,
            project_name: project_match.project_name,
            root_label: project_match.root_label,
        });
    }

    contexts.sort_by(|left, right| {
        left.project_name
            .cmp(&right.project_name)
            .then(left.root_label.cmp(&right.root_label))
            .then(left.port.cmp(&right.port))
            .then(left.command.cmp(&right.command))
    });
    Ok(contexts)
}

fn parse_lsof_tcp_listeners(output: &str) -> Vec<ProcessListener> {
    let mut listeners = Vec::new();
    let mut current_pid: Option<u32> = None;
    let mut current_command = String::new();

    for line in output.lines().filter(|line| !line.is_empty()) {
        let (field, value) = line.split_at(1);
        match field {
            "p" => {
                current_pid = value.parse::<u32>().ok();
                current_command.clear();
            }
            "c" => {
                current_command = value.to_string();
            }
            "n" => {
                let Some(pid) = current_pid else {
                    continue;
                };
                let Some(port) = parse_lsof_tcp_port(value) else {
                    continue;
                };

                listeners.push(ProcessListener {
                    pid,
                    command: current_command.clone(),
                    port,
                });
            }
            _ => {}
        }
    }

    listeners
}

fn parse_lsof_tcp_port(name: &str) -> Option<u16> {
    let endpoint = name.split("->").next().unwrap_or(name).trim();
    let port_text = endpoint.rsplit(':').next()?.trim();
    let digits: String = port_text
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();

    if digits.is_empty() {
        return None;
    }

    digits.parse::<u16>().ok()
}

fn process_cwd(pid: u32) -> Option<PathBuf> {
    let output = Command::new("lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
        .output()
        .ok()?;

    if !output.status.success() && output.stdout.is_empty() {
        return None;
    }

    parse_lsof_cwd(&String::from_utf8_lossy(&output.stdout))
}

fn parse_lsof_cwd(output: &str) -> Option<PathBuf> {
    output
        .lines()
        .filter_map(|line| line.strip_prefix('n'))
        .map(str::trim)
        .find(|path| !path.is_empty())
        .map(PathBuf::from)
}

fn runtime_context_project_for_cwd(
    cwd: &Path,
    projects: &[RuntimeContextProject],
) -> Option<RuntimeContextProjectMatch> {
    let cwd_path = normalized_path_string(cwd);

    for project in projects {
        let project_path = normalized_path_string(Path::new(&project.path));
        if path_is_within(&cwd_path, &project_path) {
            return Some(RuntimeContextProjectMatch {
                project_id: Some(project.id.clone()),
                project_name: project.name.clone(),
                root_label: runtime_context_root_label(Path::new(&project.path)),
            });
        }
    }

    let (worktree_repo, _) = worktree_repo_and_slug(cwd)?;
    projects
        .iter()
        .find(|project| {
            project.name.eq_ignore_ascii_case(&worktree_repo)
                || path_last_segment(Path::new(&project.path))
                    .is_some_and(|segment| segment.eq_ignore_ascii_case(&worktree_repo))
        })
        .map(|project| RuntimeContextProjectMatch {
            project_id: Some(project.id.clone()),
            project_name: project.name.clone(),
            root_label: runtime_context_root_label(cwd),
        })
}

fn runtime_context_root_label(path: &Path) -> String {
    if let Some((_, slug)) = worktree_repo_and_slug(path) {
        return format!("worktree:{slug}");
    }

    let segments = path_segments(path);
    if segments
        .len()
        .checked_sub(2)
        .and_then(|index| segments.get(index))
        .is_some_and(|segment| segment == "work")
    {
        return "main checkout".to_string();
    }

    path_last_segment(path).unwrap_or_else(|| "unknown".to_string())
}

fn worktree_repo_and_slug(path: &Path) -> Option<(String, String)> {
    let segments = path_segments(path);
    let worktrees_index = segments.iter().position(|segment| segment == "worktrees")?;
    let repo = segments.get(worktrees_index + 1)?.clone();
    let slug = segments.get(worktrees_index + 2)?.clone();
    Some((repo, slug))
}

fn path_segments(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| component.as_os_str().to_str())
        .filter(|segment| !segment.is_empty() && *segment != "/")
        .map(ToString::to_string)
        .collect()
}

fn path_last_segment(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|segment| segment.to_str())
        .filter(|segment| !segment.is_empty())
        .map(ToString::to_string)
}

fn normalized_path_string(path: &Path) -> String {
    path.to_string_lossy().trim_end_matches('/').to_string()
}

fn path_is_within(path: &str, root: &str) -> bool {
    path == root
        || path
            .strip_prefix(root)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn parse_project_git_status(output: &str) -> Result<ProjectGitStatus, String> {
    let mut git_status = ProjectGitStatus {
        branch: None,
        ahead: 0,
        behind: 0,
        files: Vec::new(),
    };

    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        if let Some(header) = line.strip_prefix("## ") {
            let (branch, ahead, behind) = parse_git_branch_header(header);
            git_status.branch = branch;
            git_status.ahead = ahead;
            git_status.behind = behind;
            continue;
        }

        if line.len() < 4 {
            continue;
        }

        let index_code = line.chars().next().unwrap_or(' ');
        let worktree_code = line.chars().nth(1).unwrap_or(' ');
        let relative_path = normalize_git_status_path(&line[3..]);
        if relative_path.is_empty() {
            continue;
        }

        let index_status = git_status_name(index_code).to_string();
        let worktree_status = git_status_name(worktree_code).to_string();
        let status = if !worktree_status.is_empty() {
            worktree_status.clone()
        } else {
            index_status.clone()
        };
        let badge = git_status_badge(index_code, worktree_code).to_string();

        git_status.files.push(GitFileStatus {
            relative_path,
            index_status,
            worktree_status,
            status,
            badge,
        });
    }

    Ok(git_status)
}

fn parse_git_branch_header(header: &str) -> (Option<String>, usize, usize) {
    let mut branch_part = header.trim();
    let mut ahead = 0;
    let mut behind = 0;

    if let Some(metadata_index) = branch_part.find(" [") {
        let metadata = branch_part[metadata_index + 2..].trim_end_matches(']');
        branch_part = &branch_part[..metadata_index];
        for item in metadata.split(',').map(str::trim) {
            if let Some(value) = item.strip_prefix("ahead ") {
                ahead = value.parse::<usize>().unwrap_or(0);
            } else if let Some(value) = item.strip_prefix("behind ") {
                behind = value.parse::<usize>().unwrap_or(0);
            }
        }
    }

    if let Some(branch) = branch_part.strip_prefix("No commits yet on ") {
        return (Some(branch.trim().to_string()), ahead, behind);
    }

    let branch = branch_part
        .split("...")
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    (branch, ahead, behind)
}

fn normalize_git_status_path(path: &str) -> String {
    path.split(" -> ")
        .last()
        .unwrap_or(path)
        .trim_matches('"')
        .trim()
        .to_string()
}

fn git_status_name(code: char) -> &'static str {
    match code {
        'M' => "modified",
        'A' => "added",
        'D' => "deleted",
        'R' => "renamed",
        'C' => "copied",
        'U' => "unmerged",
        '?' => "untracked",
        '!' => "ignored",
        _ => "",
    }
}

fn git_status_badge(index_code: char, worktree_code: char) -> &'static str {
    let status_code = if worktree_code != ' ' {
        worktree_code
    } else {
        index_code
    };

    match status_code {
        '?' => "?",
        'M' => "M",
        'A' => "A",
        'D' => "D",
        'R' => "R",
        'C' => "C",
        'U' => "U",
        '!' => "!",
        _ => "",
    }
}

fn detect_language(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if file_name == "dockerfile" || file_name.ends_with(".dockerfile") {
        return "dockerfile".to_string();
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase();

    match extension.as_str() {
        "cs" => "csharp",
        "csproj" | "fsproj" | "vbproj" | "props" | "targets" | "xaml" | "xml" => "xml",
        "swift" => "swift",
        "rs" => "rust",
        "ts" => "typescript",
        "tsx" => "tsx",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "jsx",
        "svelte" => "svelte",
        "astro" | "html" | "htm" | "vue" => "html",
        "css" => "css",
        "scss" => "scss",
        "less" => "less",
        "json" | "jsonc" => "json",
        "md" | "markdown" => "markdown",
        "mdx" => "mdx",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "ini" | "env" => "ini",
        "sh" | "bash" | "zsh" => "shell",
        "ps1" | "psm1" => "powershell",
        "py" => "python",
        "rb" => "ruby",
        "php" => "php",
        "dart" => "dart",
        "lua" => "lua",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "c" | "cc" | "cpp" | "cxx" | "h" | "hh" | "hpp" | "hxx" => "cpp",
        "tf" | "tfvars" => "hcl",
        "proto" => "protobuf",
        "sql" => "sql",
        "graphql" | "gql" => "graphql",
        "fs" | "fsx" => "fsharp",
        "razor" => "razor",
        _ => "plain",
    }
    .to_string()
}

fn is_source_file(path: &Path) -> bool {
    !matches!(detect_language(path).as_str(), "plain")
}

fn compare_source_walk_entries(
    root: &Path,
    left: &std::fs::DirEntry,
    right: &std::fs::DirEntry,
) -> std::cmp::Ordering {
    let left_path = left.path();
    let right_path = right.path();
    let left_file_type = left.file_type().ok();
    let right_file_type = right.file_type().ok();
    let left_is_directory = left_file_type
        .as_ref()
        .map(|file_type| file_type.is_dir())
        .unwrap_or(false);
    let right_is_directory = right_file_type
        .as_ref()
        .map(|file_type| file_type.is_dir())
        .unwrap_or(false);
    let left_language = if left_file_type
        .as_ref()
        .map(|file_type| file_type.is_file())
        .unwrap_or(false)
    {
        detect_language(&left_path)
    } else {
        String::new()
    };
    let right_language = if right_file_type
        .as_ref()
        .map(|file_type| file_type.is_file())
        .unwrap_or(false)
    {
        detect_language(&right_path)
    } else {
        String::new()
    };
    let left_relative_path = normalized_relative_source_path(root, &left_path);
    let right_relative_path = normalized_relative_source_path(root, &right_path);

    source_path_priority(&left_relative_path, &left_language, left_is_directory, root)
        .cmp(&source_path_priority(
            &right_relative_path,
            &right_language,
            right_is_directory,
            root,
        ))
        .then_with(|| localized_path_compare(&left_relative_path, &right_relative_path))
}

fn compare_source_records(left: &SourceRecord, right: &SourceRecord) -> std::cmp::Ordering {
    source_path_priority(&left.relative_path, &left.language, false, Path::new(""))
        .cmp(&source_path_priority(
            &right.relative_path,
            &right.language,
            false,
            Path::new(""),
        ))
        .then_with(|| localized_path_compare(&left.relative_path, &right.relative_path))
}

fn normalized_relative_source_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn source_path_priority(
    relative_path: &str,
    language: &str,
    is_directory: bool,
    root: &Path,
) -> i32 {
    let normalized_path = relative_path.replace('\\', "/").to_ascii_lowercase();
    let segments = normalized_path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let first_segment = segments.first().copied().unwrap_or_default();
    let project_name = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut score = if is_directory { -6 } else { 0 };

    score += source_root_priority(first_segment, &project_name);
    score += source_language_priority(language);
    score += source_path_segment_adjustment(&segments);

    if first_segment.starts_with('.') {
        score += 80;
    }

    score
}

fn source_root_priority(first_segment: &str, project_name: &str) -> i32 {
    if first_segment.is_empty() {
        return 100;
    }

    let project_prefix = if project_name.is_empty() {
        String::new()
    } else {
        format!("{project_name}.")
    };
    if !project_name.is_empty()
        && (first_segment == project_name || first_segment.starts_with(&project_prefix))
    {
        return project_root_segment_adjustment(first_segment);
    }

    if matches!(
        first_segment,
        "src" | "source" | "sources" | "lib" | "app" | "apps" | "packages"
    ) {
        return 0;
    }

    if looks_like_source_project_segment(first_segment) {
        return 8 + project_root_segment_adjustment(first_segment);
    }
    if first_segment.contains("test") || first_segment.contains("spec") {
        return 26;
    }
    if matches!(first_segment, "scripts" | "tools") {
        return 34;
    }
    if matches!(
        first_segment,
        "config" | "deploy" | "infra" | "infrastructure" | ".config" | ".github"
    ) {
        return 60;
    }
    if matches!(first_segment, "docs" | "doc" | "documentation") {
        return 75;
    }

    45
}

fn looks_like_source_project_segment(segment: &str) -> bool {
    segment.contains('.')
        || segment.ends_with("-web")
        || segment.ends_with("-api")
        || segment.ends_with("-core")
        || segment.ends_with("-engine")
        || segment.ends_with("-data")
        || matches!(segment, "web" | "client" | "server")
}

fn project_root_segment_adjustment(segment: &str) -> i32 {
    if segment.ends_with(".core") || segment.ends_with("-core") {
        return -8;
    }
    if segment.ends_with(".api") || segment.ends_with("-api") {
        return -6;
    }
    if segment.ends_with(".engine") || segment.ends_with("-engine") {
        return -5;
    }
    if segment.ends_with(".data") || segment.ends_with("-data") {
        return -4;
    }
    if segment.ends_with("-web") || segment == "web" || segment == "client" {
        return -3;
    }
    if segment.contains("test") || segment.contains("spec") {
        return 18;
    }

    0
}

fn source_language_priority(language: &str) -> i32 {
    if matches!(
        language,
        "csharp"
            | "typescript"
            | "tsx"
            | "svelte"
            | "javascript"
            | "jsx"
            | "rust"
            | "swift"
            | "go"
            | "python"
            | "java"
            | "kotlin"
            | "cpp"
            | "dart"
            | "fsharp"
            | "razor"
    ) {
        return 0;
    }

    if matches!(
        language,
        "json"
            | "yaml"
            | "toml"
            | "xml"
            | "shell"
            | "powershell"
            | "sql"
            | "graphql"
            | "protobuf"
            | "hcl"
            | "dockerfile"
            | "makefile"
    ) {
        return 28;
    }

    if language == "markdown" || language == "mdx" {
        return 55;
    }

    80
}

fn source_path_segment_adjustment(segments: &[&str]) -> i32 {
    let mut score = 0;

    if [
        "services",
        "controllers",
        "routes",
        "components",
        "pages",
        "models",
        "entities",
        "features",
    ]
    .iter()
    .any(|segment| segments.contains(segment))
    {
        score -= 7;
    }

    if [
        "migrations",
        "generated",
        "snapshots",
        "fixtures",
        "samples",
        "docs",
        "documentation",
    ]
    .iter()
    .any(|segment| segments.contains(segment))
    {
        score += 22;
    }

    score
}

fn localized_path_compare(left: &str, right: &str) -> std::cmp::Ordering {
    left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase())
}

fn should_skip_dir(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    if normalized.ends_with("_files") {
        return true;
    }

    matches!(
        normalized.as_str(),
        "__pycache__"
            | ".cache"
            | ".git"
            | ".hg"
            | ".svn"
            | ".agents"
            | ".build"
            | ".claude"
            | ".codex"
            | ".dev"
            | ".gradle"
            | ".history"
            | ".idea"
            | ".merge-backups"
            | ".next"
            | ".nuxt"
            | ".omx"
            | ".parcel-cache"
            | ".playwright"
            | ".playwright-cli"
            | ".pytest_cache"
            | ".run"
            | ".slots"
            | ".svelte-kit"
            | ".tmp"
            | ".turbo"
            | ".vite"
            | ".vscode"
            | ".zed"
            | "bin"
            | "build"
            | "coverage"
            | "deriveddata"
            | "dist"
            | "node_modules"
            | "obj"
            | "pods"
            | "target"
            | "testresults"
            | "vendor"
            | "worktrees"
    )
}

fn source_file_matches_query(relative_path: &str, file_name: &str, query: Option<&str>) -> bool {
    let Some(query) = query else {
        return true;
    };
    relative_path.to_lowercase().contains(query) || file_name.to_lowercase().contains(query)
}

fn main() {
    tauri::Builder::default()
        .manage(SourceScanRegistry::default())
        .manage(lsp::SourceLspRegistry::default())
        .manage(terminal::TerminalRegistry::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_source_files,
            cancel_source_scan,
            read_source_file,
            write_source_file,
            open_source_file,
            reveal_source_file,
            open_path,
            reveal_path,
            open_terminal_path,
            open_terminal_command,
            search_source_files,
            find_source_definitions,
            find_source_references,
            read_source_lsp_status,
            find_source_lsp_definitions,
            find_source_lsp_completions,
            find_source_lsp_implementations,
            find_source_lsp_type_definitions,
            find_source_lsp_document_highlights,
            find_source_lsp_signature_help,
            find_source_lsp_inlay_hints,
            find_source_lsp_semantic_tokens,
            format_source_with_lsp,
            rename_source_with_lsp,
            find_source_lsp_code_actions,
            find_source_lsp_references,
            find_source_lsp_hover,
            find_source_lsp_symbols,
            read_source_lsp_diagnostics,
            project_git_status,
            read_source_git_diff,
            stage_git_paths,
            unstage_git_paths,
            commit_git_repository,
            fetch_git_repository,
            pull_git_repository,
            push_git_repository,
            read_git_commit_history,
            list_project_worktrees,
            list_git_repository_summaries,
            list_agent_sessions,
            list_runtime_contexts,
            list_orchestration_runs,
            record_orchestration_event,
            start_terminal_session,
            list_terminal_sessions,
            read_terminal_session_scrollback,
            write_terminal_session,
            resize_terminal_session,
            close_terminal_session
        ])
        .run(tauri::generate_context!())
        .expect("failed to run MacCommandBar webview preview");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn source_scan_groups_supported_files_and_skips_build_dirs() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("packages/ui")).unwrap();
        std::fs::create_dir_all(root.join("src/Workers")).unwrap();
        std::fs::create_dir_all(root.join("target/debug")).unwrap();
        std::fs::write(
            root.join("packages/ui/Button.tsx"),
            "export function Button() {}",
        )
        .unwrap();
        std::fs::write(root.join("Package.swift"), "let package = 1").unwrap();
        std::fs::write(root.join("src/App.svelte"), "<script></script>").unwrap();
        std::fs::write(root.join("src/settings.json"), "{}").unwrap();
        std::fs::write(root.join("src/Workers/Worker.cs"), "public class Worker {}").unwrap();
        std::fs::write(root.join("target/debug/generated.rs"), "fn generated() {}").unwrap();
        std::fs::write(root.join("README.md"), "# docs").unwrap();

        let scan = list_source_files_sync(root.clone(), 20, None).unwrap();
        let relative_paths = scan
            .records
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            relative_paths,
            vec![
                "packages/ui/Button.tsx",
                "src/App.svelte",
                "src/Workers/Worker.cs",
                "Package.swift",
                "src/settings.json",
                "README.md"
            ]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_scan_prioritizes_app_source_before_docs_when_truncated() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("Docs")).unwrap();
        std::fs::create_dir_all(root.join("EdiPlatform.Core/Models")).unwrap();
        std::fs::create_dir_all(root.join("EdiPlatform.Core/Services")).unwrap();
        std::fs::create_dir_all(root.join("src/Services")).unwrap();
        std::fs::write(root.join("Docs/A.md"), "# docs").unwrap();
        std::fs::write(root.join("Docs/B.md"), "# docs").unwrap();
        std::fs::write(
            root.join("EdiPlatform.Core/Models/RuntimeModel.cs"),
            "public sealed class RuntimeModel {}",
        )
        .unwrap();
        std::fs::write(
            root.join("EdiPlatform.Core/Services/RuntimeService.cs"),
            "public sealed class RuntimeService {}",
        )
        .unwrap();
        std::fs::write(
            root.join("src/Services/FormatResolver.cs"),
            "public sealed class FormatResolver {}",
        )
        .unwrap();

        let scan = list_source_files_sync(root.clone(), 3, None).unwrap();
        let relative_paths = scan
            .records
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            relative_paths,
            vec![
                "EdiPlatform.Core/Models/RuntimeModel.cs",
                "EdiPlatform.Core/Services/RuntimeService.cs",
                "src/Services/FormatResolver.cs"
            ]
        );
        assert!(scan.truncated);
        assert!(scan
            .records
            .iter()
            .all(|record| !record.relative_path.starts_with("Docs/")));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_scan_skips_tool_cache_and_coverage_dirs() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join(".cache/generated")).unwrap();
        std::fs::create_dir_all(root.join(".history/Docs")).unwrap();
        std::fs::create_dir_all(root.join(".idea")).unwrap();
        std::fs::create_dir_all(root.join(".pytest_cache")).unwrap();
        std::fs::create_dir_all(root.join(".vscode")).unwrap();
        std::fs::create_dir_all(root.join(".turbo/cache")).unwrap();
        std::fs::create_dir_all(root.join(".parcel-cache")).unwrap();
        std::fs::create_dir_all(root.join(".nuxt")).unwrap();
        std::fs::create_dir_all(root.join(".vite/deps")).unwrap();
        std::fs::create_dir_all(root.join("coverage/lcov-report")).unwrap();
        std::fs::create_dir_all(root.join("DerivedData/Build")).unwrap();
        std::fs::create_dir_all(root.join("TestResults/run")).unwrap();
        std::fs::create_dir_all(root.join("Pods/SomeDependency")).unwrap();
        std::fs::write(root.join("src/Keep.ts"), "export const keep = true;").unwrap();
        std::fs::write(
            root.join(".cache/generated/Cache.ts"),
            "export const cache = true;",
        )
        .unwrap();
        std::fs::write(root.join(".history/Docs/Old.md"), "# old").unwrap();
        std::fs::write(root.join(".idea/workspace.xml"), "<project />").unwrap();
        std::fs::write(root.join(".pytest_cache/README.md"), "# cache").unwrap();
        std::fs::write(root.join(".vscode/settings.json"), "{}").unwrap();
        std::fs::write(
            root.join(".turbo/cache/Turbo.ts"),
            "export const turbo = true;",
        )
        .unwrap();
        std::fs::write(
            root.join(".parcel-cache/Parcel.ts"),
            "export const parcel = true;",
        )
        .unwrap();
        std::fs::write(root.join(".nuxt/App.vue"), "<template></template>").unwrap();
        std::fs::write(root.join(".vite/deps/Vite.ts"), "export const vite = true;").unwrap();
        std::fs::write(
            root.join("coverage/lcov-report/Coverage.ts"),
            "export const covered = true;",
        )
        .unwrap();
        std::fs::write(
            root.join("DerivedData/Build/Generated.swift"),
            "let generated = true",
        )
        .unwrap();
        std::fs::write(
            root.join("TestResults/run/TestLog.cs"),
            "public class TestLog {}",
        )
        .unwrap();
        std::fs::write(
            root.join("Pods/SomeDependency/Dependency.swift"),
            "let dependency = true",
        )
        .unwrap();

        let scan = list_source_files_sync(root.clone(), 20, None).unwrap();
        let relative_paths = scan
            .records
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(relative_paths, vec!["src/Keep.ts"]);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_language_detection_covers_common_project_files() {
        assert_eq!(detect_language(Path::new("Program.cs")), "csharp");
        assert_eq!(detect_language(Path::new("Package.swift")), "swift");
        assert_eq!(detect_language(Path::new("main.rs")), "rust");
        assert_eq!(
            detect_language(Path::new("src/routes/+page.svelte")),
            "svelte"
        );
        assert_eq!(detect_language(Path::new("src/main.tsx")), "tsx");
        assert_eq!(detect_language(Path::new("src/app.jsx")), "jsx");
        assert_eq!(detect_language(Path::new("README.md")), "markdown");
        assert_eq!(detect_language(Path::new("package.json")), "json");
        assert_eq!(detect_language(Path::new("pnpm-lock.yaml")), "yaml");
        assert_eq!(detect_language(Path::new("Cargo.toml")), "toml");
        assert_eq!(detect_language(Path::new("scripts/build.sh")), "shell");
        assert_eq!(detect_language(Path::new("tools/import.py")), "python");
        assert_eq!(detect_language(Path::new("Dockerfile")), "dockerfile");
        assert_eq!(detect_language(Path::new("EdiPlatform.Api.csproj")), "xml");
    }

    #[test]
    fn source_language_detection_covers_more_preview_formats() {
        assert_eq!(detect_language(Path::new("web/App.vue")), "html");
        assert_eq!(detect_language(Path::new("web/Page.astro")), "html");
        assert_eq!(detect_language(Path::new("index.php")), "php");
        assert_eq!(detect_language(Path::new("lib/main.dart")), "dart");
        assert_eq!(detect_language(Path::new("scripts/tool.lua")), "lua");
        assert_eq!(detect_language(Path::new("infra/main.tf")), "hcl");
        assert_eq!(detect_language(Path::new("infra/dev.tfvars")), "hcl");
        assert_eq!(
            detect_language(Path::new("schemas/service.proto")),
            "protobuf"
        );
    }

    #[test]
    fn source_scan_filters_by_query() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src/Workers")).unwrap();
        std::fs::write(root.join("src/App.svelte"), "<script></script>").unwrap();
        std::fs::write(root.join("src/Workers/Worker.cs"), "public class Worker {}").unwrap();

        let scan = list_source_files_sync(root.clone(), 20, Some("worker".to_string())).unwrap();

        assert_eq!(scan.records.len(), 1);
        assert_eq!(scan.records[0].relative_path, "src/Workers/Worker.cs");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_scan_default_limit_covers_large_project_trees() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        for index in 0..350 {
            std::fs::write(
                root.join(format!("src/File{index:03}.ts")),
                "export const value = 1;",
            )
            .unwrap();
        }

        let scan = list_source_files_sync(root.clone(), 0, None).unwrap();

        assert_eq!(scan.records.len(), 350);
        assert!(!scan.truncated);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_scan_allows_explicit_limits_above_the_default_preview_cap() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        for index in 0..1_200 {
            std::fs::write(
                root.join(format!("src/File{index:04}.ts")),
                "export const value = 1;",
            )
            .unwrap();
        }

        let scan = list_source_files_sync(root.clone(), 1_200, None).unwrap();

        assert_eq!(scan.records.len(), 1_200);
        assert!(!scan.truncated);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_scan_reports_when_the_limit_was_reached() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/A.ts"), "export const a = 1;").unwrap();
        std::fs::write(root.join("src/B.ts"), "export const b = 1;").unwrap();

        let scan = list_source_files_sync(root.clone(), 1, None).unwrap();

        assert_eq!(scan.records.len(), 1);
        assert_eq!(scan.limit, 1);
        assert!(scan.truncated);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_scan_can_be_cancelled_before_collection() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/A.ts"), "export const a = 1;").unwrap();

        let cancellation = SourceScanCancellation::cancelled_for_test();
        let error = list_source_files_sync_with_cancellation(root.clone(), 20, None, cancellation)
            .unwrap_err();

        assert!(error.contains("Source scan cancelled"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_scan_reports_progress_during_collection() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/A.ts"), "export const a = 1;").unwrap();
        std::fs::write(root.join("src/B.ts"), "export const b = 1;").unwrap();

        let progress = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let progress_clone = std::sync::Arc::clone(&progress);
        let cancellation = SourceScanCancellation::active_for_test(move |_| {
            progress_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });

        let scan =
            list_source_files_sync_with_cancellation(root.clone(), 20, None, cancellation).unwrap();

        assert_eq!(scan.records.len(), 2);
        assert!(progress.load(std::sync::atomic::Ordering::Relaxed) > 0);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_write_file_persists_utf8_content_and_returns_preview() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        let file_path = root.join("src/App.ts");
        std::fs::write(&file_path, "export const oldValue = 1;\n").unwrap();

        let preview = write_source_file_sync(
            file_path.clone(),
            "export const newValue = 2;\n".to_string(),
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(&file_path).unwrap(),
            "export const newValue = 2;\n"
        );
        assert_eq!(preview.path, file_path.display().to_string());
        assert_eq!(preview.file_name, "App.ts");
        assert_eq!(preview.language, "typescript");
        assert_eq!(preview.line_count, 1);
        assert_eq!(preview.content, "export const newValue = 2;\n");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_file_action_builds_open_and_reveal_commands() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        let file_path = root.join("src/App.svelte");
        std::fs::write(&file_path, "<script></script>").unwrap();

        let open_command = source_file_action_command(&file_path, SourceFileAction::Open).unwrap();
        assert_eq!(open_command.program, "open");
        assert_eq!(open_command.args, vec![file_path.display().to_string()]);

        let reveal_command =
            source_file_action_command(&file_path, SourceFileAction::Reveal).unwrap();
        assert_eq!(reveal_command.program, "open");
        assert_eq!(
            reveal_command.args,
            vec!["-R".to_string(), file_path.display().to_string()]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_file_action_rejects_directories() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();

        let error = source_file_action_command(&root, SourceFileAction::Open).unwrap_err();
        assert!(error.contains("Source path is not a file"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn path_action_builds_open_and_reveal_commands_for_files_and_directories() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        let file_path = root.join("src/App.svelte");
        std::fs::write(&file_path, "<script></script>").unwrap();

        let open_dir_command = path_action_command(&root, PathAction::Open).unwrap();
        assert_eq!(open_dir_command.program, "open");
        assert_eq!(open_dir_command.args, vec![root.display().to_string()]);

        let reveal_dir_command = path_action_command(&root, PathAction::Reveal).unwrap();
        assert_eq!(reveal_dir_command.program, "open");
        assert_eq!(
            reveal_dir_command.args,
            vec!["-R".to_string(), root.display().to_string()]
        );

        let open_file_command = path_action_command(&file_path, PathAction::Open).unwrap();
        assert_eq!(open_file_command.program, "open");
        assert_eq!(
            open_file_command.args,
            vec![file_path.display().to_string()]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn path_action_rejects_missing_paths() {
        let root = unique_temp_root();
        let error = path_action_command(&root.join("missing"), PathAction::Open).unwrap_err();

        assert!(error.contains("Could not read path metadata"));
    }

    #[test]
    fn terminal_path_action_builds_allowlisted_terminal_commands_for_directories() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();

        let warp_command = terminal_path_action_command(&root, Some("Warp".to_string())).unwrap();
        assert_eq!(warp_command.program, "open");
        assert_eq!(
            warp_command.args,
            vec![format!(
                "warp://action/new_tab?path={}",
                uri_query_encode(&root.display().to_string())
            )]
        );

        let default_command = terminal_path_action_command(&root, None).unwrap();
        assert_eq!(
            default_command.args,
            vec![
                "-a".to_string(),
                "Terminal".to_string(),
                root.display().to_string()
            ]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn terminal_path_action_rejects_files_missing_paths_and_unknown_apps() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        let file_path = root.join("src/App.svelte");
        std::fs::write(&file_path, "<script></script>").unwrap();

        let file_error =
            terminal_path_action_command(&file_path, Some("Warp".to_string())).unwrap_err();
        assert!(file_error.contains("Terminal path is not a directory"));

        let missing_error =
            terminal_path_action_command(&root.join("missing"), Some("Warp".to_string()))
                .unwrap_err();
        assert!(missing_error.contains("Could not read terminal path metadata"));

        let app_error =
            terminal_path_action_command(&root, Some("UnknownTerminal".to_string())).unwrap_err();
        assert!(app_error.contains("Unsupported terminal app"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn terminal_command_action_builds_osascript_for_terminal_and_iterm() {
        let root = unique_temp_root();
        let quoted_root = shell_quote(&root.display().to_string());
        std::fs::create_dir_all(&root).unwrap();

        let terminal_command = terminal_command_action_command(
            &root,
            "codex resume session-123",
            Some("Terminal".to_string()),
        )
        .unwrap();
        assert_eq!(terminal_command.program, "osascript");
        assert_eq!(terminal_command.args[0], "-e");
        assert!(terminal_command.args[1].contains("tell application \"Terminal\""));
        assert!(terminal_command.args[1]
            .contains(&format!("cd {quoted_root} && codex resume session-123")));

        let iterm_command = terminal_command_action_command(
            &root,
            "claude --resume abc",
            Some("iTerm2".to_string()),
        )
        .unwrap();
        assert_eq!(iterm_command.program, "osascript");
        assert!(iterm_command.args[1].contains("tell application \"iTerm2\""));
        assert!(iterm_command.args[1].contains("write text"));
        assert!(iterm_command.args[1].contains(&format!("cd {quoted_root} && claude --resume abc")));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn terminal_command_action_rejects_unsafe_or_unsupported_commands() {
        let root = unique_temp_root();
        std::fs::create_dir_all(&root).unwrap();

        let empty_error =
            terminal_command_action_command(&root, "   ", Some("Terminal".to_string()))
                .unwrap_err();
        assert!(empty_error.contains("Terminal command is empty"));

        let multiline_error = terminal_command_action_command(
            &root,
            "codex resume one\nrm -rf nope",
            Some("Terminal".to_string()),
        )
        .unwrap_err();
        assert!(multiline_error.contains("single line"));

        let unsupported_error = terminal_command_action_command(
            &root,
            "codex resume session-123",
            Some("Warp".to_string()),
        )
        .unwrap_err();
        assert!(unsupported_error.contains("not supported for Warp"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn git_status_parser_reads_branch_counts_and_file_badges() {
        let status = parse_project_git_status(
            "## main...origin/main [ahead 1, behind 2]\n M src/App.ts\nA  src/Added.ts\n?? src/New.ts\n",
        )
        .unwrap();

        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.ahead, 1);
        assert_eq!(status.behind, 2);
        assert_eq!(
            status
                .files
                .iter()
                .map(|file| (
                    file.relative_path.as_str(),
                    file.index_status.as_str(),
                    file.worktree_status.as_str(),
                    file.badge.as_str(),
                    file.status.as_str()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("src/App.ts", "", "modified", "M", "modified"),
                ("src/Added.ts", "added", "", "A", "added"),
                ("src/New.ts", "untracked", "untracked", "?", "untracked")
            ]
        );
    }

    #[test]
    fn git_status_parser_handles_clean_branch_header() {
        let status = parse_project_git_status("## feature/source-browser\n").unwrap();

        assert_eq!(status.branch.as_deref(), Some("feature/source-browser"));
        assert_eq!(status.ahead, 0);
        assert_eq!(status.behind, 0);
        assert!(status.files.is_empty());
    }

    #[test]
    fn source_git_diff_reads_selected_file_worktree_diff() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        let file_path = root.join("src/App.ts");
        std::fs::write(&file_path, "export const value = 1;\n").unwrap();

        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "src/App.ts"]);
        run_git_for_test(
            &root,
            &[
                "-c",
                "user.name=MacCommandBar Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-m",
                "initial",
            ],
        );
        std::fs::write(&file_path, "export const value = 2;\n").unwrap();

        let diff = read_source_git_diff_sync(root.clone(), file_path).unwrap();

        assert_eq!(diff.relative_path, "src/App.ts");
        assert_eq!(diff.status, "modified");
        assert!(!diff.is_binary);
        assert!(diff.diff.contains("-export const value = 1;"));
        assert!(diff.diff.contains("+export const value = 2;"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn git_repository_summary_counts_dirty_files_and_task_branch() {
        let root = unique_temp_root();
        std::fs::create_dir_all(&root).unwrap();
        let readme_path = root.join("README.md");
        std::fs::write(&readme_path, "initial\n").unwrap();

        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["add", "README.md"]);
        run_git_for_test(
            &root,
            &[
                "-c",
                "user.name=MacCommandBar Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-m",
                "initial",
            ],
        );
        run_git_for_test(&root, &["checkout", "-b", "tsk-127-repo-dashboard"]);

        std::fs::write(root.join("staged.txt"), "staged\n").unwrap();
        run_git_for_test(&root, &["add", "staged.txt"]);
        std::fs::write(&readme_path, "changed\n").unwrap();
        std::fs::write(root.join("untracked.txt"), "untracked\n").unwrap();

        let project = RuntimeContextProject {
            id: "mac-command-bar".to_string(),
            name: "MacCommandBar".to_string(),
            path: root.display().to_string(),
        };
        let summaries = list_git_repository_summaries_sync(vec![project]).unwrap();
        let summary = summaries
            .iter()
            .find(|summary| summary.path == root.display().to_string())
            .unwrap();

        assert_eq!(summary.project_id, "mac-command-bar");
        assert_eq!(summary.project_name, "MacCommandBar");
        assert_eq!(summary.branch, "tsk-127-repo-dashboard");
        assert_eq!(summary.task_id.as_deref(), Some("TSK-127"));
        assert_eq!(summary.staged_count, 1);
        assert_eq!(summary.unstaged_count, 1);
        assert_eq!(summary.untracked_count, 1);
        assert_eq!(summary.dirty_count, 3);
        assert!(summary.is_dirty);
        assert_eq!(summary.last_commit_subject.as_deref(), Some("initial"));
        assert!(summary
            .last_commit_sha
            .as_ref()
            .is_some_and(|sha| !sha.is_empty()));
        assert!(summary.dirty_since_epoch_ms.is_some());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn git_repository_summary_infers_task_from_last_commit_subject() {
        let root = unique_temp_root();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("README.md"), "initial\n").unwrap();

        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["add", "README.md"]);
        run_git_for_test(
            &root,
            &[
                "-c",
                "user.name=MacCommandBar Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-m",
                "feat: add TSK-127 repo task links",
            ],
        );

        let project = RuntimeContextProject {
            id: "mac-command-bar".to_string(),
            name: "MacCommandBar".to_string(),
            path: root.display().to_string(),
        };
        let summary = git_repository_summary_for_path_result(&project, &root, false).unwrap();

        assert_eq!(summary.task_id.as_deref(), Some("TSK-127"));
        assert_eq!(
            summary.last_commit_subject.as_deref(),
            Some("feat: add TSK-127 repo task links")
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn git_commit_history_reads_graph_metadata_and_task_ids() {
        let root = unique_temp_root();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("README.md"), "initial\n").unwrap();

        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "README.md"]);
        run_git_for_test(&root, &["commit", "-m", "initial"]);

        std::fs::write(root.join("README.md"), "dashboard\n").unwrap();
        run_git_for_test(&root, &["commit", "-am", "[TSK-127] add dashboard"]);

        std::fs::write(root.join("README.md"), "remote\n").unwrap();
        run_git_for_test(
            &root,
            &["commit", "-am", "feat: add TSK-128 remote controls"],
        );

        run_git_for_test(&root, &["checkout", "-b", "tsk-129-history"]);
        std::fs::write(root.join("README.md"), "history\n").unwrap();
        run_git_for_test(&root, &["commit", "-am", "history panel"]);

        let history = read_git_commit_history_sync(root.clone(), Some(4)).unwrap();

        assert_eq!(history.len(), 4);
        assert_eq!(history[0].subject, "history panel");
        assert_eq!(history[0].task_id.as_deref(), Some("TSK-129"));
        assert!(history[0].refs.contains("HEAD -> tsk-129-history"));
        assert!(!history[0].short_sha.is_empty());
        assert!(!history[0].sha.is_empty());
        assert_eq!(history[0].author, "MacCommandBar Test");
        assert!(!history[0].committed_at.is_empty());
        assert_eq!(history[0].parent_count, 1);
        assert_eq!(history[0].parent_shas.len(), 1);
        assert_eq!(history[0].task_source.as_deref(), Some("refs"));
        assert_eq!(history[1].task_id.as_deref(), Some("TSK-128"));
        assert_eq!(history[1].task_source.as_deref(), Some("subject"));
        assert_eq!(history[2].task_id.as_deref(), Some("TSK-127"));
        assert_eq!(history[3].task_id, None);
        assert_eq!(history[3].parent_count, 0);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn git_commit_history_parser_tracks_merge_parent_metadata() {
        let output = "abc1234\x1fabc1234abc1234abc1234abc1234abc1234abc1234\x1fmerge TSK-127 pane work\x1fMacCommandBar Test\x1f2026-06-11T12:00:00Z\x1fHEAD -> main\x1fparent-one parent-two\n";

        let history = parse_git_commit_history(output).unwrap();

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].parent_count, 2);
        assert_eq!(
            history[0].parent_shas,
            vec!["parent-one".to_string(), "parent-two".to_string()]
        );
        assert_eq!(history[0].task_id.as_deref(), Some("TSK-127"));
        assert_eq!(history[0].task_source.as_deref(), Some("subject"));
    }

    #[test]
    fn git_actions_stage_unstage_and_commit_repo_paths() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        let file_path = root.join("src/App.ts");
        std::fs::write(&file_path, "export const value = 1;\n").unwrap();

        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "src/App.ts"]);
        run_git_for_test(
            &root,
            &[
                "-c",
                "user.name=MacCommandBar Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-m",
                "initial",
            ],
        );
        std::fs::write(&file_path, "export const value = 2;\n").unwrap();

        let staged = stage_git_paths_sync(root.clone(), vec!["src/App.ts".to_string()]).unwrap();
        assert_eq!(staged.message, "Staged 1 path");
        assert_eq!(staged.status.files[0].index_status, "modified");
        assert_eq!(staged.status.files[0].worktree_status, "");

        let unstaged =
            unstage_git_paths_sync(root.clone(), vec!["src/App.ts".to_string()]).unwrap();
        assert_eq!(unstaged.message, "Unstaged 1 path");
        assert_eq!(unstaged.status.files[0].index_status, "");
        assert_eq!(unstaged.status.files[0].worktree_status, "modified");

        stage_git_paths_sync(root.clone(), vec!["src/App.ts".to_string()]).unwrap();
        let committed =
            commit_git_repository_sync(root.clone(), "Update app value".to_string()).unwrap();
        assert_eq!(committed.message, "Committed staged changes");
        assert!(committed.status.files.is_empty());
        let latest_subject = run_git_text(&root, &["log", "-1", "--format=%s"]).unwrap();
        assert_eq!(latest_subject.trim(), "Update app value");

        let empty_message_error =
            commit_git_repository_sync(root.clone(), "   ".to_string()).unwrap_err();
        assert!(empty_message_error.contains("Commit message is required"));

        let no_staged_error =
            commit_git_repository_sync(root.clone(), "No staged changes".to_string()).unwrap_err();
        assert!(no_staged_error.contains("No staged changes to commit"));

        let outside_path_error =
            stage_git_paths_sync(root.clone(), vec!["../outside.txt".to_string()]).unwrap_err();
        assert!(outside_path_error.contains("relative repo paths"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn git_remote_actions_fetch_pull_and_push_current_upstream() {
        let remote = unique_temp_root();
        let local = unique_temp_root();
        let peer = unique_temp_root();

        std::fs::create_dir_all(&remote).unwrap();
        std::fs::create_dir_all(&peer).unwrap();
        run_git_for_test(&remote, &["init", "--bare"]);
        std::fs::create_dir_all(local.join("src")).unwrap();
        std::fs::write(local.join("src/App.ts"), "export const value = 1;\n").unwrap();
        run_git_for_test(&local, &["init"]);
        run_git_for_test(&local, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&local, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&local, &["add", "src/App.ts"]);
        run_git_for_test(&local, &["commit", "-m", "initial"]);
        run_git_for_test(&local, &["branch", "-M", "main"]);
        let remote_path = remote.display().to_string();
        run_git_for_test(&local, &["remote", "add", "origin", remote_path.as_str()]);
        run_git_for_test(&local, &["push", "-u", "origin", "main"]);

        let remote_path = remote.display().to_string();
        run_git_for_test(&peer, &["clone", remote_path.as_str(), "."]);
        run_git_for_test(&peer, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&peer, &["config", "user.email", "test@example.invalid"]);
        std::fs::write(peer.join("src/App.ts"), "export const value = 2;\n").unwrap();
        run_git_for_test(&peer, &["commit", "-am", "remote update"]);
        run_git_for_test(&peer, &["push"]);

        let fetched = fetch_git_repository_sync(local.clone()).unwrap();
        assert_eq!(fetched.message, "Fetched repository remotes");
        assert_eq!(fetched.status.behind, 1);

        let pulled = pull_git_repository_sync(local.clone()).unwrap();
        assert_eq!(pulled.message, "Pulled fast-forward updates");
        assert_eq!(pulled.status.behind, 0);
        assert_eq!(
            std::fs::read_to_string(local.join("src/App.ts")).unwrap(),
            "export const value = 2;\n"
        );

        std::fs::write(local.join("src/App.ts"), "export const value = 3;\n").unwrap();
        stage_git_paths_sync(local.clone(), vec!["src/App.ts".to_string()]).unwrap();
        commit_git_repository_sync(local.clone(), "local update".to_string()).unwrap();
        let ahead_status = project_git_status_sync(local.clone()).unwrap();
        assert_eq!(ahead_status.ahead, 1);

        let pushed = push_git_repository_sync(local.clone()).unwrap();
        assert_eq!(pushed.message, "Pushed current branch");
        assert_eq!(pushed.status.ahead, 0);
        let remote_subject = run_git_text(&remote, &["log", "-1", "--format=%s", "main"]).unwrap();
        assert_eq!(remote_subject.trim(), "local update");

        std::fs::remove_dir_all(remote).unwrap();
        std::fs::remove_dir_all(local).unwrap();
        std::fs::remove_dir_all(peer).unwrap();
    }

    #[test]
    fn lsof_listener_parser_reads_pid_command_and_ports() {
        let listeners = parse_lsof_tcp_listeners(
            "p123\ncnode\nn*:5177\nn127.0.0.1:24678\np456\ncdotnet\nn[::1]:5001\n",
        );

        assert_eq!(
            listeners,
            vec![
                ProcessListener {
                    pid: 123,
                    command: "node".to_string(),
                    port: 5177,
                },
                ProcessListener {
                    pid: 123,
                    command: "node".to_string(),
                    port: 24678,
                },
                ProcessListener {
                    pid: 456,
                    command: "dotnet".to_string(),
                    port: 5001,
                }
            ]
        );
    }

    #[test]
    fn runtime_context_project_matching_uses_main_and_worktree_roots() {
        let projects = vec![RuntimeContextProject {
            id: "ediplatform".to_string(),
            name: "EdiPlatform".to_string(),
            path: "/Users/blackcolours/dev/work/EdiPlatform".to_string(),
        }];

        let main_context = runtime_context_project_for_cwd(
            Path::new("/Users/blackcolours/dev/work/EdiPlatform/EdiPlatform.Api"),
            &projects,
        )
        .unwrap();
        assert_eq!(main_context.project_id.as_deref(), Some("ediplatform"));
        assert_eq!(main_context.project_name, "EdiPlatform");
        assert_eq!(main_context.root_label, "main checkout");

        let worktree_context = runtime_context_project_for_cwd(
            Path::new(
                "/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-126-m3-design-polish/ediplatform-web",
            ),
            &projects,
        )
        .unwrap();
        assert_eq!(worktree_context.project_id.as_deref(), Some("ediplatform"));
        assert_eq!(worktree_context.project_name, "EdiPlatform");
        assert_eq!(
            worktree_context.root_label,
            "worktree:tsk-126-m3-design-polish"
        );

        assert!(
            runtime_context_project_for_cwd(Path::new("/tmp/other-project"), &projects).is_none()
        );
    }

    #[test]
    fn project_worktree_parser_reads_porcelain_branches() {
        let records = parse_project_worktree_porcelain(
            "worktree /repo\nHEAD abc\nbranch refs/heads/main\n\nworktree /worktrees/feature\nHEAD def\nbranch refs/heads/cdx/tsk-126-feature\n\nworktree /detached\nHEAD fed\ndetached\n",
        );

        assert_eq!(
            records
                .iter()
                .map(|record| {
                    (
                        record.path.as_str(),
                        record.branch.as_str(),
                        record.task_id.as_deref(),
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("/repo", "main", None),
                ("/worktrees/feature", "cdx/tsk-126-feature", Some("TSK-126")),
                ("/detached", "detached", None)
            ]
        );
    }

    #[test]
    fn project_worktree_delete_eligibility_prioritizes_dirty_then_unmerged() {
        assert_eq!(
            project_worktree_delete_eligibility(true, true),
            "blocked: dirty worktree"
        );
        assert_eq!(
            project_worktree_delete_eligibility(false, true),
            "blocked: unmerged commits"
        );
        assert_eq!(
            project_worktree_delete_eligibility(false, false),
            "requires-confirmation"
        );
    }

    #[test]
    fn source_search_reads_indexed_files_case_insensitively_and_caps_results() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        let first_path = root.join("src/A.ts");
        let second_path = root.join("src/B.ts");
        std::fs::write(
            &first_path,
            "export function createWidget() {}\nconst label = \"Widget\";\n",
        )
        .unwrap();
        std::fs::write(&second_path, "export const WidgetName = \"B\";\n").unwrap();

        let matches = search_source_files_sync(
            vec![
                SourceRecord {
                    path: first_path.display().to_string(),
                    relative_path: "src/A.ts".to_string(),
                    file_name: "A.ts".to_string(),
                    language: "typescript".to_string(),
                    byte_count: 64,
                },
                SourceRecord {
                    path: second_path.display().to_string(),
                    relative_path: "src/B.ts".to_string(),
                    file_name: "B.ts".to_string(),
                    language: "typescript".to_string(),
                    byte_count: 32,
                },
            ],
            "widget".to_string(),
            Some(2),
        )
        .unwrap();

        assert_eq!(
            matches
                .iter()
                .map(|source_match| (
                    source_match.relative_path.as_str(),
                    source_match.line,
                    source_match.column,
                    source_match.excerpt.as_str()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("src/A.ts", 1, 23, "export function createWidget() {}"),
                ("src/A.ts", 2, 16, "const label = \"Widget\";")
            ]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_definition_lookup_reads_indexed_symbols_and_caps_results() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        let resolver_path = root.join("src/FormatResolver.cs");
        let detector_path = root.join("src/FormatDetector.cs");
        std::fs::write(
            &resolver_path,
            [
                "namespace Demo;",
                "public sealed class FormatResolver",
                "{",
                "    private readonly FormatDetector _detector;",
                "}",
            ]
            .join("\n"),
        )
        .unwrap();
        std::fs::write(
            &detector_path,
            [
                "namespace Demo;",
                "public sealed class FormatDetector",
                "{",
                "    public Task DetectAsync() => Task.CompletedTask;",
                "}",
            ]
            .join("\n"),
        )
        .unwrap();

        let targets = find_source_definitions_sync(
            vec![
                SourceRecord {
                    path: resolver_path.display().to_string(),
                    relative_path: "src/FormatResolver.cs".to_string(),
                    file_name: "FormatResolver.cs".to_string(),
                    language: "csharp".to_string(),
                    byte_count: 96,
                },
                SourceRecord {
                    path: detector_path.display().to_string(),
                    relative_path: "src/FormatDetector.cs".to_string(),
                    file_name: "FormatDetector.cs".to_string(),
                    language: "csharp".to_string(),
                    byte_count: 96,
                },
            ],
            "formatdetector".to_string(),
            Some(1),
        )
        .unwrap();

        assert_eq!(
            targets
                .iter()
                .map(|target| (
                    target.relative_path.as_str(),
                    target.symbol_name.as_str(),
                    target.kind.as_str(),
                    target.line,
                    target.column,
                    target.detail.as_str()
                ))
                .collect::<Vec<_>>(),
            vec![(
                "src/FormatDetector.cs",
                "FormatDetector",
                "class",
                2,
                21,
                "public sealed class FormatDetector"
            )]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_reference_lookup_reads_token_bounded_matches_and_caps_results() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        let resolver_path = root.join("src/FormatResolver.cs");
        let detector_path = root.join("src/FormatDetector.cs");
        std::fs::write(
            &resolver_path,
            [
                "public sealed class FormatResolver",
                "{",
                "    private readonly FormatDetector _detector;",
                "    private readonly FormatDetectorFactory _factory;",
                "}",
            ]
            .join("\n"),
        )
        .unwrap();
        std::fs::write(
            &detector_path,
            ["public sealed class FormatDetector", "{"].join("\n"),
        )
        .unwrap();

        let targets = find_source_references_sync(
            vec![
                SourceRecord {
                    path: resolver_path.display().to_string(),
                    relative_path: "src/FormatResolver.cs".to_string(),
                    file_name: "FormatResolver.cs".to_string(),
                    language: "csharp".to_string(),
                    byte_count: 128,
                },
                SourceRecord {
                    path: detector_path.display().to_string(),
                    relative_path: "src/FormatDetector.cs".to_string(),
                    file_name: "FormatDetector.cs".to_string(),
                    language: "csharp".to_string(),
                    byte_count: 64,
                },
            ],
            "formatdetector".to_string(),
            Some(2),
        )
        .unwrap();

        assert_eq!(
            targets
                .iter()
                .map(|target| (
                    target.relative_path.as_str(),
                    target.symbol_name.as_str(),
                    target.line,
                    target.column,
                    target.excerpt.as_str()
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "src/FormatResolver.cs",
                    "FormatDetector",
                    3,
                    22,
                    "private readonly FormatDetector _detector;"
                ),
                (
                    "src/FormatDetector.cs",
                    "FormatDetector",
                    1,
                    21,
                    "public sealed class FormatDetector"
                )
            ]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    fn unique_temp_root() -> PathBuf {
        static TEMP_ROOT_COUNTER: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = TEMP_ROOT_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "mac-command-bar-tauri-source-test-{}-{unique}-{counter}",
            std::process::id()
        ))
    }

    fn run_git_for_test(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(["-C", root.to_str().unwrap()])
            .args(args)
            .output()
            .unwrap_or_else(|error| panic!("could not run git {args:?}: {error}"));
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
