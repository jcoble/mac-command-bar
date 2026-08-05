# TSK-808 A3-terminal receipt

## Outcome

A3-terminal is implemented on branch `tsk-808-assembly-wave` from base commit `d68c238`. The legacy monolithic transcript reader is split by provider, and terminal-backed conversations can now register one Rust-owned incremental transcript watcher that continues emitting canonical `AgentEvent` records whether its session is visible or hidden. No frontend polling code was changed in this lane; the exact controller patch is included below.

No commit was created.

## Durable projection mapping

| Durable transcript evidence | Canonical event | Projection rule |
|---|---|---|
| Codex or Claude completed user/assistant message | `item.completed` | Emits a typed `user-message` or `assistant-message` item with `historical: true`. |
| Codex turn context or Claude message config fields | `session.config.updated` | Emits only the model, effort, and approval-policy values present in the durable line, marked historical. |
| Codex token-count or Claude usage fields | `usage.updated` | Emits only durable token/window values, marked historical. |
| Codex sub-agent activity | `children.updated` | Completed/failed evidence keeps that terminal state; all other activity is projected as `historical`, never active. |
| Approval, turn-start, in-progress tool, or other ambiguous lifecycle frame | no event | The projection does not guess a live approval, active turn, or tool state. |

Every emitted event uses `agent-conversation-event`, the public canonical `AgentEvent` wire shape, a redacted raw-frame reference, monotonic per-registration sequence, and a new generation when a watcher is replaced or re-registered.

## Owned files and symbols

- `tauri-svelte-preview/src-tauri/src/agent_conversation/transcript.rs` — removed; its public snapshot command compatibility and three baseline tests moved into the module split.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/transcript/mod.rs:55` — `FileIdentity`; `:61` — `TranscriptLocation`; `:102` — provider discovery; `:114` — cached-path inspection; `:152` — complete-line/partial-line splitting; `:290` — preserved baseline transcript tests.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/transcript/codex.rs:13` — Codex transcript discovery; `:46` — durable-line projection; `:189` — bounded compatibility snapshot.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/transcript/claude.rs:13` — Claude transcript/archive discovery; `:36` — durable-line projection; `:135` — bounded compatibility snapshot.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/terminal_projection.rs:48` — cached cursor; `:59` — incremental projector; `:74` — cached-path poll; `:137` — bounded reconciliation; `:194` — watcher registry; `:304` — app-state drop cleanup; `:318` — canonical event construction; `:427-644` — terminal-projection fixture and cleanup tests.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:7` — projection module; `:108` — start command; `:117` — explicit stop command.
- `tauri-svelte-preview/src-tauri/src/main.rs:5180` — managed projection registry; `:5267` and `:5268` — command registration.
- `tauri-svelte-preview/scripts/agentConversationTerminalProjection.test.mjs:1` — public event/source contract test.
- `tauri-svelte-preview/src-tauri/fixtures/agent_conversation/terminal_projection/` — owned redacted fixtures listed below.
- `docs/superpowers/evidence/tsk-808/a3-terminal-receipt.md:1` — this return receipt.

The concurrent edits in `providers/adapter_support.rs`, `providers/codex.rs`, and `providers/claude.rs` are not owned by A3-terminal and were preserved. No frontend source file was modified.

## Fixture list

- `codex_append.jsonl` — one complete durable Codex assistant item for initial and append-only reads.
- `codex_partial.jsonl` — an intentionally incomplete final Codex line for offset/partial-line resume.
- `claude_append.jsonl` — a durable Claude assistant item with model and usage evidence.
- `no_invented_state.jsonl` — historical turn, approval, and in-progress tool frames plus one durable message; only the message may project.
- Temporary test files created under the system temp directory cover truncation, inode rotation, same-inode archive move, an over-limit read gap, duplicate reconciliation, and the 512-event reconciliation cap. Every test removes its temporary directory.

## Tests and results

### Required Rust verification — PASS

Command:

```text
RUST_TEST_THREADS=1 cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml agent_conversation -- --nocapture
```

Fresh result: `41 passed; 0 failed; 0 ignored; 164 filtered out`.

This preserves all 32 baseline tests, including the three original transcript test names, and adds these 9 A3-terminal tests:

1. `append_only_parse_reads_only_new_durable_lines` — PASS
2. `offset_and_partial_line_resume_after_append` — PASS
3. `detects_truncation_rotation_and_archive_move` — PASS
4. `detects_incremental_gap_and_uses_bounded_reconciliation` — PASS
5. `suppresses_duplicates_during_reconciliation` — PASS
6. `bounded_reconciliation_caps_emitted_records` — PASS
7. `claude_fixture_projects_only_completed_historical_messages` — PASS
8. `no_invented_live_state_from_historical_frames` — PASS
9. `registry_drop_stops_and_joins_every_watcher` — PASS

The command printed 21 existing dead-code warnings and no errors.

### Required event-contract verification — PASS

Command:

```text
node --experimental-strip-types tauri-svelte-preview/scripts/agentConversationTerminalProjection.test.mjs
```

Fresh result: `agentConversationTerminalProjection contract: PASS`.

The contract proves the module split, cached path/identity/offset/partial line, all reconciliation reasons and bounds, the canonical event channel, explicit stop/drop cleanup, the absence of PTY/process creation, valid durable JSONL fixtures, and the absence of production live turn/approval/active-item projection.

`git diff --check` also passed. The three named frontend files remained clean.

## Frontend controller receipt

Apply this unified diff only after accepting the Rust evidence above. It removes the 500 ms whole-tail mirror, registers the Rust watcher once per terminal-backed session, applies the returned bounded event snapshot, keeps watchers alive across visible-session switches, and stops them on session dismissal or conversation-event teardown.

```diff
--- a/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts
+++ b/tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts
@@ -4,7 +4,6 @@
   applyAgentConversationEvent,
   applyAgentConversationSnapshot,
   applyChildConversationTranscript,
-  applyConversationTranscript,
   ensureConversationSession,
   getConversationSession,
   setConversationConnection,
@@ -33,29 +32,12 @@
 
 let unlisten: UnlistenFn | null = null;
 const resyncing = new Map<string, Promise<void>>();
-const transcriptMirrors = new Map<string, ReturnType<typeof setInterval>>();
+const terminalProjections = new Map<string, string>();
 
-async function refreshTranscript(
-  ownedId: string,
-  provider: AgentConversationProvider,
-  nativeSessionId: string
-): Promise<void> {
-  const snapshot = await invoke<ConversationTranscriptSnapshot>('read_agent_conversation_transcript', {
-    provider,
-    nativeSessionId,
-    childSessionId: null
-  });
-  applyConversationTranscript(ownedId, provider, snapshot);
-  const childSessionId = getConversationSession(ownedId)?.selectedChildId;
-  if (childSessionId) {
-    const child = await invoke<ConversationTranscriptSnapshot>('read_agent_conversation_transcript', {
-      provider,
-      nativeSessionId,
-      childSessionId
-    });
-    applyChildConversationTranscript(ownedId, childSessionId, child.messages);
-  }
-}
+type TerminalProjectionRegistration = {
+  generation: number;
+  events: AgentEvent[];
+};
 
 export async function readChildConversationTranscript(input: {
   ownedId: string;
@@ -254,29 +236,28 @@
   }
 }
 
-export function startConversationTranscriptMirror(input: {
+export function startConversationTerminalProjection(input: {
   ownedId: string;
   provider: AgentConversationProvider;
   nativeSessionId: string;
 }): void {
-  if (!isTauri() || transcriptMirrors.has(input.ownedId)) return;
-  const refresh = (): void => {
-    void refreshTranscript(input.ownedId, input.provider, input.nativeSessionId).catch(() => {
-      // The transcript may not exist until Claude accepts its first prompt.
-      // The next poll retries without breaking the conversation surface.
-    });
-  };
-  refresh();
-  const timer = setInterval(() => {
-    refresh();
-  }, 500);
-  transcriptMirrors.set(input.ownedId, timer);
+  if (!isTauri()) return;
+  const signature = `${input.provider}:${input.nativeSessionId}`;
+  if (terminalProjections.get(input.ownedId) === signature) return;
+  terminalProjections.set(input.ownedId, signature);
+  const generation = getConversationSession(input.ownedId)?.generation ?? 0;
+  void invoke<TerminalProjectionRegistration>('start_agent_conversation_terminal_projection', {
+    request: { ...input, generation }
+  }).then((registration) => {
+    for (const event of registration.events) applyAgentConversationEvent(event);
+  }).catch(() => {
+    if (terminalProjections.get(input.ownedId) === signature) terminalProjections.delete(input.ownedId);
+  });
 }
 
-export function stopConversationTranscriptMirror(ownedId: string): void {
-  const timer = transcriptMirrors.get(ownedId);
-  if (timer) clearInterval(timer);
-  transcriptMirrors.delete(ownedId);
+export function stopConversationTerminalProjection(ownedId: string): void {
+  if (!terminalProjections.delete(ownedId) || !isTauri()) return;
+  void invoke<boolean>('stop_agent_conversation_terminal_projection', { ownedId });
 }
 
 async function resyncConversation(ownedId: string): Promise<void> {
@@ -315,7 +296,7 @@
 export function stopConversationEvents(): void {
   unlisten?.();
   unlisten = null;
-  for (const ownedId of transcriptMirrors.keys()) stopConversationTranscriptMirror(ownedId);
+  for (const ownedId of [...terminalProjections.keys()]) stopConversationTerminalProjection(ownedId);
 }
 
 export async function closeStructuredConversation(ownedId: string): Promise<void> {
--- a/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte
+++ b/tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte
@@ -25,7 +25,7 @@
     saveConversationClipboardImage,
     sendStructuredMessage,
     setConversationConfigOption,
-    startConversationTranscriptMirror
+    startConversationTerminalProjection
   } from '$lib/shell/conversation/conversationService';
   import {
     filterConversationCommandCatalog,
@@ -104,7 +104,7 @@
 
   $effect(() => {
     if (structured && (active?.agent === 'claude' || active?.agent === 'codex') && active.nativeSessionId && active.ptySessionId) {
-      startConversationTranscriptMirror({ ownedId: active.ownedId, provider: active.agent, nativeSessionId: active.nativeSessionId });
+      startConversationTerminalProjection({ ownedId: active.ownedId, provider: active.agent, nativeSessionId: active.nativeSessionId });
     }
   });
 
--- a/tauri-svelte-preview/src/routes/next/+page.svelte
+++ b/tauri-svelte-preview/src/routes/next/+page.svelte
@@ -50,8 +50,8 @@
     closeStructuredConversation,
     ensureStructuredConversation,
     startConversationEvents,
-    startConversationTranscriptMirror,
-    stopConversationTranscriptMirror,
+    startConversationTerminalProjection,
+    stopConversationTerminalProjection,
     stopConversationEvents
   } from '$lib/shell/conversation/conversationService';
   import type { AgentConversationProvider } from '$lib/shell/conversation/conversationTypes';
@@ -507,7 +507,6 @@
     // session actually had (rows are clickable for seconds while the first
     // scan runs — including the close button, which switches sessions too).
     if (switching && previous !== null && shellPanels.loadsAllowed()) snapshotWorkspace(previous);
-    if (switching && previous !== null) stopConversationTranscriptMirror(previous);
     if (switching && selected) {
       const root = selected.cwd.trim() || (selected.projectPath ?? '').trim();
       if (root) await setExtensionApiProbeWorkspace({ ownedId: selected.ownedId, root });
@@ -532,7 +531,7 @@
         void closeStructuredConversation(ownedId);
         if (selected.nativeSessionId) {
           setConversationMode(ownedId, 'structured');
-          startConversationTranscriptMirror({
+          startConversationTerminalProjection({
             ownedId,
             provider,
             nativeSessionId: selected.nativeSessionId
@@ -831,7 +830,7 @@
     awaitingReattach.delete(ownedId);
     removeOwnedSession(ownedId);
     removeConversationSession(ownedId);
-    stopConversationTranscriptMirror(ownedId);
+    stopConversationTerminalProjection(ownedId);
     // A removed row takes its stack tag with it, rather than leaving one
     // pointing at a session that is gone.
     noteSessionRemoved(ownedId);
```

The on-demand `readChildConversationTranscript` command remains. It is invoked only when a person selects a child and is not a polling loop.

## Cleanup and native evidence

- No PTY was spawned, and neither `terminal.rs` nor `terminalService.ts` was changed.
- No network or browser session was used.
- A watcher owns a stop channel and join handle. Replacement stops and joins the prior watcher; explicit dismissal stops and joins it; `TerminalProjectionRegistry::drop` stops and joins every remaining watcher on app-state teardown.
- `registry_drop_stops_and_joins_every_watcher` proves the cleanup path. The test process exited normally, so no test watcher remains.
- The sandbox denied process-table inspection (`ps: operation not permitted`); no runtime watcher was actually registered during this lane, and the cleanup proof is the joined-thread fixture plus normal test-process exit.
- Worktree cleanup: not removed — `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave`, branch `tsk-808-assembly-wave`, is the controller-provided active worktree at base `d68c238`, not a disposable worktree created by this lane.

## Limitations

- Transcript records cannot reliably distinguish current live lifecycle state from historical state for approvals, turn starts, and in-progress tools. Those frame kinds are not emitted. Nonterminal child activity is labeled `historical`, never `active`.
- Reconciliation is deliberately bounded to the final 4 MiB and at most 512 newly emitted records. Older history outside that window is not invented or loaded.
- The current frontend canonical reducer renders projected completed items. It does not yet materialize terminal `session.config.updated`, `usage.updated`, or `children.updated` payloads into the legacy metadata/children fields; Rust emits those canonical events for the controller/A4 reducer, while the A3 receipt stays within the three authorized frontend files.

## Deviations

No deviations.
