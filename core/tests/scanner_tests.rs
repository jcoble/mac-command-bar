use mcb_core::crypto::SecretBox;
use mcb_core::scanners::processes::parse_lsof_listeners;
use mcb_core::scanners::sessions::{
    decode_claude_project_dir_with_users_root, merge_agent_session_records,
    merge_codex_session_metadata, parse_claude_jsonl, parse_cmux_hook_sessions_json,
    parse_codex_index_jsonl, parse_codex_rollout_jsonl, read_tail_utf8, scan_sessions,
    AgentSessionRecord,
};
use mcb_core::scanners::worktrees::parse_worktree_porcelain;
use mcb_core::scanners::worktrees::WorktreeScanOptions;
use serde_json::json;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

fn home_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct HomeEnvGuard {
    previous: Option<String>,
}

impl HomeEnvGuard {
    fn set(path: &Path) -> Self {
        let previous = std::env::var("HOME").ok();
        std::env::set_var("HOME", path);
        Self { previous }
    }
}

impl Drop for HomeEnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
    }
}

#[test]
fn parses_git_worktree_porcelain() {
    let records = parse_worktree_porcelain(
        "worktree /repo\nHEAD abc\nbranch refs/heads/main\n\nworktree /worktrees/feature\nHEAD def\nbranch refs/heads/feature\n",
    );

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].path, "/repo");
    assert_eq!(records[0].branch, "main");
    assert_eq!(records[0].task_id, None);
    assert_eq!(records[1].branch, "feature");
    assert_eq!(records[1].task_id, None);
}

#[test]
fn parses_worktree_task_ids_from_branch_or_path_text() {
    let records = parse_worktree_porcelain(
        "worktree /repo\nHEAD abc\nbranch refs/heads/main\n\nworktree /worktrees/feature\nHEAD def\nbranch refs/heads/cdx/tsk-127-feature\n\nworktree /worktrees/TSK-128-path-only\nHEAD fed\nbranch refs/heads/no-task\n\nworktree /worktrees/upper\nHEAD cab\nbranch refs/heads/cdx/TSK-129-upper\n",
    );

    assert_eq!(records.len(), 4);
    assert_eq!(records[0].task_id, None);
    assert_eq!(records[1].task_id.as_deref(), Some("TSK-127"));
    assert_eq!(records[2].task_id.as_deref(), Some("TSK-128"));
    assert_eq!(records[3].task_id.as_deref(), Some("TSK-129"));
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
        "{\"id\":\"019d\",\"thread_name\":\"Fix runtime\",\"updated_at\":\"2026-06-08T22:00:00Z\",\"model\":\"gpt-5.5-codex\"}\n",
    );
    let claude = parse_claude_jsonl(
        "{\"sessionId\":\"abc\",\"cwd\":\"/repo\",\"timestamp\":\"2026-06-08T22:01:00Z\",\"message\":{\"role\":\"user\",\"model\":\"claude-opus-4-8\",\"content\":\"resume work\"}}\n",
        "/repo",
    );

    assert_eq!(codex[0].provider, "codex");
    assert_eq!(codex[0].title, "Fix runtime");
    assert_eq!(codex[0].model.as_deref(), Some("gpt-5.5-codex"));
    assert_eq!(claude[0].provider, "claude");
    assert_eq!(claude[0].project_path.as_deref(), Some("/repo"));
    assert_eq!(claude[0].model.as_deref(), Some("claude-opus-4-8"));
    assert_eq!(
        claude[0].resume_commands,
        vec![
            "claude --resume abc".to_string(),
            "cd '/repo' && claude --resume abc".to_string()
        ]
    );
}

#[test]
fn parses_codex_rollout_metadata_without_transcript_content() {
    let records = parse_codex_rollout_jsonl(
        "{\"timestamp\":\"2026-06-09T01:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"019e\",\"timestamp\":\"2026-06-09T00:59:00Z\",\"cwd\":\"/Users/blackcolours/dev/work/mac-command-bar\",\"model_slug\":\"gpt-5.5-codex\"}}\n\
         {\"timestamp\":\"2026-06-09T01:01:00Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"do not use transcript text as a title\"}]}}\n",
    );

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].provider, "codex");
    assert_eq!(records[0].id, "019e");
    assert_eq!(
        records[0].project_path.as_deref(),
        Some("/Users/blackcolours/dev/work/mac-command-bar")
    );
    assert_eq!(
        records[0].last_activity.as_deref(),
        Some("2026-06-09T01:00:00Z")
    );
    assert_eq!(records[0].title, "Codex session");
    assert_eq!(records[0].model.as_deref(), Some("gpt-5.5-codex"));
    assert_eq!(records[0].resume_commands, vec!["codex resume 019e"]);
}

#[test]
fn codex_turn_context_updates_rollout_project_path() {
    let records = parse_codex_rollout_jsonl(
        "{\"timestamp\":\"2026-06-09T01:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"019e\",\"cwd\":\"/Users/blackcolours/dev/work/EdiPlatform\",\"model_slug\":\"gpt-5.5-codex\"}}\n\
         {\"timestamp\":\"2026-06-09T01:30:00Z\",\"type\":\"turn_context\",\"payload\":{\"cwd\":\"/Users/blackcolours/dev/work/mac-command-bar\",\"model\":\"gpt-5.5\"}}\n",
    );

    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].project_path.as_deref(),
        Some("/Users/blackcolours/dev/work/mac-command-bar")
    );
    assert_eq!(
        records[0].last_activity.as_deref(),
        Some("2026-06-09T01:30:00Z")
    );
    assert_eq!(records[0].model.as_deref(), Some("gpt-5.5"));
}

#[test]
fn codex_function_call_workdir_updates_rollout_project_path() {
    let records = parse_codex_rollout_jsonl(
        "{\"timestamp\":\"2026-06-09T01:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"019e\",\"cwd\":\"/Users/blackcolours/dev/work/EdiPlatform\",\"model_slug\":\"gpt-5.5-codex\"}}\n\
         {\"timestamp\":\"2026-06-09T01:45:00Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"exec_command\",\"arguments\":\"{\\\"cmd\\\":\\\"pwd\\\",\\\"workdir\\\":\\\"/Users/blackcolours/dev/work/mac-command-bar\\\"}\",\"call_id\":\"call_1\"}}\n",
    );

    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].project_path.as_deref(),
        Some("/Users/blackcolours/dev/work/mac-command-bar")
    );
    assert_eq!(
        records[0].last_activity.as_deref(),
        Some("2026-06-09T01:45:00Z")
    );
}

#[test]
fn merges_codex_rollout_project_path_into_index_record() {
    let index = parse_codex_index_jsonl(
        "{\"id\":\"019e\",\"thread_name\":\"Build command bar\",\"updated_at\":\"2026-06-09T01:05:00Z\"}\n",
    );
    let rollout = parse_codex_rollout_jsonl(
        "{\"timestamp\":\"2026-06-09T01:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"019e\",\"cwd\":\"/Users/blackcolours/dev/work/mac-command-bar\",\"modelName\":\"gpt-5.5-xhigh\"}}\n",
    );

    let merged = merge_codex_session_metadata(index, rollout);

    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].title, "Build command bar");
    assert_eq!(
        merged[0].last_activity.as_deref(),
        Some("2026-06-09T01:05:00Z")
    );
    assert_eq!(
        merged[0].project_path.as_deref(),
        Some("/Users/blackcolours/dev/work/mac-command-bar")
    );
    assert_eq!(merged[0].model.as_deref(), Some("gpt-5.5-xhigh"));
}

#[test]
fn parses_cmux_hook_sessions_without_body_content() {
    let records = parse_cmux_hook_sessions_json(
        "codex",
        "{\"version\":1,\"sessions\":{\"019e\":{\"sessionId\":\"019e\",\"cwd\":\"/Users/blackcolours/dev/work/mac-command-bar\",\"updatedAt\":\"2026-06-09T01:07:00Z\",\"runtimeStatus\":\"running\",\"modelId\":\"gpt-5.5-codex\",\"lastSubtitle\":\"main · context 42%\",\"lastBody\":\"do not use this transcript body\"}}}\n",
    );

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].provider, "cmux-codex");
    assert_eq!(records[0].id, "019e");
    assert_eq!(records[0].title, "Codex · main · context 42%");
    assert!(!records[0].title.contains("transcript"));
    assert_eq!(
        records[0].project_path.as_deref(),
        Some("/Users/blackcolours/dev/work/mac-command-bar")
    );
    assert_eq!(
        records[0].last_activity.as_deref(),
        Some("2026-06-09T01:07:00Z")
    );
    assert_eq!(records[0].model.as_deref(), Some("gpt-5.5-codex"));
    assert_eq!(
        records[0].resume_commands,
        vec![
            "codex resume 019e",
            "cd '/Users/blackcolours/dev/work/mac-command-bar' && codex resume 019e"
        ]
    );
}

#[test]
fn parses_cmux_numeric_activity_and_status_titles() {
    let records = parse_cmux_hook_sessions_json(
        "claude",
        "{\"version\":1,\"sessions\":{\"abc\":{\"sessionId\":\"abc\",\"cwd\":\"/Users/blackcolours/dev/work/EDIEngine\",\"updatedAt\":1780880368.0588,\"agentLifecycle\":\"needsInput\",\"lastSubtitle\":\"Waiting\",\"launchCommand\":{\"capturedAt\":1780880300.0}}}}\n",
    );

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].provider, "cmux-claude");
    assert_eq!(records[0].title, "Claude · Waiting");
    assert_eq!(
        records[0].last_activity.as_deref(),
        Some("2026-06-08T00:59:28.058Z")
    );
    assert_eq!(
        records[0].resume_commands,
        vec![
            "claude --resume abc".to_string(),
            "cd '/Users/blackcolours/dev/work/EDIEngine' && claude --resume abc".to_string()
        ]
    );
}

#[test]
fn scans_enough_agent_sessions_for_all_conversations_switcher() {
    let _lock = home_env_lock().lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let _home = HomeEnvGuard::set(temp.path());

    let codex_root = temp.path().join(".codex/sessions/2026/06/09");
    let claude_root = temp.path().join(".claude/projects/test-project");
    let cmux_root = temp.path().join(".cmuxterm");
    std::fs::create_dir_all(&codex_root).unwrap();
    std::fs::create_dir_all(&claude_root).unwrap();
    std::fs::create_dir_all(&cmux_root).unwrap();

    for index in 0..220 {
        let id = format!("codex-{index:03}");
        let timestamp = fixture_timestamp(index);
        std::fs::write(
            codex_root.join(format!("{id}.jsonl")),
            format!(
                "{{\"timestamp\":\"{timestamp}\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"{id}\",\"cwd\":\"/repo/{id}\",\"model_slug\":\"gpt-5.5-codex\"}}}}\n"
            ),
        )
        .unwrap();
    }

    for index in 0..180 {
        let id = format!("claude-{index:03}");
        let timestamp = fixture_timestamp(300 + index);
        std::fs::write(
            claude_root.join(format!("{id}.jsonl")),
            format!(
                "{{\"sessionId\":\"{id}\",\"cwd\":\"/repo/{id}\",\"timestamp\":\"{timestamp}\",\"message\":{{\"role\":\"user\",\"model\":\"claude-opus-4-8\",\"content\":\"Claude {index}\"}}}}\n"
            ),
        )
        .unwrap();
    }

    let mut cmux_sessions = serde_json::Map::new();
    for index in 0..80 {
        let id = format!("cmux-{index:03}");
        cmux_sessions.insert(
            id.clone(),
            json!({
                "sessionId": id,
                "cwd": format!("/repo/cmux-{index:03}"),
                "updatedAt": fixture_timestamp(600 + index),
                "runtimeStatus": "running",
                "modelId": "gpt-5.5-codex",
                "lastSubtitle": format!("cmux {index}"),
            }),
        );
    }
    std::fs::write(
        cmux_root.join("codex-hook-sessions.json"),
        json!({ "version": 1, "sessions": cmux_sessions }).to_string(),
    )
    .unwrap();

    let records = scan_sessions();
    let codex_count = records
        .iter()
        .filter(|record| record.provider == "codex")
        .count();
    let claude_count = records
        .iter()
        .filter(|record| record.provider == "claude")
        .count();
    let cmux_count = records
        .iter()
        .filter(|record| record.provider == "cmux-codex")
        .count();

    assert_eq!(codex_count, 220);
    assert_eq!(claude_count, 180);
    assert_eq!(cmux_count, 80);
    assert_eq!(records.len(), 480);
}

fn fixture_timestamp(index: usize) -> String {
    format!(
        "2026-06-{:02}T{:02}:{:02}:00Z",
        1 + index / 1_440,
        (index / 60) % 24,
        index % 60
    )
}

#[test]
fn merges_duplicate_agent_session_records_by_provider_and_id() {
    let records = merge_agent_session_records(vec![
        AgentSessionRecord {
            provider: "claude".to_string(),
            id: "abc".to_string(),
            title: "Older title".to_string(),
            description: None,
            model: Some("claude-sonnet-4-5".to_string()),
            project_path: Some("/repo".to_string()),
            last_activity: Some("2026-06-09T01:00:00Z".to_string()),
            resume_commands: vec!["claude --resume abc".to_string()],
            log_path: None,
            branch_hint: None,
            task_id: None,
            pull_request_hint: None,
            source_label: None,
            message_count: None,
            latest_turn_preview: None,
        },
        AgentSessionRecord {
            provider: "claude".to_string(),
            id: "abc".to_string(),
            title: "Newer title".to_string(),
            description: None,
            model: Some("claude-opus-4-8".to_string()),
            project_path: Some("/repo/worktree".to_string()),
            last_activity: Some("2026-06-09T02:00:00Z".to_string()),
            resume_commands: vec![
                "claude --resume abc".to_string(),
                "cd /repo/worktree && claude --resume abc".to_string(),
            ],
            log_path: None,
            branch_hint: None,
            task_id: None,
            pull_request_hint: None,
            source_label: None,
            message_count: None,
            latest_turn_preview: None,
        },
    ]);

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].title, "Newer title");
    assert_eq!(records[0].project_path.as_deref(), Some("/repo/worktree"));
    assert_eq!(
        records[0].last_activity.as_deref(),
        Some("2026-06-09T02:00:00Z")
    );
    assert_eq!(records[0].model.as_deref(), Some("claude-opus-4-8"));
    assert_eq!(
        records[0].resume_commands,
        vec![
            "claude --resume abc".to_string(),
            "cd /repo/worktree && claude --resume abc".to_string()
        ]
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

/// Times the session scan the way the app calls it, against the transcripts
/// actually on this machine — the only realistic input there is.
///
/// Ignored on purpose: it reads whatever happens to be on disk, so it measures
/// rather than checks, and its number means nothing on another machine. Run it
/// with:
///   cargo test --manifest-path core/Cargo.toml --test scanner_tests -- \
///     --ignored --nocapture session_scan_time
#[test]
#[ignore]
fn session_scan_time() {
    for pass in 1..=3 {
        let start = std::time::Instant::now();
        let records = scan_sessions();
        println!(
            "pass {pass}: {} sessions in {:?}",
            records.len(),
            start.elapsed()
        );
    }
}
