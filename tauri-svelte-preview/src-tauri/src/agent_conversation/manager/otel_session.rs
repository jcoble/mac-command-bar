use super::*;

impl AgentRuntimeManager {
    /// The live Assembly session a provider session id belongs to, including a
    /// Claude session's own child sessions. Telemetry carries only the
    /// provider's id.
    pub fn owned_id_for_native_session(&self, native_session_id: &str) -> Option<String> {
        let sessions = self.sessions.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        sessions.values()
            .find(|session| session.native_session_id.as_deref() == Some(native_session_id)
                || session.claude_children.contains_key(native_session_id))
            .map(|session| session.owned_id.clone())
    }
}
