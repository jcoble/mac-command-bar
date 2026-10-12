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

/// The repository's workflow files, as paths relative to `repo_root`.
#[tauri::command]
pub(crate) async fn list_workflow_files(repo_root: String) -> Result<Vec<String>, String> {
    let directory = std::path::Path::new(&repo_root).join(".assembly/workflows");
    let Ok(entries) = std::fs::read_dir(&directory) else { return Ok(Vec::new()) };
    let mut files: Vec<String> = entries
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.ends_with(".yml"))
        .map(|name| format!(".assembly/workflows/{name}"))
        .collect();
    files.sort();
    Ok(files)
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
pub(crate) async fn redirect_workflow_node(
    app: tauri::AppHandle,
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    node_id: String,
    provider: String,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    let run = engine
        .redirect_node(&run_id, &node_id, &provider, &idempotency_key)
        .map_err(|error| error.to_string())?;
    emit_workflow_run_updated(&app, &run);
    Ok(run)
}

#[tauri::command]
pub(crate) async fn skip_workflow_node(
    app: tauri::AppHandle,
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    node_id: String,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    let run = engine
        .skip_node(&run_id, &node_id, &idempotency_key)
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

#[tauri::command]
pub(crate) async fn submit_workflow_result(
    app: tauri::AppHandle,
    engine: tauri::State<'_, WorkflowEngine>,
    run_id: String,
    node_id: String,
    result: serde_json::Value,
    idempotency_key: String,
) -> Result<WorkflowRunRecord, String> {
    let run = engine
        .submit_result(&run_id, &node_id, result, &idempotency_key)
        .await
        .map_err(|error| error.to_string())?;
    emit_workflow_run_updated(&app, &run);
    Ok(run)
}
