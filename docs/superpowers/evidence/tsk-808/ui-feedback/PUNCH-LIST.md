# TSK-808 acceptance feedback — UI punch list (2026-08-08)

Owner feedback from the first rebuilt-native acceptance pass. Every item below is a rework of the
current /next shell toward the owner's references: the ChatGPT/Codex desktop app, Orca, and T3
Code. All reference and current-state screenshots live in this folder.

The owner's framing, verbatim in spirit: the AI is not a chatbot. It is an agent that lives in the
editor and does things — writes your commit message, creates the PR, works the tools. The shell
should read as a calm, native, product-grade workbench, not a collection of outlined boxes.

## 1. Theme: Houston everywhere, kill the "contrast theme" look

Current state came from one remark about low contrast between elements; it was wrongly expanded
into a full high-contrast theme (hard outlines around every control, stark black, bordered
buttons). Wrong direction.

- Target: the Houston theme feel across the whole app — soft dark surfaces in layered grays,
  rounded corners, subtle borders only where surfaces meet, muted text hierarchy, restrained
  accent color. See `ref-chatgpt-app.png`, `ref-orca-full-app.png`, `ref-orca-sidebar.png`.
- Buttons: quiet ghost/filled styles like the references — not outlined pill boxes on every
  control (`current-browser-tab.png` shows the anti-pattern).
- This is a token-level rework in the /next shell's style system, then a sweep of components that
  hard-code the outlined look.

## 2. Find a session: its own tab, styled like Orca's session history

Current: a collapsed "Find a session · 644" row at the bottom of the left column
(`current-find-a-session-row.png`) and a cramped AGENT SESSIONS list
(`current-agent-sessions-panel.png`).

- Target: a full tab dedicated to finding sessions, on the right side, modeled on
  `ref-orca-session-history-panel.png`: header with count + host + filter + refresh,
  Workspace / Project / All segmented filter, search box, sessions grouped by project with counts,
  each row showing title, agent summary line, provider icon, message count, age, model.
- Session detail expansion opens inline like `ref-orca-session-details-expanded.png`: Resume in
  Worktree / Continue in New Session / View Log actions, FIRST PROMPT block with copy, LATEST
  TURNS blocks. Plus drag-to-resume affordance.

## 3. Browser: rebuild the chrome to match Orca's embedded browser

Current: `current-browser-tab.png` — a row of ~15 outlined text buttons, URL box lost among them,
annotation buttons stranded in the far-left corner (`current-browser-annotation-corner.png`).

- Target: `ref-orca-browser-full.png`, `ref-orca-browser-toolbar.png`,
  `ref-orca-browser-viewport-menu.png`, `ref-annotation-buttons-placement.png`.
  - Browser tabs across the top (favicon + title + close, plus a + button).
  - One compact toolbar: back/forward/reload, one rounded URL field, then icon-only actions to
    the RIGHT of the URL: Import (labeled), element picker, annotate/comment, draw, devtools,
    open external, overflow menu.
  - Overflow menu holds: profiles (Default / New Profile), Import Cookies, Viewport Size submenu
    (Mobile S/M/L, Tablet, Laptop, Laptop L, Desktop presets), Browser Settings.
  - Floating maximize/minimize pair top-right of the browser surface, like Orca.
  - Feedback queue stays but restyled to the theme; annotation actions live in the toolbar, not
    the corner.

## 4. Resources: real per-workspace data, Orca Resource Manager layout

Current: `current-resources-popover.png` — raw external system processes (launchd, logd...) with
"External" badges, no workspace grouping, unreadable dim text, and the popover overlaps the space
section.

- Target: `ref-orca-resource-manager.png`: header with total CPU% and RSS, a tree grouped by
  project → workspace → named terminals/agents with live CPU and RSS per row and sparklines,
  "Review inactive workspaces (N)", and a Space section with a Scan action.
- The full Space view should follow `ref-orca-space-page.png`: scanned/reclaimable/workspaces
  summary tiles, treemap of workspaces by size, per-workspace top-level item breakdown, filter +
  select + delete-selected over the workspace list with keep-state badges.
- The data side must actually attribute processes to owned sessions/workspaces (PTY process
  trees), not list system daemons. External/system processes do not belong in the default view.

## 5. Usage: real provider quota + analytics, Orca style

Current: `current-usage-popover.png` — "provider has not advertised quota data", zero-value
history, overlapping popovers.

- Target popover: `ref-orca-usage-popovers.png` — per-provider cards (Claude: 5h session %,
  weekly %, Fable %; Codex: weekly %) with reset countdowns, Detailed/Compact toggle, account
  submenu, and a status-bar usage strip along the bottom of the shell.
- Target full page: `ref-orca-stats-usage-page.png` — agents spawned / time worked / PRs created
  tiles, total tokens, est. cost, active days, cache share, daily-intensity heatmap, token mix,
  per-provider cards with tokens/sessions/cost.
- Data sources: local provider session stores (Claude/Codex JSONL and quota endpoints) — the
  SQLite history pipeline from A10 exists; wire real ingestion so the numbers are non-zero, and
  read live quota where the CLI exposes it.

## 6. Session card details + context menus

- Expanding a session's details opens the inline expanded card
  (`ref-orca-session-details-expanded.png`), not a bare disclosure.
- Right-click context menus throughout the session lists and tabs (resume, continue in new
  session, view log, copy id, archive, delete...).

## 7. Agent does things: commit messages and PRs (NEW work item)

Not present in this wave; owner expected it. Model on `ref-orca-pr-panel.png`:

- A source-control flow where the agent generates the commit message.
- A "New pull request" panel: branch → base, generated title + description (with a visible
  "Generating title & description..." state), draft checkbox, Push & Create PR button, and a
  merged/checks status card after creation (see the PR hover card in `ref-orca-full-app.png`).

## 8. Bug: diff editor breaks the center layout

`current-diff-layout-breakage.png`: opening the diff view corrupts the whole center arrangement —
duplicated panes, overlapping surfaces, misplaced overlays. Reproduce, root-cause, fix, and add a
regression test around center layout + diff activation.

## Sequencing

Theme (1) first — it defines the tokens every other item styles against. Diff-layout bug (8) and
resources/usage data plumbing (4/5 backend) can run in parallel with it (disjoint files). Then
browser chrome (3), find-a-session tab (2), details/context menus (6), then PR/commit flow (7).
