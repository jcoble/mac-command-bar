# /next Workbench Roadmap — agreed design (2026-08-11)

Owner-approved decisions from the 2026-08-11 brainstorming session. This is a roadmap
spec: each numbered section is a sub-project that gets its own implementation plan when
its turn comes. Build order is at the bottom.

## App concept (the frame every section serves)

Mission control for many agents across projects, with real code review. The app opens to
the last session; the My Work rail plus attention dots are the cockpit — no separate
home/overview screen. The owner monitors, dives in on approvals and finishes, and reads
the actual code before anything ships (especially EdiPlatform). Multi-step work becomes
recipes the owner triggers and checks in on. Resource visibility and thrift are
first-class product values, not diagnostics.

## 1. Editor modes (LSP on/off)

Monaco with Roslyn/LSP already exists in the app. What is new is the control:

- **Read mode (default):** jumping from a diff hunk or file tree opens the file in the
  editor with syntax highlighting only — no language server process started. Cheap
  enough to leave open for every project at once.
- **Full mode (per-workspace toggle):** the owner turns the LSP on for the project they
  are deep in — peek references, code lens, go-to-definition, edit/save with
  diagnostics. Turning it off terminates the server process and reclaims its memory.
- The toggle is per workspace/project, visible in the Editor surface, and its resource
  cost shows in the Resource Manager (the LSP process attributes to its workspace).
- Diff → editor: clicking a diff hunk opens the real editor at that line in whichever
  mode the workspace currently has.

## 2. Workflow recipes (and the Agents surface, later)

- **Recipe** = named ordered steps. Each step: role name ("implement", "review",
  "test"), agent (codex or claude, with model + effort), prompt template with variables
  (`{task}`, `{repo}`, previous step's report), and a gate.
- **Default gate: auto-run, stop on trouble.** Steps flow automatically; the run stops
  and raises needs-attention only when a step fails, a review finds problems, or a step
  explicitly requires approval. Per-step override available in the builder.
- **Harness:** the existing Rust workflow engine (`src-tauri/src/workflow.rs` — DAG
  validation, roles, gates, audit ledger). A run creates one ACP session per step,
  grouped under the run in My Work with per-step status.
- **Isolation/handoff:** each run gets one fresh worktree; steps share it so later steps
  see earlier steps' code. Each step also receives the previous step's final report as
  prompt context. Worktrees are removed when the run's work is merged or abandoned
  (DISK-CRITICAL rule applies).
- **Building recipes:** a form in the Agents surface — name, steps list, reorder.
  Stored as JSON (app data, optionally per-repo) so recipes survive and can be shared.
- The Agents surface rework itself is deferred until the fixes in this roadmap land;
  the recipe builder is its eventual centerpiece.

## 3. Browser overlay (Codex-app style — pinned by owner screenshots)

- Per-session, full-window overlay covering the left sidebar. Browser state — page,
  tabs, annotations — belongs to the session and swaps entirely when the session
  changes.
- The session's own composer floats bottom-center ("Do anything", same model/effort
  pill, attachment and access chips). The turn status strip ("Worked for 19m 27s")
  stays visible above it while browsing.
- **Annotate mode:** an "Annotating" chip top-right; boxing a region opens an inline
  comment field at the region; each saved annotation becomes a numbered marker on the
  page.
- **Batch send:** annotations collect into a counted chip ("1 annotation", expandable
  to thumbnail + comment). The header shows "Annotating · <url>" with close/trash and a
  **Send (n)** button; one send delivers all annotations (region screenshots +
  comments) plus the typed prompt to the session.

## 4. Git panel completeness

All four, reusing the existing panel and context menus:

- **Discard changes** — per file and all-changes, with confirmation.
- **Branch operations** — create/switch, clear current-branch display, stash.
- **PR flow** — open a PR from the current branch via `gh`; show open PRs and their
  check status for the repo.
- **Commit ergonomics** — amend, stage-all shortcut, suggested commit message generated
  from the staged diff.

## 5. Resources upgrades

- **Kill/stop from panel** — stop a session's process tree or a stray daemon directly
  from its Resource Manager row (with confirm; never silently).
- **History sparklines** — small CPU/RAM-over-time graphs per session and for the app,
  so leaks and spikes are visible as shapes, not just numbers.
- **Disk usage** — track the known disk hogs: cargo target dirs, worktrees,
  node_modules, transcript/usage stores; show sizes and let the owner reclaim from the
  panel. (Motivated by the 67 GB cargo-cache discovery.)
- No alert thresholds for now.

## 6. Session rail cleanup

- **Bug:** the My Work rail must scroll when the session list is taller than the
  window. Today it cannot.
- Rows restyled to look better (aligned with the transcript/color polish below).
- **Row quick-jump buttons** (hover/button overlay on each row): the working (orange)
  indicator is a button — clicking jumps straight to that session's Session tab; beside
  it small Editor / Git (and possibly Agents) buttons jump to that session with that
  surface already selected. One click from the left rail to the exact surface — no
  left-click-then-mouse-to-the-far-right trip.

## 7. Conversation capabilities

- **Images in prompts:** paste (⌘V) or attach screenshots/files in the composer;
  attachments stay visible until Send.
- **Slash commands:** a composer menu surfacing the agent's advertised commands
  (`/compact`, `/review`, custom skills).
- **Plan & sub-agent display:** the agent's plan steps and child-agent activity render
  as structured, collapsible transcript sections.
- (@-file references: not now.)

## 8. Polish pass

- **Transcript rendering:** spacing, code-block styling, collapsing long tool output,
  markdown quality in replies.
- **Color & contrast pass:** a deliberate accent/color pass across surfaces — the app
  should stop reading as a "contrast theme".
- **Animations & transitions:** smooth surface/tab switches, popover and session-open
  transitions instead of instant jumps.
- (Keyboard shortcuts: not now.)

## Queued quick fixes (already in the ledger)

- Remove the green focus ring on the composer textarea (focus = subtle border token).
- Claude model list shows only Default/Sonnet/Haiku — investigate whether
  `session/set_model` accepts other ids (Opus/Fable) or the adapter needs a flag.
- Notification permission + Keychain Always Allow are owner clicks pending in the app.

## Build order (owner-approved)

1. Quick fixes + transcript/color polish (daily feel)
2. Session rail cleanup (scroll bug + quick-jump buttons)
3. Editor modes (LSP toggle; read mode default)
4. Git panel completeness
5. Browser overlay
6. Conversation extras
7. Resources upgrades
8. Workflow recipes
9. Agents surface rework (last)
