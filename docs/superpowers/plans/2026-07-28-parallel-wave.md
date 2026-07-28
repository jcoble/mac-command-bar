# Parallel Wave — Implementation Plan (lanes on the walking skeleton)

> **For agentic workers:** This plan is executed as a Workflow fan-out (parallel lanes) + one serialized integration task — not the one-task-at-a-time SDD loop. Each lane's spec below is that lane's complete brief.

**Goal:** Land the bulk of the remaining system on the `/next` skeleton in one wave: code reading (Monaco + LSP), git panel, right-context cards, file explorer, browser panel, settings/theming, command palette.

**Architecture:** Every lane produces ONLY new files (one directory per lane under `src/lib/shell/<lane>/` plus components under `src/lib/shell/components/`). No lane edits a shared file — the page, `ShellFrame.svelte`, `centerDock.ts`, `frame.ts`, `terminalService.ts`, and the stores are **frozen seams** during the wave. A single integration task afterwards wires every lane's exported panel into the shell and runs the one `tsc` pass.

**Tech stack:** Svelte 5 runes, dockview-core 6.6.1 (already mounted), monaco-editor 0.55.1, existing Rust backend (NO src-tauri/ or core/ changes — the dev watch is live and a rebuild kills live PTYs).

## Global Constraints (every lane inherits these verbatim)

- **New files only.** A lane that believes it must edit a shared file reports the need in its output instead of editing — the integrator owns shared files. The ONE exception: a lane may append to its own lane directory freely.
- **Old shell frozen:** `src/routes/+page.svelte` read-only (and it carries an uncommitted user diff — never stage, commit, or revert it; `.vscode/` stays untracked). Old `$lib` modules are quarry: copy code out only after the poison audit (no IO at import/mount; no `$effect` that calls backend; no hidden singletons; in-flight guards on multi-call-site backend work — see `docs/superpowers/notes/2026-07-28-old-shell-autopsy.md`).
- **Constitution:** effects never do IO — backend calls live in explicit service functions, invoked imperatively, counted via `countInvoke` (import from `$lib/shell/devInvokeCounter.svelte`). Nothing hydrates at launch: a lane's data loads on first user activation of its panel, never at mount of the shell. LSP spawns only when a code view is opened by user action.
- **No builds in lanes:** lanes run ONLY their own pure node test scripts (`node --experimental-strip-types scripts/<lane>.test.mjs`). No `pnpm run check`, no vite build, no cargo — the integrator runs `check` once at the end.
- **No commits in lanes.** Lanes leave their files in the working tree and report the file list; the integrator commits lane-by-lane, plain messages, **no Co-Authored-By trailer**.
- **Plain English** in every user-visible string (no internal jargon like "lane", "wave", "hydrate").
- localStorage keys: `mac-command-bar.next.<lane>.*`, quota-safe writes only (reuse `saveLayout`-style try/catch or `layoutStorage.ts` helpers where they fit).
- Palette: bg `#101014`, raised `#17171d`, border `#22222c`, text `#d8d8e0`, muted `#6d6d7d`, faint `#4c4c5a`, terminal bg `#282a36` (Dracula). Fonts: ui-sans-serif system stack; mono `ui-monospace, Menlo`.

## The frozen seams (the integration contract)

Lanes code against these interfaces exactly as they exist on the branch today; the integrator is the only writer of the files that implement them.

1. **Panel contract (center tabs)** — a lane deliverable is a Svelte component with NO required props, self-contained, that fills `height:100%/width:100%` and renders lazily (does nothing until visible/activated). The integrator adds it to the page as a snippet and to `createCenterDock`'s roster (`CenterPanelSpec { id, title, element }`). Panels must tolerate `display:none` hosts (the parking stage) and never measure themselves while hidden.
2. **Region contract (context / dock)** — same as the panel contract; the integrator teleports it into the `context` or `dock` region snippet.
3. **Activation signal** — a lane needing "load on first show" exports from its service module: `activate(): void` (idempotent, first call does the initial fetch) — the integrator calls it from the dockview panel's first activation (via `onPanelLayout`/active-panel events it already owns). Until `activate()` is called, the component renders an inert empty state.
4. **Backend access** — via existing wrappers in `$lib/tauriSource.ts` ONLY (plus `invoke` for commands that have no wrapper yet — noted per-lane below). Every backend call is wrapped: `countInvoke('<command_name>')` immediately before it.
5. **Open-file bus (explorer ↔ editor, palette ↔ editor)** — the ONE new shared module, created by the controller BEFORE fan-out so both lanes import it read-only: `src/lib/shell/openFileBus.ts` (spec in Pre-work below).
6. **State** — per-lane rune stores live in the lane's own directory (`src/lib/shell/<lane>/<lane>Store.svelte.ts`), never in `src/lib/shell/stores/`.

## Pre-work (controller, before fan-out — single commit)

`src/lib/shell/openFileBus.ts` — pure, ~40 lines, no IO:

```ts
/** Cross-panel requests: "open this file (at this line) in the editor". */
export interface OpenFileRequest {
  path: string;          // absolute path
  line?: number;         // 1-based
  column?: number;       // 1-based
}
type Listener = (request: OpenFileRequest) => void;
const listeners = new Set<Listener>();
let pending: OpenFileRequest | null = null; // last request before the editor subscribed

export function requestOpenFile(request: OpenFileRequest): void {
  if (listeners.size === 0) {
    pending = request;
    return;
  }
  for (const listener of listeners) listener(request);
}

export function onOpenFile(listener: Listener): () => void {
  listeners.add(listener);
  if (pending) {
    const request = pending;
    pending = null;
    listener(request);
  }
  return () => listeners.delete(listener);
}
```

## Lanes

(Filled per-lane from recon: deliverable, files, backend surface with exact command/wrapper names, quarry pointers with poison notes, test script scope, acceptance.)

### Lane E — Editor / code reading (Monaco + LSP) — the big lane

**Deliverable:** a working code-reading panel: open file → Monaco renders it read-only(+editable later), go-to-definition, peek references, hover, code lens with reference counts, document symbols. Subscribes to the open-file bus.

**Reuse decision (recon-verified):** `src/lib/MonacoSourceEditor.svelte` is REUSED AS-IS (imported from its current location — it passed the poison audit: no backend IO, two trivial `$effect`s, thorough disposal, all language intelligence arrives via callback props). Do NOT copy or fork it. The lane builds the service that feeds its callbacks.

**New files only:**
- `src/lib/shell/editor/sourceIntelligence.ts` — the service owning `{projectRoot, activePreview, draftContent}` and exporting the callback set for MonacoSourceEditor. Port from the old page (READ `src/routes/+page.svelte`, never import): `findSourceDefinitionTargetsForEditor` (:8446), `findSourceReferenceTargetsForEditor` (:8486), `countSourceReferencesForCodeLens` (:8548 — port VERBATIM including `codeLensReferenceCountTimeoutMs = 700`, `maxCodeLensNativeReferenceScanRecords = 1500`, and the isNativeTauriRuntime size-cap guard; this function encodes regressions cad76d3 + 52ad343/0593c20), `loadEditorExternalSourcePreview` (:8607) with its `externalPreviewCache` (invalidated on save; expose `invalidatePreview(path)` for future lanes), plus the simple hover/completion/highlight/semantic-token handlers (:10104-10214). Three-tier pattern per lookup: LSP → native text scan (`findSourceDefinitionsFromTauri`/`findSourceReferencesFromTauri`) → give up (NO demo-data tier in /next). `countInvoke` before every backend call.
- `src/lib/shell/editor/sourceRecordFromPath.ts` + `scripts/sourceRecordFromPath.test.mjs` — PURE: port `sourceRecordFromRestoredPath` (old page :6769, ~30 lines): path + projectRoot → synthesized `SourceRecord {path, relativePath, fileName, language, byteCount}`. This is the "open file by path" bridge; the lane does NOT need the project scan.
- `src/lib/shell/editor/editorStore.svelte.ts` — open files (ordered), active path, per-file {preview, targetLine, targetLineRequestId, loading}.
- `src/lib/shell/components/EditorPanel.svelte` — thin: open-file strip (simple buttons this slice, not dockview tabs), MonacoSourceEditor with the service's callbacks, empty state "Open a file from the explorer or palette". Subscribes `onOpenFile` from `$lib/shell/openFileBus`: request → `sourceRecordFromPath` → `readSourceFromTauri(record)` (NOTE: the wrapper re-overlays relativePath/language/byteCount from the record — always build the record first) → store → render. `warm_source_lsp_for_root`: call `warmSourceLspForRootFromTauri(root)` once per root on first file-open of that root (idempotent, no-op if nothing running).
- `scripts/editorStore.test.mjs`.

**Hard constraints:** ONE MonacoSourceEditor instance ever (Monaco providers are registered process-globally; two instances = the last one wins all lookups — recon-verified hazard). Model switching, not editor duplication. LSP spawns only from user file-open (constitution). No `$effect` calling the service; UI events call it imperatively.

### Lane G — Git panel (source control)

**Deliverable:** a VS Code-style source-control panel component + a diff viewer, live against the active session's project root.

**New files only:**
- `src/lib/shell/git/gitService.ts` — imperative loaders + actions. Wraps the EXISTING `tauriSource.ts` functions (`readProjectGitStatusFromTauri(root)`, `readSourceGitDiffFromTauri(root, path)`, `stageGitPathsFromTauri`, `unstageGitPathsFromTauri`, `commitGitRepositoryFromTauri`, `fetchGitRepositoryFromTauri`, `pullGitRepositoryFromTauri`, `pushGitRepositoryFromTauri`, `readGitCommitHistoryFromTauri(root, limit≤80)`). Every call: `countInvoke('<command>')` first. Copy the request-guard shape from the old shell (`+page.svelte:3776-3801` monotonic request id + superseded-bail) — imperative only, no `$effect`. `null` return = not-Tauri: render an inert "native only" state, DO NOT fake demo data.
- `src/lib/shell/git/gitPanelStore.svelte.ts` — rune store; start from old `src/lib/stores/gitStore.svelte.ts` (105 lines, audited clean — pure value bag, reusable nearly verbatim; keep its no-effects rule).
- `src/lib/shell/git/parseUnifiedDiff.ts` + `scripts/parseUnifiedDiff.test.mjs` — PURE: parse unified-diff text (`SourceGitDiff.diff`) into `{ before: string, after: string, hunks: [...] }` for Monaco's diff editor; handle new/deleted/binary (`isBinary`) files. This is the lane's real test surface.
- `src/lib/shell/components/GitPanel.svelte` — changes list (staged/unstaged groups, per-file badge strings from the backend — `GitFileStatus.status/.badge` are pre-computed, never parse porcelain), stage/unstage/commit (commit message input), fetch/pull/push row, commit-history list (limit 24). Quarry: `ActivityGitPanel.svelte` props contract (`GitPanelData`/`GitPanelFormatters`/`GitPanelActions` at :94-170) — copy the grouping idea, NOT the 1777-line chrome.
- `src/lib/shell/components/GitDiffView.svelte` — renders a selected file's diff. Phase 1: readable unified text with syntax-neutral +/− coloring; Phase 2 (same lane, if time): Monaco `createDiffEditor` fed by `parseUnifiedDiff`. Lazy-import monaco ONLY when a diff is first shown.
- `scripts/gitPanelStore.test.mjs` — store mutations + guard logic.

**Constraints:** project root comes from the active owned session (`rail.owned` active entry's `projectPath`/`cwd`) — passed IN by the integrator via `activate(root)`; the lane must not import sessionRailStore. No branch-list/checkout UI (no backend command exists — do not add one). After every mutating action, re-read status (the backend returns fresh status in the action result — use it).

### Lane X — Right context cards

**Deliverable:** the Runs / Runtime / Agents / Worktrees / Git-summaries card stack for the context region.

**New files only:**
- `src/lib/shell/context/contextService.ts` — imperative loaders wrapping `listRuntimeContextsFromTauri(projects)`, `listProjectWorktreesFromTauri(root)`, `listOrchestrationRunsFromTauri(projects)`, `listGitRepositorySummariesFromTauri(projects)`, `listAgentSessionsFromTauri()` (+ local-bridge fallback, reuse the pattern in `routes/next/+page.svelte` scanRail — read it, do not import the page). `countInvoke` on every call. NO polling this slice — refresh happens on `activate()` and an explicit refresh button only.
- `src/lib/shell/context/contextStore.svelte.ts` — rune store for the five card states (the old shell kept 4 of 5 in page-locals — this store is their new home).
- `src/lib/shell/components/ContextPanel.svelte` — quarry HARD from `src/lib/WorkbenchContextPanel.svelte` (568 lines, audited CLEAN: props-only, no effects, no IO; its prop types are minimal structural shapes that accept raw backend types). Copy it nearly verbatim, restyle to the /next palette, keep `ROW_LIMIT = 6` + "+N more".
- `scripts/contextStore.test.mjs`.

**Constraints:** `projects` arrays come in via `activate(input: { projects, activeRoot })` from the integrator. Worktree cards are read-only this slice (no remove/archive buttons — the safety flow is its own future lane; `worktreeSafety.ts` etc. stay untouched).

### Lane F — File explorer

**Deliverable:** a file-tree panel for the active session's project, click-to-open via the open-file bus.

**New files only:**
- `src/lib/shell/explorer/explorerService.ts` — one scan path: `listSourceFilesFromTauri(root, query, limit, scanId)` with `createSourceScanId()` (`tauriSource.ts:321`), and on supersede `cancelSourceScanFromTauri(scanId)` — cancel the BACKEND walk, not just discard results (regression cd2f525). `countInvoke` per call. NO scan cache this wave (the old localStorage cache is uncapped and silently dies on big repos — recorded deferred; scan happens on `activate(root)` and on an explicit refresh only).
- `src/lib/shell/explorer/explorerStore.svelte.ts` — records, tree expansion set, scroll state, scanning/error.
- `src/lib/shell/components/ExplorerPanel.svelte` — REBUILD the UI (do NOT reuse `ActivityFilesPanel.svelte` — 45 props, old-store coupling). Tree building/flattening/virtualization: reuse VERBATIM the pure helpers `buildSourceTree` / `flattenSourceTree` / `virtualizeSourceTreeRows` / `scrollTopForSourceTreeReveal` / `folderIdsForSourceRecord` from `$lib/sourceData` (:2688-2782 — import, don't copy). Click on a file → `requestOpenFile({path})` from `$lib/shell/openFileBus` (this lane is the bus's first producer).
- `scripts/explorerStore.test.mjs` — expansion/virtualization interplay on a synthetic tree.

**Known scope limits (state them in UI copy where visible):** the backend lists ~70 source-file extensions only (images/lockfiles/unknown never appear) and skips `worktrees/`, `node_modules/` etc. — it is a code index, not a general file manager. No file watcher exists; refresh is manual.

### Lane B — Browser panel

**Deliverable:** the iframe browser panel with URL bar, reload, and per-`/next` URL persistence.

**New files only:**
- `src/lib/shell/browser/browserStore.svelte.ts` — url, inputUrl, frameKey, error; persists url to `mac-command-bar.next.browser.url` (quota-safe).
- `src/lib/shell/browser/normalizeBrowserUrl.ts` + `scripts/normalizeBrowserUrl.test.mjs` — PURE: lift `normalizeBrowserDockUrl` VERBATIM from old page :7136-7155 (accepts `:5177`, `localhost:5177`, full URLs; rejects non-http(s)).
- `src/lib/shell/components/BrowserPanel.svelte` — start from the old `src/lib/components/panels/BrowserPanel.svelte` (audited CLEAN, pure presentational): keep the `{#key frameKey:url}` iframe remount + sandbox attrs verbatim; drop the `panelAction` prop (the new dock has its own teleport); inline the `.file-action-button` styles it needs (they live in old `app.css` — copy the rules into the component, do not touch app.css).

**Constraint:** the iframe's page state survives tab switches for free because the dock keeps panels attached (`defaultRenderer:'always'`); nothing may remount the iframe except the explicit reload button (frameKey bump).

### Lane P — Command palette

**Deliverable:** Cmd+K/Cmd+Shift+P palette over a proper action registry (the old shell's 131 inline page-closured commands are NOT liftable — greenfield registry, seeded small).

**New files only:**
- `src/lib/shell/palette/commandRegistry.ts` + `scripts/commandRegistry.test.mjs` — PURE: `PaletteCommand { id, label, detail, disabled?(): boolean, perform(): void | Promise<void> }`, `registerCommands(source: string, commands: PaletteCommand[])` (idempotent per source), `allCommands()`, `filterCommands(query, limit = 12)` (copy the old text-match: label+detail substring, slice 12).
- `src/lib/shell/components/PalettePanel.svelte` — a thin host that mounts the OLD `src/lib/components/overlays/CommandPaletteOverlay.svelte` REUSED AS-IS (audited clean: 7 props, self-contained `CommandItem` type, zero old-shell deps) + a `<svelte:window>` keydown handler for Cmd/Ctrl+K and Cmd/Ctrl+Shift+P (copy the chord checks from old page :8905-8912; Escape closes).
- Seed registry (in the component, not the page): `reset-layout`, `rescan-sessions`, `open-settings` — wired via callbacks passed from the integrator; everything else registers in later waves.

**Constraint:** the palette executes registered commands only — it must not import any other lane's module (commands arrive via `registerCommands` at integration).

### Lane S — Settings & /next theming

**Deliverable:** the Settings dialog live on `/next` + a `/next`-scoped design-token stylesheet.

**Facts (recon-verified):** `src/lib/settingsStore.svelte.ts` is complete and REUSED VERBATIM (module `$state` + `$effect.root` auto-persist to `mac-command-bar.settings`; safe merge). `src/lib/SettingsPanel.svelte` (Bits UI Dialog+Tabs) is droppable as-is but styled in `--color-*` tokens. `src/lib/styles/tokens.css` EXISTS, is globally loaded, and carries a DIFFERENT palette than /next — it is shared with the old shell and is FROZEN for this wave.

**New files only:**
- `src/lib/shell/styles/nextTokens.css` — the /next palette as `--color-*` values under a `.next-shell` scope (so SettingsPanel and future Bits components render correctly INSIDE /next without recoloring the old shell): bg `#101014`, surface `#17171d`, border `#22222c`, text `#d8d8e0`, text-2 `#6d6d7d`, text-3 `#4c4c5a`, plus the status tones copied from tokens.css. Document each mapping.
- `src/lib/shell/components/SettingsHost.svelte` — mounts the reused `SettingsPanel` with `bind:open`, exposes `open()`; nothing else.
- `scripts/nextTokens.test.mjs` — sanity: the css file parses (regex-level) and defines every token name SettingsPanel consumes (grep the list from SettingsPanel.svelte first).

**Explicit non-goals this wave (recorded deferred):** live-applying settings to running xterm instances (construction-time application already works via `xtermFactory.terminalAppearance()`); Monaco theme switching (the appearanceOverride prop has no theme field); the five decorative knobs in SettingsPanel stay decorative — add `(not wired yet)` to their labels IF that is a ≤5-line edit to a COPY; otherwise leave and record.

## Integration task (serialized, after all lanes)

1. Commit each lane's files (one commit per lane, plain message).
2. Wire panels: editor + git + browser (+ markdown if delivered) into the center roster; context cards into the context region; palette mounted at page level (keyboard-driven overlay); settings entry point.
3. Wire the open-file bus into the editor component; wire explorer + palette to `requestOpenFile`.
4. Wire per-panel `activate()` calls on first activation.
5. `pnpm run check` + all `test:*` node suites + live smoke against the running vite (compiled-module fetches).
6. The page may exceed the ~300-line cap only by what snippet registration costs; anything more gets extracted to components.
7. Final commit set pushed; milestone review + user live pass.

## Perf gates (unchanged from the constitution)

Cold launch hydrates nothing but the session rail. Opening the shell adds ZERO backend calls from any new lane until its panel is first activated. The invoke counter is the proof and the review lens.
