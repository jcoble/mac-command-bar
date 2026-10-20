use super::*;
use std::io::Write;

/// An imported session whose transcript has since grown by a line Assembly's own run wrote.
fn imported_and_grown(fixture: &FixtureManager) -> (std::path::PathBuf, u64) {
    let transcript = fixture.root.join("imported.jsonl");
    fs::write(&transcript, "{\"type\":\"user\"}\n").unwrap();
    let store = fixture.manager.store();
    let mut row = store.get_session(&fixture.owned_id).unwrap().unwrap();
    let mut extra: Value = serde_json::from_str(&row.extra_json).unwrap();
    extra["import"] = json!({ "transcriptPath": transcript, "cutoffOffset": 0, "reachedStart": true, "importedLength": 16 });
    row.extra_json = extra.to_string();
    store.upsert_session(&row).unwrap();
    // Claude appends bookkeeping like this whenever Assembly loads or runs the session.
    fs::OpenOptions::new().append(true).open(&transcript).unwrap().write_all(b"{\"type\":\"cost-state\"}\n").unwrap();
    let grown = fs::metadata(&transcript).unwrap().len();
    (transcript, grown)
}

fn saved_length(fixture: &FixtureManager) -> Option<u64> {
    let row = fixture.manager.store().get_session(&fixture.owned_id).unwrap().unwrap();
    serde_json::from_str::<Value>(&row.extra_json).unwrap()["import"]["importedLength"].as_u64()
}

#[tokio::test(flavor = "current_thread")]
async fn a_finished_turn_moves_the_saved_transcript_length_along() {
    let fixture = fixture_manager_with_acp_session("import_length_turn").await;
    let (transcript, grown) = imported_and_grown(&fixture);
    let (sender, mut events) = tokio::sync::mpsc::unbounded_channel();
    fixture.manager.set_emitter(Arc::new(move |event: AgentConversationEvent| { let _ = sender.send(event.payload); }));
    fixture.manager.prompt(&fixture.owned_id, fixture.generation, test_prompt("hello")).await.expect("prompt starts");
    // The turn's end is emitted after the length is saved.
    while let Some(payload) = events.recv().await {
        if matches!(payload, AgentConversationPayload::Turn { state: crate::agent_conversation::protocol::TurnState::Completed, .. }) { break; }
    }
    assert_eq!(saved_length(&fixture), Some(grown));

    let native = fixture.manager.snapshot(&fixture.owned_id).unwrap().unwrap().connection.native_session_id.unwrap();
    let imported = fixture.manager.begin_import_transcript_session(AgentConversationProvider::Codex, &native,
        &transcript, fixture.root.to_str().unwrap(), None, None).await;
    assert_eq!(imported, Ok((fixture.owned_id.clone(), false)));
}

#[tokio::test(flavor = "current_thread")]
async fn warming_moves_the_saved_transcript_length_along() {
    let fixture = fixture_manager_with_acp_session("suspend_runtime").await;
    let (_, grown) = imported_and_grown(&fixture);
    fixture.manager.warm_conversation_config(&fixture.owned_id, fixture.generation).await.expect("warm config");
    assert_eq!(saved_length(&fixture), Some(grown));
}

#[tokio::test(flavor = "current_thread")]
async fn suspending_moves_the_saved_transcript_length_along() {
    let fixture = fixture_manager_with_acp_session("suspend_runtime").await;
    let (_, grown) = imported_and_grown(&fixture);
    assert!(fixture.manager.suspend_if_quiescent(&fixture.owned_id, fixture.generation).await.unwrap());
    assert_eq!(saved_length(&fixture), Some(grown));
}

#[test]
fn two_readers_extending_one_import_at_once_read_each_page_once() {
    let directory = std::env::temp_dir().join(format!("extend-at-once-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&directory).unwrap();
    let path = directory.join("rollout.jsonl");
    let meta = "{\"type\":\"session_meta\",\"payload\":{\"id\":\"native\",\"cwd\":\"/work\",\"thread_source\":\"user\",\"source\":\"cli\"}}\n";
    let text = "x".repeat(20_000);
    let turns = (1..=250).map(|index| format!("{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"id\":\"item-{index}\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"{text}\"}}]}}}}\n"));
    fs::write(&path, std::iter::once(meta.to_string()).chain(turns).collect::<String>()).unwrap();
    let manager = AgentRuntimeManager::open(ProviderRegistry::default(), &directory.join("sessions.db")).unwrap();
    let (owned_id, _) = crate::agent_conversation::history::begin_import_session(&manager.store, AgentConversationProvider::Codex,
        "native", &path, "/work", None, None, false).unwrap();
    manager.finish_import_transcript_session(&owned_id, crate::agent_conversation::IMPORT_MAX_BYTES, crate::agent_conversation::IMPORT_MAX_RECORDS).unwrap();
    // Two Macs downloading the same session each read older pages in as their saved rows run out.
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| for _ in 0..2 { scope.spawn(|| { barrier.wait();
        while !manager.extend_imported_session(&owned_id, crate::agent_conversation::IMPORT_MAX_BYTES, crate::agent_conversation::IMPORT_MAX_RECORDS).unwrap().reached_start {}
    }); });
    assert_eq!(manager.store.history_download_facts(&owned_id, None).unwrap().row_count, 250);
    fs::remove_dir_all(directory).unwrap();
}
