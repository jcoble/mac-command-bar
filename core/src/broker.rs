use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, Params, Row, TransactionBehavior};

use crate::session_store::{Result, SessionStore, StoreError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    Message,
    Status,
    DecisionRequest,
    DecisionResponse,
    System,
}

impl MessageKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Message => "message",
            Self::Status => "status",
            Self::DecisionRequest => "decision_request",
            Self::DecisionResponse => "decision_response",
            Self::System => "system",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "message" => Some(Self::Message),
            "status" => Some(Self::Status),
            "decision_request" => Some(Self::DecisionRequest),
            "decision_response" => Some(Self::DecisionResponse),
            "system" => Some(Self::System),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Receipt {
    Queued,
    Delivered,
    Failed,
    Expired,
}

impl Receipt {
    fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Delivered => "delivered",
            Self::Failed => "failed",
            Self::Expired => "expired",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "delivered" => Some(Self::Delivered),
            "failed" => Some(Self::Failed),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope {
    pub id: String,
    pub group_id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub sender_name: String,
    pub run_id: Option<String>,
    pub kind: MessageKind,
    pub body: String,
    pub receipt: Receipt,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// One agent-to-agent message as the owner's traffic view shows it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficMessage {
    pub id: String,
    pub created_at_ms: i64,
    pub sender_name: String,
    pub from_owned_id: String,
    pub to_owned_id: String,
    /// The receiver's `handle · title`, or its owned id when it has none.
    pub receiver: String,
    pub body: String,
    pub receipt: String,
    pub run_id: Option<String>,
    pub kind: String,
}

/// Newest first, one page. ?1 run, ?2 session (sender or receiver), ?3 and ?4 the
/// (time, id) of the oldest message already shown, ?5 page size. NULL skips a filter.
const TRAFFIC: &str = "SELECT m.id, m.created_at_ms, m.sender_name, m.from_owned_id, m.to_owned_id,
        COALESCE(s.handle || ' · ' || s.title, s.handle, m.to_owned_id), m.body, m.receipt, m.run_id, m.kind
    FROM workflow_messages m LEFT JOIN sessions s ON s.owned_id = m.to_owned_id
    WHERE (?1 IS NULL OR m.run_id = ?1)
      AND (?2 IS NULL OR m.from_owned_id = ?2 OR m.to_owned_id = ?2)
      AND (?3 IS NULL OR (m.created_at_ms, m.id) < (?3, ?4))
    ORDER BY m.created_at_ms DESC, m.id DESC LIMIT ?5";

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryEntry {
    pub name: String,
    pub handle: String,
    pub owned_id: String,
    pub provider: String,
    pub project: Option<String>,
    pub state: String,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    /// The session the owner marked as the main orchestrator.
    pub orchestrator: bool,
}

/// The `app_settings` row naming the main orchestrator on this machine.
const MAIN_ORCHESTRATOR: &str = "orchestrator.main";

/// The sessions caller `?1` may see and message on this machine (scope `?2` = `local`), or, for a
/// request relayed from another machine, those a caller with that role may reach: `orchestrator`
/// gets the top-level sessions, `top` gets the main orchestrator.
const SCOPED_DIRECTORY: &str = "SELECT d.* FROM session_directory d
    LEFT JOIN (SELECT run_id, helper_of, run_lead, orchestrator FROM session_directory WHERE owned_id = ?1) me ON 1
    WHERE d.owned_id IS NOT ?1 AND CASE
      WHEN ?2 = 'orchestrator' THEN d.run_id IS NULL AND d.helper_of IS NULL
      WHEN ?2 = 'top' THEN d.orchestrator
      WHEN ?2 <> 'local' THEN 0
      -- A worker: its own run, and the session that started it (the main orchestrator when the owner did).
      WHEN me.run_id IS NOT NULL THEN d.run_id = me.run_id OR d.owned_id IS COALESCE(me.run_lead,
          (SELECT json_extract(value_json,'$') FROM app_settings WHERE setting_key = 'orchestrator.main'))
      -- A helper: its parent, and workers of runs it started.
      WHEN me.helper_of IS NOT NULL THEN d.owned_id = me.helper_of OR (d.run_id IS NOT NULL AND d.run_lead IS ?1)
      -- The main orchestrator: every top-level session, its helpers, and workers of runs it or the owner started.
      WHEN me.orchestrator THEN (d.run_id IS NULL AND d.helper_of IS NULL) OR d.helper_of IS ?1
          OR (d.run_id IS NOT NULL AND (d.run_lead IS ?1 OR d.run_lead IS NULL))
      -- Any other top-level session: the main orchestrator, its helpers, and workers of runs it started.
      ELSE d.orchestrator OR d.helper_of IS ?1 OR (d.run_id IS NOT NULL AND d.run_lead IS ?1) END";

fn directory_entry(row: &rusqlite::Row) -> rusqlite::Result<DirectoryEntry> {
    Ok(DirectoryEntry {
        name: row.get(0)?, owned_id: row.get(1)?, provider: row.get(2)?, project: row.get(3)?,
        state: row.get(4)?, run_id: row.get(5)?, step_id: row.get(6)?, handle: row.get(7)?,
        orchestrator: row.get(8)?,
    })
}

impl SessionStore {
    pub fn list_directory(&self) -> Result<Vec<DirectoryEntry>> {
        let connection = self.lock()?;
        let mut statement = db("could not prepare session directory", connection.prepare(
            "SELECT name,owned_id,provider,project,state,run_id,step_id,handle,orchestrator FROM session_directory ORDER BY name,owned_id"))?;
        let rows = db("could not query session directory", statement.query_map([], directory_entry))?;
        db("could not read session directory", rows.collect())
    }

    /// The sessions `caller` may message, or, with a relayed role as `scope`, those that role may reach.
    pub fn list_directory_for(&self, caller: &str, scope: &str) -> Result<Vec<DirectoryEntry>> {
        let connection = self.lock()?;
        let mut statement = db("could not prepare session directory", connection.prepare(&format!(
            "SELECT name,owned_id,provider,project,state,run_id,step_id,handle,orchestrator
             FROM ({SCOPED_DIRECTORY}) ORDER BY name,owned_id")))?;
        let rows = db("could not query session directory", statement.query_map(params![caller, scope], directory_entry))?;
        db("could not read session directory", rows.collect())
    }

    /// `resolve_session_name` among the sessions `list_directory_for` returns; any other is unknown.
    pub fn resolve_session_name_for(&self, name: &str, caller: &str, scope: &str) -> Result<String> {
        let connection = self.lock()?;
        let handle = name.split(" · ").next().unwrap_or(name);
        let (count, owned_id): (i64, Option<String>) = db("could not resolve session name", connection.query_row(
            &format!("SELECT COUNT(DISTINCT owned_id),MIN(owned_id) FROM ({SCOPED_DIRECTORY})
             WHERE name=?3 OR handle=?4 OR owned_id=?3"), params![caller, scope, name, handle],
            |row| Ok((row.get(0)?, row.get(1)?))))?;
        match (count, owned_id) {
            (1, Some(id)) => Ok(id),
            (0, _) => Err(StoreError::message("Unknown session name")),
            _ => Err(StoreError::message("Ambiguous session name")),
        }
    }

    /// `worker`, `helper`, `orchestrator` or `top`: what decides whom `owned_id` may reach.
    pub fn session_role(&self, owned_id: &str) -> Result<String> {
        use rusqlite::OptionalExtension;
        let connection = self.lock()?;
        let role: Option<String> = db("could not read the session role", connection.query_row(
            "SELECT CASE WHEN run_id IS NOT NULL THEN 'worker' WHEN helper_of IS NOT NULL THEN 'helper'
                    WHEN orchestrator THEN 'orchestrator' ELSE 'top' END
             FROM session_directory WHERE owned_id = ?", [owned_id], |row| row.get(0)).optional())?;
        Ok(role.unwrap_or_else(|| "top".into()))
    }

    /// Helpers of `parent` that are starting, working or waiting on the owner.
    pub fn unfinished_helpers_of(&self, parent: &str) -> Result<i64> {
        let connection = self.lock()?;
        db("could not count helpers", connection.query_row(
            "SELECT COUNT(*) FROM sessions WHERE json_extract(extra,'$.helperOf') = ?
               AND state IN ('starting','working','interrupting','waiting-approval','waiting-input')",
            [parent], |row| row.get(0)))
    }

    /// The session marked as the main orchestrator on this machine, and when it was marked (ms).
    pub fn main_orchestrator(&self) -> Result<Option<(String, i64)>> {
        use rusqlite::OptionalExtension;
        let connection = self.lock()?;
        db("could not read the main orchestrator", connection.query_row(
            "SELECT json_extract(value_json,'$'), updated_at FROM app_settings WHERE setting_key = ?",
            [MAIN_ORCHESTRATOR], |row| Ok((row.get(0)?, row.get(1)?))).optional())
    }

    /// Marks `owned_id` as the main orchestrator on this machine, or clears the mark.
    pub fn set_main_orchestrator(&self, owned_id: Option<&str>) -> Result<()> {
        match owned_id {
            Some(owned_id) => self.upsert_app_setting(MAIN_ORCHESTRATOR, &serde_json::Value::from(owned_id).to_string()),
            None => self.delete_app_setting(MAIN_ORCHESTRATOR),
        }
    }

    /// The handle and name other sessions use to reach `owned_id`.
    pub fn directory_name(&self, owned_id: &str) -> Result<Option<(String, String)>> {
        use rusqlite::OptionalExtension;
        let connection = self.lock()?;
        db("could not read the session name", connection.query_row(
            "SELECT handle, name FROM session_directory WHERE owned_id = ?", [owned_id],
            |row| Ok((row.get(0)?, row.get(1)?))).optional())
    }

    /// Finds a session by its title, its handle, its owned id, or a message sender name
    /// (`handle · title`), whose handle comes before the dot.
    pub fn resolve_session_name(&self, name: &str) -> Result<String> {
        let connection = self.lock()?;
        let handle = name.split(" · ").next().unwrap_or(name);
        let (count, owned_id): (i64, Option<String>) = db("could not resolve session name", connection.query_row(
            "SELECT COUNT(DISTINCT owned_id),MIN(owned_id) FROM session_directory
             WHERE name=?1 OR handle=?2 OR owned_id=?1", params![name, handle],
            |row| Ok((row.get(0)?, row.get(1)?))))?;
        match (count, owned_id) {
            (1, Some(id)) => Ok(id),
            (0, _) => Err(StoreError::message("Unknown session name")),
            _ => Err(StoreError::message("Ambiguous session name")),
        }
    }

    pub fn open_group_for(&self, owned_id: &str) -> Result<Option<(String, Option<String>)>> {
        use rusqlite::OptionalExtension;
        let connection = self.lock()?;
        db("could not find session run group", connection.query_row(
            "SELECT g.id,g.orchestrator_owned_id FROM workflow_groups g
             WHERE g.closed_at_ms IS NULL AND EXISTS(SELECT 1 FROM workflow_messages m
                 WHERE m.group_id=g.id AND m.kind='system' AND m.body='member:' || ?)
             ORDER BY g.created_at_ms,g.id LIMIT 1", [owned_id],
            |row| Ok((row.get(0)?, row.get(1)?))).optional())
    }

    pub fn decision_pending_for(&self, owned_id: &str) -> Result<bool> {
        let connection = self.lock()?;
        db("could not inspect pending decisions", connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM workflow_messages WHERE to_owned_id=?
                AND receipt='queued' AND kind='decision_request')", [owned_id], |row| row.get(0)))
    }

    pub fn create_group(
        &self,
        name: &str,
        orchestrator: Option<&str>,
        member_owned_ids: &[String],
    ) -> Result<String> {
        let mut connection = self.lock_write()?;
        let transaction = db(
            "could not begin the workflow group write",
            connection.transaction_with_behavior(TransactionBehavior::Immediate),
        )?;
        let group_id = new_id(&transaction)?;
        let created_at_ms = now_ms()?;
        db(
            "could not create the workflow group",
            transaction.execute(
                "INSERT INTO workflow_groups
                 (id, name, orchestrator_owned_id, created_at_ms, closed_at_ms)
                 VALUES (?, ?, ?, ?, NULL)",
                params![group_id, name, orchestrator, created_at_ms],
            ),
        )?;
        for member in member_owned_ids {
            db(
                "could not add the workflow member",
                transaction.execute(
                    "INSERT INTO workflow_messages
                     (id, group_id, from_owned_id, to_owned_id, sender_name, kind, body, receipt,
                      created_at_ms, updated_at_ms)
                     VALUES (?, ?, ?, ?, ?, 'system', ?, 'delivered', ?, ?)",
                    params![
                        new_id(&transaction)?,
                        group_id,
                        orchestrator.unwrap_or("system"),
                        member,
                        orchestrator.unwrap_or("system"),
                        format!("member:{member}"),
                        created_at_ms,
                        created_at_ms,
                    ],
                ),
            )?;
        }
        db(
            "could not finish the workflow group write",
            transaction.commit(),
        )?;
        Ok(group_id)
    }

    pub fn append_message(
        &self,
        group_id: &str,
        from: &str,
        to: &str,
        kind: MessageKind,
        body: &str,
    ) -> Result<Envelope> {
        self.append_message_as(group_id, from, to, kind, body, None)
    }

    /// `append_message` with the sender's label given, for a message from a
    /// session on another machine, which this machine's directory does not know.
    pub fn append_message_as(
        &self,
        group_id: &str,
        from: &str,
        to: &str,
        kind: MessageKind,
        body: &str,
        sender_name: Option<&str>,
    ) -> Result<Envelope> {
        let connection = self.lock_write()?;
        let id = new_id(&connection)?;
        let created_at_ms = now_ms()?;
        let (sender_name, run_id): (String, Option<String>) = db("could not name message sender", connection.query_row(
            "SELECT COALESCE(?2, d.handle || ' · ' || d.name, ?1), d.run_id FROM (SELECT 1) LEFT JOIN session_directory d ON d.owned_id = ?1",
            params![from, sender_name], |row| Ok((row.get(0)?, row.get(1)?))))?;
        db(
            "could not append the workflow message",
            connection.execute(
                "INSERT INTO workflow_messages
                 (id, group_id, from_owned_id, to_owned_id, sender_name, run_id, kind, body, receipt,
                  created_at_ms, updated_at_ms)
                 VALUES (?, NULLIF(?,''), ?, ?, ?, ?, ?, ?, 'queued', ?, ?)",
                params![
                    id,
                    group_id,
                    from,
                    to,
                    sender_name,
                    run_id,
                    kind.as_str(),
                    body,
                    created_at_ms,
                    created_at_ms
                ],
            ),
        )?;
        Ok(Envelope {
            id,
            group_id: group_id.to_owned(),
            from_agent: from.to_owned(),
            to_agent: to.to_owned(),
            sender_name,
            run_id,
            kind,
            body: body.to_owned(),
            receipt: Receipt::Queued,
            created_at_ms,
            updated_at_ms: created_at_ms,
        })
    }

    pub fn set_receipt(&self, message_id: &str, receipt: Receipt) -> Result<()> {
        let connection = self.lock_write()?;
        let changed = db(
            "could not update the workflow receipt",
            connection.execute(
                "UPDATE workflow_messages SET receipt = ?, updated_at_ms = ?
                 WHERE id = ? AND receipt = 'queued'",
                params![receipt.as_str(), now_ms()?, message_id],
            ),
        )?;
        if changed == 1 {
            return Ok(());
        }
        let exists = db(
            "could not inspect the workflow receipt",
            connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM workflow_messages WHERE id = ?)",
                [message_id],
                |row| row.get::<_, bool>(0),
            ),
        )?;
        Err(StoreError::message(if exists {
            "the workflow receipt is terminal"
        } else {
            "the workflow message does not exist"
        }))
    }

    /// Records a workflow reviewer's `report_review` call against its attempt.
    /// Kept as a delivered row addressed to the attempt, so it is never sent.
    pub fn add_review_report(&self, reviewer: &str, run_id: &str, attempt_id: &str, report: &str) -> Result<()> {
        let connection = self.lock_write()?;
        let now = now_ms()?;
        db("could not record the review report", connection.execute(
            "INSERT INTO workflow_messages
             (id, group_id, from_owned_id, to_owned_id, sender_name, run_id, kind, body, receipt,
              created_at_ms, updated_at_ms)
             VALUES (?, NULL, ?, ?, ?, ?, 'review', ?, 'delivered', ?, ?)",
            params![new_id(&connection)?, reviewer, attempt_id, reviewer, run_id, report, now, now],
        ))?;
        Ok(())
    }

    /// Every report recorded for a review attempt, oldest first.
    pub fn review_reports(&self, attempt_id: &str) -> Result<Vec<String>> {
        let connection = self.lock()?;
        let mut statement = db("could not prepare review reports", connection.prepare(
            "SELECT body FROM workflow_messages WHERE to_owned_id = ? AND kind = 'review'
             ORDER BY created_at_ms, rowid"))?;
        let rows = db("could not read review reports", statement.query_map([attempt_id], |row| row.get(0)))?;
        db("could not read review report", rows.collect())
    }

    /// The receipt of a message the caller sent. Another sender's message reads
    /// as unknown.
    pub fn message_status(&self, id: &str, from: &str) -> Result<String> {
        use rusqlite::OptionalExtension;
        let connection = self.lock()?;
        db("could not read the message receipt", connection.query_row(
            "SELECT receipt FROM workflow_messages WHERE id = ? AND from_owned_id = ?",
            [id, from], |row| row.get(0)).optional())?
            .ok_or_else(|| StoreError::message("Unknown message"))
    }

    pub fn pending_for(&self, to: &str) -> Result<Option<Envelope>> {
        use rusqlite::OptionalExtension;
        let connection = self.lock()?;
        db("could not read target queue", connection.query_row(
            "SELECT id, COALESCE(group_id,''), from_owned_id, to_owned_id, kind, body, receipt,
                    created_at_ms, updated_at_ms, sender_name, run_id FROM workflow_messages
             WHERE to_owned_id = ? AND receipt = 'queued' ORDER BY created_at_ms,id LIMIT 1",
            [to], envelope_from_row).optional())
    }

    pub fn events_for(&self, group_id: &str, since_ms: Option<i64>) -> Result<Vec<Envelope>> {
        let connection = self.lock()?;
        query_envelopes(
            &connection,
            "SELECT id, COALESCE(group_id,''), from_owned_id, to_owned_id, kind, body, receipt,
                    created_at_ms, updated_at_ms, sender_name, run_id FROM workflow_messages
             WHERE group_id = ? AND (? IS NULL OR created_at_ms > ?)
             ORDER BY created_at_ms ASC, rowid ASC",
            params![group_id, since_ms, since_ms],
        )
    }

    /// A page of agent-to-agent messages stored on this machine, newest first. Each message is
    /// stored on its receiver's machine, so pages from every machine together list each once.
    pub fn list_messages(&self, run_id: Option<&str>, session: Option<&str>,
                         before: Option<(i64, &str)>, limit: i64) -> Result<Vec<TrafficMessage>> {
        let connection = self.lock()?;
        let mut statement = db("could not prepare the message list", connection.prepare(TRAFFIC))?;
        let rows = db("could not list messages", statement.query_map(
            params![run_id, session, before.map(|(at, _)| at), before.map(|(_, id)| id), limit],
            |row| Ok(TrafficMessage {
                id: row.get(0)?, created_at_ms: row.get(1)?, sender_name: row.get(2)?,
                from_owned_id: row.get(3)?, to_owned_id: row.get(4)?, receiver: row.get(5)?,
                body: row.get(6)?, receipt: row.get(7)?, run_id: row.get(8)?, kind: row.get(9)?,
            })))?;
        db("could not read messages", rows.collect())
    }

    pub fn list_open_groups(&self) -> Result<Vec<(String, String, Option<String>, Vec<String>)>> {
        let connection = self.lock()?;
        let mut statement = db(
            "could not prepare open workflow groups",
            connection.prepare(
                "SELECT groups.id, groups.name, groups.orchestrator_owned_id,
                        COALESCE((SELECT json_group_array(member) FROM (
                            SELECT substr(body, 8) AS member FROM workflow_messages
                            WHERE group_id = groups.id AND kind = 'system'
                              AND body LIKE 'member:%'
                            ORDER BY created_at_ms ASC, rowid ASC)), '[]')
                 FROM workflow_groups AS groups WHERE groups.closed_at_ms IS NULL
                 ORDER BY groups.created_at_ms ASC, groups.rowid ASC",
            ),
        )?;
        let rows = db(
            "could not list open workflow groups",
            statement.query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get::<_, String>(3)?,
                ))
            }),
        )?;
        let raw = db(
            "could not read open workflow groups",
            rows.collect::<rusqlite::Result<Vec<(String, String, Option<String>, String)>>>(),
        )?;
        raw.into_iter()
            .map(|(id, name, orchestrator, members)| {
                let members = serde_json::from_str(&members)
                    .map_err(|_| StoreError::message("could not read workflow group members"))?;
                Ok((id, name, orchestrator, members))
            })
            .collect()
    }
}

fn db<T>(context: &'static str, result: rusqlite::Result<T>) -> Result<T> {
    result.map_err(|error| StoreError::sqlite(context, error))
}

fn new_id(connection: &Connection) -> Result<String> {
    db(
        "could not create a workflow id",
        connection.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0)),
    )
}

fn now_ms() -> Result<i64> {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StoreError::message("the system clock is before the Unix epoch"))?
        .as_millis();
    i64::try_from(milliseconds).map_err(|_| StoreError::message("the system time cannot be stored"))
}

fn query_envelopes<P: Params>(
    connection: &Connection,
    sql: &str,
    params: P,
) -> Result<Vec<Envelope>> {
    let mut statement = db(
        "could not prepare workflow messages",
        connection.prepare(sql),
    )?;
    let rows = db(
        "could not list workflow messages",
        statement.query_map(params, envelope_from_row),
    )?;
    db("could not read workflow messages", rows.collect())
}

fn envelope_from_row(row: &Row<'_>) -> rusqlite::Result<Envelope> {
    let kind = row.get::<_, String>(4)?;
    let receipt = row.get::<_, String>(6)?;
    Ok(Envelope {
        id: row.get(0)?,
        group_id: row.get(1)?,
        from_agent: row.get(2)?,
        to_agent: row.get(3)?,
        sender_name: row.get(9)?,
        run_id: row.get(10)?,
        kind: MessageKind::parse(&kind).ok_or(rusqlite::Error::InvalidQuery)?,
        body: row.get(5)?,
        receipt: Receipt::parse(&receipt).ok_or(rusqlite::Error::InvalidQuery)?,
        created_at_ms: row.get(7)?,
        updated_at_ms: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{MessageKind, Receipt};
    use crate::session_store::SessionStore;

    fn empty_group(store: &SessionStore) -> String {
        store
            .create_group("build", None, &[])
            .expect("create group")
    }

    fn append(store: &SessionStore, group: &str, to: &str, body: &str) -> super::Envelope {
        store
            .append_message(group, "orchestrator", to, MessageKind::Message, body)
            .expect("append message")
    }

    #[test]
    fn broker_delivers_to_session_outside_any_group() {
        let store = SessionStore::open_in_memory().unwrap();
        store.lock_write().unwrap().execute("INSERT INTO sessions
            (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra)
            VALUES ('target','codex','/work','ready',0,1,1,'{}')", []).unwrap();
        let message = store.append_message("", "sender", "target", MessageKind::Message, "hello").unwrap();
        assert_eq!(store.pending_for("target").unwrap(), Some(message.clone()));
        store.set_receipt(&message.id, Receipt::Delivered).unwrap();
        assert!(store.pending_for("target").unwrap().is_none());
        store.lock_write().unwrap().execute_batch(r#"INSERT INTO sessions
            (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra)
            VALUES ('worker','claude','/work','waiting-input',0,1,1,'{}');
            INSERT INTO orchestration_events (id,run_id,kind,timestamp,sequence,payload_json)
            VALUES ('e','run-1','workflow.node.started','1',1,
            '{"ownedId":"worker","nodeId":"build","workflowPayload":{"run":{"inputSnapshot":{"task":{"key":"tsk-1416"}}}}}');"#).unwrap();
        assert_eq!(store.resolve_session_name("tsk-1416/build").unwrap(), "worker");
        let message = store.append_message("", "worker", "target", MessageKind::Message, "reply to me").unwrap();
        assert!(message.sender_name.ends_with(" · tsk-1416/build"), "{}", message.sender_name);
        assert_eq!(message.run_id.as_deref(), Some("run-1"));
    }

    #[test]
    fn session_list_says_where_each_session_came_from() {
        let store = SessionStore::open_in_memory().unwrap();
        store.lock_write().unwrap().execute_batch(r#"INSERT INTO sessions
            (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra) VALUES
            ('parent','codex','/work','ready',0,1,1,'{}'),
            ('helper','claude','/work','ready',0,1,1,'{"helperOf":"parent"}'),
            ('worker','codex','/work','ready',0,1,1,'{}');
            INSERT INTO orchestration_events (id,run_id,kind,timestamp,sequence,payload_json) VALUES
            ('e','run-1','workflow.node.started','1',1,
             '{"ownedId":"worker","nodeId":"build","workflowPayload":{"run":{"definition":{"name":"Fix and review"},"inputSnapshot":{"task":{"key":"TSK-9"}}}}}');"#).unwrap();
        let listed = store.list_sessions_with_handles().unwrap();
        let lineage = |id: &str| listed.iter().find(|entry| entry.row.owned_id == id)
            .map(|entry| (entry.helper_of.clone(), entry.run_id.clone(), entry.run_label.clone())).unwrap();
        assert_eq!(lineage("parent"), (None, None, None));
        assert_eq!(lineage("helper"), (Some("parent".into()), None, None));
        assert_eq!(lineage("worker"), (None, Some("run-1".into()), Some("TSK-9 · Fix and review".into())));
    }

    #[test]
    fn every_session_keeps_a_handle_that_outlives_renames() {
        let store = SessionStore::open_in_memory().unwrap();
        store.lock_write().unwrap().execute_batch("INSERT INTO sessions
            (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra,title) VALUES
            ('a','codex','/work','ready',0,1,1,'{}','Lane A'), ('b','claude','/work','ready',0,2,2,'{}','Lane B');").unwrap();
        let entries = store.list_directory().unwrap();
        let handle = entries.iter().find(|entry| entry.owned_id == "a").unwrap().handle.clone();
        assert!(handle.split('-').count() >= 2 && handle != entries[1].handle, "{handle}");
        store.lock_write().unwrap().execute("UPDATE sessions SET title='Renamed' WHERE owned_id='a'", []).unwrap();
        assert_eq!(store.resolve_session_name(&handle).unwrap(), "a");
        assert_eq!(store.resolve_session_name("a").unwrap(), "a");
        let message = store.append_message("", "a", "b", MessageKind::Message, "hi").unwrap();
        assert_eq!(message.sender_name, format!("{handle} · Renamed"));
        // A reply addressed to the whole label reaches the sender.
        assert_eq!(store.resolve_session_name(&message.sender_name).unwrap(), "a");
    }

    #[test]
    fn a_message_from_another_machine_keeps_its_sender_label() {
        let store = SessionStore::open_in_memory().unwrap();
        store.lock_write().unwrap().execute_batch("INSERT INTO sessions
            (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra,title) VALUES
            ('here','codex','/work','ready',0,1,1,'{}','Here');").unwrap();
        let message = store.append_message_as("", "elsewhere", "here", MessageKind::Message, "hi",
            Some("brisk-heron@workbox-test · There")).unwrap();
        assert_eq!(message.sender_name, "brisk-heron@workbox-test · There");
        assert_eq!(store.message_status(&message.id, "elsewhere").unwrap(), "queued");
    }

    #[test]
    fn directory_shows_the_session_project_title() {
        let store = SessionStore::open_in_memory().unwrap();
        store.lock_write().unwrap().execute_batch("INSERT INTO projects
            (id,machine,root_path,title,repo_key,created_at) VALUES ('p1','local','/work','demo-repo','k',1);
            INSERT INTO sessions (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra,project_id)
            VALUES ('s1','codex','/work','ready',0,1,1,'{}','p1');").unwrap();
        assert_eq!(store.list_directory().unwrap()[0].project.as_deref(), Some("demo-repo"));
    }

    #[test]
    fn retried_worker_name_points_at_the_newest_session() {
        let store = SessionStore::open_in_memory().unwrap();
        store.lock_write().unwrap().execute_batch(r#"INSERT INTO sessions
            (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra) VALUES
            ('first-try','codex','/work','closed',0,1,1,'{}'), ('second-try','codex','/work','working',0,2,2,'{}');
            INSERT INTO orchestration_events (id,run_id,kind,timestamp,sequence,payload_json) VALUES
            ('e1','run-1','workflow.node.started','1',1,'{"ownedId":"first-try","nodeId":"build","workflowPayload":{"run":{"inputSnapshot":{"task":{"key":"tsk-1416"}}}}}'),
            ('e2','run-1','workflow.node.started','2',2,'{"ownedId":"second-try","nodeId":"build","workflowPayload":{"run":{"inputSnapshot":{"task":{"key":"tsk-1416"}}}}}');"#).unwrap();
        assert_eq!(store.resolve_session_name("tsk-1416/build").unwrap(), "second-try");
        assert_eq!(store.resolve_session_name("tsk-1416/build [first-try]").unwrap(), "first-try");
        let reply = store.append_message("", "second-try", "first-try", MessageKind::Message, "hi").unwrap();
        assert!(reply.sender_name.ends_with(" · tsk-1416/build"), "{}", reply.sender_name);
        assert_eq!(store.resolve_session_name(&reply.sender_name).unwrap(), "second-try");
    }

    #[test]
    fn orchestrator_comes_from_setting_not_title() {
        let store = SessionStore::open_in_memory().unwrap();
        store.lock_write().unwrap().execute_batch(r#"INSERT INTO sessions
            (owned_id,provider,cwd,title,state,suspended,created_at,last_activity_at,extra) VALUES
            ('named','codex','/work','orchestrator','ready',0,1,1,'{}'),
            ('marked','codex','/work','Controller','ready',0,2,2,'{}');"#).unwrap();
        let marked = |store: &SessionStore| store.list_directory().unwrap().into_iter()
            .filter(|entry| entry.orchestrator).map(|entry| entry.owned_id).collect::<Vec<_>>();
        assert!(marked(&store).is_empty(), "a title does not make an orchestrator");
        store.set_main_orchestrator(Some("marked")).unwrap();
        assert_eq!(marked(&store), ["marked"]);
        assert_eq!(store.main_orchestrator().unwrap().map(|(id, _)| id).as_deref(), Some("marked"));
        store.set_main_orchestrator(None).unwrap();
        assert!(marked(&store).is_empty());
        assert_eq!(store.main_orchestrator().unwrap(), None);
    }

    #[test]
    fn directory_lookup_uses_owned_event_index() {
        let store = SessionStore::open_in_memory().unwrap();
        let connection = store.lock().unwrap();
        let mut statement = connection.prepare("EXPLAIN QUERY PLAN SELECT name FROM session_directory WHERE owned_id=?").unwrap();
        let details = statement.query_map(["target"], |row| row.get::<_, String>(3)).unwrap()
            .collect::<rusqlite::Result<Vec<_>>>().unwrap().join("\n");
        assert!(details.contains("orchestration_events_owned_idx"), "{details}");
    }

    #[test]
    fn list_messages_filters_and_pages_in_sql() {
        let store = SessionStore::open_in_memory().unwrap();
        store.lock_write().unwrap().execute_batch("INSERT INTO sessions
            (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra,title,handle) VALUES
            ('b','codex','/work','ready',0,1,1,'{}','Lane B','calm-otter');
            INSERT INTO workflow_messages (id,from_owned_id,to_owned_id,sender_name,run_id,kind,body,receipt,created_at_ms,updated_at_ms) VALUES
            ('m1','a','b','brisk-heron · A','run-1','message','one','delivered',10,10),
            ('m2','a','b','brisk-heron · A',NULL,'message','two','queued',20,20),
            ('m3','c','gone','x · C','run-2','message','three','failed',20,20),
            ('m4','b','a','calm-otter · Lane B','run-1','message','four','delivered',30,30);").unwrap();
        use super::{TrafficMessage, TRAFFIC};
        use rusqlite::params;
        let ids = |page: Vec<TrafficMessage>| page.into_iter().map(|message| message.id).collect::<Vec<_>>();
        assert_eq!(ids(store.list_messages(None, None, None, 10).unwrap()), ["m4", "m3", "m2", "m1"]);
        assert_eq!(ids(store.list_messages(Some("run-1"), None, None, 10).unwrap()), ["m4", "m1"]);
        assert_eq!(ids(store.list_messages(None, Some("b"), None, 10).unwrap()), ["m4", "m2", "m1"]);
        // Paging resumes after the last row shown, even between two rows of the same time.
        let first = store.list_messages(None, None, None, 2).unwrap();
        assert_eq!(ids(first.clone()), ["m4", "m3"]);
        let last = first.last().unwrap();
        assert_eq!(ids(store.list_messages(None, None, Some((last.created_at_ms, &last.id)), 10).unwrap()), ["m2", "m1"]);
        let messages = store.list_messages(None, None, None, 10).unwrap();
        assert_eq!(messages[2].receiver, "calm-otter · Lane B");
        assert_eq!(messages[1].receiver, "gone");
        let connection = store.lock().unwrap();
        let mut statement = connection.prepare(&format!("EXPLAIN QUERY PLAN {TRAFFIC}")).unwrap();
        let details = statement.query_map(params![None::<String>, None::<String>, None::<i64>, None::<String>, 10], |row| row.get::<_, String>(3)).unwrap()
            .collect::<rusqlite::Result<Vec<_>>>().unwrap().join("\n");
        assert!(details.contains("idx_workflow_messages_created"), "{details}");
        assert!(!details.contains("TEMP B-TREE"), "{details}");
    }

    #[test]
    fn broker_queue_queries_use_target_index() {
        let store = SessionStore::open_in_memory().unwrap();
        let connection = store.lock().unwrap();
        let mut statement = connection.prepare("EXPLAIN QUERY PLAN SELECT id FROM workflow_messages
            WHERE to_owned_id=? AND receipt='queued' ORDER BY created_at_ms,id LIMIT 1").unwrap();
        let details = statement.query_map(["target"], |row| row.get::<_, String>(3)).unwrap()
            .collect::<rusqlite::Result<Vec<_>>>().unwrap().join("\n");
        assert!(details.contains("idx_workflow_messages_target"), "{details}");
        assert!(!details.contains("TEMP B-TREE"), "{details}");
    }

    #[test]
    fn append_starts_queued_and_events_return_in_order() {
        let store = SessionStore::open_in_memory().expect("open store");
        let members = vec!["worker-a".to_owned(), "worker-b".to_owned()];
        let group = store
            .create_group("build", Some("orchestrator"), &members)
            .expect("create group");
        let first = append(&store, &group, "worker-a", "first");
        let second = store
            .append_message(
                &group,
                "worker-a",
                "orchestrator",
                MessageKind::Status,
                "second",
            )
            .expect("append second message");

        assert_eq!(
            (first.receipt, second.receipt),
            (Receipt::Queued, Receipt::Queued)
        );
        let bodies: Vec<_> = store
            .events_for(&group, None)
            .expect("list events")
            .into_iter()
            .map(|event| event.body)
            .collect();
        assert_eq!(
            bodies,
            vec!["member:worker-a", "member:worker-b", "first", "second"]
        );
        assert_eq!(store.list_open_groups().expect("list groups")[0].3, members);
    }

    #[test]
    fn terminal_receipts_refuse_change() {
        let store = SessionStore::open_in_memory().expect("open store");
        let message = append(&store, &empty_group(&store), "worker-a", "choose");
        store
            .set_receipt(&message.id, Receipt::Delivered)
            .expect("deliver message");
        assert!(store.set_receipt(&message.id, Receipt::Failed).is_err());
    }

    #[test]
    fn pending_returns_only_queued_for_recipient() {
        let store = SessionStore::open_in_memory().expect("open store");
        let group = empty_group(&store);
        let first = append(&store, &group, "worker-a", "first");
        let delivered = append(&store, &group, "worker-a", "delivered");
        append(&store, &group, "worker-b", "other recipient");
        store
            .set_receipt(&delivered.id, Receipt::Expired)
            .expect("expire message");
        assert_eq!(
            store.pending_for("worker-a").expect("list pending"),
            Some(first)
        );
    }
}
