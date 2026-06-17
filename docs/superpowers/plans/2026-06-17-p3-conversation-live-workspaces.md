# P3 — Conversation = Persistent Live Workspace — Implementation Plan

> **For agentic workers:** Executed **inline** (single session, live visual verification on the Tauri app — embedded PTYs only exist in Tauri, the web preview returns `null` from `startTerminalSessionFromTauri`). The work centers on the 30k-line `src/routes/+page.svelte` plus the workbench/dock model. Do not dispatch parallel build/test agents. Steps use checkbox (`- [ ]`) syntax.

**Goal (user's #1):** Each Codex/Claude/CMUX conversation in the left rail becomes a **persistent, live workspace**. Clicking a conversation opens/restores its LIVE terminal (the conversation keeps running) plus its files / git / CWD / branch / worktree / browser. Switching to another conversation shows that conversation's live workspace while the previous one stays **ALIVE** (PTY never killed on switch). Unlimited live conversations, instant switch; an explicit **Close** tears exactly one down.

**Branch:** `tsk-346-324-321-dockview-redesign` (continue; this is P3 of the redesign tracked under those task ids).

---

## What already exists (ground truth — cite before changing)

The redesign already shipped a large, *snapshot-based* conversation system. P3 is mostly **promoting it from "one reconstructed terminal + saved snapshot" to "N truly-live terminals with instant switch"** — not greenfield.

**Backend is already N-conversation-ready (the key enabler):**
- `src-tauri/src/terminal.rs:46` — `TerminalRegistry` is an `Arc<Mutex<HashMap<String, TerminalSessionHandle>>>`. Every PTY persists in this map until explicitly closed.
- `terminal.rs:57` `start_terminal_session` spawns a login shell at `cwd`, inserts into the registry, and **each session owns its own `scrollback: Arc<Mutex<String>>`** (bounded, UTF-8-safe; `terminal.rs:97,292`).
- `terminal.rs:275` the reader thread emits a **global** `terminal-output` event carrying `sessionId` for **every** session (not just the active one). `terminal.rs:320` emits `terminated`.
- `terminal.rs:220` `close_terminal_session` is the **only** path that calls `killer.kill()`. **Nothing kills a PTY on switch today.**
- So N live PTYs already coexist on the backend. The bottleneck is 100% frontend.

**Frontend is a SINGLETON terminal (the core limitation):**
- `+page.svelte:997` `let embeddedTerminal: XTermTerminal | null` and `+page.svelte:861` `let embeddedTerminalSession` — exactly **one** xterm instance and **one** active session.
- `+page.svelte:20834` — exactly **one** `<div class="embedded-terminal-host" bind:this={embeddedTerminalElement}>` (the singleton DOM host) inside the singleton `terminal` dock panel.
- `+page.svelte:8088` `attachEmbeddedTerminalSession` switches conversations by `embeddedTerminal.reset()` + replaying server scrollback (`readTerminalSessionScrollbackFromTauri`, `+page.svelte:8102`). The prior PTY survives, but its **live output while inactive is dropped**: `handleTerminalOutput` (`+page.svelte:8212`) early-returns unless `payload.sessionId === embeddedTerminalSession.sessionId`. So an inactive conversation's terminal is frozen-from-the-frontend's-view; you only re-sync via scrollback on re-attach.
- `disposeEmbeddedTerminal` (`+page.svelte:8235`) closes the active PTY on teardown.

**Dock model is a FIXED panel enum (the structural limitation):**
- `src/lib/sourceDockLayout.ts:1` — `SourceDockPanelID = 'activity' | 'editor' | 'context' | 'insights' | 'terminal' | 'browser' | 'markdown'`. One `terminal` panel id. No per-conversation panel id is representable.
- `SourceWorkbench.svelte:55-161` — the workbench builds a **fixed plan from the visible-panel set** (`createSourceWorkbenchPanelPlans`, `sourceDockviewWorkspace.ts:766`) and **fully rebuilds the Dockview whenever the panel signature changes** (`SourceWorkbench.svelte:156`). Center-runtime panels (`terminal`/`browser`/`markdown`) tab `within` the editor (`sourceDockviewWorkspace.ts:819`). Adding/removing panels = teardown + re-add of the *content host*, not the Svelte content (content is teleported via `setPanelElement`/`attachPanelElement`).

**Conversation list UI (the rail to redesign):**
- Activity rail mode `'conversations'` (`+page.svelte:15609`, `selectSourceActivityMode` at `:10780`).
- Rows: `{#each filteredConversationAgentSessions as session ...}` (`+page.svelte:16775`). Each row: provider badge, title, optional task chip, description, meta, path, file-context label, a `agent-session-focus-lane` status pill, and a `MoreHorizontal` action menu (save/restore/repair/copy plans).
- **Click handler:** the row's primary button calls `onclick={() => switchToConversationWorkspace(session)}` (`+page.svelte:16793`). Active styling: `class:active={activeWorkspaceSessionKey === workspaceSnapshotIDForAgentSession(session)}` (`+page.svelte:16782`).

**Conversation switch flow today (`switchToConversationWorkspace`, `+page.svelte:6105`):**
1. `captureActiveWorkspaceBeforeSwitch()` — snapshots the outgoing conversation (`:6032`).
2. snapshot = saved-for-session OR a fresh shell snapshot (`createAgentSessionShellSnapshot`, `:5987`).
3. `restoreConversationWorkspaceSnapshot(snapshot)` (`:6636`) — restores selected files, open tabs, view-state, **dock layout**, browser URL, then calls `restoreWorkspaceEmbeddedTerminal` (`:6761`) which **re-attaches** the saved PTY into the **singleton** xterm if it is still live, else marks "no longer live".
4. `markAgentSessionWorkspaceActive(session)` sets `activeWorkspaceSessionKey = workspaceSnapshotIDForAgentSession(session)` (`:6124`).
5. If `readiness.canResumeEmbedded`, `openWorkspaceSnapshotEmbeddedTerminal(... restoreWorkspace:false)` (`:6121`) — attaches existing or starts a new PTY (writes the resume command as keystrokes, `+page.svelte:6266` → `startEmbeddedTerminalSession` → `writeTerminalSessionFromTauri(sessionId, "...\r")`, `:8001`).

**Identity / data:**
- Conversation identity key = `${provider}:${sessionID}` via `workspaceSnapshotIDForAgentSession` (`+page.svelte:7044`). Stable, persisted (`activeWorkspaceSessionKey`, `:747`; `persistActiveWorkspaceSessionKey`, `:14617`).
- `AgentSession` TS type (`src/lib/tauriSource.ts:154`): `provider, id, title, description?, model, projectPath, lastActivity, resumeCommands`. **No branch / worktree / cwd-beyond-projectPath.**
- Rust `AgentSessionRecord` (`core/src/scanners/sessions.rs:18`): `provider, id, title, description?, model, project_path, last_activity, resume_commands`.
- `derive_agent_session_metadata` (`sessions.rs:39`) computes `branch_hint`, `task_id`, `pull_request_hint`, `link_hint`, `source_label` from title/path/resume-commands — **but `AgentSessionDerivedMetadata` is NOT serialized; the frontend never receives `branchHint`.** (`scanner_tests.rs` asserts the JSON shape stays metadata-free.) The frontend re-derives `task_id` itself (`agentSessionTaskID`); it does **not** derive a branch.

**Worktree / branch restore today:**
- Restore only sets the **project path CWD** (`activateWorkspaceSnapshotProject` → `activateProject`, `+page.svelte:6789`). **No `git switch`/`checkout`, no `git worktree add` is ever executed** anywhere in the frontend — the single `worktree add` string (`+page.svelte:6315`) is a `#`-commented line inside a *copy-to-clipboard repair plan*.
- Missing-worktree is detected (`describeWorkspaceSnapshotRestoreReadiness` kind `'missing-worktree'`, `src/lib/workspaceSnapshot.ts:226`) by comparing `snapshot.worktreePath` against a scanned `knownWorktreePaths` set; the only remedy offered is the copyable repair plan. Snapshot already carries `worktreePath` and `branch` (`workspaceSnapshot.ts:66`, set at capture in `captureWorkspaceSnapshot`, `+page.svelte:6064`).

---

## Architecture decision (the hardest part) — how to hold N live terminals

The blocker: the dock model has **one** `terminal` panel and **one** xterm. Two viable shapes:

- **Option A — N dynamic Dockview panels** (`terminal:codex:<id>` per conversation). Requires turning `SourceDockPanelID` from a fixed enum into `'terminal' | \`terminal:${string}\`` (template-literal union), teaching `normalizeSourceDockLayout`, `createSourceWorkbenchPanelPlans`, the content renderer's `isSourceDockviewPanelID` validator, persistence panel-id matching, and the rebuild-on-signature-change path to tolerate a *changing set* of terminal panels. High blast radius across `sourceDockLayout.ts` + `sourceDockviewWorkspace.ts` + every `SourceDockPanelID` switch. The workbench's full-rebuild-on-signature-change (`SourceWorkbench.svelte:156`) would thrash on every conversation open.

- **Option B — keep ONE `terminal` dock panel; hold N live xterm instances inside it, show one (RECOMMENDED).** The Dockview/panel model is untouched. Inside the existing terminal panel host, render **one persistent xterm per live conversation** (a `Map<conversationKey, TerminalView>`), all kept mounted/alive but with only the active one visible (`display:none` the rest, or a single absolutely-positioned stack). Each `TerminalView` keeps its own xterm + addons + input disposable + bound `sessionId`, and **consumes the global `terminal-output` stream for its own `sessionId` continuously** (so inactive conversations stay live, not frozen). Switching = flip which view is visible + `fit()` + focus. No PTY is touched on switch. Close = dispose that one view + `close_terminal_session`.

**Choose Option B.** It directly satisfies "PTY never killed on switch / instant switch / unlimited" with the smallest change to the proven Dockview architecture, and it fixes the "inactive output dropped" defect for free (each view listens for its own session). Option A is recorded as the future path if users later want to *tear terminals out into separate Dockview tabs side-by-side* (a P4+ concern). **Flag this as a product decision (see Open Questions).**

---

## Global Constraints

- **Desktop only.** Embedded terminals require the **Tauri** app (`node_modules/.bin/tauri dev --config src-tauri/tauri.dev.attach.conf.json`); the web preview at `http://127.0.0.1:5177` returns `null` PTYs and is only for non-terminal UI checks.
- **pnpm is broken.** Typecheck: `node_modules/.bin/svelte-kit sync && node_modules/.bin/tsc --noEmit`. Focused test: `node --experimental-strip-types scripts/<name>.test.mjs`. Rust: `cargo test -p <crate>` (one at a time — do not run parallel builds/tests).
- **Reuse, don't reinvent:** keep the `setPanelElement`/`attachPanelElement` bridge, the `workspaceSnapshot` capture/restore pipeline, `workspaceSnapshotIDForAgentSession` identity, and the `describeWorkspaceSnapshotRestoreReadiness` readiness model. P3 extends them; it does not replace them.
- **No kill-on-switch.** After this work, the only PTY-kill paths are explicit Close, `terminated` events, and full-page teardown (which may close-all by design — see Open Questions).
- **Verification = live visual proof in Tauri** (open 2-3 conversations, switch back and forth, confirm all keep producing output) + clean `tsc` + `git diff --check`. Focused tests for pure helpers only.
- **Commits only when the user asks**; branch already carries the task ids.

---

## File Structure

- **Create:** `tauri-svelte-preview/src/lib/components/ConversationList.svelte` — clean conversation-rail UI built from the existing `src/lib/components/` primitives (Badge, Chip, IconButton, Tooltip, ContextMenu, SearchInput). Presentational: takes `sessions`, `activeKey`, `liveKeys`, and emits `open`/`close`/`save`/`restore`/`repair`/`copy` callbacks. No business logic.
- **Create:** `tauri-svelte-preview/src/lib/liveConversationTerminals.ts` — pure, testable model + factory for the **multi-terminal manager**: `createLiveConversationTerminals()` returning `{ ensureView(key, opts), showView(key), closeView(key), hasView(key), liveKeys(), dispose() }`, plus the pure helpers (`conversationTerminalViewKey`, ordering, teardown bookkeeping). xterm construction is injected so it can be unit-tested without a DOM.
- **Create:** `tauri-svelte-preview/scripts/liveConversationTerminals.test.mjs` — focused tests for the manager's pure bookkeeping (add/show/close/liveKeys, no-double-create, close-removes-only-one).
- **Create:** `tauri-svelte-preview/src/lib/conversationWorkspaceRestore.ts` — pure planner: given an `AgentSession` + its snapshot + the **scanned worktree set** + the resolved branch, produce a `ConversationRestorePlan` = `{ cwd, branch, worktreePath, missingWorktree, preCommands: string[], resumeCommand }`. This is where "act on branch_hint" and "repair path for missing worktree" become deterministic and unit-tested.
- **Create:** `tauri-svelte-preview/scripts/conversationWorkspaceRestore.test.mjs` — tests the planner (branch present → `git switch` precommand; worktree present → cwd = worktreePath; worktree missing → `missingWorktree:true` + repair precommand; no branch → no switch).
- **Modify:** `core/src/scanners/sessions.rs` — serialize the derived branch hint to the frontend (smallest viable: add an optional `branch_hint` field to `AgentSessionRecord` populated from `derive_agent_session_metadata`, OR a sibling serialized struct). Update `core/tests/scanner_tests.rs` (the JSON-shape test currently asserts NO `branchHint`).
- **Modify:** `tauri-svelte-preview/src/lib/tauriSource.ts` — add `branchHint?: string | null` to the `AgentSession` type.
- **Modify:** `tauri-svelte-preview/src/routes/+page.svelte` — replace the inline conversation `{#each}` block with `<ConversationList>`; replace the singleton xterm wiring (`embeddedTerminal*`) with the `liveConversationTerminals` manager driven off the terminal panel host; rework `switchToConversationWorkspace` / `attachEmbeddedTerminalSession` / `closeEmbeddedTerminalSession` / `handleTerminalOutput` / `disposeEmbeddedTerminal` to be conversation-keyed; add active-conversation tracking; wire the restore planner.

---

## Task 1: Serialize the branch hint to the frontend

**Files:** `core/src/scanners/sessions.rs`, `core/tests/scanner_tests.rs`, `tauri-svelte-preview/src/lib/tauriSource.ts`

**Interfaces:** `AgentSession` gains `branchHint?: string | null` end-to-end.

- [ ] **Step 1:** In `sessions.rs`, add `pub branch_hint: Option<String>` to `AgentSessionRecord` with `#[serde(skip_serializing_if = "Option::is_none", rename_all)]` semantics; populate it in `scan_sessions` (after `merge_agent_session_records`) by running the existing `branch_hint_from_text` over each record (reuse `first_agent_session_hint`). Do not change the other derived fields' privacy.
- [ ] **Step 2:** Update `scanner_tests.rs::agent_session_record_json_shape_stays_unchanged` — it currently asserts `value.get("branchHint") == None`; change to assert the field is present only when a branch is detected, absent otherwise. Add a case where the title carries `branch: feat/x` and assert `branchHint == "feat/x"`.
- [ ] **Step 3:** In `tauriSource.ts`, add `branchHint?: string | null;` to `AgentSession`.
- [ ] **Step 4 (verify):** `cargo test -p core scanners::sessions` (or the crate's test cmd) PASS. `tsc --noEmit` clean.

---

## Task 2: Conversation restore planner (CWD + branch + worktree)

**Files:** `src/lib/conversationWorkspaceRestore.ts`, `scripts/conversationWorkspaceRestore.test.mjs`

**Interfaces:**
- Produces `planConversationRestore(input): ConversationRestorePlan` where `input = { session, snapshot, knownWorktreePaths, scannedBranchForPath?, resumeCommand }` and `ConversationRestorePlan = { cwd: string; branch: string | null; worktreePath: string | null; missingWorktree: boolean; preCommands: string[]; resumeCommand: string }`.
- Consumes: nothing app-stateful — pure functions over strings/paths (mirror `workspaceSnapshot.ts` style; reuse its path normalization helpers).

- [ ] **Step 1:** Implement the planner:
  - `cwd` = `snapshot.worktreePath ?? session.projectPath ?? snapshot.cwd` (worktree wins when present).
  - `branch` = `session.branchHint ?? snapshot.branch ?? null`.
  - `worktreePath` = `snapshot.worktreePath ?? null`; `missingWorktree` = `worktreePath` set AND not in `knownWorktreePaths` (reuse the readiness logic's normalization).
  - `preCommands`: if `missingWorktree` → `[]` (do NOT auto-mutate git; restore files only and surface the repair plan — see Open Questions). Else if `branch` and it differs from the path's current branch (when known) → `["git -C <cwd> switch <branch> 2>/dev/null || git -C <cwd> checkout <branch>"]` (shell-quoted via the existing `shellQuoteForCommand` contract; keep it idempotent + non-fatal). Always end with a `cd <cwd>` only if the shell didn't already start there.
  - `resumeCommand` = the passed resume command (already color-enved upstream).
- [ ] **Step 2:** Tests in `conversationWorkspaceRestore.test.mjs`: (a) worktree present & known → `cwd === worktreePath`, `missingWorktree:false`; (b) worktree missing → `missingWorktree:true`, `preCommands === []`; (c) `branchHint` present, different → `git switch` precommand; (d) no branch anywhere → no switch precommand; (e) branch equals scanned current branch → no switch.
- [ ] **Step 3 (verify):** `node --experimental-strip-types scripts/conversationWorkspaceRestore.test.mjs` PASS. `tsc --noEmit` clean.

---

## Task 3: Live multi-terminal manager (keep N PTYs alive, show one)

**Files:** `src/lib/liveConversationTerminals.ts`, `scripts/liveConversationTerminals.test.mjs`

**Interfaces:**
- `createLiveConversationTerminals(deps)` where `deps` injects `{ createTerminal, writeSession, resizeSession, readScrollback }` (so the manager is DOM-free for tests; in the app these are the xterm ctor + `*TerminalSessionFromTauri` wrappers).
- Returns `{ ensureView(key, { host, sessionId, cwd }), showView(key), activeKey(), liveKeys(), bindSession(key, sessionId), feed(key, data), markTerminated(key), closeView(key), disposeAll() }`.
- Each `TerminalView` owns: its xterm + fit/search/serialize/webgl addons (lazy, mirror `ensureEmbeddedTerminalRenderer`, `+page.svelte:7830`), its bound `sessionId`, its `onData` input disposable (writes to its own session), and an `isVisible` flag.

- [ ] **Step 1:** Implement the manager. `ensureView` creates a view + its own detached xterm container element (a child `<div>` the manager owns) appended into the single terminal-panel host, lazily building the xterm (reuse the exact addon set + Dracula theme from `+page.svelte:7865-7945`). All views stay mounted; visibility is toggled by adding/removing `hidden` on each view's container (`display:none`). `showView(key)` un-hides one, hides the rest, calls `fit()` + `focus()` + `resizeSession`. `feed(key,data)` writes to that view's xterm regardless of visibility (this is what keeps inactive conversations live). `closeView` disposes one xterm + removes its container; never touches others.
- [ ] **Step 2:** Pure bookkeeping tests (inject a fake `createTerminal` returning a spy): ensureView twice for same key → one view; showView flips `activeKey`; feed to a non-visible key still calls that view's write; closeView removes only that key from `liveKeys()`; disposeAll empties.
- [ ] **Step 3 (verify):** `node --experimental-strip-types scripts/liveConversationTerminals.test.mjs` PASS. `tsc --noEmit` clean. (No app wiring yet.)

---

## Task 4: Wire the manager into the terminal panel (replace the singleton)

**Files:** `src/routes/+page.svelte`

**Interfaces:** the `terminal` dock panel now hosts the manager's view-stack instead of one xterm; output routing becomes conversation-keyed.

- [ ] **Step 1:** Replace the singleton state (`embeddedTerminal`, `embeddedTerminalFitAddon`, …, `+page.svelte:997-1008`) with `let liveTerminals = createLiveConversationTerminals(...)`. Keep `embeddedTerminalSessions` (the backend session list) and `embeddedTerminalSession` *as a derived "active session"* for the existing toolbar/labels, but source it from `liveTerminals.activeKey()`. The single host `<div class="embedded-terminal-host">` (`:20834`) becomes the manager's mount root (it appends per-conversation child containers into it).
- [ ] **Step 2:** Rewrite `handleTerminalOutput` (`:8212`): look up the conversation key owning `payload.sessionId` (maintain a `Map<sessionId, conversationKey>` populated when a view binds a session) and call `liveTerminals.feed(key, payload.data)`; on `payload.terminated` call `liveTerminals.markTerminated(key)` and drop from `embeddedTerminalSessions`. **Remove the `!== active` early-return** — every live session now renders into its own view.
- [ ] **Step 3:** Rewrite `attachEmbeddedTerminalSession`/`startEmbeddedTerminalSession` call sites used by conversation switching to go through `liveTerminals.ensureView(conversationKey, …)` + `bindSession` + (on first attach) replay `readScrollback` into that view once. New PTYs (`startTerminalSessionFromTauri`) bind their `sessionId` to the conversation key. Keep `disposeEmbeddedTerminal` (`:8235`) for full-page teardown but route it to `liveTerminals.disposeAll()` (decide close-all-on-unload per Open Questions).
- [ ] **Step 4 (verify live, Tauri):** Open conversation A, start its terminal, run `while true; do date; sleep 1; done`. Open conversation B (its own terminal). Switch back to A — A's clock kept ticking (output continued while hidden), B stays alive. `tsc --noEmit` clean.

---

## Task 5: Conversation-keyed switch + active tracking + restore wiring

**Files:** `src/routes/+page.svelte`

- [ ] **Step 1:** Rework `switchToConversationWorkspace` (`:6105`):
  1. `captureActiveWorkspaceBeforeSwitch()` (unchanged — snapshots outgoing).
  2. Compute `key = workspaceSnapshotIDForAgentSession(session)`; `markAgentSessionWorkspaceActive(session)` (sets `activeWorkspaceSessionKey`).
  3. `restoreConversationWorkspaceSnapshot(snapshot)` for files/git/tabs/browser/dock — **but** make its terminal step (`restoreWorkspaceEmbeddedTerminal`, `:6761`) conversation-keyed (Task 4) instead of re-attaching the singleton.
  4. If `liveTerminals.hasView(key)` → `liveTerminals.showView(key)` (INSTANT — no PTY work). Else build the restore plan (Task 2: `planConversationRestore`), `liveTerminals.ensureView(key,…)`, then if a saved PTY for this conversation is still live (`embeddedTerminalSessionForSnapshot`, `:6923`) attach it; else start a fresh PTY at `plan.cwd`, write `plan.preCommands` then `plan.resumeCommand` as keystrokes, and `bindSession`.
  5. If `plan.missingWorktree` → restore files only, `showView` a shell, and set `fileActionStatus` to the existing "repair worktree before terminal resume" message (reuse `copyAgentSessionMissingWorktreeRepairPlan`, `:6625`).
- [ ] **Step 2:** Active tracking: `activeWorkspaceSessionKey` already exists, is persisted (`:14617`), and drives row `class:active` (`:16782`). Confirm it now also equals `liveTerminals.activeKey()` after a switch; persist on every switch (already done in `markAgentSessionWorkspaceActive`). On startup, if a stored active key maps to a still-live backend session (`list_terminal_sessions`), auto-`ensureView`+`showView` it.
- [ ] **Step 3:** Implement explicit **Close** for a conversation workspace: a new `closeConversationWorkspace(session)` = `liveTerminals.closeView(key)` + `close_terminal_session(boundSessionId)` + clear `activeWorkspaceSessionKey` if it was active + leave the snapshot (so it can be reopened cold). This is the ONLY kill path from the rail.
- [ ] **Step 4 (verify live, Tauri):** Switch A→B→A repeatedly: instant, both stay live. Close A: only A's PTY dies (`list_terminal_sessions` no longer lists it), B unaffected. Reload the app: the previously-active live conversation re-attaches its still-running PTY. `tsc --noEmit` clean.

---

## Task 6: Clean conversation-list UI

**Files:** `src/lib/components/ConversationList.svelte`, `src/routes/+page.svelte`

- [ ] **Step 1:** Build `ConversationList.svelte` from existing primitives in `src/lib/components/` (Badge, Chip, IconButton, Tooltip, ContextMenu). Props: `sessions: AgentSession[]`, `activeKey: string | null`, `liveKeys: Set<string>`, `keyFor(session)`, and event callbacks (`onOpen`, `onClose`, `onSave`, `onRestore`, `onRepair`, `onCopyResume`). Each row shows: provider badge, title + task chip, a **LIVE** indicator when `liveKeys.has(key)` (green dot using `--color-live`), the focus-lane status pill, path/meta, and an overflow `ContextMenu` (move the existing save/restore/repair/copy actions there). Active row uses `--color-accent`. Use the token system (`tokens.css`) — borderless, spacious, modern.
- [ ] **Step 2:** In `+page.svelte`, replace the inline `{#each filteredConversationAgentSessions …}` block (`:16772-` through the row markup) with `<ConversationList sessions={filteredConversationAgentSessions} activeKey={activeWorkspaceSessionKey} liveKeys={liveConversationKeys} keyFor={workspaceSnapshotIDForAgentSession} onOpen={switchToConversationWorkspace} onClose={closeConversationWorkspace} … />`. Add a derived `liveConversationKeys = $derived(new Set(liveTerminals.liveKeys()))`. Keep the command-palette entries pointing at the same handlers.
- [ ] **Step 3 (verify live):** Web preview + Tauri: rail renders cleanly; LIVE dot appears only for conversations with a live view; active row highlighted; overflow menu actions work; click opens/switches. `tsc --noEmit` clean; `git diff --check` clean.

---

## Self-Review

- **Spec coverage:** (a) clean list UI = Task 6; (b) keep-N-PTYs-alive + each conversation its own live view, no kill-on-switch = Tasks 3+4; (c) active tracking + instant switch = Task 5; (d) restore CWD+repo+worktree+branch (act on `branchHint`, detect missing worktree) = Tasks 1+2+5; (e) per-active explorer/git = already delivered by `restoreConversationWorkspaceSnapshot` (files/tabs/git/dock/browser), reused in Task 5.
- **Hardest parts (flagged in-plan):** the N-terminal architecture decision (Option B chosen — one panel, N live xterm views — to avoid turning `SourceDockPanelID` into a dynamic union and thrashing the workbench's rebuild-on-signature-change); making inactive terminals stay live by removing the `handleTerminalOutput` active-only filter and feeding every view from the global `terminal-output` stream.
- **Where the architecture is extended:** frontend only — `terminal.rs` is already N-ready. The dock model is left intact (one `terminal` panel). The xterm singleton is replaced by a keyed manager. `AgentSession` gains `branchHint`.
- **Reuse:** snapshot capture/restore, readiness model, identity key, the `setPanelElement` bridge, addon/theme setup, repair-plan copy — all reused, not rebuilt.
- **Consistency:** `conversationKey = workspaceSnapshotIDForAgentSession(session) = ${provider}:${sessionID}` used as the single identity across the manager, active tracking, snapshots, and the list UI.

---

## Open Questions / Product Decisions (resolve before/with the user)

1. **N-terminal shape (Option A vs B).** Plan picks **B** (one Dockview terminal panel, N live xterm views, show one). Option A (a separate Dockview tab per conversation, so two live terminals could be viewed side-by-side) is a bigger rewrite of `SourceDockPanelID`/the workbench and is deferred. **Confirm B is acceptable** (you can switch instantly and unlimited, but you see one terminal at a time).
2. **Auto-mutate git on restore?** This plan does **NOT** auto-run `git worktree add` for a missing worktree (destructive / disk-critical per the worktree rules) — it restores files and surfaces the existing repair plan. For an *existing* path with a `branchHint`, it runs a non-fatal idempotent `git switch || checkout` precommand. **Confirm** you want even that auto-switch, or whether branch changes should also be copy-plan-only.
3. **Teardown / unbounded PTYs.** Keeping every conversation's PTY alive means closing the app (or `disposeAll`) must decide: kill all live PTYs on unload (clean, matches the disk/RAM-hygiene rules), or leave them running for the next launch to re-attach. Plan assumes **re-attach on next launch** (PTYs survive reload) but **kill-all on real app quit**; and there is **no cap** on concurrent live conversations — consider a soft limit / LRU-evict-to-snapshot to bound RAM. **Confirm the lifecycle + whether a cap is wanted.**
