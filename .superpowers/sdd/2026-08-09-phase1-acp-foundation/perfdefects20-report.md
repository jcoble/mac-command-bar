# Performance defects lane 20 report

Status: implementation and required automated verification complete. No commit, staging, push, app launch, browser session, port 5177 binding, or real provider process was used.

## Outcome

- Verified in source and tests: a first structured-session selection now performs one bounded snapshot read instead of two 10,000-event reads, and the snapshot is the newest 2,000 events selected and ordered in one SQLite statement.
- Verified in source and tests: stored transcript replay no longer changes presence history; rail presence is based on the backend row plus events received live.
- Verified in source and tests: new-session provider/menu configuration is copied once when the draft opens, so menu open is an in-memory O(items) render with no invoke or subscription to the whole conversation map.
- Verified in pure geometry tests: the Session History menu anchors to the actual action button, flips left at the right boundary, and clamps inside its Dockview containing block.
- Assumed, per the lane prohibition on launching the real app: owner-visible latency, absence of the flash, native menu responsiveness, and final native popover pixels were not re-observed in the desktop UI.

## Defect receipts

### 1. First session click is slow after app start

- Root cause: `loadConversationForRead` read a snapshot and then immediately invoked `list_agent_conversation_events` from sequence zero, so the same stored journal was decoded and rebuilt twice (`tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts`, previous lines 393-403; cutover contract in `raillane20-report.md`). The backend snapshot itself used the 10,000-row full list (`manager.rs`, previous snapshot implementation), and manager startup also decoded a dead 10,000-event `frontend_events` cache that had no reader.
- Fix: `SessionStore::list_recent_events` limits newest rows and restores ascending transcript order inside one SQL statement (`core/src/session_store.rs:345-372`). Snapshot reads use that 2,000-event tail (`manager.rs:1188-1201`, `:1261-1273`), selection applies that one snapshot only (`conversationService.ts:393-397`), and the unread `frontend_events` cache/startup load was deleted (`manager.rs:1830-1905`, `:2020-2044`). The frontend reducer seeds itself immediately before a bounded window so omitted older rows do not create a false gap (`conversationStore.svelte.ts:320-340`).
- Proof: core paging boundary test (`session_store.rs:754-775`); 10,000-store/2,000-snapshot boundary test (`manager.rs:4075-4120`); bounded snapshot repair test (`manager.rs:5621-5657`); frontend bounded-window test (`scripts/agentConversationStore.test.ts:157-185`); rail contract asserts the redundant list wrapper is absent (`scripts/railBackendCutover.test.ts:19-20`).

### 2. Presence flashes while a transcript loads

- Root cause: creating the empty conversation object during selection made `WorktreeAgentRow` discard the backend `runtimeState` and temporarily derive a disconnected/stopped outline; snapshot replay then restored suspended/idle. The snapshot replay also called `recordConversationPresenceEvent` and `synchronizeSessionPresenceWork` for historical events, making a read mutate live presence.
- Fix: snapshot replay rebuilds transcript state without writing the presence store (`conversationStore.svelte.ts:320-389`). Rail presence keeps backend `runtimeState`, pending-permission, and active-turn truth, augmented only by the live presence history and current send state (`WorktreeAgentRow.svelte:88-130`). The row-level synchronization effect that replayed conversation state back into presence was removed.
- Proof: the bounded snapshot test asserts stored turn replay leaves `sessionPresenceHistory` unchanged (`scripts/agentConversationStore.test.ts:157-185`); the rail cutover test pins suspended presence to the backend record (`scripts/railBackendCutover.test.ts:25`).

### 3. New-session dropdowns freeze the UI

- Root cause: the page kept `providerConfigs` as a deep reactive derivation over `Object.values(conversationSessions)` and passed it through `ShellOverlays` and `ThreadStartHost` into every mounted dropdown. Any conversation-store invalidation rebuilt provider arrays and menu models while the menu was open, coupling a local open/close interaction to the entire live conversation map.
- Fix: provider configuration is read only when a new draft opens (`+page.svelte:226-243`; `ShellOverlays.svelte:141-145`) and deep-copied into draft-owned stable state (`ThreadStartHost.svelte:20-34`). Dropdown open now reads the cached model/effort/access/project arrays; it performs no invoke and stays O(rendered items).
- Proof: `scripts/newSessionSubmission.test.ts:65-74` asserts draft-owned snapshots, the absence of the old reactive derivation, and no live provider-config prop on `ThreadStartHost`. The focused suite passed.

### 4. Session History actions pane is cut off

- Root cause: the prior sweepfix17 clamp used the whole window as its right/bottom boundary and anchored a 3-dot click to the entire history row. Inside Dockview's fixed-position containing block, that could choose/clamp a location outside the visible pane.
- Fix: `menuPoint` now measures the actual event current target and records all four Dockview containing-block edges (`SessionLibraryWorkspace.svelte:181-211`). `placeSessionContextMenu` flips and clamps in viewport coordinates, then translates into containing-block coordinates (`sessionLibraryContextMenu.ts:34-89`). The component stays hidden for its single measurement pass and renders at the measured placement (`SessionContextMenu.svelte:20-66`).
- Proof: `scripts/sessionContextMenuPlacement.test.ts:1-37` covers right-edge flip inside an offset Dockview block and bottom-edge flip in the viewport.

## General redundant-work sweep

- Deleted the unused backend `frontend_events` cache and its cold-start full-journal decode (`manager.rs:1830-1905`, `:2020-2044`).
- Deleted the second per-selection event-list invoke (`conversationService.ts:393-397`).
- Replaced the always-live new-session provider-config derivation with one read per draft open (`+page.svelte:226-243`).
- No unkeyed list touched in these defect paths required a change; the relevant model, root, branch, history, and action item loops are already keyed.

## Files touched

- Backend/data: `core/src/session_store.rs`; `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs`.
- Selection/presence: `conversationService.ts`, `conversationStore.svelte.ts`, `WorktreeAgentRow.svelte`, `scripts/agentConversationStore.test.ts`, `scripts/railBackendCutover.test.ts`.
- New session: `+page.svelte`, `ShellOverlays.svelte`, `ThreadStartHost.svelte`, `scripts/newSessionSubmission.test.ts`.
- Session History: `SessionLibraryWorkspace.svelte`, `SessionContextMenu.svelte`, `sessionLibraryContextMenu.ts`, `scripts/sessionContextMenuPlacement.test.ts`.
- Report: this file.

## Verification receipts

Fresh verification on 2026-08-13:

```text
RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test --manifest-path core/Cargo.toml session_store
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 36 filtered out
```

```text
RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml agent_conversation
test result: ok. 100 passed; 0 failed; 0 ignored; 0 measured; 265 filtered out
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
node --experimental-strip-types scripts/agentConversationStore.test.ts
agent conversation store tests passed
node --experimental-strip-types scripts/newSessionSubmission.test.ts
newSessionSubmission.test.ts passed
node --experimental-strip-types scripts/sessionContextMenuPlacement.test.ts
session context menu placement tests passed
node --experimental-strip-types scripts/railBackendCutover.test.ts
rail backend cutover tests passed
```

```text
cargo fmt ran for both touched manifests; unrelated formatter spillover was removed from the diff.
git diff --check
exit 0, no output
```
