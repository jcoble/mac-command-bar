use super::*;

#[tokio::test(flavor = "current_thread")]
async fn saved_claude_model_aliases_still_open_after_a_restart() {
    for model in ["opus", "claude-opus-5", "claude-fable-5"] {
        let root = temp_root();
        let database = root.join("sessions.db");
        let log = root.join("saved-model.jsonl");
        let manifest = super::super::super::providers::acp_client::tests::fixture_manifest_named(
            &log,
            "suspend_saved_model",
        );
        let providers = || {
            ProviderRegistry::new([(AgentConversationProvider::Claude, manifest.clone())]).unwrap()
        };
        let manager = AgentRuntimeManager::open(providers(), &database).unwrap();
        let connection = manager
            .ensure_async(request(
                root.to_str().unwrap(),
                "saved-model",
                AgentConversationProvider::Claude,
            ))
            .await
            .unwrap();
        manager
            .activate(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        manager
            .update_session_meta(UpdateAgentConversationSessionMetaRequest {
                owned_id: connection.owned_id.clone(),
                model: Some(model.into()),
                effort: None,
                meta: renamed_meta("Saved model"),
            })
            .unwrap();
        manager
            .suspend_if_quiescent(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        drop(manager);
        let recovered = AgentRuntimeManager::open(providers(), &database).unwrap();
        assert_eq!(
            recovered
                .conversation_config(&connection.owned_id)
                .unwrap()
                .model
                .as_deref(),
            Some(model)
        );
        recovered
            .activate(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        let frames = fs::read_to_string(&log).unwrap();
        assert!(
            frames.contains("session/resume"),
            "the saved native session must resume: {frames}"
        );
        assert!(
            frames.contains(&format!("\"model\":\"{model}\"")),
            "the saved model must reach the adapter: {frames}"
        );
        recovered
            .close(&connection.owned_id, connection.generation)
            .await
            .unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
