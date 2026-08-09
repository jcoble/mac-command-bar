# Round-2 acceptance feedback — 2026-08-09 (native build walkthrough)

Source: user's native acceptance pass after the TSK-808 wave. Screenshots live in this
directory (24.png–36.png, numbering matches the conversation). This file is the raw capture;
the design/spec derived from it lives in docs/superpowers/specs/.

## 1. The conversation must own the workbench (BIGGEST item)

- Reference: 24.png and 32.png — the Codex desktop app. The whole middle of the app is the
  conversation/session. EVERYTHING revolves around the session.
- Our current layout treats Session as just another tab (25.png shows the tab strip: Editor,
  Browser, Diff, Session History, Agents, Session).
- Wanted: session/conversation permanently in the middle. The other tabs move to a right-side
  strip — panels that can be opened from the side bar and/or dragged into the middle, covering
  the session temporarily. The session is the only thing that "stays".
- Note: Orca revolves around the worktree; Codex revolves around the conversation. The user
  wants conversation-centric, like Codex.

## 2. Structured conversation view is gone / broken — must be ACP

- The Session tab currently shows only the terminal. The structured part is missing.
- The structured view appeared for a second, user switched to terminal output, and could not
  get back to structured.
- The agreed plan (ACP runtime decision, commit 279b453 plan) was: structured conversation is
  done through ACP, not terminal scraping. The user should not have to care about the terminal
  session at all anymore.
- Action: verify what the current Session tab actually renders, whether the ACP structured
  path was implemented or ripped out, and make structured-over-ACP the primary (terminal
  demoted to a secondary/inspector view).

## 3. Session History needs a rework

- Reference: 26.png (Orca's session card: long first prompt, long latest turns, subagents,
  worktree, actions) — ours cuts everything off and shows almost nothing when expanded.
- Grouping: drop Workspace/Project/All triple filter. Default = All, grouped by project
  (like the Codex app sidebar), with worktree groups underneath, all collapsible. Right now
  the project header cannot be collapsed.
- Missing per-session info: first prompt (27/28.png "First prompt is not available from the
  session index yet"), worktree, rich latest-turn preview.
- Wrong data (bugs, not preferences):
  - 27.png vs 28.png: same session shows "6 messages" in the list but 2760 msgs in the
    expanded card.
  - 30.png: "rental-management (4)" group header, but only 1 visible under it, and the real
    total is ~100 spread across worktrees. Duplicate adjacent "rental-management" headers
    (30/31.png) — one shows 4, the next shows 75.
  - 31.png: group says 75 but only ~9 rows are reachable; paging says "50 shown of 690".
- Search can't find sessions the user knows exist.

## 4. Browser = full-app overlay, not a tab

- Reference: Orca. The browser should overlay the whole app, not live as a center tab.
- Stretch: the session input bar becomes part of the overlay so annotations go straight into
  the prompt. Fallback: annotations attach to the prompt input of the main session view.
- Bug 33.png: navigation fails — `invalid args 'input' for command 'navigate_browser_tab':
  missing required key input`. Cannot navigate to a page at all.

## 5. Resources — half works, wrong place

- 34.png: "Open full resource view" opens an inline card; it must open a FULL-PAGE overlay
  like Orca's Space page (35.png reference).
- Missing: the app's OWN resource usage (itself + everything it owns) with a running total.
- Placement: Resources belongs in the BOTTOM status bar, not the top.

## 6. Usage — broken and wrong place

- 36.png: Stats & Usage shows "Usage history could not be read." Live quota (Codex 3%) works;
  the SQLite-backed history read fails entirely in the native build.
- Placement: Usage also belongs in the bottom bar, not the top.

## 7. Meta

- Too many other smaller issues to enumerate now; get the main things working first.
- The user has explained the session-centric/ACP direction repeatedly; it keeps getting lost
  between agent sessions. This capture + the derived spec exist so it stops getting lost.

## Screenshot index

| File | What it shows |
| ---- | ------------- |
| 24.png | Codex app: conversation owns the center, sidebar = projects/threads |
| 25.png | Our tab strip: Editor, Browser, Diff, Session History, Agents, Session |
| 26.png | Orca session card: rich first prompt + latest turns + subagents + worktree |
| 27.png | Our expanded card: "6 messages" meta line |
| 28.png | Same session showing 2760 msgs in another view |
| 29.png | Our list: "First prompt is not available from the session index yet" |
| 30.png | Group header count 4 vs 1 visible; duplicate project headers |
| 31.png | Group header 75 but few rows reachable; 50 shown of 690 paging |
| 32.png | Codex app again: prompt input anchored to conversation |
| 33.png | Browser nav error: navigate_browser_tab missing required key input |
| 34.png | Resources popover: inline "full resource view", app-self usage missing |
| 35.png | Orca Space full-page overlay (disk usage, treemap, worktree table) |
| 36.png | Usage popover: "Usage history could not be read." |
