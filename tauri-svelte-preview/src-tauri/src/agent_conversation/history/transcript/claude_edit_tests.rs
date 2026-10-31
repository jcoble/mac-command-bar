use super::super::*;
use serde_json::json;

#[test]
fn claude_import_preserves_structured_edit_patch() {
    let line = json!({"type":"user", "sessionId":"s1",
        "message":{"content":[{"type":"tool_result", "tool_use_id":"edit-1", "content":"Updated successfully"}]},
        "toolUseResult":{"filePath":"/project/notes.txt", "structuredPatch":[
            {"oldStart":2,"oldLines":2,"newStart":2,"newLines":2,"lines":[" context","-old","+new"]},
            {"oldStart":20,"oldLines":1,"newStart":20,"newLines":1,"lines":["-before","+after"]}
        ]}}).to_string();
    let rows = parse_durable_line(AgentConversationProvider::Claude, "s1", line.as_bytes());
    let row = rows
        .iter()
        .find(|row| row.item_id.as_deref() == Some("edit-1"))
        .unwrap();
    let AgentConversationPayload::Tool {
        output, path, diff, ..
    } = row.native.as_ref().unwrap()
    else {
        panic!("tool row")
    };
    assert_eq!(output.as_deref(), Some("Updated successfully"));
    assert_eq!(path.as_deref(), Some("/project/notes.txt"));
    assert_eq!(
        diff.as_deref(),
        Some("@@ -2,2 +2,2 @@\n context\n-old\n+new\n@@ -20,1 +20,1 @@\n-before\n+after\n")
    );
    assert_eq!(
        row.payload.get("diff").and_then(Value::as_str),
        diff.as_deref()
    );
}

#[test]
fn claude_import_keeps_patch_without_result_text() {
    let line = json!({"type":"user", "sessionId":"s1",
    "message":{"content":[{"type":"tool_result", "tool_use_id":"edit-1", "content":""}]},
    "toolUseResult":{"filePath":"/project/notes.txt", "structuredPatch":[
        {"oldStart":1,"oldLines":1,"newStart":1,"newLines":1,"lines":["-old","+new"]}
    ]}})
    .to_string();
    let rows = parse_durable_line(AgentConversationProvider::Claude, "s1", line.as_bytes());
    assert!(rows.iter().any(|row| matches!(&row.native, Some(AgentConversationPayload::Tool { diff: Some(diff), .. }) if diff.contains("+new"))));
}

#[test]
fn claude_import_does_not_assign_one_patch_to_multiple_results() {
    let line = json!({"type":"user", "sessionId":"s1",
    "message":{"content":[
        {"type":"tool_result", "tool_use_id":"one", "content":"first"},
        {"type":"tool_result", "tool_use_id":"two", "content":"second"}]},
    "toolUseResult":{"filePath":"/project/notes.txt", "structuredPatch":[
        {"oldStart":1,"oldLines":1,"newStart":1,"newLines":1,"lines":["-old","+new"]}
    ]}})
    .to_string();
    let rows = parse_durable_line(AgentConversationProvider::Claude, "s1", line.as_bytes());
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| matches!(
        &row.native,
        Some(AgentConversationPayload::Tool {
            output: Some(_),
            diff: None,
            ..
        })
    )));
}
