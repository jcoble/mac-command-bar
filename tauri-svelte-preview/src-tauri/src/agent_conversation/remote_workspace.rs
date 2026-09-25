//! Files and Git operations run on the machine that owns the workspace.
use std::path::PathBuf;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use crate::commands::source_control as git;
use crate::git_workspace as changes;

fn arg<T: DeserializeOwned>(args: &Value, name: &str) -> Result<T, String> {
    serde_json::from_value(args.get(name).cloned().unwrap_or(Value::Null))
        .map_err(|error| format!("Invalid workspace {name}: {error}"))
}
fn path(args: &Value, name: &str) -> Result<PathBuf, String> {
    let value: String = arg(args, name)?;
    if !value.starts_with('/') || value.contains('\0') { return Err(format!("Workspace {name} must be an absolute path")); }
    Ok(PathBuf::from(value))
}
fn json(value: impl Serialize) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|error| error.to_string())
}

pub(super) async fn execute(operation: String, mut args: Value) -> Result<Value, String> {
    if operation == "list_remote_directories" {
        let requested: String = arg(&args, "path")?;
        if requested == "~" || requested.starts_with("~/") || requested.is_empty() {
            let home = std::env::var("HOME").map_err(|_| "Remote home is unavailable")?;
            args["path"] = Value::String(format!("{home}/{}", requested.strip_prefix("~/").unwrap_or("")));
        }
    }
    for key in ["root", "path", "directory", "source", "target"] {
        if args.get(key).is_some() { path(&args, key)?; }
    }
    // Reuse the same Git implementations as the local Tauri commands.
    match operation.as_str() {
        "discard_git_paths" => return json(changes::discard_git_paths(arg(&args, "root")?, arg(&args, "paths")?).await?),
        "discard_all_git_changes" => return json(changes::discard_all_git_changes(arg(&args, "root")?, arg(&args, "includeUntracked")?).await?),
        "list_git_branches" => return json(changes::list_git_branches(arg(&args, "root")?).await?),
        "create_git_branch" => return json(changes::create_git_branch(arg(&args, "root")?, arg(&args, "name")?, arg(&args, "checkout")?).await?),
        "switch_git_branch" => return json(changes::switch_git_branch(arg(&args, "root")?, arg(&args, "name")?).await?),
        "stash_git_changes" => return json(changes::stash_git_changes(arg(&args, "root")?, arg(&args, "includeUntracked")?, arg(&args, "message")?).await?),
        "pop_git_stash" => return json(changes::pop_git_stash(arg(&args, "root")?, arg(&args, "index")?).await?),
        "list_git_stashes" => return json(changes::list_git_stashes(arg(&args, "root")?).await?),
        "amend_git_commit" => return json(changes::amend_git_commit(arg(&args, "root")?, arg(&args, "message")?).await?),
        "project_git_status" => return json(git::project_git_status(path(&args, "root")?.to_string_lossy().into()).await?),
        "read_source_git_diff" => return json(git::read_source_git_diff(arg(&args, "root")?, arg(&args, "path")?).await?),
        "stage_git_paths" => return json(git::stage_git_paths(arg(&args, "root")?, arg(&args, "paths")?).await?),
        "unstage_git_paths" => return json(git::unstage_git_paths(arg(&args, "root")?, arg(&args, "paths")?).await?),
        "commit_git_repository" => return json(git::commit_git_repository(arg(&args, "root")?, arg(&args, "message")?).await?),
        "fetch_git_repository" => return json(git::fetch_git_repository(arg(&args, "root")?).await?),
        "pull_git_repository" => return json(git::pull_git_repository(arg(&args, "root")?).await?),
        "push_git_repository" => return json(git::push_git_repository(arg(&args, "root")?).await?),
        "read_git_commit_history" => return json(git::read_git_commit_history(arg(&args, "root")?, arg(&args, "cursor")?, arg(&args, "relativePath")?).await?),
        "read_git_commit_files" => return json(git::read_git_commit_files(arg(&args, "root")?, arg(&args, "sha")?).await?),
        "read_git_commit_file_diff" => return json(git::read_git_commit_file_diff(arg(&args, "root")?, arg(&args, "sha")?, arg(&args, "relativePath")?).await?),
        "list_project_worktrees" => return json(git::list_project_worktrees(arg(&args, "root")?).await?),
        "list_repository_checkouts" => return json(git::list_repository_checkouts(arg(&args, "roots")?).await?),
        "list_project_git_refs" => return json(git::list_project_git_refs(arg(&args, "root")?).await?),
        "init_project_repository" => return json(git::init_project_repository(arg(&args, "root")?).await?),
        "remove_project_worktree" => return json(git::remove_project_worktree(arg(&args, "root")?, arg(&args, "path")?, arg(&args, "force")?).await?),
        "archive_project_worktree" => return json(git::archive_project_worktree(arg(&args, "root")?, arg(&args, "path")?).await?),
        // Hosted PR commands run gh here, with the same stale-head guards as the Mac.
        "list_github_pull_requests" => return json(crate::github::list_github_pull_requests(arg(&args, "query")?).await?),
        "read_github_pull_request" => return json(crate::github::read_github_pull_request(arg(&args, "root")?, arg(&args, "number")?).await?),
        "read_github_pull_request_file" => return json(crate::github::read_github_pull_request_file(arg(&args, "query")?).await?),
        "submit_github_pull_request_review" => return json(crate::github::submit_github_pull_request_review(arg(&args, "submission")?).await?),
        "reply_github_pull_request_comment" => return json(crate::github::reply_github_pull_request_comment(arg(&args, "reply")?).await?),
        "merge_github_pull_request" => return json(crate::github::merge_github_pull_request(arg(&args, "request")?).await?),
        _ => {}
    }
    tokio::task::spawn_blocking(move || {
        let cancellation = crate::SourceScanCancellation::until(std::time::Instant::now() + std::time::Duration::from_secs(14));
        match operation.as_str() {
            "list_remote_directories" => {
                let root = std::fs::canonicalize(path(&args, "path")?).map_err(|e| e.to_string())?;
                let mut directories = Vec::new();
                for entry in std::fs::read_dir(&root).map_err(|e| e.to_string())? {
                    let entry = entry.map_err(|e| e.to_string())?;
                    if entry.path().is_dir() { directories.push(entry.file_name().to_string_lossy().into_owned()); }
                    if directories.len() > 500 { break; }
                }
                let truncated = directories.len() > 500;
                directories.truncate(500);
                directories.sort();
                json(serde_json::json!({"path": root, "directories": directories, "truncated": truncated}))
            }
            "validate_project_root" => json(crate::validate_project_root_sync_while(path(&args, "path")?, || cancellation.ensure_active().is_ok())),
            "list_source_directory" => json(crate::list_source_directory_sync_with_cancellation(path(&args, "root")?, path(&args, "directory")?, arg::<Option<bool>>(&args, "includeExcluded")?.unwrap_or(false), cancellation)?),
            "list_source_files" => json(crate::list_source_files_sync_with_cancellation(path(&args, "root")?, arg::<Option<usize>>(&args, "limit")?.unwrap_or(crate::DEFAULT_SOURCE_LIST_LIMIT), arg(&args, "query")?, cancellation)?),
            "search_source_tree" => json(crate::search_source_tree_sync(path(&args, "root")?, arg(&args, "query")?, arg(&args, "pageSize")?, arg(&args, "cursor")?, arg::<Option<bool>>(&args, "includeExcluded")?.unwrap_or(false), cancellation)?),
            "read_source_file" => json(Some(crate::read_source_file_sync(path(&args, "path")?)?)),
            "read_source_image" => json(crate::read_source_image_sync(path(&args, "path")?)?),
            "write_source_file" => json(crate::write_source_file_sync(path(&args, "path")?, arg(&args, "content")?, arg(&args, "expectedRevision")?)?),
            "workspace_exists" => json(path(&args, "path")?.try_exists().map_err(|e| e.to_string())?),
            "workspace_mkdir" => { std::fs::create_dir(path(&args, "path")?).map_err(|e| e.to_string())?; json(()) },
            "workspace_rename" => { std::fs::rename(path(&args, "source")?, path(&args, "target")?).map_err(|e| e.to_string())?; json(()) },
            "workspace_copy_file" => { std::fs::copy(path(&args, "source")?, path(&args, "target")?).map_err(|e| e.to_string())?; json(()) },
            "workspace_write_text" => { std::fs::write(path(&args, "path")?, arg::<String>(&args, "content")?).map_err(|e| e.to_string())?; json(()) },
            "workspace_read_dir" => {
                let entries = std::fs::read_dir(path(&args, "path")?).map_err(|e| e.to_string())?
                    .map(|entry| { let entry = entry.map_err(|e| e.to_string())?; let kind = entry.file_type().map_err(|e| e.to_string())?;
                        Ok(serde_json::json!({"name": entry.file_name().to_string_lossy(), "isDirectory": kind.is_dir(), "isFile": kind.is_file(), "isSymlink": kind.is_symlink()}))
                    }).collect::<Result<Vec<_>, String>>()?;
                json(entries)
            }
            "move_to_trash" => { trash::delete(path(&args, "path")?).map_err(|e| e.to_string())?; json(()) },
            _ => Err(format!("{operation} is not available for remote workspaces")),
        }
    }).await.map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn remote_workspace_reads_writes_and_git_use_requested_root() {
        let root = std::env::temp_dir().join(format!("assembly-workspace-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let file = root.join("hello.txt");
        let run = async {
            execute("workspace_write_text".into(), serde_json::json!({"path": file, "content": "remote content"})).await.unwrap();
            let read = execute("read_source_file".into(), serde_json::json!({"path": file})).await.unwrap();
            assert_eq!(read["content"], "remote content");
            let saved = execute("write_source_file".into(), serde_json::json!({
                "path": file,
                "content": "updated remote content",
                "expectedRevision": read["revision"]
            })).await.unwrap();
            assert_eq!(saved["content"], "updated remote content");
            let listing = execute("list_source_directory".into(), serde_json::json!({"root": root, "directory": root})).await.unwrap();
            assert!(listing.as_array().unwrap().iter().any(|entry| entry["path"] == file.to_string_lossy().as_ref()));
            execute("init_project_repository".into(), serde_json::json!({"root": root})).await.unwrap();
            assert!(root.join(".git").is_dir());
            let status = execute("project_git_status".into(), serde_json::json!({"root": root})).await.unwrap();
            assert!(status.to_string().contains("hello.txt"));
        };
        run.await;
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn unsupported_and_relative_workspace_requests_are_rejected() {
        assert!(execute("run_terminal_command".into(), serde_json::json!({"root": "/tmp"})).await.unwrap_err().contains("not available"));
        assert!(execute("workspace_write_text".into(), serde_json::json!({"path": "relative", "content": "no"})).await.unwrap_err().contains("absolute"));
    }
    #[tokio::test]
    async fn github_requests_reach_the_hosted_pr_validation() {
        let query = serde_json::json!({"query": {"roots": [], "mode": "open", "pageSize": 30}});
        assert!(execute("list_github_pull_requests".into(), query).await.unwrap_err().contains("between one and 32"));
        let merge = serde_json::json!({"request": {"root": "/tmp", "number": 1, "expectedHeadSha": "a".repeat(40), "expectedBaseSha": "b".repeat(40), "method": "delete"}});
        assert_eq!(execute("merge_github_pull_request".into(), merge).await.unwrap_err(), "Invalid pull request merge target or method");
    }
}
