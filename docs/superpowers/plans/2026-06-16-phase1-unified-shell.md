# Phase 1 — Unified Dockview Shell + Token System — Implementation Plan

> **For agentic workers:** This plan is executed **inline** (single session, visual verification), not via parallel subagents — the work centers on one 26k-line component and must be checked live in the running app. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Replace the homemade outer shell (CSS grid `.shell` + 4–5 separate Dockview instances + hand-rolled resizers + `sourcePaneSizing.ts`) with a **single unified Dockview workbench** (left / center / right / bottom groups, native resize/drag/persist, borderless), and introduce a **design-token system** — without breaking the running app.

**Architecture:** Build the new workbench as a parallel component behind a flag, render the existing panel DOM into it via the **existing `setPanelElement`/`attachPanelElement` bridge** (reuse, don't reinvent), verify it live, then switch over and delete the old shell + resizers + dead CSS. One `DockviewComponent` (existing `createDockview` path) hosts all regions as groups; the `sourceDockLayout` model (groups left/center/right/bottom) already matches this shape.

**Tech Stack:** SvelteKit + Svelte 5 runes, `dockview-core` 6.x (already installed), Monaco, xterm, lucide-svelte. No new dependencies in Phase 1.

## Global Constraints

- **No mobile.** Desktop only.
- **Clean break:** delete the homemade shell grid, resizers, and dead CSS — do not preserve/patch them.
- **Borderless, modern, minimal, spacious** — no grey gutters/seams; subtle native separators only.
- **pnpm is broken** (corepack shim v11.5.1 refuses pinned 10.28.2). Run tooling via local binaries, NOT `pnpm`:
  - Typecheck: `node_modules/.bin/svelte-kit sync && node_modules/.bin/tsc --noEmit`
  - Focused test: `node --experimental-strip-types scripts/<name>.test.mjs`
- **Web viewer** runs at `http://127.0.0.1:5177` (local `node_modules/.bin/vite --host 127.0.0.1`); **Tauri** via `node_modules/.bin/tauri dev --config src-tauri/tauri.dev.attach.conf.json`.
- **Commits only when the user asks**; when committing, branch first with task ids (`tsk-346-324-321-...`).
- **Verification = live visual proof** (web preview + Tauri attach) + clean `tsc` + `git diff --check`. Focused tests only for pure helpers.
- Keep the existing Svelte↔Dockview bridge contract: a panel registers its root element via `setPanelElement(panelID, element)`; the workbench attaches it into the Dockview host.

---

## File Structure

- **Create:** `tauri-svelte-preview/src/lib/styles/tokens.css` — design tokens (color/space/text/weight/radius/focus). Global, imported once.
- **Create:** `tauri-svelte-preview/src/lib/SourceWorkbench.svelte` — the single unified Dockview workbench (region groups, resize/drag/persist). Replaces the multi-shell + homemade-grid arrangement.
- **Modify:** `tauri-svelte-preview/src/lib/sourceDockLayout.ts` — confirm/extend the group model so one Dockview renders left/center/right/bottom; add a serialization helper for the unified layout if needed.
- **Modify:** `tauri-svelte-preview/src/lib/sourceDockviewWorkspace.ts` — add/extend a factory that builds ONE dockview with multiple region groups (vs. one group per shell).
- **Modify:** `tauri-svelte-preview/src/routes/+page.svelte` — mount `SourceWorkbench` behind a flag; register existing panel content via the bridge; then remove `.shell` grid markup, the resizer elements, and the dead CSS block.
- **Modify:** `tauri-svelte-preview/src/app.css` — import tokens; drop the hardcoded body background in favor of tokens.
- **Delete (within +page.svelte / lib):** homemade resizer markup + CSS (`.side-pane-resizer`, `.editor-insight-resizer`, `.context-pane-resizer`, `.bottom-dock-resizer`), `.shell` grid CSS, and the now-unused `sourcePaneSizing.ts` wiring (the file can stay if other code references its pure helpers; remove the resize wiring from the page).
- **Test (focused only):** `tauri-svelte-preview/scripts/sourceDockLayout.test.mjs` — extend if the group/serialization model changes.

---

## Task 1: Design-token foundation

**Files:**
- Create: `tauri-svelte-preview/src/lib/styles/tokens.css`
- Modify: `tauri-svelte-preview/src/app.css`

**Interfaces:**
- Produces: CSS custom properties on `:root` consumed by every later task and component: `--color-bg`, `--color-surface`, `--color-elevated`, `--color-text`, `--color-text-2`, `--color-text-3`, `--color-accent`, `--color-border`, `--color-focus`, status `--color-{live,good,bad,attention,idle}`, `--space-{1..6}` (4px base), `--text-{xs,sm,md,lg,xl}`, `--weight-{normal,medium,semibold,bold}`, `--radius-{sm,md,lg,pill}`, `--focus-ring`.

- [ ] **Step 1:** Create `tokens.css` with the scales above. Seed color values from the current palette so nothing visually regresses: bg `#191a21`, surface `rgba(255,255,255,0.045)`, text `#f2f6f5` / `#aab6b2` / `#8d9995`, accent `#5ce2cf`, border `rgba(255,255,255,0.08)`, focus `rgba(92,226,207,0.45)`, status live `#5ce2cf` / good `#8bdc9b` / bad `#f36f6f` / attention `#d8aa55` / idle `rgba(255,255,255,0.12)`. Spacing 4/8/12/16/20/24. Radius 6/8/10/999.
- [ ] **Step 2:** In `app.css`, `@import './lib/styles/tokens.css';` and change `body { background: var(--color-bg); color: var(--color-text); }`.
- [ ] **Step 3 (verify):** Reload the web preview — app looks identical (tokens mirror current values). Run `node_modules/.bin/svelte-kit sync && node_modules/.bin/tsc --noEmit` → clean. `git diff --check` → clean.

---

## Task 2: Unified workbench factory (one Dockview, region groups)

**Files:**
- Modify: `tauri-svelte-preview/src/lib/sourceDockviewWorkspace.ts`
- Modify: `tauri-svelte-preview/src/lib/sourceDockLayout.ts`

**Interfaces:**
- Consumes: existing `createDockview`, the `IContentRenderer` host pattern, `setPanelElement`/`attachPanelElement` bridge, and `SourceDockLayout` group model (`left|center|right|bottom`, panels `activity|editor|context|insights|terminal|browser` + `markdown`).
- Produces: `createSourceWorkbench(container, options): { api, setPanelElement, dispose, toJSON, fromJSON }` — builds ONE `DockviewComponent`, seeds region groups by adding the first panel of each group with `direction: 'left'|'right'|'below'` relative to center, then adds remaining panels `within` their group. Persists/loads the full `SerializedDockview`.

- [ ] **Step 1:** Add `createSourceWorkbench(...)` adjacent to `createSourceDockviewWorkspace`. Reuse the existing renderer/bridge functions (do not duplicate the host logic). Place groups: add `editor` (center) first; then `activity` `direction:'left'`; `context` `direction:'right'`; `terminal`/`browser` as tabs `within` center; `markdown` `within` center; `insights` `within` right. Set initial sizes from `sourceDockLayout` group `size`.
- [ ] **Step 2:** Add a layout version bump + a `serializeWorkbench`/`hydrateWorkbench` pair (thin wrappers over `api.toJSON()/fromJSON`) and a localStorage key `mac-command-bar.source-browser.workbench-layout`.
- [ ] **Step 3 (focused test):** Extend `scripts/sourceDockLayout.test.mjs` only if the group model changed (e.g., adding `markdown` as a center-default panel). Assert default placement + normalize invariants. Run `node --experimental-strip-types scripts/sourceDockLayout.test.mjs` → PASS.
- [ ] **Step 4 (verify):** `tsc --noEmit` clean. (No UI yet — wired in Task 3.)

---

## Task 3: Mount `SourceWorkbench` behind a flag; rehome panels

**Files:**
- Create: `tauri-svelte-preview/src/lib/SourceWorkbench.svelte`
- Modify: `tauri-svelte-preview/src/routes/+page.svelte`

**Interfaces:**
- Consumes: `createSourceWorkbench` (Task 2), tokens (Task 1).
- Produces: a mounted unified workbench rendering the existing panel DOM (activity sidebar content, Monaco editor, context cards, terminal, browser, markdown) via the bridge — selected by a `useUnifiedWorkbench` flag.

- [ ] **Step 1:** `SourceWorkbench.svelte`: a host `<div use:workbenchAction>`; in the action, call `createSourceWorkbench`, expose `setPanelElement` to the page (prop callback or context). Style the host borderless, `inset:0`, `background: var(--color-bg)`.
- [ ] **Step 2:** In `+page.svelte`, add `const useUnifiedWorkbench = true` (flag). When true, render `<SourceWorkbench bind:setPanelElement>` and register each existing panel's root element via `setPanelElement(panelID, el)` (the panel content markup stays; only its container host changes). When false, render the old `.shell` (unchanged) so we can A/B.
- [ ] **Step 3 (verify live):** With the flag on, reload web preview: confirm all regions render — activity (left), editor (center) with terminal/browser/markdown as center tabs, context/insights (right). Confirm **native** drag-between-groups, tab switching, and **dragging region borders resizes** (no homemade resizer). Check Tauri attach shows the same. Capture before/after.
- [ ] **Step 4 (verify):** `tsc --noEmit` clean; `git diff --check` clean.

---

## Task 4: Switch over — delete the homemade shell, resizers, and dead CSS

**Files:**
- Modify: `tauri-svelte-preview/src/routes/+page.svelte` (remove old shell markup + ~9k-line CSS block in stages)

- [ ] **Step 1:** Flip `useUnifiedWorkbench` permanent; delete the old `.shell` template branch (activity-shell grid, workspace-arrangement side/bottom flip, intelligence panel column) and the resizer elements (`.side-pane-resizer`, `.editor-insight-resizer`, `.context-pane-resizer`, `.bottom-dock-resizer`).
- [ ] **Step 2:** Delete the now-dead CSS for those selectors and the `.shell*` grid rules (work top-down through lines ~21094–25712 region by region; after each deletion, reload to confirm nothing the workbench needs disappeared).
- [ ] **Step 3:** Remove the resize wiring (`beginEditorInsightResize`, body `resizing-*` classes, `--side-pane-width`/`--context-pane-*`/`--bottom-dock-*` CSS var plumbing). Keep `sourcePaneSizing.ts` file only if still imported by surviving code; otherwise drop the import.
- [ ] **Step 4 (verify live):** Reload web + Tauri: borderless, no grey seams, spacious; all panels still function (open file, terminal, context cards). `tsc --noEmit` clean; `git diff --check` clean.

---

## Task 5: Borderless + spacing polish; persist layout

**Files:**
- Modify: `tauri-svelte-preview/src/lib/SourceWorkbench.svelte`, `sourceDockviewWorkspace.ts`, Dockview theme vars in `SourceDockviewShell.svelte` (or fold into workbench)

- [ ] **Step 1:** Set Dockview theme CSS vars from tokens (`--dv-*` ← `--color-*`); `--dv-separator-border: var(--color-border)` thin; remove heavy backgrounds; generous group padding using `--space-*`. Borderless tabs via custom tab look (full custom renderer is P4; here just clean the stock tabs).
- [ ] **Step 2:** Wire `onDidLayoutChange` → save `serializeWorkbench()` to localStorage; on init, `hydrateWorkbench()` if a valid layout exists (guard against panel-id drift, like the existing `...MatchesPanelPlans` check).
- [ ] **Step 3 (verify live):** Resize/drag/stack panels, reload — layout persists. Relaunch Tauri attach — layout persists. Confirm spacious, borderless, modern look. `tsc --noEmit` clean; `git diff --check` clean.

---

## Self-Review

- **Spec coverage:** Phase 1 of the spec = "unified Gridview/Dockview shell + token system, delete homemade grid/resizers/CSS, visible reskin." Tasks 1 (tokens) + 2–3 (unified workbench) + 4 (delete old) + 5 (borderless/persist) cover it. P2–P6 are out of this plan by design.
- **Placeholders:** none — file paths, token values, factory signature, flag name, verify commands are concrete.
- **Consistency:** `createSourceWorkbench` / `setPanelElement` / `serializeWorkbench` / `useUnifiedWorkbench` / workbench layout key used consistently across tasks. Panel ids match `sourceDockLayout` (`activity|editor|context|insights|terminal|browser|markdown`).
- **Risk:** if native inter-group resizing or panel rehoming misbehaves, Task 3's flag lets us A/B against the old shell before Task 4 deletes anything — nothing is removed until the new shell is proven live.
