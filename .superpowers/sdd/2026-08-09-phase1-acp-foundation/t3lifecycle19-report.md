# T3 Code engine lifecycle recon

Source snapshot: `pingdotgg/t3code` `main` at commit `1e59b4c4004ce3c724d09ca0b140ed4523758d1e` (2026-08-12). The central correction is that T3 Code's interactive engines are **not per-turn processes**. Claude, Codex, Cursor, and Grok each start a child process when a provider session starts, reuse that child across turns, and normally keep it until explicit stop, failure, provider replacement, server shutdown, or a 30-minute inactivity reap. Only the separate title/branch/commit/PR text-generation helpers spawn one short-lived CLI process per operation.

Tags used below:

- **Verified** — established by the cited source.
- **Assumed** — plausible interpretation or external observation that the source does not prove.

## 1. Turn lifecycle: Claude and Codex

### Claude interactive session

- **Verified — the process boundary is one provider session, not one turn.** `startSession` creates an unbounded prompt queue, turns it into an async iterable, creates one Agent SDK `query`, stores it in the session context, and starts one fiber that consumes the query stream. Later `sendTurn` calls only enqueue another user message into that already-live prompt stream. (`apps/server/src/provider/Layers/ClaudeAdapter.ts:3705-3755`, `4166-4227`, `4272-4295`, `4303-4314`, `4393-4402`)

- **Verified — effective launch command.** T3 resolves the configured Claude executable and gives it to Agent SDK 0.3.170 as `pathToClaudeCodeExecutable`; on non-Windows platforms the configured path is returned unchanged. (`apps/server/src/provider/Drivers/ClaudeExecutable.ts:46-65`, `apps/server/package.json:24-26`, `pnpm-lock.yaml:1001-1003`) The SDK version pinned by T3 constructs this command (`assistant.mjs:155`, Agent SDK 0.3.170):

  ```text
  <resolved Claude binary>
    --output-format stream-json
    --verbose
    --input-format stream-json
    [--effort <selection>]
    [--model <selection>]
    --permission-prompt-tool stdio
    [--resume <provider-session-uuid>]
    [--mcp-config <json>]
    --setting-sources=user,project,local
    [--permission-mode acceptEdits|auto|bypassPermissions]
    [--allow-dangerously-skip-permissions]
    --include-partial-messages
    --add-dir <working-directory>
    --add-dir <T3-attachments-directory>
    [--session-id <new-uuid>]
    [--settings <json>]
    [configured extra arguments]
  ```

  The bracketed pieces are conditional. T3 maps `auto-accept-edits` to `acceptEdits`, `auto` to `auto`, and `full-access` to `bypassPermissions`; only the last also sets the dangerous-skip opt-in. It supplies `resume` for an existing provider session and `sessionId` for a new one. (`apps/server/src/provider/Layers/ClaudeAdapter.ts:4057-4086`, `4101-4139`) T3 enables partial messages and the three setting sources explicitly. (`apps/server/src/provider/Layers/ClaudeAdapter.ts:1171-1175`, `4121-4125`) There is no `-p` flag on this interactive path.

- **Verified — what starts and ends it.** Calling `createQuery` in `startSession` starts the SDK transport; explicit session stop shuts down the prompt queue, interrupts the stream fiber, and calls `query.close()`. A failed or ended stream completes any active turn and then uses the same stop path. (`apps/server/src/provider/Layers/ClaudeAdapter.ts:4166-4179`, `3562-3593`, `3595-3653`)

- **Verified — what survives between turns.** The child process, SDK query, prompt queue, permission callbacks, provider session UUID, model/permission state, in-flight task maps, and T3's in-memory turn list all survive. A second turn is an input frame to that process, not a resume invocation. (`apps/server/src/provider/Layers/ClaudeAdapter.ts:4200-4225`, `4303-4357`, `4399-4402`)

### Codex interactive session

- **Verified — Codex does not use `codex exec` for chat turns.** A provider session spawns `<configured binary> app-server` plus configured launch arguments and any T3 MCP `-c` arguments. It creates a JSON-RPC client on the child stdio, sends `initialize`/`initialized`, then `thread/start` or `thread/resume`. (`apps/server/src/provider/Layers/codexLaunchArgs.ts:13-16`, `42-47`; `apps/server/src/provider/Layers/CodexSessionRuntime.ts:862-903`, `1684-1712`; `apps/server/src/provider/Layers/CodexAdapter.ts:1666-1696`)

- **Verified — each later turn is an RPC on the same resident app-server.** `sendTurn` sends `turn/start` with the provider thread id and receives a turn id; it does not spawn another executable. (`apps/server/src/provider/Layers/CodexSessionRuntime.ts:1749-1801`)

- **Verified — permission mode is protocol data, not CLI turn flags.** Thread and turn requests carry the approval policy, reviewer, and sandbox. The mappings are: approval-required = untrusted/read-only/user; auto-accept-edits = on-request/workspace-write/user; auto = on-request/workspace-write/auto-review; full-access = never/danger-full-access/user. (`apps/server/src/provider/Layers/CodexSessionRuntime.ts:264-315`, `317-335`, `388-404`)

- **Verified — lifetime.** One runtime scope owns the child, JSON-RPC client, notification streams, and approvals. Closing the runtime cancels pending interactions and closes that scope, which releases the process. (`apps/server/src/provider/Layers/CodexSessionRuntime.ts:848-860`, `1725-1744`; `apps/server/src/provider/Layers/CodexAdapter.ts:1697-1705`, `1754-1761`)

### Separate short-lived text-generation helpers

- **Verified — the known Claude fragment is correct but applies only to auxiliary writing operations.** Commit message, PR content, branch name, and thread title generation spawn `claude -p --output-format json --json-schema <json> --model <selection> [--effort ...] [--settings ...] --dangerously-skip-permissions`, stream the prompt to stdin, collect stdout/stderr/exit code concurrently, and time out after 180 seconds. (`apps/server/src/textGeneration/ClaudeTextGeneration.ts:1-8`, `48-52`, `107-123`, `159-224`)

- **Verified — Codex has the equivalent auxiliary path.** It spawns `codex exec [allowed config launch args] --ephemeral --skip-git-repo-check -s read-only --model <selection> --config model_reasoning_effort=... [service tier config] --output-schema <temp-file> --output-last-message <temp-file> [images] -`, sends the prompt on stdin, waits for process output/exit, and applies a 180-second timeout. (`apps/server/src/textGeneration/CodexTextGeneration.ts:152-239`, `242-280`; `apps/server/src/provider/Layers/codexLaunchArgs.ts:18-40`)

## 2. Streaming and cancellation

- **Verified — Claude streaming.** The interactive command uses stream-JSON in both directions. T3 iterates the SDK query as an async stream; `content_block_delta` text/thinking frames become normalized `content.delta` events with a stream kind and text delta. (`apps/server/src/provider/Layers/ClaudeAdapter.ts:3533-3560`, `2358-2463`) The adapter exposes those normalized events through a queue consumed by the provider service/UI path. (`apps/server/src/provider/Layers/ClaudeAdapter.ts:1658-1677`)

- **Verified — Claude cancellation.** Stop first issues bounded `stopTask` calls for live child tasks, then invokes `query.interrupt()` for the parent turn. A full session stop additionally closes the query/process. (`apps/server/src/provider/Layers/ClaudeAdapter.ts:4413-4471`, `3595-3653`)

- **Verified — Codex streaming.** The app-server sends JSON-RPC notifications over its resident stdio. T3 maps `item/agentMessage/delta` into normalized `content.delta` events and publishes the adapter queue as a stream. (`apps/server/src/provider/Layers/CodexSessionRuntime.ts:1360-1368`, `1619-1622`; `apps/server/src/provider/Layers/CodexAdapter.ts:1170-1185`, `1718-1733`, `1986-1987`)

- **Verified — Codex cancellation.** T3 sends `turn/interrupt` to each known live child turn with bounded deadlines and then sends `turn/interrupt` for the parent provider thread/turn. The app-server remains alive and reusable afterward. (`apps/server/src/provider/Layers/CodexSessionRuntime.ts:1802-1833`)

- **Verified — ACP streaming/cancellation.** ACP adapters receive `session/update` notifications, normalize content/tool events into a queue, and send turns as `session/prompt`. Cancellation interrupts the local prompt fiber and asynchronously sends the protocol-native `session/cancel`; it does not tear down the ACP child. (`apps/server/src/provider/acp/AcpSessionRuntime.ts:370-405`, `719-770`, `844-914`)

## 3. Permissions and tool approvals

- **Verified — Claude approval is a live bidirectional control callback, not a second invocation.** Supplying `canUseTool` causes the SDK command to use `--permission-prompt-tool stdio`. T3 emits `request.opened`, stores a deferred decision, and awaits it inside the callback while the same process and turn remain alive. `respondToRequest` resolves that deferred; accept/accept-for-session returns allow data to the SDK, while decline/cancel returns deny. (`apps/server/src/provider/Layers/ClaudeAdapter.ts:3941-4055`; Agent SDK 0.3.170 `assistant.mjs:155`)

- **Verified — Codex approval is a server-initiated JSON-RPC request on the existing app-server connection.** Command and file-change approval handlers create a deferred, emit a request event, await the UI decision, then return the decision as the response to the original app-server request. (`apps/server/src/provider/Layers/CodexSessionRuntime.ts:1436-1547`) The public response call finds the pending request and completes its deferred. (`apps/server/src/provider/Layers/CodexSessionRuntime.ts:1856-1884`)

- **Verified — ACP approval follows the same protocol shape.** Cursor and Grok register `session/request_permission`, emit a normalized open event, await a deferred, and return an ACP selected/cancelled outcome on the original request. (`apps/server/src/provider/Layers/CursorAdapter.ts:666-735`, `1075-1091`; `apps/server/src/provider/Layers/GrokAdapter.ts:666-730`, `1358-1374`)

- **Verified — a valid pending approval prevents normal inactivity teardown.** The reaper skips any thread with an active turn. Since the turn stays active while its permission callback is awaiting a response, the 30-minute inactivity threshold does not kill that engine. (`apps/server/src/provider/Layers/ProviderSessionReaper.ts:57-72`)

- **Verified — the 45-second client demand lease is irrelevant to the pending process.** It is not consulted by the session reaper. Its only provider consumer gates provider-status refresh work. (`apps/server/src/background/BackgroundPolicy.ts:238-268`, `288-300`; `apps/server/src/provider/makeManagedServerProvider.ts:150-157`)

- **Verified — if the process/session actually ends first, a late approval cannot be applied.** Stop paths settle pending decisions as cancel/empty and remove or close the session; subsequent response calls report an unknown pending request or closed/missing session. (`apps/server/src/provider/Layers/ClaudeAdapter.ts:3601-3621`; `apps/server/src/provider/Layers/CodexSessionRuntime.ts:1725-1743`, `1856-1869`; `apps/server/src/provider/Layers/CursorAdapter.ts:459-469`, `1080-1090`)

## 4. Context continuity

- **Verified — `continuation.groupKey` is compatibility metadata, not the resume value.** For Claude it is exactly `claude:home:<resolved config path>`; instances sharing the same Claude config directory are considered continuation-compatible. That directory is applied through `CLAUDE_CONFIG_DIR`, deliberately leaving `HOME` unchanged so login credentials still work. (`apps/server/src/provider/Drivers/ClaudeHome.ts:9-40`; `apps/server/src/provider/Drivers/ClaudeDriver.ts:138-144`, `206-212`)

- **Verified — the actual Claude resume value is the provider session UUID.** T3 persists a cursor containing its canonical thread id, `resume: <provider session UUID>`, optional last assistant UUID, and turn count. On a new runtime it passes the provider UUID as the Agent SDK `resume` option; for a fresh session it generates a UUID and passes it as `sessionId`. (`apps/server/src/provider/Layers/ClaudeAdapter.ts:626-657`, `1736-1753`, `3736-3742`, `4119-4122`, `4189-4195`)

- **Verified — `groupKey` guards cross-instance switching.** T3 allows a thread to switch between provider instances only when the driver kind and continuation key match; otherwise it rejects the switch as incompatible resume state. (`apps/server/src/orchestration/Layers/ProviderCommandReactor.ts:591-612`)

- **Verified — conversation state is deliberately split.** T3's `state.sqlite` stores canonical orchestration events/projections plus provider runtime metadata and the opaque resume cursor. (`apps/server/src/config.ts:99-116`; `apps/server/src/persistence/Layers/OrchestrationEventStore.ts:99-158`, `184-209`; `apps/server/src/persistence/ProviderSessionRuntime.ts:27-52`, `150-186`) The provider's own config/session storage holds the provider-native transcript that `--resume`, `thread/resume`, or ACP `session/load` resolves. T3 does not reconstruct provider-native context by replaying its UI transcript.

- **Verified — Codex continuity is analogous.** The cursor is `{ threadId: <provider thread id> }`; a new app-server sends `thread/resume` for it. Codex continuation compatibility is keyed to the shared Codex home, even when a private shadow home is used for credentials. (`apps/server/src/provider/Layers/CodexSessionRuntime.ts:258-262`, `1689-1708`; `apps/server/src/provider/Drivers/CodexHomeLayout.ts:12-17`, `44-65`)

- **Verified — ACP continuity uses the ACP session id.** Cursor/Grok cursors contain `sessionId`; a new runtime calls `session/load` with that id, cwd, and MCP servers, otherwise it calls `session/new`. (`apps/server/src/provider/Layers/CursorAdapter.ts:515-540`, `753-765`; `apps/server/src/provider/acp/AcpSessionRuntime.ts:554-645`)

## 5. ACP providers and background policy

- **Verified — Cursor and Grok are resident per provider session.** Cursor spawns `cursor-agent [optional -e endpoint] acp`; Grok spawns `grok agent stdio`. `AcpSessionRuntime.make` spawns the child once and builds a persistent JSON-RPC client over its stdio. (`apps/server/src/provider/acp/CursorAcpSupport.ts:33-46`; `apps/server/src/provider/acp/GrokAcpSupport.ts:32-45`; `apps/server/src/provider/acp/AcpSessionRuntime.ts:269-368`)

- **Verified — turns do not respawn ACP.** The adapter creates a session scope and ACP runtime during `startSession`; later turns call `session/prompt` on the same runtime. Closing the session scope owns child teardown. (`apps/server/src/provider/Layers/CursorAdapter.ts:479-572`, `770-779`; `apps/server/src/provider/acp/AcpSessionRuntime.ts:719-760`; `apps/server/src/provider/Layers/CursorAdapter.ts:459-469`)

- **Verified — all interactive providers share one independent session reaper policy.** Default inactivity threshold is 30 minutes, swept every 5 minutes. It refuses to stop active turns or background-live threads, then calls `providerService.stopSession`. (`apps/server/src/provider/Layers/ProviderSessionReaper.ts:17-35`, `37-109`, `120-142`)

- **Verified — `BackgroundPolicy` demand leases govern background work, not engines.** Leases default to 45 seconds, clamp at 120 seconds, track scopes such as provider-status, VCS, git refs, diagnostics, server config, and thread, and are pruned every 15 seconds. (`apps/server/src/background/BackgroundPolicy.ts:58-74`, `238-268`, `314-334`) In current server production consumers, provider demand gates health/status snapshot refresh and VCS demand gates VCS polling; there is no call from the provider session reaper or ACP runtime. (`apps/server/src/provider/makeManagedServerProvider.ts:150-168`; source-wide production-reference check for `shouldRunScopeWork`)

## 6. Latency and process reuse

- **Verified — there is no warm pool or per-turn pre-warm path for these four interactive providers.** Source-wide searches for pre-warm/warm-pool logic found none. The latency strategy is simply session reuse: pay spawn + initialize/load once at `startSession`, then keep the child resident and issue later turns over the existing stream/RPC connection.

- **Verified — reuse lasts much longer than seconds.** Idle provider sessions remain eligible for reuse for 30 minutes, with a 5-minute sweep granularity, unless explicitly stopped or failed. (`apps/server/src/provider/Layers/ProviderSessionReaper.ts:17-18`, `31-35`, `57-60`, `134-141`)

- **Verified — ACP resume has one visible latency guard, but it is not pre-warming.** During `session/load`, replay can be considered ready after a 2-second replay-idle gap; startup shares one in-flight start attempt and permits retry after startup failure. (`apps/server/src/provider/acp/AcpSessionRuntime.ts:51`, `178-182`, `559-632`, `659-689`)

- **Assumed — no source-backed engine startup duration is published.** The code has probe/load timeouts, but they are not measurements of normal spawn latency. No defensible milliseconds-to-first-token number was found.

## 7. Failure and recovery

- **Verified — failure is surfaced before recovery.** Claude stream failure emits a runtime error, marks the active turn failed, and stops the session. Codex watches child exit, marks the session closed for code 0 or error otherwise, and emits `session/exited`. (`apps/server/src/provider/Layers/ClaudeAdapter.ts:3562-3593`; `apps/server/src/provider/Layers/CodexSessionRuntime.ts:1657-1682`)

- **Verified — restart is lazy and cursor-based.** Provider runtime metadata and its resume cursor are upserted in SQLite. When a later routed operation finds no active in-memory session, `recoverSessionForThread` starts a new adapter session with persisted cwd, model selection, runtime mode, and resume cursor. (`apps/server/src/persistence/ProviderSessionRuntime.ts:35-52`, `150-186`; `apps/server/src/provider/Layers/ProviderService.ts:358-432`, `443-490`)

- **Verified — an ordinary later `startSession` also falls back to the persisted cursor.** If the caller supplies none and the provider instance still matches, ProviderService injects the stored cursor and cwd. (`apps/server/src/provider/Layers/ProviderService.ts:568-607`)

- **Verified — recovery resumes the last provider-persisted boundary; it does not replay a killed turn.** T3 persists normalized partial events for UI/history and stores the last known provider cursor, but there is no automatic resubmission of the interrupted user prompt. The user can continue the resumed provider thread or retry explicitly. (`apps/server/src/provider/Layers/ProviderService.ts:729-748`; `apps/server/src/persistence/Layers/OrchestrationEventStore.ts:184-209`)

- **Assumed — provider-specific crash atomicity remains provider-owned.** The source cannot promise whether a provider's local transcript included the final few frames before an abrupt kill. T3's safe guarantee is only that it retains its own already-appended events and the last cursor it received.

## 8. Other load-bearing memory behavior

- **Verified — one T3 server process multiplexes many logical sessions, but not one shared engine process.** Each configured provider instance owns one adapter, and each adapter owns a map of thread id to session context. Claude and Codex each create a distinct child/runtime per map entry. (`apps/server/src/provider/Drivers/ClaudeDriver.ts:146-152`, `206-219`; `apps/server/src/provider/Layers/ClaudeAdapter.ts:1658-1659`; `apps/server/src/provider/Layers/CodexAdapter.ts:1642-1644`, `1645-1763`)

- **Verified — engines are lazy.** Driver creation constructs adapter/text-generation closures and status machinery; a session child appears only when `startSession` is called. (`apps/server/src/provider/Drivers/ClaudeDriver.ts:146-163`, `206-219`; `apps/server/src/provider/Layers/ClaudeAdapter.ts:3705-3715`, `4166-4179`)

- **Verified — inactive sessions are recoverable without keeping UI history in RAM.** Canonical events, projections, provider binding, cwd/model metadata, and cursors live in SQLite; provider-native continuation lives in the provider home. The resident process is an acceleration layer, not the only copy of thread identity. (`apps/server/src/persistence/ProviderSessionRuntime.ts:27-52`, `150-186`; `apps/server/src/persistence/Layers/OrchestrationEventStore.ts:99-158`)

- **Verified — background work blocks reap.** A settled foreground turn can still own subagents or monitors; the reaper checks `backgroundLiveness` and skips teardown to avoid silently killing them. (`apps/server/src/provider/Layers/ProviderSessionReaper.ts:62-85`)

- **Assumed — the reported 100–150 MB startup spike is not explained or quantified by these sources.** It may be the persistent T3 server/runtime plus transient provider initialization, but no comment, metric, or benchmark ties that number to a component. It should be treated as an external measurement, not a T3 design constant.

## Implications for our ACP manager

The closest T3 analogue to our shipped design is not “spawn per turn.” It is exactly a resident ACP adapter per active session, backed by a durable provider session id, with scope-owned teardown and lazy resume. Our 30-second post-turn grace is much more aggressive than T3's 30-minute threshold, but the mechanism can remain the same.

**Adopt verbatim:**

1. Give every session runtime one ownership scope. That scope must own the child, stdio/RPC client, notification consumer, pending approvals, prompt fiber, and event queue. Closing it must be idempotent and must settle every deferred interaction before killing the child.
2. Keep the provider-native resume id separate from our canonical session id and persist it after every provider response that can advance it. UI transcript persistence and provider context continuity are two different stores.
3. Use protocol-native cancellation first (`session/cancel`), interrupt the local awaiting fiber, and leave the child reusable. Reserve scope close/process kill for suspend, crash, explicit stop, or shutdown.
4. Normalize provider events immediately into our event stream. Do not make UI consumers understand raw ACP notification variants.
5. Guard teardown with both `activeTurn` and `backgroundLiveness`. A turn that appears settled can still own child work.

**Adapt to our 30-second grace:**

1. Start the grace clock only after the last prompt has completed and there are no live tools, child agents, background tasks, permission requests, or user-input requests. Any of those conditions cancels the clock.
2. Treat a pending approval as an explicit runtime lease with no ordinary idle expiry. Suspending a process while its request callback is blocked destroys the request/response channel; it cannot be repaired by merely resuming the provider session id. If product policy needs an approval timeout, expire the approval explicitly, return cancel to the live process, finish the turn, and only then begin the 30-second grace.
3. On grace expiry, persist the latest ACP session id and state marker, close the ownership scope, and mark the runtime `suspended`, not `stopped`. The next turn should lazily spawn the ACP binary, initialize/authenticate, call `session/load`, drain replay, then issue the prompt.
4. If load fails, keep our canonical transcript and surface a resumability error. Do not silently start a fresh provider session, because that makes the UI look continuous while provider context was lost.
5. A late approval after crash/suspend should receive an explicit stale-request result. Never attach it to a new process or a newly created request that happens to look similar.

**Reject:**

1. Reject the claim that T3 proves per-turn spawning is cheap. Current T3 source proves the opposite: it amortizes startup with a resident per-session child.
2. Reject copying the 45-second `BackgroundPolicy` lease as an engine-lifetime controller. It gates health/VCS background work and does not understand protocol-critical pending requests.
3. Reject warm pools or speculative pre-start. T3 has none, and they would create exactly the idle resident cost our manager is meant to remove.
4. Reject using only “no active foreground turn” as the suspend condition. Background liveness and pending request channels are equally load-bearing.

The concrete target for our manager is therefore: **lazy per-session spawn, protocol-native suspend boundary, durable ACP session id, 30-second quiescent grace, and zero child processes only after quiescence is proven.** That gives us a stronger idle-memory result than T3 while preserving the lifecycle properties that make its recovery reliable.

## Recon cleanup

- **Verified:** the shallow clone and the separately unpacked Agent SDK source were held only under `/tmp/t3lifecycle19.R821u9/` for this recon, then deleted. A post-delete existence check confirmed that path is gone.
