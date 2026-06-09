use mcb_core::crypto::SecretBox;
use mcb_core::scanners::processes::parse_lsof_listeners;
use mcb_core::scanners::sessions::{
    decode_claude_project_dir_with_users_root, parse_claude_jsonl, parse_codex_index_jsonl,
};
use mcb_core::scanners::worktrees::parse_worktree_porcelain;

#[test]
fn parses_git_worktree_porcelain() {
    let records = parse_worktree_porcelain(
        "worktree /repo\nHEAD abc\nbranch refs/heads/main\n\nworktree /worktrees/feature\nHEAD def\nbranch refs/heads/feature\n",
    );

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].path, "/repo");
    assert_eq!(records[0].branch, "main");
    assert_eq!(records[1].branch, "feature");
}

#[test]
fn parses_lsof_tcp_listener_output() {
    let records = parse_lsof_listeners(
        "COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME\nnode    456 user   20u  IPv4 0      0t0 TCP 127.0.0.1:5173 (LISTEN)\ndotnet  789 user   33u  IPv6 0      0t0 TCP *:5001 (LISTEN)\n",
    );

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].name, "node");
    assert_eq!(records[0].pid, 456);
    assert_eq!(records[0].listening_ports, vec![5173]);
    assert_eq!(records[1].listening_ports, vec![5001]);
}

#[test]
fn parses_codex_and_claude_session_indexes() {
    let codex = parse_codex_index_jsonl(
        "{\"id\":\"019d\",\"thread_name\":\"Fix runtime\",\"updated_at\":\"2026-06-08T22:00:00Z\"}\n",
    );
    let claude = parse_claude_jsonl(
        "{\"sessionId\":\"abc\",\"cwd\":\"/repo\",\"timestamp\":\"2026-06-08T22:01:00Z\",\"message\":{\"role\":\"user\",\"content\":\"resume work\"}}\n",
        "/repo",
    );

    assert_eq!(codex[0].provider, "codex");
    assert_eq!(codex[0].title, "Fix runtime");
    assert_eq!(claude[0].provider, "claude");
    assert_eq!(claude[0].project_path.as_deref(), Some("/repo"));
}

#[test]
fn secret_box_round_trips_encrypted_content() {
    let box_ = SecretBox::new_for_tests([7; 32]);

    let encrypted = box_.encrypt(b"copied private text").unwrap();
    assert_ne!(encrypted.ciphertext, "copied private text");

    let decrypted = box_.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted, b"copied private text");
}

#[test]
fn decodes_claude_project_dirs_with_hyphenated_repo_names() {
    let temp = tempfile::tempdir().unwrap();
    let users_root = temp.path().join("Users");
    std::fs::create_dir_all(users_root.join("blackcolours/dev/work/rental-management")).unwrap();

    let decoded = decode_claude_project_dir_with_users_root(
        "-Users-blackcolours-dev-work-rental-management",
        &users_root,
    )
    .unwrap();

    assert!(decoded.ends_with("/Users/blackcolours/dev/work/rental-management"));
}
