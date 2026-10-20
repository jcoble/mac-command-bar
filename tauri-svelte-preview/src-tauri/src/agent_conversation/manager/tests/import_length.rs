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
async fn turns_taken_outside_before_assembly_runs_the_session_are_still_read_again() {
    let fixture = fixture_manager_with_acp_session("suspend_runtime").await;
    assert!(fixture.manager.suspend_if_quiescent(&fixture.owned_id, fixture.generation).await.unwrap());
    // The line added while the session is suspended stands for turns taken in the CLI.
    let (transcript, _) = imported_and_grown(&fixture);
    fixture.manager.warm_conversation_config(&fixture.owned_id, fixture.generation).await.expect("warm config");
    assert_eq!(saved_length(&fixture), None, "opening it must not count those turns as read");
    let native = fixture.manager.store().get_session(&fixture.owned_id).unwrap().unwrap().native_session_id.unwrap();
    let imported = fixture.manager.begin_import_transcript_session(AgentConversationProvider::Codex, &native,
        &transcript, fixture.root.to_str().unwrap(), None, None).await;
    assert_eq!(imported, Ok((fixture.owned_id.clone(), true)));
}

#[tokio::test(flavor = "current_thread")]
async fn a_claude_session_started_here_begins_with_its_transcript_length_saved() {
    let fixture = fixture_manager_with_provider("new_claude_tracked", AgentConversationProvider::Claude, None).await;
    let row = fixture.manager.store().get_session(&fixture.owned_id).unwrap().unwrap();
    let cursor = serde_json::from_str::<Value>(&row.extra_json).unwrap()["import"].clone();
    // No transcript has been written yet, so nothing of it is counted.
    assert_eq!((cursor["importedLength"].as_u64(), cursor["reachedStart"].as_bool()), (Some(0), Some(true)));
    let codex = fixture_manager_with_acp_session("new_codex_untracked").await;
    let row = codex.manager.store().get_session(&codex.owned_id).unwrap().unwrap();
    assert!(serde_json::from_str::<Value>(&row.extra_json).unwrap().get("import").is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn a_session_tracked_from_its_start_is_read_again_only_for_turns_taken_elsewhere() {
    let fixture = fixture_manager_with_acp_session("suspend_runtime").await;
    let transcript = fixture.root.join("tracked.jsonl");
    fs::write(&transcript, "{\"type\":\"user\"}\n").unwrap();
    crate::agent_conversation::history::track_new_transcript(&fixture.manager.store(), &fixture.owned_id, Some(&transcript)).unwrap();
    assert!(fixture.manager.suspend_if_quiescent(&fixture.owned_id, fixture.generation).await.unwrap());
    let native = fixture.manager.store().get_session(&fixture.owned_id).unwrap().unwrap().native_session_id.unwrap();
    let import = || fixture.manager.begin_import_transcript_session(AgentConversationProvider::Codex, &native, &transcript, fixture.root.to_str().unwrap(), None, None);
    assert_eq!(import().await, Ok((fixture.owned_id.clone(), false)));
    fs::OpenOptions::new().append(true).open(&transcript).unwrap().write_all(b"{\"type\":\"user\",\"entrypoint\":\"cli\"}\n").unwrap();
    assert_eq!(import().await, Ok((fixture.owned_id.clone(), true)));
}

#[tokio::test(flavor = "current_thread")]
async fn suspending_moves_the_saved_transcript_length_along() {
    let fixture = fixture_manager_with_acp_session("suspend_runtime").await;
    let (_, grown) = imported_and_grown(&fixture);
    assert!(fixture.manager.suspend_if_quiescent(&fixture.owned_id, fixture.generation).await.unwrap());
    assert_eq!(saved_length(&fixture), Some(grown));
}

/// A transcript of `count` long turns, named for import (its newest page not read yet) by a manager of its own.
fn named_import(count: usize) -> (std::path::PathBuf, std::path::PathBuf, AgentRuntimeManager, String) {
    let directory = std::env::temp_dir().join(format!("extend-at-once-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&directory).unwrap();
    let path = directory.join("rollout.jsonl");
    let meta = "{\"type\":\"session_meta\",\"payload\":{\"id\":\"native\",\"cwd\":\"/work\",\"thread_source\":\"user\",\"source\":\"cli\"}}\n";
    let text = "x".repeat(20_000);
    let turns = (1..=count).map(|index| format!("{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"id\":\"item-{index}\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"{text}\"}}]}}}}\n"));
    fs::write(&path, std::iter::once(meta.to_string()).chain(turns).collect::<String>()).unwrap();
    let manager = AgentRuntimeManager::open(ProviderRegistry::default(), &directory.join("sessions.db")).unwrap();
    let (owned_id, _) = crate::agent_conversation::history::begin_import_session(&manager.store, AgentConversationProvider::Codex,
        "native", &path, "/work", None, None, false).unwrap();
    (directory, path, manager, owned_id)
}

const PAGE: (u64, usize) = (crate::agent_conversation::IMPORT_MAX_BYTES, crate::agent_conversation::IMPORT_MAX_RECORDS);

#[test]
fn older_pages_wait_for_the_newest_page_of_a_named_import() {
    let (directory, _, manager, owned_id) = named_import(3);
    let early = manager.extend_imported_session(&owned_id, PAGE.0, PAGE.1).unwrap();
    assert_eq!((early.added, early.reached_start), (0, false), "reading it is the newest page's job, which numbers it");
    assert_eq!(manager.finish_import_transcript_session(&owned_id, PAGE.0, PAGE.1).unwrap(), 3);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn reading_a_session_again_waits_for_an_older_page_being_read_in() {
    for _ in 0..5 {
        let (directory, path, manager, owned_id) = named_import(250);
        manager.finish_import_transcript_session(&owned_id, PAGE.0, PAGE.1).unwrap();
        let mut grown = fs::OpenOptions::new().append(true).open(&path).unwrap();
        grown.write_all(b"{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"id\":\"item-251\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"cli\"}]}}\n").unwrap();
        // One Mac reads an older page in while another reads the grown transcript again.
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            scope.spawn(|| { barrier.wait(); manager.extend_imported_session(&owned_id, PAGE.0, PAGE.1).unwrap(); });
            scope.spawn(|| { barrier.wait();
                let again = runtime.block_on(manager.begin_import_transcript_session(AgentConversationProvider::Codex, "native", &path, "/work", None, None));
                assert!(again.unwrap().1);
                manager.finish_import_transcript_session(&owned_id, PAGE.0, PAGE.1).unwrap();
            });
        });
        while !manager.extend_imported_session(&owned_id, PAGE.0, PAGE.1).unwrap().reached_start {}
        assert_eq!(manager.store.history_download_facts(&owned_id, None).unwrap().row_count, 251, "every turn once");
        fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn two_readers_extending_one_import_at_once_read_each_page_once() {
    let (directory, _, manager, owned_id) = named_import(250);
    manager.finish_import_transcript_session(&owned_id, PAGE.0, PAGE.1).unwrap();
    // Two Macs downloading the same session each read older pages in as their saved rows run out.
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| for _ in 0..2 { scope.spawn(|| { barrier.wait();
        while !manager.extend_imported_session(&owned_id, PAGE.0, PAGE.1).unwrap().reached_start {}
    }); });
    assert_eq!(manager.store.history_download_facts(&owned_id, None).unwrap().row_count, 250);
    fs::remove_dir_all(directory).unwrap();
}
