# Backend design review: Rust and Tauri backend

## Executive summary

1. The backend has two frontend-invoked conversation commands that do not exist or are not registered today; this is the clearest immediate user-visible defect.
2. Structured runtime events and terminal transcript events use the same Tauri event name but allocate generations and sequences independently, so valid events can be dropped, replayed twice, or treated as stale.
3. Activation is serialized with itself, but close and suspend do not take the same lock; an activation can therefore finish after close and put a closed session back into `Ready`.
4. Close and metadata updates do not require a generation, so a delayed frontend request can mutate or close a newer session generation.
5. Session rows, event rows, and in-memory state are updated in separate steps; a failed write or crash can leave the three representations disagreeing.
6. Handoff ownership is changed only in memory, recovery rebuilds a contradictory writer lease, and handoff detachment bypasses adapter-pool membership rules.
7. The Codex bridge is a sensible place for provider translation and preserves output order, but model changes stay inside the bridge and do not update Rust or the frontend.
8. The adapter trait does not contain most runtime behavior, while `manager.rs` directly speaks ACP; the current abstraction obscures the real boundary without isolating provider differences.
9. String errors and selective logging turn command names and prose into an accidental protocol; one small serializable error shape is enough to fix the highest-value paths.
10. The first fixes should be contract parity, one durable event stream, one lifecycle lock, and one transactional state-write path; splitting large files or adding a framework can wait.

## Review basis

This was a read-only review using source search and file reads. I did not run Cargo, touch Git, or modify source files. The Tauri backend currently declares 163 `#[tauri::command]` functions under `src-tauri/src`; 87 are still in `main.rs`, 25 are in `agent_conversation/mod.rs`, and the remainder are spread across domain modules. The problem is not the raw count by itself. The problem is that frontend command names, Rust functions, registration, error shapes, and logging are maintained as separate lists with only partial checks.

## Findings, ranked by expected user-visible-bug value

### 1. Two live frontend commands are absent from the backend

**Expected failure:** attachments silently disappear after reload, and changing a provider config option always rejects at runtime.

The attachment restore path invokes `read_agent_conversation_attachments` and then catches every rejection and returns an empty list (`tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:199-210`). The backend attachment module exposes save and delete only (`tauri-svelte-preview/src-tauri/src/agent_conversation/attachments.rs:26-76`), the conversation command module registers wrappers for save and delete only (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:301-316`), and the central handler list contains those two names but no read command (`tauri-svelte-preview/src-tauri/src/main.rs:5794-5819`). This is a confirmed current defect, not only a missing-risk pattern.

The config-option UI invokes `set_agent_conversation_config_option` (`tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:267-274`). There is no Tauri wrapper or registration for that name, although the manager already has `set_config` (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1092-1137`). This is stranded backend behavior behind a missing command boundary.

The existing registration test cannot catch either defect because it checks a hand-written list of six conversation commands and three rail commands (`tauri-svelte-preview/src-tauri/src/main.rs:8708-8755`). The production registry is another hand-written list (`tauri-svelte-preview/src-tauri/src/main.rs:5678-5819`). The most fragile command contracts are also inconsistent about accepting a request object versus raw scalar arguments: compare ensure and send (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:80-120`), draft commands (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:123-146`), transcript reads (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:275-282`), and attachment save/delete (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:301-316`).

**Smallest fix:** implement and register the two missing commands, and replace the hand-picked conversation registration test with one check over every literal `invoke(...)` name in `conversationService.ts`. Assert that each name has both a Tauri command function and an entry in `generate_handler!`. Keep the central Tauri macro and the existing domain modules; a binding generator or command framework is not needed to prevent this defect. Standardize touched mutating commands on one request object containing `ownedId` and, where a live session is involved, `generation`.

### 2. The frontend receives two incompatible event streams under one event name

**Expected failure:** a message or tool event disappears, an old event renders again, the UI enters unnecessary resync, or a structured event is rejected as stale after terminal projection starts.

The structured manager assigns `session.next_sequence`, appends the event to SQLite, persists the session, and then dispatches it (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:2103-2179`, `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:2182-2192`). `main.rs` emits those events as `agent-conversation-event` (`tauri-svelte-preview/src-tauri/src/main.rs:5663-5666`).

Terminal projection independently emits the same Tauri event name (`tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:13-18`, `tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:357-368`). It deliberately creates a generation greater than the manager generation and every previous projection generation (`tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:198-221`), then starts its own sequence at 1 (`tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:222-267`). Those events are retained only in the projection watcher's deque and are not appended to `SessionStore` (`tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:224-240`, `tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:357-368`).

The frontend listens once to a union of the two shapes (`tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:401-422`) and sends each shape down a different reducer path (`tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:179-199`). Both paths nevertheless share `generation`, `lastSequence`, and desynchronization state. Both reject a sequence less than or equal to the current value and flag gaps (`tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:222-233`, `tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:439-456`). A terminal projection event with generation `G+1` therefore makes later structured events at generation `G` stale. If both streams use the same generation, their independent sequence counters collide instead.

Initial terminal events are also emitted during registration and returned in the registration response (`tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:222-240`); the frontend applies the returned copy again (`tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:299-308`). The reducers often suppress that exact duplicate, but suppression depends on which producer interleaved first. This proves duplicate delivery and creates the collision window; it does not provide exactly-once delivery.

The structured path alone is substantially better. Prompt completion is put on the manager's ordered channel (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:2232-2239`), and the inbound pump gives already-queued provider updates priority (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:2286-2312`). The Codex bridge serializes every ACP write through one promise chain (`tauri-svelte-preview/tools/codex-acp-bridge/bridge.mjs:28-56`), emits updates through that writer (`tauri-svelte-preview/tools/codex-acp-bridge/bridge.mjs:332-340`), completes pending tool output before settling the turn (`tauri-svelte-preview/tools/codex-acp-bridge/bridge.mjs:930-945`), and writes the prompt response through the same writer (`tauri-svelte-preview/tools/codex-acp-bridge/bridge.mjs:139-152`, `tauri-svelte-preview/tools/codex-acp-bridge/bridge.mjs:1234-1268`). For the current Codex bridge, provider updates are therefore written before the prompt response. The ordered structured path is not the source of the reported double rendering; the second producer is.

Structured reconnect recovery is also reasonable: a gap marks the session desynchronized, the service reads a snapshot, and it retries when a live event passed the snapshot (`tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:379-392`, `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:401-422`). That recovery cannot restore terminal projection events because those events never enter the durable journal.

**Smallest fix:** make terminal projection submit projected payloads to the manager's existing journal-and-emit path. The manager must assign the generation and next sequence. Stop calling `app.emit` from terminal projection, and stop returning already-emitted initial events. Once that path is live, delete the second frontend reducer path and accept one event envelope only. Do not add deduplication rules around the two producers; that preserves the underlying ambiguity.

### 3. Session lifecycle operations are not serialized with each other

**Expected failure:** a user closes a conversation but its runtime appears connected again, a detached adapter is installed into a closed session, or a delayed close request closes a newly ensured generation.

`activate` takes a per-owned-session activation lock (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:487-506`), releases the sessions mutex while it starts or resumes the adapter (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:527-719`), then commits the runtime and `Ready` state (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:720-751`). `close` does not take that lock. It finds a session by `owned_id` only, marks it `Closed`, removes its runtime, records the close, and then awaits adapter shutdown (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1441-1528`).

This creates a concrete resurrection race. If close runs while activation is doing adapter I/O, close can mark the same generation closed while seeing no runtime to shut down. Activation then reacquires the sessions mutex and calls `current_session_mut`; that helper validates only `owned_id` and generation, not terminal state (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:2075-2100`). Activation is then allowed to overwrite the closed session with `Connected`, `Ready`, and a live runtime (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:720-751`).

Suspend has the same structure and also omits the activation lock: it removes runtime fields, awaits detachment, then writes `Suspended` (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1369-1438`). Close can run inside that gap. Its view of the runtime is then incomplete, while the suspend completion still has permission to update the same generation.

The state machine is spread across four independent representations: execution owner and runtime state (`tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs:7-29`), writer lease and transition (`tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs:309-343`), and connection state (`tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs:354-363`). `ManagedAgentSession` stores all of them separately (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:101-136`). There is no single transition function that rejects impossible combinations.

Generation protection is also incomplete at the command boundary. Close accepts only `owned_id` (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:235-241`), and metadata update has no generation in its request (`tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs:619-626`). A response or cleanup action issued by an old frontend view can therefore affect the current generation.

**Smallest fix:** use the existing per-session lifecycle lock for ensure/activate, suspend, close, and both handoff directions. Require `generation` on close and every live-session mutation. After any awaited adapter operation, recheck both generation and allowed source state before committing. Add one small transition function that updates runtime state, connection state, owner, and writer lease together and rejects `Closed -> Ready`; do not start by replacing the manager with a new framework.

### 4. The event row and session row do not commit as one state change

**Expected failure:** after a write error or process exit, the event history says one thing while the session list, next sequence, capabilities, connection, or live tool state says another.

`record_payload_for_session` increments the in-memory sequence and mutates connection, capabilities, last activity, and live-tool state before it attempts a database append (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:2103-2154`). It then appends the event, enforces the cap, updates the in-memory recent deque, and separately upserts the session row (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:2155-2179`). If append fails, memory has already advanced. If the later session upsert fails or the process stops between writes, the event is durable but its related session state is not.

`SessionStore::append_event` uses a transaction, but that transaction contains only the event insert and `last_activity_at` update (`core/src/session_store.rs:287-321`). It does not persist the state and `extra` values changed by that event. The schema formally links events to sessions by foreign key and sequence key (`core/src/session_store.rs:136-160`), but no transaction enforces the stronger invariant used by the manager: “the session row describes the state after its highest committed event.”

The session row itself contains two copies of several values. Model, effort, worktree, branch, title, project, and state have columns, while generation, owner, config, capabilities, and rail metadata live in `extra_json` (`core/src/session_store.rs:50-77`, `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1880-1930`). Config and rail metadata are represented in both places. Recovery has ad hoc reconciliation rules for some fields (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1933-1969`), which proves that drift is expected rather than prevented.

`recent_events` is yet another event representation, but snapshots read the durable store rather than the deque (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1256-1269`, `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1329-1340`). Maintaining the deque in `record_payload_for_session` (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:2174-2177`) adds state without being the read authority.

**Smallest fix:** add one `SessionStore` operation that accepts the next session row and optional event row, writes both in one immediate transaction, and applies the event cap in that transaction. Build a candidate next state first, commit it, then replace the in-memory fields and emit. Route lifecycle and provider-event transitions through that one operation. Keep ordinary rail-only metadata writes on the same session upsert, but stop duplicating config and rail fields between columns and `extra` when the schema is next changed. Remove `recent_events` after confirming there is no remaining reader.

### 5. Handoff can corrupt ownership and pooled runtime state

**Expected failure:** after restart both the terminal and structured path believe they can write, or handing off one pooled session disconnects other sessions using the same adapter process.

Handoff prepare, commit, and rollback change `owner`, `writer_lease.owner`, and `writer_lease_transition` in memory (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1531-1601`, `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1689-1787`). None of those paths persists the updated session. Recovery restores `owner` from the stored JSON but unconditionally reconstructs the writer lease as `Structured` and discards any transition (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1998-2020`). A persisted terminal owner can therefore restart beside a structured writer lease.

The detach step is also outside the pooling contract. `detach_structured_runtime` takes only `session.runtime` and calls `detach_session` directly (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1603-1633`). It does not clear `transport`, `ordered_events`, or `pool_key`, and it does not remove the member from `AdapterPoolEntry`. In contrast, suspend and close call `release_pool_scope` (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1396-1406`, `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1517-1526`). Because a shared pool contains one runtime for multiple owned sessions (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:535-575`), direct detachment can stop the shared process while the pool still lists other members.

**Smallest fix:** put handoff under the lifecycle lock from finding 3, persist prepare/commit/rollback through the state-write chokepoint from finding 4, and make detachment use `release_pool_scope` with the session's pool key and native session id. Clear runtime, transport, ordered sender, and pool key together. Recovery should derive writer lease owner from the persisted execution owner or, preferably, persist one owner value and derive both runtime views from it.

### 6. Provider translation is ordered, but the Rust adapter seam does not contain provider behavior

**Expected failure:** the UI shows a model or capability that the active provider did not accept, or a config request is sent to a stale native session after resume falls back to a fresh session.

The bridge is the right place to translate Codex app-server messages into ACP. Its ordered writer is a strong design choice, and the turn-completion ordering described in finding 2 should be kept. However, `model/rerouted` and `thread/settings/updated` only update the bridge's local session map (`tauri-svelte-preview/tools/codex-acp-bridge/bridge.mjs:903-927`). They do not emit a standard config update to Rust. The next prompt can therefore use the new bridge model while `manager.rs`, SQLite, and the frontend continue to show the previous model.

Rust maintains the same information in more places. `AcpClient` keeps per-session config, the bridge keeps its own session map, and `ManagedAgentSession` keeps `config` plus `connection.config`. The rail metadata command directly writes requested model and effort into both manager config copies without asking the provider (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1343-1361`). Its request does not carry generation (`tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs:619-626`). A rail refresh can therefore make persisted runtime config claim a value the provider never accepted.

There is a separate stale-id bug in the real config command. It captures `native_session_id` before `runtime_or_activate` (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1153-1187`). Activation is allowed to fall back from an unstarted stored id to a fresh native session (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:528-534`, `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:606-653`). The subsequent config request still uses the old id (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1188-1202`).

The nominal adapter seam does not solve these problems. `AgentRuntimeAdapter` covers initialize, create/load/resume, detach, and close only (`tauri-svelte-preview/src-tauri/src/agent_conversation/providers/mod.rs:101-123`). `manager.rs` imports ACP transport types directly and handles ACP request and update shapes itself (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:26-33`, `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:2278-2533`). `StructuredRuntimeHandle` has one `Acp` variant and forwards a large second API not present on the trait (`tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs:214-322`). Provider details also leak into manager startup and config behavior through explicit Claude branches (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:690-707`, `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1168-1174`).

Capability reporting adds another false source of truth. Session capabilities are read from the ACP initialize result, but permissions, structured input, tool terminals, plans, tasks, subagents, and resource links are set to `true` unconditionally (`tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:500-575`). Persisted capability snapshots are then used to decide whether a stored session is recoverable (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1970-1977`) and are served unchanged while suspended (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:344-356`, `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:2005-2008`). An adapter upgrade can therefore leave both UI affordances and resume policy based on old or fabricated facts.

**Smallest fix:** keep ACP concrete end to end for now instead of expanding a trait for hypothetical transports. Treat the bridge or provider adapter as the only owner of provider-specific translation. Have it emit one ordinary config-update message when Codex reroutes or changes settings, then persist that accepted state through the manager chokepoint. Make `update_session_meta` rail-only and generation-checked. In `set_conversation_config`, activate first and read the current native id afterward. Default provider capabilities to false unless initialize advertises them; app-owned rendering abilities should not masquerade as provider capabilities. Stored capabilities may be shown as last-known data, but they should not decide whether recovery is attempted.

### 7. Error strings are an accidental API and most conversation failures are not logged

**Expected failure:** the frontend takes the wrong fallback based on wording, the user sees an unactionable error, and `backend.log` lacks enough identity to reconstruct the failing session.

The provider layer already has a useful error seed: `AgentRuntimeError` carries a stable code and message (`tauri-svelte-preview/src-tauri/src/agent_conversation/providers/mod.rs:78-99`). That information is repeatedly flattened with `.to_string()` in manager paths, including activation and config (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:585-588`, `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:1188-1193`). The Tauri conversation commands then expose `Result<T, String>` throughout (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:49-316`).

The frontend already treats prose as a protocol: permission response catches an error, searches for `not found` or `unknown command`, and selects a legacy approval command if the text matches (`tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:633-648`). That is both a provider seam leak and an old/new side-by-side path.

`backend.log` is installed before the runtime starts (`tauri-svelte-preview/src-tauri/src/main.rs:5634-5652`), but the conversation command logging helper is called only by ensure and send (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:80-120`). The helper itself is adequate as a starting point but logs only command, owned id, and the flattened message (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:328-336`). Permission, input, cancel, config, close, projection, transcript, attachment, and handoff failures bypass it.

**Minimal convention worth adopting:** one serializable `CommandError { code, message, recoverable }`. Keep the code set small: `invalid_request`, `not_found`, `stale_generation`, `busy`, `not_supported`, `adapter_failure`, and `store_failure` cover the existing boundary. Preserve `AgentRuntimeError.code` in `From<AgentRuntimeError>`. Log once at the Tauri boundary with command, owned id, generation when present, provider when known, code, and message; do not log prompt or attachment content. Frontend fallbacks must branch on `code`, never regex the message.

Convert these ten call sites first because they cover the active user and lifecycle paths:

1. `ensure_agent_conversation` (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:80-91`).
2. `send_agent_conversation_message` (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:93-121`).
3. `respond_agent_conversation_permission` (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:164-178`).
4. `respond_agent_conversation_input` (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:180-199`).
5. `stop_agent_conversation_turn` (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:201-209`).
6. `set_agent_conversation_config` (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:211-217`).
7. `close_agent_conversation` (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:235-241`).
8. `start_agent_conversation_terminal_projection` (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:284-291`).
9. `save_agent_conversation_attachment` (`tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:301-309`).
10. `handoff_agent_conversation` (`tauri-svelte-preview/src-tauri/src/agent_conversation/handoff.rs:138-190`).

The recent resume-card fix should be kept: the error payload and returned error now include the underlying adapter error (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:655-681`). The issue is that this preservation is local rather than the default boundary behavior.

### 8. Global adapter-pool locks and unbounded queues can turn one noisy session into an app-wide stall

**Expected failure:** opening, closing, or resuming one slow provider session blocks unrelated sessions; a provider producing output faster than SQLite and the frontend consume it grows memory without a bound.

`release_pool_scope` holds the global `adapter_pools` mutex while it awaits runtime lock acquisition and provider close/detach requests (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:216-258`). Activation holds the same global pool mutex while it resumes or creates a native session and, on a new pool, while it initializes and starts the adapter (`tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:535-650`). These are process and protocol operations with no short upper bound. The result is app-wide serialization, not merely per-provider or per-session serialization.

The transport inbound queue, prompt-update routes, and manager ordered-event queue are all unbounded (`tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:67-78`, `tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:166-167`, `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:720-720`). The bridge itself already propagates stdout backpressure through `OrderedWriter` (`tauri-svelte-preview/tools/codex-acp-bridge/bridge.mjs:28-56`), but the Rust reader removes that protection by accepting unlimited messages into memory.

**Smallest fix:** under the pool mutex, reserve or remove membership and clone the runtime handle; drop the pool mutex before any protocol await; reacquire it only to finalize or roll back membership. Keep the per-runtime mutex to serialize operations on one adapter. Replace the inbound and ordered queues with bounded channels sized for a short burst so backpressure reaches the bridge reader. Do this after the correctness fixes above, and measure idle CPU plus memory under a deliberately noisy transcript before and after.

### 9. Source scanning has two backend implementations that can continue to diverge

**Expected failure:** the same workspace returns different files, limits, skip diagnostics, or previews depending on whether a caller reaches the core dispatcher or the Tauri command.

Core defines source result types and its own recursive scan and preview (`core/src/source.rs:14-68`, `core/src/source.rs:105-178`, `core/src/source.rs:258-296`), and the core dispatcher exposes those as `source.list` and `source.preview` (`core/src/dispatcher.rs:9-83`). `main.rs` defines another set of result types and another recursive collector (`tauri-svelte-preview/src-tauri/src/main.rs:71-157`, `tauri-svelte-preview/src-tauri/src/main.rs:611-685`, `tauri-svelte-preview/src-tauri/src/main.rs:1794-2024`). The Tauri version has cancellation and progress behavior that core does not, so these are already behaviorally different rather than simple duplicate wrappers.

The other scanners are better bounded: the dispatcher calls focused scanner functions (`core/src/dispatcher.rs:9-23`, `core/src/dispatcher.rs:86-134`), resource actions have a typed validation error (`core/src/scanners/resources.rs:89-107`, `core/src/scanners/resources.rs:358-398`), and session transcript parsing is separated from record merging (`core/src/scanners/sessions.rs:282-336`, `core/src/scanners/sessions.rs:727-745`). The large session scanner deserves ordinary targeted maintenance, but this review found no equivalent cross-layer state authority problem in it.

**Smallest fix:** choose one source scan implementation as the owner in a dedicated change. The least disruptive direction is to keep Tauri's command orchestration and progress reporting but call one pure collector owned by core. Delete the other collector in the same change. Do not mix this cleanup into the conversation fixes, and do not generalize the other scanners merely for symmetry.

## Do first / do later / leave alone

### Do first

1. Add and register `read_agent_conversation_attachments` and `set_agent_conversation_config_option`; make the conversation command parity check exhaustive for literal frontend invokes.
2. Route terminal projection events through the manager's durable sequence allocator, stop direct terminal `app.emit`, and remove the second frontend event interpretation path.
3. Put activate, suspend, close, and handoff under the same per-session lifecycle lock; require generation on close and metadata mutation; reject state commits after close.
4. Add the transactional event-plus-session store operation and make it the only path that commits event-driven session state.
5. Persist handoff ownership and detach pooled sessions through pool membership logic.

### Do later

1. Adopt `CommandError` at the ten listed Tauri boundaries, log every boundary failure with session identity, and delete message-regex fallbacks.
2. Make accepted config changes flow from the bridge/provider back to Rust, make rail metadata rail-only, and stop advertising unconfirmed capabilities.
3. Shorten adapter-pool lock holds, bound ACP queues, and verify the result under output load.
4. Consolidate the two source scanners in a separate, deletion-complete change.
5. Remove `recent_events` if a final read search confirms that snapshots and replay use SQLite exclusively.

### Leave alone

1. Keep the Codex bridge's single ordered writer and its update-before-prompt-response behavior.
2. Keep the structured manager's durable sequence, gap detection, and snapshot retry design once terminal projection joins that same stream.
3. Keep the recent resume failure message that includes the underlying adapter error.
4. Do not split `manager.rs` or `main.rs` merely to reduce line count; move code only when one of the fixes above establishes a clear owner and deletes the old path.
5. Do not add a generated command framework, a new provider hierarchy, compatibility shims, retries, or feature flags. None is required to prevent the evidenced failures.
