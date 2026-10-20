# Timer, observer, and subscription leak audit

Date: 2026-08-20

## Audit basis

- [verified — command: git rev-parse HEAD; git show HEAD:<path>; git grep ... HEAD -- tauri-svelte-preview/src/lib tauri-svelte-preview/src/routes] The audit uses committed HEAD 0266047232fb01af669c97ca7efb6db598a0d616 (Explorer serves revisited roots from the in-memory index). Dirty profiling edits were not used; all source receipts below come from HEAD. The remaining dirty paths and untracked reports are recorded by [verified — command: git status --short].
- [verified — tauri-svelte-preview/src/lib/shell/components/RightPanel.svelte:8-11] The eight right-panel children stay mounted for the whole session; tab changes only change display. [verified — tauri-svelte-preview/src/routes/next/+page.svelte:1662-1673] ConversationSurface stays in the shell, and [verified — tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:424-467] its terminal layer and structured transcript are not keyed by active session. [verified — tauri-svelte-preview/src/lib/shell/components/TerminalSurface.svelte:132-148] Terminal hosts are keyed by ownedId and are hidden or shown, not rebuilt for each active-session change.
- [verified — tauri-svelte-preview/src/lib/shell/editor/editorStore.svelte.ts:167-177] Restoring a session clears activePath before the arriving session is replayed. That matters to the EditorPanel native-C# finding below.
- [verified — audit method] Class labels are the four requested labels. SAFE-once is reserved for work created once for the app lifetime (or once per shared process service) and never recreated by a session/root change. SAFE-torn-down means cleanup runs before every effect/action re-entry or at destruction. A finite callback that can outlive its host through an async race, teardown, or error is LEAK-conditional even when it ends on its own. “Weight” is a closure-retention estimate; lifecycle and line receipts are verified, while the size of retained object graphs is marked assumed.

## Executive result

[verified — inventory below] Most current /next timers, intervals, observers, DOM listeners, and Tauri listeners have reliable effect/action teardown. There is no committed new EventSource, WebSocket, event.listen, or appWindow.listen call in src.

[verified — inventory below] No requested primitive is a verified LEAK-per-switch in the current /next shell. The active findings are conditional stale work or per-failed/per-event retries; the clearly per-session growth is adjacent retained state such as hidden xterm views and browser snapshots.

[verified — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:593-611,947-955,1204-1214] The clearest timer-count growth is semantic reference-count retry work: each failed symbol can leave up to three sequential untracked delayed timers after a root/file change, and many symbols can overlap.

[verified — tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:538-555,882-899; tauri-svelte-preview/src/lib/shell/editor/csharpLanguageClient.ts:646-652] A native C# diagnostics/action callback can remain registered when the always-mounted EditorPanel leaves a C# file for a session with no active C# file. This is a persistent closure leak, not a timer.

[verified — tauri-svelte-preview/src/lib/shell/components/editor/languageServerStatus.ts:274-309; tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:402-445,882-899] A single 20-second language-server gate timer can retain an ever-growing waiter array across session changes because releaseSessionResources does not release the gate. The timer count is one, but the retained callbacks can grow per request/switch.

[verified — tauri-svelte-preview/src/lib/shell/editor/extensionApiProbeController.ts:185-225,267-280; tauri-svelte-preview/src/lib/shell/terminalService.ts:725-742] A stale extension-probe request can create an xterm/PTY lease after the ownedId/root has changed and then lose the lease without disposing it. This is conditional on a probe request racing a switch, but its retained weight is high.

[assumed — tauri-svelte-preview/src/lib/shell/xtermFactory.ts:160-166; tauri-svelte-preview/src/lib/liveConversationTerminals.ts:128-168,273-303] The largest ordinary memory slope is likely intentional xterm retention: approximately 20 MB of scrollback per live view, including hidden views. That is one view per live owned session, not one view per switch, and it is released only when the session view is closed or the service is disposed.

## Leak findings, ranked by growth and retained weight

The numbered order is the ranking. Each entry states whether growth is per switch, per failed/event request, or only a teardown race; lifecycle facts are marked `[verified]` and retained-object size is marked `[assumed]`.

### 1. Semantic reference-count retry timers — LEAK-conditional

- [verified — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:593-611] When a semantic count returns null, onCounted creates askAgain and schedules an untracked setTimeout. The callback only checks waitingSpots at :603-605; there is no timer id to clear.
- [verified — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:136-140] The retry policy permits three retries with 2, 6, and 12 second delays.
- [verified — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:947-955,1204-1214] Root or active-preview changes clear the scheduler and waitingSpots, then resolve old waiters, but do not cancel already scheduled retry handles. Those handles still fire and only then no-op.
- [verified — tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:700-715] The always-mounted editor changes sourceIntelligence projectRoot and active preview as the session/file changes, so this stale-timer path is reachable during ordinary switching.
- [verified — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:593-611,947-955] Classification: LEAK-conditional. It is event-growth (one to three sequential timers per failed symbol, with many symbols overlapping), not a permanent one-per-switch timer.
- [assumed — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:593-611] Each timer retains its WaitingSpot, request/source preview references, and waiter closures until it fires. It is unlikely to pin the whole shell, but repeated failed-symbol events can explain a visible timer pile.

### 2. Native C# diagnostics and document actions — LEAK-conditional

- [verified — tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:538-555] A successful C# attach stores stopNativeCsharpActions and stopNativeCsharpDiagnostics. The diagnostics callback closes over the root and the EditorPanel state.
- [verified — tauri-svelte-preview/src/lib/shell/editor/csharpLanguageClient.ts:646-652] subscribeNativeCsharpDiagnostics adds the callback to a process-wide Set and returns the only removal function.
- [verified — tauri-svelte-preview/src/routes/next/+page.svelte:906-910; tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:882-899] Session switching calls releaseSessionResources, but that function clears diagnostics timers/models/previews and never calls either native-C# stop handle or resets nativeCsharpRoot/nativeCsharpPath.
- [verified — tauri-svelte-preview/src/lib/shell/editor/editorStore.svelte.ts:167-177] The arriving empty session has activePath null, so no later ensureNativeCsharpForActiveFile call reaches the replacement/stop lines :538-544.
- [verified — tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:627-641,994-1003] The callbacks are stopped by an explicit language-intelligence-off action or eventual component destruction, neither of which occurs on an ordinary active-session switch.
- [verified — tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:538-555,882-899; tauri-svelte-preview/src/lib/shell/editor/csharpLanguageClient.ts:646-652] Classification: LEAK-conditional (switch-to-no-C# trigger). It is not proven to create a new Set entry on every switch; one stale callback pair can remain until the next successful C# attach or destruction.
- [assumed — tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:538-555; tauri-svelte-preview/src/lib/shell/editor/csharpLanguageClient.ts:634-652] The diagnostics-Set callback and global document-action closures retain the EditorPanel, Monaco-facing state, and native-client references, making the retained weight high even when the callback ignores mismatched roots.

### 3. Language-server gate waiters — LEAK-conditional

- [verified — tauri-svelte-preview/src/lib/shell/components/editor/languageServerStatus.ts:267-315] waitUntilReady pushes every waiter into an array at :304-309 and arms one max-wait timer at :274-275. The timer is cancelled when state becomes non-busy or releaseAll runs at :282-314.
- [verified — tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:402-411,432-445] Diagnostics and inlay-hint paths can enqueue waits keyed by the current path. Their stale checks only run after the wait resolves.
- [verified — tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:882-899] releaseSessionResources increments its own generation and clears diagnosticsTimer, but does not call languageServerGate.releaseAll. The only releaseAll call is onDestroy at :1002-1003.
- [verified — tauri-svelte-preview/src/lib/shell/components/editor/languageServerStatus.ts:267-315; tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:402-445,882-899] Classification: LEAK-conditional. The timer itself is single-flight, but stale waiter closures can accumulate per request/switch for up to 20 seconds.
- [assumed — tauri-svelte-preview/src/lib/shell/components/editor/languageServerStatus.ts:304-309] Waiters retain EditorPanel callbacks and request state while the gate is busy; the retained weight is medium to high for a busy language-server start, but bounded by the deadline.

### 4. Extension-probe terminal and SCM leases — LEAK-conditional

- [verified — tauri-svelte-preview/src/lib/shell/editor/extensionApiProbeController.ts:185-225] createExtensionApiProbeTerminal awaits createProbe at :221 and then calls requireCurrent at :222. If ownedId/root changes during the await, requireCurrent throws before the local lease is assigned to terminalLease or disposed.
- [verified — tauri-svelte-preview/src/lib/shell/terminalService.ts:725-742] createProbe creates a PTY/view lease and records it in probesByOwned before returning it.
- [verified — tauri-svelte-preview/src/lib/shell/editor/extensionApiProbeController.ts:267-280] acquireExtensionApiProbeScm has the same await-then-requireCurrent pattern; a stale SCM lease is not disposed when :278 throws.
- [verified — tauri-svelte-preview/src/routes/next/+page.svelte:868-897] Session switching changes the extension-probe workspace by ownedId/root. [assumed — tauri-svelte-preview/src/lib/shell/editor/extensionApiProbeController.ts:185-225,267-280] A probe command is in flight during that switch; no committed proof showed that race in a normal user path.
- [verified — tauri-svelte-preview/src/lib/shell/editor/extensionApiProbeController.ts:185-225,267-280; tauri-svelte-preview/src/routes/next/+page.svelte:868-897] Classification: LEAK-conditional, per raced probe/session change.
- [assumed — tauri-svelte-preview/src/lib/shell/editor/extensionApiProbeController.ts:221-225; tauri-svelte-preview/src/lib/shell/terminalService.ts:725-742] A lost terminal lease retains an xterm view, addon state, PTY mapping, and host closure; this is very high weight. A lost SCM lease retains its adapter resources.

### 5. Legacy embedded xterm async setup — LEAK-conditional

- [verified — tauri-svelte-preview/src/routes/+page.svelte:7730-7763] ensureEmbeddedTerminalRenderer awaits several addon imports and only checks embeddedTerminalElement/embeddedTerminal before constructing.
- [verified — tauri-svelte-preview/src/routes/+page.svelte:7765-7839] The late continuation constructs XTerm, Fit/Search/Serialize/Unicode/WebLinks/WebGL addons, opens the host, and installs an onData callback. There is no route generation/disposed check.
- [verified — tauri-svelte-preview/src/routes/+page.svelte:3201-3221,12084-12089] Both the terminal ResizeObserver effect and the dock action call ensureEmbeddedTerminalRenderer().then(scheduleEmbeddedTerminalFit) without a dropped/generation flag.
- [verified — tauri-svelte-preview/src/routes/+page.svelte:8185-8211,8237-8255,14822-14829] The fit rAF/timers are coalesced by clearEmbeddedTerminalFitSchedule, but effect cleanup only cancels its local resize frame/observer; route disposal clears the current terminal and current fit schedule only. A late import can start a new chain after either cleanup.
- [verified — tauri-svelte-preview/src/routes/+page.svelte:3201-3221,7730-7839,8185-8211,8237-8255] Classification: LEAK-conditional, one full renderer/addon chain per teardown/import race.
- [assumed — tauri-svelte-preview/src/routes/+page.svelte:7765-7839] The late xterm and addon graph can pin the route, detached host, input callback, and terminal buffers. This is high retained weight, but the route is the legacy route rather than the current /next shell.

### 6. Legacy Tauri listener promises — LEAK-conditional

- [verified — tauri-svelte-preview/src/lib/tauriSource.ts:575-586,824-835] The source-scan and terminal-output helpers return a Tauri unlisten function only after the async listen call resolves.
- [verified — tauri-svelte-preview/src/routes/+page.svelte:14641-14660] The old route stores each handle only in a .then callback and has no disposed/dropped flag.
- [verified — tauri-svelte-preview/src/routes/+page.svelte:14822-14829] Teardown calls nullable handles immediately. If teardown wins the race, the handle is null at cleanup and the late .then stores a live subscription afterward.
- [verified — tauri-svelte-preview/src/lib/tauriSource.ts:575-586,824-835; tauri-svelte-preview/src/routes/+page.svelte:14641-14660,14822-14829] Classification: LEAK-conditional, one native listener per route-teardown race.
- [assumed — tauri-svelte-preview/src/routes/+page.svelte:14641-14660] Each leaked listener retains the old route callback and route state until native teardown, so the weight is high. This is dormant for /next, which uses the race-safe terminalService path.

### 7. Legacy pointer-resize listeners — LEAK-conditional

- [verified — tauri-svelte-preview/src/routes/+page.svelte:13199-13201,13660-13662,13801-13803] Three resize paths add global pointermove, pointerup, and pointercancel listeners.
- [verified — tauri-svelte-preview/src/routes/+page.svelte:13172-13197,13620-13658,13778-13799] Each path removes its listeners only inside finishResize.
- [verified — tauri-svelte-preview/src/routes/+page.svelte:14822-14839] Route teardown does not call any active finish/cancel function.
- [verified — tauri-svelte-preview/src/routes/+page.svelte:13199-13201,13660-13662,13801-13803,14822-14839] Classification: LEAK-conditional, one listener trio per abandoned drag if no pointerup/pointercancel arrives before teardown.
- [assumed — tauri-svelte-preview/src/routes/+page.svelte:13199-13201,13660-13662,13801-13803] The global handlers retain route closures and sizing state; weight is medium to high, but growth is per abandoned drag rather than per ordinary session switch.

### 8. Monaco layout observer after a failed retry — LEAK-conditional

- [verified — tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2921-2928] buildEditor assigns layoutObserver and schedules layoutFrame.
- [verified — tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2645-2656] retryStartingEditor disposes editor/actions and starts another build, but does not disconnect the previous layoutObserver or cancel its frame.
- [verified — tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:3001-3028] onDestroy disconnects only the current layoutObserver and cancels only the current layoutFrame.
- [assumed — tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2921-2967] A build exception after :2921 but before a successful retry is the trigger. Each such failed retry can orphan one observer/frame retaining the editor host/component.
- [verified — tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2645-2656,2921-2928,3001-3028] Classification: LEAK-conditional, per build-error/retry event. Normal successful startup is torn down correctly.

### 9. Pull-request status poll — LEAK-conditional, legacy/dormant in /next

- [verified — tauri-svelte-preview/src/lib/shell/components/git/pr/PullRequestPanel.svelte:155-163] A pending status schedules another 2.5-second poll.
- [verified — tauri-svelte-preview/src/lib/shell/components/git/pr/PullRequestPanel.svelte:73-79] pollTimer is cleared only when the PullRequestPanel is destroyed.
- [verified — tauri-svelte-preview/src/lib/shell/components/git/pr/SourceControlPanes.svelte:183-191] The panel can remain mounted while root/activeAgent props change; there is no prop-change cancellation or generation check around pollStatus.
- [verified — tauri-svelte-preview/src/lib/shell/components/RightPanel.svelte:19-26; command: git grep -n '<GitPanel\|import GitPanel' HEAD -- tauri-svelte-preview/src] Current /next imports SourceControlPanel, and HEAD has no committed GitPanel/SourceControlPanes caller. The PullRequestPanel definition is therefore dormant in this committed tree.
- [verified — tauri-svelte-preview/src/lib/shell/components/git/pr/PullRequestPanel.svelte:73-79,155-163; tauri-svelte-preview/src/lib/shell/components/git/pr/SourceControlPanes.svelte:183-191] Classification: LEAK-conditional, stale cross-root/agent poll chain. A second overlapping chain per switch was not proven; do not count this as a verified one-per-switch timer.
- [assumed — tauri-svelte-preview/src/lib/shell/components/git/pr/PullRequestPanel.svelte:155-163] A pending timer retains the pull-request component, model, root, and agent state until it fires or the panel is destroyed.

### 10. ConversationTimeline stale frames — LEAK-conditional, bounded

- [verified — tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:193-205,207-221] A renderWindowId change resets scroll state, and the restore effect schedules an uncancelled rAF at :217 after tick(). Re-entry can leave a stale frame aimed at the old host/target.
- [verified — tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:282-325] animateTo cancels its prior animation before starting; however the renderWindowId effect at :193-205 does not call cancelProgrammaticScroll, so an old 180ms chain can run against the arriving transcript.
- [verified — tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:360-375] anchorUser schedules a retry rAF at :371 without cancelling an existing retry first; overlapping retries are possible for at most eight frames.
- [verified — tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:413-435,558-569] Hydration frames are cancelled before replacement and user-input action teardown cancels both animationFrame and hydrationFrame.
- [verified — tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:193-221,282-325,360-375] Classification: LEAK-conditional for :217, :315, :324, and :371 when a switch/re-entry races; bounded one-frame/eight-frame/180ms stale work, not permanent accumulation.
- [assumed — tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:217,315,324,371] The frames retain the timeline host and small callback closures only briefly; weight is low.

### 11. Editor diagnostics settle timer gate race — LEAK-conditional, bounded

- [verified — tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:384-420] diagnosticsTimer is cleared before replacement and the 650ms callback checks destroyed.
- [verified — tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:402-411,882-885] The gate continuation captures only path, while releaseSessionResources clears the current timer and increments sessionResourceGeneration without cancelling the pending wait. If the same path is restored, the late continuation can start a new timer.
- [verified — tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:997-999] Destruction clears the currently stored timer, but not a late continuation that has not yet restarted it.
- [verified — tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:384-420,402-411,882-885,997-999] Classification: LEAK-conditional, one 650ms timer per gate race; not an unbounded timer chain.
- [assumed — tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:416-420] The timer closure retains the EditorPanel for the settle window; weight is medium for the short interval.

### 12. Reference-count batch window — LEAK-conditional, bounded

- [verified — tauri-svelte-preview/src/lib/shell/editor/referenceCountBatcher.ts:131-167] count arms one shared 50ms windowTimer at :161, and askForWaitingSymbols clears the handle at :135 before draining the waiting map.
- [verified — tauri-svelte-preview/src/lib/shell/editor/referenceCountBatcher.ts:164-166] forget() clears only remembered memory. It does not clear waiting callbacks or windowTimer.
- [verified — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:1204-1217] Root and active-preview changes stop the semantic scheduler but never call a batcher cancellation method.
- [verified — tauri-svelte-preview/src/lib/shell/editor/referenceCountBatcher.ts:131-167; tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:1204-1217] Classification: LEAK-conditional, one stale batch timer/queue per timing race. If countReferences itself is slow, its batch closure can live longer than 50ms.
- [assumed — tauri-svelte-preview/src/lib/shell/editor/referenceCountBatcher.ts:131-167] The closure retains pending symbol names and caller promises; whole-subtree retention is not shown.

### 13. Reference-count persistence timer after service disposal — LEAK-conditional, bounded

- [verified — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:487-501] Browser persistence is single-flight and schedules one 100ms timer.
- [verified — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:1197-1203; tauri-svelte-preview/src/routes/next/+page.svelte:1415-1424] Route teardown calls sourceIntelligence.dispose(), but dispose does not clear persistCountsTimer.
- [verified — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:487-501,1197-1203; tauri-svelte-preview/src/routes/next/+page.svelte:1415-1424] Classification: LEAK-conditional, at most one 100ms callback after route teardown. It is not a session-switch accumulation in the app-lifetime path.
- [assumed — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:495-501] The callback retains countStore and persistence entries briefly; weight is low.

### 14. Conversation draft timer on session deletion — LEAK-conditional, bounded

- [verified — tauri-svelte-preview/src/lib/shell/conversation/conversationDraftPersistence.ts:31-37,40-45,57-68] Normal scheduling, flush, and clear each cancel the prior per-ownedId timer.
- [verified — tauri-svelte-preview/src/routes/next/+page.svelte:884-891] Ordinary session switching flushes the previous draft.
- [verified — tauri-svelte-preview/src/routes/next/+page.svelte:1393-1413] removeSession deletes the session but never flushes or clears its pending draft; the 500ms timer therefore survives until it fires.
- [verified — tauri-svelte-preview/src/lib/shell/conversation/conversationDraftPersistence.ts:31-37,40-45,57-68; tauri-svelte-preview/src/routes/next/+page.svelte:1393-1413] Classification: LEAK-conditional, one stale timer per deletion while a draft is pending.
- [assumed — tauri-svelte-preview/src/lib/shell/conversation/conversationDraftPersistence.ts:31-37] The closure retains draft text and store callbacks, not the whole panel.

### 15. Conversation event setup when the second Tauri listen rejects — LEAK-conditional

- [verified — tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:533-571] The first listener handle is held only in the local stop variable until both listens complete. The global handles are assigned at :569-570.
- [verified — tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:560-567] If session-title-changed rejects after the first listen succeeds, the first stop is never assigned or called.
- [verified — tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:579-586; tauri-svelte-preview/src/routes/next/+page.svelte:1415-1424] Normal stop/dispose can only call the global handles, so it cannot recover that error-path handle.
- [verified — tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:533-577,560-576,579-586] Classification: LEAK-conditional, one global listener per failed two-listen setup/retry. The setup promise is cleared in :572-576, so a later start can retry while a lost first handle remains. Normal setup is SAFE-once.
- [assumed — tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:538-570] The listener closure retains module stores rather than a single panel subtree; weight is medium, lifetime can be app-long.

### 16. Workflow snapshot subscription race — LEAK-conditional, dormant

- [verified — tauri-svelte-preview/src/lib/shell/workflows/workflowService.ts:118-141] Concurrent subscribers can both see unlisten null and call listen; the later handle overwrites the earlier one at :127. An unsubscribe while the await is pending can also return with no handle and then receive a late handle.
- [verified — tauri-svelte-preview/src/lib/shell/workflows/workflowService.ts:144-148] stopWorkflowSnapshotSubscription only stops the handle still in the global slot.
- [verified — command: git grep -n subscribeWorkflowSnapshots HEAD -- tauri-svelte-preview/src/lib tauri-svelte-preview/src/routes] There is no committed caller outside the definition.
- [verified — tauri-svelte-preview/src/lib/shell/workflows/workflowService.ts:118-148] Classification: LEAK-conditional, dormant API. [assumed — tauri-svelte-preview/src/lib/shell/workflows/workflowService.ts:118-141] The trigger would be concurrent/future subscription or an early unsubscribe; current /next has no live instance.

### 17. Monaco startup polling after teardown — LEAK-conditional, bounded

- [verified — tauri-svelte-preview/src/lib/shell/components/editor/editorStartup.ts:50-68] waitForRegisteredStart arms an untracked 25ms setTimeout for each polling step. The loop eventually stops at its caller deadline, but it has no cancellation handle or destroyed signal.
- [verified — tauri-svelte-preview/src/lib/shell/editor/vscodeServices.ts:59-60; tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2673-2677,3001-3006] Monaco calls that poll during buildEditor, including retryStartingEditor, while onDestroy only clears startLimitTimer. No teardown path cancels the polling Promise.
- [verified — tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2702-2705] A late continuation eventually checks destroyed, so it normally stops building, but the queued polling timers still run until the registration/deadline condition wins.
- [verified — tauri-svelte-preview/src/lib/shell/components/editor/editorStartup.ts:50-68; tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2673-2677,3001-3006] Classification: LEAK-conditional, bounded teardown race; one polling chain can retain its build closure for up to the caller's ten-second deadline. It is not a per-session-switch accumulator.
- [assumed — tauri-svelte-preview/src/lib/shell/components/editor/editorStartup.ts:50-68] The retained graph is low to medium weight (startup closure and service references, not a mounted subtree), but a repeatedly abandoned/retried startup can leave several short-lived chains.

### 18. Event-local callbacks that can outlive their host — LEAK-conditional, bounded

- [verified — tauri-svelte-preview/src/lib/components/ui/select/select-trigger.svelte:34-39; tauri-svelte-preview/src/lib/sourceDockviewWorkspace.ts:902-913; tauri-svelte-preview/src/routes/next/+page.svelte:685-688,752,1788] These paths schedule untracked zero-delay/next-frame callbacks on clicks, panel attaches, refits, bounded host retries, or shell readiness. No callback is a persistent loop, but teardown can occur before it runs.
- [verified — tauri-svelte-preview/src/routes/+page.svelte:8530-8545,9168-9196,10450-10572,11142-11567,13106-13247,14798] Legacy code-lens, focus, measurement, and panel-sync timers likewise have finite event lifetimes but no universal route-teardown cancellation.
- [verified — tauri-svelte-preview/src/lib/shell/floatingSurface.ts:4-9] The floating-surface helper deliberately queues two nested frames and exposes no cancel handle; callers can close before the second frame.
- [verified — tauri-svelte-preview/src/lib/components/ui/select/select-trigger.svelte:34-39; tauri-svelte-preview/src/lib/sourceDockviewWorkspace.ts:902-913; tauri-svelte-preview/src/routes/+page.svelte:8530-8545,9168-9196,10450-10572,11142-11567,13106-13247,14798; tauri-svelte-preview/src/routes/next/+page.svelte:685-688,752,1788; tauri-svelte-preview/src/lib/shell/floatingSurface.ts:4-9] Classification: LEAK-conditional, bounded and low priority. Growth is per UI event or layout burst, not per session switch; each stale callback lasts at most a task, frame, or short retry window.
- [assumed — tauri-svelte-preview/src/lib/components/ui/select/select-trigger.svelte:34-39; tauri-svelte-preview/src/lib/sourceDockviewWorkspace.ts:912; tauri-svelte-preview/src/lib/shell/floatingSurface.ts:6-8] Retained weight is low: a trigger, element/workspace, or popover closure survives briefly, with no evidence of a pinned shell subtree.

## Complete runtime occurrence inventory

Every runtime occurrence from the requested grep is classified below. Type-only declarations, comments, and test source matches are called out separately at the end.

### Top-level editor and UI components

- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:690,991,1043,2025,2210,2254,2267,2615,2660-2661. Deadline, CodeLens repaint/recount, target-line rAF, cancellation/notice/start timers, and window error listeners are cleared/replaced at :672-681, :1081-1100, :1431-1448, :3010-3025. The retry-specific layout exception is the LEAK-conditional finding at :2921-2928 above.
- [verified] LEAK-conditional — tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2549,2921-2923. The host-wait rAF is bounded by editorStartup isWanted checks at tauri-svelte-preview/src/lib/shell/components/editor/editorStartup.ts:75-95. The layout observer/frame are normally cleaned at :3010-3013 and :3027-3028, but can be orphaned by the retry/error path described above.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/SourceWorkbench.svelte:125-126. initialize disposes the previous workbench at :91-105, stale async tokens dispose late instances at :116-119, and the action/effect teardown runs at :139-161.
- [verified] LEAK-conditional, bounded — tauri-svelte-preview/src/lib/components/ui/select/select-trigger.svelte:34-39. The click-local zero-delay callback normally ends in the next task, but can outlive a trigger teardown briefly; see finding 18.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/components/CenterCornerTabs.svelte:76-79; tauri-svelte-preview/src/lib/shell/components/SegmentedTabs.svelte:50-58; tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte:137-140; tauri-svelte-preview/src/lib/shell/components/ShellOverlays.svelte:75-78. Hold timers, tab ResizeObserver/rAF, frame ResizeObserver, and overlay MutationObservers have matching release/disconnect paths at CenterCornerTabs:59-62,82-84; ShellFrame:160-166; ShellOverlays:81-87.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/components/TerminalSurface.svelte:105,107,110,113,118. Each host action cancels the rAF, disconnects both observers, and removes the fonts listener at :120-126. Hosts are keyed by ownedId at :132-148; an active-session switch does not rerun the action for the same host.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/components/UtilityStrip.svelte:70,84; tauri-svelte-preview/src/lib/shell/components/WorktreeAgentRow.svelte:298. Visibility listener/interval cleanup is at UtilityStrip:77-85. The row hover timer is cleared on show/hide and onDestroy at WorktreeAgentRow:244-249,320-323,362.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/components/conversation/CodeBlock.svelte:42,61; ConversationComposer.svelte:229; ConversationMessage.svelte:41; FileChangeItem.svelte:99; SessionPresenceIndicator.svelte:62; TurnMetadata.svelte:45. Intersection/ResizeObservers and copy/presence timers are returned or cleared before replacement and on effect/action teardown at CodeBlock:52-54,60; ConversationComposer:229-233; ConversationMessage:41-44; FileChangeItem:90-92; SessionPresenceIndicator:62-66; TurnMetadata:36-45.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:187,247,421,434,559-561. The two ResizeObservers and user listeners are torn down at :187-191,247-253,562-569. Hydration rAF :421/:434 is cancelled before replacement at :433.
- [verified] LEAK-conditional, bounded — tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:217,315,324,371. The restore/animation/anchor frames have the bounded switch races described in finding 10.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:361. The status Tauri listener is late-stopped by onMount :987-992 and :999.
- [verified] LEAK-conditional — tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:416,544. The diagnostics timer is normally cleared at :386-387, :884-885, and :997-998 but can be restarted by the late gate continuation in finding 11; the native C# subscription at :544 is the persistent switch-to-empty case in finding 2.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/components/git/SourceControlContextMenu.svelte:38. The pointerdown listener is removed at :40.
- [verified] LEAK-conditional — tauri-svelte-preview/src/lib/shell/components/git/pr/PullRequestPanel.svelte:162. See finding 9; the only cleanup is :73-79.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/panels/agents/AgentsPanel.svelte:67; HistoryPanel.svelte:156. Each interval is effect-keyed by visibility/session state and cleared at AgentsPanel:75 and HistoryPanel:157.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/panels/browser/AnnotationCanvas.svelte:132; BrowserPanel.svelte:666,681; FilesPanel.svelte:132-133,159-188; SourceControlPanel.svelte:186. Observers and scroll/resize listeners are removed in their effect cleanups at AnnotationCanvas:132-140, BrowserPanel:674,683, FilesPanel:128-140, and SourceControlPanel:179-191. The FilesPanel Tauri file watch uses an abandoned flag, unwatch, and late-stop at :159-188.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/components/railElapsedTicker.ts:47. The singleton interval clears its previous id before retuning at :43-47 and stops when the last watcher releases at :62-68. WorktreeAgentRow's committed diagnostic-disable guard at :200 prevents a ticker watcher in current HEAD; if enabled, its release is returned at :204-211.

### Services and shared helpers

- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/usage/usageStore.svelte.ts:18. Promise.race finally clears the timeout at :20-22; it is request-scoped, not root/session keyed.
- [verified] LEAK-conditional, bounded — tauri-svelte-preview/src/lib/shell/components/editor/editorStartup.ts:67. One 25ms polling timeout is awaited at a time and the loop is bounded by its deadline, but teardown does not cancel the chain; see finding 17. It is not recreated by an active-session prop effect.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/components/editor/languageServerStatus.ts:274-275 in the normal ready/release path. The conditional switch-retained waiter queue is finding 3.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:135,297-308,809. The finite reference-count lookup deadline (1.5 seconds at :135) clears on resolve/reject, and the Tauri status listener at :809-831 uses generation checks and dispose.
- [verified] LEAK-conditional — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:495-501. The persistence timer is single-flight and normally replaces no live timer, but dispose does not clear the final 100ms callback (finding 13).
- [verified] SAFE-once — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:517-520. The hydration timeout is created once during service startup and is not keyed by root/session.
- [verified] LEAK-conditional — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:608,610. These are the untracked semantic retries in finding 1.
- [verified] LEAK-conditional — tauri-svelte-preview/src/lib/shell/editor/referenceCountBatcher.ts:161. See finding 12; normal batching is one finite timer, but forget/root change cannot cancel a pending batch.
- [verified] LEAK-conditional — tauri-svelte-preview/src/lib/shell/conversation/conversationDraftPersistence.ts:34. Normal schedule/flush/clear cancels the per-ownedId timer; deletion omission is finding 14.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/terminalService.ts:53,208,452,579. The :53 interface and :208 backend implementation are the service wrapper; the actual shared Tauri listen is :579. Nudge timers are replaced/cancelled at :449-477, :409-423, and :854-865. The shared backend listen is guarded by attaching/disposed at :569-617 and stops late handles at :609-614.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/stacks/runOutputService.ts:91. RunPanel cleanup at tauri-svelte-preview/src/lib/shell/panels/run/RunPanel.svelte:136-151 cancels the watch and stops a late Tauri handle; run list/scrollback/listen checks are at runOutputService:68-106.
- [verified] SAFE-once — tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:538,560-563. The normal global setup is guarded at :533-577 and stopped at :579-586; the second-listen rejection omission is finding 15.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/browser/browserBackend.ts:86. BrowserPanel's dropped flag and late unsubscribe at BrowserPanel.svelte:737-750 cover the async listen race.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/browser/browserElementEvents.ts:52. The helper returns a stop function at :47-58, and there is no committed caller in src.
- [verified] LEAK-conditional, dormant — tauri-svelte-preview/src/lib/shell/workflows/workflowService.ts:127. See finding 16; there is no committed caller.
- [verified] LEAK-conditional through the legacy caller — tauri-svelte-preview/src/lib/tauriSource.ts:583,832. The wrappers return handles correctly; the old route's late-resolution teardown race is finding 6.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/elementVisibility.ts:40. One shared IntersectionObserver serves the map and each caller receives removal/unobserve at :44-51. Current callers WorktreeAgentRow.svelte:185-191 and WorkingSpinner.svelte:36-42 return those releases; the observer disconnects when the map is empty.
- [verified] LEAK-conditional, bounded — tauri-svelte-preview/src/lib/sourceDockviewWorkspace.ts:912. A panel attach queues one microtask and one next-frame layout dispatch with no cancel handle; it ends within one frame and has no session-switch loop (finding 18).
- [verified] LEAK-conditional, bounded — tauri-svelte-preview/src/lib/shell/floatingSurface.ts:6,8. The caller-provided scheduler runs exactly two frames and then ends, but callers do not retain/cancel the frames when a popover closes; weight is low and event-local.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/lib/shell/layout/frame.ts:329,374,412,434; centerDock.ts:204,226,287,350; paneStack.ts:468,513,552. Layout and roster callbacks are one-shot, persist timers clear before replacement, and factory dispose clears the persistent timer at frame:439-444, centerDock:352-357, paneStack:561-566. Disposed guards make an already queued one-shot harmless.
- [verified] SAFE-once — tauri-svelte-preview/src/lib/shell/editor/csharpLanguageClient.ts:359. The global diagnostics registration is guarded by commandsRegistered at :325-327 and is not session-keyed; the client map is bounded/evicted at :44,83-84,448-458.
- [verified] SAFE-once — tauri-svelte-preview/src/lib/shell/openFileBus.ts:26,44-53 with EditorPanel.svelte:976-1007. EditorPanel registers one open-file callback for its app-life mount and removes it on destruction; it is not re-registered per active session.

### Legacy route inventory

- [verified] SAFE-torn-down — tauri-svelte-preview/src/routes/+page.svelte:3096; :3118,3123,3125,3131; :3148,3150; :3210,3212. Workspace-symbol debounce, fallback/window resize listeners, all three local ResizeObservers, and their rAFs return cleanup at :3100, :3119, :3132-3136, :3156-3159, and :3218-3221. The embedded renderer promise race is the exception described in finding 5.
- [verified] LEAK-conditional — tauri-svelte-preview/src/routes/+page.svelte:8205,8210. scheduleEmbeddedTerminalFit clears the previous rAF/timers at :8185-8196 and route disposal clears the current schedule, but late renderer completion can start a new chain after effect cleanup.
- [verified] LEAK-conditional, bounded — tauri-svelte-preview/src/routes/+page.svelte:8533,9168,9179,9188,9196. Code-lens timeout settlement and quick-open focus callbacks are finite event-local timers; promise/deadline clears exist at :8530-8545, but route teardown can precede the callback (finding 18).
- [verified] LEAK-conditional — tauri-svelte-preview/src/routes/+page.svelte:9967,9985,11448. Diagnostics and warm-root timers coalesce/guard normally at :9944-9955 and :9981-9989, and the authored-dock timer coalesces at :11445-11451, but route teardown :14822-14839 does not clear them. Each is at most one short timer (650ms, 400ms, or 160ms) per teardown/early-return race.
- [verified] LEAK-conditional, bounded — tauri-svelte-preview/src/routes/+page.svelte:10450,10497,10572,11142,11216,11304,11333,13106,13170,13238,13247,14798; :11565-11566. These zero/80ms measure or panel-sync callbacks can burst during a layout event and have no universal route-teardown cancel; each still expires and is not recreated by a session effect (finding 18).
- [verified] SAFE-torn-down — tauri-svelte-preview/src/routes/+page.svelte:12091; :12244,12294,12351,12388,12428,12473,12541,12588,12648,12702. The dockview node listener is removed at :12101-12103. Each of the ten observers is installed after a host-token check and disconnected by the matching disposal functions at :12993-13070.
- [verified] LEAK-conditional — tauri-svelte-preview/src/routes/+page.svelte:13199-13201,13660-13662,13801-13803. See finding 7.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/routes/+page.svelte:14662-14664,14665-14668. beforeunload/pagehide/visibilitychange listeners are removed at :14823-14825 and the orchestration interval is cleared at :14826.
- [verified] LEAK-conditional — tauri-svelte-preview/src/routes/+page.svelte:14644-14659. See finding 6.

### Current /next route inventory

- [verified] LEAK-conditional, bounded — tauri-svelte-preview/src/routes/next/+page.svelte:685-688,752,1788. scheduleRefit coalesces to one frame, hostFor has at most 12 short retries, and the panel-load callback runs once after ShellFrame ready; none has a universal teardown cancel (finding 18).
- [verified] SAFE-torn-down — tauri-svelte-preview/src/routes/next/+page.svelte:1437-1438,1452,1586. Disposers remove drag/drop, devtools-key, and pagehide listeners at :1439-1442, :1453, and :1587.
- [verified] SAFE-torn-down — tauri-svelte-preview/src/routes/next/+page.svelte:1454-1457,1463,1594-1599. Extension observation, conversation event setup, and terminal-service disposal are registered once in onMount and disposed on route teardown; conversationService's second-listen error path remains finding 15.

## Adjacent ownedId/root retention that is not a requested timer API

### Browser session snapshots — retained per session, deletion gap

- [verified — tauri-svelte-preview/src/lib/shell/browser/browserSessionSnapshots.ts:11-27,49-61] A process-lifetime Map stores one browser/panel snapshot per ownedId, including annotations, strokes, and capture.
- [verified — tauri-svelte-preview/src/lib/shell/panels/browser/BrowserPanel.svelte:720-735,752-758] BrowserPanel writes the previous session snapshot on every ownedId change and writes the current snapshot on effect cleanup.
- [verified — tauri-svelte-preview/src/routes/next/+page.svelte:1393-1413] removeSession removes the agent/session/workspace records but never deletes the browser snapshot. This is one retained snapshot per unique ownedId, not one timer per switch.
- [verified — tauri-svelte-preview/src/lib/shell/browser/browserSessionSnapshots.ts:11-27,49-61; tauri-svelte-preview/src/routes/next/+page.svelte:1393-1413] Classification: adjacent per-session retention, with a deletion leak. [assumed — tauri-svelte-preview/src/lib/shell/browser/browserSessionSnapshots.ts:11-24] A captured screenshot or annotation stroke can make a snapshot materially larger than the surrounding session metadata.
- [verified — tauri-svelte-preview/src/lib/shell/browser/browserBackend.ts:111-115,196-199,238-256] The in-memory browser backend also keeps a captures Map after closing a tab; its fake PNG payload is small and this path is preview/test-oriented. [assumed — tauri-svelte-preview/src/lib/shell/browser/browserBackend.ts:111-115,238-256] Treat it as low-weight adjacent retention, not the source of a large production slope.

### Draft revision map — adjacent, low weight

- [verified — tauri-svelte-preview/src/lib/shell/conversation/conversationDraftPersistence.ts:14-16,70-72; tauri-svelte-preview/src/routes/next/+page.svelte:1393-1413] The per-ownedId revisions map is updated by draft persistence but removeSession does not remove the deleted id's revision entry.
- [verified — tauri-svelte-preview/src/lib/shell/conversation/conversationDraftPersistence.ts:14-16,70-72; tauri-svelte-preview/src/routes/next/+page.svelte:1393-1413] Classification: adjacent per-session retention, not a timer/listener leak. [assumed — tauri-svelte-preview/src/lib/shell/conversation/conversationDraftPersistence.ts:14-16] Each retained value is only a revision number, so this is low weight even if many sessions are deleted.

### xterm views — expected per-session retention, not a timer/listener leak

- [verified — tauri-svelte-preview/src/lib/shell/xtermFactory.ts:149-175,177-205] makeTerminalView creates one Terminal, FitAddon, SerializeAddon, optional WebGL addon, and an input callback per view.
- [verified — tauri-svelte-preview/src/lib/shell/xtermFactory.ts:160-166] The committed comment estimates roughly 20 MB per view at 120 columns and explicitly includes hidden views.
- [verified — tauri-svelte-preview/src/lib/liveConversationTerminals.ts:128-168,273-303] The manager is idempotent per key, keeps views across switches, and disposes one view on close or all views on disposeAll.
- [verified — tauri-svelte-preview/src/lib/shell/xtermFactory.ts:149-175; tauri-svelte-preview/src/lib/liveConversationTerminals.ts:128-168,273-303] Classification: expected per-session retention. It grows when new sessions/views are created and remain open, not when switching repeatedly among the same ownedIds. [assumed — tauri-svelte-preview/src/lib/shell/xtermFactory.ts:160-166] This is the most likely explanation for a roughly equal memory increment when each switch introduces another live session.

### Root-keyed editor caches — adjacent retention, not a timer/listener occurrence

- [verified — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:567-569,655-706,947-980,1204-1217] rememberedSemanticTargets stores reference-target arrays under root/file/symbol keys. A file invalidation removes only that file's prefix, and a root/active-preview change calls stopCountingForTheOldFile but does not clear the target map. The all-previews invalidation at :1230-1232 is the broad clear.
- [verified — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:751-768] rememberedReadiness also keeps a root/language entry after its three-second freshness window; expiry makes it stale, but does not delete the key.
- [verified — tauri-svelte-preview/src/lib/shell/projectSourceIndex.ts:10-23,34-42] recordsByRoot keeps every completed SourceRecord[] index for every normalized root. Normal switching deliberately retains indexes; only explicit forgetProjectSourceRecords/forgetAllProjectSourceRecords removes them.
- [verified — tauri-svelte-preview/src/lib/shell/editor/csharpLanguageClient.ts:93,264-276,362-364] browserWorkspaceRoots similarly keeps one marker per root and has no removal path. [verified — tauri-svelte-preview/src/lib/shell/editor/csharpLanguageClient.ts:448-458] The native C# client map is bounded to MAX_WARM_CSHARP_ROOTS and evicts/disposes old clients, so it is not an unbounded counterpart.
- [verified — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:567-569,751-768,947-980,1204-1217; tauri-svelte-preview/src/lib/shell/projectSourceIndex.ts:10-42] Classification: adjacent per-distinct-root/file retention, not one timer per switch. [assumed — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:655-706; tauri-svelte-preview/src/lib/shell/projectSourceIndex.ts:10-23] Reference-target arrays and SourceRecord[] indexes can be materially heavier than the tiny root markers; this is a stronger candidate for equal memory growth when each switch visits a new root or file, while repeated switches among the same keys should plateau.

## Top three fixes

1. [verified trigger — tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:593-611,947-955,1204-1214] Make semantic retries generation-aware and retain/cancel their timeout ids on root/preview change; give the reference-count batcher an equivalent cancellation path for :161. This addresses the fastest timer growth: up to three timers per failed symbol event.
2. [verified trigger — tauri-svelte-preview/src/routes/next/+page.svelte:906-910; tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:882-899,1000-1003; tauri-svelte-preview/src/lib/shell/components/editor/languageServerStatus.ts:304-314] Make releaseSessionResources stop native C# action/diagnostic callbacks and release language-server waiters before replaying the arriving session. This fixes the high-weight stale EditorPanel closure and the switch-retained gate waiter queue together.
3. [verified trigger — tauri-svelte-preview/src/lib/shell/editor/extensionApiProbeController.ts:185-225,267-280; tauri-svelte-preview/src/routes/+page.svelte:7730-7839,14641-14659] Put a generation/disposed guard around async probe/embedded-xterm setup and stop or dispose any late lease/listener before returning. This is the highest-weight conditional path because a race can retain a full xterm/PTY/addon graph or native listener. The old route Tauri/embedded fixes are lower current reachability than the first two, but they remove persistent teardown races.

[verified — tauri-svelte-preview/src/lib/shell/xtermFactory.ts:149-175; tauri-svelte-preview/src/lib/liveConversationTerminals.ts:128-168,273-303; tauri-svelte-preview/src/lib/shell/browser/browserSessionSnapshots.ts:11-27,49-61] For the reported equal memory increment per newly visited session, inspect intentional per-session xterm views and browser snapshots alongside these timer fixes. They are adjacent retention findings, not timer/listener accumulation: xterm views are expected to remain until close, while snapshots have no deletion call on removeSession.

## What is not present

- [verified — command: git grep -n -E 'new[[:space:]]+(EventSource|WebSocket)|appWindow\.listen|event\.listen' HEAD -- tauri-svelte-preview/src/lib tauri-svelte-preview/src/routes] No new EventSource, WebSocket, appWindow.listen, or event.listen occurrence exists in committed src.
- [verified — command: git grep -n -P '(?<!un)subscribe[[:space:]]*\(' HEAD -- tauri-svelte-preview/src/lib tauri-svelte-preview/src/routes] No bare subscribe( call exists when the unsubscribe prefix is excluded. The named native C# subscription at EditorPanel.svelte:544 / csharpLanguageClient.ts:646-652 was audited because it is the actual Set-based subscription despite not matching the literal spelling.
- [verified — command: git grep ... HEAD] Test-only matches at editorPanelLanguageServer.test.ts:257, editorStartup.test.ts:153, and conversationDraftPersistence.test.ts:31 do not create production timers/listeners and are excluded from runtime classifications. Type declarations such as MonacoSourceEditor.svelte:377 and SourceWorkbench.svelte:46 are also not runtime allocations.
