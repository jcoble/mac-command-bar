use mcb_core::scanners::disk::{
    disk_protection_allows_cleanup, scan_disk_roots, stable_entry_id, DiskProtection,
    DiskScanOptions, DiskScanReport, DiskScanRoot, WorkspaceDiskKind,
};
use mcb_core::scanners::resources::{
    scan_resource_snapshot, validate_resource_action, ProcessOwner, ResourceActionError,
    ResourceOwnerHint, ResourceSnapshot,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{ProcessesToUpdate, System};
use tauri::State;

pub struct ResourceRegistry {
    generation: AtomicU64,
    active_source_root: Arc<Mutex<Option<PathBuf>>>,
    sample_system: Arc<Mutex<System>>,
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self {
            generation: AtomicU64::default(),
            active_source_root: Arc::new(Mutex::new(None)),
            sample_system: Arc::new(Mutex::new(System::new())),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSample {
    pub generated_at_ms: u128,
    pub totals: ResourceSampleTotals,
    pub app: ResourceSampleApp,
    pub groups: Vec<ResourceSampleGroup>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSampleTotals {
    pub cpu_percent: f32,
    pub rss_bytes: u64,
    pub process_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSampleApp {
    pub parts: Vec<ResourceSampleAppPart>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSampleAppPart {
    pub label: String,
    pub pid: u32,
    pub cpu_percent: f32,
    pub rss_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSampleGroup {
    pub workspace: String,
    pub sessions: Vec<ResourceSampleSession>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSampleSession {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned_id: Option<String>,
    pub label: String,
    pub kind: ResourceSampleSessionKind,
    pub processes: Vec<ResourceSampleProcess>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceSampleSessionKind {
    Terminal,
    Conversation,
    Other,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSampleProcess {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub rss_bytes: u64,
}

#[derive(Debug, Clone)]
struct ObservedProcess {
    pid: u32,
    parent_pid: Option<u32>,
    name: String,
    cpu_percent: f32,
    rss_bytes: u64,
}

#[derive(Debug, Clone)]
struct ResourceSampleOwner {
    root_pid: u32,
    owned_id: Option<String>,
    label: String,
    kind: ResourceSampleSessionKind,
    workspace: String,
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
                .or_else(|| {
                    session
                        .tool_terminal_identity
                        .as_ref()
                        .map(|value| value.owned_id.clone())
                })
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

/// Read the live footprint without blocking Tauri's command thread. The
/// `System` value is intentionally reused: process CPU usage is a delta, so a
/// fresh instance on every call would report zeros instead of a live sample.
#[tauri::command]
pub async fn read_resource_sample(
    registry: State<'_, ResourceRegistry>,
    terminal_registry: State<'_, crate::terminal::TerminalRegistry>,
    agent_runtime: State<'_, crate::agent_conversation::manager::AgentRuntimeManager>,
) -> Result<ResourceSample, String> {
    let system = Arc::clone(&registry.sample_system);
    let terminal_registry = terminal_registry.inner().clone();
    let agent_runtime = agent_runtime.inner().clone();

    tauri::async_runtime::spawn_blocking(move || {
        let owners = resource_sample_owners(&terminal_registry, &agent_runtime)?;
        let mut system = system
            .lock()
            .map_err(|_| "Resource sampler is unavailable".to_string())?;
        system.refresh_processes(ProcessesToUpdate::All, true);
        let processes = system
            .processes()
            .values()
            .map(|process| ObservedProcess {
                pid: process.pid().as_u32(),
                parent_pid: process.parent().map(|pid| pid.as_u32()),
                name: process.name().to_string_lossy().into_owned(),
                cpu_percent: process.cpu_usage(),
                rss_bytes: process.memory(),
            })
            .collect::<Vec<_>>();
        Ok(build_resource_sample(
            resource_sample_timestamp_millis(),
            std::process::id(),
            &processes,
            &owners,
        ))
    })
    .await
    .map_err(|error| format!("Resource sampling task failed: {error}"))?
}

fn resource_sample_owners(
    terminal_registry: &crate::terminal::TerminalRegistry,
    agent_runtime: &crate::agent_conversation::manager::AgentRuntimeManager,
) -> Result<Vec<ResourceSampleOwner>, String> {
    let mut sessions = crate::terminal::list_terminal_sessions(terminal_registry)?;
    sessions.sort_by_key(|session| session.started_at);
    let mut terminal_number = 0usize;
    let mut run_number = 0usize;
    let mut browser_number = 0usize;
    let mut tool_number = 0usize;
    let mut owners = Vec::new();

    for session in sessions {
        if session.exited {
            continue;
        }
        let Some(pid) = session.pid else {
            continue;
        };
        let (label, kind) = match session.kind {
            crate::terminal::TerminalKind::UserPty => {
                terminal_number += 1;
                (
                    format!("Terminal {terminal_number}"),
                    ResourceSampleSessionKind::Terminal,
                )
            }
            crate::terminal::TerminalKind::AgentTool => {
                tool_number += 1;
                (
                    format!("Agent tool {tool_number}"),
                    ResourceSampleSessionKind::Other,
                )
            }
            crate::terminal::TerminalKind::RunConfiguration => {
                run_number += 1;
                (
                    format!("Run {run_number}"),
                    ResourceSampleSessionKind::Other,
                )
            }
            crate::terminal::TerminalKind::BrowserAutomation => {
                browser_number += 1;
                (
                    format!("Browser automation {browser_number}"),
                    ResourceSampleSessionKind::Other,
                )
            }
        };
        let owned_id = session
            .owned_id
            .clone()
            .or_else(|| {
                session
                    .tool_terminal_identity
                    .as_ref()
                    .map(|identity| identity.owned_id.clone())
            })
            .or_else(|| Some(session.session_id.clone()));
        owners.push(ResourceSampleOwner {
            root_pid: pid,
            owned_id,
            label,
            kind,
            workspace: resource_workspace_label(&session.cwd),
        });
    }

    let mut conversations = agent_runtime.resource_roots();
    conversations.sort_by(|left, right| left.owned_id.cmp(&right.owned_id));
    owners.extend(
        conversations
            .into_iter()
            .map(|conversation| ResourceSampleOwner {
                root_pid: conversation.pid,
                owned_id: Some(conversation.owned_id),
                label: format!(
                    "{} conversation",
                    resource_provider_label(conversation.provider)
                ),
                kind: ResourceSampleSessionKind::Conversation,
                workspace: resource_workspace_label(&conversation.cwd),
            }),
    );
    Ok(owners)
}

fn resource_workspace_label(cwd: &str) -> String {
    let (project, workspace) = resource_path_labels(cwd);
    match (project, workspace) {
        (Some(project), Some(workspace)) if project != workspace => {
            format!("{project} / {workspace}")
        }
        (Some(project), _) => project,
        (_, Some(workspace)) => workspace,
        _ => cwd.to_string(),
    }
}

fn resource_sample_timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn descendant_distances(root_pid: u32, processes: &[ObservedProcess]) -> Vec<(u32, usize)> {
    let process_ids = processes
        .iter()
        .map(|process| process.pid)
        .collect::<HashSet<_>>();
    if !process_ids.contains(&root_pid) {
        return Vec::new();
    }

    let mut children = HashMap::<u32, Vec<u32>>::new();
    for process in processes {
        if let Some(parent_pid) = process.parent_pid {
            children.entry(parent_pid).or_default().push(process.pid);
        }
    }
    for child_pids in children.values_mut() {
        child_pids.sort_unstable();
    }

    let mut queue = VecDeque::from([(root_pid, 0usize)]);
    let mut visited = HashSet::new();
    let mut descendants = Vec::new();
    while let Some((pid, distance)) = queue.pop_front() {
        if !visited.insert(pid) {
            continue;
        }
        descendants.push((pid, distance));
        if let Some(child_pids) = children.get(&pid) {
            queue.extend(
                child_pids
                    .iter()
                    .map(|child_pid| (*child_pid, distance + 1)),
            );
        }
    }
    descendants
}

fn descendant_process_ids(root_pid: u32, processes: &[ObservedProcess]) -> Vec<u32> {
    descendant_distances(root_pid, processes)
        .into_iter()
        .map(|(pid, _)| pid)
        .collect()
}

fn sample_process(process: &ObservedProcess) -> ResourceSampleProcess {
    ResourceSampleProcess {
        pid: process.pid,
        name: if process.name.trim().is_empty() {
            format!("Process {}", process.pid)
        } else {
            process.name.clone()
        },
        cpu_percent: process.cpu_percent,
        rss_bytes: process.rss_bytes,
    }
}

fn app_part_label(process: &ObservedProcess, app_pid: u32) -> String {
    if process.pid == app_pid {
        return "Main process".to_string();
    }
    let name = process.name.to_ascii_lowercase();
    if name.contains("webview") || name.contains("webkit") || name.contains("renderer") {
        "Webview / renderer".to_string()
    } else if name.contains("node") || name.contains("acp") || name.contains("adapter") {
        "ACP bridge / node".to_string()
    } else if process.name.trim().is_empty() {
        "App helper".to_string()
    } else {
        format!("App helper / {}", process.name)
    }
}

fn build_resource_sample(
    generated_at_ms: u128,
    app_pid: u32,
    processes: &[ObservedProcess],
    owners: &[ResourceSampleOwner],
) -> ResourceSample {
    // A process can sit beneath two roots during a handoff. Attribute it to
    // the closest registered root so nested conversations never count twice.
    let mut claims = HashMap::<u32, (usize, usize)>::new();
    for (owner_index, owner) in owners.iter().enumerate() {
        for (pid, distance) in descendant_distances(owner.root_pid, processes) {
            match claims.get_mut(&pid) {
                Some((claimed_owner, claimed_distance)) if distance < *claimed_distance => {
                    *claimed_owner = owner_index;
                    *claimed_distance = distance;
                }
                None => {
                    claims.insert(pid, (owner_index, distance));
                }
                _ => {}
            }
        }
    }

    let process_by_pid = processes
        .iter()
        .map(|process| (process.pid, process))
        .collect::<HashMap<_, _>>();
    let mut sessions_by_workspace = BTreeMap::<String, Vec<ResourceSampleSession>>::new();
    for (owner_index, owner) in owners.iter().enumerate() {
        let mut owned_processes = claims
            .iter()
            .filter_map(|(pid, (claimed_owner, _))| {
                (*claimed_owner == owner_index)
                    .then(|| process_by_pid.get(pid).copied())
                    .flatten()
            })
            .map(sample_process)
            .collect::<Vec<_>>();
        owned_processes.sort_by(|left, right| {
            right
                .rss_bytes
                .cmp(&left.rss_bytes)
                .then_with(|| left.pid.cmp(&right.pid))
        });
        if owned_processes.is_empty() {
            continue;
        }
        sessions_by_workspace
            .entry(owner.workspace.clone())
            .or_default()
            .push(ResourceSampleSession {
                owned_id: owner.owned_id.clone(),
                label: owner.label.clone(),
                kind: owner.kind,
                processes: owned_processes,
            });
    }
    let groups = sessions_by_workspace
        .into_iter()
        .map(|(workspace, sessions)| ResourceSampleGroup {
            workspace,
            sessions,
        })
        .collect::<Vec<_>>();

    let app_process_ids = descendant_process_ids(app_pid, processes)
        .into_iter()
        .filter(|pid| !claims.contains_key(pid))
        .collect::<HashSet<_>>();
    let mut app_parts = processes
        .iter()
        .filter(|process| app_process_ids.contains(&process.pid))
        .map(|process| ResourceSampleAppPart {
            label: app_part_label(process, app_pid),
            pid: process.pid,
            cpu_percent: process.cpu_percent,
            rss_bytes: process.rss_bytes,
        })
        .collect::<Vec<_>>();
    app_parts.sort_by(|left, right| {
        (left.pid != app_pid)
            .cmp(&(right.pid != app_pid))
            .then_with(|| right.rss_bytes.cmp(&left.rss_bytes))
            .then_with(|| left.pid.cmp(&right.pid))
    });

    let included = processes
        .iter()
        .filter(|process| {
            claims.contains_key(&process.pid) || app_process_ids.contains(&process.pid)
        })
        .collect::<Vec<_>>();
    ResourceSample {
        generated_at_ms,
        totals: ResourceSampleTotals {
            cpu_percent: included.iter().map(|process| process.cpu_percent).sum(),
            rss_bytes: included.iter().map(|process| process.rss_bytes).sum(),
            process_count: included.len(),
        },
        app: ResourceSampleApp { parts: app_parts },
        groups,
    }
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

fn resource_provider_label(
    provider: crate::agent_conversation::protocol::AgentConversationProvider,
) -> &'static str {
    match provider {
        crate::agent_conversation::protocol::AgentConversationProvider::Codex => "Codex",
        crate::agent_conversation::protocol::AgentConversationProvider::Claude => "Provider",
    }
}

type ResourceIdentity = (Option<String>, Option<String>);

fn resource_path_labels(cwd: &str) -> ResourceIdentity {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, ResourceIdentity>>> = OnceLock::new();
    let path = PathBuf::from(cwd);
    let cache_key = path.canonicalize().unwrap_or_else(|_| path.clone());
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(entries) = cache.lock() {
        if let Some(identity) = entries.get(&cache_key) {
            return identity.clone();
        }
    }

    let identity = git_resource_path_labels(&cache_key)
        .unwrap_or_else(|| heuristic_resource_path_labels(&cache_key));
    if let Ok(mut entries) = cache.lock() {
        entries.insert(cache_key, identity.clone());
    }
    identity
}

fn git_resource_path_labels(cwd: &Path) -> Option<ResourceIdentity> {
    let git_root = git_rev_parse_path(cwd, "--show-toplevel")?;
    let git_common_dir = git_rev_parse_path(cwd, "--git-common-dir")?;
    let project = git_common_dir.parent().and_then(path_leaf).or_else(|| {
        git_root
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
    });
    let workspace = git_root
        .file_name()
        .map(|value| value.to_string_lossy().to_string());
    Some((project, workspace))
}

fn git_rev_parse_path(cwd: &Path, argument: &str) -> Option<PathBuf> {
    let absolute_output = Command::new("git")
        .current_dir(cwd)
        .args(["rev-parse", "--path-format=absolute", argument])
        .output()
        .ok();
    let output = absolute_output
        .filter(|output| output.status.success())
        .or_else(|| {
            Command::new("git")
                .current_dir(cwd)
                .args(["rev-parse", argument])
                .output()
                .ok()
                .filter(|output| output.status.success())
        })?;
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        let path = PathBuf::from(value);
        Some(if path.is_absolute() {
            path
        } else {
            cwd.join(path)
        })
    }
}

fn path_leaf(path: &Path) -> Option<String> {
    path.file_name()
        .map(|value| value.to_string_lossy().to_string())
}

fn heuristic_resource_path_labels(path: &Path) -> ResourceIdentity {
    let workspace = path_leaf(path);
    let project = path.parent().and_then(path_leaf);
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
        let _result =
            crate::remove_project_worktree_sync(repository_root, canonical.clone(), false)?;
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
        .args([
            "-C",
            worktree.to_str()?,
            "rev-parse",
            "--path-format=absolute",
            "--git-common-dir",
        ])
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
        return Err(resource_action_error_message(
            ResourceActionError::StaleSnapshot,
        ));
    }
    let snapshot = read_resource_snapshot_at_generation(
        request.expected_generation,
        &terminal_registry,
        &agent_runtime,
    )?;
    if registry.generation.load(Ordering::Acquire) != request.expected_generation {
        return Err(resource_action_error_message(
            ResourceActionError::StaleSnapshot,
        ));
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
        return Err(format!(
            "Could not stop owned resource: {}",
            std::io::Error::last_os_error()
        ));
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
pub fn restart_language_server_root(
    _request: ResourceRootRequest,
) -> Result<ResourceUnavailable, String> {
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
pub fn read_language_server_log(
    _request: ResourceRootRequest,
) -> Result<ResourceUnavailable, String> {
    Err("Language-server logs belong to the isolated lifecycle lane".to_string())
}

fn resource_action_error_message(error: ResourceActionError) -> String {
    match error {
        ResourceActionError::External => "External resources cannot be stopped".to_string(),
        ResourceActionError::StaleSnapshot => {
            "The resource snapshot is stale; refresh first".to_string()
        }
        ResourceActionError::ProcessGroupChanged => {
            "The process group changed; refresh first".to_string()
        }
    }
}

fn validate_disk_scan_root(path: &PathBuf) -> Result<(), String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("Disk scan root is unavailable: {error}"))?;
    if canonical.parent().is_none() || canonical == PathBuf::from("/") {
        return Err("The filesystem root is not an explicit workspace root".to_string());
    }
    if let Some(home) = std::env::var_os("HOME")
        .map(PathBuf::from)
        .and_then(|path| path.canonicalize().ok())
    {
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

fn scan_disk_entry(request: &ResourceCleanupRequest, canonical: &PathBuf) -> Result<u64, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn observed(
        pid: u32,
        parent_pid: Option<u32>,
        name: &str,
        cpu_percent: f32,
        rss_bytes: u64,
    ) -> ObservedProcess {
        ObservedProcess {
            pid,
            parent_pid,
            name: name.to_string(),
            cpu_percent,
            rss_bytes,
        }
    }

    fn owner(
        root_pid: u32,
        owned_id: &str,
        label: &str,
        kind: ResourceSampleSessionKind,
        workspace: &str,
    ) -> ResourceSampleOwner {
        ResourceSampleOwner {
            root_pid,
            owned_id: Some(owned_id.to_string()),
            label: label.to_string(),
            kind,
            workspace: workspace.to_string(),
        }
    }

    fn fixture_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "mcb-resources-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after the epoch")
                .as_nanos()
        ))
    }

    fn git(cwd: &Path, args: &[&str]) -> std::process::Output {
        Command::new("git")
            .current_dir(cwd)
            .args(args)
            .output()
            .expect("fixture git command should start")
    }

    #[test]
    fn resource_identity_uses_git_repository_and_worktree_roots() {
        let root = fixture_root("identity");
        let repository = root.join("fixture-repo");
        fs::create_dir_all(&repository).expect("fixture repository should be created");
        assert!(git(&repository, &["init", "--quiet"]).status.success());
        fs::write(repository.join("README.md"), "fixture\n")
            .expect("fixture file should be written");
        assert!(git(&repository, &["add", "."]).status.success());
        assert!(git(
            &repository,
            &[
                "-c",
                "user.name=Resource Fixture",
                "-c",
                "user.email=resource-fixture@example.test",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ]
        )
        .status
        .success());

        let primary_nested = repository.join("src").join("nested");
        fs::create_dir_all(&primary_nested).expect("nested primary checkout should be created");
        assert_eq!(
            resource_path_labels(
                primary_nested
                    .to_str()
                    .expect("primary path should be UTF-8")
            ),
            (
                Some("fixture-repo".to_string()),
                Some("fixture-repo".to_string())
            )
        );

        let worktree = root.join("feature-worktree");
        assert!(git(
            &repository,
            &[
                "worktree",
                "add",
                "--detach",
                "--quiet",
                worktree.to_str().expect("worktree path should be UTF-8"),
            ]
        )
        .status
        .success());
        let worktree_nested = worktree.join("src").join("nested");
        fs::create_dir_all(&worktree_nested).expect("nested worktree checkout should be created");
        assert_eq!(
            resource_path_labels(
                worktree_nested
                    .to_str()
                    .expect("worktree path should be UTF-8")
            ),
            (
                Some("fixture-repo".to_string()),
                Some("feature-worktree".to_string())
            )
        );

        let plain = root.join("plain").join("nested");
        fs::create_dir_all(&plain).expect("non-git fixture should be created");
        assert_eq!(
            resource_path_labels(plain.to_str().expect("plain path should be UTF-8")),
            (Some("plain".to_string()), Some("nested".to_string()))
        );

        fs::remove_dir_all(root).expect("resource identity fixture should be cleaned up");
    }

    #[test]
    fn descendant_walk_includes_the_root_and_every_generation_once() {
        let processes = vec![
            observed(1, None, "app", 0.0, 1),
            observed(2, Some(1), "child-a", 0.0, 1),
            observed(3, Some(2), "grandchild", 0.0, 1),
            observed(4, Some(1), "child-b", 0.0, 1),
            observed(5, Some(99), "unrelated", 0.0, 1),
        ];

        assert_eq!(descendant_process_ids(1, &processes), vec![1, 2, 4, 3]);
        assert!(descendant_process_ids(99, &processes).is_empty());
    }

    #[test]
    fn sample_groups_owned_trees_and_keeps_unclaimed_app_parts() {
        let processes = vec![
            observed(1, None, "MacCommandBar", 1.0, 100),
            observed(2, Some(1), "WebKit Renderer", 2.0, 90),
            observed(10, Some(1), "zsh", 3.0, 80),
            observed(11, Some(10), "cargo", 4.0, 70),
            observed(20, Some(1), "acp-adapter", 5.0, 60),
            observed(21, Some(20), "node", 6.0, 50),
            observed(99, None, "external", 90.0, 900),
        ];
        let owners = vec![
            owner(
                10,
                "terminal-a",
                "Terminal 1",
                ResourceSampleSessionKind::Terminal,
                "project / workspace",
            ),
            owner(
                20,
                "conversation-a",
                "Agent conversation",
                ResourceSampleSessionKind::Conversation,
                "project / workspace",
            ),
        ];

        let sample = build_resource_sample(123, 1, &processes, &owners);

        assert_eq!(sample.generated_at_ms, 123);
        assert_eq!(sample.totals.process_count, 6);
        assert_eq!(sample.totals.rss_bytes, 450);
        assert_eq!(sample.totals.cpu_percent, 21.0);
        assert_eq!(sample.app.parts.len(), 2);
        assert_eq!(sample.app.parts[0].label, "Main process");
        assert_eq!(sample.app.parts[1].label, "Webview / renderer");
        assert_eq!(sample.groups.len(), 1);
        assert_eq!(sample.groups[0].sessions.len(), 2);
        assert_eq!(sample.groups[0].sessions[0].processes.len(), 2);
        assert_eq!(sample.groups[0].sessions[1].processes.len(), 2);
    }

    #[test]
    fn nested_owner_root_claims_its_closest_process_tree() {
        let processes = vec![
            observed(1, None, "app", 0.0, 1),
            observed(10, Some(1), "terminal", 0.0, 1),
            observed(11, Some(10), "adapter", 0.0, 1),
            observed(12, Some(11), "helper", 0.0, 1),
        ];
        let owners = vec![
            owner(
                10,
                "terminal-a",
                "Terminal 1",
                ResourceSampleSessionKind::Terminal,
                "workspace",
            ),
            owner(
                11,
                "conversation-a",
                "Agent conversation",
                ResourceSampleSessionKind::Conversation,
                "workspace",
            ),
        ];

        let sample = build_resource_sample(123, 1, &processes, &owners);
        let terminal = &sample.groups[0].sessions[0];
        let conversation = &sample.groups[0].sessions[1];

        assert_eq!(
            terminal
                .processes
                .iter()
                .map(|part| part.pid)
                .collect::<Vec<_>>(),
            vec![10]
        );
        assert_eq!(
            conversation
                .processes
                .iter()
                .map(|part| part.pid)
                .collect::<HashSet<_>>(),
            HashSet::from([11, 12])
        );
        assert_eq!(sample.totals.process_count, 4);
    }
}
