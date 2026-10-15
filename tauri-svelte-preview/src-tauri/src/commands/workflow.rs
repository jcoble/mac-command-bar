use tauri::Emitter;

use crate::workflow::{TaskSnapshot, WorkflowEngine, WorkflowRunRecord};

#[tauri::command]
pub(crate) async fn list_workflow_runs(
    engine: tauri::State<'_, WorkflowEngine>,
) -> Result<Vec<WorkflowRunRecord>, String> {
    engine.list_runs().map_err(|error| error.to_string())
}

pub(crate) fn emit_workflow_run_updated(app: &tauri::AppHandle, run: &WorkflowRunRecord) {
    let _ = app.emit("workflow-run-updated", run);
}

#[tauri::command]
pub(crate) async fn create_workflow_run(
    app: tauri::AppHandle,
    engine: tauri::State<'_, WorkflowEngine>,
    repo_root: String,
    workflow_path: String,
    task: TaskSnapshot,
    input: serde_json::Value,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    let run = engine
        .create_run_from_file(std::path::Path::new(&repo_root), std::path::Path::new(&workflow_path), task, input, idempotency_key)
        .map_err(|error| error.to_string())?;
    emit_workflow_run_updated(&app, &run);
    Ok(run)
}

#[tauri::command]
pub(crate) async fn get_workflow_run(
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
) -> Result<WorkflowRunRecord, String> {
    engine.get_run(&run_id).map_err(|error| error.to_string())
}

/// Workflow steps waiting on the owner on this Mac and every connected machine, each tagged
/// with its machine. A machine that cannot be reached is left out.
#[tauri::command]
pub(crate) async fn list_owner_inbox(
    engine: tauri::State<'_, WorkflowEngine>,
    remote: tauri::State<'_, crate::agent_conversation::remote::RemoteConnectionManager>,
    request_id: u64,
) -> Result<Vec<serde_json::Value>, String> {
    let work = async {
        let mut steps = Vec::new();
        for machine in remote.message_machines("") {
            let page = if machine == crate::agent_conversation::remote::LOCAL_MACHINE {
                crate::workflow::execute_operation(&engine, "inbox", serde_json::Value::Null).await
            } else {
                remote.workflow_operation(&machine, "inbox".into(), serde_json::Value::Null, Some(request_id)).await
            };
            let Ok(serde_json::Value::Array(page)) = page else { continue };
            steps.extend(page.into_iter().map(|mut step| { step["machine"] = serde_json::json!(machine); step }));
        }
        steps
    };
    Ok(remote.unless_cancelled(request_id, work).await.unwrap_or_default())
}

/// The repository's workflow files, as paths relative to `repo_root`.
#[tauri::command]
pub(crate) async fn list_workflow_files(repo_root: String) -> Result<Vec<String>, String> {
    Ok(crate::workflow::workflow_files(std::path::Path::new(&repo_root)))
}

#[tauri::command]
pub(crate) async fn answer_workflow_pause(
    app: tauri::AppHandle,
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    node_id: String,
    decision: String,
    reply: Option<String>,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    let run = engine
        .answer_pause(&run_id, &node_id, &decision, reply, "owner", &idempotency_key)
        .await
        .map_err(|error| error.to_string())?;
    emit_workflow_run_updated(&app, &run);
    Ok(run)
}

#[tauri::command]
pub(crate) async fn start_workflow_run(
    app: tauri::AppHandle,
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    let run = engine
        .start(&run_id, &idempotency_key)
        .await
        .map_err(|error| error.to_string())?;
    emit_workflow_run_updated(&app, &run);
    Ok(run)
}

#[tauri::command]
pub(crate) async fn pause_workflow_run(
    app: tauri::AppHandle,
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    let run = engine
        .pause(&run_id, &idempotency_key)
        .map_err(|error| error.to_string())?;
    emit_workflow_run_updated(&app, &run);
    Ok(run)
}

#[tauri::command]
pub(crate) async fn resume_workflow_run(
    app: tauri::AppHandle,
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    let run = engine
        .resume(&run_id, &idempotency_key)
        .await
        .map_err(|error| error.to_string())?;
    emit_workflow_run_updated(&app, &run);
    Ok(run)
}

#[tauri::command]
pub(crate) async fn cancel_workflow_run(
    app: tauri::AppHandle,
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    let run = engine
        .cancel(&run_id, &idempotency_key)
        .await
        .map_err(|error| error.to_string())?;
    emit_workflow_run_updated(&app, &run);
    Ok(run)
}

#[tauri::command]
pub(crate) async fn retry_workflow_node(
    app: tauri::AppHandle,
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    node_id: String,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    let run = engine
        .retry_node(&run_id, &node_id, &idempotency_key)
        .await
        .map_err(|error| error.to_string())?;
    emit_workflow_run_updated(&app, &run);
    Ok(run)
}

#[tauri::command]
pub(crate) async fn approve_workflow_gate(
    app: tauri::AppHandle,
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    node_id: String,
    approval: serde_json::Value,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    let run = engine
        .approve_gate(&run_id, &node_id, approval, "owner", &idempotency_key)
        .await
        .map_err(|error| error.to_string())?;
    emit_workflow_run_updated(&app, &run);
    Ok(run)
}
