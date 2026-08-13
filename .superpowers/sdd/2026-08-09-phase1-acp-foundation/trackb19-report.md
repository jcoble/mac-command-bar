# Track B report: manager rewire

Status: implementation complete; required Track B and full Rust verification green.

## Outcome

- `sessions.db` in Application Support is the backend session authority. Manager recovery, list snapshots, lifecycle rows, and event history now come from `mcb-core::SessionStore`; live state is an overlay (`main.rs:5572-5584`, `manager.rs:1231-1266`, `manager.rs:1800-1891`).
- Adapter processes are pooled by provider only when initialization explicitly advertises multi-session safety. The same pool mechanism uses an isolated key for fallback providers (`manager.rs:90-98`, `manager.rs:556-638`).
- Runtime suspension starts only at full quiescence, uses one delayed 30-second check plus a 30-second backstop, retains the native session id, and resumes lazily without silently creating a replacement session (`manager.rs:207-263`, `manager.rs:1338-1394`, `manager.rs:3389-3408`).
- The persisted journal implementation and the ten-minute policy are deleted (`agent_conversation/mod.rs:1-8`, `manager.rs:38-39`). `git status --short` shows no frontend source file change.

## Delivered behavior

### SQLite session truth

- Startup opens `Application Support/sessions.db`, runs the one-time importer only when the store is empty, recovers durable rows/events, and validates the store-backed snapshot list (`main.rs:5572-5584`, `manager.rs:181-201`, `manager.rs:1800-1891`).
- Every manager lifecycle path writes through the shared row persistence helper: create/ensure, activation, configuration, suspension, resumption, closure, and recovered-session normalization (`manager.rs:427-519`, `manager.rs:1216-1238`, `manager.rs:1347-1377`, `manager.rs:1747-1792`, `manager.rs:1933-2011`, `manager.rs:2930-2972`).
- Conversation events append with their canonical sequence and immediately enforce the 10,000-event cap using the store's single `DELETE` (`manager.rs:1993-2010`).
- Snapshot lists start from ordered store rows and overlay currently live manager state (`manager.rs:1231-1266`).
- The importer accepts the current canonical event envelope as JSONL and an owned-session export as JSON, imports only into an empty store, and caps imported event history (`legacy_import.rs:15-126`). Its round-trip test builds fixtures with the current event shape before the old journal module is removed (`legacy_import.rs:169-246`).

### Shared adapter scope

- `AdapterPoolKey::Shared(provider)` and `AdapterPoolKey::Isolated(provider, owned_id)` are the one pool mechanism; explicit `sessionCapabilities.multiSession` chooses only the key shape (`manager.rs:90-98`, `manager.rs:556-638`, `manager.rs:3361-3363`, `protocol.rs:128-136`).
- Multi-session requests address the native session id for new/load/resume/config/prompt/cancel, and inbound events route by native session id (`providers/acp.rs:78-152`, `providers/acp_client.rs:234-281`, `providers/acp_client.rs:587-633`, `providers/acp_client.rs:646-785`). The former single-session wrappers were removed from `providers/mod.rs:1-18`.
- Suspending or closing removes the session from its pool. The transport is closed only when the final pool member leaves (`manager.rs:274-331`).
- Reaper markers now record provider scope (`provider:<provider>`), with readback coverage in `reaper.rs:604-610`.

### Thirty-second quiescent suspend

- The only policy constants are 30 seconds (`manager.rs:38-39`). The periodic check starts after 30 seconds, not immediately (`manager.rs:207-220`).
- A quiescence transition records one generation and schedules one delayed check; repeated observations do not create timer storms (`manager.rs:225-263`).
- Eligibility requires a ready runtime with no active turn, no one-shot request, no permission or input callback, no transition, no live tool call, and no child/background activity (`manager.rs:3389-3408`). Any activity clears the clock and advances the generation (`manager.rs:1988-1991`).
- Suspension persists the native id/state, settles deferred callbacks idempotently, drops pool membership, and records `suspended`; resume uses load/resume and produces an error event if the stored session cannot be restored (`manager.rs:1338-1394`, `manager.rs:2892-2972`).
- Late permission/input responses return an explicit stale-request error. Turn cancellation remains the protocol-native session cancellation path (`manager.rs:934-1048`, `manager.rs:1189-1212`).

## Files touched

- `core/src/session_store.rs` — in-memory constructor for manager tests while preserving WAL for file stores.
- `tauri-svelte-preview/src-tauri/Cargo.lock` — records the core store's SQLite dependency in this crate lock.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs` — default multi-session capability.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/journal.rs` — deleted persisted journal/ring implementation.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/legacy_import.rs` — one-time legacy import and fixture test.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs` — store authority, recovery, pool, quiescence, lifecycle/event persistence, and regression tests.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs` — removes journal module; registers importer.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs` — explicit multi-session capability and suspended state.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs` — shared-client session-addressed operations and routing.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs` — multi-session transport routing and per-session closure.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/providers/mod.rs` — removes obsolete single-session wrappers.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/reaper.rs` — provider-scope process marker semantics and test.
- `tauri-svelte-preview/src-tauri/src/main.rs` — Application Support database startup/recovery and 30-second backstop.
- This report.

## Verification receipts

All commands ran serially with no real app process restart.

- `RUST_TEST_THREADS=1 RUSTFLAGS='-D warnings' cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml agent_conversation` — **98 passed, 0 failed, 265 filtered**.
- `RUST_TEST_THREADS=1 RUSTFLAGS='-D warnings' cargo test --manifest-path core/Cargo.toml session_store` — **10 passed, 0 failed**.
- `RUSTFLAGS='-D warnings' cargo check --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml` — **passed**.
- `RUSTFLAGS='-D warnings' cargo check --manifest-path core/Cargo.toml` — **passed**.
- `RUST_TEST_THREADS=1 RUSTFLAGS='-D warnings' cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml tests::agent_conversation_response_command_names_match_frontend_invokes_and_registration -- --exact` — **1 passed, 0 failed**; frontend invokes and registered response commands have zero dead entries.
- `RUST_TEST_THREADS=1 RUSTFLAGS='-D warnings' cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml` — **359 passed, 0 failed, 4 ignored**. A first integration-smoke attempt returned one unrelated language-server result mismatch; the unchanged clean retry passed. The ignored tests are explicitly opt-in live/performance smokes.
- `rustfmt --edition 2021 --check <all touched Rust files>` — **passed**.
- `git diff --check` — **passed**.
- The full-suite dependency link to the assembly worktree was removed after the run; `tauri-svelte-preview/node_modules` is absent in this worktree.

Named Track B regression tests present and green:

- `store_row_updated_on_every_lifecycle_transition`
- `events_persisted_and_capped`
- `importer_roundtrip_from_legacy_fixtures`
- `restart_recovers_sessions_from_store`
- `pool_shares_one_process_across_two_sessions`
- `last_session_close_kills_process`
- `grace_starts_only_at_quiescence`
- `approval_lease_never_expires`
- `suspend_after_30s_quiescence`
- `rapid_followup_within_grace_keeps_runtime`
- `load_failure_surfaces_resumability_error`
- `late_approval_gets_stale_result`

## Legacy deletion and grep receipts

- `rg -n "AgentEventJournal|mod journal|journal::" tauri-svelte-preview/src-tauri/src/agent_conversation tauri-svelte-preview/src-tauri/src/main.rs` — **no matches**.
- `journal.rs` is deleted; only `legacy_import` is registered in `agent_conversation/mod.rs:1-8`.
- `rg -n "IDLE_RUNTIME_SUSPEND|from_secs\\(600\\)|10 \\* 60" manager.rs main.rs` — only the two 30-second constants and their call sites match; **no 600-second/ten-minute policy remains**.
- `git diff --check` — no whitespace errors.

## Frontend fan-out follow-ups

No frontend file was changed here, per Track B's exception. The fan-out lane must cut these remaining browser-side truths over to the authoritative backend list:

- `src/lib/shell/stores/sessionRailStore.svelte.ts:11-18` declares local storage as the store's only I/O and requires every mutator to persist.
- `src/lib/shell/stores/sessionRailStore.svelte.ts:33-34, 63-104` defines, writes, and reads `mac-command-bar.next.owned-sessions`.
- `src/lib/shell/stores/sessionRailStore.svelte.ts:108-138, 156-180` persists hydration, add/update/remove, and activation state.
- `src/routes/next/+page.svelte:1249-1261` hydrates the rail from `loadStoredOwned()` rather than the backend session list.
- `src/routes/next/+page.svelte:820-838` adopts a session and writes the PTY identity through the local store.
- `src/lib/shell/ownedSessions.ts:10-28, 118-136` does not yet model backend `suspended` state and still includes the old stopped/exited vocabulary.

## Contract deviations

- The removed Rust journal was an in-memory ring, not a disk journal, so there was no prior Rust journal file to copy from real Application Support data. The importer fixture therefore uses the exact current canonical event serialization. Existing owned-session persistence is WebView local storage, which backend Rust cannot read without the explicitly excluded frontend fan-out; the importer supports an Application Support JSON export if that fan-out supplies one before deleting the browser key.
- Red-state output was not separately retained before the implementation changes. All requested named regression tests were added and are green, but this report cannot claim a preserved red-first receipt.
- The first full-suite run exposed a pre-existing integration-smoke fluctuation; no unrelated language-server code was changed. The unchanged retry passed the full suite.

## Handoff state

- No files were staged, committed, or pushed.
- No frontend files were modified.
- No worktree or browser session was created by this lane.
