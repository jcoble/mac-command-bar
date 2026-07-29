# Shell Chrome Wave: shadcn-svelte + Layout Flip + Task Rows + Restart

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The /next shell stops looking hand-rolled: its chrome rebuilds on shadcn-svelte (the design system the user runs in Rental Command and RetailReady EDI), the layout flips to sessions-always-left / tools-right, session rows become rich task cards, and a finished session can actually be started again from its row.

**Architecture:** One frontend lane (sequential tasks, shared files) + one parallel scanner-enrichment task (core/ + TS bridge, disjoint files). Foundation task first (Tailwind v4 without global reset + shadcn-svelte primitives themed to the existing Houston tokens), then the frame flip, then the sessions column rebuild consuming both, then restart-from-row.

**Tech Stack:** Svelte 5 runes, shadcn-svelte (bits-ui ^2.18.1 — already a dependency), Tailwind v4 via `@tailwindcss/vite`, `@lucide/svelte` (already a dependency), Rust mcb-core scanner, Node test scripts.

**Covers:** TSK-761 (two surfaces, rich rows, restart), theme-system task's library decision (2026-07-29 direction change), layout-flip direction (left = sessions / right = tools, decided 2026-07-28 evening).

## Global Constraints

- The old shell `tauri-svelte-preview/src/routes/+page.svelte` is FROZEN, carries uncommitted user changes, and must render **pixel-identical** after this wave: Tailwind ships WITHOUT its global preflight/reset (no `@import "tailwindcss"` wholesale — import `theme` and `utilities` layers only), and any base styles shadcn components need are provided scoped under the `/next` shell's root element. Verify the old shell visually after the foundation task.
- Monaco, xterm, and dockview keep their own rendering and stylesheets. The terminal DOM is sacred — no library wrapper may re-render or re-parent terminal hosts. Dockview tabs/panels are restyled ONLY via its CSS variables/classes from the shared tokens.
- `src-tauri/` untouched in the primary checkout (watched). `core/` editable (unwatched; desktop needs a restart to see it — report, don't restart).
- Stores: no backend calls, no `$effect`; IO in explicit functions; persistence via explicit `persist()`.
- shadcn conventions mirror Rental Command: `components.json` with aliases `$lib/components/ui`, registry `https://shadcn-svelte.com/registry`, TypeScript. Components are vendored into `src/lib/components/ui/` (registry add), not npm-imported.
- All user-visible copy in plain English. Commits plain, NO Co-Authored-By trailer. `pnpm run check` clean and `pnpm build` green at every commit.
- No `window.confirm`/`alert`/`prompt` anywhere in /next — confirmations are AlertDialog.
- Fonts: body text in the rail/cards never below 13px; metadata/chips never below 12px (user has complained twice about tiny type).
- VISUAL IDENTITY IS FIXED (user, 2026-07-29): "I still want the look that's here, but just more polished by using a library." The current /next dark look — its palette, density, and feel — stays; shadcn components are skinned entirely from the existing tokens. No Material styling of any kind. Do NOT reuse or import the legacy `src/lib/components/` set (Tooltip/Menu/Dialog/…) — it belongs to the old shell; /next uses only the freshly vendored `$lib/components/ui` and keeps its current visual identity.

---

### Task 1: Foundation — Tailwind v4 (no global reset) + shadcn-svelte themed to the Houston tokens

**Files:**
- Create: `tauri-svelte-preview/components.json` (mirror Rental Command's: schema `https://shadcn-svelte.com/schema.json`, tailwind css `src/lib/shell/styles/next.css`, baseColor zinc, aliases `$lib/components`, `$lib/utils`, `$lib/components/ui`, `$lib/hooks`, `$lib`)
- Create: `tauri-svelte-preview/src/lib/shell/styles/next.css` — the /next-only stylesheet: `@import "tailwindcss/theme.css" layer(theme); @import "tailwindcss/utilities.css" layer(utilities);` (NO preflight), plus a scoped base block under `.next-shell` providing the minimal resets shadcn components assume (border-box, border-color defaults, focus-visible ring vars), plus the shadcn CSS variables (`--background`, `--foreground`, `--primary`, `--muted`, `--accent`, `--destructive`, `--border`, `--ring`, `--radius`, …) DEFINED FROM the existing Houston tokens in `nextTokens.css` (map, don't duplicate hex values — `--background: var(--mcb-bg)` style; read nextTokens.css for the real names).
- Create: `tauri-svelte-preview/src/lib/utils.ts` if absent (the `cn` helper shadcn components import; add `clsx` + `tailwind-merge` deps)
- Modify: `tauri-svelte-preview/vite.config.ts` (add `@tailwindcss/vite` plugin), `package.json` (deps: `tailwindcss`, `@tailwindcss/vite`, `clsx`, `tailwind-merge`, `tw-animate-css` if the registry components need it)
- Create (registry add): `src/lib/components/ui/` — button, badge, input, dialog, alert-dialog, dropdown-menu, tooltip, separator, scroll-area, collapsible, tabs, card. Use `pnpm dlx shadcn-svelte@latest add <name> --yes` after writing components.json; if the CLI fights the non-SvelteKit-standard layout, vendor the component sources by hand from the registry, unmodified except import paths.
- Modify: `tauri-svelte-preview/src/routes/next/+page.svelte` (import `next.css`; ensure the root `<main>` carries class `next-shell`)
- Modify: `tauri-svelte-preview/src/lib/shell/components/ShellOverlays.svelte` — the pilot conversion: settings overlay becomes shadcn `Dialog`, the error strip becomes an `Alert`-styled block, any confirm becomes `AlertDialog`.

**Proof obligations (steps):**
- [ ] Setup + theme mapping compiles; `pnpm run check` + `pnpm build` green.
- [ ] OLD-SHELL REGRESSION CHECK: open `http://localhost:5177/` (old shell) — visually unchanged (no reset leakage: check body margins, button styling, font inheritance). State the check in the report with what was compared. The new stylesheet must not be imported anywhere the old shell loads.
- [ ] Pilot: ShellOverlays renders via shadcn Dialog in the /next preview; keyboard (Esc) and focus trap work.
- [ ] Commit: `feat: shadcn-svelte foundation for the /next shell, themed from the Houston tokens`

**Interfaces produced:** `$lib/components/ui/*` (shadcn), `cn()` from `$lib/utils`, the `.next-shell` scoped variable set. Later tasks import ONLY through these.

### Task 2: Layout flip — sessions always left, tools right

**What changes:** `ShellFrame.svelte`'s gridview regions reorder: leftmost region = the sessions column (Task 3 rebuilds its contents; this task moves the EXISTING SessionRail there unchanged), rightmost region = the tool column — the current `ShellSidebar` activity-bar-plus-views moves right wholesale (explorer, source control, worktrees views), and the existing right-side `ContextPanel` becomes a view inside it (its icon joins the activity strip; `ActivityBar.svelte` icons order: explorer, source control, worktrees, context). The activity ICON strip sits on the far right edge (Rider-style), views open to its left. Center dock untouched.

**Files:**
- Modify: `src/lib/shell/components/ShellFrame.svelte` (region order + sizes: sessions column default 300px min 220, tool column default 320 min 240, icon strip fixed ~44px)
- Modify: `src/lib/shell/components/ShellSidebar.svelte` (host the context view; activity strip alignment right; `onSourceControlVisible` wiring preserved)
- Modify: `src/lib/shell/layout/frame.ts`, `layoutStorage.ts` — NEW storage key for the flipped grid (`mac-command-bar.next.frame-layout-v3` or increment the existing versioned key by its established pattern — read it first); stale v2 layouts are NOT migrated, they fall back to the new default (plain-words comment saying why).
- Modify: `src/lib/shell/sidebarViews.ts` (add context view to the roster)
- Modify: `src/routes/next/+page.svelte` (snippet wiring: rail snippet moves to the sessions region; context snippet becomes a sidebar view)

**Steps:**
- [ ] Flip + storage-key bump; reset-layout command resets BOTH columns.
- [ ] Verify in preview: default layout shows sessions left, icon strip far right with 4 views, context cards load only when its view opens (panelActivation semantics unchanged — `sourceControlVisible` behavior must survive; check `panelActivation.test.mjs` still passes and extend if the context view gains the same visibility gating).
- [ ] `pnpm run check` + suites green.
- [ ] Commit: `feat: sessions live on the left always; every tool view moves right`

**Interfaces produced:** the sessions region hosts whatever component the page passes as the rail snippet (Task 3 swaps its contents); tool-view roster in `sidebarViews.ts` now includes `context`.

### Task 3: The sessions column — task cards on shadcn, two surfaces, collapse-to-strip

**What it builds (consumes Task 1 components + Task 4 fields when present):**
- `src/lib/shell/components/SessionsColumn.svelte` (new; replaces SessionRail in the sessions region — SessionRail stays in the tree until this lands, then is deleted along with styles only it used):
  - **My work** (top, the room): Working / Done sections. Each session is a shadcn `Card`-based row: title (14px+), provider badge, state dot, chips (branch/task/PR — from Task 3 of the previous round), relative time; when Task 4's fields exist: model chip, message count, one-line latest-turn preview (muted, truncated). Row actions as icon `Button`s with `Tooltip`: Mark done / Reopen; Close terminal; Remove (Done rows only) via `AlertDialog` (replaces the interim inline confirm from tsk-789 if present — reconcile with whatever is on main).
  - **Find a session** (bottom, `Collapsible`, default collapsed to a header with count): search `Input`, project groups (existing `sessionGroups.ts` logic reused verbatim), 8-row caps with "Show N more", Adopt buttons.
  - **Collapse-to-strip:** a header toggle collapses the whole column to ~52px showing one icon-sized cell per My-work session (provider icon + state dot, tooltip = title); clicking a cell selects that session; the toggle state persists in `layoutStorage` under its own key. Expanded/collapsed width is applied through the frame's API (Task 2's region), not by hiding content with CSS only.
- Grouping/caps/relative-time logic stays in the existing pure modules (`sessionGroups.ts`, `relativeTime.ts`) — this task is presentational + wiring; new pure logic (e.g. strip-cell derivation) goes in a pure module with a Node test.

**Steps:**
- [ ] Build SessionsColumn behind the existing props contract (`owned`, `available`, `activeOwnedId`, `onSelect/onAdopt/onClose/onComplete/onReopen/onRemove/onRescan`, + whatever tsk-789 added — read the current SessionRail props first).
- [ ] Swap it into the sessions region; delete SessionRail + dead styles.
- [ ] Node test for the strip-cell pure module; `pnpm run check`; full script sweep.
- [ ] Live check in preview: both surfaces, collapse strip, all actions incl. AlertDialog remove.
- [ ] Commit: `feat: sessions column — task cards, find-a-session drawer, collapse to strip`

### Task 4 (parallel with Task 2): Scanner enrichment — message count + latest-turn preview

**Fields added to `AgentSessionRecord` (Rust `core/src/scanners/sessions.rs`) and the TS bridge (`src/lib/server/localSourceFs.ts`), both `skip_serializing_if = None` / optional:**
- `message_count: Option<u32>` — user+assistant turns counted while the scanner already reads the transcript (Claude path reads lines already; Codex path counts within the probe budget it already has — do NOT add new full-file reads for capped files; a count the budget cut off serializes as the count so far).
- `latest_turn_preview: Option<String>` — the last user or assistant text line the existing read already saw, prefixed `You: ` / `Agent: `, single line, max 120 chars, whitespace-collapsed.
**Frontend:** `AgentSession` type + `adoptAgentSession` carry-through (`messageCount`, `latestTurnPreview` on `OwnedSession`, tolerant parse) + fixture-pinned parity tests both sides (same fixtures in Rust tests and `localSourceFs.test.mjs`).
**Constraint:** scan time must not regress meaningfully — the fields come from data the scanner already holds in memory; state in the report what the scan-time impact was and how you know (the `#[ignore]`d scanbench or timing prints both suites already have).

- [ ] Rust fields + tests (shape test updated) green; TS parity + tests green; types + carry-through + `ownedSessions.test.mjs` cases.
- [ ] Commit: `feat: sessions carry a message count and a last-turn preview`

### Task 5: Restart a session from its row

**The gap (user, twice):** "if you take it out of done, you can't start it back up." Rows already carry `resumeCommand`; the adopt flow already spawns PTYs for scanned sessions.

**What it does:** exited owned rows (Working or Done) get **"Start again"**: spawns a fresh PTY for that EXISTING owned row (keeps `ownedId`, so its workspace snapshot, chips, and completedAt survive), replays the session's `resumeCommand` exactly like `adopt` does for scanned records, updates `ptySessionId`/`state: 'live'`, selects the session. A session with no `resumeCommand` (fresh shells) restarts as a plain shell in its `cwd`. Failure lands on `rail.error` in plain words and the row stays exited.

**Files:** `src/routes/next/+page.svelte` (new explicit-IO `restartOwned(ownedId)` using `service.startOwned` — read `terminalService.ts:startOwned` first: it takes the owned record + host; reuse `hostFor`), `SessionsColumn.svelte` (action button, only on `state === 'exited'` rows), `terminalService.ts` ONLY if `startOwned` genuinely can't take an existing owned record (report if so, keep the change minimal).

- [ ] Implement + wire; `pnpm run check`; suites green.
- [ ] Live check limited on web (no PTY backend): verify the button renders on exited rows and the failure path lands on `rail.error` plainly; flag the desktop pass for the user.
- [ ] Commit: `feat: a finished session can be started again from its row`

---

## Verification & merge

- Milestone review (one reviewer, whole branch, both the frontend lane and Task 4's core changes) → fix wave → scoped re-review → PR → merge.
- Wave branch: cut from main AFTER `tsk-789-desktop-fixes` merges (it touches SessionRail/sourceIntelligence; Task 3 deletes SessionRail — the reviewer must see the reconciliation, and Task 1's implementer must read main's state, not assume this plan's).
- Desktop restart note for the user at the end (core scanner fields need it; frontend arrives via HMR).
