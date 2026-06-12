# TSK-127 Implementation Plan

## Goal

Build MacCommandBar into a daily-driver macOS command center for agent-heavy development:

- browse and lightly edit code with IDE-grade navigation;
- see Git, worktree, runtime, session, and orchestration state at a glance;
- resume the right agent/session/worktree without manual setup;
- keep clipboard/paste cleanup and local utility workflows close at hand;
- stay much lighter than a full IDE.

## Execution Rules

- Work one slice at a time. Do not start the next slice until the current one has a checkpoint commit or an explicit blocked note.
- Keep changes tied to this plan unless the user interrupts with a higher-priority bug.
- Prefer compact UI and command-palette actions over always-visible buttons.
- Do not add destructive Git/worktree actions without confirmation, backup, or a dry-run plan.
- Do not scan every worktree/repo by default. Use selected project roots, cached state, and explicit refreshes.
- After browser or Playwright testing, shut down the browser automation process and report that cleanup.
- Each integration checkpoint needs: changed files, behavior delivered, validation commands, and remaining risk.

## Current Baseline

Completed and committed:

- Native Rust core scans source files, Git state, worktrees, processes, sessions, runtime contexts, orchestration events, and local source actions.
- Tauri + Svelte preview UI is the main fast iteration surface.
- Monaco editor is wired with syntax highlighting, theme/font settings, save, hover, diagnostics, symbols, definition, and references.
- C# and TypeScript LSP bridges are wired.
- Source browser has directory tree, scrollbars, filtering, content search, recent files, open tabs, dirty indicators, native read/write, and root auto-correction.
- Command palette exists and carries lower-frequency actions.
- Git basics exist: summaries, dirty state, diffs, stage, unstage, commit, fetch, pull, push, history, task-id extraction.
- Worktree safety exists: stale/dirty checks, backup/remove/audit command copy, compact cleanup decision lanes, open in browser/terminal/embedded terminal, and compact live-session/saved-workspace ownership chips.
- Orchestration event model exists: event CLI, native event store, UI run cards, scenario/issue/fix/retest/sign-off tallies, timeline helpers, `/run-e2e-tests` sample events, artifacts, task links, command copy actions, and compact live digest chips for decisions, retests, UI proof, fixes, and handoffs.
- Embedded terminal exists as a basic PTY-backed surface.
- Workspace restore snapshots preserve project, worktree, branch, selected file/line, open tabs, view mode, terminal choice, browser URL, and layout state.
- Embedded terminal sessions now resolve matching workspace snapshots, show saved-workspace readiness inline, expose copyable focus plans, and can restore/attach through compact command-palette actions.
- Agent/session rows now show compact recommended focus lanes for reattach, repair, restore, resume, files-only, and save-workspace states across the activity list, context card, and terminal dock.
- Saved nested project roots now auto-repair to the real Git root before scan.
- Hidden dock panels now expose compact restore chips, so closed panes are recoverable without opening the command palette first.
- Source scans now show compact evidence for cache/fresh/background/tiny/failed/stopped state, root label, count, age, and scan limit.
- Project activation scans now share an explicit cache/missing/force/tiny-index plan, so auto-scans explain why they are running.
- Project controls now show a compact setup notice with validation state, detected Git root, scan evidence, and scan cap so add/switch/restore does not feel mysterious.
- Editor LSP recovery actions are visible in the editor toolbar: retry status, copy selected-file status, and copy install command when fallback is active.
- Editor navigation now has a compact local drawer for problems, symbols, definitions, and references so common code-browsing results do not require keeping the full insight pane open.
- Editor LSP command depth now stays behind the command palette and action menu: local problems/symbols/definition/reference drawers, clear lookup results, completions, signature help, and copyable source intelligence briefs are available without adding persistent chrome.
- Git task source rows now copy a full task handoff with task link, source summary, and source details; the same action is available from the command palette, and task ledger rows expose compact open-task actions.
- Git history rows now use normalized ownership badges for HEAD, upstream refs, tags, task IDs, merges, and root commits while keeping raw refs in copyable detail/handoff text.
- Git task ledger rows now show owner and cleanup summaries, stale-clean candidates, backup-needed worktrees, active sessions, saved workspace ownership, runs, and commits without adding persistent panels.
- Git history now uses compact selected-commit drawers, dense commit metadata lines, and hover/focus row actions so larger histories fit without widening the Git pane.
- Worktree cleanup now understands Git `prunable` and `locked` metadata: missing paths show as metadata-prune review items, locked worktrees block removal, dirty/unmerged worktrees still route to backup/archive guidance, and native remove prunes missing metadata without adding a force-delete path.
- Conversation workspace restore now treats missing saved worktrees as repairable context drift: rows and command-palette actions copy an audit/recreate plan, and terminal resume copies that plan instead of opening a dead path.
- Conversation workspace restore now reopens saved files immediately while project indexing runs in the background; background scans preserve the restored file selection if the refreshed index cannot see that path yet.
- Orchestration event ingest now accepts real-ish agent/orchestrator payloads: snake_case aliases, nested project/task/agent/step/artifact/link/counts objects, event aliases such as `ui_verified`, JSON arrays, JSON envelope events, and JSONL/JSON files via `--json-file`.
- The Runs pane now has a compact orchestration ingest strip for choosing an event JSON/JSONL file, copying the exact import command, and recording a native heartbeat event without starting a watcher by default.
- Paste cleanup now keeps compact local history for copied cleaned text and reply drafts, exposes restore/copy chips in the Clipboard pane, and keeps history actions available through the command palette.
- Native Tauri/LSP validation currently passes through `pnpm test:tauri-app`, including Tauri source bridge wrappers, dev/attach config guards, C# and TypeScript language-server smoke tests, and the Tauri Rust build.

Current checkpoint:

- Priority 0.1 docking is implemented enough for daily iteration: persisted dock model, panel close/restore, side/bottom groups, resizable panes, stacked context cards, hidden-panel restore chips.
- Priority 0.2 project scan stability is implemented for nested-root repair, scan evidence, explicit activation scan reasons, forced refresh on missing/stale/tiny activation plans, and worktree-ancestor scan coverage. Remaining work is hands-on validation on more real project roots and any follow-up from user testing.
- Priority 0.3 native validation has current automated coverage through `pnpm test:tauri-app`: source bridge wrappers, dev/attach config, web build, 25 native LSP tests, TypeScript and C# language-server smoke, and Tauri Rust build. Remaining work is hands-on app validation for the full GUI flow.
- Priority 1 Git/worktree/conversation foundations are already implemented. Worktree rows now expose linked saved conversations/snapshots and live session ownership. Git task source handoffs are copyable, task ledger rows can open task links directly, history rows show compact ownership badges, and the task ledger summarizes cleanup/ownership state. Worktree cleanup now has covered states for clean, stale-clean, dirty, unmerged, locked, active-session, missing/prunable, and primary checkout flows, with compact lane labels for keep/active/backup/save-commits/stale-clean/prune/locked/review. Conversation workspace restore now has covered missing-worktree repair plans, terminal/path safety, and non-blocking restore with background index preservation. Remaining work is hands-on GUI validation and larger graph UX, not first scaffolding.
- Priority 1.4 Git graph density and task detail polish is implemented enough for iteration: selected commit details collapse to a one-line drawer, full metadata/actions sit behind disclosure and Cmd+K, commit rows use dense metadata, and row action buttons only appear on hover/focus.
- Priority 1.5 worktree cleanup confidence polish is implemented enough for iteration: shared decision-lane labels distinguish keep-main, active-session blocked, backup dirty work, save unmerged commits, stale clean cleanup, missing metadata prune, locked, and generic review states across the Worktrees activity list, decision queue, and safety context list.
- Priority 2.7 orchestration timeline/detail and event ingest are implemented enough for iteration: event schema, sample event generation, run cards, compact context rows, current activity, loop tallies, attention/sign-off queues, handoff copy, import normalization, JSON/JSONL file ingestion, selected import paths, import command copy, and native heartbeat recording are covered by tests.
- Priority 2.8 orchestration live digest polish is implemented enough for iteration: run cards and compact context rows now surface the highest-signal live ingest items as tiny chips for sign-off decisions, retest loops, handoffs/artifacts, UI proof, resolved fixes, and delegated fix batches without adding a watcher or persistent chrome.
- Priority 2.9 agent/session focus action polish is implemented enough for iteration: a shared focus-lane helper now labels the safest next action for each session, and conversation rows, agent rows, the compact agent context card, and the terminal dock show those lanes without adding more persistent toolbar chrome.
- Priority 3.9 LSP recovery and command depth are implemented enough for iteration: status/fallback state is visible on the editor file badge, recovery actions are available from the toolbar and command palette, local drawer actions cover problems/symbols/definitions/references, completions/signature help are tucked into palette/menu surfaces, copyable intelligence briefs exist, and C#/TypeScript native LSP smoke tests pass.
- Priority 4.11 terminal session restore polish is implemented enough for iteration: embedded PTYs can copy focus plans, restore matching workspace snapshots, attach directly when no snapshot exists, and route missing-worktree restores to the same repair-plan guard as conversation snapshots.
- Priority 4.13 clipboard paste cleanup utility polish is implemented enough for iteration: cleaned output and reply drafts are saved to capped local history on copy, recent items render as compact restore/copy chips, history can be cleared, and Cmd+K exposes restore/copy/clear actions.

## Priority 0: Make The App Usable As A Workspace

### 1. Real Docking And Pane Layout

Target result:
- The app behaves like an IDE canvas, not a dashboard squeezed into cards.

Work:
- Add a dock model with zones: activity rail, left side pane, editor center, right side pane, bottom pane, overlay/flyout.
- Support resize, move, stack, pin, close, restore, and persisted layouts.
- Make cards closable individually and restorable through command palette or layout menu.
- Keep directory tree, conversations, active sessions, agents, worktrees, Git, and orchestration as dockable panes.
- Make editor fill remaining height and width.
- Replace bulky status chips/buttons with toolbar icons, menus, shortcuts, or palette actions.

Acceptance:
- Half-width laptop layout has no overlapping controls.
- Editor can occupy most of the window from top to bottom.
- Left pane can switch between files, conversations, active sessions, agents, worktrees, and Git without changing the editor.
- Right/bottom panes can be hidden, stacked, resized, and restored.
- Layout persists across reload/restart.

Validation:
- `pnpm test:source-ui`
- `pnpm test:source-dock-layout`
- `pnpm check`
- `pnpm build`
- Browser screenshots for wide, half-width, and narrow layouts, followed by browser automation shutdown.

### 2. Stabilize Project Add And Scan

Target result:
- Adding a project feels boring and reliable.

Work:
- Make add-project validate the selected folder and store the detected Git root.
- Auto-scan once after project add/switch/restore when cache is missing or stale.
- Show scan status, file count, limit used, and root being scanned.
- Keep the default scan large enough for real projects without freezing the UI.
- Provide explicit full-rescan and rebuild-index commands.
- Surface ignored folders and scan limits when results look suspicious.

Acceptance:
- EdiPlatform scan no longer returns only the two stale files after root repair.
- User does not need to repeatedly press Scan for normal project switching.
- Scan stays responsive and cancellable.

Validation:
- `pnpm test:source-ui`
- `pnpm test:local-source-fs`
- `cargo test --manifest-path core/Cargo.toml`
- Manual scan of at least MacCommandBar and one large project.

### 3. Native Tauri Validation

Target result:
- Browser preview behavior and packaged Tauri behavior match for file, Git, LSP, session, and terminal actions.

Work:
- Run the real Tauri app, not only browser preview.
- Verify source read/write, Cmd+S, LSP startup, hover, definition, references, diagnostics, Git actions, terminal launch, and embedded terminal.
- Add targeted smoke tests where practical.
- Document the exact run commands.

Acceptance:
- C# and TypeScript LSP actions work inside Tauri.
- Native commands do not depend on browser preview bridge behavior.
- Known limitations are written down.

Validation:
- `pnpm tauri dev` or equivalent dev attach flow.
- `pnpm check`
- `pnpm build`
- Targeted Rust tests for native command paths.

## Priority 1: Git, Worktrees, And Task Context

### 4. Git Graph And Task Links

Target result:
- The app shows what changed, where it belongs, and what Notion/task branch it maps to.

Work:
- Existing: Git history pane, graph markers, commit selection, compact selected-commit drawer, dense commit metadata, hover/focus row actions, branch health chips, staged/unstaged/untracked groups, selected-file diff, task extraction, Notion task links/fallback search, commit-message entry, fetch/pull/push/stage/unstage/commit, copyable task-source handoffs, command-palette task-source actions, compact task-open ledger actions, and normalized HEAD/upstream/task ownership badges.
- Next: validate larger histories in the real app and decide whether a full branch graph lane is worth the complexity.

Acceptance:
- User can tell which repo/worktree is dirty, for how long, and which task it belongs to.
- User can stage, unstage, commit, fetch, pull, and push without leaving the app.
- No destructive action runs without confirmation.

Validation:
- `cargo test --manifest-path core/Cargo.toml`
- `pnpm test:source-ui`
- Manual Git flow in a disposable test repo.

### 5. Worktree Command Center

Target result:
- Worktrees stop being mystery folders and become manageable work contexts.

Work:
- Existing: worktree list, branch/task/dirty/unmerged/activity fields, active-session blocking, saved conversation/workspace ownership chips, safety status, decision queue, copy cleanup plan, backup command, remove clean worktree, open in source browser, open external/embedded terminal, task links, and compact task-ledger owner/cleanup chips.
- Next: validate the full worktree cleanup flow manually with clean, dirty, active-session, and missing-path examples.

Acceptance:
- User can decide what can be deleted, what needs review, and what is actively being used.
- Dirty/unmerged worktrees cannot be silently removed.
- Worktree cleanup produces clear backup/remove commands.

Validation:
- `pnpm test:worktree-safety`
- Rust scanner tests.
- Manual dry run with clean, dirty, and missing-path worktrees.

### 6. Conversation Workspace Restore

Target result:
- Selecting a conversation/session restores the correct project, worktree, branch, files, layout, and terminal context.

Work:
- Store workspace snapshots with conversation/session ids.
- Show snapshot readiness and missing context inline.
- Resume Codex/Claude/CMUX session with the right cwd/worktree.
- Restore editor tabs, selected file/line, side pane, layout, terminal app, and browser URL.
- Add "save current workspace to conversation" and "restore conversation workspace" palette actions.

Acceptance:
- Moving between conversations does not require manually choosing repo, worktree, files, or session.
- Missing worktree/project is reported clearly with a repair path.
- Restore should reopen saved files immediately and let indexing refresh in the background without stealing the restored file selection.

Validation:
- `pnpm test:workspace-snapshot`
- `pnpm test:source-ui`
- Manual restore for at least one saved Codex and one CMUX-like context.

## Priority 2: Orchestration And Agent Visibility

### 7. Orchestration Timeline

Target result:
- Long agent loops show live state instead of opaque terminal output.

Work:
- Existing: orchestration event CLI, native event store, run cards, timeline helpers, artifacts, task links, copyable event commands, compact event-file import controls, native heartbeat recording, scenario/agent/issue/fix/UI-test/retry/approval/blocker fields, live tallies for found/fixed/retested/sign-off, decision queues, current activity summaries, and compact live digest chips for new handoffs, UI proof, retests, fixes, and sign-off needs.
- Next: ingest richer real orchestrator output from live agent loops and attach artifacts/transcripts as they are produced.

Acceptance:
- User can answer "what is this agent doing now?" without reading the whole transcript.
- User can see which bugs were found, fixed, retested, and which need sign-off.
- Orchestration runs are grouped by project/worktree/task.

Validation:
- `pnpm test:orchestration-event`
- `pnpm test:orchestration-view`
- Manual event ingest with `scripts/mcb-orch`.

### 8. Agent And Session Focus Actions

Target result:
- Clicking a session gets the user back to the right terminal/app/context.

Work:
- Existing: Codex, Claude, and CMUX session rows, workspace snapshots, repair-plan guards for missing worktrees, external terminal resume, embedded terminal resume/attach, copyable focus plans, shell resume command copy, and compact recommended focus lanes for reattach/repair/restore/resume/files-only/save states.
- Next: improve native session identity and app-focus commands for Warp, Ghostty, Terminal, and CMUX where those apps expose stable targets.
- Continue showing session age, cwd, branch, worktree, status, active command, and last activity.
- Avoid paid Warp agent features; use Warp only as a terminal target.

Acceptance:
- User can jump to the matching session or resume it with one command.
- Session list does not expose noisy transcript snippets as titles.

Validation:
- Rust session scanner tests.
- Manual focus/resume commands on available terminal apps.

## Priority 3: Editor Depth

### 9. LSP Hardening

Target result:
- Code browsing is good enough for day-to-day review, even if heavy edits still happen in a full IDE.

Work:
- Harden C# and TypeScript LSP lifecycle, workspace roots, cancellation, diagnostics, and reconnect.
- Add support for more languages by adding language server adapters, not rewriting editor logic.
- Improve definition/references UI so results feel in-editor, not like external cards.
- Add symbol outline, quick open, problems list, and references popup/flyout.

Acceptance:
- F12, Cmd-click, Shift+F12, hover, diagnostics, and symbols are reliable across open files.
- LSP failures are visible and recoverable.

Validation:
- `pnpm test:source-ui`
- Native Tauri manual checks for C# and TypeScript projects.

### 10. Lightweight Editing Polish

Target result:
- Safe small edits are comfortable without turning the app into a full IDE clone.

Work:
- Keep Cmd+S as primary save.
- Add dirty tab indicators and unsaved-close prompts.
- Add revert, compare, copy path, reveal, open in IDE, and command-palette actions.
- Keep formatter/refactor/build/run out of scope until navigation is solid.

Acceptance:
- User can tweak a file, save it, inspect the diff, and commit it.
- App never loses unsaved edits silently.

Validation:
- `pnpm test:source-ui`
- Manual edit/save/revert/diff flow.

## Priority 4: Terminal, Browser, And Utilities

### 11. Embedded Terminal Decision

Target result:
- Decide whether the embedded terminal becomes first-class or stays a convenience surface.

Work:
- Stabilize basic PTY terminal sessions.
- Support multiple sessions tied to project/worktree/snapshot.
- Add terminal restore with cwd/branch/context.
- Compare embedded terminal behavior against CMUX/Warp/Ghostty workflows.
- Defer libghostty until the current PTY/xterm path is proven insufficient.

Acceptance:
- User can run Codex/Claude/basic shell commands in-app without breaking layout.
- Sessions survive normal workspace switching well enough for daily testing.

Validation:
- Manual terminal launch, resize, input/output, cwd, and restore checks.

### 12. Browser Surface Decision

Target result:
- Browser integration is added only if it supports the agent workflow better than launching an external browser.

Work:
- Keep browser lower priority than terminal and orchestration.
- Evaluate a Tauri webview panel for local app preview and artifact inspection.
- Tie browser URL to workspace snapshots.

Acceptance:
- Browser panel does not make the main IDE layout cramped.
- Browser state restores with project/session snapshots.

Validation:
- Manual local URL load and restore check.

### 13. Clipboard And Paste Cleanup

Target result:
- Clipboard workflows remain part of the command center without dominating the IDE surface.

Work:
- Keep clipboard vault and paste cleanup in utility panes/palette.
- Add editor-like paste cleanup surface for drafting responses.
- Provide copy-back and history actions.
- Existing: read clipboard, cleanup modes, cleaned output, reply draft normalization, copy-back actions, capped local history, compact history chips, command-palette restore/copy/clear actions.

Acceptance:
- User can paste a long answer, clean/edit it, copy it back, and keep history.

Validation:
- `pnpm test:paste-cleanup`
- Existing Swift/core clipboard tests where applicable.

## Parking Lot

Do not start these until the higher priorities are usable:

- Full browser automation dashboard.
- Heavy refactoring/code actions.
- Full VS Code extension compatibility.
- Cross-device sync.
- Cloud-hosted state.
- Destructive bulk cleanup without explicit dry runs.

## Next Slice

Last checkpoint: Priority 0.2, project scan/add-project hardening.

- Changed files: `tauri-svelte-preview/src/routes/+page.svelte`, `tauri-svelte-preview/scripts/sourceUi.test.mjs`, `tauri-svelte-preview/scripts/localSourceFs.test.mjs`, `tauri-svelte-preview/src-tauri/src/main.rs`, `docs/TSK-127-IMPLEMENTATION-PLAN.md`.
- Behavior delivered: project activation now turns missing, stale, forced, or suspiciously tiny activation scan plans into a real refresh so stale two-file indexes cannot remain visible during add/switch/repair; browser bridge and Tauri scanner tests now cover roots that live under a `/worktrees/...` ancestor while still allowing repo-local child `worktrees` folders to be skipped.
- Validation: direct local scan of `/Users/blackcolours/dev/work/EdiPlatform` returned 4,591 files; `pnpm test:local-source-fs`; `pnpm test:source-ui`; `cargo test source_scan --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml`; `pnpm test:source-data`; `pnpm check`; `pnpm build`.
- Remaining risk: hands-on validation in the real Tauri GUI still needs to confirm add-project, switching, LSP, Git, and native file commands together.

Next implementation slice: Priority 0.3 native Tauri GUI validation.

Definition of done for the next slice:
- Run the real Tauri app without triggering the dev relaunch/focus loop.
- Verify add-project/switch-project scanning against MacCommandBar and EdiPlatform roots.
- Verify native source read/write, Cmd+S, LSP status, hover/definition/references, Git status/diff/actions, and embedded terminal startup.
- Record any native-only failures as targeted follow-up items instead of mixing them into broad layout polish.
- Validate with `pnpm test:tauri-app` or the closest focused subset, `pnpm test:source-ui`, `pnpm check`, `pnpm build`, `git diff --check`, and a short hands-on Tauri smoke pass.
