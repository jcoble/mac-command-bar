//! Every agent and review step reports a typed result through `report_result`, checked by the engine (TSK-1609).
use super::*;

const SPEC_BUILD: &str = "  spec: { result: spec, agent: { provider: codex }, prompt: prompts/p.md }\n  build: { needs: [spec], result: build, agent: { provider: codex }, prompt: prompts/p.md }\n";

fn reporter(engine: &WorkflowEngine) -> Arc<MessagingEndpoint> {
    let endpoint = MessagingEndpoint::start().unwrap();
    endpoint.set_workflow(engine.clone());
    endpoint
}
/// The token of the session running step `id`.
fn token_of(engine: &WorkflowEngine, endpoint: &MessagingEndpoint, run_id: &str, id: &str) -> String {
    let token = endpoint.issue(&step_of(&engine.require_run(run_id).unwrap(), id).owned_id).token;
    endpoint.bind_store(&token, &engine.store, None);
    token
}
fn report(engine: &WorkflowEngine, endpoint: &MessagingEndpoint, run_id: &str, id: &str, result: Value) -> Result<Value, String> {
    let token = token_of(engine, endpoint, run_id, id);
    tauri::async_runtime::block_on(endpoint.handle(&token, json!({"name":"report_result","arguments":result})))
}
/// `tools/list` over HTTP, as an agent sees it.
fn listed(endpoint: &MessagingEndpoint, token: &str) -> Vec<Value> {
    let url = endpoint.issue("lister").url;
    tauri::async_runtime::block_on(async {
        let body: Value = reqwest::Client::new().post(url).bearer_auth(token).header("Accept", "application/json, text/event-stream")
            .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}})).send().await.unwrap().json().await.unwrap();
        body["result"]["tools"].as_array().unwrap().clone()
    })
}
fn report_schema(endpoint: &MessagingEndpoint, token: &str) -> Option<Value> {
    listed(endpoint, token).into_iter().find(|tool| tool["name"] == "report_result").map(|tool| tool["inputSchema"].clone())
}
fn head(root: &Path) -> String {
    let output = std::process::Command::new("git").args(["rev-parse", "HEAD"]).current_dir(root).output().unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
fn commit(repo: &RunRepo, message: &str) -> String {
    repo.git(&["-c", "user.name=Test", "-c", "user.email=test@test", "commit", "--allow-empty", "-m", message]);
    head(&repo.0)
}
fn edge(id: &str) -> Value { json!({"id":id,"text":"a refused write shows its error","edge":true,"restart":true,"proved_by":"app"}) }
/// A valid spec report for `repo`, with its file outside the checkout and one line to reuse.
fn spec_report(repo: &RunRepo) -> Value {
    let path = repo.0.parent().unwrap().join("spec.md");
    std::fs::write(&path, "The spec").unwrap();
    std::fs::write(repo.0.join("reused.rs"), "fn a() {}\nfn b() {}\n").unwrap();
    json!({"path":path,"summary":"Spec","requirements":["one"],"acceptance":[
        {"id":"a1","text":"it works","proved_by":"test:src/a.rs"}, edge("a2")],
        "reuse":["reused.rs:2"],"budget":"+20 lines","assumptions":[]})
}

#[test]
fn report_result_schema_matches_the_step_contract() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    let run = loop_run(&engine, &repo.0, LOOP_REVIEW);
    let schema = report_schema(&endpoint, &token_of(&engine, &endpoint, &run.id, "build")).unwrap();
    assert!(schema["properties"]["commits"].is_object(), "{schema}");
    end_turn(&engine, &run.id, "build", &[built()]);
    let schema = report_schema(&endpoint, &token_of(&engine, &endpoint, &run.id, "review")).unwrap();
    assert!(schema["properties"]["verdict"].is_object() && schema["properties"]["commits"].is_null(), "{schema}");
}

#[test]
fn a_report_missing_a_field_is_rejected_with_that_field() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    let run = loop_run(&engine, &repo.0, LOOP_REVIEW);
    let error = report(&engine, &endpoint, &run.id, "build", json!({"tests_added":[],"summary":"Built"})).unwrap_err();
    assert!(error.contains("commits"), "{error}");
    assert!(engine.store.review_reports(&step_of(&run, "build").id).unwrap().is_empty());
}

#[test]
fn a_rejected_report_can_be_retried_and_then_passes() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    let run = loop_run(&engine, &repo.0, LOOP_REVIEW);
    assert!(report(&engine, &endpoint, &run.id, "build", json!({"summary":"Built"})).is_err());
    report(&engine, &endpoint, &run.id, "build", built()).unwrap();
    let reviewing = end_turn(&engine, &run.id, "build", &[]);
    assert_eq!(step_of(&reviewing, "build").state, WorkflowNodeState::Completed);
    assert_eq!(step_of(&reviewing, "build").structured_output.as_ref().unwrap()["summary"], "Built.");
}

#[test]
fn a_turn_that_ends_without_a_report_fails_no_report_after_one_reminder() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let run = loop_run(&engine, &repo.0, LOOP_REVIEW);
    let reminded = end_turn(&engine, &run.id, "build", &[]);
    let build = step_of(&reminded, "build");
    assert_eq!(build.state, WorkflowNodeState::Running);
    let reminder = engine.store.pending_for(&build.owned_id).unwrap().unwrap();
    assert!(reminder.body.contains("report_result"), "{}", reminder.body);
    let failed = end_turn(&engine, &run.id, "build", &[]);
    assert_eq!(step_of(&failed, "build").state, WorkflowNodeState::Failed);
    assert_eq!(step_of(&failed, "build").failure.as_ref().unwrap().code, "no-report");
}

#[test]
fn a_build_that_changes_the_checkout_after_its_report_is_checked_again() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    let run = loop_run(&engine, &repo.0, LOOP_REVIEW);
    report(&engine, &endpoint, &run.id, "build", built()).unwrap();
    std::fs::write(repo.0.join("stray.txt"), "left behind").unwrap();
    let reminded = end_turn(&engine, &run.id, "build", &[]);
    assert_eq!(step_of(&reminded, "build").state, WorkflowNodeState::Running);
    let failed = end_turn(&engine, &run.id, "build", &[]);
    assert_eq!(step_of(&failed, "build").failure.as_ref().unwrap().code, "no-report");
}

#[test]
fn a_spec_without_an_edge_item_is_rejected() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    let run = loop_run(&engine, &repo.0, SPEC_BUILD);
    let mut spec = spec_report(&repo);
    spec["acceptance"].as_array_mut().unwrap().pop();
    let error = report(&engine, &endpoint, &run.id, "spec", spec).unwrap_err();
    assert!(error.contains("edge"), "{error}");
    report(&engine, &endpoint, &run.id, "spec", spec_report(&repo)).unwrap();
}

#[test]
fn a_spec_reuse_line_that_does_not_exist_is_rejected() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    let run = loop_run(&engine, &repo.0, SPEC_BUILD);
    for reuse in ["reused.rs:3", "missing.rs:1", "../reused.rs:1", "reused.rs"] {
        let mut spec = spec_report(&repo);
        spec["reuse"] = json!([reuse]);
        let error = report(&engine, &endpoint, &run.id, "spec", spec).unwrap_err();
        assert!(error.contains("reuse"), "{reuse}: {error}");
    }
}

#[test]
fn an_edge_item_without_fixture_or_restart_is_rejected() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    let run = loop_run(&engine, &repo.0, SPEC_BUILD);
    let mut spec = spec_report(&repo);
    spec["acceptance"][1]["restart"] = Value::Null;
    let error = report(&engine, &endpoint, &run.id, "spec", spec).unwrap_err();
    assert!(error.contains("fixture"), "{error}");
    let mut spec = spec_report(&repo);
    spec["acceptance"][0]["proved_by"] = json!("eyeballed");
    assert!(report(&engine, &endpoint, &run.id, "spec", spec).unwrap_err().contains("proved_by"));
}

#[test]
fn a_build_citing_a_commit_not_on_the_branch_is_rejected() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    let run = loop_run(&engine, &repo.0, LOOP_REVIEW);
    repo.git(&["checkout", "-q", "-b", "elsewhere"]);
    let elsewhere = commit(&repo, "elsewhere");
    repo.git(&["checkout", "-q", "main"]);
    let error = report(&engine, &endpoint, &run.id, "build", json!({"commits":[elsewhere],"tests_added":["a.rs"],"summary":"Built"})).unwrap_err();
    assert!(error.contains("commits"), "{error}");
    let new = commit(&repo, "the build");
    report(&engine, &endpoint, &run.id, "build", json!({"commits":[new],"tests_added":["a.rs"],"summary":"Built"})).unwrap();
}

#[test]
fn a_build_citing_an_old_commit_is_rejected() {
    let repo = RunRepo::new();
    let old = head(&repo.0);
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    let run = loop_run(&engine, &repo.0, LOOP_REVIEW);
    commit(&repo, "the build");
    let error = report(&engine, &endpoint, &run.id, "build", json!({"commits":[old],"tests_added":["a.rs"],"summary":"Built"})).unwrap_err();
    assert!(error.contains("commits") && error.contains("new"), "{error}");
}

#[test]
fn a_build_that_leaves_out_a_commit_it_made_is_rejected() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    let run = loop_run(&engine, &repo.0, LOOP_REVIEW);
    let first = commit(&repo, "the build");
    let second = commit(&repo, "more of the build");
    let error = report(&engine, &endpoint, &run.id, "build", json!({"commits":[],"tests_added":[],"summary":"Nothing to do"})).unwrap_err();
    assert!(error.contains("commits") && error.contains(&second), "{error}");
    let error = report(&engine, &endpoint, &run.id, "build", json!({"commits":[second],"tests_added":["a.rs"],"summary":"Built"})).unwrap_err();
    assert!(error.contains(&first), "{error}");
    report(&engine, &endpoint, &run.id, "build", json!({"commits":[&first[..8], second],"tests_added":["a.rs"],"summary":"Built"})).unwrap();
}

#[test]
fn a_build_with_a_dirty_tree_or_no_tests_is_rejected() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    let run = loop_run(&engine, &repo.0, LOOP_REVIEW);
    let new = commit(&repo, "the build");
    let error = report(&engine, &endpoint, &run.id, "build", json!({"commits":[new],"tests_added":[],"summary":"Built"})).unwrap_err();
    assert!(error.contains("tests_added"), "{error}");
    std::fs::write(repo.0.join("stray.txt"), "left behind").unwrap();
    let error = report(&engine, &endpoint, &run.id, "build", json!({"commits":[new],"tests_added":["a.rs"],"summary":"Built"})).unwrap_err();
    assert!(error.contains("clean"), "{error}");
}

#[test]
fn a_plan_with_a_cycle_is_rejected() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    let run = loop_run(&engine, &repo.0, "  plan: { result: plan, agent: { provider: codex }, prompt: prompts/p.md }\n");
    let slice = |number: u32, needs: &[u32]| json!({"number":number,"title":"slice","needs":needs,"budget":"+50","files":["a.rs"],"acceptance":[edge("a")]});
    let error = report(&engine, &endpoint, &run.id, "plan", json!({"slices":[slice(1, &[2]), slice(2, &[1])]})).unwrap_err();
    assert!(error.contains("cycle"), "{error}");
    report(&engine, &endpoint, &run.id, "plan", json!({"slices":[slice(1, &[]), slice(2, &[1])]})).unwrap();
}

#[test]
fn the_acceptance_list_comes_from_the_spec_step_only() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    // Written before the spec step starts, so the step itself leaves the checkout as it found it.
    let spec = spec_report(&repo);
    repo.git(&["add", "reused.rs"]);
    commit(&repo, "Code to reuse");
    let run = loop_run(&engine, &repo.0, SPEC_BUILD);
    report(&engine, &endpoint, &run.id, "spec", spec.clone()).unwrap();
    let building = end_turn(&engine, &run.id, "spec", &[]);
    assert_eq!(building.input_snapshot["acceptance"], spec["acceptance"]);
    let mut build = built();
    build["acceptance"] = json!([edge("forged")]);
    report(&engine, &endpoint, &run.id, "build", build).unwrap();
    let done = end_turn(&engine, &run.id, "build", &[]);
    assert_eq!(done.state, WorkflowRunState::Completed);
    assert_eq!(done.input_snapshot["acceptance"], spec["acceptance"]);
}

#[test]
fn a_caller_with_no_running_step_cannot_see_or_call_report_result() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    let token = agent(&engine, &endpoint, "bystander", &repo.0);
    assert!(report_schema(&endpoint, &token).is_none());
    assert!(!listed(&endpoint, &token).is_empty());
    let error = tauri::async_runtime::block_on(endpoint.handle(&token, json!({"name":"report_result","arguments":built()}))).unwrap_err();
    assert_eq!(error, "Only a session running a workflow step can report its result");
}

#[test]
fn a_starting_step_already_sees_and_can_call_report_result() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    let mut run = loop_run(&engine, &repo.0, "  build: { result: build, agent: { provider: codex }, prompt: prompts/p.md }\n");
    // A provider lists its tools while its session activates, before dispatch returns and the step is running.
    run.nodes.iter_mut().find(|node| node.node_id == "build").unwrap().state = WorkflowNodeState::Starting;
    engine.append(&mut run, "workflow.test.starting", "starting", Some("build")).unwrap();
    let token = token_of(&engine, &endpoint, &run.id, "build");
    assert!(report_schema(&endpoint, &token).unwrap()["properties"]["commits"].is_object());
    report(&engine, &endpoint, &run.id, "build", built()).unwrap();
}

#[test]
fn a_run_stored_before_typed_results_still_loads() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let run = loop_run(&engine, &repo.0, "  build: { result: build, agent: { provider: codex }, prompt: prompts/p.md }\n");
    for legacy in ["VerificationReceipt", "SpecComplianceReceipt"] {
        let mut stored = serde_json::to_value(&run).unwrap();
        stored["nodes"][0]["outputContract"] = json!(legacy);
        let loaded: WorkflowRunRecord = serde_json::from_value(stored).unwrap();
        // No step reports one any more, so a report against it is refused rather than guessed at.
        assert!(WorkflowPolicy::validate_output(loaded.nodes[0].output_contract, &built()).is_err(), "{legacy}");
    }
}

#[test]
fn an_agent_step_whose_result_is_not_build_fails_changed_code() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let run = loop_run(&engine, &repo.0, "  build: { result: build, agent: { provider: codex }, prompt: prompts/p.md }\n  proof: { needs: [build], result: proof, agent: { provider: codex }, prompt: prompts/p.md }\n");
    end_turn(&engine, &run.id, "build", &[built()]);
    commit(&repo, "a proof that commits");
    let failed = end_turn(&engine, &run.id, "proof", &[json!({"summary":"Proved","files":[]})]);
    assert_eq!(step_of(&failed, "proof").failure.as_ref().unwrap().code, "changed-code");
}

#[test]
fn spec_to_pr_runs_to_its_merge_step_with_typed_reports() {
    let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let dev = WorkflowEngine::validate_file(&shipped, Path::new(".assembly/workflows/dev-pipeline.yml")).unwrap();
    assert!(dev.nodes.iter().filter(|node| node.step.get("run").is_none() && node.step.get("gate").is_none()).all(|node| node.step["result"].is_string()));
    let mut definition = WorkflowEngine::validate_file(&shipped, Path::new(".assembly/workflows/spec-to-pr.yml")).unwrap();
    for node in &mut definition.nodes {
        if node.step.get("run").is_some() { node.step["run"] = json!(["true"]); }
    }
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = reporter(&engine);
    let proof = repo.0.parent().unwrap().join("proof");
    std::fs::create_dir_all(&proof).unwrap();
    let run = engine.create_run(definition, json!({"cwd":repo.0,"worktree":{"path":repo.0,"branch":"main"},
        "run":{"proof_dir":proof,"pr_body":proof.join("pr-body.md")},
        "task":{"key":"TSK-1609","title":"Typed results","body":"Spec"},"inputs":{"i_merge":false}}), "create".into()).unwrap();
    tauri::async_runtime::block_on(engine.start(&run.id, "start")).unwrap();
    let new = commit(&repo, "the build");
    let reports = [
        ("build", json!({"commits":[new],"tests_added":["src/a.rs"],"summary":"Built"})),
        ("code-review", json!({"verdict":"pass","findings":[],"head":new})),
        ("proof", json!({"summary":"Proved","files":[]})),
        ("pr-feedback", json!({"commits":[],"tests_added":[],"summary":"The bot found nothing"})),
        ("final-review", json!({"verdict":"pass","findings":[],"head":new})),
    ];
    for (id, result) in reports {
        settle(&engine, &run.id);
        report(&engine, &endpoint, &run.id, id, result).unwrap_or_else(|error| panic!("{id}: {error}"));
        end_turn(&engine, &run.id, id, &[]);
    }
    let run = settle(&engine, &run.id);
    assert_eq!(step_of(&run, "merge").state, WorkflowNodeState::Completed);
    assert_eq!(run.state, WorkflowRunState::Completed);
}
