use super::*;

#[test]
fn a_running_call_keeps_its_output_and_diff_when_it_finishes() {
    let mut script = Script::open();
    let call = |state: ToolState, output: Option<&str>, diff: Option<&str>| Payload::Tool { item_id: "tool-1".into(), name: "Edit".into(), state,
        summary: None, output: output.map(str::to_string), path: Some("notes.txt".into()), diff: diff.map(str::to_string), };
    let mut chunks = script.send(turn(TurnState::Started));
    chunks.extend(script.send(call(ToolState::Started, None, None)));
    let content = |chunk: &Value| serde_json::from_str::<Value>(chunk["content"].as_str().unwrap()).unwrap();
    let running = script.send(call(ToolState::Updated, Some("Edited notes.txt"), Some("+hello")));
    assert_eq!((types(&running), content(&running[0])["output"].clone()), (vec!["TOOL_CALL_RESULT"], json!("Edited notes.txt")));
    let done = script.send(call(ToolState::Completed, None, None));
    assert_eq!(types(&done), ["TOOL_CALL_END", "TOOL_CALL_RESULT"]);
    assert_eq!(content(&done[1])["output"], "Edited notes.txt");
    assert_eq!(content(&done[1])["diff"], "+hello");
    chunks.extend(running); chunks.extend(done);
    chunks.extend(script.send(call(ToolState::Completed, Some("final output"), Some("+final"))));
    let imported = script.send(Payload::Tool { item_id: "tool-1".into(), name: String::new(), state: ToolState::Completed,
        summary: None, output: Some("2 files".into()), path: None, diff: None, });
    chunks.extend(imported.clone()); chunks.extend(script.send(turn(TurnState::Completed)));
    if let Some(folder) = std::env::var_os("AGUI_FIXTURE_OUT") {
        std::fs::write(std::path::Path::new(&folder).join("eager-tools.jsonl"), chunks.iter().map(Value::to_string).collect::<Vec<_>>().join("\n") + "\n").unwrap();
    }
    assert_eq!(content(&imported[0])["diff"], "+final");
    assert_eq!(content(&imported[0])["path"], "notes.txt");
    assert_eq!(content(&imported[0])["name"], "Edit");
    assert_eq!((types(&imported), content(&imported[0])["output"].clone()), (vec!["TOOL_CALL_RESULT"], json!("2 files")));
}
