# Workbench Restructure — Design Spec

Date: 2026-08-13. Owner-approved in conversation. Supersedes the queued "uniformity pass" (ledger item 70) — that work folds into this restructure.

## Goal

Restructure the /next workbench into the owner's target layout: a single Orca-style icon tab strip over the right panel, VS Code-style small tabs on the center pane, the right icon rail deleted, and every panel rebuilt on shared kit primitives so the whole app is uniform and polished. The owner will not use the app until this lands — optimize for landing the whole thing, not per-wave usability checkpoints.

## Reference screenshots

All under `/Users/blackcolours/.claude/image-cache/6dffc68b-2b1e-4ba3-8751-3dc89475e376/`:

- `173.png`–`178.png` — Orca session-history cards (grouping, hover actions, expanded detail, subagents list, more-actions menu). Target look for the History panel.
- `179.png`–`184.png` — Orca source-control panel (branch header, message box, Stage All, file lists, commit timeline, menus). Target look for Source Control; v1 scope reduced below.
- `185.png`, `186.png` — T3 browser: fills right panel; annotate toolbar (Select/Region/Draw/Erase) with attach-to-prompt; expanded browser covering the app.
- `187.png` — Orca worktrees panel (the "commands to run yourself" blocks that become agent-run buttons).
- `188.png` — Resources/Usage icons at the bottom of a rail (target: bottom strip of right panel).
- `189.png` — the current far-right icon rail. This is deleted.
- `190.png` — "No agents yet" Agents empty state (target copy/tone for Agents panel).
- `191.png` — settings gear placement reference; settings moves to the lower-left corner of the app.
- `192.png` — Orca icon strip at the top of its right panel. Target for our right-panel tab strip.
- `193.png` — VS Code-style small icon tabs in a pane corner. Target for the center pane's Session/Editor/Diff tabs.
- The two unnumbered screenshots in the same conversation turn as the Run-tab answer — T3 "Add action" top-bar button and its Add Action dialog (Name, Keybinding, Command, Preview URL, "Run automatically on worktree creation", "Open preview automatically when this action runs"). Target for the Run panel.

## 1. Geometry

Three columns plus a slim top bar:

- **Left rail** — the session list, unchanged in role. New: a Settings gear pinned to the bottom-left corner of the app (opens the existing settings surface). The rail keeps its recent improvements (row actions, needs-you, drag reorder, popout).
- **Center pane** — the main work surface. Small VS Code-style tabs in its **upper-right corner**: **Session · Editor · Diff** (icon + short label, kit-sized, like `193.png`). Session is the default.
- **Right panel** — resizable as today. Across its top, an Orca-style **icon tab strip** (like `192.png`) with exactly eight tabs in this order: **Files · Source Control · Worktrees · Run · Context · Agents · Browser · History**. The selected tab's content fills the panel. At the panel's **bottom**, a slim always-visible strip hosting **Resources** and **Usage** (the two surfaces currently on the far-right rail), opening as popovers or a bottom sheet — not tabs.
- **Top bar** — slims down to window controls + the Language Intelligence controls. The `+ Run` button moves into the Run panel. The session-history browser button dies (History panel replaces it).

**Deleted:** the far-right icon rail (`189.png`), the old right-side tab row, the top-bar Run button, the top-bar session-browser button.

Tab selection state (center tab + right tab) persists per session in localStorage. Keyboard: existing shortcuts keep working; no new global shortcuts required in v1.

## 2. Center tabs

- **Session** — the conversation surface, unchanged.
- **Editor** — the current editor surface, unchanged internally.
- **Diff** — the current diff view relocated from its old home to a center tab. Clicking a changed file in Source Control opens that file's diff here; clicking a file in Files opens it in Editor. Cross-panel navigation goes through one small module (`workbenchNavigation`) so panels never import each other.

## 3. Panels (right side)

### 3.1 History (Orca-style session cards)
Replaces the current session-browser overlay. Cards grouped by project with a count chip per group (`173.png`). Each card: title, meta line (agent icon, msgs count, subagents count, age, model id). Hover actions: resume, continue-in-new-session, expand/hide details, more-actions menu (`176.png`–`178.png`). Expanded card (`174.png`): action row (Resume in Worktree / Continue in New Session / View Log), First Prompt block with Copy, Latest Turns (last few user/agent snippets), Subagents list (name, type chip, msgs count, log link — `175.png`). More-actions menu (`178.png`): Resume in Worktree, Continue in New Session, Copy Resume Command, Open Log, Reveal Log, Open Working Directory, Copy Session ID, Copy Log Path, Delete (destructive, confirm). Data comes from the existing history scanner; actions reuse the existing resume/continue backends where they exist — any action lacking a backend ships disabled with honest hint text and is reported.

### 3.2 Source Control (read + basics)
V1 scope (`179.png` look, reduced writes): header with current branch vs base and total diffstat; commit message box; **Stage All** and **Commit** (these two write operations only); sections for Untracked and Changed files with per-file diffstat and status letter; commit timeline (subject, author, date, per-commit files on expand; commit-message popover on hover like `180.png`); per-file context menu: View (opens center Diff), Copy Path, Copy Relative Path, Open in VS Code/Finder. Clicking a file opens the center Diff tab. **Not in v1:** push, pull, sync, rebase, fast-forward, Create PR, Change Base Ref, tree view — the header reserves space for a follow-up "more actions" menu. New Rust commands as needed for status/log/stage/commit; all git via Command with args, never a string shell.

### 3.3 Worktrees (agent-run cleanup)
Keep the current inventory (per-repo worktree list, status badges, filter) but cleaned up onto kit primitives. The "COMMANDS TO RUN YOURSELF" copy-paste blocks (`187.png`) are replaced by **action buttons**: Inspect, Archive & Remove, Remove. Each button **spawns a short-lived agent session** in/for that worktree (visible in the left rail like any session) whose prompt instructs it to: inspect the worktree state, explain what it found in plain English, warn about anything risky (dirty files, unpushed commits, unmerged branches), and only then perform the requested cleanup, reporting what it did. Uses the existing session-spawn machinery; a small prompt-template module owns the per-action prompts. Never silently deletes dirty/unmerged work — the agent must surface it.

### 3.4 Run (T3-style project actions)
Project-scoped actions replacing the top-bar Run button. Panel: list of saved actions (name, keybinding chip, command preview, run button), an **Add action** button opening a dialog matching T3's: Name, Keybinding (capture field), Command, Preview URL (optional), toggle "Run automatically on worktree creation", toggle "Open preview automatically when this action runs". Below the list: running processes with live output tail, stop/restart per process. Preview URL opens in the Browser panel when the action runs (when its toggle is on). Actions persist per project (extend the existing run/actions storage if present; else a new store in app data). Auto-run-on-worktree-creation is stored and honored if a worktree-creation hook exists; otherwise stored and honestly labeled "takes effect when worktree creation lands".

### 3.5 Context (session context/usage)
For the selected session: model + effort + access mode, token/context usage (whatever the adapter reports today — no fabricated numbers), files read/touched this session (from the transcript/tool events already streamed), attachments. Read-only in v1. Where a number is unavailable for a provider, show "not reported" — never invent.

### 3.6 Agents (live subagent activity)
V1: the selected session's subagents, live: name/label, status (working/done/failed), activity line, msgs count, log link when available. Empty state copy per `190.png`: "No agents yet — when this thread spawns subagents or runs a workflow, they show up here with live status, activity, and token usage." Workflow authoring is explicitly **out of scope** (later wave; ledger items 41–42).

### 3.7 Browser (fill + expand + annotate)
The browser fills the right panel. An **expand** control stretches it leftward over the center pane, up to the left rail's edge (`186.png` reading); collapse restores. Full T3-style annotation (`185.png`): toolbar with **Select / Region / Draw / Erase**; Select highlights DOM elements and captures the element (tag chip like "svg"); Region drags a rectangle; Draw freehand; Erase removes marks. An inline mini-composer ("Describe the change… / Attach") attaches the annotated screenshot (PNG of the page with annotations burned in) plus a structured note (URL, element selector/tag when Select was used, user's description) to the active session's composer as an attachment. Screenshot capture uses the existing webview/browser machinery; if the current embedded browser cannot produce a composited screenshot with annotations, burn annotations client-side onto a captured image in the app layer. Nothing is sent until the user sends the prompt from the main composer.

### 3.8 Files
File tree of the active session's checkout (cwd/worktree): folders collapsible, click file → opens center Editor tab on that file. Reuses the editor's existing file-open path. Basic niceties only: git-ignored files dimmed or hidden (pick one, be consistent), no rename/move/delete operations in v1.

## 4. Shared kit primitives (the uniformity pass, folded in)

Wave 1 ships these in `src/lib/shell/kit/` (or the existing kit location) and every panel lane must consume them — no panel-local buttons/rows/headers:

- **PanelHeader** — title, optional count chip, right-side action slot.
- **ListRow** — the standard row: content + reserved 28px hover-action cluster (18px glyphs), matching the rail's settled sizing.
- **HoverActionCluster** — the settled `size="sm"` kit buttons with tooltips.
- **Badge/Chip** — status and count chips (needs-you amber, presence colors, count).
- **Menus/popovers/dialogs** — existing shadcn kit components only.
- **EmptyState** — icon + title + body, per `190.png` tone.

Rules that bind every lane: 12px minimum font; CSS grid/flex layouts, no magic-number absolute positioning except where the design demands overlay; NO `:has()` selectors or `has-[` utilities; every animation ends at rest; TypeScript only, `any` last resort; plain-English UI copy.

## 5. Backend work (rides inside the owning lane)

- Source Control: Rust commands for status/diffstat/log/stage/commit (Command-with-args only).
- Worktrees: prompt-template + spawn wiring onto the existing session-start machinery; a worktree-inspect Rust helper if the panel's current data is insufficient.
- Browser: screenshot capture + annotation compositing + composer-attachment plumbing.
- Run: actions store (per-project persistence), process spawn/stop with output streaming (reuse the existing run infrastructure).
- Context: read-only aggregation over data the manager already holds.
- History/Files/Agents: no new backend expected; disabled-with-hint for anything missing, reported.

## 6. Execution plan shape

- **Wave 1 — shell lane (sequential, lands first):** new geometry, center corner tabs, right-panel icon strip, bottom Resources/Usage strip, settings gear lower-left, deletions (right rail, old tab row, top-bar Run/history buttons), kit primitives, re-hosting today's existing content into the new slots (existing browser → Browser tab, existing diff → center Diff tab, existing worktrees → Worktrees tab, etc.), `workbenchNavigation` module. App must be fully working at this wave's end, just with old panel content in new slots.
- **Wave 2 — eight parallel panel lanes (Opus 5, Agent tool):** History, Source Control, Worktrees, Run, Context, Agents, Browser, Files. Strict file ownership: each lane owns only its panel directory (+ its Rust module where applicable); shared files (`+page.svelte`, kit, navigation) are frozen after Wave 1 — a lane needing a shared-file change reports it and the merge agent/controller applies it.
- Merging, gate-running, and worktree cleanup are delegated to an Opus 5 merge agent where possible; the controller reviews reports and makes small fixes only.

## 7. Gates and hard rules (paste into every lane)

- `pnpm run check:svelte` ends `Files the /next shell owns: 0 error(s), 0 warning(s)`.
- Rust lanes: `RUST_TEST_THREADS=1 RUSTFLAGS="-D warnings" cargo test <module>` green, `cargo fmt -- --check` clean, one cargo at a time.
- Script tests via `node --experimental-strip-types`; update guards they own; never touch the known-red `nextTokens.test.mjs` allowlist or `conversationActivation.test.mjs`.
- Never bind port 5177; lane vite on 5181/5182/5183 only; sandbox usually blocks sockets — attempt once, report, move on; controller verifies visually at 1710×990 (verified via innerWidth eval).
- No commits/pushes by lanes; plain commit messages by whoever commits; no co-author trailers; no AI vendor/model mentions in code, comments, or reports; never log tokens/credentials/prompt text.
- Worktrees under `/Users/blackcolours/dev/work/worktrees/mac-command-bar/<lane>`, node_modules symlinked from the wave worktree, symlink removed before `git worktree remove`, removal verified with `git worktree list`.
- Reports first: every lane writes its receipt-report file before doing anything else; every claim carries file:line or command output, labelled verified/assumed.

## 8. Out of scope (explicitly deferred)

- Git write operations beyond Stage All + Commit (push/pull/sync/rebase/PR/base-ref).
- Workflow authoring in Agents (later wave), model handoffs, supervisor.
- Worktree *creation* backend (New worktree in the new-session pane stays honestly disabled).
- File operations in Files (rename/move/delete).
- The ticker/age-display rebuild; context-menu missing backends (rename/fork/reveal); main.rs split; DB browser.
