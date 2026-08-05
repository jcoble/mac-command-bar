# A6 WorkflowEngine receipt

## Outcome

Work Package A6 is implemented in the Tauri backend. The engine owns explicit, idempotent workflow transitions; validates and schedules a versioned DAG; dispatches only through the injected `AgentRuntimePort`; enforces the injected worktree/file lease port and policy budgets; accepts only typed structured receipts; and rebuilds from the existing orchestration JSONL ledger.

Stop result: no stop condition was triggered. There is one ledger, one JSONL path, no prose/regex gate authority, no direct child process path, and stale multi-writer transitions are rejected by expected-sequence comparison under the shared ledger writer lock.

No commit was created.

## Files and symbols

- `tauri-svelte-preview/src-tauri/src/workflow.rs:24` — `WorkflowDefinitionV1` and the workflow contract/policy types.
- `tauri-svelte-preview/src-tauri/src/workflow.rs:394` — named structured contracts `ImplementationReceipt`, `ReviewReceipt`, `SpecComplianceReceipt`, `VerificationReceipt`, and `PlanReceipt`.
- `tauri-svelte-preview/src-tauri/src/workflow.rs:544` — injected `AgentRuntimePort`; the `AgentRuntimeManager` adapter activates ACP, maps model/effort/permission through advertised provider capability categories, and submits a structured task envelope.
- `tauri-svelte-preview/src-tauri/src/workflow.rs:668` — injected `WorktreeLeasePort`; overlapping write paths are rejected and active leases are rebuilt from replayed node state.
- `tauri-svelte-preview/src-tauri/src/workflow.rs:803` — pure `WorkflowReducer`.
- `tauri-svelte-preview/src-tauri/src/workflow.rs:827` — deterministic `WorkflowScheduler` with lexical ready ordering over both `dependsOn` and explicit edges.
- `tauri-svelte-preview/src-tauri/src/workflow.rs:863` — `WorkflowPolicy`: definition/cycle validation, typed output validation, provider allow-listing, delegation parent/depth/role/provider authorization.
- `tauri-svelte-preview/src-tauri/src/workflow.rs:1082` — `WorkflowEngine` and all required entry points: `create_run`, `start`, `pause`, `resume`, `cancel`, `retry_node`, `skip_node`, `approve_gate`, `submit_result`, `dispatch_ready`, `rebuild_from_events`, and `list_runs`.
- `tauri-svelte-preview/src-tauri/src/orchestration.rs:103` — additive, serde-defaulted workflow fields on `OrchestrationEvent`.
- `tauri-svelte-preview/src-tauri/src/orchestration.rs:258` — the legacy record entry point refuses workflow transitions; the frontend cannot append them directly.
- `tauri-svelte-preview/src-tauri/src/orchestration.rs:276` — shared writer lock.
- `tauri-svelte-preview/src-tauri/src/orchestration.rs:283` — idempotent workflow append, deterministic sequence assignment, and stale-snapshot rejection.
- `tauri-svelte-preview/src-tauri/src/orchestration.rs:335` — workflow replay reads the existing ledger.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:74` — capability snapshot accessor used for role-to-provider config mapping.
- `tauri-svelte-preview/src-tauri/src/main.rs:1418` — ten Tauri workflow command wrappers.
- `tauri-svelte-preview/src-tauri/src/main.rs:5297` — one managed `WorkflowEngine` sharing the one managed `AgentRuntimeManager`.
- `tauri-svelte-preview/src-tauri/src/main.rs:5382` — workflow commands registered in `generate_handler!`.

No file under `tauri-svelte-preview/src/lib` or `tauri-svelte-preview/src/routes` was edited by A6.

## Ledger migration

The existing `orchestration-events.jsonl`, `MAC_COMMAND_BAR_ORCHESTRATION_EVENTS` override, schema version, legacy event kinds, CLI/demo presets, and `OrchestrationRun` presentation reducer are preserved.

`OrchestrationEvent` gained optional/defaulted fields for workflow identity/version, node and node-run identity, parent node-run identity, `ownedId`, attempt/depth, input hash, output contract, structured artifact references, gate, lease, provider instance, provenance, sequence, idempotency key, and the structured workflow payload. Old rows deserialize with defaults.

Workflow transitions are full structured run snapshots in additive `workflow.*` rows in that same ledger. `WorkflowReducer` is pure: ordered rows reduce to the latest run snapshot and set the replayed sequence. Appends share one process writer lock with legacy appends. Workflow appends allocate `max(sequence)+1`, deduplicate `(runId,idempotencyKey)`, and compare the snapshot's expected `lastSequence` with the current sequence before writing. A stale writer receives a typed ledger error and cannot overwrite a newer transition.

## Determinism and policy

- Definition version, unique role/node IDs, references, self-dependencies, and cycles are validated before run creation.
- Input JSON is canonicalized by sorted object keys and frozen with a stable FNV-1a 64-bit hash.
- Ready nodes are ordered lexically and become ready only when all dependency and edge predecessors are completed or skipped.
- Global, per-workflow, and per-provider active counts are checked before dispatch.
- Depth, attempts, wall time, node timeout, tokens, tool terminals, and worktree allocations produce `WorkflowError::BudgetExhausted(WorkflowFailure)`; the associated failure transition is appended where the run/node changes state.
- Retry requires a failed node, remaining node/role/workflow attempts, and a role-allow-listed failure code when the role declares retryable codes.
- Failure descendants are cancelled when `cancelDescendantsOnFailure` is enabled.
- Phase is derived from run/node state plus the active node's configured role; input/result prose is never inspected.
- Gate approval requires `{ approved: boolean }` structured input.
- Results must deserialize into the named receipt structure. A string, hostile prose, missing fields, or wrong field types is refused.
- Active worktree/file leases are reconstructed from replayed node state. Overlapping write paths in one cwd are rejected. Dispatch cannot bypass the lease port.

## Command registration receipt

Registered backend commands:

1. `list_workflow_runs`
2. `create_workflow_run`
3. `start_workflow_run`
4. `pause_workflow_run`
5. `resume_workflow_run`
6. `cancel_workflow_run`
7. `retry_workflow_node`
8. `skip_workflow_node`
9. `approve_workflow_gate`
10. `submit_workflow_result`

The existing public `record_orchestration_event` command remains for legacy events but rejects `workflow.*` kinds and any event carrying `workflowId`.

## Exact `tauriSource.ts` wrapper additions

These additions are a receipt only; A6 did not edit `src/lib/tauriSource.ts`. They assume the frontend contract lane supplies `WorkflowDefinitionV1` and `WorkflowRunRecord` types matching the serialized Rust contracts.

```ts
export async function listWorkflowRunsFromTauri(): Promise<WorkflowRunRecord[] | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord[]>('list_workflow_runs');
}

export async function createWorkflowRunFromTauri(
  definition: WorkflowDefinitionV1,
  input: Record<string, unknown>,
  idempotencyKey: string
): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('create_workflow_run', { definition, input, idempotencyKey });
}

export async function startWorkflowRunFromTauri(runId: string, idempotencyKey: string): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('start_workflow_run', { runId, idempotencyKey });
}

export async function pauseWorkflowRunFromTauri(runId: string, idempotencyKey: string): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('pause_workflow_run', { runId, idempotencyKey });
}

export async function resumeWorkflowRunFromTauri(runId: string, idempotencyKey: string): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('resume_workflow_run', { runId, idempotencyKey });
}

export async function cancelWorkflowRunFromTauri(runId: string, idempotencyKey: string): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('cancel_workflow_run', { runId, idempotencyKey });
}

export async function retryWorkflowNodeFromTauri(runId: string, nodeId: string, idempotencyKey: string): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('retry_workflow_node', { runId, nodeId, idempotencyKey });
}

export async function skipWorkflowNodeFromTauri(runId: string, nodeId: string, idempotencyKey: string): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('skip_workflow_node', { runId, nodeId, idempotencyKey });
}

export async function approveWorkflowGateFromTauri(
  runId: string,
  nodeId: string,
  approval: { approved: boolean; [key: string]: unknown },
  idempotencyKey: string
): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('approve_workflow_gate', { runId, nodeId, approval, idempotencyKey });
}

export async function submitWorkflowResultFromTauri(
  runId: string,
  nodeId: string,
  result: ImplementationReceipt | ReviewReceipt | SpecComplianceReceipt | VerificationReceipt | PlanReceipt,
  idempotencyKey: string
): Promise<WorkflowRunRecord | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<WorkflowRunRecord>('submit_workflow_result', { runId, nodeId, result, idempotencyKey });
}
```

## Test receipt

`RUST_TEST_THREADS=1 cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml workflow -- --nocapture`

PASS — 11 passed, 0 failed:

- `workflow_budget_and_timeout_failures_are_typed`
- `workflow_concurrency_provider_depth_and_attempt_policy_are_bounded`
- `workflow_dag_validation_rejects_cycles_and_unknown_references`
- `workflow_engine_idempotency_replay_and_audit_sequence_use_one_ledger`
- `workflow_phase_is_derived_from_node_state_and_role_not_prose`
- `workflow_ready_ordering_is_lexical_and_dependency_bound`
- `workflow_reducer_restart_replay_equivalence_and_audit_order`
- `workflow_retry_cancel_skip_and_gate_transitions_are_state_based`
- `workflow_role_capability_mapping_and_delegation_authorization_are_enforced`
- `workflow_structured_output_refuses_hostile_prose`
- `workflow_worktree_file_lease_conflicts_are_rejected`

`RUST_TEST_THREADS=1 cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml orchestration -- --nocapture`

PASS — 6 passed, 0 failed. Legacy JSONL parsing/reduction and demo behavior remain intact.

`RUST_TEST_THREADS=1 cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml agent_conversation -- --nocapture`

PASS — all 41 passed, 0 failed.

Only non-failing dead-code warnings were emitted; there were no compile errors or test failures in the final runs.

## Recovery evidence

`workflow_engine_idempotency_replay_and_audit_sequence_use_one_ledger` uses a temporary instance of the real orchestration JSONL path with fake runtime, lease, clock, and ID ports. It proves:

- canonical-equivalent input plus the same create idempotency key returns the same run;
- repeating `start` does not dispatch a second child;
- workflow audit sequences are contiguous and ordered;
- a deliberately stale expected sequence is rejected;
- top-level workflow rows contain node ownership identity;
- `rebuild_from_events()` equals `list_runs()` after replay;
- the temporary ledger is deleted and the prior environment is restored.

No test starts a provider process or accesses the network.

## Cleanup

- Worktree cleanup: no worktree was created by A6. The user-provided active worktree `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave` was not removed.
- Browser cleanup: no browser or Playwright session was opened.
- Process cleanup: no runtime child process was started by the workflow tests; fake ports were used.

## Deviations

The current backend exposes list/remove/archive worktree operations but no create-worktree authority API. `dedicated-new` therefore returns `WorkflowError::PolicyDenied` instead of creating a worktree or bypassing the authority. `read-only-current`, `shared-current`, and preselected `dedicated-existing` lease paths are enforced. This is a safe operational limitation, not a second process/store path; wiring a future create-worktree authority implementation requires only a different injected `WorktreeLeasePort`.

Tracker note: the required TSK-808 Notion status check could not reach `api.notion.com` in the network-restricted environment. The user-provided active task/branch and approved packet were used as authority; task status was not mutated.
