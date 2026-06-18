# Phase 0b — Decompose `+page.svelte` — Implementation Plan (review-hardened)

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.
> **EXTRACTION plan, not greenfield.** Tasks MOVE existing markup + state + **its scoped CSS** into new files; they do not rewrite logic. "Code steps" cite the source line range to move + the target file + the component/module interface. `grep -n` to confirm CURRENT ranges before each cut — **ranges shift as earlier tasks land.**

**Goal:** Turn the 28,929-line `src/routes/+page.svelte` into a thinner orchestrator by extracting self-contained sections into focused `.svelte` components + safe shared state into `.svelte.ts` rune-state modules — proving the extraction pattern and creating file-disjoint seams, while deferring everything coupled to the (unextracted) files/project state to the master plan.

**Architecture:** State modules mirror `src/lib/settingsStore.svelte.ts` (module-level `const store = $state({...})`; consumers `import { store }` + read `store.x`; NEVER `export let x`+reassign). Components are presentational where possible (props) and read shared stores directly otherwise. Extract safe leaves first; then ONE safe state module + one clean panel.

**Tech stack:** SvelteKit (static SPA), Svelte 5 runes (verified svelte@5.56.3), TypeScript, Dockview teleport bridge, Tauri.

## Global Constraints
- **CSS MIGRATION IS MANDATORY AND PART OF EVERY EXTRACTION.** The page `<style>` is ~lines **20446–28929 (~97% scoped)**. Svelte hashes scoped selectors per-component, so when markup moves to a child its page styles **stop applying** (and the compiler prunes them as "unused"). Each task MUST move the section's matching `<style>` rules into the new component's own `<style>` **in the same change as the markup** (no intermediate unstyled state). Ancestor combos already written `:global(.source-dockview-*-shell .X)` must move to **`src/app.css`** (a child can't scope an ancestor it doesn't render). Screenshot-diff to confirm styling survived.
- **Verify every extraction live before commit:** Vite compile clean (no overlay) + `playwright-cli` screenshot of the affected panel rendering **correctly styled** + `console error` = 0 + `tsc`/`svelte-check` clean. Commit green per task; git is the revert net. (`tsc` MISSES `.svelte`/template/teleport/CSS regressions — the screenshot is non-negotiable.)
- **Single-file bottleneck = serial integration.** Every extraction edits `+page.svelte`. **ONE integrator agent holds the `+page.svelte` edit lock**; new files are authored in parallel, integrated one at a time.
- **NEVER break the teleport bridge, and it STAYS IN THE PAGE.** The template is wrapped by the live `<SourceDockviewShell>` host (15770→20199); `<SourceWorkbench bind:setPanelElement={unifiedWorkbenchSetPanelElement} bind:ready bind:error>` is at **~20205** and MUST stay in the page. The 5 teleport action fns (`sourceDockviewPanelAction` etc., 12836-12922) + their `registerSourceDockviewPanelElement`/`syncSourceDockviewPanelElement` chain read `unifiedWorkbenchSetPanelElement` AND call terminal hooks (`ensureEmbeddedTerminalRenderer`/`scheduleEmbeddedTerminalFit`) — **they stay in the page too.** A Wave-1 component containing a `use:sourceDockviewPanelAction={id}` node receives the action **as a prop** and keeps the node; the page passes its own fn. A lost action = silent blank panel.
- **`.svelte.ts` modules can't host `$effect`** (not component context). State → module; the `$effect`/ResizeObserver/timer that acts on it stays in a component; move a `bind:this` ref together with its effect.
- **DO NOT extract the terminal** (`embeddedTerminal*` 928-935/1064-1075, `sourceTerminalApp` 927, `embeddedTerminalAppearance` 794 + fns) — Plan 1 owns/rewrites it; leave a `// terminal: Plan 1 owns — do not extract` marker.
- **DO NOT extract anything coupled to files/project state in 0b** (see Deferred). Verified bidirectional coupling makes editor/insights/git unsafe without `filesStore` first.
- Never run parallel heavy builds/tests; never auto-mutate git.

## Parallelism model
- **Author lane (parallel, file-disjoint):** agents write brand-new `.svelte`/`.svelte.ts` files (move the cited markup + CSS + local state) + return the integration diff. Realistic gain is **Wave 1** (leaves are self-contained). Wave 2 authors must read the page (coupling), so parallelism there is limited.
- **Integrate lane (serial):** ONE integrator applies each into `+page.svelte`, verifies live, commits — in order.

## Extraction protocol (every task)
1. **Author:** create the new file; move the cited markup; **move its scoped `<style>` rules** into the component (ancestor `:global` combos → `app.css`); declare the interface (props/exports); for a teleport section keep the `use:...PanelAction` node + take the action as a prop; move any `bind:this` ref + its `$effect` together.
2. **Integrate (serial):** replace the page region with `<Component .../>` + import; delete the now-orphan page CSS rules; repoint state reads.
3. **Verify:** Vite compile clean + screenshot (panel renders + **styled**) + `console error` 0 + `tsc` clean.
4. **Commit green.**

## File Structure
**Create — Wave 1 (safe leaves):**
- `src/lib/components/panels/BrowserPanel.svelte` (G) · `panels/ActivityClipboardPanel.svelte` (paste-cleanup)
- `src/lib/components/overlays/CommandPaletteOverlay.svelte` (M) · `overlays/QuickOpenOverlay.svelte` (L)
- `src/lib/components/chrome/HiddenDockRail.svelte` (I) · `chrome/WorkbenchControls.svelte` (J)
**Create — Wave 2 (safe, limited):**
- `src/lib/stores/dockLayoutStore.svelte.ts` (layout STATE + pure helpers ONLY)
- `src/lib/components/panels/ActivityWorktreesPanel.svelte` (presentational; props)
**Modify (serial):** `src/routes/+page.svelte`, `src/app.css` (ancestor `:global` combos).

---

## Wave 1 — Safe self-contained leaves

### Task 1: BrowserPanel.svelte
**Files:** Create `src/lib/components/panels/BrowserPanel.svelte`. Move: markup section **G 20100-20194**; **CSS `.browser-dock*` ~25838-25943** (and move the `:global(.source-dockview-*-shell .browser-dock)` ancestor combos, e.g. ~25856, to `app.css`); browser state `browserUrl/browserInputUrl/browserFrameKey/browserError` (936-939) + fns (`grep -n` `submitBrowserUrl|reloadBrowserFrame|openBrowserUrlExternal`). `activeBrowserUrl` is a read-only `$derived` (keep in page, pass as prop).
**Interface:** props `{ url: string /* = activeBrowserUrl, NOT bindable */; inputUrl = $bindable(''); frameKey: number; error: string; panelAction: Action; onSubmit; onReload; onOpenExternal; onHide }`. Keep `use:panelAction={'browser'}`.
- [ ] Author (markup + CSS + props; keep teleport node). 
- [ ] Integrate: `<BrowserPanel url={activeBrowserUrl} bind:inputUrl={browserInputUrl} frameKey={browserFrameKey} error={browserError} panelAction={sourceDockviewPanelAction} onSubmit={submitBrowserUrl} onReload={reloadBrowserFrame} onOpenExternal={openBrowserUrlExternal} onHide={() => hideDockPanel('browser')} />`; delete orphan page CSS.
- [ ] Verify (Browser dock renders + styled + URL bar works). 
- [ ] Commit `refactor(0b): extract BrowserPanel (+CSS)`.

### Task 2: CommandPaletteOverlay.svelte
**Files:** Create `overlays/CommandPaletteOverlay.svelte`. Move: markup **M 20389-20445**; **CSS `.command-palette-*` ~28584-28761** (grep to confirm — command-palette & quick-open CSS are adjacent/interleaved, separate carefully); state `commandPalette*` (1001-1004) + the `commandPaletteInput` `bind:this` + its focus `$effect`; command list/filter fns. No teleport.
**Interface:** props `{ visible; query=$bindable; index=$bindable; commands: CommandItem[]; onSelect; onClose }` (results computed in page, passed in).
- [ ] Author → Integrate (`<CommandPaletteOverlay visible={commandPaletteVisible} bind:query={commandPaletteQuery} bind:index={commandPaletteIndex} commands={filteredCommandPaletteItems} onSelect={runCommandPaletteItem} onClose={closeCommandPalette} />`) → Verify (open via eval-click; styled; arrows/enter work) → Commit `refactor(0b): extract CommandPaletteOverlay (+CSS)`.

### Task 3: QuickOpenOverlay.svelte
**Files:** Create `overlays/QuickOpenOverlay.svelte`. Move: markup **L 20297-20387**; **CSS `.quick-open-*` ~28583-28752** (grep; separate from command-palette); state `quickOpen*`+`workspaceSymbol*` (993-1000) + `quickOpenInput` ref + focus `$effect`; quick-open fns. No teleport.
**Interface:** props `{ visible; query=$bindable; index=$bindable; fileResults; symbolResults; loading; onSelectFile; onSelectSymbol; onClose }`.
- [ ] Author → Integrate → Verify (file + symbol lists render + styled) → Commit `refactor(0b): extract QuickOpenOverlay (+CSS)`.

### Task 4: ActivityClipboardPanel.svelte
**Files:** Create `panels/ActivityClipboardPanel.svelte`. Move: markup **16340-16470** (inside its `{#if sourceActivityMode==='clipboard'}` gate — gate stays in page); **CSS `.paste-cleanup-*` ~20999-21098**; state `pasteCleanup*` (920-923) + fns. No teleport.
**Interface:** props `{ input=$bindable; replyDraft=$bindable; mode=$bindable; history; onClean; onCopy; onClear }`.
- [ ] Author → Integrate (inside the existing gate) → Verify (switch to clipboard mode; styled) → Commit `refactor(0b): extract ActivityClipboardPanel (+CSS)`.

### Task 5: HiddenDockRail.svelte + WorkbenchControls.svelte
**Files:** Create `chrome/HiddenDockRail.svelte` (markup **I 20213-20237**; CSS `.hidden-dock-panel-*` ~24376-24431) + `chrome/WorkbenchControls.svelte` (markup **J 20240-20291**, clean block closing before `</main>` 20292; CSS `.workbench-global-controls`/`.view-menu-*` ~24108-24192 + 24433-24448). No teleport.
**Interfaces:** `HiddenDockRail { hiddenIDs; label; onRestore }`; `WorkbenchControls { presets; activePreset; viewMenuOpen=$bindable; on* }` (page keeps `let viewMenuOpen` + its Escape handler at 9419, binds it).
- [ ] Author both (parallel) → Integrate each → Verify (rail buttons + view menu render/work + styled) → Commit `refactor(0b): extract HiddenDockRail + WorkbenchControls (+CSS)`.

**Wave 1 gate:** pattern proven (component + teleport-prop + overlay + CSS-migration, all green); `+page.svelte` + its `<style>` materially smaller. Reassess before Wave 2.

---

## Wave 2 — Safe limited (state-only module + one clean panel)

### Task 6: dockLayoutStore.svelte.ts — STATE + PURE HELPERS ONLY  ⚠️ (re-scoped per review C2)
**Files:** Create `src/lib/stores/dockLayoutStore.svelte.ts`. **Moves in (ONLY):** `sourceActivityMode`(906), `sourceDockLayout`(951), pane widths/collapsed (943-950), layout presets list (924-925), context-card visibility (969-970), + PURE layout helpers (no DOM, no terminal, no workbench binding).
**STAYS IN PAGE (do NOT move):** the 5 teleport action fns + `registerSourceDockviewPanelElement`/`syncSourceDockviewPanelElement` (12836-13573), the 11 dockview workspace handles + host tokens (1023-1063), **`unifiedWorkbenchSetPanelElement`(1027)** (it's the `<SourceWorkbench bind:>` target at 20205), all panel-visibility/resize `$effect`s + the fns that touch terminal/workbench (13929-14760 — move only the pure ones, if any; when in doubt leave in page).
**Interface:** `export const dock = $state({ activityMode, layout, paneWidths, presets, contextCardVisibility, ... })`; consumers read/write `dock.x`. No action fns on the store (so no `use:dock.panelAction`).
- [ ] Author the module (state + pure helpers). 
- [ ] Integrate (serial, small compile-green sub-steps — many refs): repoint `sourceActivityMode`→`dock.activityMode`, `sourceDockLayout`→`dock.layout`, etc.; commit each green sub-step. 
- [ ] Verify: all panels still teleport + render (activity/editor/context/insights/terminal/browser); presets + resize work; screenshot each. 
- [ ] Commit `refactor(0b): extract dockLayoutStore (state-only)`.

### Task 7: ActivityWorktreesPanel.svelte (presentational)
**Files:** Create `panels/ActivityWorktreesPanel.svelte`. Move: markup **17244-17485**; its scoped CSS (grep `.worktree-*` rules used only by this markup). Reviewer-confirmed clean: it only reads `fileActionBusy` for writes; worktrees DATA + handlers come in as props (worktrees state stays in page until the master plan's gitStore).
**Interface:** props `{ worktrees; summary; safetyStats; loading; fileActionBusy; on* handlers }`.
- [ ] Author → Integrate → Verify (worktrees panel renders + actions work + styled) → Commit `refactor(0b): extract ActivityWorktreesPanel (+CSS)`.

**Wave 2 gate:** layout state is a module; one more panel is out. STOP 0b here — the remaining extractions are files-coupled (below).

---

## Deferred to the master plan (NOT in 0b — review C3)
These read AND write `preview`/`selectedRecord`/`records`/tabs/`dirty` pervasively (verified bidirectional coupling), so they require **`filesStore` extracted first** — which is high-churn (referenced almost everywhere) and best done as the master plan's opening "data-spine" sub-phase, then:
- **`filesStore.svelte.ts`** + **`projectStore.svelte.ts`** (the spine — extract first, with extreme care, many ref repoints).
- **`gitStore.svelte.ts`** + **`ActivityGitPanel`** (writes `projectGitStatus` + `selectedRecord` → needs filesStore/callbacks).
- **`intelligenceStore.svelte.ts`** + **`InsightsPanel`** (writes `selectedRecord`/`preview` on def/ref/diagnostic clicks).
- **`EditorPanel`** (reads/writes `selectedRecord`/`preview`/tabs/dirty ~20×; its `{#each projectOpenSourceTabs}` per-tab teleport at 18597-18607 reads files state; the cited 18585-19999 range straddles into outer `{#if fileActionStatus}`/`{#if error}` — needs care).
- Remaining `Activity*Panel` leaves (runs/sessions/agents — read runtime/conversation state), `runtimeStore`, `conversationsStore` (Plan 1 snapshot overlap), terminal (Plan 1).
The master plan extracts `filesStore`/`projectStore` first, THEN these become clean — and only then are the master plan's git ‖ LSP lanes truly file-disjoint/parallel.

## Self-Review (review incorporated)
- **C1 fixed:** CSS migration is now a mandatory, first-class step in the protocol + every task, with rule locations.
- **C2 fixed:** Task 6 re-scoped to state + pure helpers; the teleport action fns + `unifiedWorkbenchSetPanelElement` + workspace handles + terminal hooks **stay in the page**; no `use:dock.panelAction`.
- **C3 fixed:** Editor/Insights/Git dropped from 0b → Deferred (need `filesStore`); only the cleanly-separable Worktrees panel kept in Wave 2.
- **Verified (reviewer I1):** Svelte 5 `use:` accepts member-expr + prop actions (compiles), so the Wave-1 `panelAction` prop pattern is sound; action read once untracked at mount with the page's stable fn.
- **Honest parallelism:** real parallel gain is Wave 1 (self-contained leaves); Wave 2 integration is serial and coupling-aware.
- **Type consistency:** teleport prop = `panelAction` everywhere; store object = `dock` (read `dock.x`); `$bindable` for two-way overlay/input/menu state. Line ranges are pre-shift — `grep -n` before each cut.

## Execution
**superpowers:subagent-driven-development** — parallel author agents for the Wave-1 new files; ONE serial integrator owns `+page.svelte`; two-stage review per task; screenshot-verify each. Run **Wave 1 first** (safest, proves the pattern), then Wave 2.
