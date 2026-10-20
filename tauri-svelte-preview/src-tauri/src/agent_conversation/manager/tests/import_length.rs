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
    fixture.manager.prompt(&fixture.owned_id, fixture.generation, test_prompt("hello")).await.expect("prompt starts");
    wait_until(|| saved_length(&fixture) == Some(grown)).await;

    let native = fixture.manager.snapshot(&fixture.owned_id).unwrap().unwrap().connection.native_session_id.unwrap();
    let imported = fixture.manager.begin_import_transcript_session(AgentConversationProvider::Codex, &native,
        &transcript, fixture.root.to_str().unwrap(), None).await;
    assert_eq!(imported, Ok((fixture.owned_id.clone(), false)));
}

#[tokio::test(flavor = "current_thread")]
async fn suspending_moves_the_saved_transcript_length_along() {
    let fixture = fixture_manager_with_acp_session("suspend_runtime").await;
    let (_, grown) = imported_and_grown(&fixture);
    assert!(fixture.manager.suspend_if_quiescent(&fixture.owned_id, fixture.generation).await.unwrap());
    assert_eq!(saved_length(&fixture), Some(grown));
}
