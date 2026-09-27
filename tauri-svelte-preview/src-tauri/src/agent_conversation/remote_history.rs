//! Remote history uses the local SQLite event journal and paging code.
//! An explicit source column keeps cached sessions out of local runtime startup.
use super::protocol::{
    AgentConversationConnection, AgentConversationEvent, AgentConversationEventPage,
    AgentConversationSnapshot,
};
use mcb_core::session_store::{EventRow, SessionRow, SessionStore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Coverage {
    remote_profile_id: String,
    connection: Option<AgentConversationConnection>,
    suspended: bool,
    oldest: Option<i64>,
    through: Option<i64>,
    start_complete: bool,
}

#[cfg(test)]
mod tests {
    use super::super::protocol::{
        AgentConversationPayload, AgentConversationProvider, ConversationConnectionState,
    };
    use super::super::safe_markdown::parse_safe_markdown;
    use super::*;

    fn event(sequence: i64) -> AgentConversationEvent {
        AgentConversationEvent {
            owned_id: "same-session-id".into(),
            provider: AgentConversationProvider::Codex,
            generation: 1,
            sequence,
            timestamp_ms: 123,
            turn_id: Some("turn".into()),
            payload: AgentConversationPayload::AssistantMessage {
                item_id: format!("message-{sequence}"),
                text: "**prepared**".into(),
                completed: true,
                blocks: Some(parse_safe_markdown("**prepared**")),
            },
        }
    }

    fn snapshot(start: i64, end: i64) -> AgentConversationSnapshot {
        AgentConversationSnapshot {
            connection: AgentConversationConnection {
                owned_id: "same-session-id".into(),
                provider: AgentConversationProvider::Codex,
                generation: 1,
                native_session_id: Some("native".into()),
                state: ConversationConnectionState::Disconnected,
                config: Default::default(),
            },
            suspended: true,
            last_sequence: end,
            events: (start..=end).map(event).collect(),
        }
    }

    #[test]
    fn remote_history_reopens_with_prepared_markdown_and_isolated_sources() {
        let directory =
            std::env::temp_dir().join(format!("assembly-history-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("sessions.db");
        let store = Arc::new(SessionStore::open(&path).unwrap());
        let history = RemoteHistory::new(store.clone());
        let expected = snapshot(-2, 5);
        history.snapshot("a", 0, None, &expected).unwrap();
        history.snapshot("a", 0, None, &expected).unwrap();
        history.snapshot("b", 0, None, &snapshot(20, 22)).unwrap();
        assert_eq!(store.count_sessions().unwrap(), 0);
        drop(history);
        drop(store);
        let mut history = RemoteHistory::new(Arc::new(SessionStore::open(&path).unwrap()));
        assert_eq!(
            history
                .read_snapshot("a", 0, "same-session-id")
                .unwrap()
                .events,
            expected.events
        );
        history.purge("a").unwrap();
        assert!(history.through("a", "same-session-id").unwrap().is_none());
        assert_eq!(
            history
                .read_snapshot("b", 0, "same-session-id")
                .unwrap()
                .events
                .len(),
            3
        );
        assert!(history.live("a", 0, &event(6)).is_err());
        assert!(history.snapshot("a", 0, None, &expected).is_err());
        assert!(history
            .page(
                "a",
                0,
                "same-session-id",
                6,
                true,
                &AgentConversationEventPage {
                    events: vec![event(5)],
                    has_more: false
                }
            )
            .is_err());
        drop(history);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn remote_history_fills_gaps_without_claiming_unfetched_events() {
        let history = RemoteHistory::new(Arc::new(SessionStore::open_in_memory().unwrap()));
        history.snapshot("a", 0, None, &snapshot(10, 12)).unwrap();
        history.live("a", 0, &event(15)).unwrap();
        assert_eq!(history.through("a", "same-session-id").unwrap(), Some(12));
        assert!(history
            .read_page("a", "same-session-id", 12, 1024, false)
            .unwrap()
            .is_none());
        history
            .snapshot("a", 0, Some(12), &snapshot(13, 15))
            .unwrap();
        assert_eq!(
            history
                .read_snapshot("a", 0, "same-session-id")
                .unwrap()
                .events,
            (10..=15).map(event).collect::<Vec<_>>()
        );
        let older = AgentConversationEventPage {
            events: (5..10).map(event).collect(),
            has_more: true,
        };
        history
            .page("a", 0, "same-session-id", 10, true, &older)
            .unwrap();
        assert_eq!(
            history
                .read_page("a", "same-session-id", 10, 100_000, true)
                .unwrap()
                .unwrap()
                .events,
            older.events
        );
        assert!(history
            .read_page("a", "same-session-id", 5, 1024, true)
            .unwrap()
            .is_none());
        history
            .page(
                "a",
                0,
                "same-session-id",
                5,
                true,
                &AgentConversationEventPage {
                    events: vec![event(4)],
                    has_more: false,
                },
            )
            .unwrap();
        let page = history
            .read_page("a", "same-session-id", 4, 1024, true)
            .unwrap()
            .unwrap();
        assert!(page.events.is_empty());
        assert!(!page.has_more);
        history.imported_older("a", 0, "same-session-id").unwrap();
        assert!(history
            .read_page("a", "same-session-id", 4, 1024, true)
            .unwrap()
            .is_none());
    }

    #[test]
    fn remote_history_caches_live_messages_before_a_snapshot_and_bounds_reads() {
        let history = RemoteHistory::new(Arc::new(SessionStore::open_in_memory().unwrap()));
        for sequence in 1..=10 {
            history.live("a", 0, &event(sequence)).unwrap();
        }
        history
            .snapshot("a", 0, Some(10), &snapshot(11, 10))
            .unwrap();
        assert_eq!(
            history
                .read_snapshot("a", 0, "same-session-id")
                .unwrap()
                .events
                .len(),
            10
        );
        let page = history
            .read_page("a", "same-session-id", 11, 400, true)
            .unwrap()
            .unwrap();
        assert!(!page.events.is_empty());
        assert!(page.events.len() < 10);
        assert_eq!(page.events.last().unwrap().sequence, 10);
        assert!(page.has_more);
    }
}

pub(super) struct RemoteHistory {
    store: Arc<SessionStore>,
    epochs: HashMap<String, u64>,
}

impl RemoteHistory {
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self {
            store,
            epochs: HashMap::new(),
        }
    }

    pub fn epoch(&self, profile: &str) -> u64 {
        *self.epochs.get(profile).unwrap_or(&0)
    }

    pub fn check(&self, profile: &str, epoch: u64) -> Result<(), String> {
        if self.epoch(profile) != epoch {
            return Err("Remote history request was disconnected".into());
        }
        Ok(())
    }

    pub fn purge(&mut self, profile: &str) -> Result<(), String> {
        *self.epochs.entry(profile.into()).or_default() += 1;
        self.store
            .purge_remote_history(profile)
            .map_err(|e| e.to_string())
    }

    fn key(profile: &str, owned: &str) -> String {
        // Tuple encoding prevents two machines with the same owned ID colliding.
        serde_json::to_string(&(profile, owned)).expect("string tuple is serializable")
    }

    fn coverage(&self, profile: &str, owned: &str) -> Result<Coverage, String> {
        let Some(row) = self
            .store
            .get_session(&Self::key(profile, owned))
            .map_err(|e| e.to_string())?
        else {
            return Ok(Coverage {
                remote_profile_id: profile.into(),
                ..Default::default()
            });
        };
        serde_json::from_str(&row.extra_json).map_err(|e| e.to_string())
    }

    pub fn through(&self, profile: &str, owned: &str) -> Result<Option<i64>, String> {
        Ok(self.coverage(profile, owned)?.through)
    }

    pub fn forget(&self, profile: &str, owned: &str) -> Result<(), String> {
        self.store
            .delete_session(&Self::key(profile, owned))
            .map_err(|e| e.to_string())
    }

    fn write(
        &self,
        profile: &str,
        owned: &str,
        coverage: &Coverage,
        events: &[AgentConversationEvent],
    ) -> Result<(), String> {
        let key = Self::key(profile, owned);
        let rows = events
            .iter()
            .map(|event| {
                Ok(EventRow {
                    owned_id: key.clone(),
                    seq: event.sequence,
                    turn_id: event.turn_id.clone(),
                    kind: "remote.cached".into(),
                    // The server has already prepared safe Markdown in these payloads.
                    payload_json: serde_json::to_string(event).map_err(|e| e.to_string())?,
                    created_at_ms: i64::try_from(event.timestamp_ms).unwrap_or(i64::MAX),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let row = SessionRow {
            owned_id: key,
            native_session_id: coverage
                .connection
                .as_ref()
                .and_then(|c| c.native_session_id.clone()),
            provider: "remote-cache".into(),
            model: None,
            effort: None,
            cwd: String::new(),
            worktree: None,
            branch: None,
            title: None,
            title_source: None,
            project: None,
            state: "cached".into(),
            suspended: coverage.suspended,
            created_at_ms: 0,
            last_activity_at_ms: rows.last().map_or(0, |row| row.created_at_ms),
            extra_json: serde_json::to_string(coverage).map_err(|e| e.to_string())?,
        };
        self.store
            .cache_remote_events(profile, &row, &rows)
            .map_err(|e| e.to_string())
    }

    pub fn live(
        &self,
        profile: &str,
        epoch: u64,
        event: &AgentConversationEvent,
    ) -> Result<(), String> {
        self.check(profile, epoch)?;
        let mut coverage = self.coverage(profile, &event.owned_id)?;
        if coverage.oldest.is_none() {
            coverage.oldest = Some(event.sequence);
        }
        if coverage.through.is_none() {
            coverage.oldest = Some(event.sequence);
            coverage.through = Some(event.sequence);
        } else if coverage.through.and_then(|seq| seq.checked_add(1)) == Some(event.sequence) {
            coverage.through = Some(event.sequence);
        }
        // A gap is stored, but never advertised as covered. Snapshot catch-up
        // fills it from the last confirmed sequence before reading it locally.
        self.write(
            profile,
            &event.owned_id,
            &coverage,
            std::slice::from_ref(event),
        )
    }

    pub fn snapshot(
        &self,
        profile: &str,
        epoch: u64,
        previous: Option<i64>,
        snapshot: &AgentConversationSnapshot,
    ) -> Result<i64, String> {
        self.check(profile, epoch)?;
        let owned = &snapshot.connection.owned_id;
        let mut coverage = self.coverage(profile, owned)?;
        coverage.connection = Some(snapshot.connection.clone());
        coverage.suspended = snapshot.suspended;
        if let Some(first) = snapshot.events.first() {
            coverage.oldest = Some(
                coverage
                    .oldest
                    .map_or(first.sequence, |old| old.min(first.sequence)),
            );
        }
        let end = snapshot
            .events
            .last()
            .map(|event| event.sequence)
            .or(previous)
            .unwrap_or(snapshot.last_sequence);
        // Only a page that reaches the advertised head may confirm the head.
        coverage.through = Some(coverage.through.map_or(end, |old| old.max(end)));
        self.write(profile, owned, &coverage, &snapshot.events)?;
        Ok(end)
    }

    pub fn read_snapshot(
        &self,
        profile: &str,
        epoch: u64,
        owned: &str,
    ) -> Result<AgentConversationSnapshot, String> {
        self.check(profile, epoch)?;
        let coverage = self.coverage(profile, owned)?;
        let through = coverage.through.unwrap_or(0);
        let page = self
            .store
            .list_events_before(
                &Self::key(profile, owned),
                through.saturating_add(1),
                512 * 1024,
            )
            .map_err(|e| e.to_string())?;
        Ok(AgentConversationSnapshot {
            connection: coverage
                .connection
                .ok_or("Remote history has no connection metadata")?,
            suspended: coverage.suspended,
            last_sequence: through,
            events: Self::decode(page.events)?,
        })
    }

    fn decode(rows: Vec<EventRow>) -> Result<Vec<AgentConversationEvent>, String> {
        rows.into_iter()
            .map(|row| serde_json::from_str(&row.payload_json).map_err(|e| e.to_string()))
            .collect()
    }

    pub fn read_page(
        &self,
        profile: &str,
        owned: &str,
        cursor: i64,
        bytes: u32,
        before: bool,
    ) -> Result<Option<AgentConversationEventPage>, String> {
        let coverage = self.coverage(profile, owned)?;
        let (Some(oldest), Some(through)) = (coverage.oldest, coverage.through) else {
            return Ok(None);
        };
        // We only answer from a range proven continuous, not merely MAX(seq).
        if before
            && (cursor > through.saturating_add(1)
                || (cursor <= oldest && !coverage.start_complete))
            || !before && (cursor < oldest || cursor >= through)
        {
            return Ok(None);
        }
        let key = Self::key(profile, owned);
        let page = if before {
            self.store.list_events_before(&key, cursor, bytes)
        } else {
            self.store.list_events_after(&key, cursor, bytes)
        }
        .map_err(|e| e.to_string())?;
        // Live rows beyond a gap must not leak into a supposedly complete page.
        if page.events.first().is_some_and(|event| event.seq < oldest)
            || page.events.last().is_some_and(|event| event.seq > through)
        {
            return Ok(None);
        }
        Ok(Some(AgentConversationEventPage {
            events: Self::decode(page.events)?,
            has_more: page.has_more || (before && !coverage.start_complete),
        }))
    }

    pub fn imported_older(&self, profile: &str, epoch: u64, owned: &str) -> Result<(), String> {
        self.check(profile, epoch)?;
        let mut coverage = self.coverage(profile, owned)?;
        coverage.start_complete = false;
        self.write(profile, owned, &coverage, &[])
    }

    pub fn page(
        &self,
        profile: &str,
        epoch: u64,
        owned: &str,
        cursor: i64,
        before: bool,
        page: &AgentConversationEventPage,
    ) -> Result<(), String> {
        self.check(profile, epoch)?;
        let mut coverage = self.coverage(profile, owned)?;
        let low = if before {
            page.events.first().map_or(cursor, |e| e.sequence)
        } else {
            cursor.saturating_add(1)
        };
        let high = if before {
            cursor.saturating_sub(1)
        } else {
            page.events.last().map_or(cursor, |e| e.sequence)
        };
        match (coverage.oldest, coverage.through) {
            (Some(old), Some(end))
                if low <= end.saturating_add(1) && high >= old.saturating_sub(1) =>
            {
                coverage.oldest = Some(old.min(low));
                coverage.through = Some(end.max(high));
                if before && !page.has_more {
                    coverage.start_complete = true;
                }
            }
            (None, _) => {
                coverage.oldest = Some(low);
                coverage.through = Some(high);
                coverage.start_complete = before && !page.has_more;
            }
            _ => {} // Store disjoint data without pretending its missing gap exists.
        }
        self.write(profile, owned, &coverage, &page.events)
    }
}
