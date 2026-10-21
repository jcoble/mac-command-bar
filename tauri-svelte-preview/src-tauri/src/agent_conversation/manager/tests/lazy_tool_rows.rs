use super::*;
use super::super::tool_call::read_tool_call;

const EDIT_DIFF: &str = "--- a/notes.txt\n+++ b/notes.txt\n@@ -1,2 +1,3 @@\n-old line\n+new line\n+--- a removed-looking added line\n keep\n";

fn tool(item_id: &str, state: ToolState, summary: Option<&str>, output: Option<&str>, path: Option<&str>, diff: Option<&str>) -> AgentConversationPayload {
    let text = |value: Option<&str>| value.map(str::to_owned);
    AgentConversationPayload::Tool { item_id: item_id.into(), name: "Bash".into(), state, summary: text(summary), output: text(output),
        path: text(path), diff: text(diff), has_detail: false }
}

fn calls() -> Vec<AgentConversationPayload> {
    vec![
        tool("run", ToolState::Started, Some("seq 3"), Some("1\n"), None, None),
        tool("run", ToolState::Completed, Some("seq 3"), Some("1\n2\n3\n"), None, None),
        tool("grep", ToolState::Completed, None, Some("a.ts:1 found\nb.ts:2 found"), None, None),
        tool("edit", ToolState::Completed, None, None, Some("notes.txt"), Some(EDIT_DIFF)),
        tool("empty", ToolState::Completed, Some("nothing"), None, None, None),
    ]
}

/// The key a remote machine's session is saved under on this Mac (`RemoteHistory::key`).
fn remote_key(profile: &str, owned_id: &str) -> String {
    serde_json::to_string(&(profile, owned_id)).unwrap()
}

/// Saves `payloads` after the newest event of `owned_id`, as the journal or a remote session's saved copy keeps them.
fn save(store: &SessionStore, owned_id: &str, kind: &str, payloads: Vec<AgentConversationPayload>) {
    let first = store.latest_seq(owned_id).unwrap() + 1;
    for (offset, payload) in payloads.into_iter().enumerate() {
        let event = AgentConversationEvent { runtime_state: None, owned_id: owned_id.into(), provider: AgentConversationProvider::Codex,
            generation: 1, sequence: first + offset as i64, timestamp_ms: 1, turn_id: Some("turn".into()), payload };
        store.append_event(&EventRow { owned_id: owned_id.into(), seq: event.sequence, turn_id: event.turn_id.clone(),
            kind: kind.into(), payload_json: serde_json::to_string(&event).unwrap(), created_at_ms: 1 }).unwrap();
    }
}

/// Each tool row a page carries, as (item, summary, has detail), after checking it has no output.
fn closed_rows(page: &AgentConversationItemPage) -> Vec<(String, Option<String>, bool)> {
    let mut rows: Vec<(String, Option<String>, bool)> = Vec::new();
    for event in &page.events {
        if let AgentConversationPayload::Tool { item_id, summary, output, has_detail, .. } = &event.payload {
            assert_eq!(output, &None, "{item_id} must carry no output");
            rows.retain(|row| &row.0 != item_id);
            rows.push((item_id.clone(), summary.clone(), *has_detail));
        }
    }
    rows
}

fn expected_rows() -> Vec<(String, Option<String>, bool)> {
    vec![
        ("run".into(), Some("seq 3".into()), true),
        // A call with no summary keeps its output's first line as the one line its closed row shows.
        ("grep".into(), Some("a.ts:1 found".into()), true),
        ("edit".into(), None, true),
        ("empty".into(), Some("nothing".into()), false),
    ]
}

/// A local session with the tool calls saved, a sub-agent of it with the same calls, and a remote
/// session's saved copy with them too. Returns the manager, root folder and the three ids.
fn sessions() -> (AgentRuntimeManager, std::path::PathBuf, String, String, String) {
    let root = temp_root();
    let manager = AgentRuntimeManager::new(ProviderRegistry::default());
    let parent = manager.ensure_inner(request(root.to_str().unwrap(), "lazy-parent", AgentConversationProvider::Codex)).unwrap().0.owned_id;
    let mut row = manager.store.get_session(&parent).unwrap().unwrap();
    row.native_session_id = Some("native-lazy-parent".into());
    manager.store.upsert_session(&row).unwrap();
    let child = super::super::super::history::ensure_child_import(&manager.store, &parent, None, "lazy-child").unwrap();
    let remote = remote_key("workbox-test", "remote-session");
    row.owned_id = remote.clone();
    manager.store.upsert_session(&row).unwrap();
    save(&manager.store, &parent, "item.updated", calls());
    save(&manager.store, &child, "item.updated", calls());
    save(&manager.store, &remote, "remote.cached", calls());
    (manager, root, parent, child, remote)
}

#[test]
fn history_pages_carry_tool_rows_without_output() {
    let (manager, root, parent, child, _) = sessions();
    let page = manager.list_items_before(&parent, i64::MAX, 1 << 20).unwrap();
    assert_eq!(closed_rows(&page), expected_rows());
    // A sub-agent's transcript is paged the same way.
    let child_page = manager.list_items_before(&child, i64::MAX, 1 << 20).unwrap();
    assert_eq!(closed_rows(&child_page), expected_rows());
    // The saved rows keep everything.
    let saved = manager.store.list_events(&parent, 0, 100).unwrap();
    assert!(saved.iter().any(|row| row.payload_json.contains("1\\n2\\n3\\n")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn an_opened_row_reads_the_whole_call_for_local_sub_agent_and_remote_sessions() {
    let (manager, root, parent, child, remote) = sessions();
    for owned_id in [&parent, &child, &remote] {
        let run = read_tool_call(&manager.store, owned_id, "run").unwrap();
        assert_eq!((run.summary.as_deref(), run.output.as_deref(), run.diff), (Some("seq 3"), Some("1\n2\n3\n"), None), "{owned_id}");
        let edit = read_tool_call(&manager.store, owned_id, "edit").unwrap();
        assert_eq!((edit.path.as_deref(), edit.diff.as_deref()), (Some("notes.txt"), Some(EDIT_DIFF)), "{owned_id}");
    }
    assert_eq!(read_tool_call(&manager.store, &remote_key("workbox-test", "never-saved"), "run").unwrap_err(),
        "This part of the conversation is not saved on this Mac yet");
    fs::remove_dir_all(root).unwrap();
}
