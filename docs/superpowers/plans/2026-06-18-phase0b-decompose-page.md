# Phase 0b — Decompose `+page.svelte` — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.
> **This is an EXTRACTION plan, not a greenfield one.** Tasks MOVE existing code into new files; they do not rewrite it. "Code steps" cite the exact source line range to move and the target file + the component/module interface — the executing agent moves the cited region verbatim and adjusts wiring. Do NOT re-author moved logic.

**Goal:** Turn the 28,929-line `src/routes/+page.svelte` into a thin orchestrator by extracting self-contained sections into focused `.svelte` components and shared state into `.svelte.ts` rune-state modules — creating **file-disjoint seams** so the subsequent master plan (Plan 1 live-agent-sessions + Plan 2 LSP/git) can run maximally parallel.

**Architecture:** Mirror the proven `src/lib/settingsStore.svelte.ts` pattern for state modules (module-level `const store = $state({...})`; consumers `import { store } from '...'` and read `store.x` — stays reactive; NEVER `export let x` + reassign). Components read/write those stores directly (avoid prop-drilling) except where a section is a pure presentational leaf (then props). Extract safe leaves first to prove the pattern, then subsystem state modules + their components.

**Tech stack:** SvelteKit (static adapter SPA), Svelte 5 runes, TypeScript, Dockview teleport bridge, Tauri.

## Global Constraints
- **Verify every extraction live before commit:** Vite compile clean (no HMR overlay/error) + a `playwright-cli` screenshot of the affected panel rendering + `console error` = 0 + `tsc`/`svelte-check` clean. Commit green per task. Git is the safety net (each task independently revertable).
- **Single-file bottleneck:** every extraction edits `+page.svelte` (delete moved code + add import/usage). **All `+page.svelte` integrations are SERIAL — one integrator agent holds the edit lock.** New files are authored in parallel (file-disjoint), then integrated one at a time. (Honors the user rule: one agent edits `+page.svelte` at a time.)
- **NEVER break the teleport bridge.** The template is wrapped by the live `<SourceDockviewShell>` host (15770→20199). The 5 teleport action fns live at `+page.svelte:12836-12908`. Any extracted component containing a `use:sourceDockviewPanelAction={panelID}` (or a nested `*PanelAction`) node MUST receive that action as a prop and keep the node, so its closure still reaches `unifiedWorkbenchSetPanelElement` / the workspace handles. A teleport node that loses its action = silent blank panel — screenshot-verify each panel after extraction.
- **Effects stay in components.** `.svelte.ts` modules CANNOT host `$effect` (not component context). State → module; the `$effect`/ResizeObserver/timer that acts on it stays in a `.svelte` component (the page or the extracted one), reading the store. Move a `bind:this` ref together with the `$effect`/measure logic that uses it.
- **DO NOT extract the terminal.** `embeddedTerminal*` (928-935, 1064-1075), `sourceTerminalApp` (927), `embeddedTerminalAppearance` (794) + all terminal fns are owned by **Plan 1**, which rewrites them. In 0b, leave them in the page (a clearly-commented `// --- terminal (Plan 1 owns; do not extract) ---` region is enough). Extracting now = wasted work + merge conflict.
- **DO NOT extract snapshot-restore / `conversationsStore` yet.** Workspace-snapshot capture + restore orchestration overlaps Plan 1's session model — keep it in the page until Plan 1 settles.
- **Never run parallel heavy builds/tests** (serialize cargo/svelte-check); never auto-mutate git.

## Parallelism model (how this plan fans out)
- **Author lane (parallel, file-disjoint):** an agent writes a brand-new `.svelte`/`.svelte.ts` file (moving the cited region's code into it) + returns the integration diff snippet. Multiple author agents run concurrently on DIFFERENT new files.
- **Integrate lane (serial):** ONE integrator agent applies each integration into `+page.svelte` (replace region with `<Component/>` + import; point state reads at the store), verifies live, commits. Integrations are processed one at a time in dependency order.
- Per `subagent-driven-development`, the controller dispatches author agents in parallel, then serializes the integrator.

## File Structure
**Create — Wave 1 (leaf components, presentational, props-based; no core-store dependency):**
- `src/lib/components/panels/BrowserPanel.svelte` — section G
- `src/lib/components/overlays/CommandPaletteOverlay.svelte` — section M
- `src/lib/components/overlays/QuickOpenOverlay.svelte` — section L
- `src/lib/components/panels/ActivityClipboardPanel.svelte` — paste-cleanup sub-panel
- `src/lib/components/chrome/HiddenDockRail.svelte` — section I
- `src/lib/components/chrome/WorkbenchControls.svelte` — section J

**Create — Wave 2 (subsystem state modules + their components; enable master-plan parallelism):**
- `src/lib/stores/dockLayoutStore.svelte.ts` — dock layout/visibility/resize state + the 5 teleport action fns
- `src/lib/stores/gitStore.svelte.ts` — git status/worktrees/repo summaries/commit history/diff
- `src/lib/stores/intelligenceStore.svelte.ts` — LSP symbols/diagnostics/definitions/references + search
- `src/lib/components/panels/ActivityGitPanel.svelte` + `ActivityWorktreesPanel.svelte` — consume gitStore
- `src/lib/components/panels/InsightsPanel.svelte` — section E; consumes intelligenceStore
- `src/lib/components/panels/EditorPanel.svelte` — section D toolbar/tabs shell around the existing `MonacoSourceEditor`

**Modify (integration only):** `src/routes/+page.svelte` (serial).

**Deferred to a follow-up 0b pass or folded into the master plan (flagged, NOT in this plan):** `projectStore`/`filesStore` (referenced almost everywhere → highest edit-churn/risk — extract last, with extreme care), `runtimeStore`, the remaining 5 `Activity*Panel` leaves, `conversationsStore` (Plan 1 overlap), terminal (Plan 1).

---

## Wave 1 — Safe self-contained leaves (prove the pattern)

### Task 1: BrowserPanel.svelte
**Files:** Create `src/lib/components/panels/BrowserPanel.svelte`; Modify `+page.svelte` (section G `20100-20194`; browser state `936-939`; browser fns — `grep -n "browser" +page.svelte` for `submitBrowserUrl`/`reloadBrowserFrame`/`openBrowserUrlExternal`/`activeBrowserUrl`).
**Interface:** Props `{ url: string; inputUrl: $bindable<string>; frameKey: number; error: string; panelAction: (node, id) => any; onSubmit, onReload, onOpenExternal, onHide }`. Keeps the `use:panelAction={'browser'}` node. Self-contained (browser state is near-isolated).
- [ ] **Step 1 — author:** create the file; move the section-G markup (the `<section class="browser-dock">`…`</section>` and the URL form) into it; declare props above; replace internal state refs with props; keep `use:panelAction={'browser'}`.
- [ ] **Step 2 — integrate:** in `+page.svelte`, replace the `20100-20194` markup with `<BrowserPanel url={activeBrowserUrl} bind:inputUrl={browserInputUrl} frameKey={browserFrameKey} error={browserError} panelAction={sourceDockviewPanelAction} onSubmit={submitBrowserUrl} onReload={reloadBrowserFrame} onOpenExternal={openBrowserUrlExternal} onHide={() => hideDockPanel('browser')} />`; add the import.
- [ ] **Step 3 — verify:** Vite compile clean; `playwright-cli` open + screenshot → the **Browser** dock renders + URL bar works; `console error` = 0; `tsc` clean.
- [ ] **Step 4 — commit:** `refactor(0b): extract BrowserPanel`.

### Task 2: CommandPaletteOverlay.svelte
**Files:** Create `src/lib/components/overlays/CommandPaletteOverlay.svelte`; Modify `+page.svelte` (section M `20389-20445`; state `1001-1004` `commandPalette*`; the command list/filter fns + `commandPaletteInput` ref).
**Interface:** Props `{ visible: boolean; query: $bindable<string>; index: $bindable<number>; commands: CommandItem[]; onSelect, onClose }`. Presentational (results computed in page, passed in). No teleport.
- [ ] **Step 1 — author:** move the overlay markup; props as above; move the `commandPaletteInput` `bind:this` + its focus `$effect` INTO the component (refs+effects move together).
- [ ] **Step 2 — integrate:** replace markup with `<CommandPaletteOverlay visible={commandPaletteVisible} bind:query={commandPaletteQuery} bind:index={commandPaletteIndex} commands={filteredCommandPaletteItems} onSelect={runCommandPaletteItem} onClose={closeCommandPalette} />`; import.
- [ ] **Step 3 — verify:** open palette (⌘K or its trigger) in the web preview via `playwright-cli` eval-click; screenshot shows it; arrow/enter work; `console error` = 0; `tsc` clean.
- [ ] **Step 4 — commit:** `refactor(0b): extract CommandPaletteOverlay`.

### Task 3: QuickOpenOverlay.svelte
**Files:** Create `src/lib/components/overlays/QuickOpenOverlay.svelte`; Modify `+page.svelte` (section L `20297-20387`; state `993-1000` `quickOpen*` + `workspaceSymbol*`; quick-open fns + `quickOpenInput` ref).
**Interface:** Props `{ visible; query: $bindable<string>; index: $bindable<number>; fileResults: QuickOpenItem[]; symbolResults: SourceWorkspaceSymbol[]; loading: boolean; onSelectFile, onSelectSymbol, onClose }`. Presentational; results computed in page. No teleport.
- [ ] **Step 1 — author:** move overlay markup + the `quickOpenInput` ref + its focus `$effect`.
- [ ] **Step 2 — integrate:** replace with `<QuickOpenOverlay … />`; import.
- [ ] **Step 3 — verify:** trigger quick-open; screenshot; file + symbol lists render; `console error` = 0; `tsc` clean.
- [ ] **Step 4 — commit:** `refactor(0b): extract QuickOpenOverlay`.

### Task 4: ActivityClipboardPanel.svelte
**Files:** Create `src/lib/components/panels/ActivityClipboardPanel.svelte`; Modify `+page.svelte` (paste-cleanup sub-panel `16340-16470`; state `920-923` `pasteCleanup*`; the paste-cleanup fns).
**Interface:** Props `{ input: $bindable<string>; replyDraft: $bindable<string>; mode: $bindable<PasteCleanupMode>; history: PasteCleanupHistoryItem[]; onClean, onCopy, onClear }`. Gated by `sourceActivityMode === 'clipboard'` (gate stays in page around the component). Self-contained.
- [ ] **Step 1 — author:** move the paste-cleanup markup + props.
- [ ] **Step 2 — integrate:** replace markup with `<ActivityClipboardPanel … />` inside the existing `{#if sourceActivityMode === 'clipboard'}` (or its current gate); import.
- [ ] **Step 3 — verify:** switch activity mode to clipboard via `playwright-cli` eval-click; screenshot renders; `console error` = 0; `tsc` clean.
- [ ] **Step 4 — commit:** `refactor(0b): extract ActivityClipboardPanel`.

### Task 5: HiddenDockRail.svelte + WorkbenchControls.svelte (chrome leaves)
**Files:** Create `src/lib/components/chrome/HiddenDockRail.svelte` (section I `20213-20237`) + `src/lib/components/chrome/WorkbenchControls.svelte` (section J `20240-20291`); Modify `+page.svelte`.
**Interface:** `HiddenDockRail` props `{ hiddenIDs: SourceDockPanelID[]; label: (id)=>string; onRestore: (id)=>void }`. `WorkbenchControls` props `{ presets, activePreset, viewMenuOpen: $bindable<boolean>, on* handlers }`. No teleport; no shared state beyond passed props.
- [ ] **Step 1 — author both** (file-disjoint → parallelizable).
- [ ] **Step 2 — integrate** each, one at a time.
- [ ] **Step 3 — verify:** hidden-dock rail buttons + view menu render/work; screenshot; `console error` = 0; `tsc` clean.
- [ ] **Step 4 — commit:** `refactor(0b): extract HiddenDockRail + WorkbenchControls`.

**Wave 1 gate:** `+page.svelte` shrinks ~700–900 lines; the component+teleport+overlay extraction pattern is proven green. Reassess before Wave 2.

---

## Wave 2 — Subsystem state modules + components (enable master-plan parallelism)

### Task 6: dockLayoutStore.svelte.ts  ⚠️ HIGH-CARE (foundation)
**Files:** Create `src/lib/stores/dockLayoutStore.svelte.ts`; Modify `+page.svelte`.
**Moves in (state):** `sourceActivityMode`(906), `sourceDockLayout`(951), pane widths/collapsed (943-950), layout presets (924-925), the 11 dockview workspace handles + host tokens (1023-1063), `unifiedWorkbenchSetPanelElement`(1027), context-card visibility (969-970). **Moves in (fns):** the 5 teleport action fns (12836-12908) + panel-visibility/resize fns (13929-14760).
**Interface:** `export const dock = $state({...})` + exported action factories `dock.panelAction`, `dock.filesPanelAction`, … (so `use:dock.panelAction={id}` works in any component). Effects that observe DOM (resize observers) STAY in the page/components, reading `dock`.
- [ ] **Step 1 — author** the module (state object + action fns + pure helpers). Keep `$effect`s OUT (leave in page).
- [ ] **Step 2 — integrate (serial, incremental):** repoint references in `+page.svelte` to `dock.*`; pass `dock.panelAction` (etc.) to the already-extracted components (BrowserPanel, etc.) instead of the old fn. Do it in small compile-green sub-steps (the references are many) — commit each green sub-step.
- [ ] **Step 3 — verify:** all panels still teleport + render (activity/editor/context/insights/terminal/browser); layout presets + resize work; screenshot each; `console error` = 0; `tsc`/`svelte-check` clean.
- [ ] **Step 4 — commit:** `refactor(0b): extract dockLayoutStore`.

### Task 7: gitStore.svelte.ts
**Files:** Create `src/lib/stores/gitStore.svelte.ts`; Modify `+page.svelte`.
**Moves in:** git status(819), worktrees(832), repo summaries(836), commit history(840), diff/staging(892-898) + the git/worktree fns (`grep -n` the range ~3899-7405; move pure data fns, leave any that touch terminal/snapshot). Writes `preview` cross to files state — keep that write as a passed callback until filesStore exists.
**Interface:** `export const git = $state({...})` + git action fns. Honor safety: never auto-mutate git; `planConversationRestore`-style cwd-only.
- [ ] **Step 1 — author** module. **Step 2 — integrate** (repoint refs, serial). **Step 3 — verify** activity-git + worktrees + insights-git render with data; screenshot; clean. **Step 4 — commit** `refactor(0b): extract gitStore`.

### Task 8: ActivityGitPanel.svelte + ActivityWorktreesPanel.svelte
**Files:** Create both under `src/lib/components/panels/`; Modify `+page.svelte` (git sub-panel `17486-18014`, worktrees `17244-17485`).
**Interface:** consume `gitStore` (`git.*`) + `projectStore` reads via prop `selectedProject` (projectStore not yet extracted) + their `use:*PanelAction` if any. Author in parallel (file-disjoint); integrate serially.
- [ ] author → integrate → verify (both panels render + actions work; screenshot) → commit `refactor(0b): extract Activity git + worktrees panels`.

### Task 9: intelligenceStore.svelte.ts + InsightsPanel.svelte + EditorPanel.svelte
**Files:** Create `src/lib/stores/intelligenceStore.svelte.ts`, `src/lib/components/panels/InsightsPanel.svelte` (section E `19360-19945`), `src/lib/components/panels/EditorPanel.svelte` (section D toolbar/tabs `18585-19999` shell around existing `MonacoSourceEditor`); Modify `+page.svelte`.
**intelligenceStore moves in:** LSP/symbols/diagnostics + definition/reference/impl/typedef result sets + search state (870-905) + LSP/search fns. EditorPanel keeps the `'editor'` teleport (18585) + Monaco-related `$effect`s/refs; InsightsPanel keeps the `'insights'` teleport (19379).
- [ ] author intelligenceStore + the two components (store first, then components in parallel) → integrate serially → verify editor renders a file + insights problems/symbols/refs nav works; screenshot → commit `refactor(0b): extract intelligenceStore + Insights/Editor panels`.

**Wave 2 gate:** the git, LSP/intelligence, and dock subsystems now live in separate files → master-plan Plan-2 lanes (git ‖ LSP) become file-disjoint. `+page.svelte` materially thinner.

---

## Self-Review
- **Coverage vs goal:** Wave 1 proves the pattern + shrinks the template (leaves). Wave 2 extracts the subsystems whose master-plan lanes (git, LSP, dock) must be file-disjoint for parallelism. Terminal + snapshot-restore deliberately excluded (Plan 1). projectStore/filesStore deferred (highest-churn — flagged for a careful follow-up). This satisfies "decompose enough to make the master plan parallel" without atomizing everything or colliding with Plan 1.
- **No-placeholder note:** this is an extraction plan — "code" = the cited source region moved verbatim; interfaces (props/exports) are specified per task. Executing agents must `grep -n` to confirm current line ranges before each cut (ranges shift as earlier tasks land) and re-verify the teleport nodes.
- **Type consistency:** teleport action prop is named `panelAction` everywhere; store objects are `dock` / `git` / `intelligence` (read as `store.x`); `$bindable` used for two-way overlay/input state.
- **Risk ordering:** safe leaves → dock foundation → subsystem stores+components. Each task independently revertable (green commit). Verify-live (screenshot + console) is mandatory because `tsc` does NOT catch Svelte template/CSS/teleport regressions.

## Execution
Recommended: **subagent-driven-development** — parallel author agents for new files, a single serial integrator for `+page.svelte`, two-stage review per task. Scope decision for the user at review: run **Wave 1 only** first (safest, proves pattern), or **Wave 1 + Wave 2** (full master-plan-parallelism enablement).
