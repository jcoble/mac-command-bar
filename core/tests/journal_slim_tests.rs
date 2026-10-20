//! The live journal keeps only the newest row of each value the next row
//! replaces: a tool's update, the command list, usage and session config.

use mcb_core::session_store::{EventRow, SessionRow, SessionStore};
use rusqlite::{params, Connection};
use serde_json::Value;
use tempfile::TempDir;

fn session(extra_json: &str) -> SessionRow {
    SessionRow {
        owned_id: "session-a".into(),
        native_session_id: Some("native-a".into()),
        provider: "provider-a".into(),
        model: None,
        effort: None,
        cwd: "/work/project".into(),
        worktree: None,
        branch: None,
        title: None,
        title_source: None,
        project: None,
        project_id: None,
        state: "running".into(),
        suspended: false,
        created_at_ms: 1_000,
        last_activity_at_ms: 1_000,
        extra_json: extra_json.into(),
    }
}

fn event(seq: i64, kind: &str, payload: Value) -> EventRow {
    EventRow {
        owned_id: "session-a".into(),
        seq,
        turn_id: Some("turn-a".into()),
        kind: kind.into(),
        payload_json: serde_json::json!({ "generation": 1, "payload": payload }).to_string(),
        created_at_ms: 20_000 + seq,
    }
}

fn tool(seq: i64, item_id: &str, payload: Value) -> EventRow {
    let mut payload = payload;
    payload["kind"] = "tool".into();
    payload["itemId"] = item_id.into();
    event(seq, "item.updated", payload)
}

fn delta(seq: i64) -> EventRow {
    event(seq, "content.delta", serde_json::json!({
        "kind": "assistantDelta", "itemId": "message-a", "delta": format!("word {seq} ")
    }))
}

fn commands(seq: i64, name: &str) -> EventRow {
    event(seq, "commands.updated", serde_json::json!({
        "kind": "availableCommandsUpdate", "availableCommands": [{ "name": name }]
    }))
}

fn open_store() -> (TempDir, std::path::PathBuf, SessionStore) {
    let directory = TempDir::new().expect("create temporary directory");
    let path = directory.path().join("sessions.db");
    let store = SessionStore::open(&path).expect("open store");
    (directory, path, store)
}

fn write_live(store: &SessionStore, row: &EventRow) {
    store
        .upsert_session_with_event(&session(r#"{"source":"test"}"#), Some(row))
        .expect("write live event");
}

fn sequences(store: &SessionStore) -> Vec<i64> {
    store
        .list_events("session-a", i64::MIN, 1_000)
        .expect("list events")
        .iter()
        .map(|row| row.seq)
        .collect()
}

fn payload(row: &EventRow) -> Value {
    serde_json::from_str(&row.payload_json).expect("parse payload")
}

#[test]
fn fifty_updates_to_one_tool_keep_one_row_with_the_final_state() {
    let (_directory, _path, store) = open_store();
    write_live(&store, &tool(1, "tool-a", serde_json::json!({ "name": "shell", "state": "running" })));
    let mut output = String::new();
    for seq in 2..=49 {
        output.push_str(&format!("line {seq}\n"));
        write_live(&store, &tool(seq, "tool-a", serde_json::json!({ "state": "running", "output": output })));
    }
    write_live(&store, &tool(50, "tool-a", serde_json::json!({ "state": "completed" })));

    let rows = store.list_events("session-a", i64::MIN, 1_000).expect("list events");
    assert_eq!(rows.iter().map(|row| row.seq).collect::<Vec<_>>(), [50]);
    let last = payload(&rows[0]);
    assert_eq!(last["payload"]["name"], "shell");
    assert_eq!(last["payload"]["output"], output.as_str());
    assert_eq!(last["payload"]["state"], "completed");
    assert_eq!(last["payload"]["firstSequence"], 1);
    assert_eq!(last["payload"]["firstTimestampMs"], 20_001);
    for page in [
        store.list_items_before("session-a", i64::MAX, 512 * 1024, None).expect("older page"),
        store.list_items_after("session-a", i64::MIN, 512 * 1024, None).expect("newer page"),
    ] {
        assert_eq!(
            page.items
                .iter()
                .map(|item| (item.stable_id.as_str(), item.first_sequence, item.last_sequence, item.authority_sequence))
                .collect::<Vec<_>>(),
            [("tool:tool-a", 1, 50, Some(50))]
        );
    }
}

#[test]
fn live_tool_updates_leave_other_tools_and_streamed_words_alone() {
    let (_directory, _path, store) = open_store();
    for row in [
        tool(1, "tool-a", serde_json::json!({ "name": "shell", "state": "running" })),
        delta(2),
        tool(3, "tool-b", serde_json::json!({ "name": "read", "state": "running" })),
        delta(4),
        tool(5, "tool-a", serde_json::json!({ "state": "completed", "output": "a done" })),
        delta(6),
        tool(7, "tool-b", serde_json::json!({ "state": "completed", "output": "b done" })),
        delta(8),
    ] {
        write_live(&store, &row);
    }

    assert_eq!(sequences(&store), [2, 4, 5, 6, 7, 8]);
}

#[test]
fn repeated_command_lists_keep_only_the_newest() {
    let (_directory, _path, store) = open_store();
    for seq in 1..=10 {
        write_live(&store, &commands(seq, &format!("command-{seq}")));
    }

    let rows = store.list_events("session-a", i64::MIN, 1_000).expect("list events");
    assert_eq!(rows.iter().map(|row| row.seq).collect::<Vec<_>>(), [10]);
    assert_eq!(payload(&rows[0])["payload"]["availableCommands"][0]["name"], "command-10");
}

#[test]
fn opening_a_version_21_journal_prunes_superseded_rows_once() {
    let (_directory, path, store) = open_store();
    store.upsert_session(&session(r#"{"source":"test"}"#)).expect("write session");
    drop(store);
    let mut rows = Vec::new();
    for seq in 1..=5 {
        rows.push(tool(seq, "tool-a", serde_json::json!({ "name": "shell", "state": "running", "output": format!("part {seq}") })));
    }
    for seq in 6..=8 {
        rows.push(commands(seq, &format!("command-{seq}")));
    }
    for seq in 9..=12 {
        rows.push(event(seq, "usage.updated", serde_json::json!({ "kind": "usage", "used": seq })));
    }
    for seq in 13..=15 {
        rows.push(delta(seq));
    }
    let connection = Connection::open(&path).expect("open raw database");
    for row in &rows {
        connection
            .execute(
                "INSERT INTO events (owned_id, seq, turn_id, kind, payload, created_at) VALUES (?, ?, ?, ?, ?, ?)",
                params![row.owned_id, row.seq, row.turn_id, row.kind, row.payload_json, row.created_at_ms],
            )
            .expect("insert unpruned row");
    }
    connection.pragma_update(None, "user_version", 21).expect("set version 21");
    drop(connection);

    let store = SessionStore::open(&path).expect("upgrade database");
    let kept = store.list_events("session-a", i64::MIN, 1_000).expect("list events");
    let expected = [&rows[4], &rows[7], &rows[11], &rows[12], &rows[13], &rows[14]]
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(kept, expected);
    drop(store);
    let version: i64 = Connection::open(&path)
        .expect("open raw database")
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read version");
    assert_eq!(version, 22);

    let store = SessionStore::open(&path).expect("reopen database");
    assert_eq!(store.list_events("session-a", i64::MIN, 1_000).expect("list events"), expected);
}

#[test]
fn unchanged_capabilities_across_100_events_are_written_once() {
    let (_directory, path, store) = open_store();
    let commands = (0..800)
        .map(|index| serde_json::json!({ "name": format!("command-{index}"), "description": "does one thing well" }))
        .collect::<Vec<_>>();
    let runtime_extra = serde_json::json!({ "capabilities": { "commands": commands } });
    assert!(runtime_extra.to_string().len() > 40_000);
    let mut stored_extra = runtime_extra.clone();
    stored_extra["import"] = serde_json::json!({ "cursor": "line-12" });
    store.upsert_session(&session(&stored_extra.to_string())).expect("write session");
    drop(store);
    Connection::open(&path)
        .expect("open raw database")
        .execute_batch(
            "CREATE TABLE extra_writes (n INTEGER NOT NULL);
             CREATE TRIGGER count_extra_writes AFTER UPDATE OF extra ON sessions
             WHEN OLD.extra IS NOT NEW.extra BEGIN INSERT INTO extra_writes VALUES (1); END;",
        )
        .expect("install write counter");

    let store = SessionStore::open(&path).expect("reopen store");
    let runtime_session = session(&runtime_extra.to_string());
    for seq in 1..=100 {
        store
            .upsert_session_with_event(&runtime_session, Some(&delta(seq)))
            .expect("write live event");
    }

    let stored = store.get_session("session-a").expect("read session").expect("session exists");
    assert_eq!(serde_json::from_str::<Value>(&stored.extra_json).expect("parse extra")["import"]["cursor"], "line-12");
    drop(store);
    let writes: i64 = Connection::open(&path)
        .expect("open raw database")
        .query_row("SELECT count(*) FROM extra_writes", [], |row| row.get(0))
        .expect("count writes");
    assert!(writes <= 1, "extra changed {writes} times");
}
