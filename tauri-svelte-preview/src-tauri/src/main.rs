use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// The counting pass behind the margin reference counts lives in mcb-core, not
// here, so that it is compiled optimized even when this crate is not — see the
// module's own note and the `[profile.dev.package."*"]` block in Cargo.toml.
use mcb_core::reference_counts::{
    count_reference_lines_across_files, is_source_token_boundary,
    normalized_reference_count_symbols, ReferenceCountFile, ReferenceCountPlan,
    MAX_REFERENCE_SCAN_BYTES,
};
use mcb_core::scanners::sessions::{scan_sessions, AgentSessionRecord};
use orchestration::{
    list_orchestration_runs_sync, record_orchestration_event_sync, OrchestrationEvent,
    OrchestrationRun,
};
use tauri::{Emitter, Manager};
use workflow::{WorkflowDefinitionV1, WorkflowEngine, WorkflowRunRecord};

mod agent_conversation;
mod browser;
mod git_diff_models;
mod git_pr;
mod lsp;
mod orchestration;
mod resources;
mod terminal;
mod usage_current;
mod usage_db;
mod usage_history;
mod usage_indexer;
mod usage_sources;
mod workflow;

const MAX_PREVIEW_BYTES: u64 = 512 * 1024;
const DEFAULT_SOURCE_LIST_LIMIT: usize = 10_000;
const MAX_SOURCE_LIST_LIMIT: usize = 25_000;
const DEFAULT_SOURCE_SEARCH_LIMIT: usize = 50;
const MAX_SOURCE_SEARCH_LIMIT: usize = 200;
const DEFAULT_SOURCE_DEFINITION_LIMIT: usize = 20;
const MAX_SOURCE_DEFINITION_LIMIT: usize = 100;
const DEFAULT_SOURCE_REFERENCE_LIMIT: usize = 50;
const MAX_SOURCE_REFERENCE_LIMIT: usize = 200;
/// How long one batched margin-count request may spend before it answers with
/// whatever it has counted so far. This is a ceiling, not a wait: a pass that
/// finishes early answers early. A four-thousand-file C# and TypeScript
/// project measures around 400ms in a release build, so this leaves room for a
/// cold disk before giving up on the rest.
const DEFAULT_REFERENCE_COUNT_DEADLINE_MS: u64 = 1_500;
const MAX_REFERENCE_COUNT_DEADLINE_MS: u64 = 10_000;
const MAX_SOURCE_SCAN_SKIPPED_DIRECTORY_SAMPLES: usize = 16;
const DEFAULT_GIT_HISTORY_LIMIT: usize = 24;
/// Most commits one history request will read. The commits list pages: it opens
/// with a couple of dozen and asks for another hundred each time the reader
/// wants more, so this is the ceiling on the accumulated ask rather than a page
/// size. Five hundred entries of subject-and-author is a small read for git and
/// still a list a person can scroll.
const MAX_GIT_HISTORY_LIMIT: usize = 500;
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
    stats: SourceScanStats,
}

/// How many lines mention each requested symbol, across the whole project.
///
/// `approximate` is true when the count stopped early — either the deadline ran
/// out or the project has more files than one walk collects. An approximate
/// result is still worth drawing for symbols that were found, but a zero in an
/// approximate result means "not seen yet", not "nowhere in the project".
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceReferenceCountResult {
    counts: HashMap<String, u32>,
    approximate: bool,
    scanned_files: usize,
    elapsed_ms: u64,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceScanStats {
    visited_entries: usize,
    matched_files: usize,
    skipped_directories: usize,
    unsupported_files: usize,
    unreadable_entries: usize,
    requested_limit: usize,
    returned_files: usize,
    collection_limit: usize,
    collection_limit_reached: bool,
    skipped_directory_samples: Vec<SourceSkippedDirectory>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceSkippedDirectory {
    path: String,
    name: String,
    reason: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectRootValidationResult {
    path: String,
    exists: bool,
    is_directory: bool,
    is_git_repository: bool,
    git_root: Option<String>,
    message: String,
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
    has_upstream: bool,
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
    original_content: Option<String>,
    modified_content: Option<String>,
}

/// One file touched by one commit. Same three fields the working-copy status list
/// shows for a file, so a commit's file list and the changed-files list render the
/// same way.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct GitCommitFileChange {
    relative_path: String,
    status: String,
    badge: String,
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
    is_prunable: bool,
    prunable_reason: Option<String>,
    is_locked: bool,
    locked_reason: Option<String>,
    last_activity: Option<String>,
    delete_eligibility: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectWorktreeActionResult {
    message: String,
    worktrees: Vec<ProjectWorktree>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectWorktreeArchiveResult {
    message: String,
    archive_path: String,
    worktrees: Vec<ProjectWorktree>,
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

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaywrightProcessInfo {
    pid: u32,
    pgid: u32,
    command: String,
    name: String,
    label: String,
    elapsed: String,
    args: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaywrightSessionInfo {
    pgid: u32,
    label: String,
    pids: Vec<u32>,
    processes: Vec<PlaywrightProcessInfo>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaywrightCleanupResult {
    sessions: Vec<PlaywrightSessionInfo>,
    terminated_pgids: Vec<u32>,
    terminated_pids: Vec<u32>,
    failed_pgids: Vec<PlaywrightCleanupFailure>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaywrightCleanupFailure {
    pgid: u32,
    pid: Option<u32>,
    message: String,
}

/// What happened when the app was asked to stop one process.
///
/// `message` is a whole sentence for the reader, whichever way it went — asking
/// a process to stop can fail for reasons the reader can act on (it belongs to
/// another user, it already exited) and "failed" on its own tells them nothing.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProcessKillResult {
    ok: bool,
    message: String,
}

/// What happened when the C# language server was switched off or on.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CsharpLanguageServerToggleResult {
    enabled: bool,
    stopped_servers: usize,
    message: String,
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
    has_upstream: bool,
    last_commit_sha: Option<String>,
    last_commit_subject: Option<String>,
    last_commit_at: Option<String>,
    dirty_since_epoch_ms: Option<u64>,
    dirty_status_fingerprint: String,
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
    progress: Option<Arc<dyn Fn(SourceScanStats) + Send + Sync>>,
    /// When set, the walk stops itself at this moment even if nobody cancelled
    /// it. Only the batched margin-count walk uses this; the file-list scan the
    /// user watches runs to completion.
    deadline: Option<Instant>,
}

impl SourceScanCancellation {
    fn new(
        cancelled: Arc<AtomicBool>,
        progress: Option<Arc<dyn Fn(SourceScanStats) + Send + Sync>>,
    ) -> Self {
        Self {
            cancelled,
            progress,
            deadline: None,
        }
    }

    fn none() -> Self {
        Self::new(Arc::new(AtomicBool::new(false)), None)
    }

    fn until(deadline: Instant) -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            progress: None,
            deadline: Some(deadline),
        }
    }

    #[cfg(test)]
    fn cancelled_for_test() -> Self {
        Self::new(Arc::new(AtomicBool::new(true)), None)
    }

    #[cfg(test)]
    fn active_for_test(progress: impl Fn(SourceScanStats) + Send + Sync + 'static) -> Self {
        Self::new(Arc::new(AtomicBool::new(false)), Some(Arc::new(progress)))
    }

    fn ensure_active(&self) -> Result<(), String> {
        if self.cancelled.load(Ordering::Relaxed) {
            return Err("Source scan cancelled".to_string());
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err("Source scan ran out of time".to_string());
        }
        Ok(())
    }

    fn report_progress(&self, progress: SourceScanStats) {
        if let Some(callback) = &self.progress {
            callback(progress);
        }
    }
}

#[derive(Default)]
struct SourceScanWalkProgress {
    stats: SourceScanStats,
    next_report_at: usize,
}

impl SourceScanWalkProgress {
    fn visit_entry(&mut self, cancellation: &SourceScanCancellation) {
        self.stats.visited_entries += 1;
        if self.stats.visited_entries == 1 || self.stats.visited_entries >= self.next_report_at {
            self.next_report_at = self.stats.visited_entries + SOURCE_SCAN_PROGRESS_INTERVAL;
            self.report(cancellation);
        }
    }

    fn match_file(&mut self, cancellation: &SourceScanCancellation) {
        self.stats.matched_files += 1;
        self.report(cancellation);
    }

    fn skip_directory(&mut self, root: &Path, path: &Path, name: &str, reason: &str) {
        self.stats.skipped_directories += 1;
        if self.stats.skipped_directory_samples.len() < MAX_SOURCE_SCAN_SKIPPED_DIRECTORY_SAMPLES {
            self.stats
                .skipped_directory_samples
                .push(SourceSkippedDirectory {
                    path: normalized_relative_source_path(root, path),
                    name: name.to_string(),
                    reason: reason.to_string(),
                });
        }
    }

    fn skip_unsupported_file(&mut self) {
        self.stats.unsupported_files += 1;
    }

    fn skip_unreadable_entry(&mut self) {
        self.stats.unreadable_entries += 1;
    }

    fn report(&self, cancellation: &SourceScanCancellation) {
        cancellation.report_progress(self.stats.clone());
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

#[tauri::command]
async fn validate_project_root(path: String) -> Result<ProjectRootValidationResult, String> {
    tauri::async_runtime::spawn_blocking(move || validate_project_root_sync(PathBuf::from(path)))
        .await
        .map_err(|error| format!("Project root validation task failed: {error}"))
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
    let progress = Arc::new(move |snapshot: SourceScanStats| {
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

/// Read one UTF-8 source file for native C# Peek, confined to the canonical
/// workspace root. This command is intentionally read-only.
#[tauri::command]
async fn read_native_csharp_file(root: String, path: String) -> Result<SourcePreview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = std::fs::canonicalize(root)
            .map_err(|error| format!("Could not resolve C# workspace root: {error}"))?;
        let path = std::fs::canonicalize(path)
            .map_err(|error| format!("Could not resolve C# source path: {error}"))?;
        if !path.starts_with(&root) {
            return Err("C# source path is outside the workspace root".to_string());
        }
        read_source_file_sync(path)
    })
    .await
    .map_err(|error| format!("Native C# source read task failed: {error}"))?
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

/// Count, in one pass over the project, how many lines mention each of the
/// given symbols.
///
/// This exists because the margin counts above every symbol in a file used to
/// ask one question per symbol, each of which re-read the whole project and
/// carried the entire file list across the bridge. One hundred and twenty of
/// those at once is what froze the editor. Here the file list never leaves the
/// backend and the project is read once for all of the symbols together.
#[tauri::command]
async fn count_source_references(
    root: String,
    symbol_names: Vec<String>,
    deadline_ms: Option<u64>,
) -> Result<SourceReferenceCountResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        count_source_references_sync(PathBuf::from(root), symbol_names, deadline_ms)
    })
    .await
    .map_err(|error| format!("Source reference count task failed: {error}"))?
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
async fn list_source_lsp_statuses(root: String) -> Result<Vec<lsp::SourceLspStatus>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        lsp::list_source_lsp_statuses_sync(PathBuf::from(root))
    })
    .await
    .map_err(|error| format!("Source LSP readiness task failed: {error}"))
}

/// Proactively re-point any already-running language server(s) at a freshly-selected
/// project root so the cold re-index happens in the background on switch, not on the first
/// file-open under the new project. No-op when no server is running for that root's
/// languages (see `SourceLspRegistry::warm_running_servers_for_root`).
#[tauri::command]
async fn warm_source_lsp_for_root(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    root: String,
) -> Result<usize, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.warm_running_servers_for_root(&root))
        .await
        .map_err(|error| format!("Source LSP warm task failed: {error}"))?
}

/// Ensure the one native C# language-client endpoint for this canonical root.
/// Both workspace warming and editor startup call this command; the registry
/// coalesces them into the same bounded slot.
#[tauri::command]
async fn ensure_native_csharp_language_client(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    root: String,
) -> Result<Option<lsp::NativeCsharpEndpoint>, String> {
    if !lsp::workspace_has_csharp_project_marker(&root) {
        return Ok(None);
    }
    registry.ensure_native_csharp_endpoint(&root).map(Some)
}

#[tauri::command]
async fn mark_native_csharp_language_client_ready(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    root: String,
) -> Result<(), String> {
    registry.mark_native_csharp_client_ready(&root)
}

/// Turn the C# language server off or on, and stop it now if it is running.
///
/// The reader's setting drives this. Call it with what the setting says when
/// the app starts as well as when the switch is flipped: the flag lives in this
/// process and starts out on, so a reader who turned it off last week would
/// otherwise get the server back on the next launch.
#[tauri::command]
async fn set_csharp_language_server_enabled(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    enabled: bool,
) -> Result<CsharpLanguageServerToggleResult, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let changed = lsp::set_csharp_language_server_enabled(enabled);
        let stopped_servers = if enabled {
            0
        } else {
            registry.stop_servers_for_language("csharp")?
        };

        Ok(CsharpLanguageServerToggleResult {
            enabled,
            stopped_servers,
            message: describe_csharp_language_server_toggle(enabled, changed, stopped_servers),
        })
    })
    .await
    .map_err(|error| format!("C# language server switch task failed: {error}"))?
}

fn describe_csharp_language_server_toggle(
    enabled: bool,
    changed: bool,
    stopped_servers: usize,
) -> String {
    if enabled {
        return if changed {
            "The C# language server is back on. It starts the next time you open a C# file, and takes a minute to read the solution."
                .to_string()
        } else {
            "The C# language server was already on.".to_string()
        };
    }

    if stopped_servers > 0 {
        "The C# language server is off and the running one has been stopped, freeing its memory. Reference counts and project search still work; mistake squiggles and precise go-to-definition do not."
            .to_string()
    } else {
        "The C# language server is off. It was not running, so nothing had to be stopped. Reference counts and project search still work; mistake squiggles and precise go-to-definition do not."
            .to_string()
    }
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
async fn find_source_lsp_workspace_symbols(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    preview: lsp::SourceLspPreview,
    request: lsp::SourceLspWorkspaceSymbolRequest,
) -> Result<Vec<lsp::SourceLspWorkspaceSymbol>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.find_workspace_symbols(preview, request))
        .await
        .map_err(|error| format!("Source LSP workspace-symbol task failed: {error}"))?
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

/// Every symbol in one file, as the editor's margin counts want them: flattened, with
/// both numbers counted from zero.
///
/// This is what switches the margin counts on for Rust and Svelte, where the app's own
/// pattern-based symbol reader has never worked. Unlike the other language-server
/// lookups it is handed a path rather than the editor's copy of the text, and reads the
/// file from disk itself.
#[tauri::command]
async fn find_source_lsp_document_symbols(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    root: String,
    language: String,
    path: String,
) -> Result<Vec<lsp::SourceLspDocumentSymbol>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        registry.find_document_symbols(root, language, path)
    })
    .await
    .map_err(|error| format!("Source LSP document symbol task failed: {error}"))?
}

/// The last few hundred lines a running language server printed to its own error output.
///
/// When a server refuses to answer, this is usually the only explanation there is. Empty
/// when no server for that language is running.
#[tauri::command]
async fn read_source_lsp_log(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    root: String,
    language: String,
) -> Result<Vec<String>, String> {
    // The project folder is accepted but not used: one server per language serves every
    // project, so there is only ever one log to hand back.
    let _ = root;
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || registry.read_server_log(&language))
        .await
        .map_err(|error| format!("Source LSP log task failed: {error}"))?
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
async fn list_source_lsp_diagnostics_for_root(
    registry: tauri::State<'_, lsp::SourceLspRegistry>,
    root: String,
) -> Result<Vec<lsp::SourceLspDiagnostic>, String> {
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        registry.list_diagnostics_for_root(PathBuf::from(root))
    })
    .await
    .map_err(|error| format!("Source LSP project diagnostics task failed: {error}"))?
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

/// What this build of the backend can do, by name.
///
/// The frontend needs this because some additions are new ARGUMENTS on commands that
/// already existed. Tauri quietly drops a payload key a command does not declare, so an
/// older desktop build handed `force: true` runs the ordinary safe removal and reports
/// success — there is no unknown-command error to catch and nothing else to test. Asking
/// for this list first is the only way to know before offering the button.
///
/// Anything added here is a promise: check the name before offering the feature, and treat
/// this command being missing as "none of these are available".
const BACKEND_CAPABILITIES: [&str; 26] = [
    // `remove_project_worktree` accepts `force`.
    "worktreeForceRemove",
    // `kill_playwright_session` stops one process group.
    "playwrightSessionKill",
    // `list_source_lsp_diagnostics_for_root` reads a whole project's diagnostics.
    "lspDiagnosticsForRoot",
    // `start_terminal_session` accepts `command` and exits with its code.
    "terminalCommandSpawn",
    // `count_source_references` counts every requested symbol in one project pass.
    "referenceCounts",
    // `kill_process` stops one process the app is showing, by number.
    "processKill",
    // `remove_project_worktree` on a worktree whose folder is gone clears only that
    // one entry's records, instead of every entry whose folder is gone.
    "worktreePruneSingle",
    // `set_csharp_language_server_enabled` turns the C# language server off and on.
    "csharpLanguageServerToggle",
    // `find_source_lsp_document_symbols` lists a file's symbols using the language
    // server, which is what gives Rust and Svelte margin counts.
    "lspDocumentSymbols",
    // The app sends a `source-lsp-status-changed` event whenever a language server
    // changes what it is doing, so nothing has to ask on a timer.
    "lspStatusEvents",
    // `read_source_lsp_log` hands back what a running language server printed to its
    // own error output.
    "lspLog",
    // A10 resource and space inventory/action commands.
    "resourceSnapshot",
    "resourceDiskScan",
    "resourceDiskCleanup",
    "resourceStopOwned",
    // Provider-authored quota and SQLite usage history queries.
    "providerUsageQuota",
    "usageHistory",
    "usageHistoryIncremental",
    "usageProviderSummary",
    "usageDailyTotals",
    // Source-control agent actions and the GitHub CLI PR lifecycle.
    "generate_commit_message",
    "read_pull_request_context",
    "generate_pull_request_details",
    "create_pull_request",
    "read_pull_request_status",
    // The conversation manager pushes ACP turn and item events while a turn is live.
    "acpLiveConversationEvents",
];

/// The event the app sends whenever a language server changes what it is doing.
const SOURCE_LSP_STATUS_CHANGED_EVENT: &str = "source-lsp-status-changed";

fn backend_capabilities() -> Vec<String> {
    BACKEND_CAPABILITIES
        .iter()
        .map(|capability| capability.to_string())
        .collect()
}

#[tauri::command]
async fn read_backend_capabilities() -> Result<Vec<String>, String> {
    Ok(backend_capabilities())
}

#[tauri::command]
async fn read_git_commit_files(
    root: String,
    sha: String,
) -> Result<Vec<GitCommitFileChange>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        read_git_commit_files_sync(PathBuf::from(root), sha)
    })
    .await
    .map_err(|error| format!("Git commit file list task failed: {error}"))?
}

#[tauri::command]
async fn read_git_commit_file_diff(
    root: String,
    sha: String,
    relative_path: String,
) -> Result<SourceGitDiff, String> {
    tauri::async_runtime::spawn_blocking(move || {
        read_git_commit_file_diff_sync(PathBuf::from(root), sha, relative_path)
    })
    .await
    .map_err(|error| format!("Git commit file diff task failed: {error}"))?
}

#[tauri::command]
async fn list_project_worktrees(root: String) -> Result<Vec<ProjectWorktree>, String> {
    tauri::async_runtime::spawn_blocking(move || list_project_worktrees_sync(PathBuf::from(root)))
        .await
        .map_err(|error| format!("Worktree scan task failed: {error}"))?
}

#[tauri::command]
async fn remove_project_worktree(
    root: String,
    path: String,
    force: Option<bool>,
) -> Result<ProjectWorktreeActionResult, String> {
    // Leaving `force` out keeps the safe remove that refuses to lose work.
    let force = force.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || {
        remove_project_worktree_sync(PathBuf::from(root), PathBuf::from(path), force)
    })
    .await
    .map_err(|error| format!("Worktree remove task failed: {error}"))?
}

#[tauri::command]
async fn archive_project_worktree(
    root: String,
    path: String,
) -> Result<ProjectWorktreeArchiveResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        archive_project_worktree_sync(PathBuf::from(root), PathBuf::from(path))
    })
    .await
    .map_err(|error| format!("Worktree archive task failed: {error}"))?
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
async fn list_playwright_sessions() -> Result<Vec<PlaywrightSessionInfo>, String> {
    tauri::async_runtime::spawn_blocking(list_playwright_sessions_sync)
        .await
        .map_err(|error| format!("Playwright session scan task failed: {error}"))?
}

#[tauri::command]
async fn kill_playwright_session(pgid: i32) -> Result<PlaywrightCleanupResult, String> {
    tauri::async_runtime::spawn_blocking(move || kill_playwright_session_sync(pgid))
        .await
        .map_err(|error| format!("Playwright session cleanup task failed: {error}"))?
}

#[tauri::command]
async fn kill_playwright_sessions() -> Result<PlaywrightCleanupResult, String> {
    tauri::async_runtime::spawn_blocking(kill_playwright_sessions_sync)
        .await
        .map_err(|error| format!("Playwright cleanup task failed: {error}"))?
}

/// Ask one process to stop — the button next to a running process in the
/// context panel.
///
/// This sends the polite stop signal only. A process that ignores it keeps
/// running and the reader is told so, rather than the app escalating to a kill
/// nobody asked for: these are the reader's own dev servers and test runners,
/// and losing unsaved work in one because a click was read as "destroy" is not
/// a trade this makes on their behalf.
#[tauri::command]
async fn kill_process(
    pid: u32,
    expected_command: Option<String>,
) -> Result<ProcessKillResult, String> {
    tauri::async_runtime::spawn_blocking(move || kill_process_sync(pid, expected_command))
        .await
        .map_err(|error| format!("Stop process task failed: {error}"))
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
fn list_workflow_runs(
    engine: tauri::State<'_, WorkflowEngine>,
) -> Result<Vec<WorkflowRunRecord>, String> {
    engine.list_runs().map_err(|error| error.to_string())
}

#[tauri::command]
fn create_workflow_run(
    engine: tauri::State<'_, WorkflowEngine>,
    definition: WorkflowDefinitionV1,
    input: serde_json::Value,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    engine
        .create_run(definition, input, idempotency_key)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn start_workflow_run(
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    engine
        .start(&run_id, &idempotency_key)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn pause_workflow_run(
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    engine
        .pause(&run_id, &idempotency_key)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn resume_workflow_run(
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    engine
        .resume(&run_id, &idempotency_key)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn cancel_workflow_run(
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    engine
        .cancel(&run_id, &idempotency_key)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn retry_workflow_node(
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    node_id: String,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    engine
        .retry_node(&run_id, &node_id, &idempotency_key)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn skip_workflow_node(
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    node_id: String,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    engine
        .skip_node(&run_id, &node_id, &idempotency_key)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn approve_workflow_gate(
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    node_id: String,
    approval: serde_json::Value,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    engine
        .approve_gate(&run_id, &node_id, approval, &idempotency_key)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn submit_workflow_result(
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    node_id: String,
    result: serde_json::Value,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    engine
        .submit_result(&run_id, &node_id, result, &idempotency_key)
        .await
        .map_err(|error| error.to_string())
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
    let collect_limit = source_collection_limit(limit);
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
    let collected_file_count = records.len();
    records.sort_by(compare_source_records);
    let truncated = collected_file_count > limit;
    records.truncate(limit);
    let mut stats = progress.stats;
    stats.requested_limit = limit;
    stats.returned_files = records.len();
    stats.collection_limit = collect_limit;
    stats.collection_limit_reached = collected_file_count >= collect_limit;
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
        stats,
    })
}

fn validate_project_root_sync(path: PathBuf) -> ProjectRootValidationResult {
    let path_label = path.display().to_string();
    let metadata = std::fs::metadata(&path);
    let Ok(metadata) = metadata else {
        return ProjectRootValidationResult {
            path: path_label,
            exists: false,
            is_directory: false,
            is_git_repository: false,
            git_root: None,
            message: "Project path not found".to_string(),
        };
    };

    if !metadata.is_dir() {
        return ProjectRootValidationResult {
            path: path_label,
            exists: true,
            is_directory: false,
            is_git_repository: false,
            git_root: None,
            message: "Project path points to a file. Choose the repository folder instead."
                .to_string(),
        };
    }

    let git_root_path = git_repository_root(&path);
    let is_git_repository = git_root_path
        .as_ref()
        .is_some_and(|git_root| paths_refer_to_same_location(git_root, &path));
    let git_root = git_root_path
        .as_ref()
        .map(|root| normalized_path_string(root));
    ProjectRootValidationResult {
        path: path_label,
        exists: true,
        is_directory: true,
        is_git_repository,
        git_root: git_root.clone(),
        message: if is_git_repository {
            "Project root ready".to_string()
        } else if let Some(git_root) = git_root {
            format!("Folder is inside a Git repository. Add {git_root} for full project context.")
        } else {
            "Folder is not a Git repository. Source browsing will work, but Git/worktree panels may be unavailable."
                .to_string()
        },
    }
}

fn git_repository_root(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate.join(".git").exists() {
            return Some(candidate.to_path_buf());
        }
        current = candidate.parent();
    }

    let output = Command::new("git")
        .args([
            "-C",
            path.to_str().unwrap_or_default(),
            "rev-parse",
            "--show-toplevel",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if root.is_empty() {
        None
    } else {
        Some(PathBuf::from(root))
    }
}

fn paths_refer_to_same_location(left: &Path, right: &Path) -> bool {
    let normalized_left = std::fs::canonicalize(left)
        .map(|path| normalized_path_string(&path))
        .unwrap_or_else(|_| normalized_path_string(left));
    let normalized_right = std::fs::canonicalize(right)
        .map(|path| normalized_path_string(&path))
        .unwrap_or_else(|_| normalized_path_string(right));
    normalized_left == normalized_right
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
            progress.skip_unreadable_entry();
            continue;
        };

        if metadata.is_dir() {
            if let Some(reason) = skip_dir_reason(&file_name) {
                progress.skip_directory(root, &path, &file_name, reason);
                continue;
            }
            collect_source_files(root, &path, limit, query, records, cancellation, progress)?;
            continue;
        }

        if !metadata.is_file() {
            progress.skip_unsupported_file();
            continue;
        }

        if !is_source_file(&path) {
            progress.skip_unsupported_file();
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

fn count_source_references_sync(
    root: PathBuf,
    symbol_names: Vec<String>,
    deadline_ms: Option<u64>,
) -> Result<SourceReferenceCountResult, String> {
    let started = Instant::now();
    let metadata = std::fs::metadata(&root)
        .map_err(|error| format!("Could not read source root metadata: {error}"))?;
    if !metadata.is_dir() {
        return Err("Source root is not a directory".to_string());
    }

    let names = normalized_reference_count_symbols(symbol_names);
    if names.is_empty() {
        return Ok(SourceReferenceCountResult {
            counts: HashMap::new(),
            approximate: false,
            scanned_files: 0,
            elapsed_ms: elapsed_millis(started),
        });
    }

    let budget = Duration::from_millis(
        deadline_ms
            .unwrap_or(DEFAULT_REFERENCE_COUNT_DEADLINE_MS)
            .clamp(1, MAX_REFERENCE_COUNT_DEADLINE_MS),
    );
    let deadline = started + budget;

    // Walking the project here rather than accepting a file list from the
    // caller is the point: the list is thousands of entries and used to be
    // serialized across the bridge once per symbol.
    let collection_limit = source_collection_limit(DEFAULT_SOURCE_LIST_LIMIT);
    let mut records = Vec::new();
    let mut walk_progress = SourceScanWalkProgress::default();
    let walk = collect_source_files(
        &root,
        &root,
        collection_limit,
        None,
        &mut records,
        &SourceScanCancellation::until(deadline),
        &mut walk_progress,
    );
    let mut approximate = walk.is_err() || records.len() >= collection_limit;

    // The counting pass takes only a path and a size per file; the rest of a
    // record (language, display names) means nothing to it.
    let files = records
        .iter()
        .map(|record| ReferenceCountFile {
            path: Path::new(&record.path),
            byte_count: record.byte_count,
        })
        .collect::<Vec<_>>();

    let plan = ReferenceCountPlan::new(&names);
    // Not `MAX_PREVIEW_BYTES`: that ceiling decides how much text is worth putting on
    // screen, which has nothing to do with how much text this pass can walk through.
    // Reusing it meant one 660KB source file made every count in the project a floor.
    let pass =
        count_reference_lines_across_files(&files, &plan, deadline, MAX_REFERENCE_SCAN_BYTES);
    log_reference_count_timing(&pass, names.len(), started);
    // Both mean the same thing to the reader: not every file was counted, so
    // the totals are floors and the margin must say "at least", never an exact
    // number nobody actually took.
    approximate = approximate || pass.ran_out_of_time || pass.skipped_files;

    let counts = names
        .iter()
        .cloned()
        .zip(pass.counts)
        .collect::<HashMap<String, u32>>();

    Ok(SourceReferenceCountResult {
        counts,
        approximate,
        scanned_files: pass.scanned_files,
        elapsed_ms: elapsed_millis(started),
    })
}

fn elapsed_millis(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Say, on the terminal the app was started from, how long one pass of counting
/// mentions across the project took — but only when the reader asked for timings
/// by starting the app with `MCB_TIMING=1`. Silence otherwise.
fn log_reference_count_timing(
    pass: &mcb_core::reference_counts::ReferenceCountPass,
    symbol_count: usize,
    started: Instant,
) {
    if !lsp::timing_enabled() {
        return;
    }

    let outcome = if pass.ran_out_of_time {
        "it ran out of time, so the totals are floors"
    } else if pass.skipped_files {
        "some files were too big or could not be read, so the totals are floors"
    } else {
        "every file on the list was read, so the totals are exact"
    };
    eprintln!(
        "Timing: counting {symbol_count} names across {} files took {} ms; {outcome}.",
        pass.scanned_files,
        elapsed_millis(started)
    );
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

fn read_git_commit_files_sync(
    root: PathBuf,
    sha: String,
) -> Result<Vec<GitCommitFileChange>, String> {
    validate_git_root(&root)?;
    let sha = validate_git_commit_id(&sha)?;

    let output = run_git_text(&root, &["show", "--name-status", "--format=", sha.as_str()])?;
    Ok(parse_git_commit_file_changes(&output))
}

fn read_git_commit_file_diff_sync(
    root: PathBuf,
    sha: String,
    relative_path: String,
) -> Result<SourceGitDiff, String> {
    validate_git_root(&root)?;
    let relative_path = validate_git_relative_paths(&[relative_path])?
        .into_iter()
        .next()
        .expect("one validated path");
    let sha = validate_git_commit_id(&sha)?;

    let name_status = run_git_text(
        &root,
        &[
            "show",
            "--name-status",
            "--format=",
            sha.as_str(),
            "--",
            relative_path.as_str(),
        ],
    )?;
    let status = parse_git_commit_file_changes(&name_status)
        .into_iter()
        .next()
        .map(|change| change.status)
        .unwrap_or_default();

    // `--format=` drops the commit header, so what comes back is only the patch —
    // the same text shape the working-copy diff returns, readable by the same parser.
    let diff = run_git_text(
        &root,
        &[
            "show",
            "--no-ext-diff",
            "--format=",
            sha.as_str(),
            "--",
            relative_path.as_str(),
        ],
    )?;
    let is_binary = diff.contains("Binary files ") || diff.contains("GIT binary patch");
    let models = if is_binary {
        git_diff_models::GitDiffModels {
            original_content: None,
            modified_content: None,
        }
    } else {
        git_diff_models::commit_models(&root, &sha, &relative_path)?
    };
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
        original_content: models.original_content,
        modified_content: models.modified_content,
    })
}

/// `git show --name-status --format=` prints one tab-separated line per file:
/// a status letter (renames and copies add a similarity number, and a second path)
/// then the path. A merge commit prints nothing here, which reads as an empty list.
fn parse_git_commit_file_changes(output: &str) -> Vec<GitCommitFileChange> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let status_field = fields.next()?.trim();
            let status_code = status_field.chars().next()?;
            // For a rename or a copy git prints the old path then the new one; the
            // file lives at the last path, so that is the one we report.
            let relative_path = fields.last().map(str::trim).unwrap_or_default();
            if relative_path.is_empty() {
                return None;
            }

            Some(GitCommitFileChange {
                relative_path: normalize_git_status_path(relative_path),
                status: git_status_name(status_code).to_string(),
                badge: git_status_badge(status_code, ' ').to_string(),
            })
        })
        .collect()
}

/// Commit ids reach git as a bare argument, so anything that could be read as an
/// option (or as a shell-ish path) is refused instead of forwarded.
fn validate_git_commit_id(sha: &str) -> Result<String, String> {
    let trimmed = sha.trim();
    let is_safe = !trimmed.is_empty()
        && !trimmed.starts_with('-')
        && trimmed.len() <= 200
        && trimmed.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '_' | '-' | '.' | '/' | '^' | '~')
        });

    if is_safe {
        Ok(trimmed.to_string())
    } else {
        Err("Git commit id must be a plain commit id or ref name".to_string())
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
    let models = if is_binary {
        git_diff_models::GitDiffModels {
            original_content: None,
            modified_content: None,
        }
    } else {
        git_diff_models::working_tree_models(&canonical_root, &relative_path, &canonical_path)?
    };
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
        original_content: models.original_content,
        modified_content: models.modified_content,
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
                if !record.is_prunable {
                    record.is_dirty = project_worktree_is_dirty(&record.path);
                    record.has_unmerged_commits =
                        project_worktree_has_unmerged_commits(&record.path);
                    record.last_activity = project_worktree_last_activity(&record.path);
                }
                record.delete_eligibility = project_worktree_delete_eligibility(
                    record.is_dirty,
                    record.has_unmerged_commits,
                    record.is_prunable,
                    record.is_locked,
                );
                record
            })
            .collect(),
    )
}

/// `force = false` is the everyday remove: it refuses anything that would lose work.
/// `force = true` is the "I know, delete it anyway" remove: it unlocks the worktree if
/// it is locked, deletes it even when files are uncommitted or commits are unmerged, and
/// reports in the returned message exactly what went away. Neither mode will ever touch
/// the primary checkout.
pub(crate) fn remove_project_worktree_sync(
    root: PathBuf,
    path: PathBuf,
    force: bool,
) -> Result<ProjectWorktreeActionResult, String> {
    validate_git_root(&root)?;

    let root_metadata = std::fs::metadata(&root)
        .map_err(|error| format!("Could not read worktree root metadata: {error}"))?;
    if !root_metadata.is_dir() {
        return Err("Worktree root is not a directory".to_string());
    }

    let canonical_root = std::fs::canonicalize(&root)
        .map_err(|error| format!("Could not resolve Git root: {error}"))?;
    let canonical_path = std::fs::canonicalize(&path).ok();
    if canonical_path
        .as_ref()
        .map(|path| normalized_path_string(&canonical_root) == normalized_path_string(path))
        .unwrap_or(false)
    {
        return Err("Refusing to remove the primary checkout".to_string());
    }

    let worktrees = list_project_worktrees_sync(root.clone())?;
    let worktree = worktrees
        .iter()
        .find(|worktree| {
            project_worktree_path_matches(&worktree.path, &path, canonical_path.as_deref())
        })
        .ok_or_else(|| "Worktree path is not registered for this repository".to_string())?;

    // Comparing the path against the root that was passed only catches "remove the repo you
    // asked through". Ask through a linked worktree FOR the primary checkout and those two
    // paths differ, so that check waves it through. `git worktree list --porcelain` always
    // prints the main working tree first, so that is the answer that actually holds.
    if worktrees
        .first()
        .is_some_and(|main_working_tree| main_working_tree.path == worktree.path)
    {
        return Err("Refusing to remove the primary checkout".to_string());
    }

    if !force {
        if worktree.is_locked {
            return Err("Refusing to remove locked worktree".to_string());
        }
        if worktree.is_dirty {
            return Err("Refusing to remove dirty worktree".to_string());
        }
        if worktree.has_unmerged_commits {
            return Err("Refusing to remove worktree with unmerged commits".to_string());
        }
    }
    if worktree.is_prunable {
        // Only this row. `git worktree prune` would clear the records of EVERY
        // worktree whose folder is gone, and it did: a reader who clicked one
        // stale row watched two of them disappear. Git documents removing a
        // single `.git/worktrees/<name>` folder by hand as the supported way to
        // forget one worktree, which is exactly the promise the button makes.
        let branch = worktree.branch.clone();
        let cleared = clear_one_worktree_record(&root, &worktree.path)?;
        return Ok(ProjectWorktreeActionResult {
            message: if cleared {
                format!(
                    "Cleared git's records for {branch}. Its folder was already gone, and nothing on disk was touched."
                )
            } else {
                format!("Git had no records left for {branch}, so there was nothing to clear.")
            },
            worktrees: list_project_worktrees_sync(root)?,
        });
    }

    let path_metadata = std::fs::metadata(&path)
        .map_err(|error| format!("Could not read worktree path metadata: {error}"))?;
    if !path_metadata.is_dir() {
        return Err("Worktree path is not a directory".to_string());
    }

    let worktree_path = worktree.path.clone();

    if force {
        // Count what is about to be destroyed BEFORE destroying it, so the message can
        // name it. After the remove there is nothing left to look at.
        let dirty_file_count = project_worktree_dirty_file_count(&worktree_path);
        let was_dirty = worktree.is_dirty;
        let had_unmerged_commits = worktree.has_unmerged_commits;
        let unlocked_reason = worktree.is_locked.then(|| {
            worktree
                .locked_reason
                .clone()
                .unwrap_or_default()
                .trim()
                .to_string()
        });

        if unlocked_reason.is_some() {
            run_git_text(&root, &["worktree", "unlock", worktree_path.as_str()])?;
        }
        run_git_text(
            &root,
            &["worktree", "remove", "--force", worktree_path.as_str()],
        )?;
        run_git_text(&root, &["worktree", "prune"])?;

        return Ok(ProjectWorktreeActionResult {
            message: describe_forced_worktree_removal(
                &worktree.branch,
                dirty_file_count,
                was_dirty,
                had_unmerged_commits,
                unlocked_reason.as_deref(),
            ),
            worktrees: list_project_worktrees_sync(root)?,
        });
    }

    run_git_text(&root, &["worktree", "remove", worktree_path.as_str()])?;
    run_git_text(&root, &["worktree", "prune"])?;

    Ok(ProjectWorktreeActionResult {
        message: format!("Removed worktree {}", worktree.branch),
        worktrees: list_project_worktrees_sync(root)?,
    })
}

/// Forget ONE worktree, leaving every other worktree's records alone.
///
/// Git keeps a small folder of records per linked worktree under the
/// repository's `worktrees` directory. The folder is named after the worktree
/// but not reliably so — two worktrees whose folders share a name get suffixed
/// names — so this does not guess from the name. Each record folder holds a
/// `gitdir` file naming the worktree it belongs to, and that is what is matched.
///
/// Returns whether a record folder was actually found and removed.
fn clear_one_worktree_record(root: &Path, worktree_path: &str) -> Result<bool, String> {
    let records_directory = git_worktree_records_directory(root)?;
    let Ok(entries) = std::fs::read_dir(&records_directory) else {
        return Ok(false);
    };

    for entry in entries.flatten() {
        let record_directory = entry.path();
        if !record_directory.is_dir() {
            continue;
        }
        let Ok(gitdir) = std::fs::read_to_string(record_directory.join("gitdir")) else {
            continue;
        };
        if worktree_record_gitdir_names(gitdir.trim()) != Some(worktree_path.to_string()) {
            continue;
        }

        std::fs::remove_dir_all(&record_directory).map_err(|error| {
            format!(
                "Could not clear git's records for this worktree: {error} ({})",
                record_directory.display()
            )
        })?;
        return Ok(true);
    }

    Ok(false)
}

/// Where this repository keeps its per-worktree record folders.
///
/// Asked of git rather than assumed to be `<root>/.git/worktrees`: the answer
/// may be relative to the root, and in a repository that is itself a linked
/// worktree the records live in the primary checkout, not next door.
fn git_worktree_records_directory(root: &Path) -> Result<PathBuf, String> {
    let common_directory = run_git_text(root, &["rev-parse", "--git-common-dir"])?
        .trim()
        .to_string();
    if common_directory.is_empty() {
        return Err("Could not find where this repository keeps its records".to_string());
    }

    let common_directory = PathBuf::from(&common_directory);
    let common_directory = if common_directory.is_absolute() {
        common_directory
    } else {
        root.join(common_directory)
    };

    Ok(common_directory.join("worktrees"))
}

/// Which worktree a record folder's `gitdir` file points at.
///
/// The file names the worktree's own `.git` file — `/path/to/worktree/.git` —
/// so the worktree is its parent folder. The folder is normally gone by the
/// time this is asked, which is why this is pure text and never touches disk.
fn worktree_record_gitdir_names(gitdir: &str) -> Option<String> {
    let gitdir = gitdir.trim();
    if gitdir.is_empty() {
        return None;
    }
    Path::new(gitdir).parent().map(normalized_path_string)
}

fn archive_project_worktree_sync(
    root: PathBuf,
    path: PathBuf,
) -> Result<ProjectWorktreeArchiveResult, String> {
    validate_git_root(&root)?;

    let root_metadata = std::fs::metadata(&root)
        .map_err(|error| format!("Could not read worktree root metadata: {error}"))?;
    if !root_metadata.is_dir() {
        return Err("Worktree root is not a directory".to_string());
    }

    let path_metadata = std::fs::metadata(&path)
        .map_err(|error| format!("Could not read worktree path metadata: {error}"))?;
    if !path_metadata.is_dir() {
        return Err("Worktree path is not a directory".to_string());
    }

    let canonical_root = std::fs::canonicalize(&root)
        .map_err(|error| format!("Could not resolve Git root: {error}"))?;
    let canonical_path = std::fs::canonicalize(&path)
        .map_err(|error| format!("Could not resolve worktree path: {error}"))?;
    if normalized_path_string(&canonical_root) == normalized_path_string(&canonical_path) {
        return Err("Refusing to archive the primary checkout as a removable worktree".to_string());
    }

    let worktrees = list_project_worktrees_sync(root.clone())?;
    let worktree = worktrees
        .iter()
        .find(|worktree| {
            project_worktree_path_matches(&worktree.path, &path, Some(&canonical_path))
        })
        .ok_or_else(|| "Worktree path is not registered for this repository".to_string())?;
    let archive_path = project_worktree_archive_path(worktree)?;
    std::fs::create_dir_all(&archive_path)
        .map_err(|error| format!("Could not create worktree archive directory: {error}"))?;

    write_git_command_output(
        &canonical_path,
        &["status", "--short", "--branch"],
        &archive_path.join("status.txt"),
    )?;
    write_git_command_output(
        &canonical_path,
        &["log", "--oneline", "--decorate", "--max-count=40"],
        &archive_path.join("commits.txt"),
    )?;
    write_git_command_output(
        &canonical_path,
        &["diff", "--binary"],
        &archive_path.join("unstaged.patch"),
    )?;
    write_git_command_output(
        &canonical_path,
        &["diff", "--cached", "--binary"],
        &archive_path.join("staged.patch"),
    )?;

    let untracked_paths = git_untracked_paths(&canonical_path)?;
    std::fs::write(
        archive_path.join("untracked.txt"),
        format!("{}\n", untracked_paths.join("\n")),
    )
    .map_err(|error| format!("Could not write untracked file list: {error}"))?;
    copy_untracked_worktree_files(
        &canonical_path,
        &archive_path.join("untracked"),
        &untracked_paths,
    )?;

    let bundle_path = archive_path.join("head.bundle");
    run_git_text(
        &canonical_path,
        &[
            "bundle",
            "create",
            bundle_path.to_str().unwrap_or_default(),
            "HEAD",
        ],
    )?;

    Ok(ProjectWorktreeArchiveResult {
        message: format!("Archived worktree {}", worktree.branch),
        archive_path: normalized_path_string(&archive_path),
        worktrees: list_project_worktrees_sync(root)?,
    })
}

fn project_worktree_path_matches(
    record_path: &str,
    target_path: &Path,
    canonical_target: Option<&Path>,
) -> bool {
    let normalized_target = normalized_path_string(target_path);
    let normalized_record = normalized_path_string(Path::new(record_path));
    if normalized_record == normalized_target {
        return true;
    }

    let Some(canonical_target) = canonical_target else {
        return false;
    };
    let normalized_canonical_target = normalized_path_string(canonical_target);
    if normalized_record == normalized_canonical_target {
        return true;
    }

    std::fs::canonicalize(record_path)
        .ok()
        .map(|canonical_record| {
            normalized_path_string(&canonical_record) == normalized_canonical_target
        })
        .unwrap_or(false)
}

fn parse_project_worktree_porcelain(output: &str) -> Vec<ProjectWorktree> {
    output
        .split("\n\n")
        .filter_map(|block| {
            let mut path = None;
            let mut branch = None;
            let mut is_prunable = false;
            let mut prunable_reason = None;
            let mut is_locked = false;
            let mut locked_reason = None;

            for line in block.lines() {
                if let Some(value) = line.strip_prefix("worktree ") {
                    path = Some(value.to_string());
                } else if let Some(value) = line.strip_prefix("branch ") {
                    branch = Some(value.trim_start_matches("refs/heads/").to_string());
                } else if line == "detached" {
                    branch = Some("detached".to_string());
                } else if line == "prunable" {
                    is_prunable = true;
                } else if let Some(value) = line.strip_prefix("prunable ") {
                    is_prunable = true;
                    prunable_reason = Some(value.to_string());
                } else if line == "locked" {
                    is_locked = true;
                } else if let Some(value) = line.strip_prefix("locked ") {
                    is_locked = true;
                    locked_reason = Some(value.to_string());
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
                    is_prunable,
                    prunable_reason,
                    is_locked,
                    locked_reason,
                    last_activity: None,
                    delete_eligibility: "unknown".to_string(),
                }
            })
        })
        .collect()
}

fn project_worktree_delete_eligibility(
    is_dirty: bool,
    has_unmerged_commits: bool,
    is_prunable: bool,
    is_locked: bool,
) -> String {
    if is_dirty {
        "blocked: dirty worktree".to_string()
    } else if has_unmerged_commits {
        "blocked: unmerged commits".to_string()
    } else if is_locked {
        "blocked: locked worktree".to_string()
    } else if is_prunable {
        "review: prunable missing worktree metadata".to_string()
    } else {
        "requires-confirmation".to_string()
    }
}

/// Plain sentences describing what a forced removal actually threw away, so the person
/// who clicked the button can read the consequence rather than decode a status code.
/// `dirty_file_count` is `None` when git could not be asked how much was there. That is NOT
/// the same as zero, and it must never be reported as "nothing was lost" — the removal has
/// already happened by the time this runs. `was_dirty` is what the worktree listing said a
/// moment earlier, and it is believed over a count of zero when the two disagree.
fn describe_forced_worktree_removal(
    branch: &str,
    dirty_file_count: Option<usize>,
    was_dirty: bool,
    had_unpushed_commits: bool,
    unlocked_reason: Option<&str>,
) -> String {
    let mut message = format!("Force removed worktree {branch}.");

    match dirty_file_count {
        Some(count) if count > 0 => {
            let noun = if count == 1 { "file" } else { "files" };
            message.push_str(&format!(
                " Deleted {count} {noun} with changes that were never committed."
            ));
        }
        // Counted zero, but the listing had already seen changes — the count is the one
        // that is wrong, so say what is known and admit the number is not.
        _ if was_dirty => {
            message.push_str(
                " Deleted files with changes that were never committed; they could not be counted before the removal.",
            );
        }
        Some(_) => {
            message.push_str(" It had no uncommitted changes.");
        }
        // Nothing could be read at all, so promising a clean worktree would be a guess.
        None => {
            message.push_str(
                " Could not check for uncommitted changes before the removal, so anything not committed is gone.",
            );
        }
    }

    // The check behind this is "does this branch have commits that are on no remote", so
    // the sentence says that and nothing wider.
    if had_unpushed_commits {
        message.push_str(" Deleted commits on this branch that were never pushed to a remote.");
    }

    if let Some(reason) = unlocked_reason {
        if reason.trim().is_empty() {
            message.push_str(" Unlocked it first; it was locked with no reason given.");
        } else {
            message.push_str(&format!(
                " Unlocked it first; it was locked because: {}.",
                reason.trim()
            ));
        }
    }

    message
}

/// How many files in the worktree have changes git has not been told to keep — the
/// number a forced removal is about to delete for good.
/// How many files in the worktree have changes that were never committed, or `None` when
/// git could not answer — a stale `.git` file, an index lock, a folder that moved. The
/// caller is about to destroy this worktree and then tell someone what was in it, so a
/// failed count must NOT come back looking like a confident zero.
fn project_worktree_dirty_file_count(path: &str) -> Option<usize> {
    let output = Command::new("git")
        .args([
            "-C",
            path,
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
        ])
        .output();
    output
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count()
        })
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
            "--oneline",
            "-1",
            "HEAD",
            "--not",
            "--remotes",
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

fn project_worktree_archive_path(worktree: &ProjectWorktree) -> Result<PathBuf, String> {
    let archive_root = project_worktree_archive_root(Path::new(&worktree.path));
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("Could not build archive timestamp: {error}"))?
        .as_millis();
    let repo = safe_archive_path_segment(&worktree.repo);
    let branch = safe_archive_path_segment(&worktree.branch);
    Ok(archive_root
        .join(repo)
        .join(format!("{branch}-{timestamp}")))
}

fn project_worktree_archive_root(path: &Path) -> PathBuf {
    let normalized_path = normalized_path_string(path);
    let marker = "/worktrees/";
    if let Some(index) = normalized_path.find(marker) {
        return PathBuf::from(format!("{}/worktree-archives", &normalized_path[..index]));
    }

    path.parent()
        .map(|parent| parent.join("worktree-archives"))
        .unwrap_or_else(|| PathBuf::from("worktree-archives"))
}

fn safe_archive_path_segment(value: &str) -> String {
    let mut segment = value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    while segment.contains("--") {
        segment = segment.replace("--", "-");
    }
    let segment = segment.trim_matches('-').to_string();
    if segment.is_empty() {
        "worktree".to_string()
    } else {
        segment
    }
}

fn write_git_command_output(root: &Path, args: &[&str], destination: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
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

    std::fs::write(destination, output.stdout)
        .map_err(|error| format!("Could not write {}: {error}", destination.display()))
}

fn git_untracked_paths(root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .output()
        .map_err(|error| format!("Could not list untracked files: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("git ls-files exited with {}", output.status)
        } else {
            stderr
        });
    }

    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| String::from_utf8_lossy(chunk).to_string())
        .filter(|path| is_safe_git_relative_path(path))
        .collect())
}

fn copy_untracked_worktree_files(
    root: &Path,
    destination_root: &Path,
    relative_paths: &[String],
) -> Result<(), String> {
    for relative_path in relative_paths {
        if !is_safe_git_relative_path(relative_path) {
            continue;
        }

        let source = root.join(relative_path);
        let Ok(metadata) = std::fs::metadata(&source) else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }

        let destination = destination_root.join(relative_path);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!("Could not create untracked archive directory: {error}")
            })?;
        }
        std::fs::copy(&source, &destination).map_err(|error| {
            format!(
                "Could not copy untracked file {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
    }

    Ok(())
}

fn is_safe_git_relative_path(path: &str) -> bool {
    let trimmed = path.trim();
    let relative_path = Path::new(trimmed);
    !trimmed.is_empty()
        && !trimmed.contains('\0')
        && !relative_path.is_absolute()
        && relative_path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
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
        has_upstream: status.has_upstream,
        last_commit_sha,
        last_commit_subject,
        last_commit_at,
        dirty_since_epoch_ms: git_dirty_since_epoch_ms(path, &status.files),
        dirty_status_fingerprint: git_dirty_status_fingerprint(&status.files),
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
        has_upstream: false,
        last_commit_sha: None,
        last_commit_subject: None,
        last_commit_at: None,
        dirty_since_epoch_ms: None,
        dirty_status_fingerprint: git_dirty_status_fingerprint(&[]),
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

fn git_dirty_status_fingerprint(files: &[GitFileStatus]) -> String {
    let mut rows: Vec<String> = files
        .iter()
        .map(|file| {
            [
                file.relative_path.as_str(),
                file.index_status.as_str(),
                file.worktree_status.as_str(),
                file.status.as_str(),
                file.badge.as_str(),
            ]
            .join("\x1f")
        })
        .collect();
    rows.sort();

    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for row in rows {
        for byte in row.as_bytes().iter().copied().chain([0]) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    format!("{hash:016x}")
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

fn list_playwright_sessions_sync() -> Result<Vec<PlaywrightSessionInfo>, String> {
    let output = Command::new("ps")
        .args(["-axo", "pid=,pgid=,etime=,command="])
        .output()
        .map_err(|error| format!("Could not run ps: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "Could not list processes: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(group_playwright_processes(parse_playwright_processes(
        &String::from_utf8_lossy(&output.stdout),
    )))
}

fn kill_playwright_sessions_sync() -> Result<PlaywrightCleanupResult, String> {
    let sessions = list_playwright_sessions_sync()?;
    kill_playwright_sessions_with(
        sessions,
        signal_process,
        || std::thread::sleep(std::time::Duration::from_millis(800)),
        list_playwright_sessions_sync,
    )
}

fn kill_playwright_session_sync(pgid: i32) -> Result<PlaywrightCleanupResult, String> {
    let sessions = list_playwright_sessions_sync()?;
    kill_playwright_session_with(
        sessions,
        pgid,
        signal_process,
        || std::thread::sleep(std::time::Duration::from_millis(800)),
        list_playwright_sessions_sync,
    )
}

/// Stops ONE Playwright process group, the same polite-then-forceful way the
/// stop-everything command does. The group has to be one this app just listed as a
/// Playwright session; anything else is refused, so this can never be used to stop an
/// arbitrary process by number.
fn kill_playwright_session_with<SignalProcess, SleepAfterTerm, ListSessions>(
    sessions: Vec<PlaywrightSessionInfo>,
    pgid: i32,
    signal_process: SignalProcess,
    sleep_after_term: SleepAfterTerm,
    list_sessions: ListSessions,
) -> Result<PlaywrightCleanupResult, String>
where
    SignalProcess: FnMut(u32, &str) -> Result<(), String>,
    SleepAfterTerm: FnOnce(),
    ListSessions: FnMut() -> Result<Vec<PlaywrightSessionInfo>, String>,
{
    let session = select_playwright_session(sessions, pgid)?;
    kill_playwright_sessions_with(
        vec![session],
        signal_process,
        sleep_after_term,
        list_sessions,
    )
}

fn select_playwright_session(
    sessions: Vec<PlaywrightSessionInfo>,
    pgid: i32,
) -> Result<PlaywrightSessionInfo, String> {
    let matched = u32::try_from(pgid).ok().and_then(|pgid| {
        sessions
            .into_iter()
            .find(|session| session.pgid == pgid && !session.pids.is_empty())
    });

    matched.ok_or_else(|| {
        format!("No Playwright session is running in process group {pgid}, so nothing was stopped")
    })
}

fn kill_playwright_sessions_with<SignalProcess, SleepAfterTerm, ListSessions>(
    sessions: Vec<PlaywrightSessionInfo>,
    mut signal_process: SignalProcess,
    sleep_after_term: SleepAfterTerm,
    mut list_sessions: ListSessions,
) -> Result<PlaywrightCleanupResult, String>
where
    SignalProcess: FnMut(u32, &str) -> Result<(), String>,
    SleepAfterTerm: FnOnce(),
    ListSessions: FnMut() -> Result<Vec<PlaywrightSessionInfo>, String>,
{
    let mut terminated_pgids = Vec::new();
    let mut terminated_pids = Vec::new();
    let mut failed_pgids = Vec::new();

    for session in &sessions {
        let mut session_had_signal = false;
        for pid in &session.pids {
            if let Err(message) = signal_process(*pid, "TERM") {
                failed_pgids.push(PlaywrightCleanupFailure {
                    pgid: session.pgid,
                    pid: Some(*pid),
                    message,
                });
                continue;
            }
            session_had_signal = true;
            terminated_pids.push(*pid);
        }
        if session_had_signal {
            terminated_pgids.push(session.pgid);
        }
    }

    sleep_after_term();

    let remaining_pids = match list_sessions() {
        Ok(remaining_sessions) => remaining_sessions
            .into_iter()
            .flat_map(|session| session.pids)
            .collect::<HashSet<_>>(),
        Err(error) => {
            for pgid in &terminated_pgids {
                failed_pgids.push(PlaywrightCleanupFailure {
                    pgid: *pgid,
                    pid: None,
                    message: format!("Could not verify Playwright cleanup after TERM: {error}"),
                });
            }
            return Ok(PlaywrightCleanupResult {
                sessions,
                terminated_pgids,
                terminated_pids,
                failed_pgids,
            });
        }
    };
    let terminated_pid_set = terminated_pids.iter().copied().collect::<HashSet<_>>();
    for session in &sessions {
        for pid in session
            .pids
            .iter()
            .copied()
            .filter(|pid| terminated_pid_set.contains(pid) && remaining_pids.contains(pid))
        {
            if let Err(message) = signal_process(pid, "KILL") {
                failed_pgids.push(PlaywrightCleanupFailure {
                    pgid: session.pgid,
                    pid: Some(pid),
                    message,
                });
            }
        }
    }

    Ok(PlaywrightCleanupResult {
        sessions,
        terminated_pgids,
        terminated_pids,
        failed_pgids,
    })
}

fn kill_process_sync(pid: u32, expected_command: Option<String>) -> ProcessKillResult {
    // Process numbers get handed out again after a process exits. The panel the
    // reader clicked in may be minutes old, so before anything is signalled,
    // check the number still belongs to the command the panel showed them.
    if let Some(expected) = expected_command.as_deref().map(str::trim) {
        if !expected.is_empty() && pid > 1 {
            match process_command_name(pid) {
                None => {
                    return ProcessKillResult {
                        ok: false,
                        message: format!(
                            "Process {pid} has already exited, so there was nothing to stop."
                        ),
                    };
                }
                Some(actual) => {
                    if !process_commands_match(&actual, expected) {
                        return ProcessKillResult {
                            ok: false,
                            message: format!(
                                "Process {pid} now belongs to \"{actual}\", not \"{expected}\" — the number was reused by something else, so nothing was stopped. Refresh the list and try again."
                            ),
                        };
                    }
                }
            }
        }
    }
    kill_process_with(pid, std::process::id(), signal_process)
}

/// The executable a process number belongs to right now, or None when no such
/// process exists.
fn process_command_name(pid: u32) -> Option<String> {
    let output = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// The panel shows short names ("node") while `ps` answers with full paths
/// ("/usr/local/bin/node"), so the comparison is between file names, either
/// containing the other to survive versioned names like "node22".
fn process_commands_match(actual: &str, expected: &str) -> bool {
    fn file_name(command: &str) -> &str {
        command.trim().rsplit('/').next().unwrap_or(command)
    }
    let actual_name = file_name(actual);
    let expected_name = file_name(expected);
    if actual_name.is_empty() || expected_name.is_empty() {
        return false;
    }
    actual_name == expected_name
        || actual_name.contains(expected_name)
        || expected_name.contains(actual_name)
}

/// Stop one process, with the refusals spelled out.
///
/// Numbers 0 and 1 are not ordinary processes — 0 means "every process in my
/// group" to the kill signal and 1 is the system's own first process, so both
/// are refused before anything is sent. So is this app's own number, which
/// would otherwise close the window the reader clicked in.
fn kill_process_with<SignalProcess>(
    pid: u32,
    own_pid: u32,
    mut signal_process: SignalProcess,
) -> ProcessKillResult
where
    SignalProcess: FnMut(u32, &str) -> Result<(), String>,
{
    if pid <= 1 {
        return ProcessKillResult {
            ok: false,
            message: format!(
                "{pid} is not a process this app will stop — it is the system's own, and stopping it would take the machine down."
            ),
        };
    }
    if pid == own_pid {
        return ProcessKillResult {
            ok: false,
            message: "That number belongs to this app itself, so nothing was stopped.".to_string(),
        };
    }

    match signal_process(pid, "TERM") {
        Ok(()) => ProcessKillResult {
            ok: true,
            message: format!(
                "Asked process {pid} to stop. It may take a moment to finish shutting down."
            ),
        },
        Err(reason) => ProcessKillResult {
            ok: false,
            message: format!(
                "Could not stop process {pid}. It may have already exited, or it may belong to another user. ({reason})"
            ),
        },
    }
}

fn signal_process(pid: u32, signal: &str) -> Result<(), String> {
    if pid == 0 || pid == std::process::id() {
        return Err(format!("Refusing to signal unsafe process {pid}"));
    }

    let output = Command::new("kill")
        .args([format!("-{signal}"), pid.to_string()])
        .output()
        .map_err(|error| format!("Could not run kill -{signal} for pid {pid}: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "kill -{signal} {pid} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn parse_playwright_processes(output: &str) -> Vec<PlaywrightProcessInfo> {
    output
        .lines()
        .filter_map(parse_playwright_process_line)
        .filter_map(|process| {
            playwright_process_label(&process.command, &process.args)
                .map(|label| PlaywrightProcessInfo { label, ..process })
        })
        .collect()
}

fn parse_playwright_process_line(line: &str) -> Option<PlaywrightProcessInfo> {
    let (pid_text, rest) = next_ps_field(line)?;
    let (pgid_text, rest) = next_ps_field(rest)?;
    let (elapsed, args) = next_ps_field(rest)?;
    let pid = pid_text.parse::<u32>().ok()?;
    let pgid = pgid_text.parse::<u32>().ok()?;
    let elapsed = elapsed.to_string();
    let args = args.trim().to_string();
    let command = args.clone();
    let name = playwright_process_name(&args);

    Some(PlaywrightProcessInfo {
        pid,
        pgid,
        command,
        name,
        label: String::new(),
        elapsed,
        args,
    })
}

fn next_ps_field(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    let split_index = trimmed.find(char::is_whitespace)?;
    let (field, rest) = trimmed.split_at(split_index);
    (!field.is_empty()).then_some((field, rest.trim_start()))
}

fn playwright_process_name(args: &str) -> String {
    if args.contains("Google Chrome") {
        return "Google Chrome".to_string();
    }
    if args.contains("Chromium") {
        return "Chromium".to_string();
    }
    if has_playwright_mcp_command(args) {
        return "playwright-mcp".to_string();
    }
    if has_playwright_cli_daemon(args) {
        return "cliDaemon.js".to_string();
    }
    if has_playwright_cli_server(args) {
        return "playwright/cli.js".to_string();
    }

    args.split_whitespace()
        .next()
        .map(|command| {
            Path::new(command)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(command)
                .to_string()
        })
        .unwrap_or_else(|| "process".to_string())
}

fn group_playwright_processes(processes: Vec<PlaywrightProcessInfo>) -> Vec<PlaywrightSessionInfo> {
    let mut by_pgid: HashMap<u32, Vec<PlaywrightProcessInfo>> = HashMap::new();
    for process in processes {
        by_pgid.entry(process.pgid).or_default().push(process);
    }

    let mut sessions = by_pgid
        .into_iter()
        .map(|(pgid, mut processes)| {
            processes.sort_by_key(|process| process.pid);
            let pids = processes
                .iter()
                .map(|process| process.pid)
                .collect::<Vec<_>>();
            let label = processes
                .iter()
                .find(|process| process.label != "Playwright browser")
                .or_else(|| processes.first())
                .map(|process| process.label.clone())
                .unwrap_or_else(|| "Playwright session".to_string());
            PlaywrightSessionInfo {
                pgid,
                label,
                pids,
                processes,
            }
        })
        .collect::<Vec<_>>();

    sessions.sort_by(|left, right| left.pgid.cmp(&right.pgid));
    sessions
}

fn playwright_process_label(command: &str, args: &str) -> Option<String> {
    let combined = format!("{command} {args}");
    if has_playwright_cli_daemon(&combined) {
        return Some("Playwright CLI daemon".to_string());
    }
    if has_playwright_cli_server(&combined) {
        return Some("Playwright CLI server".to_string());
    }
    if has_playwright_mcp_command(&combined) {
        return Some("Playwright MCP".to_string());
    }
    if looks_like_chrome_command(command, args) && has_playwright_profile_arg(&combined) {
        return Some("Playwright browser".to_string());
    }

    None
}

fn has_playwright_cli_daemon(command_line: &str) -> bool {
    command_line.split_whitespace().any(|arg| {
        let normalized = arg.replace('\\', "/");
        normalized.ends_with("/cliDaemon.js")
            && (normalized.contains("/node_modules/playwright-core/")
                || normalized.contains("/node_modules/playwright/")
                || normalized.contains("/node_modules/@playwright/"))
    })
}

fn has_playwright_cli_server(command_line: &str) -> bool {
    command_line
        .split_whitespace()
        .any(|arg| path_ends_with(arg, "/node_modules/playwright/cli.js"))
        && command_line
            .split_whitespace()
            .any(|arg| arg == "run-cli-server")
}

fn has_playwright_mcp_command(command_line: &str) -> bool {
    command_line.split_whitespace().any(|arg| {
        let name = Path::new(arg)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(arg);
        matches!(name, "playwright-mcp" | "playwright-mcp.js")
    })
}

fn has_playwright_profile_arg(command_line: &str) -> bool {
    command_line.split_whitespace().any(|arg| {
        let Some(profile_path) = arg.strip_prefix("--user-data-dir=") else {
            return false;
        };
        Path::new(profile_path)
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.starts_with("playwright_chromiumdev_profile-"))
    })
}

fn path_ends_with(value: &str, suffix: &str) -> bool {
    value.replace('\\', "/").ends_with(suffix)
}

fn looks_like_chrome_command(command: &str, args: &str) -> bool {
    let lower_command = command.to_ascii_lowercase();
    let lower_args = args.to_ascii_lowercase();
    lower_command.contains("chrome")
        || lower_command.contains("chromium")
        || lower_args.contains("google chrome")
        || lower_args.contains("chromium")
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
        has_upstream: false,
        files: Vec::new(),
    };

    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        if let Some(header) = line.strip_prefix("## ") {
            let branch_header = parse_git_branch_header(header);
            git_status.branch = branch_header.branch;
            git_status.ahead = branch_header.ahead;
            git_status.behind = branch_header.behind;
            git_status.has_upstream = branch_header.has_upstream;
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitBranchHeader {
    branch: Option<String>,
    ahead: usize,
    behind: usize,
    has_upstream: bool,
}

fn parse_git_branch_header(header: &str) -> GitBranchHeader {
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

    let mut branch_parts = branch_part.splitn(2, "...");
    let branch_name_part = branch_parts.next().unwrap_or_default().trim();
    let has_upstream = branch_parts
        .next()
        .map(str::trim)
        .is_some_and(|upstream| !upstream.is_empty());

    if let Some(branch) = branch_part.strip_prefix("No commits yet on ") {
        let branch = branch
            .split("...")
            .next()
            .unwrap_or(branch)
            .trim()
            .to_string();
        return GitBranchHeader {
            branch: Some(branch),
            ahead,
            behind,
            has_upstream,
        };
    }

    let branch = Some(branch_name_part)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    GitBranchHeader {
        branch,
        ahead,
        behind,
        has_upstream,
    }
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
    if file_name == "makefile" {
        return "makefile".to_string();
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

fn source_collection_limit(limit: usize) -> usize {
    limit
        .max(DEFAULT_SOURCE_LIST_LIMIT)
        .min(MAX_SOURCE_LIST_LIMIT)
        .saturating_add(1)
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
    left.to_ascii_lowercase()
        .cmp(&right.to_ascii_lowercase())
        .then_with(|| left.cmp(right))
}

fn skip_dir_reason(name: &str) -> Option<&'static str> {
    let normalized = name.to_ascii_lowercase();
    if normalized.ends_with("_files") {
        return Some("saved web page asset directory");
    }

    match normalized.as_str() {
        ".git" | ".hg" | ".svn" => Some("version-control metadata directory"),
        ".agents" | ".claude" | ".codex" | ".dev" | ".history" | ".idea" | ".omx"
        | ".playwright" | ".playwright-cli" | ".run" | ".slots" | ".vscode" | ".zed" => {
            Some("agent/tool state directory")
        }
        "__pycache__" | ".cache" | ".build" | ".gradle" | ".next" | ".nuxt" | ".parcel-cache"
        | ".pytest_cache" | ".svelte-kit" | ".tmp" | ".turbo" | ".vite" | "bin" | "build"
        | "coverage" | "deriveddata" | "dist" | "obj" | "target" | "testresults" => {
            Some("build output/cache directory")
        }
        ".merge-backups" => Some("merge backup directory"),
        "node_modules" | "pods" | "vendor" => Some("dependency directory"),
        "worktrees" => Some("session worktree directory"),
        _ => None,
    }
}

fn source_file_matches_query(relative_path: &str, file_name: &str, query: Option<&str>) -> bool {
    let Some(query) = query else {
        return true;
    };
    relative_path.to_lowercase().contains(query) || file_name.to_lowercase().contains(query)
}

fn main() {
    let agent_runtime = agent_conversation::manager::AgentRuntimeManager::new(
        agent_conversation::providers::ProviderRegistry::bundled_from_environment()
            .expect("packaged ACP adapter configuration is invalid"),
    );
    let workflow_engine = WorkflowEngine::managed(agent_runtime.clone());
    let conversation_events = agent_runtime.clone();
    tauri::Builder::default()
        .manage(SourceScanRegistry::default())
        .manage(agent_runtime)
        .manage(workflow_engine)
        .manage(agent_conversation::terminal_projection::TerminalProjectionRegistry::default())
        .manage(lsp::SourceLspRegistry::default())
        .manage(terminal::TerminalRegistry::default())
        .manage(browser::BrowserRegistry::default())
        .manage(resources::ResourceRegistry::default())
        .manage(usage_history::UsageHistoryState::default())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let handle = app.handle().clone();
            conversation_events.set_emitter(Arc::new(move |event| {
                let _ = handle.emit("agent-conversation-event", event);
            }));
            // Every time a language server starts, finishes reading a project, or stops,
            // tell the editor straight away. Without this the editor would have to ask
            // over and over to notice, which is what it used to do.
            let app = app.handle().clone();
            lsp::set_source_lsp_status_listener(Arc::new(move |change| {
                let _ = app.emit(SOURCE_LSP_STATUS_CHANGED_EVENT, change);
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_backend_capabilities,
            list_source_files,
            cancel_source_scan,
            validate_project_root,
            read_source_file,
            read_native_csharp_file,
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
            count_source_references,
            read_source_lsp_status,
            list_source_lsp_statuses,
            warm_source_lsp_for_root,
            ensure_native_csharp_language_client,
            mark_native_csharp_language_client_ready,
            set_csharp_language_server_enabled,
            find_source_lsp_definitions,
            find_source_lsp_completions,
            find_source_lsp_implementations,
            find_source_lsp_type_definitions,
            find_source_lsp_document_highlights,
            find_source_lsp_signature_help,
            find_source_lsp_inlay_hints,
            find_source_lsp_semantic_tokens,
            find_source_lsp_workspace_symbols,
            format_source_with_lsp,
            rename_source_with_lsp,
            find_source_lsp_code_actions,
            find_source_lsp_references,
            find_source_lsp_hover,
            find_source_lsp_symbols,
            find_source_lsp_document_symbols,
            read_source_lsp_log,
            read_source_lsp_diagnostics,
            list_source_lsp_diagnostics_for_root,
            project_git_status,
            read_source_git_diff,
            stage_git_paths,
            unstage_git_paths,
            commit_git_repository,
            fetch_git_repository,
            pull_git_repository,
            push_git_repository,
            read_git_commit_history,
            read_git_commit_files,
            read_git_commit_file_diff,
            git_pr::generate_commit_message,
            git_pr::read_pull_request_context,
            git_pr::generate_pull_request_details,
            git_pr::create_pull_request,
            git_pr::read_pull_request_status,
            list_project_worktrees,
            remove_project_worktree,
            archive_project_worktree,
            list_git_repository_summaries,
            list_agent_sessions,
            list_runtime_contexts,
            list_playwright_sessions,
            kill_playwright_session,
            kill_playwright_sessions,
            kill_process,
            resources::read_resource_snapshot,
            resources::read_resource_disk_scan,
            resources::cleanup_workspace_disk_entry,
            resources::stop_owned_resource,
            resources::restart_language_server_root,
            resources::set_active_source_root,
            resources::apply_resource_memory_pressure,
            resources::read_language_server_log,
            usage_current::read_current_provider_usage,
            usage_history::read_usage_summary,
            usage_history::read_usage_breakdown,
            usage_history::read_usage_provider_summary,
            usage_history::read_usage_daily,
            usage_history::read_usage_daily_totals,
            usage_history::refresh_usage_history,
            list_orchestration_runs,
            record_orchestration_event,
            list_workflow_runs,
            create_workflow_run,
            start_workflow_run,
            pause_workflow_run,
            resume_workflow_run,
            cancel_workflow_run,
            retry_workflow_node,
            skip_workflow_node,
            approve_workflow_gate,
            submit_workflow_result,
            agent_conversation::ensure_agent_conversation,
            agent_conversation::send_agent_conversation_message,
            agent_conversation::respond_agent_conversation_approval,
            agent_conversation::stop_agent_conversation_turn,
            agent_conversation::close_agent_conversation,
            agent_conversation::read_agent_conversation_snapshot,
            agent_conversation::read_agent_conversation_transcript,
            agent_conversation::start_agent_conversation_terminal_projection,
            agent_conversation::stop_agent_conversation_terminal_projection,
            agent_conversation::save_agent_conversation_attachment,
            agent_conversation::handoff::handoff_agent_conversation,
            start_terminal_session,
            list_terminal_sessions,
            read_terminal_session_scrollback,
            write_terminal_session,
            resize_terminal_session,
            close_terminal_session,
            browser::create_browser_tab,
            browser::set_browser_tab_bounds,
            browser::set_browser_tab_viewport,
            browser::show_browser_tab,
            browser::hide_browser_workspace,
            browser::navigate_browser_tab,
            browser::reload_browser_tab,
            browser::go_back_browser_tab,
            browser::go_forward_browser_tab,
            browser::close_browser_tab,
            browser::clear_browser_workspace_data,
            browser::arm_browser_element_picker,
            browser::cancel_browser_element_picker,
            browser::capture_browser_viewport,
            browser::open_browser_tab_devtools,
            browser::open_browser_tab_external
        ])
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                window.state::<browser::BrowserRegistry>().shutdown();
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to run MacCommandBar webview preview");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn playwright_process_parser_groups_only_owned_markers() {
        let output = "\
  101   100 00:01:02 /usr/local/bin/node node /repo/node_modules/playwright/cli.js run-cli-server
  102   100 00:00:59 /Applications/Google Chrome.app/Contents/MacOS/Google Chrome --user-data-dir=/tmp/playwright_chromiumdev_profile-abc --type=browser
  103   100 00:00:03 /usr/local/bin/node node /important/unrelated-worker.js
  104   104 00:00:03 /Applications/Google Chrome.app/Contents/MacOS/Google Chrome --profile-directory=Default
  105   105 00:00:03 /usr/local/bin/node node server.js
";

        let sessions = group_playwright_processes(parse_playwright_processes(output));

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].pgid, 100);
        assert_eq!(sessions[0].pids, vec![101, 102]);
        assert_eq!(sessions[0].label, "Playwright CLI server");
    }

    #[test]
    fn playwright_process_detector_ignores_normal_chrome_vite_and_node() {
        assert_eq!(
            playwright_process_label(
                "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
                "--profile-directory=Default"
            ),
            None
        );
        assert_eq!(
            playwright_process_label(
                "/usr/local/bin/node",
                "node ./node_modules/vite/bin/vite.js"
            ),
            None
        );
        assert_eq!(
            playwright_process_label("/usr/local/bin/node", "node ./scripts/worker.js"),
            None
        );
    }

    #[test]
    fn playwright_process_detector_ignores_substring_marker_false_positives() {
        let cases = [
            (
                "/usr/local/bin/node",
                "node /tmp/not-playwright-mcp-helper.js",
            ),
            (
                "/usr/local/bin/node",
                "node /tmp/playwright/cli.js.backup run-cli-server",
            ),
            ("/usr/local/bin/node", "node /tmp/cliDaemon.js.notes"),
        ];

        for (command, args) in cases {
            assert_eq!(playwright_process_label(command, args), None, "{args}");
        }
    }

    #[test]
    fn lsp_diagnostics_remember_which_file_they_came_from() {
        let message = serde_json::json!({
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///repo/src/App.ts",
                "diagnostics": [{
                    "range": { "start": { "line": 4, "character": 2 } },
                    "severity": 1,
                    "message": "Cannot find name 'valu'.",
                    "source": "ts"
                }]
            }
        });

        let diagnostics = lsp::diagnostics_from_message(&message, "file:///repo/src/App.ts");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].path.as_deref(), Some("/repo/src/App.ts"));
        assert_eq!(diagnostics[0].line, 5);
        assert_eq!(diagnostics[0].column, 3);
        assert_eq!(diagnostics[0].severity, "error");
    }

    #[test]
    fn lsp_diagnostics_for_root_collect_every_file_inside_that_root() {
        let inside_root = PathBuf::from("/repo");
        let other_root = PathBuf::from("/elsewhere");

        let sessions = vec![
            (
                inside_root.clone(),
                HashMap::from([
                    (
                        "file:///repo/src/Later.ts".to_string(),
                        vec![lsp::SourceLspDiagnostic::for_test(
                            "warning",
                            "Unused import.",
                            2,
                            1,
                            Some("/repo/src/Later.ts"),
                        )],
                    ),
                    (
                        "file:///repo/src/App.ts".to_string(),
                        vec![
                            lsp::SourceLspDiagnostic::for_test(
                                "error",
                                "Second on the same file.",
                                9,
                                4,
                                Some("/repo/src/App.ts"),
                            ),
                            lsp::SourceLspDiagnostic::for_test(
                                "error",
                                "First on the same file.",
                                3,
                                1,
                                Some("/repo/src/App.ts"),
                            ),
                        ],
                    ),
                ]),
            ),
            (
                other_root.clone(),
                HashMap::from([(
                    "file:///elsewhere/src/Other.ts".to_string(),
                    vec![lsp::SourceLspDiagnostic::for_test(
                        "error",
                        "Belongs to another project.",
                        1,
                        1,
                        Some("/elsewhere/src/Other.ts"),
                    )],
                )]),
            ),
        ];

        let collected = lsp::collect_diagnostics_for_root(&sessions, &inside_root);

        assert_eq!(collected.len(), 3);
        // Grouped by file, then in the order they appear down the file, so a problems
        // list can render them without sorting again.
        assert_eq!(collected[0].path.as_deref(), Some("/repo/src/App.ts"));
        assert_eq!(collected[0].message, "First on the same file.");
        assert_eq!(collected[1].path.as_deref(), Some("/repo/src/App.ts"));
        assert_eq!(collected[1].message, "Second on the same file.");
        assert_eq!(collected[2].path.as_deref(), Some("/repo/src/Later.ts"));
        assert!(collected
            .iter()
            .all(|diagnostic| diagnostic.message != "Belongs to another project."));

        // A root nobody has diagnostics for reports an empty list, not an error.
        assert!(
            lsp::collect_diagnostics_for_root(&sessions, &PathBuf::from("/nothing")).is_empty()
        );
        assert!(lsp::collect_diagnostics_for_root(&[], &inside_root).is_empty());
    }

    #[test]
    fn lsp_diagnostics_for_root_do_not_match_a_sibling_with_the_same_name_prefix() {
        let sessions = vec![(
            PathBuf::from("/repo-backup"),
            HashMap::from([(
                "file:///repo-backup/src/App.ts".to_string(),
                vec![lsp::SourceLspDiagnostic::for_test(
                    "error",
                    "Different repository.",
                    1,
                    1,
                    Some("/repo-backup/src/App.ts"),
                )],
            )]),
        )];

        assert!(lsp::collect_diagnostics_for_root(&sessions, &PathBuf::from("/repo")).is_empty());
    }

    #[test]
    fn kill_playwright_session_only_signals_the_group_that_was_asked_for() {
        let sessions = vec![
            PlaywrightSessionInfo {
                pgid: 100,
                label: "Playwright CLI server".to_string(),
                pids: vec![101, 102],
                processes: Vec::new(),
            },
            PlaywrightSessionInfo {
                pgid: 200,
                label: "Playwright MCP server".to_string(),
                pids: vec![201],
                processes: Vec::new(),
            },
        ];
        let mut signals = Vec::new();

        let result = kill_playwright_session_with(
            sessions,
            200,
            |pid, signal| {
                signals.push((pid, signal.to_string()));
                Ok(())
            },
            || {},
            || Ok(Vec::new()),
        )
        .unwrap();

        // Only the requested group is touched; the other session's pids are never signalled.
        assert_eq!(signals, vec![(201, "TERM".to_string())]);
        assert_eq!(result.terminated_pgids, vec![200]);
        assert_eq!(result.terminated_pids, vec![201]);
        assert!(result.failed_pgids.is_empty());
        assert_eq!(result.sessions.len(), 1);
        assert_eq!(result.sessions[0].pgid, 200);
    }

    #[test]
    fn kill_playwright_session_escalates_to_kill_when_term_leaves_it_running() {
        let sessions = vec![PlaywrightSessionInfo {
            pgid: 100,
            label: "Playwright CLI server".to_string(),
            pids: vec![101, 102],
            processes: Vec::new(),
        }];
        let survivors = vec![PlaywrightSessionInfo {
            pgid: 100,
            label: "Playwright CLI server".to_string(),
            pids: vec![102],
            processes: Vec::new(),
        }];
        let mut signals = Vec::new();

        kill_playwright_session_with(
            sessions,
            100,
            |pid, signal| {
                signals.push((pid, signal.to_string()));
                Ok(())
            },
            || {},
            || Ok(survivors.clone()),
        )
        .unwrap();

        assert_eq!(
            signals,
            vec![
                (101, "TERM".to_string()),
                (102, "TERM".to_string()),
                (102, "KILL".to_string()),
            ]
        );
    }

    #[test]
    fn kill_playwright_session_refuses_a_group_that_is_not_a_listed_playwright_session() {
        let sessions = vec![PlaywrightSessionInfo {
            pgid: 100,
            label: "Playwright CLI server".to_string(),
            pids: vec![101],
            processes: Vec::new(),
        }];

        for unlisted in [200, 0, -1, 1] {
            let mut signals = Vec::new();
            let error = kill_playwright_session_with(
                sessions.clone(),
                unlisted,
                |pid, signal| {
                    signals.push((pid, signal.to_string()));
                    Ok(())
                },
                || {},
                || Ok(Vec::new()),
            )
            .unwrap_err();

            assert!(
                error.contains("Playwright"),
                "unexpected message for {unlisted}: {error}"
            );
            assert!(
                signals.is_empty(),
                "nothing may be signalled for {unlisted}"
            );
        }
    }

    #[test]
    fn kill_playwright_sessions_reports_post_term_rescan_failure() {
        let sessions = vec![PlaywrightSessionInfo {
            pgid: 100,
            label: "Playwright CLI server".to_string(),
            pids: vec![101, 102],
            processes: Vec::new(),
        }];
        let mut signals = Vec::new();
        let mut list_calls = 0;

        let result = kill_playwright_sessions_with(
            sessions,
            |pid, signal| {
                signals.push((pid, signal.to_string()));
                Ok(())
            },
            || {},
            || {
                list_calls += 1;
                Err("ps unavailable".to_string())
            },
        )
        .unwrap();

        assert_eq!(
            signals,
            vec![(101, "TERM".to_string()), (102, "TERM".to_string())]
        );
        assert_eq!(list_calls, 1);
        assert_eq!(result.terminated_pgids, vec![100]);
        assert_eq!(result.terminated_pids, vec![101, 102]);
        assert_eq!(result.failed_pgids.len(), 1);
        assert_eq!(result.failed_pgids[0].pgid, 100);
        assert_eq!(result.failed_pgids[0].pid, None);
        assert!(result.failed_pgids[0].message.contains("ps unavailable"));
    }

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
        std::fs::write(root.join("notes.txt"), "plain notes").unwrap();

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
        assert_eq!(scan.stats.matched_files, 6);
        assert!(scan.stats.visited_entries >= 8);
        assert!(scan.stats.skipped_directories >= 1);
        assert!(scan.stats.unsupported_files >= 1);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_root_validation_rejects_missing_paths() {
        let root = unique_temp_root();

        let validation = validate_project_root_sync(root.clone());

        assert_eq!(validation.path, root.display().to_string());
        assert!(!validation.exists);
        assert!(!validation.is_directory);
        assert!(!validation.is_git_repository);
        assert_eq!(validation.git_root, None);
        assert!(validation.message.contains("not found"));
    }

    #[test]
    fn project_root_validation_rejects_files() {
        let root = unique_temp_root();
        std::fs::create_dir_all(&root).unwrap();
        let file_path = root.join("Project.csproj");
        std::fs::write(&file_path, "<Project />").unwrap();

        let validation = validate_project_root_sync(file_path.clone());

        assert_eq!(validation.path, file_path.display().to_string());
        assert!(validation.exists);
        assert!(!validation.is_directory);
        assert!(!validation.is_git_repository);
        assert_eq!(validation.git_root, None);
        assert!(validation.message.contains("file"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_root_validation_accepts_git_worktrees() {
        let root = unique_temp_root();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".git"), "gitdir: /tmp/repo/.git/worktrees/test\n").unwrap();

        let validation = validate_project_root_sync(root.clone());

        assert_eq!(validation.path, root.display().to_string());
        assert!(validation.exists);
        assert!(validation.is_directory);
        assert!(validation.is_git_repository);
        assert_eq!(validation.git_root, Some(root.display().to_string()));
        assert!(validation.message.contains("ready"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_scan_allows_worktree_ancestor_paths() {
        let root = unique_temp_root();
        let worktree_root = root
            .join("worktrees")
            .join("EdiPlatform")
            .join("tsk-127-source-scan");
        std::fs::create_dir_all(worktree_root.join("EdiPlatform.Core/Services")).unwrap();
        std::fs::create_dir_all(worktree_root.join("EdiPlatform.Api/Controllers")).unwrap();
        std::fs::write(
            worktree_root.join(".git"),
            "gitdir: /tmp/repo/.git/worktrees/tsk-127\n",
        )
        .unwrap();
        std::fs::write(
            worktree_root.join("EdiPlatform.Core/Services/FormatDetector.cs"),
            "public sealed class FormatDetector {}",
        )
        .unwrap();
        std::fs::write(
            worktree_root.join("EdiPlatform.Api/Controllers/DashboardController.cs"),
            "public sealed class DashboardController {}",
        )
        .unwrap();

        let scan = list_source_files_sync(worktree_root, 20, None).unwrap();
        let relative_paths = scan
            .records
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            relative_paths,
            vec![
                "EdiPlatform.Core/Services/FormatDetector.cs",
                "EdiPlatform.Api/Controllers/DashboardController.cs"
            ]
        );
        assert!(!scan.truncated);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_root_validation_finds_parent_git_root() {
        let root = unique_temp_root();
        let nested = root.join("src").join("Services");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.join(".git"), "gitdir: /tmp/repo/.git/worktrees/test\n").unwrap();

        let validation = validate_project_root_sync(nested.clone());

        assert_eq!(validation.path, nested.display().to_string());
        assert!(validation.exists);
        assert!(validation.is_directory);
        assert!(!validation.is_git_repository);
        assert_eq!(validation.git_root, Some(root.display().to_string()));
        assert!(validation.message.contains("inside a Git repository"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_root_validation_warns_for_non_git_directories() {
        let root = unique_temp_root();
        std::fs::create_dir_all(&root).unwrap();

        let validation = validate_project_root_sync(root.clone());

        assert_eq!(validation.path, root.display().to_string());
        assert!(validation.exists);
        assert!(validation.is_directory);
        assert!(!validation.is_git_repository);
        assert_eq!(validation.git_root, None);
        assert!(validation.message.contains("not a Git repository"));

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
    fn source_scan_final_sort_keeps_src_before_project_named_shared_dirs_when_truncated() {
        let temp_root = unique_temp_root();
        let root = temp_root.join("Project");
        std::fs::create_dir_all(root.join("Project.Shared")).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Project.Shared/Alpha.cs"),
            "namespace Project.Shared;\npublic sealed class Alpha {}",
        )
        .unwrap();
        std::fs::write(
            root.join("Project.Shared/Beta.cs"),
            "namespace Project.Shared;\npublic sealed class Beta {}",
        )
        .unwrap();
        std::fs::write(
            root.join("src/App.cs"),
            "namespace Project;\npublic sealed class App {}",
        )
        .unwrap();

        let scan = list_source_files_sync(root, 2, None).unwrap();

        assert_eq!(
            scan.records
                .iter()
                .map(|file| file.relative_path.as_str())
                .collect::<Vec<_>>(),
            vec!["src/App.cs", "Project.Shared/Alpha.cs"]
        );
        assert!(scan.truncated);

        std::fs::remove_dir_all(temp_root).unwrap();
    }

    #[test]
    fn source_scan_low_requested_limit_still_walks_beyond_returned_files() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        std::fs::write(root.join("src/App.ts"), "export const app = true;").unwrap();
        std::fs::write(root.join("src/Worker.ts"), "export const worker = true;").unwrap();
        std::fs::write(root.join("src/Widget.ts"), "export const widget = true;").unwrap();
        std::fs::write(
            root.join("node_modules/pkg/index.ts"),
            "export const dependency = true;",
        )
        .unwrap();

        let scan = list_source_files_sync(root.clone(), 2, None).unwrap();

        assert_eq!(scan.records.len(), 2);
        assert!(scan.truncated);
        assert_eq!(scan.stats.requested_limit, 2);
        assert_eq!(scan.stats.returned_files, 2);
        assert_eq!(scan.stats.matched_files, 3);
        assert_eq!(scan.stats.collection_limit, 10_001);
        assert!(!scan.stats.collection_limit_reached);
        assert_eq!(scan.stats.skipped_directories, 1);
        assert_eq!(scan.stats.skipped_directory_samples[0].name, "node_modules");

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
    fn source_scan_reports_skipped_directory_samples() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join(".git/objects")).unwrap();
        std::fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        std::fs::create_dir_all(root.join("worktrees/session/src")).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join(".git/objects/Hidden.ts"),
            "export const hidden = true;",
        )
        .unwrap();
        std::fs::write(
            root.join("node_modules/pkg/index.ts"),
            "export const dependency = true;",
        )
        .unwrap();
        std::fs::write(
            root.join("worktrees/session/src/Stale.cs"),
            "public class Stale {}",
        )
        .unwrap();
        std::fs::write(root.join("src/Keep.ts"), "export const keep = true;").unwrap();

        let scan = list_source_files_sync(root.clone(), 20, None).unwrap();
        let skipped_names = scan
            .stats
            .skipped_directory_samples
            .iter()
            .map(|sample| sample.name.as_str())
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(scan.records.len(), 1);
        assert_eq!(scan.stats.skipped_directories, 3);
        assert_eq!(
            skipped_names,
            std::collections::BTreeSet::from([".git", "node_modules", "worktrees"])
        );
        assert!(scan
            .stats
            .skipped_directory_samples
            .iter()
            .all(|sample| !sample.reason.is_empty()));

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
        assert_eq!(detect_language(Path::new("Makefile")), "makefile");
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
        assert_eq!(scan.stats.requested_limit, 1);
        assert_eq!(scan.stats.returned_files, 1);
        assert_eq!(scan.stats.matched_files, 2);

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
        assert!(status.has_upstream);
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
        assert!(!status.has_upstream);
        assert!(status.files.is_empty());
    }

    #[test]
    fn parse_git_branch_header_reads_upstream_backed_branch() {
        let parsed = parse_git_branch_header("main...origin/main");

        assert_eq!(parsed.branch.as_deref(), Some("main"));
        assert!(parsed.has_upstream);
        assert_eq!(parsed.ahead, 0);
        assert_eq!(parsed.behind, 0);
    }

    #[test]
    fn parse_git_branch_header_reads_no_upstream_branch() {
        let parsed = parse_git_branch_header("feature/source-browser");

        assert_eq!(parsed.branch.as_deref(), Some("feature/source-browser"));
        assert!(!parsed.has_upstream);
        assert_eq!(parsed.ahead, 0);
        assert_eq!(parsed.behind, 0);
    }

    #[test]
    fn parse_git_branch_header_reads_ahead_behind_and_diverged_counts() {
        let ahead = parse_git_branch_header("feature...origin/feature [ahead 1]");
        assert_eq!(ahead.branch.as_deref(), Some("feature"));
        assert!(ahead.has_upstream);
        assert_eq!(ahead.ahead, 1);
        assert_eq!(ahead.behind, 0);

        let behind = parse_git_branch_header("feature...origin/feature [behind 2]");
        assert_eq!(behind.branch.as_deref(), Some("feature"));
        assert!(behind.has_upstream);
        assert_eq!(behind.ahead, 0);
        assert_eq!(behind.behind, 2);

        let diverged = parse_git_branch_header("feature...origin/feature [ahead 3, behind 4]");
        assert_eq!(diverged.branch.as_deref(), Some("feature"));
        assert!(diverged.has_upstream);
        assert_eq!(diverged.ahead, 3);
        assert_eq!(diverged.behind, 4);
    }

    #[test]
    fn parse_git_branch_header_does_not_infer_upstream_for_detached_or_no_commits() {
        let detached = parse_git_branch_header("HEAD (no branch)");
        assert_eq!(detached.branch.as_deref(), Some("HEAD (no branch)"));
        assert!(!detached.has_upstream);

        let no_commits = parse_git_branch_header("No commits yet on feature/source-browser");
        assert_eq!(no_commits.branch.as_deref(), Some("feature/source-browser"));
        assert!(!no_commits.has_upstream);
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
        assert!(!summary.has_upstream);
        assert!(summary.is_dirty);
        assert_eq!(summary.last_commit_subject.as_deref(), Some("initial"));
        assert!(summary
            .last_commit_sha
            .as_ref()
            .is_some_and(|sha| !sha.is_empty()));
        assert!(summary.dirty_since_epoch_ms.is_some());
        assert_ne!(
            summary.dirty_status_fingerprint,
            git_dirty_status_fingerprint(&[]),
            "dirty repositories should include a dirty status fingerprint"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn git_dirty_status_fingerprint_changes_for_path_or_status_changes() {
        let first = git_dirty_status_fingerprint(&[GitFileStatus {
            relative_path: "src/App.ts".to_string(),
            index_status: "".to_string(),
            worktree_status: "modified".to_string(),
            status: "modified".to_string(),
            badge: "M".to_string(),
        }]);
        let renamed_path = git_dirty_status_fingerprint(&[GitFileStatus {
            relative_path: "src/Renamed.ts".to_string(),
            index_status: "".to_string(),
            worktree_status: "modified".to_string(),
            status: "modified".to_string(),
            badge: "M".to_string(),
        }]);
        let staged = git_dirty_status_fingerprint(&[GitFileStatus {
            relative_path: "src/App.ts".to_string(),
            index_status: "modified".to_string(),
            worktree_status: "".to_string(),
            status: "modified".to_string(),
            badge: "M".to_string(),
        }]);

        assert_eq!(
            git_dirty_status_fingerprint(&[]),
            git_dirty_status_fingerprint(&[])
        );
        assert_ne!(first, renamed_path);
        assert_ne!(first, staged);
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
            "worktree /repo\nHEAD abc\nbranch refs/heads/main\n\nworktree /worktrees/feature\nHEAD def\nbranch refs/heads/cdx/tsk-126-feature\n\nworktree /detached\nHEAD fed\ndetached\n\nworktree /missing\nHEAD abc\nbranch refs/heads/cdx/tsk-127-missing\nprunable gitdir file points to non-existent location\n\nworktree /locked\nHEAD def\nbranch refs/heads/cdx/tsk-128-locked\nlocked agent still running\n",
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
                ("/detached", "detached", None),
                ("/missing", "cdx/tsk-127-missing", Some("TSK-127")),
                ("/locked", "cdx/tsk-128-locked", Some("TSK-128"))
            ]
        );
        assert!(records[3].is_prunable);
        assert_eq!(
            records[3].prunable_reason.as_deref(),
            Some("gitdir file points to non-existent location")
        );
        assert!(records[4].is_locked);
        assert_eq!(
            records[4].locked_reason.as_deref(),
            Some("agent still running")
        );
    }

    #[test]
    fn project_worktree_delete_eligibility_prioritizes_dirty_then_unmerged() {
        assert_eq!(
            project_worktree_delete_eligibility(true, true, true, true),
            "blocked: dirty worktree"
        );
        assert_eq!(
            project_worktree_delete_eligibility(false, true, true, true),
            "blocked: unmerged commits"
        );
        assert_eq!(
            project_worktree_delete_eligibility(false, false, false, true),
            "blocked: locked worktree"
        );
        assert_eq!(
            project_worktree_delete_eligibility(false, false, true, false),
            "review: prunable missing worktree metadata"
        );
        assert_eq!(
            project_worktree_delete_eligibility(false, false, false, false),
            "requires-confirmation"
        );
    }

    #[test]
    fn project_worktree_unmerged_detection_is_scoped_to_the_worktree_branch() {
        let root = unique_temp_root();
        let clean_sibling = unique_temp_root();
        let local_commit_sibling = unique_temp_root();
        let remote = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(&remote).unwrap();
        std::fs::write(root.join("src/App.ts"), "export const value = 1;\n").unwrap();

        run_git_for_test(&remote, &["init", "--bare"]);
        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "src/App.ts"]);
        run_git_for_test(&root, &["commit", "-m", "initial"]);
        run_git_for_test(&root, &["branch", "-M", "main"]);
        run_git_for_test(
            &root,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        run_git_for_test(&root, &["push", "-u", "origin", "main"]);
        run_git_for_test(
            &root,
            &[
                "worktree",
                "add",
                "-b",
                "cdx/tsk-200-clean",
                clean_sibling.to_str().unwrap(),
            ],
        );
        run_git_for_test(
            &root,
            &[
                "worktree",
                "add",
                "-b",
                "cdx/tsk-201-local",
                local_commit_sibling.to_str().unwrap(),
            ],
        );
        std::fs::write(
            local_commit_sibling.join("src/App.ts"),
            "export const value = 2;\n",
        )
        .unwrap();
        run_git_for_test(
            &local_commit_sibling,
            &["commit", "-am", "local worktree commit"],
        );

        let clean_has_unmerged =
            project_worktree_has_unmerged_commits(clean_sibling.to_str().unwrap());
        let local_has_unmerged =
            project_worktree_has_unmerged_commits(local_commit_sibling.to_str().unwrap());

        run_git_for_test(
            &root,
            &[
                "worktree",
                "remove",
                "--force",
                clean_sibling.to_str().unwrap(),
            ],
        );
        run_git_for_test(
            &root,
            &[
                "worktree",
                "remove",
                "--force",
                local_commit_sibling.to_str().unwrap(),
            ],
        );
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(remote).unwrap();

        assert!(
            local_has_unmerged,
            "the worktree branch with a local commit should be blocked"
        );
        assert!(
            !clean_has_unmerged,
            "a clean sibling should not inherit unmerged state from another local branch"
        );
    }

    #[test]
    fn stopping_a_process_asks_it_politely_and_says_so() {
        let mut signalled = Vec::new();
        let result = kill_process_with(4242, 99, |pid, signal| {
            signalled.push((pid, signal.to_string()));
            Ok(())
        });

        assert_eq!(signalled, vec![(4242, "TERM".to_string())]);
        assert!(result.ok);
        assert!(result.message.contains("Asked process 4242 to stop"));
    }

    #[test]
    fn stopping_the_system_or_this_app_is_refused_before_any_signal() {
        for pid in [0, 1] {
            let result = kill_process_with(pid, 99, |_, _| panic!("must not signal pid {pid}"));
            assert!(!result.ok);
            assert!(result.message.contains("system's own"));
        }

        let result = kill_process_with(99, 99, |_, _| panic!("must not signal this app"));
        assert!(!result.ok);
        assert!(result.message.contains("this app itself"));
    }

    #[test]
    fn a_process_that_could_not_be_stopped_is_reported_as_a_sentence() {
        let result = kill_process_with(4242, 99, |_, _| Err("No such process".to_string()));

        assert!(!result.ok);
        assert!(result.message.contains("Could not stop process 4242"));
        assert!(result.message.contains("already exited"));
        assert!(result.message.contains("No such process"));
    }

    #[test]
    fn a_process_number_still_running_the_expected_command_is_recognised() {
        assert!(process_commands_match("node", "node"));
        assert!(
            process_commands_match("/usr/local/bin/node", "node"),
            "the list shows a short name while ps answers with a full path"
        );
        assert!(
            process_commands_match("node", "/usr/local/bin/node"),
            "and the full path can arrive from either side"
        );
        assert!(
            process_commands_match("node22", "node"),
            "a versioned executable is still the same program"
        );
    }

    #[test]
    fn a_process_number_now_running_something_else_is_not_recognised() {
        assert!(
            !process_commands_match("node", "RentalCommand.Api"),
            "a reused process number must not be mistaken for the original"
        );
        assert!(
            !process_commands_match("/usr/bin/python3", "node"),
            "comparing file names must not match across different programs"
        );
        assert!(
            !process_commands_match("", "node"),
            "no name is never a match"
        );
        assert!(
            !process_commands_match("node", ""),
            "and neither is no expectation"
        );
        assert!(!process_commands_match("   ", "node"));
    }

    #[test]
    fn switching_the_csharp_language_server_off_says_what_was_lost() {
        let stopped = describe_csharp_language_server_toggle(false, true, 1);
        assert!(stopped.contains("freeing its memory"));
        assert!(stopped.contains("Reference counts and project search still work"));

        let nothing_running = describe_csharp_language_server_toggle(false, true, 0);
        assert!(nothing_running.contains("was not running"));

        let back_on = describe_csharp_language_server_toggle(true, true, 0);
        assert!(back_on.contains("back on"));
        assert!(describe_csharp_language_server_toggle(true, false, 0).contains("already on"));
    }

    #[test]
    fn a_worktree_record_points_at_the_folder_above_its_git_file() {
        assert_eq!(
            worktree_record_gitdir_names("/Users/reader/work/trees/tsk-1/.git\n"),
            Some("/Users/reader/work/trees/tsk-1".to_string())
        );
        assert_eq!(worktree_record_gitdir_names("   "), None);
    }

    #[test]
    fn remove_project_worktree_removes_clean_sibling_and_refreshes_list() {
        let root = unique_temp_root();
        let sibling = unique_temp_root();
        let remote = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(&remote).unwrap();
        std::fs::write(root.join("src/App.ts"), "export const value = 1;\n").unwrap();

        run_git_for_test(&remote, &["init", "--bare"]);
        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "src/App.ts"]);
        run_git_for_test(&root, &["commit", "-m", "initial"]);
        run_git_for_test(&root, &["branch", "-M", "main"]);
        run_git_for_test(
            &root,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        run_git_for_test(&root, &["push", "-u", "origin", "main"]);
        run_git_for_test(
            &root,
            &[
                "worktree",
                "add",
                "-b",
                "cdx/tsk-127-cleanup",
                sibling.to_str().unwrap(),
            ],
        );

        let result = remove_project_worktree_sync(root.clone(), sibling.clone(), false).unwrap();

        assert!(result.message.contains("Removed worktree"));
        assert!(!sibling.exists());
        assert!(!result
            .worktrees
            .iter()
            .any(|worktree| worktree.path == normalized_path_string(&sibling)));

        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(remote).unwrap();
    }

    #[test]
    fn remove_project_worktree_prunes_missing_registered_sibling() {
        let root = unique_temp_root();
        let sibling = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/App.ts"), "export const value = 1;\n").unwrap();

        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "src/App.ts"]);
        run_git_for_test(&root, &["commit", "-m", "initial"]);
        run_git_for_test(
            &root,
            &[
                "worktree",
                "add",
                "-b",
                "cdx/tsk-127-missing",
                sibling.to_str().unwrap(),
            ],
        );
        std::fs::remove_dir_all(&sibling).unwrap();

        let before = list_project_worktrees_sync(root.clone()).unwrap();
        let missing = before
            .iter()
            .find(|worktree| worktree.branch == "cdx/tsk-127-missing")
            .expect("deleted sibling should still be registered before pruning");
        assert!(missing.is_prunable);
        assert!(missing.delete_eligibility.contains("prunable"));
        let missing_path = PathBuf::from(&missing.path);

        let result = remove_project_worktree_sync(root.clone(), missing_path, false).unwrap();

        assert!(result.message.contains("Cleared git's records for"));
        assert!(result.message.contains("cdx/tsk-127-missing"));
        assert!(!result
            .worktrees
            .iter()
            .any(|worktree| worktree.branch == "cdx/tsk-127-missing"));

        std::fs::remove_dir_all(root).unwrap();
    }

    /// The incident this replaced: a reader clicked one row whose folder was
    /// gone and watched every other gone-folder row disappear with it, because
    /// the removal ran a repo-wide prune.
    #[test]
    fn remove_project_worktree_clears_only_the_asked_for_missing_sibling() {
        let root = unique_temp_root();
        let first_sibling = unique_temp_root();
        let second_sibling = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/App.ts"), "export const value = 1;\n").unwrap();

        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "src/App.ts"]);
        run_git_for_test(&root, &["commit", "-m", "initial"]);
        for (branch, sibling) in [
            ("cdx/tsk-500-first", &first_sibling),
            ("cdx/tsk-500-second", &second_sibling),
        ] {
            run_git_for_test(
                &root,
                &["worktree", "add", "-b", branch, sibling.to_str().unwrap()],
            );
            std::fs::remove_dir_all(sibling).unwrap();
        }

        let before = list_project_worktrees_sync(root.clone()).unwrap();
        let first = before
            .iter()
            .find(|worktree| worktree.branch == "cdx/tsk-500-first")
            .expect("both deleted siblings should still be registered");
        assert!(first.is_prunable);
        let first_path = PathBuf::from(&first.path);

        let result = remove_project_worktree_sync(root.clone(), first_path, false).unwrap();

        assert!(!result
            .worktrees
            .iter()
            .any(|worktree| worktree.branch == "cdx/tsk-500-first"));
        assert!(
            result
                .worktrees
                .iter()
                .any(|worktree| worktree.branch == "cdx/tsk-500-second"),
            "clearing one gone-folder row must leave the other one alone"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn remove_project_worktree_refuses_locked_sibling() {
        let root = unique_temp_root();
        let sibling = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/App.ts"), "export const value = 1;\n").unwrap();

        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "src/App.ts"]);
        run_git_for_test(&root, &["commit", "-m", "initial"]);
        run_git_for_test(
            &root,
            &[
                "worktree",
                "add",
                "-b",
                "cdx/tsk-127-locked",
                sibling.to_str().unwrap(),
            ],
        );
        run_git_for_test(
            &root,
            &[
                "worktree",
                "lock",
                "--reason",
                "agent running",
                sibling.to_str().unwrap(),
            ],
        );

        let error = remove_project_worktree_sync(root.clone(), sibling.clone(), false).unwrap_err();

        assert!(error.contains("locked worktree"));
        assert!(sibling.exists());

        run_git_for_test(&root, &["worktree", "unlock", sibling.to_str().unwrap()]);
        run_git_for_test(
            &root,
            &["worktree", "remove", "--force", sibling.to_str().unwrap()],
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn remove_project_worktree_refuses_dirty_sibling() {
        let root = unique_temp_root();
        let sibling = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/App.ts"), "export const value = 1;\n").unwrap();

        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "src/App.ts"]);
        run_git_for_test(&root, &["commit", "-m", "initial"]);
        run_git_for_test(
            &root,
            &[
                "worktree",
                "add",
                "-b",
                "cdx/tsk-127-dirty",
                sibling.to_str().unwrap(),
            ],
        );
        std::fs::write(sibling.join("dirty.txt"), "keep me\n").unwrap();

        let error = remove_project_worktree_sync(root.clone(), sibling.clone(), false).unwrap_err();

        assert!(error.contains("dirty worktree"));
        assert!(sibling.exists());

        run_git_for_test(
            &root,
            &["worktree", "remove", "--force", sibling.to_str().unwrap()],
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn forced_worktree_removal_message_says_what_was_destroyed() {
        assert_eq!(
            describe_forced_worktree_removal("cdx/tsk-127-clean", Some(0), false, false, None),
            "Force removed worktree cdx/tsk-127-clean. It had no uncommitted changes."
        );
        assert_eq!(
            describe_forced_worktree_removal("cdx/tsk-127-one", Some(1), true, false, None),
            "Force removed worktree cdx/tsk-127-one. Deleted 1 file with changes that were never committed."
        );
        assert_eq!(
            describe_forced_worktree_removal(
                "cdx/tsk-127-all",
                Some(3),
                true,
                true,
                Some("agent running")
            ),
            "Force removed worktree cdx/tsk-127-all. Deleted 3 files with changes that were never committed. Deleted commits on this branch that were never pushed to a remote. Unlocked it first; it was locked because: agent running."
        );
        assert_eq!(
            describe_forced_worktree_removal("cdx/tsk-127-locked", Some(0), false, false, Some("")),
            "Force removed worktree cdx/tsk-127-locked. It had no uncommitted changes. Unlocked it first; it was locked with no reason given."
        );
    }

    #[test]
    fn forced_worktree_removal_never_claims_nothing_was_lost_when_it_could_not_look() {
        // The count failed but the listing already knew there were changes: say that,
        // rather than reporting a zero the count never actually established.
        assert_eq!(
            describe_forced_worktree_removal("cdx/tsk-127-blind", None, true, false, None),
            "Force removed worktree cdx/tsk-127-blind. Deleted files with changes that were never committed; they could not be counted before the removal."
        );
        // The count disagrees with what the listing said. Same answer: do not claim zero.
        assert_eq!(
            describe_forced_worktree_removal("cdx/tsk-127-disagree", Some(0), true, false, None),
            "Force removed worktree cdx/tsk-127-disagree. Deleted files with changes that were never committed; they could not be counted before the removal."
        );
        // Neither check could look. Do not promise the worktree was clean.
        assert_eq!(
            describe_forced_worktree_removal("cdx/tsk-127-unknown", None, false, false, None),
            "Force removed worktree cdx/tsk-127-unknown. Could not check for uncommitted changes before the removal, so anything not committed is gone."
        );
    }

    #[test]
    fn worktree_dirty_file_count_is_unknown_when_git_cannot_answer() {
        let missing = unique_temp_root();

        assert_eq!(
            project_worktree_dirty_file_count(missing.to_str().unwrap()),
            None,
            "a path git cannot read reports unknown, not a confident zero"
        );

        let root = unique_temp_root();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("README.md"), "initial\n").unwrap();
        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "README.md"]);
        run_git_for_test(&root, &["commit", "-m", "initial"]);

        assert_eq!(
            project_worktree_dirty_file_count(root.to_str().unwrap()),
            Some(0)
        );

        std::fs::write(root.join("untracked.txt"), "new\n").unwrap();
        std::fs::write(root.join("README.md"), "changed\n").unwrap();

        assert_eq!(
            project_worktree_dirty_file_count(root.to_str().unwrap()),
            Some(2)
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn force_remove_project_worktree_deletes_a_dirty_sibling_and_says_so() {
        let root = unique_temp_root();
        let sibling = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/App.ts"), "export const value = 1;\n").unwrap();

        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "src/App.ts"]);
        run_git_for_test(&root, &["commit", "-m", "initial"]);
        run_git_for_test(
            &root,
            &[
                "worktree",
                "add",
                "-b",
                "cdx/tsk-127-force-dirty",
                sibling.to_str().unwrap(),
            ],
        );
        std::fs::write(sibling.join("dirty.txt"), "never committed\n").unwrap();
        std::fs::write(sibling.join("src/App.ts"), "export const value = 2;\n").unwrap();

        let result = remove_project_worktree_sync(root.clone(), sibling.clone(), true).unwrap();

        assert_eq!(
            result.message,
            // This fixture repo has no remote, so the branch's commit counts as never
            // pushed and the message says so alongside the uncommitted file count.
            "Force removed worktree cdx/tsk-127-force-dirty. Deleted 2 files with changes that were never committed. Deleted commits on this branch that were never pushed to a remote."
        );
        assert!(!sibling.exists());
        assert!(!result
            .worktrees
            .iter()
            .any(|worktree| worktree.branch == "cdx/tsk-127-force-dirty"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn force_remove_project_worktree_unlocks_a_locked_sibling_first() {
        let root = unique_temp_root();
        let sibling = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/App.ts"), "export const value = 1;\n").unwrap();

        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "src/App.ts"]);
        run_git_for_test(&root, &["commit", "-m", "initial"]);
        run_git_for_test(
            &root,
            &[
                "worktree",
                "add",
                "-b",
                "cdx/tsk-127-force-locked",
                sibling.to_str().unwrap(),
            ],
        );
        run_git_for_test(
            &root,
            &[
                "worktree",
                "lock",
                "--reason",
                "agent running",
                sibling.to_str().unwrap(),
            ],
        );

        let result = remove_project_worktree_sync(root.clone(), sibling.clone(), true).unwrap();

        assert!(
            result
                .message
                .contains("Unlocked it first; it was locked because: agent running."),
            "unexpected message: {}",
            result.message
        );
        assert!(!sibling.exists());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn force_remove_project_worktree_still_refuses_the_primary_checkout() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/App.ts"), "export const value = 1;\n").unwrap();

        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "src/App.ts"]);
        run_git_for_test(&root, &["commit", "-m", "initial"]);

        let error = remove_project_worktree_sync(root.clone(), root.clone(), true).unwrap_err();

        assert!(
            error.contains("primary checkout"),
            "unexpected message: {error}"
        );
        assert!(root.join("src/App.ts").exists());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn force_remove_project_worktree_refuses_the_primary_checkout_reached_through_a_sibling() {
        let root = unique_temp_root();
        let sibling = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/App.ts"), "export const value = 1;\n").unwrap();

        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "src/App.ts"]);
        run_git_for_test(&root, &["commit", "-m", "initial"]);
        run_git_for_test(
            &root,
            &[
                "worktree",
                "add",
                "-b",
                "cdx/tsk-127-sneak",
                sibling.to_str().unwrap(),
            ],
        );

        // Ask through the LINKED worktree for the primary checkout. The two paths differ,
        // so comparing them to each other proves nothing; the answer has to come from the
        // repository's own list of worktrees, whose first entry is the main working tree.
        let error = remove_project_worktree_sync(sibling.clone(), root.clone(), true).unwrap_err();

        assert!(
            error.contains("primary checkout"),
            "unexpected message: {error}"
        );
        assert!(root.join("src/App.ts").exists());
        assert!(sibling.exists());

        // The same refusal without force, so neither path depends on the other.
        let error = remove_project_worktree_sync(sibling.clone(), root.clone(), false).unwrap_err();
        assert!(
            error.contains("primary checkout"),
            "unexpected message: {error}"
        );

        run_git_for_test(
            &root,
            &["worktree", "remove", "--force", sibling.to_str().unwrap()],
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn archive_project_worktree_preserves_dirty_state_without_removing_it() {
        let root = unique_temp_root();
        let sibling = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/App.ts"), "export const value = 1;\n").unwrap();

        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "src/App.ts"]);
        run_git_for_test(&root, &["commit", "-m", "initial"]);
        run_git_for_test(
            &root,
            &[
                "worktree",
                "add",
                "-b",
                "cdx/tsk-127-dirty-backup",
                sibling.to_str().unwrap(),
            ],
        );
        std::fs::write(sibling.join("src/App.ts"), "export const value = 2;\n").unwrap();
        std::fs::create_dir_all(sibling.join("notes")).unwrap();
        std::fs::write(sibling.join("notes/handoff.md"), "keep this\n").unwrap();

        let result = archive_project_worktree_sync(root.clone(), sibling.clone()).unwrap();

        let archive_path = PathBuf::from(result.archive_path);
        assert!(archive_path.join("status.txt").exists());
        assert!(archive_path.join("commits.txt").exists());
        assert!(archive_path.join("unstaged.patch").exists());
        assert!(archive_path.join("staged.patch").exists());
        assert!(archive_path.join("untracked.txt").exists());
        assert!(archive_path.join("head.bundle").exists());
        assert!(archive_path.join("untracked/notes/handoff.md").exists());
        assert!(std::fs::read_to_string(archive_path.join("unstaged.patch"))
            .unwrap()
            .contains("value = 2"));
        assert_eq!(
            std::fs::read_to_string(archive_path.join("untracked/notes/handoff.md")).unwrap(),
            "keep this\n"
        );
        assert!(sibling.exists());
        assert!(project_worktree_is_dirty(sibling.to_str().unwrap()));
        assert!(result.message.contains("Archived worktree"));

        run_git_for_test(
            &root,
            &["worktree", "remove", "--force", sibling.to_str().unwrap()],
        );
        std::fs::remove_dir_all(root).unwrap();
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

    #[test]
    fn reference_counts_answer_every_symbol_from_one_walk() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src/FormatResolver.cs"),
            [
                "public sealed class FormatResolver",
                "{",
                "    private readonly FormatDetector _detector;",
                // Two mentions on one line still count as one, matching the
                // reference list this count stands in for.
                "    private readonly FormatDetector _other = new FormatDetector();",
                "    private readonly FormatDetectorFactory _factory;",
                "}",
            ]
            .join("\n"),
        )
        .unwrap();
        std::fs::write(
            root.join("src/FormatDetector.cs"),
            ["public sealed class FormatDetector", "{", "}"].join("\n"),
        )
        .unwrap();
        // Skipped by the walk, so its mentions must not be counted.
        std::fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        std::fs::write(
            root.join("node_modules/pkg/index.ts"),
            "export const FormatDetector = 1;",
        )
        .unwrap();

        let result = count_source_references_sync(
            root.clone(),
            vec![
                "FormatDetector".to_string(),
                "FormatResolver".to_string(),
                "MissingSymbol".to_string(),
            ],
            Some(5_000),
        )
        .unwrap();

        assert!(!result.approximate);
        assert_eq!(result.scanned_files, 2);
        assert_eq!(result.counts.get("FormatDetector"), Some(&3));
        assert_eq!(result.counts.get("FormatResolver"), Some(&1));
        assert_eq!(result.counts.get("MissingSymbol"), Some(&0));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reference_counts_are_case_sensitive() {
        let root = unique_temp_root();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("Sample.cs"),
            ["var formatdetector = 1;", "var FormatDetector = 2;"].join("\n"),
        )
        .unwrap();

        let result = count_source_references_sync(
            root.clone(),
            vec!["FormatDetector".to_string()],
            Some(5_000),
        )
        .unwrap();

        assert_eq!(result.counts.get("FormatDetector"), Some(&1));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reference_counts_stop_at_the_deadline_and_say_so() {
        let root = unique_temp_root();
        std::fs::create_dir_all(&root).unwrap();
        for index in 0..120 {
            std::fs::write(
                root.join(format!("File{index}.cs")),
                "var formatDetector = new FormatDetector();\n".repeat(400),
            )
            .unwrap();
        }

        let result =
            count_source_references_sync(root.clone(), vec!["FormatDetector".to_string()], Some(1))
                .unwrap();

        assert!(result.approximate);
        assert!(result.scanned_files < 120);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reference_counts_ignore_blank_and_repeated_symbols() {
        let root = unique_temp_root();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("Sample.cs"), "var detector = new Detector();").unwrap();

        let result = count_source_references_sync(
            root.clone(),
            vec![
                "  Detector  ".to_string(),
                "Detector".to_string(),
                "   ".to_string(),
            ],
            Some(5_000),
        )
        .unwrap();

        assert_eq!(result.counts.len(), 1);
        assert_eq!(result.counts.get("Detector"), Some(&1));

        std::fs::remove_dir_all(root).unwrap();
    }

    /// Not part of the normal suite: it reads a real, large checkout. Run it by
    /// hand when the margin-count budget is in question:
    ///
    /// ```text
    /// cargo test --release --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml \
    ///   reference_count_pass_over_a_large_project -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn reference_count_pass_over_a_large_project() {
        let root = PathBuf::from(
            std::env::var("MCB_REFERENCE_COUNT_BENCH_ROOT")
                .unwrap_or_else(|_| "/Users/blackcolours/dev/work/EdiPlatform".to_string()),
        );
        if !root.is_dir() {
            eprintln!("skipping: {} is not a directory", root.display());
            return;
        }

        let symbol_names = (0..120)
            .map(|index| format!("BenchmarkSymbol{index}"))
            .collect::<Vec<_>>();

        {
            let started = Instant::now();
            let mut records = Vec::new();
            let _ = collect_source_files(
                &root,
                &root,
                source_collection_limit(DEFAULT_SOURCE_LIST_LIMIT),
                None,
                &mut records,
                &SourceScanCancellation::none(),
                &mut SourceScanWalkProgress::default(),
            );
            eprintln!(
                "walk only: {} files in {} ms",
                records.len(),
                started.elapsed().as_millis()
            );

            let started = Instant::now();
            let mut bytes_read = 0usize;
            for record in &records {
                if record.byte_count > MAX_PREVIEW_BYTES {
                    continue;
                }
                if let Ok(bytes) = std::fs::read(&record.path) {
                    bytes_read += bytes.len();
                }
            }
            eprintln!(
                "read only: {} MB in {} ms",
                bytes_read / 1_000_000,
                started.elapsed().as_millis()
            );
        }

        // First pass warms the file cache; the second is the number that matters.
        for attempt in 1..=2 {
            let started = Instant::now();
            let result =
                count_source_references_sync(root.clone(), symbol_names.clone(), Some(600_000))
                    .unwrap();
            eprintln!(
                "pass {attempt}: {} files in {} ms (approximate: {})",
                result.scanned_files,
                started.elapsed().as_millis(),
                result.approximate
            );
        }
    }

    #[test]
    fn backend_capabilities_name_every_addition_the_frontend_cannot_otherwise_detect() {
        // Pinned on purpose. Each name is a promise the frontend checks before it offers a
        // feature, so quietly renaming one turns that feature off in the app instead of
        // failing loudly.
        assert_eq!(
            backend_capabilities(),
            vec![
                "worktreeForceRemove".to_string(),
                "playwrightSessionKill".to_string(),
                "lspDiagnosticsForRoot".to_string(),
                "terminalCommandSpawn".to_string(),
                "referenceCounts".to_string(),
                "processKill".to_string(),
                "worktreePruneSingle".to_string(),
                "csharpLanguageServerToggle".to_string(),
                "lspDocumentSymbols".to_string(),
                "lspStatusEvents".to_string(),
                "lspLog".to_string(),
                "resourceSnapshot".to_string(),
                "resourceDiskScan".to_string(),
                "resourceDiskCleanup".to_string(),
                "resourceStopOwned".to_string(),
                "providerUsageQuota".to_string(),
                "usageHistory".to_string(),
                "usageHistoryIncremental".to_string(),
                "usageProviderSummary".to_string(),
                "usageDailyTotals".to_string(),
                "generate_commit_message".to_string(),
                "read_pull_request_context".to_string(),
                "generate_pull_request_details".to_string(),
                "create_pull_request".to_string(),
                "read_pull_request_status".to_string(),
                "acpLiveConversationEvents".to_string(),
            ]
        );
    }

    #[test]
    fn git_commit_file_changes_parse_name_status_lines_with_status_badges() {
        let output =
            "M\tsrc/App.ts\nA\tsrc/New.ts\nD\tsrc/Old.ts\nR100\tsrc/Old.ts\tsrc/Renamed.ts\n";

        let changes = parse_git_commit_file_changes(output);

        assert_eq!(changes.len(), 4);
        assert_eq!(changes[0].relative_path, "src/App.ts");
        assert_eq!(changes[0].status, "modified");
        assert_eq!(changes[0].badge, "M");
        assert_eq!(changes[1].relative_path, "src/New.ts");
        assert_eq!(changes[1].status, "added");
        assert_eq!(changes[1].badge, "A");
        assert_eq!(changes[2].relative_path, "src/Old.ts");
        assert_eq!(changes[2].status, "deleted");
        assert_eq!(changes[2].badge, "D");
        // A rename reports the path the file ended up at, the way the status panel does.
        assert_eq!(changes[3].relative_path, "src/Renamed.ts");
        assert_eq!(changes[3].status, "renamed");
        assert_eq!(changes[3].badge, "R");
    }

    #[test]
    fn git_commit_file_reads_refuse_option_shaped_commit_ids_and_absolute_paths() {
        let root = unique_temp_root();
        std::fs::create_dir_all(&root).unwrap();

        let error =
            read_git_commit_files_sync(root.clone(), "--upload-pack=evil".to_string()).unwrap_err();
        assert!(error.contains("commit id"), "unexpected message: {error}");

        let error = read_git_commit_file_diff_sync(
            root.clone(),
            "HEAD".to_string(),
            "/etc/passwd".to_string(),
        )
        .unwrap_err();
        assert!(
            error.contains("relative repo paths"),
            "unexpected message: {error}"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn git_commit_files_and_diff_read_one_commit() {
        let root = unique_temp_root();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/App.ts"), "export const value = 1;\n").unwrap();
        std::fs::write(root.join("README.md"), "initial\n").unwrap();

        run_git_for_test(&root, &["init"]);
        run_git_for_test(&root, &["config", "user.name", "MacCommandBar Test"]);
        run_git_for_test(&root, &["config", "user.email", "test@example.invalid"]);
        run_git_for_test(&root, &["add", "."]);
        run_git_for_test(&root, &["commit", "-m", "initial"]);

        std::fs::write(root.join("src/App.ts"), "export const value = 2;\n").unwrap();
        std::fs::write(root.join("src/Added.ts"), "export const added = true;\n").unwrap();
        std::fs::remove_file(root.join("README.md")).unwrap();
        run_git_for_test(&root, &["add", "-A"]);
        run_git_for_test(&root, &["commit", "-m", "second"]);

        let history = read_git_commit_history_sync(root.clone(), Some(1)).unwrap();
        let sha = history[0].sha.clone();

        let mut files = read_git_commit_files_sync(root.clone(), sha.clone()).unwrap();
        files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

        assert_eq!(files.len(), 3);
        assert_eq!(files[0].relative_path, "README.md");
        assert_eq!(files[0].status, "deleted");
        assert_eq!(files[0].badge, "D");
        assert_eq!(files[1].relative_path, "src/Added.ts");
        assert_eq!(files[1].status, "added");
        assert_eq!(files[1].badge, "A");
        assert_eq!(files[2].relative_path, "src/App.ts");
        assert_eq!(files[2].status, "modified");
        assert_eq!(files[2].badge, "M");

        let diff =
            read_git_commit_file_diff_sync(root.clone(), sha, "src/App.ts".to_string()).unwrap();

        assert_eq!(diff.relative_path, "src/App.ts");
        assert_eq!(diff.status, "modified");
        assert!(!diff.is_binary);
        assert!(diff.diff.contains("--- a/src/App.ts"));
        assert!(diff.diff.contains("+export const value = 2;"));
        assert!(diff.diff.contains("-export const value = 1;"));
        // The commit subject line never leaks into the patch text, so the same unified
        // diff parser the working-copy diff uses can read this one.
        assert!(!diff.diff.contains("second"));

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
