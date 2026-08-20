use std::fmt;
use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};

const SCHEMA_VERSION: i64 = 6;

/// Event kinds where only the newest row still means anything.
///
/// Both are running state rather than history: the conversation reads each one
/// into a single value and the next one overwrites it, and neither is ever drawn
/// in the transcript. Keeping every tick of them was two thirds of every row in
/// the database — one session held two hundred and seventy-nine rows to carry a
/// twenty-three message conversation — and dropping the older ones leaves what
/// is read back byte for byte the same.
///
/// `content.delta` is deliberately not here. It looks like the same kind of
/// noise and is not: on a live session the assistant's own words are only ever
/// stored as deltas, so deleting them would delete half the conversation. Those
/// are worth merging one day, which is a rewrite rather than a delete.
const SUPERSEDED_BY_NEWER: [&str; 2] = ["usage.updated", "session.config.updated"];

/// Tool updates cannot join `SUPERSEDED_BY_NEWER` because they share one event
/// kind and must instead be superseded per item within each session.
const TOOL_ITEM_COLUMN_SCHEMA: &str = "ALTER TABLE events ADD COLUMN item_id TEXT
    GENERATED ALWAYS AS (json_extract(payload, '$.payload.itemId')) VIRTUAL;";
const TOOL_ITEM_INDEX_SCHEMA: &str =
    "CREATE INDEX IF NOT EXISTS events_item_idx ON events(owned_id, kind, item_id, seq);";

/// The attachment index, created by both the first-run schema and the upgrade
/// from version one. The row names the file and the application names the root,
/// so `relative_path` stays valid when the data folder moves to another machine.
const ATTACHMENTS_SCHEMA: &str = "CREATE TABLE attachments (
    id TEXT PRIMARY KEY,
    owned_id TEXT NOT NULL REFERENCES sessions(owned_id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    byte_length INTEGER NOT NULL,
    relative_path TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX attachments_owned_id_idx ON attachments(owned_id, file_name);";

const BROKER_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS workflow_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    orchestrator_owned_id TEXT,
    created_at_ms INTEGER NOT NULL,
    closed_at_ms INTEGER
);
CREATE TABLE IF NOT EXISTS workflow_messages (
    id TEXT PRIMARY KEY,
    group_id TEXT NOT NULL REFERENCES workflow_groups(id),
    from_agent TEXT NOT NULL,
    to_agent TEXT NOT NULL,
    kind TEXT NOT NULL,
    body TEXT NOT NULL,
    receipt TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_workflow_messages_group
    ON workflow_messages(group_id, created_at_ms);";

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug)]
pub struct StoreError {
    context: &'static str,
    source: Option<rusqlite::Error>,
}

impl StoreError {
    pub(crate) fn sqlite(context: &'static str, source: rusqlite::Error) -> Self {
        Self {
            context,
            source: Some(source),
        }
    }

    pub(crate) fn message(context: &'static str) -> Self {
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
    /// Where the title came from: the first prompt, the helper model, or the
    /// person. A row written before this column existed reads as `None`, which
    /// counts as the first prompt.
    pub title_source: Option<String>,
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

/// One backward page of events, with whether older history remains.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OlderEvents {
    pub events: Vec<EventRow>,
    pub has_more: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachmentRow {
    pub id: String,
    pub owned_id: String,
    pub file_name: String,
    pub mime_type: String,
    pub byte_length: i64,
    /// Where the file sits under the application's attachment folder.
    pub relative_path: String,
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
                            extra TEXT NOT NULL,
                            title_source TEXT
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
                    .execute_batch(ATTACHMENTS_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the attachment table", error)
                    })?;
                transaction.execute_batch(BROKER_SCHEMA).map_err(|error| {
                    StoreError::sqlite("could not create the broker tables", error)
                })?;
                add_tool_item_schema(&transaction)?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish schema creation", error)
                })?;
            }
            1 => {
                // A version one database holds real conversations, so the upgrade
                // only adds the attachment table and its index and leaves every
                // existing table and row exactly as it found them.
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| {
                        StoreError::sqlite("could not begin the attachment upgrade", error)
                    })?;
                transaction
                    .execute_batch(ATTACHMENTS_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the attachment table", error)
                    })?;
                add_tool_item_schema(&transaction)?;
                // A version one database has the same superseded rows a version
                // two one does, and goes straight to the current version, so it
                // is cleared here rather than falling through to that upgrade.
                clear_superseded_events(&transaction)?;
                add_title_source_column(&transaction)?;
                transaction.execute_batch(BROKER_SCHEMA).map_err(|error| {
                    StoreError::sqlite("could not create the broker tables", error)
                })?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the upgraded schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish the attachment upgrade", error)
                })?;
            }
            2 => {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| {
                        StoreError::sqlite("could not begin the event cleanup", error)
                    })?;
                add_tool_item_schema(&transaction)?;
                clear_superseded_events(&transaction)?;
                add_title_source_column(&transaction)?;
                transaction.execute_batch(BROKER_SCHEMA).map_err(|error| {
                    StoreError::sqlite("could not create the broker tables", error)
                })?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the cleaned schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish the event cleanup", error)
                })?;
            }
            3 => {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| {
                        StoreError::sqlite("could not begin the title source upgrade", error)
                    })?;
                add_title_source_column(&transaction)?;
                add_tool_item_schema(&transaction)?;
                clear_superseded_events(&transaction)?;
                transaction.execute_batch(BROKER_SCHEMA).map_err(|error| {
                    StoreError::sqlite("could not create the broker tables", error)
                })?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the upgraded schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish the title source upgrade", error)
                })?;
            }
            4 => {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| {
                        StoreError::sqlite("could not begin the broker upgrade", error)
                    })?;
                transaction.execute_batch(BROKER_SCHEMA).map_err(|error| {
                    StoreError::sqlite("could not create the broker tables", error)
                })?;
                add_tool_item_schema(&transaction)?;
                clear_superseded_events(&transaction)?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the upgraded schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish the broker upgrade", error)
                })?;
            }
            5 => {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| {
                        StoreError::sqlite("could not begin the tool event upgrade", error)
                    })?;
                add_tool_item_schema(&transaction)?;
                clear_superseded_events(&transaction)?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the upgraded schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish the tool event upgrade", error)
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
        upsert_session_on(&connection, row)?;
        Ok(())
    }

    /// Saves the next session row and its optional event as one durable change.
    ///
    /// The immediate transaction guarantees that a failed event insert cannot
    /// leave the session row ahead of its journal.
    ///
    /// Nothing is trimmed here. This used to keep only the newest ten thousand
    /// events of a session, which was a reasonable guard when opening a
    /// conversation meant handing over all of them. It stopped being one once a
    /// window became a bounded read: history costs disk and nothing else, and
    /// the trim silently deleted the reading a person had just scrolled back to
    /// fetch, on their next message.
    pub fn upsert_session_with_event(
        &self,
        session: &SessionRow,
        event: Option<&EventRow>,
    ) -> Result<()> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                StoreError::sqlite("could not begin the session event write", error)
            })?;
        upsert_session_on(&transaction, session)?;
        if let Some(event) = event {
            transaction
                .execute(
                    "INSERT INTO events (owned_id, seq, turn_id, kind, payload, created_at)
                     VALUES (?, ?, ?, ?, ?, ?)",
                    params![
                        event.owned_id,
                        event.seq,
                        event.turn_id,
                        event.kind,
                        event.payload_json,
                        event.created_at_ms
                    ],
                )
                .map_err(|error| StoreError::sqlite("could not append the event", error))?;
        }
        transaction
            .commit()
            .map_err(|error| StoreError::sqlite("could not finish the session event write", error))
    }

    pub fn get_session(&self, owned_id: &str) -> Result<Option<SessionRow>> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT owned_id, native_session_id, provider, model, effort, cwd, worktree,
                        branch, title, project, state, suspended, created_at, last_activity_at,
                        extra, title_source
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
                        branch, title, project, state, suspended, created_at, last_activity_at,
                        extra, title_source
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
        // Written in the same transaction as the row that supersedes them, so
        // the database is never briefly missing the value they carried.
        if SUPERSEDED_BY_NEWER.contains(&row.kind.as_str()) {
            transaction
                .execute(
                    "DELETE FROM events
                     WHERE owned_id = ? AND kind = ? AND seq < ?",
                    params![row.owned_id, row.kind, row.seq],
                )
                .map_err(|error| {
                    StoreError::sqlite("could not drop the superseded events", error)
                })?;
        }
        if row.kind == "item.updated" {
            transaction
                .execute(
                    "DELETE FROM events
                     WHERE owned_id = ?
                       AND kind = 'item.updated'
                       AND item_id = json_extract(?, '$.payload.itemId')
                       AND json_extract(payload, '$.payload.kind') = 'tool'
                       AND json_extract(?, '$.payload.kind') = 'tool'
                       AND seq < ?",
                    params![row.owned_id, row.payload_json, row.payload_json, row.seq],
                )
                .map_err(|error| {
                    StoreError::sqlite("could not drop the superseded tool updates", error)
                })?;
        }
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

    pub fn first_user_message_payload(&self, owned_id: &str) -> Result<Option<String>> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT payload
                 FROM events
                 WHERE owned_id = ?1
                   AND json_extract(payload, '$.payload.kind') = 'userMessage'
                   AND seq = (
                       SELECT MIN(seq)
                       FROM events
                       WHERE owned_id = ?1
                         AND json_extract(payload, '$.payload.kind') = 'userMessage'
                   )",
                [owned_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| StoreError::sqlite("could not read the first user message", error))
    }

    /// How many rows a byte-budgeted window will ever look at.
    ///
    /// The budget is bytes, but a running total has to be built row by row, and
    /// left unbounded that walk covers every older event in the session. This
    /// ceiling is far above any real page; it exists only so the walk is a page
    /// of work rather than a session of it.
    const WINDOW_ROW_CEILING: u32 = 2_000;

    /// The newest window of a conversation, bounded by bytes.
    ///
    /// Bytes rather than a count of rows, because a row is anything from a
    /// config record of a couple of hundred bytes to a tool result of sixteen
    /// thousand. Rows are what the reader is given; bytes are what it costs to
    /// give them, and what the transcript already uses to guess how tall a row
    /// will be. Both the limiting and the final ordering stay in SQLite.
    pub fn list_recent_events(&self, owned_id: &str, max_bytes: u32) -> Result<Vec<EventRow>> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT owned_id, seq, turn_id, kind, payload, created_at
                 FROM (
                     SELECT owned_id, seq, turn_id, kind, payload, created_at,
                            SUM(LENGTH(payload)) OVER (
                                ORDER BY seq DESC
                                ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING
                            ) AS spent
                     FROM (
                         SELECT owned_id, seq, turn_id, kind, payload, created_at
                         FROM events
                         WHERE owned_id = ?
                         ORDER BY seq DESC
                         LIMIT ?
                     )
                 )
                 WHERE COALESCE(spent, 0) <= ?
                 ORDER BY seq ASC",
            )
            .map_err(|error| {
                StoreError::sqlite("could not prepare the recent event list", error)
            })?;
        let rows = statement
            .query_map(
                params![
                    owned_id,
                    i64::from(Self::WINDOW_ROW_CEILING),
                    i64::from(max_bytes)
                ],
                event_from_row,
            )
            .map_err(|error| StoreError::sqlite("could not list recent events", error))?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|error| StoreError::sqlite("could not read the recent event list", error))
    }

    /// The window of events just older than `before_seq`, bounded by bytes.
    ///
    /// This is what scrolling up asks for. The budget is spent before a row is
    /// counted rather than after, so the oldest row always fits: a single event
    /// larger than the whole budget would otherwise return an empty page
    /// forever and the reader would never get past it.
    pub fn list_events_before(
        &self,
        owned_id: &str,
        before_seq: i64,
        max_bytes: u32,
    ) -> Result<OlderEvents> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT owned_id, seq, turn_id, kind, payload, created_at
                 FROM (
                     SELECT owned_id, seq, turn_id, kind, payload, created_at,
                            SUM(LENGTH(payload)) OVER (
                                ORDER BY seq DESC
                                ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING
                            ) AS spent
                     FROM (
                         SELECT owned_id, seq, turn_id, kind, payload, created_at
                         FROM events
                         WHERE owned_id = ? AND seq < ?
                         ORDER BY seq DESC
                         LIMIT ?
                     )
                 )
                 WHERE COALESCE(spent, 0) <= ?
                 ORDER BY seq ASC",
            )
            .map_err(|error| StoreError::sqlite("could not prepare the older event list", error))?;
        let rows = statement
            .query_map(
                params![
                    owned_id,
                    before_seq,
                    i64::from(Self::WINDOW_ROW_CEILING),
                    i64::from(max_bytes)
                ],
                event_from_row,
            )
            .map_err(|error| StoreError::sqlite("could not list older events", error))?;
        let events: Vec<EventRow> = rows
            .collect::<rusqlite::Result<_>>()
            .map_err(|error| StoreError::sqlite("could not read the older event list", error))?;
        // Whether anything older than this page exists. An index probe on the
        // primary key, not a count.
        let oldest = events.first().map_or(before_seq, |event| event.seq);
        let has_more: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM events WHERE owned_id = ? AND seq < ?)",
                params![owned_id, oldest],
                |row| row.get(0),
            )
            .map_err(|error| StoreError::sqlite("could not look past the older window", error))?;
        Ok(OlderEvents { events, has_more })
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

    /// Whether the durable journal already contains transcript display content.
    pub fn has_display_events(&self, owned_id: &str) -> Result<bool> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM events
                    WHERE owned_id = ?
                      AND kind IN ('item.started', 'item.updated', 'item.completed', 'content.delta')
                 )",
                [owned_id],
                |row| row.get(0),
            )
            .map_err(|error| {
                StoreError::sqlite("could not inspect conversation display events", error)
            })
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

    /// Records one saved attachment file against the session that owns it.
    ///
    /// The insert reads the owner from `sessions`, so an attachment for a session
    /// that is not stored fails with a plain reason instead of a foreign key error.
    pub fn add_attachment(&self, row: &AttachmentRow) -> Result<()> {
        let connection = self.lock()?;
        let changed = connection
            .execute(
                "INSERT INTO attachments (
                    id, owned_id, file_name, mime_type, byte_length, relative_path, created_at
                 )
                 SELECT ?, owned_id, ?, ?, ?, ?, ?
                 FROM sessions
                 WHERE owned_id = ?",
                params![
                    row.id,
                    row.file_name,
                    row.mime_type,
                    row.byte_length,
                    row.relative_path,
                    row.created_at_ms,
                    row.owned_id,
                ],
            )
            .map_err(|error| StoreError::sqlite("could not save the attachment", error))?;
        if changed != 1 {
            return Err(StoreError::message(
                "could not save the attachment because the session does not exist",
            ));
        }
        Ok(())
    }

    pub fn list_attachments(&self, owned_id: &str) -> Result<Vec<AttachmentRow>> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT id, owned_id, file_name, mime_type, byte_length, relative_path, created_at
                 FROM attachments
                 WHERE owned_id = ?
                 ORDER BY file_name ASC",
            )
            .map_err(|error| StoreError::sqlite("could not prepare the attachment list", error))?;
        let rows = statement
            .query_map([owned_id], attachment_from_row)
            .map_err(|error| StoreError::sqlite("could not list attachments", error))?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|error| StoreError::sqlite("could not read the attachment list", error))
    }

    pub fn delete_attachment(&self, id: &str) -> Result<()> {
        let connection = self.lock()?;
        connection
            .execute("DELETE FROM attachments WHERE id = ?", [id])
            .map_err(|error| StoreError::sqlite("could not delete the attachment", error))?;
        Ok(())
    }

    pub(crate) fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.connection
            .lock()
            .map_err(|_| StoreError::message("the session database lock is unavailable"))
    }
}

/// Keeps only the newest row of each superseded kind, per session.
///
/// Runs on the caller's connection or transaction, so an upgrade can do it as
/// part of the same durable change that records the new schema version.
fn clear_superseded_events(connection: &Connection) -> Result<()> {
    for kind in SUPERSEDED_BY_NEWER {
        connection
            .execute(
                "DELETE FROM events
                 WHERE kind = ?
                   AND seq < (
                       SELECT MAX(newest.seq)
                       FROM events AS newest
                       WHERE newest.owned_id = events.owned_id
                         AND newest.kind = events.kind
                   )",
                [kind],
            )
            .map_err(|error| StoreError::sqlite("could not clear the superseded events", error))?;
    }
    connection
        .execute(
            "DELETE FROM events
             WHERE kind = 'item.updated'
               AND json_extract(payload, '$.payload.kind') = 'tool'
               AND seq < (
                   SELECT MAX(newest.seq)
                   FROM events AS newest
                   WHERE newest.owned_id = events.owned_id
                     AND newest.kind = events.kind
                     AND newest.item_id = events.item_id
                     AND json_extract(newest.payload, '$.payload.kind') = 'tool'
               )",
            [],
        )
        .map_err(|error| {
            StoreError::sqlite("could not clear the superseded tool updates", error)
        })?;
    Ok(())
}

fn add_tool_item_schema(connection: &Connection) -> Result<()> {
    let has_item_id: bool = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM pragma_table_xinfo('events') WHERE name = 'item_id'
            )",
            [],
            |row| row.get(0),
        )
        .map_err(|error| StoreError::sqlite("could not inspect the event columns", error))?;
    if !has_item_id {
        connection
            .execute_batch(TOOL_ITEM_COLUMN_SCHEMA)
            .map_err(|error| StoreError::sqlite("could not add the tool item column", error))?;
    }
    connection
        .execute_batch(TOOL_ITEM_INDEX_SCHEMA)
        .map_err(|error| StoreError::sqlite("could not add the tool item index", error))
}

/// Writes every persisted session field on the caller's connection or transaction.
/// Where the transcript importer records how far back through a past transcript
/// a session has been read.
const IMPORT_CURSOR_KEY: &str = "import";

/// The session's stored metadata, with an import cursor it already had carried
/// forward when the writer did not bring one.
///
/// Only the importer writes this key. Every other writer rebuilds `extra` from
/// the fields the runtime knows about — generation, owner, config, capabilities,
/// rail metadata — and the cursor is not among them, so an ordinary save deleted
/// it. Resuming a past session does both at once: it reads the transcript and it
/// starts the agent, and whichever finished second decided whether reading the
/// transcript worked at all. When it lost, the reader got "Session has no import
/// cursor" instead of a conversation.
///
/// Saving is not a decision to forget. A writer that supplies its own cursor
/// still wins; this only refuses to drop one on the floor.
fn extra_json_keeping_import_cursor(connection: &Connection, row: &SessionRow) -> Result<String> {
    let Ok(serde_json::Value::Object(mut next)) =
        serde_json::from_str::<serde_json::Value>(&row.extra_json)
    else {
        return Ok(row.extra_json.clone());
    };
    if next.contains_key(IMPORT_CURSOR_KEY) {
        return Ok(row.extra_json.clone());
    }
    let stored: Option<String> = connection
        .query_row(
            "SELECT extra FROM sessions WHERE owned_id = ?",
            params![row.owned_id],
            |stored| stored.get(0),
        )
        .optional()
        .map_err(|error| {
            StoreError::sqlite("could not read the stored session metadata", error)
        })?;
    let Some(stored) = stored else {
        return Ok(row.extra_json.clone());
    };
    let Ok(serde_json::Value::Object(previous)) =
        serde_json::from_str::<serde_json::Value>(&stored)
    else {
        return Ok(row.extra_json.clone());
    };
    let Some(cursor) = previous.get(IMPORT_CURSOR_KEY) else {
        return Ok(row.extra_json.clone());
    };
    next.insert(IMPORT_CURSOR_KEY.to_string(), cursor.clone());
    serde_json::to_string(&serde_json::Value::Object(next))
        .map_err(|_| StoreError::message("could not encode the session metadata"))
}

fn upsert_session_on(connection: &Connection, row: &SessionRow) -> Result<()> {
    let extra_json = extra_json_keeping_import_cursor(connection, row)?;
    connection
        .execute(
            "INSERT INTO sessions (
                owned_id, native_session_id, provider, model, effort, cwd, worktree,
                branch, title, project, state, suspended, created_at, last_activity_at, extra,
                title_source
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
                extra = excluded.extra,
                title_source = excluded.title_source",
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
                extra_json,
                row.title_source,
            ],
        )
        .map_err(|error| StoreError::sqlite("could not save the session", error))?;
    Ok(())
}

/// Adds the column that records where a session's name came from, unless the
/// database already has it.
///
/// Version one and two databases go straight to the current version rather than
/// climbing one step at a time, so each of them adds this column as well. A
/// database whose version has been set back by hand already has the column, and
/// adding it twice is an error, so the column list is read first.
fn add_title_source_column(connection: &Connection) -> Result<()> {
    let present: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('sessions') WHERE name = 'title_source'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| StoreError::sqlite("could not read the session columns", error))?;
    if present > 0 {
        return Ok(());
    }
    connection
        .execute("ALTER TABLE sessions ADD COLUMN title_source TEXT", [])
        .map_err(|error| StoreError::sqlite("could not add the title source column", error))?;
    Ok(())
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
        title_source: row.get(15)?,
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

fn attachment_from_row(row: &Row<'_>) -> rusqlite::Result<AttachmentRow> {
    Ok(AttachmentRow {
        id: row.get(0)?,
        owned_id: row.get(1)?,
        file_name: row.get(2)?,
        mime_type: row.get(3)?,
        byte_length: row.get(4)?,
        relative_path: row.get(5)?,
        created_at_ms: row.get(6)?,
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
            title_source: None,
            project: Some("Command Bar".to_owned()),
            state: "idle".to_owned(),
            suspended: false,
            created_at_ms: 1_000,
            last_activity_at_ms: activity_ms,
            extra_json: r#"{"source":"test"}"#.to_owned(),
        }
    }

    /// Resuming a past session reads its transcript and starts its agent at the
    /// same time. The importer writes the cursor; the runtime rebuilds `extra`
    /// from what it knows, which never includes one. Whichever finished second
    /// used to decide whether reading the transcript worked, and when the
    /// runtime won the reader got "Session has no import cursor".
    #[test]
    fn saving_a_session_does_not_drop_an_import_cursor_it_was_not_given() {
        let (_directory, _path, store) = open_temp_store();
        let mut imported = fixture_session("owned-import", 2_000);
        imported.extra_json = r#"{"source":"test","import":{"transcriptPath":"/tmp/a.jsonl","cutoffOffset":42,"reachedStart":false}}"#.to_owned();
        store.upsert_session(&imported).expect("import the session");

        // What the runtime writes: no cursor, because it has never had one.
        let runtime = fixture_session("owned-import", 3_000);
        store.upsert_session(&runtime).expect("save from the runtime");

        let stored = store
            .get_session("owned-import")
            .expect("read the session back")
            .expect("the session is there");
        let extra: serde_json::Value =
            serde_json::from_str(&stored.extra_json).expect("stored metadata is JSON");
        assert_eq!(
            extra.get("import").and_then(|cursor| cursor.get("cutoffOffset")),
            Some(&serde_json::Value::from(42)),
            "the cursor survives a save that did not carry one"
        );
        assert_eq!(
            extra.get("source"),
            Some(&serde_json::Value::from("test")),
            "and the writer's own metadata is what was written"
        );
    }

    /// The importer moving its own cursor still wins.
    #[test]
    fn a_writer_that_brings_an_import_cursor_replaces_the_stored_one() {
        let (_directory, _path, store) = open_temp_store();
        let mut first = fixture_session("owned-import", 2_000);
        first.extra_json = r#"{"import":{"transcriptPath":"/tmp/a.jsonl","cutoffOffset":42,"reachedStart":false}}"#.to_owned();
        store.upsert_session(&first).expect("import the session");

        let mut moved = fixture_session("owned-import", 3_000);
        moved.extra_json = r#"{"import":{"transcriptPath":"/tmp/a.jsonl","cutoffOffset":7,"reachedStart":true}}"#.to_owned();
        store.upsert_session(&moved).expect("move the cursor");

        let stored = store
            .get_session("owned-import")
            .expect("read the session back")
            .expect("the session is there");
        let extra: serde_json::Value =
            serde_json::from_str(&stored.extra_json).expect("stored metadata is JSON");
        assert_eq!(
            extra.get("import").and_then(|cursor| cursor.get("cutoffOffset")),
            Some(&serde_json::Value::from(7))
        );
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

    fn fixture_event_of_kind(owned_id: &str, seq: i64, kind: &str) -> EventRow {
        EventRow {
            kind: kind.to_owned(),
            ..fixture_event(owned_id, seq)
        }
    }

    fn tool_event(owned_id: &str, seq: i64, kind: &str, item_id: &str, status: &str) -> EventRow {
        EventRow {
            payload_json: format!(
                r#"{{"payload":{{"kind":"tool","itemId":"{item_id}","status":"{status}"}}}}"#
            ),
            ..fixture_event_of_kind(owned_id, seq, kind)
        }
    }

    #[test]
    fn a_tool_update_replaces_the_one_before_it() {
        let (_directory, _path, store) = open_temp_store();
        store
            .upsert_session(&fixture_session("session-a", 10_000))
            .expect("write session");
        for (seq, status) in [(1, "started"), (2, "running"), (3, "finished")] {
            store
                .append_event(&tool_event("session-a", seq, "item.updated", "tool-a", status))
                .expect("append tool update");
        }
        store
            .append_event(&tool_event(
                "session-a",
                4,
                "item.updated",
                "tool-b",
                "started",
            ))
            .expect("append other tool update");

        let events = store
            .list_events("session-a", i64::MIN, 100)
            .expect("list events");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].seq, 3);
        assert!(events[0].payload_json.contains(r#""status":"finished""#));
    }

    #[test]
    fn a_tool_update_leaves_other_items_alone() {
        let (_directory, _path, store) = open_temp_store();
        store
            .upsert_session(&fixture_session("session-a", 10_000))
            .expect("write session");
        for event in [
            tool_event("session-a", 1, "item.updated", "tool-a", "started"),
            tool_event("session-a", 2, "item.updated", "tool-b", "started"),
            tool_event("session-a", 3, "item.updated", "tool-a", "finished"),
            tool_event("session-a", 4, "item.updated", "tool-b", "finished"),
        ] {
            store.append_event(&event).expect("append tool update");
        }

        let events = store
            .list_events("session-a", i64::MIN, 100)
            .expect("list events");
        assert_eq!(events.iter().map(|event| event.seq).collect::<Vec<_>>(), [3, 4]);
    }

    #[test]
    fn streamed_words_are_never_superseded() {
        let (_directory, _path, store) = open_temp_store();
        store
            .upsert_session(&fixture_session("session-a", 10_000))
            .expect("write session");
        for seq in 1..=20 {
            store
                .append_event(&fixture_event_of_kind("session-a", seq, "content.delta"))
                .expect("append delta");
        }
        assert_eq!(
            store
                .list_events("session-a", i64::MIN, 100)
                .expect("list events")
                .len(),
            20
        );
    }

    #[test]
    fn a_finished_tool_keeps_its_completion() {
        let (_directory, _path, store) = open_temp_store();
        store
            .upsert_session(&fixture_session("session-a", 10_000))
            .expect("write session");
        store
            .append_event(&tool_event(
                "session-a",
                1,
                "item.updated",
                "tool-a",
                "running",
            ))
            .expect("append tool update");
        store
            .append_event(&tool_event(
                "session-a",
                2,
                "item.completed",
                "tool-a",
                "finished",
            ))
            .expect("append tool completion");

        let events = store
            .list_events("session-a", i64::MIN, 100)
            .expect("list events");
        assert_eq!(events.iter().map(|event| event.kind.as_str()).collect::<Vec<_>>(), ["item.updated", "item.completed"]);
    }

    #[test]
    fn an_upgraded_database_loses_only_the_duplicates() {
        let directory = TempDir::new().expect("create temporary directory");
        let path = directory.path().join("sessions.db");
        let connection = Connection::open(&path).expect("create version five database");
        connection
            .execute_batch(
                "CREATE TABLE events (
                    owned_id TEXT NOT NULL,
                    seq INTEGER NOT NULL,
                    turn_id TEXT,
                    kind TEXT NOT NULL,
                    payload TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    PRIMARY KEY (owned_id, seq)
                );
                PRAGMA user_version = 5;",
            )
            .expect("create version five schema");
        for (seq, status) in [(1, "started"), (2, "running"), (3, "finished")] {
            connection
                .execute(
                    "INSERT INTO events VALUES (?, ?, NULL, 'item.updated', ?, ?)",
                    rusqlite::params![
                        "session-a",
                        seq,
                        tool_event("session-a", seq, "item.updated", "tool-a", status)
                            .payload_json,
                        10_000 + seq
                    ],
                )
                .expect("insert duplicate tool update");
        }
        connection
            .execute(
                "INSERT INTO events VALUES ('session-a', 4, NULL, 'message', '{}', 10004)",
                [],
            )
            .expect("insert message");
        drop(connection);

        let store = SessionStore::open(&path).expect("upgrade database");
        let events = store
            .list_events("session-a", i64::MIN, 100)
            .expect("list upgraded events");
        assert_eq!(events.iter().map(|event| event.seq).collect::<Vec<_>>(), [3, 4]);
        drop(store);

        let connection = Connection::open(&path).expect("inspect upgraded database");
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read schema version");
        assert_eq!(version, 6);
    }

    #[test]
    fn appending_a_running_state_event_drops_the_one_it_replaces() {
        let (_directory, _path, store) = open_temp_store();
        store
            .upsert_session(&fixture_session("session-a", 10_000))
            .expect("write session");
        for seq in 1..=4 {
            store
                .append_event(&fixture_event_of_kind("session-a", seq, "usage.updated"))
                .expect("append usage");
        }
        // The conversation is unchanged either way: only the newest usage row is
        // ever read, so what survives is what was going to be used.
        let usage = store
            .list_events("session-a", i64::MIN, 100)
            .expect("list events");
        assert_eq!(usage.len(), 1);
        assert_eq!(usage[0].seq, 4);
    }

    #[test]
    fn appending_a_conversation_event_keeps_every_one_of_them() {
        let (_directory, _path, store) = open_temp_store();
        store
            .upsert_session(&fixture_session("session-a", 10_000))
            .expect("write session");
        // A live assistant message is only ever stored as its deltas, so these
        // are the conversation itself and none of them may be dropped.
        for seq in 1..=4 {
            store
                .append_event(&fixture_event_of_kind("session-a", seq, "content.delta"))
                .expect("append delta");
        }
        assert_eq!(
            store
                .list_events("session-a", i64::MIN, 100)
                .expect("list events")
                .len(),
            4
        );
    }

    #[test]
    fn opening_an_older_database_clears_the_rows_it_filled_up_with() {
        let (_directory, path, store) = open_temp_store();
        store
            .upsert_session(&fixture_session("session-a", 10_000))
            .expect("write session");
        store
            .append_event(&fixture_event_of_kind("session-a", 1, "content.delta"))
            .expect("append delta");
        for seq in 2..=5 {
            store
                .append_event(&fixture_event_of_kind("session-a", seq, "usage.updated"))
                .expect("append usage");
        }
        drop(store);

        // Put the database back the way one written before this rule looked:
        // every usage row still there, and the older schema version on it.
        let connection = Connection::open(&path).expect("open database directly");
        for seq in 2..=4 {
            connection
                .execute(
                    "INSERT INTO events (owned_id, seq, turn_id, kind, payload, created_at)
                     VALUES ('session-a', ?, NULL, 'usage.updated', '{}', 1)",
                    [seq],
                )
                .expect("restore superseded row");
        }
        connection
            .pragma_update(None, "user_version", 2)
            .expect("set the older schema version");
        drop(connection);

        let store = SessionStore::open(&path).expect("reopen session store");
        let events = store
            .list_events("session-a", i64::MIN, 100)
            .expect("list events");
        let kinds: Vec<&str> = events.iter().map(|event| event.kind.as_str()).collect();
        assert_eq!(kinds, vec!["content.delta", "usage.updated"]);
        assert_eq!(events[1].seq, 5);
    }

    /// A database written before the helper could name a session has no record
    /// of where a name came from. Opening one adds the column and leaves every
    /// row exactly as it was, so a name already on screen stays on screen.
    #[test]
    fn schema_v4_adds_title_source() {
        let (_directory, path, store) = open_temp_store();
        store
            .upsert_session(&fixture_session("session-a", 10_000))
            .expect("write session");
        drop(store);

        // Put the database back the way one written before this column looked.
        let connection = Connection::open(&path).expect("open database directly");
        connection
            .execute("ALTER TABLE sessions DROP COLUMN title_source", [])
            .expect("drop the column an older database never had");
        connection
            .pragma_update(None, "user_version", 3)
            .expect("set the older schema version");
        drop(connection);

        let store = SessionStore::open(&path).expect("reopen session store");
        let row = store
            .get_session("session-a")
            .expect("read the upgraded row")
            .expect("the row survives the upgrade");
        assert_eq!(row.title, Some("Session session-a".to_owned()));
        assert_eq!(row.title_source, None);
        drop(store);

        let connection = Connection::open(&path).expect("inspect the upgraded database");
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read schema version");
        assert_eq!(version, super::SCHEMA_VERSION);
        let columns: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('sessions') WHERE name = 'title_source'",
                [],
                |row| row.get(0),
            )
            .expect("read the session columns");
        assert_eq!(columns, 1);
    }

    #[test]
    fn broker_tables_exist_after_migration() {
        let store = SessionStore::open_in_memory().expect("open in-memory session store");
        let connection = store.connection.lock().expect("lock session store");
        connection
            .prepare("SELECT id FROM workflow_groups")
            .expect("prepare workflow groups query");
        connection
            .prepare("SELECT receipt FROM workflow_messages")
            .expect("prepare workflow messages query");
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
        assert_eq!(version, super::SCHEMA_VERSION);

        let tables: Vec<String> = {
            let mut statement = connection
                .prepare(
                    "SELECT name FROM sqlite_master \
                     WHERE type = 'table' \
                       AND name IN ('sessions', 'events', 'drafts', 'annotations', 'attachments') \
                     ORDER BY name",
                )
                .expect("prepare schema query");
            statement
                .query_map([], |row| row.get(0))
                .expect("query schema")
                .collect::<rusqlite::Result<_>>()
                .expect("read table names")
        };
        assert_eq!(
            tables,
            ["annotations", "attachments", "drafts", "events", "sessions"]
        );
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

    /// The session row and its event are one durable change, and the journal
    /// keeps everything written to it.
    #[test]
    fn session_and_event_commit_is_atomic_and_keeps_every_event() {
        let (_directory, _path, store) = open_temp_store();
        let original = fixture_session("owned-atomic", 2_000);
        store.upsert_session(&original).expect("insert session");
        let duplicate = fixture_event(&original.owned_id, 1);
        store
            .append_event(&duplicate)
            .expect("seed duplicate event");

        let mut failed_candidate = original.clone();
        failed_candidate.state = "working".into();
        failed_candidate.last_activity_at_ms = 30_000;
        assert!(store
            .upsert_session_with_event(&failed_candidate, Some(&duplicate))
            .is_err());
        let stored_after_failure = store
            .get_session(&original.owned_id)
            .expect("read session after failure")
            .expect("session remains");
        assert_eq!(stored_after_failure.state, original.state);
        assert_eq!(
            stored_after_failure.last_activity_at_ms,
            duplicate.created_at_ms
        );

        for seq in 2..=3 {
            store
                .append_event(&fixture_event(&original.owned_id, seq))
                .expect("seed an earlier event");
        }
        let committed_event = fixture_event(&original.owned_id, 4);
        let mut committed_session = failed_candidate;
        committed_session.last_activity_at_ms = committed_event.created_at_ms;
        store
            .upsert_session_with_event(&committed_session, Some(&committed_event))
            .expect("commit session and event");

        assert_eq!(
            store
                .get_session(&original.owned_id)
                .expect("read committed session"),
            Some(committed_session)
        );
        let events = store
            .list_events(&original.owned_id, 0, 10)
            .expect("read the journal back");
        assert_eq!(
            events.iter().map(|event| event.seq).collect::<Vec<_>>(),
            [1, 2, 3, 4],
            "writing an event never drops an older one"
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

    /// A fixture event's payload is nine bytes, so a budget is stated as a
    /// multiple of that. The budget is spent before a row is counted, which is
    /// what keeps the newest row in the window however large that row is.
    const FIXTURE_PAYLOAD_BYTES: u32 = 9;

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
            .list_recent_events(&session.owned_id, FIXTURE_PAYLOAD_BYTES * 2)
            .expect("list recent event page")
            .into_iter()
            .map(|event| event.seq)
            .collect();
        assert_eq!(seqs, [4, 5, 6]);

        // A budget too small for anything still returns the newest event. A
        // window that could come back empty would leave a reader stuck behind a
        // row bigger than the budget with no way past it.
        let seqs: Vec<i64> = store
            .list_recent_events(&session.owned_id, 0)
            .expect("list a window with no budget")
            .into_iter()
            .map(|event| event.seq)
            .collect();
        assert_eq!(seqs, [6]);
    }

    /// Scrolling up asks for the window just older than what is on screen, and
    /// the answer says whether anything older remains, so the transcript knows
    /// when to stop asking.
    #[test]
    fn list_events_before_returns_the_window_just_older_than_the_cursor() {
        let (_directory, _path, store) = open_temp_store();
        let session = fixture_session("owned-older", 2_000);
        store.upsert_session(&session).expect("insert session");
        for seq in 1..=600 {
            store
                .append_event(&fixture_event(&session.owned_id, seq))
                .expect("append event");
        }

        let page = store
            .list_events_before(&session.owned_id, 400, 1_024)
            .expect("list the page before the cursor");
        let seqs: Vec<i64> = page.events.iter().map(|event| event.seq).collect();
        assert_eq!(seqs.last().copied(), Some(399), "the page ends at the cursor");
        assert!(seqs.windows(2).all(|pair| pair[1] == pair[0] + 1), "no gaps");
        let spent: usize = page
            .events
            .iter()
            .skip(1)
            .map(|event| event.payload_json.len())
            .sum();
        assert!(spent <= 1_024, "the window stays inside its budget");
        assert!(page.has_more, "there is more behind a bounded window");

        let start = store
            .list_events_before(&session.owned_id, 1, 1_024)
            .expect("list the page before the first event");
        assert!(start.events.is_empty());
        assert!(!start.has_more);
    }

    #[test]
    fn list_events_before_does_not_cross_sessions() {
        let (_directory, _path, store) = open_temp_store();
        for owned_id in ["owned-left", "owned-right"] {
            store
                .upsert_session(&fixture_session(owned_id, 2_000))
                .expect("insert session");
            for seq in 1..=10 {
                store
                    .append_event(&fixture_event(owned_id, seq))
                    .expect("append event");
            }
        }

        let page = store
            .list_events_before("owned-left", 8, FIXTURE_PAYLOAD_BYTES * 4)
            .expect("list the page before the cursor");
        assert!(page
            .events
            .iter()
            .all(|event| event.owned_id == "owned-left"));
        let seqs: Vec<i64> = page.events.iter().map(|event| event.seq).collect();
        assert_eq!(seqs, [3, 4, 5, 6, 7]);
        assert!(page.has_more);
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
