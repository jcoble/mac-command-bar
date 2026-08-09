# Phase-1 plan review — Opus 5 (independent agent)

Verdict: REWORK. 22 findings. Reproduced verbatim from the reviewer's report.

## BLOCKERS

1. Task 3's design contradicts the plan's own architecture paragraph, and cannot work.
   ACP v1 reports turn completion in the `session/prompt` RESPONSE (schema
   src/v2/conversion.rs:1755; PromptResponse src/v1/agent.rs:3268 carries only stop_reason),
   so `AcpClient::request` (acp_client.rs:349-371) blocks reading frames for the entire turn
   and `manager.prompt` (manager.rs:317-323) holds the per-session
   AsyncMutex<StructuredRuntimeHandle> the whole time. cancel_turn (manager.rs:489-494),
   respond_permission (manager.rs:355-360), steer (manager.rs:377-382), set_config
   (manager.rs:410-415) all take the same mutex → all deadlock for the length of a turn.
   Fix: real reader task — split SidecarProcess so a spawned reader owns stdout and
   dispatches frames to (a) HashMap<u64, oneshot::Sender<Value>> of pending requests and
   (b) the update sink; write_json keeps stdin behind its own lock; request = write + await
   oneshot, no read lock held.

2. Agent-to-client REQUESTS are still dropped, so any turn needing approval hangs forever —
   and Task 5 removes the PTY fallback. `session/request_permission` (src/v1/client.rs:2284)
   is a JSON-RPC request from the agent mid-turn; the agent MUST have all pending permission
   responses before completing the turn (src/v1/client.rs:839). Router must also route
   inbound requests: emit Approval { request_id, state: Requested, summary }
   (protocol.rs:495-499) and answer via the existing respond_permission path. Add a
   permission-frame test.

3. Task 6 Step 2 breaks TerminalSurface's documented invariant (TerminalSurface.svelte:1-19:
   hosts are NEVER wrapped in {#if} — gating destroys the xterm DOM on switch, the old
   shell's "my agent restarted when I clicked away" bug). Also TerminalSurface has no
   `readonly` prop (TerminalSurface.svelte:23-39) — the plan's snippet won't compile.
   Fix: keep the terminal layer mounted unconditionally; render the inspector as an overlay
   above it (like .structured overlays at ConversationSurface.svelte:224). Rewrite the
   Step 1 regex, which as written also matches the wrong shape.

## MAJOR

4. Task 5 Step 1's premise is wrong: scripts/agentConversationHandoff.test.mjs:12-96 does
   NOT pin PTY-presence handoff; it tests handoff request/receipt round-tripping, which
   Phase 1 keeps. Line 58's raw-mode assertion is about the EXPLICIT structured→terminal
   handoff. The file uses a manual runner, no node:test import — the plan's test snippets
   would throw ReferenceError. Fix: new source-contract file for +page.svelte;
   agentConversationHandoff.test.mjs must pass unmodified.

5. Task 5 Step 1's regex (doesNotMatch startNewSession...startOwned) contradicts Step 2's
   instruction to keep the plain-terminal path on service.startOwned inside the same
   function (createFreshSession produces agent: 'other'). Fix: assert the branch exists,
   not that startOwned is absent.

6. Task 5's replacement startNewSession drops behavior: the real function
   (+page.svelte:1130-1152) ends with frameControls?.showCenterPanel('session') and needs
   the hostFor lookup for the surviving plain-shell path. Provide the full replacement.

7. Deleting the force-raw branch unconditionally re-creates the double-writer hazard it
   guards (+page.svelte:985-993: two agents appending to one conversation — includes
   sessions restored from localStorage and native-CLI handoffs). Fix: keep the guard for
   actually-live PTYs; remove only the new-session PTY path.

8. Task 4 points the emitter install at the wrong file: agent_conversation/mod.rs has no
   manager construction and no AppHandle. Manager is built in main.rs:5318-5321 BEFORE
   tauri::Builder; the only viable site is the .setup(|app| …) closure at main.rs:5334
   with app.handle().clone() + a clone of the managed AgentRuntimeManager.

9. Emitting Turn { state: Started } immediately clears the frontend sending state:
   conversationService.ts:362-366 clears on ANY kind === 'turn'. Fix: narrow to
   payload.state !== 'started' and pin it in scripts/agentConversationStore.test.mjs.

10. Turn { turn_id } has no source: ACP v1 PromptResponse has no turnId
    (src/v1/agent.rs:3268-3271); acp_client.rs:145-148 reading result["turnId"] only
    succeeds against the test fixture (acp_client.rs:462). Fix: mint a client-side turn id
    at prompt time, store in session.active_turn_id, use for Started and Completed — also
    fixes canonical_event's turn_id (manager.rs:928) being permanently None.

11. Task 2 fixes a guessed root cause; spec section 6 says diagnose, don't guess.
    /usr/bin/sqlite3 exists (verified) and /usr/bin is in a Finder-launched app's PATH.
    The proposed resolver also probes on EVERY call (usage_db.rs:386,404,410 →
    per-operation spawns) and probe.is_ok() ignores exit status. Fix: split the task —
    land error surfacing + append resolved binary path to the error first; reproduce
    natively; fix what the message reports. If a fallback is kept, cache in OnceLock.

12. Double emission / sequence collision with terminal projection unaddressed: projection
    emits on the same channel with its own counter starting at 1
    (terminal_projection.rs:13,225); manager's counter also starts at 1 (manager.rs:172);
    reducers enforce strict contiguity and set desynchronized on gaps
    (conversationStore.svelte.ts:172-179). Projection starts after a native handoff
    (+page.svelte:1058). Fix: Task 4 must not pump when the session's writer lease is
    terminal; test asserting no emission while owner != Structured.

13. Task 3's test harness does not exist (fake_client_with_frames, client_request,
    block_on). The real harness at acp_client.rs:502-586 is a #[tokio::test] driving a
    shell-script fake process via fixture_manifest — and its fixture already emits a
    session/update frame before the prompt result (acp_client.rs:462), a ready-made
    positive case. Specify the harness or reuse the fixture.

## MINOR

14. Task 4 Step 4: first sequence is 1, not 0 (manager.rs:172). manager_with_active_session
    helper doesn't exist; plain manager.ensure(...) suffices.
15. ToolState variants are Started/Updated/Completed/Failed (protocol.rs:441-447), not
    Running. tool_call → Started, tool_call_update → Updated.
16. "sessionUpdate" as a method name is fiction (real wire method: session/update,
    src/v1/client.rs:2282; rpc.rs:356-392 is a hand-built test string). Accepting it is
    harmless; the second test tests fiction — drop or relabel defensive.
17. AcpRuntimeAdapter::client_mut already exists (private, Result-returning,
    providers/acp.rs:29-33). Prefer a passthrough pub fn set_update_sink on the adapter.
18. Task 1 understates the bug: ALL FIFTEEN Tauri browser commands take input: SomeStruct
    (browser.rs:1202-1340) and all fifteen frontend call sites pass unwrapped
    (browserBackend.ts:222-278). The native browser has never worked at all. Scope for
    fifteen fixes. (clear_browser_workspace_data browser.rs:1294 has no frontend caller.)
19. Vacuous tests: Task 2's fallback assertion is a tautology; Tasks 5/6 deserve at least
    one behavioral test each (store mode transitions; the Turn-state sending-flag fix).
20. Coverage gaps vs spec: reconnect on stream drop (emit Connection{Failed}+Error when
    next_json returns sidecar-exited, process.rs:133-135); ACP-start failure needs a
    retry affordance on the session card, not just rail.error; projection-only-for-external
    restriction never actually added; multi-session groundwork already satisfied — say so.
    Missing from Task 5's sweep: scripts/agentRuntimeContracts.test.mjs (pins ptySessionId
    reconciliation :38-84, mode 'raw' :128) and scripts/nextTerminalService.test.mjs
    (pins startOwned :143-232).
21. Seams: Tasks 5 and 6 cannot land separately as drawn (between them, raw mode mounts a
    terminal over a dead PTY). Merge them or land 6 first.
22. Unverified baselines (16 svelte errors / 252 cargo tests / 2 script failures) — capture
    from a real run into the receipt before Task 1.

## Anchors verified correct by the reviewer

browser.rs:1254-1260; tauriSource.ts:808-817; browserBackend.ts:241-243;
usageStore.svelte.ts:80-82; usage_db.rs sqlite_command:418; acp_client.rs:184-241,349-371,
502-586; manager.rs:58-62,190-263,239,265-308; protocol.rs:467-516;
conversationService.ts:381-395; +page.svelte force-raw :985-993 (plan said 983-1002),
startNewSession :1130-1152 (plan said 1128-1149); ConversationSurface.svelte :62/:223/
:272-277; ConversationHeader.svelte:18; main.rs:8254-8288. emit_payload confirmed to have
no production callers (tests only: manager.rs:1052,1078).
