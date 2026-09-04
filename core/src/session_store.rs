use std::fmt;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

use rusqlite::{
    params, params_from_iter, Connection, InterruptHandle, OptionalExtension, Row,
    TransactionBehavior,
};

const SCHEMA_VERSION: i64 = 13;

static SESSION_STORE_OPEN_HANDLES: AtomicUsize = AtomicUsize::new(0);
static SESSION_STORE_ACTIVE_READS: AtomicUsize = AtomicUsize::new(0);
static SESSION_STORE_ACTIVE_WRITES: AtomicUsize = AtomicUsize::new(0);

const DURABLE_UI_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS session_workspaces (
    owned_id TEXT PRIMARY KEY REFERENCES sessions(owned_id) ON DELETE CASCADE,
    snapshot_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS app_settings (
    setting_key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL CHECK (json_valid(value_json)),
    updated_at INTEGER NOT NULL
);";
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
    thumbnail_mime_type TEXT,
    thumbnail_byte_length INTEGER,
    thumbnail_relative_path TEXT,
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

const ORCHESTRATION_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS orchestration_events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    sequence INTEGER,
    workflow_id TEXT,
    idempotency_key TEXT,
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json))
);
CREATE INDEX IF NOT EXISTS orchestration_events_run_idx
    ON orchestration_events(run_id);
CREATE INDEX IF NOT EXISTS orchestration_events_workflow_idx
    ON orchestration_events(workflow_id, kind);
CREATE UNIQUE INDEX IF NOT EXISTS orchestration_events_idempotency_idx
    ON orchestration_events(run_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL;";

const EVIDENCE_ARTIFACT_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS evidence_artifacts (
    id TEXT PRIMARY KEY,
    orchestration_run_id TEXT NOT NULL,
    task_id TEXT,
    agent TEXT NOT NULL,
    provider TEXT NOT NULL,
    scenario TEXT NOT NULL,
    commit_hash TEXT NOT NULL,
    branch TEXT NOT NULL,
    worktree TEXT NOT NULL,
    captured_at_ms INTEGER NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    byte_size INTEGER NOT NULL,
    original_ref TEXT NOT NULL,
    thumbnail_ref TEXT,
    thumbnail_byte_size INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL CHECK (pinned IN (0, 1)),
    expires_at_ms INTEGER
);
CREATE INDEX IF NOT EXISTS evidence_artifacts_newest_idx
    ON evidence_artifacts(captured_at_ms DESC, id ASC);
CREATE INDEX IF NOT EXISTS evidence_artifacts_run_idx
    ON evidence_artifacts(orchestration_run_id, captured_at_ms DESC);";

const NOTION_TASK_PROJECTION_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS notion_task_projections (
    source_task_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    project TEXT NOT NULL,
    status TEXT NOT NULL,
    priority TEXT,
    assignee TEXT,
    due_date TEXT,
    source_url TEXT NOT NULL,
    fetched_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS notion_task_projections_offline_page_idx
    ON notion_task_projections(
        project COLLATE NOCASE ASC,
        status COLLATE NOCASE ASC,
        due_date ASC,
        title COLLATE NOCASE ASC,
        source_task_id ASC
    );";

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

/// One directional page of events, with whether more history remains in that
/// direction.
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
    pub thumbnail_mime_type: Option<String>,
    pub thumbnail_byte_length: Option<i64>,
    pub thumbnail_relative_path: Option<String>,
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
    interrupt_handle: InterruptHandle,
    recent_events_read_active: AtomicBool,
}

struct RecentEventsReadGuard<'a>(&'a AtomicBool);

impl Drop for RecentEventsReadGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub fn session_store_open_handles() -> usize {
    SESSION_STORE_OPEN_HANDLES.load(Ordering::Relaxed)
}

pub fn session_store_active_reads() -> usize {
    SESSION_STORE_ACTIVE_READS.load(Ordering::Relaxed)
}

pub fn session_store_active_writes() -> usize {
    SESSION_STORE_ACTIVE_WRITES.load(Ordering::Relaxed)
}

#[derive(Clone, Copy)]
enum SessionStoreOperation {
    Read,
    Write,
}

pub(crate) struct SessionStoreConnection<'a> {
    connection: MutexGuard<'a, Connection>,
    operation: SessionStoreOperation,
}

impl Deref for SessionStoreConnection<'_> {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}

impl DerefMut for SessionStoreConnection<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection
    }
}

impl Drop for SessionStoreConnection<'_> {
    fn drop(&mut self) {
        match self.operation {
            SessionStoreOperation::Read => {
                SESSION_STORE_ACTIVE_READS.fetch_sub(1, Ordering::Relaxed);
            }
            SessionStoreOperation::Write => {
                SESSION_STORE_ACTIVE_WRITES.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrchestrationEventRow {
    pub id: String,
    pub run_id: String,
    pub kind: String,
    pub timestamp: String,
    pub sequence: Option<i64>,
    pub workflow_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub payload_json: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceArtifact {
    pub id: String,
    pub orchestration_run_id: String,
    pub task_id: Option<String>,
    pub agent: String,
    pub provider: String,
    pub scenario: String,
    pub commit_hash: String,
    pub branch: String,
    pub worktree: String,
    pub captured_at_ms: i64,
    pub kind: String,
    pub status: String,
    pub byte_size: i64,
    pub original_ref: String,
    pub thumbnail_ref: Option<String>,
    pub thumbnail_byte_size: i64,
    pub pinned: bool,
    pub expires_at_ms: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvidenceArtifactQuery {
    pub before_captured_at_ms: Option<i64>,
    pub before_id: Option<String>,
    pub limit: u32,
    pub task_id: Option<String>,
    pub commit_hash: Option<String>,
    pub orchestration_run_id: Option<String>,
    pub agent: Option<String>,
    pub scenario: Option<String>,
    pub captured_from_ms: Option<i64>,
    pub captured_to_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRunDiskUsage {
    pub orchestration_run_id: String,
    pub byte_size: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceDiskUsage {
    pub total_bytes: i64,
    pub runs: Vec<EvidenceRunDiskUsage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotionTaskProjection {
    pub source_task_id: String,
    pub title: String,
    pub project: String,
    pub status: String,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub due_date: Option<String>,
    pub source_url: String,
    pub fetched_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotionTaskProjectionPage {
    pub tasks: Vec<NotionTaskProjection>,
    pub projects: Vec<String>,
    pub statuses: Vec<String>,
    pub has_more: bool,
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
                transaction
                    .execute_batch(DURABLE_UI_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the durable UI tables", error)
                    })?;
                add_tool_item_schema(&transaction)?;
                transaction
                    .execute_batch(ORCHESTRATION_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the orchestration table", error)
                    })?;
                add_evidence_artifact_schema(&transaction)?;
                add_notion_task_projection_schema(&transaction)?;
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
                    .execute_batch(DURABLE_UI_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the durable UI tables", error)
                    })?;
                transaction
                    .execute_batch(ORCHESTRATION_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the orchestration table", error)
                    })?;
                add_evidence_artifact_schema(&transaction)?;
                add_notion_task_projection_schema(&transaction)?;
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
                add_attachment_thumbnail_schema(&transaction)?;
                add_tool_item_schema(&transaction)?;
                clear_superseded_events(&transaction)?;
                add_title_source_column(&transaction)?;
                transaction.execute_batch(BROKER_SCHEMA).map_err(|error| {
                    StoreError::sqlite("could not create the broker tables", error)
                })?;
                transaction
                    .execute_batch(DURABLE_UI_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the durable UI tables", error)
                    })?;
                transaction
                    .execute_batch(ORCHESTRATION_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the orchestration table", error)
                    })?;
                add_evidence_artifact_schema(&transaction)?;
                add_notion_task_projection_schema(&transaction)?;
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
                add_attachment_thumbnail_schema(&transaction)?;
                add_title_source_column(&transaction)?;
                add_tool_item_schema(&transaction)?;
                clear_superseded_events(&transaction)?;
                transaction.execute_batch(BROKER_SCHEMA).map_err(|error| {
                    StoreError::sqlite("could not create the broker tables", error)
                })?;
                transaction
                    .execute_batch(DURABLE_UI_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the durable UI tables", error)
                    })?;
                transaction
                    .execute_batch(ORCHESTRATION_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the orchestration table", error)
                    })?;
                add_evidence_artifact_schema(&transaction)?;
                add_notion_task_projection_schema(&transaction)?;
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
                add_attachment_thumbnail_schema(&transaction)?;
                transaction.execute_batch(BROKER_SCHEMA).map_err(|error| {
                    StoreError::sqlite("could not create the broker tables", error)
                })?;
                transaction
                    .execute_batch(DURABLE_UI_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the durable UI tables", error)
                    })?;
                add_tool_item_schema(&transaction)?;
                clear_superseded_events(&transaction)?;
                transaction
                    .execute_batch(ORCHESTRATION_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the orchestration table", error)
                    })?;
                add_evidence_artifact_schema(&transaction)?;
                add_notion_task_projection_schema(&transaction)?;
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
                add_attachment_thumbnail_schema(&transaction)?;
                add_tool_item_schema(&transaction)?;
                clear_superseded_events(&transaction)?;
                transaction
                    .execute_batch(DURABLE_UI_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the durable UI tables", error)
                    })?;
                transaction
                    .execute_batch(ORCHESTRATION_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the orchestration table", error)
                    })?;
                add_evidence_artifact_schema(&transaction)?;
                add_notion_task_projection_schema(&transaction)?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the upgraded schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish the tool event upgrade", error)
                })?;
            }
            6 => {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| {
                        StoreError::sqlite("could not begin the session workspace upgrade", error)
                    })?;
                add_attachment_thumbnail_schema(&transaction)?;
                transaction
                    .execute_batch(DURABLE_UI_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the durable UI tables", error)
                    })?;
                transaction
                    .execute_batch(ORCHESTRATION_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the orchestration table", error)
                    })?;
                add_evidence_artifact_schema(&transaction)?;
                add_notion_task_projection_schema(&transaction)?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the upgraded schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish the session workspace upgrade", error)
                })?;
            }
            7 => {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| {
                        StoreError::sqlite("could not begin the app settings upgrade", error)
                    })?;
                add_attachment_thumbnail_schema(&transaction)?;
                transaction
                    .execute_batch(DURABLE_UI_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the durable UI tables", error)
                    })?;
                transaction
                    .execute_batch(ORCHESTRATION_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the orchestration table", error)
                    })?;
                add_evidence_artifact_schema(&transaction)?;
                add_notion_task_projection_schema(&transaction)?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the upgraded schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish the app settings upgrade", error)
                })?;
            }
            8 => {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| {
                        StoreError::sqlite("could not begin the orchestration upgrade", error)
                    })?;
                add_attachment_thumbnail_schema(&transaction)?;
                transaction
                    .execute_batch(ORCHESTRATION_SCHEMA)
                    .map_err(|error| {
                        StoreError::sqlite("could not create the orchestration table", error)
                    })?;
                add_evidence_artifact_schema(&transaction)?;
                add_notion_task_projection_schema(&transaction)?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the upgraded schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish the orchestration upgrade", error)
                })?;
            }
            9 => {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| {
                        StoreError::sqlite(
                            "could not begin the attachment thumbnail upgrade",
                            error,
                        )
                    })?;
                add_attachment_thumbnail_schema(&transaction)?;
                add_evidence_artifact_schema(&transaction)?;
                add_notion_task_projection_schema(&transaction)?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the upgraded schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish the attachment thumbnail upgrade", error)
                })?;
            }
            10 => {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| {
                        StoreError::sqlite("could not begin the evidence artifact upgrade", error)
                    })?;
                add_evidence_artifact_schema(&transaction)?;
                add_notion_task_projection_schema(&transaction)?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the upgraded schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish the evidence artifact upgrade", error)
                })?;
            }
            11 => {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| {
                        StoreError::sqlite(
                            "could not begin the Notion task projection upgrade",
                            error,
                        )
                    })?;
                add_evidence_artifact_schema(&transaction)?;
                add_notion_task_projection_schema(&transaction)?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the upgraded schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish the Notion task projection upgrade", error)
                })?;
            }
            12 => {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| {
                        StoreError::sqlite("could not begin the evidence disk usage upgrade", error)
                    })?;
                add_evidence_artifact_schema(&transaction)?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the upgraded schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish the evidence disk usage upgrade", error)
                })?;
            }
            SCHEMA_VERSION => {}
            _ => {
                return Err(StoreError::message(
                    "the session database schema version is not supported",
                ));
            }
        }
        connection
            .execute_batch(ORCHESTRATION_SCHEMA)
            .map_err(|error| {
                StoreError::sqlite("could not create the orchestration table", error)
            })?;
        add_evidence_artifact_schema(&connection)?;
        add_notion_task_projection_schema(&connection)?;

        let interrupt_handle = connection.get_interrupt_handle();
        SESSION_STORE_OPEN_HANDLES.fetch_add(1, Ordering::Relaxed);
        Ok(Self {
            connection: Mutex::new(connection),
            interrupt_handle,
            recent_events_read_active: AtomicBool::new(false),
        })
    }

    pub fn upsert_session(&self, row: &SessionRow) -> Result<()> {
        let connection = self.lock_write()?;
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
        let mut connection = self.lock_write()?;
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

    /// Saves a session row and event while removing its stale workspace as one
    /// durable checkout change.
    pub fn upsert_session_with_event_and_clear_workspace(
        &self,
        session: &SessionRow,
        event: Option<&EventRow>,
    ) -> Result<()> {
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| StoreError::sqlite("could not begin the checkout change", error))?;
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
            .execute(
                "DELETE FROM session_workspaces WHERE owned_id = ?",
                [session.owned_id.as_str()],
            )
            .map_err(|error| StoreError::sqlite("could not clear the session workspace", error))?;
        transaction
            .commit()
            .map_err(|error| StoreError::sqlite("could not finish the checkout change", error))
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

    pub fn count_sessions(&self) -> Result<usize> {
        let connection = self.lock()?;
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .map_err(|error| StoreError::sqlite("could not count sessions", error))?;
        usize::try_from(count)
            .map_err(|_| StoreError::message("the session count could not be represented"))
    }

    pub fn delete_session(&self, owned_id: &str) -> Result<()> {
        let connection = self.lock_write()?;
        connection
            .execute("DELETE FROM sessions WHERE owned_id = ?", [owned_id])
            .map_err(|error| StoreError::sqlite("could not delete the session", error))?;
        Ok(())
    }

    pub fn upsert_workspace_snapshot(&self, owned_id: &str, snapshot_json: &str) -> Result<()> {
        let connection = self.lock_write()?;
        let changed = connection
            .execute(
                "INSERT INTO session_workspaces (owned_id, snapshot_json, updated_at)
                 SELECT owned_id, ?, CAST(strftime('%s', 'now') AS INTEGER) * 1000
                 FROM sessions WHERE owned_id = ?
                 ON CONFLICT(owned_id) DO UPDATE SET
                    snapshot_json = excluded.snapshot_json,
                    updated_at = excluded.updated_at",
                params![snapshot_json, owned_id],
            )
            .map_err(|error| StoreError::sqlite("could not save the session workspace", error))?;
        if changed != 1 {
            return Err(StoreError::message(
                "could not save the workspace because the session does not exist",
            ));
        }
        Ok(())
    }

    pub fn get_workspace_snapshot(&self, owned_id: &str) -> Result<Option<String>> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT snapshot_json FROM session_workspaces WHERE owned_id = ?",
                [owned_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| StoreError::sqlite("could not read the session workspace", error))
    }

    /// Reads only one checkout's expanded directory paths. The full workspace
    /// snapshot remains in SQLite and never crosses into the active tree
    /// projection just to restore folder disclosure state.
    pub fn get_workspace_expanded_paths(&self, owned_id: &str, root: &str) -> Result<Vec<String>> {
        let connection = self.lock()?;
        let snapshot: Option<String> = connection
            .query_row(
                "SELECT snapshot_json FROM session_workspaces WHERE owned_id = ?",
                [owned_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| StoreError::sqlite("could not read tree expansion state", error))?;
        let Some(snapshot) = snapshot else {
            return Ok(Vec::new());
        };
        let value: serde_json::Value = serde_json::from_str(&snapshot)
            .map_err(|_| StoreError::message("the session workspace is not valid JSON"))?;
        Ok(value
            .get("expandedPathsByRoot")
            .and_then(|roots| roots.get(root))
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|path| path.as_str().map(str::to_owned))
            .collect())
    }

    /// Merges one checkout's expanded directory paths into the durable
    /// workspace row without reading or replacing editor/tab state in the
    /// frontend.
    pub fn set_workspace_expanded_paths(
        &self,
        owned_id: &str,
        root: &str,
        paths: &[String],
    ) -> Result<()> {
        let connection = self.lock_write()?;
        let stored: Option<String> = connection
            .query_row(
                "SELECT snapshot_json FROM session_workspaces WHERE owned_id = ?",
                [owned_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| StoreError::sqlite("could not read tree expansion state", error))?;
        let mut snapshot = match stored {
            Some(snapshot) => serde_json::from_str::<serde_json::Value>(&snapshot)
                .map_err(|_| StoreError::message("the session workspace is not valid JSON"))?,
            None => serde_json::json!({}),
        };
        let object = snapshot
            .as_object_mut()
            .ok_or_else(|| StoreError::message("the session workspace is not a JSON object"))?;
        let roots = object
            .entry("expandedPathsByRoot")
            .or_insert_with(|| serde_json::json!({}));
        if !roots.is_object() {
            *roots = serde_json::json!({});
        }
        roots
            .as_object_mut()
            .expect("tree expansion roots were normalized to an object")
            .insert(root.to_string(), serde_json::json!(paths));
        let snapshot = serde_json::to_string(&snapshot)
            .map_err(|_| StoreError::message("could not encode tree expansion state"))?;
        let changed = connection
            .execute(
                "INSERT INTO session_workspaces (owned_id, snapshot_json, updated_at)
                 SELECT owned_id, ?, CAST(strftime('%s', 'now') AS INTEGER) * 1000
                 FROM sessions WHERE owned_id = ?
                 ON CONFLICT(owned_id) DO UPDATE SET
                    snapshot_json = excluded.snapshot_json,
                    updated_at = excluded.updated_at",
                params![snapshot, owned_id],
            )
            .map_err(|error| StoreError::sqlite("could not save tree expansion state", error))?;
        if changed != 1 {
            return Err(StoreError::message(
                "could not save tree expansion state because the session does not exist",
            ));
        }
        Ok(())
    }

    pub fn delete_workspace_snapshot(&self, owned_id: &str) -> Result<()> {
        let connection = self.lock_write()?;
        connection
            .execute(
                "DELETE FROM session_workspaces WHERE owned_id = ?",
                [owned_id],
            )
            .map_err(|error| StoreError::sqlite("could not delete the session workspace", error))?;
        Ok(())
    }

    pub fn clear_workspace_editor_tabs(&self) -> Result<()> {
        let connection = self.lock_write()?;
        connection
            .execute(
                "UPDATE session_workspaces
                 SET snapshot_json = json_remove(
                        json_set(
                            CASE WHEN json_type(snapshot_json) = 'object' THEN snapshot_json ELSE '{}' END,
                            '$.openPaths', json('[]'), '$.activePath', json('null')
                        ),
                        '$.fileStates'
                     ),
                     updated_at = CAST(strftime('%s', 'now') AS INTEGER) * 1000
                 WHERE json_valid(snapshot_json)",
                [],
            )
            .map_err(|error| StoreError::sqlite("could not clear session workspace editors", error))?;
        Ok(())
    }

    pub fn clear_workspace_selected_tabs(&self) -> Result<()> {
        let connection = self.lock_write()?;
        connection
            .execute(
                "UPDATE session_workspaces
                 SET snapshot_json = json_set(
                        CASE WHEN json_type(snapshot_json) = 'object' THEN snapshot_json ELSE '{}' END,
                        '$.rightTab', 'files', '$.center.activePanelId', 'session'
                     ),
                     updated_at = CAST(strftime('%s', 'now') AS INTEGER) * 1000
                 WHERE json_valid(snapshot_json)",
                [],
            )
            .map_err(|error| StoreError::sqlite("could not reset session workspace tabs", error))?;
        Ok(())
    }

    pub fn upsert_app_setting(&self, setting_key: &str, value_json: &str) -> Result<()> {
        let connection = self.lock_write()?;
        connection
            .execute(
                "INSERT INTO app_settings (setting_key, value_json, updated_at)
                 VALUES (?, ?, CAST(strftime('%s', 'now') AS INTEGER) * 1000)
                 ON CONFLICT(setting_key) DO UPDATE SET
                    value_json = excluded.value_json,
                    updated_at = excluded.updated_at",
                params![setting_key, value_json],
            )
            .map_err(|error| StoreError::sqlite("could not save the app setting", error))?;
        Ok(())
    }

    pub fn get_app_setting(&self, setting_key: &str) -> Result<Option<String>> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT value_json FROM app_settings WHERE setting_key = ?",
                [setting_key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| StoreError::sqlite("could not read the app setting", error))
    }

    pub fn append_orchestration_event(&self, row: &OrchestrationEventRow) -> Result<()> {
        let connection = self.lock_write()?;
        insert_orchestration_event_on(&connection, row)
    }

    pub fn import_orchestration_events(&self, rows: &[OrchestrationEventRow]) -> Result<()> {
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                StoreError::sqlite("could not begin the orchestration import", error)
            })?;
        for row in rows {
            insert_orchestration_event_or_ignore_on(&transaction, row)?;
        }
        transaction
            .commit()
            .map_err(|error| StoreError::sqlite("could not finish the orchestration import", error))
    }

    pub fn list_orchestration_events(&self) -> Result<Vec<OrchestrationEventRow>> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT id, run_id, kind, timestamp, sequence, workflow_id, idempotency_key,
                        payload_json
                 FROM orchestration_events
                 ORDER BY rowid ASC",
            )
            .map_err(|error| {
                StoreError::sqlite("could not prepare the orchestration event list", error)
            })?;
        let rows = statement
            .query_map([], orchestration_event_from_row)
            .map_err(|error| StoreError::sqlite("could not list orchestration events", error))?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|error| StoreError::sqlite("could not read orchestration events", error))
    }

    pub fn list_workflow_orchestration_events(&self) -> Result<Vec<OrchestrationEventRow>> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT id, run_id, kind, timestamp, sequence, workflow_id, idempotency_key,
                        payload_json
                 FROM orchestration_events
                 WHERE workflow_id IS NOT NULL AND kind LIKE 'workflow.%'
                 ORDER BY rowid ASC",
            )
            .map_err(|error| {
                StoreError::sqlite("could not prepare the workflow event list", error)
            })?;
        let rows = statement
            .query_map([], orchestration_event_from_row)
            .map_err(|error| StoreError::sqlite("could not list workflow events", error))?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|error| StoreError::sqlite("could not read workflow events", error))
    }

    pub fn find_orchestration_event_by_idempotency_key(
        &self,
        run_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<OrchestrationEventRow>> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT id, run_id, kind, timestamp, sequence, workflow_id, idempotency_key,
                        payload_json
                 FROM orchestration_events
                 WHERE run_id = ? AND idempotency_key = ?
                 ORDER BY rowid ASC
                 LIMIT 1",
                params![run_id, idempotency_key],
                orchestration_event_from_row,
            )
            .optional()
            .map_err(|error| {
                StoreError::sqlite("could not read the idempotent orchestration event", error)
            })
    }

    pub fn latest_orchestration_sequence(&self, run_id: &str) -> Result<i64> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT COALESCE(MAX(sequence), 0)
                 FROM orchestration_events
                 WHERE run_id = ?",
                [run_id],
                |row| row.get(0),
            )
            .map_err(|error| {
                StoreError::sqlite("could not read the latest orchestration sequence", error)
            })
    }

    pub fn upsert_evidence_artifact(&self, artifact: &EvidenceArtifact) -> Result<()> {
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                StoreError::sqlite("could not begin the evidence artifact write", error)
            })?;
        transaction
            .execute(
                "INSERT INTO evidence_artifacts (
                    id, orchestration_run_id, task_id, agent, provider, scenario,
                    commit_hash, branch, worktree, captured_at_ms, kind, status, byte_size,
                    original_ref, thumbnail_ref, thumbnail_byte_size, pinned, expires_at_ms
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                    orchestration_run_id = excluded.orchestration_run_id,
                    task_id = excluded.task_id,
                    agent = excluded.agent,
                    provider = excluded.provider,
                    scenario = excluded.scenario,
                    commit_hash = excluded.commit_hash,
                    branch = excluded.branch,
                    worktree = excluded.worktree,
                    captured_at_ms = excluded.captured_at_ms,
                    kind = excluded.kind,
                    status = excluded.status,
                    byte_size = excluded.byte_size,
                    original_ref = excluded.original_ref,
                    thumbnail_ref = excluded.thumbnail_ref,
                    thumbnail_byte_size = excluded.thumbnail_byte_size,
                    pinned = excluded.pinned,
                    expires_at_ms = excluded.expires_at_ms",
                params![
                    artifact.id,
                    artifact.orchestration_run_id,
                    artifact.task_id,
                    artifact.agent,
                    artifact.provider,
                    artifact.scenario,
                    artifact.commit_hash,
                    artifact.branch,
                    artifact.worktree,
                    artifact.captured_at_ms,
                    artifact.kind,
                    artifact.status,
                    artifact.byte_size,
                    artifact.original_ref,
                    artifact.thumbnail_ref,
                    artifact.thumbnail_byte_size,
                    artifact.pinned,
                    artifact.expires_at_ms,
                ],
            )
            .map_err(|error| StoreError::sqlite("could not save the evidence artifact", error))?;
        transaction.commit().map_err(|error| {
            StoreError::sqlite("could not finish the evidence artifact write", error)
        })
    }

    pub fn list_evidence_artifacts(
        &self,
        query: &EvidenceArtifactQuery,
    ) -> Result<Vec<EvidenceArtifact>> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT id, orchestration_run_id, task_id, agent, provider, scenario,
                        commit_hash, branch, worktree, captured_at_ms, kind, status, byte_size,
                        original_ref, thumbnail_ref, thumbnail_byte_size, pinned, expires_at_ms
                 FROM evidence_artifacts
                 WHERE (
                    ?1 IS NULL
                    OR captured_at_ms < ?1
                    OR (captured_at_ms = ?1 AND id > ?2)
                 )
                   AND (?3 IS NULL OR task_id = ?3)
                   AND (?4 IS NULL OR commit_hash = ?4)
                   AND (?5 IS NULL OR orchestration_run_id = ?5)
                   AND (?6 IS NULL OR agent = ?6)
                   AND (?7 IS NULL OR scenario = ?7)
                   AND (?8 IS NULL OR captured_at_ms >= ?8)
                   AND (?9 IS NULL OR captured_at_ms <= ?9)
                 ORDER BY captured_at_ms DESC, id ASC
                 LIMIT ?10",
            )
            .map_err(|error| {
                StoreError::sqlite("could not prepare the evidence artifact page", error)
            })?;
        let rows = statement
            .query_map(
                params![
                    query.before_captured_at_ms,
                    query.before_id,
                    query.task_id,
                    query.commit_hash,
                    query.orchestration_run_id,
                    query.agent,
                    query.scenario,
                    query.captured_from_ms,
                    query.captured_to_ms,
                    i64::from(query.limit),
                ],
                evidence_artifact_from_row,
            )
            .map_err(|error| StoreError::sqlite("could not list evidence artifacts", error))?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|error| StoreError::sqlite("could not read the evidence artifact page", error))
    }

    pub fn evidence_artifact(&self, id: &str) -> Result<Option<EvidenceArtifact>> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT id, orchestration_run_id, task_id, agent, provider, scenario,
                        commit_hash, branch, worktree, captured_at_ms, kind, status, byte_size,
                        original_ref, thumbnail_ref, thumbnail_byte_size, pinned, expires_at_ms
                 FROM evidence_artifacts
                 WHERE id = ?",
                [id],
                evidence_artifact_from_row,
            )
            .optional()
            .map_err(|error| StoreError::sqlite("could not read the evidence artifact", error))
    }

    pub fn evidence_disk_usage(&self) -> Result<EvidenceDiskUsage> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT orchestration_run_id,
                        SUM(byte_size + thumbnail_byte_size) AS run_bytes,
                        SUM(SUM(byte_size + thumbnail_byte_size)) OVER () AS total_bytes
                 FROM evidence_artifacts
                 GROUP BY orchestration_run_id
                 ORDER BY run_bytes DESC, orchestration_run_id ASC",
            )
            .map_err(|error| StoreError::sqlite("could not prepare evidence disk usage", error))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    EvidenceRunDiskUsage {
                        orchestration_run_id: row.get(0)?,
                        byte_size: row.get(1)?,
                    },
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|error| StoreError::sqlite("could not read evidence disk usage", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| StoreError::sqlite("could not read evidence disk usage", error))?;
        Ok(EvidenceDiskUsage {
            total_bytes: rows.first().map(|(_, total)| *total).unwrap_or(0),
            runs: rows.into_iter().map(|(run, _)| run).collect(),
        })
    }

    pub fn delete_expired_unpinned_evidence_artifacts(
        &self,
        now_ms: i64,
        retention_ms: i64,
        limit: u32,
    ) -> Result<Vec<EvidenceArtifact>> {
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                StoreError::sqlite("could not begin expired evidence cleanup", error)
            })?;
        let artifacts = {
            let mut statement = transaction
                .prepare(
                    "DELETE FROM evidence_artifacts
                     WHERE id IN (
                        SELECT id
                        FROM evidence_artifacts
                        WHERE pinned = 0
                          AND COALESCE(expires_at_ms, captured_at_ms + ?2) <= ?1
                        ORDER BY COALESCE(expires_at_ms, captured_at_ms + ?2) ASC, id ASC
                        LIMIT ?3
                     )
                     RETURNING id, orchestration_run_id, task_id, agent, provider, scenario,
                               commit_hash, branch, worktree, captured_at_ms, kind, status,
                               byte_size, original_ref, thumbnail_ref, thumbnail_byte_size,
                               pinned, expires_at_ms",
                )
                .map_err(|error| {
                    StoreError::sqlite("could not prepare expired evidence cleanup", error)
                })?;
            let rows = statement
                .query_map(
                    params![now_ms, retention_ms, i64::from(limit)],
                    evidence_artifact_from_row,
                )
                .map_err(|error| {
                    StoreError::sqlite("could not remove expired evidence rows", error)
                })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| {
                    StoreError::sqlite("could not read removed evidence rows", error)
                })?
        };
        transaction.commit().map_err(|error| {
            StoreError::sqlite("could not finish expired evidence cleanup", error)
        })?;
        Ok(artifacts)
    }

    pub fn set_evidence_artifact_pinned(&self, id: &str, pinned: bool) -> Result<bool> {
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                StoreError::sqlite("could not begin the evidence artifact pin update", error)
            })?;
        let updated = transaction
            .execute(
                "UPDATE evidence_artifacts SET pinned = ? WHERE id = ?",
                params![pinned, id],
            )
            .map_err(|error| {
                StoreError::sqlite("could not update the evidence artifact pin", error)
            })?;
        transaction.commit().map_err(|error| {
            StoreError::sqlite("could not finish the evidence artifact pin update", error)
        })?;
        Ok(updated > 0)
    }

    pub fn set_evidence_run_pinned(&self, run_id: &str, pinned: bool) -> Result<usize> {
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                StoreError::sqlite("could not begin the evidence run pin update", error)
            })?;
        let updated = transaction
            .execute(
                "UPDATE evidence_artifacts SET pinned = ? WHERE orchestration_run_id = ?",
                params![pinned, run_id],
            )
            .map_err(|error| StoreError::sqlite("could not update the evidence run pin", error))?;
        transaction.commit().map_err(|error| {
            StoreError::sqlite("could not finish the evidence run pin update", error)
        })?;
        Ok(updated)
    }

    pub fn delete_evidence_artifacts(&self, ids: &[String]) -> Result<Vec<EvidenceArtifact>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                StoreError::sqlite("could not begin the evidence artifact delete", error)
            })?;
        let placeholders = (0..ids.len()).map(|_| "?").collect::<Vec<_>>().join(", ");
        let artifacts = {
            let mut statement = transaction
                .prepare(&format!(
                    "DELETE FROM evidence_artifacts
                     WHERE id IN ({placeholders})
                     RETURNING id, orchestration_run_id, task_id, agent, provider, scenario,
                               commit_hash, branch, worktree, captured_at_ms, kind, status,
                               byte_size, original_ref, thumbnail_ref, thumbnail_byte_size,
                               pinned, expires_at_ms"
                ))
                .map_err(|error| {
                    StoreError::sqlite("could not prepare the evidence artifact delete", error)
                })?;
            let rows = statement
                .query_map(params_from_iter(ids), evidence_artifact_from_row)
                .map_err(|error| {
                    StoreError::sqlite("could not delete the evidence artifacts", error)
                })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| {
                    StoreError::sqlite("could not read the deleted evidence artifacts", error)
                })?
        };
        transaction.commit().map_err(|error| {
            StoreError::sqlite("could not finish the evidence artifact delete", error)
        })?;
        Ok(artifacts)
    }

    pub fn delete_evidence_run(&self, run_id: &str) -> Result<Vec<EvidenceArtifact>> {
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                StoreError::sqlite("could not begin the evidence run delete", error)
            })?;
        let artifacts = {
            let mut statement = transaction
                .prepare(
                    "DELETE FROM evidence_artifacts
                     WHERE orchestration_run_id = ?
                     RETURNING id, orchestration_run_id, task_id, agent, provider, scenario,
                               commit_hash, branch, worktree, captured_at_ms, kind, status,
                               byte_size, original_ref, thumbnail_ref, thumbnail_byte_size,
                               pinned, expires_at_ms",
                )
                .map_err(|error| {
                    StoreError::sqlite("could not prepare the evidence run delete", error)
                })?;
            let rows = statement
                .query_map([run_id], evidence_artifact_from_row)
                .map_err(|error| StoreError::sqlite("could not delete the evidence run", error))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| {
                    StoreError::sqlite("could not read the deleted evidence run", error)
                })?
        };
        transaction.commit().map_err(|error| {
            StoreError::sqlite("could not finish the evidence run delete", error)
        })?;
        Ok(artifacts)
    }

    pub fn replace_notion_task_projections(&self, tasks: &[NotionTaskProjection]) -> Result<()> {
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                StoreError::sqlite("could not begin the Notion task projection snapshot", error)
            })?;
        transaction
            .execute("DELETE FROM notion_task_projections", [])
            .map_err(|error| {
                StoreError::sqlite("could not clear the Notion task projection snapshot", error)
            })?;
        for task in tasks {
            transaction
                .execute(
                    "INSERT INTO notion_task_projections (
                        source_task_id, title, project, status, priority, assignee, due_date,
                        source_url, fetched_at_ms
                     )
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    params![
                        task.source_task_id,
                        task.title,
                        task.project,
                        task.status,
                        task.priority,
                        task.assignee,
                        task.due_date,
                        task.source_url,
                        task.fetched_at_ms,
                    ],
                )
                .map_err(|error| {
                    StoreError::sqlite("could not save a Notion task projection", error)
                })?;
        }
        transaction.commit().map_err(|error| {
            StoreError::sqlite(
                "could not finish the Notion task projection snapshot",
                error,
            )
        })
    }

    pub fn query_notion_task_projections(
        &self,
        offset: u32,
        limit: u32,
        search: &str,
        project: &str,
        status: &str,
    ) -> Result<NotionTaskProjectionPage> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT source_task_id, title, project, status, priority, assignee, due_date,
                        source_url, fetched_at_ms
                 FROM notion_task_projections
                 WHERE (?1 = '' OR
                        instr(lower(title), lower(?1)) > 0 OR
                        instr(lower(project), lower(?1)) > 0 OR
                        instr(lower(status), lower(?1)) > 0 OR
                        instr(lower(COALESCE(priority, '')), lower(?1)) > 0 OR
                        instr(lower(COALESCE(assignee, '')), lower(?1)) > 0)
                   AND (?2 = '' OR project = ?2)
                   AND (?3 <> '' OR lower(status) <> 'future')
                   AND (?3 = '' OR status = ?3)
                 ORDER BY project COLLATE NOCASE ASC,
                          status COLLATE NOCASE ASC,
                          due_date IS NULL ASC,
                          due_date ASC,
                          title COLLATE NOCASE ASC,
                          source_task_id ASC
                 LIMIT ?4 OFFSET ?5",
            )
            .map_err(|error| {
                StoreError::sqlite("could not prepare the Notion task projection page", error)
            })?;
        let lookahead_limit = limit.saturating_add(1);
        let rows = statement
            .query_map(
                params![
                    search.trim(),
                    project.trim(),
                    status.trim(),
                    i64::from(lookahead_limit),
                    i64::from(offset)
                ],
                |row| {
                    Ok(NotionTaskProjection {
                        source_task_id: row.get(0)?,
                        title: row.get(1)?,
                        project: row.get(2)?,
                        status: row.get(3)?,
                        priority: row.get(4)?,
                        assignee: row.get(5)?,
                        due_date: row.get(6)?,
                        source_url: row.get(7)?,
                        fetched_at_ms: row.get(8)?,
                    })
                },
            )
            .map_err(|error| StoreError::sqlite("could not list Notion task projections", error))?;
        let mut tasks = rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| {
                StoreError::sqlite("could not read the Notion task projection page", error)
            })?;
        let has_more = tasks.len() > limit as usize;
        if has_more {
            tasks.pop();
        }

        let mut project_statement = connection
            .prepare(
                "SELECT DISTINCT project
                 FROM notion_task_projections
                 WHERE project <> ''
                 ORDER BY project COLLATE NOCASE ASC",
            )
            .map_err(|error| {
                StoreError::sqlite("could not prepare Notion task project filters", error)
            })?;
        let projects = project_statement
            .query_map([], |row| row.get(0))
            .map_err(|error| {
                StoreError::sqlite("could not list Notion task project filters", error)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| {
                StoreError::sqlite("could not read Notion task project filters", error)
            })?;

        let mut status_statement = connection
            .prepare(
                "SELECT DISTINCT status
                 FROM notion_task_projections
                 WHERE status <> ''
                 ORDER BY status COLLATE NOCASE ASC",
            )
            .map_err(|error| {
                StoreError::sqlite("could not prepare Notion task status filters", error)
            })?;
        let statuses = status_statement
            .query_map([], |row| row.get(0))
            .map_err(|error| {
                StoreError::sqlite("could not list Notion task status filters", error)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| {
                StoreError::sqlite("could not read Notion task status filters", error)
            })?;

        Ok(NotionTaskProjectionPage {
            tasks,
            projects,
            statuses,
            has_more,
        })
    }

    pub fn append_event(&self, row: &EventRow) -> Result<()> {
        let mut connection = self.lock_write()?;
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
        Self::query_recent_events(&connection, owned_id, max_bytes)
    }

    /// Reads the active conversation window while allowing a newer activation
    /// to interrupt SQLite itself, rather than waiting for the old query and
    /// merely throwing its completed object graph away.
    pub fn list_recent_events_cancellable(
        &self,
        owned_id: &str,
        max_bytes: u32,
    ) -> Result<Vec<EventRow>> {
        let connection = self.lock()?;
        self.recent_events_read_active
            .store(true, Ordering::Release);
        let _active_read = RecentEventsReadGuard(&self.recent_events_read_active);
        Self::query_recent_events(&connection, owned_id, max_bytes)
    }

    /// Interrupts only the active-session conversation read. Other SQLite
    /// reads and writes never set this ownership flag and are left alone.
    pub fn cancel_recent_events_read(&self) {
        if self.recent_events_read_active.load(Ordering::Acquire) {
            self.interrupt_handle.interrupt();
        }
    }

    fn query_recent_events(
        connection: &Connection,
        owned_id: &str,
        max_bytes: u32,
    ) -> Result<Vec<EventRow>> {
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

    /// The window of events just newer than `after_seq`, bounded by bytes.
    pub fn list_events_after(
        &self,
        owned_id: &str,
        after_seq: i64,
        max_bytes: u32,
    ) -> Result<OlderEvents> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT owned_id, seq, turn_id, kind, payload, created_at
                 FROM (
                     SELECT owned_id, seq, turn_id, kind, payload, created_at,
                            SUM(LENGTH(payload)) OVER (
                                ORDER BY seq ASC
                                ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING
                            ) AS spent
                     FROM (
                         SELECT owned_id, seq, turn_id, kind, payload, created_at
                         FROM events
                         WHERE owned_id = ? AND seq > ?
                         ORDER BY seq ASC
                         LIMIT ?
                     )
                 )
                 WHERE COALESCE(spent, 0) <= ?
                 ORDER BY seq ASC",
            )
            .map_err(|error| StoreError::sqlite("could not prepare the newer event list", error))?;
        let rows = statement
            .query_map(
                params![
                    owned_id,
                    after_seq,
                    i64::from(Self::WINDOW_ROW_CEILING),
                    i64::from(max_bytes)
                ],
                event_from_row,
            )
            .map_err(|error| StoreError::sqlite("could not list newer events", error))?;
        let events: Vec<EventRow> = rows
            .collect::<rusqlite::Result<_>>()
            .map_err(|error| StoreError::sqlite("could not read the newer event list", error))?;
        let newest = events.last().map_or(after_seq, |event| event.seq);
        let has_more: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM events WHERE owned_id = ? AND seq > ?)",
                params![owned_id, newest],
                |row| row.get(0),
            )
            .map_err(|error| StoreError::sqlite("could not look past the newer window", error))?;
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
        let connection = self.lock_write()?;
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
        let connection = self.lock_write()?;
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
        let connection = self.lock_write()?;
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
        let connection = self.lock_write()?;
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
        let connection = self.lock_write()?;
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
        let connection = self.lock_write()?;
        let changed = connection
            .execute(
                "INSERT INTO attachments (
                    id, owned_id, file_name, mime_type, byte_length, relative_path,
                    thumbnail_mime_type, thumbnail_byte_length, thumbnail_relative_path,
                    created_at
                 )
                 SELECT ?, owned_id, ?, ?, ?, ?, ?, ?, ?, ?
                 FROM sessions
                 WHERE owned_id = ?",
                params![
                    row.id,
                    row.file_name,
                    row.mime_type,
                    row.byte_length,
                    row.relative_path,
                    row.thumbnail_mime_type,
                    row.thumbnail_byte_length,
                    row.thumbnail_relative_path,
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
                "SELECT id, owned_id, file_name, mime_type, byte_length, relative_path,
                    thumbnail_mime_type, thumbnail_byte_length, thumbnail_relative_path,
                    created_at
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

    pub fn update_attachment_thumbnail(
        &self,
        id: &str,
        mime_type: &str,
        byte_length: i64,
        relative_path: &str,
    ) -> Result<()> {
        let connection = self.lock_write()?;
        let changed = connection
            .execute(
                "UPDATE attachments
                 SET thumbnail_mime_type = ?, thumbnail_byte_length = ?, thumbnail_relative_path = ?
                 WHERE id = ?",
                params![mime_type, byte_length, relative_path, id],
            )
            .map_err(|error| {
                StoreError::sqlite("could not save the attachment thumbnail", error)
            })?;
        if changed != 1 {
            return Err(StoreError::message(
                "could not save the attachment thumbnail because the attachment does not exist",
            ));
        }
        Ok(())
    }

    pub fn delete_attachment(&self, id: &str) -> Result<()> {
        let connection = self.lock_write()?;
        connection
            .execute("DELETE FROM attachments WHERE id = ?", [id])
            .map_err(|error| StoreError::sqlite("could not delete the attachment", error))?;
        Ok(())
    }

    pub(crate) fn lock(&self) -> Result<SessionStoreConnection<'_>> {
        self.lock_operation(SessionStoreOperation::Read)
    }

    pub(crate) fn lock_write(&self) -> Result<SessionStoreConnection<'_>> {
        self.lock_operation(SessionStoreOperation::Write)
    }

    fn lock_operation(
        &self,
        operation: SessionStoreOperation,
    ) -> Result<SessionStoreConnection<'_>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreError::message("the session database lock is unavailable"))?;
        match operation {
            SessionStoreOperation::Read => {
                SESSION_STORE_ACTIVE_READS.fetch_add(1, Ordering::Relaxed);
            }
            SessionStoreOperation::Write => {
                SESSION_STORE_ACTIVE_WRITES.fetch_add(1, Ordering::Relaxed);
            }
        }
        Ok(SessionStoreConnection {
            connection,
            operation,
        })
    }
}

impl Drop for SessionStore {
    fn drop(&mut self) {
        SESSION_STORE_OPEN_HANDLES.fetch_sub(1, Ordering::Relaxed);
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

fn add_attachment_thumbnail_schema(connection: &Connection) -> Result<()> {
    let table_present: bool = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'attachments'
            )",
            [],
            |row| row.get(0),
        )
        .map_err(|error| StoreError::sqlite("could not inspect the attachment table", error))?;
    if !table_present {
        connection
            .execute_batch(ATTACHMENTS_SCHEMA)
            .map_err(|error| StoreError::sqlite("could not create the attachment table", error))?;
        return Ok(());
    }
    for (column, schema) in [
        (
            "thumbnail_mime_type",
            "ALTER TABLE attachments ADD COLUMN thumbnail_mime_type TEXT",
        ),
        (
            "thumbnail_byte_length",
            "ALTER TABLE attachments ADD COLUMN thumbnail_byte_length INTEGER",
        ),
        (
            "thumbnail_relative_path",
            "ALTER TABLE attachments ADD COLUMN thumbnail_relative_path TEXT",
        ),
    ] {
        let present: bool = connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM pragma_table_info('attachments') WHERE name = ?
                )",
                [column],
                |row| row.get(0),
            )
            .map_err(|error| {
                StoreError::sqlite("could not inspect the attachment columns", error)
            })?;
        if !present {
            connection.execute_batch(schema).map_err(|error| {
                StoreError::sqlite("could not add the attachment thumbnail columns", error)
            })?;
        }
    }
    Ok(())
}

fn add_evidence_artifact_schema(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(EVIDENCE_ARTIFACT_SCHEMA)
        .map_err(|error| {
            StoreError::sqlite("could not create the evidence artifact table", error)
        })?;
    let has_thumbnail_bytes: bool = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM pragma_table_info('evidence_artifacts')
                WHERE name = 'thumbnail_byte_size'
            )",
            [],
            |row| row.get(0),
        )
        .map_err(|error| {
            StoreError::sqlite("could not inspect the evidence artifact columns", error)
        })?;
    if !has_thumbnail_bytes {
        connection
            .execute_batch(
                "ALTER TABLE evidence_artifacts
                 ADD COLUMN thumbnail_byte_size INTEGER NOT NULL DEFAULT 0",
            )
            .map_err(|error| {
                StoreError::sqlite("could not add evidence thumbnail byte size", error)
            })?;
    }
    Ok(())
}

fn add_notion_task_projection_schema(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(NOTION_TASK_PROJECTION_SCHEMA)
        .map_err(|error| {
            StoreError::sqlite("could not create the Notion task projection table", error)
        })
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
        .map_err(|error| StoreError::sqlite("could not read the stored session metadata", error))?;
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

fn orchestration_event_from_row(row: &Row<'_>) -> rusqlite::Result<OrchestrationEventRow> {
    Ok(OrchestrationEventRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        kind: row.get(2)?,
        timestamp: row.get(3)?,
        sequence: row.get(4)?,
        workflow_id: row.get(5)?,
        idempotency_key: row.get(6)?,
        payload_json: row.get(7)?,
    })
}

fn evidence_artifact_from_row(row: &Row<'_>) -> rusqlite::Result<EvidenceArtifact> {
    Ok(EvidenceArtifact {
        id: row.get(0)?,
        orchestration_run_id: row.get(1)?,
        task_id: row.get(2)?,
        agent: row.get(3)?,
        provider: row.get(4)?,
        scenario: row.get(5)?,
        commit_hash: row.get(6)?,
        branch: row.get(7)?,
        worktree: row.get(8)?,
        captured_at_ms: row.get(9)?,
        kind: row.get(10)?,
        status: row.get(11)?,
        byte_size: row.get(12)?,
        original_ref: row.get(13)?,
        thumbnail_ref: row.get(14)?,
        thumbnail_byte_size: row.get(15)?,
        pinned: row.get(16)?,
        expires_at_ms: row.get(17)?,
    })
}

fn insert_orchestration_event_on(
    connection: &Connection,
    row: &OrchestrationEventRow,
) -> Result<()> {
    connection
        .execute(
            "INSERT INTO orchestration_events
             (id, run_id, kind, timestamp, sequence, workflow_id, idempotency_key, payload_json)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                row.id,
                row.run_id,
                row.kind,
                row.timestamp,
                row.sequence,
                row.workflow_id,
                row.idempotency_key,
                row.payload_json
            ],
        )
        .map_err(|error| StoreError::sqlite("could not append the orchestration event", error))?;
    Ok(())
}

fn insert_orchestration_event_or_ignore_on(
    connection: &Connection,
    row: &OrchestrationEventRow,
) -> Result<()> {
    connection
        .execute(
            "INSERT OR IGNORE INTO orchestration_events
             (id, run_id, kind, timestamp, sequence, workflow_id, idempotency_key, payload_json)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                row.id,
                row.run_id,
                row.kind,
                row.timestamp,
                row.sequence,
                row.workflow_id,
                row.idempotency_key,
                row.payload_json
            ],
        )
        .map_err(|error| StoreError::sqlite("could not import the orchestration event", error))?;
    Ok(())
}

fn attachment_from_row(row: &Row<'_>) -> rusqlite::Result<AttachmentRow> {
    Ok(AttachmentRow {
        id: row.get(0)?,
        owned_id: row.get(1)?,
        file_name: row.get(2)?,
        mime_type: row.get(3)?,
        byte_length: row.get(4)?,
        relative_path: row.get(5)?,
        thumbnail_mime_type: row.get(6)?,
        thumbnail_byte_length: row.get(7)?,
        thumbnail_relative_path: row.get(8)?,
        created_at_ms: row.get(9)?,
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

    use super::{
        EventRow, EvidenceArtifact, EvidenceArtifactQuery, EvidenceDiskUsage, EvidenceRunDiskUsage,
        NotionTaskProjection, SessionRow, SessionStore,
    };

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
        store
            .upsert_session(&runtime)
            .expect("save from the runtime");

        let stored = store
            .get_session("owned-import")
            .expect("read the session back")
            .expect("the session is there");
        let extra: serde_json::Value =
            serde_json::from_str(&stored.extra_json).expect("stored metadata is JSON");
        assert_eq!(
            extra
                .get("import")
                .and_then(|cursor| cursor.get("cutoffOffset")),
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
        moved.extra_json =
            r#"{"import":{"transcriptPath":"/tmp/a.jsonl","cutoffOffset":7,"reachedStart":true}}"#
                .to_owned();
        store.upsert_session(&moved).expect("move the cursor");

        let stored = store
            .get_session("owned-import")
            .expect("read the session back")
            .expect("the session is there");
        let extra: serde_json::Value =
            serde_json::from_str(&stored.extra_json).expect("stored metadata is JSON");
        assert_eq!(
            extra
                .get("import")
                .and_then(|cursor| cursor.get("cutoffOffset")),
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

    fn fixture_evidence_artifact(id: &str, captured_at_ms: i64) -> EvidenceArtifact {
        EvidenceArtifact {
            id: id.to_owned(),
            orchestration_run_id: "run-1".to_owned(),
            task_id: Some("TSK-808".to_owned()),
            agent: "Codex".to_owned(),
            provider: "openai".to_owned(),
            scenario: "phase-2-proof".to_owned(),
            commit_hash: "abcdef0".to_owned(),
            branch: "tsk-808-evidence-schema".to_owned(),
            worktree: "/worktrees/mac-command-bar/tsk-808-evidence-schema".to_owned(),
            captured_at_ms,
            kind: "screenshot".to_owned(),
            status: "ready".to_owned(),
            byte_size: 1234,
            original_ref: format!("managed/originals/{id}.png"),
            thumbnail_ref: Some(format!("managed/thumbs/{id}.png")),
            thumbnail_byte_size: 0,
            pinned: false,
            expires_at_ms: Some(captured_at_ms + 10_000),
        }
    }

    fn fixture_notion_task(
        source_task_id: &str,
        project: &str,
        status: &str,
        title: &str,
        due_date: Option<&str>,
    ) -> NotionTaskProjection {
        NotionTaskProjection {
            source_task_id: source_task_id.to_owned(),
            title: title.to_owned(),
            project: project.to_owned(),
            status: status.to_owned(),
            priority: Some("High".to_owned()),
            assignee: Some("Codex".to_owned()),
            due_date: due_date.map(str::to_owned),
            source_url: format!("https://notion.local/{source_task_id}"),
            fetched_at_ms: 123_456,
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
                .append_event(&tool_event(
                    "session-a",
                    seq,
                    "item.updated",
                    "tool-a",
                    status,
                ))
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
        assert_eq!(
            events.iter().map(|event| event.seq).collect::<Vec<_>>(),
            [3, 4]
        );
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
        assert_eq!(
            events
                .iter()
                .map(|event| event.kind.as_str())
                .collect::<Vec<_>>(),
            ["item.updated", "item.completed"]
        );
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
                        tool_event("session-a", seq, "item.updated", "tool-a", status).payload_json,
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
        assert_eq!(
            events.iter().map(|event| event.seq).collect::<Vec<_>>(),
            [3, 4]
        );
        drop(store);

        let connection = Connection::open(&path).expect("inspect upgraded database");
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read schema version");
        assert_eq!(version, super::SCHEMA_VERSION);
    }

    #[test]
    fn schema_v6_upgrade_adds_evidence_artifacts_and_keeps_events() {
        let directory = TempDir::new().expect("create temporary directory");
        let path = directory.path().join("sessions.db");
        let connection = Connection::open(&path).expect("create version six database");
        connection
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
                    item_id TEXT GENERATED ALWAYS AS (json_extract(payload, '$.payload.itemId')) VIRTUAL,
                    PRIMARY KEY (owned_id, seq)
                );
                CREATE INDEX events_item_idx ON events(owned_id, kind, item_id, seq);
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
                CREATE TABLE attachments (
                    id TEXT PRIMARY KEY,
                    owned_id TEXT NOT NULL REFERENCES sessions(owned_id) ON DELETE CASCADE,
                    file_name TEXT NOT NULL,
                    mime_type TEXT NOT NULL,
                    byte_length INTEGER NOT NULL,
                    relative_path TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );
                CREATE INDEX attachments_owned_id_idx ON attachments(owned_id, file_name);
                PRAGMA user_version = 6;",
            )
            .expect("create version six schema");
        let session = fixture_session("session-v6", 20_000);
        let event = fixture_event("session-v6", 1);
        connection
            .execute(
                "INSERT INTO sessions (
                    owned_id, native_session_id, provider, model, effort, cwd, worktree,
                    branch, title, project, state, suspended, created_at, last_activity_at,
                    extra, title_source
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    session.owned_id,
                    session.native_session_id,
                    session.provider,
                    session.model,
                    session.effort,
                    session.cwd,
                    session.worktree,
                    session.branch,
                    session.title,
                    session.project,
                    session.state,
                    session.suspended,
                    session.created_at_ms,
                    session.last_activity_at_ms,
                    session.extra_json,
                    session.title_source,
                ],
            )
            .expect("insert sentinel session");
        connection
            .execute(
                "INSERT INTO events (owned_id, seq, turn_id, kind, payload, created_at)
                 VALUES (?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    event.owned_id,
                    event.seq,
                    event.turn_id,
                    event.kind,
                    event.payload_json,
                    event.created_at_ms
                ],
            )
            .expect("insert sentinel event");
        drop(connection);

        let store = SessionStore::open(&path).expect("upgrade database");
        assert_eq!(
            store
                .list_events("session-v6", i64::MIN, 10)
                .expect("read sentinel event"),
            [event]
        );
        drop(store);

        let connection = Connection::open(&path).expect("inspect upgraded database");
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read schema version");
        assert_eq!(version, super::SCHEMA_VERSION);
        connection
            .prepare("SELECT original_ref, thumbnail_ref, pinned FROM evidence_artifacts")
            .expect("prepare evidence artifact query");
    }

    #[test]
    fn evidence_artifact_schema_is_idempotent_on_reopen() {
        let (_directory, path, store) = open_temp_store();
        drop(store);

        let reopened = SessionStore::open(&path).expect("reopen session store");
        drop(reopened);

        let connection = Connection::open(&path).expect("inspect reopened database");
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read schema version");
        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name = 'evidence_artifacts'",
                [],
                |row| row.get(0),
            )
            .expect("count evidence tables");
        assert_eq!(version, super::SCHEMA_VERSION);
        assert_eq!(table_count, 1);
    }

    #[test]
    fn schema_v12_upgrade_adds_thumbnail_bytes_without_losing_evidence() {
        let directory = TempDir::new().expect("create temporary directory");
        let path = directory.path().join("sessions.db");
        let connection = Connection::open(&path).expect("create version twelve database");
        connection
            .execute_batch(super::EVIDENCE_ARTIFACT_SCHEMA)
            .expect("create current evidence schema");
        connection
            .execute_batch(
                "ALTER TABLE evidence_artifacts DROP COLUMN thumbnail_byte_size;
                 INSERT INTO evidence_artifacts (
                    id, orchestration_run_id, agent, provider, scenario, commit_hash, branch,
                    worktree, captured_at_ms, kind, status, byte_size, original_ref,
                    thumbnail_ref, pinned
                 ) VALUES (
                    'sentinel', 'run-1', 'Codex', 'openai', 'restart', 'abc123', 'main',
                    '/repo', 1000, 'screenshot', 'ready', 100, 'evidence/originals/sentinel.png',
                    'evidence/thumbnails/sentinel.webp', 0
                 );
                 PRAGMA user_version = 12;",
            )
            .expect("create version twelve evidence row");
        drop(connection);

        let store = SessionStore::open(&path).expect("upgrade database");
        let artifacts = store
            .list_evidence_artifacts(&EvidenceArtifactQuery {
                limit: 1,
                ..Default::default()
            })
            .expect("read migrated evidence");
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].id, "sentinel");
        assert_eq!(artifacts[0].thumbnail_byte_size, 0);
        drop(store);

        let connection = Connection::open(&path).expect("inspect upgraded database");
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read schema version");
        assert_eq!(version, super::SCHEMA_VERSION);
    }

    #[test]
    fn schema_v11_upgrade_adds_notion_projection_and_keeps_evidence() {
        let directory = TempDir::new().expect("create temporary directory");
        let path = directory.path().join("sessions.db");
        let connection = Connection::open(&path).expect("create version eleven database");
        connection
            .execute_batch(super::EVIDENCE_ARTIFACT_SCHEMA)
            .expect("create evidence artifact schema");
        connection
            .execute_batch("ALTER TABLE evidence_artifacts DROP COLUMN thumbnail_byte_size")
            .expect("restore version eleven evidence columns");
        let artifact = fixture_evidence_artifact("sentinel", 55_000);
        connection
            .execute(
                "INSERT INTO evidence_artifacts (
                    id, orchestration_run_id, task_id, agent, provider, scenario,
                    commit_hash, branch, worktree, captured_at_ms, kind, status, byte_size,
                    original_ref, thumbnail_ref, pinned, expires_at_ms
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    artifact.id,
                    artifact.orchestration_run_id,
                    artifact.task_id,
                    artifact.agent,
                    artifact.provider,
                    artifact.scenario,
                    artifact.commit_hash,
                    artifact.branch,
                    artifact.worktree,
                    artifact.captured_at_ms,
                    artifact.kind,
                    artifact.status,
                    artifact.byte_size,
                    artifact.original_ref,
                    artifact.thumbnail_ref,
                    artifact.pinned,
                    artifact.expires_at_ms,
                ],
            )
            .expect("insert evidence sentinel");
        connection
            .pragma_update(None, "user_version", 11)
            .expect("set version eleven");
        drop(connection);

        let store = SessionStore::open(&path).expect("upgrade database");
        assert_eq!(
            store
                .list_evidence_artifacts(&EvidenceArtifactQuery {
                    limit: 10,
                    ..Default::default()
                })
                .expect("read evidence sentinel"),
            [artifact]
        );
        drop(store);

        let connection = Connection::open(&path).expect("inspect upgraded database");
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read schema version");
        assert_eq!(version, super::SCHEMA_VERSION);
        connection
            .prepare(
                "SELECT source_task_id, title, project, status, priority, assignee, due_date,
                        source_url, fetched_at_ms
                 FROM notion_task_projections",
            )
            .expect("prepare Notion task projection query");
    }

    #[test]
    fn notion_task_projection_snapshot_replaces_existing_rows() {
        let (_directory, _path, store) = open_temp_store();
        let first = [
            fixture_notion_task("task-a", "Assembly", "Todo", "Alpha", None),
            fixture_notion_task("task-b", "Assembly", "Todo", "Beta", Some("2026-08-25")),
        ];
        store
            .replace_notion_task_projections(&first)
            .expect("write first Notion task snapshot");

        let second = [fixture_notion_task(
            "task-c",
            "Rental Command",
            "Doing",
            "Gamma",
            Some("2026-08-24"),
        )];
        store
            .replace_notion_task_projections(&second)
            .expect("replace Notion task snapshot");

        assert_eq!(
            store
                .query_notion_task_projections(0, 10, "", "", "")
                .expect("read Notion task snapshot"),
            super::NotionTaskProjectionPage {
                tasks: second.to_vec(),
                projects: vec!["Rental Command".to_string()],
                statuses: vec!["Doing".to_string()],
                has_more: false,
            }
        );
    }

    #[test]
    fn notion_task_projection_is_available_after_store_reopen() {
        let (_directory, path, store) = open_temp_store();
        let tasks = [fixture_notion_task(
            "task-offline",
            "Assembly",
            "Doing",
            "Offline projection",
            Some("2026-08-27"),
        )];
        store
            .replace_notion_task_projections(&tasks)
            .expect("write Notion task snapshot");
        drop(store);

        let reopened = SessionStore::open(&path).expect("reopen session store offline");
        assert_eq!(
            reopened
                .query_notion_task_projections(0, 10, "", "", "")
                .expect("read offline Notion task snapshot")
                .tasks,
            tasks
        );
    }

    #[test]
    fn notion_task_projection_page_is_bounded_and_sql_ordered() {
        let (_directory, _path, store) = open_temp_store();
        let tasks = [
            fixture_notion_task("task-4", "Rental Command", "Todo", "Zeta", None),
            fixture_notion_task("task-2", "Assembly", "Doing", "Beta", Some("2026-08-24")),
            fixture_notion_task("task-1", "Assembly", "Doing", "Alpha", Some("2026-08-24")),
            fixture_notion_task("task-3", "Assembly", "Todo", "Gamma", None),
        ];
        store
            .replace_notion_task_projections(&tasks)
            .expect("write Notion task snapshot");

        let first_page = store
            .query_notion_task_projections(0, 2, "", "", "")
            .expect("read first Notion task page");
        assert_eq!(
            first_page
                .tasks
                .iter()
                .map(|task| task.source_task_id.as_str())
                .collect::<Vec<_>>(),
            ["task-1", "task-2"]
        );

        let second_page = store
            .query_notion_task_projections(2, 2, "", "", "")
            .expect("read second Notion task page");
        assert_eq!(
            second_page
                .tasks
                .iter()
                .map(|task| task.source_task_id.as_str())
                .collect::<Vec<_>>(),
            ["task-3", "task-4"]
        );
        assert!(!second_page.has_more);
    }

    #[test]
    fn notion_task_projection_filters_run_in_sql() {
        let (_directory, _path, store) = open_temp_store();
        let tasks = [
            fixture_notion_task("task-1", "Assembly", "Doing", "Workbench", None),
            fixture_notion_task("task-2", "Rental Command", "Todo", "Billing", None),
        ];
        store
            .replace_notion_task_projections(&tasks)
            .expect("write Notion task snapshot");

        let page = store
            .query_notion_task_projections(0, 10, "work", "Assembly", "Doing")
            .expect("filter Notion task snapshot");
        assert_eq!(page.tasks, tasks[..1]);
        assert_eq!(page.projects, ["Assembly", "Rental Command"]);
        assert_eq!(page.statuses, ["Doing", "Todo"]);
    }

    #[test]
    fn notion_task_projection_hides_future_tasks_unless_requested() {
        let (_directory, _path, store) = open_temp_store();
        let tasks = [
            fixture_notion_task("task-active", "Assembly", "To Do", "Active", None),
            fixture_notion_task("task-future", "Assembly", "Future", "Later", None),
        ];
        store
            .replace_notion_task_projections(&tasks)
            .expect("write Notion task snapshot");

        let default_page = store
            .query_notion_task_projections(0, 10, "", "", "")
            .expect("read default Notion task page");
        assert_eq!(default_page.tasks, tasks[..1]);

        let future_page = store
            .query_notion_task_projections(0, 10, "", "", "Future")
            .expect("read future Notion task page");
        assert_eq!(future_page.tasks, tasks[1..]);
    }

    #[test]
    fn evidence_artifact_page_is_bounded_and_newest_first() {
        let (_directory, _path, store) = open_temp_store();
        for (id, captured_at_ms) in [
            ("oldest", 1_000),
            ("middle", 2_000),
            ("newest", 3_000),
            ("newer", 4_000),
        ] {
            store
                .upsert_evidence_artifact(&fixture_evidence_artifact(id, captured_at_ms))
                .expect("upsert evidence artifact");
        }

        let first_page = store
            .list_evidence_artifacts(&EvidenceArtifactQuery {
                limit: 2,
                ..Default::default()
            })
            .expect("list newest evidence page");
        assert_eq!(
            first_page
                .iter()
                .map(|artifact| artifact.id.as_str())
                .collect::<Vec<_>>(),
            ["newer", "newest"]
        );

        let second_page = store
            .list_evidence_artifacts(&EvidenceArtifactQuery {
                before_captured_at_ms: first_page.last().map(|artifact| artifact.captured_at_ms),
                before_id: first_page.last().map(|artifact| artifact.id.clone()),
                limit: 2,
                ..Default::default()
            })
            .expect("list older evidence page");
        assert_eq!(
            second_page
                .iter()
                .map(|artifact| artifact.id.as_str())
                .collect::<Vec<_>>(),
            ["middle", "oldest"]
        );

        let mut updated = fixture_evidence_artifact("middle", 5_000);
        updated.status = "pinned".to_owned();
        updated.pinned = true;
        store
            .upsert_evidence_artifact(&updated)
            .expect("update evidence artifact");
        assert_eq!(
            store
                .list_evidence_artifacts(&EvidenceArtifactQuery {
                    limit: 1,
                    ..Default::default()
                })
                .expect("read updated evidence artifact"),
            [updated.clone()]
        );
        assert_eq!(
            store
                .evidence_artifact("middle")
                .expect("read evidence artifact by id"),
            Some(updated)
        );

        let deleted = store
            .delete_evidence_artifacts(&["middle".to_owned(), "missing".to_owned()])
            .expect("delete selected evidence artifacts");
        assert_eq!(
            deleted
                .iter()
                .map(|artifact| artifact.id.as_str())
                .collect::<Vec<_>>(),
            ["middle"]
        );
        assert!(!store
            .list_evidence_artifacts(&EvidenceArtifactQuery {
                limit: 10,
                ..Default::default()
            })
            .expect("list after evidence delete")
            .iter()
            .any(|artifact| artifact.id == "middle"));
    }

    #[test]
    fn evidence_artifact_query_filters_in_sql_and_pages_tied_timestamps() {
        let (_directory, _path, store) = open_temp_store();
        for id in ["a", "b", "c"] {
            store
                .upsert_evidence_artifact(&fixture_evidence_artifact(id, 2_000))
                .expect("upsert tied evidence artifact");
        }
        let first_page = store
            .list_evidence_artifacts(&EvidenceArtifactQuery {
                limit: 2,
                ..Default::default()
            })
            .expect("list first tied page");
        let second_page = store
            .list_evidence_artifacts(&EvidenceArtifactQuery {
                before_captured_at_ms: Some(2_000),
                before_id: Some("b".to_owned()),
                limit: 2,
                ..Default::default()
            })
            .expect("list second tied page");
        assert_eq!(
            first_page
                .iter()
                .map(|artifact| artifact.id.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!(
            second_page
                .iter()
                .map(|artifact| artifact.id.as_str())
                .collect::<Vec<_>>(),
            ["c"]
        );

        let mut target = fixture_evidence_artifact("target", 3_000);
        target.task_id = Some("TSK-900".to_owned());
        target.commit_hash = "fedcba9".to_owned();
        target.orchestration_run_id = "run-2".to_owned();
        target.agent = "Claude".to_owned();
        target.scenario = "filtered".to_owned();
        store
            .upsert_evidence_artifact(&target)
            .expect("upsert filtered evidence artifact");
        assert_eq!(
            store
                .list_evidence_artifacts(&EvidenceArtifactQuery {
                    limit: 10,
                    task_id: target.task_id.clone(),
                    commit_hash: Some(target.commit_hash.clone()),
                    orchestration_run_id: Some(target.orchestration_run_id.clone()),
                    agent: Some(target.agent.clone()),
                    scenario: Some(target.scenario.clone()),
                    captured_from_ms: Some(2_500),
                    captured_to_ms: Some(3_500),
                    ..Default::default()
                })
                .expect("filter evidence artifacts"),
            [target]
        );
    }

    #[test]
    fn evidence_artifact_and_run_pins_update_the_selected_rows() {
        let (_directory, _path, store) = open_temp_store();
        let first = fixture_evidence_artifact("first", 1_000);
        let second = fixture_evidence_artifact("second", 2_000);
        let mut other_run = fixture_evidence_artifact("other-run", 3_000);
        other_run.orchestration_run_id = "run-2".to_owned();
        for artifact in [&first, &second, &other_run] {
            store
                .upsert_evidence_artifact(artifact)
                .expect("upsert evidence artifact");
        }

        assert!(store
            .set_evidence_artifact_pinned("other-run", true)
            .expect("pin one artifact"));
        assert!(!store
            .set_evidence_artifact_pinned("missing", true)
            .expect("missing artifact is unchanged"));
        assert_eq!(
            store
                .set_evidence_run_pinned("run-1", true)
                .expect("pin one run"),
            2
        );

        let artifacts = store
            .list_evidence_artifacts(&EvidenceArtifactQuery {
                limit: 10,
                ..Default::default()
            })
            .expect("list pinned artifacts");
        assert!(artifacts.iter().all(|artifact| artifact.pinned));
        assert_eq!(
            store
                .set_evidence_run_pinned("run-1", false)
                .expect("unpin one run"),
            2
        );
        let artifacts = store
            .list_evidence_artifacts(&EvidenceArtifactQuery {
                limit: 10,
                ..Default::default()
            })
            .expect("list after unpinning run");
        assert!(artifacts
            .iter()
            .filter(|artifact| artifact.orchestration_run_id == "run-1")
            .all(|artifact| !artifact.pinned));
        assert!(
            artifacts
                .iter()
                .find(|artifact| artifact.id == "other-run")
                .expect("other run artifact")
                .pinned
        );
    }

    #[test]
    fn evidence_run_delete_returns_only_the_deleted_run() {
        let (_directory, _path, store) = open_temp_store();
        let first = fixture_evidence_artifact("first", 1_000);
        let second = fixture_evidence_artifact("second", 2_000);
        let mut other_run = fixture_evidence_artifact("other-run", 3_000);
        other_run.orchestration_run_id = "run-2".to_owned();
        for artifact in [&first, &second, &other_run] {
            store
                .upsert_evidence_artifact(artifact)
                .expect("upsert evidence artifact");
        }

        let deleted = store
            .delete_evidence_run("run-1")
            .expect("delete evidence run");
        assert_eq!(deleted.len(), 2);
        assert!(deleted
            .iter()
            .all(|artifact| artifact.orchestration_run_id == "run-1"));
        assert_eq!(
            store
                .list_evidence_artifacts(&EvidenceArtifactQuery {
                    limit: 10,
                    ..Default::default()
                })
                .expect("list after evidence run delete"),
            [other_run]
        );
    }

    #[test]
    fn evidence_disk_usage_is_aggregated_by_sql_for_originals_and_thumbnails() {
        let (_directory, _path, store) = open_temp_store();
        let mut first = fixture_evidence_artifact("first", 1_000);
        first.byte_size = 100;
        first.thumbnail_byte_size = 10;
        let mut second = fixture_evidence_artifact("second", 2_000);
        second.byte_size = 200;
        second.thumbnail_byte_size = 20;
        let mut other_run = fixture_evidence_artifact("other-run", 3_000);
        other_run.orchestration_run_id = "run-2".to_owned();
        other_run.byte_size = 50;
        other_run.thumbnail_byte_size = 5;
        for artifact in [&first, &second, &other_run] {
            store
                .upsert_evidence_artifact(artifact)
                .expect("upsert evidence artifact");
        }

        assert_eq!(
            store.evidence_disk_usage().expect("read disk usage"),
            EvidenceDiskUsage {
                total_bytes: 385,
                runs: vec![
                    EvidenceRunDiskUsage {
                        orchestration_run_id: "run-1".to_owned(),
                        byte_size: 330,
                    },
                    EvidenceRunDiskUsage {
                        orchestration_run_id: "run-2".to_owned(),
                        byte_size: 55,
                    },
                ],
            }
        );
    }

    #[test]
    fn expired_evidence_cleanup_deletes_only_unpinned_rows() {
        let (_directory, _path, store) = open_temp_store();
        let mut expired = fixture_evidence_artifact("expired", 1_000);
        expired.expires_at_ms = Some(2_000);
        let mut pinned = fixture_evidence_artifact("pinned", 1_000);
        pinned.expires_at_ms = Some(2_000);
        pinned.pinned = true;
        let mut default_expired = fixture_evidence_artifact("default-expired", 1_000);
        default_expired.expires_at_ms = None;
        let mut recent = fixture_evidence_artifact("recent", 9_000);
        recent.expires_at_ms = None;
        for artifact in [&expired, &pinned, &default_expired, &recent] {
            store
                .upsert_evidence_artifact(artifact)
                .expect("upsert evidence artifact");
        }

        let mut deleted = store
            .delete_expired_unpinned_evidence_artifacts(10_000, 5_000, 10)
            .expect("delete expired evidence");
        deleted.sort_by(|left, right| left.id.cmp(&right.id));
        assert_eq!(
            deleted
                .iter()
                .map(|artifact| artifact.id.as_str())
                .collect::<Vec<_>>(),
            ["default-expired", "expired"]
        );
        assert_eq!(
            store
                .list_evidence_artifacts(&EvidenceArtifactQuery {
                    limit: 10,
                    ..Default::default()
                })
                .expect("list retained evidence")
                .iter()
                .map(|artifact| artifact.id.as_str())
                .collect::<Vec<_>>(),
            ["recent", "pinned"]
        );
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
        assert_eq!(
            seqs.last().copied(),
            Some(399),
            "the page ends at the cursor"
        );
        assert!(
            seqs.windows(2).all(|pair| pair[1] == pair[0] + 1),
            "no gaps"
        );
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

    #[test]
    fn workspace_expanded_paths_roundtrip_in_sqlite() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::open(&dir.path().join("sessions.db")).expect("open store");

        let session = fixture_session("session-tree-1", 10_000);
        store.upsert_session(&session).expect("insert session");

        // Initially empty
        let initial = store
            .get_workspace_expanded_paths("session-tree-1", "/test/project")
            .expect("get initial");
        assert!(initial.is_empty());

        // Save expanded paths for root A
        let paths_a = vec!["/test/project/src".to_string(), "/test/project/lib".to_string()];
        store
            .set_workspace_expanded_paths("session-tree-1", "/test/project", &paths_a)
            .expect("set paths a");

        let read_a = store
            .get_workspace_expanded_paths("session-tree-1", "/test/project")
            .expect("read paths a");
        assert_eq!(read_a, paths_a);

        // Save expanded paths for root B without disturbing root A
        let paths_b = vec!["/other/repo/docs".to_string()];
        store
            .set_workspace_expanded_paths("session-tree-1", "/other/repo", &paths_b)
            .expect("set paths b");

        let read_b = store
            .get_workspace_expanded_paths("session-tree-1", "/other/repo")
            .expect("read paths b");
        assert_eq!(read_b, paths_b);

        let reread_a = store
            .get_workspace_expanded_paths("session-tree-1", "/test/project")
            .expect("reread paths a");
        assert_eq!(reread_a, paths_a);
    }
}
