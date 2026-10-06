use std::fmt;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

use rusqlite::{
    named_params, params, params_from_iter, Connection, InterruptHandle, OptionalExtension, Row,
    TransactionBehavior,
};
use serde::Serialize;

mod evidence;

const SCHEMA_VERSION: i64 = 17;

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
/// noise and is not: replies streamed before each one also ended as a whole
/// message are stored only as deltas, so deleting them would delete half of
/// those conversations. Those are worth merging one day, which is a rewrite
/// rather than a delete.
const SUPERSEDED_BY_NEWER: [&str; 2] = ["usage.updated", "session.config.updated"];

/// Tool updates cannot join `SUPERSEDED_BY_NEWER` because they share one event
/// kind and must instead be superseded per item within each session.
const TOOL_ITEM_COLUMN_SCHEMA: &str = "ALTER TABLE events ADD COLUMN item_id TEXT
    GENERATED ALWAYS AS (json_extract(payload, '$.payload.itemId')) VIRTUAL;";
const TOOL_ITEM_INDEX_SCHEMA: &str =
    "CREATE INDEX IF NOT EXISTS events_item_idx ON events(owned_id, kind, item_id, seq);";
const TOOL_ITEM_SEQUENCE_INDEX_SCHEMA: &str =
    "CREATE INDEX IF NOT EXISTS events_owned_item_seq_idx ON events(owned_id, item_id, seq);
     CREATE INDEX IF NOT EXISTS events_tool_name_idx ON events(owned_id, item_id, seq)
       WHERE json_extract(payload,'$.payload.kind')='tool'
         AND NULLIF(json_extract(payload,'$.payload.name'),'') IS NOT NULL;
     CREATE INDEX IF NOT EXISTS events_tool_summary_idx ON events(owned_id, item_id, seq)
       WHERE json_extract(payload,'$.payload.kind')='tool'
         AND json_type(payload,'$.payload.summary') NOT IN ('null');
     CREATE INDEX IF NOT EXISTS events_tool_output_idx ON events(owned_id, item_id, seq)
       WHERE json_extract(payload,'$.payload.kind')='tool'
         AND json_type(payload,'$.payload.output') NOT IN ('null');
     CREATE INDEX IF NOT EXISTS events_tool_path_idx ON events(owned_id, item_id, seq)
       WHERE json_extract(payload,'$.payload.kind')='tool'
         AND json_type(payload,'$.payload.path') NOT IN ('null');
     CREATE INDEX IF NOT EXISTS events_tool_diff_idx ON events(owned_id, item_id, seq)
       WHERE json_extract(payload,'$.payload.kind')='tool'
         AND json_type(payload,'$.payload.diff') NOT IN ('null');";

const ITEM_PAGE_INDEX_SCHEMA: &str = "CREATE INDEX IF NOT EXISTS events_command_generation_idx
  ON events(owned_id,json_extract(payload,'$.generation'),seq)
  WHERE json_extract(payload,'$.payload.kind')='availableCommandsUpdate';
CREATE INDEX IF NOT EXISTS events_turn_item_seq_idx ON events(owned_id,turn_id,item_id,seq);
CREATE INDEX IF NOT EXISTS events_assistant_page_idx ON events(owned_id,seq)
  WHERE item_id IS NOT NULL AND ((kind='content.delta' AND json_extract(payload,'$.payload.kind')='assistantDelta')
    OR (kind='item.completed' AND json_extract(payload,'$.payload.kind')='assistantMessage'));
CREATE INDEX IF NOT EXISTS events_user_command_page_idx ON events(owned_id,json_extract(payload,'$.payload.kind'),seq)
  WHERE json_extract(payload,'$.payload.kind')='userMessage' OR json_extract(payload,'$.payload.kind')='availableCommandsUpdate';
CREATE INDEX IF NOT EXISTS events_replay_page_idx ON events(owned_id,seq)
  WHERE item_id IS NOT NULL AND (kind='remote.cached' OR json_extract(payload,'$.payload.kind')='terminalProjection');
CREATE INDEX IF NOT EXISTS events_side_page_idx ON events(owned_id,CASE
  WHEN kind IN ('usage.updated','session.config.updated') THEN kind
  WHEN kind='remote.cached' AND json_extract(payload,'$.payload.kind')='usage' THEN 'usage.updated'
  WHEN kind='remote.cached' AND json_extract(payload,'$.payload.kind')='terminalProjection'
    THEN json_extract(payload,'$.payload.eventType') END,seq)
  WHERE kind IN ('usage.updated','session.config.updated') OR (kind='remote.cached'
    AND (json_extract(payload,'$.payload.kind')='usage' OR
      (json_extract(payload,'$.payload.kind')='terminalProjection'
        AND json_extract(payload,'$.payload.eventType') IN ('usage.updated','session.config.updated'))));
CREATE INDEX IF NOT EXISTS events_tool_position_idx
  ON events(owned_id,COALESCE(json_extract(payload,'$.payload.firstSequence'),seq),seq)
  WHERE kind='item.updated' AND json_extract(payload,'$.payload.kind')='tool';";

const ITEM_PAGE_DESCRIPTOR_SQL: &str = r#"-- READ 1: persisted item descriptors, byte-budgeted in either page direction.
WITH assistant_starts AS (
  SELECT e.item_id,e.seq AS first_seq,e.created_at AS first_timestamp_ms,e.turn_id
  FROM events e WHERE e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.item_id IS NOT NULL
    AND ((e.kind='content.delta' AND json_extract(e.payload,'$.payload.kind')='assistantDelta')
      OR (e.kind='item.completed' AND json_extract(e.payload,'$.payload.kind')='assistantMessage'))
    AND NOT EXISTS (SELECT 1 FROM events p INDEXED BY events_item_idx
      WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high AND p.kind='content.delta' AND p.item_id=e.item_id AND p.seq<e.seq
        AND json_extract(p.payload,'$.payload.kind')='assistantDelta')
    AND NOT EXISTS (SELECT 1 FROM events p INDEXED BY events_item_idx
      WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high AND p.kind='item.completed' AND p.item_id=e.item_id AND p.seq<e.seq
        AND json_extract(p.payload,'$.payload.kind')='assistantMessage')
    AND e.seq < :cursor
  ORDER BY e.seq DESC LIMIT :item_ceiling
), assistant_items AS (
  SELECT c.*,MAX(e.seq) AS authority_seq
  FROM assistant_starts c LEFT JOIN events e INDEXED BY events_item_idx
    ON e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.kind='item.completed' AND e.item_id=c.item_id
      AND json_extract(e.payload,'$.payload.kind')='assistantMessage'
  GROUP BY c.item_id
), assistant_rows AS (
  SELECT a.item_id,a.first_seq,a.first_timestamp_ms,a.authority_seq,a.turn_id,e.seq,LENGTH(CAST(e.payload AS BLOB)) AS required_bytes,COALESCE(json_extract(e.payload,'$.payload.completed'),0) AS completed
  FROM assistant_items a CROSS JOIN events e INDEXED BY events_item_idx
  ON e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.kind='content.delta' AND e.item_id=a.item_id
  WHERE json_extract(e.payload,'$.payload.kind')='assistantDelta'
    AND (a.authority_seq IS NULL OR e.seq>a.authority_seq)
  UNION ALL
  SELECT a.item_id,a.first_seq,a.first_timestamp_ms,a.authority_seq,a.turn_id,e.seq,LENGTH(CAST(e.payload AS BLOB)) AS required_bytes,COALESCE(json_extract(e.payload,'$.payload.completed'),0) AS completed
  FROM assistant_items a CROSS JOIN events e INDEXED BY events_item_idx
  ON e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.kind='item.completed' AND e.item_id=a.item_id AND e.seq=a.authority_seq
  WHERE json_extract(e.payload,'$.payload.kind')='assistantMessage'
), native_user_starts AS MATERIALIZED (
  SELECT e.item_id,e.seq AS first_seq,e.created_at AS first_timestamp_ms
  FROM events e WHERE e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.kind='item.completed'
    AND json_extract(e.payload,'$.payload.kind')='userMessage' AND e.seq < :cursor
    AND NOT EXISTS (SELECT 1 FROM events p INDEXED BY events_item_idx
      WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high AND p.kind='item.completed' AND p.item_id=e.item_id AND p.seq<e.seq
        AND json_extract(p.payload,'$.payload.kind')='userMessage')
  ORDER BY e.seq DESC LIMIT :item_ceiling
), native_user_selected AS (
  SELECT e.item_id,e.seq,e.turn_id,s.first_seq,s.first_timestamp_ms,
    COALESCE(json_extract(e.payload,'$.payload.completed'),0) AS completed,LENGTH(CAST(e.payload AS BLOB)) AS required_bytes FROM native_user_starts s CROSS JOIN events e
  ON e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.seq=(SELECT p.seq FROM events p INDEXED BY events_item_idx
    WHERE p.owned_id=:owned_id AND p.seq BETWEEN :low AND :high AND p.kind='item.completed' AND p.item_id=s.item_id
      AND json_extract(p.payload,'$.payload.kind')='userMessage' ORDER BY p.seq DESC LIMIT 1)
), native_tool_selected AS (
  SELECT e.item_id,e.seq,e.turn_id,COALESCE(json_extract(e.payload,'$.payload.firstSequence'),e.seq) AS first_seq,
    COALESCE(json_extract(e.payload,'$.payload.firstTimestampMs'),e.created_at) AS first_timestamp_ms,
    json_extract(e.payload,'$.payload.state') IN ('completed','failed') AS completed,LENGTH(CAST(e.payload AS BLOB)) AS required_bytes
  FROM events e INDEXED BY events_tool_position_idx
  WHERE e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.kind='item.updated' AND json_extract(e.payload,'$.payload.kind')='tool'
    AND COALESCE(json_extract(e.payload,'$.payload.firstSequence'),e.seq) < :cursor
    AND NOT EXISTS (SELECT 1 FROM events p INDEXED BY events_item_idx
      WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high AND p.kind='item.updated' AND p.item_id=e.item_id AND p.seq>e.seq
        AND json_extract(p.payload,'$.payload.kind')='tool')
  ORDER BY COALESCE(json_extract(e.payload,'$.payload.firstSequence'),e.seq) DESC LIMIT :item_ceiling
), replay_starts AS (
  SELECT e.item_id FROM events e WHERE e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.item_id IS NOT NULL
    AND (e.kind='remote.cached' OR json_extract(e.payload,'$.payload.kind')='terminalProjection')
    AND NOT EXISTS (SELECT 1 FROM events p INDEXED BY events_owned_item_seq_idx
      WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high AND p.item_id=e.item_id AND p.seq<e.seq
        AND (p.kind='remote.cached' OR json_extract(p.payload,'$.payload.kind')='terminalProjection'))
    AND e.seq < :cursor
  ORDER BY e.seq DESC LIMIT :item_ceiling
), replay_rows AS (
  SELECT e.item_id,e.seq,e.created_at,e.turn_id,LENGTH(CAST(e.payload AS BLOB)) AS required_bytes FROM replay_starts s CROSS JOIN events e INDEXED BY events_owned_item_seq_idx
  ON e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.item_id=s.item_id WHERE e.item_id IS NOT NULL
    AND (e.kind='remote.cached' OR json_extract(e.payload,'$.payload.kind')='terminalProjection')
), replay_items AS (
  SELECT item_id,MIN(seq) AS first_seq,MIN(created_at) AS first_timestamp_ms,MAX(seq) AS last_seq,
         MIN(turn_id) AS turn_id,SUM(required_bytes) AS required_bytes
  FROM replay_rows GROUP BY item_id
), command_starts AS MATERIALIZED (
  SELECT e.seq AS first_seq,e.created_at AS first_timestamp_ms,json_extract(e.payload,'$.generation') AS generation
  FROM events e INDEXED BY events_user_command_page_idx WHERE e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high
    AND json_extract(e.payload,'$.payload.kind')='availableCommandsUpdate' AND e.seq < :cursor
    AND NOT EXISTS (SELECT 1 FROM events p INDEXED BY events_command_generation_idx
      WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high AND json_extract(p.payload,'$.payload.kind')='availableCommandsUpdate'
        AND json_extract(p.payload,'$.generation')=json_extract(e.payload,'$.generation') AND p.seq<e.seq)
  ORDER BY e.seq DESC LIMIT :item_ceiling
), command_items AS (
  SELECT s.*,e.seq,e.turn_id,LENGTH(CAST(e.payload AS BLOB)) AS required_bytes
  FROM command_starts s CROSS JOIN events e ON e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.seq=(
    SELECT p.seq FROM events p INDEXED BY events_command_generation_idx WHERE p.owned_id=:owned_id AND p.seq BETWEEN :low AND :high
      AND json_extract(p.payload,'$.payload.kind')='availableCommandsUpdate'
      AND json_extract(p.payload,'$.generation')=s.generation ORDER BY p.seq DESC LIMIT 1)
), exact_rows AS (
  SELECT e.seq,e.created_at,e.turn_id,LENGTH(CAST(e.payload AS BLOB)) AS required_bytes FROM events e WHERE e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.item_id IS NULL
    AND (json_extract(e.payload,'$.payload.kind') IN
      ('plan','contextCompaction','checkoutChanged','error','approval','userInputRequested')
      OR (json_extract(e.payload,'$.payload.kind')='terminalProjection' AND json_extract(e.payload,'$.payload.eventType')='item.completed'))
    AND e.seq < :cursor
  ORDER BY e.seq DESC LIMIT :item_ceiling
), side_candidates AS NOT MATERIALIZED (
  SELECT e.seq,e.created_at,e.turn_id,LENGTH(CAST(e.payload AS BLOB)) AS required_bytes,CASE
    WHEN e.kind IN ('usage.updated','session.config.updated') THEN e.kind
    WHEN e.kind='remote.cached' AND json_extract(e.payload,'$.payload.kind')='usage' THEN 'usage.updated'
    WHEN e.kind='remote.cached' AND json_extract(e.payload,'$.payload.kind')='terminalProjection'
      THEN json_extract(e.payload,'$.payload.eventType') END AS side_kind
  FROM events e WHERE e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high
    AND (e.kind IN ('usage.updated','session.config.updated') OR (e.kind='remote.cached'
      AND (json_extract(e.payload,'$.payload.kind')='usage' OR
        (json_extract(e.payload,'$.payload.kind')='terminalProjection'
          AND json_extract(e.payload,'$.payload.eventType') IN ('usage.updated','session.config.updated')))))
), side_latest AS MATERIALIZED (
  SELECT * FROM (SELECT * FROM side_candidates WHERE side_kind='usage.updated' ORDER BY seq DESC LIMIT 1)
  UNION ALL
  SELECT * FROM (SELECT * FROM side_candidates WHERE side_kind='session.config.updated' ORDER BY seq DESC LIMIT 1)
), side_selected AS (
  SELECT * FROM side_latest WHERE seq < :cursor
), descriptors AS (
  SELECT 'assistant:'||item_id AS page_key,item_id,'assistant' AS selection_mode,
         MIN(first_seq) AS first_seq,MIN(first_timestamp_ms) AS first_timestamp_ms,MAX(seq) AS last_seq,
         MAX(authority_seq) AS authority_seq,MIN(turn_id) AS turn_id,
         CASE WHEN MAX(seq)=MAX(authority_seq) THEN MAX(completed) ELSE 0 END AS completed,
         SUM(required_bytes) AS required_bytes
  FROM assistant_rows GROUP BY item_id
  UNION ALL
  SELECT 'user:'||item_id,item_id,'native-user',first_seq,first_timestamp_ms,seq,seq,turn_id,
         completed,required_bytes
  FROM native_user_selected
  UNION ALL
  SELECT 'tool:'||item_id,item_id,'native-tool',first_seq,first_timestamp_ms,seq,seq,turn_id,completed,required_bytes
  FROM native_tool_selected
  UNION ALL
  SELECT 'replay:'||item_id,item_id,'replay',first_seq,first_timestamp_ms,last_seq,NULL,turn_id,0,required_bytes
  FROM replay_items
  UNION ALL
  SELECT 'commands:'||generation,'commands:'||generation,'exact',first_seq,first_timestamp_ms,seq,seq,turn_id,1,required_bytes FROM command_items
  UNION ALL
  SELECT 'event:'||seq,'event:'||seq,'exact',seq,created_at,seq,seq,turn_id,1,required_bytes FROM exact_rows
  UNION ALL
  SELECT 'side:'||side_kind,'side:'||side_kind,'side',seq,created_at,seq,seq,turn_id,1,required_bytes FROM side_selected
), candidates AS (
  SELECT *,-first_seq AS page_order FROM descriptors
  WHERE first_seq < :cursor
  ORDER BY page_order,page_key LIMIT :item_ceiling
), spending AS (
  SELECT *,COALESCE(SUM(required_bytes) OVER (ORDER BY page_order,page_key
    ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING),0) AS spent_before FROM candidates
), selected AS (
  SELECT * FROM spending WHERE spent_before=0 OR spent_before+required_bytes<=:max_bytes
), bounds AS (
  SELECT MIN(first_seq) AS before_cursor,MAX(first_seq) AS after_cursor,
         COALESCE(SUM(required_bytes),0) AS transfer_bytes FROM selected
), page_facts AS (
  SELECT b.*,
         (EXISTS(SELECT 1 FROM events e WHERE e.owned_id=:owned_id AND e.seq BETWEEN :low AND MIN(:high,b.before_cursor) AND e.seq<b.before_cursor
           AND e.item_id IS NOT NULL AND ((e.kind='content.delta' AND json_extract(e.payload,'$.payload.kind')='assistantDelta') OR (e.kind='item.completed' AND json_extract(e.payload,'$.payload.kind')='assistantMessage'))
           AND NOT EXISTS (SELECT 1 FROM events p INDEXED BY events_owned_item_seq_idx
             WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high AND p.item_id=e.item_id AND p.seq<e.seq))
         OR EXISTS(SELECT 1 FROM events e WHERE e.owned_id=:owned_id AND e.seq BETWEEN :low AND MIN(:high,b.before_cursor) AND e.seq<b.before_cursor
           AND e.kind='item.completed' AND json_extract(e.payload,'$.payload.kind')='userMessage'
           AND NOT EXISTS (SELECT 1 FROM events p INDEXED BY events_owned_item_seq_idx
             WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high AND p.item_id=e.item_id AND p.seq<e.seq))
         OR EXISTS(SELECT 1 FROM events e WHERE e.owned_id=:owned_id AND e.seq BETWEEN :low AND MIN(:high,b.before_cursor) AND e.seq<b.before_cursor
           AND e.item_id IS NOT NULL AND (e.kind='remote.cached' OR json_extract(e.payload,'$.payload.kind')='terminalProjection')
           AND NOT EXISTS (SELECT 1 FROM events p INDEXED BY events_owned_item_seq_idx
             WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high AND p.item_id=e.item_id AND p.seq<e.seq))
         OR EXISTS(SELECT 1 FROM events e INDEXED BY events_tool_position_idx
           WHERE e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.kind='item.updated' AND json_extract(e.payload,'$.payload.kind')='tool'
             AND COALESCE(json_extract(e.payload,'$.payload.firstSequence'),e.seq) < b.before_cursor
             AND NOT EXISTS (SELECT 1 FROM events p INDEXED BY events_item_idx
               WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high AND p.kind='item.updated' AND p.item_id=e.item_id AND p.seq>e.seq
                 AND json_extract(p.payload,'$.payload.kind')='tool'))
         OR EXISTS(SELECT 1 FROM events e WHERE e.owned_id=:owned_id AND e.seq BETWEEN :low AND MIN(:high,b.before_cursor) AND e.seq<b.before_cursor AND e.item_id IS NULL
           AND (json_extract(e.payload,'$.payload.kind') IN
             ('plan','contextCompaction','checkoutChanged','error','approval','userInputRequested')
             OR (json_extract(e.payload,'$.payload.kind')='terminalProjection' AND json_extract(e.payload,'$.payload.eventType')='item.completed')))
         OR EXISTS(SELECT 1 FROM events e INDEXED BY events_user_command_page_idx
           WHERE e.owned_id=:owned_id AND e.seq BETWEEN :low AND MIN(:high,b.before_cursor) AND e.seq<b.before_cursor AND json_extract(e.payload,'$.payload.kind')='availableCommandsUpdate'
             AND NOT EXISTS(SELECT 1 FROM events p INDEXED BY events_command_generation_idx WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high
               AND json_extract(p.payload,'$.payload.kind')='availableCommandsUpdate'
               AND json_extract(p.payload,'$.generation')=json_extract(e.payload,'$.generation') AND p.seq<e.seq))
         OR EXISTS(SELECT 1 FROM side_latest WHERE seq < b.before_cursor)) AS has_before,
         (EXISTS(SELECT 1 FROM events e WHERE e.owned_id=:owned_id AND e.seq BETWEEN MAX(:low,b.after_cursor) AND :high AND e.seq>b.after_cursor
           AND e.item_id IS NOT NULL AND ((e.kind='content.delta' AND json_extract(e.payload,'$.payload.kind')='assistantDelta') OR (e.kind='item.completed' AND json_extract(e.payload,'$.payload.kind')='assistantMessage'))
           AND NOT EXISTS (SELECT 1 FROM events p INDEXED BY events_owned_item_seq_idx
             WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high AND p.item_id=e.item_id AND p.seq<e.seq))
         OR EXISTS(SELECT 1 FROM events e WHERE e.owned_id=:owned_id AND e.seq BETWEEN MAX(:low,b.after_cursor) AND :high AND e.seq>b.after_cursor
           AND e.kind='item.completed' AND json_extract(e.payload,'$.payload.kind')='userMessage'
           AND NOT EXISTS (SELECT 1 FROM events p INDEXED BY events_owned_item_seq_idx
             WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high AND p.item_id=e.item_id AND p.seq<e.seq))
         OR EXISTS(SELECT 1 FROM events e WHERE e.owned_id=:owned_id AND e.seq BETWEEN MAX(:low,b.after_cursor) AND :high AND e.seq>b.after_cursor
           AND e.item_id IS NOT NULL AND (e.kind='remote.cached' OR json_extract(e.payload,'$.payload.kind')='terminalProjection')
           AND NOT EXISTS (SELECT 1 FROM events p INDEXED BY events_owned_item_seq_idx
             WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high AND p.item_id=e.item_id AND p.seq<e.seq))
         OR EXISTS(SELECT 1 FROM events e INDEXED BY events_tool_position_idx
           WHERE e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.kind='item.updated' AND json_extract(e.payload,'$.payload.kind')='tool'
             AND COALESCE(json_extract(e.payload,'$.payload.firstSequence'),e.seq) > b.after_cursor
             AND NOT EXISTS (SELECT 1 FROM events p INDEXED BY events_item_idx
               WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high AND p.kind='item.updated' AND p.item_id=e.item_id AND p.seq>e.seq
                 AND json_extract(p.payload,'$.payload.kind')='tool'))
         OR EXISTS(SELECT 1 FROM events e WHERE e.owned_id=:owned_id AND e.seq BETWEEN MAX(:low,b.after_cursor) AND :high AND e.seq>b.after_cursor AND e.item_id IS NULL
           AND (json_extract(e.payload,'$.payload.kind') IN
             ('plan','contextCompaction','checkoutChanged','error','approval','userInputRequested')
             OR (json_extract(e.payload,'$.payload.kind')='terminalProjection' AND json_extract(e.payload,'$.payload.eventType')='item.completed')))
         OR EXISTS(SELECT 1 FROM events e INDEXED BY events_user_command_page_idx
           WHERE e.owned_id=:owned_id AND e.seq BETWEEN MAX(:low,b.after_cursor) AND :high AND e.seq>b.after_cursor AND json_extract(e.payload,'$.payload.kind')='availableCommandsUpdate'
             AND NOT EXISTS(SELECT 1 FROM events p INDEXED BY events_command_generation_idx WHERE p.owned_id=e.owned_id AND p.seq BETWEEN :low AND :high
               AND json_extract(p.payload,'$.payload.kind')='availableCommandsUpdate'
               AND json_extract(p.payload,'$.generation')=json_extract(e.payload,'$.generation') AND p.seq<e.seq))
         OR EXISTS(SELECT 1 FROM side_latest WHERE seq > b.after_cursor)) AS has_after
  FROM bounds b
)
SELECT s.item_id,s.page_key,s.selection_mode,s.first_seq,s.first_timestamp_ms,s.last_seq,
       s.authority_seq,s.turn_id,s.completed,s.required_bytes,b.before_cursor,b.after_cursor,b.transfer_bytes,
       (SELECT COALESCE(MAX(seq),0) FROM events WHERE owned_id=:owned_id AND seq BETWEEN :low AND :high) AS watermark,
       b.has_before,b.has_after
FROM page_facts b LEFT JOIN selected s ON TRUE ORDER BY s.first_seq ASC,s.page_key ASC;
"#;

const ITEM_PAGE_RECORD_SQL: &str = r#"WITH selected AS MATERIALIZED (
  SELECT json_extract(value,'$.itemId') AS item_id,json_extract(value,'$.selectionMode') AS selection_mode,
         json_extract(value,'$.authoritySeq') AS authority_seq FROM json_each(:selected_descriptors)
), assistant_selected AS MATERIALIZED (
  SELECT item_id,authority_seq FROM selected WHERE selection_mode='assistant'
), required AS (
  SELECT e.owned_id,e.seq,e.turn_id,e.kind,e.payload,e.created_at
  FROM assistant_selected s CROSS JOIN events e INDEXED BY events_item_idx
  ON e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.kind='content.delta' AND e.item_id=s.item_id
  WHERE json_extract(e.payload,'$.payload.kind')='assistantDelta'
    AND (s.authority_seq IS NULL OR e.seq>s.authority_seq)
  UNION ALL
  SELECT e.owned_id,e.seq,e.turn_id,e.kind,e.payload,e.created_at
  FROM assistant_selected s CROSS JOIN events e INDEXED BY events_item_idx
  ON e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.kind='item.completed' AND e.item_id=s.item_id AND e.seq=s.authority_seq
  WHERE json_extract(e.payload,'$.payload.kind')='assistantMessage'
  UNION ALL
  SELECT e.owned_id,e.seq,e.turn_id,e.kind,e.payload,e.created_at
  FROM selected s CROSS JOIN events e
  ON e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND s.selection_mode IN ('native-user','native-tool') AND e.seq=s.authority_seq
  UNION ALL
  SELECT e.owned_id,e.seq,e.turn_id,e.kind,e.payload,e.created_at
  FROM selected s CROSS JOIN events e INDEXED BY events_owned_item_seq_idx
  ON e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND s.selection_mode='replay' AND e.item_id=s.item_id
  WHERE e.kind='remote.cached' OR json_extract(e.payload,'$.payload.kind')='terminalProjection'
  UNION ALL
  SELECT e.owned_id,e.seq,e.turn_id,e.kind,e.payload,e.created_at
  FROM selected s CROSS JOIN events e
  ON e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND s.selection_mode IN ('exact','side') AND e.seq=s.authority_seq
)
SELECT * FROM required ORDER BY seq ASC;
"#;

const ITEM_PAGE_TURN_SQL: &str = r#"WITH represented AS (
  SELECT DISTINCT json_extract(value,'$.turnId') AS turn_id FROM json_each(:selected_descriptors)
  WHERE json_extract(value,'$.turnId') IS NOT NULL
), matched AS (
  SELECT e.turn_id,e.seq,e.kind,e.item_id,e.payload,e.created_at
  FROM represented r CROSS JOIN events e INDEXED BY events_turn_item_seq_idx
    ON e.owned_id=:owned_id AND e.seq BETWEEN :low AND :high AND e.turn_id=r.turn_id
), turn_rows AS (
  SELECT *,json_extract(payload,'$.payload.state') AS state FROM matched
  WHERE kind IN ('turn.started','turn.completed','turn.interrupted')
    OR (kind='remote.cached' AND json_extract(payload,'$.payload.kind')='turn')
), terminal_rows AS (
  SELECT *,ROW_NUMBER() OVER (PARTITION BY turn_id ORDER BY seq DESC) AS rank FROM turn_rows WHERE state<>'started'
), assistant_rows AS (
  SELECT *,ROW_NUMBER() OVER (PARTITION BY turn_id ORDER BY first_seq DESC) AS rank FROM (
    SELECT turn_id,item_id,MIN(seq) AS first_seq FROM matched WHERE item_id IS NOT NULL
      AND ((kind='content.delta' AND json_extract(payload,'$.payload.kind')='assistantDelta')
        OR (kind='item.completed' AND json_extract(payload,'$.payload.kind')='assistantMessage')
        OR (kind='remote.cached' AND json_extract(payload,'$.payload.kind') IN ('assistantDelta','assistantMessage')))
    GROUP BY turn_id,item_id
  )
), turn_bounds AS (
  SELECT turn_id,MIN(CASE WHEN state='started' THEN created_at END) AS started_at_ms,
         MAX(CASE WHEN state<>'started' THEN created_at END) AS ended_at_ms FROM turn_rows GROUP BY turn_id
)
SELECT r.turn_id,b.started_at_ms,b.ended_at_ms,x.state AS terminal_state,a.item_id AS final_assistant_item_id
FROM represented r LEFT JOIN turn_bounds b ON b.turn_id=r.turn_id
LEFT JOIN terminal_rows x ON x.turn_id=r.turn_id AND x.rank=1
LEFT JOIN assistant_rows a ON a.turn_id=r.turn_id AND a.rank=1 ORDER BY r.turn_id;"#;


const NORMALIZE_TOOL_EVENTS: &str = "WITH latest AS MATERIALIZED (
  SELECT e.rowid, e.owned_id, e.item_id, e.seq, e.created_at, e.payload
  FROM events e
  WHERE e.kind='item.updated' AND e.item_id IS NOT NULL
    AND json_extract(e.payload,'$.payload.kind')='tool'
    AND NOT EXISTS (SELECT 1 FROM events n
      WHERE n.owned_id=e.owned_id AND n.item_id=e.item_id AND n.kind='item.updated' AND n.seq>e.seq
        AND json_extract(n.payload,'$.payload.kind')='tool')
), facts AS MATERIALIZED (
  SELECT l.*,
    COALESCE((SELECT json_extract(e.payload,'$.payload.name') FROM events e
      WHERE e.owned_id=l.owned_id AND e.item_id=l.item_id AND json_extract(e.payload,'$.payload.kind')='tool'
        AND e.seq<=l.seq
        AND NULLIF(json_extract(e.payload,'$.payload.name'),'') IS NOT NULL
      ORDER BY e.seq DESC LIMIT 1),'') name,
    json_extract(l.payload,'$.payload.state') state,
    (SELECT json_extract(e.payload,'$.payload.summary') FROM events e
      WHERE e.owned_id=l.owned_id AND e.item_id=l.item_id AND json_extract(e.payload,'$.payload.kind')='tool'
        AND e.seq<=l.seq
        AND json_type(e.payload,'$.payload.summary') NOT IN ('null') ORDER BY e.seq DESC LIMIT 1) summary,
    (SELECT json_extract(e.payload,'$.payload.output') FROM events e
      WHERE e.owned_id=l.owned_id AND e.item_id=l.item_id AND json_extract(e.payload,'$.payload.kind')='tool'
        AND e.seq<=l.seq
        AND json_type(e.payload,'$.payload.output') NOT IN ('null') ORDER BY e.seq DESC LIMIT 1) output,
    (SELECT json_extract(e.payload,'$.payload.path') FROM events e
      WHERE e.owned_id=l.owned_id AND e.item_id=l.item_id AND json_extract(e.payload,'$.payload.kind')='tool'
        AND e.seq<=l.seq
        AND json_type(e.payload,'$.payload.path') NOT IN ('null') ORDER BY e.seq DESC LIMIT 1) path,
    (SELECT json_extract(e.payload,'$.payload.diff') FROM events e
      WHERE e.owned_id=l.owned_id AND e.item_id=l.item_id AND json_extract(e.payload,'$.payload.kind')='tool'
        AND e.seq<=l.seq
        AND json_type(e.payload,'$.payload.diff') NOT IN ('null') ORDER BY e.seq DESC LIMIT 1) diff,
    (SELECT json_extract(e.payload,'$.payload.firstSequence') FROM events e
      WHERE e.owned_id=l.owned_id AND e.item_id=l.item_id AND json_extract(e.payload,'$.payload.kind')='tool'
        AND e.seq<=l.seq
        AND json_type(e.payload,'$.payload.firstSequence')='integer' ORDER BY e.seq DESC LIMIT 1) stored_first_seq,
    (SELECT json_extract(e.payload,'$.payload.firstTimestampMs') FROM events e
      WHERE e.owned_id=l.owned_id AND e.item_id=l.item_id AND json_extract(e.payload,'$.payload.kind')='tool'
        AND e.seq<=l.seq
        AND json_type(e.payload,'$.payload.firstTimestampMs')='integer' ORDER BY e.seq DESC LIMIT 1) stored_first_time,
    (SELECT e.seq FROM events e WHERE e.owned_id=l.owned_id AND e.item_id=l.item_id
      AND json_extract(e.payload,'$.payload.kind')='tool' ORDER BY e.seq ASC LIMIT 1) surviving_first_seq,
    (SELECT e.created_at FROM events e WHERE e.owned_id=l.owned_id AND e.item_id=l.item_id
      AND json_extract(e.payload,'$.payload.kind')='tool' ORDER BY e.seq ASC LIMIT 1) surviving_first_time
  FROM latest l
), merged AS MATERIALIZED (
  SELECT *,
    CASE WHEN stored_first_seq IS NOT NULL AND stored_first_time IS NOT NULL
                   AND stored_first_seq<=surviving_first_seq
         THEN stored_first_seq ELSE surviving_first_seq END first_seq,
    CASE WHEN stored_first_seq IS NOT NULL AND stored_first_time IS NOT NULL
                   AND stored_first_seq<=surviving_first_seq
         THEN stored_first_time ELSE surviving_first_time END first_time
  FROM facts
), payloads AS MATERIALIZED (
  SELECT rowid, payload, json_patch(json_patch(json_patch(json_patch(
    json_object('kind','tool','itemId',item_id,'name',name,'state',state,
      'firstSequence',first_seq,'firstTimestampMs',first_time),
    CASE WHEN summary IS NULL THEN '{}' ELSE json_object('summary',summary) END),
    CASE WHEN output IS NULL THEN '{}' ELSE json_object('output',output) END),
    CASE WHEN path IS NULL THEN '{}' ELSE json_object('path',path) END),
    CASE WHEN diff IS NULL THEN '{}' ELSE json_object('diff',diff) END) tool
  FROM merged
)
UPDATE events AS e
SET payload=json_set(p.payload,'$.payload',json(p.tool))
FROM payloads AS p
WHERE e.rowid=p.rowid;";

const INSERT_NORMALIZED_TOOL_EVENT: &str = "WITH incoming(owned_id,seq,turn_id,kind,raw,created_at,item_id) AS (
  VALUES (?1,?2,?3,?4,json(?5),?6,json_extract(?5,'$.payload.itemId'))
), facts AS MATERIALIZED (
  SELECT i.*,
    COALESCE(NULLIF(json_extract(i.raw,'$.payload.name'),''),
      (SELECT json_extract(e.payload,'$.payload.name') FROM events e
       WHERE e.owned_id=i.owned_id AND e.item_id=i.item_id
         AND json_extract(e.payload,'$.payload.kind')='tool'
         AND NULLIF(json_extract(e.payload,'$.payload.name'),'') IS NOT NULL
       ORDER BY e.seq DESC LIMIT 1),'') name,
    json_extract(i.raw,'$.payload.state') state,
    COALESCE(json_extract(i.raw,'$.payload.summary'),
      (SELECT json_extract(e.payload,'$.payload.summary') FROM events e
       WHERE e.owned_id=i.owned_id AND e.item_id=i.item_id
         AND json_extract(e.payload,'$.payload.kind')='tool'
         AND json_type(e.payload,'$.payload.summary') NOT IN ('null') ORDER BY e.seq DESC LIMIT 1)) summary,
    COALESCE(json_extract(i.raw,'$.payload.output'),
      (SELECT json_extract(e.payload,'$.payload.output') FROM events e
       WHERE e.owned_id=i.owned_id AND e.item_id=i.item_id
         AND json_extract(e.payload,'$.payload.kind')='tool'
         AND json_type(e.payload,'$.payload.output') NOT IN ('null') ORDER BY e.seq DESC LIMIT 1)) output,
    COALESCE(json_extract(i.raw,'$.payload.path'),
      (SELECT json_extract(e.payload,'$.payload.path') FROM events e
       WHERE e.owned_id=i.owned_id AND e.item_id=i.item_id
         AND json_extract(e.payload,'$.payload.kind')='tool'
         AND json_type(e.payload,'$.payload.path') NOT IN ('null') ORDER BY e.seq DESC LIMIT 1)) path,
    COALESCE(json_extract(i.raw,'$.payload.diff'),
      (SELECT json_extract(e.payload,'$.payload.diff') FROM events e
       WHERE e.owned_id=i.owned_id AND e.item_id=i.item_id
         AND json_extract(e.payload,'$.payload.kind')='tool'
         AND json_type(e.payload,'$.payload.diff') NOT IN ('null') ORDER BY e.seq DESC LIMIT 1)) diff,
    (SELECT json_extract(e.payload,'$.payload.firstSequence') FROM events e
     WHERE e.owned_id=i.owned_id AND e.item_id=i.item_id
       AND json_extract(e.payload,'$.payload.kind')='tool'
       AND json_type(e.payload,'$.payload.firstSequence')='integer' ORDER BY e.seq DESC LIMIT 1) stored_first_seq,
    (SELECT json_extract(e.payload,'$.payload.firstTimestampMs') FROM events e
     WHERE e.owned_id=i.owned_id AND e.item_id=i.item_id
       AND json_extract(e.payload,'$.payload.kind')='tool'
       AND json_type(e.payload,'$.payload.firstTimestampMs')='integer' ORDER BY e.seq DESC LIMIT 1) stored_first_time,
    (SELECT e.seq FROM events e WHERE e.owned_id=i.owned_id AND e.item_id=i.item_id
     AND json_extract(e.payload,'$.payload.kind')='tool'
     ORDER BY e.seq ASC LIMIT 1) surviving_first_seq,
    (SELECT e.created_at FROM events e WHERE e.owned_id=i.owned_id AND e.item_id=i.item_id
     AND json_extract(e.payload,'$.payload.kind')='tool'
     ORDER BY e.seq ASC LIMIT 1) surviving_first_time
  FROM incoming i
), merged AS MATERIALIZED (
  SELECT *,
    CASE WHEN stored_first_seq IS NOT NULL AND stored_first_time IS NOT NULL
                   AND stored_first_seq<=surviving_first_seq
         THEN stored_first_seq ELSE COALESCE(surviving_first_seq,seq) END first_seq,
    CASE WHEN stored_first_seq IS NOT NULL AND stored_first_time IS NOT NULL
                   AND stored_first_seq<=surviving_first_seq
         THEN stored_first_time ELSE COALESCE(surviving_first_time,created_at) END first_time
  FROM facts
), payloads AS (
  SELECT *,json_patch(json_patch(json_patch(json_patch(
    json_object('kind','tool','itemId',item_id,'name',name,'state',state,
      'firstSequence',first_seq,'firstTimestampMs',first_time),
    CASE WHEN summary IS NULL THEN '{}' ELSE json_object('summary',summary) END),
    CASE WHEN output IS NULL THEN '{}' ELSE json_object('output',output) END),
    CASE WHEN path IS NULL THEN '{}' ELSE json_object('path',path) END),
    CASE WHEN diff IS NULL THEN '{}' ELSE json_object('diff',diff) END) tool
  FROM merged
)
INSERT INTO events(owned_id,seq,turn_id,kind,payload,created_at)
SELECT owned_id,seq,turn_id,kind,
  json_set(raw,'$.sequence',seq,'$.timestampMs',created_at,'$.payload',json(tool)),created_at
FROM payloads;";

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventCoverage {
    pub low: i64,
    pub high: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemPageDescriptor {
    pub stable_id: String,
    pub item_id: String,
    #[serde(rename = "selectionMode")]
    pub record_mode: String,
    pub first_sequence: i64,
    pub first_timestamp_ms: i64,
    pub last_sequence: i64,
    #[serde(rename = "authoritySeq")]
    pub authority_sequence: Option<i64>,
    pub turn_id: Option<String>,
    pub completed: bool,
    pub required_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepresentedTurnFacts {
    pub turn_id: String,
    pub started_at_ms: Option<i64>,
    pub ended_at_ms: Option<i64>,
    pub terminal_state: Option<String>,
    pub final_assistant_item_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemPage {
    pub items: Vec<ItemPageDescriptor>,
    pub events: Vec<EventRow>,
    pub turns: Vec<RepresentedTurnFacts>,
    pub before_cursor: Option<i64>,
    pub after_cursor: Option<i64>,
    pub has_before: bool,
    pub has_after: bool,
    pub watermark: i64,
    pub transfer_bytes: u64,
    pub oversized: bool,
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
    pub priorities: Vec<String>,
    pub has_more: bool,
}

impl SessionStore {
    const ITEM_PAGE_CANDIDATE_CEILING: i64 = 512;
    pub fn open(path: &Path) -> Result<Self> {
        let connection = Connection::open(path)
            .map_err(|error| StoreError::sqlite("could not open the session database", error))?;
        Self::from_connection(connection)
    }

    /// Writes a bounded remote page and its coverage metadata atomically.
    /// Replayed events replace the same primary key, never duplicate history.
    pub fn cache_remote_events(&self, profile_id: &str, session: &SessionRow, events: &[EventRow]) -> Result<()> {
        let mut connection = self.lock_write()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| StoreError::sqlite("could not begin remote history write", error))?;
        let local_collision: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM sessions WHERE owned_id = ?1
                AND cached_remote_profile_id IS NOT ?2)",
            params![session.owned_id, profile_id], |row| row.get(0),
        ).map_err(|error| StoreError::sqlite("could not check remote history ownership", error))?;
        if local_collision { return Err(StoreError::message("remote cache cannot overwrite a local session")); }
        upsert_session_on(&transaction, session)?;
        transaction.execute("UPDATE sessions SET cached_remote_profile_id = ? WHERE owned_id = ?",
            params![profile_id, session.owned_id])
            .map_err(|error| StoreError::sqlite("could not mark remote history ownership", error))?;
        for event in events {
            transaction.execute(
                "INSERT INTO events (owned_id, seq, turn_id, kind, payload, created_at)
                 VALUES (?, ?, ?, ?, ?, ?)
                 ON CONFLICT(owned_id, seq) DO UPDATE SET turn_id=excluded.turn_id,
                   kind=excluded.kind, payload=excluded.payload, created_at=excluded.created_at",
                params![session.owned_id, event.seq, event.turn_id, event.kind,
                    event.payload_json, event.created_at_ms],
            ).map_err(|error| StoreError::sqlite("could not cache remote event", error))?;
        }
        transaction.commit()
            .map_err(|error| StoreError::sqlite("could not finish remote history write", error))
    }

    pub fn upsert_child_session(
        &self,
        parent_owned_id: &str,
        expected_remote_profile_id: Option<&str>,
        child: &SessionRow,
    ) -> Result<()> {
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| StoreError::sqlite("could not begin child session write", error))?;
        let parent_matches: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sessions
                    WHERE owned_id = ?1 AND cached_remote_profile_id IS ?2)",
                params![parent_owned_id, expected_remote_profile_id],
                |row| row.get(0),
            )
            .map_err(|error| StoreError::sqlite("could not check child session parent", error))?;
        if !parent_matches {
            return Err(StoreError::message("child session parent or source does not match"));
        }
        let ownership_conflict: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sessions WHERE owned_id = ?1
                    AND (parent_owned_id IS NOT ?2 OR cached_remote_profile_id IS NOT ?3))",
                params![child.owned_id, parent_owned_id, expected_remote_profile_id],
                |row| row.get(0),
            )
            .map_err(|error| StoreError::sqlite("could not check child session ownership", error))?;
        if ownership_conflict {
            return Err(StoreError::message("child session ownership cannot change"));
        }
        upsert_session_on(&transaction, child)?;
        let changed = transaction
            .execute(
                "UPDATE sessions SET parent_owned_id = ?1, cached_remote_profile_id = ?2
                 WHERE owned_id = ?3",
                params![parent_owned_id, expected_remote_profile_id, child.owned_id],
            )
            .map_err(|error| StoreError::sqlite("could not record child session ownership", error))?;
        if changed != 1 {
            return Err(StoreError::message("child session ownership was not recorded"));
        }
        transaction
            .commit()
            .map_err(|error| StoreError::sqlite("could not finish child session write", error))
    }

    pub fn purge_remote_history(&self, profile_id: &str) -> Result<()> {
        self.lock_write()?.execute(
            "DELETE FROM sessions WHERE cached_remote_profile_id = ?",
            [profile_id],
        ).map_err(|error| StoreError::sqlite("could not purge remote history", error))?;
        Ok(())
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
                add_child_session_schema(&transaction)?;
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
                upgrade_tool_events(&transaction)?;
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
                upgrade_tool_events(&transaction)?;
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
                upgrade_tool_events(&transaction)?;
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
                upgrade_tool_events(&transaction)?;
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
                upgrade_tool_events(&transaction)?;
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
                upgrade_tool_events(&transaction)?;
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
                upgrade_tool_events(&transaction)?;
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
                upgrade_tool_events(&transaction)?;
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
                upgrade_tool_events(&transaction)?;
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
                upgrade_tool_events(&transaction)?;
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
                upgrade_tool_events(&transaction)?;
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
                upgrade_tool_events(&transaction)?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| {
                        StoreError::sqlite("could not record the upgraded schema version", error)
                    })?;
                transaction.commit().map_err(|error| {
                    StoreError::sqlite("could not finish the evidence disk usage upgrade", error)
                })?;
            }
            13 => {
                let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| StoreError::sqlite("could not begin remote history upgrade", error))?;
                add_remote_history_column(&transaction)?;
                upgrade_tool_events(&transaction)?;
                transaction.pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| StoreError::sqlite("could not record remote history upgrade", error))?;
                transaction.commit().map_err(|error| StoreError::sqlite("could not finish remote history upgrade", error))?;
            }
            14 => {
                let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| StoreError::sqlite("could not begin the tool normalization upgrade", error))?;
                upgrade_tool_events(&transaction)?;
                transaction.pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| StoreError::sqlite("could not record the tool normalization upgrade", error))?;
                transaction.commit()
                    .map_err(|error| StoreError::sqlite("could not finish the tool normalization upgrade", error))?;
            }
            15 => {
                let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| StoreError::sqlite("could not begin the item page upgrade", error))?;
                add_tool_item_schema(&transaction)?;
                transaction.pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| StoreError::sqlite("could not record the item page upgrade", error))?;
                transaction.commit()
                    .map_err(|error| StoreError::sqlite("could not finish the item page upgrade", error))?;
            }
            16 => {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|error| StoreError::sqlite("could not begin the child session upgrade", error))?;
                add_child_session_schema(&transaction)?;
                transaction
                    .pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(|error| StoreError::sqlite("could not record the child session upgrade", error))?;
                transaction
                    .commit()
                    .map_err(|error| StoreError::sqlite("could not finish the child session upgrade", error))?;
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
        add_remote_history_column(&connection)?;
        add_child_session_schema(&connection)?;

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
            insert_chronological_event(&transaction, event)?;
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
            insert_chronological_event(&transaction, event)?;
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
                 WHERE cached_remote_profile_id IS NULL AND parent_owned_id IS NULL
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
            .query_row("SELECT COUNT(*) FROM sessions
                        WHERE cached_remote_profile_id IS NULL AND parent_owned_id IS NULL", [], |row| row.get(0))
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
                    snapshot_json = CASE
                        WHEN json_type(session_workspaces.snapshot_json, '$.expandedPathsByRoot') = 'object'
                        THEN json_set(excluded.snapshot_json, '$.expandedPathsByRoot',
                            json_extract(session_workspaces.snapshot_json, '$.expandedPathsByRoot'))
                        ELSE excluded.snapshot_json END,
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
        statuses: &[String],
        priorities: &[String],
        sort_by: &str,
        sort_direction: &str,
    ) -> Result<NotionTaskProjectionPage> {
        let statuses_json = serde_json::to_string(statuses)
            .map_err(|_| StoreError::message("could not encode Notion status filters"))?;
        let priorities_json = serde_json::to_string(priorities)
            .map_err(|_| StoreError::message("could not encode Notion priority filters"))?;
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
                   AND (json_array_length(?3) > 0 OR lower(status) <> 'future')
                   AND (json_array_length(?3) = 0 OR
                        COALESCE(NULLIF(status, ''), 'Unspecified') IN (SELECT value FROM json_each(?3)))
                   AND (json_array_length(?8) = 0 OR
                        COALESCE(NULLIF(priority, ''), 'Unspecified') IN (SELECT value FROM json_each(?8)))
                 ORDER BY CASE WHEN ?4 = 'taskNumber' AND ?5 = 'asc' THEN
                              CASE
                                WHEN upper(title) GLOB '[[]TSK-[0-9]*[]]*' THEN
                                  CAST(substr(title, 6, instr(substr(title, 6), ']') - 1) AS INTEGER)
                                WHEN ltrim(title) GLOB '[0-9]*' THEN CAST(ltrim(title) AS INTEGER)
                                ELSE 2147483647
                              END
                            END ASC,
                          CASE WHEN ?4 = 'taskNumber' AND ?5 = 'desc' THEN
                              CASE
                                WHEN upper(title) GLOB '[[]TSK-[0-9]*[]]*' THEN
                                  CAST(substr(title, 6, instr(substr(title, 6), ']') - 1) AS INTEGER)
                                WHEN ltrim(title) GLOB '[0-9]*' THEN CAST(ltrim(title) AS INTEGER)
                                ELSE -1
                              END
                            END DESC,
                          CASE WHEN ?4 = 'title' AND ?5 = 'asc' THEN
                              CASE
                                WHEN upper(title) LIKE '[TSK-%]%' THEN
                                  ltrim(substr(title, instr(title, ']') + 1))
                                WHEN ltrim(title) GLOB '[0-9]*' THEN
                                  ltrim(ltrim(ltrim(title), '0123456789'))
                                ELSE title
                              END
                            END COLLATE NOCASE ASC,
                          CASE WHEN ?4 = 'title' AND ?5 = 'desc' THEN
                              CASE
                                WHEN upper(title) LIKE '[TSK-%]%' THEN
                                  ltrim(substr(title, instr(title, ']') + 1))
                                WHEN ltrim(title) GLOB '[0-9]*' THEN
                                  ltrim(ltrim(ltrim(title), '0123456789'))
                                ELSE title
                              END
                            END COLLATE NOCASE DESC,
                          CASE WHEN ?4 = '' THEN project END COLLATE NOCASE ASC,
                          CASE WHEN ?4 = '' THEN status END COLLATE NOCASE ASC,
                          CASE WHEN ?4 = '' THEN due_date IS NULL END ASC,
                          CASE WHEN ?4 = '' THEN due_date END ASC,
                          title COLLATE NOCASE ASC,
                          source_task_id ASC
                 LIMIT ?6 OFFSET ?7",
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
                    statuses_json,
                    sort_by.trim(),
                    sort_direction.trim(),
                    i64::from(lookahead_limit),
                    i64::from(offset),
                    priorities_json
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
                "SELECT DISTINCT COALESCE(NULLIF(status, ''), 'Unspecified')
                 FROM notion_task_projections
                 ORDER BY 1 COLLATE NOCASE ASC",
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

        let mut priority_statement = connection
            .prepare(
                "SELECT DISTINCT COALESCE(NULLIF(priority, ''), 'Unspecified') AS choice
                 FROM notion_task_projections
                 ORDER BY CASE lower(choice)
                            WHEN 'high' THEN 0 WHEN 'medium' THEN 1 WHEN 'low' THEN 2
                            WHEN 'unspecified' THEN 4 ELSE 3
                          END,
                          choice COLLATE NOCASE ASC",
            )
            .map_err(|error| {
                StoreError::sqlite("could not prepare Notion task priority filters", error)
            })?;
        let priorities = priority_statement
            .query_map([], |row| row.get(0))
            .map_err(|error| {
                StoreError::sqlite("could not list Notion task priority filters", error)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| {
                StoreError::sqlite("could not read Notion task priority filters", error)
            })?;

        Ok(NotionTaskProjectionPage {
            tasks,
            projects,
            statuses,
            priorities,
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

    /// Latest provider usage for selected sessions, without loading their transcripts.
    pub fn latest_usage_events(&self, owned_ids: &[String]) -> Result<Vec<EventRow>> {
        if owned_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids_json = serde_json::to_string(owned_ids)
            .map_err(|_| StoreError::message("could not encode session ids"))?;
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT e.owned_id, e.seq, e.turn_id, e.kind, e.payload, e.created_at
                 FROM json_each(?1) AS selected
                 JOIN events e ON e.rowid = (
                     SELECT rowid FROM events
                     WHERE owned_id = selected.value AND kind = 'usage.updated'
                       AND item_id IS NULL
                     ORDER BY seq DESC LIMIT 1
                 )",
            )
            .map_err(|error| StoreError::sqlite("could not prepare latest usage", error))?;
        let rows = statement
            .query_map([ids_json], event_from_row)
            .map_err(|error| StoreError::sqlite("could not list latest usage", error))?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|error| StoreError::sqlite("could not read latest usage", error))
    }

    /// Return the summary only when this request's latest approval event is still pending.
    pub fn pending_approval_summary(
        &self,
        owned_id: &str,
        generation: u64,
        request_id: &str,
    ) -> Result<Option<String>> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT CASE WHEN json_extract(payload, '$.payload.state') = 'requested'
                        THEN json_extract(payload, '$.payload.summary') END
                 FROM events
                 WHERE owned_id = ?1 AND kind = 'approval.requested'
                   AND json_extract(payload, '$.generation') = ?2
                   AND json_extract(payload, '$.payload.requestId') = ?3
                 ORDER BY seq DESC LIMIT 1",
                params![owned_id, generation, request_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map(Option::flatten)
            .map_err(|error| StoreError::sqlite("could not read the pending approval", error))
    }

    /// The final assistant text for one turn. Claude can finish with streamed
    /// deltas and no completed assistant-message event.
    pub fn latest_assistant_text_for_turn(
        &self,
        owned_id: &str,
        turn_id: &str,
    ) -> Result<Option<String>> {
        let connection = self.lock()?;
        connection
            .query_row(
                "WITH last_delta AS (
                     SELECT json_extract(payload, '$.payload.itemId') AS item_id
                     FROM events
                     WHERE owned_id = ?1 AND turn_id = ?2
                       AND kind = 'content.delta'
                       AND json_extract(payload, '$.payload.kind') = 'assistantDelta'
                     ORDER BY seq DESC LIMIT 1
                 ), deltas AS (
                     SELECT e.seq, json_extract(e.payload, '$.payload.delta') AS delta
                     FROM events e, last_delta
                     WHERE e.owned_id = ?1 AND e.turn_id = ?2
                       AND e.kind = 'content.delta'
                       AND json_extract(e.payload, '$.payload.kind') = 'assistantDelta'
                       AND json_extract(e.payload, '$.payload.itemId') = last_delta.item_id
                 )
                 SELECT COALESCE(
                     (SELECT json_extract(payload, '$.payload.text')
                      FROM events
                      WHERE owned_id = ?1 AND turn_id = ?2
                        AND json_extract(payload, '$.payload.kind') = 'assistantMessage'
                        AND json_extract(payload, '$.payload.completed') = 1
                      ORDER BY seq DESC LIMIT 1),
                     (SELECT group_concat(delta, '') OVER (
                         ORDER BY seq ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
                      ) FROM deltas LIMIT 1)
                 )",
                params![owned_id, turn_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .map_err(|error| StoreError::sqlite("could not read the assistant text", error))
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
    const HISTORY_PAGE_ROW_CEILING: u32 = 8_000;

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

    /// The window of events just older than `before_seq` and no older than
    /// `lowest_seq`, bounded by bytes.
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
        lowest_seq: i64,
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
                         WHERE owned_id = ? AND seq < ? AND seq >= ?
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
                    lowest_seq,
                    i64::from(Self::HISTORY_PAGE_ROW_CEILING),
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
                "SELECT EXISTS(SELECT 1 FROM events WHERE owned_id = ? AND seq < ? AND seq >= ?)",
                params![owned_id, oldest, lowest_seq],
                |row| row.get(0),
            )
            .map_err(|error| StoreError::sqlite("could not look past the older window", error))?;
        Ok(OlderEvents { events, has_more })
    }

    /// The window of events just newer than `after_seq` and no newer than
    /// `highest_seq`, bounded by bytes.
    pub fn list_events_after(
        &self,
        owned_id: &str,
        after_seq: i64,
        max_bytes: u32,
        highest_seq: i64,
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
                         WHERE owned_id = ? AND seq > ? AND seq <= ?
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
                    highest_seq,
                    i64::from(Self::HISTORY_PAGE_ROW_CEILING),
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
                "SELECT EXISTS(SELECT 1 FROM events WHERE owned_id = ? AND seq > ? AND seq <= ?)",
                params![owned_id, newest, highest_seq],
                |row| row.get(0),
            )
            .map_err(|error| StoreError::sqlite("could not look past the newer window", error))?;
        Ok(OlderEvents { events, has_more })
    }

    pub fn list_items_before(
        &self,
        owned_id: &str,
        before_sequence: i64,
        max_bytes: u32,
        coverage: Option<EventCoverage>,
    ) -> Result<ItemPage> {
        self.query_item_page(owned_id, before_sequence, max_bytes, coverage, true)
    }

    pub fn list_items_after(
        &self,
        owned_id: &str,
        after_sequence: i64,
        max_bytes: u32,
        coverage: Option<EventCoverage>,
    ) -> Result<ItemPage> {
        self.query_item_page(owned_id, after_sequence, max_bytes, coverage, false)
    }

    fn query_item_page(
        &self,
        owned_id: &str,
        cursor: i64,
        max_bytes: u32,
        coverage: Option<EventCoverage>,
        before: bool,
    ) -> Result<ItemPage> {
        let low = coverage.map_or(i64::MIN, |range| range.low);
        let requested_high = coverage.map_or(i64::MAX, |range| range.high);
        if low > requested_high {
            return Err(StoreError::message("the event coverage range is invalid"));
        }
        let connection = self.lock()?;
        self.recent_events_read_active.store(true, Ordering::Release);
        let _active_read = RecentEventsReadGuard(&self.recent_events_read_active);
        let transaction = connection.unchecked_transaction().map_err(|error| {
            StoreError::sqlite("could not begin the item page read", error)
        })?;
        let descriptor_sql = if before {
            std::borrow::Cow::Borrowed(ITEM_PAGE_DESCRIPTOR_SQL)
        } else {
            std::borrow::Cow::Owned(ITEM_PAGE_DESCRIPTOR_SQL.replace(" < :cursor", " > :cursor")
                .replace(" DESC LIMIT :item_ceiling", " ASC LIMIT :item_ceiling")
                .replace("SELECT *,-first_seq AS page_order", "SELECT *,first_seq AS page_order"))
        };
        let mut statement = transaction.prepare(&descriptor_sql).map_err(|error| {
            StoreError::sqlite("could not prepare the item page descriptors", error)
        })?;
        let rows = statement.query_map(
            named_params! {
                ":owned_id": owned_id,
                ":cursor": cursor,
                ":max_bytes": i64::from(max_bytes),
                ":item_ceiling": Self::ITEM_PAGE_CANDIDATE_CEILING,
                ":low": low,
                ":high": requested_high,
            },
            item_page_descriptor_from_row,
        ).map_err(|error| StoreError::sqlite("could not query the item page descriptors", error))?;
        let descriptor_rows = rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| StoreError::sqlite("could not read the item page descriptors", error))?;
        drop(statement);
        let facts = descriptor_rows.first().ok_or_else(|| {
            StoreError::message("the item page descriptor query returned no page facts")
        })?;
        let watermark = facts.watermark;
        let items = descriptor_rows.iter().filter_map(|row| row.item.clone()).collect::<Vec<_>>();
        let descriptor_json = serde_json::to_string(&items)
            .map_err(|_| StoreError::message("could not encode the selected item descriptors"))?;
        let mut record_statement = transaction.prepare(ITEM_PAGE_RECORD_SQL).map_err(|error| {
            StoreError::sqlite("could not prepare the selected item records", error)
        })?;
        let records = record_statement.query_map(
            named_params! {
                ":owned_id": owned_id,
                ":selected_descriptors": descriptor_json,
                ":low": low,
                ":high": watermark,
            },
            event_from_row,
        ).map_err(|error| StoreError::sqlite("could not query the selected item records", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| StoreError::sqlite("could not read the selected item records", error))?;
        drop(record_statement);
        let mut turn_statement = transaction.prepare(ITEM_PAGE_TURN_SQL).map_err(|error| {
            StoreError::sqlite("could not prepare the represented turn facts", error)
        })?;
        let turns = turn_statement.query_map(
            named_params! {
                ":owned_id": owned_id,
                ":selected_descriptors": descriptor_json,
                ":low": low,
                ":high": watermark,
            },
            represented_turn_facts_from_row,
        ).map_err(|error| StoreError::sqlite("could not query the represented turn facts", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| StoreError::sqlite("could not read the represented turn facts", error))?;
        drop(turn_statement);
        transaction.commit()
            .map_err(|error| StoreError::sqlite("could not finish the item page read", error))?;
        Ok(ItemPage {
            oversized: facts.transfer_bytes > u64::from(max_bytes),
            items,
            events: records,
            turns,
            before_cursor: facts.before_cursor,
            after_cursor: facts.after_cursor,
            has_before: facts.has_before,
            has_after: facts.has_after,
            watermark,
            transfer_bytes: facts.transfer_bytes,
        })
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

    /// Fetch only requested attachment ids that belong to this session.
    pub fn get_attachments(&self, owned_id: &str, ids: &[String]) -> Result<Vec<AttachmentRow>> {
        let ids_json = serde_json::to_string(ids)
            .map_err(|_| StoreError::message("could not encode attachment ids"))?;
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT attachments.id, attachments.owned_id, file_name, mime_type,
                    byte_length, relative_path, thumbnail_mime_type,
                    thumbnail_byte_length, thumbnail_relative_path, created_at
                 FROM json_each(?) AS selected
                 CROSS JOIN attachments ON attachments.id = selected.value
                 WHERE attachments.owned_id = ?",
            )
            .map_err(|error| StoreError::sqlite("could not prepare the attachment lookup", error))?;
        let rows = statement
            .query_map(params![ids_json, owned_id], attachment_from_row)
            .map_err(|error| StoreError::sqlite("could not find attachments", error))?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|error| StoreError::sqlite("could not read selected attachments", error))
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

fn insert_chronological_event(connection: &Connection, row: &EventRow) -> Result<()> {
    let is_tool_update = if row.kind == "item.updated" {
        let payload = serde_json::from_str::<serde_json::Value>(&row.payload_json).ok();
        payload
            .as_ref()
            .and_then(|payload| payload.pointer("/payload/kind"))
            .and_then(serde_json::Value::as_str)
            == Some("tool")
    } else {
        false
    };
    let sql = if is_tool_update {
        INSERT_NORMALIZED_TOOL_EVENT
    } else {
        "INSERT INTO events (owned_id, seq, turn_id, kind, payload, created_at)
         VALUES (?, ?, ?, ?, ?, ?)"
    };
    connection
        .execute(
            sql,
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
    Ok(())
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
        .map_err(|error| StoreError::sqlite("could not add the tool item index", error))?;
    connection
        .execute_batch(TOOL_ITEM_SEQUENCE_INDEX_SCHEMA)
        .map_err(|error| StoreError::sqlite("could not add the tool sequence index", error))?;
    connection
        .execute_batch(ITEM_PAGE_INDEX_SCHEMA)
        .map_err(|error| StoreError::sqlite("could not add the item page indexes", error))
}

fn upgrade_tool_events(connection: &Connection) -> Result<()> {
    add_tool_item_schema(connection)?;
    connection
        .execute_batch(NORMALIZE_TOOL_EVENTS)
        .map_err(|error| StoreError::sqlite("could not normalize tool events", error))
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

/// Marks remote cache rows in the ordinary session journal. NULL is local.
fn add_remote_history_column(connection: &Connection) -> Result<()> {
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('sessions') WHERE name = 'cached_remote_profile_id')",
        [], |row| row.get(0),
    ).map_err(|error| StoreError::sqlite("could not inspect remote history marker", error))?;
    if !exists {
        connection.execute_batch("ALTER TABLE sessions ADD COLUMN cached_remote_profile_id TEXT;")
            .map_err(|error| StoreError::sqlite("could not add remote history marker", error))?;
    }
    connection.execute_batch("CREATE INDEX IF NOT EXISTS sessions_cached_remote_idx ON sessions(cached_remote_profile_id);")
        .map_err(|error| StoreError::sqlite("could not index remote history marker", error))
}

fn add_child_session_schema(connection: &Connection) -> Result<()> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('sessions') WHERE name = 'parent_owned_id')",
            [],
            |row| row.get(0),
        )
        .map_err(|error| StoreError::sqlite("could not inspect child session ownership", error))?;
    if !exists {
        connection
            .execute_batch(
                "ALTER TABLE sessions ADD COLUMN parent_owned_id TEXT
                    REFERENCES sessions(owned_id) ON DELETE CASCADE;",
            )
            .map_err(|error| StoreError::sqlite("could not add child session ownership", error))?;
    }
    connection
        .execute_batch(
            "CREATE INDEX IF NOT EXISTS sessions_parent_owned_idx ON sessions(parent_owned_id);",
        )
        .map_err(|error| StoreError::sqlite("could not index child session ownership", error))
}

/// Adds the column that records where a session's name came from, unless the
/// database already has it. Older schemas upgrade directly to the current one.
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

struct ItemPageDescriptorRow {
    item: Option<ItemPageDescriptor>,
    before_cursor: Option<i64>,
    after_cursor: Option<i64>,
    transfer_bytes: u64,
    watermark: i64,
    has_before: bool,
    has_after: bool,
}

fn item_page_descriptor_from_row(row: &Row<'_>) -> rusqlite::Result<ItemPageDescriptorRow> {
    let item_id = row.get::<_, Option<String>>(0)?;
    let item = match item_id {
        Some(item_id) => Some(ItemPageDescriptor {
            item_id,
            stable_id: row.get(1)?,
            record_mode: row.get(2)?,
            first_sequence: row.get(3)?,
            first_timestamp_ms: row.get(4)?,
            last_sequence: row.get(5)?,
            authority_sequence: row.get(6)?,
            turn_id: row.get(7)?,
            completed: row.get(8)?,
            required_bytes: row.get(9)?,
        }),
        None => None,
    };
    Ok(ItemPageDescriptorRow {
        item,
        before_cursor: row.get(10)?,
        after_cursor: row.get(11)?,
        transfer_bytes: row.get(12)?,
        watermark: row.get(13)?,
        has_before: row.get(14)?,
        has_after: row.get(15)?,
    })
}

fn represented_turn_facts_from_row(row: &Row<'_>) -> rusqlite::Result<RepresentedTurnFacts> {
    Ok(RepresentedTurnFacts {
        turn_id: row.get(0)?,
        started_at_ms: row.get(1)?,
        ended_at_ms: row.get(2)?,
        terminal_state: row.get(3)?,
        final_assistant_item_id: row.get(4)?,
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
        EventCoverage, EventRow, EvidenceArtifact, EvidenceArtifactQuery, EvidenceDiskUsage,
        EvidenceRunDiskUsage, NotionTaskProjection, RepresentedTurnFacts, SessionRow, SessionStore,
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

    fn item_page_event(
        owned_id: &str,
        seq: i64,
        turn_id: Option<&str>,
        kind: &str,
        generation: i64,
        payload: serde_json::Value,
    ) -> EventRow {
        EventRow {
            owned_id: owned_id.to_owned(),
            seq,
            turn_id: turn_id.map(str::to_owned),
            kind: kind.to_owned(),
            payload_json: serde_json::json!({
                "ownedId": owned_id,
                "generation": generation,
                "sequence": seq,
                "timestampMs": seq * 10,
                "turnId": turn_id,
                "payload": payload,
            }).to_string(),
            created_at_ms: seq * 10,
        }
    }

    #[test]
    fn complete_item_page_keeps_authority_position_exact_records_and_turn_facts() {
        let store = SessionStore::open_in_memory().unwrap();
        store.upsert_session(&fixture_session("page", 0)).unwrap();
        for event in [
            item_page_event("page", 0, Some("turn-a"), "turn.started", 1,
                serde_json::json!({"kind":"turn","turnId":"turn-a","state":"started"})),
            item_page_event("page", 1, Some("turn-a"), "content.delta", 1,
                serde_json::json!({"kind":"assistantDelta","itemId":"answer","delta":"old"})),
            item_page_event("page", 2, Some("turn-a"), "item.completed", 1,
                serde_json::json!({"kind":"assistantMessage","itemId":"answer","text":"full","completed":true,"blocks":[]})),
            item_page_event("page", 3, Some("turn-a"), "content.delta", 1,
                serde_json::json!({"kind":"assistantDelta","itemId":"answer","delta":" tail"})),
            item_page_event("page", 4, Some("turn-a"), "turn.completed", 1,
                serde_json::json!({"kind":"turn","turnId":"turn-a","state":"completed"})),
        ] {
            store.append_event(&event).unwrap();
        }

        let page = store.list_items_before("page", i64::MAX, 512 * 1024, None).unwrap();
        assert_eq!(page.watermark, 4);
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].stable_id, "assistant:answer");
        assert_eq!(page.items[0].first_sequence, 1);
        assert_eq!(page.items[0].first_timestamp_ms, 10);
        assert_eq!(page.items[0].authority_sequence, Some(2));
        assert!(!page.items[0].completed, "a later delta reopens the authority");
        assert_eq!(page.events.iter().map(|event| event.seq).collect::<Vec<_>>(), [2, 3]);
        assert_eq!(page.turns, [RepresentedTurnFacts {
            turn_id: "turn-a".into(), started_at_ms: Some(0), ended_at_ms: Some(40),
            terminal_state: Some("completed".into()), final_assistant_item_id: Some("answer".into()),
        }]);
    }

    #[test]
    fn item_pages_keep_logical_identities_and_directional_cursors() {
        let store = SessionStore::open_in_memory().unwrap();
        store.upsert_session(&fixture_session("logical", 0)).unwrap();
        for event in [
            item_page_event("logical", 1, Some("turn-a"), "commands.updated", 1,
                serde_json::json!({"kind":"availableCommandsUpdate","availableCommands":[]})),
            item_page_event("logical", 2, Some("turn-a"), "plan.updated", 1,
                serde_json::json!({"kind":"plan","items":[]})),
            item_page_event("logical", 3, Some("turn-a"), "commands.updated", 1,
                serde_json::json!({"kind":"availableCommandsUpdate","availableCommands":[{"name":"x"}]})),
            item_page_event("logical", 4, Some("turn-b"), "commands.updated", 2,
                serde_json::json!({"kind":"availableCommandsUpdate","availableCommands":[]})),
            item_page_event("logical", 5, Some("turn-b"), "item.completed", 2,
                serde_json::json!({"kind":"terminalProjection","eventType":"item.completed","payload":{"historical":true}})),
        ] {
            store.append_event(&event).unwrap();
        }

        let page = store.list_items_before("logical", i64::MAX, 512 * 1024, None).unwrap();
        assert_eq!(page.items.iter().map(|item| item.stable_id.as_str()).collect::<Vec<_>>(),
            ["commands:1", "event:2", "commands:2", "event:5"]);
        assert_eq!(page.items[0].first_sequence, 1);
        assert_eq!(page.items[0].authority_sequence, Some(3));
        assert_eq!(page.events.iter().map(|event| event.seq).collect::<Vec<_>>(), [2, 3, 4, 5]);
        let newer = store.list_items_after("logical", 2, 512 * 1024, None).unwrap();
        assert_eq!(newer.items.iter().map(|item| item.stable_id.as_str()).collect::<Vec<_>>(),
            ["commands:2", "event:5"]);
        assert!(newer.has_before);
        assert!(!newer.has_after);
    }

    #[test]
    fn item_page_uses_the_confirmed_closed_coverage_for_every_read() {
        let store = SessionStore::open_in_memory().unwrap();
        store.upsert_session(&fixture_session("covered", 0)).unwrap();
        for seq in 1..=5 {
            store.append_event(&item_page_event("covered", seq, Some("turn-a"), "plan.updated", 1,
                serde_json::json!({"kind":"plan","items":[{"content":seq}]}))).unwrap();
        }
        let page = store.list_items_before("covered", i64::MAX, 1, Some(EventCoverage { low: 2, high: 4 })).unwrap();
        assert_eq!(page.watermark, 4);
        assert_eq!(page.items.iter().map(|item| item.first_sequence).collect::<Vec<_>>(), [4]);
        assert_eq!(page.events.iter().map(|event| event.seq).collect::<Vec<_>>(), [4]);
        assert!(page.oversized);
        assert!(page.has_before);
        assert!(!page.has_after);
    }

    #[test]
    fn remote_cache_is_excluded_from_local_startup_and_purged_by_source() {
        let (_directory, path, store) = open_temp_store();
        store.upsert_session(&fixture_session("local", 1)).unwrap();
        store.set_draft("local", "keep my draft").unwrap();
        for (profile, key) in [("workbox", "cache-a"), ("other", "cache-b")] {
            let row = fixture_session(key, 2);
            let events = [fixture_event(key, -1), fixture_event(key, 1)];
            store.cache_remote_events(profile, &row, &events).unwrap();
            store.cache_remote_events(profile, &row, &events).unwrap();
            assert_eq!(store.list_events_before(key, 2, 1024, i64::MIN).unwrap().events.len(), 2);
        }
        assert_eq!(store.count_sessions().unwrap(), 1);
        assert_eq!(store.list_sessions().unwrap()[0].owned_id, "local");
        drop(store);
        let store = SessionStore::open(&path).unwrap();
        store.purge_remote_history("workbox").unwrap();
        assert!(store.get_session("cache-a").unwrap().is_none());
        assert!(store.list_events("cache-a", -10, 100).unwrap().is_empty());
        assert!(store.get_session("cache-b").unwrap().is_some());
        assert_eq!(store.get_draft("local").unwrap().as_deref(), Some("keep my draft"));
        assert_eq!(store.count_sessions().unwrap(), 1);
    }

    #[test]
    fn remote_cache_cannot_replace_a_local_session_or_another_source() {
        let store = SessionStore::open_in_memory().unwrap();
        let local = fixture_session("collision", 1);
        store.upsert_session(&local).unwrap();
        assert!(store.cache_remote_events("workbox", &local, &[]).is_err());
        let remote = fixture_session("remote", 2);
        store.cache_remote_events("workbox", &remote, &[]).unwrap();
        assert!(store.cache_remote_events("other", &remote, &[]).is_err());
        store.purge_remote_history("workbox").unwrap();
        assert!(store.get_session("collision").unwrap().is_some());
    }

    #[test]
    fn child_session_is_hidden_from_roots_and_cascades_with_its_parent() {
        let store = SessionStore::open_in_memory().unwrap();
        let parent = fixture_session("parent", 1);
        let child = fixture_session("child", 2);
        store.upsert_session(&parent).unwrap();
        store.upsert_child_session("parent", None, &child).unwrap();
        store.append_event(&fixture_event("child", 1)).unwrap();

        assert_eq!(store.count_sessions().unwrap(), 1);
        assert_eq!(store.list_sessions().unwrap()[0].owned_id, "parent");
        assert!(store.get_session("child").unwrap().is_some());

        store.delete_session("parent").unwrap();
        assert!(store.get_session("child").unwrap().is_none());
        assert!(store.list_events("child", i64::MIN, 100).unwrap().is_empty());
    }

    #[test]
    fn child_session_requires_unchanged_parent_and_remote_source() {
        let store = SessionStore::open_in_memory().unwrap();
        let parent = fixture_session("remote-parent", 1);
        let other_parent = fixture_session("other-parent", 1);
        store.cache_remote_events("workbox", &parent, &[]).unwrap();
        store.cache_remote_events("workbox", &other_parent, &[]).unwrap();
        let child = fixture_session("remote-child", 2);

        assert!(store.upsert_child_session("remote-parent", None, &child).is_err());
        assert!(store.upsert_child_session("remote-parent", Some("other"), &child).is_err());
        store.upsert_child_session("remote-parent", Some("workbox"), &child).unwrap();
        store.cache_remote_events("workbox", &child, &[fixture_event("remote-child", 1)]).unwrap();
        assert!(store.upsert_child_session("other-parent", Some("workbox"), &child).is_err());

        let local_root = fixture_session("local-root", 3);
        store.upsert_session(&local_root).unwrap();
        assert!(store.upsert_child_session("remote-parent", Some("workbox"), &local_root).is_err());

        let connection = store.connection.lock().unwrap();
        let ownership: (Option<String>, Option<String>) = connection.query_row(
            "SELECT parent_owned_id, cached_remote_profile_id FROM sessions WHERE owned_id = ?",
            ["remote-child"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        assert_eq!(ownership, (Some("remote-parent".into()), Some("workbox".into())));
    }

    #[test]
    fn schema_v17_adds_child_session_ownership() {
        let (_directory, path, store) = open_temp_store();
        store.upsert_session(&fixture_session("existing", 1)).unwrap();
        drop(store);

        let connection = Connection::open(&path).unwrap();
        connection.execute_batch(
            "DROP INDEX sessions_parent_owned_idx;
             ALTER TABLE sessions DROP COLUMN parent_owned_id;
             PRAGMA user_version = 16;",
        ).unwrap();
        drop(connection);

        let store = SessionStore::open(&path).unwrap();
        assert!(store.get_session("existing").unwrap().is_some());
        drop(store);
        let connection = Connection::open(&path).unwrap();
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
        let column_exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('sessions') WHERE name = 'parent_owned_id')",
            [], |row| row.get(0),
        ).unwrap();
        let index_exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master
                WHERE type = 'index' AND name = 'sessions_parent_owned_idx')",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(version, super::SCHEMA_VERSION);
        assert!(column_exists);
        assert!(index_exists);
    }

    #[test]
    fn remote_cache_marker_upgrade_preserves_version_thirteen_history() {
        let (_directory, path, store) = open_temp_store();
        store.upsert_session(&fixture_session("local", 1)).unwrap();
        store.set_draft("local", "existing").unwrap();
        drop(store);
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch("DROP INDEX sessions_cached_remote_idx;
            ALTER TABLE sessions DROP COLUMN cached_remote_profile_id;
            PRAGMA user_version = 13;").unwrap();
        drop(connection);
        let store = SessionStore::open(&path).unwrap();
        assert_eq!(store.count_sessions().unwrap(), 1);
        assert_eq!(store.get_draft("local").unwrap().as_deref(), Some("existing"));
        store.cache_remote_events("workbox", &fixture_session("remote", 2), &[]).unwrap();
        store.purge_remote_history("workbox").unwrap();
        assert_eq!(store.count_sessions().unwrap(), 1);
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
    fn chronological_tool_completion_keeps_sparse_fields_and_actual_position() {
        let store = SessionStore::open_in_memory().expect("open store");
        let session = fixture_session("session-a", 10_000);
        let started = EventRow {
            payload_json: r#"{"sequence":999,"timestampMs":999,"payload":{"kind":"tool","itemId":"tool-a","name":"shell","state":"running","output":"partial","path":"/tmp/a"}}"#.into(),
            ..fixture_event_of_kind("session-a", 7, "item.updated")
        };
        store
            .upsert_session_with_event(&session, Some(&started))
            .expect("write tool start");
        let completed = EventRow {
            payload_json: r#"{"sequence":998,"timestampMs":998,"payload":{"kind":"tool","itemId":"tool-a","state":"completed","summary":"done"}}"#.into(),
            created_at_ms: 20_009,
            ..fixture_event_of_kind("session-a", 9, "item.updated")
        };
        store
            .upsert_session_with_event(&session, Some(&completed))
            .expect("write sparse tool completion");

        let events = store
            .list_events("session-a", i64::MIN, 10)
            .expect("read tool history");
        let payload: serde_json::Value = serde_json::from_str(&events[1].payload_json).expect("parse completion");
        assert_eq!(events.iter().map(|event| event.seq).collect::<Vec<_>>(), [7, 9]);
        assert_eq!(payload["sequence"], 9);
        assert_eq!(payload["timestampMs"], 20_009);
        assert_eq!(payload["payload"]["name"], "shell");
        assert_eq!(payload["payload"]["state"], "completed");
        assert_eq!(payload["payload"]["summary"], "done");
        assert_eq!(payload["payload"]["output"], "partial");
        assert_eq!(payload["payload"]["path"], "/tmp/a");
        assert_eq!(payload["payload"]["firstSequence"], 7);
        assert_eq!(payload["payload"]["firstTimestampMs"], started.created_at_ms);
        assert!(payload["payload"].get("input").is_none());
    }

    #[test]
    fn historical_prepend_stays_raw_until_a_native_tool_update_merges_the_item() {
        let store = SessionStore::open_in_memory().expect("open store");
        let session = fixture_session("session-a", 10_000);
        let imported_output = EventRow {
            payload_json: r#"{"payload":{"kind":"tool","itemId":"tool-a","state":"running","output":"result"}}"#.into(),
            created_at_ms: 20_020,
            ..fixture_event_of_kind("session-a", 20, "item.completed")
        };
        store.upsert_session(&session).expect("write session");
        store.append_event(&imported_output).expect("import output tail");
        let earlier_call = EventRow {
            payload_json: r#"{"payload":{"kind":"tool","itemId":"tool-a","name":"shell","state":"running"}}"#.into(),
            created_at_ms: 20_010,
            ..fixture_event_of_kind("session-a", 10, "item.completed")
        };
        store.append_event(&earlier_call).expect("prepend earlier call");

        let before = store
            .list_events("session-a", i64::MIN, 10)
            .expect("read imported history");
        let raw_call: serde_json::Value =
            serde_json::from_str(&before[0].payload_json).expect("parse raw call");
        assert!(raw_call["payload"].get("output").is_none());
        assert!(raw_call["payload"].get("firstSequence").is_none());

        let completed = EventRow {
            payload_json: r#"{"payload":{"kind":"tool","itemId":"tool-a","state":"completed","summary":"done"}}"#.into(),
            created_at_ms: 20_030,
            ..fixture_event_of_kind("session-a", 30, "item.updated")
        };
        store
            .upsert_session_with_event_and_clear_workspace(&session, Some(&completed))
            .expect("write native completion");

        let events = store
            .list_events("session-a", i64::MIN, 10)
            .expect("read merged history");
        let merged: serde_json::Value = serde_json::from_str(&events[2].payload_json).expect("parse merged completion");
        assert_eq!(events.iter().map(|event| event.seq).collect::<Vec<_>>(), [10, 20, 30]);
        assert_eq!(merged["payload"]["name"], "shell");
        assert_eq!(merged["payload"]["output"], "result");
        assert_eq!(merged["payload"]["firstSequence"], 10);
        assert_eq!(merged["payload"]["firstTimestampMs"], 20_010);
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
    fn schema_v14_open_repairs_latest_tools_without_compacting_history() {
        let (_directory, path, store) = open_temp_store();
        store
            .upsert_session(&fixture_session("session-a", 10_000))
            .expect("write session");
        drop(store);
        let connection = Connection::open(&path).expect("open version fourteen database");
        for (seq, kind, payload) in [
            (1, "item.updated", r#"{"payload":{"kind":"tool","itemId":"tool-a","name":"shell","state":"running","output":"partial"}}"#),
            (2, "item.updated", r#"{"payload":{"kind":"tool","itemId":"tool-a","state":"completed"}}"#),
            (3, "item.completed", r#"{"payload":{"kind":"tool","itemId":"tool-a","state":"completed","output":"later import"}}"#),
        ] {
            connection
                .execute(
                    "INSERT INTO events (owned_id, seq, turn_id, kind, payload, created_at)
                     VALUES ('session-a', ?, NULL, ?, ?, ?)",
                    rusqlite::params![seq, kind, payload, 10_000 + seq],
                )
                .expect("insert legacy tool event");
        }
        connection
            .execute_batch(
                "DROP INDEX events_owned_item_seq_idx;
                 PRAGMA user_version = 14;",
            )
            .expect("restore version fourteen schema");
        drop(connection);

        let store = SessionStore::open(&path).expect("upgrade database");
        let events = store
            .list_events("session-a", i64::MIN, 10)
            .expect("read repaired tools");
        let latest: serde_json::Value = serde_json::from_str(&events[1].payload_json).expect("parse repaired tool");
        let imported: serde_json::Value =
            serde_json::from_str(&events[2].payload_json).expect("parse imported tool");
        assert_eq!(events.iter().map(|event| event.seq).collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(latest["payload"]["name"], "shell");
        assert_eq!(latest["payload"]["output"], "partial");
        assert_eq!(latest["payload"]["firstSequence"], 1);
        assert_eq!(latest["payload"]["firstTimestampMs"], 10_001);
        assert_eq!(imported["payload"]["output"], "later import");
        assert!(imported["payload"].get("firstSequence").is_none());
        let version: i64 = store.lock().expect("lock store").pragma_query_value(None, "user_version", |row| row.get(0)).expect("read schema version");
        assert_eq!(version, super::SCHEMA_VERSION);
    }

    #[test]
    fn an_upgraded_database_loses_only_the_duplicates() {
        let directory = TempDir::new().expect("create temporary directory");
        let path = directory.path().join("sessions.db");
        let connection = Connection::open(&path).expect("create version five database");
        connection.execute_batch("CREATE TABLE sessions (owned_id TEXT PRIMARY KEY);").unwrap();
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
        for (seq, payload) in [
            (1, r#"{"payload":{"kind":"tool","itemId":"tool-a","name":"shell","state":"running","output":"partial"}}"#),
            (2, r#"{"payload":{"kind":"tool","itemId":"tool-a","state":"running","summary":"working"}}"#),
            (3, r#"{"payload":{"kind":"tool","itemId":"tool-a","state":"completed"}}"#),
        ] {
            connection
                .execute(
                    "INSERT INTO events VALUES (?, ?, NULL, 'item.updated', ?, ?)",
                    rusqlite::params!["session-a", seq, payload, 10_000 + seq],
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
        let tool: serde_json::Value =
            serde_json::from_str(&events[0].payload_json).expect("parse compacted tool");
        assert_eq!(tool["payload"]["name"], "shell");
        assert_eq!(tool["payload"]["state"], "completed");
        assert_eq!(tool["payload"]["summary"], "working");
        assert_eq!(tool["payload"]["output"], "partial");
        assert_eq!(tool["payload"]["firstSequence"], 1);
        assert_eq!(tool["payload"]["firstTimestampMs"], 10_001);
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
        connection.execute_batch("CREATE TABLE sessions (owned_id TEXT PRIMARY KEY); CREATE TABLE events (owned_id TEXT NOT NULL, seq INTEGER NOT NULL, turn_id TEXT, kind TEXT NOT NULL, payload TEXT NOT NULL, created_at INTEGER NOT NULL, PRIMARY KEY (owned_id, seq));").unwrap();
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
        connection.execute_batch("CREATE TABLE sessions (owned_id TEXT PRIMARY KEY); CREATE TABLE events (owned_id TEXT NOT NULL, seq INTEGER NOT NULL, turn_id TEXT, kind TEXT NOT NULL, payload TEXT NOT NULL, created_at INTEGER NOT NULL, PRIMARY KEY (owned_id, seq));").unwrap();
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
                .query_notion_task_projections(0, 10, "", "", &[], &[], "", "")
                .expect("read Notion task snapshot"),
            super::NotionTaskProjectionPage {
                tasks: second.to_vec(),
                projects: vec!["Rental Command".to_string()],
                statuses: vec!["Doing".to_string()],
                priorities: vec!["High".to_string()],
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
                .query_notion_task_projections(0, 10, "", "", &[], &[], "", "")
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
            .query_notion_task_projections(0, 2, "", "", &[], &[], "", "")
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
            .query_notion_task_projections(2, 2, "", "", &[], &[], "", "")
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
    fn notion_task_projection_sorts_by_task_number_or_title() {
        let (_directory, _path, store) = open_temp_store();
        let tasks = [
            fixture_notion_task("task-1116", "Assembly", "Doing", "[TSK-1116] Alpha", None),
            fixture_notion_task("task-2", "Assembly", "Doing", "[TSK-2] Zulu", None),
            fixture_notion_task("task-997", "Assembly", "Doing", "997 pipeline gap", None),
            fixture_notion_task("task-new", "Assembly", "Doing", "[TSK-NEW] Beta", None),
        ];
        store
            .replace_notion_task_projections(&tasks)
            .expect("write sortable Notion task snapshot");

        let by_number = store
            .query_notion_task_projections(0, 10, "", "", &[], &[], "taskNumber", "asc")
            .expect("sort Notion tasks by task number");
        assert_eq!(
            by_number
                .tasks
                .iter()
                .map(|task| task.source_task_id.as_str())
                .collect::<Vec<_>>(),
            ["task-2", "task-997", "task-1116", "task-new"]
        );

        let by_title = store
            .query_notion_task_projections(0, 10, "", "", &[], &[], "title", "asc")
            .expect("sort Notion tasks by title");
        assert_eq!(
            by_title
                .tasks
                .iter()
                .map(|task| task.source_task_id.as_str())
                .collect::<Vec<_>>(),
            ["task-1116", "task-new", "task-997", "task-2"]
        );

        let number_descending = store
            .query_notion_task_projections(0, 10, "", "", &[], &[], "taskNumber", "desc")
            .expect("sort Notion tasks by descending task number");
        assert_eq!(
            number_descending
                .tasks
                .iter()
                .map(|task| task.source_task_id.as_str())
                .collect::<Vec<_>>(),
            ["task-1116", "task-997", "task-2", "task-new"]
        );

        let title_descending = store
            .query_notion_task_projections(0, 10, "", "", &[], &[], "title", "desc")
            .expect("sort Notion tasks by descending title");
        assert_eq!(
            title_descending
                .tasks
                .iter()
                .map(|task| task.source_task_id.as_str())
                .collect::<Vec<_>>(),
            ["task-2", "task-997", "task-new", "task-1116"]
        );
    }

    #[test]
    fn notion_task_projection_filters_run_in_sql() {
        let (_directory, _path, store) = open_temp_store();
        let with_priority = |mut task: NotionTaskProjection, priority: Option<&str>| {
            task.priority = priority.map(str::to_owned);
            task
        };
        let tasks = [
            fixture_notion_task("task-1", "Assembly", "Doing", "Workbench", None),
            fixture_notion_task("task-2", "Rental Command", "Todo", "Billing", None),
            with_priority(
                fixture_notion_task("task-3", "Assembly", "Todo", "Work queue", None),
                Some("Low"),
            ),
            with_priority(
                fixture_notion_task("task-4", "Assembly", "Done", "Workflow", None),
                Some("Medium"),
            ),
            with_priority(
                fixture_notion_task("task-5", "Assembly", "Doing", "Work notes", None),
                None,
            ),
        ];
        store
            .replace_notion_task_projections(&tasks)
            .expect("write Notion task snapshot");

        let page = store
            .query_notion_task_projections(0, 10, "work", "Assembly", &[], &[], "title", "asc")
            .expect("filter Notion task snapshot by search and project");
        assert_eq!(page.tasks.len(), 4);
        assert_eq!(page.projects, ["Assembly", "Rental Command"]);
        assert_eq!(page.statuses, ["Doing", "Done", "Todo"]);
        assert_eq!(page.priorities, ["High", "Medium", "Low", "Unspecified"]);

        let statuses = ["Doing".to_owned(), "Todo".to_owned()];
        let page = store
            .query_notion_task_projections(0, 10, "", "Assembly", &statuses, &[], "title", "asc")
            .expect("filter Notion task snapshot by several statuses");
        assert_eq!(
            page.tasks
                .iter()
                .map(|task| task.source_task_id.as_str())
                .collect::<Vec<_>>(),
            ["task-5", "task-3", "task-1"]
        );

        let priorities = ["High".to_owned(), "Low".to_owned()];
        let page = store
            .query_notion_task_projections(0, 10, "", "", &statuses, &priorities, "title", "asc")
            .expect("filter Notion task snapshot by statuses and priorities");
        assert_eq!(
            page.tasks
                .iter()
                .map(|task| task.source_task_id.as_str())
                .collect::<Vec<_>>(),
            ["task-2", "task-3", "task-1"]
        );
    }

    #[test]
    fn notion_task_projection_keeps_missing_status_explicit() {
        let (_directory, _path, store) = open_temp_store();
        let task = fixture_notion_task("task-1", "Assembly", "", "Missing status", None);
        store
            .replace_notion_task_projections(&[task.clone()])
            .expect("write Notion task snapshot");

        let page = store
            .query_notion_task_projections(0, 10, "", "", &["Unspecified".to_owned()], &[], "", "")
            .expect("filter missing status");
        assert_eq!(page.tasks, [task]);
        assert_eq!(page.statuses, ["Unspecified"]);
    }

    #[test]
    fn notion_task_projection_keeps_missing_priority_explicit() {
        let (_directory, _path, store) = open_temp_store();
        let mut blank = fixture_notion_task("task-blank", "Assembly", "Doing", "Blank", None);
        blank.priority = Some(String::new());
        let mut missing = fixture_notion_task("task-missing", "Assembly", "Doing", "Missing", None);
        missing.priority = None;
        let high = fixture_notion_task("task-high", "Assembly", "Doing", "High", None);
        store
            .replace_notion_task_projections(&[blank, missing, high])
            .expect("write Notion task snapshot");

        let unspecified = ["Unspecified".to_owned()];
        let page = store
            .query_notion_task_projections(0, 10, "", "", &[], &unspecified, "title", "asc")
            .expect("filter missing priority");
        assert_eq!(
            page.tasks
                .iter()
                .map(|task| task.source_task_id.as_str())
                .collect::<Vec<_>>(),
            ["task-blank", "task-missing"]
        );
        assert_eq!(page.priorities, ["High", "Unspecified"]);

        let every = ["High".to_owned(), "Unspecified".to_owned()];
        let page = store
            .query_notion_task_projections(0, 10, "", "", &[], &every, "title", "asc")
            .expect("filter every priority");
        assert_eq!(page.tasks.len(), 3);
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
            .query_notion_task_projections(0, 10, "", "", &[], &[], "", "")
            .expect("read default Notion task page");
        assert_eq!(default_page.tasks, tasks[..1]);

        let future_page = store
            .query_notion_task_projections(0, 10, "", "", &["Future".to_owned()], &[], "", "")
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
    fn latest_usage_query_returns_one_event_per_requested_session() {
        let (_directory, _path, store) = open_temp_store();
        for owned_id in ["session-a", "session-b"] {
            store.upsert_session(&fixture_session(owned_id, 10_000)).unwrap();
        }
        for (owned_id, seq, kind) in [
            ("session-a", 1, "usage.updated"),
            ("session-a", 2, "usage.updated"),
            ("session-a", 3, "content.delta"),
            ("session-b", 1, "usage.updated"),
        ] {
            store.append_event(&fixture_event_of_kind(owned_id, seq, kind)).unwrap();
        }
        let mut rows = store.latest_usage_events(&[
            "session-b".into(), "missing".into(), "session-a".into(),
        ]).unwrap();
        rows.sort_by(|left, right| left.owned_id.cmp(&right.owned_id));
        assert_eq!(rows.iter().map(|row| (row.owned_id.as_str(), row.seq)).collect::<Vec<_>>(),
            [("session-a", 2), ("session-b", 1)]);
        assert!(store.latest_usage_events(&[]).unwrap().is_empty());
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
        let page_indexes: i64 = connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name IN
             ('events_command_generation_idx','events_turn_item_seq_idx','events_assistant_page_idx',
              'events_user_command_page_idx','events_replay_page_idx','events_side_page_idx','events_tool_position_idx')",
            [], |row| row.get(0),
        ).expect("read item page indexes");
        assert_eq!(page_indexes, 7);

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

    #[test]
    fn completed_assistant_text_is_selected_by_turn_in_sql() {
        let store = SessionStore::open_in_memory().expect("open store");
        store
            .upsert_session(&fixture_session("workflow-agent", 1_000))
            .expect("insert session");
        for (seq, turn_id, completed, text) in [
            (1, "turn-a", true, "old"),
            (2, "turn-b", false, "partial"),
            (3, "turn-b", true, "receipt"),
        ] {
            store
                .append_event(&EventRow {
                    owned_id: "workflow-agent".into(),
                    seq,
                    turn_id: Some(turn_id.into()),
                    kind: "item.completed".into(),
                    payload_json: format!(
                        r#"{{"payload":{{"kind":"assistantMessage","itemId":"a","text":"{text}","completed":{completed}}}}}"#
                    ),
                    created_at_ms: 1_000 + seq,
                })
                .expect("append event");
        }

        assert_eq!(
            store
                .latest_assistant_text_for_turn("workflow-agent", "turn-b")
                .expect("read receipt")
                .as_deref(),
            Some("receipt")
        );
    }

    #[test]
    fn streamed_assistant_text_is_assembled_by_turn_and_item_in_sql() {
        let store = SessionStore::open_in_memory().expect("open store");
        store
            .upsert_session(&fixture_session("workflow-agent", 1_000))
            .expect("insert session");
        for (seq, turn_id, item_id, delta) in [
            (1, "turn-a", "old", "ignore"),
            (2, "turn-b", "first", "ignore"),
            (3, "turn-b", "last", "{\"ordered"),
            (4, "turn-b", "last", "Steps\":[]}"),
        ] {
            store
                .append_event(&EventRow {
                    owned_id: "workflow-agent".into(),
                    seq,
                    turn_id: Some(turn_id.into()),
                    kind: "content.delta".into(),
                    payload_json: serde_json::json!({
                        "payload": {"kind": "assistantDelta", "itemId": item_id, "delta": delta}
                    })
                    .to_string(),
                    created_at_ms: 1_000 + seq,
                })
                .expect("append event");
        }
        assert_eq!(
            store
                .latest_assistant_text_for_turn("workflow-agent", "turn-b")
                .expect("read streamed receipt")
                .as_deref(),
            Some("{\"orderedSteps\":[]}")
        );
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
            .list_events_before(&session.owned_id, 400, 1_024, i64::MIN)
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
            .list_events_before(&session.owned_id, 1, 1_024, i64::MIN)
            .expect("list the page before the first event");
        assert!(start.events.is_empty());
        assert!(!start.has_more);
    }

    #[test]
    fn history_page_can_exceed_the_recent_snapshot_row_limit() {
        let (_directory, _path, store) = open_temp_store();
        let session = fixture_session("owned-long-history", 2_000);
        store.upsert_session(&session).expect("insert session");
        for seq in 1..=2_100 {
            store
                .append_event(&fixture_event(&session.owned_id, seq))
                .expect("append event");
        }

        let page = store
            .list_events_before(&session.owned_id, 2_101, 4 * 1024 * 1024, i64::MIN)
            .expect("read the larger history page");
        assert_eq!(page.events.len(), 2_100);
        assert!(!page.has_more);
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
            .list_events_before("owned-left", 8, FIXTURE_PAYLOAD_BYTES * 4, i64::MIN)
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

        // Editor checkpoints must never overwrite tree state, in either order.
        let stale = r#"{"tabs":[],"expandedPathsByRoot":{"/test/project":["/test/project/old"]}}"#;
        store
            .upsert_workspace_snapshot("session-tree-1", stale)
            .unwrap();
        assert_eq!(
            store.get_workspace_expanded_paths("session-tree-1", "/test/project").unwrap(),
            paths_a
        );
        store
            .set_workspace_expanded_paths("session-tree-1", "/test/project", &[])
            .unwrap();
        store
            .upsert_workspace_snapshot("session-tree-1", stale)
            .unwrap();
        drop(store);
        let reopened = SessionStore::open(&dir.path().join("sessions.db")).unwrap();
        assert!(reopened
            .get_workspace_expanded_paths("session-tree-1", "/test/project")
            .unwrap()
            .is_empty());
        assert_eq!(
            reopened.get_workspace_expanded_paths("session-tree-1", "/other/repo").unwrap(),
            paths_b
        );
        let snapshot: serde_json::Value = serde_json::from_str(
            &reopened.get_workspace_snapshot("session-tree-1").unwrap().unwrap(),
        ).unwrap();
        assert_eq!(snapshot["tabs"], serde_json::json!([]));
    }
}
