# CommandBar Master Plan — Data-Spine → Consolidation → Live Agent Sessions

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development, phase-by-phase. Steps use checkbox (`- [ ]`).
>
> **Built for maximum parallelism.** Every task carries a `Files · Consumes · Produces · Depends` tuple so a controller can fan out all file-disjoint tasks with no upstream dependency at once, and serialize only true conflicts. The **only** deferred detail is exact line anchors — re-grep them at task start, because the file shifts as earlier tasks land (hard-coding them would be stale, not detailed). Behavior detail for Phase C lives in the spec (`docs/superpowers/specs/2026-06-17-live-agent-sessions-design.md`, §-cited).

**Goal:** Make `+page.svelte` (26,568 lines post-0b) a thin orchestrator by extracting the files/project data-spine + the remaining coupled subsystems, consolidate LSP onto a single Rust-backed path, fix git/worktree, then build the headline **Live Agent Sessions** core.

**Architecture:** Continue the proven Phase-0b pattern — rune-state modules (`export const store = $state({...})` in `.svelte.ts`, mirror `settingsStore.svelte.ts`; `$effect`s stay in components) + presentational `.svelte` components fed props/callbacks. Data-spine first (`filesStore`/`projectStore`) so the deferred editor/insights/git extractions become file-disjoint parallel lanes. Then the live-session core per spec (backend `terminal.rs` first; one terminal surface, N live xterm views; 3-state lifecycle; Available→Owned rail).

**Tech stack:** SvelteKit + Svelte 5 runes, TS, Dockview teleport bridge, Tauri 2 (Rust: `portable_pty`, `lsp.rs`, `terminal.rs`), Monaco, xterm.js (WebGL + SerializeAddon).

## Global Constraints (bind every task)
- **Extraction = MOVE, not rewrite.** Behavior-identical; source region DELETED (never duplicated).
- **State-module pattern:** `export const <store> = $state({...})` in `.svelte.ts`; read/write `store.x`; `.svelte.ts` **can't host `$effect`** (effects/refs/timers stay in components, moved together).
- **Teleport bridge stays in `+page.svelte`:** `<SourceWorkbench bind:setPanelElement={unifiedWorkbenchSetPanelElement}>`, the 5 `sourceDockviewPanelAction` fns + `register/syncSourceDockviewPanelElement`, the 11 workspace handles. Extracted panels take the action as a `panelAction` prop. A lost action = blank panel.
- **CSS migration mandatory:** scoped rules → component; ancestor-combo rules (page-rendered ancestor) → `app.css :global`; shared bases = ONE `:global` def in `app.css`; no orphaned `.<panel>-*` in the page `<style>`.
- **Verify LIVE before commit:** `svelte-kit sync && tsc --noEmit` clean + `playwright-cli` screenshot of the surface rendering correctly + `console error` = 0. tsc misses `.svelte`/CSS regressions — screenshot non-negotiable. Live-visual proof, not test sprawl; focused unit tests only for pure helpers. Green commit per task.
- **Everything on Rust where reasonable** (core principle): IDE/LSP/terminal route through Rust/Tauri; Monaco LSP providers already hit `find_source_lsp_*`→`lsp.rs`; don't add JS-side duplicates.
- **Git safety:** never auto-create/destroy worktrees; never `cd`/`git switch`/`worktree add` in a backgrounded PTY; missing worktree → copyable repair plan.
- **Process:** opus subagents; branch `tsk-346-324-321-dockview-redesign`; commit footer `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

## SERIALIZATION INVARIANTS (the only limits on fan-out)
1. **ONE agent edits `+page.svelte` AND `src/app.css` at a time** — author new files in parallel; the per-task *integration* step (which touches `+page.svelte` and/or `app.css` — every panel extraction migrates CSS into the shared `app.css :global` bases) is serialized through a single integrator (any order). [`app.css` is a real shared surface across B1.1/B1.2/B1.3, not just B1.1.]
2. **ONE git committer at a time** on the branch (concurrent commits corrupt the index — observed).
3. **Cargo builds serialized** — `terminal.rs` and `lsp.rs` share one Rust build; never two `cargo` at once (RAM).
4. Everything else fans out: disjoint new `.svelte`/`.svelte.ts` files, disjoint Rust modules (authored, build-gated), disjoint docs.

## Dependency graph (fan out everything with no inbound arrow)
```
PHASE A (mostly serial — monolith repoints)
  A0  scaffold filesStore.svelte.ts ‖ projectStore.svelte.ts        [parallel: 2 new files]
  A1→A2→A3→A4→A5  repoints into +page.svelte                        [SERIAL sub-steps, green/commit]
        │ unlocks file-disjoint lanes
        ▼
PHASE B  (3 lanes run concurrently; +page.svelte integration + cargo serialized)
  ┌ B1 panels ─────────┐  ┌ B2 LSP (Rust+Monaco) ─┐  ┌ B3 git/worktree ┐
  │ B1.1 EditorPanel   │  │ B2.1 lsp dedupe (cargo)│  │ B3.1 inventory  │
  │ B1.2 InsightsPanel ⇄──┼─⇄ B2.5 Insights-remove │  │ B3.2 invariant  │
  │ B1.3 ActivityGit   │  │ B2.2 warm re-index     │  │ B3.3 collision  │
  │ B1.4 Activity*      │  │ B2.3 L1 attach servers │  │ B3.4 fixes      │
  └────────────────────┘  │ B2.4 L2 kill JS worker │  └─────────────────┘
                          │ B2.6 view-button merge │
                          └────────────────────────┘
        ▼  (B1.2 ⇄ B2.5 coordinate: same surface — order them, don't co-edit)
PHASE C  (C1 backend GATES the frontend; then lanes)
  C1 terminal.rs (cargo) ──gates──► C2 manager wiring ──► C3 lifecycle FSM
                                    C4 rail (parallel to C2/C3 — new files)
                                    C5 restore (depends C2; LSP-repoint depends B2.2)
  (C1 may be authored alongside the TAIL of B — disjoint files; serialize cargo with B2.1/B2.3.)
        ▼
PHASE D  D1 diagnostics ‖ D2 delete old scan-resume ‖ D3 deferred-minors → D4 final review
```

---

## PHASE A — Data-spine `filesStore` + `projectStore` (mostly SERIAL — it repoints the monolith)
**Parallelism here:** only A0 (scaffold the two stores as disjoint new files) fans out; A1–A5 are serial sub-steps on `+page.svelte` (overlapping state repoints), one green commit each — exactly the `dockLayoutStore` model. This is the unavoidable serial spine that BUYS Phase B's parallelism.

**Files:** create `src/lib/stores/filesStore.svelte.ts`, `src/lib/stores/projectStore.svelte.ts` (rune-state SHAPE mirrors `settingsStore.svelte.ts`; LOCATION follows `dockLayoutStore` under `src/lib/stores/`); modify `src/routes/+page.svelte`. **Produces (interface later phases code against):** `files = $state({ records, selectedRecord, preview, openTabs, dirtyByPath, selectedSourceLine, … })`; `project = $state({ selected, root, projects, scanStats })`; pure helpers (`rankSourceRecords`, dirty/path/tab predicates) — side-effect-free only. **Stays in page:** every `read_source_file`/`write_source_file`/`list_source_files` Tauri call + all `$effect`s.

- [ ] **A0 scaffold** both stores (empty `$state` shells mirroring `settingsStore.svelte.ts`), import types. Parallel-authorable. Commit.
- [ ] **A1 migrate `records`** → `files.records` (grep all reads/writes; delete page decl; the scan fn writes `files.records`). Verify: tree + file-open render. Commit.
- [ ] **A2 migrate `selectedRecord` + `preview`** → `files.*` (the `read_source_file` setter stays in page). Verify: select file → Monaco opens. Commit.
- [ ] **A3 migrate open tabs + dirty** → `files.openTabs`/`dirtyByPath` (+ pure tab/dirty derivations). Verify: multi-tab + dirty chip + tab switch. Commit.
- [ ] **A4 migrate `selectedProject` + scan stats** → `project.*` (scan/switch fns stay, write `project.*`). Verify: project dropdown switch. Commit. [Optional parallelism: project state (`selectedProjectID`/`sourceScanStats`/the `selectedProject` $derived) is a DISJOINT island from A1's `records`/`selectedRecord` derivations + writes the separate `project` store → A4 MAY run as a parallel sub-lane alongside A1–A3, shortening the serial spine — only if the integrator confirms no shared `$derived`.]
- [ ] **A5 move pure helpers** (only side-effect-free). Verify full set (tree/open/edit/tabs/project/quick-open/palette). Commit.
- **Gate:** stores own the spine; editor/insights/git markup reads `files.*`/`project.*` → extractable in B1. Re-grep anchors for B.

---

## PHASE B — Consolidation (3 PARALLEL lanes; depends A)
Author new files in parallel across lanes; serialize `+page.svelte` integration (one integrator) + cargo (B2.1/B2.3). Each task: **Files · Consumes · Produces · Depends**.

### Lane B1 — panel extractions
- [ ] **B1.1 EditorPanel.svelte** · Files: create `panels/EditorPanel.svelte`, modify `+page.svelte`+`app.css` · Consumes: `files.preview/selectedRecord/openTabs/dirtyByPath`, `panelAction` prop, Monaco lookup callbacks (passed through) · Produces: `<EditorPanel/>` · Depends: A. Keep `use:panelAction={'editor'}` + per-tab teleport + the `<MonacoSourceEditor>` child. Verify: open/edit/tabs + go-to-def/peek/codelens, styled. ⚠️ **coordinate with B2.6** — the editor `<header class="topbar">` + its ~290-line View menu (`+page.svelte` ~17695-18004) + `WorkbenchControls` are shared: do B1.1 FIRST (settle the topbar's home in the extracted component), THEN B2.6 reconciles the two View buttons inside it; never co-edit.
- [ ] **B1.2 InsightsPanel.svelte + intelligenceStore.svelte.ts** · Files: create both, modify `+page.svelte`+`app.css` · Consumes: `files.selectedRecord` · Produces: `<InsightsPanel/>`, `intelligence` store · Depends: A · ⚠️ **coordinate with B2.5** (same surface): order them, never co-edit. If B2.5 removes the source-intelligence lists first, B1.2 extracts only the kept Git-insights.
- [ ] **B1.3 ActivityGitPanel.svelte + gitStore.svelte.ts** · Files: create both, modify `+page.svelte`+`app.css` · Consumes: `project.root`; writes `files.selectedRecord` via callback · Produces: `<ActivityGitPanel/>`, `git` store · Depends: A. Verify: git status/changes/history + actions.
- [ ] **B1.4 remaining Activity* leaves** (runs/sessions/agents) · extract presentationally if cleanly separable; DEFER any that couple to Phase-C runtime/conversation state (name them) · Depends: A.

### Lane B2 — LSP consolidation (spec §5.5; mostly backend/Monaco → parallel to B1/B3)
- [ ] **B2.1 dedupe per-(root,language) servers** · Files: `src-tauri/src/lsp.rs` (+tests) · Produces: one server/language across worktree roots · Depends: none (backend-only; cargo-serialized). Verify: `cargo test -p`.
- [ ] **B2.2 warm + debounced re-index on switch** · Files: `lsp.rs` + switch hook in `+page.svelte` · re-point warm server at new root, debounced, cancel-in-flight, first-activation/file-change only · Depends: B2.1.
- [ ] **B2.3 L1 attach real servers (kill index-fallback)** · Files: `lsp.rs`, status surface · ensure real server attaches when available; clear install-state · Depends: none.
- [ ] **B2.4 L2 route TS/JS/JSON through Rust only** · Files: `MonacoSourceEditor.svelte` · Per spec §5.5 KEEP the bundled worker LOADED for offline/instant feel but SUPPRESS its diagnostics/validation (`setDiagnosticsOptions` ~`:354/357`) so Rust LSP owns markers — a load-but-suppress, NOT a hard disable · Depends: the Rust path must feed Monaco ALL THREE first — documentSymbol/folding (symbols+sticky-scroll) AND **diagnostics via `applyExternalDiagnostics`/`externalDiagnostics` (~`:1317`)** · ⚠️ **B2.4 ⇄ B2.5 data-flow:** `sourceDiagnostics` is a Monaco→page OUTPUT aggregate (`publishDiagnostics` ~`:1296` reads ALL model markers incl. the worker's) → suppressing the worker empties TS/JS `sourceDiagnostics` UNLESS Rust `externalDiagnostics` is wired+active · Verify: TS/JS squiggles + symbols + sticky-scroll all appear Rust-sourced; `sourceDiagnostics` non-empty for a TS file with a real error.
- [ ] **B2.5 remove Insights source-intelligence lists** · Files: `+page.svelte`/InsightsPanel **+ `src/lib/workspaceSnapshot.ts` + `src/lib/workspaceSnapshotPlan.ts`** — `editorInsightCollapsed` (`workspaceSnapshot.ts:38/467`) AND `sourceIntelligencePanel` (`:48/490`) PERSIST there, not just in `+page.svelte`; you can't unwind them from snapshots without editing both · do the unwind WITH a persisted-shape MIGRATION (guard/default so old `viewState` parses — mirror the defensive parse at `workspaceSnapshot.ts:467`) · KEEP the `sourceDiagnostics`/`sourceSymbols` data FLOWING OUT (Monaco→page outputs `onDiagnosticsChange`/`onSymbolsChange`) to markers/symbol-provider/codelens consumers (it's an OUTPUT, not "fed to Monaco") · ⚠️ coordinate with **B1.2** (same InsightsPanel surface) + **B2.4** (diagnostics source) · ⚠️ **B2.5 → C5 ORDERING:** make the snapshot-shape change in Phase B so C5's `restoreConversationWorkspaceSnapshot` codes against the FINAL shape; `workspaceSnapshot*.ts` is a shared surface for the serial integrator · Verify: Monaco go-to/peek/refs/codelens/squiggles/Cmd+Shift+O all work; OLD persisted snapshots still load.
- [ ] **B2.6 reconcile the two View buttons** · Files: `+page.svelte` + WorkbenchControls/editor-topbar · drop/merge one (share `viewMenuOpen`, overlap in workbench mode) · Verify: no overlap.

### Lane B3 — git/worktree correctness (spec §5.6; parallel to B1/B2)
- [ ] **B3.1 inventory** · reproduce + list each broken behavior · Produces: the fix list (feeds B3.4).
- [ ] **B3.2 in-place-switch invariant** · Files: `conversationWorkspaceRestore.ts`/`planConversationRestore` (`preCommands: []`) · switching the view never mutates another session's cwd/branch.
- [ ] **B3.3 same-worktree collision warning** · extend the readiness model; missing-worktree → copyable repair plan (never auto-add).
- [ ] **B3.4 fix each inventoried breakage** · Files per breakage (`tauriSource.ts`/git Rust/`+page.svelte`) · Depends: B3.1.

**Phase B gate:** one-shot spec-compliance review.

---

## PHASE C — Live Agent Sessions (spec Plan 1; C1 backend GATES frontend)

### C1 — `terminal.rs` backend (Rust; cargo-serialized) — spec §5.2/5.4/5.7
- [ ] **C1.1 no-kill-on-reload for owned PTYs** · Files: `terminal.rs` + dispose guard in `+page.svelte` · Produces: owned PTYs survive webview reload.
- [ ] **C1.2 tombstone terminated sessions** (+ `closed`/`exited` flag on `TerminalSessionInfo`) · Files: `terminal.rs` · Produces: finished/crashed sessions persist for "read final output".
- [ ] **C1.3 `COMMANDBAR_SESSION_ID`** in `TerminalStartRequest` + env block · Files: `terminal.rs` + `tauriSource.ts` request type · Produces: spawned agents correlate to `ownedId`.
- **Gate:** C1's backend contract is the interface C2 codes against.

### C2 — live-terminal manager wiring (depends C1) — spec §5.2
- [ ] **C2.1 replace singleton with `liveConversationTerminals`** · Files: `+page.svelte` + `liveConversationTerminals.ts` (built+tested, unwired).
- [ ] **C2.2 single listener → `feedSession`; remove active-only early-return; `terminated`→`markTerminated`** · Files: `+page.svelte`.
- [ ] **C2.3 bind `ptySessionId→ownedId` BEFORE subscribe/replay; normalize `cmux-*` identity** · Files: `+page.svelte` + identity helper — **EXTEND the existing normalizer** (`workspaceSnapshotPlan.ts:155` already maps `cmux-*`→enum; scanner emits `cmux-${agent}` at `localSourceFs.ts:312`), don't reinvent · Produces: `feedSession(sessionId,data)` routing; `ownedId` primary (spec §4).

### C3 — lifecycle FSM (depends C2) — spec §5.4
- [ ] **C3.1 Background = alive+hidden, WebGL released, ZERO replay** (re-insert+`fit()`+`focus()`+reacquire GPU) · Files: manager module + `TerminalView` seam.
- [ ] **C3.2 3-state FSM + LRU + N≈12–14 (WebGL ~16-context cap)** + throttle hidden writes · Files: manager + `settingsStore`.
- [ ] **C3.3 idle-timer auto-Hibernate (default 30min, configurable)** — stops the agent (real RAM lever); dispose after SerializeAddon snapshot · Files: manager + `settingsStore`.

### C4 — Available→Owned rail (PARALLEL to C2/C3 — mostly new files) — spec §5.1/§4
- [ ] **C4.1 rail UI** (Owned tabs + state badge/LIVE dot; Resume group) · Files: new rail component (replaces `ConversationList` usage) · Consumes: session list + states (interface from C2/C3).
- [ ] **C4.2 start-fresh + adopt/resume → Owned** · Files: rail + manager + identity.

### C5 — workspace auto-load + cold-path (depends C2; LSP-repoint depends B2.2) — spec §5.3/5.7/5.8
- [ ] **C5.1 auto-load workspace in place** (reuse `restoreConversationWorkspaceSnapshot`) · Files: `+page.svelte` + restore module.
- [ ] **C5.2 SerializeAddon snapshot (Hibernate save + cold revive); reload-survive; relaunch cold-resume** · Files: manager + restore.

**Phase C gate:** spec §10 criteria; live Tauri verification; spec review. Cross-cutting (spec §5.9, no silent drop): accessibility (xterm focus across hidden/shown views) + the error/empty-state matrix (PTY spawn fail mid-switch · agent crash while Background · WebGL context-loss · dead session in `list_terminal_sessions` · missing-worktree on switch-to · finished-agent final-output for `markTerminated`) are handled in the C2–C5 verify steps + D1.

---

## PHASE D — tail (parallel) + finish
- [ ] **D1 diagnostics surface** (per-session state/PID/GL-context/LSP servers) · spec §5.9 · Files: new diagnostics component.
- [ ] **D2 delete old scan-and-resume / external-app-open paths** once Owned replaces them · Files: `+page.svelte` + scanner.
- [ ] **D3 deferred minors** (`.topbar-command-button` → `app.css :global`; factor the 2 overlay siblings; the 2 `dockLayoutStore` dupes).
- [ ] **D4 final whole-branch code review** (the "once in a great while" deep review) + `finishing-a-development-branch`.

---

## Self-Review
- **Spec coverage:** C↔spec §4/§5.1–5.8/§6/§10; §5.5→B2; §5.6→B3; §5.0 decompose→A+B1; §5.9→D1. ✅
- **Parallelism is explicit + maximal:** every task has Files+interface+Depends; the dependency graph fans out everything with no inbound arrow; serialization is limited to the 3 invariants (one `+page.svelte` editor, one committer, one cargo). Honest serial points: Phase A spine (monolith repoints), the integration step (now `+page.svelte` AND `app.css` AND `workspaceSnapshot*.ts` through one integrator), C1-before-C2, and the same-surface coordination pairs B1.2⇄B2.5 (InsightsPanel), B1.1⇄B2.6 (editor topbar), B2.4⇄B2.5 (`sourceDiagnostics` data-flow), B2.5→C5 (snapshot shape). ✅ **(adversarial plan-review incorporated: 1 Critical + 3 Important + minors)**
- **No placeholders for structure:** task boundaries, files, interfaces, deps are concrete; only exact line anchors are re-grepped at task start (file shifts as tasks land). ✅
- **Type/name consistency:** stores `files`/`project`/`git`/`intelligence`/`dock`(shipped); read `store.x`; teleport prop `panelAction`; identity `ownedId` primary. ✅

## Execution
**superpowers:subagent-driven-development**, phase-by-phase. Phase A serial sub-step commits. Phase B: dispatch all file-disjoint author tasks across the 3 lanes concurrently; ONE serial `+page.svelte` integrator; ONE committer; cargo serialized. Phase C: C1 first, then fan out C2/C4 then C3/C5. One spec-compliance review per phase; one deep code review at the end. Verify every task live. Opus subagents; green commits.
