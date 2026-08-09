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
    fn journal_recovery_rejects_live_duplicate_sequence() {
        let mut journal = AgentEventJournal::new(4);
        journal.append(event(1)).unwrap();
        assert!(journal.append(event(1)).is_err());
    }
}
