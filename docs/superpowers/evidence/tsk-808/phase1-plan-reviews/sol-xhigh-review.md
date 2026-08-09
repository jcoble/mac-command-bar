390,677
Verdict: REWORK. Tasks 3–6 cannot safely deliver the ACP-only foundation: the transport design blocks approvals and cancellation, external sessions can acquire two writers, and several required event and recovery paths are absent. The browser diagnosis is correct; the usage diagnosis remains an unproven hypothesis.

1. BLOCKER — Task 3 does not implement the dedicated ACP reader described by the architecture.

   Evidence: The plan promises a dedicated reader and pending-request router at `docs/superpowers/plans/2026-08-09-phase1-acp-foundation.md:7`, but Task 3 only calls `route_stray_frame` from existing request loops at `:225-229` and `:295-304`. Today the sole reader is `AcpClient::request` at `src-tauri/src/agent_conversation/providers/acp_client.rs:349-371`; `SidecarProcess::next_json` also requires exclusive `&mut self` access at `providers/process.rs:126-139`.

   `session/prompt` does remain pending for the turn: ACP’s response reports why processing stopped (`agent-client-protocol-schema-1.5.0/src/v1/agent.rs:3259-3270`), and the existing fixture sends the update before the prompt response (`acp_client.rs:451-468`). Therefore simple deltas can flow while a prompt is in flight, but there is no reader between requests or for background frames.

   Fix: split stdin/stdout ownership and create one reader task per sidecar. Route response IDs through a pending `oneshot` map and notifications/requests through channels. No request method should call `next_json` directly.

2. BLOCKER — The runtime mutex causes a real approval/cancellation deadlock.

   Evidence: `AgentRuntimeManager::prompt` holds the runtime `AsyncMutex` across the entire pending prompt at `manager.rs:317-323`. Permission responses require that same mutex at `:353-361`, user-input responses at `:364-372`, and cancellation at `:478-494`. ACP agents can issue `session/request_permission` while processing a turn (`agent-client-protocol-schema-1.5.0/src/v1/client.rs:2332-2342`).

   The cycle is: prompt owns mutex → provider waits for approval response → UI response waits for mutex → prompt cannot finish. Task 3 also routes only notifications, not agent-to-client JSON-RPC requests.

   The proposed sink does not directly deadlock on the manager’s sessions mutex—`prompt` releases that mutex before awaiting—but the runtime mutex remains fatal.

   Fix: the transport actor must permit writes while a prompt response is pending. Parse incoming permission/user-input requests, emit their typed events, and send responses/cancel through a writer channel without acquiring a turn-long runtime lock.

3. BLOCKER — Capturing the manager in the client sink creates a self-retaining cycle.

   Evidence: Task 4 tells the sink to capture a clone of the manager’s shared internals at plan `:410-415`. The manager owns sessions (`manager.rs:58-62`), each session owns its runtime (`:239-261`), the runtime owns `AcpClient`, and the proposed client would own a closure that owns the manager again. `Drop` only clears sessions when the sessions `Arc` has one owner (`:842-848`), which this cycle prevents.

   Fix: send updates over a channel owned outside the runtime, or capture `Weak` references. Add a test proving that closing/dropping a session terminates the provider process and releases the sink.

4. BLOCKER — Turn IDs and lifecycle timing are undefined, and “turn started” would clear the frontend’s busy state.

   Evidence: Task 4 requires `Turn::Started` before the prompt and `Completed` after it (`plan:329`, `:420`) but does not say where the pre-request `turn_id` comes from. ACP v1 `PromptResponse` contains `stop_reason`, not a required turn ID (`agent.rs:3259-3291`). The current client reads an optional extension only after the response (`acp_client.rs:132-149`), and the manager currently sets `active_turn_id` only after that await (`manager.rs:317-330`). Consequently streamed canonical events use no turn ID (`manager.rs:919-929`).

   Separately, the frontend clears `sending` for every legacy `kind: "turn"` event, including Started (`conversationService.ts:356-366`).

   Fix: allocate an app-owned turn ID before sending, store Working state first, correlate every update to it, and emit one terminal state after response/error. Change the frontend to clear `sending` only for completed/interrupted/failed/error events. Test the full state sequence.

5. BLOCKER — Task 5 introduces a second writer for externally started sessions.

   Evidence: The current guard explicitly prevents starting an ACP runtime when a PTY already owns the session (`+page.svelte:983-992`). Task 5 says to delete that block and unconditionally call `ensureStructuredConversation` (`plan:512-515`). But adopted external sessions spawn and retain a PTY at `+page.svelte:1094-1112`.

   Terminal projection also emits on the same channel (`terminal_projection.rs:13`, `:357-369`) with its own incremented generation and sequence (`:218-235`). Running projection and ACP for one `ownedId` can suppress one stream as stale or interleave two authorities.

   Manager generation validation itself correctly rejects an old captured generation (`manager.rs:852-877`); the defect is allowing two independently generated authorities.

   Fix: make session source/ownership explicit. App-owned agent sessions use ACP only. Externally active sessions use projection only and never call ACP activation. Add an exactly-once test proving one source per `ownedId`.

6. BLOCKER — The required typed event contract is not implemented.

   Evidence: The spec requires tool, file-change, plan, subagent, approval, error, and turn events at `conversation-centric-workbench-design.md:58-63`. The plan explicitly drops `plan` and every unknown update (`phase1-acp-foundation.md:372-380`) and never routes incoming approval requests. `AgentConversationPayload` has no plan, file-change, or subagent variants (`protocol.rs:467-516`).

   Rich canonical types already exist for plan, tasks, children, file changes, and subagents (`protocol.rs:169-231`), but Task 4 bypasses them.

   Fix: define the complete normalized mapping before calling the contract stable. Map ACP `Plan`, tool content/diffs/locations, usage, approvals, and supported subagent extensions into canonical `AgentEvent`/`AgentItem` forms. Preserve unknown extensions diagnostically rather than silently losing them.

7. BLOCKER — Start failure, retry, reconnect, and provider-death handling are missing.

   Evidence: The spec requires an error card with retry and ACP load/resume after a stream drop (`spec:180-185`). Task 5 only marks the rail session exited and writes a transient rail error (`plan:532-535`). Current `ensure_agent_conversation` can return a successful Connecting response when no provider manifest exists (`agent_conversation/mod.rs:20-33`). EOF from `next_json` becomes a transport error, with no reconnect state or resume attempt (`providers/process.rs:126-139`).

   Fix: add a tested connection state machine: Connecting → Connected → Reconnecting → Connected/Failed. Persist a session-level error and retry action; on stream loss, restart the adapter and load/resume the native session; emit a terminal failure when the provider cannot be restarted.

8. MAJOR — Task 6 preserves the dual-mode product the spec explicitly removes.

   Evidence: The spec says “No dual-mode toggle, no PTY per agent session” (`spec:38-55`). Task 6 deliberately retains `'structured' | 'raw'` and promises a reversible toggle (`plan:571-573`, `:612`). Existing Open/Fork native CLI controls remain at `ConversationSurface.svelte:227-236`, along with `/terminal` and `/conversation` mode commands at `ConversationSurface.svelte:176-185`.

   Task 6 also substitutes reduced canonical events for the spec’s optional raw process-output inspector.

   Fix: make the inspector a separate read-only diagnostic panel, not the agent’s interaction mode. Remove agent-session handoff/mode controls and persistence. Keep plain terminal sessions as a distinct session type and external-session projection as a separate read-only source.

9. MAJOR — The inspector implementation relies on state and props that do not exist.

   Evidence: The plan says the frontend store already holds a recent event list (`plan:571-573`), but its state contains reduced timeline/items only (`conversationStore.svelte.ts:38-66`), and event application discards the original event after reduction (`:127-170`). The proposed `<TerminalSurface readonly />` at plan `:607-609` is invalid: `TerminalSurface` has no `readonly` prop (`TerminalSurface.svelte:23-42`).

   The Task 6 file list also omits the store, event types, and terminal input layer that its implementation requires.

   Fix: add an explicitly bounded diagnostic-frame/event ring or a backend snapshot selector, create a dedicated inspector component, and suppress terminal writes at the terminal service boundary for external read-only terminals. Do not simulate read-only behavior with an unsupported component prop.

10. MAJOR — Several cold-start code recipes are factually wrong and will not compile as written.

   Evidence:

   - Emitter setup belongs in `src-tauri/src/main.rs:5317-5343`, where the manager and `AppHandle` exist—not `agent_conversation/mod.rs` as claimed at plan `:317-326` and `:422`.
   - `AcpRuntimeAdapter::client_mut` already exists as a private `Result<&mut AcpClient, AgentRuntimeError>` (`providers/acp.rs:29-33`), not the proposed infallible signature.
   - `ToolState::Running` does not exist; variants are Started, Updated, Completed, Failed (`protocol.rs:440-447`).
   - `manager_with_active_session` does not exist at the cited manager test lines; only `request` and `temp_root` helpers exist (`manager.rs:999-1020`).
   - The first manager event sequence is 1 (`manager.rs:171`, `:275-277`), not 0 as asserted at plan `:447`.
   - `fake_client_with_frames`, `client_request`, and `block_on` do not exist. The current ACP harness is an async shell fixture (`acp_client.rs:451-480`, `:502-586`).

   Fix: rewrite Tasks 3 and 4 with exact compile-ready APIs, files, async test annotations, fixture changes, and expected sequences.

11. MAJOR — The proposed tests do not prove the behavior, and several existing pins are omitted.

   Evidence: Task 4’s “end-to-end” test calls `pump_update` directly (`plan:430-448`), so it passes even if `activate` never installs the ACP sink. Task 5’s regex merely detects an `ensureStructuredConversation` call, and Task 6’s regex only detects markup and the string `TerminalSurface`; neither proves ownership, streaming, event rendering, or input suppression.

   Existing pins omitted from the targeted sweep include:

   - `scripts/conversationCommandCatalog.test.mjs:18`, which requires the terminal mode command.
   - `scripts/agentRuntimeContracts.test.mjs:112-140`, which persists raw mode and terminal ownership.
   - `scripts/sessionWorkspaces.test.mjs:596-625`, which preserves raw mode.
   - The underlying workspace type still requires `'structured' | 'raw'` at `sessionWorkspaces.ts:34-52`.

   The planned capability string is also not consumed anywhere in the frontend, despite the capability test saying every name is a frontend-checked promise (`main.rs:8254-8258`).

   Fix: add an async fake-sidecar test that drives activation → prompt → update → approval/cancel → terminal response → completion through the real manager and emitter. Add idle update, stale generation, reconnect, exactly-once, and external-projection isolation tests. Update all mode/handoff pins deliberately.

12. MAJOR — Tasks 3–6 cannot land independently behavior-green in the stated order.

   Evidence: Task 3 supplies no transport primitive capable of supporting Task 4. Task 4 would emit Turn Started while the unchanged frontend clears `sending` on that event (`conversationService.ts:356-366`), yet the file is absent from Task 4. Task 5 activates ACP for external PTY sessions before source ownership is separated. Task 6 depends on store and terminal APIs outside its file list.

   Fix: reorder the foundation:

   1. Bidirectional ACP transport actor and pending-request routing.
   2. Turn correlation, complete normalized events, emitter setup, and reconnect.
   3. Frontend lifecycle/error/store handling.
   4. App-owned ACP versus external-projection ownership split.
   5. Remove agent dual mode and add the independent inspector.
   6. Run integrated multi-session/native acceptance.

13. MAJOR — The browser root cause is confirmed, but the exact fix surface and test recipe are incomplete.

   Evidence: Every one of the 15 `TauriBrowserBackend` methods passes its payload unwrapped (`browserBackend.ts:220-279`). Every corresponding Rust command declares a named `input` struct (`browser.rs:1202-1339`). `clear_browser_workspace_data` has the same Rust signature (`:1294-1300`) but currently has no backend method.

   The plan’s sample test uses `test` and `readFileSync`, while the existing test imports neither (`scripts/browserBackend.test.mjs:1-3`). A navigation-only assertion would leave the other 14 methods broken.

   Fix: explicitly enumerate and wrap all 15 methods with `{ input }`; add the missing test imports and one assertion per method. If `clear_browser_workspace_data` gets a frontend adapter, wrap that too. Prefer an injected/mock invoke assertion where practical; the spec-requested source pin is acceptable if it covers the entire adapter.

14. MAJOR — Usage string masking is confirmed, but the SQLite cause is not diagnosed and the proposed tests can pass without fixing it.

   Evidence: Tauri string rejection is discarded in history at `usageStore.svelte.ts:80-82`; the same defect exists in current-usage refresh at `:53-55`. The backend shells out using PATH at `usage_db.rs:383-410` and `:418-423`, but the discovery receipt labels PATH only an assumption and lists several other causes (`phase1-contracts.md:990-996`). That conflicts with “diagnose, do not guess” (`spec:137-145`).

   The resolver test accepts either bare `sqlite3` or `/usr/bin/sqlite3`, so an implementation that always returns the current broken value passes. It never sanitizes PATH, tests a spawn fallback, opens the real usage DB, or proves history loads. Appending the resolved path to errors also needs the path returned alongside `Command`; three call sites currently lose it (`usage_db.rs:383-410`).

   On this reviewed Mac, `/usr/bin/sqlite3` exists and runs—macOS 26.5.2, SQLite 3.51.0—while the interactive PATH resolves an Android SDK copy first. That confirms PATH is unstable, not that PATH caused the packaged-app failure.

   Fix: first capture the native backend error. Then add a deterministic sanitized-PATH/injected-spawn test and an actual database query test; share one string-preserving error formatter across both frontend catches; retain binary-path diagnostics at all three backend call sites. Native acceptance must require history to load, not “loads or shows a specific error.”

Verdict: REWORK. Rewrite Tasks 3–6 around a bidirectional transport actor and explicit session-source ownership, then repair both blocker tests before handing the plan to implementation agents. No files were edited or committed.


