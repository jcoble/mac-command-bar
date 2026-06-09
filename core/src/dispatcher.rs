use crate::protocol::{ConfirmableAction, CoreRequest, CoreResponse};
use crate::scanners::processes::scan_lsof_listeners;
use crate::scanners::sessions::scan_sessions;
use crate::scanners::worktrees::{scan_worktrees_with_options, WorktreeScanOptions};
use crate::source::{list_source_files, preview_source_file};
use serde_json::{json, Value};
use std::path::Path;

pub fn dispatch(request: CoreRequest) -> CoreResponse {
    match request.action.as_str() {
        "scan.worktrees" => scan_worktrees_action(request),
        "scan.processes" => scan_processes_action(request),
        "scan.sessions" => scan_sessions_action(request),
        "health.snapshot" => health_snapshot_action(request),
        "source.list" => source_list_action(request),
        "source.preview" => source_preview_action(request),
        "plan.killProcess" => plan_kill_process_action(request),
        action => CoreResponse::error(
            request.id,
            format!("unknown action: {action}"),
            vec![format!("No dispatcher is registered for {action}")],
        ),
    }
}

fn source_list_action(request: CoreRequest) -> CoreResponse {
    let Some(root_path) = request.payload.get("rootPath").and_then(Value::as_str) else {
        return CoreResponse::error(
            request.id,
            "missing root path",
            vec!["payload.rootPath must be a project directory".to_string()],
        );
    };
    let limit = request
        .payload
        .get("limit")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(200);

    match list_source_files(Path::new(root_path), limit) {
        Ok(files) => CoreResponse::ok(
            request.id,
            format!("found {} source files", files.len()),
            json!({ "files": files, "count": files.len() }),
        ),
        Err(error) => CoreResponse::error(
            request.id,
            "source list failed",
            vec![format!("{error:#}")],
        ),
    }
}

fn source_preview_action(request: CoreRequest) -> CoreResponse {
    let Some(path) = request.payload.get("path").and_then(Value::as_str) else {
        return CoreResponse::error(
            request.id,
            "missing path",
            vec!["payload.path must be a source file path".to_string()],
        );
    };

    match preview_source_file(Path::new(path)) {
        Ok(preview) => CoreResponse::ok(
            request.id,
            format!("previewed {}", preview.file_name),
            serde_json::to_value(preview).unwrap_or_else(|_| json!({})),
        ),
        Err(error) => CoreResponse::error(
            request.id,
            "source preview failed",
            vec![format!("{error:#}")],
        ),
    }
}

fn scan_worktrees_action(request: CoreRequest) -> CoreResponse {
    let repo_path = request
        .payload
        .get("repoPath")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|path| path.display().to_string())
        });

    let Some(repo_path) = repo_path else {
        return CoreResponse::error(request.id, "repo path unavailable", Vec::new());
    };

    let records = scan_worktrees_with_options(
        &repo_path,
        WorktreeScanOptions {
            include_disk_bytes: request
                .payload
                .get("includeDiskBytes")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        },
    );
    CoreResponse::ok(
        request.id,
        format!("found {} worktrees", records.len()),
        json!({ "worktrees": records, "count": records.len() }),
    )
}

fn scan_processes_action(request: CoreRequest) -> CoreResponse {
    let records = scan_lsof_listeners();
    CoreResponse::ok(
        request.id,
        format!("found {} listeners", records.len()),
        json!({ "processes": records, "count": records.len() }),
    )
}

fn scan_sessions_action(request: CoreRequest) -> CoreResponse {
    let records = scan_sessions();
    CoreResponse::ok(
        request.id,
        format!("found {} agent sessions", records.len()),
        json!({ "sessions": records, "count": records.len() }),
    )
}

fn health_snapshot_action(request: CoreRequest) -> CoreResponse {
    CoreResponse::ok(
        request.id,
        "health snapshot ready",
        json!({
            "hostname": hostname(),
            "cwd": std::env::current_dir().ok().map(|path| path.display().to_string()),
            "home": std::env::var("HOME").ok()
        }),
    )
}

fn plan_kill_process_action(request: CoreRequest) -> CoreResponse {
    let Some(pid) = request.payload.get("pid").and_then(Value::as_u64) else {
        return CoreResponse::error(
            request.id,
            "missing pid",
            vec!["payload.pid must be a process id".to_string()],
        );
    };

    let command = format!("kill -TERM {pid}");
    let action = ConfirmableAction {
        action_id: format!("kill-{pid}"),
        kind: "kill-process".to_string(),
        target_label: format!("pid {pid}"),
        risk: "high".to_string(),
        command_preview: command.clone(),
        requires_confirmation: true,
    };

    let mut response = CoreResponse::ok(
        request.id,
        "kill plan requires confirmation",
        json!({ "action": action }),
    );
    response.proposed_command = Some(command);
    response
}

fn hostname() -> Option<String> {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}
