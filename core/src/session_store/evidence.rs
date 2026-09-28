use super::*;

impl SessionStore {
    pub fn upsert_evidence_artifact(&self, artifact: &EvidenceArtifact) -> Result<()> {
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                StoreError::sqlite("could not begin the evidence artifact write", error)
            })?;
        transaction
            .execute(
                "INSERT INTO evidence_artifacts (
                    id, orchestration_run_id, task_id, agent, provider, scenario,
                    commit_hash, branch, worktree, captured_at_ms, kind, status, byte_size,
                    original_ref, thumbnail_ref, thumbnail_byte_size, pinned, expires_at_ms
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                    orchestration_run_id = excluded.orchestration_run_id,
                    task_id = excluded.task_id,
                    agent = excluded.agent,
                    provider = excluded.provider,
                    scenario = excluded.scenario,
                    commit_hash = excluded.commit_hash,
                    branch = excluded.branch,
                    worktree = excluded.worktree,
                    captured_at_ms = excluded.captured_at_ms,
                    kind = excluded.kind,
                    status = excluded.status,
                    byte_size = excluded.byte_size,
                    original_ref = excluded.original_ref,
                    thumbnail_ref = excluded.thumbnail_ref,
                    thumbnail_byte_size = excluded.thumbnail_byte_size,
                    pinned = excluded.pinned,
                    expires_at_ms = excluded.expires_at_ms",
                params![
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
                    artifact.thumbnail_byte_size,
                    artifact.pinned,
                    artifact.expires_at_ms,
                ],
            )
            .map_err(|error| StoreError::sqlite("could not save the evidence artifact", error))?;
        transaction.commit().map_err(|error| {
            StoreError::sqlite("could not finish the evidence artifact write", error)
        })
    }

    pub fn list_evidence_artifacts(
        &self,
        query: &EvidenceArtifactQuery,
    ) -> Result<Vec<EvidenceArtifact>> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT id, orchestration_run_id, task_id, agent, provider, scenario,
                        commit_hash, branch, worktree, captured_at_ms, kind, status, byte_size,
                        original_ref, thumbnail_ref, thumbnail_byte_size, pinned, expires_at_ms
                 FROM evidence_artifacts
                 WHERE (
                    ?1 IS NULL
                    OR captured_at_ms < ?1
                    OR (captured_at_ms = ?1 AND id > ?2)
                 )
                   AND (?3 IS NULL OR task_id = ?3)
                   AND (?4 IS NULL OR commit_hash = ?4)
                   AND (?5 IS NULL OR orchestration_run_id = ?5)
                   AND (?6 IS NULL OR agent = ?6)
                   AND (?7 IS NULL OR scenario = ?7)
                   AND (?8 IS NULL OR captured_at_ms >= ?8)
                   AND (?9 IS NULL OR captured_at_ms <= ?9)
                 ORDER BY captured_at_ms DESC, id ASC
                 LIMIT ?10",
            )
            .map_err(|error| {
                StoreError::sqlite("could not prepare the evidence artifact page", error)
            })?;
        let rows = statement
            .query_map(
                params![
                    query.before_captured_at_ms,
                    query.before_id,
                    query.task_id,
                    query.commit_hash,
                    query.orchestration_run_id,
                    query.agent,
                    query.scenario,
                    query.captured_from_ms,
                    query.captured_to_ms,
                    i64::from(query.limit),
                ],
                evidence_artifact_from_row,
            )
            .map_err(|error| StoreError::sqlite("could not list evidence artifacts", error))?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|error| StoreError::sqlite("could not read the evidence artifact page", error))
    }

    pub fn evidence_artifact(&self, id: &str) -> Result<Option<EvidenceArtifact>> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT id, orchestration_run_id, task_id, agent, provider, scenario,
                        commit_hash, branch, worktree, captured_at_ms, kind, status, byte_size,
                        original_ref, thumbnail_ref, thumbnail_byte_size, pinned, expires_at_ms
                 FROM evidence_artifacts
                 WHERE id = ?",
                [id],
                evidence_artifact_from_row,
            )
            .optional()
            .map_err(|error| StoreError::sqlite("could not read the evidence artifact", error))
    }

    pub fn evidence_disk_usage(&self) -> Result<EvidenceDiskUsage> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT orchestration_run_id,
                        SUM(byte_size + thumbnail_byte_size) AS run_bytes,
                        SUM(SUM(byte_size + thumbnail_byte_size)) OVER () AS total_bytes
                 FROM evidence_artifacts
                 GROUP BY orchestration_run_id
                 ORDER BY run_bytes DESC, orchestration_run_id ASC",
            )
            .map_err(|error| StoreError::sqlite("could not prepare evidence disk usage", error))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    EvidenceRunDiskUsage {
                        orchestration_run_id: row.get(0)?,
                        byte_size: row.get(1)?,
                    },
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|error| StoreError::sqlite("could not read evidence disk usage", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| StoreError::sqlite("could not read evidence disk usage", error))?;
        Ok(EvidenceDiskUsage {
            total_bytes: rows.first().map(|(_, total)| *total).unwrap_or(0),
            runs: rows.into_iter().map(|(run, _)| run).collect(),
        })
    }

    pub fn delete_expired_unpinned_evidence_artifacts(
        &self,
        now_ms: i64,
        retention_ms: i64,
        limit: u32,
    ) -> Result<Vec<EvidenceArtifact>> {
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                StoreError::sqlite("could not begin expired evidence cleanup", error)
            })?;
        let artifacts = {
            let mut statement = transaction
                .prepare(
                    "DELETE FROM evidence_artifacts
                     WHERE id IN (
                        SELECT id
                        FROM evidence_artifacts
                        WHERE pinned = 0
                          AND COALESCE(expires_at_ms, captured_at_ms + ?2) <= ?1
                        ORDER BY COALESCE(expires_at_ms, captured_at_ms + ?2) ASC, id ASC
                        LIMIT ?3
                     )
                     RETURNING id, orchestration_run_id, task_id, agent, provider, scenario,
                               commit_hash, branch, worktree, captured_at_ms, kind, status,
                               byte_size, original_ref, thumbnail_ref, thumbnail_byte_size,
                               pinned, expires_at_ms",
                )
                .map_err(|error| {
                    StoreError::sqlite("could not prepare expired evidence cleanup", error)
                })?;
            let rows = statement
                .query_map(
                    params![now_ms, retention_ms, i64::from(limit)],
                    evidence_artifact_from_row,
                )
                .map_err(|error| {
                    StoreError::sqlite("could not remove expired evidence rows", error)
                })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| {
                    StoreError::sqlite("could not read removed evidence rows", error)
                })?
        };
        transaction.commit().map_err(|error| {
            StoreError::sqlite("could not finish expired evidence cleanup", error)
        })?;
        Ok(artifacts)
    }

    pub fn set_evidence_artifact_pinned(&self, id: &str, pinned: bool) -> Result<bool> {
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                StoreError::sqlite("could not begin the evidence artifact pin update", error)
            })?;
        let updated = transaction
            .execute(
                "UPDATE evidence_artifacts SET pinned = ? WHERE id = ?",
                params![pinned, id],
            )
            .map_err(|error| {
                StoreError::sqlite("could not update the evidence artifact pin", error)
            })?;
        transaction.commit().map_err(|error| {
            StoreError::sqlite("could not finish the evidence artifact pin update", error)
        })?;
        Ok(updated > 0)
    }

    pub fn set_evidence_run_pinned(&self, run_id: &str, pinned: bool) -> Result<usize> {
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                StoreError::sqlite("could not begin the evidence run pin update", error)
            })?;
        let updated = transaction
            .execute(
                "UPDATE evidence_artifacts SET pinned = ? WHERE orchestration_run_id = ?",
                params![pinned, run_id],
            )
            .map_err(|error| StoreError::sqlite("could not update the evidence run pin", error))?;
        transaction.commit().map_err(|error| {
            StoreError::sqlite("could not finish the evidence run pin update", error)
        })?;
        Ok(updated)
    }

    pub fn delete_evidence_artifacts(&self, ids: &[String]) -> Result<Vec<EvidenceArtifact>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                StoreError::sqlite("could not begin the evidence artifact delete", error)
            })?;
        let placeholders = (0..ids.len()).map(|_| "?").collect::<Vec<_>>().join(", ");
        let artifacts = {
            let mut statement = transaction
                .prepare(&format!(
                    "DELETE FROM evidence_artifacts
                     WHERE id IN ({placeholders})
                     RETURNING id, orchestration_run_id, task_id, agent, provider, scenario,
                               commit_hash, branch, worktree, captured_at_ms, kind, status,
                               byte_size, original_ref, thumbnail_ref, thumbnail_byte_size,
                               pinned, expires_at_ms"
                ))
                .map_err(|error| {
                    StoreError::sqlite("could not prepare the evidence artifact delete", error)
                })?;
            let rows = statement
                .query_map(params_from_iter(ids), evidence_artifact_from_row)
                .map_err(|error| {
                    StoreError::sqlite("could not delete the evidence artifacts", error)
                })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| {
                    StoreError::sqlite("could not read the deleted evidence artifacts", error)
                })?
        };
        transaction.commit().map_err(|error| {
            StoreError::sqlite("could not finish the evidence artifact delete", error)
        })?;
        Ok(artifacts)
    }

    pub fn delete_evidence_run(&self, run_id: &str) -> Result<Vec<EvidenceArtifact>> {
        let mut connection = self.lock_write()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                StoreError::sqlite("could not begin the evidence run delete", error)
            })?;
        let artifacts = {
            let mut statement = transaction
                .prepare(
                    "DELETE FROM evidence_artifacts
                     WHERE orchestration_run_id = ?
                     RETURNING id, orchestration_run_id, task_id, agent, provider, scenario,
                               commit_hash, branch, worktree, captured_at_ms, kind, status,
                               byte_size, original_ref, thumbnail_ref, thumbnail_byte_size,
                               pinned, expires_at_ms",
                )
                .map_err(|error| {
                    StoreError::sqlite("could not prepare the evidence run delete", error)
                })?;
            let rows = statement
                .query_map([run_id], evidence_artifact_from_row)
                .map_err(|error| StoreError::sqlite("could not delete the evidence run", error))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| {
                    StoreError::sqlite("could not read the deleted evidence run", error)
                })?
        };
        transaction.commit().map_err(|error| {
            StoreError::sqlite("could not finish the evidence run delete", error)
        })?;
        Ok(artifacts)
    }
}

fn evidence_artifact_from_row(row: &Row<'_>) -> rusqlite::Result<EvidenceArtifact> {
    Ok(EvidenceArtifact {
        id: row.get(0)?,
        orchestration_run_id: row.get(1)?,
        task_id: row.get(2)?,
        agent: row.get(3)?,
        provider: row.get(4)?,
        scenario: row.get(5)?,
        commit_hash: row.get(6)?,
        branch: row.get(7)?,
        worktree: row.get(8)?,
        captured_at_ms: row.get(9)?,
        kind: row.get(10)?,
        status: row.get(11)?,
        byte_size: row.get(12)?,
        original_ref: row.get(13)?,
        thumbnail_ref: row.get(14)?,
        thumbnail_byte_size: row.get(15)?,
        pinned: row.get(16)?,
        expires_at_ms: row.get(17)?,
    })
}
