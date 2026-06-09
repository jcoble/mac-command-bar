use mcb_core::dispatcher::dispatch;
use mcb_core::protocol::CoreRequest;
use serde_json::json;

#[test]
fn unknown_action_returns_error_with_same_id() {
    let response = dispatch(CoreRequest {
        id: "req-unknown".to_string(),
        action: "unknown.action".to_string(),
        dry_run: true,
        payload: json!({}),
    });

    assert_eq!(response.id, "req-unknown");
    assert!(!response.ok);
    assert!(response.summary.contains("unknown"));
}

#[test]
fn core_request_uses_camel_case_wire_keys() {
    let request = CoreRequest {
        id: "req-1".to_string(),
        action: "scan.worktrees".to_string(),
        dry_run: true,
        payload: json!({ "repoPath": "/tmp/repo" }),
    };

    let encoded = serde_json::to_string(&request).unwrap();

    assert!(encoded.contains("\"dryRun\":true"));
    assert!(encoded.contains("\"repoPath\""));
}
