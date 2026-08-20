# Agent Harness Slice 1 — Broker, Messaging, Visibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Agents in Assembly can message each other and the human through an app-owned broker with receipts; a manually assembled group shows in the sessions rail with live status, a hover card, and a message timeline. No workflow engine changes yet (slice 2).

**Architecture:** Broker = envelope log in the existing SQLite session store + per-agent delivery queues drained at quiescent boundaries via the existing `send_message` path. Workers reach it through a small `assembly-broker` MCP stdio server (node, mirrors `codex-acp-bridge` style) that connects back to the app over a unix socket. UI reuses the rail group/section machinery and the existing `agent-conversation-event` plumbing pattern.

**Tech Stack:** Rust (tokio, rusqlite) in `core/` + `src-tauri/`; node ESM for the MCP server; Svelte 5 + shell kit for UI.

**Spec:** `docs/superpowers/specs/2026-08-20-agent-harness-workflows-design.md` (binding). Recon anchors: `.superpowers/sdd/2026-08-20-tsk-935-wave4/harness-slice1-recon.md` (all line numbers below verified there, 2026-08-20).

## Global Constraints

- No API-key execution paths — providers keep riding subscription CLI auth exactly as spawned today (`providers/mod.rs:129-227`).
- Broker delivery only at quiescent boundaries (`session_is_quiescent`, `manager.rs:5264-5273`); never mid-turn injection.
- Receipts vocabulary exactly: `queued`, `delivered`, `failed`, `expired`.
- Status vocabulary exactly: `running`, `waiting`, `blocked`, `idle`, `done`, `failed`.
- UI: shell tokens, px only, never CSS `:has()`, kit controls from `src/lib/components/ui` for any picker/input/button.
- Commits: body ends `Committed-by: Claude` (or the implementing harness); never a co-author trailer; guide row or `Architecture: unchanged` per the commit hook.
- KISS/YAGNI: smallest diff that passes the named test; no new abstractions beyond those named here; ~2x the stated diff budget or an unnamed new file = STOP and report back.
- Tests first in every task; run only the named suites (no full-suite sweeps); `MSBUILDDISABLENODEREUSE=1` n/a (no .NET here); one build at a time.

---

### Task 1: Broker tables — SQLite migration v5

**Files:**
- Modify: `core/src/session_store.rs` (`SCHEMA_VERSION` at `:7`, `from_connection` match at `:159-318`)
- Test: same file, `#[cfg(test)]` module (existing pattern at bottom of file)

**Interfaces:**
- Produces: tables `workflow_groups(id TEXT PRIMARY KEY, name TEXT NOT NULL, orchestrator_owned_id TEXT, created_at_ms INTEGER NOT NULL, closed_at_ms INTEGER)` and `workflow_messages(id TEXT PRIMARY KEY, group_id TEXT NOT NULL REFERENCES workflow_groups(id), from_agent TEXT NOT NULL, to_agent TEXT NOT NULL, kind TEXT NOT NULL, body TEXT NOT NULL, receipt TEXT NOT NULL, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL)`; index `idx_workflow_messages_group ON workflow_messages(group_id, created_at_ms)`.

- [ ] **Step 1: Failing test** `broker_tables_exist_after_migration`: open `SessionStore::open_in_memory()`, prepare `SELECT id FROM workflow_groups` and `SELECT receipt FROM workflow_messages` — both must prepare without error. Run: `cargo test -p mcb-core broker_tables_exist_after_migration` → FAIL (no such table).
- [ ] **Step 2: Implement** bump `SCHEMA_VERSION` to 5; add `4 => { ... }` arm copying the shape of arm 3 (`:292-307`): immediate transaction, `execute_batch` with the two CREATE TABLE + index, `pragma_update`, commit.
- [ ] **Step 3: Test passes**; also run existing store tests: `cargo test -p mcb-core session_store` → all green.
- [ ] **Step 4: Commit** `feat: broker tables for workflow groups and messages (schema v5)`.

Diff budget: ~60 lines. No new files.

### Task 2: Broker core — envelopes, receipts, queries

**Files:**
- Create: `core/src/broker.rs` (declare in `core/src/lib.rs`)
- Test: inline `#[cfg(test)]`

**Interfaces:**
- Produces (consumed by Tasks 3-6):
```rust
pub enum MessageKind { Message, Status, DecisionRequest, DecisionResponse, System }
pub enum Receipt { Queued, Delivered, Failed, Expired }
pub struct Envelope { pub id: String, pub group_id: String, pub from_agent: String,
  pub to_agent: String, pub kind: MessageKind, pub body: String, pub receipt: Receipt,
  pub created_at_ms: i64, pub updated_at_ms: i64 }
impl SessionStore {
  pub fn create_group(&self, name: &str, orchestrator: Option<&str>, member_owned_ids: &[String]) -> Result<String>;
  pub fn append_message(&self, group_id: &str, from: &str, to: &str, kind: MessageKind, body: &str) -> Result<Envelope>; // receipt starts Queued
  pub fn set_receipt(&self, message_id: &str, receipt: Receipt) -> Result<()>;   // Queued may move anywhere; Delivered/Failed/Expired are terminal — error otherwise
  pub fn pending_for(&self, group_id: &str, to: &str) -> Result<Vec<Envelope>>;  // Queued only, oldest first
  pub fn events_for(&self, group_id: &str, since_ms: Option<i64>) -> Result<Vec<Envelope>>;
  pub fn list_open_groups(&self) -> Result<Vec<(String, String, Option<String>, Vec<String>)>>;
}
```
  (Group membership: store member ids as `system`-kind envelopes `kind='system', body='member:<owned_id>'` at create time — no third table.)

- [ ] **Step 1: Failing tests** (names fixed): `append_starts_queued_and_events_return_in_order`, `terminal_receipts_refuse_change`, `pending_returns_only_queued_for_recipient`. Run: `cargo test -p mcb-core broker` → FAIL (module missing).
- [ ] **Step 2: Implement** `broker.rs` with the exact signatures above; ids via the store's existing id helper (see how `sessions` rows are created in v0 arm).
- [ ] **Step 3: Tests pass**: `cargo test -p mcb-core broker`.
- [ ] **Step 4: Commit** `feat: broker envelope log with receipt state machine`.

Diff budget: ~220 lines including tests. One new file (named here).

### Task 3: Status normalization + broker Tauri events

**Files:**
- Create: `src-tauri/src/agent_conversation/broker_status.rs` (pure mapping + tests)
- Modify: `src-tauri/src/agent_conversation/manager.rs` — hook the two seams recon verified: raw update liveness (`:3938-3987`) and ordered prompt completion (`:4213-4370`); emit alongside existing canonical events (`:4513-4549`)
- Modify: `src-tauri/src/main.rs:5961-5964` area — emit `workflow-broker-event` next to `agent-conversation-event`

**Interfaces:**
- Produces: `pub enum AgentWorkStatus { Running, Waiting, Blocked, Idle, Done, Failed }`; `pub fn status_for(turn_active: bool, awaiting_permission_or_input: bool, decision_pending: bool, runtime_error: bool, session_closed: bool) -> AgentWorkStatus`; Tauri event `workflow-broker-event` payload `{ groupId, envelope }` (serde camelCase) fired on every append/receipt change/status transition for sessions that belong to an open group.
- Consumes: Task 2 `append_message` (`kind: Status`, body = the status string).

- [ ] **Step 1: Failing tests** in `broker_status.rs`: table-driven `status_mapping_covers_all_inputs` (decision_pending→Blocked beats awaiting→Waiting beats turn_active→Running; error→Failed; closed→Done; else Idle). Run: `cargo test -p mac-command-bar-webview-preview broker_status` → FAIL.
- [ ] **Step 2: Implement** mapping; wire manager: on the two seams, if the session's owned_id is in an open group and the derived status changed, append a Status envelope and emit the event. Guard with a `HashMap<String, AgentWorkStatus>` on the manager (last known).
- [ ] **Step 3: Tests pass**; then `cargo test -p mac-command-bar-webview-preview agent_conversation` (175 existing tests must stay green).
- [ ] **Step 4: Commit** `feat: normalize session activity to six broker statuses`.

Diff budget: ~200 lines. One new file (named).

### Task 4: Delivery queue — prompt-boundary drain

**Files:**
- Modify: `src-tauri/src/agent_conversation/manager.rs` — after `suspend_if_quiescent` sites and on `turn.completed` handling (`:4213-4370`), drain pending broker messages for that session
- Test: existing manager test module pattern

**Interfaces:**
- Consumes: Task 2 `pending_for`/`set_receipt`; existing `send_message` (`manager.rs:1206-1245`).
- Produces: delivered messages arrive as a prompt formatted exactly `[workflow message from <from_agent>]\n<body>`; receipt `Delivered` on successful prompt start, `Failed` on error; messages older than 30 minutes at drain time → `Expired` (constant `BROKER_MESSAGE_TTL_MS: i64 = 30 * 60 * 1000`).

- [ ] **Step 1: Failing test** `queued_message_delivers_when_session_goes_quiescent`: using the existing in-memory manager test harness (see the 175-test module for the pattern — e.g. how `config_change_on_a_suspended_session_leaves_no_process_behind` drives state), enqueue via `append_message`, drive the session to quiescent, assert receipt became `Delivered` and a user-role prompt containing `[workflow message from` was recorded. Also `stale_message_expires_instead_of_delivering`. Run: `cargo test -p mac-command-bar-webview-preview queued_message` → FAIL.
- [ ] **Step 2: Implement** the drain (one message per quiescent transition, then re-check — FIFO preserved; a delivery starts a turn, so the next drains on the following boundary).
- [ ] **Step 3: Tests pass** + `agent_conversation` suite still green.
- [ ] **Step 4: Commit** `feat: broker delivery at quiescent prompt boundaries with receipts`.

Diff budget: ~150 lines. No new files.

### Task 5: assembly-broker MCP server + app socket

**Files:**
- Create: `tools/assembly-broker-mcp/server.mjs` (node ESM, no deps — mirror the JSON-RPC framing of `tools/codex-acp-bridge/bridge.mjs:28-63,100-153`)
- Create: `src-tauri/src/agent_conversation/broker_socket.rs` — tokio `UnixListener` at `$TMPDIR/assembly-broker-<pid>.sock`, line-delimited JSON
- Modify: `src-tauri/src/main.rs:5982` area — start the listener with the manager/store handles
- Test: `tauri-svelte-preview/scripts/brokerMcpServer.test.mjs` (node test, spawns server.mjs against a mock socket)

**Interfaces:**
- MCP server implements protocol methods `initialize`, `tools/list`, `tools/call` (2024-11-05 protocol rev, stdio framing = newline-delimited JSON-RPC as the provider CLIs expect) exposing tools:
  `send_message {to: string, body: string}` · `request_decision {question: string}` (returns after the decision-response envelope arrives — long-poll the socket) · `report_status {note: string}`.
- Env contract (set by Task 6 at spawn): `ASSEMBLY_BROKER_SOCKET` (path), `ASSEMBLY_AGENT_ID` (owned_id), `ASSEMBLY_GROUP_ID`.
- Socket wire: request `{op:"append"|"pending"|"await_decision", ...envelope fields}` → response `{ok:true, ...}`; the Rust side calls the Task 2 APIs and emits the Task 3 event.

- [ ] **Step 1: Failing node test** `broker mcp server lists three tools and forwards send_message`: spawn `server.mjs` with env pointing at a script-created mock unix socket; drive `initialize` + `tools/list` + `tools/call send_message`; assert the mock socket received the append op. Run: `node --test scripts/brokerMcpServer.test.mjs` → FAIL.
- [ ] **Step 2: Implement** server.mjs (~150 lines) and `broker_socket.rs` (~120 lines).
- [ ] **Step 3: Tests pass**; add rust test `socket_append_lands_in_store` (connect to listener, send append op, read envelope back via `events_for`). `cargo test -p mac-command-bar-webview-preview broker_socket`.
- [ ] **Step 4: Commit** `feat: assembly-broker MCP server bridged to the app over a unix socket`.

Diff budget: ~350 lines across the named new files. Add `test:broker-mcp` script to `tauri-svelte-preview/package.json`.

### Task 6: Per-provider MCP registration (claude + codex; agy = report only)

**Files:**
- Modify: `src-tauri/src/agent_conversation/providers/mod.rs:129-227` (manifest args/env per provider) and the activation seam `manager.rs:949-975` (pass group/agent env when the session belongs to a group)
- Modify: `tools/codex-acp-bridge/bridge.mjs:28-63` (forward `-c` config overrides for `mcp_servers` when env present)
- Test: rust unit on manifest construction

**Interfaces:**
- Consumes: Task 5 env contract.
- Contract per provider — VERIFY against the installed CLI before wiring, and paste the `--help` line into the report:
  - claude-agent-acp: pass `--mcp-config <path>` with a generated JSON file declaring `assembly-broker` as a stdio server (`node tools/assembly-broker-mcp/server.mjs`). If the adapter does not forward the flag to the claude CLI, set the documented env fallback and report which was used.
  - codex bridge: `-c mcp_servers.assembly-broker.command=...` style overrides on the `codex app-server` spawn (or `thread/start` config if the app-server protocol carries it — check with `codex app-server --help` / protocol docs and report).
  - agy: investigate flags only; if no MCP support, REPORT and leave agy without a bridge this slice (fallback file-inbox is slice-2 scope). Do not build a fallback now.
- Registration happens only for sessions that are members of an open group — plain sessions spawn exactly as today (zero-diff behavior, assert in test).

- [ ] **Step 1: Failing rust test** `group_member_manifest_carries_broker_env_and_plain_sessions_do_not`. → FAIL.
- [ ] **Step 2: Implement** per verified flags.
- [ ] **Step 3: Tests pass** + `cargo test -p mac-command-bar-webview-preview agent_conversation` green.
- [ ] **Step 4: Live check** (controller-assisted, no second app instance): spawn one claude session in a scratch group from the dev build and confirm a `send_message` tool call lands one envelope in the DB (`sqlite3` query pasted in report).
- [ ] **Step 5: Commit** `feat: register assembly-broker MCP server for group member sessions`.

Diff budget: ~180 lines. No new files.

### Task 7: Group service + rail group UI

**Files:**
- Create: `src/lib/shell/workflowGroups/workflowGroupStore.svelte.ts` + `workflowGroupService.ts` (listen `workflow-broker-event`, invoke new Tauri commands `create_workflow_group`, `list_workflow_groups`, `send_workflow_message` — add these thin commands in `src-tauri/src/agent_conversation/mod.rs` next to `:121-145`)
- Modify: `src/lib/shell/components/SessionRail.svelte:177-243` — render a workflow group section above `buildMyWorkGroups` sections: bordered group (`data-group-key="workflow:<id>"`), header = name + aggregate state, rows = existing `<WorktreeAgentRow>` for member sessions
- Test: `scripts/workflowGroupStore.test.mjs` (+ `test:workflow-groups` script)

**Interfaces:**
- Consumes: Tasks 2/3 via events + commands; `rail.owned` rows (`sessionRailStore.svelte.ts:1-72`).
- Produces: `workflowGroups.groups: {id, name, orchestratorOwnedId, memberOwnedIds, statusByOwnedId, events}[]`; `createGroup(name, memberIds, orchestratorId)`, `sendMessage(groupId, to, body)`.

- [ ] **Step 1: Failing test**: store reduces a `workflow-broker-event` stream (create → two status envelopes → one message) into one group with per-member status and an ordered event list. → FAIL.
- [ ] **Step 2: Implement** store/service + Tauri commands.
- [ ] **Step 3: Implement** rail section (group renders only when a group is open; rows are the same component — no new row variant).
- [ ] **Step 4: Tests pass**: `pnpm test:workflow-groups`; `npx tsc --noEmit` clean; `pnpm check:svelte` no NEW errors.
- [ ] **Step 5: Commit** `feat: workflow groups in the sessions rail with live member status`.

Diff budget: ~350 lines. New files only those named. UI lane: screenshot at 1710x990 headless required.

### Task 8: Hover card + timeline tab + compose

**Files:**
- Create: `src/lib/shell/workflowGroups/WorkflowGroupHoverCard.svelte` — port the existing floating-card mechanism verbatim (`WorktreeAgentRow.svelte:237-301,617-627,931-946`, card look from `SessionHoverCard.svelte:47-124`): members with status dots, last 3 events, `Open timeline` button
- Create: `src/lib/shell/workflowGroups/WorkflowTimelineView.svelte` — center tab `workflow` via the verified five-file pattern (`workbenchNavigation.ts:19-38`, `centerDock.ts:28-70`, `CenterCornerTabs.svelte:39-50`, `ShellFrame.svelte:25-42,95-135`, `routes/next/+page.svelte:242-251,329-359`): event timeline (from/arrow/to, kind badge, receipt chip, time) with filter chips per member + kind, compose row = kit `Select` (recipient) + `Input` + send → `sendMessage`
- Test: extend `scripts/workflowGroupStore.test.mjs` with timeline selectors (filtering, receipt text)

**Interfaces:** Consumes Task 7 store only. Mockup reference: `docs/superpowers/specs/2026-08-20-workflow-board-mockup.html` (right column + hover card; the full board/lanes view is slice 2 — do NOT build lanes/gates).

- [ ] **Step 1: Failing test**: timeline filter helper returns only the selected member's envelopes; receipt chip text for each receipt state. → FAIL.
- [ ] **Step 2: Implement** hover card, then the tab.
- [ ] **Step 3: Verify**: `pnpm test:workflow-groups`, tsc clean, screenshots (rail group + hover card + timeline) headless 1710x990, viewport verified by eval.
- [ ] **Step 4: Commit** `feat: workflow group hover card and message timeline tab`.

Diff budget: ~450 lines. UI lane (Opus). Kit controls only.

### Task 9: End-to-end proof + guide

**Files:**
- Modify: `main-architecture-explained.html` (change-log row + a short broker section)
- Test: `scripts/brokerEndToEnd.test.mjs` — drive store→socket→DB→event round trip with the real rust binary? NO — keep it in-process: node test spawns `server.mjs` against a real unix socket served by a tiny rust test helper? If that helper would be a new binary, SKIP the cross-process test (report why) and rely on Task 5's two half-tests. Judgment call recorded in the report.

- [ ] **Step 1**: Controller-run live pass in the dev build (single instance): create a group of two real sessions, orchestrator messages a worker, worker's `send_message` back, human message from the timeline — screenshot the timeline showing all three with receipts.
- [ ] **Step 2**: Guide row + section; commit `docs: broker architecture and change log`.

---

## Execution notes (controller)

- Routing: T1-T6 SOL medium (rust/protocol; T5's node test too); T7-T8 Opus (visible UI); nothing to Luna except optional flag-verification recon inside T6.
- Sequence: T1→T2→(T3,T5 parallel — different files)→T4→T6→T7→T8→T9. One rust-building lane at a time (shared `src-tauri` target dir); T5's node half may overlap T3.
- Worktrees per lane off `tsk-808-assembly-wave`; lanes never commit; controller reviews (untracked-file check!), integrates, commits, removes worktrees.
- Every dispatch carries: the task text, the recon file path, the global constraints block, stop-don't-fabricate, cleanup contract.

## Self-review (done at write time)

- Spec coverage: broker/bridge/status/delivery/rail/hover/timeline/receipts/no-API-cost all mapped; board+gates+engine correctly deferred to slice 2.
- Consistency: `Envelope`/`Receipt`/`MessageKind` names match across T2-T8; event name `workflow-broker-event` used consistently; group tables named identically in T1/T2.
- Known risk, stated: provider MCP flags (T6) are the researchable unknown — task requires pasting verified `--help` evidence before wiring; agy explicitly report-only.
