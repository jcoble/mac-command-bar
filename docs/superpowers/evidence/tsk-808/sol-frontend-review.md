# Frontend architecture review: `/next` shell to Tauri seam

## Executive summary

1. The most likely next user-visible failure is stale native state replacing newer event-driven state: ordinary snapshot loads accept an older same-generation snapshot after live events have already advanced the store.
2. Session activation and surface selection are not atomic; the rail jump starts an asynchronous session switch and immediately persists the requested tab under the previously active session.
3. Conversation capabilities are retained across connection generations and written without a generation check, so a suspended-session answer can govern a revived session.
4. The frontend has 146 direct `invoke` source locations across 13 non-test files, but no generated or otherwise shared TypeScript/Rust command contract and no common error policy.
5. The scoped tree contains 89 bare `catch {}` blocks and 16 explicit `.catch(() => undefined)` drops; many are harmless parsing or cleanup fallbacks, but several discard failures from user actions or required subscriptions.
6. Conversation event reduction is sensibly centralized, but terminal output and language-server status each have two subscribers with different lifecycle and ordering rules.
7. Page teardown does not release several module-level handler registries, and conversation event setup can complete after teardown and leave a live listener behind.
8. Load gates, handler-presence gates, capability gates, and connection gates all encode “not ready” differently; several user actions return without an error, a retry, or a durable deferred action.
9. `+page.svelte` is 1,840 lines and owns navigation, persistence, lifecycle, handoff, startup, and registry wiring despite describing itself as a thin terminal orchestrator; extraction is warranted, not a rewrite.
10. The smallest high-value sequence is monotonic snapshot/capability application, atomic session activation, explicit subscription ownership, then a typed logged invoke boundary migrated one domain at a time.

## Review basis and counts

This was a source-only review. I used reads and `rg`; I did not run git, builds, tests, or the app.

- The four central files total 5,495 lines: `+page.svelte` 1,840, `conversationService.ts` 661, `conversationStore.svelte.ts` 1,142, and `tauriSource.ts` 1,852.
- The direct-invoke count is 146 source locations, found with `rg -n --glob '!**/*.test.*' --glob '!**/*.spec.*' '\binvoke\s*(?:<|\()' tauri-svelte-preview/src`.
- Distribution: 107 in `lib/tauriSource.ts`; 18 in `conversationService.ts`; 4 in `resourceSampleBackend.ts`; 3 in `playwrightBackend.ts`; 2 each in `conversationConfig.ts`, `sourceIntelligence.ts`, `gitBackendExtra.ts`, `processBackend.ts`, and `worktreesBackend.ts`; and 1 each in `backendCapabilities.ts`, `newSessionBackend.ts`, `problemsBackend.ts`, and `usageBackend.ts`.
- “Silent catch” means a syntactic `catch {}` or `.catch(() => undefined)`, not necessarily a defect. The scoped non-test tree has 89 and 16 respectively. Storage parsing, URL parsing, best-effort cleanup, and optional compatibility probes account for many of them. The findings below distinguish those from failures that affect a requested action or required state.
- The scoped tree has 47 `$effect` declarations. Most are presentation or DOM lifecycle work; the review calls out only effects that cross the backend boundary, start subscriptions, or mutate architecture state.

## Ranked findings

### 1. Critical: snapshot and connection application are not monotonic within a generation

`loadConversationForRead` awaits a snapshot and applies it unconditionally. Meanwhile the one global conversation listener remains active and can apply newer events. Snapshot application rejects only a lower generation, not a lower `lastSequence` in the same generation, and then replaces the entire reactive session object. A snapshot read at sequence 40 can therefore land after event 41 and erase it until another event or resync repairs the view.

Receipts:

- `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:395-399` reads and applies without comparing the sequence that was current when the read returns.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:401-422` keeps dispatching live events while that invoke is pending.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:527-533` rejects only an older generation.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:549-603` rebuilds and replaces the stored session.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:379-390` already contains the needed comparison idea in the resync path, but ordinary reads bypass it.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:451-459` applies an ensure response after its invoke and then resyncs; `setConversationConnection` assigns generation and connection fields without checking whether an event already advanced them (`conversationStore.svelte.ts:1080-1091`).

There is a second destructive edge at the same boundary: `applyAgentConversationEvent` calls `ensureConversationSession` before validating the event. If an old or malformed event carries a different provider for an existing `ownedId`, `ensureConversationSession` replaces the whole workspace, including draft and attachments, before sequence checks run (`conversationStore.svelte.ts:168-180`).

Smallest fix direction: make one monotonic store entry point for snapshots, connections, and events. Reject an event whose provider differs from an existing session unless an explicit session-creation path is changing provider; reject a same-generation snapshot whose `lastSequence` is below the store’s current sequence; reject a connection response older than the current generation. Route `loadConversationForRead`, `resyncConversation`, and `ensureStructuredConversation` through those checks. Do not add retries to the invoke wrapper.

### 2. Critical: session selection and session-keyed UI writes are separate operations

The rail quick-jump contract says it activates a session and then its surface, but its `selectSession` method returns `void`. The page adapts the asynchronous `selectOwned` with `void`, so the surface is selected immediately. `selectCenterTab` and `selectRightTab` persist against `rail.activeOwnedId`, while `selectOwned` does not update that id until after a draft flush and an extension-workspace invoke. The requested tab is therefore written under the old session whenever either await yields.

Receipts:

- `tauri-svelte-preview/src/lib/shell/components/sessionRowJump.ts:30-34` makes selection synchronous in the interface.
- `tauri-svelte-preview/src/lib/shell/components/sessionRowJump.ts:78-89` dispatches session selection and surface selection back-to-back.
- `tauri-svelte-preview/src/routes/next/+page.svelte:467-474` discards the `selectOwned` promise.
- `tauri-svelte-preview/src/routes/next/+page.svelte:320-345` writes tab state using the currently active rail id.
- `tauri-svelte-preview/src/routes/next/+page.svelte:825-850` awaits prior-draft flush and extension workspace setup before changing `rail.activeOwnedId`.

Smallest fix direction: change the row-jump target to `selectSession(ownedId): Promise<void>`, make `sessionRowJump` async, and await selection before showing or persisting the surface. Pass the target `ownedId` to the persistence write rather than rereading a mutable global. Apply the same rule to any “session plus surface” action: one awaited page-owned operation with the target id carried through explicitly.

### 3. Critical: capability state is generation-blind and can be stale by design

The store describes capabilities as authoritative for an owned session, but it has no generation field. Snapshot rebuild deliberately carries the existing capability object forward; a new connection does not clear it; the surface refuses to reload when any capability object exists; and an in-flight capability read is accepted based only on provider. This precisely allows a suspended generation’s `prompt.image: false` to control a revived generation.

Receipts:

- `tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:94-96` stores capabilities without generation provenance.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:552-575` preserves them across a snapshot rebuild.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:1080-1091` applies a new connection without invalidating them.
- `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:195-200` skips the read when any capability object is present; the rejection is then explicitly discarded.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:240-255` captures no generation before the invoke.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:849-855` validates only provider when the answer lands.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:557-565` uses the stored snapshot to decide how image attachments are encoded.

Smallest fix direction: store `{ generation, value, error }` for capabilities. Clear or mark unknown when a connection advances generation, and accept a read result only if both provider and generation still match. “Unknown” must remain distinct from “unsupported” and “failed”; a send may ask the backend when unknown, but must not silently treat a failed refresh as a confirmed refusal.

### 4. High: the native command seam is broad, hand-typed twice, and inconsistent about failures

There is no single frontend seam. `tauriSource.ts` holds most calls, but ten domain files also import `invoke` directly. Browser commands go through a generic string-based `invokeBrowserCommandFromTauri<T>`, and usage/path helpers likewise accept arbitrary command strings, so TypeScript cannot connect a command name to its argument and result types.

The apparent type safety is local assertion, not a shared Rust contract. TypeScript independently declares capabilities, event names, event envelopes, connections, and snapshots (`conversationTypes.ts:45-65`, `225-295`, `507-514`, `624-637`). Rust independently declares the corresponding serde types (`src-tauri/src/agent_conversation/protocol.rs:112-123`, `168-235`, `475-508`, `578-596`). No Specta, `tauri-specta`, `ts-rs`, or TypeShare binding setup exists in the searched source.

Current error policies by invoke-owning file group:

| Locations | Current behavior | Architectural issue |
|---|---|---|
| `tauriSource.ts` (107) | Usually returns a browser-runtime sentinel and otherwise lets invoke reject; no command-level logging or normalization. Dynamic browser and path helpers accept arbitrary strings (`tauriSource.ts:922-931`, `1801-1808`). | Callers must each remember to catch, word, and surface errors. |
| `conversationService.ts` (18) | Main send/config/handoff paths rethrow; attachment listing falls back to empty (`199-210`); terminal projection start drops the reason and retries on a later activation (`294-312`); projection stop is fire-and-forget without a rejection handler (`315-317`). | Required and optional failures are not machine-distinguishable. |
| Resource, Playwright, usage, new-session, and most panel backends | Mostly propagate or return explicit browser-unavailable results. `newSessionBackend`’s `ok/unavailable/failed` result is the clearest existing pattern (`newSessionBackend.ts:36-49`). | Good local patterns exist, but they are not the seam’s contract. |
| Capability helpers | Convert every rejection to “no capabilities” and cache it (`backendCapabilities.ts:20-35`; similar copies in `processBackend.ts:62-73` and `worktreesBackend.ts:102-111`). | A transient failure becomes permanent feature absence for the page. |
| Problems backend | Converts only a positively identified unknown command to unavailable and rethrows other failures (`problemsBackend.ts:43-70`). | This is the correct narrow compatibility behavior. |
| Browser model/store | Most model calls attach an async error callback (`browserModel.ts:186-200`), but Back, Forward, and Reload call promise-returning backends with `void` inside synchronous `try` blocks, so rejections escape (`BrowserPanel.svelte:283-295`; `browserStore.svelte.ts:230-246`; backend promises at `browserBackend.ts:245-254`). | User actions can fail without the panel’s error state being set. |

A single thin wrapper is warranted. The smallest useful shape is a `NativeCommandMap` whose keys map to `{ args, result }`, plus `nativeInvoke<K extends keyof NativeCommandMap>(command: K, args: NativeCommandMap[K]['args'], context): Promise<...>`. It should add the command name and an optional `ownedId`/request id to a sanitized log, normalize Tauri errors, publish one shell-visible failure for user actions, and rethrow. It should not retry, cache, add compatibility fallbacks, or decide business policy. Optional and unavailable behavior belongs in the domain adapter and must be explicit.

Migrate one complete domain at a time, beginning with conversation and browser, and prohibit direct `@tauri-apps/api/core` imports for that domain once migrated. Generating the command/type map from Rust is the reliable end state; another hand-maintained TypeScript map alone would improve call-site typing but would not be a shared contract.

### 5. High: subscription ownership has teardown races and module-level leaks

Conversation event setup is fire-and-forget from page mount. If the page unmounts before `listen` resolves, `stopConversationEvents` sees no unlisten function; the pending setup can then finish and retain the old page/store graph. The terminal service already demonstrates the correct late-resolution disposal pattern.

Receipts:

- `tauri-svelte-preview/src/routes/next/+page.svelte:1374-1378` starts conversation events without awaiting or retaining the setup promise.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:401-428` has no in-flight token or disposed check.
- `tauri-svelte-preview/src/routes/next/+page.svelte:1526-1549` calls stop during teardown, but cannot cancel a not-yet-resolved listener.
- `tauri-svelte-preview/src/lib/shell/terminalService.ts:569-621` handles the same race correctly by sharing setup and disposing a late result.

Other ownership gaps:

- `+page.svelte` ignores the disposers returned by `registerShellCommands` and `registerSessionRowJumpTarget` (`+page.svelte:446-475`). It imports `clearWorkbenchNavigation` but never calls it (`+page.svelte:109-113`, teardown at `1526-1550`). Stack handlers also have a clear function (`stackService.ts:83-96`) that the page does not call. These registries retain stale callbacks after same-document route changes.
- The session-library host has the same lifetime problem with no release API at all: registration replaces a module-level host and leaves it there (`sessionLibraryService.ts:141-154`; page registration at `+page.svelte:547-549`).
- `sourceIntelligence` installs a module-singleton `source-lsp-status-changed` listener and discards the unlisten function (`sourceIntelligence.ts:797-826`). `EditorPanel` independently subscribes to the same event and does clean up a resolved listener, but its setup promise has no rejection handler (`EditorPanel.svelte:319-334`, `859-890`).
- Browser listener setup correctly drops a late subscription on unmount, but discards setup failure (`BrowserPanel.svelte:561-574`), leaving Select armable with no event receiver.
- `RunPanel` handles a late successful watcher but has no catch for a rejected `watchRunOutput` promise (`RunPanel.svelte:117-132`).
- The workflow service exposes a singleton `workflow-run-updated` listener and listener fan-out (`workflowService.ts:106-148`), but no non-test caller subscribes. The source-scan progress helper is also unused (`tauriSource.ts:552-563`). These are dormant surfaces, not current dispatch paths.

Smallest fix direction: every async subscribe function should return or share one setup promise and accept teardown-before-resolution. Page mount should collect all registration and listener disposers in one local list and run it once. Remove the duplicate permanent language-server listener by giving the source-intelligence singleton an explicit `dispose`, or route both consumers through one status service. Surface required subscription failures where their controls live.

### 6. High: state ownership is split across stores, page locals, Dockview, and storage

The same facts have multiple writers:

| Fact | Copies | Consequence |
|---|---|---|
| Native session id and execution/runtime owner | Conversation workspace (`conversationStore.svelte.ts:68-96`) and rail `OwnedSession` (`ownedSessions.ts:36-65`). Ensure updates both through separate calls (`conversationService.ts:451-457`); send updates the rail separately (`577-584`); handoff applies only the conversation receipt (`326-342`). | `handoffInput` explicitly chooses rail id first and conversation id second (`+page.svelte:886-897`), masking disagreement rather than preventing it. |
| Model/effort/approval | Conversation `agentConfig` and `metadata` are manually synchronized (`conversationStore.svelte.ts:862-875`, `899-915`), while the rail also stores `model`; persistence combines rail model with conversation effort (`+page.svelte:377-385`). | A config change can leave later metadata persistence using the old rail model. |
| Active center tab | Page `$state`, Dockview’s active panel, per-session center layout, and separate localStorage tab records (`+page.svelte:220-229`, `299-324`; `sessionWorkspaces.ts:60-95`; `workbenchTabs.ts:24-90`). | Restore order, rather than one owner, decides which copy wins. |
| Browser session state | `SessionWorkspaceSnapshot` supports per-session browser state (`sessionWorkspaces.ts:54-95`), but page capture omits it (`+page.svelte:739-755`). The live browser workspace is one global `workspaceId: 'next-browser'` plus one global URL key (`browserStore.svelte.ts:50-66`). | Browser page, ownership, and history can bleed between sessions; the declared per-session browser field is effectively dead. |
| Browser URL and activation | Browser workspace and a compatibility facade duplicate URL/input/activation through `syncLegacy` (`browserStore.svelte.ts:1-8`, `91-120`). | Async native failures and local compatibility values can diverge. |
| Draft | Reactive conversation store plus backend draft record. | This duplication is intentional durability, and revision checks prevent a late load overwriting new typing (`conversationDraftPersistence.ts:48-54`), but debounced write failure is discarded (`31-36`). |

Smallest fix direction: do not create a new general state layer. First add one page/service operation that applies conversation runtime changes to the two current projections, and route ensure/send/handoff updates through it. Then remove rail runtime fields that can be derived safely. Make the per-session workspace record the sole persisted owner of center/right selection and browser state; Dockview and the live browser model remain runtime owners. Remove the separate tab keys and legacy browser URL facade only when their readers have moved in the same change.

### 7. High: readiness gates fail silently and are not reset per page lifetime

`panelActivation` has separate panel-load and session-load booleans. Calls made before either gate opens are silently ignored. The page repairs startup by replaying `sessionPicked` and `restoreWorkspace`, and user-created sessions force the global session gate open early. This is evidence that callers cannot know whether an operation ran.

Receipts:

- `tauri-svelte-preview/src/lib/shell/panelActivation.ts:120-149` owns module-instance gate and “loaded for” state.
- `tauri-svelte-preview/src/lib/shell/panelActivation.ts:219-245` silently returns while gates are closed.
- `tauri-svelte-preview/src/routes/next/+page.svelte:1502-1513` manually replays work skipped during startup.
- `tauri-svelte-preview/src/routes/next/+page.svelte:1050-1062` opens the session gate from the new-session path and calls the loader again.
- The shared `shellPanels` instance is created at module scope (`shellPanels.ts:122-141`) and has no reset/dispose operation. Once a page mount opens its gates, a later mount inherits them.
- The panel gate is opened by an uncancelled timer (`+page.svelte:1714-1720`).
- Workbench navigation uses handler presence as another readiness gate and silently no-ops when absent (`workbenchNavigation.ts:89-107`, `144-158`).
- Backend capability reads collapse “not native”, “old backend”, and “invoke failed” to the same empty answer and cache it (`backendCapabilities.ts:20-35`).

One convention: every gated operation returns `ran`, `deferred`, or `failed`. Startup notifications may be deferred and coalesced to the latest session/panel. A user action must await readiness or receive a displayed failure; it must never silently return. Gates are created and disposed with one page mount, not held in a module singleton. Capability state uses `unknown`, `supported`, `unsupported`, and `failed`, so an invoke failure cannot masquerade as a product limitation.

### 8. Medium-high: backend IO inside effects lacks one consistent cancellation and retry discipline

The architecture claim at the top of `+page.svelte` says IO lives only in explicit functions and never in `$effect` (`+page.svelte:2-11`), but several child effects do backend work:

- Conversation config, capability, and attachment restoration (`ConversationSurface.svelte:177-208`). Config has a generation guard; capabilities do not; attachment failure is silent.
- Context event history (`SessionContextPanel.svelte:66-97`). It guards stale answers, but turns every failure into a successfully loaded empty list.
- Run output watch creation (`RunPanel.svelte:112-132`). It cleans up resolved handles but does not catch setup rejection.
- Browser native placement and browser event subscription (`BrowserPanel.svelte:543-574`).

The browser placement effect documents a prior self-invalidating loop and now uses `untrack` (`BrowserPanel.svelte:543-555`). The remaining issue is acknowledgement: `sendPlacement` records `placed = next` immediately after model calls that start promise-returning native commands (`BrowserPanel.svelte:213-240`; `browserModel.ts:454-495`). If the native bounds/hide call rejects, the local dedupe says that placement succeeded and the same desired rectangle will not be sent again.

No infinite invoke retry loop was found. Conversation resync is bounded to three attempts but makes them back-to-back with no delay (`conversationService.ts:379-390`). The language-server readiness path has a different race: it reads status first and only then subscribes (`sourceIntelligence.ts:769-805`); a transition to ready between those operations can be missed, leaving held questions waiting for a later status event.

Smallest fix direction: effects may request an idempotent controller operation, but they should not own native acknowledgement. Track desired and confirmed browser placement separately and update confirmed only when the promise resolves. Give every IO effect a request token/generation check, a visible error destination, and late-resolution cleanup. For language-server readiness, subscribe first and re-read current status after subscription. Keep retries bounded and single-flight; add delay only where a repeated immediate snapshot can observe the same stale state.

### 9. Medium: event topology is understandable per feature but not consistent at the seam

Current live subscription topology:

| Backend event | Subscriber and lifecycle owner | Dispatch destination |
|---|---|---|
| `agent-conversation-event` | Module singleton in `conversationService.startConversationEvents`; page starts/stops it (`conversationService.ts:401-428`, `+page.svelte:1377`, `1535`). | Sole conversation-store dispatcher via `applyAgentConversationEvent`; also updates rail title and sending state and starts resync. This centralization is good. |
| `terminal_output` | Terminal service subscribes for the page lifetime (`terminalService.ts:569-617`). Run output service opens a second subscription only while Run is visible (`runOutputService.ts:49-106`, `RunPanel.svelte:117-132`). | Terminal service feeds terminal views and rail exit callback. Run service builds per-run tails. |
| `browser-element-selected` | BrowserPanel effect, for component lifetime (`browserElementEvents.ts:47-58`, `BrowserPanel.svelte:561-574`). | Browser model through the panel’s `receiveElement`. |
| `source-lsp-status-changed` | EditorPanel component subscription plus an independent permanent source-intelligence subscription (`EditorPanel.svelte:319-334`, `859-890`; `sourceIntelligence.ts:797-826`). | Editor status state and source-intelligence readiness cache/held questions separately. |
| `workflow-run-updated` | Subscription service exists (`workflowService.ts:106-148`), but no live non-test consumer was found. | Dormant listener fan-out. |
| `source_scan_progress` | Helper exists (`tauriSource.ts:552-563`), but no caller was found. | None. |

Smallest fix direction: retain feature-specific reducers, but standardize subscription ownership: one event adapter per backend event, one explicit start/stop owner, validation at the adapter, and fan-out only when there are genuinely multiple consumers. Consolidate the duplicate language-server subscription. The terminal split is defensible because consumers need different lifetimes, but both should use the same adapter and sequence/seed contract.

### 10. Medium: `+page.svelte` is an ownership boundary without extracted lifecycle units

The page calls itself thin and terminal-only (`+page.svelte:2-11`) but imports and coordinates conversation state/service, editor/explorer/git state, browser navigation, workspaces, session library, stack handlers, terminal service, new-session flow, and three global registries (`+page.svelte:26-155`). It directly owns tab persistence (`299-352`), adapters/registrations (`409-549`), workspace switching (`729-884`), handoff (`886-973`), session lifecycle (`975-1358`), and startup/teardown (`1360-1550`). This makes cross-cutting order bugs hard to see because the contract is a series of local callbacks rather than one operation.

Extraction order, without redesigning behavior:

1. Extract `sessionActivation.ts`: move `selectOwned`, workspace snapshot/restore, and tab selection/persistence together. Its public operation is `activateSession(ownedId, optionalSurface)` and it does not resolve until the rail id, panels, workspace, and requested surface agree. This directly fixes Finding 2.
2. Extract `shellRegistrations.ts`: register shell commands, row jump, workbench navigation, session library, and stack handlers; return one disposer. This directly fixes stale registry ownership in Finding 5.
3. Extract `sessionLifecycle.ts`: move adopt, start, restart, close, remove, and handoff orchestration while continuing to call the existing stores/services. Do not introduce a new store or base controller.
4. Extract `shellStartup.ts`: own terminal-service creation, backend hydration, reattach, gate opening, event startup, and the matching teardown. It returns the hydrated dependencies and one disposer.

After those moves, the page should compose controllers, hold component bindings/snippets, and pass props. Keep each extraction mechanical first; do not combine it with state-model replacement or command migration.

## Every invoke/event ordering point found

This list distinguishes unsafe assumptions from places where the existing code already handles order correctly.

1. **Snapshot read versus conversation events — unsafe.** Live events may arrive during `readAgentConversationSnapshotFromTauri`; ordinary load applies the older answer anyway (`conversationService.ts:395-422`, `conversationStore.svelte.ts:527-603`).
2. **Ensure response versus conversation events — unsafe.** Events may establish a newer connection while ensure is pending; the response then assigns connection fields before resync (`conversationService.ts:451-459`, `conversationStore.svelte.ts:1080-1091`).
3. **Capability read versus generation change — unsafe.** The answer is accepted by provider only (`conversationService.ts:240-255`, `conversationStore.svelte.ts:849-855`).
4. **Send invoke versus user-message event — intentionally handled.** Sent attachments are recorded before invoke because the backend event can arrive while invoke is awaiting, and the reducer claims them on the user message (`conversationService.ts:586-602`, `conversationStore.svelte.ts:242-257`). Preserve this order and make it an explicit seam test.
5. **Terminal-projection registration response versus global conversation events — handled by sequence rules, with silent setup failure.** Registration events are replayed after invoke (`conversationService.ts:294-312`); duplicate/old events are rejected by sequence (`conversationStore.svelte.ts:225-233`).
6. **Resync snapshots versus continuing events — partially handled.** It compares sequence around apply and retries at most three times (`conversationService.ts:379-390`), but the store itself still accepts the stale same-generation snapshot.
7. **Config response versus generation change — handled.** The service rejects a response after generation changes (`conversationService.ts:263-284`).
8. **Handoff receipt versus generation change — handled in the conversation store.** The service validates receipt generation before applying (`conversationService.ts:344-364`). Rail runtime projection is still updated separately or not at all.
9. **Child-transcript response versus another child selection — handled.** The store applies only if the requested child remains selected (`conversationService.ts:96-108`, `conversationStore.svelte.ts:814-820`). The panel discards read errors (`AgentsPanel.svelte:57-72`).
10. **Run scrollback seed versus terminal-output subscription — unsafe gap.** Session mapping and every scrollback are read before the output listener is installed, so bytes emitted in between are absent from both seed and tail (`runOutputService.ts:67-97`).
11. **Terminal-service listener versus terminal creation — handled.** Page awaits `service.attach()` before hydration and session operations (`+page.svelte:1403-1428`), and the service handles teardown during subscription (`terminalService.ts:569-621`).
12. **Browser picker command versus selection event — intended event-after-command contract, but receiver readiness is not guaranteed.** The component keeps a listener while mounted, but setup failure is silent; the picker can be armed with no receiver (`browserElementEvents.ts:43-58`, `BrowserPanel.svelte:561-574`, `browserModel.ts:530-543`).
13. **Language-server status read versus status subscription — unsafe gap.** Source intelligence reads first and subscribes later; a ready event between them can be missed (`sourceIntelligence.ts:769-805`).
14. **Session switch versus surface persistence — unsafe.** This is not a backend event, but it is the same ordering class: the async selection promise is discarded before tab writes (`sessionRowJump.ts:78-89`, `+page.svelte:467-474`, `825-850`).

For the Run output gap, the smallest reliable repair is to make terminal output carry a monotonic cursor and request scrollback through that cursor. Without a cursor, “subscribe first, then seed” trades loss for duplication and cannot be made exact. Until the backend has a cursor, explicitly buffer live chunks during the seed and deduplicate only if the payload exposes an offset; do not pretend ordering is solved with timing.

## Do first / do later / leave alone

### Do first

1. Enforce provider, generation, and sequence monotonicity for every conversation snapshot, connection, event, and capability result.
2. Make session activation plus requested surface one awaited operation carrying the target `ownedId`; stop reading a mutable active id when persisting the target session’s UI.
3. Fix subscription lifetime: conversation setup must handle teardown while `listen` is pending; collect and execute all page registry disposers; remove the permanent duplicate language-server listener.
4. Make user-triggered native failures visible for Back, Forward, Reload, Stop, attachment restore, browser picker subscription, and Run watch setup. Keep best-effort cleanup silent only where failure cannot change the user-visible result.
5. Replace silent readiness returns with `ran/deferred/failed`, keeping gates mount-scoped and replaying deferred startup state in one place.

### Do later

1. Add the typed, logged `nativeInvoke` boundary and migrate conversation, then browser, then the remaining domains. Generate shared command and payload types from Rust rather than maintaining a second handwritten contract.
2. Consolidate rail/conversation runtime projections, center/right persisted selection, and browser per-session state one fact at a time; delete each old storage/read path in the same migration.
3. Extract the four page-owned units in the order listed above, mechanically before altering their behavior.
4. Add a cursor to terminal output so scrollback seeding and live events can be joined without loss or duplication.
5. Remove dormant event APIs (`workflow-run-updated`, source scan progress) if no planned current caller exists, rather than leaving unowned seam surface.

### Leave alone

1. Keep conversation events reduced in `conversationService`/`conversationStore`; components should not become event dispatchers.
2. Keep the attachment-before-send recording order. It is the correct explicit response to event-before-invoke-return behavior.
3. Keep generation checks already present for config, handoff, child transcript, and terminal-projection replay; extend that discipline instead of replacing those paths.
4. Keep per-panel services responsible for their domain state and backend wording. The invoke wrapper should transport, type, log, surface, and rethrow; it should not absorb domain policy.
5. Keep browser and terminal subscriptions visibility-scoped where they are visibility-specific. Consolidate adapters and ownership, not all lifetimes into one global listener.
