//! The main orchestrator decides paused steps (TSK-1577): only the crowned session may, each
//! decision records who made it and why, and gates stay the owner's.
use super::*;

fn decide(endpoint: &MessagingEndpoint, token: &str, run_id: &str, step: &str, why: Value) -> Result<Value, String> {
    tool(endpoint, token, "decide_step", json!({"run_id":run_id,"step":step,"decision":"accept","why":why}))
}

#[test]
fn the_main_orchestrator_decides_a_paused_step_and_the_run_records_it() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = agent_endpoint(&engine, &repo.0);
    let token = agent(&engine, &endpoint, "orch", &repo.0);
    engine.store.set_main_orchestrator(Some("orch")).unwrap();
    let run = loop_stopped_run(&engine, &repo.0);
    // A decision without a why is refused and changes nothing.
    assert!(decide(&endpoint, &token, &run.id, "review", json!(" ")).is_err());
    assert_eq!(engine.get_run(&run.id).unwrap().state, WorkflowRunState::WaitingInput);
    let decided = decide(&endpoint, &token, &run.id, "review", json!("Both findings are wording nits the spec allows.")).unwrap();
    assert_eq!(decided["state"], "completed");
    let answer = json!({"step":"review","decision":"accept","reply":null,"by":"orchestrator","why":"Both findings are wording nits the spec allows."});
    assert_eq!(engine.get_run(&run.id).unwrap().input_snapshot["answers"], json!([answer]));
    let history = engine.store.list_workflow_run_history(&run.id).unwrap();
    let recorded = history.iter().find(|row| row.kind == "workflow.node.owner-answered").unwrap();
    assert_eq!(serde_json::to_value(recorded).unwrap()["answer"], answer);
}

#[test]
fn sessions_other_than_the_main_orchestrator_cannot_decide() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = agent_endpoint(&engine, &repo.0);
    agent(&engine, &endpoint, "orch", &repo.0);
    engine.store.set_main_orchestrator(Some("orch")).unwrap();
    let run = loop_stopped_run(&engine, &repo.0);
    let top = agent(&engine, &endpoint, "top", &repo.0);
    let helper = agent(&engine, &endpoint, "helper", &repo.0);
    let mut row = engine.store.get_session("helper").unwrap().unwrap();
    row.extra_json = r#"{"helperOf":"orch"}"#.into();
    engine.store.upsert_session(&row).unwrap();
    // The run's own build step session is a workflow worker.
    let worker = endpoint.issue(&step_of(&run, "build").owned_id).token;
    endpoint.bind_store(&worker, &engine.store, None);
    assert_eq!(engine.store.session_role(&step_of(&run, "build").owned_id).unwrap(), "worker");
    for token in [top, helper, worker] {
        let error = decide(&endpoint, &token, &run.id, "review", json!("Looks fine.")).unwrap_err();
        assert!(error.contains("main orchestrator"), "{error}");
    }
    assert_eq!(engine.get_run(&run.id).unwrap().state, WorkflowRunState::WaitingInput);
}

#[test]
fn a_gate_stays_the_owners_even_for_the_main_orchestrator() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = agent_endpoint(&engine, &repo.0);
    let token = agent(&engine, &endpoint, "orch", &repo.0);
    engine.store.set_main_orchestrator(Some("orch")).unwrap();
    let run = step_run(&engine, &repo.0, json!({"gate":{"prompt":"Approve the slice?","options":["yes","no"]}}));
    let run = tauri::async_runtime::block_on(engine.start(&run.id, "start")).unwrap();
    let error = decide(&endpoint, &token, &run.id, "a", json!("The slice matches the plan.")).unwrap_err();
    assert!(error.contains("owner"), "{error}");
    assert_eq!(engine.get_run(&run.id).unwrap().nodes[0].state, WorkflowNodeState::WaitingApproval);
}

#[test]
fn a_stop_by_the_main_orchestrator_is_recorded_with_its_why() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = agent_endpoint(&engine, &repo.0);
    let token = agent(&engine, &endpoint, "orch", &repo.0);
    engine.store.set_main_orchestrator(Some("orch")).unwrap();
    let run = loop_stopped_run(&engine, &repo.0);
    let stopped = tool(&endpoint, &token, "decide_step", json!({"run_id":run.id,"step":"review","decision":"stop","why":"The task needs splitting."})).unwrap();
    assert_eq!(stopped["state"], "cancelled");
    assert_eq!(engine.get_run(&run.id).unwrap().input_snapshot["answers"][0]["why"], "The task needs splitting.");
}
