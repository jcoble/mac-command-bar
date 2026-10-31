use super::*;

#[tokio::test(flavor = "current_thread")]
async fn both_providers_negotiate_native_child_sessions() {
    for provider in [AgentConversationProvider::Codex, AgentConversationProvider::Claude] {
        let root = fixture_root();
        let log = root.join("native-children.jsonl");
        let mut client = AcpClient::spawn(&fixture_manifest(&log), &root, "children").unwrap();
        client.initialize(provider).await.unwrap();
        client.close().await.unwrap();
        let frames = std::fs::read_to_string(&log).unwrap();
        let initialize = frames.lines().filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .find(|frame| frame["method"] == "initialize").unwrap();
        assert_eq!(initialize["params"]["clientCapabilities"]["subagents"], json!({}));
        std::fs::remove_dir_all(root).unwrap();
    }
}

    #[tokio::test(flavor = "current_thread")]
    async fn prompt_once_reads_text_directly_from_response_result() {
        let root = fixture_root();
        let log = root.join("direct.jsonl");
        let mut client = AcpClient::spawn(
            &fixture_manifest_named(&log, "direct_result"),
            &root,
            "direct",
        )
        .unwrap();
        client
            .initialize(AgentConversationProvider::Codex)
            .await
            .unwrap();
        let session = client.new_session(&root, vec![]).await.unwrap();
        let generated = client
            .prompt_on(
                &session.native_session_id,
                AgentPrompt {
                    text: "write".into(),
                    images: Vec::new(),
                    attachment_ids: Vec::new(),
                },
            )
            .await
            .unwrap();
        assert_eq!(generated.turn_id.as_deref(), Some("turn-direct"));
        assert_eq!(generated.text, "direct response text");
        client.close().await.unwrap();
        let frames = std::fs::read_to_string(&log).unwrap();
        let initialize = frames.lines().filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .find(|frame| frame["method"] == "initialize").unwrap();
        assert!(initialize["params"]["clientCapabilities"].get("elicitation").is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
