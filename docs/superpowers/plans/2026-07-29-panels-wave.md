# Panels Wave: Source Control, Worktrees, Stacks, New Session, Playwright, Themes, Problems

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The /next shell gets its tool surfaces: VS Code-grade source control with a real commit graph, the worktree manager with safe + destructive cleanup, a stack runner with live process states, a project picker + new-session flow, the Playwright process card, a theme system that actually applies, and a Problems panel in the bottom dock.

**Architecture:** Fan-out. One Rust lane (Task 0) in an isolated worktree collects every backend gap. Seven frontend lanes (Tasks 1-7) run in PARALLEL in the primary checkout under a strict **new-files-only** rule — each builds its components/stores/services/tests as new files and writes an integration contract; none edits shared files. One integrator (Task 8) wires all contracts into the page/sidebar/dock/palette and runs the single combined pass. Then milestone review → fixes → PR.

**Tech Stack:** Svelte 5 runes, shadcn-svelte (vendored `$lib/components/ui`), Tailwind v4 (no preflight), dockview-core 6.6.1, Rust (src-tauri + mcb-core), Node test scripts.

**Covers:** TSK-344 (source control, High), TSK-763 (worktrees, High), TSK-767 (stacks, High), TSK-760 (new session, High), TSK-764 (Playwright), TSK-765 (themes, chooser slice), TSK-770 (problems).

## Global Constraints

- Old shell `tauri-svelte-preview/src/routes/+page.svelte` FROZEN + carries the user's uncommitted diff — never edit/stage; old shell renders pixel-identical (next.css never reachable from it; `nextTokens.test.mjs` guards).
- `src-tauri/` in the primary checkout is UNTOUCHABLE (watched by the user's tauri dev). ALL Rust work happens in Task 0's worktree. `core/` in primary is editable but no lane here needs it.
- **New-files-only for Tasks 1-7**: a parallel lane may create files and extend ONLY its own new files. The ONLY shared-file exception: `package.json` test-script entries are NOT added by lanes — each lane documents its test command in its contract; the integrator adds all script entries at once. Lanes needing a new backend wrapper write it in their own `<lane>Backend.ts` (import `@tauri-apps/api/core` directly, same null-when-not-Tauri convention as tauriSource.ts) — the integrator may consolidate later.
- Each lane ships `_(lane)-INTEGRATION.md` in its component folder: exact wiring instructions (imports, props, roster entries, palette commands, storage keys) for the integrator.
- Stores: no backend calls, no `$effect`; IO in explicit service functions; every persisting mutator calls its own `persist()`.
- shadcn/typography traps (all documented in the wave-1 ledger and `next.css:36-42`): explicit font sizes (`text-[13px]` body floor, 12px meta floor — `text-sm` here is 12px); any NEWLY vendored ui component must be opened in a browser before it is trusted (pinned bits-ui 2.18.1 vs newer registry: `data-state` families, zero-height variants); `Collapsible.Content` hides via `hidden` — layout classes go on an inner div; utilities stay un-layered; variants in sibling `variants.ts`.
- Visual identity FIXED: today's dark Houston look, tighter and more polished. No Material, no zinc-on-white drift. Row density VS Code-tight where specified, achieved with padding — never sub-floor fonts.
- Plain-English copy everywhere. No `window.confirm`/`alert`/`prompt` — AlertDialog. Commits plain, NO Co-Authored-By trailer.
- Verification per lane: `pnpm run check` clean, `node scripts/checkSvelteNext.mjs` clean, `pnpm build` green, own Node tests red-first. Pre-existing failures not yours: sourceDockviewWorkspace, sourceUi, workspaceSnapshotPlan (+ native-lsp/tauri-app need missing binaries).
- Desktop-only data: every lane renders an honest desktop-only/empty state on web (`contextService.ts` DESKTOP_ONLY pattern); git gets bridge routes (Task 1) so the graph is iterable in Chrome.
- Task 0 is the only cargo builder. It runs in `/Users/blackcolours/dev/work/worktrees/mac-command-bar/panels-backend` and the controller removes that worktree after merge.

---

### Task 0 (Rust lane, worktree, runs parallel with 1-7): the backend gaps, in one pass

Branch `tsk-344-763-764-770-backend` from main. Everything below is `src-tauri` (main.rs / lsp.rs / terminal.rs / orchestration.rs) unless noted. Frontend lanes code against these shapes with graceful-absence fallbacks, so exact names below are CONTRACT — do not rename.

1. **`read_git_commit_files(root, sha)` → `Vec<GitCommitFileChange { relativePath: String, status: String, badge: String }>`** — `git show --name-status --format= <sha>` parsed like `project_git_status` badges. Plus **`read_git_commit_file_diff(root, sha, relativePath)` → `SourceGitDiff`** — `git show <sha> -- <path>` shaped like `read_source_git_diff`.
2. **`remove_project_worktree` gains `force: Option<bool>`** — default keeps today's hard gates; `force: true` runs `git worktree remove --force` + prune, still refusing the primary checkout and still refusing a LOCKED worktree unless it also unlocks first (`git worktree unlock`); the returned message says exactly what was destroyed in plain words.
3. **`kill_playwright_session(pgid: i32)`** — TERM→wait→KILL for ONE listed session group (reuse `kill_playwright_sessions_with` internals), refusing pgids not currently in `parse_playwright_processes` output (never a generic pid killer exposed to the UI).
4. **Diagnostics for a Problems panel:** `SourceLspDiagnostic` gains `path: Option<String>` (populated from the URI in `record_diagnostics`), and new command **`list_source_lsp_diagnostics_for_root(root)` → Vec<SourceLspDiagnostic>** returning everything in `diagnostics_by_uri` for sessions under that root (empty vec when none). No push events this round.
5. **`TerminalStartRequest` gains `command: Option<String>`** — when present, spawn `shell -lc <command>` (portable_pty CommandBuilder args) instead of an interactive shell, so stack runs get clean exit codes without keystroke injection. Existing callers unaffected (field optional).
6. **Gate the demo runs:** `orchestration.rs:181` — the fabricated `demo_orchestration_runs` fallback only when env `MCB_DEMO_RUNS=1`; otherwise an empty log returns an empty list. Plain-words comment.
7. Tests: each addition red-first in the existing test modules (`main.rs` test mod + `terminal.rs` + `orchestration.rs`); `cargo test -- --skip lsp::` green; `cargo test --manifest-path core/Cargo.toml` untouched-green.
- [ ] Commit(s) + push branch; NO PR (controller merges after review). Report scan of what frontend contracts consume each command.

### Task 1: Source control — VS Code-grade panes, commit graph, dockable diff

The store/service/guard layer (`gitPanelStore`, `gitService`, `parseUnifiedDiff`, `GitDiffView`) is sound — keep it. This lane rebuilds presentation + adds the graph.

**New files:** `src/lib/shell/git/gitGraphLanes.ts` (pure lane-assignment over `parentShas` → per-row `{lane, edges[]}` feeding `gitGraphViewModel`'s `GitGraphCommitRow`; red-first Node test), `src/lib/shell/git/gitBackendExtra.ts` (wrappers for `read_git_commit_files` / `read_git_commit_file_diff`, null off-desktop), `src/lib/shell/components/git/SourceControlPanes.svelte` (+ children: `RepoPane.svelte`, `ChangesPane.svelte`, `GraphPane.svelte`), `src/lib/server/gitBridge.ts` + its vite routes FILE (exported route-registrar the integrator mounts — do NOT edit vite.config.ts yourself) so status/history/commit-files work on web via `simple-git`-free `git` child_process calls mirroring the Rust commands, `scripts/gitGraphLanes.test.mjs`.
**Spec highlights (from TSK-344 + screenshot 28):** Changes pane = message box placeholder `Message (⌘Enter to commit on '<branch>')`, full-width Commit button, tight rows (icon + name + dimmed path + status letter right), ⌘Enter commits. Graph pane = dots+edges from `gitGraphLanes`, branch/tag pills, each commit expands in place (Collapsible, classes on inner div) to its file list; file click → per-commit diff. Diff = the existing propless `GitDiffView` teleported into a FOURTH center-dock panel `{id:'diff', title:'Diff', group:'display'}` — contract entry for the integrator (roster + `CENTER_LAYOUT_KEY` bump + `gitService.selectFile` activation hop), not an edit here.
- [ ] Panes render against live data in Chrome via the bridge; graph verified on this repo's history; contract written; commit.

### Task 2: Worktree manager view

**New files:** `src/lib/shell/worktrees/worktreeManagerStore.svelte.ts` + `worktreeManagerService.ts` (list via existing `listProjectWorktreesFromTauri`, join `GitRepositorySummary` by path for ahead/behind, join sessions by cwd/projectPath for "who worked here"), `src/lib/shell/components/worktrees/WorktreeManagerPane.svelte` (+ row/detail children), `scripts/worktreeManager.test.mjs` (pure join/derive logic).
**Spec:** rows = branch, task chip, age, dirty/unmerged/locked/prunable chips, ahead/behind, sessions-that-touched-it; detail = `worktreeSafety.ts` decision lane + copyable audit/backup/cleanup commands (import the tested module, don't rewrite); SAFE remove = existing `remove_project_worktree`; ARCHIVE = existing command; DESTRUCTIVE remove = `force:true` (Task 0) behind an AlertDialog that lists exactly what dies (dirty files count, unmerged commits) and requires typing the worktree folder name. Force button renders disabled with a plain tooltip until the desktop has the new command (probe: call returns unknown-command error → catch → mark unavailable).
- [ ] Contract: replaces the worktrees placeholder view in `sidebarViews`; palette command "Clean up worktrees". Commit.

### Task 3: Stack runner

**New files:** `src/lib/shell/stacks/stackStore.svelte.ts` (defs persisted per project: `{id, name, script, cwd}`, runs keyed by ownedId), `stackService.ts` (start = createFreshSession(cwd)+startOwned with the script as the command — via the page contract; watch state from terminal exit events + `list_runtime_contexts` join: running(:port)/starting/failed(exitCode)/stopped), `src/lib/shell/components/stacks/StacksPane.svelte`, `scripts/stackStore.test.mjs`.
**Spec:** per-project stack list with Add (name + script path picker from a plain input this round), Start/Stop per stack, per-process chips, click-through = select the stack's owned session (it IS a session in the rail, badged "stack"). `OwnedSession` needs a `kind: 'agent'|'stack'` style tag — that is a SHARED file, so: define the tag in your own module keyed by ownedId (persisted), contract the integrator to fold it into `OwnedSession` later if wanted. No polling: refresh on pane open + explicit refresh + terminal exit events.
- [ ] Contract: pane joins the tool-view roster OR a context-region card (recommend roster view; state your pick). Commit.

### Task 4: Playwright process card

**New files:** `src/lib/shell/processes/playwrightStore.svelte.ts` + `playwrightService.ts` (wrappers for `list_playwright_sessions`, `kill_playwright_sessions`, and Task 0's `kill_playwright_session(pgid)` with graceful absence), `src/lib/shell/components/processes/PlaywrightCard.svelte`, `scripts/playwrightStore.test.mjs` (grouping/label/age derivation).
**Spec:** groups by session (pgid) with label (daemon/server/mcp/chrome-profile), age, pid list; "Stop this session" per group (AlertDialog), "Stop all Playwright" (AlertDialog, says how many groups); clearly worded "these are Playwright's browsers, not your Chrome". Desktop-only empty state on web.
- [ ] Contract: card mounts in the context view (with the other cards). Commit.

### Task 5: Project picker + new session

**New files:** `src/lib/shell/newSession/projectRootsStore.svelte.ts` (known roots = `defaultProjectRoots` + persisted custom roots + roots derived from owned sessions; add-folder via `@tauri-apps/plugin-dialog` `open({directory:true})` wrapped null-off-desktop), `newSessionFlow.ts` (pure: the launch catalog — fresh `claude`, `codex`, plain shell, each `{agent, command, label}`; command preview assembly; validation), `src/lib/shell/components/newSession/NewSessionDialog.svelte` (shadcn Dialog: project → worktree (from `listProjectWorktreesFromTauri`, primary checkout listed first) → agent → editable command preview → Start), `scripts/newSessionFlow.test.mjs`.
**Spec:** NEVER creates worktrees (standing invariant — the worktree step only picks existing ones; a "new worktree" affordance shows the copyable `git worktree add` command instead). Start = contract handoff: the integrator wires `onStart({cwd, title, agent, command})` to `createFreshSession` + `addOwnedSession` + `startOwned` + `selectOwned` in the page (createFreshSession currently has zero callers — this is its wiring). Dialog opens from a "+ New session" button contracted into the sessions column header and a palette command.
- [ ] Catalog verified: `claude` and `codex` fresh-launch commands confirmed against `--help`/docs (state source). Commit.

### Task 6: Themes that actually apply

Scope = the chooser slice of TSK-765 (VSIX import stays queued). **New files:** `src/lib/shell/themes/themeRegistry.ts` (theme = named set of the ~20 nextTokens color values + Monaco `ThemeDefinition` + xterm `ITheme`; ships `houston` (extracted from today's values — byte-identical) + `dracula` (xtermFactory already has its terminal half) as proof of switching), `themeService.ts` (apply = set CSS vars on the `.next-shell` root via inline style properties, `monaco.editor.defineTheme`+`setTheme` through a registration seam, live `terminal.options.theme` walk via a callback the terminal factory registers, persist `settings.appearance.themeId` — the setting exists and is read by nothing today), `scripts/themeRegistry.test.mjs` (registry completeness: every token name present in every theme; houston matches nextTokens.css values read from the file).
**Also this lane (its own new-file guard):** extend `scripts/nextTokens.test.mjs`-style coverage with a NEW test asserting no `--dv-*` hex remains (the ~26 dockview vars must become `var(--color-*)` refs — those live in shell component styles; list the exact edits in the CONTRACT for the integrator rather than editing shared components yourself). The ~246-hex component sweep is explicitly OUT of scope — captured as follow-up; the theme applies to shadcn chrome + Monaco + xterm + dockview vars this round.
- [ ] Contract: SettingsDialog's Appearance select consumes the registry (integrator swaps its item list + wires `themeService.apply`); Monaco/xterm registration points named precisely. Commit.

### Task 7: Problems panel

**New files:** `src/lib/shell/problems/problemsStore.svelte.ts` + `problemsService.ts` (v1 source: Task 0's `list_source_lsp_diagnostics_for_root` wrapper in own backend file with graceful absence + per-open-file fallback via existing `readSourceLspDiagnosticsFromTauri`; loads on panel activation + explicit refresh, never at launch), `src/lib/shell/components/problems/ProblemsPanel.svelte` (grouped by file, severity badges + counts, filter input, row click → `requestOpenFile({path, line, column})`), `scripts/problemsStore.test.mjs` (grouping/sorting/counts).
**Also contract (shared-file wiring for the integrator):** the bottom dock's placeholder (`DockPanel.svelte`) becomes the Problems host; and the found live bug — /next Monaco shows NO squiggles — fixed by wiring diagnostics into the editor via `MonacoSourceEditor`'s existing `externalDiagnostics` prop from `sourceIntelligence` (spell out the exact wiring in the contract; if it is small enough the integrator lands it, else it is reported as its own follow-up).
- [ ] Commit.

### Task 8 (after 1-7 land): the integrator

Reads every `_(lane)-INTEGRATION.md` and wires: sidebar roster (worktrees view replacement, stacks view), context view (playwright card), center dock (diff panel + layout-key bump), bottom dock (problems), sessions column header (+ New session), page handlers (`onStart` flow, stack session tagging fold-in if accepted), SettingsDialog theme select + apply call, `--dv-*` var edits from Task 6's contract, vite.config.ts mounting `gitBridge` routes, palette commands, package.json script entries for every lane's tests. Resolves conflicts in favor of contracts; anything ambiguous goes in the report, not guessed.
- [ ] Full pass: `pnpm run check` + `node scripts/checkSvelteNext.mjs` + `pnpm build` + ALL Node suites + a live Chrome walkthrough of every new surface. One commit (or few, logical). Contract files deleted after wiring (they are scratch, and the SDD workspace keeps the reports).

---

## Verification & merge

- Task 0 merges FIRST (own PR after the milestone reviewer covers its diff; controller removes the worktree immediately after; user restarts tauri once at round end).
- Milestone review after Task 8: whole frontend branch + Task 0 diff, fix wave, scoped re-review, PR `tsk-344-760-763-764-767-770-panels-wave` → main.
- End-of-round desktop pass items are collected in the ledger as they accrue.
