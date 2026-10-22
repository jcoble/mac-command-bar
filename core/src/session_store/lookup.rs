use super::{session_from_row, Result, SessionRow, SessionStore, StoreError};
use rusqlite::{params, OptionalExtension};

impl SessionStore {
    /// The local session holding this provider conversation, looked up the way the session list
    /// would find it: cached remote rows and child sessions are not candidates, and the most
    /// recently active row wins.
    pub fn session_for_native_id(&self, provider: &str, native_session_id: &str) -> Result<Option<SessionRow>> {
        self.lock()?
            .query_row(
                "SELECT owned_id, native_session_id, provider, model, effort, cwd, worktree,
                        branch, title, project, state, suspended, created_at, last_activity_at,
                        extra, title_source, project_id
                 FROM sessions
                 WHERE provider = ?1 AND native_session_id = ?2
                   AND cached_remote_profile_id IS NULL AND parent_owned_id IS NULL
                 ORDER BY last_activity_at DESC, owned_id ASC
                 LIMIT 1",
                params![provider, native_session_id],
                session_from_row,
            )
            .optional()
            .map_err(|error| StoreError::sqlite("could not find the session for that conversation", error))
    }

    /// The workflow step attempt a session runs, from its latest workflow event (one indexed read).
    pub fn node_run_for_session(&self, owned_id: &str) -> Result<Option<String>> {
        self.lock()?
            .query_row(
                "SELECT json_extract(payload_json,'$.nodeRunId') FROM orchestration_events
                 WHERE json_extract(payload_json,'$.ownedId') = ?1 ORDER BY sequence DESC LIMIT 1",
                [owned_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map(Option::flatten)
            .map_err(|error| StoreError::sqlite("could not find the session's workflow step", error))
    }
}

#[cfg(test)]
mod tests {
    use super::SessionStore;

    fn store() -> SessionStore {
        let store = SessionStore::open_in_memory().unwrap();
        store.lock_write().unwrap().execute_batch(
            "INSERT INTO sessions
                (owned_id,native_session_id,provider,cwd,state,suspended,created_at,last_activity_at,extra) VALUES
                ('older','native-1','claude','/work','ready',0,1,1,'{}'),
                ('newer','native-1','claude','/work','ready',0,1,5,'{}'),
                ('other-provider','native-1','codex','/work','ready',0,1,9,'{}'),
                ('other-native','native-2','claude','/work','ready',0,1,9,'{}'),
                ('cached','native-3','claude','/work','ready',0,1,9,'{}'),
                ('child','native-4','claude','/work','ready',0,1,9,'{}');
             UPDATE sessions SET cached_remote_profile_id = 'machine' WHERE owned_id = 'cached';
             UPDATE sessions SET parent_owned_id = 'newer' WHERE owned_id = 'child';",
        ).unwrap();
        store
    }

    #[test]
    fn session_for_native_id_finds_the_listed_row_for_that_provider() {
        let store = store();
        let found = |provider: &str, native: &str| store.session_for_native_id(provider, native).unwrap().map(|row| row.owned_id);
        assert_eq!(found("claude", "native-1").as_deref(), Some("newer"));
        assert_eq!(found("codex", "native-1").as_deref(), Some("other-provider"));
        assert_eq!(found("claude", "native-3"), None, "a cached remote row is not a local session");
        assert_eq!(found("claude", "native-4"), None, "a child session is not listed");
        assert_eq!(found("claude", "missing"), None);
    }

    #[test]
    fn session_for_native_id_does_not_decode_unrelated_rows() {
        let store = store();
        store.lock_write().unwrap().execute(
            "UPDATE sessions SET created_at = 'unreadable' WHERE owned_id = 'other-native'", [],
        ).unwrap();
        assert_eq!(store.session_for_native_id("claude", "native-1").unwrap().unwrap().owned_id, "newer");
    }

    #[test]
    fn node_run_for_session_reads_the_latest_workflow_event_of_that_session() {
        let store = store();
        store.lock_write().unwrap().execute_batch(
            r#"INSERT INTO orchestration_events (id, run_id, kind, timestamp, sequence, payload_json) VALUES
                ('e1','run-1','workflow.node.started','1',1,'{"ownedId":"agent-a","nodeRunId":"run-1:build:1"}'),
                ('e2','run-1','workflow.node.started','2',2,'{"ownedId":"agent-b","nodeRunId":"run-1:review:1"}'),
                ('e3','run-1','workflow.node.finished','3',3,'{"ownedId":"agent-a","nodeRunId":"run-1:build:1"}');"#,
        ).unwrap();
        assert_eq!(store.node_run_for_session("agent-a").unwrap().as_deref(), Some("run-1:build:1"));
        assert_eq!(store.node_run_for_session("agent-b").unwrap().as_deref(), Some("run-1:review:1"));
        assert_eq!(store.node_run_for_session("chat").unwrap(), None);
    }
}
