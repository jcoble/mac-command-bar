//! Explicit ownership handoffs between the structured view and a user terminal.
//!
//! The state machine in this module is deliberately independent of Tauri and the
//! provider transport.  That keeps the identity, generation, history, and
//! process-tree rules testable before a native window is available.

use serde::{Deserialize, Serialize};

use super::manager::AgentRuntimeManager;
use super::protocol::{AgentExecutionOwner, AgentWriterLeaseOwner};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentConversationHandoffDirection {
    StructuredToTerminal,
    TerminalToStructured,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentConversationHandoffMode {
    SameSession,
    Fork,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentConversationHandoffPhase {
    Prepare,
    Commit,
    Rollback,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationHistoryBoundary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_session_id: Option<String>,
    #[serde(default)]
    pub first_sequence: u64,
    #[serde(default)]
    pub last_sequence: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reconciled_sequence: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationProcessTreeAssertion {
    #[serde(default)]
    pub checked: bool,
    #[serde(default)]
    pub tui_live: bool,
    #[serde(default)]
    pub tui_released: bool,
    #[serde(default)]
    pub writer_count: u32,
    #[serde(default, rename = "ptyCount", alias = "userPtyCount")]
    pub user_pty_count: u32,
    #[serde(default)]
    pub sidecar_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pty_session_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationHandoffRequest {
    pub owned_id: String,
    pub generation: u64,
    pub direction: AgentConversationHandoffDirection,
    pub mode: AgentConversationHandoffMode,
    pub phase: AgentConversationHandoffPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_owner: Option<AgentWriterLeaseOwner>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_owned_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pty_session_id: Option<String>,
    #[serde(default)]
    pub history_boundary: AgentConversationHistoryBoundary,
    #[serde(default)]
    pub process_tree: AgentConversationProcessTreeAssertion,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversationHandoffReceipt {
    pub owned_id: String,
    pub generation: u64,
    pub direction: AgentConversationHandoffDirection,
    pub mode: AgentConversationHandoffMode,
    pub phase: AgentConversationHandoffPhase,
    pub previous_owner: AgentWriterLeaseOwner,
    pub owner: AgentWriterLeaseOwner,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pty_session_id: Option<String>,
    pub history_boundary: AgentConversationHistoryBoundary,
    pub process_tree: AgentConversationProcessTreeAssertion,
    pub rollback_available: bool,
    pub message: String,
}

#[derive(Clone, Debug)]
struct PendingHandoff {
    direction: AgentConversationHandoffDirection,
    mode: AgentConversationHandoffMode,
    previous_owner: AgentWriterLeaseOwner,
    target_owned_id: String,
}

/// Pure handoff state used by the controller and by the focused Rust tests.
#[derive(Clone, Debug)]
pub struct HandoffState {
    owned_id: String,
    generation: u64,
    native_session_id: Option<String>,
    owner: AgentWriterLeaseOwner,
    last_sequence: u64,
    fork_capable: bool,
    scrollback_preserved: bool,
    pending: Option<PendingHandoff>,
}

impl HandoffState {
    pub fn new(
        owned_id: impl Into<String>,
        generation: u64,
        owner: AgentWriterLeaseOwner,
        native_session_id: Option<String>,
        last_sequence: u64,
        fork_capable: bool,
    ) -> Self {
        Self {
            owned_id: owned_id.into(),
            generation,
            native_session_id,
            owner,
            last_sequence,
            fork_capable,
            scrollback_preserved: true,
            pending: None,
        }
    }

    pub fn owner(&self) -> AgentWriterLeaseOwner {
        self.owner
    }

    pub fn scrollback_preserved(&self) -> bool {
        self.scrollback_preserved
    }

    pub fn prepare(
        &mut self,
        request: &AgentConversationHandoffRequest,
    ) -> Result<AgentConversationHandoffReceipt, String> {
        self.validate_identity(request)?;
        if request.phase != AgentConversationHandoffPhase::Prepare {
            return Err("Handoff prepare requires phase=prepare".to_string());
        }
        if self.pending.is_some() {
            return Err("Another handoff is already in progress".to_string());
        }
        let expected = request
            .expected_owner
            .unwrap_or_else(|| match request.direction {
                AgentConversationHandoffDirection::StructuredToTerminal => {
                    AgentWriterLeaseOwner::Structured
                }
                AgentConversationHandoffDirection::TerminalToStructured => {
                    AgentWriterLeaseOwner::Terminal
                }
            });
        if self.owner != expected {
            return Err("The current writer owner does not match the handoff request".to_string());
        }
        self.validate_mode(request)?;
        let target_owned_id = request
            .target_owned_id
            .clone()
            .unwrap_or_else(|| self.owned_id.clone());
        self.pending = Some(PendingHandoff {
            direction: request.direction,
            mode: request.mode,
            previous_owner: self.owner,
            target_owned_id,
        });
        self.owner = match request.direction {
            AgentConversationHandoffDirection::StructuredToTerminal => {
                AgentWriterLeaseOwner::Structured
            }
            AgentConversationHandoffDirection::TerminalToStructured => {
                AgentWriterLeaseOwner::Terminal
            }
        };
        Ok(self.receipt(
            request,
            AgentConversationHandoffPhase::Prepare,
            expected,
            expected,
            true,
            "Handoff prepared; the previous owner remains authoritative until commit".to_string(),
        ))
    }

    pub fn commit(
        &mut self,
        request: &AgentConversationHandoffRequest,
    ) -> Result<AgentConversationHandoffReceipt, String> {
        self.validate_identity(request)?;
        if request.phase != AgentConversationHandoffPhase::Commit {
            return Err("Handoff commit requires phase=commit".to_string());
        }
        let pending = self
            .pending
            .clone()
            .ok_or_else(|| "Handoff commit has no prepared transition".to_string())?;
        if pending.direction != request.direction || pending.mode != request.mode {
            return Err("Handoff commit does not match the prepared transition".to_string());
        }
        if request.direction == AgentConversationHandoffDirection::StructuredToTerminal {
            validate_terminal_assertion(&request.process_tree)?;
        } else {
            validate_tui_release(&request.process_tree)?;
            self.validate_history(&request.history_boundary)?;
        }
        let previous_owner = pending.previous_owner;
        let target_owned_id = pending.target_owned_id;
        let target_owner = if request.mode == AgentConversationHandoffMode::Fork {
            AgentWriterLeaseOwner::Terminal
        } else {
            match request.direction {
                AgentConversationHandoffDirection::StructuredToTerminal => {
                    AgentWriterLeaseOwner::Terminal
                }
                AgentConversationHandoffDirection::TerminalToStructured => {
                    AgentWriterLeaseOwner::Structured
                }
            }
        };
        self.owner = if request.mode == AgentConversationHandoffMode::Fork {
            previous_owner
        } else {
            target_owner
        };
        self.pending = None;
        Ok(self.receipt_for_target(
            request,
            AgentConversationHandoffPhase::Commit,
            previous_owner,
            self.owner,
            target_owned_id,
            false,
            "Handoff committed without closing the terminal scrollback".to_string(),
        ))
    }

    pub fn rollback(
        &mut self,
        request: &AgentConversationHandoffRequest,
    ) -> Result<AgentConversationHandoffReceipt, String> {
        self.validate_identity(request)?;
        if request.phase != AgentConversationHandoffPhase::Rollback {
            return Err("Handoff rollback requires phase=rollback".to_string());
        }
        let pending = self
            .pending
            .take()
            .ok_or_else(|| "Handoff rollback has no prepared transition".to_string())?;
        self.owner = pending.previous_owner;
        self.scrollback_preserved = true;
        Ok(self.receipt_for_target(
            request,
            AgentConversationHandoffPhase::Rollback,
            pending.previous_owner,
            pending.previous_owner,
            self.owned_id.clone(),
            false,
            "Handoff rolled back; existing terminal scrollback was preserved".to_string(),
        ))
    }

    fn validate_identity(&self, request: &AgentConversationHandoffRequest) -> Result<(), String> {
        if request.owned_id != self.owned_id {
            return Err("Handoff owned id does not match the current session".to_string());
        }
        if request.generation != self.generation {
            return Err(
                "Conversation connection changed; retry on the current session".to_string(),
            );
        }
        Ok(())
    }

    fn validate_mode(&self, request: &AgentConversationHandoffRequest) -> Result<(), String> {
        if request.mode == AgentConversationHandoffMode::Fork {
            if request.direction != AgentConversationHandoffDirection::StructuredToTerminal {
                return Err("Only structured conversations can fork to a terminal".to_string());
            }
            if !self.fork_capable {
                return Err("The current provider did not advertise fork support".to_string());
            }
            let target = request
                .target_owned_id
                .as_deref()
                .ok_or_else(|| "Fork handoff requires a distinct target owned id".to_string())?;
            if target.trim().is_empty() || target == self.owned_id {
                return Err("Fork handoff requires a distinct target owned id".to_string());
            }
        }
        Ok(())
    }

    fn validate_history(&self, boundary: &AgentConversationHistoryBoundary) -> Result<(), String> {
        if boundary.native_session_id != self.native_session_id {
            return Err(
                "Handoff history native session does not match the current session".to_string(),
            );
        }
        if boundary.first_sequence > boundary.last_sequence
            || boundary.last_sequence > self.last_sequence
        {
            return Err("Handoff history boundary is outside the stored conversation".to_string());
        }
        if boundary.reconciled_sequence != Some(self.last_sequence) {
            return Err("Handoff history was not reconciled to the current sequence".to_string());
        }
        Ok(())
    }

    fn receipt(
        &self,
        request: &AgentConversationHandoffRequest,
        phase: AgentConversationHandoffPhase,
        previous_owner: AgentWriterLeaseOwner,
        owner: AgentWriterLeaseOwner,
        rollback_available: bool,
        message: String,
    ) -> AgentConversationHandoffReceipt {
        self.receipt_for_target(
            request,
            phase,
            previous_owner,
            owner,
            self.owned_id.clone(),
            rollback_available,
            message,
        )
    }

    fn receipt_for_target(
        &self,
        request: &AgentConversationHandoffRequest,
        phase: AgentConversationHandoffPhase,
        previous_owner: AgentWriterLeaseOwner,
        owner: AgentWriterLeaseOwner,
        owned_id: String,
        rollback_available: bool,
        message: String,
    ) -> AgentConversationHandoffReceipt {
        AgentConversationHandoffReceipt {
            owned_id,
            generation: request.generation,
            direction: request.direction,
            mode: request.mode,
            phase,
            previous_owner,
            owner,
            native_session_id: self.native_session_id.clone(),
            pty_session_id: request
                .pty_session_id
                .clone()
                .or_else(|| request.process_tree.pty_session_id.clone()),
            history_boundary: request.history_boundary.clone(),
            process_tree: request.process_tree.clone(),
            rollback_available,
            message,
        }
    }
}

fn validate_terminal_assertion(
    assertion: &AgentConversationProcessTreeAssertion,
) -> Result<(), String> {
    if !assertion.checked || !assertion.tui_live {
        return Err("Commit requires proof that the native terminal UI is live".to_string());
    }
    if assertion.writer_count != 1 || assertion.user_pty_count != 1 || assertion.sidecar_count != 0
    {
        return Err(
            "Commit requires one writer, one user pty, and no duplicate sidecar".to_string(),
        );
    }
    if assertion.pty_session_id.is_none() {
        return Err("Commit requires the live user pty session id".to_string());
    }
    Ok(())
}

fn validate_tui_release(assertion: &AgentConversationProcessTreeAssertion) -> Result<(), String> {
    if !assertion.checked || !assertion.tui_released {
        return Err(
            "Resume requires proof that the native terminal UI released the writer".to_string(),
        );
    }
    if assertion.writer_count > 1 || assertion.user_pty_count > 1 || assertion.sidecar_count > 1 {
        return Err("Resume rejected a duplicate writer or process".to_string());
    }
    Ok(())
}

/// Restart recovery only restores a stored row. It never starts a provider as
/// a side effect of reading that row.
pub fn should_start_provider_for_stored_row(_owner: AgentExecutionOwner) -> bool {
    false
}

#[tauri::command]
pub async fn handoff_agent_conversation(
    manager: tauri::State<'_, AgentRuntimeManager>,
    terminal_registry: tauri::State<'_, crate::terminal::TerminalRegistry>,
    request: AgentConversationHandoffRequest,
) -> Result<AgentConversationHandoffReceipt, String> {
    match request.phase {
        AgentConversationHandoffPhase::Prepare => {
            manager.validate_handoff_identity(&request)?;
            let context = manager.handoff_prepare(&request)?;
            Ok(receipt_from_context(
                &request,
                context,
                true,
                "Handoff prepared",
            ))
        }
        AgentConversationHandoffPhase::Commit => {
            manager.validate_handoff_identity(&request)?;
            if request.direction == AgentConversationHandoffDirection::StructuredToTerminal {
                validate_terminal_assertion(&request.process_tree)?;
                let pty_id = request
                    .pty_session_id
                    .as_deref()
                    .or(request.process_tree.pty_session_id.as_deref())
                    .ok_or_else(|| "Commit requires a user pty session id".to_string())?;
                let session = crate::terminal::list_terminal_sessions(&terminal_registry)?
                    .into_iter()
                    .find(|session| session.session_id == pty_id)
                    .ok_or_else(|| "The requested user pty is not registered".to_string())?;
                if session.kind != crate::terminal::TerminalKind::UserPty || session.exited {
                    return Err("The requested user pty is not live".to_string());
                }
                if request.mode == AgentConversationHandoffMode::SameSession {
                    manager
                        .detach_structured_runtime(&request.owned_id, request.generation)
                        .await?;
                }
            } else {
                validate_tui_release(&request.process_tree)?;
                manager.validate_handoff_history(&request)?;
                manager
                    .activate(&request.owned_id, request.generation)
                    .await?;
            }
            let receipt = manager.handoff_commit(&request).await?;
            Ok(receipt)
        }
        AgentConversationHandoffPhase::Rollback => {
            manager.validate_handoff_identity(&request)?;
            Ok(manager.handoff_rollback(&request)?)
        }
    }
}

fn receipt_from_context(
    request: &AgentConversationHandoffRequest,
    context: super::manager::HandoffContext,
    rollback_available: bool,
    message: &str,
) -> AgentConversationHandoffReceipt {
    AgentConversationHandoffReceipt {
        owned_id: request.owned_id.clone(),
        generation: request.generation,
        direction: request.direction,
        mode: request.mode,
        phase: AgentConversationHandoffPhase::Prepare,
        previous_owner: context.previous_owner,
        owner: context.owner,
        native_session_id: context.native_session_id,
        pty_session_id: request
            .pty_session_id
            .clone()
            .or_else(|| request.process_tree.pty_session_id.clone()),
        history_boundary: request.history_boundary.clone(),
        process_tree: request.process_tree.clone(),
        rollback_available,
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(
        phase: AgentConversationHandoffPhase,
        direction: AgentConversationHandoffDirection,
    ) -> AgentConversationHandoffRequest {
        AgentConversationHandoffRequest {
            owned_id: "owned-a".into(),
            generation: 7,
            direction,
            mode: AgentConversationHandoffMode::SameSession,
            phase,
            expected_owner: None,
            target_owned_id: None,
            native_session_id: Some("native-7".into()),
            pty_session_id: Some("pty-7".into()),
            history_boundary: AgentConversationHistoryBoundary {
                native_session_id: Some("native-7".into()),
                first_sequence: 1,
                last_sequence: 9,
                reconciled_sequence: Some(9),
            },
            process_tree: AgentConversationProcessTreeAssertion {
                checked: true,
                tui_live: true,
                tui_released: false,
                writer_count: 1,
                user_pty_count: 1,
                sidecar_count: 0,
                pty_session_id: Some("pty-7".into()),
            },
        }
    }

    #[test]
    fn prepare_rejects_stale_generation_and_wrong_owner() {
        let mut state = HandoffState::new(
            "owned-a",
            7,
            AgentWriterLeaseOwner::Structured,
            Some("native-7".into()),
            9,
            true,
        );
        let mut stale = request(
            AgentConversationHandoffPhase::Prepare,
            AgentConversationHandoffDirection::StructuredToTerminal,
        );
        stale.generation = 6;
        assert!(state.prepare(&stale).is_err());
        let mut wrong_owner = request(
            AgentConversationHandoffPhase::Prepare,
            AgentConversationHandoffDirection::TerminalToStructured,
        );
        wrong_owner.expected_owner = Some(AgentWriterLeaseOwner::Terminal);
        assert!(state.prepare(&wrong_owner).is_err());
    }

    #[test]
    fn structured_to_terminal_prepare_blocks_prompt_and_commit_requires_live_tui() {
        let mut state = HandoffState::new(
            "owned-a",
            7,
            AgentWriterLeaseOwner::Structured,
            Some("native-7".into()),
            9,
            true,
        );
        state
            .prepare(&request(
                AgentConversationHandoffPhase::Prepare,
                AgentConversationHandoffDirection::StructuredToTerminal,
            ))
            .unwrap();
        assert_eq!(state.owner(), AgentWriterLeaseOwner::Structured);
        let mut commit = request(
            AgentConversationHandoffPhase::Commit,
            AgentConversationHandoffDirection::StructuredToTerminal,
        );
        commit.process_tree.tui_live = false;
        assert!(state.commit(&commit).is_err());
        commit.process_tree.tui_live = true;
        assert_eq!(
            state.commit(&commit).unwrap().owner,
            AgentWriterLeaseOwner::Terminal
        );
    }

    #[test]
    fn terminal_to_structured_requires_tui_release_before_resume() {
        let mut state = HandoffState::new(
            "owned-a",
            7,
            AgentWriterLeaseOwner::Terminal,
            Some("native-7".into()),
            9,
            true,
        );
        state
            .prepare(&request(
                AgentConversationHandoffPhase::Prepare,
                AgentConversationHandoffDirection::TerminalToStructured,
            ))
            .unwrap();
        let mut commit = request(
            AgentConversationHandoffPhase::Commit,
            AgentConversationHandoffDirection::TerminalToStructured,
        );
        assert!(state.commit(&commit).is_err());
        commit.process_tree.tui_released = true;
        assert_eq!(
            state.commit(&commit).unwrap().owner,
            AgentWriterLeaseOwner::Structured
        );
    }

    #[test]
    fn same_session_preserves_owned_id_and_native_session_id() {
        let mut state = HandoffState::new(
            "owned-a",
            7,
            AgentWriterLeaseOwner::Structured,
            Some("native-7".into()),
            9,
            true,
        );
        state
            .prepare(&request(
                AgentConversationHandoffPhase::Prepare,
                AgentConversationHandoffDirection::StructuredToTerminal,
            ))
            .unwrap();
        let receipt = state
            .commit(&request(
                AgentConversationHandoffPhase::Commit,
                AgentConversationHandoffDirection::StructuredToTerminal,
            ))
            .unwrap();
        assert_eq!(receipt.owned_id, "owned-a");
        assert_eq!(receipt.native_session_id.as_deref(), Some("native-7"));
    }

    #[test]
    fn fork_requires_distinct_target_and_advertised_capability() {
        let mut state = HandoffState::new(
            "owned-a",
            7,
            AgentWriterLeaseOwner::Structured,
            Some("native-7".into()),
            9,
            false,
        );
        let mut request = request(
            AgentConversationHandoffPhase::Prepare,
            AgentConversationHandoffDirection::StructuredToTerminal,
        );
        request.mode = AgentConversationHandoffMode::Fork;
        request.target_owned_id = Some("owned-b".into());
        assert!(state.prepare(&request).is_err());
        state.fork_capable = true;
        request.target_owned_id = Some("owned-a".into());
        assert!(state.prepare(&request).is_err());
        request.target_owned_id = Some("owned-b".into());
        assert!(state.prepare(&request).is_ok());
    }

    #[test]
    fn rollback_restores_previous_owner_without_killing_scrollback() {
        let mut state = HandoffState::new(
            "owned-a",
            7,
            AgentWriterLeaseOwner::Structured,
            Some("native-7".into()),
            9,
            true,
        );
        state
            .prepare(&request(
                AgentConversationHandoffPhase::Prepare,
                AgentConversationHandoffDirection::StructuredToTerminal,
            ))
            .unwrap();
        let receipt = state
            .rollback(&request(
                AgentConversationHandoffPhase::Rollback,
                AgentConversationHandoffDirection::StructuredToTerminal,
            ))
            .unwrap();
        assert_eq!(receipt.owner, AgentWriterLeaseOwner::Structured);
        assert!(state.scrollback_preserved());
    }

    #[test]
    fn history_boundary_mismatch_fails_closed() {
        let mut state = HandoffState::new(
            "owned-a",
            7,
            AgentWriterLeaseOwner::Terminal,
            Some("native-7".into()),
            9,
            true,
        );
        state
            .prepare(&request(
                AgentConversationHandoffPhase::Prepare,
                AgentConversationHandoffDirection::TerminalToStructured,
            ))
            .unwrap();
        let mut commit = request(
            AgentConversationHandoffPhase::Commit,
            AgentConversationHandoffDirection::TerminalToStructured,
        );
        commit.history_boundary.reconciled_sequence = Some(8);
        assert!(state.commit(&commit).is_err());
    }

    #[test]
    fn process_tree_assertion_rejects_duplicate_user_pty() {
        let mut state = HandoffState::new(
            "owned-a",
            7,
            AgentWriterLeaseOwner::Structured,
            Some("native-7".into()),
            9,
            true,
        );
        state
            .prepare(&request(
                AgentConversationHandoffPhase::Prepare,
                AgentConversationHandoffDirection::StructuredToTerminal,
            ))
            .unwrap();
        let mut commit = request(
            AgentConversationHandoffPhase::Commit,
            AgentConversationHandoffDirection::StructuredToTerminal,
        );
        commit.process_tree.user_pty_count = 2;
        assert!(state.commit(&commit).is_err());
    }

    #[test]
    fn restart_recovery_does_not_start_provider_for_stored_row() {
        assert!(!should_start_provider_for_stored_row(
            AgentExecutionOwner::Structured
        ));
        assert!(!should_start_provider_for_stored_row(
            AgentExecutionOwner::Terminal
        ));
    }
}
