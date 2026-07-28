# /next shell — v2 feedback batch (user live review, 2026-07-28)

Source: the user's first hands-on pass over the parallel-wave build (branch
`tsk-324-369-slice2-walking-skeleton`, commits `24c67a5..1ce69f0`), given as a running dump with
screenshots of T3 Code, the Codex app, VS Code, and the Git Worktree Manager VS Code plugin.
Screenshots cached at `~/.claude/image-cache/bb193e50-c3ef-49ec-9c23-e19f5d831d43/{9..16}.png`.

This is the input for the v2 redesign brainstorm/plan. Items are grouped by size, not by the
order given. "DONE" items were fixed during the review session itself.

## North star (the user's words, condensed)

A mixture of the Codex app (rail, conversation-first center), VS Code (left panes, source
control), T3 Code (sessions as task-like items with lifecycle states), CMUX (multi-session,
agent orchestration backend), and Warp (terminal) — **lightweight and fast above all**: many
live sessions without bogging the machine down "like VS Code or Rider". That is why the stack
is Rust + Tauri; every design decision gets judged against this first. The perf constitution
(nothing loads until asked, dev invoke counter as referee) stays binding.

## Fixed during the review

- **Sash resize feedback** — dividers now light teal on hover/drag (`--dv-active-sash-color`,
  commit `1ce69f0`). Follow-up if it still feels thin: widen the grab zone.
- (Earlier same-day: close button removed from permanent center tabs, `951f2b4`; minimize-to-edge
  captured as its own Notion task.)

## The big shape change — Codex-style frame (v2 IA)

- **Left** = Codex-app-style rail: sessions grouped by project (pinned section on top,
  project folders below, "show more" per project), search, **New session** entry point —
  PLUS VS Code-style panes below/behind it. The left side becomes a **Paneview** (dockview's
  collapsible accordion): Sessions / Files / Source control / Worktrees sections.
- **Center** = the conversation/terminal ONLY by default. Nothing else starts there.
- **Right** = "things that display" — browser, editor, previews — as its own tabbed dock;
  panels draggable into the center like today. Right side also Paneview-or-dock (to decide).
- **Git moves to the left** like VS Code (item below). Context cards fold into the new sides.
- Subsumes the old TSK-368 idea (right context → dockview tabs).

## Session model upgrades

- **Lifecycle states (T3 Code style):** rows read like tasks — *Working* (live, elapsed-time
  badge), *Done*, and a reversible **Settled** archive section (hover action "Settle"; can be
  brought back). Maps onto existing machinery: live/exited states + tombstones = Working/Done;
  `lastActivity` = elapsed; Settled = new flag on the owned-session record + a rail section.
  Row metadata: project/repo, title, branch/worktree, PR number, provider icon, timestamps —
  `gitTaskLinks.ts` already extracts task ids; branch/PR hints exist in scanner derived
  metadata (currently not serialized — needs the small Rust struct change + test update noted
  in the shellfeat recon).
- **Top-level sessions only** in the rail: helper/teammate agent transcripts currently appear
  as sessions. The scanner filters one subagent shape (sidechains) but not session-teammates.
  Scanner-side filter fix (core/ + bridge parity).
- **Pick a session → its project loads automatically**: file tree, context, git target, and the
  **correct worktree** — no separate click. (Today: nothing loads until a session row click,
  and the tree/cards need that click even after reload.)
- **New-session flow**: pick project → pick/create worktree → start session → it persists in
  the rail. Plus a **project picker** in general — there is currently no way to choose a project.
- **Naming settled**: the thing you talk to = **session**; the saved panel arrangement around
  it = its **workspace**.

## Feature panes

- **File click opens the Editor tab** (center comes forward via the existing
  `showCenterPanel('editor')` — just not wired to the open-file bus yet). Small fix.
- **Rich source control** like VS Code/Rider on the LEFT, plus a **center "Git graph" tab**
  for the whole history/branch tree in a big view. `gitGraphViewModel.ts` (600 lines, tested,
  pure) exists for exactly this and has never had a UI home.
- **Worktree manager pane** (the Git Worktree Manager plugin, but richer): per-worktree age,
  dirty, committed vs uncommitted, unmerged, prunable/locked + reasons, last activity,
  delete-eligibility — ALL already returned by `list_project_worktrees`. Add: which sessions
  worked on each worktree (join session records' paths). Cleanup: the guided SAFE path (the
  existing `worktreeSafety` / `worktreeCleanupPlan` / `worktreeCleanupRunbook` pure modules)
  AND an explicit confirm-gated DESTRUCTIVE path. This is a standing pain point: worktrees
  hang around and resist removal.
- **Playwright process card**: list stale Playwright/Chromium sessions distinctly from real
  Chrome (name, age, PID), kill the killable ones. Backend commands already exist:
  `list_playwright_sessions()` / `kill_playwright_sessions()`. Standing pain point: today this
  requires asking an agent to spelunk Activity Monitor.

## Theming & settings

- **Houston** (the VS Code theme) is the color direction for now — it was the first
  iteration's look. Monaco already renders it (`sourcePreviewAppearance` baked theme). Wire
  the settings dialog's theme chooser to actually apply themes (it stores a value nothing
  reads); Houston is theme #1.
- **VS Code extension support — agreed tiers:** (1) themes + TextMate grammars from VSIX:
  realistic, do it; (2) LSP servers extracted from VSIX driven by our own `lsp.rs` client:
  case-by-case; (3) full VS Code extension API: NO — it would cost the lightweight identity
  (that's a VS Code fork, per Cursor).

## Agent orchestration (endgame lane)

Assign tasks to agents, run loops, agents talking to each other. CMUX has a rich backend worth
evaluating for reuse: https://cmux.com/docs/agent-integrations/oh-my-claudecode — research pass
required before committing to an approach (what their backend provides vs what our Rust side
already does with sessions/PTYs/orchestration events).

## Notion capture

Captured as tasks (MacCommandBar project) alongside this note; TSK-344 (VS Code-style git
panel) updated rather than duplicated. The minimize-to-edge task from earlier today also
belongs to this batch.

## Post-batch additions (same day)

- **Stack runner** (start-dev.sh-style stacks, process states) — captured, High.
- **Code formatting** in the editor (backend + Monaco provider exist; wiring task captured).
- **File-type icons** (appended to the theme task — icon themes are VSIX tier 1).
- **Problems panel** (build errors + LSP diagnostics; bottom dock's first tenant) — captured.
- **DB browser** — explicit later, Low, captured.
- **PRs #1 and #2 merged to main** during this review; next branch: `tsk-759-762-v2-base`.

## Scanner-filter evidence (for TSK-762's top-level-only work)

Probed this project's transcript dir (`~/.claude/projects/-Users-blackcolours-dev-work-mac-command-bar/`):
helper/teammate session files carry `isSidechain: false` and `userType: "external"` — identical to
real user sessions, and NO `agentName`/`agentId`/`slug` field appears anywhere in them. The existing
sidechain filter therefore cannot catch them. Leads for the filter, in order of promise:
(1) helper transcripts' first `type:"user"` message is a machine-authored dispatch prompt
("You are the implementer for…", "Review this change for…", report-file paths) while real sessions
start with human text — a first-user-message heuristic; (2) real sessions observed starting with a
`type:"last-prompt"` record vs helpers starting with `queue-operation` (small sample, verify);
(3) check ~/.claude sidecar/index files for parent-session linkage before trusting either heuristic.
