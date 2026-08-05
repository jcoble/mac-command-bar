use std::collections::VecDeque;

use super::protocol::AgentEvent;

#[derive(Clone, Debug)]
pub struct AgentEventJournal {
    capacity: usize,
    events: VecDeque<AgentEvent>,
}

impl AgentEventJournal {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            events: VecDeque::new(),
        }
    }

    pub fn append(&mut self, event: AgentEvent) -> Result<(), String> {
        if let Some(previous) = self.events.back() {
            if previous.owned_id == event.owned_id
                && previous.generation == event.generation
                && event.sequence <= previous.sequence
            {
                return Err("Journal sequence must increase monotonically".to_string());
            }
        }
        self.events.push_back(event);
        while self.events.len() > self.capacity {
            self.events.pop_front();
        }
        Ok(())
    }

    pub fn rebuild<I>(capacity: usize, events: I) -> (Self, usize)
    where
        I: IntoIterator<Item = AgentEvent>,
    {
        let mut journal = Self::new(capacity);
        let mut repaired = 0;
        for mut event in events {
            let expected = journal
                .events
                .back()
                .map_or(1, |last| last.sequence.saturating_add(1));
            if event.sequence != expected {
                event.sequence = expected;
                repaired += 1;
            }
            let _ = journal.append(event);
        }
        (journal, repaired)
    }

    pub fn snapshot(&self) -> Vec<AgentEvent> {
        self.events.iter().cloned().collect()
    }

    pub fn next_sequence(&self) -> u64 {
        self.events
            .back()
            .map_or(1, |event| event.sequence.saturating_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_conversation::protocol::{AgentConversationProvider, AgentEventType};
    use std::collections::BTreeMap;

    fn event(sequence: u64) -> AgentEvent {
        AgentEvent {
            event_type: AgentEventType::RuntimeWarning,
            owned_id: "owned-a".into(),
            provider: AgentConversationProvider::Codex,
            provider_instance_id: "fake".into(),
            generation: 1,
            sequence,
            timestamp_ms: 1,
            native_session_id: None,
            turn_id: None,
            item_id: None,
            request_id: None,
            payload: BTreeMap::new(),
            provider_metadata: None,
            raw_frame_reference: None,
        }
    }

    #[test]
    fn sequence_repair_rebuilds_a_bounded_journal() {
        let (journal, repaired) =
            AgentEventJournal::rebuild(2, vec![event(4), event(4), event(20)]);
        assert_eq!(repaired, 3);
        assert_eq!(
            journal
                .snapshot()
                .iter()
                .map(|event| event.sequence)
                .collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert_eq!(journal.next_sequence(), 4);
    }

    #[test]
    fn journal_recovery_rejects_live_duplicate_sequence() {
        let mut journal = AgentEventJournal::new(4);
        journal.append(event(1)).unwrap();
        assert!(journal.append(event(1)).is_err());
    }
}
