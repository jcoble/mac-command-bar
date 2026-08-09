# Phase 1: ACP-Only Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Agent sessions (Codex/Claude) run exclusively through ACP with live `session/update` events streamed to the structured UI; the force-raw PTY handoff is deleted; the two daily-use blockers (browser navigation, usage history read) are fixed.

**Architecture:** A dedicated ACP frame reader task replaces the read-inside-request loops, routing responses to pending requests and `session/update` notifications into the manager, which normalizes them via the existing `emit_payload` and emits them on the existing `agent-conversation-event` channel. The frontend creates sessions through `ensureStructuredConversation` instead of spawning PTYs. Raw mode becomes a read-only event inspector.

**Tech Stack:** Rust (tauri 2, tokio, agent-client-protocol =2.0.0), Svelte 5 runes, node script tests (`node --experimental-strip-types`).

**Spec:** docs/superpowers/specs/2026-08-09-conversation-centric-workbench-design.md (section 1 + the two bug fixes from sections 4/6). Contract receipts: docs/superpowers/evidence/tsk-808/round2-discovery/phase1-contracts.md.

**Phase roadmap (later phases get their own plans after this one lands):** 2 shell layout · 3 session-history index · 4 browser annotations · 5 composer parity · 6 resources/usage placement.

## Global Constraints

- Commits: plain messages, NO Co-Authored-By or any trailer, never mention Claude/Anthropic.
- Worktree: /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave. All paths below are relative to `tauri-svelte-preview/` unless they start with `docs/`.
- One build/test process at a time. Rust tests: `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml <filter>`.
- Gates that must stay green after every task: `pnpm check`, `pnpm check:svelte` (0 errors in /next files; 16 old-shell errors are pre-existing), `pnpm build`, and the script tests named in the task.
- TDD: write the failing test first, run it, see it fail, then implement.
- No client-side aggregation of DB rows; no API-key usage anywhere.
- New backend commands/behavior the frontend cannot detect must be appended to the pinned capabilities test at `src-tauri/src/main.rs:8254-8289` (`backend_capabilities_name_every_addition_the_frontend_cannot_otherwise_detect`).
- Do NOT commit unless the task's steps say so (lanes report; the controller commits).

---

### Task 1: Browser navigation — wrap Tauri command args correctly

The native browser cannot navigate: `invalid args 'input' for command 'navigate_browser_tab'`. The frontend sends the navigation fields as the top-level invoke args, but the Rust command declares a named `input` parameter, so Tauri looks for an `input` key.

**Files:**
- Modify: `src/lib/shell/browser/browserBackend.ts:241-243` (and sibling Tauri-backed methods, see audit step)
- Test: `scripts/browserBackend.test.mjs`

**Interfaces:**
- Consumes: `invokeBrowserCommandFromTauri<T>(command, input)` at `src/lib/tauriSource.ts:808-817` — passes `input` straight to `invoke(command, input)`; unchanged.
- Produces: every Tauri-backed browser method passes args shaped exactly as the Rust command signature declares them (named `input` struct → `{ input: {...} }`).

Rust contract being matched (`src-tauri/src/browser.rs:1254-1260`, do not modify):

```rust
#[tauri::command]
pub fn navigate_browser_tab(
    registry: tauri::State<'_, BrowserRegistry>,
    input: BrowserNavigationInput,
) -> Result<(), BrowserCommandError>
```

- [ ] **Step 1: Audit every Tauri-backed browser method against its Rust command signature**

Run:
```bash
grep -n "#\[tauri::command\]" -A 6 src-tauri/src/browser.rs
grep -n "invokeBrowserCommandFromTauri" src/lib/shell/browser/browserBackend.ts
```
For each command, note whether the Rust fn takes a named struct param (like `input:`) or flattened named params. Record the list in the receipt file. Commands taking `input: SomeStruct` must be invoked with `{ input: {...} }`.

- [ ] **Step 2: Write the failing test**

Append to `scripts/browserBackend.test.mjs` a source-contract test that fails while the wrapper is missing:

```js
test('tauri browser backend wraps struct-arg commands in an input key', () => {
  const source = readFileSync(new URL('../src/lib/shell/browser/browserBackend.ts', import.meta.url), 'utf8');
  assert.match(
    source,
    /navigate_browser_tab\(input: BrowserBackendNavigationInput\): Promise<void> \{\s*return invokeBrowserCommandFromTauri<void>\('navigate_browser_tab', \{ input \}\);/,
    'navigate_browser_tab must nest its payload under an input key to match the Rust command signature'
  );
});
```

(Match the file's existing import style for `readFileSync`/`assert` — the suite already runs under `node --experimental-strip-types`.)

- [ ] **Step 3: Run test to verify it fails**

Run: `node --experimental-strip-types scripts/browserBackend.test.mjs`
Expected: FAIL on the new assertion (source still passes `input` unwrapped).

- [ ] **Step 4: Implement the fix**

In `src/lib/shell/browser/browserBackend.ts:241-243`:

```ts
navigate_browser_tab(input: BrowserBackendNavigationInput): Promise<void> {
  return invokeBrowserCommandFromTauri<void>('navigate_browser_tab', { input });
}
```

Apply the same `{ input }` wrapper to every other method the Step 1 audit flagged as a named-struct command (extend the Step 2 regex test with one assertion per fixed method).

- [ ] **Step 5: Run test to verify it passes**

Run: `node --experimental-strip-types scripts/browserBackend.test.mjs`
Expected: PASS (all assertions).

- [ ] **Step 6: Gates**

Run: `pnpm check && pnpm check:svelte && pnpm build`
Expected: green, 0 /next svelte errors.

---

### Task 2: Usage history native read — surface the real error and stop depending on PATH

The native build shows the generic "Usage history could not be read." The catch at `src/lib/shell/usage/usageStore.svelte.ts:80-82` discards non-`Error` rejections — and Tauri `invoke` rejects with a **string**. Separately, `usage_db.rs:402-420` shells out to `sqlite3` resolved from PATH, which is fragile in a packaged .app.

**Files:**
- Modify: `src/lib/shell/usage/usageStore.svelte.ts:80-82`
- Modify: `src-tauri/src/usage_db.rs` (`sqlite_command`, ~line 418)
- Test: `src-tauri/src/usage_db.rs` (new unit test), `scripts/usageViewModel.test.mjs` (error-surfacing assertion if the store is covered there; otherwise a source-contract assertion in the same file)

**Interfaces:**
- Consumes: `UsageDb::open(path)` and `sqlite_command()` in `usage_db.rs`; `usageState.error: string | null` in the store.
- Produces: `usageState.error` always contains the backend's actual error string; `sqlite_command()` falls back to `/usr/bin/sqlite3` when the PATH lookup cannot spawn.

- [ ] **Step 1: Write the failing Rust test**

In `src-tauri/src/usage_db.rs` tests module:

```rust
#[test]
fn sqlite_binary_falls_back_to_the_system_path_when_env_override_is_missing() {
    // resolve_sqlite_binary is the pure resolution helper introduced in Step 3.
    let fallback = resolve_sqlite_binary(Some(std::ffi::OsString::from(
        "/nonexistent/definitely-not-sqlite3",
    )));
    assert_eq!(fallback, std::path::PathBuf::from("/nonexistent/definitely-not-sqlite3"));
    let default = resolve_sqlite_binary(None);
    // With no override: prefer bare "sqlite3" if it spawns, else the absolute macOS binary.
    assert!(
        default == std::path::PathBuf::from("sqlite3")
            || default == std::path::PathBuf::from("/usr/bin/sqlite3")
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml sqlite_binary_falls_back`
Expected: FAIL to compile — `resolve_sqlite_binary` does not exist.

- [ ] **Step 3: Implement the resolver**

In `src-tauri/src/usage_db.rs`, replace the body of `sqlite_command()`:

```rust
fn resolve_sqlite_binary(override_bin: Option<std::ffi::OsString>) -> std::path::PathBuf {
    if let Some(bin) = override_bin {
        return bin.into();
    }
    // Packaged .app processes often run with a minimal PATH; probe the bare
    // name and fall back to the macOS system binary.
    let probe = Command::new("sqlite3")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    if probe.is_ok() {
        std::path::PathBuf::from("sqlite3")
    } else {
        std::path::PathBuf::from("/usr/bin/sqlite3")
    }
}

fn sqlite_command() -> Command {
    let binary = resolve_sqlite_binary(std::env::var_os("MCB_SQLITE_BIN"));
    let mut command = Command::new(binary);
    // ... keep the existing arg setup below unchanged
```

Keep the existing `"SQLite is unavailable: {error}"` message but append the resolved path so failures are diagnosable: `format!("SQLite is unavailable ({}): {error}", binary_display)`.

- [ ] **Step 4: Run Rust tests**

Run: `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml usage`
Expected: new test PASS, all existing usage tests PASS.

- [ ] **Step 5: Write the failing frontend test, then fix error surfacing**

Add to `scripts/usageViewModel.test.mjs` a source-contract assertion:

```js
test('usage store surfaces string rejections from invoke', () => {
  const source = readFileSync(new URL('../src/lib/shell/usage/usageStore.svelte.ts', import.meta.url), 'utf8');
  assert.match(
    source,
    /typeof error === 'string' \? error :/,
    'the catch must pass through Tauri string rejections instead of masking them'
  );
});
```

Run it, see it FAIL, then change `usageStore.svelte.ts:80-82`:

```ts
} catch (error) {
  usageState.error =
    error instanceof Error ? error.message
    : typeof error === 'string' ? error
    : 'Usage history could not be read.';
  return null;
}
```

Run: `node --experimental-strip-types scripts/usageViewModel.test.mjs` — PASS.

- [ ] **Step 6: Gates**

Run: `pnpm check && pnpm check:svelte && pnpm build`
Expected: green.

---

### Task 3: ACP frame router — stop discarding `session/update` notifications

Today every frame that doesn't match the awaited request id is dropped (`acp_client.rs:349-371` `continue`), so live updates die. Introduce a reader that routes: responses → their pending request; `session/update` (and legacy `sessionUpdate`) notifications → an update sink.

**Files:**
- Modify: `src-tauri/src/agent_conversation/providers/acp_client.rs`
- Test: same file's tests module (extend the fake-process harness used by `acp_initialize_new_prompt_image_config_correlations_cancel_and_close` at acp_client.rs:502-586)

**Interfaces:**
- Consumes: `self.process.next_json()` / `self.process.write_json()` (unchanged transport).
- Produces:
  - `pub type AcpUpdateSink = std::sync::Arc<dyn Fn(serde_json::Value) + Send + Sync>;`
  - `AcpClient::set_update_sink(&mut self, sink: AcpUpdateSink)` — sink receives the full notification `params` object of every `session/update` frame.
  - `fn route_stray_frame(&self, frame: &serde_json::Value)` — called by EVERY frame-reading loop (`request`, `prompt_once`'s loop at acp_client.rs:184-241, and any turn/response loop) in place of silently continuing.

- [ ] **Step 1: Write the failing test**

In the acp_client tests module, using the existing fake process harness:

```rust
#[test]
fn session_update_notifications_reach_the_sink_instead_of_being_dropped() {
    // Queue: one session/update notification frame, then the matching response.
    let frames = vec![
        json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {
                "sessionId": "sess-1",
                "update": { "sessionUpdate": "agent_message_chunk",
                            "content": { "type": "text", "text": "Hello" } }
            }
        }),
        json!({ "jsonrpc": "2.0", "id": 1, "result": {} }),
    ];
    let mut client = fake_client_with_frames(frames); // mirror the existing harness constructor
    let seen: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>> = Default::default();
    let sink_seen = seen.clone();
    client.set_update_sink(std::sync::Arc::new(move |params| {
        sink_seen.lock().unwrap().push(params);
    }));
    block_on(client_request(&mut client)); // any request that reads both frames
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1, "the notification must be routed, not discarded");
    assert_eq!(seen[0]["update"]["sessionUpdate"], "agent_message_chunk");
}
```

Also assert the legacy method spelling is accepted (second test, identical but `"method": "sessionUpdate"` — the vendored schema's own wire test uses that spelling, `agent-client-protocol-schema-1.5.0/src/rpc.rs:356-392`).

- [ ] **Step 2: Run test to verify it fails**

Run: `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml session_update_notifications`
Expected: FAIL to compile (`set_update_sink` missing).

- [ ] **Step 3: Implement routing**

In `acp_client.rs`:

```rust
pub type AcpUpdateSink = std::sync::Arc<dyn Fn(serde_json::Value) + Send + Sync>;

// field on AcpClient:
update_sink: Option<AcpUpdateSink>,

pub fn set_update_sink(&mut self, sink: AcpUpdateSink) {
    self.update_sink = Some(sink);
}

fn route_stray_frame(&self, frame: &serde_json::Value) {
    let method = frame.get("method").and_then(serde_json::Value::as_str);
    if matches!(method, Some("session/update") | Some("sessionUpdate")) {
        if let (Some(sink), Some(params)) = (&self.update_sink, frame.get("params")) {
            sink(params.clone());
        }
    }
}
```

In `request` (acp_client.rs:349-371) replace the bare `continue` with:

```rust
if frame.get("id") != Some(&json!(id)) {
    self.route_stray_frame(&frame);
    continue;
}
```

In `prompt_once`'s manual loop (acp_client.rs:184-241): call `self.route_stray_frame(&frame)` for every non-response frame BEFORE its existing inline text extraction (keep the extraction — `prompt_once` callers still rely on the aggregated text). Apply the same one-line routing to any other loop in the file that reads frames and skips non-matching ones (grep `next_json` to find them all).

- [ ] **Step 4: Run tests**

Run: `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml acp`
Expected: both new tests PASS; the existing pins `acp_initialize_new_prompt_image_config_correlations_cancel_and_close` and `acp_load_and_resume_use_distinct_session_methods` PASS unchanged.

---

### Task 4: Live event pump — session updates become frontend conversation events

Route the sink's raw params through the manager into the existing normalized pipeline and out on the existing frontend channel. After this task, a prompt's streamed chunks arrive in the UI as `assistantDelta` events with no transcript tailing involved.

**Files:**
- Modify: `src-tauri/src/agent_conversation/manager.rs`
- Modify: `src-tauri/src/agent_conversation/mod.rs` (activation call site gains the emitter)
- Test: `src-tauri/src/agent_conversation/manager.rs` tests module

**Interfaces:**
- Consumes: `AcpClient::set_update_sink` (Task 3); `emit_payload(owned_id, generation, payload) -> Result<AgentConversationEvent, String>` (manager.rs:265-308, unchanged); `AgentConversationPayload` (protocol.rs:467-516, unchanged).
- Produces:
  - `pub type ConversationEmitter = std::sync::Arc<dyn Fn(AgentConversationEvent) + Send + Sync>;`
  - `AgentRuntimeManager::set_emitter(&self, emitter: ConversationEmitter)` — stored once; the Tauri layer installs `Arc::new(move |event| { let _ = app_handle.emit("agent-conversation-event", event); })` during setup in `mod.rs`.
  - `pub fn payload_from_session_update(params: &serde_json::Value) -> Option<AgentConversationPayload>` — pure mapping, unit-testable.
  - `activate` wires the client sink to: map → `emit_payload` → emitter.
  - Turn lifecycle: when `prompt` starts a turn, emit `Turn { state: Started }`; when the `session/prompt` response resolves, emit `Turn { state: Completed }` (or the error payload on failure).

- [ ] **Step 1: Write the failing mapping tests**

```rust
#[test]
fn session_updates_map_to_conversation_payloads() {
    let chunk = json!({
        "sessionId": "s", "update": {
            "sessionUpdate": "agent_message_chunk",
            "content": { "type": "text", "text": "Hi" },
            "messageId": "m1"
        }
    });
    assert_eq!(
        payload_from_session_update(&chunk),
        Some(AgentConversationPayload::AssistantDelta { item_id: "m1".into(), delta: "Hi".into() })
    );

    let tool = json!({
        "sessionId": "s", "update": {
            "sessionUpdate": "tool_call",
            "toolCallId": "t1", "title": "Read file", "status": "in_progress"
        }
    });
    match payload_from_session_update(&tool) {
        Some(AgentConversationPayload::Tool { item_id, name, .. }) => {
            assert_eq!(item_id, "t1");
            assert_eq!(name, "Read file");
        }
        other => panic!("expected Tool payload, got {other:?}"),
    }

    let thought = json!({
        "sessionId": "s", "update": {
            "sessionUpdate": "agent_thought_chunk",
            "content": { "type": "text", "text": "..." }
        }
    });
    assert_eq!(payload_from_session_update(&thought), None, "thought chunks are not rendered in v1");
}
```

Mapping table to implement (unknown kinds → `None`, never panic):

| `update.sessionUpdate` | `AgentConversationPayload` |
| --- | --- |
| `agent_message_chunk` | `AssistantDelta { item_id: messageId or "assistant-live", delta: content.text }` |
| `user_message_chunk` | `UserMessage { item_id: messageId or "user-live", text, completed: false }` |
| `tool_call` | `Tool { item_id: toolCallId, name: title, state: from status (pending/in_progress→Running, completed→Completed, failed→Failed), summary: None }` |
| `tool_call_update` | `Tool` (same field sourcing; missing title → name `"tool"`) |
| `plan`, `agent_thought_chunk`, anything else | `None` (v1) |

Check the exact `ToolState` variant names in protocol.rs before coding the status mapping and use those names verbatim.

- [ ] **Step 2: Run to verify failure**

Run: `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml session_updates_map`
Expected: FAIL to compile.

- [ ] **Step 3: Implement mapping + wiring**

Implement `payload_from_session_update` in manager.rs. Add the emitter:

```rust
pub type ConversationEmitter = std::sync::Arc<dyn Fn(AgentConversationEvent) + Send + Sync>;

// field: emitter: Arc<Mutex<Option<ConversationEmitter>>> on AgentRuntimeManager
pub fn set_emitter(&self, emitter: ConversationEmitter) { /* store */ }

fn pump_update(&self, owned_id: &str, generation: u64, params: serde_json::Value) {
    if let Some(payload) = payload_from_session_update(&params) {
        if let Ok(event) = self.emit_payload(owned_id, generation, payload) {
            if let Some(emitter) = self.emitter.lock().ok().and_then(|e| e.clone()) {
                emitter(event);
            }
        }
    }
}
```

In `activate` (manager.rs:190-263), after constructing the adapter and before wrapping it (`manager.rs:239`), install the sink — clone `self`'s `Arc` internals (manager is already `Arc`-shared state, manager.rs:58-62), plus `owned_id`/`generation`:

```rust
adapter.client_mut().set_update_sink(std::sync::Arc::new(move |params| {
    manager_handle.pump_update(&owned_id_for_sink, generation, params);
}));
```

(If `AcpRuntimeAdapter` doesn't expose its client, add `pub fn client_mut(&mut self) -> &mut AcpClient` in providers/acp.rs.)

Turn lifecycle: in the manager's prompt path, `emit_payload(.., Turn { turn_id, state: TurnState::Started })` before delegating to the runtime and `Turn { state: Completed }` when the prompt future resolves Ok (map Err to the existing `Error` payload). Route these through the same emitter.

In `mod.rs` setup (where the manager is created with the Tauri app handle available), install:

```rust
manager.set_emitter(std::sync::Arc::new(move |event| {
    let _ = app_handle.emit("agent-conversation-event", event);
}));
```

- [ ] **Step 4: Write the failing end-to-end pump test, then make it pass**

```rust
#[test]
fn pumped_updates_flow_through_emit_payload_to_the_emitter() {
    let (manager, owned_id, generation) = manager_with_active_session(); // reuse existing test setup helpers from manager.rs:1022-1063
    let seen: std::sync::Arc<std::sync::Mutex<Vec<AgentConversationEvent>>> = Default::default();
    let sink = seen.clone();
    manager.set_emitter(std::sync::Arc::new(move |event| sink.lock().unwrap().push(event)));
    manager.pump_update(&owned_id, generation, json!({
        "sessionId": "s",
        "update": { "sessionUpdate": "agent_message_chunk",
                     "content": { "type": "text", "text": "Hi" } }
    }));
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert!(matches!(seen[0].payload, AgentConversationPayload::AssistantDelta { .. }));
    assert_eq!(seen[0].sequence, 0);
}
```

Run: `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml pumped_updates`
Expected: PASS after wiring.

- [ ] **Step 5: Capabilities pin**

Append `"acpLiveConversationEvents"` to `backend_capabilities()` and to the pinned list in the test at main.rs:8254-8289. Run:
`RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml backend_capabilities`
Expected: PASS.

- [ ] **Step 6: Full Rust suite**

Run: `RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 0 failures (252 pre-existing + new).

---

### Task 5: ACP-only session creation — delete the force-raw handoff

New sessions stop spawning PTYs; selecting a session never tears down the structured runtime.

**Files:**
- Modify: `src/routes/next/+page.svelte:983-1002` (force-raw branch), `src/routes/next/+page.svelte:1128-1149` (`startNewSession`)
- Modify: `src/lib/shell/conversation/conversationService.ts` (no signature changes expected; `ensureStructuredConversation` at :381-395 is reused)
- Test: `scripts/agentConversationHandoff.test.mjs`, `scripts/conversationSessionIsolation.test.mjs`, `scripts/agentConversationStore.test.mjs`

**Interfaces:**
- Consumes: `ensureStructuredConversation({ ownedId, provider, cwd, nativeSessionId })` (conversationService.ts:381-395); `addOwnedSession` / `updateOwnedSession` / `selectOwned` (existing, +page.svelte:1128-1149 context).
- Produces: `startNewSession(request: NewSessionRequest)` creates an owned session with NO `ptySessionId` for agent providers and calls `ensureStructuredConversation`; the selection path never calls `closeStructuredConversation` or `setConversationMode(ownedId, 'raw')` for live sessions. Plain-terminal sessions (non-agent) keep the PTY path.

- [ ] **Step 1: Update the pinned handoff tests to the new contract (failing first)**

`scripts/agentConversationHandoff.test.mjs:12-96` pins the old behavior (structured/raw handoff on PTY presence). Rewrite the pins to assert the NEW contract as source-contract tests:

```js
test('selecting a live agent session must not force raw mode or close the structured runtime', () => {
  const source = readFileSync(new URL('../src/routes/next/+page.svelte', import.meta.url), 'utf8');
  assert.doesNotMatch(
    source,
    /closeStructuredConversation\(ownedId\);\s*setConversationMode\(ownedId, 'raw'\)/,
    'the force-raw selection branch must be gone'
  );
});

test('startNewSession creates agent sessions through ACP, not a PTY', () => {
  const source = readFileSync(new URL('../src/routes/next/+page.svelte', import.meta.url), 'utf8');
  assert.match(
    source,
    /async function startNewSession[\s\S]*?ensureStructuredConversation\(\{/,
    'startNewSession must call ensureStructuredConversation'
  );
  assert.doesNotMatch(
    source,
    /async function startNewSession[\s\S]{0,1200}service\.startOwned/,
    'startNewSession must not spawn a PTY for agent sessions'
  );
});
```

Run: `node --experimental-strip-types scripts/agentConversationHandoff.test.mjs`
Expected: FAIL (old code still present).

- [ ] **Step 2: Implement the frontend change**

In `+page.svelte:983-1002`, delete the `if (selected.ptySessionId) { ... return; }` block entirely, leaving the unconditional `ensureStructuredConversation` call (keep its existing error handling). In `startNewSession` (:1128-1149) replace the PTY spawn:

```ts
async function startNewSession(request: NewSessionRequest): Promise<void> {
  if (!service || disposed) return;
  const owned = {
    ...createFreshSession({ cwd: request.cwd, title: request.title }),
    agent: request.agent,
    resumeCommand: request.command
  };
  addOwnedSession(owned);
  updateOwnedSession(owned.ownedId, { state: 'live' });
  try {
    await ensureStructuredConversation({
      ownedId: owned.ownedId,
      provider: owned.agent,
      cwd: owned.cwd
    });
  } catch (error) {
    updateOwnedSession(owned.ownedId, { state: 'exited' });
    rail.error = `could not start ${owned.agent} session: ${describeError(error)}`;
    return;
  }
  await selectOwned(owned.ownedId);
}
```

Keep the plain-terminal creation path (whatever non-agent session type exists) on `service.startOwned` — the audit in Step 1's grep tells you if `startNewSession` serves both; if it does, branch on the provider being an agent (`request.agent === 'codex' || request.agent === 'claude'`).

- [ ] **Step 3: Sweep the other pinned scripts**

Run each; update only assertions that pin the deleted PTY-first behavior (do NOT weaken unrelated pins):

```bash
node --experimental-strip-types scripts/agentConversationHandoff.test.mjs
node --experimental-strip-types scripts/conversationSessionIsolation.test.mjs
node --experimental-strip-types scripts/agentConversationStore.test.mjs
node --experimental-strip-types scripts/agentConversationTerminalProjection.test.mjs
```
Expected end state: all PASS. `agentConversationTerminalProjection.test.mjs` must still pass UNMODIFIED (projection remains for externally-started sessions).

- [ ] **Step 4: Gates**

Run: `pnpm check && pnpm check:svelte && pnpm build`
Expected: green, 0 /next errors.

---

### Task 6: Raw mode becomes a read-only inspector

With no PTY behind agent sessions, "Open raw terminal" must not present an interactive terminal. Raw mode renders the session's recent canonical events as a read-only scrollback.

**Files:**
- Modify: `src/lib/shell/components/ConversationSurface.svelte` (structured condition :60-65, terminal mount :222-224, raw branch :272-278)
- Modify: `src/lib/shell/components/conversation/ConversationHeader.svelte:15-18` (button label → "Inspect events")
- Test: `scripts/agentConversationStore.test.mjs` (mode semantics), new `scripts/conversationInspector.test.mjs`

**Interfaces:**
- Consumes: `conversation?.mode` (`'structured' | 'raw'`, conversationStore.svelte.ts:78-89); the session's event list already held by the store (the reducer's applied events — expose a `recentEvents` selector if one does not exist: `export function conversationRecentEvents(ownedId: string): ConversationTimelineEntry[]`).
- Produces: raw mode renders `<pre class="inspector">` rows of `[sequence] kind: summary` from the store; no xterm instance is created for agent sessions without a `ptySessionId`; the toggle is reversible in both directions (the Task 5 deletion already removed the runtime teardown that made "Return to structured" fail).

- [ ] **Step 1: Write the failing test**

Create `scripts/conversationInspector.test.mjs`:

```js
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('raw mode renders a read-only inspector, not an interactive terminal, for pty-less sessions', () => {
  const source = readFileSync(new URL('../src/lib/shell/components/ConversationSurface.svelte', import.meta.url), 'utf8');
  assert.match(source, /data-testid="conversation-inspector"/, 'inspector container must exist');
  assert.match(
    source,
    /\{#if [^}]*ptySessionId[^}]*\}[\s\S]*?TerminalSurface/,
    'TerminalSurface must only mount when the session actually has a PTY'
  );
});
```

Run: `node --experimental-strip-types scripts/conversationInspector.test.mjs`
Expected: FAIL.

- [ ] **Step 2: Implement**

In `ConversationSurface.svelte`: guard the always-mounted terminal (:222-224) behind the session having a `ptySessionId`; in the raw branch (:272-278) render for PTY-less sessions:

```svelte
{#if !active?.ptySessionId}
  <pre class="inspector" data-testid="conversation-inspector" aria-label="Session event inspector">
{#each inspectorRows as row (row.key)}[{row.sequence}] {row.kind}: {row.summary}
{/each}</pre>
{:else}
  <TerminalSurface ... existing props ... readonly />
{/if}
```

with `inspectorRows` derived from the store's recent events (sequence, payload kind, first 120 chars of text/summary). Rename the header button (ConversationHeader.svelte:15-18) to "Inspect events" / "Back to conversation".

- [ ] **Step 3: Run tests**

Run: `node --experimental-strip-types scripts/conversationInspector.test.mjs && node --experimental-strip-types scripts/agentConversationStore.test.mjs`
Expected: PASS.

- [ ] **Step 4: Full verification sweep + receipt**

```bash
RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml
pnpm check && pnpm check:svelte && pnpm build
for f in scripts/*.test.mjs; do node --experimental-strip-types "$f" || echo "FAIL: $f"; done
```
Expected: cargo 0 failures; gates green; script failures limited to the 2 pre-existing old-shell ones (`sourceUi`, `workspaceSnapshotPlan`). Write `docs/superpowers/evidence/tsk-808/phase1/receipt.md` summarizing each task with file:line receipts and the verification output.

---

## Native acceptance (user, after controller merges)

1. `pnpm tauri:dev:next` from the worktree.
2. Start a new Codex session → structured view appears immediately, stays structured, streams deltas live (no transcript lag).
3. "Inspect events" toggles to the read-only inspector and back.
4. Browser panel: enter a URL → page loads (Task 1).
5. Stats & Usage → Refresh → history loads, or shows a SPECIFIC error message (Task 2).
