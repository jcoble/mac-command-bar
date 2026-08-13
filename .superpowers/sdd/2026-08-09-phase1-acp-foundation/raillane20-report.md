# Rail lane 20 report: backend list, rail cutover, and lazy lifecycle

Status: implementation complete; required automated checks are green. No commit, staging, push, app launch, browser session, or real provider process was used.

## Outcome

- Verified: rail hydration now projects the SQLite session list, and selection reads the stored snapshot/events plus the stored draft without ensuring or activating a runtime (`src/routes/next/+page.svelte:655`, `:703`; `src/lib/shell/conversation/conversationService.ts:394`).
- Verified: sending is the only normal interaction that activates the structured runtime; startup model/access configuration travels in the same send request (`src/lib/shell/conversation/conversationService.ts:460`, `:507`; `src-tauri/src/agent_conversation/mod.rs:89`).
- Verified: full quiescence suspends inline with no timer, generation-delay, or periodic backstop. Pending permission/input, live tools, active turns, and background work block teardown (`src-tauri/src/agent_conversation/manager.rs:1289`, `:3408`).
- Verified: durable rail metadata is stored with the session row through one metadata command, and `sessionRailStore.svelte.ts` is now an I/O-free in-memory projection (`src-tauri/src/agent_conversation/manager.rs:1263`; `src/lib/shell/stores/sessionRailStore.svelte.ts:1`).

## Command contracts

### `list_agent_conversation_sessions`

- Request: none.
- Response: `AgentConversationSessionRecord[]` (`src-tauri/src/agent_conversation/protocol.rs:630`).
- Persisted fields: owned id, provider, model, effort, cwd, worktree, branch, title, project, state, suspended, creation/activity timestamps, native id, and flattened rail metadata.
- Live overlay: active turn, pending permission, pending input, and current runtime state (`src-tauri/src/agent_conversation/manager.rs:1206`).
- Registration: `src-tauri/src/main.rs:5733`; frontend invoke: `src/lib/tauriSource.ts:1263`.

### `list_agent_conversation_events`

- Request: `ownedId`, optional `fromSequence` (default 0).
- Response: canonical stored `AgentConversationEvent[]`, read from SQLite without runtime activation (`src-tauri/src/agent_conversation/mod.rs:249`; `manager.rs:1245`).
- Registration: `src-tauri/src/main.rs:5734`; frontend invoke: `src/lib/tauriSource.ts:1269`.

### `update_agent_conversation_session_meta`

- Request: owned id, model, effort, and one `AgentConversationSessionMeta` payload (`protocol.rs:600`, `:621`).
- Response: the updated `AgentConversationSessionRecord` (`mod.rs:258`).
- PTY identity, worktree/branch/title/project, origin/source, completion/settlement, task/PR, and scan summaries share this one path; there is no browser-storage metadata writer.
- Registration: `src-tauri/src/main.rs:5735`; frontend invoke: `src/lib/tauriSource.ts:1278`.

### Changed send/ensure boundary

- `ensure_agent_conversation` creates/updates only the durable manager row; it does not activate a process (`src-tauri/src/agent_conversation/mod.rs:81`). It remains only for new-row/adoption preparation.
- `send_agent_conversation_message` activates, applies first-send configuration when supplied, then prompts (`mod.rs:89`; request extension at `protocol.rs:501`).

## Lifecycle deletion receipts

- Verified: `IDLE_RUNTIME_SUSPEND_TICK`, `IDLE_RUNTIME_SUSPEND_AFTER_MS`, idle task startup, delayed checks, and quiescence-generation machinery are absent. Receipt: `idle grace machinery matches: 0`.
- Verified tests: `teardown_at_quiescence` (`manager.rs:4294`) and `quiescence_gates_teardown` (`:4538`).
- Preserved gates: `approval_lease_never_expires` (`:4329`), `load_failure_surfaces_resumability_error` (`:4649`), and `late_approval_gets_stale_result` (`:4680`).
- Read-only no-spawn proof: `selecting_and_reading_session_never_spawns_runtime` (`manager.rs:4193`) exercises list, snapshot, events, and metadata reads while asserting no transport/process fixture exists.

## Frontend cutover receipts

- SQLite hydration and one-time adoption: `src/routes/next/+page.svelte:1225-1259`. The legacy key is removed only after every parsed record is found in the backend list.
- Exact legacy key matches in active project files: `1`, at the adoption constant (`+page.svelte:174`).
- Ensure calls in `selectOwned`: `0` (`+page.svelte:655-715`).
- Browser-storage matches in `sessionRailStore.svelte.ts`: `0`.
- Suspended backend records project as suspended runtime state (`ownedSessions.ts:273-292`) and render idle with 0.6 dot opacity (`WorktreeAgentRow.svelte:102-103`, `:606`).
- The obsolete browser-seeded performance measurement script was deleted because it encoded the removed local-storage truth. Touched executable tests were renamed from `.mjs` to `.ts`.

## Files touched

- Rust: `src-tauri/src/agent_conversation/{manager.rs,mod.rs,protocol.rs}`, `src-tauri/src/main.rs`.
- Frontend: `src/lib/tauriSource.ts`, `src/lib/shell/{ownedSessions.ts,stores/sessionRailStore.svelte.ts,conversation/conversationService.ts,components/WorktreeAgentRow.svelte}`, `src/routes/next/+page.svelte`.
- Tests/tooling: `package.json`, `scripts/{agentRuntimeContracts.test.ts,conversationSendRecovery.test.ts,newSessionSubmission.test.ts,ownedSessions.test.ts,railBackendCutover.test.ts,sessionRowActions.test.ts}`; removed their touched `.mjs` predecessors and `scripts/perffix18Measure.mjs`.
- Report: this file.

## Verification receipts

Verified on 2026-08-13 in this worktree:

```text
RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml agent_conversation
test result: ok. 100 passed; 0 failed; 0 ignored; 0 measured; 265 filtered out
```

```text
cargo test --manifest-path core/Cargo.toml session_store
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 36 filtered out
```

```text
RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml tests::agent_conversation_response_command_names_match_frontend_invokes_and_registration -- --exact
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 364 filtered out
```

```text
pnpm run check:svelte
Files the /next shell owns: 0 error(s), 0 warning(s).
Elsewhere in the project (not checked by this gate): 16 error(s) — the old shell's own backlog.
```

```text
pnpm run test:conversation-send-recovery
conversationSendRecovery.test.ts passed
pnpm run test:new-session-submission
newSessionSubmission.test.ts passed
pnpm run test:session-rail
sessionRowActions: ok; ownedSessions tests passed; supporting rail suites passed
pnpm run test:rail-backend-cutover
rail backend cutover tests passed
node --experimental-strip-types scripts/agentRuntimeContracts.test.ts
agent runtime contract tests passed
```

```text
rustfmt --edition 2021 --check <four touched Rust files>
git diff --check
both exited 0 with no output
```

## Inputs and deferred items

- Verified: `trackb19-report.md` was present and used for lifecycle/store context.
- Assumed: `raillist19-report.md` was absent at the supplied path, so the lane brief's exact source map was treated as authoritative.
- Deferred: native visual/performance proof was not run because this lane explicitly forbids launching the real app. No UI geometry or animation was changed.
