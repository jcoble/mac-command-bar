//! Workflow run housekeeping on the session store (TSK-1606).
use crate::session_store::{Result, SessionStore, StoreError};

impl SessionStore {
    /// The ids of runs whose latest snapshot is completed, failed or cancelled and was last changed before `before_ms`.
    pub fn finished_workflow_runs_before(&self, before_ms: u64) -> Result<Vec<String>> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT e.run_id FROM orchestration_events e
             WHERE e.workflow_id IS NOT NULL AND e.kind LIKE 'workflow.%'
               AND e.rowid = (SELECT rowid FROM orchestration_events
                   WHERE run_id = e.run_id AND workflow_id IS NOT NULL AND kind LIKE 'workflow.%'
                   ORDER BY sequence DESC LIMIT 1)
               AND json_extract(e.payload_json, '$.workflowPayload.run.state') IN ('completed', 'failed', 'cancelled')
               AND json_extract(e.payload_json, '$.workflowPayload.run.updatedAtMs') < ?1
             ORDER BY e.run_id",
        ).map_err(|error| StoreError::sqlite("could not prepare the old finished runs", error))?;
        let rows = statement.query_map([before_ms as i64], |row| row.get(0))
            .map_err(|error| StoreError::sqlite("could not query the old finished runs", error))?;
        rows.collect::<rusqlite::Result<_>>().map_err(|error| StoreError::sqlite("could not read the old finished runs", error))
    }

    /// Deletes every workflow event of one run, which removes the run.
    pub fn delete_workflow_run_events(&self, run_id: &str) -> Result<usize> {
        let connection = self.lock()?;
        connection.execute(
            "DELETE FROM orchestration_events WHERE run_id = ?1 AND workflow_id IS NOT NULL AND kind LIKE 'workflow.%'",
            [run_id],
        ).map_err(|error| StoreError::sqlite("could not delete the run's events", error))
    }
}
