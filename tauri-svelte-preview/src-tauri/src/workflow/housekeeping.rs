//! The owner's housekeeping (TSK-1606): mark a run finished by hand, and remove finished runs.
use super::*;

const DAY_MS: u64 = 86_400_000;

fn ended(run: &WorkflowRunRecord) -> bool {
    matches!(run.state, WorkflowRunState::Completed | WorkflowRunState::Failed | WorkflowRunState::Cancelled)
}

/// The owner's housekeeping operations for `execute_operation` (the desktop, a remote machine, the phone):
/// `markFinished` a run, or `remove` one finished run or every finished run older than `olderThanDays`.
pub(super) async fn operation(engine: &WorkflowEngine, operation: &str, args: &Value) -> Result<Value, String> {
    let text = |key: &str| args[key].as_str().map(str::to_string).ok_or_else(|| format!("{key} is required"));
    let answer = match operation {
        "markFinished" => engine.mark_finished(&text("runId")?, &text("note")?, "owner", &text("idempotencyKey")?).await
            .and_then(|run| serde_json::to_value(run).map_err(|error| WorkflowError::Ledger(error.to_string()))),
        "remove" => match args["olderThanDays"].as_u64() {
            Some(days) => engine.remove_finished_runs_older_than(days),
            None => { let id = text("runId")?; engine.remove_run(&id).map(|()| vec![id]) }
        }.map(|ids| json!(ids)),
        other => return Err(format!("Unknown workflow operation {other}")),
    };
    answer.map_err(|error| error.to_string())
}

impl WorkflowEngine {
    /// The owner ends a run by hand with a one-line note. The run becomes Completed with a `closed-by-owner` event and
    /// every agent of it is cancelled. `append` keeps a dirty or unmerged worktree and records its status; no merge
    /// step completes, so the run never counts as merged.
    pub async fn mark_finished(&self, run_id: &str, note: &str, origin: &str, idempotency_key: &str) -> Result<WorkflowRunRecord, WorkflowError> {
        if origin != "owner" {
            return Err(WorkflowError::PolicyDenied("Only the owner can mark a run finished by hand".into()));
        }
        let note = note.trim();
        if note.is_empty() || note.contains('\n') {
            return Err(WorkflowError::InvalidTransition("Marking a run finished needs a one-line note".into()));
        }
        if let Some(existing) = self.idempotent_run(run_id, idempotency_key)? {
            return Ok(existing);
        }
        let mut run = self.require_run(run_id)?;
        if ended(&run) {
            return Err(WorkflowError::InvalidTransition("This run has already ended".into()));
        }
        // A `run` step's shell command is not an agent session, so cancelling cannot stop it.
        if run.nodes.iter().any(|node| matches!(node.state, WorkflowNodeState::Starting | WorkflowNodeState::Running)
            && run.definition.nodes.iter().any(|step| step.id == node.node_id && step.step.get("run").is_some())) {
            return Err(WorkflowError::InvalidTransition("A command step is still running; wait for it to end, or Stop the run first".into()));
        }
        let now = self.clock.now_ms();
        for node in &mut run.nodes {
            if matches!(node.state, WorkflowNodeState::Queued | WorkflowNodeState::Starting | WorkflowNodeState::Running
                | WorkflowNodeState::WaitingInput | WorkflowNodeState::WaitingApproval) {
                self.runtime.cancel(&node.owned_id).await?;
            }
            if !matches!(node.state, WorkflowNodeState::Completed | WorkflowNodeState::Skipped | WorkflowNodeState::Failed) {
                node.state = WorkflowNodeState::Cancelled;
                node.finished_at_ms = Some(now);
            }
            if let Some(lease_id) = node.lease_id.take() {
                self.leases.release(&lease_id)?;
            }
        }
        run.state = WorkflowRunState::Completed;
        run.input_snapshot["closed_by_owner"] = json!({"note": note, "at_ms": now});
        self.append(&mut run, "workflow.run.closed-by-owner", idempotency_key, None)?;
        Ok(run)
    }

    /// Deletes a finished run's events and its proof folder. The worktree is left alone.
    pub fn remove_run(&self, run_id: &str) -> Result<(), WorkflowError> {
        let run = self.require_run(run_id)?;
        if !ended(&run) {
            return Err(WorkflowError::InvalidTransition("Only a finished run can be removed".into()));
        }
        // Only a folder the engine made for proof (`workflow-proof/<id>`); a run's input could name any folder.
        if let Some(proof) = run.input_snapshot["run"]["proof_dir"].as_str().map(Path::new)
            .filter(|proof| proof.parent().and_then(Path::file_name).is_some_and(|name| name == "workflow-proof")) {
            match std::fs::remove_dir_all(proof) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(WorkflowError::Runtime(error.to_string())),
                _ => {}
            }
        }
        self.store.delete_workflow_run_events(run_id).map_err(|error| WorkflowError::Ledger(error.to_string()))?;
        Ok(())
    }

    /// Removes every finished run last changed more than `days` days ago; returns the ids removed.
    pub fn remove_finished_runs_older_than(&self, days: u64) -> Result<Vec<String>, WorkflowError> {
        let cutoff = self.clock.now_ms().saturating_sub(days.saturating_mul(DAY_MS));
        let ids = self.store.finished_workflow_runs_before(cutoff).map_err(|error| WorkflowError::Ledger(error.to_string()))?;
        for id in &ids { self.remove_run(id)?; }
        Ok(ids)
    }
}
