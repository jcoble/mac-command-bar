use super::*;

impl SessionStore {
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
        let mut guard = self.lock_write()?;
        let connection = db("could not begin the message write",
            guard.transaction_with_behavior(TransactionBehavior::Immediate))?;
        let id = new_id(&connection)?;
        let created_at_ms = now_ms()?;
        let (sender_name, run_id, sibling_parent, receiver): (String, Option<String>, Option<String>, String) = db("could not name message sender", connection.query_row(
            // A message takes the sender's run, or else the receiver's, so a reply to a worker
            // stays with its run. A workflow's own notices come from `workflow:<run id>`.
            "SELECT COALESCE(?2, d.handle || ' · ' || d.name, ?1),
                    COALESCE(d.run_id, CASE WHEN ?1 LIKE 'workflow:%' THEN substr(?1, 10) END, r.run_id),
                    CASE WHEN d.helper_of IS NOT NULL AND d.helper_of=r.helper_of THEN d.helper_of END,
                    COALESCE(r.handle || ' · ' || r.name, ?3)
             FROM (SELECT 1) LEFT JOIN session_directory d ON d.owned_id = ?1
             LEFT JOIN session_directory r ON r.owned_id = ?3",
            params![from, sender_name, to], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))))?;
        let sibling_parent = sibling_parent.filter(|_| kind == MessageKind::Message);
        if sibling_parent.is_some() {
            let count: i64 = db("could not count sibling messages", connection.query_row(
                "SELECT COUNT(*) FROM workflow_messages WHERE from_owned_id=?1 AND to_owned_id=?2 AND kind='message'",
                params![from, to], |row| row.get(0)))?;
            if count >= 20 {
                return Err(StoreError::message("Too many messages between these two helpers; send it through your orchestrator."));
            }
        }
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
        if let Some(parent) = sibling_parent {
            db("could not copy sibling message", connection.execute(
                "INSERT INTO workflow_messages
                 (id,group_id,from_owned_id,to_owned_id,sender_name,run_id,kind,body,receipt,created_at_ms,updated_at_ms)
                 VALUES (?,NULLIF(?,''),?,?,?,?, 'message',?,'queued',?,?)",
                params![new_id(&connection)?, group_id, from, parent, sender_name, run_id,
                    format!("[copy: {sender_name} to {receiver}]\n{body}"), created_at_ms, created_at_ms]))?;
        }
        db("could not finish the message write", connection.commit())?;
        drop(guard);
        messages_changed();
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

}
