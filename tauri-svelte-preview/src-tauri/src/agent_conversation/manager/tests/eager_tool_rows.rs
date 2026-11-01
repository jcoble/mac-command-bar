use super::*;

/// The same diff `conversationTurnStream.test.ts` counts: a `---` line after the hunk is an added
/// or removed line, not a file name.
const EDIT_DIFF: &str = "--- a/notes.txt\n+++ b/notes.txt\n@@ -1,2 +1,3 @@\n-old line\n+new line\n+--- a removed-looking added line\n keep\n";

fn tool(item_id: &str, state: ToolState, summary: Option<&str>, output: Option<&str>, path: Option<&str>, diff: Option<&str>) -> AgentConversationPayload {
    let text = |value: Option<&str>| value.map(str::to_owned);
    AgentConversationPayload::Tool { item_id: item_id.into(), name: "Bash".into(), state, summary: text(summary), output: text(output),
        path: text(path), diff: text(diff), }
}

fn calls() -> Vec<AgentConversationPayload> {
    // Imported history keeps a call's output in a later event with no name or summary.
    let mut output_only = tool("read", ToolState::Completed, None, Some("# Notes\nbody"), None, None);
    if let AgentConversationPayload::Tool { name, .. } = &mut output_only { name.clear(); }
    vec![
        tool("run", ToolState::Started, Some("seq 3"), Some("1\n"), None, None),
        tool("run", ToolState::Completed, Some("seq 3"), Some("1\n2\n3\n"), None, None),
        tool("grep", ToolState::Completed, None, Some("a.ts:1 found\nb.ts:2 found"), None, None),
        tool("edit", ToolState::Completed, None, None, Some("notes.txt"), Some(EDIT_DIFF)),
        tool("empty", ToolState::Completed, Some("nothing"), None, None, None),
        tool("read", ToolState::Completed, Some("notes.md"), None, None, None),
        output_only,
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
fn history_pages_carry_complete_tools_for_parent_child_and_remote() {
    let (manager, root, parent, child, remote) = sessions();
    for id in [&parent, &child, &remote] {
        let page = manager.list_items_before(id, i64::MAX, 1 << 20).unwrap();
        let tools = page.events.iter().filter_map(|event| match &event.payload {
            AgentConversationPayload::Tool { item_id, output, path, diff, .. } => Some((item_id.as_str(), output.as_deref(), path.as_deref(), diff.as_deref())),
            _ => None,
        }).collect::<Vec<_>>();
        assert!(tools.contains(&("run", Some("1\n2\n3\n"), None, None)), "output missing for {id}: {tools:?}");
        assert!(tools.contains(&("edit", None, Some("notes.txt"), Some(EDIT_DIFF))), "diff missing for {id}");
        assert!(tools.contains(&("read", Some("# Notes\nbody"), None, None)), "imported output missing for {id}");
    }
    fs::remove_dir_all(root).unwrap();
}
