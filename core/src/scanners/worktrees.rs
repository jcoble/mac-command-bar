use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeRecord {
    pub repo: String,
    pub path: String,
    pub branch: String,
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
            if !record.is_prunable {
                record.is_dirty = is_dirty(&record.path);
                record.has_unmerged_commits = has_unmerged_commits(&record.path);
                record.disk_bytes = options
                    .include_disk_bytes
                    .then(|| disk_bytes(&record.path))
                    .flatten();
                record.last_activity = last_activity(&record.path);
            }
            record.delete_eligibility = if record.is_dirty {
                "blocked: dirty worktree".to_string()
            } else if record.has_unmerged_commits {
                "blocked: unmerged commits".to_string()
            } else if record.is_locked {
                "blocked: locked worktree".to_string()
            } else if record.is_prunable {
                "review: prunable missing worktree metadata".to_string()
            } else {
                "requires-confirmation".to_string()
            };
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

            path.map(|path| WorktreeRecord {
                repo: String::new(),
                path,
                branch: branch.unwrap_or_else(|| "unknown".to_string()),
                is_dirty: false,
                has_unmerged_commits: false,
                is_prunable,
                prunable_reason,
                is_locked,
                locked_reason,
                last_activity: None,
                disk_bytes: None,
                delete_eligibility: "unknown".to_string(),
            })
        })
        .collect()
}

fn is_dirty(path: &str) -> bool {
    let output = Command::new("git")
        .args(["-C", path, "status", "--short"])
        .output();
    output
        .ok()
        .filter(|output| output.status.success())
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false)
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

#[cfg(test)]
mod tests {
    use super::parse_worktree_porcelain;

    #[test]
    fn parse_worktree_porcelain_reads_prunable_and_locked_metadata() {
        let records = parse_worktree_porcelain(
            "worktree /repo\nHEAD abc\nbranch refs/heads/main\n\nworktree /missing\nHEAD def\nbranch refs/heads/cdx/tsk-127-missing\nprunable gitdir file points to non-existent location\n\nworktree /locked\nHEAD fed\nbranch refs/heads/cdx/tsk-128-locked\nlocked agent still running\n",
        );

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].branch, "main");
        assert!(records[1].is_prunable);
        assert_eq!(
            records[1].prunable_reason.as_deref(),
            Some("gitdir file points to non-existent location")
        );
        assert!(records[2].is_locked);
        assert_eq!(
            records[2].locked_reason.as_deref(),
            Some("agent still running")
        );
    }
}
