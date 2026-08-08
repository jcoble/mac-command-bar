# TSK-808 overnight run — morning summary (2026-08-09)

All eight punch-list items are implemented, verified as far as the web preview allows, and
committed on `tsk-808-assembly-wave`. 12 commits since the punch list (17f3f96), HEAD fc03749,
working tree clean.

## Commits, in order

| Commit  | What |
| ------- | ---- |
| 1c5aed6 | Diff-layout bug fix: parent Gridview lays out before the center Dockview restores (item 8) |
| 5cf470b | Houston theme across /next, contrast-theme look removed (item 1) |
| bdd59d0 | Real resource attribution (owned process trees) + usage ingestion/analytics (items 4/5) |
| def7936 | Orca-style browser chrome: tabs, compact toolbar, overflow with viewport presets (item 3) |
| feb81ac | Session History full tab, inline details card, context menus (items 2/6) |
| 556265b | Stale code-lens test pins fixed (pre-existing failure from acae026) |
| 45588f6 | Resource polish: sparklines, inactive-workspaces row, Space filter, git-based workspace identity, Space default roots |
| 7a07d8e | Agent commit messages + Push & Create PR via gh (item 7) |
| 42f22e7 | Playwright web-preview verification: 23 screenshots, verdicts; context-menu positioning fix |
| e1e9a33 | Residual Houston sweep: resources/usage/PR/assistance surfaces |
| 7044f08 | Provider usage summary aggregated in SQLite (distinct-session counting, no client reduce) |
| fc03749 | Daily heatmap totals aggregated in SQLite (one cell per day) |

## Verification state

- cargo: 252 passed / 0 failed (unsandboxed, includes the socket-bind test).
- Node script sweep: 95/97; the 2 failures (`sourceUi`, `workspaceSnapshotPlan`) are OLD-SHELL
  tests failing since before this wave (files untouched here) — same class as the 16 old-shell
  svelte-check errors.
- `pnpm check`, `pnpm check:svelte` (0 /next errors), `pnpm build`: green.
- Web-preview Playwright pass: receipt `web-preview-verify-receipt.md`, shots in
  `web-preview-shots/`. Diff-layout fix and context menus VERIFIED in browser; theme, session
  history, browser chrome, resources, usage verified to the limit of the preview.
- Periodic SOL (medium) checkpoint reviews ran all night; every substantive finding was either
  fixed (select-all bug, Codex incremental attribution, missing sparklines/filter, session
  double-counting, heatmap day cells, context-menu position) or is listed below.

## What needs YOU + the native build (rebuild first: `pnpm tauri:build`)

1. Visual acceptance of the Houston theme and browser chrome against your references — the
   embedded webview, floating max/min over real content, and viewport presets only behave in
   the native app.
2. Resources popover: confirm real per-workspace process trees with live CPU/RSS (needs real
   PTY/agent sessions running).
3. Usage: hit Refresh in Stats & Usage to index your local Claude/Codex session files; confirm
   non-zero numbers and the quota bars.
4. PR flow end-to-end: generate a commit message with a live agent session, and Push & Create PR
   against a real branch (needs gh auth).
5. Resume in Worktree / Continue in New Session actions from Session History against real
   sessions.

## Known follow-ups (not blocking)

- Houston terminal palette still uses the Dracula terminal colors (themeRegistry.ts) — decide
  whether to add Houston terminal tokens.
- Space default roots derive from ACTIVE processes only; discovering inactive/reclaimable
  worktrees with no live process needs worktree-registry plumbing.
- Old-shell backlog: 16 svelte-check errors + 2 script tests (`sourceUi`,
  `workspaceSnapshotPlan`) — pre-existing, untouched by this wave.
