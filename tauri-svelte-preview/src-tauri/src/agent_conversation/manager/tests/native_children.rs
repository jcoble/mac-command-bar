use super::*;

#[tokio::test(flavor = "current_thread")]
async fn rfd_children_keep_nested_routing_titles_and_terminal_state_for_both_providers() {
    for provider in [AgentConversationProvider::Claude, AgentConversationProvider::Codex] {
        let fixture = fixture_manager_with_provider("rfd-children", provider, None).await;
        let mut sessions = fixture.manager.sessions.lock().unwrap();
        let session = sessions.get_mut(&fixture.owned_id).unwrap();
        let root = session.native_session_id.clone().unwrap();
        let announce = json!({"sessionId": root, "update": {"sessionUpdate": "subagent_update", "sessionId": "child", "capabilities": {}, "title": "Reviewer", "description": "Read notes", "state": {"state": "running"}}});
        let payload = native_child_payload(session, &announce).expect("RFD child announcement");
        assert!(matches!(payload, AgentConversationPayload::ChildUpdate {child_id, parent_id: Some(parent), label: Some(label), state, ..}
            if child_id == "child" && parent == root && label == "Reviewer" && state == "running"));
        assert_eq!(session.background_work.len(), 1);
        let title = json!({"sessionId": root, "update": {"sessionUpdate":"subagent_update", "sessionId":"child", "title":"Updated reviewer"}});
        assert!(matches!(native_child_payload(session, &title), Some(AgentConversationPayload::ChildUpdate {label: Some(label), state, ..}) if label == "Updated reviewer" && state == "running"));
        let nested = json!({"sessionId": "child", "update": {"sessionUpdate": "subagent_update", "sessionId": "nested", "title": "Reader", "state": {"state": "running"}}});
        assert!(matches!(native_child_payload(session, &nested), Some(AgentConversationPayload::ChildUpdate {parent_id: Some(parent), ..}) if parent == "child"));
        assert!(is_known_native_child_inbound(session, &json!({"sessionId":"nested", "update":{"sessionUpdate":"agent_message_chunk"}})));
        for (parent, child) in [("child", "nested"), (root.as_str(), "child")] {
            let done = json!({"sessionId": parent, "update": {"sessionUpdate": "subagent_update", "sessionId": child, "state": {"state": "idle", "stopReason": "end_turn"}}});
            assert!(matches!(native_child_payload(session, &done), Some(AgentConversationPayload::ChildUpdate {state, ..}) if state == "finished"));
        }
        assert!(session.background_work.is_empty());
        for (reason, expected) in [("error", "failed"), ("cancelled", "cancelled")] {
            let ended = json!({"sessionId":root,"update":{"sessionUpdate":"subagent_update","sessionId":"ended","state":{"state":"idle","stopReason":reason}}});
            assert!(matches!(native_child_payload(session, &ended), Some(AgentConversationPayload::ChildUpdate {state, ..}) if state == expected));
            assert!(session.background_work.is_empty());
        }
        drop(sessions);
        fixture.manager.close(&fixture.owned_id, fixture.generation).await.unwrap();
        fs::remove_dir_all(fixture.root).unwrap();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn air_children_are_native_for_codex_too() {
    let fixture = fixture_manager_with_provider("air-codex", AgentConversationProvider::Codex, None).await;
    let mut sessions = fixture.manager.sessions.lock().unwrap();
    let session = sessions.get_mut(&fixture.owned_id).unwrap();
    let root = session.native_session_id.clone().unwrap();
    assert!(native_child_payload(session, &json!({"sessionId":root,"update":{"sessionUpdate":"subagent_spawned","subagentSessionId":"child","name":"Reader"}})).is_some());
    drop(sessions);
    fixture.manager.close(&fixture.owned_id, fixture.generation).await.unwrap();
    fs::remove_dir_all(fixture.root).unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn a_parent_turn_can_finish_while_its_native_child_keeps_the_runtime_alive() {
    let fixture = fixture_manager_with_provider("suspend_native_child_liveness", AgentConversationProvider::Codex, None).await;
    let transport = {
        let mut sessions = fixture.manager.sessions.lock().unwrap();
        let session = sessions.get_mut(&fixture.owned_id).unwrap();
        session.active_turn_id = Some("parent-turn".into());
        session.state = AgentRuntimeState::Working;
        let root = session.native_session_id.clone().unwrap();
        native_child_payload(session, &json!({"sessionId":root,"update":{"sessionUpdate":"subagent_spawned","subagentSessionId":"child","name":"Reader"}})).expect("native child");
        Arc::downgrade(session.transport.as_ref().unwrap())
    };
    handle_ordered_session_event(OrderedSessionEvent::PromptResult {turn_id: "parent-turn".into(), result: Ok(json!({"stopReason":"end_turn"}))},
        &fixture.manager, &transport, &fixture.owned_id, fixture.generation).await;
    assert!(fixture.manager.sessions.lock().unwrap()[&fixture.owned_id].active_turn_id.is_none());
    assert!(!fixture.manager.suspend_if_quiescent(&fixture.owned_id, fixture.generation).await.unwrap());
    let (events, mut received) = mpsc::unbounded_channel();
    fixture.manager.set_emitter(Arc::new(move |event| { events.send(event).unwrap(); }));
    let (incoming, inbox) = mpsc::unbounded_channel();
    let (_ordered, ordered) = mpsc::unbounded_channel();
    let root = fixture.manager.sessions.lock().unwrap()[&fixture.owned_id].native_session_id.clone().unwrap();
    incoming.send(AcpInbound::SessionUpdate(json!({"sessionId":root,"update":{"sessionUpdate":"subagent_state_update","subagentSessionId":"child","state":"completed"}}))).unwrap();
    let pump = tokio::spawn(pump_inbound(fixture.manager.clone(), transport, inbox, ordered, fixture.owned_id.clone(), fixture.generation));
    tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(event) = received.recv().await {
            if matches!(event.payload, AgentConversationPayload::Connection {state: ConversationConnectionState::Disconnected, ..}) { break; }
        }
    }).await.expect("native child completion must suspend its idle parent");
    drop(incoming);
    pump.await.unwrap();
    assert!(fixture.manager.resource_roots().is_empty());
    fixture.manager.close(&fixture.owned_id, fixture.generation).await.unwrap();
    fs::remove_dir_all(fixture.root).unwrap();
}
