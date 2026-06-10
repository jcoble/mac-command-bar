use mcb_core::crypto::SecretBox;
use mcb_core::scanners::processes::parse_lsof_listeners;
use mcb_core::scanners::sessions::{
    decode_claude_project_dir_with_users_root, merge_codex_session_metadata, parse_claude_jsonl,
    parse_codex_index_jsonl, parse_codex_rollout_jsonl, read_tail_utf8,
};
use mcb_core::scanners::worktrees::parse_worktree_porcelain;
use mcb_core::scanners::worktrees::WorktreeScanOptions;

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
fn parses_codex_rollout_metadata_without_transcript_content() {
    let records = parse_codex_rollout_jsonl(
        "{\"timestamp\":\"2026-06-09T01:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"019e\",\"timestamp\":\"2026-06-09T00:59:00Z\",\"cwd\":\"/Users/blackcolours/dev/work/mac-command-bar\"}}\n\
         {\"timestamp\":\"2026-06-09T01:01:00Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"do not use transcript text as a title\"}]}}\n",
    );

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].provider, "codex");
    assert_eq!(records[0].id, "019e");
    assert_eq!(
        records[0].project_path.as_deref(),
        Some("/Users/blackcolours/dev/work/mac-command-bar")
    );
    assert_eq!(records[0].last_activity.as_deref(), Some("2026-06-09T01:00:00Z"));
    assert_eq!(records[0].title, "Codex session");
    assert_eq!(records[0].resume_commands, vec!["codex resume 019e"]);
}

#[test]
fn merges_codex_rollout_project_path_into_index_record() {
    let index = parse_codex_index_jsonl(
        "{\"id\":\"019e\",\"thread_name\":\"Build command bar\",\"updated_at\":\"2026-06-09T01:05:00Z\"}\n",
    );
    let rollout = parse_codex_rollout_jsonl(
        "{\"timestamp\":\"2026-06-09T01:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"019e\",\"cwd\":\"/Users/blackcolours/dev/work/mac-command-bar\"}}\n",
    );

    let merged = merge_codex_session_metadata(index, rollout);

    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].title, "Build command bar");
    assert_eq!(merged[0].last_activity.as_deref(), Some("2026-06-09T01:05:00Z"));
    assert_eq!(
        merged[0].project_path.as_deref(),
        Some("/Users/blackcolours/dev/work/mac-command-bar")
    );
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

#[test]
fn worktree_scan_options_default_to_skipping_expensive_disk_usage() {
    let options = WorktreeScanOptions::default();

    assert!(!options.include_disk_bytes);
}

#[test]
fn reads_only_tail_of_large_jsonl_files() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("session.jsonl");
    let mut body = String::new();
    for index in 0..200 {
        body.push_str(&format!("{{\"line\":{index}}}\n"));
    }
    std::fs::write(&file, body).unwrap();

    let tail = read_tail_utf8(&file, 128).unwrap();

    assert!(tail.contains("\"line\":199"));
    assert!(!tail.contains("\"line\":0"));
    assert!(tail.starts_with('{'));
}
