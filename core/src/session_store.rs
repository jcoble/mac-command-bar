use std::fmt;
use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};

const SCHEMA_VERSION: i64 = 1;

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug)]
pub struct StoreError {
    context: &'static str,
    source: Option<rusqlite::Error>,
}

impl StoreError {
    fn sqlite(context: &'static str, source: rusqlite::Error) -> Self {
        Self {
            context,
            source: Some(source),
        }
    }

    fn message(context: &'static str) -> Self {
        Self {
            context,
            source: None,
        }
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.source {
            Some(source) => write!(formatter, "{}: {source}", self.context),
            None => formatter.write_str(self.context),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|source| source as &(dyn std::error::Error + 'static))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionRow {
    pub owned_id: String,
    pub native_session_id: Option<String>,
    pub provider: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub cwd: String,
    pub worktree: Option<String>,
    pub branch: Option<String>,
    pub title: Option<String>,
    pub project: Option<String>,
    pub state: String,
    pub suspended: bool,
    pub created_at_ms: i64,
    pub last_activity_at_ms: i64,
    pub extra_json: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventRow {
    pub owned_id: String,
    pub seq: i64,
    pub turn_id: Option<String>,
    pub kind: String,
    pub payload_json: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnotationRow {
    pub id: i64,
    pub owned_id: String,
    pub url: String,
    pub rect_json: String,
    pub note: String,
    pub created_at_ms: i64,
}

pub struct SessionStore {
    connection: Mutex<Connection>,
}

impl SessionStore {
    pub fn open(path: &Path) -> Result<Self> {
        let connection = Connection::open(path)
            .map_err(|error| StoreError::sqlite("could not open the session database", error))?;
        Self::from_connection(connection)
    }

    pub fn open_in_memory() -> Result<Self> {
        let connection = Connection::open_in_memory().map_err(|error| {
            StoreError::sqlite("could not open the in-memory session database", error)
        })?;
        Self::from_connection(connection)
    }

    fn from_connection(mut connection: Connection) -> Result<Self> {
        let journal_mode: String = connection
            .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))
            .map_err(|error| StoreError::sqlite("could not enable WAL journal mode", error))?;
        if !journal_mode.eq_ignore_ascii_case("wal") && !journal_mode.eq_ignore_ascii_case("memory")
        {
            return Err(StoreError::message(
                "the session database did not enable WAL journal mode",
            ));
        }
        connection
            .pragma_update(None, "synchronous", "NORMAL")
            .map_err(|error| StoreError::sqlite("could not set normal synchronization", error))?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(|error| StoreError::sqlite("could not enable foreign keys", error))?;

        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|error| StoreError::sqlite("could not read the schema version", error))?;
        match version {
            0 => {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| {
                        StoreError::sqlite("could not begin schema creation", error)
                    })?;
                transaction
                    .execute_batch(
                        "CREATE TABLE sessions (
                            owned_id TEXT PRIMARY KEY,
                            native_session_id TEXT,
                            provider TEXT NOT NULL,
                            model TEXT,
                            effort TEXT,
                            cwd TEXT NOT NULL,
                            worktree TEXT,
                            branch TEXT,
                            title TEXT,
                            project TEXT,
                            state TEXT NOT NULL,
                            suspended INTEGER NOT NULL CHECK (suspended IN (0, 1)),
                            created_at INTEGER NOT NULL,
                            last_activity_at INTEGER NOT NULL,
                            extra TEXT NOT NULL
                        );
                        CREATE TABLE events (
                            owned_id TEXT NOT NULL REFERENCES sessions(owned_id) ON DELETE CASCADE,
                            seq INTEGER NOT NULL,
                            turn_id TEXT,
                            kind TEXT NOT NULL,
                            payload TEXT NOT NULL,
                            created_at INTEGER NOT NULL,
                            PRIMARY KEY (owned_id, seq)
                        );
                        CREATE TABLE drafts (
                            owned_id TEXT PRIMARY KEY REFERENCES sessions(owned_id) ON DELETE CASCADE,
                            text TEXT NOT NULL,
                            updated_at INTEGER NOT NULL
                        );
                        CREATE TABLE annotations (
                            id INTEGER PRIMARY KEY,
                            owned_id TEXT NOT NULL REFERENCES sessions(owned_id) ON DELETE CASCADE,
                            url TEXT NOT NULL,
                            rect TEXT NOT NULL,
                            note TEXT NOT NULL,
                            created_at INTEGER NOT NULL
                        );
                        CREATE INDEX annotations_owned_id_idx ON annotations(owned_id, id);",
                    )
                    .map_err(|error| {
                        StoreError::sqlite("could not create the session schema", error)
                    })?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish schema creation", error)
                })?;
            }
            SCHEMA_VERSION => {}
            _ => {
                return Err(StoreError::message(
                    "the session database schema version is not supported",
                ));
            }
        }

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn upsert_session(&self, row: &SessionRow) -> Result<()> {
        let connection = self.lock()?;
        connection
            .execute(
                "INSERT INTO sessions (
                    owned_id, native_session_id, provider, model, effort, cwd, worktree,
                    branch, title, project, state, suspended, created_at, last_activity_at, extra
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(owned_id) DO UPDATE SET
                    native_session_id = excluded.native_session_id,
                    provider = excluded.provider,
                    model = excluded.model,
                    effort = excluded.effort,
                    cwd = excluded.cwd,
                    worktree = excluded.worktree,
                    branch = excluded.branch,
                    title = excluded.title,
                    project = excluded.project,
                    state = excluded.state,
                    suspended = excluded.suspended,
                    created_at = excluded.created_at,
                    last_activity_at = excluded.last_activity_at,
                    extra = excluded.extra",
                params![
                    row.owned_id,
                    row.native_session_id,
                    row.provider,
                    row.model,
                    row.effort,
                    row.cwd,
                    row.worktree,
                    row.branch,
                    row.title,
                    row.project,
                    row.state,
                    row.suspended,
                    row.created_at_ms,
                    row.last_activity_at_ms,
                    row.extra_json,
                ],
            )
            .map_err(|error| StoreError::sqlite("could not save the session", error))?;
        Ok(())
    }

    pub fn get_session(&self, owned_id: &str) -> Result<Option<SessionRow>> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT owned_id, native_session_id, provider, model, effort, cwd, worktree,
                        branch, title, project, state, suspended, created_at, last_activity_at, extra
                 FROM sessions
                 WHERE owned_id = ?",
                [owned_id],
                session_from_row,
            )
            .optional()
            .map_err(|error| StoreError::sqlite("could not read the session", error))
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionRow>> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT owned_id, native_session_id, provider, model, effort, cwd, worktree,
                        branch, title, project, state, suspended, created_at, last_activity_at, extra
                 FROM sessions
                 ORDER BY last_activity_at DESC, owned_id ASC",
            )
            .map_err(|error| StoreError::sqlite("could not prepare the session list", error))?;
        let rows = statement
            .query_map([], session_from_row)
            .map_err(|error| StoreError::sqlite("could not list sessions", error))?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|error| StoreError::sqlite("could not read the session list", error))
    }

    pub fn delete_session(&self, owned_id: &str) -> Result<()> {
        let connection = self.lock()?;
        connection
            .execute("DELETE FROM sessions WHERE owned_id = ?", [owned_id])
            .map_err(|error| StoreError::sqlite("could not delete the session", error))?;
        Ok(())
    }

    pub fn append_event(&self, row: &EventRow) -> Result<()> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| StoreError::sqlite("could not begin the event write", error))?;
        transaction
            .execute(
                "INSERT INTO events (owned_id, seq, turn_id, kind, payload, created_at)
                 VALUES (?, ?, ?, ?, ?, ?)",
                params![
                    row.owned_id,
                    row.seq,
                    row.turn_id,
                    row.kind,
                    row.payload_json,
                    row.created_at_ms
                ],
            )
            .map_err(|error| StoreError::sqlite("could not append the event", error))?;
        let updated = transaction
            .execute(
                "UPDATE sessions
                 SET last_activity_at = MAX(last_activity_at, ?)
                 WHERE owned_id = ?",
                params![row.created_at_ms, row.owned_id],
            )
            .map_err(|error| StoreError::sqlite("could not update the session activity", error))?;
        if updated != 1 {
            return Err(StoreError::message(
                "could not update activity because the session does not exist",
            ));
        }
        transaction
            .commit()
            .map_err(|error| StoreError::sqlite("could not finish the event write", error))
    }

    pub fn list_events(&self, owned_id: &str, from_seq: i64, limit: u32) -> Result<Vec<EventRow>> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT owned_id, seq, turn_id, kind, payload, created_at
                 FROM events
                 WHERE owned_id = ? AND seq >= ?
                 ORDER BY seq ASC
                 LIMIT ?",
            )
            .map_err(|error| StoreError::sqlite("could not prepare the event list", error))?;
        let rows = statement
            .query_map(
                params![owned_id, from_seq, i64::from(limit)],
                event_from_row,
            )
            .map_err(|error| StoreError::sqlite("could not list events", error))?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|error| StoreError::sqlite("could not read the event list", error))
    }

    /// The newest bounded event window, returned in transcript order.
    ///
    /// Both the limiting and final ordering stay in SQLite so opening a long
    /// conversation never materializes its full journal just to discard the
    /// oldest rows in application code.
    pub fn list_recent_events(&self, owned_id: &str, limit: u32) -> Result<Vec<EventRow>> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT owned_id, seq, turn_id, kind, payload, created_at
                 FROM (
                     SELECT owned_id, seq, turn_id, kind, payload, created_at
                     FROM events
                     WHERE owned_id = ?
                     ORDER BY seq DESC
                     LIMIT ?
                 )
                 ORDER BY seq ASC",
            )
            .map_err(|error| {
                StoreError::sqlite("could not prepare the recent event list", error)
            })?;
        let rows = statement
            .query_map(params![owned_id, i64::from(limit)], event_from_row)
            .map_err(|error| StoreError::sqlite("could not list recent events", error))?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|error| StoreError::sqlite("could not read the recent event list", error))
    }

    pub fn latest_seq(&self, owned_id: &str) -> Result<i64> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) FROM events WHERE owned_id = ?",
                [owned_id],
                |row| row.get(0),
            )
            .map_err(|error| StoreError::sqlite("could not read the latest event sequence", error))
    }

    pub fn enforce_event_cap(&self, owned_id: &str, keep: u32) -> Result<u64> {
        let connection = self.lock()?;
        let deleted = connection
            .execute(
                "DELETE FROM events
                 WHERE owned_id = ?
                   AND seq NOT IN (
                       SELECT seq
                       FROM events
                       WHERE owned_id = ?
                       ORDER BY seq DESC
                       LIMIT ?
                   )",
                params![owned_id, owned_id, i64::from(keep)],
            )
            .map_err(|error| StoreError::sqlite("could not enforce the event limit", error))?;
        u64::try_from(deleted)
            .map_err(|_| StoreError::message("the deleted event count could not be represented"))
    }

    pub fn set_draft(&self, owned_id: &str, text: &str) -> Result<()> {
        let connection = self.lock()?;
        let changed = connection
            .execute(
                "INSERT INTO drafts (owned_id, text, updated_at)
                 SELECT owned_id, ?, last_activity_at
                 FROM sessions
                 WHERE owned_id = ?
                 ON CONFLICT(owned_id) DO UPDATE SET
                    text = excluded.text,
                    updated_at = excluded.updated_at",
                params![text, owned_id],
            )
            .map_err(|error| StoreError::sqlite("could not save the draft", error))?;
        if changed != 1 {
            return Err(StoreError::message(
                "could not save the draft because the session does not exist",
            ));
        }
        Ok(())
    }

    pub fn get_draft(&self, owned_id: &str) -> Result<Option<String>> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT text FROM drafts WHERE owned_id = ?",
                [owned_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| StoreError::sqlite("could not read the draft", error))
    }

    pub fn clear_draft(&self, owned_id: &str) -> Result<()> {
        let connection = self.lock()?;
        connection
            .execute("DELETE FROM drafts WHERE owned_id = ?", [owned_id])
            .map_err(|error| StoreError::sqlite("could not clear the draft", error))?;
        Ok(())
    }

    pub fn add_annotation(
        &self,
        owned_id: &str,
        url: &str,
        rect_json: &str,
        note: &str,
    ) -> Result<i64> {
        let connection = self.lock()?;
        let changed = connection
            .execute(
                "INSERT INTO annotations (owned_id, url, rect, note, created_at)
                 SELECT owned_id, ?, ?, ?, last_activity_at
                 FROM sessions
                 WHERE owned_id = ?",
                params![url, rect_json, note, owned_id],
            )
            .map_err(|error| StoreError::sqlite("could not add the annotation", error))?;
        if changed != 1 {
            return Err(StoreError::message(
                "could not add the annotation because the session does not exist",
            ));
        }
        Ok(connection.last_insert_rowid())
    }

    pub fn list_annotations(&self, owned_id: &str) -> Result<Vec<AnnotationRow>> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT id, owned_id, url, rect, note, created_at
                 FROM annotations
                 WHERE owned_id = ?
                 ORDER BY id ASC",
            )
            .map_err(|error| StoreError::sqlite("could not prepare the annotation list", error))?;
        let rows = statement
            .query_map([owned_id], annotation_from_row)
            .map_err(|error| StoreError::sqlite("could not list annotations", error))?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|error| StoreError::sqlite("could not read the annotation list", error))
    }

    pub fn delete_annotation(&self, id: i64) -> Result<()> {
        let connection = self.lock()?;
        connection
            .execute("DELETE FROM annotations WHERE id = ?", [id])
            .map_err(|error| StoreError::sqlite("could not delete the annotation", error))?;
        Ok(())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.connection
            .lock()
            .map_err(|_| StoreError::message("the session database lock is unavailable"))
    }
}

fn session_from_row(row: &Row<'_>) -> rusqlite::Result<SessionRow> {
    Ok(SessionRow {
        owned_id: row.get(0)?,
        native_session_id: row.get(1)?,
        provider: row.get(2)?,
        model: row.get(3)?,
        effort: row.get(4)?,
        cwd: row.get(5)?,
        worktree: row.get(6)?,
        branch: row.get(7)?,
        title: row.get(8)?,
        project: row.get(9)?,
        state: row.get(10)?,
        suspended: row.get(11)?,
        created_at_ms: row.get(12)?,
        last_activity_at_ms: row.get(13)?,
        extra_json: row.get(14)?,
    })
}

fn event_from_row(row: &Row<'_>) -> rusqlite::Result<EventRow> {
    Ok(EventRow {
        owned_id: row.get(0)?,
        seq: row.get(1)?,
        turn_id: row.get(2)?,
        kind: row.get(3)?,
        payload_json: row.get(4)?,
        created_at_ms: row.get(5)?,
    })
}

fn annotation_from_row(row: &Row<'_>) -> rusqlite::Result<AnnotationRow> {
    Ok(AnnotationRow {
        id: row.get(0)?,
        owned_id: row.get(1)?,
        url: row.get(2)?,
        rect_json: row.get(3)?,
        note: row.get(4)?,
        created_at_ms: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::{EventRow, SessionRow, SessionStore};

    fn fixture_session(owned_id: &str, activity_ms: i64) -> SessionRow {
        SessionRow {
            owned_id: owned_id.to_owned(),
            native_session_id: Some(format!("native-{owned_id}")),
            provider: "provider-a".to_owned(),
            model: Some("model-a".to_owned()),
            effort: Some("medium".to_owned()),
            cwd: "/work/project".to_owned(),
            worktree: Some("/work/project-tree".to_owned()),
            branch: Some("lane/store".to_owned()),
            title: Some(format!("Session {owned_id}")),
            project: Some("Command Bar".to_owned()),
            state: "idle".to_owned(),
            suspended: false,
            created_at_ms: 1_000,
            last_activity_at_ms: activity_ms,
            extra_json: r#"{"source":"test"}"#.to_owned(),
        }
    }

    fn fixture_event(owned_id: &str, seq: i64) -> EventRow {
        EventRow {
            owned_id: owned_id.to_owned(),
            seq,
            turn_id: Some(format!("turn-{seq}")),
            kind: "message".to_owned(),
            payload_json: format!(r#"{{"seq":{seq}}}"#),
            created_at_ms: 10_000 + seq,
        }
    }

    fn open_temp_store() -> (TempDir, std::path::PathBuf, SessionStore) {
        let directory = TempDir::new().expect("create temporary directory");
        let path = directory.path().join("sessions.db");
        let store = SessionStore::open(&path).expect("open session store");
        (directory, path, store)
    }

    #[test]
    fn open_creates_schema_and_reopens() {
        let (directory, path, store) = open_temp_store();
        {
            let connection = store.connection.lock().expect("lock session store");
            let journal_mode: String = connection
                .pragma_query_value(None, "journal_mode", |row| row.get(0))
                .expect("read journal mode");
            let synchronous: i64 = connection
                .pragma_query_value(None, "synchronous", |row| row.get(0))
                .expect("read synchronization mode");
            let foreign_keys: i64 = connection
                .pragma_query_value(None, "foreign_keys", |row| row.get(0))
                .expect("read foreign key setting");
            assert_eq!(journal_mode, "wal");
            assert_eq!(synchronous, 1);
            assert_eq!(foreign_keys, 1);
        }
        drop(store);

        let connection = Connection::open(&path).expect("inspect database");
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read schema version");
        assert_eq!(version, 1);

        let tables: Vec<String> = {
            let mut statement = connection
                .prepare(
                    "SELECT name FROM sqlite_master \
                     WHERE type = 'table' AND name IN ('sessions', 'events', 'drafts', 'annotations') \
                     ORDER BY name",
                )
                .expect("prepare schema query");
            statement
                .query_map([], |row| row.get(0))
                .expect("query schema")
                .collect::<rusqlite::Result<_>>()
                .expect("read table names")
        };
        assert_eq!(tables, ["annotations", "drafts", "events", "sessions"]);
        drop(connection);

        let reopened = SessionStore::open(&path).expect("reopen session store");
        drop(reopened);
        let connection = Connection::open(directory.path().join("sessions.db"))
            .expect("inspect reopened database");
        let reopened_version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read reopened schema version");
        assert_eq!(reopened_version, version);
    }

    #[test]
    fn session_crud_roundtrip() {
        let (_directory, _path, store) = open_temp_store();
        let mut row = fixture_session("owned-1", 2_000);

        assert_eq!(
            store.get_session(&row.owned_id).expect("get missing row"),
            None
        );
        store.upsert_session(&row).expect("insert session");
        assert_eq!(
            store.get_session(&row.owned_id).expect("get inserted row"),
            Some(row.clone())
        );

        row.native_session_id = None;
        row.model = None;
        row.effort = None;
        row.worktree = None;
        row.branch = None;
        row.title = None;
        row.project = None;
        row.state = "running".to_owned();
        row.suspended = true;
        row.last_activity_at_ms = 3_000;
        row.extra_json = "{}".to_owned();
        store.upsert_session(&row).expect("update session");
        assert_eq!(
            store.get_session(&row.owned_id).expect("get updated row"),
            Some(row.clone())
        );

        store.delete_session(&row.owned_id).expect("delete session");
        assert_eq!(
            store.get_session(&row.owned_id).expect("get deleted row"),
            None
        );
    }

    #[test]
    fn list_ordering_by_activity() {
        let (_directory, _path, store) = open_temp_store();
        for row in [
            fixture_session("oldest", 100),
            fixture_session("newest", 300),
            fixture_session("middle", 200),
        ] {
            store.upsert_session(&row).expect("insert session");
        }

        let ids: Vec<String> = store
            .list_sessions()
            .expect("list sessions")
            .into_iter()
            .map(|row| row.owned_id)
            .collect();
        assert_eq!(ids, ["newest", "middle", "oldest"]);
    }

    #[test]
    fn events_append_bumps_activity() {
        let (_directory, _path, store) = open_temp_store();
        let session = fixture_session("owned-1", 2_000);
        store.upsert_session(&session).expect("insert session");

        let event = fixture_event(&session.owned_id, 1);
        store.append_event(&event).expect("append event");

        assert_eq!(
            store
                .get_session(&session.owned_id)
                .expect("get session")
                .expect("session exists")
                .last_activity_at_ms,
            event.created_at_ms
        );
        assert_eq!(
            store
                .list_events(&session.owned_id, 0, 10)
                .expect("list events"),
            [event]
        );
    }

    #[test]
    fn list_from_seq_pagination() {
        let (_directory, _path, store) = open_temp_store();
        let session = fixture_session("owned-1", 2_000);
        store.upsert_session(&session).expect("insert session");
        for seq in 1..=6 {
            store
                .append_event(&fixture_event(&session.owned_id, seq))
                .expect("append event");
        }

        let seqs: Vec<i64> = store
            .list_events(&session.owned_id, 3, 2)
            .expect("list event page")
            .into_iter()
            .map(|event| event.seq)
            .collect();
        assert_eq!(seqs, [3, 4]);
        assert!(store
            .list_events(&session.owned_id, 1, 0)
            .expect("list zero-sized page")
            .is_empty());
    }

    #[test]
    fn recent_event_page_is_bounded_and_keeps_transcript_order() {
        let (_directory, _path, store) = open_temp_store();
        let session = fixture_session("owned-recent", 2_000);
        store.upsert_session(&session).expect("insert session");
        for seq in 1..=6 {
            store
                .append_event(&fixture_event(&session.owned_id, seq))
                .expect("append event");
        }

        let seqs: Vec<i64> = store
            .list_recent_events(&session.owned_id, 3)
            .expect("list recent event page")
            .into_iter()
            .map(|event| event.seq)
            .collect();
        assert_eq!(seqs, [4, 5, 6]);
        assert!(store
            .list_recent_events(&session.owned_id, 0)
            .expect("list zero-sized recent page")
            .is_empty());
    }

    #[test]
    fn latest_seq() {
        let (_directory, _path, store) = open_temp_store();
        let session = fixture_session("owned-1", 2_000);
        store.upsert_session(&session).expect("insert session");
        assert_eq!(
            store
                .latest_seq(&session.owned_id)
                .expect("get empty latest seq"),
            0
        );

        for seq in [2, 7, 4] {
            store
                .append_event(&fixture_event(&session.owned_id, seq))
                .expect("append event");
        }
        assert_eq!(
            store.latest_seq(&session.owned_id).expect("get latest seq"),
            7
        );
    }

    #[test]
    fn enforce_event_cap_keeps_newest() {
        let (_directory, _path, store) = open_temp_store();
        let session = fixture_session("owned-1", 2_000);
        store.upsert_session(&session).expect("insert session");
        for seq in 1..=120 {
            store
                .append_event(&fixture_event(&session.owned_id, seq))
                .expect("append event");
        }

        assert_eq!(
            store
                .enforce_event_cap(&session.owned_id, 100)
                .expect("enforce event cap"),
            20
        );
        let events = store
            .list_events(&session.owned_id, 0, 200)
            .expect("list retained events");
        assert_eq!(events.len(), 100);
        assert_eq!(events.first().expect("first retained event").seq, 21);
        assert_eq!(events.last().expect("last retained event").seq, 120);
        assert_eq!(
            store
                .enforce_event_cap(&session.owned_id, 100)
                .expect("enforce satisfied cap"),
            0
        );
    }

    #[test]
    fn drafts_roundtrip_and_clear() {
        let (_directory, _path, store) = open_temp_store();
        let session = fixture_session("owned-1", 2_000);
        store.upsert_session(&session).expect("insert session");

        assert_eq!(
            store
                .get_draft(&session.owned_id)
                .expect("get missing draft"),
            None
        );
        store
            .set_draft(&session.owned_id, "first draft")
            .expect("insert draft");
        assert_eq!(
            store
                .get_draft(&session.owned_id)
                .expect("get inserted draft"),
            Some("first draft".to_owned())
        );
        store
            .set_draft(&session.owned_id, "updated draft")
            .expect("update draft");
        assert_eq!(
            store
                .get_draft(&session.owned_id)
                .expect("get updated draft"),
            Some("updated draft".to_owned())
        );
        store.clear_draft(&session.owned_id).expect("clear draft");
        assert_eq!(
            store
                .get_draft(&session.owned_id)
                .expect("get cleared draft"),
            None
        );
    }

    #[test]
    fn annotations_roundtrip_and_cascade_on_session_delete() {
        let (_directory, _path, store) = open_temp_store();
        let session = fixture_session("owned-1", 2_000);
        store.upsert_session(&session).expect("insert session");
        store
            .set_draft(&session.owned_id, "saved draft")
            .expect("insert draft for cascade");
        let event = fixture_event(&session.owned_id, 1);
        store
            .append_event(&event)
            .expect("insert event for cascade");

        let first_id = store
            .add_annotation(&session.owned_id, "https://one.test", "[1,2,3,4]", "first")
            .expect("add first annotation");
        let second_id = store
            .add_annotation(&session.owned_id, "https://two.test", "[5,6,7,8]", "second")
            .expect("add second annotation");
        assert!(second_id > first_id);

        let annotations = store
            .list_annotations(&session.owned_id)
            .expect("list annotations");
        assert_eq!(annotations.len(), 2);
        assert_eq!(annotations[0].id, first_id);
        assert_eq!(annotations[0].created_at_ms, event.created_at_ms);
        assert_eq!(annotations[1].id, second_id);

        store
            .delete_annotation(first_id)
            .expect("delete first annotation");
        assert_eq!(
            store
                .list_annotations(&session.owned_id)
                .expect("list after annotation delete")
                .len(),
            1
        );

        store
            .delete_session(&session.owned_id)
            .expect("delete session");
        assert!(store
            .list_annotations(&session.owned_id)
            .expect("list after session delete")
            .is_empty());
        assert_eq!(
            store
                .get_draft(&session.owned_id)
                .expect("get cascaded draft"),
            None
        );
        assert!(store
            .list_events(&session.owned_id, 0, 10)
            .expect("list cascaded events")
            .is_empty());
    }

    #[test]
    fn crash_safety_recovers_committed_wal_rows() {
        let directory = TempDir::new().expect("create temporary directory");
        let path = directory.path().join("sessions.db");
        let store = SessionStore::open(&path).expect("open writer store");

        // Hold a read transaction open before the write. This prevents the writer's close from
        // checkpointing away the WAL, so the later store must read committed rows through WAL
        // recovery rather than merely reopening a fully checkpointed database file.
        let guardian = Connection::open(&path).expect("open independent reader");
        guardian
            .execute_batch("BEGIN; SELECT count(*) FROM sessions;")
            .expect("hold read transaction");

        let mut session = fixture_session("owned-wal", 2_000);
        let event = fixture_event(&session.owned_id, 1);
        store
            .upsert_session(&session)
            .expect("write session to WAL");
        store.append_event(&event).expect("write event to WAL");
        session.last_activity_at_ms = event.created_at_ms;
        let wal_path = path.with_extension("db-wal");
        assert!(fs::metadata(&wal_path).expect("WAL file exists").len() > 0);

        drop(store);
        assert!(
            fs::metadata(&wal_path)
                .expect("WAL remains after writer closes")
                .len()
                > 0
        );

        let recovered = SessionStore::open(&path).expect("open recovering store");
        assert_eq!(
            recovered
                .get_session(&session.owned_id)
                .expect("recover session"),
            Some(session)
        );
        assert_eq!(
            recovered
                .list_events("owned-wal", 0, 10)
                .expect("recover event"),
            [event]
        );

        guardian
            .execute_batch("ROLLBACK;")
            .expect("release read transaction");
    }
}
