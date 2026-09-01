# Phase 1: ACP-Only Foundation Implementation Plan (v2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

v2 after two independent reviews (both REWORK — see
docs/superpowers/evidence/tsk-808/phase1-plan-reviews/). v1's Tasks 3-6 were redesigned:
the read-inside-request loops could never deliver approvals or cancellation (turn-long
mutex deadlock), the sink design leaked sessions via an Arc cycle, and external sessions
could acquire two writers.

**Goal:** Agent sessions (Codex/Claude) run exclusively through ACP with live typed events streamed to the structured UI; approvals and cancellation work mid-turn; app-owned ACP sessions and externally-started projected sessions are mutually exclusive per session; the browser and usage-history blockers are fixed.

**Architecture:** A bidirectional transport actor per ACP sidecar: one reader task owns stdout and routes response frames to a pending-request oneshot map, notifications and agent-to-client requests to an inbound channel; stdin sits behind its own lock so writes (responses, cancels) happen while a prompt is pending. The manager consumes the inbound channel per session, normalizes into the existing `emit_payload` pipeline, and a Tauri-side emitter broadcasts on the existing `agent-conversation-event` channel. Turn ids are minted app-side. The runtime mutex is never held across a turn.

**Tech Stack:** Rust (tauri 2, tokio, agent-client-protocol =2.0.0), Svelte 5 runes, node script tests.

**Spec:** docs/superpowers/specs/2026-08-09-conversation-centric-workbench-design.md (section 1 + bug fixes). Contracts: docs/superpowers/evidence/tsk-808/round2-discovery/phase1-contracts.md. Reviews: docs/superpowers/evidence/tsk-808/phase1-plan-reviews/{opus-review.md,sol-xhigh-review.md}.

**Phase roadmap (later phases planned after this lands):** 2 shell layout · 3 session-history index · 4 browser annotations · 5 composer parity · 6 resources/usage placement.

## Global Constraints

- Commits: plain messages, NO Co-Authored-By or any trailer, never mention Claude/Anthropic.
- Worktree: /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave. Paths relative to `tauri-svelte-preview/` unless they start with `docs/`.
- One build/test process at a time. Rust: `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml <filter>`.
- Gates after every task: `pnpm check`, `pnpm check:svelte` (0 errors in /next files), `pnpm build`, plus the task's named tests.
- TDD: failing test first, watch it fail, implement, watch it pass.
- Do NOT commit unless a step says so; lanes report, the controller commits.
- ACP wire facts (verified): turn completion arrives in the `session/prompt` RESPONSE (`stop_reason`; no turn id exists in ACP v1). `session/update` is the notification method. `session/request_permission` is a JSON-RPC REQUEST from the agent that MUST be answered before the turn can complete. The pinned crate is `agent-client-protocol = "=2.0.0"` (schema 1.5.0).
- Key invariants (violating any is a defect): TerminalSurface hosts are NEVER wrapped in `{#if}` (TerminalSurface.svelte:1-19); the frontend reducers require strictly contiguous sequences per `ownedId` (conversationStore.svelte.ts:172-179); manager event sequences start at 1 (manager.rs:171); exactly ONE event source (ACP pump XOR terminal projection) may emit for a given `ownedId`.

### Task 0 (baseline, run before Task 1, no code changes)

- [ ] Capture real baselines into `docs/superpowers/evidence/tsk-808/phase1/receipt.md`: full `cargo test` pass/fail counts, `pnpm check:svelte` error count and which files, and the exact list of failing `scripts/*.test.mjs`. Later tasks compare against THESE numbers, not the ones quoted in conversation.

---

### Task 1: Browser — wrap all fifteen Tauri command payloads in `{ input }`

Every `TauriBrowserBackend` method passes its payload unwrapped (`browserBackend.ts:220-279`) while every Rust browser command declares a named `input` struct (`browser.rs:1202-1339`). The native browser has never worked at all — not just navigation.

**Files:**
- Modify: `src/lib/shell/browser/browserBackend.ts:220-279`
- Test: `scripts/browserBackend.test.mjs`

**Interfaces:**
- Consumes: `invokeBrowserCommandFromTauri<T>(command, input)` (`tauriSource.ts:808-817`) — unchanged.
- Produces: all fifteen Tauri-backed methods call `invokeBrowserCommandFromTauri<T>('<command>', { input })`. (`clear_browser_workspace_data` at browser.rs:1294-1300 has no frontend method today — do not add one.)

- [ ] **Step 1: Enumerate the fifteen methods**

Run: `grep -n "invokeBrowserCommandFromTauri" src/lib/shell/browser/browserBackend.ts`
List every method name in the receipt file. Cross-check each against its `#[tauri::command]` in `src-tauri/src/browser.rs:1202-1339` — confirm all take `input: SomeStruct`.

- [ ] **Step 2: Write the failing test**

`scripts/browserBackend.test.mjs` currently imports neither `node:test` nor `node:fs` (lines 1-3) — check its existing runner style first and match it (if it uses plain functions + a manual runner, add a plain function). Add, with the needed imports added at the top of the file:

```js
import { readFileSync } from 'node:fs';

function testTauriBackendWrapsEveryCommandInInputKey() {
  const source = readFileSync(new URL('../src/lib/shell/browser/browserBackend.ts', import.meta.url), 'utf8');
  const commands = [...source.matchAll(/invokeBrowserCommandFromTauri<[^>]*>\('([a-z_]+)'/g)].map((m) => m[1]);
  assert.equal(commands.length, 15, `expected 15 tauri browser commands, saw ${commands.length}`);
  const unwrapped = [...source.matchAll(/invokeBrowserCommandFromTauri<[^>]*>\('([a-z_]+)',\s*(?!\{ input \})/g)].map((m) => m[1]);
  assert.deepEqual(unwrapped, [], `these commands pass their payload unwrapped: ${unwrapped.join(', ')}`);
}
```

Register it in the file's existing runner list.

- [ ] **Step 3: Run to verify failure**

Run: `node --experimental-strip-types scripts/browserBackend.test.mjs`
Expected: FAIL listing all fifteen commands as unwrapped.

- [ ] **Step 4: Fix all fifteen methods**

Mechanical edit, same shape for each, e.g.:

```ts
navigate_browser_tab(input: BrowserBackendNavigationInput): Promise<void> {
  return invokeBrowserCommandFromTauri<void>('navigate_browser_tab', { input });
}
```

- [ ] **Step 5: Run to verify pass, then gates**

Run: `node --experimental-strip-types scripts/browserBackend.test.mjs` → PASS.
Run: `pnpm check && pnpm check:svelte && pnpm build` → green.

---

### Task 2: Usage history — surface the real error; diagnose before fixing

Spec rule: diagnose, do not guess. `/usr/bin/sqlite3` exists on this Mac and `/usr/bin` is in a Finder-launched app's PATH, so v1's PATH-fallback theory is unproven. This task makes the real error visible and diagnosable; the actual fix is a follow-up driven by the captured error.

**Files:**
- Modify: `src/lib/shell/usage/usageStore.svelte.ts:53-55` and `:80-82` (BOTH catches mask string rejections)
- Modify: `src-tauri/src/usage_db.rs` (error strings at the three shell-out sites, ~:383-410 and :418-423)
- Test: `scripts/usageViewModel.test.mjs`, `src-tauri/src/usage_db.rs` tests

**Interfaces:**
- Consumes: `sqlite_command()` (usage_db.rs:418-423), `usageState.error: string | null`.
- Produces: `function describeUsageError(error: unknown): string` exported from the usage store module and used by BOTH catches; every sqlite failure string includes the resolved binary path and the db path.

- [ ] **Step 1: Failing frontend test**

Add to `scripts/usageViewModel.test.mjs` (match its existing runner/import style):

```js
function testUsageErrorsPreserveStringRejections() {
  // describeUsageError must pass Tauri string rejections through untouched.
  assert.equal(describeUsageError('SQLite is unavailable (sqlite3): spawn failed'), 'SQLite is unavailable (sqlite3): spawn failed');
  assert.equal(describeUsageError(new Error('boom')), 'boom');
  assert.equal(describeUsageError({ weird: true }), 'Usage history could not be read.');
}
```

Import `describeUsageError` from the store module. Run → FAIL (function missing).

- [ ] **Step 2: Implement**

In `usageStore.svelte.ts`:

```ts
export function describeUsageError(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  return 'Usage history could not be read.';
}
```

Use it in BOTH catch blocks (:53-55 and :80-82): `usageState.error = describeUsageError(error);`

- [ ] **Step 3: Failing Rust test for diagnosable errors**

```rust
#[test]
fn sqlite_failures_name_the_binary_and_database_path() {
    let db = UsageDb::open(temp_dir().join("phase1-usage-test.sqlite3")).expect("open");
    std::env::set_var("MCB_SQLITE_BIN", "/nonexistent/not-sqlite3");
    let error = db.query("SELECT 1;").unwrap_err();
    std::env::remove_var("MCB_SQLITE_BIN");
    assert!(error.contains("/nonexistent/not-sqlite3"), "error must name the binary: {error}");
    assert!(error.contains("phase1-usage-test.sqlite3"), "error must name the db path: {error}");
}
```

(Adapt the method name to the file's real query entry point at usage_db.rs:383-410; use the module's existing temp-dir helper if one exists. Env-var tests must run under `RUST_TEST_THREADS=1`, which is the global rule anyway.)

- [ ] **Step 4: Implement diagnosable errors**

In `usage_db.rs`, capture the resolved binary once per call and thread it into the three error sites:

```rust
fn sqlite_binary() -> std::ffi::OsString {
    std::env::var_os("MCB_SQLITE_BIN").unwrap_or_else(|| "sqlite3".into())
}
// at each shell-out site:
let binary = sqlite_binary();
let output = Command::new(&binary) /* existing args */ .output().map_err(|error| {
    format!(
        "SQLite is unavailable ({}) for {}: {error}",
        binary.to_string_lossy(),
        self.path.display()
    )
})?;
```

Apply the same `({binary}) for {path}` naming to the "SQLite command failed" / "SQLite query failed" strings. Do NOT add spawn probes or fallback binaries — no evidence yet.

- [ ] **Step 5: Run tests + gates**

`RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml usage` → PASS.
`node --experimental-strip-types scripts/usageViewModel.test.mjs` → PASS.
`pnpm check && pnpm check:svelte && pnpm build` → green.

- [ ] **Step 6: Record the diagnosis step**

Append to the receipt: "Native repro required: run the packaged/dev native app, open Stats & Usage, record the now-specific error text. The follow-up fix is driven by that text." Native acceptance for this task is "history loads, or the error names binary+path" — the FIX task (follow-up) requires history to load.

---

### Task 3: Bidirectional ACP transport actor

Replace read-inside-request with a reader task. This is the load-bearing task: without it, approvals and cancellation deadlock for the length of a turn (the `session/prompt` response IS the turn-completion signal, so `request` blocks for the whole turn while `manager.prompt` holds the runtime `AsyncMutex` that `cancel_turn`/`respond_permission`/`steer`/`set_config` also need — manager.rs:317-323, 489-494, 355-360, 377-382, 410-415).

**Files:**
- Modify: `src-tauri/src/agent_conversation/providers/process.rs` (split reader/writer ownership; `next_json` currently requires `&mut self`, :126-139)
- Modify: `src-tauri/src/agent_conversation/providers/acp_client.rs` (all request paths; delete frame loops)
- Test: `src-tauri/src/agent_conversation/providers/acp_client.rs` tests (extend the existing async shell-script fixture harness at :451-480 and :502-586 — there is no frame-queue harness; do not invent one)

**Interfaces:**
- Consumes: the sidecar's stdin/stdout pipes currently wrapped by `SidecarProcess`.
- Produces (exact API later tasks depend on):

```rust
pub enum AcpInbound {
    /// params of a session/update notification
    SessionUpdate(serde_json::Value),
    /// an agent->client JSON-RPC REQUEST (session/request_permission, etc.)
    AgentRequest { wire_id: serde_json::Value, method: String, params: serde_json::Value },
    /// reader ended: sidecar exited or stdout closed
    TransportClosed { reason: String },
}

pub struct AcpTransport { /* private */ }

impl AcpTransport {
    /// Splits the process pipes, spawns the reader task, returns the transport
    /// plus the single consumer end of the inbound channel.
    pub fn start(process: SidecarProcess) -> (std::sync::Arc<AcpTransport>, tokio::sync::mpsc::UnboundedReceiver<AcpInbound>);
    /// JSON-RPC request: allocate id, register oneshot, write, await. Holds NO lock while awaiting.
    pub async fn request(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, AgentRuntimeError>;
    /// Fire-and-forget notification (e.g. session/cancel).
    pub async fn notify(&self, method: &str, params: serde_json::Value) -> Result<(), AgentRuntimeError>;
    /// Answer an agent->client request by wire id.
    pub async fn respond(&self, wire_id: serde_json::Value, result: serde_json::Value) -> Result<(), AgentRuntimeError>;
}
```

Reader-task routing rules (in `start`): frame has `id` and NO `method` → response → resolve the pending oneshot (send an Err into it when the frame carries `error`); frame has `method` AND `id` → `AgentRequest`; frame has `method` only → `SessionUpdate` if method is `session/update`, else ignore with a log line. On read error/EOF: fail ALL pending oneshots with a transport error, send `TransportClosed`, exit.

- [ ] **Step 1: Failing tests (extend the existing async fixture harness)**

Three `#[tokio::test]`s in acp_client.rs's tests module, using the existing shell-script fixture pattern (the fixture at :451-468 already emits a `session/update` before the prompt response — reuse and extend it):

```rust
#[tokio::test]
async fn transport_routes_updates_and_responses_concurrently() {
    // fixture script: emits session/update, then answers the prompt request.
    let (transport, mut inbound) = AcpTransport::start(fixture_process("prompt_with_update"));
    let response = transport.request("session/prompt", json!({"sessionId": "s"})).await.expect("prompt response");
    assert_eq!(response.get("stopReason").and_then(|v| v.as_str()), Some("end_turn"));
    match inbound.recv().await.expect("inbound") {
        AcpInbound::SessionUpdate(params) => {
            assert_eq!(params["update"]["sessionUpdate"], "agent_message_chunk");
        }
        other => panic!("expected SessionUpdate, got {other:?}"),
    }
}

#[tokio::test]
async fn agent_requests_surface_on_the_inbound_channel_and_can_be_answered() {
    // fixture script: sends a session/request_permission REQUEST (with id),
    // waits for the response on stdin, THEN answers the pending prompt.
    let (transport, mut inbound) = AcpTransport::start(fixture_process("permission_midturn"));
    let prompt = tokio::spawn({ let t = transport.clone(); async move {
        t.request("session/prompt", json!({"sessionId": "s"})).await
    }});
    let (wire_id, method) = match inbound.recv().await.expect("inbound") {
        AcpInbound::AgentRequest { wire_id, method, .. } => (wire_id, method),
        other => panic!("expected AgentRequest, got {other:?}"),
    };
    assert_eq!(method, "session/request_permission");
    transport.respond(wire_id, json!({"outcome": {"outcome": "selected", "optionId": "allow"}})).await.expect("respond");
    prompt.await.expect("join").expect("prompt completes after permission answered");
}

#[tokio::test]
async fn transport_close_fails_pending_requests_and_notifies() {
    // fixture script: exits without answering.
    let (transport, mut inbound) = AcpTransport::start(fixture_process("dies_midturn"));
    let error = transport.request("session/prompt", json!({"sessionId": "s"})).await.expect_err("must fail");
    assert!(error.to_string().contains("transport"), "{error}");
    assert!(matches!(inbound.recv().await, Some(AcpInbound::TransportClosed { .. })));
}
```

`fixture_process(name)` builds on the existing fixture_manifest shell-script mechanism — add the three named scripts next to the current one. Run: `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml transport_` → FAIL to compile.

- [ ] **Step 2: Implement `AcpTransport`**

In process.rs: add a method that splits `SidecarProcess` into a read half and a write half (keep the existing `SidecarProcess` API for non-ACP callers if any exist — grep first). In acp_client.rs implement the struct above: `pending: std::sync::Mutex<HashMap<u64, tokio::sync::oneshot::Sender<Result<Value, AgentRuntimeError>>>>`, `next_id: AtomicU64`, `writer: tokio::sync::Mutex<WriteHalf>`. The pending mutex is only ever held for map insert/remove — never across an await.

- [ ] **Step 3: Migrate `AcpClient` onto the transport**

Every method that currently calls `self.process.write_json` + frame loop (`request` :349-371, `prompt_once`'s loop :184-241, `prompt` :132-150) delegates to `transport.request(...)`. `prompt_once` aggregates text by consuming... nothing locally anymore — it becomes: send `session/prompt`, and aggregation of `agent_message_chunk` text moves to the caller side via the inbound channel; keep a thin compatibility implementation inside AcpClient that snoops `SessionUpdate` frames from a broadcast copy ONLY if existing callers require the aggregated `GeneratedText` (grep `prompt_once` callers first; if the only caller is the commit-message/PR flow, aggregate there in Task 4's manager loop instead). Delete every `next_json` call from acp_client.rs outside the reader.

- [ ] **Step 4: Run**

`RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml acp` — the three new tests PASS; the two existing pins (`acp_initialize_new_prompt_image_config_correlations_cancel_and_close` :502-586, `acp_load_and_resume_use_distinct_session_methods` :588-609) PASS, adapted only where they drove the old loop directly.

---

### Task 4: Manager integration — turn lifecycle, complete mapping, emitter, ownership guard

**Files:**
- Modify: `src-tauri/src/agent_conversation/manager.rs`
- Modify: `src-tauri/src/agent_conversation/protocol.rs` (add `Plan` payload variant)
- Modify: `src-tauri/src/agent_conversation/providers/acp.rs` (adapter exposes the transport + inbound receiver)
- Modify: `src-tauri/src/main.rs:5317-5343` (emitter install inside `.setup(|app| …)` — NOT mod.rs; the manager is built at main.rs:5318-5321 before the Builder, so the setup closure is the only place both the manager and an `AppHandle` exist)
- Modify: `src/lib/shell/conversation/conversationService.ts:356-368` (sending-state narrowing), `src/lib/shell/conversation/conversationTypes.ts` + `conversationReducer.ts` (Plan payload)
- Test: manager.rs tests, `scripts/agentConversationProtocol.test.mjs`, `scripts/agentConversationStore.test.mjs`

**Interfaces:**
- Consumes: `AcpTransport` + `UnboundedReceiver<AcpInbound>` (Task 3); `emit_payload` (manager.rs:265-308, unchanged); `session.active_turn_id`, `session.writer_lease`.
- Produces:
  - `pub type ConversationEmitter = std::sync::Arc<dyn Fn(AgentConversationEvent) + Send + Sync>;` and `AgentRuntimeManager::set_emitter(&self, emitter: ConversationEmitter)`.
  - `pub fn payload_from_session_update(params: &serde_json::Value) -> Option<AgentConversationPayload>` (pure).
  - New payload variant in protocol.rs AND conversationTypes.ts: `Plan { items: Vec<PlanItem> }` / `{ kind: 'plan'; items: { text: string; status: string }[] }` with a reducer case.
  - Per-session pump task: spawned in `activate`, consumes the inbound receiver, holds a `Weak` reference to the manager internals (NEVER a strong Arc — a strong ref creates the cycle manager→session→runtime→closure→manager and sessions never drop).
  - App-minted turn ids: `prompt` generates `turn-{uuid}` BEFORE sending, stores it in `session.active_turn_id`, emits `Turn { turn_id, state: Started }`; a spawned completion task awaits the transport response WITHOUT holding the runtime mutex and emits `Turn { turn_id, state: Completed }` or the `Error` payload.
  - Ownership guard: the pump drops (does not emit) events whenever the session's writer lease/owner is not Structured — exactly one live source per `ownedId` (terminal projection is the other, terminal_projection.rs:13; both counters start at 1 and the reducers require contiguity, so interleaving would force resync loops).

Mapping table (complete — unknown kinds are IGNORED with a debug log, never a panic; nothing spec-required is dropped):

| inbound | payload |
| --- | --- |
| `agent_message_chunk` | `AssistantDelta { item_id: messageId or "assistant-{turn_id}", delta: content.text }` |
| `user_message_chunk` | `UserMessage { item_id: messageId or "user-{turn_id}", text, completed: false }` |
| `agent_thought_chunk` | ignored (log) — thoughts are not rendered in phase 1 |
| `tool_call` | `Tool { item_id: toolCallId, name: title, state: ToolState::Started, summary: from content/locations when present }` |
| `tool_call_update` | `Tool { …, state: map status: in_progress→Updated, completed→Completed, failed→Failed }` (variants are Started/Updated/Completed/Failed — protocol.rs:441-447; there is no Running) |
| `plan` | `Plan { items }` (new variant) |
| `AgentRequest session/request_permission` | `Approval { request_id: minted "perm-{n}", state: Requested, summary: from params.toolCall/title }` + store `request_id → wire_id` in the session so `respond_permission` (manager.rs:355-360) answers via `transport.respond` |
| `TransportClosed` | `Connection { state: Failed }` + `Error { code: "acp-transport", message: reason, recoverable: true }`, then mark the session state exited |

- [ ] **Step 1: Failing mapping tests**

```rust
#[test]
fn session_updates_map_to_conversation_payloads() {
    let chunk = json!({ "sessionId": "s", "update": {
        "sessionUpdate": "agent_message_chunk",
        "content": { "type": "text", "text": "Hi" }, "messageId": "m1" } });
    assert_eq!(
        payload_from_session_update(&chunk),
        Some(AgentConversationPayload::AssistantDelta { item_id: "m1".into(), delta: "Hi".into() })
    );
    let tool = json!({ "sessionId": "s", "update": {
        "sessionUpdate": "tool_call", "toolCallId": "t1", "title": "Read file", "status": "in_progress" } });
    match payload_from_session_update(&tool) {
        Some(AgentConversationPayload::Tool { item_id, name, state, .. }) => {
            assert_eq!(item_id, "t1"); assert_eq!(name, "Read file");
            assert_eq!(state, ToolState::Started);
        }
        other => panic!("expected Tool, got {other:?}"),
    }
    let plan = json!({ "sessionId": "s", "update": {
        "sessionUpdate": "plan", "entries": [ { "content": "step one", "status": "pending" } ] } });
    match payload_from_session_update(&plan) {
        Some(AgentConversationPayload::Plan { items }) => {
            assert_eq!(items.len(), 1); assert_eq!(items[0].text, "step one");
        }
        other => panic!("expected Plan, got {other:?}"),
    }
}
```

Run `…cargo test… session_updates_map` → FAIL to compile. Implement mapping + the `Plan` variant (Rust serde `camelCase` tagged like its siblings, protocol.rs:467-516; mirror in conversationTypes.ts:388-410 and add the reducer case in conversationReducer.ts:47-72; extend `scripts/agentConversationProtocol.test.mjs` with a plan-payload application test). Re-run → PASS. Also run `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml protocol` — the round-trip pin `canonical_event_round_trips_without_acp_frames` (protocol.rs:639-659) must be extended for `Plan`, not broken.

- [ ] **Step 2: Failing pump/lifecycle test through the real manager**

Use the real `ensure`/`activate` path with the Task 3 fixture process (this catches "activate never installed the pump", which a direct pump_update call would not):

```rust
#[tokio::test]
async fn a_prompt_streams_updates_and_lifecycle_through_the_emitter() {
    let (manager, owned_id) = fixture_manager_with_acp_session("prompt_with_update").await; // helper added in this task: ensure + activate against the fixture sidecar
    let seen: std::sync::Arc<std::sync::Mutex<Vec<AgentConversationEvent>>> = Default::default();
    let sink = seen.clone();
    manager.set_emitter(std::sync::Arc::new(move |e| sink.lock().unwrap().push(e)));
    manager.prompt(&owned_id, test_prompt("hello")).await.expect("prompt");
    wait_until(|| seen.lock().unwrap().iter().any(|e| matches!(e.payload, AgentConversationPayload::Turn { state: TurnState::Completed, .. }))).await;
    let seen = seen.lock().unwrap();
    let kinds: Vec<&'static str> = seen.iter().map(|e| payload_kind(&e.payload)).collect();
    assert!(kinds.starts_with(&["turn"]), "Turn Started must be first, got {kinds:?}");
    assert!(kinds.contains(&"assistantDelta"), "streamed delta missing: {kinds:?}");
    assert_eq!(seen[0].sequence, 1, "manager sequences start at 1");
    let turn_ids: Vec<_> = seen.iter().filter_map(|e| match &e.payload {
        AgentConversationPayload::Turn { turn_id, .. } => Some(turn_id.clone()), _ => None }).collect();
    assert_eq!(turn_ids.len(), 2); assert_eq!(turn_ids[0], turn_ids[1], "app-minted id correlates Started and Completed");
}

#[tokio::test]
async fn the_pump_is_silent_while_the_writer_lease_is_terminal() { /* set lease to terminal via the existing lease API, feed an update through the fixture, assert emitter saw nothing */ }

#[tokio::test]
async fn dropping_a_session_releases_the_pump_and_kills_the_sidecar() { /* close_session; assert the fixture process exited and Weak::upgrade inside the pump now fails (observable: no emission after close) */ }
```

Run → FAIL to compile; implement:
- `activate` (manager.rs:190-263): after building the adapter, take its inbound receiver + transport Arc; spawn the pump task with `Weak` manager internals, `owned_id`, `generation`; the pump maps inbound → `emit_payload` (skip while owner ≠ Structured; drop silently on stale generation — `current_session_mut` already rejects those) → emitter.
- `prompt`: mint turn id, `emit_payload(Turn Started)`, clone the transport Arc under a SHORT runtime-mutex lock, release, send `session/prompt` via the transport, spawn the completion task. The runtime `AsyncMutex` is never held across the turn — `cancel_turn`/`respond_permission`/`steer` stay responsive (they also switch to short-lock + transport calls: cancel = `transport.notify("session/cancel", …)`, permission = look up stored wire id + `transport.respond`).
- `main.rs` setup closure (`main.rs:5334`): `let handle = app.handle().clone(); manager.set_emitter(Arc::new(move |event| { let _ = handle.emit("agent-conversation-event", event); }));`

Re-run → PASS. Then the full suite: `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml` → no regressions vs the Task 0 baseline.

- [ ] **Step 3: Frontend sending-state narrowing (failing test first)**

Add to `scripts/agentConversationStore.test.mjs` (match its existing runner style): apply a `turn` payload with `state: 'started'` → sending stays true; apply `state: 'completed'` → sending false. Run → FAIL. Fix `conversationService.ts:362-366`: clear sending only when the turn payload's `state !== 'started'` (keep the canonical-event `['turn.completed','turn.interrupted','runtime.error']` branch as is). Run → PASS.

- [ ] **Step 4: Capability, consumed**

Append `"acpLiveConversationEvents"` to `backend_capabilities()` and its pin (main.rs:8254-8288). The capability test's contract says every name is a frontend-checked promise — so ALSO consume it: in `conversationService.ts`, only skip the legacy post-turn `resyncConversation` polling when the capability is present (grep `backendCapabilities` for the existing lookup pattern and follow it). Run the capabilities test → PASS.

---

### Task 5: Session ownership split — app-owned ACP vs external projection

New agent sessions are ACP-only. Externally-started/adopted sessions (which spawn a PTY at `+page.svelte:1094-1112`) remain projection-only and NEVER activate ACP — one writer per session, structurally.

**Files:**
- Modify: `src/routes/next/+page.svelte` (selection path :983-1002, `startNewSession` :1130-1152)
- Modify: `src/lib/shell/stores/sessionRailStore.svelte.ts` (or wherever the owned-session record lives — the Step 1 grep confirms) — add `origin: 'app' | 'external'`
- Test: NEW `scripts/sessionOwnership.test.mjs`; the existing handoff/runtime pins must pass (see Step 4)

**Interfaces:**
- Consumes: `ensureStructuredConversation({ ownedId, provider, cwd, nativeSessionId })` (conversationService.ts:381-395); `addOwnedSession`/`updateOwnedSession`/`selectOwned`/`hostFor`/`frameControls` (all already in +page.svelte scope).
- Produces: `OwnedSession.origin` persisted with the record; selection logic keyed on origin: `origin === 'app'` → `ensureStructuredConversation` (never raw-forced); `origin === 'external'` → projection only (current PTY+projection behavior, unchanged); plain-shell sessions (`agent: 'other'`) keep the PTY path.

- [ ] **Step 1: Failing source-contract tests (new file)**

`scripts/sessionOwnership.test.mjs`, matching the manual-runner style of its siblings (no node:test):

```js
function testAppOwnedAgentSessionsNeverForceRawOnSelection() {
  const source = readFileSync(new URL('../src/routes/next/+page.svelte', import.meta.url), 'utf8');
  assert.doesNotMatch(
    source,
    /closeStructuredConversation\(ownedId\);\s*setConversationMode\(ownedId, 'raw'\);\s*return;/,
    'the unconditional force-raw selection branch must be gone'
  );
  assert.match(source, /origin === 'external'/, 'selection must branch on session origin');
}

function testStartNewSessionCreatesAgentSessionsThroughAcp() {
  const source = readFileSync(new URL('../src/routes/next/+page.svelte', import.meta.url), 'utf8');
  assert.match(source, /async function startNewSession[\s\S]*?ensureStructuredConversation\(\{/, 'agent sessions go through ACP');
  assert.match(source, /async function startNewSession[\s\S]*?startOwned/, 'the plain-shell branch must survive');
  assert.match(source, /origin: 'app'/, 'app-created sessions are marked app-owned');
}
```

Run → FAIL.

- [ ] **Step 2: Implement**

Add `origin` to the owned-session type with a migration default: records restored from localStorage without the field get `origin: 'external'` (conservative: they may have a live PTY). Adoption of externally-discovered sessions (:1094-1112) sets `origin: 'external'`.

`startNewSession` — FULL replacement (keeps every behavior of :1130-1152 including the final `showCenterPanel`):

```ts
async function startNewSession(request: NewSessionRequest): Promise<void> {
  if (!service || disposed) return;
  const owned = {
    ...createFreshSession({ cwd: request.cwd, title: request.title }),
    agent: request.agent,
    resumeCommand: request.command,
    origin: request.agent === 'codex' || request.agent === 'claude' ? ('app' as const) : ('external' as const)
  };
  addOwnedSession(owned);
  if (owned.origin === 'app') {
    updateOwnedSession(owned.ownedId, { state: 'live' });
    try {
      await ensureStructuredConversation({ ownedId: owned.ownedId, provider: owned.agent, cwd: owned.cwd });
    } catch (error) {
      updateOwnedSession(owned.ownedId, { state: 'exited', lastError: describeError(error) });
      rail.error = `could not start ${owned.agent} session: ${describeError(error)}`;
      return;
    }
  } else {
    const host = await hostFor(owned.ownedId);
    if (!host) { rail.error = `no terminal host for "${owned.title}"`; return; }
    const ptySessionId = await service.startOwned(owned, host);
    if (!ptySessionId) {
      updateOwnedSession(owned.ownedId, { state: 'exited' });
      rail.error = `failed to start a terminal for "${owned.title}"`;
      return;
    }
    updateOwnedSession(owned.ownedId, { ptySessionId, state: 'live' });
  }
  await selectOwned(owned.ownedId);
  frameControls?.showCenterPanel('session');
}
```

(`lastError` on the record is the retry hook: the session card in the rail shows it with a Retry button that re-calls `ensureStructuredConversation` — add the button where the rail renders session state; grep `state === 'exited'` in the rail components for the render site.)

Selection path (:983-1002): replace the `if (selected.ptySessionId)` guard with an origin branch — `external` → keep today's projection behavior; `app` → `ensureStructuredConversation` with the existing catch.

- [ ] **Step 3: Run new tests** → PASS. **Gates** → green.

- [ ] **Step 4: Pin sweep — these must pass UNMODIFIED unless a listed assertion pins the deleted branch**

```bash
for f in agentConversationHandoff agentRuntimeContracts nextTerminalService \
         conversationSessionIsolation agentConversationStore \
         agentConversationTerminalProjection conversationCommandCatalog sessionWorkspaces; do
  node --experimental-strip-types scripts/$f.test.mjs || echo "REVIEW: $f"
done
```

Known contacts: `agentRuntimeContracts.test.mjs:38-84` (ptySessionId reconciliation — external sessions still satisfy it), `:112-140` and `sessionWorkspaces.test.mjs:596-625` (raw-mode persistence — still valid for external sessions in this task; Task 6 revisits), `nextTerminalService.test.mjs:143-232` (startOwned — survives via the plain-shell/external branch). `agentConversationHandoff.test.mjs` pins the EXPLICIT handoff flow, which phase 1 keeps for external sessions — do not modify it. Any other failure = stop and report, don't force.

---

### Task 6: Agent sessions lose the dual-mode toggle; inspector becomes a separate panel

For `origin: 'app'` sessions there is no raw MODE: the structured view is the only interaction surface, and a read-only "Inspector" panel (event ring) replaces "Open raw terminal". External and plain-shell sessions keep their terminal (that IS their surface). TerminalSurface stays mounted unconditionally — its hosts are never `{#if}`-gated (TerminalSurface.svelte:1-19) and it has no `readonly` prop, so we don't touch it at all.

**Files:**
- Modify: `src/lib/shell/components/ConversationSurface.svelte` (:60-65 structured condition, :227-236 native-CLI handoff controls, :176-185 `/terminal` `/conversation` commands)
- Modify: `src/lib/shell/components/conversation/ConversationHeader.svelte:15-18`
- Modify: `src/lib/shell/conversation/conversationStore.svelte.ts` (bounded event ring)
- Create: `src/lib/shell/components/conversation/ConversationInspector.svelte`
- Test: NEW `scripts/conversationInspector.test.mjs`, plus deliberate updates to `scripts/conversationCommandCatalog.test.mjs:18`, `scripts/agentRuntimeContracts.test.mjs:112-140`, `scripts/sessionWorkspaces.test.mjs:596-625` (raw-mode persistence narrows to non-app sessions; the `'structured' | 'raw'` union in `sessionWorkspaces.ts:34-52` STAYS — external sessions still use it)

**Interfaces:**
- Consumes: `conversation?.mode`; session `origin` (Task 5); `AgentConversationEvent` stream already applied by the store.
- Produces:
  - Store ring: `export function conversationRecentEvents(ownedId: string): ReadonlyArray<{ sequence: number; kind: string; summary: string; timestampMs: number }>` — populated in `applyAgentConversationEvent` (conversationStore.svelte.ts:127-170) BEFORE reduction discards the raw event; capped at 200 entries per session (matching SNAPSHOT_EVENT_CAP's spirit).
  - `ConversationInspector.svelte`: renders the ring read-only (`<div data-testid="conversation-inspector" role="log">` rows `[seq] kind: summary`); no input elements.
  - For `origin === 'app'` sessions: the header button becomes "Inspector" toggling ONLY the inspector overlay (never `setConversationMode`); the `/terminal` command and Open/Fork native-CLI controls are hidden; `mode` is pinned `'structured'`.
  - For other sessions: everything unchanged.

- [ ] **Step 1: Failing tests**

`scripts/conversationInspector.test.mjs` (manual-runner style):

```js
function testInspectorRingIsCappedAndReadOnly() {
  const ring = makeRingFixture(); // exercise conversationRecentEvents via the store's apply path with 250 synthetic events
  assert.equal(ring.length, 200, 'ring caps at 200');
  assert.equal(ring[0].sequence, 51, 'oldest entries evicted first');
  const source = readFileSync(new URL('../src/lib/shell/components/conversation/ConversationInspector.svelte', import.meta.url), 'utf8');
  assert.doesNotMatch(source, /<input|<textarea|contenteditable/, 'inspector is read-only');
  assert.match(source, /data-testid="conversation-inspector"/);
}

function testAppSessionsHaveNoModeToggle() {
  const source = readFileSync(new URL('../src/lib/shell/components/ConversationSurface.svelte', import.meta.url), 'utf8');
  assert.match(source, /origin === 'app'/, 'surface must branch on origin');
  assert.doesNotMatch(source, /setConversationMode\([^)]*'raw'\)\s*(?![\s\S]{0,120}origin)/, "app sessions must never be switched to raw mode");
}
```

(Write `makeRingFixture` against the real store module — import it, apply 250 events through `applyAgentConversationEvent`, read the ring back. This is the behavioral test the reviews demanded.) Run → FAIL.

- [ ] **Step 2: Implement** (store ring first, then the component, then the surface/header branching). The inspector overlays the structured view like `.structured` itself overlays (ConversationSurface.svelte:224) — no TerminalSurface changes.

- [ ] **Step 3: Update the three pinned scripts deliberately** — each keeps its raw-mode assertions for external sessions and adds/loosens only what origin-branching requires. Run all three + the new file → PASS.

- [ ] **Step 4: Gates** → green.

---

### Task 7: Integrated verification + receipt

- [ ] **Step 1: Full sweep**

```bash
RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml
pnpm check && pnpm check:svelte && pnpm build
for f in scripts/*.test.mjs; do node --experimental-strip-types "$f" || echo "FAIL: $f"; done
```

Expected: cargo failures = 0; script failures identical to the Task 0 baseline list; gates green.

- [ ] **Step 2: Receipt**

Finish `docs/superpowers/evidence/tsk-808/phase1/receipt.md`: per task — what changed (file:line), test names proving it, deviations from this plan with reasons, and the usage-history native error text once captured.

## Native acceptance (user, after controller merges)

1. `pnpm tauri:dev:next` from the worktree.
2. New Codex session → structured immediately, streams deltas live, no mode toggle; Inspector shows the event ring.
3. Mid-turn: an approval prompt appears and answering it lets the turn finish; Stop cancels a running turn.
4. An externally-started CLI session still shows its terminal + projected history (no ACP takeover).
5. Browser: create a tab, navigate, screenshot — all fifteen commands now reach the backend.
6. Stats & Usage → Refresh: history loads, or the error names the sqlite binary + db path (that text drives the follow-up fix).
