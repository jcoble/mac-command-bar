use crate::protocol::{ConfirmableAction, CoreRequest, CoreResponse};
use crate::scanners::processes::scan_lsof_listeners;
use crate::scanners::sessions::scan_sessions;
use crate::scanners::worktrees::scan_worktrees;
use serde_json::{json, Value};

pub fn dispatch(request: CoreRequest) -> CoreResponse {
    match request.action.as_str() {
        "scan.worktrees" => scan_worktrees_action(request),
        "scan.processes" => scan_processes_action(request),
        "scan.sessions" => scan_sessions_action(request),
        "health.snapshot" => health_snapshot_action(request),
        "plan.killProcess" => plan_kill_process_action(request),
        action => CoreResponse::error(
            request.id,
            format!("unknown action: {action}"),
            vec![format!("No dispatcher is registered for {action}")],
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

    let records = scan_worktrees(&repo_path);
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
