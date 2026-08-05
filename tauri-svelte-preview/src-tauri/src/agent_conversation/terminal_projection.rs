use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Runtime};

use super::protocol::{AgentConversationProvider, AgentEvent, AgentRawFrameReference};
use super::transcript::{self, FileIdentity, ProjectedRecord, TranscriptLocation};

pub const TERMINAL_PROJECTION_EVENT: &str = "agent-conversation-event";
const WATCH_INTERVAL: Duration = Duration::from_millis(250);
const MAX_INCREMENTAL_BYTES: u64 = 1024 * 1024;
const RECONCILE_EVERY_POLLS: u32 = 40;
const MAX_RECONCILIATION_EVENTS: usize = 512;
const MAX_SEEN_RECORDS: usize = 4096;
const MAX_RECENT_EVENTS: usize = 2000;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTerminalProjectionRequest {
    pub owned_id: String,
    pub provider: AgentConversationProvider,
    pub native_session_id: String,
    pub generation: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalProjectionRegistration {
    pub generation: u64,
    pub events: Vec<AgentEvent>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReconcileReason {
    Initial,
    Periodic,
    Truncated,
    Rotated,
    Archived,
    Gap,
}

#[derive(Default)]
struct ProjectionCursor {
    canonical_path: Option<PathBuf>,
    identity: Option<FileIdentity>,
    offset: u64,
    partial_line: Vec<u8>,
    seen: HashSet<String>,
    seen_order: VecDeque<String>,
    polls_since_reconcile: u32,
    last_reconcile: Option<ReconcileReason>,
}

struct TranscriptProjector {
    provider: AgentConversationProvider,
    native_session_id: String,
    cursor: ProjectionCursor,
}

impl TranscriptProjector {
    fn new(provider: AgentConversationProvider, native_session_id: String) -> Self {
        Self {
            provider,
            native_session_id,
            cursor: ProjectionCursor::default(),
        }
    }

    fn poll(&mut self) -> Result<Vec<ProjectedRecord>, String> {
        let cached = self
            .cursor
            .canonical_path
            .as_deref()
            .map(transcript::inspect)
            .transpose()?
            .flatten();
        let Some(location) = cached.or(transcript::discover(
            self.provider,
            &self.native_session_id,
        )?) else {
            return Ok(Vec::new());
        };
        self.poll_location(location)
    }

    fn poll_location(
        &mut self,
        location: TranscriptLocation,
    ) -> Result<Vec<ProjectedRecord>, String> {
        let reason = match (&self.cursor.identity, &self.cursor.canonical_path) {
            (None, _) => Some(ReconcileReason::Initial),
            (Some(identity), Some(path))
                if identity == &location.identity && path != &location.canonical_path =>
            {
                Some(ReconcileReason::Archived)
            }
            (Some(identity), _) if identity != &location.identity => Some(ReconcileReason::Rotated),
            (Some(_), _) if location.len < self.cursor.offset => Some(ReconcileReason::Truncated),
            (Some(_), _)
                if location.len.saturating_sub(self.cursor.offset) > MAX_INCREMENTAL_BYTES =>
            {
                Some(ReconcileReason::Gap)
            }
            _ if self.cursor.polls_since_reconcile >= RECONCILE_EVERY_POLLS => {
                Some(ReconcileReason::Periodic)
            }
            _ => None,
        };
        if let Some(reason) = reason {
            return self.reconcile(location, reason);
        }
        self.cursor.polls_since_reconcile = self.cursor.polls_since_reconcile.saturating_add(1);
        if location.len == self.cursor.offset {
            return Ok(Vec::new());
        }
        let bytes = transcript::read_range(
            &location.canonical_path,
            self.cursor.offset,
            location.len.saturating_sub(self.cursor.offset),
        )?;
        self.cursor.offset = location.len;
        self.cursor.canonical_path = Some(location.canonical_path);
        self.cursor.identity = Some(location.identity);
        let mut combined = std::mem::take(&mut self.cursor.partial_line);
        combined.extend_from_slice(&bytes);
        let (lines, partial) = transcript::complete_lines(&combined, false);
        let records = self.project_lines(lines);
        self.cursor.partial_line = partial;
        Ok(records)
    }

    fn reconcile(
        &mut self,
        location: TranscriptLocation,
        reason: ReconcileReason,
    ) -> Result<Vec<ProjectedRecord>, String> {
        let (start, bytes) = transcript::read_bounded(&location.canonical_path, location.len)?;
        let (lines, partial) = transcript::complete_lines(&bytes, start > 0);
        let mut records = self.project_lines(lines);
        if records.len() > MAX_RECONCILIATION_EVENTS {
            records = records.split_off(records.len() - MAX_RECONCILIATION_EVENTS);
        }
        self.cursor.canonical_path = Some(location.canonical_path);
        self.cursor.identity = Some(location.identity);
        self.cursor.offset = location.len;
        self.cursor.partial_line = partial;
        self.cursor.polls_since_reconcile = 0;
        self.cursor.last_reconcile = Some(reason);
        Ok(records)
    }

    fn project_lines(&mut self, lines: Vec<&[u8]>) -> Vec<ProjectedRecord> {
        let mut projected = Vec::new();
        for line in lines {
            for record in
                transcript::parse_durable_line(self.provider, &self.native_session_id, line)
            {
                if self.cursor.seen.insert(record.key.clone()) {
                    self.cursor.seen_order.push_back(record.key.clone());
                    projected.push(record);
                }
            }
        }
        while self.cursor.seen_order.len() > MAX_SEEN_RECORDS {
            if let Some(expired) = self.cursor.seen_order.pop_front() {
                self.cursor.seen.remove(&expired);
            }
        }
        projected
    }
}

struct WatcherHandle {
    generation: u64,
    stop: Sender<()>,
    join: Option<JoinHandle<()>>,
}

impl WatcherHandle {
    fn stop(mut self) {
        let _ = self.stop.send(());
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

#[derive(Default)]
pub struct TerminalProjectionRegistry {
    watchers: Mutex<HashMap<String, WatcherHandle>>,
}

impl TerminalProjectionRegistry {
    pub fn start<R: Runtime>(
        &self,
        app: tauri::AppHandle<R>,
        request: StartTerminalProjectionRequest,
    ) -> Result<TerminalProjectionRegistration, String> {
        let owned_id = required_id(&request.owned_id, "Owned session id")?;
        let native_session_id = transcript::safe_session_id(&request.native_session_id)?;
        let previous = self
            .watchers
            .lock()
            .map_err(|_| "Terminal projection registry is unavailable".to_string())?
            .remove(&owned_id);
        let previous_generation = previous
            .as_ref()
            .map(|watcher| watcher.generation)
            .unwrap_or(0);
        if let Some(previous) = previous {
            previous.stop();
        }
        let generation = request
            .generation
            .max(previous_generation)
            .saturating_add(1);
        let mut projector = TranscriptProjector::new(request.provider, native_session_id.clone());
        let initial = projector.poll()?;
        let events = Arc::new(Mutex::new(VecDeque::new()));
        let mut next_sequence = 1_u64;
        for record in initial {
            let event = canonical_event(
                &owned_id,
                request.provider,
                &native_session_id,
                generation,
                next_sequence,
                record,
            );
            next_sequence = next_sequence.saturating_add(1);
            remember_and_emit(&app, &events, event);
        }
        let registration_events = events
            .lock()
            .map_err(|_| "Terminal projection snapshot is unavailable".to_string())?
            .iter()
            .cloned()
            .collect();
        let (stop, stopped) = mpsc::channel();
        let thread_owned_id = owned_id.clone();
        let thread_events = Arc::clone(&events);
        let join = thread::Builder::new()
            .name(format!("terminal-projection-{owned_id}"))
            .spawn(move || loop {
                match stopped.recv_timeout(WATCH_INTERVAL) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                let Ok(records) = projector.poll() else {
                    continue;
                };
                for record in records {
                    let event = canonical_event(
                        &thread_owned_id,
                        request.provider,
                        &native_session_id,
                        generation,
                        next_sequence,
                        record,
                    );
                    next_sequence = next_sequence.saturating_add(1);
                    remember_and_emit(&app, &thread_events, event);
                }
            })
            .map_err(|error| format!("Could not start terminal transcript projection: {error}"))?;
        self.watchers
            .lock()
            .map_err(|_| "Terminal projection registry is unavailable".to_string())?
            .insert(
                owned_id,
                WatcherHandle {
                    generation,
                    stop,
                    join: Some(join),
                },
            );
        Ok(TerminalProjectionRegistration {
            generation,
            events: registration_events,
        })
    }

    pub fn stop(&self, owned_id: &str) -> Result<bool, String> {
        let owned_id = required_id(owned_id, "Owned session id")?;
        let watcher = self
            .watchers
            .lock()
            .map_err(|_| "Terminal projection registry is unavailable".to_string())?
            .remove(&owned_id);
        if let Some(watcher) = watcher {
            watcher.stop();
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl Drop for TerminalProjectionRegistry {
    fn drop(&mut self) {
        let watchers = self
            .watchers
            .get_mut()
            .ok()
            .map(std::mem::take)
            .unwrap_or_default();
        for (_, watcher) in watchers {
            watcher.stop();
        }
    }
}

fn canonical_event(
    owned_id: &str,
    provider: AgentConversationProvider,
    native_session_id: &str,
    generation: u64,
    sequence: u64,
    record: ProjectedRecord,
) -> AgentEvent {
    AgentEvent {
        event_type: record.event_type,
        owned_id: owned_id.to_string(),
        provider,
        provider_instance_id: format!("terminal-transcript:{native_session_id}"),
        generation,
        sequence,
        timestamp_ms: if record.timestamp_ms == 0 {
            now_millis()
        } else {
            record.timestamp_ms
        },
        native_session_id: Some(native_session_id.to_string()),
        turn_id: None,
        item_id: record.item_id.clone(),
        request_id: None,
        payload: record.payload,
        provider_metadata: Some(std::collections::BTreeMap::from([
            (
                "source".into(),
                serde_json::Value::String("terminal-transcript".into()),
            ),
            ("historical".into(), serde_json::Value::Bool(true)),
        ])),
        raw_frame_reference: Some(AgentRawFrameReference {
            id: record.key,
            redacted: true,
        }),
    }
}

fn remember_and_emit<R: Runtime>(
    app: &tauri::AppHandle<R>,
    events: &Arc<Mutex<VecDeque<AgentEvent>>>,
    event: AgentEvent,
) {
    if let Ok(mut recent) = events.lock() {
        recent.push_back(event.clone());
        while recent.len() > MAX_RECENT_EVENTS {
            recent.pop_front();
        }
    }
    let _ = app.emit(TERMINAL_PROJECTION_EVENT, event);
}

fn required_id(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label} is required"))
    } else {
        Ok(value.to_string())
    }
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn temp_file(name: &str, contents: &[u8]) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("mcb-terminal-projection-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join(name);
        fs::write(&path, contents).unwrap();
        path
    }

    fn location(path: &PathBuf) -> TranscriptLocation {
        let canonical_path = fs::canonicalize(path).unwrap();
        let metadata = canonical_path.metadata().unwrap();
        #[cfg(unix)]
        let identity = {
            use std::os::unix::fs::MetadataExt;
            FileIdentity {
                device: metadata.dev(),
                inode: metadata.ino(),
            }
        };
        #[cfg(not(unix))]
        let identity = FileIdentity {
            device: 0,
            inode: metadata.len(),
        };
        TranscriptLocation {
            canonical_path,
            identity,
            len: metadata.len(),
        }
    }

    #[test]
    fn append_only_parse_reads_only_new_durable_lines() {
        let path = temp_file(
            "codex.jsonl",
            include_bytes!(
                "../../fixtures/agent_conversation/terminal_projection/codex_append.jsonl"
            ),
        );
        let mut projector =
            TranscriptProjector::new(AgentConversationProvider::Codex, "codex-session".into());
        assert_eq!(projector.poll_location(location(&path)).unwrap().len(), 1);
        fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(b"{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"id\":\"a2\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"second\"}]}}\n").unwrap();
        let records = projector.poll_location(location(&path)).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].item_id.as_deref(), Some("a2"));
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn offset_and_partial_line_resume_after_append() {
        let fixture = include_bytes!(
            "../../fixtures/agent_conversation/terminal_projection/codex_partial.jsonl"
        );
        let path = temp_file(
            "codex.jsonl",
            fixture.strip_suffix(b"\n").unwrap_or(fixture),
        );
        let mut projector =
            TranscriptProjector::new(AgentConversationProvider::Codex, "codex-session".into());
        assert!(projector.poll_location(location(&path)).unwrap().is_empty());
        assert!(!projector.cursor.partial_line.is_empty());
        fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"}]} }\n")
            .unwrap();
        assert_eq!(projector.poll_location(location(&path)).unwrap().len(), 1);
        assert!(projector.cursor.partial_line.is_empty());
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn detects_truncation_rotation_and_archive_move() {
        let path = temp_file(
            "session.jsonl",
            include_bytes!(
                "../../fixtures/agent_conversation/terminal_projection/codex_append.jsonl"
            ),
        );
        let mut projector =
            TranscriptProjector::new(AgentConversationProvider::Codex, "codex-session".into());
        projector.poll_location(location(&path)).unwrap();
        fs::write(&path, b"\n").unwrap();
        projector.poll_location(location(&path)).unwrap();
        assert_eq!(
            projector.cursor.last_reconcile,
            Some(ReconcileReason::Truncated)
        );
        fs::remove_file(&path).unwrap();
        fs::write(
            &path,
            include_bytes!(
                "../../fixtures/agent_conversation/terminal_projection/codex_append.jsonl"
            ),
        )
        .unwrap();
        projector.poll_location(location(&path)).unwrap();
        assert_eq!(
            projector.cursor.last_reconcile,
            Some(ReconcileReason::Rotated)
        );
        let archived = path.with_file_name("archived.jsonl");
        fs::rename(&path, &archived).unwrap();
        projector.poll_location(location(&archived)).unwrap();
        assert_eq!(
            projector.cursor.last_reconcile,
            Some(ReconcileReason::Archived)
        );
        fs::remove_dir_all(archived.parent().unwrap()).unwrap();
    }

    #[test]
    fn suppresses_duplicates_during_reconciliation() {
        let path = temp_file(
            "codex.jsonl",
            include_bytes!(
                "../../fixtures/agent_conversation/terminal_projection/codex_append.jsonl"
            ),
        );
        let mut projector =
            TranscriptProjector::new(AgentConversationProvider::Codex, "codex-session".into());
        assert_eq!(projector.poll_location(location(&path)).unwrap().len(), 1);
        projector.cursor.polls_since_reconcile = RECONCILE_EVERY_POLLS;
        assert!(projector.poll_location(location(&path)).unwrap().is_empty());
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn detects_incremental_gap_and_uses_bounded_reconciliation() {
        let path = temp_file(
            "codex.jsonl",
            include_bytes!(
                "../../fixtures/agent_conversation/terminal_projection/codex_append.jsonl"
            ),
        );
        let mut projector =
            TranscriptProjector::new(AgentConversationProvider::Codex, "codex-session".into());
        projector.poll_location(location(&path)).unwrap();
        projector.cursor.offset = 0;
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(&vec![b' '; MAX_INCREMENTAL_BYTES as usize + 1])
            .unwrap();
        file.write_all(b"\n").unwrap();
        drop(file);
        projector.poll_location(location(&path)).unwrap();
        assert_eq!(projector.cursor.last_reconcile, Some(ReconcileReason::Gap));
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn claude_fixture_projects_only_completed_historical_messages() {
        let input = include_bytes!(
            "../../fixtures/agent_conversation/terminal_projection/claude_append.jsonl"
        );
        let records = input
            .split(|byte| *byte == b'\n')
            .flat_map(|line| {
                transcript::parse_durable_line(
                    AgentConversationProvider::Claude,
                    "claude-session",
                    line,
                )
            })
            .collect::<Vec<_>>();
        assert!(records.iter().any(|record| {
            record.event_type == super::super::protocol::AgentEventType::ItemCompleted
                && record.item_id.as_deref() == Some("claude-a1")
        }));
        assert!(records.iter().all(|record| {
            record.payload.get("historical") == Some(&serde_json::Value::Bool(true))
        }));
    }

    #[test]
    fn bounded_reconciliation_caps_emitted_records() {
        let path = temp_file("codex.jsonl", b"");
        let line = b"{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"id\":\"ITEM\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"x\"}]}}\n";
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        for index in 0..(MAX_RECONCILIATION_EVENTS + 20) {
            file.write_all(
                &String::from_utf8_lossy(line)
                    .replace("ITEM", &format!("item-{index}"))
                    .into_bytes(),
            )
            .unwrap();
        }
        drop(file);
        let mut projector =
            TranscriptProjector::new(AgentConversationProvider::Codex, "codex-session".into());
        assert_eq!(
            projector.poll_location(location(&path)).unwrap().len(),
            MAX_RECONCILIATION_EVENTS
        );
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn no_invented_live_state_from_historical_frames() {
        let input = include_bytes!(
            "../../fixtures/agent_conversation/terminal_projection/no_invented_state.jsonl"
        );
        let records = input
            .split(|byte| *byte == b'\n')
            .flat_map(|line| {
                transcript::parse_durable_line(
                    AgentConversationProvider::Codex,
                    "codex-session",
                    line,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(records.len(), 1);
        assert!(records.iter().all(|record| !matches!(
            record.event_type,
            super::super::protocol::AgentEventType::TurnStarted
                | super::super::protocol::AgentEventType::ApprovalRequested
                | super::super::protocol::AgentEventType::ItemStarted
                | super::super::protocol::AgentEventType::ItemUpdated
        )));
        assert!(
            records
                .iter()
                .all(|record| record.payload.get("historical")
                    == Some(&serde_json::Value::Bool(true)))
        );
    }

    #[test]
    fn registry_drop_stops_and_joins_every_watcher() {
        let stopped = Arc::new(AtomicBool::new(false));
        let thread_stopped = Arc::clone(&stopped);
        let (stop, receiver) = mpsc::channel();
        let join = thread::spawn(move || {
            let _ = receiver.recv();
            thread_stopped.store(true, Ordering::SeqCst);
        });
        let registry = TerminalProjectionRegistry::default();
        registry.watchers.lock().unwrap().insert(
            "owned-a".into(),
            WatcherHandle {
                generation: 1,
                stop,
                join: Some(join),
            },
        );
        drop(registry);
        assert!(stopped.load(Ordering::SeqCst));
    }
}
