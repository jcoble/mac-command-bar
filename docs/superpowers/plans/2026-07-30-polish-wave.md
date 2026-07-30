# Polish wave — 2026-07-30 (TSK-798)

User feedback dump after live-testing PRs #9–#11. Eight parallel implementation lanes, one
integrator, at most ONE milestone review. Cost directive in force: tight prescriptive work, no
review stacking, controller applies review fixes directly.

## Standing rules (paste-level: every lane obeys ALL of these)

- Work happens on branch `tsk-798-polish-wave` in the primary checkout
  `/Users/blackcolours/dev/work/mac-command-bar`. **Lanes never run git commands** — no add,
  commit, stash, checkout, worktree. Write files, verify, report. The integrator commits.
- **FROZEN:** `tauri-svelte-preview/src/routes/+page.svelte` (the OLD shell). Never edit it. It
  also carries the user's own uncommitted diff — never stage or revert anything.
- Each lane edits ONLY the files listed as owned in its section, plus NEW files under its named
  folders, plus its integration note. Anything else you need changed goes in the note.
- Write an integration note at
  `.superpowers/sdd/2026-07-30-polish-wave/<lane>-INTEGRATION.md`: what you built, exactly what
  the integrator must wire (file + where + code snippet), settings keys you expect, and how to
  verify in the browser.
- Verify before reporting done: `cd tauri-svelte-preview && node scripts/checkSvelteNext.mjs`
  must pass (it runs svelte-check over owned paths AND fails any font-size under 12px).
  Do NOT run `pnpm build` (unsafe concurrently); the integrator runs it once.
- UI rules: shadcn components from `$lib/components/ui/*` (Dialog, AlertDialog, DropdownMenu,
  Collapsible, Badge, Input, Tooltip already vendored — check `src/lib/components/ui/`). Never
  `window.confirm`/`alert` — Tauri's webview silently ignores them; use AlertDialog. Fonts:
  explicit `text-[13px]` body / `text-[12px]` floor (Tailwind `text-sm` here is 12px — avoid
  named sizes). Colors ONLY via `var(--color-…)` tokens, no hex. bits-ui trap:
  `Collapsible.Content` is hidden via the `hidden` attribute — put layout classes on an inner
  div, never a display class on Content itself.
- ALL user-visible copy is plain English a non-programmer can read. No invented jargon.
- Commit messages are the integrator's job; no Co-Authored-By trailers anywhere, ever.
- Backend calls: Tauri drops unknown payload keys silently. New commands/fields are only used
  when their capability name appears in `read_backend_capabilities()` (see Lane H contract);
  otherwise the control is disabled with an honest tooltip ("This build of the app cannot do
  this yet — restart the desktop app after updating.").

## Backend contract (Lane H implements; frontend lanes code against it, capability-gated)

New capability names returned by `read_backend_capabilities()`:
- `referenceCounts` — command `count_source_references { root, symbolNames: string[] }` →
  `{ counts: [{ name, count, approximate }] }`. One project pass for ALL names, case-sensitive.
- `processKill` — command `kill_process { pid }` → `{ ok, message }`. SIGTERM; refuses pid ≤ 1
  and its own pid; message says what happened in a sentence.
- `worktreePruneSingle` — `remove_project_worktree` on a folder-gone row clears ONLY that row's
  `.git/worktrees/<id>` metadata instead of running repo-wide `git worktree prune`.
- Commit history limit: backend accepts `limit` up to 500.

## Lane A — Sessions column

Owns: `tauri-svelte-preview/src/lib/shell/components/SessionsColumn.svelte`, new files under
`tauri-svelte-preview/src/lib/shell/components/sessions/`.

The problems (screenshots 43/45 vs T3 Code 44):
1. Selecting/growing one card scrunches every other card to a sliver. Cards must keep natural
   height; the COLUMN scrolls. Kill whatever flex rule lets cards compress below content height
   (`min-height: 0`/`flex-shrink` on cards); Working and Done become independent collapsible
   panes (Collapsible), each its own scroll region inside the scrolling column.
2. Readability: card titles full contrast `var(--color-text)` at 13px, secondary line 12px
   `--color-text-2` — audit the whole column; nothing dimmer than text-2 for information the
   user must read. More vertical breathing room per card (T3 look), status conveyed with a
   colored dot + word, not dimness.
3. Richer cards: title, agent chip (exists), plus branch/worktree name if known, relative time
   ("2h ago"), message count when the session store has it, and a status word (Running / Done /
   Idle). Look at what the session store already carries before inventing fields.
4. Expandable detail: clicking a chevron on a row expands an inline detail block (like the
   worktree rows do) — full first prompt, project path, started time, message count, and the
   action buttons currently crammed on the card.
5. Integration note: ask the integrator to call `showPanel('session')` after a session is
   started, resumed, or restarted (the hooks live in `/next/+page.svelte` — name the exact
   handlers you find referenced from SessionsColumn props).

## Lane B — Terminal rendering, workspace switching, problems dock location

Owns: `components/TerminalSurface.svelte`, `layout/centerDock.ts`, `layout/sidebarViews.ts`,
`sessionWorkspaces.ts`, `components/DockPanel.svelte`, `settingsStore.svelte.ts` (this lane is
the ONLY one allowed to edit settingsStore this wave — add BOTH keys below).

1. New/resumed session tab renders wrong (screenshots 46, 52): prompt cursor drawn above the
   input line, input box top clipped, content cut off at the bottom — classic xterm fit against
   a container measured while hidden or mid-layout. Find the fit path in TerminalSurface;
   ensure fit()+PTY resize re-runs when the panel becomes visible/active and after the dockview
   panel's own resize settles (ResizeObserver + a fit on visibility change; don't trust one
   rAF after mount). Verify by: create session → tab renders with prompt at bottom, no clip;
   resize window; switch tabs and back.
2. Diff panel does not follow the active session's project: when the workspace switches
   (`sessionWorkspaces.ts` snapshot/restore), the diff panel keeps showing the previous
   project's file. Make the diff panel's subject part of the per-session snapshot, or clear it
   on restore when its file belongs to another root — whichever the existing snapshot shape
   supports more honestly.
3. Problems dock location setting: `settings.panels.problemsLocation: 'bottom' | 'right' |
   'hidden'` (default 'bottom'). 'right' renders the Problems panel as a right tool-column view
   (add a `problems` SidebarViewId + icon in ActivityBar — coordinate via integration note if
   ActivityBar isn't yours; it is NOT yours — describe the exact insertion for the integrator).
   'hidden'/'right' collapse the bottom dock entirely. A small button on the bottom dock header
   ("Move to the right side" / "Hide") plus the setting in SettingsDialog via integration note.
   Also add (do not wire) `settings.intelligence.csharpLanguageServer: boolean` default true —
   Lane H's toggle reads it; note it for the integrator.

## Lane C — Context panel cleanup

Owns: `components/ContextPanel.svelte`, `context/*`, `processes/*` (PlaywrightCard etc.), new
files under `components/context/`.

Screenshot 49. 1. "Runs" means commands this app itself launched and recorded — nobody can
tell. Give the card a plain-English subtitle ("Commands this app has started for you — empty
until you run something from here") or fold it away when empty. 2. The refresh button must
refresh EVERY card (runs, processes, playwright, agent sessions, worktrees) — today some panes
don't reload; go through `contextService.refreshAll` and make it actually cover each card,
loading states included. 3. Running processes: add a kill button per row → `kill_process`
backend contract (capability `processKill`, gate + honest disabled tooltip), AlertDialog
confirm naming the process and port, then refresh. 4. The "+368"/"+4" overflow counts must be
clickable: agent sessions +N expands the list in place (paged, e.g. 25 at a time); worktrees +N
opens the Worktrees view (integration note: needs the `showView('worktrees')` hook — describe
where). 5. Redundancy: keep the agent-sessions card as a glance list but add a "Search all
sessions" link that opens the session finder (hook via integration note); worktrees card
header links to the Worktrees view.

## Lane D — Worktree pane safety and CSS

Owns: `components/worktrees/*`, `worktrees/worktreeManagerRows.ts`,
`worktreeManagerService.ts`, `worktreeManagerStore.svelte.ts`, `worktreesBackend.ts`.

Screenshot 50 + a real incident: user clicked Remove/Delete anyway on a "Folder is gone" row —
no confirmation appeared, and BOTH stale rows vanished at once (backend runs repo-wide `git
worktree prune` for any prunable row — main.rs:2804; Lane H is scoping it to one row,
capability `worktreePruneSingle`).
1. EVERY remove path gets an AlertDialog confirm first — including folder-gone rows. The
   dialog says in plain English exactly what will happen: for a folder-gone row, "The folder is
   already gone. This only clears git's records for it — nothing on disk is touched." When the
   backend lacks `worktreePruneSingle`, the dialog must also say "Because this app build is
   older, clearing this row will also clear every other row whose folder is gone." — that
   sentence is the difference between today's surprise and informed consent.
2. Collapsed-row CSS is broken: closed rows show a squished multi-line mess ("· / no activity
   recorded / · / No session has worked here" stacked). Collapsed row = ONE line: name badge +
   status chips + actions; the detail lines only in the expanded body. Also drop the stray "·"
   separators when a field is empty.
3. Buttons: "Remove" / "Back up" / "Delete anyway" render for folder-gone rows where Back
   up/Delete anyway make no sense (nothing to back up or delete) — for those rows show a single
   "Clear this entry" action.

## Lane E — Source control depth

Owns: `git/*` (gitService.ts, gitPanelStore, gitGraph*), `components/git/*`,
`components/GitPanel.svelte`, `components/GitDiffView.svelte`.

Screenshot 51: only 24 commits (`COMMIT_HISTORY_LIMIT = 24`, gitService.ts:55) of thousands.
1. "Load more" at the bottom of the commits list: fetch in pages (24 → +100 per click, backend
   cap 500 per Lane H; if the old backend caps lower, show what came back and say so). Keep the
   graph lanes correct across pages (gitGraphLanes is pure over parentShas — feed it the full
   accumulated list).
2. Commit count header shows "24 of many" honestly — e.g. "Showing 24 · Load more".
3. Small VS Code-isms that fit this wave without a backend change: per-file discard button
   (uses existing checkout/restore plumbing IF it already exists in gitService — do not add
   new backend calls; if absent, skip and say so in the note), collapse/expand Changes vs
   Commits sections independently, and a branch name + ahead/behind line that doesn't truncate
   into "cdx/outbound-rule-generat…" without a tooltip.

## Lane F — Stacks → Run configurations

Owns: `stacks/*`, `components/stacks/*`, new files under `components/run/`.

Screenshot 48 (Rider) is the reference for spirit, not scope. 1. Rename the concept everywhere
user-visible: the tab/section is "Run" (configurations), a stack is one saved way to start
things. 2. A play button + dropdown component (`components/run/RunButton.svelte`) listing every
configuration: click = run it, dropdown row shows name + short command + last exit state. It
must be embeddable in the shell's top bar — build it standalone, integration note tells the
integrator exactly where to mount it (top bar markup lives in `/next/+page.svelte`; describe
the slot). 3. GUI editor (Dialog): add/edit/remove a configuration — fields: name, command,
folder (project-relative default), env lines (KEY=value textarea). Persist wherever stacks
persist today; extend that shape compatibly (old saved stacks must still load — write the
migration/fallback). 4. Keep the honest exit codes work from last wave intact
(`recordStackStart`, runCommandDirectly path).

## Lane G — Explorer icons, folder counts, syntax highlighting

Owns: `components/ExplorerPanel.svelte`, `components/EditorPanel.svelte`, new files under
`components/explorer/` (e.g. `fileIcons.ts`).

1. File-type icons (screenshot 55): a small local icon map — colored glyphs per extension
   (svelte, ts, js, rust, cs, css, html, json, md, sql, xml, sh, images, lock, config…), one
   default. Use lucide icons + per-type accent color, or tiny inline SVGs — NO external icon
   font or CDN (strict CSP). Folders keep chevrons.
2. Folder counts (screenshot 53/55): the number badge reads as "files in here" but shows
   direct-children counts (EdiEngine.Cli shows 11 while Commands alone holds 23). Make the
   badge the recursive file count, computed from the already-scanned tree, and cheap (compute
   once per refresh, not per render).
3. Syntax highlighting audit in EditorPanel's Monaco setup: confirm ts/js/json/html/xml/sql/
   rust/cs come from monaco built-ins and that `.svelte` files get real highlighting (register
   the svelte language or map to html as an honest fallback — say which in the note). Make
   sure the language picks up from extension for: svelte ts js rust sql xml json html cs sh
   toml yaml md.

## Lane H — Backend: reference counts, matcher, caps, kill, prune scoping, C# LS toggle

Owns: `src-tauri/**` (exclusive — the ONLY lane that builds rust) and
`tauri-svelte-preview/src/lib/sourceIntelligence.ts`. This is the revised TSK-789 plan; it was
never built — build it now.

1. `count_source_references { root, symbolNames } → { counts: [{name, count, approximate}] }`:
   ONE walk of the project counting every requested symbol (replaces ~120 per-lens passes per
   file open, and the 5,781-record IPC payload). Reuse the existing scan's file walker and
   filters. Case-sensitive `str::find` (kill the per-line `to_lowercase()` around
   main.rs:1652). Deadline a few hundred ms: when it trips, return what you have with
   `approximate: true` per unfinished symbol — approximate counts flagged, never blanks.
2. Register in the invoke handler + add `referenceCounts` to `read_backend_capabilities`.
3. Frontend (`sourceIntelligence.ts`): when `referenceCounts` capability exists, code lenses
   get counts from ONE `count_source_references` call per file-open (all symbols batched);
   re-key the count cache on symbol name + content hash so it survives typing (today it dies
   per keystroke on model version). Lift `maxCodeLensNativeReferenceScanRecords = 1500` only
   on this new path. "50+" style caps go away when exact counts are cheap.
4. `kill_process { pid }` per the contract; capability `processKill`.
5. Worktree prune scoping: in `remove_project_worktree_sync` (main.rs:2750), the prunable
   branch (line 2804) must clear ONLY the clicked row: delete `.git/worktrees/<id>` for that
   entry (git blesses manual removal) instead of `git worktree prune`. Message names the one
   branch cleared. Capability `worktreePruneSingle`. Update the sibling tests (6591 area).
6. Commit history: find the backend limit cap for `read_git_commit_history` and raise to 500.
7. C# language server toggle: find where the Roslyn/C# LS is spawned. If backend-spawned, add
   a way to (a) not start it and (b) shut it down, driven by a command the frontend toggle
   calls; if frontend-spawned, do it in sourceIntelligence.ts. When the LS is off, reference
   counts still work via (1) and the UI must say diagnostics/peek precision are reduced —
   define the exact seam in your integration note; the settings key is
   `settings.intelligence.csharpLanguageServer` (Lane B adds it; integrator wires the
   SettingsDialog control).
8. Verify: `cargo test` in src-tauri (your builds only — no other lane compiles rust), plus
   measure the counter on EdiPlatform (`core/examples/scanbench.rs` pattern) and put the
   number in your integration note. Sequence your own builds; never two cargo builds at once.

## Integration & review

Integrator (after all lanes): wire every INTEGRATION.md (top-bar RunButton, showPanel hooks,
SettingsDialog controls, ActivityBar problems view, shellCommands additions), add settings keys
lanes documented but didn't own, run `node scripts/checkSvelteNext.mjs` + one `pnpm build` +
`cargo check`, browser-verify each lane's "how to verify" list, commit per-lane then the wiring,
open PR `tsk-798-polish-wave`. ONE milestone review afterwards (destructive code: kill_process,
worktree clearing); controller applies findings directly, no re-review.
