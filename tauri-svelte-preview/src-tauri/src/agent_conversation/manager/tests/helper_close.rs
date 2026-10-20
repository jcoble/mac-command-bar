use super::*;
use super::super::helper_close::HELPER_CLOSE_AFTER_MS;

#[tokio::test(flavor = "current_thread")]
async fn a_finished_helper_is_marked_done_once_and_the_rail_hears_it() {
    let fixture = fixture_manager_with_acp_session("multiplex").await;
    let (sender, mut events) = tokio::sync::mpsc::unbounded_channel();
    fixture.manager.set_emitter(Arc::new(move |event| { let _ = sender.send(event); }));
    let helper = fixture.manager.start_helper(&fixture.owned_id, AgentConversationProvider::Codex,
        None, "H", "Reply OK", None).await.unwrap();
    // The parent's completed reply turn follows delivery of the helper's finished report.
    while let Some(event) = events.recv().await {
        if event.owned_id == fixture.owned_id && matches!(event.payload,
            AgentConversationPayload::Turn { state: super::super::super::protocol::TurnState::Completed, .. }) { break; }
    }
    assert!(fixture.manager.store.pending_for(&fixture.owned_id).unwrap().is_none());
    let now = store_timestamp(timestamp_millis());
    assert_eq!(fixture.manager.close_finished_helpers(now + HELPER_CLOSE_AFTER_MS - 1000).unwrap(), 0);
    assert_eq!(fixture.manager.close_finished_helpers(now + HELPER_CLOSE_AFTER_MS + 1).unwrap(), 1);
    let row = fixture.manager.store.get_session(&helper).unwrap().unwrap();
    let stamp = serde_json::from_str::<Value>(&row.extra_json).unwrap()["railMeta"]["completedAt"].as_str().unwrap().to_string();
    let event = fixture.manager.snapshot(&helper).unwrap().unwrap().events.into_iter()
        .find(|event| serde_json::to_value(&event.payload).unwrap()["kind"] == "sessionCompleted").unwrap();
    assert_eq!(serde_json::to_value(&event.payload).unwrap()["completedAt"], stamp);
    let mut emitted = false;
    while let Ok(event) = events.try_recv() {
        if event.owned_id == helper && serde_json::to_value(&event.payload).unwrap()["kind"] == "sessionCompleted" { emitted = true; }
    }
    assert!(emitted, "the rail must hear completion without opening the helper");
    let mut meta = fixture.manager.sessions.lock().unwrap().get(&helper).unwrap().rail_meta.clone();
    meta.completed_at = None;
    fixture.manager.update_session_meta(UpdateAgentConversationSessionMetaRequest {
        owned_id: helper.clone(), model: None, effort: None, meta,
    }).unwrap();
    assert_eq!(fixture.manager.close_finished_helpers(now + 2 * HELPER_CLOSE_AFTER_MS).unwrap(), 0);
    let generation = fixture.manager.sessions.lock().unwrap().get(&helper).unwrap().generation;
    fixture.manager.close(&helper, generation).await.unwrap();
    fixture.manager.close(&fixture.owned_id, fixture.generation).await.unwrap();
    fs::remove_dir_all(fixture.root).unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn helper_close_reads_the_minutes_setting_each_sweep_and_zero_disables_it() {
    let fixture = fixture_manager_with_acp_session("multiplex").await;
    {
        let mut sessions = fixture.manager.sessions.lock().unwrap();
        let session = sessions.get_mut(&fixture.owned_id).unwrap();
        session.helper_of = Some("parent".into());
        persist_session(session).unwrap();
    }
    let now = store_timestamp(timestamp_millis());
    fixture.manager.write_app_setting("helper_auto_done_minutes", "0").unwrap();
    assert_eq!(fixture.manager.close_finished_helpers(now + 2 * HELPER_CLOSE_AFTER_MS).unwrap(), 0);
    fixture.manager.write_app_setting("helper_auto_done_minutes", "1440").unwrap();
    assert_eq!(fixture.manager.close_finished_helpers(now + 2 * HELPER_CLOSE_AFTER_MS).unwrap(), 0);
    fixture.manager.write_app_setting("helper_auto_done_minutes", "15").unwrap();
    assert_eq!(fixture.manager.close_finished_helpers(now + 14 * 60_000).unwrap(), 0);
    assert_eq!(fixture.manager.close_finished_helpers(now + 16 * 60_000).unwrap(), 1);
    fixture.manager.close(&fixture.owned_id, fixture.generation).await.unwrap();
    fs::remove_dir_all(fixture.root).unwrap();
}
