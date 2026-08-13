# Suspend18 report

Status: Rust implementation complete; frontend follow-up is documented below.

## Scope

- Verified: one background interval starts with the app, ticks every 60 seconds, and uses a hard-coded ten-minute idle threshold (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:36-38`, `:162-180`; startup wiring at `tauri-svelte-preview/src-tauri/src/main.rs:5582-5585`).
- Verified: eligibility excludes the manager's active session, missing runtimes/native ids, providers without resume, non-ready/running sessions, one-shot work, pending permission/input, writer transitions, and recent activity (`manager.rs:2635-2651`). Every recorded event refreshes activity (`manager.rs:1417-1430`); activation refreshes it at `manager.rs:370-394`.
- Verified: suspend detaches the transport, preserves the owned record and native id, changes only the runtime state to `Suspended`, forces future activation through resume mode, and emits the existing disconnected session-state event (`manager.rs:966-1031`). Expected transport shutdown is suppressed instead of being reported as a failure (`manager.rs:1726-1735`).
- Verified: resume uses the existing activation path and validates that the provider returns the same native id (`manager.rs:415-450`). It restores `Ready`, reconnects the event pump, and emits the connected session-state event (`manager.rs:462-510`).
- Verified: snapshots expose `suspended: bool` (`tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs:583-590`; populated at `manager.rs:919-931`).
- Verified: this lane did not edit any frontend file and did not start, stop, or restart the real app.

## Red-green receipts

- Verified red: `RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test --manifest-path src-tauri/Cargo.toml suspend` exited 101 before production work. The compiler named the missing suspend constant, active-session field, activity field, suspend method, snapshot flag, and `Suspended` state.
- Verified intermediate red: the event-order assertion caught an intentional detach being misreported as `Failed`; the focused command exited 101 with `[Failed, Disconnected, Connected]`. The suspend-in-progress guard fixed that race.
- Verified green: the same focused command exited 0 with 7 passed, 0 failed. Tests are at `manager.rs:3244-3507` and cover active session, running turn, pending permission, pending input, runtime/process teardown with record/native-id retention, same-id ensure/resume/prompt, event order, and missing resume capability.
- Verified end-to-end: `ensure_after_suspend_resumes_with_same_native_session_id` proves one new session, one resume, one prompt, the same generation/native id, a completed turn, and exactly `Disconnected -> Connected` state events (`manager.rs:3388-3475`).

## Verification receipts

- Verified: `RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test --manifest-path src-tauri/Cargo.toml agent_conversation` exited 0: 85 passed, 0 failed.
- Verified: `RUSTFLAGS="-D warnings" cargo check --manifest-path src-tauri/Cargo.toml` exited 0.
- Verified: `cargo fmt` was run on every touched Rust file; `git diff --check` exited 0 before the final test-only assertion and formatting remained clean afterward.

## Provider resume support

- Verified policy: suspension is based only on the capability parsed from `agentCapabilities.sessionCapabilities.resume`; unknown or false never suspends (`tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:488-500`; `manager.rs:2643-2645`).
- Repository-grounded, not live-process verified: the pinned Codex adapter is version 1.1.9 and the pinned Claude adapter is version 0.23.1 (`tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs:10-11`). Both matching repository fixtures advertise `resume: true` (`tauri-svelte-preview/src-tauri/fixtures/agent_conversation/codex/all-categories.jsonl:2`, `tauri-svelte-preview/src-tauri/fixtures/agent_conversation/claude/all-categories.jsonl:2`). Therefore both will suspend when their live initialize response continues to advertise resume; either one will automatically stay running if that capability is absent.
- Assumed: live packaged-adapter capability responses were not sampled because this lane was forbidden from touching real processes.

## Frontend follow-up

- Verified follow-up needed: the Rust snapshot now serializes `suspended`, but the untouched TypeScript snapshot interface has no field for it (`tauri-svelte-preview/src/lib/shell/conversation/conversationTypes.ts:630-633`). A frontend lane should add and consume that flag.
- Verified current rendering caveat: the deliberate disconnected event enables existing click/send revival (`conversationActivation.ts:35-47`, `:76-109`; `conversationService.ts:443-479`), but session presence maps disconnected to disconnected (`sessionPresence.ts:121-125`) and the rail maps that to `Stopped` (`WorktreeAgentRow.svelte:127-150`). Until the frontend lane consumes `suspended`, a suspended session can briefly read `Stopped` even though its owner remains structured and its record is resumable.
- Assumed integration caveat: the manager's active-session marker updates when backend activation runs (`manager.rs:370-394`). An already-connected rail selection can take the frontend's view-only return without a backend call (`tauri-svelte-preview/src/routes/next/+page.svelte:623-679`). A frontend follow-up should send a lightweight activation notification on every actual rail selection so the ten-minute policy always protects the visible session, including connected view-only switches.
