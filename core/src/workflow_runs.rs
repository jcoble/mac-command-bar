//! Workflow run housekeeping on the session store (TSK-1606).
use crate::session_store::{Result, SessionStore, StoreError};

impl SessionStore {
    /// Deletes every workflow event of one run, which removes the run.
    pub fn delete_workflow_run_events(&self, run_id: &str) -> Result<usize> {
        let connection = self.lock()?;
        connection.execute(
            "DELETE FROM orchestration_events WHERE run_id = ?1 AND workflow_id IS NOT NULL AND kind LIKE 'workflow.%'",
            [run_id],
        ).map_err(|error| StoreError::sqlite("could not delete the run's events", error))
    }
}
