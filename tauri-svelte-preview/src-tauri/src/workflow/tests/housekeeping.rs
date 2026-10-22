//! Housekeeping (TSK-1606): the owner marks a run finished by hand, and removes finished runs.
use super::*;

/// A started run whose build agent is running, with its proof folder where the engine keeps proof.
fn running_run(engine: &WorkflowEngine, root: &Path) -> WorkflowRunRecord {
    std::fs::create_dir_all(root.join(".assembly/workflows/prompts")).unwrap();
    std::fs::write(root.join(".assembly/workflows/prompts/p.md"), "Do the work").unwrap();
    let path = Path::new(".assembly/workflows/loop.yml");
    std::fs::write(root.join(path), format!("name: Loop\nsteps:\n{LOOP_REVIEW}")).unwrap();
    let definition = WorkflowEngine::validate_file(root, path).unwrap();
    let proof = root.parent().unwrap().join("workflow-proof").join("run");
    std::fs::create_dir_all(&proof).unwrap();
    std::fs::write(proof.join("screenshot.png"), "png").unwrap();
    let run = engine.create_run(definition, json!({"cwd":root,"worktree":{"path":root,"branch":"main"},
        "run":{"proof_dir":proof,"pr_body":proof.join("pr-body.md")},
        "task":{"key":"TSK-1606","title":"Task title","body":"Task body"},"inputs":{}}), "create".into()).unwrap();
    let run = tauri::async_runtime::block_on(engine.start(&run.id, "start")).unwrap();
    assert_eq!(step_of(&run, "build").state, WorkflowNodeState::Running);
    run
}

fn mark(engine: &WorkflowEngine, run_id: &str, origin: &str) -> Result<WorkflowRunRecord, WorkflowError> {
    tauri::async_runtime::block_on(engine.mark_finished(run_id, "Finished by hand on the Mac", origin, "mark"))
}

#[test]
fn mark_finished_completes_the_run_with_closed_by_owner_and_cancels_its_agents() {
    let repo = RunRepo::new();
    let runtime = Arc::new(FakeRuntime::default());
    let engine = run_engine(runtime.clone());
    let run = running_run(&engine, &repo.0);
    let done = mark(&engine, &run.id, "owner").unwrap();
    assert_eq!(done.state, WorkflowRunState::Completed);
    assert_eq!(done.input_snapshot["closed_by_owner"]["note"], "Finished by hand on the Mac");
    assert_eq!(step_of(&done, "build").state, WorkflowNodeState::Cancelled);
    assert_eq!(runtime.cancelled.load(Ordering::SeqCst), 1, "the running build session is cancelled");
    let history = engine.store.list_workflow_run_history(&run.id).unwrap();
    assert_eq!(history.last().unwrap().kind, "workflow.run.closed-by-owner");
    // A run that has ended cannot be marked again.
    assert!(tauri::async_runtime::block_on(engine.mark_finished(&run.id, "again", "owner", "mark-2")).is_err());
}

#[test]
fn mark_finished_keeps_and_reports_a_dirty_worktree() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let run = running_run(&engine, &repo.0);
    std::fs::write(repo.0.join("unsaved.txt"), "work in progress").unwrap();
    let done = mark(&engine, &run.id, "owner").unwrap();
    let worktree = &done.input_snapshot["worktree"];
    assert_ne!(worktree["removed"], true);
    assert!(worktree["status"].as_str().unwrap().contains("unsaved.txt"), "{worktree}");
    assert!(repo.0.join("unsaved.txt").exists());
}

#[test]
fn only_the_owner_can_mark_a_run_finished_and_it_needs_a_one_line_note() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let run = running_run(&engine, &repo.0);
    for origin in ["agent:session-1", "orchestrator"] {
        assert!(matches!(mark(&engine, &run.id, origin), Err(WorkflowError::PolicyDenied(_))), "{origin}");
    }
    for note in [" ", "two\nlines"] {
        assert!(tauri::async_runtime::block_on(engine.mark_finished(&run.id, note, "owner", "note")).is_err(), "{note:?}");
    }
    assert_eq!(engine.get_run(&run.id).unwrap().state, WorkflowRunState::Running);
}

#[test]
fn remove_refuses_a_run_that_has_not_finished() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let run = running_run(&engine, &repo.0);
    assert!(matches!(engine.remove_run(&run.id), Err(WorkflowError::InvalidTransition(_))));
    assert_eq!(engine.get_run(&run.id).unwrap().state, WorkflowRunState::Running);
}

#[test]
fn remove_deletes_a_finished_runs_events_and_proof() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let run = running_run(&engine, &repo.0);
    let proof = std::path::PathBuf::from(run.input_snapshot["run"]["proof_dir"].as_str().unwrap());
    mark(&engine, &run.id, "owner").unwrap();
    let removed = tauri::async_runtime::block_on(execute_operation(&engine, "remove", json!({"runId": run.id}))).unwrap();
    assert_eq!(removed, json!([run.id]));
    assert!(matches!(engine.get_run(&run.id), Err(WorkflowError::NotFound(_))));
    assert!(engine.store.list_workflow_run_history(&run.id).unwrap().is_empty());
    assert!(!proof.exists());
    // The worktree is not the run's to delete.
    assert!(repo.0.exists());
}

#[test]
fn remove_leaves_a_proof_folder_outside_the_engines_proof_folders() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let elsewhere = repo.0.join("keep");
    std::fs::create_dir_all(&elsewhere).unwrap();
    let run = engine.create_run(definition(), json!({"cwd":repo.0,"run":{"proof_dir":elsewhere}}), "create".into()).unwrap();
    tauri::async_runtime::block_on(engine.cancel(&run.id, "stop")).unwrap();
    engine.remove_run(&run.id).unwrap();
    assert!(elsewhere.exists());
}

#[test]
fn remove_finished_runs_older_than_keeps_newer_and_unfinished_runs() {
    let repo = RunRepo::new();
    let clock = Arc::new(FakeClock(AtomicU64::new(1)));
    let engine = WorkflowEngine::new(Arc::new(FakeRuntime::default()), Arc::new(InMemoryWorktreeLeasePort::default()),
        clock.clone(), Arc::new(FakeIds(AtomicU64::new(1))), Arc::new(SessionStore::open_in_memory().unwrap()));
    let create = |key: &str| {
        let mut workflow = definition();
        workflow.id = key.into();
        engine.create_run(workflow, json!({"cwd": repo.0}), key.into()).unwrap()
    };
    let old = create("old");
    tauri::async_runtime::block_on(engine.cancel(&old.id, "stop")).unwrap();
    let unfinished = create("unfinished");
    const DAY: u64 = 86_400_000;
    clock.0.store(10 * DAY, Ordering::SeqCst);
    let recent = create("recent");
    tauri::async_runtime::block_on(engine.cancel(&recent.id, "stop")).unwrap();
    assert_eq!(engine.remove_finished_runs_older_than(7).unwrap(), vec![old.id.clone()]);
    let left: Vec<String> = engine.list_runs().unwrap().into_iter().map(|run| run.id).collect();
    assert!(!left.contains(&old.id));
    assert!(left.contains(&unfinished.id) && left.contains(&recent.id), "{left:?}");
}
