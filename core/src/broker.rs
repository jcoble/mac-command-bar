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

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryEntry {
    pub name: String,
    pub owned_id: String,
    pub provider: String,
    pub project: Option<String>,
    pub state: String,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
}

impl SessionStore {
    pub fn list_directory(&self) -> Result<Vec<DirectoryEntry>> {
        let connection = self.lock()?;
        let mut statement = db("could not prepare session directory", connection.prepare(
            "SELECT name,owned_id,provider,project,state,run_id,step_id FROM session_directory ORDER BY name,owned_id"))?;
        let rows = db("could not query session directory", statement.query_map([], |row| Ok(DirectoryEntry {
            name: row.get(0)?, owned_id: row.get(1)?, provider: row.get(2)?, project: row.get(3)?,
            state: row.get(4)?, run_id: row.get(5)?, step_id: row.get(6)?,
        })))?;
        db("could not read session directory", rows.collect())
    }

    pub fn resolve_session_name(&self, name: &str) -> Result<String> {
        let connection = self.lock()?;
        let (count, owned_id): (i64, Option<String>) = db("could not resolve session name", connection.query_row(
            "SELECT COUNT(*),MIN(owned_id) FROM session_directory WHERE name=?", [name],
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
        let connection = self.lock_write()?;
        let id = new_id(&connection)?;
        let created_at_ms = now_ms()?;
        let (sender_name, run_id): (String, Option<String>) = db("could not name message sender", connection.query_row(
            "SELECT COALESCE(d.name, ?1), d.run_id FROM (SELECT 1) LEFT JOIN session_directory d ON d.owned_id = ?1",
            params![from], |row| Ok((row.get(0)?, row.get(1)?))))?;
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
        assert_eq!(message.sender_name, "tsk-1416/build");
        assert_eq!(message.run_id.as_deref(), Some("run-1"));
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
        assert_eq!(reply.sender_name, "tsk-1416/build");
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
