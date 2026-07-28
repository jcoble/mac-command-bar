# Dockview Redesign — Design Spec

- Date: 2026-06-16
- Status: **Approved** (user-confirmed via brainstorming, 2026-06-16)
- Relates to: TSK-127, and the open layout cluster TSK-324 / TSK-321 / TSK-346 / TSK-280 / TSK-344 / TSK-360 / TSK-362
- Supersedes the "homemade dock model vs. migrate" open question in `docs/TSK-127-IMPLEMENTATION-PLAN.md` (Priority 0.1): **decision = migrate fully to Dockview.**

## Goal

Turn MacCommandBar's preview UI into a daily-driver, IDE-grade command center that is a cross between **Codex** (conversation rail), **VS Code** (borderless panes/tabs), **Warp** (terminal), and **CMUX** (multi-session). The defining capability: a **conversation is a live, persistent workspace** you switch between instantly with nothing lost.

## Governing principles

1. **Modern, minimal, spacious.** Generous whitespace, borderless panes, restrained iconography, clear typographic hierarchy. Not a dense dashboard of tiny cards.
2. **Fewer always-visible controls.** Prefer overflow (⋯) menus, right-click context menus, and the command palette over rows of buttons.
3. **Clean break from the old CSS.** The ~9k-line hardcoded CSS blob in `+page.svelte` (lines ~21093–30180), the homemade shell grid, the hand-rolled resizers, and the tacked-on intelligence panel are **deleted and rebuilt**, not patched.
4. **Visual proof over test sprawl.** Verify each phase live in the running app (web preview + Tauri attach). Add focused tests only to lock a specific behavior; do not add broad UI/source assertions (per handoff direction).
5. **No mobile.** Desktop only.

## Tech stack (decided)

| Concern | Choice | Notes |
|---|---|---|
| Panes / tabs / grids | **Dockview** (`dockview-core` 6.x), **Gridview-rooted** | Adopt for real; one unified shell. `createDockview`/`createPaneview` already used; **Gridview + custom tab renderers are NOT yet used** — this is the lever. |
| Controls | **Bits UI** (headless Svelte 5) + **own CSS design tokens** | Menus, dropdown, dialog, popover, tooltip, select/combobox, tabs, slider, switch, command. shadcn-svelte sits on Bits UI, so styles are borrowable later. No Tailwind. |
| Icons | **lucide-svelte** (already clean, 22 icons, no inline SVG) | Keep. |
| Editor | **Monaco** | Keep wiring; make appearance runtime-adjustable. |
| Terminal | **xterm** + addons | Keep wiring; make appearance runtime-adjustable; Warp/Dracula defaults. |
| Persistence | **localStorage** + extend **`WorkspaceSnapshotViewState`** | Existing pattern (60+ keys). `WorkspaceSnapshot` already stores open tabs, file/line, layout, terminal, browser URL. |

## Architecture

### Pillar 1 — Conversation = live, persistent workspace (the centerpiece)

Each conversation/session in the left rail owns a **complete, always-live environment**, kept resident so switching loses nothing:

- its **terminal (PTY)** with the **Codex/Claude/CMUX conversation still running** — never killed on switch;
- the **same editor files open** (Monaco models stay in memory);
- the **same file-explorer root + tree**;
- the **same git/source-control** target;
- the **same browser** tab/URL.

Switching conversations swaps the **visible** workspace; nothing is torn down. **Unlimited live conversations, instant switching.** Only an explicit per-conversation **Close** frees its PTY/state.

**Implementation model:**
- Keep **N PTYs alive simultaneously** in the Rust `TerminalRegistry` (`terminal.rs:44–47`, a `HashMap`). The current bug: switching **closes** the old PTY (`closeTerminalSessionFromTauri` on switch). **Fix: stop closing on switch; close only on explicit Close.**
- Each conversation has its **own Dockview layout** (its set of tabs/panes). Switching = swap which conversation's layout is mounted/visible; underlying PTYs, Monaco models, and browser state stay resident. (Approach A — per-conversation layout swap — chosen over a single tagged Dockview for clean isolation.)
- **Reuse `WorkspaceSnapshot`**: a conversation's workspace IS a snapshot kept **live** rather than serialize-and-rebuild. New conversation → hydrate from parsed session (`AgentSessionRecord`: CWD + repo + worktree + branch + `resume_commands`) and any saved snapshot. Already-open conversation → just re-show.
- **Restore must act on worktree + branch**, not only CWD. Today `branch_hint` and worktree are parsed (`sessions.rs:30–36`) then **ignored** — the terminal only `cd`s into `project_path`. Add: resolve/attach the worktree and check out the branch on hydrate, with a **repair path** when the saved worktree is gone (reuse existing missing-worktree repair guards).
- **Lazy + explicit:** a conversation hydrates on first open, then persists. Per-conversation **Close** is the only teardown. Surface resource cost honestly; default to keep-alive.

### Unified shell (Gridview root)

Replace `.shell` (homemade CSS grid, `+page.svelte:21094`) + the 4–5 separate Dockview shells + `sourcePaneSizing.ts` rail/collapse logic with a **single Gridview-rooted Dockview**:

- Root grid cells: **left / center / right / bottom**, each a Dockview group.
- Native resize / drag / stack / collapse / persist (`SerializedGrid` for the meta-layout). **Deletes** the homemade resizers (`.side-pane-resizer`, `.editor-insight-resizer`, `.context-pane-resizer`, `.bottom-dock-resizer`) and the invisible 10px hit-zones (the grey seams).
- Borderless throughout; separators are subtle/native.

### Left — activity rail + conversation switcher + per-active explorer/git

- Slim **activity rail** (icons + tooltip; lucide).
- **Conversation list** = the global switcher: Codex / Claude Code / CMUX, grouped + searchable, clean titles (not transcript noise). This is the Codex-style rail.
- **Explorer** and **Git** views are **per-active-conversation** (they follow the focused conversation's project/CWD/worktree). Replaces the cramped monolithic `.sidebar` grid.

### Center — Dockview tabs with custom modern renderers

- **Editor, Terminal(s), Browser, Markdown preview** as real Dockview tabs using **custom tab renderers** (lucide icon + label, borderless, overflow menu) for the Codex/VS Code look.
- `.md` files render in a **preview tab** (`SourceMarkdownPreview`), promoted from the side-card.
- Editor fills available space; terminal/browser fill their panels (no leftover bottom row).

### Right — cleaned context sidecar

- The 5 cards (Runs / Runtime / Agents / Worktrees / Git) become a proper **collapsible Paneview** with real section separation and breathing room (fix `.paneview-card-stack` density: card gaps, visible separators, per-card scroll).
- Per-card refresh/copy/X clutter → **one overflow (⋯) menu** + right-click context menus.
- **Git** gets a familiar **VS Code-style source-control view** (changes list, stage/unstage, diff, commit).

### Design system — tokens + Bits UI

- New `tokens.css` (or `:root` block): `--color-*` (semantic: bg/surface/elevated/text-{primary,secondary,tertiary}/accent/border/focus + status live/good/bad/attention/idle), `--space-*` (4/8 scale), `--text-*` (size scale), `--weight-*`, `--radius-*`, `--focus-ring`. Replace hardcoded hex/granular values.
- Tone system → shared palette via `[data-tone="..."]`, not per-class.
- Small Bits UI-based component set: `Button`, `IconButton`, `Chip`, `Badge`, `SearchInput`, `Select`/`Combobox`, `CollapsibleSection`, `Menu`/`ContextMenu`, `Dialog`, `Tooltip`, `Tabs`, `Slider`, `Switch`. Replace bare `<select>`, homemade segmented tabs, `.icon-button`/`.form-button`/`.scan-button`, etc.

### Settings & theming

- A **Settings surface** (Bits UI dialog/pane) with: **theme** selection, **app/editor/terminal** font family + size + line-height, and prefs (terminal app, etc.).
- **Applied live:** Monaco via the existing `applyAppearance()` (`MonacoSourceEditor.svelte:1170–1182`); xterm by mutating terminal options / re-fit (currently hardcoded `+page.svelte:7853–7887`).
- **Persisted** in localStorage (extend `WorkspaceSnapshotViewState` with `appearanceSettings`, or a dedicated key following the established pattern).
- Default = refined **dark** theme (Dracula-adjacent, current teal accent tokenized). Token system makes a **light theme** and alternates drop-in later.

### Scan-cache fix (Rust ↔ frontend)

- **Split the cache signature** into a **structure version** (file tree / paths) vs a **worktree-dirty version** (`dirtyStatusFingerprint`, `dirtySinceEpochMs`). Today *any* dirty change invalidates the whole entry, so **editing one file forces a full re-walk** even though the tree is unchanged (`sourceData.ts:2104–2346`, signature at `+page.svelte:3887–3920`). After the split, dirty-only changes **reuse the file list**.
- Add a **lightweight Rust-side stamp** (directory mtime / cheap pre-walk check) so repeated scans of an unchanged tree return fast — `list_source_files` is currently a pure re-walk with no memoization (`core/src/source.rs:143–192`).
- **Session list**: add caching + incremental update so `list_agent_sessions` stops re-reading all four providers (~500ms) on every call (`sessions.rs:49–100`, `main.rs:990–994`).

## Kept vs. deleted

**Kept:** Rust backend (scanners, terminal, lsp, git, worktrees, orchestration), Monaco + xterm wiring, session-scan logic (`sessions.rs`), lucide, data/business logic (`sourceData.ts`, `workspaceSnapshot.ts`, `tauriSource.ts`, `sourceDockLayout.ts`, `sourcePreviewAppearance.ts`).

**Deleted / rebuilt:** homemade shell CSS grid, hand-rolled resizers + `sourcePaneSizing.ts` rail logic, the tacked-on `.source-intelligence-panel`, the 4–5 separate Dockview shells (→ one Gridview shell), the ~9k-line hardcoded CSS, homemade controls.

## Phases

Each phase ends with: changed files, behavior delivered, **live visual proof** (web + Tauri attach), `git diff --check`, and only the focused tests needed.

- **P1 — Foundation.** `tokens.css` + Bits UI install; single **Gridview-rooted Dockview** shell (left/center/right/bottom), borderless, native resize/persist; delete homemade grid + resizers. Includes a visible reskin so progress is immediately apparent.
- **P2 — Controls.** Bits UI component set; replace old buttons/tabs/select/search/headers; tone-token migration.
- **P3 — Conversation live workspaces.** Conversation switcher; keep-N-PTYs-alive (stop kill-on-switch); per-conversation Dockview layout swap; full restore (CWD + repo + worktree + branch) with repair path; per-active explorer/git.
- **P4 — Center + right.** Custom tab renderers; markdown preview tab; right context Paneview cleanup; VS Code-style Git SCM view.
- **P5 — Settings & theming.** Settings surface; live-apply Monaco/xterm appearance; persisted prefs; refined dark theme + tokens ready for light.
- **P6 — Cache.** Structure-vs-dirty cache split; Rust dir-stamp; session-list caching/incremental.

## Risks / open questions

- **Resource use** with many live workspaces (N PTYs + Monaco models + browser state). Mitigation: keep-alive by default, explicit Close, lazy hydrate; revisit suspend strategy only if it bites (PTYs must stay live to keep convos running).
- **Browser panel persistence** across switches (Tauri WebView/iframe constraints). May keep URL + re-show rather than truly background a webview.
- **Gridview migration cost**: moving from per-region DockviewApi instances to one Gridview root is the biggest structural change; do it first (P1) and prove it before layering features.
- **`+page.svelte` size** (~26k lines): extract shell/components into focused files as we go; the file doing too much is itself a problem to reduce.

## Out of scope (parking lot)

Full browser-automation dashboard, heavy refactor/code actions, VS Code extension compatibility, cross-device sync, cloud state, destructive bulk cleanup without dry-runs, mobile.

## Key code anchors (for implementers)

- Shell grid: `+page.svelte:15490` (`.shell`), CSS `21094`+; resizers `21343`, `28476`, `25599`, `25663`.
- Dockview: `sourceDockviewWorkspace.ts` (`createSourceDockviewWorkspace` ~`496`, Svelte↔host bridge `506–596`, persistence `542–568`, storage key `23`); model `sourceDockLayout.ts` (`normalize` `64–112`, `move` `114–148`, descriptors `32–39`).
- Sessions: `core/src/scanners/sessions.rs:16–36` (record + derived), discovery `119–428`; command `main.rs:990–994`; bridge `localSourceFs.ts:109–153`; wrapper `tauriSource.ts:717–724`.
- Terminal: `terminal.rs` (registry `44–47`, start `335–367`, reader `265–289`, close `220–238`); frontend `tauriSource.ts:386–462`.
- Right panel: `+page.svelte:18258–26096`, CSS `25893–26059`, card data stores `1632–1637`, visibility `896–897`.
- Scan cache: `core/src/source.rs:26–192`; `sourceData.ts:351–363, 2104–2346`; signature `+page.svelte:3887–3920`; storage `+page.svelte:321`.
- Appearance: `sourcePreviewAppearance.ts:24–146`; Monaco apply `MonacoSourceEditor.svelte:1170–1182`; xterm `+page.svelte:7853–7887`; persistence `workspaceSnapshot.ts:58–119`.
