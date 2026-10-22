use super::*;

#[test]
fn agent_start_workflow_passes_inputs_and_keeps_defaults() {
    let repo = RunRepo::new();
    let engine = run_engine(Arc::new(FakeRuntime::default()));
    let endpoint = agent_endpoint(&engine, &repo.0);
    std::fs::write(repo.0.join(AGENT_FLOW), format!("name: Loop\ninputs: {{ i_merge: {{ default: false }} }}\nsteps:\n{LOOP_REVIEW}")).unwrap();
    let token = agent(&engine, &endpoint, "starter", &repo.0);
    // Inputs the agent names override the workflow's defaults.
    let mut args = start_args("TSK-1");
    args["inputs"] = json!({"i_merge": true});
    let started = tool(&endpoint, &token, "start_workflow", args).unwrap();
    let run = engine.get_run(started["run_id"].as_str().unwrap()).unwrap();
    assert_eq!(run.input_snapshot["inputs"]["i_merge"], true);
    // Without inputs the defaults apply.
    let started = tool(&endpoint, &token, "start_workflow", start_args("TSK-2")).unwrap();
    let run = engine.get_run(started["run_id"].as_str().unwrap()).unwrap();
    assert_eq!(run.input_snapshot["inputs"]["i_merge"], false);
}
