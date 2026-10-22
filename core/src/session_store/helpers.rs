use super::{Result, SessionStore, StoreError};
use rusqlite::params;

const DUE: &str = "SELECT s.owned_id FROM sessions s
WHERE json_extract(s.extra,'$.helperOf') IS NOT NULL
  AND s.cached_remote_profile_id IS NULL AND s.parent_owned_id IS NULL
  AND json_extract(s.extra,'$.railMeta.completedAt') IS NULL
  AND s.state NOT IN ('starting','working','interrupting','waiting-approval','waiting-input')
  AND s.last_activity_at <= ?1 - ?2
  AND NOT EXISTS (SELECT 1 FROM workflow_messages m
      WHERE m.to_owned_id = json_extract(s.extra,'$.helperOf') AND m.receipt = 'queued'
        AND m.from_owned_id = s.owned_id)
  AND NOT EXISTS (SELECT 1 FROM events e WHERE e.owned_id = s.owned_id
      AND json_extract(e.payload,'$.payload.kind') = 'userMessage'
      AND substr(json_extract(e.payload,'$.payload.text'), 1, 14) <> '[message from ')
  AND NOT EXISTS (SELECT 1 FROM events e WHERE e.owned_id = s.owned_id
      AND json_extract(e.payload,'$.payload.kind') = 'sessionCompleted')
ORDER BY s.owned_id";

impl SessionStore {
    /// Helpers that finished at least `quiet_ms` before `now_ms` and may be marked done.
    pub fn finished_helpers_due(&self, now_ms: i64, quiet_ms: i64) -> Result<Vec<String>> {
        let connection = self.lock()?;
        let read_error = |error| StoreError::sqlite("could not read finished helpers", error);
        let mut statement = connection.prepare(DUE).map_err(read_error)?;
        let rows = statement.query_map(params![now_ms, quiet_ms], |row| row.get(0)).map_err(read_error)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(read_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::{MessageKind, Receipt};
    use crate::session_store::{EventRow, SessionRow};
    use serde_json::json;

    #[test]
    fn finished_helpers_due_lists_only_quiet_untouched_helpers() {
        let store = SessionStore::open_in_memory().unwrap();
        for id in ["quiet", "approval", "input", "working", "recent", "queued", "owner", "relayed",
            "done", "reopened", "top", "remote", "child", "delivered"] {
            let mut extra = json!({"helperOf":"parent"});
            if id == "top" { extra = json!({}); }
            if id == "done" { extra["railMeta"] = json!({"completedAt":"2026-10-20T00:00:00Z"}); }
            let row = SessionRow {
                owned_id: id.into(), native_session_id: None, provider: "codex".into(),
                model: None, effort: None, cwd: "/work".into(), worktree: None, branch: None,
                title: Some(id.into()), title_source: None, project: None, project_id: None,
                state: match id { "approval" => "waiting-approval", "input" => "waiting-input",
                    "working" => "working", _ => "ready" }.into(),
                suspended: false, created_at_ms: 1,
                last_activity_at_ms: if id == "recent" { 950 } else { 100 }, extra_json: extra.to_string(),
            };
            if id == "remote" { store.cache_remote_events("box", &row, &[]).unwrap(); }
            else { store.upsert_session(&row).unwrap(); }
        }
        // Provider sub-agents and cached remote copies belong to another lifecycle.
        store.lock_write().unwrap().execute("UPDATE sessions SET parent_owned_id='top' WHERE owned_id='child'", []).unwrap();
        let queued = store.append_message("", "queued", "parent", MessageKind::Message, "reply").unwrap();
        let delivered = store.append_message("", "delivered", "parent", MessageKind::Message, "reply").unwrap();
        store.set_receipt(&delivered.id, Receipt::Delivered).unwrap();
        for (id, payload) in [
            ("owner", json!({"kind":"userMessage","text":"please also check this"})),
            ("relayed", json!({"kind":"userMessage","text":"[message from x]\nCheck this"})),
            ("reopened", json!({"kind":"sessionCompleted","completedAt":"earlier"})),
        ] {
            store.append_event(&EventRow { owned_id: id.into(), seq: 1, turn_id: None,
                kind: "message".into(), payload_json: json!({"payload":payload}).to_string(), created_at_ms: 100 }).unwrap();
        }
        let plan = store.lock().unwrap().prepare(&format!("EXPLAIN QUERY PLAN {DUE}")).unwrap()
            .query_map(params![1000, 500], |row| row.get::<_, String>(3)).unwrap()
            .collect::<rusqlite::Result<Vec<_>>>().unwrap().join("\n");
        assert!(plan.contains("events_user_command_page_idx"), "{plan}");
        assert!(plan.contains("idx_workflow_messages_target"), "{plan}");
        assert!(!plan.contains("SCAN e") && !plan.contains("SCAN m"), "{plan}");
        println!("{plan}");
        assert_eq!(store.finished_helpers_due(1000, 500).unwrap(), ["delivered", "quiet", "relayed"]);
        store.set_receipt(&queued.id, Receipt::Delivered).unwrap();
        assert_eq!(store.finished_helpers_due(1000, 500).unwrap(), ["delivered", "queued", "quiet", "relayed"]);
    }
}
