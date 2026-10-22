use super::*;
use crate::agent_conversation::agui::ResumeEntry;
use crate::agent_conversation::protocol::{AgentConfigOptionChoice, AgentConversationProvider, PlanItem};

const THREAD: &str = "session-1";

/// One open view on one session, fed native events in sequence order.
struct Script {
    views: AguiProjector,
    sequence: i64,
}

impl Script {
    fn open() -> Self {
        let mut views = AguiProjector::default();
        views.begin_open(THREAD);
        views.finish_open(THREAD, Some((&[], &HashSet::new(), 0)));
        Self { views, sequence: 0 }
    }

    fn event(&mut self, payload: Payload) -> AgentConversationEvent {
        self.sequence += 1;
        event(self.sequence, payload)
    }

    fn send(&mut self, payload: Payload) -> Vec<Value> {
        let event = self.event(payload);
        self.views.project(&event)
    }

    fn run(&mut self, run_id: &str, parent: Option<&str>, resume: Vec<(&str, ResumeStatus, Value)>) -> Result<Vec<Answer>, String> {
        self.views.start_run(&RunAgentInput {
            thread_id: THREAD.into(), run_id: run_id.into(), parent_run_id: parent.map(str::to_string),
            resume: resume.into_iter().map(|(id, status, payload)| ResumeEntry { interrupt_id: id.into(), status, payload }).collect(),
        })
    }
}

fn event(sequence: i64, payload: Payload) -> AgentConversationEvent {
    AgentConversationEvent {
        runtime_state: None, owned_id: THREAD.into(), provider: AgentConversationProvider::Claude, generation: 2,
        sequence, timestamp_ms: 0, turn_id: Some("turn-1".into()), payload,
    }
}

fn types(chunks: &[Value]) -> Vec<&str> {
    chunks.iter().map(|chunk| chunk["type"].as_str().unwrap()).collect()
}

fn turn(state: TurnState) -> Payload { Payload::Turn { turn_id: "turn-1".into(), state } }
fn delta(id: &str, text: &str) -> Payload { Payload::AssistantDelta { item_id: id.into(), delta: text.into() } }
fn reply(id: &str, text: &str) -> Payload { Payload::AssistantMessage { item_id: id.into(), text: text.into(), completed: true, blocks: None } }
fn user(id: &str, text: &str) -> Payload { Payload::UserMessage { item_id: id.into(), text: text.into(), completed: true, attachment_ids: vec!["shot-1".into()] } }
fn tool(id: &str, state: ToolState, summary: Option<&str>, output: Option<&str>) -> Payload {
    Payload::Tool { item_id: id.into(), name: "Write".into(), state, summary: summary.map(str::to_string),
        output: output.map(str::to_string), path: summary.map(|_| "notes/hello.txt".into()), diff: None }
}
fn permission(id: &str, state: ApprovalState) -> Payload {
    let options = if state == ApprovalState::Requested {
        vec![("allow-once".into(), "Yes".into(), "allow_once".into()), ("reject".into(), "No".into(), "reject_once".into())]
    } else {
        Vec::new()
    };
    Payload::Approval { request_id: id.into(), state, summary: "Write notes/hello.txt".into(), options }
}
fn question(id: &str) -> Payload {
    let choice = |value: Value, label: &str| AgentConfigOptionChoice { value, label: label.into(), description: None };
    Payload::UserInputRequested {
        request_id: id.into(), title: "How much may this spend?".into(), description: Some("Pick a budget".into()), can_decline: true,
        fields: vec![AgentUserInputField {
            id: "budget".into(), label: "Budget".into(), description: None, required: true, kind: AgentUserInputKind::Select,
            choices: Some(vec![choice(json!(0.25), "A quarter"), choice(json!(100.0), "A hundred"), choice(json!(12345678901234567890u64), "Unlimited")]),
        }],
    }
}

/// The client starts a turn, the agent asks to write a file, and the client
/// answers through `resume`; the continuation names the paused run.
fn permission_scenario(choice: &str) -> (Script, Vec<Value>) {
    let mut script = Script::open();
    let mut chunks = Vec::new();
    script.run("run-client-1", None, Vec::new()).unwrap();
    chunks.extend(script.send(user("user-1", "Write hello into notes/hello.txt")));
    chunks.extend(script.send(turn(TurnState::Started)));
    chunks.extend(script.send(tool("tool-1", ToolState::Started, None, None)));
    chunks.extend(script.send(permission("perm-1", ApprovalState::Requested)));
    let answers = script.run("run-client-2", Some("run-client-1"), vec![("perm-1", ResumeStatus::Resolved, json!({ "optionId": choice }))]).unwrap();
    assert_eq!(answers, vec![Answer::Permission { request_id: "perm-1".into(), generation: 2, option_id: choice.into() }]);
    let accepted = if choice == "reject" { ApprovalState::Declined } else { ApprovalState::Accepted };
    chunks.extend(script.send(permission("perm-1", accepted)));
    let state = if choice == "reject" { ToolState::Failed } else { ToolState::Completed };
    let output = if choice == "reject" { "User refused permission to run tool" } else { "notes/hello.txt" };
    chunks.extend(script.send(tool("tool-1", state, Some("Write notes/hello.txt"), Some(output))));
    chunks.extend(script.send(delta("msg-1", "I created ")));
    chunks.extend(script.send(delta("msg-1", "notes/hello.txt.")));
    chunks.extend(script.send(reply("msg-1", "I created notes/hello.txt.")));
    chunks.extend(script.send(Payload::Usage { input_tokens: Some(1200), output_tokens: Some(80), used_tokens: None, context_window: None, total_tokens: Some(1280) }));
    chunks.extend(script.send(turn(TurnState::Completed)));
    (script, chunks)
}

#[test]
fn a_permission_pauses_the_run_and_the_client_resume_continues_it() {
    let (_, chunks) = permission_scenario("allow-once");
    assert_eq!(types(&chunks), [
        "RUN_STARTED", "TEXT_MESSAGE_START", "TEXT_MESSAGE_CONTENT", "TEXT_MESSAGE_END", "TOOL_CALL_START", "RUN_FINISHED",
        "RUN_FINISHED", "RUN_STARTED", "TOOL_CALL_ARGS", "TOOL_CALL_END", "TOOL_CALL_RESULT",
        "TEXT_MESSAGE_START", "TEXT_MESSAGE_CONTENT", "TEXT_MESSAGE_CONTENT", "TEXT_MESSAGE_END", "CUSTOM", "RUN_FINISHED",
    ]);
    assert_eq!(chunks[0]["runId"], "run-client-1");
    assert!(chunks[0].get("parentRunId").is_none(), "a first run has no parent field: {}", chunks[0]);
    assert_eq!(chunks[1]["metadata"]["assembly"]["attachmentIds"], json!(["shot-1"]));
    let pause = &chunks[5];
    assert_eq!(pause["outcome"]["type"], "interrupt");
    let interrupt = &pause["outcome"]["interrupts"][0];
    assert_eq!(interrupt["responseSchema"]["properties"]["optionId"]["enum"], json!(["allow-once", "reject"]));
    let binding = &interrupt["metadata"]["tanstack:interruptBinding"];
    assert_eq!(binding["interruptedRunId"], "run-client-1");
    assert_eq!(binding["generation"], 2);
    assert_eq!(binding["responseSchemaHash"], agui::response_schema_hash(&interrupt["responseSchema"]));
    // Even the client's own answer finishes the paused run at once; its next run continues it.
    assert_eq!(chunks[6]["runId"], "run-client-1");
    assert!(chunks[6].get("outcome").is_none(), "{}", chunks[6]);
    assert_eq!(chunks[7], json!({ "type": "RUN_STARTED", "threadId": THREAD, "runId": "run-client-2", "parentRunId": "run-client-1" }));
    // The reply the provider re-sends whole at the end adds nothing.
    assert_eq!(chunks[12]["delta"], "I created ");
    assert_eq!(chunks[13]["delta"], "notes/hello.txt.");
    assert_eq!(chunks[16]["usage"], json!([{ "inputTokens": 1200, "outputTokens": 80, "totalTokens": 1280, "provider": "claude" }]));
}

#[test]
fn a_refused_call_keeps_its_error_on_the_call_and_the_run_finishes_cleanly() {
    let (_, chunks) = permission_scenario("reject");
    let result = chunks.iter().find(|chunk| chunk["type"] == "TOOL_CALL_RESULT").unwrap();
    assert_eq!(result["metadata"]["tanstack"]["state"], "output-error");
    assert!(!types(&chunks).contains(&"RUN_ERROR"));
}

/// Every surface answers through the manager (`respond_permission_selection`,
/// `respond_user_input`), which records the same resolution event: the
/// approval card and the inbox or board record it locally, the phone through
/// its route, a remote backend over the wire. Each must end the paused run.
#[test]
fn an_answer_from_any_surface_ends_the_paused_run_at_once() {
    let resolutions = [
        ("permission accepted on the card, inbox or phone", permission("perm-1", ApprovalState::Accepted)),
        ("permission declined", permission("perm-1", ApprovalState::Declined)),
        ("permission expired by its session", permission("perm-1", ApprovalState::Expired)),
        ("question answered", Payload::UserInputResolved { request_id: "perm-1".into(), cancelled: false }),
    ];
    for (surface, resolution) in resolutions {
        let mut script = Script::open();
        script.send(turn(TurnState::Started));
        let request = if matches!(resolution, Payload::UserInputResolved { .. }) { question("perm-1") } else { permission("perm-1", ApprovalState::Requested) };
        let pause = script.send(request);
        let paused_run = pause[0]["runId"].as_str().unwrap().to_string();
        let answered = script.send(resolution);
        assert_eq!(types(&answered), ["RUN_FINISHED"], "{surface}");
        assert_eq!(answered[0]["runId"], paused_run.as_str(), "{surface}");
        assert!(answered[0].get("outcome").is_none(), "{surface}: {}", answered[0]);
        let continued = script.send(delta("msg-1", "Done."));
        assert_eq!(continued[0]["type"], "RUN_STARTED", "{surface}");
        assert_eq!(continued[0]["parentRunId"], paused_run.as_str(), "{surface}");
    }
}

#[test]
fn a_session_that_dies_while_a_request_waits_ends_the_paused_run() {
    for state in [ConversationConnectionState::Failed, ConversationConnectionState::Disconnected, ConversationConnectionState::Closed] {
        let mut script = Script::open();
        script.send(turn(TurnState::Started));
        let pause = script.send(permission("perm-1", ApprovalState::Requested));
        let ended = script.send(Payload::Connection { state, native_session_id: None });
        assert_eq!(types(&ended), ["RUN_FINISHED", "CUSTOM"], "{state:?}");
        assert_eq!(ended[0]["runId"], pause[0]["runId"]);
        assert_eq!(ended[0]["outcome"]["type"], "cancelled");
        assert!(script.send(permission("perm-1", ApprovalState::Expired)).is_empty(), "nothing is left open to end");
    }
    // A run with no pause ends the same way.
    let mut script = Script::open();
    script.send(turn(TurnState::Started));
    script.send(delta("msg-1", "Half a"));
    let ended = script.send(Payload::Connection { state: ConversationConnectionState::Failed, native_session_id: None });
    assert_eq!(ended[0]["type"], "RUN_FINISHED");
    assert_eq!(ended[0]["outcome"]["type"], "cancelled");
}

#[test]
fn a_second_request_joins_the_pause_and_the_first_answer_leaves_the_other_open() {
    let mut script = Script::open();
    script.send(turn(TurnState::Started));
    script.send(permission("perm-1", ApprovalState::Requested));
    let both = script.send(question("ask-1"));
    assert_eq!(both[0]["outcome"]["interrupts"].as_array().unwrap().len(), 2);
    let rest = script.send(permission("perm-1", ApprovalState::Accepted));
    assert_eq!(rest[0]["outcome"]["interrupts"][0]["id"], "ask-1");
    assert_eq!(rest[0]["runId"], both[0]["runId"]);
}

#[test]
fn a_question_takes_its_answer_as_an_object_and_a_cancel_as_cancel() {
    let mut script = Script::open();
    script.send(turn(TurnState::Started));
    let pause = script.send(question("ask-1"));
    let schema = &pause[0]["outcome"]["interrupts"][0]["responseSchema"];
    assert_eq!(schema["properties"]["budget"]["enum"], json!([0.25, 100.0, 12345678901234567890u64]));
    assert_eq!(schema["required"], json!(["budget"]));
    let answer = script.run("run-2", Some("run-1"), vec![("ask-1", ResumeStatus::Resolved, json!({ "budget": 0.25 }))]).unwrap();
    assert_eq!(answer, vec![Answer::Input { request_id: "ask-1".into(), generation: 2, action: AgentUserInputAction::Accept,
        content: BTreeMap::from([("budget".to_string(), json!(0.25))]) }]);
    let cancelled = answer_of(&script, "ask-1", ResumeStatus::Cancelled, Value::Null);
    assert_eq!(cancelled, Answer::Input { request_id: "ask-1".into(), generation: 2, action: AgentUserInputAction::Cancel, content: BTreeMap::new() });
}

fn answer_of(script: &Script, id: &str, status: ResumeStatus, payload: Value) -> Answer {
    let view = &script.views.views[THREAD];
    answer(view.requests.iter().find(|request| request.id == id).unwrap(), status, &payload).unwrap()
}

#[test]
fn resume_refuses_an_unknown_interrupt_or_an_option_that_was_not_offered() {
    let mut script = Script::open();
    script.send(turn(TurnState::Started));
    script.send(permission("perm-1", ApprovalState::Requested));
    assert!(script.run("run-2", None, vec![("perm-9", ResumeStatus::Resolved, json!({ "optionId": "allow-once" }))]).is_err());
    assert!(script.run("run-2", None, vec![("perm-1", ResumeStatus::Resolved, json!({ "optionId": "allow-forever" }))]).is_err());
    assert_eq!(answer_of(&script, "perm-1", ResumeStatus::Cancelled, Value::Null),
        Answer::Approval { request_id: "perm-1".into(), generation: 2, accept: false });
}

#[test]
fn a_legacy_approval_offers_allow_and_deny_and_is_answered_with_a_decision() {
    let mut script = Script::open();
    script.send(turn(TurnState::Started));
    let pause = script.send(Payload::Approval { request_id: "approval-1".into(), state: ApprovalState::Requested, summary: "Run ls".into(), options: Vec::new() });
    let interrupt = &pause[0]["outcome"]["interrupts"][0];
    assert_eq!(interrupt["responseSchema"]["properties"]["optionId"]["enum"], json!(["accept", "decline"]));
    assert_eq!(answer_of(&script, "approval-1", ResumeStatus::Resolved, json!({ "optionId": "accept" })),
        Answer::Approval { request_id: "approval-1".into(), generation: 2, accept: true });
}

#[test]
fn replays_and_repeated_starts_are_dropped() {
    let mut script = Script::open();
    script.send(turn(TurnState::Started));
    let first = script.event(delta("msg-1", "Hi"));
    assert_eq!(types(&script.views.project(&first)), ["TEXT_MESSAGE_START", "TEXT_MESSAGE_CONTENT"]);
    assert!(script.views.project(&first).is_empty(), "the same event again is a replay");
    assert_eq!(types(&script.send(delta("msg-1", " there"))), ["TEXT_MESSAGE_CONTENT"]);
    assert_eq!(types(&script.send(reply("msg-1", "Hi there"))), ["TEXT_MESSAGE_END"]);
    assert!(script.send(reply("msg-1", "Hi there")).is_empty(), "a reply already ended is not shown again");
}

#[test]
fn only_a_failed_turn_is_a_run_error_and_stop_cancels_the_run() {
    let mut script = Script::open();
    script.send(turn(TurnState::Started));
    script.send(delta("msg-1", "Working"));
    let failed_error = script.send(Payload::Error { code: "provider_error".into(), message: "The model is overloaded".into(), recoverable: false });
    assert_eq!(types(&failed_error), ["CUSTOM"]);
    let failed = script.send(turn(TurnState::Failed));
    assert_eq!(failed[0]["type"], "RUN_ERROR");
    assert_eq!(failed[0]["message"], "The model is overloaded");
    script.send(turn(TurnState::Started));
    script.send(permission("perm-2", ApprovalState::Requested));
    let stopped = script.send(turn(TurnState::Interrupted));
    assert_eq!(stopped.len(), 1);
    assert_eq!(stopped[0]["outcome"]["type"], "cancelled");
}

#[test]
fn child_agents_start_and_finish_once_and_other_events_are_custom() {
    let mut script = Script::open();
    script.send(turn(TurnState::Started));
    let child = |state: &str| Payload::ChildUpdate { child_id: "child-1".into(), parent_tool_call_id: "tool-9".into(), parent_id: None,
        transcript_id: None, label: Some("Explore".into()), state: state.into(), latest_activity: None };
    assert_eq!(types(&script.send(child("running"))), ["SUBAGENT_STARTED", "CUSTOM"]);
    assert_eq!(types(&script.send(child("finished"))), ["SUBAGENT_FINISHED", "CUSTOM"]);
    assert_eq!(types(&script.send(child("finished"))), ["CUSTOM"]);
    let plan = script.send(Payload::Plan { items: vec![PlanItem { text: "Write".into(), status: "pending".into() }] });
    assert_eq!(plan[0]["name"], "assembly.plan");
}

#[test]
fn reopening_replays_the_snapshot_then_the_open_request_then_live_events() {
    let snapshot = [
        event(10, turn(TurnState::Started)),
        event(11, delta("msg-1", "Let me ")),
        event(12, delta("msg-1", "check.")),
        event(13, permission("stale-1", ApprovalState::Requested)),
        event(14, permission("perm-1", ApprovalState::Requested)),
    ];
    let mut views = AguiProjector::default();
    views.begin_open(THREAD);
    // A delta that lands while the snapshot is being read waits for it.
    assert!(views.project(&event(15, delta("msg-1", " Done")).clone()).is_empty());
    let pending = HashSet::from(["perm-1".to_string()]);
    let opened = views.finish_open(THREAD, Some((&snapshot, &pending, 14)));
    assert!(!types(&opened.events).iter().any(|kind| kind.starts_with("RUN_")), "history leaves runs out");
    assert_eq!(opened.events[1]["delta"], "Let me check.", "history joins a reply's pieces");
    let reopened = opened.open_requests.unwrap();
    let interrupts = reopened["outcome"]["interrupts"].as_array().unwrap();
    assert_eq!(interrupts.len(), 1, "a request the session no longer waits on is not shown again");
    assert_eq!(interrupts[0]["id"], "perm-1");
    assert_eq!(opened.live.len(), 1);
    assert_eq!(opened.live[0].1, 15);
    assert!(views.project(&event(12, delta("msg-1", "check."))).is_empty(), "covered by the snapshot");
    // The whole reply re-sent at the end adds nothing to what the view shows.
    let tail = views.project(&event(16, reply("msg-1", "Let me check. Done")));
    assert_eq!(types(&tail), ["TEXT_MESSAGE_END"]);
}

#[test]
fn a_session_without_an_open_view_projects_nothing() {
    let mut views = AguiProjector::default();
    assert!(views.project(&event(1, turn(TurnState::Started))).is_empty());
}

/// Writes the scenarios above as chunk files for the frontend's contract test,
/// when `AGUI_FIXTURE_OUT` names a folder. `rust-chunks.jsonl` holds client
/// runs only; `rust-chunks-extra.jsonl` holds the answers from other surfaces,
/// a Stop during a pause, and a reopened view.
#[test]
fn writes_contract_fixtures() {
    let Some(folder) = std::env::var_os("AGUI_FIXTURE_OUT") else { return };
    let line = |scenario: &str, chunk: &Value| json!({ "scenario": scenario, "ownedId": THREAD, "chunk": chunk }).to_string();
    let mut runs = Vec::new();
    for (scenario, choice) in [("permission-allow", "allow-once"), ("permission-reject", "reject")] {
        runs.extend(permission_scenario(choice).1.iter().map(|chunk| line(scenario, chunk)));
    }
    let mut script = Script::open();
    script.run("run-client-3", None, Vec::new()).unwrap();
    let mut question_run = script.send(turn(TurnState::Started));
    question_run.extend(script.send(question("ask-1")));
    script.run("run-client-4", Some("run-client-3"), vec![("ask-1", ResumeStatus::Resolved, json!({ "budget": 0.25 }))]).unwrap();
    question_run.extend(script.send(Payload::UserInputResolved { request_id: "ask-1".into(), cancelled: false }));
    question_run.extend(script.send(delta("msg-2", "A quarter it is.")));
    question_run.extend(script.send(Payload::ChildUpdate { child_id: "child-1".into(), parent_tool_call_id: String::new(), parent_id: None,
        transcript_id: None, label: Some("Explore".into()), state: "running".into(), latest_activity: Some("Reading".into()) }));
    question_run.extend(script.send(Payload::Plan { items: vec![PlanItem { text: "Spend".into(), status: "completed".into() }] }));
    question_run.extend(script.send(turn(TurnState::Completed)));
    question_run.extend(script.send(turn(TurnState::Started)));
    question_run.extend(script.send(Payload::Error { code: "provider_error".into(), message: "Overloaded".into(), recoverable: false }));
    question_run.extend(script.send(turn(TurnState::Failed)));
    runs.extend(question_run.iter().map(|chunk| line("question-then-failed-turn", chunk)));

    let mut extra = Vec::new();
    let mut other = Script::open();
    for payload in [turn(TurnState::Started), permission("perm-1", ApprovalState::Requested), permission("perm-1", ApprovalState::Accepted),
        delta("msg-3", "Done."), reply("msg-3", "Done."), turn(TurnState::Completed), turn(TurnState::Started),
        permission("perm-2", ApprovalState::Requested), turn(TurnState::Interrupted)] {
        extra.extend(other.send(payload).iter().map(|chunk| line("other-surface-and-stop", chunk)));
    }
    let snapshot = [event(10, turn(TurnState::Started)), event(11, delta("msg-4", "Let me check.")), event(12, permission("perm-3", ApprovalState::Requested))];
    let mut views = AguiProjector::default();
    views.begin_open(THREAD);
    let opened = views.finish_open(THREAD, Some((&snapshot, &HashSet::from(["perm-3".to_string()]), 12)));
    extra.extend(opened.events.iter().chain(opened.open_requests.iter()).map(|chunk| line("reopened-view", chunk)));
    let folder = std::path::PathBuf::from(folder);
    std::fs::write(folder.join("rust-chunks.jsonl"), runs.join("\n") + "\n").unwrap();
    std::fs::write(folder.join("rust-chunks-extra.jsonl"), extra.join("\n") + "\n").unwrap();
}
