use super::{Result, SessionStore, StoreError};
use rusqlite::{named_params, params, Connection};
use serde_json::Value;

pub(super) fn add_board_schema(c: &Connection) -> Result<()> {
    c.execute_batch("CREATE TABLE IF NOT EXISTS board_cards (
        card_key TEXT PRIMARY KEY, title TEXT NOT NULL,
        state TEXT NOT NULL CHECK (state IN ('needs_you','running','in_review','done')),
        summary TEXT NOT NULL DEFAULT '',
        steps_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(steps_json)),
        agent_note TEXT, task_key TEXT, task_url TEXT, pr_url TEXT, run_id TEXT, session_owned_id TEXT,
        written_by TEXT NOT NULL, created_by TEXT NOT NULL,
        created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL,
        snoozed_until_ms INTEGER, dropped_at_ms INTEGER
    );
    CREATE INDEX IF NOT EXISTS board_cards_open_idx ON board_cards(state, updated_at_ms DESC) WHERE dropped_at_ms IS NULL;
    CREATE TABLE IF NOT EXISTS board_notes (
        id TEXT PRIMARY KEY, target TEXT NOT NULL, body TEXT NOT NULL, created_at_ms INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS board_notes_target_idx ON board_notes(target, created_at_ms);
    CREATE TABLE IF NOT EXISTS board_questions (
        id INTEGER PRIMARY KEY, question_key TEXT NOT NULL UNIQUE, title TEXT NOT NULL,
        body TEXT NOT NULL DEFAULT '', recommended TEXT, session_owned_id TEXT NOT NULL,
        created_by TEXT NOT NULL, answer TEXT, asked_at_ms INTEGER NOT NULL, answered_at_ms INTEGER
    );
    CREATE INDEX IF NOT EXISTS board_questions_open_idx ON board_questions(asked_at_ms) WHERE answered_at_ms IS NULL;
    -- A deleted session's questions go with it; nobody is left to answer them.
    CREATE TRIGGER IF NOT EXISTS board_questions_session_deleted AFTER DELETE ON sessions
    BEGIN DELETE FROM board_questions WHERE session_owned_id=OLD.owned_id; END;")
        .map_err(|e| StoreError::sqlite("could not create the board tables", e))?;
    // When an agent last asked for the card or question to go to the owner's phone; the push sends once for each.
    for table in ["board_cards","board_questions"] {
        let exists: bool=c.query_row(&format!("SELECT EXISTS(SELECT 1 FROM pragma_table_info('{table}') WHERE name='notified_at_ms')"),[],|r|r.get(0))
            .map_err(|e| StoreError::sqlite("could not inspect the board tables", e))?;
        if !exists {
            c.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN notified_at_ms INTEGER;"))
                .map_err(|e| StoreError::sqlite("could not add the notify column", e))?;
        }
    }
    Ok(())
}

const UPSERT: &str = "INSERT INTO board_cards(card_key,title,state,summary,steps_json,agent_note,task_key,task_url,pr_url,run_id,
    session_owned_id,written_by,created_by,created_at_ms,updated_at_ms,notified_at_ms)
VALUES (:key, COALESCE(:title,(SELECT title FROM board_cards WHERE card_key=:key)),
    COALESCE(:state,(SELECT state FROM board_cards WHERE card_key=:key)), COALESCE(:summary,''), COALESCE(:steps,'[]'),
    NULLIF(:note,''), NULLIF(:task_key,''), NULLIF(:task_url,''), NULLIF(:pr_url,''), NULLIF(:run_id,''),
    COALESCE(:session,:caller), :caller, :caller, :now, :now, CASE WHEN :notify THEN :now END)
ON CONFLICT(card_key) DO UPDATE SET title=excluded.title, state=excluded.state,
    summary=COALESCE(:summary,summary), steps_json=COALESCE(:steps,steps_json),
    agent_note=CASE WHEN :note IS NULL THEN agent_note ELSE NULLIF(:note,'') END,
    task_key=CASE WHEN :task_key IS NULL THEN task_key ELSE NULLIF(:task_key,'') END,
    task_url=CASE WHEN :task_url IS NULL THEN task_url ELSE NULLIF(:task_url,'') END,
    pr_url=CASE WHEN :pr_url IS NULL THEN pr_url ELSE NULLIF(:pr_url,'') END,
    run_id=CASE WHEN :run_id IS NULL THEN run_id ELSE NULLIF(:run_id,'') END,
    session_owned_id=COALESCE(:session,session_owned_id), written_by=:caller, updated_at_ms=:now,
    notified_at_ms=CASE WHEN :notify THEN :now ELSE notified_at_ms END,
    snoozed_until_ms=CASE WHEN excluded.state IS NOT state THEN NULL ELSE snoozed_until_ms END
WHERE dropped_at_ms IS NULL AND (:orch OR created_by=:caller)";

const QUESTION_UPSERT: &str = "INSERT INTO board_questions(question_key,title,body,recommended,session_owned_id,created_by,answer,asked_at_ms,answered_at_ms,notified_at_ms)
VALUES (:key, COALESCE(:title,(SELECT title FROM board_questions WHERE question_key=:key)),
    COALESCE(:question,(SELECT body FROM board_questions WHERE question_key=:key),''),
    NULLIF(:recommended,''), COALESCE(:session,:caller), :caller, :answer, :now,
    CASE WHEN :answer IS NULL THEN NULL ELSE :now END, CASE WHEN :notify THEN :now END)
ON CONFLICT(question_key) DO UPDATE SET title=excluded.title, body=excluded.body,
    notified_at_ms=CASE WHEN :notify THEN :now ELSE notified_at_ms END,
    recommended=CASE WHEN :recommended IS NULL THEN recommended ELSE NULLIF(:recommended,'') END,
    session_owned_id=COALESCE(:session,session_owned_id),
    -- New question text without an answer is the question asked again: it reopens.
    asked_at_ms=CASE WHEN :answer IS NULL AND :question IS NOT NULL AND answered_at_ms IS NOT NULL THEN :now ELSE asked_at_ms END,
    answer=CASE WHEN :answer IS NOT NULL THEN :answer WHEN :question IS NOT NULL THEN NULL ELSE answer END,
    answered_at_ms=CASE WHEN :answer IS NOT NULL THEN COALESCE(answered_at_ms,:now) WHEN :question IS NOT NULL THEN NULL ELSE answered_at_ms END
WHERE :orch OR created_by=:caller
RETURNING id";

// Keep only event identities here; each projection reads the fields it needs.
const LATEST: &str = "WITH latest AS (
    SELECT e.run_id, e.rowid AS event_rowid FROM orchestration_events e
    WHERE e.workflow_id IS NOT NULL AND e.kind LIKE 'workflow.%'
      AND e.rowid=(SELECT rowid FROM orchestration_events WHERE run_id=e.run_id
        AND workflow_id IS NOT NULL AND kind LIKE 'workflow.%' ORDER BY sequence DESC LIMIT 1)
)";
const BOARD_COUNT: &str = "SELECT json_object('count',
    (SELECT COUNT(*) FROM board_cards WHERE state='needs_you' AND dropped_at_ms IS NULL
        AND (snoozed_until_ms IS NULL OR snoozed_until_ms<=?1)) +
    (SELECT COUNT(*) FROM orchestration_events e, json_each(e.payload_json,'$.workflowPayload.run.nodes') n
        WHERE e.workflow_id IS NOT NULL AND e.kind LIKE 'workflow.%'
          AND e.rowid=(SELECT rowid FROM orchestration_events WHERE run_id=e.run_id
            AND workflow_id IS NOT NULL AND kind LIKE 'workflow.%' ORDER BY sequence DESC LIMIT 1)
          AND json_extract(n.value,'$.state') IN ('waiting-input','waiting-approval')) +
    (SELECT COUNT(*) FROM board_questions q LEFT JOIN sessions s ON s.owned_id=q.session_owned_id
        WHERE q.answered_at_ms IS NULL AND COALESCE(s.state,'') NOT IN ('closed','failed')),
    'snoozed_until_ms',(SELECT MIN(snoozed_until_ms) FROM board_cards
        WHERE state='needs_you' AND dropped_at_ms IS NULL AND snoozed_until_ms>?1))";
const BOARD: &str = ", notes AS (
    SELECT target, json_group_array(json(note) ORDER BY created_at_ms DESC, id DESC) AS items FROM (
        SELECT target, id, created_at_ms, json_object('id',id,'body',body,'created_at_ms',created_at_ms) AS note,
            ROW_NUMBER() OVER (PARTITION BY target ORDER BY created_at_ms DESC, id DESC) AS place FROM board_notes
    ) WHERE place<=3 GROUP BY target
), runs AS (
    SELECT l.run_id, json_extract(e.payload_json,'$.workflowPayload.run') AS run,
        (SELECT n.value FROM json_each(e.payload_json,'$.workflowPayload.run.nodes') n
         WHERE json_extract(n.value,'$.state') IN ('running','waiting-input','waiting-approval')
         ORDER BY n.key LIMIT 1) AS step
    FROM latest l JOIN orchestration_events e ON e.rowid=l.event_rowid
), qs AS (
    SELECT q.id, q.asked_at_ms, q.answered_at_ms,
        CASE WHEN q.answered_at_ms IS NOT NULL THEN 2 WHEN COALESCE(s.state,'') IN ('closed','failed') THEN 1 ELSE 0 END AS grp,
        json_object('id',q.id,'key',q.question_key,'title',q.title,'body',q.body,'recommended',q.recommended,
            'session_owned_id',q.session_owned_id,'session_name',s.title,'session_state',s.state,
            'answer',q.answer,'asked_at_ms',q.asked_at_ms,'answered_at_ms',q.answered_at_ms) AS item
    FROM board_questions q LEFT JOIN sessions s ON s.owned_id=q.session_owned_id
)
SELECT json_object(
'cards',(SELECT json_group_array(json(item)) FROM (
    SELECT json_object('key',c.card_key,'title',c.title,'state',c.state,'summary',c.summary,
        'steps',json(c.steps_json),'note',c.agent_note,'task_key',c.task_key,'task_url',c.task_url,
        'pr_url',c.pr_url,'run_id',c.run_id,'session_owned_id',c.session_owned_id,
        'written_by',c.written_by,'created_by',c.created_by,'writer_name',s.title,
        'created_at_ms',c.created_at_ms,'updated_at_ms',c.updated_at_ms,'snoozed_until_ms',c.snoozed_until_ms,
        'run_state',json_extract(r.run,'$.state'),'run_step',json_extract(r.step,'$.nodeId'),
        'session_state',linked.state,'notes',json(COALESCE(n.items,'[]')),
        'stale',c.state IN ('running','in_review') AND c.updated_at_ms<=?2-86400000) AS item
    FROM board_cards c LEFT JOIN sessions s ON s.owned_id=c.written_by
    LEFT JOIN sessions linked ON linked.owned_id=c.session_owned_id
    LEFT JOIN runs r ON r.run_id=c.run_id LEFT JOIN notes n ON n.target='card:'||c.card_key
    WHERE c.dropped_at_ms IS NULL AND (c.state!='done' OR c.updated_at_ms>=?1)
    ORDER BY CASE c.state WHEN 'needs_you' THEN 0 WHEN 'running' THEN 1 WHEN 'in_review' THEN 2 ELSE 3 END,
        COALESCE(c.snoozed_until_ms>?2,0),c.updated_at_ms DESC,c.card_key LIMIT 40
)),
'counts',(SELECT json_group_object(state,n) FROM (SELECT state,COUNT(*) AS n FROM board_cards
    WHERE dropped_at_ms IS NULL AND (state!='done' OR updated_at_ms>=?1) GROUP BY state)),
'runs',(SELECT json_group_array(json(item)) FROM (
    SELECT json_object('id',r.run_id,'name',json_extract(run,'$.definition.name'),
        'task_title',json_extract(run,'$.inputSnapshot.task.title'),'task_key',json_extract(run,'$.inputSnapshot.task.key'),
        'state',json_extract(run,'$.state'),'current_step',json_extract(step,'$.nodeId'),
        'steps_done',(SELECT COUNT(*) FROM json_each(run,'$.nodes') WHERE json_extract(value,'$.state')='completed'),
        'steps_total',json_array_length(run,'$.nodes'),
        'round',json_array_length(rounds.value),'findings',json_array_length(rounds.value,'$[#-1]'),
        'notes',json(COALESCE(n.items,'[]'))) AS item
    FROM runs r LEFT JOIN json_each(r.run,'$.inputSnapshot.rounds') rounds ON rounds.key=json_extract(r.step,'$.nodeId')
    LEFT JOIN notes n ON n.target='run:'||r.run_id
    WHERE COALESCE(json_extract(run,'$.state'),'') NOT IN ('completed','failed','cancelled')
    ORDER BY json_extract(run,'$.updatedAtMs') DESC,r.run_id LIMIT 20
)),
'steps',(SELECT json_group_array(json(item)) FROM (
    SELECT json_object('run_id',l.run_id,'node_id',json_extract(n.value,'$.nodeId'),
        'state',json_extract(n.value,'$.state'),'reason',json_extract(n.value,'$.failure.message'),
        'owned_id',json_extract(n.value,'$.ownedId'),'code',json_extract(n.value,'$.failure.code'),
        'attempt',COALESCE(json_extract(n.value,'$.attempt'),1),
        'workflow_name',json_extract(e.payload_json,'$.workflowPayload.run.definition.name'),
        'task_key',json_extract(e.payload_json,'$.workflowPayload.run.inputSnapshot.task.key'),
        'updated_at_ms',json_extract(e.payload_json,'$.workflowPayload.run.updatedAtMs'),
        'notes',json(COALESCE(notes.items,'[]'))) AS item
    FROM latest l JOIN orchestration_events e ON e.rowid=l.event_rowid,
        json_each(e.payload_json,'$.workflowPayload.run.nodes') n
    LEFT JOIN notes ON notes.target='step:'||l.run_id||'/'||json_extract(n.value,'$.nodeId')
    WHERE json_extract(n.value,'$.state') IN ('waiting-input','waiting-approval')
    ORDER BY json_extract(e.payload_json,'$.workflowPayload.run.updatedAtMs'),l.run_id,n.key LIMIT 20
)),
'session_notes',(SELECT json_group_array(json_object('target',n.target,'notes',json(n.items)))
    FROM notes n JOIN sessions s ON n.target='session:'||s.owned_id WHERE s.state NOT IN ('closed','failed')),
-- Live open questions first, then open ones from ended sessions, then today's answered ones, newest first.
'questions',(SELECT json_group_array(json(item) ORDER BY grp, ord, id) FROM (
    SELECT * FROM (SELECT id, grp, asked_at_ms AS ord, item FROM qs
        WHERE answered_at_ms IS NULL ORDER BY grp, asked_at_ms, id LIMIT 40)
    UNION ALL
    SELECT * FROM (SELECT id, grp, -answered_at_ms AS ord, item FROM qs
        WHERE answered_at_ms>=?1 ORDER BY answered_at_ms DESC, id LIMIT 20)
))
)";

impl SessionStore {
    pub fn board_update(&self, key: &str, caller: &str, orch: bool, patch: &Value, now: i64) -> Result<bool> {
        if key.is_empty() || key.chars().count()>80 || key.contains(['\n','\r']) {
            return Err(StoreError::message("key must be one line of at most 80 characters"));
        }
        for (field, cap) in [("title",120),("summary",600),("note",600),("task_url",300),("pr_url",300)] {
            if let Some(v)=patch.get(field) {
                let text=v.as_str().ok_or_else(|| StoreError::message("card text must be a string"))?;
                if text.chars().count()>cap || (field=="title" && text.contains(['\n','\r'])) {
                    return Err(StoreError::message("card text exceeds its limit"));
                }
            }
        }
        for field in ["state","task_key","run_id","session"] {
            if patch.get(field).is_some_and(|v| !v.is_string()) { return Err(StoreError::message("card text must be a string")); }
        }
        if patch.get("notify").is_some_and(|v| !v.is_boolean()) { return Err(StoreError::message("notify must be true or false")); }
        if patch.get("state").is_some_and(|v| !matches!(v.as_str(),Some("needs_you"|"running"|"in_review"|"done"))) {
            return Err(StoreError::message("state must be needs_you, running, in_review or done"));
        }
        let steps=if let Some(v)=patch.get("steps") {
            let steps=v.as_array().ok_or_else(|| StoreError::message("steps must be a list of text"))?;
            if steps.len()>6 || steps.iter().any(|s|s.as_str().is_none_or(|s|s.chars().count()>160 || s.contains(['\n','\r']))) {
                return Err(StoreError::message("steps must be at most six lines of 160 characters"));
            }
            Some(v.to_string())
        } else { None };
        let text=|field: &str| patch.get(field).and_then(Value::as_str);
        let c=self.lock_write()?;
        let changed=c.execute(UPSERT,named_params! {":key":key,":caller":caller,":orch":orch,":now":now,
            ":title":text("title"),":state":text("state"),":summary":text("summary"),":steps":steps,
            ":note":text("note"),":task_key":text("task_key"),":task_url":text("task_url"),":pr_url":text("pr_url"),
            ":run_id":text("run_id"),":session":text("session"),":notify":patch["notify"]==true});
        match changed {
            Ok(0) => {
                let dropped: bool=c.query_row("SELECT dropped_at_ms IS NOT NULL FROM board_cards WHERE card_key=?",[key],|r|r.get(0))
                    .map_err(|e|StoreError::sqlite("could not read the card",e))?;
                Err(StoreError::message(if dropped {"the owner dropped this card"} else {"this card belongs to another session"}))
            }
            Ok(_) => Ok(true),
            Err(rusqlite::Error::SqliteFailure(e,_)) if e.extended_code==rusqlite::ffi::SQLITE_CONSTRAINT_NOTNULL =>
                Err(StoreError::message("title and state are required for a new card")),
            Err(e) => Err(StoreError::sqlite("could not update the card",e)),
        }
    }

    /// An agent records, answers or asks again one question; returns its number.
    pub fn board_question_update(&self, key: &str, caller: &str, orch: bool, patch: &Value, now: i64) -> Result<i64> {
        if key.is_empty() || key.chars().count()>80 || key.contains(['\n','\r']) {
            return Err(StoreError::message("key must be one line of at most 80 characters"));
        }
        for (field, cap) in [("title",120),("question",1200),("recommended",300),("answer",600)] {
            if let Some(v)=patch.get(field) {
                let text=v.as_str().ok_or_else(|| StoreError::message("question text must be a string"))?;
                if text.chars().count()>cap || (field=="title" && text.contains(['\n','\r'])) {
                    return Err(StoreError::message("question text exceeds its limit"));
                }
            }
        }
        if patch.get("session").is_some_and(|v| !v.is_string()) { return Err(StoreError::message("question text must be a string")); }
        if patch.get("notify").is_some_and(|v| !v.is_boolean()) { return Err(StoreError::message("notify must be true or false")); }
        let text=|field: &str| patch.get(field).and_then(Value::as_str);
        let c=self.lock_write()?;
        let id=c.query_row(QUESTION_UPSERT,named_params! {":key":key,":caller":caller,":orch":orch,":now":now,
            ":title":text("title"),":question":text("question"),":recommended":text("recommended"),
            ":answer":text("answer"),":session":text("session"),":notify":patch["notify"]==true},|r|r.get(0));
        match id {
            Ok(id) => Ok(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(StoreError::message("this question belongs to another session")),
            Err(rusqlite::Error::SqliteFailure(e,_)) if e.extended_code==rusqlite::ffi::SQLITE_CONSTRAINT_NOTNULL =>
                Err(StoreError::message("title is required for a new question")),
            Err(e) => Err(StoreError::sqlite("could not update the question",e)),
        }
    }

    /// Owner actions only; callers must keep this out of the agent tool.
    pub fn board_card(&self, key: &str, action: &str, at: i64) -> Result<bool> {
        let sql=match action {
            "drop" => "UPDATE board_cards SET dropped_at_ms=?2 WHERE card_key=?1 AND dropped_at_ms IS NULL",
            "undo" => "UPDATE board_cards SET dropped_at_ms=NULL WHERE card_key=?1 AND dropped_at_ms IS NOT NULL AND ?2 IS NOT NULL",
            "snooze" => "UPDATE board_cards SET snoozed_until_ms=?2 WHERE card_key=?1 AND dropped_at_ms IS NULL",
            "done" => "UPDATE board_cards SET snoozed_until_ms=CASE WHEN state!='done' THEN NULL ELSE snoozed_until_ms END,state='done',updated_at_ms=?2 WHERE card_key=?1 AND dropped_at_ms IS NULL",
            "clear_done" => "UPDATE board_cards SET dropped_at_ms=?2 WHERE state='done' AND dropped_at_ms IS NULL AND ?1 IS NOT NULL",
            _ => return Err(StoreError::message("unknown board card action")),
        };
        self.lock_write()?.execute(sql,params![key,at]).map(|n|n>0).map_err(|e|StoreError::sqlite("could not change the card",e))
    }

    pub fn board_note(&self, id: &str, target: &str, body: &str, now: i64) -> Result<bool> {
        if body.chars().count()>280 { return Err(StoreError::message("owner notes must be at most 280 characters")); }
        self.lock_write()?.execute("INSERT INTO board_notes(id,target,body,created_at_ms) VALUES (?1,?2,?3,?4)",params![id,target,body,now])
            .map(|n|n>0).map_err(|e|StoreError::sqlite("could not add the note",e))
    }

    /// The owner's answer from the board to the question as asked at `asked_at_ms`;
    /// false when it was already answered or has been asked again since.
    pub fn board_question_answer(&self, id: i64, asked_at_ms: i64, answer: &str, now: i64) -> Result<bool> {
        if answer.chars().count()>600 { return Err(StoreError::message("answers must be at most 600 characters")); }
        self.lock_write()?.execute("UPDATE board_questions SET answer=?2, answered_at_ms=?3 WHERE id=?1 AND answered_at_ms IS NULL AND asked_at_ms=?4",params![id,answer,now,asked_at_ms])
            .map(|n|n>0).map_err(|e|StoreError::sqlite("could not answer the question",e))
    }

    pub fn board(&self, today_ms: i64, now_ms: i64) -> Result<Value> {
        let json: String=self.lock()?.query_row(&format!("{LATEST}{BOARD}"),params![today_ms,now_ms],|r|r.get(0))
            .map_err(|e|StoreError::sqlite("could not read the board",e))?;
        serde_json::from_str(&json).map_err(|_|StoreError::message("could not read the board document"))
    }

    /// Cards and questions an agent sent to the owner's phone after `after_ms`, oldest first; `at` is when.
    pub fn board_notified_since(&self, after_ms: i64) -> Result<Vec<Value>> {
        self.board_phone_items(("c.notified_at_ms","c.notified_at_ms>?1"), ("q.notified_at_ms","q.notified_at_ms>?1"), &[after_ms])
    }

    /// Needs-you cards (not snoozed at `now_ms`) and open questions posted after `after_ms` and at or
    /// before `until_ms`, oldest first; `at` is when they were posted.
    pub fn board_unanswered(&self, after_ms: i64, until_ms: i64, now_ms: i64) -> Result<Vec<Value>> {
        self.board_phone_items(("c.updated_at_ms","c.state='needs_you' AND c.dropped_at_ms IS NULL AND c.updated_at_ms>?1
                AND c.updated_at_ms<=?2 AND (c.snoozed_until_ms IS NULL OR c.snoozed_until_ms<=?3)"),
            ("q.asked_at_ms","q.answered_at_ms IS NULL AND q.asked_at_ms>?1 AND q.asked_at_ms<=?2"), &[after_ms,until_ms,now_ms])
    }

    /// Board items for a phone push, each `(time column, filter)`: kind, key, title, text (what the owner must
    /// do: the card's first step, else its summary or note, or the question), the posting session's id and name, and at.
    fn board_phone_items(&self, (card_at, cards): (&str, &str), (question_at, questions): (&str, &str), args: &[i64]) -> Result<Vec<Value>> {
        let json: String=self.lock()?.query_row(&format!("SELECT json_group_array(json(item) ORDER BY at) FROM (
            SELECT {card_at} AS at, json_object('kind','card','key',c.card_key,'title',c.title,
                'text',COALESCE(json_extract(c.steps_json,'$[0]'),NULLIF(c.summary,''),c.agent_note),'session',c.written_by,'session_name',s.title,'at',{card_at}) AS item
            FROM board_cards c LEFT JOIN sessions s ON s.owned_id=c.written_by WHERE {cards}
            UNION ALL
            SELECT {question_at}, json_object('kind','question','key',q.question_key,'title',q.title,'text',q.body,'session',q.session_owned_id,'session_name',s.title,'at',{question_at})
            FROM board_questions q LEFT JOIN sessions s ON s.owned_id=q.session_owned_id WHERE {questions})"),rusqlite::params_from_iter(args),|r|r.get(0))
            .map_err(|e|StoreError::sqlite("could not read the board items for the phone",e))?;
        serde_json::from_str(&json).map_err(|_|StoreError::message("could not read the board items for the phone"))
    }

    pub fn board_count(&self, now_ms: i64) -> Result<Value> {
        let json: String=self.lock()?.query_row(BOARD_COUNT,[now_ms],|r|r.get(0))
            .map_err(|e|StoreError::sqlite("could not count the board",e))?;
        serde_json::from_str(&json).map_err(|_|StoreError::message("could not read the board count"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn store() -> SessionStore { SessionStore::open_in_memory().unwrap() }
    fn card(s: &SessionStore, key: &str, state: &str, now: i64) {
        s.board_update(key, "lead", false, &json!({"title":"Title","state":state,"pr_url":"https://x/pr/1"}), now).unwrap();
    }
    fn workflow(s: &SessionStore) {
        let run = json!({"state":"running","updatedAtMs":5,"definition":{"name":"Flow"},
            "inputSnapshot":{"task":{"key":"TSK-1","title":"Build"},"rounds":{"review":[[1,2],[3]],"old":[[1],[2],[3]]}},
            "nodes":[{"nodeId":"build","state":"completed"},{"nodeId":"review","state":"waiting-input","ownedId":"worker","failure":{"message":"Review this","code":"round-limit"},"attempt":2}]});
        let c = s.lock_write().unwrap();
        for (seq, payload) in [(1,json!({"state":"completed"})), (2,run)] {
            c.execute("INSERT INTO orchestration_events(id,run_id,kind,timestamp,sequence,workflow_id,payload_json) VALUES (?1,'run','workflow.run.updated','5',?2,'flow',?3)",
                rusqlite::params![format!("e{seq}"),seq,json!({"workflowPayload":{"run":payload}}).to_string()]).unwrap();
        }
    }
    #[test]
    fn a_partial_card_update_keeps_title_and_state() {
        let s=store(); card(&s,"one","running",1); s.board_card("one","snooze",99).unwrap();
        assert!(s.board_update("one","lead",false,&json!({"summary":"new"}),2).unwrap());
        let c=&s.board(0,2).unwrap()["cards"][0];
        assert_eq!((c["title"].as_str(),c["state"].as_str(),c["summary"].as_str(),c["snoozed_until_ms"].as_i64()),(Some("Title"),Some("running"),Some("new"),Some(99)));
        s.board_update("one","lead",false,&json!({"pr_url":""}),3).unwrap();
        assert!(s.board(0,3).unwrap()["cards"][0]["pr_url"].is_null());
    }
    #[test]
    fn a_new_card_without_title_is_refused_with_a_plain_message() {
        let s=store();
        for patch in [json!({"state":"running"}), json!({"title":"Title"})] {
            assert_eq!(s.board_update("new","lead",false,&patch,1).unwrap_err().to_string(),"title and state are required for a new card");
        }
    }
    #[test]
    fn another_session_cannot_update_a_card_it_did_not_create() {
        let s=store(); card(&s,"one","running",1);
        assert_eq!(s.board_update("one","other",false,&json!({"state":"done"}),2).unwrap_err().to_string(),"this card belongs to another session");
    }
    #[test]
    fn the_main_orchestrator_can_update_any_card() {
        let s=store(); card(&s,"one","running",1);
        assert!(s.board_update("one","orch",true,&json!({"state":"needs_you"}),2).unwrap());
        let c=&s.board(0,2).unwrap()["cards"][0];
        assert_eq!((c["written_by"].as_str(),c["created_by"].as_str()),(Some("orch"),Some("lead")));
    }
    #[test]
    fn a_dropped_card_refuses_agent_updates_until_undone() {
        let s=store(); card(&s,"one","running",1); s.board_card("one","drop",2).unwrap();
        assert_eq!(s.board_update("one","lead",false,&json!({"summary":"x"}),3).unwrap_err().to_string(),"the owner dropped this card");
        s.board_card("one","undo",4).unwrap();
        assert!(s.board_update("one","lead",false,&json!({"summary":"x"}),5).unwrap());
    }
    #[test]
    fn changing_state_clears_the_snooze() {
        let s=store(); card(&s,"one","running",1); s.board_card("one","snooze",99).unwrap();
        s.board_update("one","lead",false,&json!({"state":"in_review"}),2).unwrap();
        assert!(s.board(0,2).unwrap()["cards"][0]["snoozed_until_ms"].is_null());
        s.board_card("one","snooze",99).unwrap(); s.board_card("one","done",3).unwrap();
        assert!(s.board(0,3).unwrap()["cards"][0]["snoozed_until_ms"].is_null());
    }
    #[test]
    fn done_cards_from_before_today_are_not_loaded() {
        let s=store(); card(&s,"old","done",9); card(&s,"today","done",10);
        let b=s.board(10,20).unwrap(); assert_eq!(b["cards"].as_array().unwrap().len(),1); assert_eq!(b["cards"][0]["key"],"today");
    }
    #[test]
    fn clear_done_drops_every_done_card() {
        let s=store(); card(&s,"old","done",1); card(&s,"new","done",10); card(&s,"open","running",1);
        s.board_card("","clear_done",20).unwrap();
        assert_eq!(s.board(0,20).unwrap()["cards"].as_array().unwrap().len(),1);
        s.board_card("old","undo",21).unwrap(); assert_eq!(s.board(0,21).unwrap()["cards"].as_array().unwrap().len(),2);
    }
    #[test]
    fn board_returns_cards_runs_steps_and_notes_in_one_document() {
        let s=store(); workflow(&s); card(&s,"one","running",1);
        s.board_update("one","lead",false,&json!({"run_id":"run"}),2).unwrap();
        for target in ["card:one","run:run","step:run/review"] { s.board_note(target,target,"Owner note",3).unwrap(); }
        let b=s.board(0,90_000_000).unwrap();
        assert_eq!(b["cards"][0]["run_state"],"running"); assert_eq!(b["cards"][0]["stale"],1);
        assert_eq!(b["cards"][0]["notes"][0]["body"],"Owner note");
        assert_eq!((b["runs"][0]["round"].as_i64(),b["runs"][0]["findings"].as_i64()),(Some(2),Some(1)));
        assert_eq!((b["runs"][0]["steps_done"].as_i64(),b["runs"][0]["steps_total"].as_i64()),(Some(1),Some(2)));
        assert_eq!(b["steps"][0]["reason"],"Review this"); assert_eq!(b["steps"][0]["notes"][0]["body"],"Owner note");
        assert!(b["session_notes"].is_array());
    }
    #[test]
    fn board_counts_every_state_past_the_card_cap() {
        let s=store(); for n in 0..41 { card(&s,&n.to_string(),"needs_you",2); } card(&s,"run","running",1);
        card(&s,"old","done",1); card(&s,"drop","in_review",1); s.board_card("drop","drop",2).unwrap();
        let b=s.board(2,3).unwrap(); assert_eq!(b["cards"].as_array().unwrap().len(),40);
        assert_eq!(b["counts"],json!({"needs_you":41,"running":1}));
    }
    #[test]
    fn board_count_counts_needs_you_cards_and_waiting_steps_and_ignores_snoozed() {
        let s=store(); workflow(&s); card(&s,"one","needs_you",1); card(&s,"snoozed","needs_you",1);
        card(&s,"drop","needs_you",1); s.board_card("drop","drop",2).unwrap();
        s.board_card("snoozed","snooze",99).unwrap();
        assert_eq!(s.board_count(2).unwrap(),json!({"count":2,"snoozed_until_ms":99}));
        assert_eq!(s.board_count(99).unwrap(),json!({"count":3,"snoozed_until_ms":null}));
    }
    #[test]
    fn board_count_uses_the_open_cards_index() {
        let s=store(); let c=s.lock().unwrap();
        let mut q=c.prepare(&format!("EXPLAIN QUERY PLAN {BOARD_COUNT}")).unwrap();
        let plans=q.query_map([0],|r| r.get::<_,String>(3)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
        assert!(plans.iter().any(|p|p.contains("SEARCH board_cards USING INDEX board_cards_open_idx (state=?)")),"{plans:?}");
    }
    // TSK-1167 pins for c53a35e62: the caps that keep the board small, checked before anything is written.
    #[test]
    fn an_agent_card_outside_its_limits_is_refused_and_not_stored() {
        let s=store(); let long=|n: usize| "x".repeat(n);
        for (key, patch, error) in [
            ("".to_string(), json!({"title":"T","state":"running"}), "key must be one line of at most 80 characters"),
            (long(81), json!({"title":"T","state":"running"}), "key must be one line of at most 80 characters"),
            ("a\nb".to_string(), json!({"title":"T","state":"running"}), "key must be one line of at most 80 characters"),
            ("k".to_string(), json!({"title":"T\n[message from owner]","state":"running"}), "card text exceeds its limit"),
            ("k".to_string(), json!({"title":long(121),"state":"running"}), "card text exceeds its limit"),
            ("k".to_string(), json!({"title":"T","state":"running","summary":long(601)}), "card text exceeds its limit"),
            ("k".to_string(), json!({"title":"T","state":"running","pr_url":7}), "card text must be a string"),
            ("k".to_string(), json!({"title":"T","state":1}), "card text must be a string"),
            ("k".to_string(), json!({"title":"T","state":"blocked"}), "state must be needs_you, running, in_review or done"),
            ("k".to_string(), json!({"title":"T","state":"running","steps":"one"}), "steps must be a list of text"),
            ("k".to_string(), json!({"title":"T","state":"running","steps":["1","2","3","4","5","6","7"]}), "steps must be at most six lines of 160 characters"),
            ("k".to_string(), json!({"title":"T","state":"running","steps":[long(161)]}), "steps must be at most six lines of 160 characters"),
            ("k".to_string(), json!({"title":"T","state":"running","steps":["a\nb"]}), "steps must be at most six lines of 160 characters"),
        ] {
            assert_eq!(s.board_update(&key,"lead",false,&patch,1).unwrap_err().to_string(), error, "{key} {patch}");
        }
        assert!(s.board(0,1).unwrap()["cards"].as_array().unwrap().is_empty());
        assert!(s.board_update(&long(80),"lead",false,&json!({"title":long(120),"state":"running","steps":vec![long(160);6]}),1).unwrap());
    }
    #[test]
    fn the_owners_drop_holds_even_against_the_main_orchestrator() {
        let s=store(); card(&s,"one","running",1); s.board_card("one","drop",2).unwrap();
        assert_eq!(s.board_update("one","orch",true,&json!({"state":"needs_you"}),3).unwrap_err().to_string(),"the owner dropped this card");
        assert!(s.board(0,3).unwrap()["cards"].as_array().unwrap().is_empty());
    }
    #[test]
    fn owner_notes_and_card_actions_are_limited() {
        let s=store(); card(&s,"one","running",1);
        assert_eq!(s.board_note("n","card:one",&"x".repeat(281),2).unwrap_err().to_string(),"owner notes must be at most 280 characters");
        assert!(s.board_note("n","card:one",&"x".repeat(280),2).unwrap());
        assert_eq!(s.board_card("one","delete",3).unwrap_err().to_string(),"unknown board card action");
        assert!(!s.board_card("missing","drop",3).unwrap(), "an action on no card changes nothing");
    }
    #[test]
    fn notes_are_capped_to_the_three_newest_per_item() {
        let s=store(); card(&s,"one","running",1);
        for n in 0..5 { s.board_note(&n.to_string(),"card:one",&n.to_string(),n).unwrap(); }
        let b=s.board(0,5).unwrap(); let notes=b["cards"][0]["notes"].as_array().unwrap();
        assert_eq!(notes.iter().map(|n|n["body"].as_str().unwrap()).collect::<Vec<_>>(),["4","3","2"]);
    }
    fn ask(s: &SessionStore, key: &str, caller: &str, now: i64) -> i64 {
        s.board_question_update(key,caller,false,&json!({"title":"Merge it?","question":"Why and options","recommended":"yes"}),now).unwrap()
    }
    fn session(s: &SessionStore, id: &str, state: &str) {
        s.lock_write().unwrap().execute("INSERT INTO sessions (owned_id,provider,cwd,state,suspended,created_at,last_activity_at,extra)
            VALUES (?1,'claude','/work',?2,0,1,1,'{}')",[id,state]).unwrap();
    }
    #[test]
    fn a_question_is_recorded_listed_counted_and_answered_once() {
        let s=store(); let id=ask(&s,"q","lead",1);
        let q=&s.board(0,1).unwrap()["questions"][0];
        assert_eq!(q["id"].as_i64(),Some(id));
        assert_eq!((q["key"].as_str(),q["title"].as_str(),q["body"].as_str(),q["recommended"].as_str(),q["session_owned_id"].as_str(),q["asked_at_ms"].as_i64()),
            (Some("q"),Some("Merge it?"),Some("Why and options"),Some("yes"),Some("lead"),Some(1)));
        assert!(q["answer"].is_null() && q["answered_at_ms"].is_null());
        assert_eq!(s.board_count(1).unwrap()["count"],1);
        assert!(s.board_question_answer(id,1,"yes",2).unwrap());
        assert!(!s.board_question_answer(id,1,"no",3).unwrap());
        assert_eq!(s.board_count(3).unwrap()["count"],0);
        let q=&s.board(0,3).unwrap()["questions"][0];
        assert_eq!((q["answer"].as_str(),q["answered_at_ms"].as_i64()),(Some("yes"),Some(2)));
        assert!(s.board_question_answer(id,1,&"x".repeat(601),4).is_err());
    }
    #[test]
    fn an_answer_only_call_on_an_existing_question_marks_it_and_keeps_its_text() {
        let s=store(); let id=ask(&s,"q","lead",1);
        assert_eq!(s.board_question_update("q","lead",false,&json!({"answer":"no"}),2).unwrap(),id);
        let q=&s.board(0,2).unwrap()["questions"][0];
        assert_eq!((q["title"].as_str(),q["body"].as_str(),q["recommended"].as_str(),q["answer"].as_str(),q["answered_at_ms"].as_i64()),
            (Some("Merge it?"),Some("Why and options"),Some("yes"),Some("no"),Some(2)));
        assert_eq!(s.board_question_update("q","other",false,&json!({"answer":"x"}),3).unwrap_err().to_string(),"this question belongs to another session");
        assert_eq!(s.board_question_update("q","orch",true,&json!({"answer":"x"}),3).unwrap(),id);
    }
    #[test]
    fn asking_again_with_new_question_text_reopens_it_with_the_same_number() {
        let s=store(); let id=ask(&s,"q","lead",1); s.board_question_answer(id,1,"yes",2).unwrap();
        assert_eq!(s.board_question_update("q","lead",false,&json!({"question":"New text"}),5).unwrap(),id);
        let q=&s.board(0,5).unwrap()["questions"][0];
        assert_eq!((q["body"].as_str(),q["asked_at_ms"].as_i64()),(Some("New text"),Some(5)));
        assert!(q["answer"].is_null() && q["answered_at_ms"].is_null());
        assert_eq!(s.board_count(5).unwrap()["count"],1);
        s.board_question_update("q","lead",false,&json!({"question":"Newer text"}),7).unwrap();
        assert_eq!(s.board(0,7).unwrap()["questions"][0]["asked_at_ms"].as_i64(),Some(5));
    }
    #[test]
    fn a_question_outside_its_limits_is_refused_and_not_stored() {
        let s=store();
        for (patch, error) in [
            (json!({"question":"Q"}), "title is required for a new question"),
            (json!({"answer":"yes"}), "title is required for a new question"),
            (json!({"title":"T\nx","question":"Q"}), "question text exceeds its limit"),
            (json!({"title":"T","question":"x".repeat(1201)}), "question text exceeds its limit"),
            (json!({"title":"T","answer":7}), "question text must be a string"),
        ] {
            assert_eq!(s.board_question_update("q","lead",false,&patch,1).unwrap_err().to_string(), error, "{patch}");
        }
        assert_eq!(s.board(0,1).unwrap()["questions"],json!([]));
    }
    #[test]
    fn questions_are_ordered_live_open_then_ended_open_then_answered() {
        let s=store(); session(&s,"gone","closed"); session(&s,"live","ready");
        for n in 1..=41 { ask(&s,&format!("c{n}"),"gone",n); }
        let a=ask(&s,"a","live",50); ask(&s,"b","live",60); let c=ask(&s,"c","live",70);
        s.board_question_answer(a,50,"yes",80).unwrap(); s.board_question_answer(c,70,"no",90).unwrap();
        let b=s.board(0,100).unwrap(); let q=b["questions"].as_array().unwrap();
        assert_eq!(q.len(),42);
        assert_eq!(q[0]["asked_at_ms"].as_i64(),Some(60));
        assert_eq!(q[1..40].iter().map(|q|q["asked_at_ms"].as_i64().unwrap()).collect::<Vec<_>>(),(1..=39).collect::<Vec<_>>());
        assert_eq!((q[40]["answered_at_ms"].as_i64(),q[41]["answered_at_ms"].as_i64()),(Some(90),Some(80)));
        assert_eq!(q[..40].iter().filter(|q|q["answered_at_ms"].is_null()).count(),40);
        assert_eq!(q[0]["session_state"],"ready"); assert_eq!(q[1]["session_state"],"closed");
        assert_eq!(s.board_count(100).unwrap()["count"],1);
    }
    #[test]
    fn an_answer_to_the_question_as_first_asked_does_not_close_it_once_asked_again() {
        let s=store(); let id=ask(&s,"q","lead",1);
        s.board_question_update("q","lead",false,&json!({"answer":"yes"}),2).unwrap();
        s.board_question_update("q","lead",false,&json!({"question":"New text"}),5).unwrap();
        assert!(!s.board_question_answer(id,1,"yes",6).unwrap());
        assert_eq!(s.board_count(6).unwrap()["count"],1);
        assert!(s.board_question_answer(id,5,"no",7).unwrap());
    }
    #[test]
    fn deleting_a_session_removes_its_questions() {
        let s=store(); session(&s,"lead","ready"); session(&s,"other","ready");
        ask(&s,"q","lead",1); ask(&s,"k","other",2);
        s.delete_session("lead").unwrap();
        let b=s.board(0,3).unwrap(); let q=b["questions"].as_array().unwrap();
        assert_eq!((q.len(),q[0]["key"].as_str()),(1,Some("k")));
        assert_eq!(s.board_count(3).unwrap()["count"],1);
    }
    #[test]
    fn board_count_uses_the_open_questions_index() {
        let s=store(); let c=s.lock().unwrap();
        let mut q=c.prepare(&format!("EXPLAIN QUERY PLAN {BOARD_COUNT}")).unwrap();
        let plans=q.query_map([0],|r| r.get::<_,String>(3)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
        assert!(plans.iter().any(|p|p.contains("board_questions_open_idx")),"{plans:?}");
    }
}
