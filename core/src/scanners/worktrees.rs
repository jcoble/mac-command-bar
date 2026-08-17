use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeRecord {
    pub repo: String,
    pub path: String,
    pub branch: String,
    pub task_id: Option<String>,
    pub is_dirty: bool,
    pub has_unmerged_commits: bool,
    #[serde(default)]
    pub is_prunable: bool,
    #[serde(default)]
    pub prunable_reason: Option<String>,
    #[serde(default)]
    pub is_locked: bool,
    #[serde(default)]
    pub locked_reason: Option<String>,
    pub last_activity: Option<String>,
    pub disk_bytes: Option<u64>,
    pub delete_eligibility: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WorktreeScanOptions {
    pub include_disk_bytes: bool,
}

pub fn scan_worktrees(repo_path: &str) -> Vec<WorktreeRecord> {
    scan_worktrees_with_options(repo_path, WorktreeScanOptions::default())
}

/// One checkout of a repository, as the History panel lists them.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryCheckout {
    pub path: String,
    pub branch: String,
    /// True for the repository's own folder rather than one of its worktrees.
    pub is_main: bool,
}

/// Every checkout of a repository that is still on disk: the repository's own
/// folder first, then its worktrees.
///
/// Git is the authority on which worktrees exist, so the panel no longer infers
/// the list from whichever sessions happen to have been run. A worktree that
/// hosted only dispatched lanes is still a worktree, and it belongs in the tree.
///
/// Deleted checkouts are left out. Git keeps a worktree's administrative record
/// until someone prunes it, so `git worktree list` names folders that are gone;
/// listing those would fill the panel with checkouts nobody can open.
///
/// Deliberately light. `scan_worktrees` above asks each worktree whether it is
/// dirty, whether it has unmerged commits, and how big it is — three more
/// commands per checkout — and none of that is needed to draw a heading.
pub fn repository_checkouts(repo_path: &str) -> Vec<RepositoryCheckout> {
    let output = Command::new("git")
        .args(["-C", repo_path, "worktree", "list", "--porcelain"])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    parse_worktree_porcelain(&String::from_utf8_lossy(&output.stdout))
        .into_iter()
        .filter(|record| Path::new(&record.path).is_dir())
        .map(|record| RepositoryCheckout {
            is_main: Path::new(&record.path) == Path::new(repo_path),
            path: record.path,
            branch: record.branch,
        })
        .collect()
}

pub fn scan_worktrees_with_options(
    repo_path: &str,
    options: WorktreeScanOptions,
) -> Vec<WorktreeRecord> {
    let output = Command::new("git")
        .args(["-C", repo_path, "worktree", "list", "--porcelain"])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    let repo = Path::new(repo_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo")
        .to_string();

    parse_worktree_porcelain(&String::from_utf8_lossy(&output.stdout))
        .into_iter()
        .map(|mut record| {
            record.repo = repo.clone();
            let mut dirty_state = DirtyState::Clean;
            if !record.is_prunable {
                dirty_state = worktree_dirty_state(&record.path);
                record.is_dirty = dirty_state.is_dirty();
                record.has_unmerged_commits = has_unmerged_commits(&record.path);
                record.disk_bytes = options
                    .include_disk_bytes
                    .then(|| disk_bytes(&record.path))
                    .flatten();
                record.last_activity = last_activity(&record.path);
            }
            record.delete_eligibility = delete_eligibility(
                dirty_state,
                record.has_unmerged_commits,
                record.is_locked,
                record.is_prunable,
            );
            record
        })
        .collect()
}

pub fn parse_worktree_porcelain(output: &str) -> Vec<WorktreeRecord> {
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
                let task_id = task_id_from_text(&branch).or_else(|| task_id_from_text(&path));

                WorktreeRecord {
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
                    disk_bytes: None,
                    delete_eligibility: "unknown".to_string(),
                }
            })
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirtyState {
    Clean,
    UntrackedOnly,
    Dirty,
}

impl DirtyState {
    fn is_dirty(self) -> bool {
        !matches!(self, DirtyState::Clean)
    }
}

fn worktree_dirty_state(path: &str) -> DirtyState {
    let output = Command::new("git")
        .args(["-C", path, "status", "--short"])
        .output();
    output
        .ok()
        .filter(|output| output.status.success())
        .map(|output| dirty_state_from_status(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or(DirtyState::Clean)
}

fn dirty_state_from_status(status: &str) -> DirtyState {
    let mut saw_status = false;

    for line in status.lines().filter(|line| !line.trim().is_empty()) {
        saw_status = true;
        if !line.starts_with("?? ") {
            return DirtyState::Dirty;
        }
    }

    if saw_status {
        DirtyState::UntrackedOnly
    } else {
        DirtyState::Clean
    }
}

fn has_unmerged_commits(path: &str) -> bool {
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

fn disk_bytes(path: &str) -> Option<u64> {
    let output = Command::new("du").args(["-sk", path]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let kb = stdout.split_whitespace().next()?.parse::<u64>().ok()?;
    Some(kb * 1024)
}

fn last_activity(path: &str) -> Option<String> {
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

fn delete_eligibility(
    dirty_state: DirtyState,
    has_unmerged_commits: bool,
    is_locked: bool,
    is_prunable: bool,
) -> String {
    match dirty_state {
        DirtyState::Dirty => "blocked: dirty worktree".to_string(),
        DirtyState::UntrackedOnly => "blocked: untracked files".to_string(),
        DirtyState::Clean if has_unmerged_commits => "blocked: unmerged commits".to_string(),
        DirtyState::Clean if is_locked => "blocked: locked worktree".to_string(),
        DirtyState::Clean if is_prunable => {
            "review: prunable missing worktree metadata".to_string()
        }
        DirtyState::Clean => "requires-confirmation".to_string(),
    }
}

fn task_id_from_text(text: &str) -> Option<String> {
    let lower_text = text.to_ascii_lowercase();
    for (index, _) in lower_text.match_indices("tsk") {
        if index > 0 {
            let previous = lower_text.as_bytes()[index - 1] as char;
            if previous.is_ascii_alphanumeric() {
                continue;
            }
        }

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

#[cfg(test)]
mod tests {
    use super::{
        delete_eligibility, dirty_state_from_status, parse_worktree_porcelain, DirtyState,
    };

    #[test]
    fn parse_worktree_porcelain_reads_prunable_and_locked_metadata() {
        let records = parse_worktree_porcelain(
            "worktree /repo\nHEAD abc\nbranch refs/heads/main\n\nworktree /missing\nHEAD def\nbranch refs/heads/cdx/tsk-127-missing\nprunable gitdir file points to non-existent location\n\nworktree /locked\nHEAD fed\nbranch refs/heads/cdx/tsk-128-locked\nlocked agent still running\n",
        );

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].branch, "main");
        assert_eq!(records[0].task_id, None);
        assert!(records[1].is_prunable);
        assert_eq!(records[1].task_id.as_deref(), Some("TSK-127"));
        assert_eq!(
            records[1].prunable_reason.as_deref(),
            Some("gitdir file points to non-existent location")
        );
        assert!(records[2].is_locked);
        assert_eq!(records[2].task_id.as_deref(), Some("TSK-128"));
        assert_eq!(
            records[2].locked_reason.as_deref(),
            Some("agent still running")
        );
    }

    #[test]
    fn dirty_state_distinguishes_untracked_only_worktrees() {
        assert_eq!(dirty_state_from_status(""), DirtyState::Clean);
        assert_eq!(
            dirty_state_from_status("?? scratch.txt\n?? notes.md\n"),
            DirtyState::UntrackedOnly
        );
        assert_eq!(
            dirty_state_from_status(" M src/main.rs\n?? scratch.txt\n"),
            DirtyState::Dirty
        );
    }

    #[test]
    fn delete_eligibility_names_untracked_only_before_cleanup() {
        assert_eq!(
            delete_eligibility(DirtyState::UntrackedOnly, false, false, false),
            "blocked: untracked files"
        );
        assert_eq!(
            delete_eligibility(DirtyState::Dirty, true, true, true),
            "blocked: dirty worktree"
        );
        assert_eq!(
            delete_eligibility(DirtyState::Clean, true, true, true),
            "blocked: unmerged commits"
        );
    }
}
