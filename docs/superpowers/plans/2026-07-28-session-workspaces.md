# Session Workspaces & Reversible Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make each session behave like its own task workspace — switching sessions swaps editor tabs and file-tree state, finishing a session is a reversible user action instead of a dead end, session rows carry the branch/task/PR metadata the scanner already derives, and the reference-count hot loop moves into the optimized core crate so desktop dev builds count as fast as the release benchmark promised.

**Architecture:** Two independent lanes. Lane A (frontend, primary checkout, branch `tsk-788-761-session-workspaces`): three sequential tasks over the /next shell's session rail and stores. Lane B (Rust, isolated worktree, branch `tsk-787-count-hot-loop`): one task moving the counting pass from the Tauri app crate into `mcb-core`. The lanes touch disjoint files except `core/src/lib.rs` (Lane B adds a module line; Lane A never edits it).

**Tech Stack:** Svelte 5 runes, TypeScript, Rust (mcb-core + Tauri 2 app crate), Node test scripts (`node --experimental-strip-types scripts/<name>.test.mjs`).

**Covers:** Notion tasks TSK-788 (per-session workspace context, High), TSK-761 (reversible session lifecycle — the "can't do anything with finished sessions" defect + metadata serialization prereq), TSK-787 (hot-loop move).

## Global Constraints

- The old shell `tauri-svelte-preview/src/routes/+page.svelte` is FROZEN. Never edit it. It also carries uncommitted user changes — never stage or commit that file.
- The user's `tauri dev` and vite dev server are running against the primary checkout. `src-tauri/` is watched: editing any file under `tauri-svelte-preview/src-tauri/` in the primary checkout restarts the app and kills live terminals. Lane B therefore works ONLY in its own worktree. `core/` is NOT watched — Lane A may edit `core/src/scanners/sessions.rs` in the primary checkout — but core changes reach the desktop app only after a manual tauri dev restart (note it in the report, don't restart anything yourself).
- Every commit must leave the tree consistent — vite HMR serves each commit's state to a live user mid-session.
- Stores (`.svelte.ts`): no backend calls, no `$effect`. IO lives in explicit service/page functions only. Persistence is an explicit `persist()` at the end of each mutator (see `sessionRailStore.svelte.ts`).
- All user-visible copy in plain English — no invented jargon, no internal stage names. Labels a non-programmer can read.
- Commit messages: plain, no Co-Authored-By trailer, no emoji.
- TypeScript checks: `pnpm run check` must stay clean. New pure logic gets a Node test script wired into `package.json` following the existing `scripts/*.test.mjs` pattern.
- Rust: `cargo test -- --skip lsp::` must pass (4 lsp tests + 3 script tests fail pre-existing on this machine — verify any failure you see is in that known list by checking it fails without your diff too).

---

### Task 1: Reversible session lifecycle — Done is a state, not a deletion

**The defect (user report, 2026-07-28):** an owned session whose process exits shows "finished" and the only affordance is ✕, which kills the record permanently. The user wants task-like rows: a button to complete a session, a button to revert it out of complete, and closed sessions that remain visible and actionable.

**Files:**
- Modify: `tauri-svelte-preview/src/lib/shell/ownedSessions.ts`
- Modify: `tauri-svelte-preview/src/lib/shell/stores/sessionRailStore.svelte.ts`
- Modify: `tauri-svelte-preview/src/lib/shell/components/SessionRail.svelte`
- Modify: `tauri-svelte-preview/src/routes/next/+page.svelte` (the `closeOwned` function and the props passed to `ShellSidebar`)
- Modify: `tauri-svelte-preview/src/lib/shell/components/ShellSidebar.svelte` (pass-through props only)
- Test: `tauri-svelte-preview/scripts/ownedSessions.test.mjs` (extend)

**Interfaces:**
- Produces on `OwnedSession`: `completedAt: string | null` — ISO stamp when the user marked the session complete; `null` = not complete. Set/cleared ONLY by the user actions below, never by process exit.
- Produces mutators in `sessionRailStore.svelte.ts`:
  - `completeOwnedSession(ownedId: string, when: Date): void` — sets `completedAt` to `when.toISOString()`, persists.
  - `reopenOwnedSession(ownedId: string): void` — sets `completedAt` to `null`, persists.
- Produces in `+page.svelte`, replacing the single close path:
  - `closeTerminal(ownedId)` — kills the PTY (existing `service.closeOwned` flow) and sets `state: 'exited'` via `updateOwnedSession`, but does NOT call `removeOwnedSession`. The row stays.
  - `removeSession(ownedId)` — the only path that calls `removeOwnedSession`. If the session still has a live PTY, it first runs the `closeTerminal` kill.
- `SessionRail.svelte` new props: `onComplete(ownedId)`, `onReopen(ownedId)`, `onRemove(ownedId)`; existing `onClose(ownedId)` becomes the kill-terminal-only action.

**Behavior spec:**
- The owned list renders in two subsections under the existing "Sessions" heading: **Working** (rows with `completedAt === null`) and **Done** (rows with `completedAt !== null`, collapsed by default when empty, ordered by `completedAt` descending). Reuse the existing group-heading look; labels are exactly "Working" and "Done".
- Row actions (compact icon buttons on the row, tooltips in plain words):
  - Working row, any state: "Mark done" (✓) → `onComplete`.
  - Working row with a live/background PTY: "Close the terminal" (✕) → `onClose`. The row stays in Working with the `finished` badge once the PTY dies.
  - Done row: "Reopen" (↩) → `onReopen` — the row returns to Working; if its PTY is dead it simply shows the finished badge there (resume comes later, out of scope).
  - Done row: "Remove from this list" (✕) → `onRemove` after `window.confirm('Remove "<title>" from your sessions? The transcript stays on disk.')`. Remove exists ONLY on Done rows — completing first is the deliberate two-step.
- Marking done does NOT kill the PTY (a completed session may still be running; the terminal keeps working). Closing the terminal does NOT mark done.
- `parseStoredOwnedSessions` accepts records without `completedAt` (older storage) as `completedAt: null`, and non-string values as `null`.

**Steps:**
- [ ] Extend `scripts/ownedSessions.test.mjs` with failing cases: serialize/parse round-trips `completedAt`; missing/invalid `completedAt` parses to `null`; `reconcileOwnedSessions` leaves `completedAt` untouched in all three branches.
- [ ] Run it, watch it fail. Implement the `OwnedSession` field + parse tolerance.
- [ ] Add the two store mutators (follow `updateOwnedSession`'s persist pattern exactly).
- [ ] Split `closeOwned` in `+page.svelte` into `closeTerminal` / `removeSession` per the interface block; wire the four callbacks through `ShellSidebar` to `SessionRail`.
- [ ] Rework the owned-rows section of `SessionRail.svelte` into Working/Done subsections with the actions above. Keep the project grouping inside each subsection (grouping logic in `sessionGroups.ts` is untouched — call `groupSessions` once per subsection with the pre-filtered list).
- [ ] `pnpm run check` + `node --experimental-strip-types scripts/ownedSessions.test.mjs` + the existing rail-related suites (`sessionGroups`, `panelActivation`) — all green.
- [ ] Commit: `feat: sessions complete and reopen like tasks; closing a terminal no longer erases the session`

### Task 2: Per-session workspace context (TSK-788)

**The defect:** editor and explorer are global singletons; two sessions on the same project are one shared workspace. Acceptance (from the Notion task): open file A in session 1, switch to session 2 (same repo), open file B, switch back → editor shows file A and session 1's tree expansion; session 2 still has file B when you return.

**Files:**
- Create: `tauri-svelte-preview/src/lib/shell/sessionWorkspaces.ts` (pure — no Svelte, no IO; storage handled through the `LayoutStorage` interface from `layout/layoutStorage.ts` like `sessionGroups.ts` does)
- Modify: `tauri-svelte-preview/src/routes/next/+page.svelte` (snapshot/restore wiring in `selectOwned`, cleanup in `removeSession`, final snapshot in the unmount cleanup)
- Modify: `tauri-svelte-preview/src/lib/shell/panelActivation.ts` (add `loadsAllowed(): boolean` — returns whether session loads are currently honoured)
- Test: `tauri-svelte-preview/scripts/sessionWorkspaces.test.mjs` (new, wire `test:session-workspaces` into package.json)

**Interfaces:**
- `sessionWorkspaces.ts` exports:
  ```ts
  export interface SessionWorkspaceSnapshot {
    openPaths: string[];          // editor strip order, capped at 12 (keep the MOST RECENT 12; activePath always kept)
    activePath: string | null;
    expandedFolderIds: string[];
    selectedPath: string | null;
    scrollTop: number;
  }
  export const SESSION_WORKSPACES_STORAGE_KEY = 'mac-command-bar.next.session-workspaces';
  export function captureWorkspace(input: {
    openFiles: { path: string }[]; activePath: string | null;
    expandedFolderIds: Set<string>; selectedPath: string | null; scrollTop: number;
  }): SessionWorkspaceSnapshot;
  export function readWorkspaces(storage: LayoutStorage): Record<string, SessionWorkspaceSnapshot>; // tolerant: junk → {}
  export function writeWorkspaces(storage: LayoutStorage, all: Record<string, SessionWorkspaceSnapshot>): boolean;
  export function pruneWorkspaces(all: Record<string, SessionWorkspaceSnapshot>, keepOwnedIds: string[]): Record<string, SessionWorkspaceSnapshot>;
  ```
- `panelActivation.ts` produces `loadsAllowed(): boolean` on the returned object (true once `allowSessionLoads()` has run).

**Wiring spec (`+page.svelte` — all in explicit functions, no effects):**
- Module-level `let workspaces: Record<string, SessionWorkspaceSnapshot>` hydrated once in `onMount` via `readWorkspaces(window.localStorage)`, then pruned against the reconciled owned ids and written back.
- `snapshotWorkspace(ownedId)`: read `editorState.openFiles`/`activePath` and `explorer.expandedFolderIds`/`selectedPath`/`scrollTop` through `captureWorkspace`, store under `ownedId`, `writeWorkspaces`.
- `restoreWorkspace(ownedId)`: only when `shellPanels.loadsAllowed()`. Always `resetEditorState()` first (a session with no snapshot gets an EMPTY editor — that emptiness is correct isolation, not a bug). Then, if a snapshot exists: replay `requestOpenFile({ path })` (from `openFileBus.ts`) for every `openPaths` entry in order with `activePath` last; apply explorer fields as direct store mutations (`explorer.expandedFolderIds = new Set(snapshot.expandedFolderIds)`, `selectPath(snapshot.selectedPath)`, `setScrollTop(snapshot.scrollTop)`) AFTER `shellPanels.sessionPicked()` has run — `activate(root)` no-ops on same root and `applyScanResult` never clears expansion, so the mutations survive both the same-root and changed-root cases (a changed-root scan that later drops `selectedPath` because the file is gone is correct).
- `selectOwned(ownedId)` order: if there is a previous `rail.activeOwnedId` different from `ownedId` → `snapshotWorkspace(previous)`; then the existing `setActiveOwned` / `service.show` / `shellPanels.sessionPicked()`; then `restoreWorkspace(ownedId)`.
- `removeSession(ownedId)` (from Task 1): also delete the snapshot and write.
- The `onMount` cleanup: `snapshotWorkspace(rail.activeOwnedId)` if one is active (synchronous localStorage write — allowed).
- The startup `finally` block already re-fires `sessionPicked()` for a re-attached session; add `restoreWorkspace(rail.activeOwnedId)` right after it so a reload restores the active session's tabs too.

**Steps:**
- [ ] Write `scripts/sessionWorkspaces.test.mjs` first (failing): capture caps `openPaths` at 12 keeping the most recent and always keeping `activePath`; read tolerates `null`/junk/non-object entries; prune drops unknown ids and keeps known ones; write returns false when the storage stub throws.
- [ ] Implement `sessionWorkspaces.ts`; tests green.
- [ ] Add `loadsAllowed()` to `panelActivation.ts` + a case in `scripts/panelActivation.test.mjs`.
- [ ] Wire `+page.svelte` per the spec.
- [ ] Verify by hand in the web preview (`http://localhost:5177/next`): the acceptance scenario at the top of this task, plus reload-restores-active-session.
- [ ] `pnpm run check` + full script-test sweep green.
- [ ] Commit: `feat: each session keeps its own editor tabs and file-tree state`

### Task 3: Serialize the scanner's derived metadata and show it on rows

**Why:** `derive_agent_session_metadata` (branch, task id, PR hint, source label) is computed in `core/src/scanners/sessions.rs` but never serialized, so rows can't show the chips the sessions-as-tasks design needs. The web preview has a parallel TS scanner that must stay in parity.

**Files:**
- Modify: `core/src/scanners/sessions.rs` (fields + populate + the test that pins the current serialization shape — find it by searching the test module for the serialization/shape assertions and update it; add a test that the new fields serialize when present and are omitted when `None`)
- Modify: `tauri-svelte-preview/src/lib/tauriSource.ts` (`AgentSession` type: `branchHint: string | null; taskId: string | null; pullRequestHint: string | null; sourceLabel: string | null` — all optional-tolerant)
- Modify: `tauri-svelte-preview/src/lib/server/localSourceFs.ts` (TS scanner parity: emit the same four fields; it already has the hint-extraction inputs — port `branch_hint_from_text` / `task_id_from_text` / `pull_request_hint_from_text` behavior exactly; if a helper is genuinely Rust-only, port the regex faithfully and pin it with the same fixture strings the Rust tests use)
- Modify: `tauri-svelte-preview/src/lib/shell/ownedSessions.ts` (`OwnedSession` gains `branch: string | null`, `taskId: string | null`, `pullRequest: string | null`; `adoptAgentSession` copies them; parse tolerant)
- Modify: `tauri-svelte-preview/src/lib/shell/components/SessionRail.svelte` (chips)
- Test: `tauri-svelte-preview/scripts/ownedSessions.test.mjs`, `tauri-svelte-preview/scripts/localSourceFs.test.mjs` (extend both)

**Rust field shape (on `AgentSessionRecord`, camelCase like the rest):**
```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub branch_hint: Option<String>,
#[serde(skip_serializing_if = "Option::is_none")]
pub task_id: Option<String>,
#[serde(skip_serializing_if = "Option::is_none")]
pub pull_request_hint: Option<String>,
#[serde(skip_serializing_if = "Option::is_none")]
pub source_label: Option<String>,
```
Populate them at the end of `scan_sessions` by running `derive_agent_session_metadata` over each finished record (one pass, after dedupe/sort). `source_label` wraps the existing `String` in `Some`. Every construction site of `AgentSessionRecord` initializes the four fields to `None` — let the compiler find them all.

**Row chips (both owned and available rows):** after the agent badge, in this order, each rendered only when present: branch chip (` <branch>` with a subtle border), task-id chip (`TSK-…`, uppercase as given), PR chip (`PR #N` — display the hint as provided). Chip font follows the existing `.badge` size; do NOT shrink below it (user complaint: metadata fonts too small).

**Steps:**
- [ ] Rust: add fields + populate + fix/extend the shape-pinning tests. `cargo test --manifest-path core/Cargo.toml` green.
- [ ] TS bridge parity in `localSourceFs.ts` + extend its test with the same fixtures the Rust tests use (a codex record whose text carries a branch name, a `tsk-NNN` id, a PR URL).
- [ ] `AgentSession` type + `adoptAgentSession` carry-through + tolerant parse + `ownedSessions.test.mjs` cases.
- [ ] Chips in `SessionRail.svelte`.
- [ ] `pnpm run check` + script sweep green; verify chips visually in the web preview.
- [ ] Commit: `feat: session rows show the branch, task and PR the scanner already knew`

### Task 4 (Lane B, worktree): Move the reference-count hot loop into mcb-core (TSK-787)

**Why:** the counting pass runs at opt-level 0 in dev because it lives in the app crate; `[profile.dev.package."*"]` optimizes `mcb-core` (a plain path dependency) but never the app crate itself.

**Workspace:** `git worktree add /Users/blackcolours/dev/work/worktrees/mac-command-bar/count-hot-loop -b tsk-787-count-hot-loop` from `main`. ALL work in the worktree. The controller removes the worktree after merge.

**Files:**
- Create: `core/src/reference_counts.rs` (the moved pass)
- Modify: `core/src/lib.rs` (module declaration + re-exports)
- Modify: `tauri-svelte-preview/src-tauri/src/main.rs` (delete the moved bodies; the `count_source_references` command and `count_source_references_sync` keep their exact signatures and call into `mcb_core`)

**What moves verbatim (adjust only visibility + imports):** `count_reference_lines_across_files` (main.rs:1903), `count_reference_lines`, the identifier-plan structs and plan-building helpers, the parallel worker split (≤8 threads), the deadline checks, and the fallback path through `find_case_sensitive_source_reference_column` (move it too if the reference-search path in main.rs can consume it from core; otherwise keep a thin re-export so behavior stays byte-identical). The four unit tests and the `#[ignore]` benchmark `reference_count_pass_over_a_large_project` (honours `MCB_REFERENCE_COUNT_BENCH_ROOT`) move into the core module's test block.

**Boundary rule:** core must not depend on Tauri types. The moved functions already take plain records/plans — if any moved signature mentions a Tauri or serde-command type, define the small plain struct in core and map at the command layer in main.rs.

**Steps:**
- [ ] Create worktree; `cargo test --manifest-path core/Cargo.toml` and `cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml -- --skip lsp::` green BEFORE changes (baseline).
- [ ] Move code + tests; both test suites green after.
- [ ] Timing proof: run the benchmark in the WORKTREE with the dev profile — `cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml reference_count_pass_over_a_large_project -- --ignored --nocapture` pointed at `MCB_REFERENCE_COUNT_BENCH_ROOT=/Users/blackcolours/dev/work/EdiPlatform` if the test stayed reachable from the app crate, else the equivalent core-crate invocation — and record the number. Target: within ~2x of 402ms warm. Also record the release number to prove no regression there.
- [ ] Secondary (timeboxed to ~30 min): reproduce "the code lens link takes a while to become clickable after the count renders" by reading `sourceIntelligence.ts` / the lens provider wiring; write down the cause in the report. Fix ONLY if it is a one-liner in the files already open; otherwise report it for a follow-up task.
- [ ] Commit: `perf: run the reference count in the optimized core crate` — push branch, no PR yet (controller handles it).

---

## Verification & merge

- Lane A: milestone review of the whole branch (one reviewer dispatch over the full diff), fix wave if needed, then PR `tsk-788-761-session-workspaces` → main.
- Lane B: reviewer covers it in the same milestone pass (separate diff file); PR `tsk-787-count-hot-loop` → main; controller removes the worktree immediately after merge and reports cleanup status.
- Desktop verification needs a tauri dev restart (core changes are compiled in) — hand that to the user at the end, don't restart their processes.
