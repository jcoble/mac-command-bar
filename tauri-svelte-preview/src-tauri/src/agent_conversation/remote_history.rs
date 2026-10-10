//! Remote history uses the local SQLite event journal and paging code.
//! An explicit source column keeps cached sessions out of local runtime startup.
use super::protocol::{
    AgentConversationConnection, AgentConversationEvent, AgentConversationEventPage,
    AgentConversationItemPage, AgentConversationPayload, AgentConversationSelectionSnapshot,
    AgentConversationSessionRecord, AgentConversationSnapshot, ApprovalState,
};
use mcb_core::session_store::{EventCoverage, EventRow, SessionRow, SessionStore};
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
    #[serde(default)]
    has_earlier_transcript: bool,
    #[serde(default)]
    pending_events: Vec<AgentConversationEvent>,
    #[serde(default)]
    pending_sequence: Option<i64>,
    #[serde(default)]
    active_turn_id: Option<String>,
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

    fn control_event(sequence: i64, payload: AgentConversationPayload) -> AgentConversationEvent {
        AgentConversationEvent {
            payload,
            ..event(sequence)
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
            has_earlier_transcript: false,
            pending_events: Vec::new(),
            active_turn_id: None,
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
        assert!(history.live_batch("a", 0, &[event(6)]).is_err());
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
    fn remote_children_reopen_under_two_exact_profile_keys() {
        let directory =
            std::env::temp_dir().join(format!("assembly-child-history-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("sessions.db");
        let store = Arc::new(SessionStore::open(&path).unwrap());
        let history = RemoteHistory::new(store.clone());
        for profile in ["a", "b"] {
            history.snapshot(profile, 0, None, &snapshot(1, 1)).unwrap();
            let child_key = history
                .bind_child(profile, "same-session-id", "same-child", "provider-child")
                .unwrap();
            let mut child_event = event(1);
            child_event.owned_id = "same-child".into();
            history.live_batch(profile, 0, &[child_event]).unwrap();
            let source = store
                .private_remote_child_source(&child_key)
                .unwrap()
                .unwrap();
            assert_eq!(source.remote_profile_id, profile);
            assert_eq!(source.source_owned_id, "same-child");
        }
        drop(history);
        drop(store);

        let history = RemoteHistory::new(Arc::new(SessionStore::open(&path).unwrap()));
        let a = history
            .read_child_item_page("a", "same-child", i64::MAX, 1024, true)
            .unwrap()
            .unwrap();
        let b = history
            .read_child_item_page("b", "same-child", i64::MAX, 1024, true)
            .unwrap()
            .unwrap();
        assert_eq!(a.events[0].owned_id, RemoteHistory::key("a", "same-child"));
        assert_eq!(b.events[0].owned_id, RemoteHistory::key("b", "same-child"));
        drop(history);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_session_first_opened_empty_pages_its_live_events_locally() {
        let history = RemoteHistory::new(Arc::new(SessionStore::open_in_memory().unwrap()));
        // A session created here opens before its first event, then streams 1..=3.
        history.snapshot("a", 0, None, &snapshot(1, 0)).unwrap();
        history.live_batch("a", 0, &[event(1), event(2), event(3)]).unwrap();
        let older = history.read_page("a", "same-session-id", 3, 1024, true).unwrap().unwrap();
        assert_eq!(older.events.iter().map(|e| e.sequence).collect::<Vec<_>>(), vec![1, 2]);
        // The copy confirms the empty start once; then the top needs no request.
        assert_eq!(history.unsaved_older("a", "same-session-id").unwrap(), Some(1));
        let start = AgentConversationEventPage { events: vec![], has_more: false };
        history.page("a", 0, "same-session-id", 1, true, &start).unwrap();
        assert_eq!(history.unsaved_older("a", "same-session-id").unwrap(), None);
        assert!(!history.read_page("a", "same-session-id", 3, 1024, true).unwrap().unwrap().has_more);
    }

    #[test]
    fn remote_history_fills_gaps_without_claiming_unfetched_events() {
        let history = RemoteHistory::new(Arc::new(SessionStore::open_in_memory().unwrap()));
        history.snapshot("a", 0, None, &snapshot(10, 12)).unwrap();
        history.live_batch("a", 0, &[event(15)]).unwrap();
        assert_eq!(history.through("a", "same-session-id").unwrap(), Some(12));
        // The saved row past the gap is not confirmed, so the Mac's copy ends at 12.
        for cursor in [11, 12] {
            let newer = history.read_page("a", "same-session-id", cursor, 1024, false).unwrap().unwrap();
            assert_eq!(newer.events.iter().map(|e| e.sequence).collect::<Vec<_>>(), (cursor + 1..=12).collect::<Vec<_>>());
            assert!(!newer.has_more);
        }
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
    fn pending_controls_follow_the_authoritative_head_not_older_pages() {
        let history = RemoteHistory::new(Arc::new(SessionStore::open_in_memory().unwrap()));
        let requested = control_event(
            15,
            AgentConversationPayload::Approval {
                request_id: "child-permission".into(),
                state: ApprovalState::Requested,
                summary: "Child permission".into(),
            },
        );
        // The authoritative control cursor can be ahead of the contiguous
        // copied content range while background history catches up.
        let mut initial = snapshot(10, 15);
        initial.events = vec![event(10)];
        initial.pending_events = vec![requested.clone()];
        history.snapshot("a", 0, None, &initial).unwrap();
        let raw = history.read_snapshot("a", 0, "same-session-id").unwrap();
        assert_eq!(raw.last_sequence, 10);
        assert_eq!(raw.pending_events, vec![requested.clone()]);
        let selected = history
            .read_selection_snapshot("a", "same-session-id", 1024)
            .unwrap()
            .unwrap();
        assert_eq!(selected.page.watermark, 10);
        assert_eq!(selected.pending_sequence, 15);

        history
            .live_batch(
                "a",
                0,
                &[
                    control_event(
                        11,
                        AgentConversationPayload::Turn {
                            turn_id: "root-turn".into(),
                            state: super::super::protocol::TurnState::Completed,
                        },
                    ),
                    event(12),
                    event(13),
                    event(14),
                    requested.clone(),
                    control_event(
                        16,
                        AgentConversationPayload::Approval {
                            request_id: "child-permission".into(),
                            state: ApprovalState::Expired,
                            summary: "Child permission".into(),
                        },
                    ),
                ],
            )
            .unwrap();
        let older = AgentConversationEventPage {
            events: vec![requested],
            has_more: false,
        };
        history.page("a", 0, "same-session-id", 10, true, &older).unwrap();
        assert!(history.read_snapshot("a", 0, "same-session-id").unwrap().pending_events.is_empty());
    }

    #[test]
    fn empty_remote_selection_preserves_head_and_copy_state() {
        let history = RemoteHistory::new(Arc::new(SessionStore::open_in_memory().unwrap()));
        let mut empty = snapshot(1, 0);
        empty.has_earlier_transcript = true;
        history.snapshot("a", 0, None, &empty).unwrap();

        let copying = history.read_selection_snapshot("a", "same-session-id", 1024)
            .unwrap().unwrap();
        assert!(copying.page.items.is_empty());
        assert_eq!(copying.page.watermark, 0);
        assert!(copying.page.has_before);
        assert!(copying.page.has_earlier_transcript);
        assert!(copying.page.before_cursor.is_none());

        history.page("a", 0, "same-session-id", 1, true,
            &AgentConversationEventPage { events: Vec::new(), has_more: false }).unwrap();
        let copied = history.read_selection_snapshot("a", "same-session-id", 1024)
            .unwrap().unwrap();
        assert!(!copied.page.has_before);
        assert!(copied.page.has_earlier_transcript);
    }

    #[test]
    fn item_pages_use_logical_positions_before_waiting_for_missing_history() {
        let history = RemoteHistory::new(Arc::new(SessionStore::open_in_memory().unwrap()));
        let saved = snapshot(100, 102);
        history.write("a", "same-session-id", &Coverage {
            connection: Some(saved.connection), oldest: Some(100), through: Some(102),
            ..Default::default()
        }, &[]).unwrap();
        for (sequence, first_sequence) in [(100, 10), (101, 10), (102, 80)] {
            let mut wire = serde_json::to_value(event(sequence)).unwrap();
            wire["payload"] = serde_json::json!({
                "kind": "tool", "itemId": if first_sequence == 10 { "early" } else { "later" },
                "name": "read", "state": "completed", "firstSequence": first_sequence,
                "firstTimestampMs": first_sequence * 10
            });
            history.store.append_event(&EventRow {
                owned_id: RemoteHistory::key("a", "same-session-id"), seq: sequence,
                turn_id: Some("turn".into()), kind: "remote.cached".into(),
                payload_json: wire.to_string(), created_at_ms: 123,
            }).unwrap();
        }
        let older = history.read_item_page("a", "same-session-id", 50, 1024, true)
            .unwrap().unwrap();
        assert_eq!(older.items.iter().map(|item| (item.item_id.as_str(), item.first_sequence))
            .collect::<Vec<_>>(), [("early", 10)]);
        let newer = history.read_item_page("a", "same-session-id", 50, 1024, false)
            .unwrap().unwrap();
        assert_eq!(newer.items.iter().map(|item| (item.item_id.as_str(), item.first_sequence))
            .collect::<Vec<_>>(), [("later", 80)]);
        assert!(history.read_item_page("a", "same-session-id", 10, 1024, true)
            .unwrap().is_none());

        // A copied control record before the first item is also a missing item
        // boundary, even though its physical sequence is below the item cursor.
        let mut controls = snapshot(200, 201);
        controls.events[0] = control_event(200, AgentConversationPayload::Turn {
            turn_id: "turn".into(), state: super::super::protocol::TurnState::Started,
        });
        history.forget("a", "same-session-id").unwrap();
        history.snapshot("a", 0, None, &controls).unwrap();
        assert!(history.read_item_page("a", "same-session-id", 201, 1024, true)
            .unwrap().is_none());
    }

    #[test]
    fn newer_runtime_generation_preserves_the_contiguous_journal() {
        let history = RemoteHistory::new(Arc::new(SessionStore::open_in_memory().unwrap()));
        history.snapshot("a", 0, None, &snapshot(1, 3)).unwrap();
        let mut restarted = snapshot(1, 5);
        restarted.connection.generation = 2;
        for event in restarted.events.iter_mut().filter(|event| event.sequence > 3) {
            event.generation = 2;
        }
        history.snapshot("a", 0, None, &restarted).unwrap();

        let saved = history.read_snapshot("a", 0, "same-session-id").unwrap();
        assert_eq!(saved.connection.generation, 2);
        assert_eq!(saved.events.len(), 5);
        assert_eq!(saved.events.first().map(|event| event.sequence), Some(1));
        assert_eq!(saved.events.last().map(|event| event.sequence), Some(5));
    }

    #[test]
    fn adjacent_live_batch_survives_reopen_in_order() {
        let directory = std::env::temp_dir().join(format!("assembly-batch-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("sessions.db");
        let history = RemoteHistory::new(Arc::new(SessionStore::open(&path).unwrap()));
        history.snapshot("a", 0, None, &snapshot(1, 1)).unwrap();
        history.live_batch("a", 0, &[event(2), event(3), event(4)]).unwrap();
        drop(history);
        let history = RemoteHistory::new(Arc::new(SessionStore::open(&path).unwrap()));
        assert_eq!(history.through("a", "same-session-id").unwrap(), Some(4));
        assert_eq!(history.read_snapshot("a", 0, "same-session-id").unwrap().events,
            vec![event(1), event(2), event(3), event(4)]);
        drop(history);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn live_skips_events_already_covered() {
        let history = RemoteHistory::new(Arc::new(SessionStore::open_in_memory().unwrap()));
        for sequence in 1..=3 {
            history.live_batch("a", 0, &[event(sequence)]).unwrap();
        }
        history.snapshot("a", 0, Some(3), &snapshot(4, 3)).unwrap();
        // A reconnect replays from the last applied cursor; the cache already has it.
        let mut replayed = event(2);
        replayed.timestamp_ms = 999;
        history.live_batch("a", 0, &[replayed]).unwrap();
        let cached = history.read_snapshot("a", 0, "same-session-id").unwrap();
        assert_eq!(cached.events[1].sequence, 2);
        assert_eq!(cached.events[1].timestamp_ms, 123);
    }

    #[test]
    fn remote_history_caches_live_messages_before_a_snapshot_and_bounds_reads() {
        let history = RemoteHistory::new(Arc::new(SessionStore::open_in_memory().unwrap()));
        for sequence in 1..=10 {
            history.live_batch("a", 0, &[event(sequence)]).unwrap();
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

    pub fn disconnect(&mut self, profile: &str) {
        *self.epochs.entry(profile.into()).or_default() += 1;
    }

    pub fn purge(&mut self, profile: &str) -> Result<(), String> {
        self.disconnect(profile);
        self.store
            .purge_remote_history(profile)
            .map_err(|e| e.to_string())
    }

    pub(super) fn key(profile: &str, owned: &str) -> String {
        // Tuple encoding prevents two machines with the same owned ID colliding.
        serde_json::to_string(&(profile, owned)).expect("string tuple is serializable")
    }

    pub fn bind_child(
        &self,
        profile: &str,
        parent_owned: &str,
        child_owned: &str,
        child_native_id: &str,
    ) -> Result<String, String> {
        let history_owned_id = Self::key(profile, child_owned);
        let coverage = self.coverage(profile, child_owned)?;
        let row = SessionRow {
            owned_id: history_owned_id.clone(),
            native_session_id: Some(child_native_id.to_string()),
            provider: "remote-cache".into(),
            model: None,
            effort: None,
            cwd: String::new(),
            worktree: None,
            branch: None,
            title: None,
            title_source: None,
            project: None,
            project_id: None,
            state: "cached".into(),
            suspended: true,
            created_at_ms: 0,
            last_activity_at_ms: 0,
            extra_json: serde_json::to_string(&coverage).map_err(|error| error.to_string())?,
        };
        if self
            .store
            .get_session(&history_owned_id)
            .map_err(|error| error.to_string())?
            .is_some()
        {
            self.store
                .bind_cached_remote_child(
                    &Self::key(profile, parent_owned),
                    profile,
                    &history_owned_id,
                    child_native_id,
                )
                .map_err(|error| error.to_string())?;
        } else {
            self.store
                .upsert_child_session(&Self::key(profile, parent_owned), Some(profile), &row)
                .map_err(|error| error.to_string())?;
        }
        Ok(history_owned_id)
    }

    /// Gives every session the machine listed a row here, filed under its
    /// project from the start, before any of its events are copied.
    pub fn file_sessions(&self, profile: &str, sessions: &[AgentConversationSessionRecord]) -> Result<(), String> {
        let rows: Vec<_> = sessions.iter()
            .map(|session| (Self::key(profile, &session.owned_id), &session.project_id, session.last_activity_at_ms))
            .collect();
        let coverage = Coverage { remote_profile_id: profile.into(), ..Default::default() };
        self.store
            .file_remote_sessions(
                profile,
                &serde_json::to_string(&coverage).map_err(|error| error.to_string())?,
                &serde_json::to_string(&rows).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())
    }

    pub fn delivery_owned_id(&self, profile: &str, source_owned_id: &str) -> Option<String> {
        let physical = Self::key(profile, source_owned_id);
        self.store
            .private_remote_child_source(&physical)
            .ok()
            .flatten()
            .map(|_| physical)
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
            project_id: None,
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

    pub fn live_batch(
        &self,
        profile: &str,
        epoch: u64,
        events: &[AgentConversationEvent],
    ) -> Result<(), String> {
        self.check(profile, epoch)?;
        let Some(first) = events.first() else { return Ok(()); };
        let mut coverage = self.coverage(profile, &first.owned_id)?;
        let control_start = coverage.pending_sequence.or(coverage.through).unwrap_or(i64::MIN);
        let start = events.iter().position(|event| !coverage.through.is_some_and(|through| event.sequence <= through));
        let Some(start) = start else { return Ok(()); };
        for event in &events[start..] {
            if event.owned_id != first.owned_id {
                return Err("remote history batch contains multiple sessions".into());
            }
            if coverage.through.is_none() {
                coverage.oldest = Some(event.sequence);
                coverage.through = Some(event.sequence);
            } else if coverage.through.and_then(|seq| seq.checked_add(1)) == Some(event.sequence) {
                coverage.through = Some(event.sequence);
            }
        }
        let confirmed = coverage.through.unwrap_or(i64::MIN);
        for event in events
            .iter()
            .filter(|event| event.sequence > control_start && event.sequence <= confirmed)
        {
            apply_pending_event(&mut coverage.pending_events, event);
            apply_active_turn(&mut coverage.active_turn_id, event);
            coverage.pending_sequence = Some(event.sequence);
        }
        coverage.pending_sequence.get_or_insert(confirmed);
        // A gap is stored, but never advertised as covered. Snapshot catch-up
        // fills it from the last confirmed sequence before reading it locally.
        self.write(profile, &first.owned_id, &coverage, &events[start..])
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
        coverage.has_earlier_transcript = snapshot.has_earlier_transcript;
        if coverage
            .pending_sequence
            .is_none_or(|sequence| snapshot.last_sequence >= sequence)
        {
            coverage.pending_events.clone_from(&snapshot.pending_events);
            coverage.pending_sequence = Some(snapshot.last_sequence);
            coverage.active_turn_id.clone_from(&snapshot.active_turn_id);
        }
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
        // A session saved before its first event starts its copy just past
        // that head, so its live events page locally and the copy checks the start.
        coverage.oldest.get_or_insert(end.saturating_add(1));
        // Only a page that reaches the advertised head may confirm the head.
        coverage.through = Some(coverage.through.map_or(end, |old| old.max(end)));
        self.write(profile, owned, &coverage, &snapshot.events)?;
        Ok(end)
    }

    #[cfg(test)]
    pub fn read_snapshot(
        &self,
        profile: &str,
        epoch: u64,
        owned: &str,
    ) -> Result<AgentConversationSnapshot, String> {
        self.check(profile, epoch)?;
        let coverage = self.coverage(profile, owned)?;
        let through = coverage.through.unwrap_or(0);
        let oldest = coverage.oldest.unwrap_or(i64::MIN);
        let page = self
            .store
            .list_events_before(
                &Self::key(profile, owned),
                through.saturating_add(1),
                512 * 1024,
                oldest,
            )
            .map_err(|e| e.to_string())?;
        Ok(AgentConversationSnapshot {
            connection: coverage
                .connection
                .ok_or("Remote history has no connection metadata")?,
            suspended: coverage.suspended,
            last_sequence: through,
            events: Self::decode(page.events)?,
            has_earlier_transcript: coverage.has_earlier_transcript,
            pending_events: coverage.pending_events,
            active_turn_id: coverage.active_turn_id,
        })
    }

    /// The confirmed tail of a session opened here before, or None when the
    /// Mac has never saved its connection details. Rows past a gap stay out.
    #[cfg(test)]
    pub fn cached_snapshot(
        &self,
        profile: &str,
        owned: &str,
    ) -> Result<Option<AgentConversationSnapshot>, String> {
        if self.coverage(profile, owned)?.connection.is_none() {
            return Ok(None);
        }
        self.read_snapshot(profile, self.epoch(profile), owned).map(Some)
    }

    /// Where the next older page must end, until the session's start is saved.
    pub fn unsaved_older(&self, profile: &str, owned: &str) -> Result<Option<i64>, String> {
        let coverage = self.coverage(profile, owned)?;
        Ok(coverage.oldest.filter(|_| !coverage.start_complete))
    }

    pub fn read_selection_snapshot(
        &self,
        profile: &str,
        owned: &str,
        max_bytes: u32,
    ) -> Result<Option<AgentConversationSelectionSnapshot>, String> {
        let coverage = self.coverage(profile, owned)?;
        let (Some(connection), Some(low), Some(high)) = (
            coverage.connection.clone(),
            coverage.oldest,
            coverage.through,
        ) else {
            return Ok(None);
        };
        if low > high {
            return Ok(Some(AgentConversationSelectionSnapshot {
                connection,
                suspended: coverage.suspended,
                page: AgentConversationItemPage {
                    items: Vec::new(),
                    events: Vec::new(),
                    turns: Vec::new(),
                    before_cursor: None,
                    after_cursor: None,
                    has_before: !coverage.start_complete,
                    has_earlier_transcript: coverage.has_earlier_transcript,
                    has_after: false,
                    watermark: high,
                    transfer_bytes: 0,
                    oversized: false,
                    coverage: None,
                },
                pending_events: coverage.pending_events,
                pending_sequence: coverage.pending_sequence.unwrap_or(high),
                active_turn_id: coverage.active_turn_id,
            }));
        }
        let page = self
            .store
            .list_items_before(
                &Self::key(profile, owned),
                i64::MAX,
                max_bytes,
                Some(EventCoverage {
                    low,
                    high,
                    start_complete: coverage.start_complete,
                }),
            )
            .map_err(|error| error.to_string())?;
        let mut page = super::manager::selected_item_page(page)?;
        page.has_before |= !coverage.start_complete;
        page.has_earlier_transcript = coverage.has_earlier_transcript;
        Ok(Some(AgentConversationSelectionSnapshot {
            connection,
            suspended: coverage.suspended,
            page,
            pending_events: coverage.pending_events,
            pending_sequence: coverage.pending_sequence.unwrap_or(high),
            active_turn_id: coverage.active_turn_id,
        }))
    }

    pub fn read_child_selection_snapshot(
        &self,
        profile: &str,
        source_owned_id: &str,
        max_bytes: u32,
    ) -> Result<Option<AgentConversationSelectionSnapshot>, String> {
        let history_owned_id = Self::key(profile, source_owned_id);
        let mut snapshot = self.read_selection_snapshot(profile, source_owned_id, max_bytes)?;
        if let Some(snapshot) = &mut snapshot {
            snapshot.connection.owned_id.clone_from(&history_owned_id);
            for event in &mut snapshot.page.events {
                event.owned_id.clone_from(&history_owned_id);
            }
            for event in &mut snapshot.pending_events {
                event.owned_id.clone_from(&history_owned_id);
            }
        }
        Ok(snapshot)
    }

    pub fn read_item_page(
        &self,
        profile: &str,
        owned: &str,
        cursor: i64,
        bytes: u32,
        before: bool,
    ) -> Result<Option<AgentConversationItemPage>, String> {
        let coverage = self.coverage(profile, owned)?;
        let (Some(low), Some(high)) = (coverage.oldest, coverage.through) else {
            return Ok(None);
        };
        if low > high {
            return Ok(coverage.start_complete.then(|| AgentConversationItemPage {
                items: Vec::new(),
                events: Vec::new(),
                turns: Vec::new(),
                before_cursor: None,
                after_cursor: None,
                has_before: false,
                has_earlier_transcript: coverage.has_earlier_transcript,
                has_after: false,
                watermark: high,
                transfer_bytes: 0,
                oversized: false,
                coverage: None,
            }));
        }
        let start_complete = coverage.start_complete;
        let has_earlier_transcript = coverage.has_earlier_transcript;
        let event_coverage = Some(EventCoverage {
            low,
            high,
            start_complete,
        });
        let page = if before {
            self.store
                .list_items_before(&Self::key(profile, owned), cursor, bytes, event_coverage)
        } else {
            self.store
                .list_items_after(&Self::key(profile, owned), cursor, bytes, event_coverage)
        }
        .map_err(|error| error.to_string())?;
        // Item positions can precede the physical copied range (for example,
        // a tool's firstSequence). Return saved items before deciding to wait.
        if page.items.is_empty() && !start_complete && (before || cursor < low) {
            return Ok(None);
        }
        let mut page = super::manager::selected_item_page(page)?;
        page.has_before |= before && !start_complete;
        page.has_earlier_transcript = has_earlier_transcript;
        Ok(Some(page))
    }

    pub fn read_child_item_page(
        &self,
        profile: &str,
        source_owned_id: &str,
        cursor: i64,
        bytes: u32,
        before: bool,
    ) -> Result<Option<AgentConversationItemPage>, String> {
        let history_owned_id = Self::key(profile, source_owned_id);
        let mut page = self.read_item_page(profile, source_owned_id, cursor, bytes, before)?;
        if let Some(page) = &mut page {
            for event in &mut page.events {
                event.owned_id.clone_from(&history_owned_id);
            }
        }
        Ok(page)
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
            || !before && cursor < oldest
        {
            return Ok(None);
        }
        let key = Self::key(profile, owned);
        // Rows saved past a gap are not confirmed: the page stops at the edge.
        let page = if before {
            self.store.list_events_before(&key, cursor, bytes, oldest)
        } else {
            self.store.list_events_after(&key, cursor, bytes, through)
        }
        .map_err(|e| e.to_string())?;
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

fn apply_pending_event(pending: &mut Vec<AgentConversationEvent>, event: &AgentConversationEvent) {
    if let AgentConversationPayload::ChildUpdate { child_id, .. } = &event.payload {
        pending.retain(|current| {
            !matches!(
                &current.payload,
                AgentConversationPayload::ChildUpdate {
                    child_id: current_child,
                    ..
                } if current_child == child_id
            )
        });
        pending.push(event.clone());
        return;
    }
    let (request_id, kind, requested) = match &event.payload {
        AgentConversationPayload::Approval {
            request_id,
            state,
            ..
        } => (request_id, "approval", *state == ApprovalState::Requested),
        AgentConversationPayload::UserInputRequested { request_id, .. } => {
            (request_id, "input", true)
        }
        AgentConversationPayload::UserInputResolved { request_id, .. } => {
            (request_id, "input", false)
        }
        _ => return,
    };
    pending.retain(|current| {
        current.generation != event.generation
            || pending_request_key(current).is_none_or(|(current_kind, id)| {
                current_kind != kind || id != request_id
            })
    });
    if requested {
        pending.push(event.clone());
    }
}

fn pending_request_key(event: &AgentConversationEvent) -> Option<(&'static str, &str)> {
    match &event.payload {
        AgentConversationPayload::Approval { request_id, .. } => Some(("approval", request_id)),
        AgentConversationPayload::UserInputRequested { request_id, .. }
        | AgentConversationPayload::UserInputResolved { request_id, .. } => {
            Some(("input", request_id))
        }
        _ => None,
    }
}

fn apply_active_turn(active_turn_id: &mut Option<String>, event: &AgentConversationEvent) {
    let AgentConversationPayload::Turn { turn_id, state } = &event.payload else {
        return;
    };
    match state {
        super::protocol::TurnState::Started => active_turn_id.clone_from(&Some(turn_id.clone())),
        super::protocol::TurnState::Completed
        | super::protocol::TurnState::Interrupted
        | super::protocol::TurnState::Failed
            if active_turn_id.as_deref() == Some(turn_id) =>
        {
            *active_turn_id = None;
        }
        _ => {}
    }
}
