use mcb_core::scanners::disk::{
    disk_protection_allows_cleanup, scan_disk_roots, stable_entry_id, DiskProtection,
    DiskScanOptions, DiskScanRoot, DiskScanReport, WorkspaceDiskKind,
};
use mcb_core::scanners::resources::{
    scan_resource_snapshot, validate_resource_action, ProcessOwner, ResourceActionError,
    ResourceOwnerHint, ResourceSnapshot,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Default)]
pub struct ResourceRegistry {
    generation: AtomicU64,
    active_source_root: Arc<Mutex<Option<PathBuf>>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDiskRootRequest {
    pub repository_id: String,
    pub workspace_id: String,
    pub path: String,
    pub kind: WorkspaceDiskKind,
    pub protection: DiskProtection,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceStopRequest {
    pub pid: u32,
    pub expected_pgid: Option<u32>,
    pub expected_generation: u64,
    pub owner_id: Option<String>,
    pub expected_root: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRootRequest {
    pub root: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCommandReceipt {
    pub action: String,
    pub pid: u32,
    pub pgid: u32,
    pub owner: ProcessOwner,
    pub owner_id: Option<String>,
    pub registry_generation: u64,
    pub signal: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUnavailable {
    pub available: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCleanupRequest {
    pub repository_id: String,
    pub workspace_id: String,
    pub repository_root: Option<String>,
    pub path: String,
    pub kind: WorkspaceDiskKind,
    pub protection: DiskProtection,
    pub expected_id: String,
    pub expected_bytes: u64,
    pub max_depth: Option<usize>,
    pub max_entries: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCleanupReceipt {
    pub action: String,
    pub entry_id: String,
    pub path: String,
    pub protection: DiskProtection,
    pub before_bytes: u64,
    pub after_bytes: u64,
    pub reclaimed_bytes: u64,
    pub message: String,
}

pub fn next_generation(registry: &ResourceRegistry) -> u64 {
    registry.generation.fetch_add(1, Ordering::Relaxed) + 1
}

#[tauri::command]
pub fn read_resource_snapshot(
    registry: State<'_, ResourceRegistry>,
    terminal_registry: State<'_, crate::terminal::TerminalRegistry>,
    agent_runtime: State<'_, crate::agent_conversation::manager::AgentRuntimeManager>,
) -> Result<ResourceSnapshot, String> {
    let generation = next_generation(&registry);
    read_resource_snapshot_at_generation(generation, &terminal_registry, &agent_runtime)
}

fn read_resource_snapshot_at_generation(
    generation: u64,
    terminal_registry: &crate::terminal::TerminalRegistry,
    agent_runtime: &crate::agent_conversation::manager::AgentRuntimeManager,
) -> Result<ResourceSnapshot, String> {
    let sessions = crate::terminal::list_terminal_sessions(terminal_registry)?;
    let mut hints = sessions
        .into_iter()
        .filter_map(|session| {
            let pid = session.pid?;
            let session_name = resource_terminal_name(&session);
            let cwd = session.cwd.clone();
            let owner_id = session
                .owned_id
                .clone()
                .or_else(|| session.tool_terminal_identity.as_ref().map(|value| value.owned_id.clone()))
                .or_else(|| Some(session.session_id.clone()));
            let (project_id, workspace_id) = resource_path_labels(&cwd);
            Some(ResourceOwnerHint {
                pid,
                pgid: None,
                owner: ProcessOwner::OwnedSession,
                owner_id,
                root: Some(cwd),
                project_id,
                workspace_id,
                session_name: Some(session_name),
                registry_generation: generation,
                can_stop: !session.exited,
            })
        })
        .collect::<Vec<_>>();

    // Structured agents do not own a PTY, but their ACP transport has the same
    // app-owned root process boundary. Include those roots beside terminal
    // sessions so the scanner can walk both kinds of trees.
    hints.extend(agent_runtime.resource_roots().into_iter().map(|agent| {
        let (project_id, workspace_id) = resource_path_labels(&agent.cwd);
        ResourceOwnerHint {
            pid: agent.pid,
            pgid: Some(agent.pid),
            owner: ProcessOwner::OwnedSession,
            owner_id: Some(agent.owned_id.clone()),
            root: Some(agent.cwd.clone()),
            project_id,
            workspace_id,
            session_name: Some(format!("{} agent", resource_provider_label(agent.provider))),
            registry_generation: generation,
            can_stop: true,
        }
    }));

    scan_resource_snapshot(&hints, generation)
}

fn resource_terminal_name(session: &crate::terminal::TerminalSessionInfo) -> String {
    match session.kind {
        crate::terminal::TerminalKind::AgentTool => session
            .tool_terminal_identity
            .as_ref()
            .map(|identity| format!("Agent tool {}", identity.tool_call_id))
            .unwrap_or_else(|| "Agent tool".to_string()),
        crate::terminal::TerminalKind::RunConfiguration => "Run configuration".to_string(),
        crate::terminal::TerminalKind::BrowserAutomation => "Browser automation".to_string(),
        crate::terminal::TerminalKind::UserPty => "Terminal".to_string(),
    }
}

fn resource_provider_label(provider: crate::agent_conversation::protocol::AgentConversationProvider) -> &'static str {
    match provider {
        crate::agent_conversation::protocol::AgentConversationProvider::Codex => "Codex",
        crate::agent_conversation::protocol::AgentConversationProvider::Claude => "Provider",
    }
}

fn resource_path_labels(cwd: &str) -> (Option<String>, Option<String>) {
    let path = PathBuf::from(cwd);
    let workspace = path.file_name().map(|value| value.to_string_lossy().to_string());
    let project = path
        .parent()
        .and_then(|parent| parent.file_name())
        .map(|value| value.to_string_lossy().to_string());
    (project, workspace)
}

#[tauri::command]
pub fn read_resource_disk_scan(
    roots: Vec<ResourceDiskRootRequest>,
    max_depth: Option<usize>,
    max_entries: Option<usize>,
) -> Result<DiskScanReport, String> {
    let roots = roots
        .into_iter()
        .map(|root| {
            let path = PathBuf::from(root.path);
            validate_disk_scan_root(&path)?;
            Ok(DiskScanRoot {
                repository_id: root.repository_id,
                workspace_id: root.workspace_id,
                path,
                kind: root.kind,
                protection: root.protection,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    scan_disk_roots(
        &roots,
        DiskScanOptions {
            max_depth: max_depth.unwrap_or(3),
            max_entries: max_entries.unwrap_or(2_000),
        },
    )
}

#[tauri::command]
pub fn cleanup_workspace_disk_entry(
    request: ResourceCleanupRequest,
) -> Result<ResourceCleanupReceipt, String> {
    if !disk_protection_allows_cleanup(request.protection) {
        return Err("Only SafeCandidate disk entries can be cleaned".to_string());
    }
    if !matches!(
        request.kind,
        WorkspaceDiskKind::Worktree
            | WorkspaceDiskKind::BuildOutput
            | WorkspaceDiskKind::DependencyCache
            | WorkspaceDiskKind::AgentData
    ) {
        return Err("This disk entry kind is not eligible for cleanup".to_string());
    }

    let path = PathBuf::from(&request.path);
    validate_cleanup_path(&path)?;
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("Cleanup entry is unavailable: {error}"))?;
    let entry_id = stable_entry_id(&request.repository_id, &request.workspace_id, &canonical);
    if entry_id != request.expected_id {
        return Err("The disk entry identity changed; scan again before cleaning".to_string());
    }
    let before = scan_disk_entry(&request, &canonical)?;
    if before != request.expected_bytes {
        return Err("The disk entry size changed; scan again before cleaning".to_string());
    }

    let message = if request.kind == WorkspaceDiskKind::Worktree {
        let repository_root = request
            .repository_root
            .as_deref()
            .map(PathBuf::from)
            .or_else(|| derive_repository_root(&canonical))
            .ok_or_else(|| "A worktree cleanup needs a discoverable repository root".to_string())?;
        let _result = crate::remove_project_worktree_sync(
            repository_root,
            canonical.clone(),
            false,
        )?;
        "Removed the clean worktree through the existing Git safety route".to_string()
    } else {
        if fs::symlink_metadata(&path)
            .map_err(|error| format!("Cleanup entry is unavailable: {error}"))?
            .file_type()
            .is_symlink()
        {
            return Err("Symlink cleanup is refused".to_string());
        }
        fs::remove_dir_all(&canonical)
            .map_err(|error| format!("Could not remove the safe cleanup entry: {error}"))?;
        "Removed the explicit safe cleanup entry after revalidation".to_string()
    };

    let after = if canonical.exists() {
        scan_disk_entry(&request, &canonical)?
    } else {
        0
    };

    Ok(ResourceCleanupReceipt {
        action: "cleanup-workspace-disk-entry".to_string(),
        entry_id: request.expected_id,
        path: canonical.display().to_string(),
        protection: request.protection,
        before_bytes: before,
        after_bytes: after,
        reclaimed_bytes: before.saturating_sub(after),
        message,
    })
}

fn derive_repository_root(worktree: &PathBuf) -> Option<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["-C", worktree.to_str()?, "rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let common_dir = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    common_dir.parent().map(PathBuf::from)
}

#[tauri::command]
pub fn stop_owned_resource(
    request: ResourceStopRequest,
    registry: State<'_, ResourceRegistry>,
    terminal_registry: State<'_, crate::terminal::TerminalRegistry>,
    agent_runtime: State<'_, crate::agent_conversation::manager::AgentRuntimeManager>,
) -> Result<ResourceCommandReceipt, String> {
    if registry.generation.load(Ordering::Acquire) != request.expected_generation {
        return Err(resource_action_error_message(ResourceActionError::StaleSnapshot));
    }
    let snapshot = read_resource_snapshot_at_generation(request.expected_generation, &terminal_registry, &agent_runtime)?;
    if registry.generation.load(Ordering::Acquire) != request.expected_generation {
        return Err(resource_action_error_message(ResourceActionError::StaleSnapshot));
    }
    let process = snapshot
        .processes
        .iter()
        .find(|process| process.pid == request.pid)
        .ok_or_else(|| "The process is no longer in the resource snapshot".to_string())?;
    validate_resource_action(process, request.expected_generation, request.expected_pgid)
        .map_err(resource_action_error_message)?;
    if request.owner_id.as_deref() != process.owner_id.as_deref() {
        return Err("The resource owner changed; refresh before stopping it".to_string());
    }
    if request.expected_root.as_deref() != process.root.as_deref() {
        return Err("The resource root changed; refresh before stopping it".to_string());
    }
    if process.pgid == 0 {
        return Err("The resource has no safe process group".to_string());
    }

    // Stop the proven process itself. The process group is still rechecked and
    // recorded, but a group-wide signal could include an unowned sibling.
    let result = unsafe { libc::kill(process.pid as i32, libc::SIGTERM) };
    if result != 0 {
        return Err(format!("Could not stop owned resource: {}", std::io::Error::last_os_error()));
    }
    Ok(ResourceCommandReceipt {
        action: "stop-owned-resource".to_string(),
        pid: process.pid,
        pgid: process.pgid,
        owner: process.owner,
        owner_id: process.owner_id.clone(),
        registry_generation: snapshot.generation,
        signal: "SIGTERM".to_string(),
    })
}

#[tauri::command]
pub fn restart_language_server_root(_request: ResourceRootRequest) -> Result<ResourceUnavailable, String> {
    Err("Language-server restart belongs to the isolated lifecycle lane".to_string())
}

#[tauri::command]
pub fn set_active_source_root(
    request: ResourceRootRequest,
    registry: State<'_, ResourceRegistry>,
) -> Result<String, String> {
    let path = PathBuf::from(request.root);
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("Source root is unavailable: {error}"))?;
    if !canonical.is_dir() {
        return Err("Source root is not a directory".to_string());
    }
    let mut active = registry
        .active_source_root
        .lock()
        .map_err(|_| "Resource registry is unavailable".to_string())?;
    *active = Some(canonical.clone());
    Ok(canonical.display().to_string())
}

#[tauri::command]
pub fn apply_resource_memory_pressure(_level: String) -> Result<ResourceUnavailable, String> {
    Err("Memory-pressure actions are not available until the lifecycle lane is rebuilt".to_string())
}

#[tauri::command]
pub fn read_language_server_log(_request: ResourceRootRequest) -> Result<ResourceUnavailable, String> {
    Err("Language-server logs belong to the isolated lifecycle lane".to_string())
}

pub fn can_remove_disk_entry(protection: DiskProtection) -> bool {
    disk_protection_allows_cleanup(protection)
}

fn resource_action_error_message(error: ResourceActionError) -> String {
    match error {
        ResourceActionError::External => "External resources cannot be stopped".to_string(),
        ResourceActionError::StaleSnapshot => "The resource snapshot is stale; refresh first".to_string(),
        ResourceActionError::ProcessGroupChanged => "The process group changed; refresh first".to_string(),
    }
}

fn validate_disk_scan_root(path: &PathBuf) -> Result<(), String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("Disk scan root is unavailable: {error}"))?;
    if canonical.parent().is_none() || canonical == PathBuf::from("/") {
        return Err("The filesystem root is not an explicit workspace root".to_string());
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from).and_then(|path| path.canonicalize().ok()) {
        if canonical == home {
            return Err("The home folder is not an explicit workspace root".to_string());
        }
    }
    Ok(())
}

fn validate_cleanup_path(path: &PathBuf) -> Result<(), String> {
    if path == &PathBuf::from("/") {
        return Err("The filesystem root cannot be cleaned".to_string());
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Cleanup entry is unavailable: {error}"))?;
    if !metadata.is_dir() {
        return Err("Cleanup entry is not a directory".to_string());
    }
    if metadata.file_type().is_symlink() {
        return Err("Symlink cleanup is refused".to_string());
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("Cleanup entry is unavailable: {error}"))?;
    if canonical.parent().is_none() {
        return Err("The filesystem root cannot be cleaned".to_string());
    }
    if let Some(home) = std::env::var_os("HOME")
        .map(PathBuf::from)
        .and_then(|value| value.canonicalize().ok())
    {
        if canonical == home {
            return Err("The home folder cannot be cleaned".to_string());
        }
    }
    Ok(())
}

fn scan_disk_entry(
    request: &ResourceCleanupRequest,
    canonical: &PathBuf,
) -> Result<u64, String> {
    let report = scan_disk_roots(
        &[DiskScanRoot {
            repository_id: request.repository_id.clone(),
            workspace_id: request.workspace_id.clone(),
            path: canonical.clone(),
            kind: request.kind,
            protection: request.protection,
        }],
        DiskScanOptions {
            max_depth: request.max_depth.unwrap_or(3),
            max_entries: request.max_entries.unwrap_or(2_000),
        },
    )?;
    report
        .entries
        .first()
        .map(|entry| entry.bytes)
        .ok_or_else(|| "Cleanup entry disappeared during revalidation".to_string())
}
