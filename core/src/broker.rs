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
    pub kind: MessageKind,
    pub body: String,
    pub receipt: Receipt,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl SessionStore {
    pub fn create_group(
        &self,
        name: &str,
        orchestrator: Option<&str>,
        member_owned_ids: &[String],
    ) -> Result<String> {
        let mut connection = self.lock()?;
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
                     (id, group_id, from_agent, to_agent, kind, body, receipt,
                      created_at_ms, updated_at_ms)
                     VALUES (?, ?, ?, ?, 'system', ?, 'delivered', ?, ?)",
                    params![
                        new_id(&transaction)?,
                        group_id,
                        orchestrator.unwrap_or("system"),
                        member,
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
        let connection = self.lock()?;
        let id = new_id(&connection)?;
        let created_at_ms = now_ms()?;
        db(
            "could not append the workflow message",
            connection.execute(
                "INSERT INTO workflow_messages
                 (id, group_id, from_agent, to_agent, kind, body, receipt,
                  created_at_ms, updated_at_ms)
                 VALUES (?, ?, ?, ?, ?, ?, 'queued', ?, ?)",
                params![
                    id,
                    group_id,
                    from,
                    to,
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
            kind,
            body: body.to_owned(),
            receipt: Receipt::Queued,
            created_at_ms,
            updated_at_ms: created_at_ms,
        })
    }

    pub fn set_receipt(&self, message_id: &str, receipt: Receipt) -> Result<()> {
        let connection = self.lock()?;
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

    pub fn pending_for(&self, group_id: &str, to: &str) -> Result<Vec<Envelope>> {
        let connection = self.lock()?;
        query_envelopes(
            &connection,
            "SELECT id, group_id, from_agent, to_agent, kind, body, receipt,
                    created_at_ms, updated_at_ms FROM workflow_messages
             WHERE group_id = ? AND to_agent = ? AND receipt = 'queued'
             ORDER BY created_at_ms ASC, rowid ASC",
            params![group_id, to],
        )
    }

    pub fn events_for(&self, group_id: &str, since_ms: Option<i64>) -> Result<Vec<Envelope>> {
        let connection = self.lock()?;
        query_envelopes(
            &connection,
            "SELECT id, group_id, from_agent, to_agent, kind, body, receipt,
                    created_at_ms, updated_at_ms FROM workflow_messages
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
            store.pending_for(&group, "worker-a").expect("list pending"),
            vec![first]
        );
    }
}
