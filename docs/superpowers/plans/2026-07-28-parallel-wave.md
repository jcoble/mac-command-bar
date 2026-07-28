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

<!-- LANE SPECS INSERTED BELOW BY CONTROLLER AFTER RECON -->

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
