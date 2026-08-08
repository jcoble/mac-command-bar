use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceDiskKind {
    Worktree,
    BuildOutput,
    DependencyCache,
    AgentData,
    Other,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DiskProtection {
    Active,
    Dirty,
    Unmerged,
    Locked,
    UserData,
    SafeCandidate,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDiskItem {
    pub path: PathBuf,
    pub bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDiskEntry {
    pub id: String,
    pub repository_id: String,
    pub workspace_id: String,
    pub path: PathBuf,
    pub kind: WorkspaceDiskKind,
    pub bytes: u64,
    pub reclaimable_bytes: u64,
    pub protection: DiskProtection,
    pub top_level_items: Vec<WorkspaceDiskItem>,
}

#[derive(Debug, Clone)]
pub struct DiskScanRoot {
    pub repository_id: String,
    pub workspace_id: String,
    pub path: PathBuf,
    pub kind: WorkspaceDiskKind,
    pub protection: DiskProtection,
}

#[derive(Debug, Clone, Copy)]
pub struct DiskScanOptions {
    pub max_depth: usize,
    pub max_entries: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiskScanReport {
    pub captured_at_ms: u128,
    pub entries: Vec<WorkspaceDiskEntry>,
    pub scanned_bytes: u64,
    pub reclaimable_bytes: u64,
    pub truncated: bool,
}

pub fn scan_disk_roots(roots: &[DiskScanRoot], options: DiskScanOptions) -> Result<DiskScanReport, String> {
    let max_depth = options.max_depth.min(8);
    let max_entries = options.max_entries.clamp(1, 10_000);
    let mut entries = Vec::new();
    let mut visited = 0usize;
    let mut truncated = false;

    for root in roots {
        if visited >= max_entries {
            truncated = true;
            break;
        }
        let canonical = root
            .path
            .canonicalize()
            .map_err(|error| format!("cannot inspect {}: {error}", root.path.display()))?;
        let (bytes, top_level_items) = measure_path(&canonical, 0, max_depth, max_entries, &mut visited, &mut truncated);
        let reclaimable_bytes = if root.protection == DiskProtection::SafeCandidate {
            bytes
        } else {
            0
        };
        entries.push(WorkspaceDiskEntry {
            id: stable_entry_id(&root.repository_id, &root.workspace_id, &canonical),
            repository_id: root.repository_id.clone(),
            workspace_id: root.workspace_id.clone(),
            path: canonical,
            kind: root.kind,
            bytes,
            reclaimable_bytes,
            protection: root.protection,
            top_level_items,
        });
    }

    Ok(DiskScanReport {
        captured_at_ms: now_ms(),
        scanned_bytes: entries.iter().map(|entry| entry.bytes).sum(),
        reclaimable_bytes: entries.iter().map(|entry| entry.reclaimable_bytes).sum(),
        entries,
        truncated,
    })
}

fn measure_path(
    path: &Path,
    depth: usize,
    max_depth: usize,
    max_entries: usize,
    visited: &mut usize,
    truncated: &mut bool,
) -> (u64, Vec<WorkspaceDiskItem>) {
    if *visited >= max_entries {
        *truncated = true;
        return (0, Vec::new());
    }
    *visited += 1;

    let Ok(metadata) = fs::symlink_metadata(path) else {
        return (0, Vec::new());
    };
    if !metadata.is_dir() {
        return (metadata.len(), Vec::new());
    }
    if depth >= max_depth {
        return (0, Vec::new());
    }

    let Ok(read_dir) = fs::read_dir(path) else {
        return (0, Vec::new());
    };
    let mut children = read_dir.filter_map(Result::ok).collect::<Vec<_>>();
    children.sort_by_key(|entry| entry.file_name());
    let mut bytes = 0u64;
    let mut top_level_items = Vec::new();
    for child in children {
        if *visited >= max_entries {
            *truncated = true;
            break;
        }
        let child_path = child.path();
        let (child_bytes, _) = measure_path(&child_path, depth + 1, max_depth, max_entries, visited, truncated);
        bytes = bytes.saturating_add(child_bytes);
        if depth == 0 {
            top_level_items.push(WorkspaceDiskItem {
                path: child_path,
                bytes: child_bytes,
            });
        }
    }
    (bytes, top_level_items)
}

pub fn disk_protection_allows_cleanup(protection: DiskProtection) -> bool {
    protection == DiskProtection::SafeCandidate
}

pub fn stable_entry_id(repository_id: &str, workspace_id: &str, path: &Path) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in repository_id
        .as_bytes()
        .iter()
        .chain([0_u8].iter())
        .chain(workspace_id.as_bytes().iter())
        .chain([0_u8].iter())
        .chain(path.to_string_lossy().as_bytes().iter())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("disk-{hash:016x}")
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_scan_uses_protection_and_totals() {
        let root = DiskScanRoot {
            repository_id: "repo-1".into(),
            workspace_id: "workspace-1".into(),
            path: std::env::temp_dir(),
            kind: WorkspaceDiskKind::BuildOutput,
            protection: DiskProtection::SafeCandidate,
        };
        let report = scan_disk_roots(&[root], DiskScanOptions { max_depth: 0, max_entries: 1 })
            .expect("bounded scan should produce a report");

        assert_eq!(report.entries.len(), 1);
        assert!(report.reclaimable_bytes <= report.scanned_bytes);
        assert_eq!(report.entries[0].protection, DiskProtection::SafeCandidate);
    }

    #[test]
    fn disk_cleanup_only_allows_safe_candidates() {
        assert!(disk_protection_allows_cleanup(DiskProtection::SafeCandidate));
        assert!(!disk_protection_allows_cleanup(DiskProtection::Dirty));
        assert!(!disk_protection_allows_cleanup(DiskProtection::UserData));
    }
}
