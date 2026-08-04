# TSK-808 Assembly Native Workbench Product Wave Implementation Plan

> For implementation agents: follow this plan one work package at a time. Use Luna Max for
> straightforward, scope-locked implementation and SOL-medium when substantial design choices or
> judgment remain. SOL-medium performs every milestone code review. Set `agent_type`, `model`,
> `reasoning_effort`, and `fork_turns: "none"` explicitly; stop before editing if an override fails.

**Goal:** Rebrand the app to **Assembly** and turn the existing /next shell into a legible,
native-feeling workbench that joins sessions, projects, Git, pull requests, browser feedback,
resources, Markdown, run configurations, diagnostics, and guarded agent actions without
replacing the working shell, losing existing user state, or regressing the semantic Roslyn
CodeLens behavior delivered by TSK-799.

**Architecture:** Keep Svelte components as views, plain TypeScript services as imperative
orchestrators, Svelte rune stores as state-only containers, and Rust/Tauri modules as the
validated native boundary. Expand existing Dockview, session-workspace, local-Git, PTY, LSP,
append-only orchestration abstractions, and the one app-owned `AgentRuntimeManager` defined by the
2026-08-04 amendment. Do not add a second shell, editor, session router, Git service, worktree
manager, settings store/host, agent runtime, workflow ledger, or Paneview authority.

**Tech stack:** Svelte 5.56, TypeScript 6, Vite 8, Tauri 2.11, Rust, Dockview 6.6, bits-ui
2.18, Monaco plus the installed VS Code-compatible services, monaco-languageclient 10.7,
vscode-ws-jsonrpc 3.5, pinned VS Code declarative language contributions 25.1.2,
typescript-language-server 5.3.0, svelte-language-server 0.18.3, xterm 6, authenticated gh
CLI, and focused Node/Rust tests.

**External API checks:** Browser profile constraints use the official
[Tauri Webview API](https://v2.tauri.app/reference/javascript/api/namespacewebview/);
permission/notification behavior uses the official
[Tauri Notifications guide](https://v2.tauri.app/plugin/notification/); and external HTTP/S
opening uses the official [Tauri Opener guide](https://v2.tauri.app/plugin/opener/).

**Plan authority:** The live MacCommandBar Notion inventory, TSK-808 and its 32-image evidence
gallery, the 19-task TSK-758-through-TSK-784 rapid-fire seed, the newer TSK-809/810 conversation
tasks, every older task that remains open, the merged TSK-799 contract, current `origin/main`, and
this file. The all-open-task audit on 2026-08-03 found 37 open MacCommandBar records: 25 marked
Rapid-fire and 12 marked Idea. The reviewed product-source baseline is `main` at
`beebda6c4dad60cd2782374a36f24737cefda605`, equal to `origin/main`; PR #14 merged the prior
native-workbench checkpoint. The current working tree contains only the three uncommitted planning
document edits named in `docs/superpowers/plans/README.md`; no product implementation or task-owned
worktree was started by this planning pass. The scan-friendly 37-task disposition and routing dashboard is
`docs/superpowers/plans/2026-08-03-tsk-808-open-task-audit.html`; the Markdown plan remains the
execution authority. Re-run the anchor command in Work Package 0 immediately before implementation;
symbols and behavior are authoritative when a line number moves.

**2026-08-03 extension/diff/language checkpoint:** The user authorized the bounded
extension-compatibility experiment, and PR #14 has since merged its checkpoint into `main` at
`167a30a`. That merge is current repository evidence, not permission to declare TSK-808 complete.
The checkpoint implemented the official Houston theme as a
declarative contribution, a singleton curated extension registry, a read-only VS Code SCM
projection over the existing Rust Git state, bounded original/modified Git text models, and a
native Monaco DiffEditor. The same singleton now also registers pinned VS Code declarative
language packs for the repository's common languages plus the official Svelte grammar,
configuration, and snippets. The existing workspace LSP registry was retained for semantic
navigation; pinned TypeScript and Svelte servers now resolve from the application dependencies,
including the corrected official Svelte executable name, `svelteserver`. It also produced
`tauri-svelte-preview/output/extension-integration-review.html` and
`tauri-svelte-preview/output/extension-integration-results.html`. Focused TypeScript/Rust checks,
the production bundle, and browser-preview interaction passed; native Tauri visual acceptance,
the complete extension API matrix, SCM mutations, VSIX lifecycle/security, and TSK-808 task
closure remain open.

**Path convention:** Repository-relative paths are used throughout. Within a work package,
unprefixed `shell/...`, `components/...`, `stacks/...`, `problems/...`, and `explorer/...` paths
mean `tauri-svelte-preview/src/lib/shell/...`; unprefixed `scripts/...` means
`tauri-svelte-preview/scripts/...`. Rust paths are always written from the repository root.
Every dispatch expands shorthand into absolute or repository-relative exact file paths.

---

## 1. Decision summary

1. The first bounded extension/API checkpoint produced useful prior evidence for Houston,
   DiffEditor, declarative language contributions, and a read-only SCM projection. The user has
   selected `Works now` and `Bounded adapter` for this wave. After authorization, packet 18.2B and
   R0 classify the real-Tauri evidence, release only those two rows, and continue without another
   user-selection pause. Existing declarative assets remain preserved, but no new `Declarative
   only`, `Elevated host required`, or `Rejected` candidate enters this wave. Contrast is the first
   broad product UI milestone, not polish. No other UI lane may invent local text,
   border, status, focus, or action-button colors before the contrast tokens and shared primitives
   land.
2. PR #14 merged the protected TSK-799/native-workbench checkpoint. The product-source base is
   `main` at `beebda6`, equal to `origin/main`; the working tree is dirty only with the three current
   planning documents. Future TSK-808 implementation starts in a newly refreshed descendant of
   that commit. This audit does not create the implementation branch or worktree; the user has
   explicitly paused implementation.
3. Already-shipped rapid-fire items are verified and closed, not rebuilt. The Notion ledger is
   stale for several tasks; Work Package 0 determines the exact disposition using tests and a
   real Tauri pass.
4. Wave 1 freezes shared contracts. In Wave 2, Luna Max implements straightforward, scope-locked
   work and may also perform discovery, audits, and preparation. SOL-medium implements work with
   substantial unresolved choices or design judgment and performs milestone code reviews. Only
   the controller edits shared seams after agents return.
5. Browser, hosted GitHub, resource control, filesystem mutation, and integrated-agent actions
   all use capability-gated native boundaries. Remote mutations and destructive local actions
   require explicit preview and confirmation.
6. Compatible extensions are a product capability, not a rejected architecture. Reuse the
   installed local web-worker extension host for capability-reviewed browser extensions and
   declarative contributions; defer unsupported Node/remote host classes until separately
   proven and approved.
7. The Settings screen that was previously available is now reported missing in the real app.
   Restore the existing surface and its reachability as part of the sequential shell foundation;
   do not create a third Settings component, a second settings store, or a replacement settings
   schema. Source presence, a successful dynamic import in a unit test, or browser-preview proof
   does not close this regression; it must be opened and exercised in a rebuilt Tauri app.
8. A 2026-08-03 native screenshot shows a CodeLens Peek state where the editor content and side
   panes remain visible, but the expected center editor tab strip/top tab controls are not visible.
   Treat this as a Peek layout regression until native reproduction proves the narrower root cause:
   Peek may resize or split the editor content, but it must not replace, hide, clip, or cover the
   workbench tab strip, active file identity, or top-bar destination controls.
9. Git graph is a native product surface, not an embedded Git Graph or GitLens view. Rust and the
   existing Git service remain the repository/history authority; a Svelte/Dockview center panel
   renders their commit/parent/ref data and opens the existing Monaco DiffEditor. A later graph
   rendering library may replace only the visual lane-layout layer after license, bundle, and
   accessibility review.
10. The public product name becomes **Assembly**. The default compact lockup is **Assembly — Build
    in parallel.** The longer descriptor is **The workspace for parallel software development.**
    The narrative variant is **Agents, worktrees, and code—working together.** Work Package 1B
    changes visible product copy, runtime self-identification, and public app artifacts, but the
    first Assembly release deliberately preserves bundle identifiers, storage keys, Application
    Support paths, Keychain services, repository/project IDs, extension IDs, and internal
    package/crate/module names. This is a compatibility migration, never a blind repository-wide
    replacement.

## 2. Non-negotiable execution contract

### 2.1 Exact native-agent routing gate

Classify each lane before dispatch:

- **Luna Max:** straightforward implementation whose files, behavior, interfaces, and acceptance
  are already decided; also discovery, audits, and bounded preparation.
- **SOL-medium:** implementation that still requires meaningful design/tradeoff decisions, plus
  every milestone code review.

Straightforward implementation dispatches use:

    spawn_agent({
      agent_type: "default",
      fork_turns: "none",
      model: "gpt-5.6-luna",
      reasoning_effort: "max",
      task_name: "tsk_808_straightforward_lane",
      message: "Implement only the fully specified, scope-locked lane. Stop on a design choice."
    })

High-judgment implementation dispatches use:

    spawn_agent({
      agent_type: "default",
      fork_turns: "none",
      model: "gpt-5.6-sol",
      reasoning_effort: "medium",
      task_name: "tsk_808_high_judgment_lane",
      message: "Implement the specified lane and resolve the named design decisions only."
    })

Milestone code-review dispatches also use explicit `gpt-5.6-sol` plus `medium`, with a read-only
review prompt and no implementation authority. The `default` role is intentional where installed
specialist roles have fixed model contracts that cannot prove the required model/effort pairing.
Every prompt must name exact owned files, forbidden shared files, acceptance commands, and stop
conditions. It must also contain this data rule verbatim:

    All SQL aggregation, grouping, filtering, joins, sorting, and paging run server-side
    (DB-side) as an ORM/EF-translated query that becomes ONE SQL statement, or a database view.
    NEVER do in-memory grouping/aggregation: do not materialize rows and then GroupBy, Sum,
    Count, or Where in code; do not load-then-loop, use N+1 or per-row follow-up queries, or use
    lazy loading. A roughly 20-row list page must use 1-3 DB queries total, not dozens. Confirm
    that the query translates to SQL by inspecting the generated statement; if it would run in
    memory, rewrite it DB-side or add a view plus covering index. When one instance is fixed,
    sweep the codebase for siblings.

Most lanes do not use SQL. They must still state “No database is touched in this lane” rather
than silently omitting the rule.

The same prompt must say:

    You are not alone in the codebase. Edit only the owned files. Do not revert or overwrite
    another lane's work. Do not start a heavy build or test until the controller assigns one of
    the at-most-two heavy-runner slots; the default schedule uses one. Use focused tests only;
    the controller owns the serialized integration build. Set
    MSBUILDDISABLENODEREUSE=1 for dotnet commands, RUST_TEST_THREADS=1 for Rust tests, run
    dotnet build-server shutdown after a heavy batch, and clean task-owned build artifacts before
    returning. The controller owns the shared integration
    worktree; report Worktree cleanup: not applicable and do not remove it.

If the routed model or effort selection is unavailable, rejected, or silently downgraded, that
implementation or milestone-review lane does not start. Planning and read-only discovery may
continue. Luna Fast is preferred but never blocking: request it when the spawn API exposes the
tier; otherwise continue with explicit Luna Max/max/no-fork and do not claim Fast.

Read-only Luna audit dispatches use:

    spawn_agent({
      agent_type: "default",
      fork_turns: "none",
      model: "gpt-5.6-luna",
      reasoning_effort: "max",
      task_name: "tsk_808_read_only_audit",
      message: "Inspect only. Do not edit, build, test, browse, create a worktree, or mutate Notion."
    })

### 2.1A Luna-executable task-packet contract

The default implementation route is Luna Max. A task is not ready for Luna because it is small;
it is ready when the controller has removed every material product/architecture choice. Before a
Luna dispatch, the corresponding task packet in this plan must contain all of the following:

1. **Outcome and non-goals:** one observable result, the exact behavior preserved, and adjacent
   work that is forbidden.
2. **Repository baseline:** current SHA plus the existing implementation to extend. Never tell an
   agent to “add” a component, service, store, or command until the packet states whether one
   already exists.
3. **Ownership:** exact editable files, exact controller-owned/shared files that may not be edited,
   and the expected returned diff. One packet cannot quietly expand into a neighboring lane.
4. **Symbol contract:** exact existing classes/functions/components to change; exact new type,
   field, function, event, command, and component names; parameter and return shapes; persistence
   key/version; and the owner of every side effect.
5. **Ordered edits:** a numbered implementation sequence detailed enough that no step asks Luna to
   “design,” “decide,” “choose a library,” “figure out the UX,” or “wire as appropriate.”
6. **States and failure behavior:** loading, empty, success, stale-response, unsupported, denied,
   retry, cancellation, cleanup, and destructive-confirmation behavior where relevant.
7. **Data and security invariants:** input bounds, canonicalization, capability checks, remote-text
   handling, no duplicate runtime/service/store, and the global DB-side SQL rule even when the
   packet says no database is touched.
8. **Focused verification:** exact test files to add/change, named cases, exact commands, expected
   receipts, and whether the controller must grant the sole heavy-runner slot.
9. **Real acceptance:** exact rebuilt-Tauri interaction, fixture/setup, screenshot/recording or
   timing/process evidence, cleanup receipt, and the defect that fails the lane even if compilation
   passes.
10. **Stop and return contract:** stop before editing on a missing symbol, overlapping WIP, rejected
    model override, new architecture choice, or unplanned shared seam. Return changed files, test
    output, remaining risks, and cleanup status.

If any item is unresolved, the controller first performs discovery or assigns a narrowly bounded
SOL-medium decision packet. The controller then writes the chosen answer back into this plan and
dispatches the resulting mechanical implementation to Luna Max. SOL-medium is not selected merely
because a packet contains many files or many lines of code. Every milestone still receives a
read-only SOL-medium code review after the Luna implementation packets are integrated.

### 2.2 Worktree and dirty-WIP contract

Current authoritative checkout:

- Path: `/Users/blackcolours/dev/work/mac-command-bar`
- Branch: `main`
- HEAD: `beebda6c4dad60cd2782374a36f24737cefda605`
- Remote: `origin/main` at the same SHA.
- State at audit start: clean; no TSK-808 implementation worktree exists.
- PR #14 contains the former `5884139` native-workbench checkpoint and the follow-up contract fix.

Do not reset, rewrite, squash, or delete the merged checkpoint history merely to obtain a smaller
diff. When the user authorizes implementation, refresh `origin/main`, verify it still contains
PR #14's `167a30a`, and create the integration worktree from the refreshed SHA:

    git fetch origin
    git worktree add \
      /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-native-workbench \
      -b tsk-808-native-workbench-product-wave origin/main

Use one controller-owned integration worktree to save disk. File-independent agents may share
that worktree only under the ownership table in section 5. Before every dispatch, the controller
records the starting SHA and path-scoped status; after return it attributes only the exact owned
path diff. Agents do not run Git commands and never remove the shared worktree. The controller
commits owned paths per work package. Remove the worktree immediately after its branch is
pushed to a PR, merged, or abandoned:

    git worktree remove /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-native-workbench
    git worktree prune

Never remove it while dirty or unmerged. In that case report its path, branch, git
status --short, and the named owner of the next action.

### 2.3 Build, browser, and native-process limits

- At most two heavy build/test runners may be active; this plan defaults to one and the
  controller alone assigns a second non-overlapping slot.
- Node focused tests may run lane-by-lane. pnpm build, cargo build, full cargo test, and real
  Tauri proof are controller-owned and serialized.
- Every browser verifier uses a unique name such as tsk-808-browser-proof and closes the
  daemon, Chrome/Chromium helpers, exact external tab, fixture server, and task-owned Tauri
  process before returning.
- A Tauri browser child webview is not a Playwright process. The verifier must also prove that
  all child webviews and task-owned native processes die on app exit.
- Browser-preview proof cannot close a native-desktop acceptance item.

### 2.4 Existing architecture rules

- No backend IO at module import.
- No backend IO from a Svelte $effect.
- Activation is imperative and visibility-gated through panelActivation.ts and shellPanels.ts.
- Stores hold state; services own requests, request guards, retries, and backend calls.
- New Tauri commands are capability-gated because older builds silently drop unknown payload
  keys.
- No window.confirm or window.alert. Use the existing AlertDialog and Dialog components.
- User-visible body text is at least 13px; metadata is at least 12px.
- All user-visible text is plain English.
- Remote text, GitHub comments, page content, and repository content are untrusted data.

---

## 3. Source-of-truth task coverage

### 3.1 Latest-round boundary and complete open-task disposition

The consolidation boundary is exact:

- **Latest rapid-fire wave — implement from this plan:** the 19-task seed TSK-758 through
  TSK-784, the TSK-808 orchestration epic, and the newer TSK-809/810 conversation tasks. These
  are 22 open records even though four of the 19 seed records are currently labeled `Idea` in
  Notion.
- **Current dependencies — do not silently absorb:** TSK-789 and TSK-802. TSK-789 supplies the
  CodeLens/Roslyn performance contract and is verified before closure; TSK-802 remains a separate
  low-priority LSP protocol-hardening packet.
- **Older open records — audit, link, and dispose explicitly:** TSK-127, 192, 253, 280, 283, 307,
  312, 315, 324, 344, 360, 369, and 378. Their overlapping product behavior is implemented or
  verified in the named package below, but the records are never closed merely because TSK-808
  exists.

This yields the live 2026-08-03 inventory of **37 open tasks: 25 Rapid-fire and 12 Idea**. Work
Package 0 re-queries the database server-side immediately before execution and updates this ledger
if the live status changed. It never materializes the task database and then filters, groups, or
sorts it in application code.

#### 3.1A The 19-task seed

| Task                                        | Notion state on 2026-08-01 | Repository evidence                                                                                                                                                                                                                 | Planned disposition                                                                |
| ------------------------------------------- | -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| TSK-758 Minimize panels to an edge strip    | To Do, Low                 | Sessions already collapse in SessionsColumn.svelte:540-577 and /next/+page.svelte:198-219; other regions lack one shared minimize model ![1785617845971](image/2026-08-01-tsk-808-native-workbench-product-wave/1785617845971.png) | Finish in Work Package 2                                                           |
| TSK-759 Rebuild /next frame                 | To Do, High                | ShellFrame.svelte:21-155, frame.ts, and centerDock.ts:26-323 already implement the Codex-shaped shell                                                                                                                               | Verify in Work Package 0; close if native proof passes                             |
| TSK-760 Project picker and new-session flow | To Do, High                | newSessionFlow.ts, ownedSessions.ts, and /next/+page.svelte:477-563 already create/adopt/persist sessions                                                                                                                           | Verify in Work Package 0; close if proof passes                                    |
| TSK-761 Working, Done, Settled lifecycle    | Doing, Medium              | sessionCardModel.ts:42 onward distinguishes running/done/stopped; rail store has complete/reopen; richer state/notifications remain                                                                                                 | Finish in Work Package 3                                                           |
| TSK-762 Rail correctness                    | To Do, High                | SessionsColumn, openFileBus, project selection, and sessionWorkspaces are present                                                                                                                                                   | Verify in Work Package 0; defects roll into Work Package 3                         |
| TSK-763 Worktree manager                    | To Do, High                | WorktreeManagerPane, WorktreeRow, RemoveWorktreeDialog, service, and safety tests are present                                                                                                                                       | Preserve; finish shared hit targets and Git context in Work Package 6              |
| TSK-764 Playwright process card             | To Do, Medium              | PlaywrightCard, backend, service, store, per-session stop, and cleanup tests are present                                                                                                                                            | Verify and close in Work Package 0                                                 |
| TSK-765 Theme system and imports            | To Do, Medium              | Houston/Dracula registry exists; the protected 2026-08-03 checkpoint now registers the official Houston contribution once and removes the late legacy color override, but native proof, contrast migration, and generic VSIX import remain open | Reconcile Houston checkpoint; contrast in Work Package 1; optional safe theme/grammar import in Work Package 12 |
| TSK-766 CMUX orchestration research         | To Do, Medium              | core session scanner knows CMUX; append-only orchestration model exists                                                                                                                                                             | Close research decision in Work Package 0; product actions live in Work Package 11 |
| TSK-767 Stack runner                        | To Do, High                | stackStore and stackService already implement saved run configurations and terminal handlers                                                                                                                                        | Verify wiring in Work Package 4; finish only evidenced gaps                        |
| TSK-768 Format document                     | To Do, Medium              | format_source_with_lsp exists at main.rs:1009-1017; editor command wiring is absent/incomplete                                                                                                                                      | Finish in Work Package 4                                                           |
| TSK-769 DB browser                          | To Do, Low                 | No implementation; right-side tool roster can host it                                                                                                                                                                               | Keep gated until Work Package 13; no hidden SQL shortcut                           |
| TSK-770 Problems panel                      | To Do, Medium              | LSP diagnostics exist in problemsService.ts:63-179; build-output ingestion is absent                                                                                                                                                | Finish in Work Package 4                                                           |
| TSK-771 Prefilter before 512-file budget    | To Do, Medium              | core sessions scanner has bounded prefilter at sessions.rs:900-982                                                                                                                                                                  | Verify and close in Work Package 0                                                 |
| TSK-773 Svelte type-check blind spot        | To Do, Medium              | package.json contains svelte-check and check:svelte; focused gate exists                                                                                                                                                            | Verify and close in Work Package 0                                                 |
| TSK-779 Top-bar quick open                  | To Do, Medium              | ShellFrame exposes showCenterPanel at lines 116-125; top bar does not surface the three primary destinations                                                                                                                        | Finish in Work Package 2                                                           |
| TSK-780 Right pane tabbed surface           | To Do, Medium              | ShellSidebar/sidebarViews and Dockview tools exist; roster needs extension-safe contract                                                                                                                                            | Verify base and freeze roster contract in Work Package 2                           |
| TSK-783 Editor paints text instantly        | To Do, High                | EditorPanel waits for Monaco and displays “Starting the code editor” at lines 659-683                                                                                                                                             | Finish in Work Package 4                                                           |
| TSK-784 Language-server status              | To Do, Medium              | Merged PR #13 contains LanguageServerStatusChip at EditorPanel lines 637-642                                                                                                                                                         | Verify the merged path, then close or finish in Work Package 4                     |

#### 3.1B Complete ledger for the remaining 18 open records

| Task and live state | Relationship to this plan | Exact disposition and Luna packet |
| --- | --- | --- |
| TSK-810, To Do, High, Rapid-fire — child-agent hierarchy and transcripts | Latest wave | Implement Work Package 10A packet 10A.4. Reuse the existing authoritative transcript mirror and owned-session identity; add recursive parent-scoped read-only children, elapsed/state/relationship fields, Claude and Codex fixtures, stale-generation guards, and native proof. |
| TSK-809, To Do, High, Rapid-fire — conversation composer | Latest wave | Implement Work Package 10A packets 10A.1–10A.3. Remove both composer boxes, add real per-session controls/attachments/commands/context telemetry, and prove one authoritative PTY without a duplicate provider host. |
| TSK-808, To Do, High, Rapid-fire — native-workbench wave | Latest wave and execution authority | Execute Work Packages 0–14 in milestone order. Keep To Do until every non-deferred acceptance row has proof and linked tasks have been disposed individually. |
| TSK-802, To Do, Low, Idea — per-method server-initiated LSP replies | Separate dependency | Keep separately open and deferred by default. If promoted, execute packet 15.9: replace the blanket `null` branch in `src-tauri/src/lsp.rs` with allow-listed per-method replies, retain unknown-method `MethodNotFound`, add Rust fixtures, and run native initialize/configuration proof. |
| TSK-789, Doing, High, Idea — CodeLens cancellation and re-arm performance | Dependency with possible shipped overlap | Work Package 0 captures baseline semantic/placeholder counts and click timings, then packet 10.1A/15.8 proves cancellation, stale-response discard, and no per-keystroke re-arm. Close only when baseline/final evidence and native two-second Peek acceptance pass. |
| TSK-378, Doing, Medium, Idea — reusable Menu/ContextMenu/Tooltip | Older shared-primitives overlap, likely implemented | Work Package 0 verifies the existing `components/Menu.svelte`, `ContextMenu.svelte`, and `Tooltip.svelte` with `check:svelte` plus rebuilt-native pointer, keyboard, Escape/focus-return, VoiceOver, portal/z-order, touch/trackpad, and contrast proof. Work Packages 1–2 add only genuinely missing compact row/card, detail popover, status, and icon-button primitives; do not fork a second overlay family. Restore the missing `[TSK-378]` title prefix before closure. |
| TSK-369, To Do, Medium, Idea — Dockview terminal render failures | Older read-only diagnosis, likely superseded by an implemented fix | Work Package 0 records the actual diagnosis—destroyed/hidden xterm hosts, stale fit/geometry, and Dockview teleport/visibility—and verifies `TerminalSurface.svelte`, `terminalService.ts`, `ShellFrame.svelte`, and `centerDock.ts` through hide/show, switch, resize, reattach, reload, and layout restore. Close with the diagnosis plus continuous native receipt; do not turn this research task into a new terminal architecture. |
| TSK-360, To Do, High, Idea — editor open files as Dockview tabs | Older genuine editor/shell gap | Work Package 0 confirms that `/next` still uses the custom `.file-strip`; packet 8.2A then migrates each open file to a real nested Dockview panel while preserving one Monaco model per URI, dirty/close/active/recent/restore state, drag/split/stack behavior, and the outer center destination. Packet 10.1A proves Peek never obscures either Dockview layer. |
| TSK-344, Doing, High, Rapid-fire — VS Code-style Git source-control panel | Older Git overlap | Implement residual presentation/actions in Work Package 5 over the one Rust Git authority. Reuse the merged SCM projection and native DiffEditor; add staged/unstaged/untracked groups, selection, inline actions, keyboard/context actions, and disposable-repository proof. |
| TSK-324, Doing, High, Rapid-fire — stack and resize context panels | Older shell overlap | Implement Work Package 2 shared right-dock roster and per-section resize persistence. Prove stacked sections, min/max bounds, keyboard access, restored ratios, and no terminal/editor re-creation. |
| TSK-315, To Do, High, Rapid-fire — resumable-session metadata | Older partial session-rail dependency | Packet 9.2A enriches the existing `OwnedSession`/`sessionCardModel` path—not `ConversationSessionSnapshot`—with model, exact repo/worktree/cwd, dirty/saved-workspace readiness, provider, branch, PR/task, and resumable/stale reason. Use deterministic scanner/workspace facts; native proof must identify the exact session without guessing. |
| TSK-312, To Do, Medium, Rapid-fire — worktree cleanup runbook helper | Older worktree overlap, likely implemented | Work Package 0 verifies `worktreeCleanupPlan.ts` and `worktreeCleanupRunbook.ts` with safe/backup/active/protected/saved/review fixtures and native copy-only output. Work Package 6 reuses them and adds nothing unless proof finds a named gap; the helper never executes cleanup. |
| TSK-307, To Do, Medium, Idea — reusable Git graph view-model helper | Older Git overlap, likely implemented | Work Package 0 verifies the existing `gitGraphViewModel.ts` and fixtures, including refs, task IDs, parents, merges, and root rows, then closes only with native graph use. Work Package 5 reuses that helper; it does not rebuild or move repository IO into TypeScript. |
| TSK-283, To Do, High, Rapid-fire — native activity-card data regression | Older legacy/native-backend verification lane | Work Package 0 proves `list_agent_sessions`, `list_runtime_contexts`, and `list_project_worktrees` return nonempty native facts and that the legacy activity cards show explicit data/empty/error states. `/next` does not recreate the legacy card UI; any scanner/bridge fix stays backend-only and preserves the route-owner boundary. |
| TSK-280, Doing, Medium, Rapid-fire — terminal and browser session integration | Older partial dependency | Work Package 0 separately certifies existing PTY lifecycle, workspace URL normalization, and Playwright process-group safety. Work Package 8 owns only the missing child-webview/profile/annotation half and owner-safe conversation-draft staging; it does not absorb legacy-route ownership or replace the existing terminal. |
| TSK-253, To Do, Medium, Rapid-fire — native LSP language matrix | Separate partial/deferred LSP task | Keep open outside TSK-808 unless explicitly promoted. Current support covers C#, TypeScript/JavaScript, Rust, and Svelte/HTML paths but not the requested Python/Go server specs. A future packet must edit `MonacoSourceEditor.svelte`, `src-tauri/src/lsp.rs`, `scripts/sourceUi.test.mjs`, and Rust LSP fixtures, with clean skip when `pyright`/`gopls` is absent and real native proof per claimed language. |
| TSK-192, To Do, Medium, Idea — conversation workspace restore snapshots | Older broad workspace-restore overlap, likely implemented | Work Package 0 verifies `sessionWorkspaces.ts` and `/next/+page.svelte` restore provider/session, project/worktree/cwd, open and selected files, layout/dock, terminal, browser, and conversation without blocking startup. Missing/moved worktrees must produce a safe recovery action. Work Package 10A adds only the newer composer/child state fields. |
| TSK-127, Doing, Medium, Idea — native macOS menu-bar diff app | Separate Swift/AppKit umbrella/parent | Explicitly retain outside TSK-808. TSK-808 is the Tauri/Svelte native workbench and may link shared Rust facts, but it does not claim the SwiftUI/AppKit menu-bar shell, signing, clipboard/keychain, launcher, or native-helper acceptance. TSK-127 has its own owner and closes only under its full Swift/Rust/signed-app contract. |

Every `verify then close` row is a real task packet: read the full current Notion body, run the
named focused checks, perform the named rebuilt-Tauri interaction, attach the evidence receipt,
then close only that task. A green compile, an older screenshot, or overlap with TSK-808 is not
closure evidence.

### 3.1C Immediate /next regressions (post-WIP parity)

| Task                         | Notion state | Repository evidence                                                                                                             | Disposition                                                                                       |
| ---------------------------  | ------------  | ---------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Settings screen reachability  | In TSK-808 scope | ShellOverlays mounts `SettingsHost` and exposes `openSettings`; check that `onOpenSettings` in `ActivityBar` opens `SettingsHost` -> `SettingsDialog` in native build. [ShellOverlays](/Users/blackcolours/dev/work/mac-command-bar/tauri-svelte-preview/src/lib/shell/components/ShellOverlays.svelte:1), [SettingsHost](/Users/blackcolours/dev/work/mac-command-bar/tauri-svelte-preview/src/lib/shell/components/SettingsHost.svelte:1), [SettingsDialog](/Users/blackcolours/dev/work/mac-command-bar/tauri-svelte-preview/src/lib/shell/components/SettingsDialog.svelte:11), [ActivityBar](/Users/blackcolours/dev/work/mac-command-bar/tauri-svelte-preview/src/lib/shell/components/ActivityBar.svelte:1), [/next activity/settings wiring](/Users/blackcolours/dev/work/mac-command-bar/tauri-svelte-preview/src/routes/next/+page.svelte:996), [/next overlay mount](/Users/blackcolours/dev/work/mac-command-bar/tauri-svelte-preview/src/routes/next/+page.svelte:1098) | Restore by proving gear-opened settings open in a rebuilt Tauri run; capture a 2-second screencapture before closing the lane. |
| CodeLens Peek hides editor tabs | In TSK-808 scope | Native path uses `roslyn.client.peekReferences` and command `editor.action.showReferences` in `csharpLanguageClient.ts`; browser fallback uses `editor.action.peekLocations` in `MonacoSourceEditor.svelte`; `/next` owns separate center-destination and open-file tab strips. [csharpLanguageClient](/Users/blackcolours/dev/work/mac-command-bar/tauri-svelte-preview/src/lib/shell/editor/csharpLanguageClient.ts:319), [Monaco fallback](/Users/blackcolours/dev/work/mac-command-bar/tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:2158), [file tabs](/Users/blackcolours/dev/work/mac-command-bar/tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:626), [center tabs](/Users/blackcolours/dev/work/mac-command-bar/tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte:198) | Reproduce and close in rebuilt Tauri: populated Peek must leave both tab layers and the future top-bar destination switch visible and usable. Prove before/after plus a short recording. Browser parity is useful, but browser-preview proof cannot close this native regression. |

### 3.2 Dependencies and stale states

- TSK-785, TSK-798, and TSK-799 are Done and are dependencies, not reopened work.
- TSK-809 and TSK-810 remain individually tracked conversation-workbench tasks, but Work Package
  10A in this master plan is their single implementation authority. It extends the merged
  conversation runtime/store, composer, child hierarchy, and transcript inspection rather than
  creating a parallel surface. TSK-808 owns the cross-workbench context/action integration and
  must consume that one certified conversation authority.
- TSK-789 remains Doing. TSK-799 shipped part of its queue/CodeLens lifecycle, but that does not
  prove cancellation, stale-response discard, or prevention of per-keystroke re-arm. Work Package
  0 captures a baseline and packet 10.1A plus 15.8 close only the evidenced remainder.
- TSK-307, TSK-312, TSK-315, TSK-344, and other older task records overlap individual
  components. TSK-808 must not silently close them. Work Package 0 links evidence and asks
  capture-task to update only when durable tracking truth changes.
- TSK-802 is a separate low-priority LSP hardening item and is not absorbed.

The Notion task and its 32 attachments are the evidence-gallery authority. Only two image files
are currently mirrored under this plan's local `image/2026-08-01-tsk-808-native-workbench-product-wave`
folder; local absence is not evidence that the remaining Notion attachments do not exist. At
execution time, record the Notion attachment reference used for each acceptance row without
copying or renumbering the gallery.

### 3.3 TSK-808 capability lanes

The epic adds eight product capability lanes beyond the 19-task seed and one sequential
extension/API decision checkpoint:

1. Sessions, navigation, status, and notifications.
2. Local Git, graph, diffs, files, and worktrees.
3. Hosted pull requests, reviews, checks, conflicts, and GitHub budgets.
4. Workspace-isolated browser, annotations, and native webview lifecycle.
5. Resources, provider usage, ports, disk visibility, and Roslyn ownership.
6. Rich Markdown reading/editing.
7. One transcript-backed conversation composer and read-only child-agent hierarchy.
8. Guarded integrated-agent actions and audit receipts.

The extension/API checkpoint runs first after Work Package 0 and produces the HTML comparison.
R0 applies the recorded `Works now` plus `Bounded adapter` policy and immediately releases the
next milestone. It is not counted as a product lane because it classifies which later extension
work is authorized and which rows are skipped.

### 3.3A Original TSK-808 Lane A–G crosswalk

The task body’s original capability lanes remain first-class scope. The work-package numbering
does not replace or hide them:

| Original TSK-808 lane | Master-plan implementation authority | Required product outcome |
| --- | --- | --- |
| Lane A — sessions, workspace navigation, history, status, and notifications | Work Packages 1–3 | Compact legible session/workspace navigation; deterministic metadata; reversible Working/Done/Settled state; explicit actions; accessible native notifications. |
| Lane B — source control, Git graph, diffs, files, and worktrees | Work Packages 5–6 | Full native Git graph/diff/files workspace over the existing Rust Git authority, plus canonical filesystem and worktree safety. |
| Lane C — pull requests, automated review, and GitHub budget | Work Package 7 | Cross-project PR queue/detail/checks/files/review workflows; create/edit/close/merge preparation; agent review recipes; REST/Search/GraphQL budget visibility and backoff. |
| Lane D — embedded browser and visual feedback | Work Package 8 | Workspace-isolated native child webviews, tabs/navigation/auth/profile lifecycle, app-global bottom control/full-shell fan, element annotations, and owner-safe draft staging. |
| Lane E — resource manager, provider usage, and Roslyn lifecycle | Work Package 9 | CPU/RSS/process/port/disk ownership, truthful Claude/Codex usage, one Roslyn owner per canonical root, warm/recent policies, cleanup/restart, and baseline/final metrics. |
| Lane F — rich Markdown display and editing | Work Package 10 | Safe high-quality GFM/Mermaid/local-image rendering, Read/Edit/Split, selection/copy/find, outline, scroll sync, watch/conflict handling, and large-document proof. |
| Lane G — integrated agent foundation and contextual actions | Work Package 10A plus Work Package 11 | One transcript-backed conversation authority; deterministic workbench context; contextual Ask Agent; guarded Git/PR/worktree/run-config proposals; confirmation/revalidation; append-only audit receipts. |

Lane C, Lane E, and Lane G are not optional cleanup items. Their detailed files, types, methods,
tests, security boundaries, native acceptance, and Luna/SOL routing are specified in sections 13,
15, 16A, and 17 respectively. They remain open until their own native proof rows pass.

---

## 4. Product and architecture decisions

### 4.1 Contrast and accessibility contract

Use computed contrast, not visual opinion:

- Normal text under 18pt: at least 4.5:1 against every surface on which it appears.
- Large text: at least 3:1.
- Focus rings, status dots, selected-row boundaries, dividers that are the sole boundary, and
  other meaningful non-text indicators: at least 3:1.
- Dense desktop icon actions: at least 32 by 32 CSS pixels; 36 by 36 when the row permits.
- Running and done use different hue families and always include a text label.
- Attention, approval, blocked, failed, idle, and done do not share one green.
- Every icon-only action has an accessible name and a tooltip available on hover and keyboard
  focus.
- Every interactive row has an explicit focus-visible style.
- Reduced Motion disables pulses and nonessential transitions.
- All information-bearing text uses --color-text or --color-text-2. --color-text-3 is reserved
  for disabled controls and decorative hints that repeat visible information.

These are reusable semantic tokens for every workbench surface, including portals, overlays,
tooltips, native-view placeholders, Dockview chrome, Markdown, Git, sessions, Problems, settings,
and extension UI. Components consume the semantic token instead of inventing a local shade.
The implementation must enumerate and migrate current information-bearing `--color-text-3`
uses before a broad literal-color scan is enabled.

Starting palette candidates:

| Token             |                                          Houston candidate |                   Dracula candidate | Meaning                       |
| ----------------- | ---------------------------------------------------------: | ----------------------------------: | ----------------------------- |
| --color-text      |                                                    #eef0f9 |                             #f8f8f2 | Primary text                  |
| --color-text-2    |                       Compute to pass 4.5:1; start #a7a7b5 | Compute to pass 4.5:1; start #b8b8c4 | Secondary information         |
| --color-text-3    | Compute to pass 4.5:1 when text; otherwise decorative-only |                           Same rule | No low-contrast required copy |
| --color-live      |                                                    #54b9ff |                             #8be9fd | Running/in turn               |
| --color-good      |                                                    #8bdc9b |                             #50fa7b | Completed/passing             |
| --color-attention |                                                    #ffd493 |                             #f1fa8c | Needs attention/approval      |
| --color-bad       |                                                    #ff6d91 |                             #ff5555 | Failed/blocked                |

The implementation test, not these candidate hex values, decides final colors. Ratios are
computed against every actual surface state, including hover, selected, disabled, focus,
overlay, and portal backgrounds. Hue-family assertions separately prove that running, done,
attention, and failed remain distinguishable without relying on text color alone.

### 4.2 Center and tool surfaces

Refactor ShellFrame.svelte:21-49 from a closed center object into a typed roster:

    export type CenterPanelId =
      | "session"
      | "editor"
      | "browser"
      | "diff"
      | "gitGraph"
      | "pullRequests"
      | "markdown";

    export interface CenterPanelRegistration {
      id: CenterPanelId;
      title: string;
      content: Snippet;
      group: "conversation" | "display";
      permanent: boolean;
    }

    interface ShellFrameProps {
      sessions: Snippet;
      centerPanels: CenterPanelRegistration[];
      tools: Snippet;
      activity: Snippet;
      dock: Snippet;
      onCenterPanelShown?(id: CenterPanelId): void;
      onReady?(controls: ShellFrameControls): void;
    }

Keep centerDock.ts generic. Add activePanelId() and setPanelVisible(id, visible) to CenterDock;
do not add browser-specific behavior to Dockview. Bump CENTER_LAYOUT_KEY once when the roster
changes, not once per lane.

### 4.3 State and IO boundaries

Every new capability uses this flow:

    Svelte component
        -> state-only rune store
        -> imperative TypeScript service with request generation/guards
        -> typed backend adapter
        -> capability-gated Tauri command
        -> validated Rust module

Event flows return through one typed listener adapter and carry stable identity plus a request
or generation number. Stale root, workspace, tab, file, PR, or session events are discarded.

### 4.4 Deterministic facts and agent summaries

The integrated agent never becomes the source of truth for:

- Git status, branch, dirty state, ahead/behind, or worktree safety.
- Pull-request state, checks, review threads, mergeability, or rate budgets.
- Process ownership, CPU/RSS, ports, or LSP process identity.
- Session state, terminal exit code, run-configuration state, or notification eligibility.

Those facts come from deterministic services. The LLM may summarize them into short prose and
prepare a proposed action. The proposal is appended to the orchestration log; the native
boundary revalidates facts before any mutation.

### 4.5 Explicitly rejected or deferred architectures

- No Code OSS or Theia embedding.
- Do not reject extensions. Reuse the installed `vscode/localExtensionHost` and emitted
  `extensionHost.worker` for capability-reviewed, browser-compatible extensions and declarative
  theme/grammar/icon contributions.
- The exact Houston theme JSON is the currently proven declarative contribution from PR #14,
  pending the user's new API/extension comparison. Preserve it while testing alternatives.
  Register it once before
  `MonacoVscodeApiWrapper.start`, exclude Houston's activation/webview extras, and let the theme
  own workbench, TextMate, and semantic-token colors. Do not layer the old hand-authored Monaco
  approximation over it; that caused the visible Houston-then-old-theme repaint.
- The prior checkpoint found that the Git Graph extension has no compatible browser entry for
  the present LocalWebWorker host, assumes a desktop/workspace Node and Git environment, and is
  not the repository authority this product needs. Implement the graph described in Work Package
  5 over the existing Rust Git model. Treat its product behavior as inspiration, not reusable
  shipped extension code.
- The prior checkpoint did not make GitLens part of the current product baseline. Its browser
  entry proves only that activation in a web extension host is possible; its valuable views still
  depend on a larger VS Code workbench/view/storage/repository-provider/auth surface than the
  current editor-service host exposes. Reconsider it only through a separately approved
  full-workbench compatibility spike with measured startup, memory, bundle, and adapter cost.
- A Node/local-process host, remote extension host, or automatic marketplace download is not
  categorically rejected; it is deferred until a separate security, resource, and distribution
  decision proves that host class. It is not silently introduced by the LocalWebWorker lane.
- A user-selected VSIX may be staged and enabled when its manifest and browser entrypoint pass
  the Work Package 12 policy. Unsupported native/process capabilities are rejected with an
  explanation rather than treating every extension package as forbidden.
- No iframe-based claim of a full browser.
- No shared cookie jar across workspaces.
- No frontend-held GitHub token.
- No shell-built gh or git command.
- No process-name-based kill.
- No repository-wide worktree prune for a single row.
- No general eval command into a browser child webview.
- No Markdown HTML injection without a sanitizer.
- No duplicate Roslyn owner or independent frontend/backend LRU policies.

---

## 5. Lane ownership and merge order

### 5.1 Controller-owned shared seams

Only the controller edits these after Wave 1:

- tauri-svelte-preview/package.json and pnpm-lock.yaml
- tauri-svelte-preview/src-tauri/Cargo.toml and Cargo.lock
- tauri-svelte-preview/src-tauri/tauri.conf.json
- tauri-svelte-preview/src-tauri/tauri.dev.next.conf.json
- tauri-svelte-preview/src-tauri/src/main.rs
- tauri-svelte-preview/src-tauri/src/product_identity.rs
- tauri-svelte-preview/src/lib/tauriSource.ts
- tauri-svelte-preview/src/lib/productIdentity.ts
- tauri-svelte-preview/src/routes/next/+page.svelte
- tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte
- tauri-svelte-preview/src/lib/shell/layout/centerDock.ts
- tauri-svelte-preview/src/lib/shell/layout/layoutStorage.ts
- tauri-svelte-preview/src/lib/shell/panelActivation.ts
- tauri-svelte-preview/src/lib/shell/shellPanels.ts
- tauri-svelte-preview/src/lib/shell/shellCommands.ts
- tauri-svelte-preview/src/lib/settingsStore.svelte.ts
- tauri-svelte-preview/src/lib/shell/components/ActivityBar.svelte
- tauri-svelte-preview/src/lib/shell/components/SettingsDialog.svelte
- tauri-svelte-preview/src/lib/shell/components/SettingsHost.svelte
- tauri-svelte-preview/src/lib/shell/components/ShellOverlays.svelte
- tauri-svelte-preview/src/lib/shell/components/PalettePanel.svelte
- tauri-svelte-preview/src-tauri/capabilities/default.json

Agents that need a shared-seam change write an integration receipt containing the exact
symbol, insertion point, imports, registration line, capability name, and test. The controller
applies all receipts in one serialized integration step.

### 5.2 File-independent implementation lanes

| Lane | Agent-owned paths after contracts freeze | Heavy work |
| --- | --- | --- |
| A Contrast/primitives | `styles/nextTokens.css`, `styles/themeChrome.css`, `themes/themeRegistry.ts`, `components/shared`, token tests | Node only |
| A2 Assembly identity | `productIdentity.ts`, `product_identity.rs`, product-identity tests, public labels in explicitly leased routes/configs/native modules and legacy Swift shell | Node plus focused Rust/Swift only if active |
| B Sessions/notifications | `components/SessionsColumn.svelte`, `components/sessions`, `stores/sessionRailStore`, notification modules | Node only |
| C Editor/run/problems | stacks, problems, editor helper modules, new editor components; `EditorPanel.svelte` only after its exact receipt is approved | Node focused |
| D Git/files/worktrees | `shell/git`, `components/git`, explorer presentation, worktree presentation, `src-tauri/src/git_diff_models.rs`, `workspace_entries.rs`, `workspace_file_watch.rs` | Node plus focused Rust |
| E Hosted GitHub | new `shell/github`, `components/github`, `src-tauri/src/github.rs` | One focused Rust lane |
| F Browser/global action surface | `shell/browser`, `components/browser`, `shell/actions/actionSurfaceModel.ts`, `actionSurfaceStore.svelte.ts`, `WorkbenchActionFab.svelte`, `src-tauri/src/browser.rs`, `browser_inspector.js` | One focused Rust lane |
| G Resources/Roslyn | new resources modules, `core/src/scanners/resources.rs`, provider usage; `lsp.rs` only for an approved packet after the TSK-799 contract is re-proven | One focused Rust lane |
| H Markdown | `sourceMarkdownPreview.ts`, `SourceMarkdownPreview.svelte`, new `shell/markdown`, Markdown tests | Node only |
| I Conversation | existing `shell/conversation`, `components/ConversationSurface.svelte`, new `ConversationAgentTree.svelte`, existing `src-tauri/src/agent_conversation`, conversation tests/fixtures | Node plus focused Rust |
| J Integrated agent | new `shell/agent` fact/action/audit helpers and action-preview components; consume lane I and never create another conversation store/provider | Node focused |
| K Extension/API probe | existing `shell/extensions`, the internal probe fixture, compatibility tests/report; post-selection adapter files only after the selected row is written into this plan | Node plus at most one focused Rust/native proof |

No more than two Rust lanes run tests at the same time; the default is one. Lanes B, D, H, I,
and J may run in parallel only after lane A and Work Package 2 contracts land. Every dispatch
expands these directory summaries into an exact path allow-list and records the starting
path-scoped diff; a directory-level row alone is not edit authority.

### 5.3 Cost-aware dispatch manifest

This manifest is the controller's routing checklist. “Luna” means explicit `gpt-5.6-luna`,
`max`, `fork_turns: "none"`. “SOL decision” and “SOL review” mean explicit `gpt-5.6-sol`,
`medium`, `fork_turns: "none"`. A SOL decision packet returns only the named design answer and
plan amendment; the controller then gives the now-mechanical code packet to Luna. A SOL review
is read-only and returns findings to the controller. No agent silently changes route.

| Dispatch packet | Task scope | Default route | Exact handoff and stop boundary |
| --- | --- | --- | --- |
| 0A inventory/re-anchor | All 37 open tasks; TSK-759/760/764/771/773/784 and older verification candidates | Luna read-only, then controller for Notion/shared-plan edits | Run section 6 evidence commands; return one disposition per task. Stop on closed TSK-808, changed base, dirty overlap, or unreadable live task status. |
| 12A API/adapter comparison | TSK-765 and the user's extension/API prerequisite | Luna implementation/probe | Implement only 18.2B's internal fixture and thin adapters, render the fixed metric matrix, and clean up native/browser processes. Return classifications to M0; do not stage a third-party VSIX, marketplace, elevated host, or pause for another selection. |
| M0 extension policy review | Evidence from 0A and 12A | SOL review | Check singleton host, security boundary, metrics, editor/Peek safety, and classification accuracy. Release Works now/Bounded adapter, skip every other new row, and continue to 1A without a user pause. |
| 1A contrast tokens/primitives | TSK-765, contrast/legibility, shared semantic controls | SOL implementation | Implement section 7 in its exact files and tests, resolving the named semantic-role and contrast choices. Stop if a theme contribution must be replaced or the packet needs a new product-level token category. |
| 1B Assembly product identity | TSK-808 public rebrand and compatibility boundary | Luna, sequential | Implement section 7A after contrast. Rename only the enumerated public surfaces and artifacts; preserve every compatibility identity in 7A.4. Stop before changing a bundle ID, storage key, Application Support path, Keychain service, repository/project identity, extension ID, or internal module/package/crate name. |
| 2A shell/shared controls | TSK-758/779/780/378/324 | SOL implementation | Freeze and implement the high-judgment roster/action/minimize/quick-open contracts in 8.1–8.4. Return shared-seam receipts; do not race controller-owned shell files. |
| 2B file Dockview | TSK-360 and Peek tab precondition | Luna | Implement 8.2A's `editorFileDock.ts` contract, one model per URI, close/dirty/restore behavior, and nested-Dockview tests. Stop if Dockview cannot preserve the outer center destination without a new architecture. |
| 2C Settings restoration | Missing Settings regression | Luna | Follow 8.5's existing route/store/schema only; restore reachability, persistence, keyboard/focus, and native proof. Do not create another Settings component or store. |
| M1 shell-foundation review | 1A, 1B, and 2A–2C integrated diff | SOL review | Review contrast, Assembly identity/compatibility, focus, overlay ownership, Dockview nesting, Settings authority, accessibility, and shared-seam edits before any later UI lane. |
| 3A session lifecycle | TSK-761/762/315 and session portions of TSK-192 | Luna | Implement section 9's exact state, migration, reversible actions, notification rules, and metadata. Stop if a lifecycle state cannot be derived from the named deterministic facts. |
| 4A editor first paint/format/run/problems | TSK-783/768/767/770/784 | Luna | Implement 10.1 and 10.2–10.4 in separate exact-file packets; preserve Monaco/LSP ownership and run only focused tests. |
| 4B Peek preservation | Confirmed tab-loss regression plus TSK-789 | Luna discovery/instrumentation, then Luna fix when the cause matches 10.1A | Capture before/during/after roster and geometry. If the cause is outside the named clipping/z-order/layout paths or requires changing editor/center identity, stop for a SOL decision; do not guess. |
| 5A local Git/diff | TSK-307/344 and TSK-808 Git surface | Luna | Implement section 11 over existing Rust Git authority and the current `gitGraphViewModel`. If a new graph library is required, stop for a SOL license/accessibility/layout decision, then resume in Luna. |
| 6A explorer/worktrees | TSK-763/312 and filesystem parts of TSK-280 | Luna | Implement section 12 with canonical containment, typed confirmations, dirty-buffer reconciliation, and one existing worktree-safety authority. |
| M2 core-workbench review | 3A–6A integrated diff | SOL review | Review session truth, LSP/Peek behavior, Git/filesystem safety, SQL declaration, test adequacy, and native evidence. |
| 7A hosted GitHub | Hosted PR/review/check/budget capability | Luna | Implement section 13 as small Rust model/query/prepare/execute and frontend packets. Any mutation not representable by the specified typed prepare/confirm/execute contract stops for SOL decision. |
| 8A browser/global fan | Browser portion of TSK-280 plus the app-wide bottom floating control and annotation queue | Luna after 14.1 proves the specified child-view API | Implement 14.2–14.5 exactly. If z-order, clipping, cookie partitioning, inspector bridge, or whole-app overlay behavior cannot meet 14.1 without a new native architecture, stop for a SOL decision packet; Luna resumes only after the decision is written here. |
| 9A resource facts/UI | Resource ownership, pressure, ports, disk, provider usage | Luna | Implement 15.2–15.5 and 15.7 as bounded snapshot/model/UI packets with explicit owner identities and cleanup. |
| 9B Roslyn registry consolidation | TSK-789 residual lifecycle and one LSP owner | SOL decision/implementation for 15.6 only; Luna may implement resulting mechanical cleanup | This is intentionally SOL because registry consolidation can change process ownership and stale-generation semantics. Stop on any second owner or unexplained CodeLens count change. |
| 10A Markdown | Rich safe Markdown workbench | Luna | Implement section 16 with the named parser/sanitizer, root containment, conflict states, scroll sync, and hostile fixtures. |
| 10B conversation | TSK-809/810 | Luna | Implement 16A packets A–D independently after shared types freeze; reuse one PTY/provider/store and stop on an unsupported live-control protocol or unprovable child-parent association. |
| 11A guarded agent | TSK-766 product outcome and TSK-808 agent actions | Luna | Implement section 17 only after deterministic services and 10B are certified. Never let the model supply facts or bypass native revalidation/confirmation. |
| M3 capability review | 7A–11A integrated diff | SOL review | Review remote/local mutation safety, browser isolation/cleanup, resource ownership, conversation single-runtime proof, Markdown security, and append-only action receipts. |
| 12B approved extension path | Only rows classified as Works now or Bounded adapter | Luna | Controller writes each exact ID/capability, files, adapters, permissions, tests, and resource budget into 18.2A first. Preserve existing declarative assets, but skip new Declarative only/Elevated/Rejected rows without blocking other work. A future third-party VSIX still needs exact package/hash approval. |
| 13A database console | TSK-769, deferred unless explicitly promoted | SOL query/view design, then Luna implementation | One translated SQL statement/view for filtering, sorting, grouping, joining, aggregation, and paging; inspect generated SQL and covering index. No in-memory shaping or N+1. |
| 14A controller integration | TSK-808 final wiring and closure ledger | Controller only | Apply returned receipts once, serialize heavy gates/native proof, and keep every independently tracked task open until its own evidence meets section 20.10. |
| M4 final review | Entire integrated wave | SOL review | Perform milestone code, relevance, security, native-evidence, data-query, cleanup, and task-disposition review. Findings return to the controller; SOL does not silently edit product code. |

The controller copies the complete packet contract from 2.1A into every dispatch, plus the exact
subsection named above. A task is not dispatched from this summary row alone. If its detailed
subsection lacks a concrete symbol, state, failure behavior, test, or native acceptance step at
dispatch time, the controller re-anchors or amends it before assigning Luna.

### 5.4 Planned commit order

PR #14 already merged the former `5884139` checkpoint and its contract follow-up into current
`main`; `167a30a` is the historical merge anchor and `6a3af55` remains earlier ancestry. The
reviewed current base is `beebda6`. Do not rewrite that merged history.

1. plan: record TSK-808 native-workbench execution contract
2. spike: run the real-Tauri extension/API comparison, review the report, and record the user's
   exact selected capability row; preserve the prior Houston/DiffEditor/SCM evidence meanwhile
3. feat: establish accessible workbench tokens and shared controls
4. feat: rebrand public product identity and app artifacts to Assembly without moving user state
5. fix: restore Settings reachability and freeze typed shell panel/action contracts
6. feat: finish sessions, lifecycle, and notifications
7. feat: finish editor, run configurations, format, and problems
8. feat: add full Git graph and safe workspace explorer
9. feat: add guarded hosted pull-request workflows
10. feat: add workspace-isolated native browser
11. feat: add resource ownership and Roslyn lifecycle controls
12. feat: replace Markdown preview with safe rich workbench
13. feat: add guarded integrated-agent actions
14. feat: certify the one conversation surface and read-only child-agent hierarchy
15. feat: enable only the user-selected compatible extension/API path
16. test: certify TSK-808 in the native desktop app

Each commit contains only its owned paths plus the controller wiring required for that lane.

---

## 6. Work Package 0 — reconcile WIP, verify shipped tasks, and re-anchor

**Purpose:** Do not rebuild features that are already present, and do not overwrite the
checkpointed CodeLens/Roslyn, extension, language, conversation, and native-diff work.

**Owner:** Controller. Preservation is complete; integration-base selection and reconciliation
remain controller-owned.

### Current anchors to inventory

- Integration base: clean `main` and `origin/main` at
  `beebda6c4dad60cd2782374a36f24737cefda605`. PR #13 merged the original TSK-799 work; PR #14
  merged the later native-workbench checkpoint and contract fix. The old `5884139` anchor is
  historical evidence only, not the implementation base.
- Existing shell: ShellFrame.svelte:21-155, centerDock.ts:26-323, frame.ts.
- New-session flow: newSessionFlow.ts:24-200, ownedSessions.ts:13-106,
  /next/+page.svelte:477-563.
- Session workspace flow: sessionWorkspaces.ts:26-53 and 340 onward,
  /next/+page.svelte:377-473.
- Playwright cleanup: processes/playwrightBackend.ts:31-79,
  processes/playwrightService.ts:60-145, PlaywrightCard.svelte.
- Scanner prefilter: core/src/scanners/sessions.rs:900-982.
- Svelte gate: package.json scripts check and check:svelte.
- Run configurations: stacks/stackService.ts:61-293 and stackStore.svelte.ts.
- Problems: problemsService.ts:63-179.
- Language status: `LanguageServerStatusChip.svelte` mounted from `EditorPanel.svelte`.

### Steps

1. **Current preservation receipt, 2026-08-03:** PR #14 merged the former protected checkpoint;
   `main` and `origin/main` pointed to `beebda6` at the final planning audit. Reconfirm the lease immediately before any
   implementation:

   git branch --show-current
   git rev-parse HEAD
   git status --short
   git diff --name-only
   git ls-files --others --exclude-standard
   git worktree list --porcelain

   Save future output in the execution log, not in this plan. If new dirty paths appear, ask the
   current owner to classify them. A discard classification is not deletion authority: obtain
   the user’s explicit approval before removing, reverting, or overwriting content.
2. Fetch and prove that the selected base contains the merge before creating any worktree:

   git fetch origin
   git merge-base --is-ancestor 5884139 origin/main
   git rev-parse origin/main

   A zero exit from `merge-base` and the recorded SHA are required. Do not merge, rebase, squash,
   or reset the current checkout during this check.
3. Only after the user authorizes implementation, create the clean TSK-808 worktree from the
   refreshed `origin/main`. Record:

   git rev-parse HEAD
   git status --short
   git worktree list --porcelain
4. Re-anchor every symbol used below. Treat all line references after this section as the
   2026-08-03 audit snapshot until this command is rerun:

   rg -n "createCenterDock|interface Props|activatePanel|registerCommands" tauri-svelte-preview/src/lib/shell
   rg -n "format_source_with_lsp|read_git_commit_history|remove_project_worktree" tauri-svelte-preview/src-tauri/src
   rg -n "SourceLspRegistry|ensure_native_csharp_endpoint|stop_server" tauri-svelte-preview/src-tauri/src/lsp.rs

   Update only moved line references in this file; do not change scope during re-anchoring.
5. Query the live Notion Tasks data source in one server-side statement: filter `Status <> Done`
   and the MacCommandBar Project relation in SQL, order in SQL, and return Task ID, Name, Status,
   Priority, Source, and URL. Confirm TSK-808 is still open, confirm all 37 rows or update section
   3 when the count changed, and fetch each task body before disposition. Never load all tasks and
   then filter/group/sort in the client. Repair missing visible `[TSK-NNN]` title prefixes when a
   task is otherwise being updated; do not rewrite unrelated task bodies.
6. Run the existing focused tests for candidate shipped tasks, serially:

   cd tauri-svelte-preview
   pnpm test:new-session-flow
   pnpm test:owned-sessions
   pnpm test:session-workspaces
   pnpm test:session-scan-filter
   pnpm test:playwright-store
   pnpm test:stack-store
   pnpm test:problems-store
   pnpm test:language-server-status
   pnpm test:git-graph-view-model
   pnpm test:worktree-cleanup-runbook
   pnpm test:worktree-safety
   pnpm test:next-terminal-service
   pnpm test:source-dockview
   pnpm test:source-dock-layout
   pnpm test:source-pane-sizing
   pnpm test:tauri-source
   pnpm test:source-ui
   pnpm check:svelte
   pnpm check
7. Run one real-Tauri audit for TSK-192, 283, 307, 312, 369, 378, 759, 760, 762, 764, 767, 771,
   773, and 784. Evidence must
   show cold launch, project pick, new session, session switch with exact workspace restore,
   file click into Editor, Playwright card status and stop flow in a disposable named session,
   Run configuration start/exit state, language-server status, session/worktree/cwd/file/layout/
   terminal/browser restoration, native activity facts, Git graph helper use, cleanup-runbook
   copy-only output, terminal hide/show/resize/reattach, and reusable overlay input/focus behavior.
   TSK-127, 253, 280, 315, 324, 344, 360, 789, 802, 809, and 810 are not closure candidates in
   this audit; their named residual packets or separate owners remain open.
8. Read TSK-808 directly and null-safely before relying on project-list output. The current
   `list-tasks.sh` helper fails against the present schema with `Cannot index object with null`;
   record that as a tooling limitation, then use the connected Notion data-source query rather
   than treating the failure as an empty worklist. If TSK-808's status cannot be read directly,
   stop. Update Notion only from evidence:

   - Close a task that is wholly shipped and verified.
   - Keep a task open when a named acceptance behavior still fails.
   - Add a short link from a stale absorbed task to TSK-808 or TSK-799 before closing it.
   - Do not reopen Done tasks.

### Stop conditions

- Stop before broad implementation if the selected `origin/main` does not contain `5884139` and
  the PR #14 merge receipt.
- Stop if the Notion status of TSK-808 is Done/closed.
- Stop if a planned shared file is dirty in the new worktree.

### Done

- The checkpoint is merged and product source had no uncommitted changes before this planning
  diff. Work Package 0 is fully done only when implementation is authorized and a clean TSK-808
  worktree exists at the mandated path from refreshed `origin/main`.
- Every one of the 37 open tasks has one current disposition: closed with its own evidence,
  active in a named packet, linked to a separate owner, or explicitly retained/deferred.
- Current line anchors and collision table are recorded.
- No current user WIP was changed.

---

## 7. Work Package 1 — contrast-first theme foundation

**Covers:** TSK-765 contrast and theme correctness; the user’s primary usability blocker.

**Dependencies:** Work Package 0 complete. This package lands before every other UI package.

**Data declaration:** No database is touched in this package. The implementation dispatch still
copies the global SQL rule so a discovered data path cannot be materialized or shaped in memory.

**Owner paths:**

- Modify src/lib/shell/styles/nextTokens.css:41-122.
- Modify src/lib/shell/styles/themeChrome.css.
- Modify src/lib/shell/themes/themeRegistry.ts:81-143 and both theme token maps.
- Modify src/routes/next/+page.svelte:1122-1123 only through a controller integration
  receipt; replace hard-coded Houston colors.
- Create src/lib/shell/themes/contrast.ts.
- Create scripts/contrastTokens.test.mjs.
- Modify scripts/themeRegistry.test.mjs.
- Modify scripts/nextTokens.test.mjs.

### Exact code

Create pure helpers in contrast.ts:

    export type Rgb = { red: number; green: number; blue: number; alpha: number };

    export function parseCssColor(value: string, backdrop?: Rgb): Rgb;
    export function composite(foreground: Rgb, backdrop: Rgb): Rgb;
    export function relativeLuminance(color: Rgb): number;
    export function contrastRatio(foreground: Rgb, background: Rgb): number;
    export function assertContrast(
      foreground: string,
      background: string,
      minimum: number,
      label: string
    ): void;

The parser supports only the formats used by the registry: 3/6/8-digit hex and rgb/rgba.
Reject unknown formats in tests instead of treating them as black.

Change the theme token contract:

- Add --color-selected, --color-selected-border, --color-hover,
  --color-focus-solid, --color-disabled-text, and --color-status-idle.
- Keep --color-focus as the translucent shadow color, but pair it with a solid
  --color-focus-solid for the 3:1 focus boundary.
- Keep --color-live for live/in-turn and --color-good for done/passing. They must never match.
- Point all information-bearing tertiary uses to --color-text-2. Rename no existing token in
  this wave; compatibility matters more than a clean token vocabulary.
- Replace .next-shell route hard-codes with background: var(--color-bg) and
  color: var(--color-text).

Add a token-use audit to contrastTokens.test.mjs:

1. Parse nextTokens.css and every theme in themeRegistry.ts.
2. Assert TOKEN_NAMES is complete for every theme.
3. Test --color-text and --color-text-2 against bg, surface, elevated, selected, and hover.
4. Test focus-solid, selected-border, border when it is a sole boundary, and all status tones
   against their actual backgrounds at 3:1.
5. Assert live and good values differ and have a perceptible hue separation.
6. Scan the files changed by TSK-808 for new raw color literals. A full-tree scan requires an
   explicit baseline allow-list because existing overlays, palette, conversation, Monaco, and
   xterm surfaces contain intentional or pre-existing literals; do not make unrelated WIP fail.
7. Scan for information-bearing font sizes below 12px and fail with file and line.

### UI audit

Audit these first because current evidence shows low contrast:

- SessionsColumn.svelte:583-586, 630-642, 663, 676-715, and row metadata.
- top-level `src/lib/SourceMarkdownPreview.svelte`:58-75, where metadata is currently 10px and
  #8f9996.
- ShellFrame Dockview token mapping at lines 190-218.
- Activity bar, context cards, Git rows, worktree chips, editor status line, Problems metadata,
  run-configuration metadata, and Settings descriptions.

Do not make every surface equally bright. Preserve hierarchy with weight, spacing, surface
depth, and primary versus secondary tokens; do not use unreadable opacity.

### Tests and proof

    cd tauri-svelte-preview
    node --experimental-strip-types scripts/contrastTokens.test.mjs
    pnpm test:next-tokens
    pnpm test:theme-registry
    pnpm check:svelte
    pnpm check

Native visual proof must include matching Houston and Dracula screenshots at:

- 1440 by 900 normal window.
- Narrow window with sessions expanded.
- Settings dialog and portaled menu.
- Git rows, Problems rows, session rows, editor status, and disabled actions.
- Keyboard focus on a row and icon action.

### Done

- Automated ratios pass.
- No required text uses --color-text-3 or opacity to fall below 4.5:1.
- Running and done are distinguishable by color and word.
- The /next root no longer bypasses theme tokens.
- The user can read subtext without selecting or hovering it.

---

## 7A. Work Package 1B — Assembly product identity and compatibility migration

**Covers:** The requested MacCommandBar-to-Assembly app rename, public runtime identity, app
artifacts, copy hierarchy, and preservation of all existing settings/session/layout/keychain data.

**Owner and route:** One explicit Luna Max implementation packet, run sequentially after Work
Package 1 and before Work Package 2. The controller leases the shared config/route/native seams
listed below for this packet only, reviews its path-scoped diff, and commits it before parallel
feature lanes start. SOL-medium reviews the integrated result at M1; SOL is not the default
implementer because all product and compatibility decisions are fixed here.

**Dependencies:** Work Package 0 must re-anchor the line references and verify both the Tauri
shell and legacy Swift shell build paths. R0 must apply the recorded Works now/Bounded adapter
policy, and Work Package 1 contrast foundation must be complete. No other UI lane may run while
this packet edits window titles, root routes, or native self-identification.

**Data declaration:** No database is touched in this package. The implementation dispatch still
copies the global SQL rule. Local browser storage, Application Support files, Keychain services,
and environment variables are compatibility state, not database work, and must be preserved as
specified below.

### 7A.1 Locked product copy

Create `tauri-svelte-preview/src/lib/productIdentity.ts` as the only TypeScript source for public
brand copy:

```ts
export const PRODUCT_NAME = 'Assembly' as const;
export const PRODUCT_TAGLINE = 'Build in parallel.' as const;
export const PRODUCT_DESCRIPTOR =
  'The workspace for parallel software development.' as const;
export const PRODUCT_NARRATIVE =
  'Agents, worktrees, and code—working together.' as const;

export const PRODUCT_DOCUMENT_TITLE = PRODUCT_NAME;
export const PRODUCT_UI_LIBRARY_TITLE = `${PRODUCT_NAME} UI Library`;
```

Copy placement is fixed:

1. `Assembly` is the app name in native window titles, menu-bar title, visible app headings,
   Tauri `productName`, LSP `clientInfo.name`, `TERM_PROGRAM`, public provider labels, errors,
   and app-artifact display names.
2. `Build in parallel.` is the primary compact tagline for an existing subtitle, About row, or
   product-lockup slot. Do not add a splash screen, hero, modal, or permanent chrome merely to
   display it.
3. `The workspace for parallel software development.` is the longer descriptor for existing
   descriptive metadata or product copy where one sentence already belongs.
4. `Agents, worktrees, and code—working together.` is an approved narrative/marketing variant;
   store it in the identity module, but do not show multiple taglines in the same surface.
5. Repository/project labels may still say `MacCommandBar` when they identify this repository,
   a fixture project, or its current on-disk path. Product name and repository name are different
   concepts until a separately authorized repository migration exists.

Create `tauri-svelte-preview/src-tauri/src/product_identity.rs` as the Rust source for native
self-identification:

```rust
pub(crate) const PRODUCT_NAME: &str = "Assembly";
pub(crate) const TERM_PROGRAM: &str = PRODUCT_NAME;
pub(crate) const LSP_CLIENT_NAME: &str = PRODUCT_NAME;
pub(crate) const STARTUP_FAILURE_CONTEXT: &str = "failed to run Assembly";
```

`tauri-svelte-preview/src-tauri/src/main.rs` declares `mod product_identity;` beside the existing
native modules. Rust code imports these constants; do not duplicate new public strings across
`terminal.rs`, `lsp.rs`, and `main.rs`. JSON and Swift cannot import either identity module, so
the product-identity test in 7A.6 enforces exact parity across those boundaries.

### 7A.2 Exact public rename surfaces

Line numbers below describe the audited `beebda6` baseline plus the current planning diff. Work
Package 0 must re-run `rg -n` and record moved anchors before dispatch; symbols and semantics win
when a line moves.

| Path and current anchor | Required Assembly edit |
| --- | --- |
| `tauri-svelte-preview/src-tauri/tauri.conf.json:3,16` | Set `productName` and the main window `title` to `Assembly`. Keep line 5 `identifier` byte-for-byte unchanged. |
| `tauri-svelte-preview/src-tauri/tauri.dev.next.conf.json:12` | Set the development window title to `Assembly`. |
| `tauri-svelte-preview/src/routes/next/+page.svelte:966` | Import `PRODUCT_DOCUMENT_TITLE`; render it in `<svelte:head><title>`. Do not add a second visible brand region. |
| `tauri-svelte-preview/src/routes/+page.svelte:14843,15018` | Replace the legacy preview document title and visible eyebrow through `productIdentity.ts`; update the user-facing terminal preview at line 5909 and rescan instruction at line 6263. Preserve repository IDs, paths, storage keys, authors, and fixture data elsewhere in this file. |
| `tauri-svelte-preview/src/routes/preview/+page.svelte:73` | Render `Assembly UI Library — visual verification` through `PRODUCT_UI_LIBRARY_TITLE`. |
| `tauri-svelte-preview/src/lib/MonacoSourceEditor.svelte:1247` | Change the public LSP display name to `Assembly LSP`, preferably through `PRODUCT_NAME`. |
| `tauri-svelte-preview/src/lib/shell/extensions/extensionCatalog.ts:2,47` | Change only public prose/label to `Assembly Git provider`; keep contribution IDs and activation contracts unchanged. |
| `tauri-svelte-preview/src/lib/shell/extensions/rustGitScmProvider.ts:57` | Change the visible source-control label to `Assembly Git`; keep internal ID `mcb-rust-git`. |
| `tauri-svelte-preview/src-tauri/src/terminal.rs:98` | Set `TERM_PROGRAM` from `product_identity::TERM_PROGRAM`; add a focused test named `product_identity_terminal_uses_assembly`. |
| `tauri-svelte-preview/src-tauri/src/lsp.rs:842,956` | Update public comment terminology and set `clientInfo.name` from `product_identity::LSP_CLIENT_NAME`; add/assert a `product_identity_lsp_client_name_is_assembly` fixture. |
| `tauri-svelte-preview/src-tauri/src/main.rs:5272` | Use `product_identity::STARTUP_FAILURE_CONTEXT` in the final `.expect(...)`; do not change fixture projects named MacCommandBar at lines 6473 onward. |
| `tauri-svelte-preview/src-tauri/Cargo.toml:4` | Describe the crate as the Tauri shell for Assembly; preserve crate `name = "mac-command-bar-webview-preview"`. |
| `tauri-svelte-preview/src-tauri/capabilities/default.json:4` | Change the human-readable description to the Assembly window; preserve capability identifier/permissions. |
| `Sources/MacCommandBar/MacCommandBarApp.swift:5,9` | Keep the Swift type/module names; change `MenuBarExtra`'s public title to `Assembly`. |
| `Sources/MacCommandBar/CommandCenterView.swift:99` | Change only the visible product heading to `Assembly`. |
| `scripts/build-app.sh:5,26,30,32,52` | Emit `dist/Assembly.app`; keep internal executable `MacCommandBar` and `CFBundleExecutable` unchanged; keep bundle ID unchanged; set `CFBundleName`/`CFBundleDisplayName` and final receipt to `Assembly`. |
| `tauri-svelte-preview/src/app.css:4`, `src/lib/styles/tokens.css:2`, and current public docs/comments | Update stale product prose only where it describes the app. Do not use comment cleanup as permission to rename identifiers. |

`Package.swift`, Swift source directory names, Swift type names, and the internal executable remain
unchanged in this wave. `Assembly.app` is the public artifact; `MacCommandBar` inside its
`Contents/MacOS` directory is a compatibility implementation detail. A later internal-module
cleanup requires its own migration plan because it changes package products, imports, test
targets, build scripts, and downstream automation without improving the public rebrand.

### 7A.3 New environment variable with legacy fallback

At `tauri-svelte-preview/src-tauri/src/orchestration.rs:8-12`, introduce:

```rust
const ASSEMBLY_ORCHESTRATION_EVENTS_ENV: &str = "ASSEMBLY_ORCHESTRATION_EVENTS";
const LEGACY_ORCHESTRATION_EVENTS_ENV: &str = "MAC_COMMAND_BAR_ORCHESTRATION_EVENTS";
```

Replace the single-variable check in `orchestration_event_store_path` at lines 265-280 with a
small helper that checks the Assembly variable first, then the legacy variable, ignores blank
values, and otherwise returns the existing
`~/Library/Application Support/MacCommandBar/orchestration-events.jsonl` path. Do not move or
copy the default file. Add focused tests named:

- `product_identity_orchestration_prefers_assembly_env`;
- `product_identity_orchestration_falls_back_to_legacy_env`;
- `product_identity_orchestration_preserves_legacy_support_path`.

The tests serialize environment mutation inside the existing Rust test process and restore both
variables in cleanup. They use temporary paths and never read or overwrite the user's real
Application Support ledger.

### 7A.4 Compatibility identities that must not change

The following values intentionally retain their old spelling during the first Assembly release.
The Luna packet stops rather than editing one accidentally.

| Compatibility identity | Exact examples | Why it remains stable |
| --- | --- | --- |
| Tauri/macOS bundle identifiers | `dev.blackcolours.MacCommandBarWebviewPreview` in `tauri.conf.json:5`; `dev.blackcolours.MacCommandBar` in `scripts/build-app.sh:28` | Changing either makes macOS treat Assembly as another app and can detach permissions, webview state, and Keychain access. |
| Local/browser storage keys | `mac-command-bar.settings` in `settingsStore.svelte.ts:96`; every `mac-command-bar.next.*` and `mac-command-bar.source-browser.*` key | Existing Settings, tabs, sessions, layout, project selection, and drafts must open unchanged after the visual rename. |
| Application Support directories | `MacCommandBar` in `orchestration.rs:279`, `AppStorageRepository.swift:68`, and `LocalFileContentCipher.swift:32` | Preserves ledgers, app state, and encrypted local content without a risky file migration. |
| Keychain and queue service IDs | `dev.blackcolours.MacCommandBar` in `KeychainContentCipher.swift:17` and `CoreClient.swift:97` | Prevents new credential prompts or loss of encrypted-content access. |
| Repository/project identity | repository slug/path, project ID `mac-command-bar`, project display/fixture value `MacCommandBar`, Git author fixtures | The app product is Assembly; the repository has not been renamed. History and orchestration joins depend on stable IDs. |
| Internal package/module/crate names | npm `mac-command-bar-tauri-svelte-preview`, Rust `mac-command-bar-webview-preview`, `mcb-core`, Swift `MacCommandBar`/`MacCommandBarKit` | These are build/API compatibility names, not user-facing product copy. |
| Extension/native IDs | `mcb-rust-git`, Tauri command names, capability IDs, event names | Renaming IDs can orphan registrations, persisted view state, or permissions. Change labels only. |
| Legacy environment input | `MAC_COMMAND_BAR_ORCHESTRATION_EVENTS` | Existing scripts keep working; the new Assembly variable wins when both are nonblank. |

Do not add key-copy migrations, dual writes, symlinks, directory moves, Keychain copies, or a
second settings namespace in this package. Preservation is achieved by leaving the compatibility
identities in place. Any later desire to change them is a separate, explicitly approved migration
with rollback and on-disk data proof.

### 7A.5 Ordered implementation steps

1. Re-run the Work Package 0 brand inventory with exact paths. Classify every match as public
   product copy, compatibility identity, repository/fixture identity, or stale comment. Save the
   classification in the packet receipt; an unclassified match is a stop condition.
2. Add the TypeScript and Rust identity modules and their focused tests before changing consumers.
3. Update Tauri JSON titles and the existing `scripts/tauriDevConfig.test.mjs` assertions while
   preserving identifiers and permissions.
4. Update `/next`, legacy `/`, and `/preview` document/visible copy by importing the TypeScript
   constants. Do not rename routes, components, storage keys, Dockview IDs, session IDs, or URLs.
5. Update the Monaco LSP display name and extension/SCM labels. Keep provider/contribution IDs.
6. Update Rust terminal, LSP, startup context, Cargo description, capability description, and the
   orchestration environment alias. Add `mod product_identity;` in the controller-owned main seam.
7. Update the legacy Swift shell's visible menu/title and public `.app` directory/plist display
   name. Keep package, target, module, executable, bundle identifier, support directory, Keychain
   service, and queue label unchanged.
8. Run the allow-listed stale-public-brand scan and focused tests in 7A.6. Review every remaining
   match rather than applying a global replacement.
9. Run the upgrade/native proof in 7A.7 with the controller's single native runner. Record the old
   and new visible identity plus preservation receipts. Stop and revert only this packet if state
   appears missing; do not create compensating copy logic during certification.

### 7A.6 Focused tests and automated guardrails

Create `tauri-svelte-preview/scripts/productIdentity.test.mjs`. It must:

1. import `productIdentity.ts` with Node type stripping and assert the four locked strings;
2. parse both Tauri config files and assert `productName`/window titles are `Assembly` while the
   Tauri identifier remains `dev.blackcolours.MacCommandBarWebviewPreview`;
3. inspect `product_identity.rs`, the LSP initialize fixture, terminal builder, extension catalog,
   SCM provider, route heads, capability description, Cargo description, Swift visible strings,
   and `scripts/build-app.sh` public plist/artifact values;
4. assert the compatibility table's exact storage prefixes, support directories, Keychain
   service, bundle IDs, environment fallback, crate/package/module names, and `mcb-rust-git` ID
   still exist;
5. scan only the enumerated public-runtime files for stale public `MacCommandBar`, `Mac Command
   Bar`, or `CommandBar · next` copy. The test carries a path-and-literal allow-list for
   compatibility/repository identities; it must not suppress a whole file or use a generic regex
   exemption.

Update `tauri-svelte-preview/scripts/tauriDevConfig.test.mjs` for `Assembly` titles and the
unchanged identifier. Add `product_identity_*` Rust tests close to the affected functions so one
filter exercises the native identity contract. Run in this order, with no parallel heavy command:

```bash
cd tauri-svelte-preview
node --experimental-strip-types scripts/productIdentity.test.mjs
pnpm test:tauri-config
pnpm check:svelte
pnpm check
RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml product_identity -- --nocapture
cd ..
swift test --filter MacCommandBarKitTests
```

The Swift command is required only if Work Package 0 confirms the Swift shell remains a supported
build/release surface; otherwise the packet still updates its public strings/build script and
records the shell as legacy. Do not run `pnpm build`, `cargo build`, and `swift build` in parallel.
The controller owns any later full build and calls `dotnet build-server shutdown` only if a
separate .NET lane used MSBuild.

### 7A.7 Native upgrade and artifact proof

Browser-preview proof alone cannot close the rename. In the real rebuilt Tauri app, capture:

1. macOS app/window identity, title bar, app switcher/process display where available, and the
   `/next` document title showing `Assembly` rather than a preview-era product name;
2. an integrated terminal command that prints `TERM_PROGRAM=Assembly`;
3. a captured LSP initialize request showing `clientInfo.name = Assembly`, followed by successful
   semantic navigation and CodeLens/Peek to prove the server did not fork or reset;
4. the same pre-existing theme, Settings values, session/workspace selection, open tabs, active
   file, Dockview layout, terminal metadata, and conversation draft before and after the rebuild;
5. the same legacy orchestration ledger visible from the preserved Application Support path, plus
   proof that both the new environment override and legacy fallback resolve as specified;
6. no new empty storage namespace, duplicate Application Support directory, Keychain prompt,
   permission reset, or second app identity;
7. if the Swift artifact is still supported, `dist/Assembly.app` with
   `CFBundleDisplayName=Assembly`, unchanged `CFBundleIdentifier` and internal executable, an
   `Assembly` menu-bar title, and successful launch.

Use non-secret sentinel values for comparison and record key names/counts or redacted hashes, not
settings contents, tokens, or Keychain values. Preserve the user's real data; do not delete or
reset it to make the proof pass.

### 7A.8 Stop conditions

Stop the Luna packet and return an exact receipt without improvising if:

- any required public rename appears to require changing a bundle identifier, storage origin/key,
  Application Support path, Keychain service, repository/project ID, extension ID, or internal
  package/crate/module/API name;
- existing settings, sessions, layouts, drafts, permissions, or encrypted content disappear or a
  new Keychain prompt appears after the renamed build;
- Tauri and Swift public artifact names cannot both be `Assembly` while preserving their current
  identifiers and internal executable/module contracts;
- an unclassified old-brand match remains in a public-runtime surface;
- displaying a tagline would require inventing a new splash/About/modal/chrome surface;
- a model or effort override cannot be proven, another agent owns an overlapping path, or native
  proof/process cleanup cannot be completed.

### 7A Done

- Public app, window, document, menu, LSP, terminal, provider, error, metadata, and artifact names
  consistently say `Assembly`.
- The copy constants preserve all three approved lines, with `Build in parallel.` as the primary
  compact tagline and no duplicate tagline clutter.
- Every compatibility identity in 7A.4 is unchanged and tested.
- The rebuilt native app opens existing user state without migration, duplication, reset, or
  credential prompt.
- Focused tests pass, native evidence is attached, the public-brand scan has no unexplained match,
  and all Tauri/browser/test processes opened for proof are stopped by their owner.

---

## 8. Work Package 2 — shared controls, shell roster, quick open, and edge minimization

**Covers:** TSK-758, TSK-779, TSK-780, the TSK-808 missing-Settings regression, and shared
contracts required by every later lane.

**Dependencies:** Work Packages 1 and 1B.

**Data declaration:** No database is touched in this package. The implementation dispatch still
copies the global SQL rule.

### 8.1 Shared action primitives

Create:

- src/lib/shell/components/shared/ShellIconButton.svelte
- src/lib/shell/components/shared/ShellStatusBadge.svelte
- src/lib/shell/components/shared/ShellToolbar.svelte
- src/lib/shell/components/shared/ShellCompactRow.svelte
- src/lib/shell/components/shared/ShellCompactCard.svelte
- src/lib/shell/components/shared/ShellDetailPopover.svelte
- src/lib/shell/components/shared/ShellAccordionSection.svelte
- src/lib/shell/components/shared/ShellEdgeStrip.svelte
- scripts/shellPrimitives.test.mjs

Reuse and certify the existing bits-ui v2 wrappers at `src/lib/components/Menu.svelte`,
`ContextMenu.svelte`, and `Tooltip.svelte`; do not create another menu/tooltip/context-menu
family under `shell/components/shared`. Consolidate their duplicated portal/menu surface styles
into the Work Package 1 tokens only after tests prove current consumers. One app-level Tooltip
provider owns skip-delay behavior; individual consumers do not mount competing providers.

ShellIconButton props:

    interface Props {
      icon: Component;
      label: string;
      tooltip?: string;
      tone?: "neutral" | "danger";
      size?: "dense" | "regular";
      disabled?: boolean;
      pressed?: boolean;
      shortcut?: string;
      onclick?: (event: MouseEvent) => void;
    }

Behavior:

- dense is 32 by 32; regular is 36 by 36; glyphs remain 14–16px.
- Real button disabled semantics; no click handler when disabled.
- aria-label always uses label; aria-pressed only when pressed is defined.
- Tooltip opens on pointer hover and keyboard focus and includes shortcut when present.
- focus-visible uses --color-focus-solid plus the focus shadow token.

ShellStatusBadge props:

    type ShellStatusTone =
      | "live"
      | "good"
      | "attention"
      | "bad"
      | "idle"
      | "neutral";

    interface Props {
      label: string;
      tone: ShellStatusTone;
      detail?: string;
      pulse?: boolean;
    }

The component renders label text and an aria-hidden dot. Pulse is ignored under Reduced
Motion. It never communicates through color alone.

`ShellCompactRow` and `ShellCompactCard` own the shared desktop information hierarchy used by
sessions, Git, resources, worktrees, and hosted reviews: leading identity, primary text,
secondary text, status slot, trailing action island, selected/current state, and a full-row
keyboard target. The action island is visible on row focus-within as well as hover, remains
reachable by touch/trackpad, and never steals the row's primary action. `ShellDetailPopover`
owns bounded supplementary facts; `ShellAccordionSection` owns labelled expand/collapse and
per-section measured height. No lane invents a local hover-only ellipsis, tooltip, border, or
details surface after this package lands.

### 8.2 Typed center roster

Modify ShellFrame.svelte:25-164 and centerDock.ts:26-73, 85-175, 298-333:

1. Evolve the current `CenterPanelSpec[]` contract into `CenterPanelRegistration[]`; do not add
   a competing roster type. Replace the four fixed bound slot variables with a
   Map<CenterPanelId, HTMLElement>.
2. The parking stage creates one host for each registration. The implementation may use a
   keyed Svelte each block; it must preserve Svelte ownership during Dockview moves.
3. CenterDock.activatePanel accepts CenterPanelId.
4. Add activePanelId(): CenterPanelId | null.
5. Add setPanelVisible(id, visible). Hidden panels return to parking and are not immediately
   re-added by the permanent-roster guard; restoring them reuses the same element.
6. Keep onPanelActivated startup suppression in panelActivation; no capability loads at
   initial Dockview construction.
7. Change close behavior from “every roster tab is permanent” to per-registration permanent.
8. Add roster version to layoutStorage.ts. Bump once to include gitGraph, pullRequests, and
   markdown.
9. Preserve the merged TSK-799 `CenterDockSnapshot` capture/restore and existing conversation/
   browser workspace restoration. Migrate old snapshot IDs and per-registration permanent
   policy explicitly; do not discard saved layouts or re-add hidden panels through the current
   permanent-roster guard.

Tests:

- Existing source-dock/layout tests remain green.
- A hidden panel has no Dockview panel and its Svelte content remains parked.
- Restore uses the same element.
- activePanelId follows user selection.
- Stored layouts with the old panel set fall back once and persist the new roster.
- No panel activation causes backend IO before allowSessionLoads.

### 8.2A Real per-file Dockview panels for TSK-360

Do not treat the outer `editor` destination tab as completion of TSK-360. The current `/next`
`EditorPanel.svelte` owns a custom `.file-strip` above one Monaco host. Replace that inner strip
with a nested editor-file Dockview using the existing SourceDockview patterns; do not add a third
docking library and do not turn every file into a new Monaco process.

Exact contract:

1. Create `shell/editor/editorFileDock.ts` with `EditorFilePanelId = string`,
   `EditorFileDockSnapshotV1`, `createEditorFileDock`, `openEditorFilePanel`,
   `activateEditorFilePanel`, `closeEditorFilePanel`, `captureEditorFileDock`, and
   `restoreEditorFileDock`. Panel identity is the canonical file URI/path, never the display name.
2. Modify `EditorPanel.svelte` to render one Svelte-owned host per open file and let Dockview move
   those hosts. The existing editor store remains the one authority for open, active, recent,
   dirty, model version, selection, and close-confirmation state.
3. Preserve one Monaco text model per canonical URI. Splitting a file creates another view over
   the same model and view-specific selection/scroll state; it does not create another language
   client, Roslyn server, dirty buffer, or file watcher.
4. Closing the last view runs the existing dirty-file confirmation. Closing one of several views
   does not close the model. A failed/cancelled close leaves the Dockview panel and active identity
   unchanged.
5. Add the editor-file layout to the existing versioned session workspace snapshot. Old snapshots
   hydrate a single active file panel; missing files render a recoverable missing-file panel and
   do not retarget another path.
6. Drag, split, stack, activate, and restore must coexist with the outer center destinations
   Session/Editor/Browser/Diff/Git/PR/Markdown. CodeLens Peek stays inside the active editor-file
   panel and may not cover the inner tabs or outer destination switch.

Add `scripts/editorFileDock.test.mjs` for identity, reuse, split-view/model sharing, dirty close,
snapshot migration, missing file, and restore. Native proof opens three files, splits one, edits
it, switches outer destinations, opens/closes Peek, restarts, restores the layout, and proves the
same dirty model and visible tab layers. Stop if implementation requires replacing the current
editor store or language-client owner; return that as a SOL-medium decision packet.

### 8.3 Top-bar quick open

Create `src/lib/shell/components/TopBarDestinationSwitch.svelte` as the three-button destination
switch. Do not create another command palette or file/symbol quick-open overlay: reuse the
existing `PalettePanel.svelte`, `shell/palette/commandRegistry.ts`, and top-level
`components/overlays/QuickOpenOverlay.svelte` for Cmd/Ctrl+K and Cmd/Ctrl+Shift+P flows.

Props:

    interface Props {
      active: CenterPanelId | null;
      open(panel: "session" | "editor" | "browser"): void;
    }

Render three labelled 32px buttons: Session, Editor, Browser. Use the shared primitive, show
selected state, and expose shortcuts:

- Cmd+1 Session
- Cmd+2 Editor
- Cmd+3 Browser

Modify shellCommands.ts and commandRegistry.ts only through controller integration:

- show-session
- show-editor
- show-browser

Mount TopBarDestinationSwitch in `/next/+page.svelte` near the current top bar around 1046 and
call the existing
showCenterPanel control. Do not create a second navigation store.

### 8.4 Edge minimization

Create src/lib/shell/layout/shellMinimizeState.ts:

    export type MinimizedRegion = "sessions" | "tools" | "dock";

    export interface ShellMinimizeSnapshot {
      sessions: boolean;
      tools: boolean;
      dock: boolean;
      lastToolView: SidebarViewId;
    }

    export const SHELL_MINIMIZE_STORAGE_KEY =
      "mac-command-bar.next.shell-minimize.v1";

    export function readShellMinimizeSnapshot(storage: LayoutStorage): ShellMinimizeSnapshot;
    export function writeShellMinimizeSnapshot(
      storage: LayoutStorage,
      snapshot: ShellMinimizeSnapshot
    ): boolean;
    export function toggleRegion(
      snapshot: ShellMinimizeSnapshot,
      region: MinimizedRegion
    ): ShellMinimizeSnapshot;

ShellEdgeStrip renders only the minimized region’s labelled restore button. It must not
destroy the region’s component or service state. The controller maps state to frame controls:

- sessions: existing collapsed width and existing SessionsColumn collapsed strip.
- tools: width equals the activity strip only; keep the selected tool view parked.
- dock: height equals a 32px edge strip rather than zero.

Restore the last nonzero dimensions from the frame before minimization. If no measured value
exists, use the frame defaults. Persist only after a real measured layout.

For TSK-324, add per-section resizing inside the right context/tool region without a second
layout engine. Extend the existing `paneStack.ts` model with `SectionSizeSnapshotV1`,
`resizeSection(id, delta, bounds)`, `collapseSection(id)`, and `restoreSection(id)`. Persist only
measured sizes, clamp each section to its declared minimum/maximum, and expose a keyboard-operable
separator with `aria-valuenow`. Hide/show, resize, preset restore, and app restart must preserve
the same terminal/editor/browser/resource instances.

For TSK-369, do not dispatch a redesign. First record whether the existing persistent
`TerminalSurface.svelte` host, `terminalService.ts` single listener, ResizeObserver/
MutationObserver/font callbacks, and Dockview parking stage already fix the former blank canvas.
The diagnosis receipt names destroyed/hidden host plus stale fit/geometry as the cause or records
the actual alternative. Only an evidenced missing visibility/refit hook may be changed.

### 8.5 Restore and certify the existing Settings screen

**Reported regression:** The user previously had a Settings screen and can no longer see or
reach it. Treat this as a native product regression to reproduce, not as evidence that Settings
must be designed from scratch. The merged PR #14 snapshot already contains two deliberate
route-specific views over one canonical store:

- The legacy route imports `src/lib/SettingsPanel.svelte` at `/routes/+page.svelte:2`, owns
  `settingsOpen` at line 739, toggles it from the gear at lines 14985-14990, and mounts
  `<SettingsPanel bind:open={settingsOpen} />` at line 16530. Freeze this route except for a
  shared-component compatibility repair proven necessary by a regression test.
- The `/next` route imports `ActivityBar`, `ShellOverlays`, and the canonical `settings` store at
  `/routes/next/+page.svelte:25-40`; the activity snippet calls `overlays?.openSettings()` at
  lines 996-1001; and the single `ShellOverlays` instance is mounted at lines 1098-1106.
- `ActivityBar.svelte:58-86` renders the Settings gear and calls `onOpenSettings`.
- `ShellOverlays.svelte:46-70` forwards both the gear and palette command to one
  `SettingsHost` instance.
- `SettingsHost.svelte:45-100` owns the open flag, lazy-loads `SettingsDialog.svelte`, and
  exposes a visible load-failure path.
- `PalettePanel.svelte:136-160` already registers exactly one `open-settings` command.
- `SettingsDialog.svelte:20-477` renders Appearance, Editor, Terminal, and General tabs plus
  Problems placement and C# language-server controls from `settingsStore.svelte.ts`.
- `settingsStore.svelte.ts:21-125, 130-188, 197-252` is the only settings schema/default/load/
  persist/update/reset authority, using `mac-command-bar.settings`.

Do not mistake the present controls for completed behavior. The current audit found that editor
ligatures and terminal cursor blink are page-local toggles, `/next` persists `terminal.theme`
without a proven xterm consumer, and the legacy route has a separate `sourceTerminalApp` state/
storage path alongside `settings.general.terminalApp`. The C# adapter also needs an explicit
rollback contract for unsupported, thrown, and response-state-mismatch cases. These are acceptance
gaps to reconcile, not permission to replace TSK-799's dirty Roslyn lifecycle work.

Work Package 0 must refresh these line anchors immediately before dispatch. It must also
record the exact failure class from a rebuilt Tauri app before an implementation agent edits a
file:

1. Is the activity strip or bottom gear clipped, hidden under another region, below the minimum
   window height, or visually indistinguishable because of the current low-contrast colors?
2. Does clicking the visible gear reach `/next/+page.svelte -> ShellOverlays.openSettings() ->
   SettingsHost.open()` exactly once?
3. Does `open-settings` from Cmd/Ctrl+K or Cmd/Ctrl+Shift+P reach the same host instance?
4. Does the lazy import resolve in the packaged Tauri build? If not, capture the import error and
   generated asset path; do not replace the lazy load until that failure is understood.
5. Does `Dialog.Content` mount but render below a Dockview layer, portaled menu, or native child
   webview? Inspect stacking context, focus owner, and actual bounds.
6. Does the dialog open only on the legacy route while the shipped app starts `/next`, or vice
   versa? Record the real launch URL from Tauri configuration rather than assuming the route.

#### Implementation contract

Implement the smallest repair that explains the reproduced failure, then harden all supported
entry points without creating a parallel surface:

1. Keep one `SettingsHost` mounted by one `ShellOverlays`. The gear, `open-settings` palette row,
   and a new direct Cmd/Ctrl+, shortcut all call that host's `open()` method. Register the direct
   shortcut in the existing page-level `PalettePanel` keyboard owner, ignore repeated keydown,
   call `preventDefault`, and do not fire while the event is already handled by a modal/text
   editor that owns the chord. Add the shortcut to the gear tooltip and palette detail.
2. Refactor `SettingsHost.open()` only if the native reproduction proves the lazy boundary is the
   cause. Keep its repeated-open and in-flight guards. A rejected import must leave a readable
   `role="alert"` with Dismiss and Retry actions; Retry invokes the same guarded loader. Do not
   perform backend IO at import or from a Svelte effect.
3. Keep `SettingsDialog` as the `/next` view and `SettingsPanel` as the frozen legacy view. Both
   remain projections of the one `settingsStore.svelte.ts` object and the same storage key. Do
   not copy settings values into page-local state except ephemeral control state. Do not add a
   third dialog to `ShellFrame`, Dockview, or the command palette.
4. Make the gear permanently reachable at normal, narrow, minimum-height, sessions-expanded,
   tools-minimized, and dock-minimized layouts. Use `ShellIconButton` and Work Package 1 tokens;
   preserve a 32px minimum hit target, a visible focus ring, a plain-English tooltip, and a
   text-equivalent accessible name. The control must not depend on hover to become discoverable.
5. Keep Settings modal and blocking: one backdrop, one dialog, no Dockview activation, and no
   shell command executing through it. Use the existing bits-ui/shadcn Dialog primitives for
   focus containment, Escape, outside-click policy, and accessible title/description. On close,
   return focus to the exact opener: gear, palette result, or keyboard owner.
6. Put the dialog in the established overlay/portal layer above Dockview and HTML overlays. When
   the native Browser lane exists, hide or lower every child webview while Settings is open and
   restore its exact visibility/bounds after close; a native child webview must never paint or
   receive input over the modal.
7. Preserve every current control and its stored value:
   - Appearance: theme and app font size.
   - Editor: font family, font size, line height, and the currently UI-only ligatures control.
   - Terminal: font family, font size, line height, color theme, and the currently UI-only cursor
     blink control.
   - General: external terminal application, Problems location, and C# language-server state.
   Convert ligatures and cursor blink into versioned store fields only when this work also wires
   them to their real Monaco/xterm owners; otherwise label them unavailable and do not display a
   switch that merely changes page-local state.
8. Audit every visible control against its real application owner. Theme changes call the
   existing `themeService`; Problems placement calls the existing shell callback; C# changes use
   `setCsharpLanguageServerEnabled`; editor and terminal settings update the existing Monaco and
   xterm instances without creating new editors/terminals. A setting that needs restart/reopen
   says so beside the control. A control that cannot affect the product is not presented as
   functional. Specifically:
   - Prove `settings.terminal.theme` repaints every active xterm instance and becomes the default
     for a new/restarted terminal; otherwise remove or disable that chooser until the real owner
     is wired.
   - Reconcile `settings.general.terminalApp` with the legacy route's separate
     `sourceTerminalApp`/storage path through one canonical value and compatibility migration.
     Because the legacy route is a large interleaved change preserved in checkpoint `5884139`,
     Work Package 0 records the exact shared bridge as a controller receipt before any edit; do
     not rewrite or reformat that route opportunistically.
   - The C# switch commits the new stored value only when the capability response confirms the
     same enabled state. Unsupported capability, thrown/rejected request, or a mismatched returned
     state restores the previous value and shows the plain-English reason. Do not change native
     Roslyn process ownership or TSK-799 semantic behavior in this repair.
9. Keep `mergeWithDefaults` behavior for partial, stale, and corrupt stored records. Add a
   versioned migration only when the schema actually changes. Never clear the old
   `mac-command-bar.settings` record just to make the screen open. Reset section preserves all
   other sections; add Reset all only through the existing AlertDialog confirmation primitive.
10. Never persist tokens, cookies, passwords, repository credentials, full environment values,
    extension secrets, or single-use confirmation IDs. Settings remains local and requires no
    database or network call. No database is touched in this lane.
11. The future Notifications, Hosted Review, Browser, Resources, Extensions/Themes, and
    conditionally promoted Database sections remain owned by Work Package 14 integration. Add
    their tabs only when their underlying work packages deliver a real policy/store contract;
    do not show empty placeholder sections while restoring reachability.

#### Focused tests

Extend `scripts/nextTokens.test.mjs` only for token/portal/static ownership assertions. Add:

- `scripts/settingsStore.test.mjs` for default merge, corrupt JSON recovery, enum validation,
  schema migration when applicable, section reset, reset-all confirmation model, persistence,
  and forbidden-secret-key assertions.
- `scripts/settingsReachability.test.mjs` for the one-host chain, gear/palette/direct-shortcut
  convergence, repeat-key suppression, retryable import failure, opener-focus restoration,
  modal stacking contract, and absence of a third settings store/dialog mount.
- `scripts/settingsApplication.test.mjs` for terminal-theme repaint/new-terminal defaults,
  canonical external-terminal selection and legacy migration, Problems placement, theme repaint,
  and C# success/unsupported/throw/mismatched-state rollback without duplicate Roslyn processes.
- Package scripts `test:settings-store` and `test:settings-reachability` in the controller-owned
  `package.json`, plus `test:settings-application`; edit the clean checkpointed manifest only in
  the selected integration worktree after Work Package 0 records its starting SHA.

Run in the focused Node slot:

    cd tauri-svelte-preview
    pnpm test:settings-store
    pnpm test:settings-reachability
    pnpm test:settings-application
    pnpm test:next-tokens
    pnpm test:theme-registry
    pnpm test:command-registry
    pnpm test:panel-activation
    pnpm check:svelte
    pnpm check

#### Native proof and stop condition

In the rebuilt Tauri app, record one uninterrupted proof that:

1. The gear is visible and readable in both themes at normal, narrow, and minimum-height sizes.
2. Gear click, palette `Open settings`, and Cmd/Ctrl+, each open the same dialog once.
3. Appearance, Editor, Terminal, General, Problems, and C# controls are present; subtext meets
   Work Package 1 contrast; keyboard-only and VoiceOver navigation identify tabs and controls.
4. Change one value in each persisted section, close/reopen Settings, restart the app, and prove
   the values and real affected surfaces remain in sync. This includes terminal-theme repaint for
   existing and new terminals, external-terminal selection, all Problems locations, and C#
   unsupported/error/mismatched-response rollback without a duplicate language-server process.
5. Reset one section without disturbing the others, then cancel and confirm Reset all.
6. Escape and Done close once and return focus to the exact opener. No shell command or native
   child webview receives click/keyboard input through the modal.
7. A forced lazy-load failure produces readable Dismiss/Retry actions and the retry opens the
   same dialog after the fixture is removed.

Stop and report rather than claiming restoration if the gear is still absent/clipped, any entry
point opens a different instance, the dialog exists only in browser preview, a displayed setting
does not affect its named owner, persistence loses prior values, focus escapes, a child webview
covers the modal, or any required subtext remains below the Work Package 1 contrast floor.

### Tests and proof

    cd tauri-svelte-preview
    node --experimental-strip-types scripts/shellPrimitives.test.mjs
    pnpm test:layout-storage
    pnpm test:pane-layout
    pnpm test:source-dock-layout
    pnpm test:source-dockview
    pnpm test:command-registry
    pnpm test:panel-activation
    pnpm test:session-strip
    pnpm check:svelte
    pnpm check

Native proof:

- Quick-open buttons and shortcuts open the existing instances.
- Minimize/restore preserves terminal grid, editor tabs, Browser placeholder, tool selection,
  Problems rows, and scroll position.
- Keyboard and VoiceOver identify all actions and selected state.
- Narrow layout does not reduce action hit targets.
- Settings opens through gear, palette, and Cmd/Ctrl+, from the same host and store; persistence,
  focus return, retry failure, and modal-over-native-webview behavior match section 8.5.

### Done

- Shared action/status primitives are the only new per-row action implementation.
- Center and tool rosters can accept later features without reopening fixed prop shapes.
- TSK-758, TSK-779, and TSK-780 acceptance behaviors are proven in Tauri.
- The previously available Settings screen is visibly restored in Tauri with no duplicate host,
  store, dialog instance, or legacy-route regression.

---

## 9. Work Package 3 — sessions, semantic status, and native notifications

**Covers:** TSK-761, any evidenced TSK-762 remainder, and the session capability lane.

**Dependencies:** Work Packages 1 and 2.

**Data declaration:** No database is touched in this package. The implementation dispatch still
copies the global SQL rule.

**Current anchors:**

- SessionsColumn.svelte:579-744 renders My Work, Working, Done, and Find a session.
- The rescan action at lines 645-651 remains enabled during scanning.
- sessionCardModel.ts:42 onward maps live/background to running, exited/completed to done, and
  exited/uncompleted to stopped.
- sessionWorkspaces.ts:340 onward captures/restores session state.
- /next/+page.svelte:455 onward selects workspaces.
- terminalService.ts:499 natural-exit callback is the correct completion boundary.
- orchestrationView.ts:535-579 already derives attention and decision queues.

### 9.1 Exact state model

Create src/lib/shell/sessions/sessionActivity.ts:

    export type SessionActivityState =
      | "in-turn"
      | "running"
      | "needs-attention"
      | "awaiting-approval"
      | "blocked"
      | "failed"
      | "idle"
      | "done"
      | "stopped";

    export interface SessionActivityInput {
      owned: OwnedSession;
      orchestrationRun: OrchestrationRun | null;
      terminalExitCode: number | null;
      activeOwnedId: string | null;
      now: number;
    }

    export interface SessionActivity {
      state: SessionActivityState;
      label: string;
      detail: string;
      tone: ShellStatusTone;
      actionable: boolean;
    }

    export function deriveSessionActivity(input: SessionActivityInput): SessionActivity;

User lifecycle and live activity are separate dimensions. Add to the existing owned-session
record and persistence migration:

    export type SessionLifecycleState = "working" | "done" | "settled";

    export interface SessionLifecycleFields {
      lifecycle: SessionLifecycleState;
      completedAt: string | null;
      settledAt: string | null;
      checkpointDetail: string | null;
    }

Old records map `completedAt != null` to `done` and otherwise `working`; no old record becomes
`settled` automatically. `done` means the user considers the work complete even if the terminal
is still inspectable. `settled` is the reversible archive tier and never deletes transcript,
workspace snapshot, terminal receipt, or action history. `Move to Working`, `Mark Done`, and
`Settle` are explicit idempotent actions. Permanent Remove remains a separate destructive action
with the existing confirmation and worktree-safety checks.

Precedence:

1. failed when a natural terminal exit has nonzero code.
2. blocked when the latest orchestration event is blocked.
3. awaiting-approval when the decision queue contains an approval for this owned session.
4. needs-attention when the attention queue contains a nonapproval item.
5. in-turn when an agent event explicitly says it is responding/working.
6. running when the terminal is live without a more specific event.
7. done when completedAt exists.
8. stopped when exited without completedAt.
9. idle otherwise.

Do not infer blocked or approval state from prose. `OwnedSession` currently lacks both exit code
and activity state, so persist natural terminal exit information in one explicit run/session
record rather than deriving failure from a transient callback. Extend both the frontend
`tauriSource.ts` event type and Rust `src-tauri/src/orchestration.rs` append-only JSONL schema
with optional ownedId/activityState fields, bump/default the schema additively, and prove old
events still deserialize and round-trip.

### 9.2 Session UI

Extend the existing cards and models; do not recreate them:

- components/SessionsColumn.svelte
- components/sessions/SessionCard.svelte, or the current equivalent after re-anchor
- components/sessions/sessionCardModel.ts and its tests
- stores/sessionRailStore.svelte.ts

Changes:

- Primary card button is the only workspace-navigation target and has aria-current when
  active.
- Working, Done, Settled, and Find are labelled sections/lists. Done and Settled remain visible,
  actionable, and reversible; collapsing either section does not change lifecycle state.
- Card summary shows project, repository/worktree, branch, agent, model, activity label,
  relative time, PR/check hint, runtime/ports, and message-count floor when available.
- Expanded detail shows first prompt, full path, timestamps, terminal/session IDs, last
  activity, the last checkpoint detail/commit or explicit `No checkpoint recorded`, and explicit
  actions.
- Keep Find a session collapsed until the user opens it. Its row only expands; it never starts
  or resumes a session.
- Resume/restart/continue are named separate actions in the overflow menu.
- Rescan is disabled while scanning.
- All actions use ShellIconButton or a 32px labelled menu row.
- The session column remains resizable and its cards never shrink below content.
- Do not introduce a new session/workspace store.
- Preserve the current status/detail/actions/reduced-motion behavior in SessionCard and
  sessionCardModel. The new activity descriptor replaces only the ambiguous state derivation.
- Define the exact per-ownedId/latest-generation orchestration event used for blocked,
  attention, and approval precedence; a global queue item cannot color the wrong session.

Add a pure sessionActionMenu.ts that returns action descriptors from deterministic facts:

    export type SessionActionId =
      | "open-worktree"
      | "resume"
      | "continue-new"
      | "copy-resume"
      | "open-log"
      | "reveal-log"
      | "open-folder"
      | "copy-session-id"
      | "copy-log-path"
      | "open-terminal"
      | "inspect-session"
      | "open-pull-request"
      | "mark-done"
      | "move-to-working"
      | "settle"
      | "remove";

    export function sessionActions(
      session: SessionCardViewModel,
      capabilities: SessionActionCapabilities
    ): SessionActionDescriptor[];

`open-terminal` activates the existing owned PTY; `inspect-session` opens the existing read-only
transcript/detail surface; `open-pull-request` is present only when deterministic hosted-Git facts
contain a PR URL. None may launch an agent or infer a URL from prose.

### 9.2A Exact resumable metadata for TSK-315

Extend the existing `OwnedSession`, scanner bridge, `sessionCardModel.ts`, and session card; do not
create a parallel resume DTO. The compact row must distinguish the exact resumable target with:

- provider and model/effort when recorded, otherwise `Unknown`;
- canonical project/repository root, exact worktree/cwd, branch, task ID, and PR;
- dirty/clean fact from the existing Git authority and whether a saved workspace snapshot exists;
- active, stopped-resumable, stale-log-only, missing-worktree, and nonresumable reasons;
- checkpoint detail and age.

Scanner/store fields are optional and additive. UI never guesses model, worktree, repo, dirty
state, or resumability from a title/path substring. Add named fixtures for two same-title sessions
in different worktrees, missing model, missing worktree, stale transcript, active owned terminal,
and saved snapshot. Native acceptance selects the correct one of the same-title pair and proves
the resume action uses its exact provider/session/worktree tuple.

### 9.3 Native notifications

Add dependencies in the controller integration commit:

- JavaScript @tauri-apps/plugin-notification
- Rust tauri-plugin-notification

Register the plugin in main.rs and grant only the official permission-check,
permission-request, and notify capabilities to the main webview.

Create:

- src/lib/shell/notifications/notificationTypes.ts
- src/lib/shell/notifications/sessionNotificationPolicy.ts
- src/lib/shell/notifications/nativeNotifications.ts
- src/lib/shell/notifications/notificationStore.svelte.ts
- src/lib/shell/components/notifications/NotificationSettings.svelte
- src/lib/shell/components/notifications/NotificationCenter.svelte
- scripts/sessionNotifications.test.mjs

`NotificationSettings.svelte` includes a **Send test notification** action. It is enabled only
after permission is granted, uses a fixed local test event with a unique ID, records the same
decision/center receipt as a real event, and never impersonates a session completion. Denial or
unsupported platform leaves the button disabled with the reason.

Settings shape:

    export interface NotificationSettings {
      enabled: boolean;
      agentTaskComplete: boolean;
      needsAttention: boolean;
      awaitingApproval: boolean;
      terminalBell: boolean;
      suppressWhileFocused: boolean;
      sound: "system" | "subtle" | "none";
    }

Defaults: all false except suppressWhileFocused true and sound system. Never request macOS
permission at startup. Enabling notifications explicitly requests permission. Denial resets
enabled to false and presents an inline explanation.

Policy API:

    export interface SessionNotificationEvent {
      id: string;
      ownedId: string;
      kind: "completed" | "failed" | "needs-attention" | "awaiting-approval" | "bell";
      title: string;
      body: string;
      occurredAt: string;
      terminalGeneration: number | null;
    }

    export function notificationDecision(
      event: SessionNotificationEvent,
      context: NotificationPolicyContext
    ): { sendNative: boolean; addToCenter: boolean; reason: string };

Deduplicate by event id plus terminal generation. Never notify for manual Close,
reload/reattach reconciliation, adopted tombstones, duplicate exit events, permission denial,
or a disabled event class. Suppress the native alert while the app is focused when configured,
but still add the item to NotificationCenter. The bell indicator opens the center and the
entry’s action selects the owned session inside the app.

If the desktop plugin cannot deliver notification-click identity on the supported macOS
version, native notifications remain passive and the in-app center carries the action. Do not
claim an OS action that the API cannot prove.

### Tests

    cd tauri-svelte-preview
    pnpm test:session-card-model
    pnpm test:session-groups
    pnpm test:session-strip
    pnpm test:session-workspaces
    pnpm test:owned-sessions
    node --experimental-strip-types scripts/sessionNotifications.test.mjs
    pnpm test:orchestration-view
    pnpm check:svelte
    pnpm check
    RUST_TEST_THREADS=1 cargo check --manifest-path src-tauri/Cargo.toml

Test lifecycle migration and reversible Working/Done/Settled transitions, state precedence,
exact labels/tones, selection semantics, rescan disabled state,
notification permission transitions, every suppression reason, deduplication, focused-app
behavior, test-notification behavior, and old persisted settings.

### Native acceptance

1. First launch makes no notification permission request.
2. Explicit enable requests once; denial restores off.
3. A background natural success emits one correctly named notification.
4. A nonzero exit emits one failure notification.
5. Manual Close, reload, and reattach emit none.
6. Needs-attention and approval events appear in the in-app center and select the exact
   session.
7. Keyboard-only and VoiceOver navigation announces sections, selected session, action names,
   and all states.
8. Mark a live/inspectable row Done, Settle it, restore it to Working, and prove transcript,
   workspace, terminal receipt, checkpoint detail, and actions were never deleted.
9. Open Terminal, Inspect, and Open Pull Request from the context menu and prove each targets the
   exact owned session; send the fixed test notification after permission is granted.

### Done

- TSK-761 has deterministic state semantics.
- TSK-762 remains correct under richer cards.
- Native notifications are opt-in, deduplicated, and honest about action support.

---

## 10. Work Package 4 — editor first paint, formatting, run configurations, and Problems

**Covers:** TSK-767, TSK-768, TSK-770, TSK-783, TSK-784, and the 2026-08-03
TSK-808 CodeLens Peek/tab-retention regression.

**Dependencies:** TSK-799 files released; Work Packages 1 and 2.

**Data declaration:** No database is touched in this package. The implementation dispatch still
copies the global SQL rule.

### 10.1 Editor paints readable text before Monaco

Current `src/lib/shell/components/EditorPanel.svelte`:324-355 dynamically imports Monaco, while
the canvas/keyed native-C# area around 671-713 can show only a loading sentence until the
component exists.

Create:

- src/lib/shell/components/editor/EditorFirstFrame.svelte
- src/lib/shell/editor/editorFirstFrame.ts
- scripts/editorFirstFrame.test.mjs

EditorFirstFrame props:

    interface Props {
      content: string;
      language: string;
      targetLine?: number | null;
      loading: boolean;
    }

Render escaped text in a pre element with:

- the same font family, size, line height, padding, background, foreground, tab size, and
  target-line scroll offset as Monaco;
- no syntax colors and no HTML injection;
- content-visibility and a bounded visible slice for very large files;
- aria-label “Read-only text while the code editor starts.”

Modify EditorPanel:

1. Begin loading Monaco when Editor is first activated, not on application launch.
2. As soon as readFileIntoEditor receives preview.content, render EditorFirstFrame.
3. Mount Monaco in the same positioned canvas, wait for its first layout/model render, then
   cross-fade for at most 80ms or swap immediately under Reduced Motion.
4. Preserve selection/target line and never blank the canvas during native C# activation.
5. Replace the current {#key nativeCsharpActive} remount at lines 659-678 with an additive
   nativeCsharpLanguageClient prop update if TSK-799 tests prove Monaco accepts it. If a remount
   is unavoidable, keep FirstFrame visible and instrument the exact remount.
6. Add timings under MCB_TIMING=1: read complete, first text paint, Monaco ready, native C#
   attached, CodeLens first paint, CodeLens settled.

Stop if changing remount behavior alters semantic CodeLens count or Peek targets.

### 10.1A CodeLens Peek preserves editor tabs and active file identity

The user-supplied 2026-08-03 screenshot `codex-clipboard-WBFL58.png` shows `UploadSettings.cs`
selected in the Files pane, CodeLens rows above the C# class and member declarations, and the
left/right workbench panes still visible after Peek is reportedly opened. The screenshot also
shows no visible center editor tab strip above the editor content. A still image cannot prove
whether Dockview removed the tab group, the Monaco/Peek widget covered it, a CSS height/z-index
rule clipped it, or a route layout state failed to render it. Implementation must diagnose that
boundary before fixing.

The visible `Build workspace | Test workspace | Build project | Test project` row aligns with the
current `/next` editor, but the rebuilt Tauri launch URL is the authority. Record the actual route
before editing. If reproduction is on `/next`, protect two independent tab layers:

1. the four permanent center destinations (`Session`, `Editor`, `Browser`, `Diff`) created by
   `createCenterDock()` in `src/lib/shell/layout/centerDock.ts:73-123` and mounted by
   `src/lib/shell/components/ShellFrame.svelte:88-115`; and
2. the open-file tablist rendered as `.editor-header > .file-strip` in
   `src/lib/shell/components/EditorPanel.svelte:626-669`, immediately before the Monaco-owned
   `.editor-canvas` at lines 671-713.

If the native launch instead reproduces on legacy `/`, issue a controller-owned compatibility
receipt before touching the frozen route. The corresponding owners are
`src/lib/components/panels/EditorPanel.svelte:248-269,591-617`, the nested editor-files host in
`src/lib/SourceDockviewShell.svelte:322-383`, and the `sourceEditorFileDockviewWorkspace`
create/sync/layout/dispose lifecycle in `src/routes/+page.svelte:12306-12358,12764-13076`.
Do not patch both routes speculatively.

Do not solve this by disabling Peek, replacing Monaco's reference widget, removing CodeLens, or
creating a second editor/tab system. Keep the TSK-799 semantic contract: the visible CodeLens
count and Peek rows use one settled semantic answer.

Inspect and instrument, in this order:

1. `src/lib/shell/editor/csharpLanguageClient.ts:319-352`, method
   `registerDocumentActions()`: the native `roslyn.client.peekReferences` command resolves
   `vscode.executeReferenceProvider` and then calls `editor.action.showReferences`. Log the command,
   URI, position, result count, elapsed time, and rejected/thrown state; never log file contents.
2. `src/lib/MonacoSourceEditor.svelte:2158-2235`, methods `showCodeLensReferences()`,
   `runCodeLensReferenceCommand()`, and `registerSourceCodeLensReferenceCommand()`: the existing
   browser/custom path calls `editor.action.peekLocations` inside Monaco. Under `MCB_TIMING=1`, log
   click start, reference-provider return, command selected, Peek open/close, editor instance ID,
   and active model URI. The trace must establish which path the rebuilt native app actually uses.
3. `src/lib/shell/components/EditorPanel.svelte:626-713`: add stable diagnostic hooks
   `data-testid="editor-file-tabs"` on `.editor-header` and
   `data-testid="editor-canvas"` on `.editor-canvas`. Capture `editorState.openFiles.length`,
   `editorState.activePath`, both elements' `isConnected`, `getBoundingClientRect()`, computed
   `display`/`visibility`/`overflow`, and whether the `{#key nativeCsharpActive}` block remounted.
   Do not introduce a persistent Peek store merely to collect this trace.
4. `src/lib/shell/layout/centerDock.ts:56-68,73-123,298-333`: capture the roster IDs, active panel,
   group count, center-tab-container bounds, and editor panel element identity before click, after
   Peek opens, after Peek closes, and after result navigation. No Peek open/close event may call
   `activatePanel()`, `restoreLayout()`, `resetLayout()`, renderer `dispose()`, or the roster re-add
   path.
5. `src/lib/shell/components/ShellFrame.svelte:98-129,198-244`: inspect both the Dockview-generated
   `.shell-center-dock .dv-tabs-and-actions-container` and the file tablist above. The existing
   35px center-tab minimum and visible contract must remain effective while Peek is open.
6. `src/routes/next/+page.svelte:934-948,1018-1061` plus
   `src/lib/shell/sessionWorkspaces.ts`: count `snapshotWorkspace()`/`restoreWorkspace()` and
   center-layout capture/restore calls. A CodeLens click, Peek open/close, or result highlight is
   not a session switch and must produce none of those calls.
7. CSS touching Dockview, `.center-panel-host`, `.editor-panel`, `.editor-header`, `.editor-canvas`,
   `.monaco-host`, `.zone-widget`, `.peekview-widget`, compact chrome, and hidden/empty tab classes.
   Fix only the rule proven by computed layout/trace evidence. Avoid global `!important`, body-level
   overflow changes, and broad z-index escalation.

Record one invariant snapshot before click, during populated Peek, and after close:

    interface PeekChromeInvariant {
      route: "/next" | "/";
      centerPanelIds: string[];
      activeCenterPanelId: string | null;
      centerTabStripHeight: number;
      openFilePaths: string[];
      activeFilePath: string | null;
      fileTabStripHeight: number;
      editorElementId: string;
      monacoModelUri: string | null;
    }

All fields must remain identical across the before-click, populated-Peek, and after-close snapshots.
Only explicit result selection may change the active file/model afterward. Rendering Peek itself
must not mutate the center roster, open-file order, dirty flags, active file, or workspace snapshot.

Expected fix shape:

- Keep the center tab strip and top-bar destination controls outside the Monaco scroll/canvas area.
- Let Monaco's Peek widget live inside the editor content area only; it may consume editor height,
  but never the workbench chrome above the editor.
- Preserve the selected file tab/active file label while Peek is open.
- Closing Peek returns focus to the editor and leaves the same active file selected.
- Switching workbench tabs while Peek is open closes or parks Peek predictably, without losing
  the editor panel element or writing a corrupted center-layout snapshot.
- Opening a Peek result in another file either navigates through the existing editor-store/open-file
  path or opens the existing file tab; it must not replace the editor group with an anonymous Peek
  surface.
- Browser child webviews, Settings dialogs, and other overlays retain their existing stacking
  contracts; do not globally raise editor/Peek above modal layers.

Choose the smallest evidenced branch:

- **Command-path defect:** if `editor.action.showReferences` alone reproduces the loss while the
  existing `editor.action.peekLocations` path does not, add one tested in-editor reference-display
  adapter and route the native command through it. Preserve the single reference-provider result;
  do not issue a second lookup, disable native CodeLens, or fall back to the old custom scanner.
- **Editor-layout defect:** if `.editor-header` remains connected but computes to zero/hidden,
  correct the owning `.editor-panel`/`.editor-header`/`.editor-canvas` flex or stacking rule. Keep
  `.editor-header` non-growing and non-shrinking and constrain the Peek widget to `.editor-canvas`.
- **Dockview defect:** if the outer center tab container disappears or the editor renderer is
  parked/disposed, guard the exact `centerDock.ts` transition that the trace identifies. Do not
  replay serialized layouts, rebuild the four-tab roster, or recreate the editor on every Peek.
- **State/remount defect:** if `openFiles`, `activePath`, the editor instance, or the workspace
  snapshot changes, remove the Peek-triggered write/remount. Reuse the existing editor/open-file
  navigation path only after the user selects a Peek result.

Stop and return to planning if more than one branch is required without trace evidence, if the
only apparent fix changes the TSK-799 count/location coupling, or if the bug cannot be reproduced
in the rebuilt native route shown by the user.

Tests:

- Add `scripts/codeLensPeekLayout.test.mjs` and package script `test:code-lens-peek-layout`. Assert
  that `/next` renders `.editor-header` as a sibling before `.editor-canvas`, the Monaco host is a
  descendant of the canvas only, the file header cannot shrink, and the permanent center tab
  container retains its nonzero minimum height. If legacy `/` is the proven route, add its nested
  Dockview assertions to this same script; do not create a second test command.
- Extend `scripts/csharpLanguageClient.test.mjs` to pin one reference-provider request, the selected
  in-editor display command, the unchanged URI/position/location list, and error/empty-result
  behavior. The test must fail if a native click triggers a second semantic lookup.
- Extend `scripts/sourceCodeLensKeys.test.mjs` and `scripts/sourceUi.test.mjs` to preserve one
  settled count/location key, populated Peek rows, the editor/file-tab DOM boundary, and the
  selected source after result navigation.
- Extend `scripts/panelActivation.test.mjs` only if `centerDock.ts` changes: Peek open/close leaves
  the roster, active Editor panel, renderer identity, and captured layout unchanged; result
  selection may change the file model but not the center panel roster.
- Add focused editor-store/session-workspace tests only if the trace finds a state write. Assert
  open-file order, active/dirty paths, and saved center layout before/during/after Peek.
- Keep `pnpm test:source-code-lens-keys`, `pnpm test:source-ui`, `pnpm test:editor-store`,
  `pnpm test:panel-activation`, `pnpm test:csharp-language-client`, `pnpm check:svelte`, and the
  native-LSP/CodeLens tests green. Run focused Node tests first; run the heavier native suite once,
  sequentially, after the UI contract passes.

Native acceptance:

1. In the rebuilt Tauri app, open a C# file with at least two open-file tabs and the permanent
   center destination tabs visible.
2. Capture a before screenshot showing the active editor tab strip/top-bar destination controls,
   selected file identity, CodeLens row, and Files pane selection.
3. Click the CodeLens references row.
4. Within two seconds, Peek opens with populated rows and the center tab strip/top-bar destination
   controls remain visible and usable.
5. Capture an after screenshot and short screen recording proving the tab strip did not disappear,
   the active file identity stayed visible, and no side pane or file tree selection was lost.
6. Select a Peek result, close Peek with Escape, switch to another center tab, then return to the
   editor tab. The same editor tab, active file, selection/target line, CodeLens count, and Peek
   behavior remain intact.
7. Repeat once at a narrow width where the tab strip may scroll/overflow. It may collapse into an
   intentional accessible overflow control only if that control remains visible, focusable, and
   named; it may not silently disappear.
8. Repeat with one open file, three open files including one dirty file, both themes, zero/one/many
   reference symbols, and three consecutive open/Escape-close cycles. The file-tab list/order,
   dirty marker, active file, center roster, cursor/selection, and editor scroll position survive.
9. Select a result in another file. Only that explicit selection may open/reuse a file tab; merely
   opening Peek must not add, remove, reorder, or activate a file.

**TSK-789 cancellation/re-arm contract:** preserve one in-flight reference lookup per canonical
root/model URI/position/generation in `csharpLanguageClient.ts`. A newer click cancels/supersedes
the previous request; a late response is discarded before calling the display command. CodeLens
registration is attached once per editor/model generation and refreshed only by semantic-result,
document-version, server-generation, or explicit refresh events—not every keystroke or render.
Add named assertions to `csharpLanguageClient.test.mjs` for superseded cancellation, late-result
drop, single registration across edits, one settled answer reused by count/Peek, and cleanup on
model/root disposal. Close TSK-789 only when baseline/final metrics in 15.8 and the native two-
second acceptance prove this behavior; do not infer closure from existing code.

### 10.2 Wire Format Document

Existing backend/wrapper:

- main.rs:1009-1017 format_source_with_lsp.
- tauriSource.ts:1016-1025 formatSourceWithLspFromTauri.
- MonacoSourceEditor.svelte:241 onFormatDocument prop and 2587 command registration.

Create src/lib/shell/editor/formatDocument.ts:

    export interface FormatDocumentRequest {
      root: string;
      preview: SourcePreview;
      version: number;
    }

    export interface FormatDocumentResult {
      edits: SourceTextEdit[];
      requestedVersion: number;
    }

    export async function requestFormatDocument(
      request: FormatDocumentRequest
    ): Promise<FormatDocumentResult | null>;

Wire EditorPanel to onFormatDocument:

- Capability-gate formatDocument.
- Reject formatting for read-only, missing-root, missing-preview, server-disabled, or
  unsupported-language state with a plain explanation.
- Capture Monaco model version before the call.
- Apply returned edits only when root, path, active model, and version still match.
- Use Monaco executeEdits followed by pushUndoStop so one Undo restores the pre-format
  document.
- Do not write the file automatically; mark it dirty and use the existing save flow.
- Register command palette action editor-format-document with the same disabled reason.

Tests cover stale result rejection, multi-edit ordering, one undo group, unsupported state,
and no silent save.

### 10.3 Finish Run configurations

Current stackService.ts:61-90 defines StackHandlers and /next/+page.svelte:784 registers them.
Do not rename persisted “stack” keys.

Extend the existing implementations; create only the missing validation/test helpers:

- components/run/RunButton.svelte
- components/run/RunConfigurationDialog.svelte
- stacks/stackValidation.ts
- scripts/runConfigurations.test.mjs

Preserve the existing persisted definition and run-record split:

    export interface StackDefinition {
      id: string;
      name: string;
      script: string;
      cwd: string;
      env?: StackEnvVar[];
    }

`StackRunRecord` remains the owner of started time, exit code, signal, and live run state. Old
records without env hydrate compatibly. The existing RunButton and RunConfigurationDialog are
extended, not recreated. The dialog validates name, project-relative folder, nonempty script,
unique ID, and KEY=value environment rows. Start uses the existing page handler and
runCommandDirectly. Stop targets the exact ownedId. No polling; terminal start/exit events
update the run record. Keep the current top-bar mount.

### 10.4 Add build output to Problems

Existing problemsService.ts:63-115 loads workspace LSP diagnostics and lines 125-166 provide an
old-build per-file fallback. Preserve both.

Create:

- problems/buildOutputParser.ts
- problems/problemsIngestion.ts
- scripts/buildOutputParser.test.mjs

Types:

    export interface BuildDiagnostic {
      source: "build";
      tool: "dotnet" | "cargo" | "typescript" | "svelte" | "unknown";
      path: string;
      line: number | null;
      column: number | null;
      severity: "error" | "warning" | "info";
      code: string | null;
      message: string;
      runId: string;
    }

    export function parseBuildOutput(
      tool: BuildDiagnostic["tool"],
      output: string,
      root: string,
      runId: string
    ): BuildDiagnostic[];

    export function mergeProblemRows(
      lsp: readonly ProblemRow[],
      builds: readonly BuildDiagnostic[]
    ): ProblemRow[];

Hook ingestion to explicit Run/build/test terminal records, not arbitrary shell output. Keep
the last bounded result per run configuration and expose a Clear build results action. Dedupe
LSP/build rows by normalized path, line, column, severity, code, and message; retain both
sources in row metadata. Opening a row uses openFileBus.

The existing old-build per-file fallback is an N-request compatibility route, not the desired
steady state. Do not expand it. The capability-backed root diagnostics call remains the normal
one-call route.

### Focused tests

    cd tauri-svelte-preview
    node --experimental-strip-types scripts/editorFirstFrame.test.mjs
    pnpm test:editor-panel-language-server
    pnpm test:language-server-status
    pnpm test:csharp-language-client
    pnpm test:source-ui
    pnpm test:open-file-bus
    pnpm test:stack-store
    node --experimental-strip-types scripts/runConfigurations.test.mjs
    pnpm test:problems-store
    node --experimental-strip-types scripts/buildOutputParser.test.mjs
    pnpm check:svelte
    pnpm check

Run the focused Rust LSP test only after frontend tests:

    RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml lsp -- --nocapture

### Native acceptance

- Cold app starts with no editor/LSP work.
- First file text appears before Monaco and does not flash blank.
- C# first-frame swap preserves settled semantic counts and Peek.
- Format Document changes the model, remains unsaved, and one Undo reverts it.
- Run configuration starts in the exact folder with exact env, reports starting/running/exit,
  and stops only its terminal.
- dotnet, cargo, TypeScript, and Svelte fixture errors appear in Problems and open the exact
  location.
- Language-server starting/indexing/ready/off/crashed text remains accurate.

### Done

- All five task records have current evidence and closure state.
- No CodeLens count, Peek target, editor tab, or session workspace regression.

---

## 11. Work Package 5 — full local Git graph and selectable diffs

**Covers:** TSK-808 local Git graph/history/diff requirements and the relevant TSK-344/307
overlap without reopening their task ownership.

**Dependencies:** Work Package 2 roster contract; local Git lane owns its files exclusively.

**Data declaration:** No database is touched in this package. Git history filtering/sorting/paging
occurs in the bounded Rust Git request, not after fetching an entire repository. The
implementation dispatch still copies the global SQL rule.

**2026-08-03 checkpoint already present on current `main`:**

- `src-tauri/src/git_diff_models.rs` now reads bounded original/modified text for working-tree
  and commit diffs. Each side is capped at 512 KiB; binary/NUL and invalid UTF-8 content returns
  no Monaco model and stays on the safe fallback.
- `SourceGitDiff` in Rust and `tauriSource.ts` now carries nullable `originalContent` and
  `modifiedContent` alongside the untouched patch path.
- `components/git/NativeGitDiffEditor.svelte` lazily joins the existing Monaco/VS Code runtime,
  creates distinct `file://` original/modified models, and disposes owned models/editor state.
- `GitDiffView.svelte` selects the native DiffEditor when both models exist and preserves the
  unified-text fallback for browser/backend mismatch, binary content, or exceeded bounds.
- `src/lib/server/gitBridge.ts` supplies the same bounded read-only models in development browser
  preview, which proved the interaction without claiming native-desktop acceptance.
- `rustGitScmProvider.ts` projects already-loaded staged/working/untracked resources through
  `vscode.scm.createSourceControl`; it deliberately has no Git/process/Tauri execution and no
  mutation commands yet.

Do not rebuild these seams. Reconcile and retain them, then implement the graph/query and
clipboard/action remainders below. The Git graph uses the existing Rust Git/history authority;
neither Git Graph nor GitLens becomes a second provider.

**Current anchors:**

- gitService.ts:55-76 limit/paging, 87-141 GitBackend, 180-212 GitService, and 267-337
  guarded history requests.
- gitPanelStore.svelte.ts:34-87 state.
- main.rs:1207-1217 current command and 2603-2629 HEAD-only Git log.
- tauriSource.ts:133-169 models and 720-730 wrapper.
- `shell/components/git/GraphPane.svelte`:31-177 model/layout helpers and 180-414 current narrow
  graph.
- `shell/components/GitDiffView.svelte`:21-110 parser/UI and 199-232 line layout.
- The legacy `/routes/+page.svelte` has a separate ActivityGitPanel/GitInsightsPanel and loader
  stack. TSK-808 makes the `/next` `shell/git` service the active authority, adapts reusable
  `gitGraphViewModel.ts` algorithms, and freezes the legacy route except for a proven shared
  compatibility wrapper. It does not create a third Git owner.

### 11.1 Git-side filtered query

Extend the existing `/next` `shell/git` service and add only the missing query contract:

- src/lib/shell/git/gitGraphQuery.ts
- scripts/gitGraphQuery.test.mjs

Contracts:

    export type GitGraphBranchFilter =
      | { kind: "current" }
      | { kind: "all" }
      | { kind: "ref"; ref: string };

    export interface GitGraphQuery {
      branch: GitGraphBranchFilter;
      author: string;
      search: string;
      limit: number;
    }

    export interface GitGraphBranchOption {
      ref: string;
      label: string;
      current: boolean;
      remote: boolean;
    }

    export interface GitGraphSnapshot {
      commits: GitCommitHistoryEntry[];
      branches: GitGraphBranchOption[];
      authors: string[];
      requestedLimit: number;
      hasMore: boolean;
    }

Add capability gitGraphSnapshotV1 and Rust command:

    read_git_graph_snapshot(
      root: String,
      query: GitGraphQuery
    ) -> Result<GitGraphSnapshot, String>

Rust implementation rules:

1. Canonicalize and validate the repository root.
2. Read branch facets with one git for-each-ref invocation.
3. Validate a requested ref against returned facets and git rev-parse --verify.
4. Reject a ref beginning with dash, NULs, and overlong author/search values.
5. Build a literal Command argument vector. Never use a shell string.
6. current queries HEAD, all adds --all, and ref passes the validated positional revision.
7. Author uses one bounded --author value; search uses --grep plus
   --regexp-ignore-case.
8. Request limit plus one, trim to limit, and set hasMore from the extra record.
9. Cap requested limit at 500.

This filtering/paging is executed by Git in one provider command, not by loading thousands of
commits and filtering in Svelte. Git is not a SQL database; the DB-side SQL contract applies
only to Work Package 13 or later structured database work.

Extend GitBackend:

    readGraph(root: string, query: GitGraphQuery): Promise<GitGraphSnapshot | null>;

Extend GitService:

    activateSelection(selection: ProjectSelection): void;
    setGraphRepository(root: string): void;
    setGraphQuery(patch: Partial<GitGraphQuery>): Promise<void>;
    refreshGraph(): Promise<void>;
    loadMoreGraph(): Promise<void>;

Changing repository or filters invalidates history and commit-file request guards, resets the
page to 24, and prevents old responses from landing. Repository choices come directly from
list_git_repository_summaries(selection.projects); they do not depend on Worktrees having
loaded.

### 11.2 Full center Git workspace

Create:

- components/git/GitGraphWorkspace.svelte
- components/git/GitGraphToolbar.svelte
- components/git/GitCommitGraphList.svelte
- components/git/GitCommitDetails.svelte

Refactor `shell/components/git/GraphPane.svelte` into a compact host for GitCommitGraphList.
Reuse `gitGraphLanes.ts`, the existing top-level `gitGraphViewModel.ts`, gitCommitFilesService,
and existing diff selection. Do not fork their algorithms.

GitGraphToolbar includes repository, branch, author, history search, and refresh. Changing a
filter calls one guarded graph refresh. The center workspace has:

- lane graph, refs, subject, author, date, paging, and an honest empty state;
- selected commit with full SHA, parents, task links, and copy actions;
- lazy changed-file list;
- selecting a changed file opens the existing Diff tab;
- read-only worktree context and an “Open Worktrees” action;
- no worktree deletion or hosted PR mutation.

Controller wiring:

- Add Git Graph registration to the center roster.
- Add gitGraph to the loadable-panel union.
- Change panelActivation Git activator from root-only to a copied ProjectSelection.
- shellPanels calls gitService.activateSelection.
- Add commands show-git-graph and refresh-git-graph.

### 11.3 Native DiffEditor baseline complete; exact clipboard and actions remain

Create:

- shell/git/gitDiffClipboard.ts
- scripts/gitDiffClipboard.test.mjs

Helpers:

    export function copyableDiffText(diff: SourceGitDiff): string;
    export function selectedDiffText(
      selection: Selection | null,
      container: HTMLElement
    ): string | null;
    export function copyText(text: string): Promise<void>;

Modify GitDiffView:

- Preserve `NativeGitDiffEditor.svelte` as the primary text-diff surface and the existing unified
  renderer as the binary/oversized/unavailable-model fallback. Do not return to a custom
  line-by-line renderer for ordinary text diffs.
- Configure selection/copy behavior through Monaco for the native surface. In the fallback,
  render hunk content as semantically contiguous preformatted text; line-number gutters are
  separate and user-select: none.
- Preserve exact tabs, spaces, and newline endings.
- Add toolbar/context actions: Copy selection, Copy patch, Copy relative path, and Open file.
- Copy patch uses an untouched backend patch payload, not the 2,000-line DOM view. The current
  Rust/browser combine paths call `trim_end()` and the parser normalizes CRLF; add an exact raw
  patch field or byte-preserving endpoint before claiming tab/newline fidelity.
- Disable selection copy when Selection is outside the diff container.
- Report success/failure through an aria-live status.
- Add equivalent Open/Copy relative/Copy full/Reveal/Show diff menus to changed-file and
  commit-file rows using the existing ContextMenu component.

### Tests

    cd tauri-svelte-preview
    pnpm test:git-panel-store
    pnpm test:parse-unified-diff
    pnpm test:git-graph-lanes
    pnpm test:git-graph-view-model
    pnpm test:git-bridge
    pnpm test:git-commit-files
    pnpm test:git-history-paging
    pnpm test:extension-integration
    node --experimental-strip-types scripts/gitGraphQuery.test.mjs
    node --experimental-strip-types scripts/gitDiffClipboard.test.mjs
    pnpm test:panel-activation
    pnpm check:svelte
    pnpm check

Focused Rust:

    RUST_TEST_THREADS=1 cargo test
    --manifest-path src-tauri/Cargo.toml git_graph_snapshot -- --nocapture

    RUST_TEST_THREADS=1 cargo test
    --manifest-path src-tauri/Cargo.toml \
    git_diff_models::tests::bounded_text_rejects_binary_and_oversized_models -- --exact

Test literal argument construction, leading-dash ref rejection, NUL/length bounds,
limit-plus-one, request supersession, graph continuity across pages, and exact clipboard
whitespace.

### Disposable-repository/native proof

Use mktemp -d, create multiple branches, a merge, two authors, searchable messages, renamed/
deleted/binary/large files, and remove that exact temp directory when proof completes.

Prove filters against git log, graph continuity, commit detail against git show, exact Cmd+C
copy, full Copy Patch, keyboard focus, and Dockview restoration.

### Done

- The existing narrow Source Control graph and full center graph share one model.
- Filtering and paging occur in Git, not in memory.
- Ordinary bounded text diffs render in Monaco's native DiffEditor with real `file://` models;
  binary, invalid UTF-8, oversized, deleted/unavailable, and older-backend cases degrade to an
  honest fallback instead of crashing or fabricating content.
- Diff text is selectable and copy-exact.

---

## 12. Work Package 6 — real workspace explorer and preserved worktree safety

**Covers:** TSK-763 presentation/integration plus TSK-808 file operations.

**Dependencies:** Work Packages 2 and 5; TSK-799 open-file/editor boundaries released.

**Data declaration:** No database is touched in this package. Directory filtering/sorting/paging
occurs in the bounded Rust traversal/request. The implementation dispatch still copies the global
SQL rule.

### 12.1 Separate workspace tree from source index

The current `shell/components/ExplorerPanel.svelte`:1-18 and
`shell/explorer/explorerService.ts`:1-24 explicitly use the source index. Keep that index
unchanged for CodeLens/search. The legacy route also has a separate old filesStore/source
browser; freeze it. Add a distinct `/next` workspace tree, not a third source index:

- `src-tauri/src/workspace_entries.rs`
- `src/lib/shell/explorer/workspaceEntries.ts`
- `src/lib/shell/explorer/workspaceEntryService.ts`
- `src/lib/shell/explorer/explorerMutationService.ts`
- `src/lib/shell/explorer/explorerActionMenu.ts`
- `src/lib/shell/components/explorer/ExplorerMutationDialog.svelte`
- `src/lib/shell/components/explorer/ExplorerSearchResults.svelte`
- `scripts/workspaceEntries.test.mjs`
- `scripts/explorerMutations.test.mjs`

Types:

    export interface WorkspaceEntry {
      path: string;
      relativePath: string;
      name: string;
      kind: "file" | "folder" | "symlink";
      size: number | null;
      descendantFileCount: number | null;
    }

    export interface WorkspaceTreePage {
      root: string;
      directory: string;
      entries: WorkspaceEntry[];
      nextCursor: string | null;
      scanId: string;
    }

    export interface WorkspaceMutationResult {
      kind: "create" | "move" | "duplicate" | "delete";
      oldPath: string | null;
      path: string;
    }

Add capabilities workspaceEntriesV1 and workspaceMutationsV1 plus commands:

    list_workspace_entries(
      root, directory, page_size, cursor, scan_id
    ) -> WorkspaceTreePage
    create_workspace_entry(root, parent_path, name, kind) -> WorkspaceMutationResult
    move_workspace_entry(
      root, source_path, destination_parent_path, new_name
    ) -> WorkspaceMutationResult
    duplicate_workspace_entry(
      root, source_path, destination_parent_path, new_name
    ) -> WorkspaceMutationResult
    delete_workspace_entry(root, path, expected_kind) -> WorkspaceMutationResult

List only one expanded directory per request, sort its direct children in Rust, clamp
page_size, and continue with an opaque cursor bound to root, directory, and scanId. This keeps
empty folders and non-source assets without loading a whole giant repository into Svelte.
Compute descendantFileCount once per refresh in the Rust scan, not once per render or through
per-folder follow-up calls. One Rust traversal may power both scanners internally, but
list_source_files exclusions and output remain byte-compatible.

### 12.2 Explorer actions

The context menu order is:

1. Open
2. New File
3. New Folder
4. Rename
5. Duplicate
6. Move
7. Reveal in Finder
8. Copy Path
9. Copy Relative Path
10. Open in Terminal
11. Find in Folder
12. Delete, visually separated

Find in Folder is a content search scoped to the directory. It is not the filename filter.
Use search_source_files for source content initially, with an explicit directory parameter
added under a capability if needed; never fetch the whole repository and filter in Svelte.

Editor reconciliation:

- Rename/move retargets open clean tabs from old to new path.
- Delete closes a clean tab.
- Move/delete is blocked when the affected tab has unsaved content.
- Duplicate/create opens the resulting file only after backend success.
- A failed backend operation changes no store or tab.

### 12.3 Native filesystem safety

Every Rust command revalidates independently:

- Canonical root must be a directory.
- User child paths must be relative and contain no empty component, parent traversal, NUL, or
  leaf separator.
- Existing sources and destination parents canonicalize under root.
- Symlink escape is refused; deleting a symlink deletes the link, not its target.
- Workspace root, .git, and protected infrastructure roots cannot be mutated.
- Existing destinations are never overwritten.
- Metadata is re-read immediately before mutation.
- Delete always uses AlertDialog, names exact full path, states permanence, and requires typed
  folder name for recursive folder deletion.
- Cancel issues zero backend calls.

### 12.4 Preserve one worktree safety authority

Current authoritative code:

- WorktreeManagerPane.svelte:70-229.
- WorktreeRow.svelte:84-229.
- RemoveWorktreeDialog.svelte:1-117.
- worktreeManagerService.ts:140-265.
- worktreeManagerRows.ts:514-620.

Do not add another remove/cleanup service. Preserve these rules and existing wording:

- Primary checkout is never removable.
- Normal remove refuses dirty, unmerged, or locked rows.
- Folder-gone cleanup says whether one or multiple stale entries are affected.
- Force removal is capability-gated and requires the exact folder name.
- Backend revalidates after confirmation.
- Archive is the recoverable alternative and reports its path.

Only replace undersized actions with ShellIconButton and add a read-only “Show in Git Graph”
action.

### Tests

    cd tauri-svelte-preview
    pnpm test:explorer
    pnpm test:explorer-file-icons
    node --experimental-strip-types scripts/workspaceEntries.test.mjs
    node --experimental-strip-types scripts/explorerMutations.test.mjs
    pnpm test:local-source-fs
    pnpm test:open-file-bus
    pnpm test:editor-store
    pnpm test:worktree-manager
    pnpm test:worktree-safety
    pnpm test:worktree-cleanup-plan
    pnpm test:worktree-cleanup-runbook
    pnpm check:svelte
    pnpm check

Focused Rust:

    RUST_TEST_THREADS=1 cargo test
    --manifest-path src-tauri/Cargo.toml workspace_entry -- --nocapture
    RUST_TEST_THREADS=1 cargo test
    --manifest-path src-tauri/Cargo.toml worktree -- --nocapture

Test empty directories, assets, traversal, absolute paths, symlink escape, .git, overwrite,
root deletion, cancel/no-call, dirty-tab blocking, and exact selected-row worktree effects.

### Native acceptance

Prove create, rename, duplicate, move, delete, reveal, copy paths, terminal open, directory
search, tab reconciliation, and worktree safe/force/archive flows against a disposable
repository. Each operation changes only the intended path.

### Done

- Explorer is a real workspace tree without corrupting the source index.
- Worktree safety remains a single well-tested authority.

---

## 13. Work Package 7 — hosted pull requests, reviews, checks, and API budgets

**Covers:** TSK-808 hosted GitHub capability.

**Dependencies:** Work Packages 2 and 5. Local Git stays separate.

**Data declaration:** No database is touched in this package. PR filtering, sorting, and paging
run in the GitHub API/Search/GraphQL request, not after fetching all PRs. The implementation
dispatch still copies the global SQL rule.

### 13.1 Rust hosted-GitHub module

Create src-tauri/src/github.rs. main.rs only registers state, commands, and capabilities.

Runner boundary:

    pub(crate) trait GithubCommandRunner: Send + Sync {
        fn run(
            &self,
            cwd: &Path,
            args: &[OsString],
            stdin: Option<&[u8]>,
        ) -> Result<GithubCliOutput, String>;
    }

Models:

    pub(crate) struct GithubRateBudget {
        pub resource: String,
        pub limit: u64,
        pub remaining: u64,
        pub used: u64,
        pub reset_at: String,
        pub retry_after_seconds: Option<u64>,
        pub query_cost: Option<u64>,
    }

    pub(crate) struct GithubPullRequestQuery {
        pub state: String,
        pub mode: String,
        pub project_roots: Vec<String>,
        pub project_filter: Option<String>,
        pub search: Option<String>,
        pub page_size: u16,
        pub cursor: Option<String>,
    }

    pub(crate) enum GithubActionRequest {
        CreateDraftPullRequest { base: String, title: String, body: String },
        UpdatePullRequest { number: u64, title: Option<String>, body: Option<String>, base: Option<String> },
        SubmitReview { number: u64, event: String, body: String },
        ReplyReviewThread { number: u64, thread_id: String, body: String },
        ResolveReviewThread { number: u64, thread_id: String },
        RerunFailedChecks { number: u64, run_ids: Vec<u64> },
        EnableAutoMerge { number: u64, method: String },
        MergePullRequest { number: u64, method: String },
        ClosePullRequest { number: u64 },
    }

    pub(crate) struct GithubActionConfirmation {
        pub confirmation_id: String,
        pub summary: String,
        pub repository: String,
        pub pull_request_number: Option<u64>,
        pub expected_head_sha: Option<String>,
        pub expires_at: String,
    }

Commands:

    read_github_connection(root) -> GithubConnection
    read_github_rate_limits(root) -> Vec<GithubRateBudget>
    list_github_pull_requests(root, query) -> GithubPullRequestPage
    list_github_pull_requests_across_projects(query) -> GithubPullRequestPage
    read_github_pull_request(root, number) -> GithubPullRequestDetail
    prepare_github_action(root, action, registry) -> GithubActionConfirmation
    execute_github_action(root, confirmed_action, registry) -> GithubActionResult

Capabilities:

- githubPullRequestsReadV1
- githubPullRequestsWriteV1
- githubRateBudgetsV1

Use authenticated gh. The UI never accepts or stores a token. Rust derives owner/repository
from the validated Git root and GitHub remote, uses fixed gh api or gh api graphql argument
vectors, and rejects a non-GitHub or mismatched remote. No frontend-provided endpoint,
hostname, GraphQL document, or header.

Pass titles, bodies, comments, and GraphQL variables as bounded JSON on stdin through fixed
gh api --input - operations; never place user/hosted prose in the shell or process argument
list. Bound and sanitize stdout/stderr. Return only host, account login, repository,
required-scope status, and plain remediation. Never serialize token text, environment values,
credential paths, headers, or raw auth output.

`mode` is exactly `open`, `mine`, or `needs-review`. The across-project command accepts at most 32
canonical known Git roots, resolves their GitHub owner/repo identities first, and sends one
server-side Search/GraphQL query with state, author/review-requested, project/repository, search,
sort, and cursor variables. It does not list each repository and concatenate/filter/sort in
Svelte. The response carries repository identity and maps only exact returned owner/repo pairs to
local roots; unknown repositories remain hosted-only and get no local mutation action.

### 13.2 Confirmation and rate policy

Every external mutation is two calls:

1. prepare_github_action re-reads PR, head SHA, checks, and mergeability and returns a
   human-readable summary plus random single-use short-lived confirmation ID.
2. execute_github_action verifies ID, exact action payload, repo, PR, expected/current head
   SHA, current base branch, current head branch, pull-request number, and expiry before the fixed
   mutation. The confirmation preview names repository, PR, base, head, and expected head SHA.

Never auto-retry a write. A timeout is “outcome unknown”; refresh before another offer.
Idempotent reads retry at most twice for 429, 502, 503, 504, or secondary-limit response.
Honor Retry-After; otherwise jittered 1s/2s, total wait at most 30s.

Keep REST core, Search, and GraphQL budgets separate. Warn at max(50, 10 percent of limit).
Disable only reads that use an exhausted bucket. Refresh on pane activation, explicit refresh,
or action completion; never poll.

### 13.3 Frontend files

Create:

- shell/github/githubTypes.ts
- shell/github/githubService.ts
- shell/github/githubStore.svelte.ts
- shell/github/githubAgentRecipes.ts
- components/github/PullRequestWorkspace.svelte
- components/github/PullRequestList.svelte
- components/github/PullRequestDetail.svelte
- components/github/PullRequestChecks.svelte
- components/github/PullRequestFiles.svelte
- components/github/GithubActionDialog.svelte
- scripts/githubService.test.mjs
- scripts/githubAgentRecipes.test.mjs

Interfaces:

    export interface GithubBackend {
      connection(root: string): Promise<GithubConnection | null>;
      budgets(root: string): Promise<GithubRateBudget[] | null>;
      list(
        root: string,
        query: GithubPullRequestQuery
      ): Promise<GithubPullRequestPage | null>;
      listAcrossProjects(query: GithubPullRequestQuery): Promise<GithubPullRequestPage | null>;
      detail(root: string, number: number): Promise<GithubPullRequestDetail | null>;
      prepare(
        root: string,
        action: GithubActionRequest
      ): Promise<GithubActionConfirmation | null>;
      execute(
        root: string,
        request: GithubConfirmedAction
      ): Promise<GithubActionResult | null>;
    }

    export interface GithubService {
      readonly state: GithubPanelState;
      activate(root: string | null): void;
      refresh(): Promise<void>;
      selectPullRequest(number: number): Promise<void>;
      loadMore(): Promise<void>;
      prepareAction(action: GithubActionRequest): Promise<void>;
      executePreparedAction(): Promise<void>;
    }

UI:

- List begins with Open, Mine, and Needs Review modes plus project and text filters. Filtering,
  sorting, and paging remain server-side. Rows show repository/project, reviewers, checks, merge
  state, updated time, and Start or Resume.
- Detail tabs: Conversation, Checks, Files.
- Conversation: threads, comments, resolve/reply after confirmation.
- Checks: jobs, annotations, logs/link, rerun failed after confirmation.
- Files: scalable tree, filter, collapse, viewed state, side-by-side/unified, selectable diff.
- Header: repository, base/head, branch/worktree, mergeability, close, draft/edit PR, and merge.
  Edit PR changes title/body/base only through `UpdatePullRequest` prepare/execute confirmation.
- Settings: per-action agent, CLI args, prompt recipe, and separate budget panels.

GitHub-side filter/search/sort/page occurs through REST Search or GraphQL variables. Do not
fetch every PR and filter locally.

### 13.4 Pure agent recipes

    export type GithubAgentRecipeKind =
      | "address-review"
      | "fix-checks"
      | "resolve-conflicts"
      | "review-code"
      | "draft-pr-title-body";

    export interface GithubAgentRecipe {
      id: string;
      kind: GithubAgentRecipeKind;
      title: string;
      cwd: string;
      pullRequestNumber: number;
      model: string | null;
      effort: string | null;
      approvalPolicy: string | null;
      cliArgs: string[];
      initialPrompt: string;
      untrustedContext: string;
      stopCondition: string;
    }

    export function buildGithubAgentRecipe(
      kind: GithubAgentRecipeKind,
      detail: GithubPullRequestDetail,
      cwd: string,
      settings: GithubAgentRecipeSettings
    ): GithubAgentRecipe;

Hosted text is delimited and labelled untrusted. It is never interpolated into a shell
command. Work Package 11 launches the recipe through the existing PTY/session abstraction.
Model, effort, approval policy, and fixed CLI args come from the visible per-recipe Settings
record and are validated against the selected provider; remote/repository text can never add an
argument. `review-code` prepares review findings only. `draft-pr-title-body` returns a bounded
title/body proposal into Edit PR/Create Draft preview and never publishes it automatically.

### Tests

    cd tauri-svelte-preview
    node --experimental-strip-types scripts/githubService.test.mjs
    node --experimental-strip-types scripts/githubAgentRecipes.test.mjs
    pnpm test:command-registry
    pnpm test:git-bridge
    pnpm check:svelte
    pnpm check
    RUST_TEST_THREADS=1 cargo test
    --manifest-path src-tauri/Cargo.toml github -- --nocapture

Test argument injection, canonical remote, cross-project Open/Mine/Needs Review modes with
server-side project/search/sort/cursor variables, unknown-repo local-action refusal, Edit PR,
recipe model/effort/approval/CLI-arg validation, title/body proposal no-auto-publish, redaction,
all three rate formats, budget gates,
read-only retries, no write retries, confirmation expiry/single-use/payload/repo/head mismatch,
request supersession, pagination, old-build/auth unavailable states, and recipe delimiters.

### Native acceptance

Use disposable test repositories/PRs or a read-only fixture first. Remote write proof requires
the user’s explicit confirmation at each prepared action. Prove list/detail/check/files,
cross-project modes/project filter, Edit PR plus cancelled title/body proposal, budgets, exact
local worktree mapping, confirmation naming repository/PR/base/head/SHA, stale-head refusal, and
outcome-unknown behavior.

### Done

- Hosted GitHub is separate from local Git.
- Every remote mutation has a revalidated one-time confirmation.
- Rate budgets are visible and accurate.

---

## 14. Work Package 8 — workspace-isolated native browser and annotations

**Covers:** TSK-808 browser capability and the browser portion of TSK-280.

**Critical decision:** The current BrowserPanel is a sandboxed iframe and cannot meet the
requirement. Replace it with managed, workspace-profile-isolated Tauri child webviews. The
native browser must first prove the requested Google sign-in redirect/cookie persistence flow
inside the real app. Use an explicit system-browser fallback only when the identity provider or
platform capability rejects an embedded user agent; never silently bounce the user out of the
app and never claim embedded Chrome parity without evidence.

**Dependencies:** Work Package 2. First execute the child-webview clipping spike below.

**Data declaration:** No database is touched in this package. The implementation dispatch still
copies the global SQL rule.

**Current anchors:**

- BrowserPanel.svelte:30-109 is one URL plus iframe.
- browserStore.svelte.ts:29-168 is one global record and URL.
- shellPanels.ts:107-115 activates without workspace.
- panelActivation.ts:247-260 excludes Browser from session switching.
- `/next/+page.svelte`:1035 mounts a no-props singleton.
- centerDock.ts:298-304 knows active tabs but not native overlay bounds.
- src-tauri/capabilities/default.json:1-12 grants only main core/dialog permissions; no browser
  child-webview or opener capability exists yet.
- `normalizeBrowserUrl.ts`:17-36 accepts HTTP/S userinfo today. Add explicit
  `parsedUrl.username === "" && parsedUrl.password === ""` validation in TypeScript and repeat
  the check in Rust before any navigation.

### 14.1 Mandatory native spike

Before higher-level code, prove a child WKWebView:

- clips to the Browser panel;
- never covers Dockview tabs, dialogs, annotation UI, or expanded overlay;
- tracks Dockview drag, nested scroll, window scale, resize, and teleport;
- hides immediately and stops intercepting clicks when the Browser tab is hidden.

If a preset larger than the dock cannot be clipped, require expanded mode for that preset or
add a narrowly isolated AppKit clipping container. Do not fake inner dimensions with CSS.

Stop this work package if the native view cannot meet z-order/input isolation without
replacing Dockview.

### 14.2 Native module

Create:

- src-tauri/src/browser.rs
- src-tauri/src/browser_inspector.js

Core types:

    pub struct BrowserRegistry {
        workspaces: Mutex<HashMap<String, BrowserWorkspace>>,
    }

    pub struct BrowserBounds {
        pub x: f64,
        pub y: f64,
        pub width: f64,
        pub height: f64,
    }

    pub struct BrowserTabSnapshot {
        pub workspace_id: String,
        pub tab_id: String,
        pub url: String,
        pub title: String,
        pub loading: bool,
        pub can_go_back: bool,
        pub can_go_forward: bool,
    }

Commands:

    create_browser_tab
    set_browser_tab_bounds
    show_browser_tab
    hide_browser_workspace
    navigate_browser_tab
    reload_browser_tab
    go_back_browser_tab
    go_forward_browser_tab
    close_browser_tab
    open_browser_tab_devtools
    open_browser_tab_external
    clear_browser_workspace_data
    arm_browser_element_picker
    cancel_browser_element_picker

Each accepts workspaceId and tabId where relevant. Events browser-tab-navigation,
browser-tab-load, browser-element-selected, and browser-tab-closed carry both IDs and a
monotonic generation. No arbitrary eval command.

`open_browser_tab_external` reads the current URL from the registry instead of trusting a second
frontend URL, repeats the HTTP/S/no-userinfo validation, and calls the official opener. It never
passes `file:`, `data:`, `javascript:`, custom schemes, credentials, or a stale tab URL to the OS.

Inspector payload:

    export interface BrowserElementSelection {
      url: string;
      selector: string;
      tagName: string;
      id: string | null;
      classes: string[];
      role: string | null;
      accessibleName: string | null;
      textSnippet: string;
      rect: { x: number; y: number; width: number; height: number };
    }

Cap selector at 2KB, snippets at 500 characters, classes at 32, and annotations at 100/tab.
Sanitize in Rust before emitting.

Controller manifest/wiring:

- Enable Tauri unstable child-webview support.
- Add official opener plugin for HTTP/S external open.
- Add devtools feature only after an explicit release-product decision; development proof may
  use debug devtools.
- Register BrowserRegistry and commands.
- Change default capability from windows main to webviews main.
- Give no capability and no remote.urls permission to browser-* child views.
- Add only main-webview opener HTTP/S permission.

### 14.3 Frontend model

Create:

- shell/browser/browserTypes.ts
- shell/browser/browserModel.ts
- shell/browser/browserBackend.ts
- shell/browser/browserBounds.ts
- shell/browser/browserAnnotations.ts
- shell/browser/browserPresentation.ts
- shell/overlay/actionSurfaceModel.ts
- shell/overlay/actionSurfaceStore.svelte.ts
- components/browser/BrowserTabs.svelte
- components/browser/BrowserToolbar.svelte
- components/browser/BrowserViewport.svelte
- components/browser/BrowserFeedbackPanel.svelte
- components/browser/BrowserAnnotationToolbar.svelte
- components/browser/BrowserAnnotationCard.svelte
- components/browser/BrowserOverlayHost.svelte
- components/browser/BrowserExpandedOverlay.svelte
- components/overlays/WorkbenchActionFab.svelte

Replace global state with:

    export type BrowserViewportPreset =
      | "responsive"
      | "mobile-s"
      | "mobile-m"
      | "mobile-l"
      | "tablet"
      | "laptop"
      | "laptop-l"
      | "desktop"
      | "custom";

    export interface BrowserProfileSummary {
      id: string;
      label: string;
      persistent: boolean;
      platformSupport: "supported" | "session-only" | "unsupported";
    }

    export type BrowserPresentationMode = "docked" | "expanded" | "collapsed";
    export type BrowserInteractionMode = "browse" | "picking" | "annotating";

    export interface BrowserPresentationState {
      mode: BrowserPresentationMode;
      interaction: BrowserInteractionMode;
      fanOrigin: { x: number; y: number } | null;
      pendingSelection: BrowserElementSelection | null;
      annotationDraft: string;
      annotationIntent: "change" | "question";
    }

    export interface BrowserAnnotation {
      id: string;
      workspaceId: string;
      tabId: string;
      url: string;
      selection: BrowserElementSelection;
      note: string;
      intent: "change" | "question";
      createdAt: string;
    }

    export interface BrowserTabState {
      id: string;
      title: string;
      url: string;
      inputUrl: string;
      loading: boolean;
      canGoBack: boolean;
      canGoForward: boolean;
      viewportPreset: BrowserViewportPreset;
      customViewport: { width: number; height: number } | null;
      annotations: BrowserAnnotation[];
      generation: number;
      error: string;
    }

    export interface BrowserWorkspaceSnapshot {
      activeTabId: string | null;
      tabs: BrowserTabState[];
    }

Service functions:

    activateBrowserWorkspace(workspaceId, snapshot)
    deactivateBrowserWorkspace(workspaceId)
    createBrowserTab(workspaceId, initialUrl)
    selectBrowserTab(workspaceId, tabId)
    closeBrowserTab(workspaceId, tabId)
    navigateActiveBrowserTab(value)
    setBrowserViewport(preset, custom)
    setBrowserPresentationMode(mode, fanOrigin)
    expandBrowserFrom(triggerRect)
    restoreBrowserToDock()
    collapseBrowserToControl()
    beginBrowserElementPicker()
    acceptBrowserElementSelection(selection)
    cancelBrowserAnnotation()
    queueBrowserAnnotation(selection, note, intent)
    removeBrowserAnnotation(annotationId)
    captureBrowserWorkspace(workspaceId)
    formatBrowserAnnotations(workspaceId, tabId)

BrowserPanel becomes composition only. BrowserViewport owns ResizeObserver, scroll listeners,
getBoundingClientRect, device-scale conversion, and coalesced native bounds. Hide the native
view before a teleport/overlay move and show only after final bounds.

Presets:

- responsive: host size
- mobile-s: Mobile S, 320 by 568
- mobile-m: Mobile M, 375 by 667
- mobile-l: Mobile L, 425 by 812
- tablet: 768 by 1024
- laptop: 1024 by 768
- laptop-l: 1440 by 900
- desktop: 1920 by 1080
- custom: clamp 320–2560 by 480–1600

Verify actual window.innerWidth/innerHeight inside the fixture.

`BrowserToolbar.svelte` matches the evidenced contract: profile menu, viewport menu, Import,
element picker, annotation/queue badge, tag/label action, development-only Devtools, Open in
External Browser, overflow, Expand/Restore, and Collapse. Restore exposes `Cmd+Shift+A` and the
tooltip `Restore (⌘⇧A)`. The profile menu lists Default, New Profile…, Import Cookies, and Browser
Settings…. New Profile creates an approved isolated profile. Import Cookies is disabled with a
plain-English security reason until a separate explicit product/security decision defines source,
format, secret handling, scope, and deletion; do not add a cookie importer to satisfy a screenshot.
Browser Settings opens the one Settings host when the Browser section exists.

### 14.3A Required bottom-control, full-app fan, and annotation interaction

This interaction is a product requirement from the TSK-808 rapid-fire evidence, not optional
polish. A generic Expand button in the Browser toolbar or a BrowserPanel-only floating button
does not satisfy it. The floating control is app-wide and context-driven; Browser expansion and
annotation are one branch of that global action surface.

**Global action-surface types**

    export type WorkbenchActionContextKind =
      | "session"
      | "editor"
      | "browser"
      | "resources"
      | "history";

    export type ActionSurfaceMode = "collapsed" | "fan-open" | "action-pending";

    export interface WorkbenchActionContext {
      kind: WorkbenchActionContextKind;
      centerPanelId: CenterPanelId;
      ownedId: string | null;
      workspaceId: string | null;
      targetId: string | null;
      generation: number | null;
    }

    export interface WorkbenchAction {
      id: string;
      label: string;
      icon: Component;
      contexts: WorkbenchActionContextKind[];
      shortcut: string | null;
      confirmation: "none" | "preview" | "destructive";
      enabled(context: WorkbenchActionContext): { enabled: boolean; reason: string | null };
      run(context: WorkbenchActionContext): Promise<void>;
    }

    export type ActionFanLayout =
      | { kind: "fan"; items: Array<{ id: string; x: number; y: number }> }
      | { kind: "horizontal"; items: Array<{ id: string; x: number; y: number }> }
      | { kind: "bottom-sheet"; items: string[] };

`actionSurfaceModel.ts` exports `reduceActionSurface`, `actionsForContext`, and
`layoutActionFan(viewport, anchor, itemSizes, occlusionRects)`. Geometry is derived from the
actual viewport, bottom/right safe areas, dock/composer height, and open overlays. It chooses a
fan only when every target is fully reachable, otherwise a horizontal arc or labelled bottom
sheet. Do not hard-code screenshot-specific angles or silently clip actions.

Register only existing/planned commands: Session Inspect/Open Terminal/Open Pull Request; Editor
Quick Open/Format/Source Control; Browser Expand-or-Restore/Annotate/Review Feedback/Ask Agent;
Resources Refresh/Inspect/confirm-gated Stop Owned Process; History Open Commit/Open Diff/Copy SHA.
The registry holds descriptors and routes to the existing services. It never forks the command
palette registry or implements business logic inside the FAB.

**Mounting and ownership**

1. `BrowserPanel.svelte` remains the docked composition surface. It does not own expanded state,
   a second webview, or a second annotation queue.
2. Add one `BrowserOverlayHost.svelte` and one `WorkbenchActionFab.svelte` under the existing
   `ShellOverlays.svelte`, next to `SettingsHost` and `NewSessionHost`. `/routes/next/+page.svelte`
   continues to mount `ShellOverlays` once at the `.next-shell` root. The overlay root is
   `position:absolute; inset:0; isolation:isolate; pointer-events:none`; interactive descendants
   opt into pointer events. The Browser host reads the same active workspace/tab state as
   `BrowserPanel`; it never copies browser state into component-local variables.
3. `WorkbenchActionFab.svelte` is the persistent bottom-right control across Session, Editor,
   Browser, Resources, and History. Its target is at least 44 by 44 CSS pixels and sits 16 pixels
   from the safe bottom/right edge without covering the composer, edge strip, or system chrome.
   Enter, Space, or click opens the context fan; Escape or outside click closes it and restores
   focus. Its accessible name states the active context and available-action count. In Browser
   context it also announces the active page and queued annotation count.
4. `BrowserExpandedOverlay.svelte` is a fixed, root-level surface with `inset: 0` inside
   `.next-shell`. Expanded browser content covers the top bar, session rail, center Dockview, tool
   panes, and status overlays. Only the macOS window/title-bar chrome remains outside it. Settings,
   confirmation dialogs, and destructive-action previews still render above it.

**Fan transition and presentation state machine**

- `collapsed -> fan-open`: `WorkbenchActionFab` captures its bounds, computes the collision-safe
  layout, and renders labelled actions above the entire `.next-shell`, not inside a Dockview panel.
  With reduced motion it appears in one frame. Otherwise it uses opacity/transform for 160–220ms.
  Arrow keys traverse visual order; Home/End and type-ahead work in the labelled fallback.
- `fan-open -> browser action -> docked|expanded`: selecting Browser Expand calls
  `expandBrowserFrom(triggerRect)`. Capture the global control center in shell coordinates, hide
  the native child view, mount the root overlay,
  measure its final viewport, then show the same native tab in the final bounds. Animate only the
  HTML chrome from the captured bottom origin using opacity plus transform/clip-path for 160–220ms;
  the browser visually fans from the bottom control across the entire app instead of appearing as
  an unrelated modal. With reduced motion, switch in one frame with no scale animation.
- `expanded -> docked`: Restore hides the child view, removes the expanded bounds, measures the
  docked `BrowserViewport`, shows the same tab there, and returns focus to the bottom control. URL,
  history, scroll, forms, cookies, selected tab, viewport preset, and annotations do not reload.
- `docked|expanded -> collapsed`: Browser Collapse hides the child view and browser chrome but the
  global FAB remains in the current app context. It preserves the workspace profile, tabs,
  navigation history, viewport, and queued feedback. Re-selecting Browser Expand restores the
  prior non-collapsed mode; a long idle period must not silently discard state.
- A session/workspace switch first hides the outgoing child view, cancels any armed picker, captures
  the outgoing snapshot, activates the incoming workspace, and shows only its active tab. Expanded
  mode is coerced to docked during cold application restore so the app never launches behind an
  unexpected full-screen browser; collapsed/docked state may restore per owned workspace.
- Escape is deterministic: cancel element picking first, then close an unsaved annotation card,
  then the feedback review, then restore expanded browser to docked, then close the action fan.
  It never closes a tab or discards queued annotations.

Implement browser transitions in `browserPresentation.ts` and global transitions/geometry in
`actionSurfaceModel.ts`; `browserModel.ts` invokes the browser transitions, and overlay components
only render resulting state. Unit tests enumerate every transition/context/layout and prove no
path creates a second browser tab, loses annotations, clips fan actions, or targets a stale owner.

**Annotation flow inside the expanded browser**

1. The expanded toolbar exposes **Annotate**. Activating it calls
   `beginBrowserElementPicker()`, then `arm_browser_element_picker(workspaceId, tabId,
   generation)`. The toolbar announces “Select an element” and exposes Cancel; the page cursor and
   hover outline are owned by `browser_inspector.js`, not simulated from an iframe.
2. `browser-element-selected` must match workspace, tab, and generation. Stale events are dropped.
   A valid selection calls `acceptBrowserElementSelection(selection)`, disarms picking, freezes the
   selected outline, and opens `BrowserAnnotationCard.svelte` above the lower browser edge without
   covering the selected element when an alternate edge is available.
3. The card shows page title/URL, accessible element name, bounded selector, and bounded text
   snippet. It provides a multiline note, Change/Question intent, Add, and Cancel; `Cmd+Enter`
   invokes Add. Add is disabled until the trimmed note is nonempty. Nothing is sent to an agent
   when the element is merely selected.
4. Save calls `queueBrowserAnnotation(selection, note, intent)`, clears the draft, returns to
   browse mode, and increments the badge on the bottom control and feedback panel. The queue is
   scoped to workspace and tab. Navigating away retains the original URL on the annotation and
   marks it as belonging to an earlier page; it must not retarget selectors to the new page.
5. `BrowserFeedbackPanel.svelte` lists queued cards in creation order and exposes the evidenced
   strip copy: `N annotations ready. Select another element or copy all feedback.` It supports
   edit, remove, Cancel, Copy All, Send, and Ask Agent. Copy All writes bounded Markdown. Send
   captures workspace/ownedId/generation, activates the existing Session destination, and calls
   `setConversationDraft` for that exact authoritative owned conversation; it never presses Enter,
   writes to another PTY, or replaces an existing nonempty draft without a merge preview. Owner
   mismatch, stale generation, no active owned session, or draft conflict keeps the queue and
   shows the reason. Ask Agent attaches the same immutable facts to Work Package 11 and also never
   auto-submits.
6. The inspector may return element metadata only. It must not expose arbitrary page evaluation,
   cookies, storage, tokens, complete HTML, or cross-origin DOM content.

**Layering and input contract**

- The bottom control, toolbar, card, feedback panel, tooltips, and focus ring are HTML owned by the
  main webview and must render above the child browser view. The mandatory native spike is failed
  if WKWebView z-order prevents these controls from receiving input.
- While a dialog or confirmation is open, hide/lower the child view before mounting the dialog and
  restore it only after the dialog closes. Do not solve z-order by giving remote child views Tauri
  capabilities.
- Expanded mode traps focus only within its visible browser chrome while the web page itself can
  receive normal keyboard input. Tab/Shift+Tab cross the toolbar, page, feedback controls, and
  bottom control in a documented order. Restore/Collapse returns focus to the triggering control.
- The fan and picker must work at minimum window size, 200% display scaling, both themes, reduced
  motion, trackpad/mouse scrolling, keyboard-only input, and VoiceOver. No control may depend on
  hover alone.

Input ownership is explicit: wheel and two-finger trackpad gestures scroll the page under the
pointer; wheel over HTML toolbar/card/feedback scrolls that HTML region; Space/PageUp/PageDown/
Home/End scroll the focused page or focused HTML region; arrow keys belong to open menus/fan/tree;
the native page scrollbar remains draggable; nested remote scroll containers retain their own
wheel chain until an edge. The main webview may not globally cancel these events. Native proof
records mouse wheel, trackpad inertia, keyboard page scrolling, scrollbar drag, and a nested
scroll fixture in docked and expanded modes.

### 14.4 Session/workspace and feedback integration

Extend SessionWorkspaceSnapshot with optional browser. Old snapshots hydrate browser null.
On session switch:

1. Capture active Browser workspace.
2. Hide previous native workspace.
3. Restore selected workspace state without network reload.
4. Show only its active tab if Browser is visible.

Send formats a bounded Markdown feedback block and stages it with the existing
`setConversationDraft(ownedId, draft)` only after the owner/generation and nonempty-draft policy
pass. The user reviews and submits in the existing conversation. Copy All uses clipboard.
Switching tab or workspace cannot leak annotations. Do not add a browser-specific terminal write
API or a second conversation store.

### 14.5 Security/profile lifecycle

- Validate HTTP/S in TypeScript and Rust; reject file, data, javascript, credentials in URL,
  and custom schemes.
- Generate opaque labels; never use raw path, URL, or workspace text.
- Profile ID is UUIDv5(app namespace, stable ownedId).
- Tabs in one workspace share data; workspaces never do.
- Strict persistent macOS profile isolation uses WKWebView data-store identifiers and
  therefore requires macOS 14+. On older versions use explicit nonpersistent isolated mode or
  disable persistence with explanation; never share default cookies.
- Tab close destroys view but keeps workspace profile. App exit destroys views and preserves
  approved profile data.
- Clear data is confirmed, closes that workspace’s tabs, clears its browsing data, and leaves
  every other workspace untouched.
- Removing a session and forgetting browser data are separate choices.
- Never serialize cookies, local storage, form values, tokens, or page HTML.
- Popups become managed tabs or external HTTP/S opens.
- Google authentication uses the active workspace's isolated persistent child-webview profile
  when the provider allows embedded user agents. Prove the complete redirect chain, SameSite and
  secure-cookie behavior, storage persistence, restart persistence, and workspace A/B isolation.
  If Google or another provider explicitly rejects the embedded user agent or requires an
  unsupported passkey/deep-link capability, show that reason and offer **Continue in system
  browser** as a user-selected fallback. Returning authenticated state to the embedded profile
  requires a separately designed callback; do not pretend the external login authenticated it.

### Tests

Create browserModel, browserBounds, browserAnnotations, browserBackend, and Rust browser tests.
Extend sessionWorkspaces, panelActivation, and tauriDevConfig tests.

    cd tauri-svelte-preview
    pnpm test:normalize-browser-url
    node --experimental-strip-types scripts/browserModel.test.mjs
    node --experimental-strip-types scripts/browserBounds.test.mjs
    node --experimental-strip-types scripts/browserAnnotations.test.mjs
    node --experimental-strip-types scripts/browserBackend.test.mjs
    node --experimental-strip-types scripts/actionSurfaceModel.test.mjs
    pnpm test:session-workspaces
    pnpm test:panel-activation
    pnpm test:tauri-config
    pnpm check:svelte
    pnpm check
    RUST_TEST_THREADS=1 cargo test
    --manifest-path src-tauri/Cargo.toml browser -- --nocapture

### Native proof

Prove cold/no-network, frame-denied remote rendering, mouse-wheel/trackpad/keyboard/scrollbar/
nested scrolling, three independent tabs, back/forward/reload, the validated
`open_browser_tab_external` action, embedded Google auth persistence plus the explicit
unsupported-provider system-browser fallback, workspace A/B cookie
isolation and restart persistence, and exact presets. Record one continuous interaction showing:

1. the global bottom-right FAB opening an unclipped, context-correct fan from Session, Editor,
   Browser, Resources, and History without obscuring the current work;
2. the Browser action fanning the same live page across the entire app shell;
3. Annotate entering element-pick mode and highlighting a real fixture element;
4. the annotation card saving a Change note without sending it;
5. the queued badge and feedback panel showing the exact selection;
6. Copy All producing the bounded payload and Send staging, but not submitting, the same text in
   the exact matching owned conversation; owner mismatch and nonempty-draft conflict retain it;
7. Restore returning the live page to the dock without reload, then Collapse leaving only the
   bottom control;
8. workspace A/B switching with no cookie, tab, annotation, picker, or navigation leakage;
9. Escape, keyboard focus, VoiceOver labels, reduced motion, collision-free fan fallback, and a
   narrow/minimum-height window;
10. Settings and confirmation dialogs rendering above the browser with no click interception.

Also prove devtools policy, child-view capability isolation, and workspace-only Clear data.

Cleanup receipts:

    Browser cleanup: stopped tsk-808-browser-proof (daemon + helper tree)
    Native cleanup: stopped tsk-808-tauri-proof (Tauri + child webviews)
    External-tab cleanup: closed the exact tab opened by the proof

### Done

- No iframe remains in the /next Browser path.
- Remote child views have no Tauri command capability.
- Workspaces do not share browsing state.
- Browser feedback is exact, bounded, reviewable, and staged only to the matching owned session.
- The global bottom-right control, app-wide fan, restore/collapse states, and in-browser annotation workflow are
  present and proven in the rebuilt Tauri app; a toolbar-only Expand button is not accepted.

---

## 15. Work Package 9 — resources, provider usage, and one Roslyn lifecycle authority

**Covers:** TSK-808 resource capability and the memory/resource pressure shown in evidence refs 11, 19, 22,
and 30.

**Dependencies:** TSK-799 reconciliation complete. Instrument before changing CodeLens or LSP
lifecycle behavior.

**Data declaration:** No database is touched in this package. Process/file/provider snapshots are
bounded native reads. The implementation dispatch still copies the global SQL rule.

### 15.1 Current Roslyn facts to preserve

- csharpLanguageClient.ts:33-40 has a five-root frontend pool.
- csharpLanguageClient.ts:353-535 handles frontend LRU, per-root wrappers, and ensure
  deduplication.
- lsp.rs:1430-1593 has separate legacy and native registries plus another five-root LRU.
- lsp.rs:2564-2587 owns Roslyn BuildHost process groups but jumps to SIGKILL.
- EditorPanel.svelte:321-401 prepares native services; 433-460 avoids the legacy C# warm path.
- EditorPanel.svelte:659-678 currently remounts Monaco at native cutover.
- shellPanels.ts:34-44 and 107-112 may eagerly warm C# on workspace selection.

The gap is authority: three pools cannot prove exactly one Roslyn per canonical root.

### 15.2 One-shot OS resource snapshot

Create core/src/scanners/resources.rs:

    pub struct OsProcessRecord {
        pub pid: u32,
        pub ppid: u32,
        pub pgid: u32,
        pub user: String,
        pub cpu_percent: f32,
        pub rss_bytes: u64,
        pub elapsed_seconds: u64,
        pub command: String,
        pub listening_ports: Vec<u16>,
    }

    pub fn parse_ps_snapshot(text: &str) -> Vec<OsProcessRecord>;
    pub fn parse_lsof_listener_snapshot(
        text: &str
    ) -> BTreeMap<u32, Vec<u16>>;
    pub fn scan_process_snapshot() -> Result<Vec<OsProcessRecord>, String>;

Run one:

    ps -axo pid=,ppid=,pgid=,%cpu=,rss=,etime=,user=,command=

and one:

    lsof -nP -iTCP -sTCP:LISTEN -Fpcn

Join the two ephemeral OS snapshots once in Rust. Do not issue per-PID ps/lsof calls. This is
not a database query and creates no persistence.

### 15.3 Native resource authority

Create src-tauri/src/resources.rs:

    pub enum ProcessOwner {
        App,
        OwnedSession {
            owned_id: String,
            terminal_session_id: String,
            session_id: Option<String>,
            workspace_id: Option<String>,
            project_root: Option<String>,
            editor_id: Option<String>,
        },
        LanguageServer {
            language: String,
            root: String,
            owned_id: Option<String>,
            session_id: Option<String>,
            workspace_id: Option<String>,
            project_root: String,
            editor_id: Option<String>,
        },
        Playwright { pgid: u32 },
        External,
    }

    pub struct ResourceSnapshot {
        pub captured_at_ms: u128,
        pub processes: Vec<ResourceProcess>,
        pub totals: ResourceTotals,
        pub disk: Vec<ResourceDiskEntry>,
        pub provider_usage: Vec<ProviderUsageSnapshot>,
        pub memory_pressure: MemoryPressureLevel,
    }

    pub fn read_resource_snapshot(
        terminals: &TerminalRegistry,
        lsp: &SourceLspRegistry,
        request: ResourceSnapshotRequest,
    ) -> Result<ResourceSnapshot, String>;

Persist owned_id in TerminalSessionInfo and TerminalSessionHandle. Ownership is established by
the PTY/LSP/Playwright registries and PPID ancestry. A Roslyn-looking process without registry
proof is External and not stoppable. Every stop/restart revalidates PID, PGID, root, language,
and registry generation.

The session/workspace/project/editor fields come only from existing owned-session,
`sessionWorkspaces`, editor, terminal, and LSP registries. A missing join is `None`/Unknown, never a
path/name guess. Resource UI groups using the returned snapshot in memory only because the OS
snapshot is already bounded and is not SQL data; it performs no per-row follow-up process query.

Capabilities:

- resourceSnapshot
- resourceDiskScan
- providerUsage
- lspRootLifecycle
- lspProcessInventory

### 15.4 Conservative disk model

    pub enum DiskProtection {
        Active,
        Dirty,
        Unmerged,
        UserData,
        SafeCandidate,
        Unknown,
    }

    pub struct ResourceDiskEntry {
        pub path: String,
        pub bytes: u64,
        pub kind: DiskKind,
        pub protection: DiskProtection,
        pub reclaimable: bool,
    }

Compact snapshot never runs du. Expanded/manual refresh scans only explicit known project
worktree, build-output, and cache paths. Dirty, unmerged, active, persistent-data, and unknown
paths are never reclaimable. TSK-808 reports candidates; actual cleanup continues through
the existing approval-gated worktree/disk workflows. Never run broad Docker or filesystem
pruning.

### 15.5 Truthful provider usage

Create a ProviderUsageReader abstraction:

    pub trait ProviderUsageReader {
        fn provider(&self) -> AgentProvider;
        fn read(&self) -> ProviderUsageSnapshot;
    }

    pub enum ProviderUsageState {
        Available { windows: Vec<UsageWindow> },
        Unavailable { reason: String },
        Error { message: String },
    }

Implement Claude and Codex adapters only against documented/local stable output, behind fixture
tests. Never infer remaining usage from transcript count, session duration, or undocumented
screen text. Never expose credentials or provider-private arguments. If a provider has no
reliable source, show Unavailable with the reason.

### 15.6 Make SourceLspRegistry authoritative

Add to SourceLspRegistry:

    pub struct SourceLspProcessInfo {
        pub language: String,
        pub root: String,
        pub pid: Option<u32>,
        pub pgid: Option<u32>,
        pub state: LanguageServerState,
        pub last_used: u64,
        pub active: bool,
        pub rss_bytes: Option<u64>,
        pub crash: Option<SourceLspCrashReport>,
    }

    pub fn set_active_source_root(&self, root: Option<&str>) -> Result<(), String>;
    pub fn source_lsp_processes(&self) -> Result<Vec<SourceLspProcessInfo>, String>;
    pub fn stop_server_for_root(
        &self, language: &str, root: &str
    ) -> Result<SourceLspProcessInfo, String>;
    pub fn restart_server_for_root(
        &self, language: &str, root: &str
    ) -> Result<SourceLspProcessInfo, String>;
    pub fn apply_memory_pressure(
        &self, level: MemoryPressureLevel
    ) -> Result<Vec<SourceLspProcessInfo>, String>;
    pub fn read_server_log(
        &self, language: &str, root: &str
    ) -> Result<Vec<String>, String>;

    pub struct ResourcePolicy {
        pub max_warm_roots: usize,
        pub max_inactive_warm_roots: usize,
        pub rss_budget_bytes: u64,
        pub recent_warm_window_seconds: u64,
        pub max_parallel_starts: usize,
        pub graceful_shutdown_ms: u64,
        pub term_shutdown_ms: u64,
    }

    pub fn resource_policy(&self) -> ResourcePolicy;
    pub fn set_resource_policy(&self, policy: ResourcePolicy) -> Result<ResourcePolicy, String>;

Policy:

- Native VS Code language-client transport is the sole C# Roslyn owner.
- Legacy C# semantic calls proxy through native transport or return a capability-unavailable
  response; they never spawn a second server.
- Active canonical root is pinned. An explicit active-session start may warm exactly that
  session's one canonical C# root before the first file opens; the first C# file must reuse the
  same backend-owned process.
- Inactive/saved rail sessions start zero language servers. Merely restoring rail metadata or
  constructing hidden panels never warms Roslyn.
- Defaults are explicit: five total warm roots, four inactive warm roots, one parallel start, a
  conservative measured RSS budget, a 15-minute recent-window, and bounded graceful/TERM waits.
  Settings validation clamps safe product ranges and shows units/restart impact.
- Normal pressure keeps active and recently used roots warm only within both root-count and RSS
  budgets. A hard-coded `five` outside the default policy is a defect.
- Warning evicts oldest inactive roots until under RSS/root budget.
- Critical stops all inactive roots.
- Frontend disposal follows backend eviction events; delete its independent LRU authority.
- Shutdown sequence is LSP shutdown/exit, bounded wait, SIGTERM group, bounded wait, SIGKILL
  fallback, reap, verify BuildHosts gone.
- Keep a root-specific 200-line stderr ring.
- Bridge crash records crashed/not-running before returning.

Expose policy fields in `LanguageServerControls.svelte` through the one Settings store. Apply
changes through the registry, show the effective clamped policy, and never persist live PID/RSS.
Failure restores the previous effective policy.

### 15.7 Resource UI

Create:

- shell/resources/resourceTypes.ts
- shell/resources/resourceBackend.ts
- shell/resources/resourceStore.svelte.ts
- shell/resources/resourceViewModel.ts
- components/resources/ResourceCard.svelte
- components/resources/ResourceWorkspaceTree.svelte
- components/resources/LanguageServerControls.svelte
- components/resources/ProviderUsageCard.svelte

Compact view:

- total CPU and RSS;
- owned/external process counts;
- ports;
- warm LSP roots;
- known reclaimable bytes;
- Claude/Codex usage state.

Expanded view:

- owned-session process tree;
- language/root PID, RSS, state, log, stop/restart;
- ports and ownership;
- protected disk entries and why;
- provider usage source/unavailable reason.

No external process shows a stop action.

### 15.8 CodeLens instrumentation and stop condition

Record under MCB_TIMING=1:

- native client start/reuse/dispose by canonical root;
- Roslyn PID/PGID;
- workspace/codeLens/refresh;
- active file URI opened;
- editor remount;
- provider registration count;
- semantic reference request/cache hit;
- Peek target count;
- reference display command (`editor.action.showReferences` or `editor.action.peekLocations`),
  one-call count, open/close time, and error;
- center roster/active-panel ID and center-tab-strip height before/during/after Peek;
- open-file paths/active path, file-tab-strip height, editor element identity, and Monaco model URI
  before/during/after Peek;
- session workspace snapshot/restore call count during the interaction, which must remain zero.

Do not change count/Peek semantics unless the trace proves a defect. Acceptance is one
semantic answer shared by visible count and Peek, with no repeat lookup on Peek. C# to
TypeScript to the same C# file retains settled counts unless invalidated.

Capture one baseline artifact before lifecycle/Peek changes and one final artifact after them at
`docs/superpowers/evidence/tsk-808/resources-code-lens-{baseline,final}.json`. Each contains the
same named rows and monotonic durations:

| Flow | Start/stop markers | Required evidence |
| --- | --- | --- |
| Cold app launch | process start -> shell interactive | duration, process count, total RSS, Roslyn/BuildHost count |
| Cold first C# file | open request -> readable text -> semantic lens ready | first-text ms, placeholder count, semantic count, click-ready ms, PID/PGID |
| Warm file switch | active file change -> readable/semantic ready | duration, same-root PID reuse, no remount |
| Top editor-file tab switch | Dockview activation -> readable/semantic ready | duration, tab identity, model URI, no lookup if valid |
| Session switch | ownedId activation -> workspace/editor settled | duration, root/PID choice, no stale results |
| Workspace/project switch | root selection -> files/Git/editor settled | duration, process/RSS delta, exact canonical root |
| Page/app refresh | pagehide snapshot -> restored interactive state | duration, process/RSS delta, no duplicate Roslyn |
| CodeLens click | click -> populated Peek ready | target count, request/cache count, placeholder/semantic count, under 2 seconds |

`final` must not regress a named latency by more than the agreed fixture tolerance and must meet
the two-second CodeLens SLA. Raw before/after values, not “felt faster,” are the acceptance record.

### Tests

    cd tauri-svelte-preview
    pnpm test:owned-sessions
    pnpm test:next-terminal-service
    pnpm test:context-store
    pnpm test:panel-activation
    pnpm test:csharp-language-client
    pnpm test:source-code-lens-keys
    pnpm test:source-ui
    pnpm test:tauri-source
    pnpm check

    cd ..
    RUST_TEST_THREADS=1 cargo test
    --manifest-path core/Cargo.toml resources -- --nocapture
    RUST_TEST_THREADS=1 cargo test
    --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml lsp -- --nocapture
    RUST_TEST_THREADS=1 cargo test
    --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml resources -- --nocapture

Test one ps/lsof join, full session/workspace/project/editor identity, missing-join Unknown,
PPID ownership, stale PID refusal, external protection, disk classes,
provider states, inactive-saved-session zero Roslyn, active-session single-root warm and reuse,
native-plus-legacy one Roslyn, policy validation, RSS/recent-window behavior, sixth-root eviction,
pressure, root stop/restart isolation, crash logs, graceful cleanup, and CodeLens stability.

### Native proof

Use pgrep, ps, and lsof evidence for:

1. Cold launch restoring only saved/inactive sessions: zero Roslyn.
2. Explicitly start/select the active session: at most one canonical root warms; opening its
   first C# file reuses that exact parent rather than spawning another.
3. Warm file, top editor-file tab, session, workspace, project, and refresh flows populate the
   baseline/final table with stable identity and no duplicate process.
4. Six roots under default policy: at most five warm; active survives; evicted group/BuildHosts
   exit. Change the policy and prove the effective count/RSS/recent-window bounds follow it.
5. Warning/critical pressure: only inactive roots evict.
6. Root stop/restart does not disturb others.
7. Crash records state/log and leaves no false ready status.
8. App quit leaves no app-owned Roslyn/BuildHost descendant.

### Done

- Resource counts are bounded snapshots with proven ownership.
- Only SourceLspRegistry controls Roslyn lifecycle.
- No duplicate/root leakage/freeze or count/Peek regression.

---

## 16. Work Package 10 — safe, rich Markdown workbench

**Covers:** TSK-808 Markdown capability and supersedes only the limited renderer, not TSK-345’s completed
basic-preview history.

**Dependencies:** Work Packages 1 and 2.

**Data declaration:** No database is touched in this package. The implementation dispatch still
copies the global SQL rule.

**Current anchors:**

- top-level `src/lib/sourceMarkdownPreview.ts`:3-89 is a hand-written block parser, 107-123 a limited inline
  parser, and 125-136 escaping.
- top-level `src/lib/SourceMarkdownPreview.svelte`:7-16 props, 28-30 raw HTML insertion, and
  33-174 hard-coded styles.
- Existing tests cover only a small basic surface.
- The active `/next` `shell/components/EditorPanel.svelte` does not mount SourceMarkdownPreview;
  the old `src/lib/components/panels/EditorPanel.svelte` does.

### 16.1 Dependencies and renderer

Add in the controller manifest commit:

- markdown-it
- markdown-it-anchor
- markdown-it-footnote
- markdown-it-task-lists
- dompurify
- shiki
- mermaid

Create:

- shell/markdown/markdownTypes.ts
- shell/markdown/markdownRenderer.ts
- shell/markdown/markdownSanitizer.ts
- shell/markdown/markdownLinks.ts
- shell/markdown/markdownOutline.ts
- shell/markdown/markdownScrollSync.ts
- shell/markdown/markdownWorker.ts
- scripts/markdownRenderer.test.mjs
- scripts/markdownSanitizer.test.mjs
- scripts/markdownScrollSync.test.mjs

Renderer API:

    export interface MarkdownRenderRequest {
      markdown: string;
      filePath: string;
      projectRoot: string;
      themeId: string;
      requestId: number;
    }

    export interface MarkdownRenderResult {
      html: string;
      headings: MarkdownHeading[];
      mermaidBlocks: MermaidBlock[];
      sourceMap: MarkdownSourceBlock[];
      requestId: number;
    }

    export async function renderMarkdown(
      request: MarkdownRenderRequest
    ): Promise<MarkdownRenderResult>;

Features:

- GFM tables, tasks, strikethrough, and autolinks.
- Footnotes.
- GitHub-style note/tip/important/warning/caution alerts through a fixed container map.
- Fenced-code Shiki highlighting with a bounded language allow-list and plaintext fallback.
- Stable heading slugs/anchors and outline.
- Mermaid placeholder extraction; render Mermaid through dynamic import only when a block is
  visible.
- Local images resolved relative to the Markdown file and canonicalized under project root.
- Raw HTML disabled before sanitization.

DOMPurify uses a fixed allow-list for the generated tags/classes/attributes. It removes
script/style/iframe/object/embed/form, event handlers, srcdoc, data URLs except safe bounded
images if explicitly approved, and javascript/custom protocols. Sanitization runs after all
render transforms. Tests inject every denied vector.

### 16.2 Workspace component

Create:

- components/markdown/MarkdownWorkspace.svelte
- components/markdown/MarkdownToolbar.svelte
- components/markdown/MarkdownOutline.svelte
- components/markdown/MarkdownPreviewPane.svelte
- components/markdown/MarkdownExternalChangeDialog.svelte

Modes:

- Read
- Edit
- Split

Use the existing Monaco editor/model for Edit, not a textarea and not a second editor service.
Split uses sourceMap to sync the nearest top-visible source block in either direction without
feedback loops. Find uses Monaco in Edit and browser text selection/search in Read. All
rendered text is selectable/copyable and has the existing ContextMenu.

Large documents:

- Under 200KB render in the main worker pipeline.
- At or above 200KB render in markdownWorker, debounce edits 150ms, and virtualize Mermaid/
  code blocks outside the viewport.
- Discard a result whose requestId/file/version no longer matches.

### 16.3 File watch and conflicts

Add `src-tauri/src/workspace_file_watch.rs` as the narrow file-watch backend module, shared with
the editor if one exists after re-anchor:

    watch_workspace_file(root, path, generation) -> WatchHandle
    unwatch_workspace_file(handle) -> ()

Event workspace-file-changed carries canonical path, modified time, size, and generation; it
does not send file content. On external change:

- clean model: reread and replace;
- dirty model: show Keep mine, Reload from disk, and Compare;
- Compare opens the existing Diff tab;
- never overwrite a dirty buffer silently.

Safe links:

- HTTP/S opens via the system opener.
- Same-project file links use openFileBus.
- Anchor links stay in the preview.
- Outside-root/file/custom links are blocked with explanation.

### 16.4 Controller wiring

- Register markdown as a center panel.
- When active file is .md/.mdx, expose Open Markdown and mode commands.
- Keep Editor tab as the source editor; Markdown workspace may show the same model in split
  mode but must not create duplicate file state.
- Replace legacy SourceMarkdownPreview implementation with a compatibility wrapper around
  markdownRenderer so the old shell receives the sanitized renderer until retired.
- Remove all hard-coded SourceMarkdownPreview colors and 10px text.

### Tests

    cd tauri-svelte-preview
    pnpm test:source-markdown-preview
    node --experimental-strip-types scripts/markdownRenderer.test.mjs
    node --experimental-strip-types scripts/markdownSanitizer.test.mjs
    node --experimental-strip-types scripts/markdownScrollSync.test.mjs
    pnpm test:open-file-bus
    pnpm test:editor-store
    pnpm check:svelte
    pnpm check

Test every feature, duplicate headings, relative paths, malicious HTML/URL/SVG, stale render,
scroll-loop prevention, large docs, watcher generation, and dirty external conflict.

### Native proof

Open a fixture document containing all features, Mermaid, local images, unsafe input, a very
large section, and an external edit. Prove Read/Edit/Split, sync, copy/find/context menu,
outline/anchors, safe links, theme parity, large-doc responsiveness, file watch, and conflict
choices.

### Done

- No hand-written Markdown parser remains on the active path.
- Raw untrusted Markdown cannot execute code or escape the project root.
- Rich Markdown is legible in both themes and usable on a large document.

---

## 16A. Work Package 10A — certify the conversation workbench (TSK-809 and TSK-810)

**Covers:** TSK-809, TSK-810, the conversation-only residuals of TSK-192/315, and the one
authoritative conversation target required by browser feedback and integrated-agent proposals.

**Dependencies:** Work Packages 1–3. This package must pass before Work Package 11 exposes an Ask
Agent surface. The former standalone TSK-809/810 plan is retained as discovery evidence; this
section is the execution authority.

**Data declaration:** No database is touched in this package. The implementation dispatch still
copies the global SQL rule.

### 16A.0 Baseline, ownership, and forbidden duplication

Extend these existing authorities:

- `src/lib/shell/conversation/conversationTypes.ts`
- `conversationReducer.ts`
- `conversationStore.svelte.ts`
- `conversationService.ts`
- `components/ConversationSurface.svelte`
- `src-tauri/src/agent_conversation/{mod.rs,protocol.rs,transcript.rs,attachments.rs}`
- `sessionWorkspaces.ts` and the existing owned-session/terminal service

The merged checkpoint already has a timeline reducer, ownedId-keyed state, transcript mirror,
structured approval events, screenshot attachment vault, metadata/usage parsing, child discovery,
and provider writes through the existing PTY. Preserve them. `agent_conversation/provider.rs`
contains process-launch code that is not the authoritative running-session path; do not wire it,
start it, or use it to create a second Claude/Codex process.

Forbidden:

- a second conversation store, transcript pane, composer, provider host, PTY, child-session
  registry, or attachment directory;
- starting, resuming, interrupting, approving for, or killing a child transcript;
- treating requested model/effort/approval values as observed runtime facts;
- retaining arbitrary pasted paths or exposing files outside the managed attachment root;
- sending a browser annotation, slash command, attachment, or agent proposal to a different
  ownedId because the user switched sessions during an async operation.

### 16A.1 Packet A — versioned per-session state and stale guards

Extend, do not replace, `ConversationWorkspaceState` and its capture/restore functions:

    export interface ConversationControlState {
      observed: ConversationMetadata;
      requested: Partial<ConversationMetadata> | null;
      applying: "model" | "effort" | "approval" | null;
      error: string | null;
    }

    export interface ConversationContextTelemetry {
      usedTokens: number | null;
      contextWindow: number | null;
      remainingTokens: number | null;
      remainingPercent: number | null;
      updatedAt: string | null;
    }

    export interface ConversationWorkspaceSnapshotV2 {
      version: 2;
      ownedId: string;
      generation: number;
      mode: "terminal" | "structured";
      draft: string;
      attachmentIds: string[];
      controls: ConversationControlState;
      selectedChildId: string | null;
      transcriptScroll: Record<string, number>;
      telemetry: ConversationContextTelemetry;
      sequence: number;
    }

1. Add one migration from the existing snapshot to v2. Missing controls/attachments/telemetry map
   to explicit unknown/empty values; a snapshot never supplies fake observed metadata.
2. Key every operation by `ownedId` plus terminal/transcript `generation`. Capture the key before
   async paste, transcript read, control change, command discovery, child selection, or send; drop
   a response when either changed.
3. Persist attachment IDs/managed paths, never blob URLs. Recreate preview URLs after restore and
   revoke them on removal, session deletion, or app teardown. A missing managed file becomes a
   removable unavailable attachment and does not block the draft.
4. Store scroll separately for parent and each child transcript. Selection/scroll/draft/
   attachments/controls remain isolated between two owned sessions and survive app restart.
5. Derive remaining tokens/percentage only when both usage and a positive context window exist;
   otherwise render `Context remaining unknown`. Clamp corrupted values and never invent a model
   default.

Tests extend `agentConversationStore.test.mjs`, `conversationSessionIsolation.test.mjs`, and
`conversationWorkspaceRestore.test.mjs` with migration, two-session isolation, stale async
responses, attachment recreation/cleanup, parent/child scroll, and unknown telemetry cases.

### 16A.2 Packet B — no-box composer and authoritative controls

Modify `ConversationSurface.svelte`; do not create another composition surface.

1. Remove border, rounded surface, fill, and drop shadow from both `.composer-row` and its textarea.
   The composer is one bottom-anchored writing area over the existing fade, with no enclosing card
   and no textarea box. A focus-visible underline/caret treatment may indicate focus without
   drawing another rectangle. Attach, Send, and explicit selectors remain discrete accessible
   controls.
2. Preserve multiline Shift+Enter, Enter-to-send, IME composition, selectable/copyable transcript,
   and textarea growth. Disable Send for empty draft, child selection, stale ownedId/generation,
   or an in-flight send. Failure restores the exact draft and attachments.
3. Replace the three generic metadata buttons with labelled Model, Effort, and Approval selectors.
   Their displayed value comes from `controls.observed`; requested values display `Applying …`
   until a later transcript metadata event confirms the same value. Timeout, rejection, process
   exit, or mismatched observed value restores the observed value and keeps a readable error.
4. Implement `requestConversationControlChange(ownedId, generation, kind, value)` in
   `conversationService.ts`. It validates the provider-specific allow-list and writes the existing
   provider command through `terminalService.writeOwned` to the authoritative PTY. It never edits
   process launch flags in memory or restarts a session silently. Unsupported live mutation is
   disabled with the reason; a future-session default belongs in New Session settings, not here.
5. The provider/model/effort/approval menus are populated from recorded capabilities for that
   running provider. Unknown values remain selectable only through the provider's own picker; do
   not hard-code a claim that a model exists. All controls, draft, attachment list, and errors are
   per ownedId.

Add `scripts/conversationControls.test.mjs` for confirmed, mismatched, timeout, unsupported,
process-exit, stale-generation, per-session isolation, and one-write/no-restart behavior. Static
UI assertions fail on `.composer-row` border/background/radius/shadow or a textarea border.

### 16A.3 Packet C — attachments, slash/skill discovery, and safe message rendering

1. Keep paste/upload in `agent_conversation/attachments.rs`. Validate PNG/JPEG/WebP MIME by
   decoded bytes, cap individual and aggregate size/count, generate opaque IDs, canonicalize the
   managed destination, use owner-only permissions, and reject symlinks/path traversal. Add
   `delete_conversation_attachment` and `prune_conversation_attachments` for exact ownedId paths;
   no broad directory delete.
2. Create `conversationCommandCatalog.ts` with `ConversationCommandDescriptor` and pure merge/
   filter/rank functions. Create Rust `agent_conversation/commands.rs` command
   `list_agent_conversation_commands(owned_id, provider, project_root)` that returns the current
   built-ins plus skill names/descriptions discovered only from canonical, allow-listed global and
   project skill roots. Read only manifest/front-matter metadata, cap 500 entries and 4 KiB per
   description, return source/provider, and never execute a skill or return its body/secrets.
3. The menu opens from `/`, filters as the user types, supports arrows/Enter/Escape, exposes
   command source, and inserts the selected command into the existing draft. Re-fetch explicitly
   or when the owned project changes; no import-time IO and no Svelte-effect IO.
4. `/model` and `/permissions` call the control path above. `/skills`, `/agent`, `/apps`, and
   `/plugins` remain provider commands only when supported; selecting a discovery-only skill
   inserts its invocation text and does not auto-send.
5. Paste shows a local preview before send. Send appends the managed attachment paths exactly once
   to the same draft/PTY write and clears/revokes them only after the write succeeds. Removing a
   preview deletes only its managed file after owner validation.
6. Extend `ConversationMessage.svelte` to render bounded Markdown with the Work Package 10
   sanitizer/link policy. Transcript text stays selectable. Raw HTML, scripts, remote images,
   credential-like links, and repository/page content never become executable DOM.

Tests: extend Rust attachment/protocol fixtures; add `conversationCommandCatalog.test.mjs` and
`conversationMessageSafety.test.mjs`; prove traversal/symlink/MIME spoof/oversize/aggregate cap,
skill-root boundary, duplicate command merge, keyboard menu, no auto-send, exact one-write, cleanup,
and hostile Markdown.

### 16A.4 Packet D — recursive, parent-scoped, read-only child hierarchy

Extend `ConversationChildAgent`; do not store a flat display-only row:

    export interface ConversationChildAgent {
      childId: string;
      parentId: string;
      provider: "claude" | "codex";
      label: string;
      identity: string | null;
      relationship: "spawned" | "sidechain" | "thread" | "unknown";
      state: "queued" | "active" | "completed" | "failed" | "unknown";
      startedAtMs: number | null;
      updatedAtMs: number | null;
      depth: number;
      transcriptAvailable: boolean;
    }

1. `transcript.rs` parses Claude parent/sidechain association from the existing parent transcript
   and canonical parent-owned subagent directory. Codex parsing requires a parent event naming the
   spawned child/thread; a globally discovered transcript with a matching-looking title is not a
   child.
2. Authorize every child transcript path against the canonical parent-associated root and stored
   provider/session mapping. Reject symlink escape, `..`, absolute injected paths, cross-provider
   IDs, and a child requested under another ownedId.
3. Build the recursive tree deterministically by parentId, then state/start time/ID. Break cycles,
   cap depth and child count, label unknown metadata truthfully, and return elapsed time from
   recorded timestamps rather than a frontend timer fact.
4. Replace the flat `.agent-tree` buttons in `ConversationSurface.svelte` with
   `ConversationAgentTree.svelte`. It uses tree/treeitem semantics, expand/collapse, roving
   tabindex, arrows/Home/End, state plus elapsed text, and per-session selected child.
5. Selecting a child calls the existing read-only transcript service and swaps only the visible
   timeline/scroll. The composer and approval actions are hidden/disabled. No child row offers
   start, resume, interrupt, kill, approve, or write.
6. Preserve current parent timeline and provider metadata if child parsing fails. Show a bounded
   per-child unavailable reason; never blank the whole conversation.

Add real redacted Claude and Codex fixture files under `src-tauri/fixtures/agent_conversation/`:
parent with nested Claude sidechain; parent with Codex spawned-by/thread events; same-title
unrelated transcript; missing child file; cycle; symlink/path escape; active/completed/failed
states. Rust tests assert association and authorization. Frontend tests assert recursive display,
elapsed/unknown states, keyboard tree, session isolation, stale response rejection, and absence of
mutating child controls.

The controller adds `test:conversation-workspace-restore` to `package.json`, pointing at the
existing `scripts/conversationWorkspaceRestore.test.mjs`; the other three conversation scripts
already exist and are reused.

### 16A.5 Focused gate and native acceptance

Run serially:

    cd tauri-svelte-preview
    pnpm test:agent-conversation-protocol
    pnpm test:agent-conversation-store
    pnpm test:conversation-session-isolation
    pnpm test:conversation-workspace-restore
    node --experimental-strip-types scripts/conversationControls.test.mjs
    node --experimental-strip-types scripts/conversationCommandCatalog.test.mjs
    node --experimental-strip-types scripts/conversationMessageSafety.test.mjs
    pnpm check:svelte
    pnpm check
    RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml agent_conversation

One rebuilt-Tauri recording uses one Claude and one Codex owned session and proves:

1. Transcript selection/copy, scrolling, hostile Markdown safety, and no composer/text-area boxes.
2. Paste two screenshots, preview/remove one, restart, restore the other, send once to the same
   authoritative PTY, and verify managed-file cleanup without a duplicate runtime.
3. Dynamic slash/skill search, keyboard selection, insertion without auto-send, and exact provider
   filtering.
4. Model/effort/approval success when supported plus unsupported, mismatch, and timeout behavior;
   process-tree evidence shows no second Claude/Codex process.
5. Two sessions retain independent drafts, attachments, observed/requested controls, telemetry,
   child selection, and parent/child scroll across switches and restart.
6. Recursive Claude and Codex children show provider, identity, state, elapsed time, and relation;
   child transcripts are read-only and an unrelated same-title transcript is excluded.
7. Context remaining is computed from observed telemetry or says unknown; it never presents a
   guessed percentage.

Stop and return a decision packet if the running provider exposes no safe supported way to change
a required control, if current transcript formats cannot prove parent association, or if a shared
session/workspace seam is dirty. Do not guess, wire `provider.rs`, or make children mutable.

### 16A Done

- TSK-809 and TSK-810 close individually only after the native recording and process-tree receipt.
- The active owned session remains the only writer/runtime authority.
- Browser feedback and Work Package 11 can target the one conversation draft/service safely.

---

## 17. Work Package 11 — guarded integrated agent

**Covers:** TSK-808 guarded-agent capability and the product outcome of TSK-766 research.

**Dependencies:** Service contracts from Work Packages 3, 5, 7, 8, and 9 are stable. TSK-809
and TSK-810 conversation contracts are merged and reusable, or this package ships only the
headless context/action/audit layer and defers conversation UI wiring.

**Data declaration:** No database is touched in this package. The implementation dispatch still
copies the global SQL rule.

**Current anchors:**

- orchestration.rs:16-169 event/run/agent/step/artifact/link/draft models; 177-347 read,
  append, and reduce; 368-649 state reducers; 1092 onward tests.
- orchestrationView.ts:501 timeline, 535 attention, 555 decision queue, 627 digest, and 708
  handoff text.
- tauriSource.ts:254-349 models and 820-831 orchestration wrappers.
- commandRegistry.ts:20-43 command model and 139-166 filter/run.
- terminalService.ts:75-104 existing PTY execution boundary.
- `shell/conversation/conversationTypes.ts`, reducer/store/service,
  `shell/components/ConversationSurface.svelte`, and `/next/+page.svelte`:490-535/839-949 are
  protected TSK-809/810 WIP for per-ownedId generation, transcripts, approvals, attachments,
  model/effort controls, and child transcript rows.
- `src-tauri/src/agent_conversation/` already has a bounded generation-guarded registry and
  typed conversation events, but `ensure_agent_conversation` is intentionally unwired and the
  live app uses the existing PTY as its sole agent process owner. This package does not create
  a second provider process or conversation runtime.

### 17.1 Deterministic context snapshot

Create only the missing deterministic context/action layer:

- shell/agent/agentContext.ts
- shell/agent/agentFacts.ts
- shell/agent/agentActionRegistry.ts
- components/agent/AgentActionPreview.svelte
- scripts/agentContext.test.mjs
- scripts/agentActions.test.mjs

Use the TSK-809/810 conversation service/store for presentation and provider I/O after those
contracts land. Do not add `IntegratedAgentPane`, a parallel agent store/service, a composer,
model picker, transcript renderer, child-agent tree, or attachment flow in TSK-808.

Snapshot:

    export interface AgentWorkbenchContext {
      capturedAt: string;
      activeOwnedId: string | null;
      session: SessionFact | null;
      project: ProjectFact | null;
      worktree: WorktreeFact | null;
      git: GitFact | null;
      pullRequest: PullRequestFact | null;
      runConfigurations: RunConfigurationFact[];
      resources: ResourceFact | null;
      browserAnnotations: BrowserAnnotationFact[];
      openFiles: OpenFileFact[];
      problems: ProblemFact[];
    }

    export interface BrowserAnnotationFact {
      annotationId: string;
      workspaceId: string;
      tabId: string;
      generation: number;
      pageUrl: string;
      pageTitle: string;
      selector: string;
      accessibleName: string | null;
      textSnippet: string;
      note: string;
      intent: "change" | "question";
      sourceHash: string;
      createdAt: string;
    }

Every fact includes source, capturedAt, and staleAfter. The snapshot service reads existing
stores/services and makes no independent backend calls unless the user explicitly presses
Refresh context.

Browser facts are copied immutably from the bounded annotation queue and carry the matching
workspace/tab/generation/hash. Revalidation rejects a changed or removed annotation. Page text is
delimited untrusted input, never instruction, and Ask Agent stages the attachment into the matching
owned conversation without auto-submit.

Add bounded summary contracts without making prose authoritative:

    export type AgentSummaryKind = "old-session" | "worktree" | "pull-request" | "project";

    export interface AgentSummaryReceipt {
      id: string;
      kind: AgentSummaryKind;
      targetId: string;
      factsHash: string;
      sourceRefs: AgentFactReference[];
      summary: string;
      generatedAt: string;
      staleAfter: string;
    }

Summaries are user-triggered or generated only for an explicitly visible stale row, bounded to the
named fact snapshot, visibly marked generated, source-linked, and invalidated when `factsHash`
changes. They cover old sessions, worktree shape/safety, PR state, and project state; they never
replace status, dirty, checks, ownership, or process facts.

### 17.2 Allow-listed actions

    export type AgentActionKind =
      | "start-run-configuration"
      | "stop-run-configuration"
      | "open-file"
      | "open-worktree"
      | "prepare-pull-request"
      | "address-review"
      | "fix-checks"
      | "resolve-conflicts"
      | "prepare-worktree-cleanup"
      | "prepare-git-reorganization"
      | "prepare-squash"
      | "prepare-git-repair"
      | "generate-run-configuration";

    export type GitRepairClass =
      | "unfinished-merge"
      | "unfinished-rebase"
      | "detached-head"
      | "missing-upstream"
      | "conflicted-index"
      | "stale-lock";

    export interface AgentActionProposal {
      id: string;
      kind: AgentActionKind;
      title: string;
      facts: AgentFactReference[];
      preview: string;
      consequences: string[];
      requiresConfirmation: boolean;
      expectedState: AgentExpectedState;
      expiresAt: string;
    }

Read/navigation actions may execute directly. Starting/stopping a configured run uses existing
validated service semantics. Git cleanup, Git history rewrite, worktree removal, PR/review/
merge, push, or external mutation always prepares a proposal and delegates execution to the
existing native confirmation boundary. The agent cannot bypass typed-name, stale-head, dirty,
unmerged, or capability checks.

There is no arbitrary shell action. A user-created Run configuration remains the sole
user-authored command path.

`prepare-git-repair` requires one named `GitRepairClass`, deterministic repository facts proving
that class, an exact preview, and the normal confirmation boundary. It may propose abort/continue/
restore/set-upstream/inspect-lock operations already modeled by the Git service; it never runs a
generic `git` string, deletes a lock it cannot prove stale, resets user changes, or guesses a
branch. Add refusal fixtures for every mismatched repair class.

`generate-run-configuration` produces a proposal, not a runnable command. Create
`shell/agent/runConfigurationProposal.ts`:

    export interface RepositoryStackSnapshot {
      root: string;
      packageScripts: string[];
      dotnetProjects: string[];
      cargoManifests: string[];
      knownPorts: number[];
      existingConfigurations: RunConfigurationFact[];
      preferences: RunConfigurationPreferences;
      factsHash: string;
    }

    export interface GeneratedRunConfiguration {
      name: string;
      executable: string;
      args: string[];
      cwd: string;
      environmentKeys: string[];
      readiness: RunReadinessRule | null;
      evidence: AgentFactReference[];
    }

Deterministic scanners read bounded `package.json` scripts, solution/project filenames, Cargo
manifests, existing configurations, and user preferences. The agent may choose among those facts
but cannot invent an executable/path/port/env secret. The preview validates the current facts hash,
allows edits, and saves through the existing run-configuration service only after explicit Accept;
it never starts the configuration automatically.

### 17.3 Contextual Ask Agent through the existing conversation

The existing TSK-809/810 ConversationSurface answers questions about settings, projects,
sessions, run configurations, worktrees, current state, Git, PRs, checks, resources, browser
feedback, and Problems. TSK-808 contributes a typed context attachment and action-preview row;
the prompt contains:

- deterministic context in a delimited JSON section;
- untrusted repository/hosted/page text in separately delimited sections;
- the exact allowed action schema;
- instruction to propose, never claim an unexecuted action happened;
- one stop condition.

Contextual Ask Agent actions live in session, Git, PR, browser annotation, Problems, resource,
and Markdown menus. Each passes an immutable fact reference, not raw UI HTML.

Continue launching the configured Codex/Claude CLI through the one existing PTY/session owner
unless TSK-809/810 deliberately wires the native provider host first. Never build a shell
command from hosted/page text. Development-agent model routing is an orchestration requirement;
it does not hard-code Luna or SOL as the app's user-facing assistant or only provider.

### 17.4 Append-only audit

Extend OrchestrationEvent additively:

    pub owned_id: Option<String>,
    pub action_id: Option<String>,
    pub action_kind: Option<String>,
    pub confirmation_id: Option<String>,
    pub facts_hash: Option<String>,
    pub expected_state_hash: Option<String>,
    pub outcome: Option<String>,
    pub scope: Option<String>,
    pub tool: Option<String>,
    pub target: Option<String>,
    pub result: Option<String>,
    pub failure: Option<String>,

Append events for proposed, confirmed, started, succeeded, failed, refused, expired, and
outcome-unknown. Never rewrite a prior event. UI shows the human preview, confirmer, exact
deterministic receipt, and links to PR/commit/worktree/artifact. Do not log credentials,
cookies, full process environments, or untrusted secrets.

`scope` is the capability/lane, `tool` is the typed service/command name, `target` is the canonical
bounded identity, `result` is the redacted deterministic receipt, and `failure` is a bounded
plain-English/code pair. Old events deserialize all five as null. Replay tests prove proposal ->
confirmation -> start -> result/failure ordering without mutation or secret leakage.

### Tests

    cd tauri-svelte-preview
    node --experimental-strip-types scripts/agentContext.test.mjs
    node --experimental-strip-types scripts/agentActions.test.mjs
    pnpm test:orchestration-event
    pnpm test:orchestration-view
    pnpm test:command-registry
    pnpm test:next-terminal-service
    pnpm check:svelte
    pnpm check
    RUST_TEST_THREADS=1 cargo test
    --manifest-path src-tauri/Cargo.toml orchestration -- --nocapture

Test stale facts, prompt-injection text, non-allow-listed action refusal, every confirmation
gate, expected-state mismatch, all Git repair-class refusals, run-configuration fact/accept/save
behavior, summary invalidation/source receipts, browser annotation owner/hash checks, expiry,
append-only replay, audit-field migration/redaction, and outcome unknown.

### Native proof

- Ask factual questions and compare answer facts with visible stores.
- Start a safe run configuration.
- Prepare but cancel a worktree cleanup and PR mutation; prove zero external change.
- Confirm one disposable safe action; prove one audit receipt.
- Feed hostile review/page text; prove it remains data.
- Generate but cancel a repository-tailored run configuration, then accept one safe fixture and
  prove it was saved but not started.
- Generate old-session/worktree/PR/project summaries, mutate the fixture facts, and prove each
  becomes stale rather than continuing as truth.
- Restart app and replay the audit transcript.

### Done

- The agent can explain and prepare work without becoming an unvalidated mutation path.
- Every consequential action is confirmed, revalidated, and auditable.

---

## 18. Work Package 12 — VSIX import and compatible extension-host graduation

**Covers:** TSK-765’s import remainder and an API-first compatibility decision. The user selected
`Works now` and `Bounded adapter` for this wave. This package produces the real-Tauri comparison,
applies that policy through R0, and continues without another selection pause.

**Dependencies:** Reconcile the already-run API-first checkpoint in 18.2 immediately after Work
Package 0 once the TSK-799 Monaco singleton is stable. Optional VSIX staging and product
enablement in 18.3-18.5 wait for Work Packages 1, 4, and 10 plus a new explicit selection of an
exact package/capability need; the 2026-08-03 decision does not authorize a generic installer.

**Data declaration:** No database is touched in this package. The implementation dispatch still
copies the global SQL rule.

**Prior checkpoint state, 2026-08-03:** PR #14 merged a bounded experiment from the protected
TSK-799 work. That evidence records earlier choices. The 2026-08-04 `Works now` plus `Bounded
adapter` decision now closes the wave-level extension/API selection:

1. The prior checkpoint used declarative contributions whole when they did not require an
   activation runtime. The official Houston theme is its first proven example.
2. The prior checkpoint preferred stable Monaco/VS Code APIs over embedding feature extensions.
   Native DiffEditor and a read-only SCM projection over the existing Rust service are the
   preserved baseline candidates for the new comparison.
3. Do not embed Git Graph. Build the native graph panel in Work Package 5 over Rust history data.
4. Do not productize GitLens in this wave. A browser entry is insufficient without its expected
   view/storage/repository/auth/workbench services; a later full-workbench spike needs separate
   approval and resource measurements.
5. Do not build a generic VSIX installer merely to prove extension support. Sections 18.3-18.5
   remain blocked graduation blueprints for a later explicitly selected package/capability need.
6. Use pinned declarative default-extension packages for grammar/snippet/language metadata.
   Semantic navigation remains a separate workspace-scoped LSP concern; do not mistake syntax
   contribution packages for language servers or add one process per editor.

Implemented checkpoint files to preserve and reconcile:

- `shell/extensions/extensionCatalog.ts` — explicit curated allowlist and integration kind.
- `shell/extensions/extensionRuntime.ts` — idempotent registration against the existing singleton
  runtime before `MonacoVscodeApiWrapper.start`.
- `shell/extensions/houston/houston.json` and `LICENSE` — exact pinned declarative contribution.
- `shell/extensions/languageContributions.ts` — pinned VS Code grammar, snippet, and language
  metadata contributions for JSON, XML, HTML/CSS, JS/TS, Markdown, Rust, Go, Python, Java,
  C/C++, shell, SQL, YAML, PHP, Ruby, Swift, F#, Razor, PowerShell, Docker, INI, Make, and npm.
- `shell/extensions/svelte/*` — the official Svelte 110.3.0 grammar, language configuration, and
  snippets, registered declaratively without loading the extension's Node entrypoint.
- `shell/extensions/rustGitScmProvider.ts` — active-root read-only SCM resource groups over the
  existing `gitService`; loaded lazily so ordinary Node tests do not import VS Code CSS/services.
- `shell/editor/csharpLanguageClient.ts` — singleton extension startup and Houston ownership. It
  clears legacy workbench, TextMate, and semantic-token overrides while Houston is selected.
- `scripts/extensionIntegration.test.mjs` and package script `test:extension-integration`.
- `output/extension-integration-review.html` and `output/extension-integration-results.html`.

Verified for this checkpoint: `pnpm test:extension-integration`, `pnpm check`, `cargo check`, the
focused Git-diff bounds/binary Rust test, native TypeScript and Svelte language-server smoke
tests, `pnpm build`, and browser-preview interaction showing stable Houston, preserved center
tabs, Source Control loading, native DiffEditor rendering, and multi-color JSON/Svelte tokens.
The existing development Tauri binary compiled/launched, but macOS UI automation could not see
that development bundle; therefore native visual parity and performance remain open.

#### 18.0A Language capability matrix fixed by the checkpoint

The editor now has a broad declarative syntax baseline, but syntax and semantic intelligence are
separate capabilities. Preserve that distinction in UI status, acceptance evidence, and future
language work:

| Capability tier | Languages now covered | Authority and lifecycle |
| --- | --- | --- |
| Syntax grammar, language metadata, brackets/comments, and bundled snippets where supplied | Batch/shell, C/C++, C#, CSS, Dockerfile, F#, Go, HTML, INI, Java, JavaScript, JSON, Make, Markdown, npm, PHP, PowerShell, Python, Razor, Ruby, Rust, shellscript, SQL, Svelte, Swift, TypeScript, XML, and YAML | Pinned declarative VS Code packages at 25.1.2 plus official Svelte 110.3.0 assets, registered exactly once by the existing Monaco/VS Code service singleton |
| Semantic definitions, references, symbols, diagnostics, and workspace intelligence | C# through Roslyn; JavaScript/TypeScript through `typescript-language-server` 5.3.0; Svelte through `svelte-language-server` 0.18.3 using executable `svelteserver` | Rust-owned persistent stdio JSON-RPC registry, keyed by canonical workspace and language; one process per active workspace/language, never one per editor tab |
| Monaco standalone fallback needed during compatible-service startup | JSON and TypeScript tokenization/features | Imported once by `MonacoSourceEditor.svelte`; it must not create a second service container or override Houston after initialization |
| Future semantic languages | Rust, Go, Python, Java, C/C++, F#, Razor, PHP, Ruby, Swift, PowerShell, shell, SQL, YAML/XML schema intelligence, and others selected by product need | Add an explicit workspace-scoped server adapter and packaged executable strategy per language. A grammar package alone must never be labeled “Go to Definition” or “Find References” support |

All editor documents keep real canonical `file://` Monaco model URIs. Workspace switching must
reuse or change the correct language/root process generation, dispose stale listeners/results,
and preserve one Monaco model per file. Rapid switching across three to five workspaces remains
a native acceptance case, not an excuse to start duplicate servers.

The development dependency lookup intentionally checks the app's `node_modules/.bin` first so
the pinned TypeScript/Svelte servers win over arbitrary GUI-app PATH contents. Packaged builds
still need a reviewed sidecar or explicit executable-location strategy; local dependency
resolution is verified development behavior, not packaged-distribution proof.

### 18.1 Existing host baseline to preserve

- `shell/editor/csharpLanguageClient.ts`:1-2 already imports `vscode/localExtensionHost` and the
  default C# extension.
- `shell/editor/monacoWorkers.ts`:1-49 already maps `extensionHostWorkerMain` to the emitted
  `extensionHost.worker` URL.
- Installed `@codingame/monaco-vscode-api` 25.1.2 exposes `registerExtension`,
  `ExtensionHostKind.LocalWebWorker`, `registerFileUrl`, `whenReady`, `isEnabled`, and `dispose`.
- `getApi` is not available for LocalWebWorker. Extensions interact through their contributed
  commands/views and the VS Code-compatible services/broker, not a direct Tauri app API.

Before adding a loader, extend `scripts/csharpLanguageClient.test.mjs` to prove one worker URL,
one local extension host, and one Monaco/VS Code service singleton across editor reopen and
workspace switch. Stop if the baseline creates duplicate hosts or changes C# diagnostics,
semantic CodeLens counts, Peek rows, or the selected source.

### 18.2 API-first compatibility checkpoint — prior evidence plus resolved row policy

The bounded first action has completed for Houston, native DiffEditor, and the read-only SCM
projection. Preserve that evidence. New candidates may proceed only when R0 classifies them as
`Works now` or `Bounded adapter`. Do not build the VSIX installer, settings UI, marketplace
surface, Node host, or product broker merely because the generic matrix below is incomplete.

Existing reports are authoritative for the completed candidate decision:

- `output/extension-integration-review.html`
- `output/extension-integration-results.html`

Create the following remaining research/probe artifacts only when an unresolved API or a newly
selected package makes them necessary:

- `docs/research/tsk-808-extension-compatibility.md`
- `scripts/extensionApiCompatibility.test.mjs`
- `src/lib/shell/editor/fixtures/mcbExtensionApiProbe.ts`
- `output/tsk-808-extension-compatibility/index.html` as the rendered dense comparison; keep
  generated output out of the product bundle and apply the repository's output/ignore policy.

Probe these existing VS Code-compatible APIs inside the one installed LocalWebWorker host:

| API/capability | Exact proof | Product dependency it could replace/adapt |
| --- | --- | --- |
| `extensions.getExtension`, activation, `whenReady`, `dispose` | activate once, read ID/version, disable, reactivate without duplicate registration | extension registry/lifecycle |
| `commands.registerCommand` and `executeCommand` | register a fixture command, invoke it from shell palette, return a typed result | command palette/actions |
| `workspace.workspaceFolders` | switch owned session and prove the extension sees only the active canonical roots | session/workspace context |
| `workspace.fs` read/stat/readDirectory | read allow-listed in-root fixtures; reject outside-root and stale-workspace access | existing filesystem adapter |
| `workspace.onDidChangeWorkspaceFolders` and file events | one generation-tagged event on session/root change; no leak from prior root | workspace/file-watch service |
| `window.showInformationMessage` and progress | render through existing accessible workbench notification/progress UI | shell feedback, not a second toast stack |
| tree/view contributions | render a bounded fixture tree, focus it, invoke a row command, dispose it | right tool/center roster |
| language/grammar/theme contributions | **Houston and language baseline implemented:** retain the exact theme; pinned common-language contributions; and official Svelte grammar/config/snippets. Keep TypeScript/Svelte semantic navigation in the existing workspace LSP registry | editor and language fast paths |
| SCM source-control APIs | **Read-only baseline implemented:** retain active-root groups over existing Git state; research command forwarding, disposal, and multi-workspace behavior before mutation | Git/Source Control surface |
| terminal APIs | either adapt to the existing owned PTY service with exact ownedId or classify unsupported | run configurations/terminal |
| webview/custom editor APIs | prove z-order, CSP, resource roots, disposal, and no Tauri IPC or classify unsupported | richer extension UI |
| network/auth/secrets | determine the actual worker/CSP boundary and prove no implicit app credential exposure | security policy, honest limitation |

For each later user-supplied candidate extension ID or VSIX, inspect its manifest/package contents and
record: publisher/name/version/hash; browser versus main entrypoint; activation events;
contributions; VS Code API symbols used; Node/native/WASM/process/network dependencies; proposed
host tier; missing adapters; expected CPU/RSS/disk; and license/distribution constraints. Do not
download from a marketplace or execute a third-party package without explicit user approval.

Classify every candidate into exactly one row:

1. **Works now:** browser entrypoint and required APIs pass unchanged in real Tauri.
2. **Bounded adapter:** LocalWebWorker compatible after a named adapter over an existing service;
   list exact files/methods and security checks.
3. **Declarative only:** theme/grammar/icon/language contributions usable without activation.
4. **Elevated host required:** Node/LocalProcess/remote/native capability needed; list the exact
   blocker and do not treat it as proof that all extensions are unsupported.
5. **Rejected package:** evidenced traversal, tampering, unsafe undeclared capability, or license/
   distribution problem.

Run each remaining API probe in the rebuilt Tauri app, record command results and process inventory, and
verify C# syntax/diagnostics/semantic CodeLens/Peek before and after. The HTML report must let
the user compare candidates, APIs, missing adapters, risk, and expected value at a glance.

#### 18.2B Mandatory real-Tauri API/adapter spike and metrics

Before asking the user to select anything, prove the candidate APIs inside the rebuilt desktop
app, not browser preview:

1. Activate the internal `mcbExtensionApiProbe` once in the existing LocalWebWorker. Prove
   allow-listed commands, bounded tree/view contribution, workspace-folder switch, in-root
   `workspace.fs`, file event generation, notification/progress adapter, and disposal without a
   duplicate command/view/service registration.
2. Add a **thin terminal adapter probe** over the existing owned PTY service. It may create/show/
   write/resize/dispose only a disposable probe terminal under an exact ownedId and generation;
   it never spawns a second terminal backend, exposes arbitrary process APIs, or touches another
   session.
3. Extend the **thin SCM adapter probe** over the existing Rust Git service with read-only status,
   groups, selection, refresh, and disposal. Mutation stays disabled unless a later selected
   extension has a named need and Work Package 5's typed operations can satisfy it. It never runs
   Git or owns a repository independently.
4. Switch three to five real workspaces, close/reopen Editor, disable/reactivate the probe, and
   verify Monaco/Roslyn CodeLens/Peek counts, selected source, worker count, command registrations,
   PTY count, and Git owner remain stable.
5. Write `output/tsk-808-extension-compatibility/index.html` with this exact metric table for each
   candidate/API set:

| Metric | Baseline and candidate evidence |
| --- | --- |
| Cold app/host launch | shell interactive ms, worker activation ms, process count, Roslyn count |
| Warm app/host launch | reactivation ms, duplicate registration count, Roslyn count |
| Command latency | p50/p95 for internal command and adapter round trip |
| Workspace switch | small and large fixture response ms, stale-event count, UI responsiveness |
| CPU/RSS | idle and peak app/worker/Roslyn values from Work Package 9 snapshot |
| Disk | installed extension bytes plus generated/bundler/build-cache bytes |
| Lifecycle | disable/re-enable/dispose ms, remaining worker/listener/PTY/SCM owners |
| Editor safety | CodeLens placeholder/semantic counts, Peek ready ms, tab/source preservation |

The spike ends after writing and rendering the comparison. It does not stage a third-party VSIX,
enable a marketplace, or edit 18.3–18.5. R0 reviews all five classifications, releases only
`Works now` and `Bounded adapter`, and continues the next milestone without another user pause.

**Recorded selection, 2026-08-04:** Houston, native DiffEditor, native Git graph, the bounded
read-only SCM adapter, and other existing declarative assets remain preserved. New `Works now`
and `Bounded adapter` rows are approved. New `Declarative only`, `Elevated host required`, and
`Rejected` rows are skipped without blocking the wave. No code from 18.3 onward starts without a
new exact third-party package/hash choice. Candidate rows are classified by evidence, never
labeled `unsupported` by default.

### 18.2A Remaining research and decision register

These are the evidenced unknowns. They do not invalidate the already-working Houston,
DiffEditor, native Git graph, read-only SCM baseline, or recorded row policy:

| Question | Why it remains open | Decision/stop condition |
| --- | --- | --- |
| SCM mutations | The checkpoint projects groups only; it does not prove stage, unstage, discard, commit, or checkout through extension commands | Reuse existing `gitService` methods with active-root generation guards; never spawn Git from extension code. Require disposable-repo plus rebuilt-Tauri proof before enabling commands. |
| Workspace switching | Browser proof covered one active session; rapid switching across 3-5 roots needs disposal/stale-event evidence | Exactly one active-root SCM provider; dispose prior groups/listeners before publishing the next root. Stop on cross-workspace resources or duplicate commands. |
| Worker network/CSP | LocalWebWorker isolation does not by itself prove outbound-network denial | Measure actual worker APIs/CSP before allowing third-party executable browser code. Until then, only pinned declarative contributions and explicitly approved internal adapters run. |
| Storage, secrets, auth, terminals, webviews, custom editors | The editor-service host has not proved these broader workbench services | Probe only for a selected extension that needs one. Missing services are named adapter gaps, not permission to enable a full workbench automatically. |
| Generic VSIX install/update/uninstall | No package was selected that requires productized import lifecycle | Keep 18.3-18.5 deferred. If selected later, require hash-bound consent, traversal/size defenses, rollback, disposal, restart, and disk cleanup proof. |
| GitLens/full workbench | Browser activation alone does not deliver its valuable views or repository integration | Separate approval requires an exact service-gap inventory plus cold-start, bundle, CPU/RSS, and lifecycle measurements. It is not part of TSK-808 baseline. |
| Graph rendering library | Git/history authority and product architecture are decided; only lane visualization implementation is open | Evaluate permissively licensed, accessible rendering/layout primitives against the current lane algorithm. The library may render data but must not execute Git, own repositories, or require a Node extension host. |
| Native acceptance | Browser preview and compilation do not prove packaged Tauri behavior | Rebuild and prove theme stability, DiffEditor, SCM lifecycle, C# CodeLens/Peek, session switching, resource/process cleanup, and performance in the desktop app. |

### 18.3 Deferred graduation — inspect and stage a newly selected VSIX

Do not execute this section in the current wave: no package has been selected that needs a
generic installer. Resume only after the user selects an exact VSIX/capability and the 18.2A
research register identifies why the curated declarative/API paths are insufficient.

Create:

- `src-tauri/src/vsix_import.rs`
- `shell/themes/vsixImportTypes.ts`
- `shell/themes/vsixImportService.ts`
- `shell/editor/webExtensionRegistry.ts`
- `shell/editor/webExtensionBroker.ts`
- `components/settings/VsixImportDialog.svelte`
- `scripts/vsixImport.test.mjs`
- `scripts/webExtensionHost.test.mjs`

Add Rust commands `inspect_vsix`, `install_inspected_vsix`, `list_installed_extensions`, and
`remove_installed_extension` behind capabilities `vsixInspectV1` and
`compatibleWebExtensionsV1`. Inspection reads `extension/package.json`, all declared package
files, declarative themes/grammars/icons/languages, and an optional `browser` entrypoint.

Reject the package before staging when it has:

- traversal, absolute paths, symlink/hard-link entries, duplicate normalized paths, or a file
  escaping `extension/`;
- excessive entry count, compressed bytes, expanded bytes, single-file bytes, or nesting;
- a missing/invalid manifest, duplicate extension ID, hash mismatch, or an entrypoint absent
  from the archive;
- native binaries, a `main`/Node entrypoint without a usable browser entrypoint, WASM or native
  language-server payloads, debuggers/tasks requiring process execution, or a required
  LocalProcess/Remote host;
- a requested capability that the current typed broker cannot represent safely.

Do not reject a package merely because it contains executable browser JavaScript. Classify it:

    export type ExtensionExecutionTier =
      | "declarative"
      | "local-web-worker"
      | "unsupported-elevated";

    export interface VsixImportPreview {
      extensionId: string;
      version: string;
      displayName: string;
      executionTier: ExtensionExecutionTier;
      browserEntrypoint: string | null;
      themes: ImportedThemeSummary[];
      grammars: ImportedGrammarSummary[];
      commands: ImportedCommandSummary[];
      requestedCapabilities: ExtensionCapabilityRequest[];
      unsupportedReasons: string[];
      warnings: string[];
      contentHash: string;
    }

The dialog shows host tier, entrypoint, contributions, requested workspace/network/command/UI
access, unsupported reasons, package hash, and disk size. Installation requires explicit user
confirmation of that exact content hash. Stage normalized files under the app-support directory
by content hash with an atomic rename; never extract into the repository. Persist only extension
ID, version, hash, enabled workspaces, approved capability policy, and install time.

Declarative themes/grammars/icons remain the fast path and execute no extension code. The theme
adapter maps VS Code colors into the complete ShellTheme token set and applies contrast-safe
overrides; the grammar adapter uses the installed Monaco VS Code services.

### 18.4 Deferred graduation — enable compatible browser extensions in LocalWebWorker

Use `registerExtension(manifest, ExtensionHostKind.LocalWebWorker)` against the existing host.
For every staged package:

1. Register each staged file URL through `registerFileUrl`; never expose an arbitrary local
   filesystem URL.
2. Bind the enable operation to extension ID, content hash, workspace trust decision,
   capability policy, and registry generation.
3. Await `whenReady`; expose disabled/activating/ready/failed states; discard stale generation
   results.
4. Keep the returned registration handle and call `dispose` on disable, uninstall, workspace
   policy change, or app shutdown.
5. Prevent duplicate command/view registrations across editor reopen and workspace switch.
6. Route selected workspace FS, command, notification, terminal, SCM, and view APIs through
   typed adapters over the existing filesystem, PTY, Git, and shell services. Do not create a
   second filesystem, terminal, Git, or editor owner.
7. Give the worker no Tauri IPC capability or application secrets. Treat extension output,
   commands, URLs, and webview content as untrusted.

LocalWebWorker is not by itself a complete malicious-code sandbox: browser JavaScript can
consume CPU and may have ambient web APIs. The graduation implementation must document which
network APIs can actually be constrained by CSP/worker setup. If network cannot be reliably
constrained, show that limitation in the preview and allow only explicitly approved extension
IDs/hashes until a stronger sandbox exists. Add activation timeout, error circuit breaker,
bounded logs, crash isolation, and disable/recovery controls.

Automatic marketplace search/download is not required by TSK-808. A manually selected,
browser-compatible VSIX is supported when it passes this policy. Node/LocalProcess and remote
extension hosts remain a separately approved elevated tier rather than a blanket rejection.

### 18.5 Deferred generic executable-extension probe

Create `shell/editor/fixtures/mcbWebExtensionProbe.ts` with this manifest:

    {
      "name": "mcb-web-extension-probe",
      "publisher": "mac-command-bar",
      "version": "0.0.1",
      "engines": { "vscode": "*" },
      "browser": "./dist/web.js",
      "activationEvents": ["onCommand:mcb.webExtensionProbe.ping"],
      "contributes": {
        "commands": [{
          "command": "mcb.webExtensionProbe.ping",
          "title": "Mac Command Bar: Web extension probe"
        }]
      }
    }

`activate` registers the command and returns extension ID plus workspace-folder count. Register
the fixture before `MonacoVscodeApiWrapper.start` in csharpLanguageClient.ts after re-anchoring;
prove its command/view contribution uses the same host that serves the C# extension.

### Tests

Current API-first gate:

    cd tauri-svelte-preview
    node --experimental-strip-types scripts/extensionApiCompatibility.test.mjs
    pnpm test:extension-integration
    pnpm test:csharp-language-client
    pnpm test:source-code-lens-keys
    pnpm test:source-ui
    pnpm check:svelte
    pnpm check

Only after an explicit VSIX/package selection resumes 18.3–18.5:

    node --experimental-strip-types scripts/vsixImport.test.mjs
    node --experimental-strip-types scripts/webExtensionHost.test.mjs

### Conditional graduation gate — blocked until explicit selection

In real Tauri:

- fixture is enabled and whenReady resolves;
- command activates inside LocalWebWorker and returns expected structure;
- no Node/remote/second-backend process exists;
- a compatible user-selected VSIX stages by exact hash, previews permissions, enables, disables,
  survives restart, and uninstalls without leaving files or registrations;
- traversal, symlink, zip-bomb, hash mismatch, unsupported host, stale generation, activation
  timeout, and capability-escalation fixtures fail closed with plain explanations;
- Monaco edit, C# syntax, diagnostics, CodeLens/Peek, workspace switch, file reads, Git, and
  PTY remain correct;
- editor close/reopen creates no duplicate worker or command;
- worker/extension failure degrades only that extension and exposes disable/retry;
- before/after cold-start, CPU/RSS/disk, and process cleanup are recorded.

Stop and request a separate elevated-host decision if a desired extension requires LocalProcess,
Node builtins, a remote host, native binaries, unbrokered filesystem/process access, Monaco
singleton replacement, or a new LSP owner. Do not misreport such an extension as globally
unsupported; report the missing host/capability tier precisely.

### Done

- The current API-first stage is done when the real-Tauri probe, adapter receipts, complete metrics,
  rendered HTML comparison, and M0/R0 classification review pass. The controller then releases
  Works now/Bounded adapter and continues without a user pause.
- The currently proven official Houston declarative contribution imports through the fast path exactly
  once, remains stable, and no legacy customizations repaint it.
- Native DiffEditor and the read-only SCM projection reuse the existing Monaco and Rust Git
  authorities without adding a second process/backend.
- If generic VSIX graduation is explicitly resumed, the selected compatible VSIX browser
  extension runs through the existing LocalWebWorker host
  with content-hash consent, workspace trust, typed capability policy, lifecycle cleanup, and
  honest sandbox limitations.
- Unsupported Node/remote/native host classes remain deferred, not falsely rejected as proof
  that extensions in general cannot be used.
- Git Graph is delivered through Work Package 5's native Rust-data/Svelte-view path, not by
  embedding an incompatible extension; GitLens remains outside the baseline unless separately
  approved after full-workbench cost research.

---

## 19. Work Package 13 — later database console, not an in-memory data grid

**Covers:** TSK-769. Notion explicitly labels this nice-to-have/later and says the run
configuration/runtime context must land first.

**Dependencies:** Work Packages 2 and 4. Do not start before the user promotes TSK-769.

### Chosen lightweight scope

Ship a database console over the existing PTY first. Do not add sqlx/database drivers or a
custom result grid in this wave.

Create:

- shell/database/databaseProfiles.ts
- shell/database/databaseConsoleService.ts
- components/database/DatabaseConsolePane.svelte
- components/database/DatabaseProfileDialog.svelte
- scripts/databaseProfiles.test.mjs

Profile:

    export type DatabaseClient = "psql" | "sqlcmd" | "sqlite3";

    export interface DatabaseConsoleProfile {
      id: string;
      projectRoot: string;
      label: string;
      client: DatabaseClient;
      cwd: string;
      arguments: string[];
      environmentKeys: string[];
    }

Do not persist secret values. environmentKeys name variables already supplied by the run
configuration/launch environment; the UI never reads or displays their values. Prefer native
credential mechanisms such as .pgpass or platform keychain-supported client auth.

databaseConsoleService:

    export function profilesForProject(
      projectRoot: string,
      runs: readonly StackDefinition[]
    ): DatabaseConsoleProfile[];

    export async function startDatabaseConsole(
      profile: DatabaseConsoleProfile
    ): Promise<string | null>;

    export function databaseStarterCommands(
      client: DatabaseClient
    ): DatabaseStarterCommand[];

Starter actions send client-native metadata commands to the selected PTY:

- psql: list schemas/tables, describe selected table, expanded rows, timing.
- sqlcmd: list schemas/tables and describe selected table with fixed catalog SQL templates.
- sqlite3: .tables, .schema, .headers on, .mode box.

User-entered queries remain in the interactive PTY. The app does not parse or page result sets.
Register Database as a right-side tool tab, not a permanent center panel.

### Hard SQL contract for any later grid

If a future task adds a structured table/result grid:

- all SQL aggregation, grouping, filtering, joins, sorting, and paging run server-side (DB-side)
  as an ORM/EF-translated query that becomes ONE SQL statement, or a database view;
- never materialize rows and then `GroupBy`, `Sum`, `Count`, `Where`, join, sort, or page in
  TypeScript, Rust, C#, or any other application code; a correct in-memory value is still a
  defect;
- never use load-then-loop, N+1, per-row schema/value follow-up queries, lazy loading, or client-
  side evaluation; a roughly 20-row list page uses 1-3 DB queries total, not dozens;
- use a bounded DB-side cursor/keyset page or `OFFSET` when appropriate to the database;
- inspect the generated SQL and prove the ORM expression translates completely; if it would run
  in memory, rewrite it DB-side or add a database view plus a covering index;
- when one violation is found or fixed, sweep the codebase for sibling materialize-then-shape,
  N+1, lazy-loading, and per-row query defects.

That future driver/grid expansion requires a separate approved plan because it changes
credential handling, SQL safety, query cancellation, and data exfiltration risk.

### Tests and proof

    cd tauri-svelte-preview
    node --experimental-strip-types scripts/databaseProfiles.test.mjs
    pnpm test:stack-store
    pnpm test:next-terminal-service
    pnpm check:svelte
    pnpm check

Native proof uses disposable local SQLite/Postgres/SQL Server fixtures as available, proves no
secret persistence, exact client process ownership, metadata commands, query interaction, and
terminal cleanup.

### Done

- While unpromoted: TSK-769 remains open/later, no database files, UI, dependencies, profiles, or
  console commands are added, and Work Package 14 does not require its native proof.
- If the user explicitly promotes it: TSK-769 closes only after the lightweight console/native
  proof above, with no application-side row aggregation/filtering/paging.

---

## 20. Work Package 14 — controller integration, certification, PR, and task closure

**Purpose:** Join the independently implemented lanes once, run one serialized build, prove
the native app, and close only tasks whose acceptance evidence is complete.

**Owner:** Controller. Straightforward, fully specified fixes route to explicit Luna Max; fixes
with unresolved choices or design judgment route to explicit SOL-medium. Every milestone code
review uses explicit SOL-medium. Read-only proof roles may use their installed fixed role models
when they are not the milestone code reviewer and do not implement.

**Data declaration:** Integration must not introduce a database path. If TSK-769 is later promoted,
its separate conditional packet supplies the SQL rule and proof; otherwise the controller receipt
states `No database touched`. Every lane dispatch/review still copies the global SQL rule.

This may remain one TSK-808 branch, but it is not one unreviewable change. Each work package has
its own commit, focused acceptance receipt, native-proof status, and rollback boundary. Do not
begin a package whose dependency receipt is failing. A capability may be split into its own PR
when reviewability or rollback safety requires it, while keeping `tsk-808` in every branch/PR
key. TSK-808 closes only after every non-deferred package passes; a partial merge updates the
disposition ledger and leaves the epic open.

### 20.1 Integrate Rust modules in one pass

After Work Package 0 re-anchors the current `main.rs` (8,171 lines in the older checkpoint),
add these modules at its module declaration seam:

    mod browser;
    mod github;
    mod resources;
    mod workspace_entries;
    mod workspace_file_watch;

Add `mod vsix_import;` only if the user explicitly selects a package that resumes Work Package
12 sections 18.3–18.5. The API-first internal probe does not require a generic installer module.

At the builder state section around main.rs:5146-5151:

    .manage(browser::BrowserRegistry::default())
    .manage(github::GithubConfirmationRegistry::default())
    .manage(resources::ResourceCoordinator::default())
    .manage(workspace_file_watch::WorkspaceFileWatchRegistry::default())

At read_backend_capabilities around main.rs:1218-1270 append the exact capability names from
the work packages:

- gitGraphSnapshotV1
- workspaceEntriesV1
- workspaceMutationsV1
- githubPullRequestsReadV1
- githubPullRequestsWriteV1
- githubRateBudgetsV1
- browserWorkspaceTabsV1
- browserElementPickerV1
- resourceSnapshot
- resourceDiskScan
- providerUsage
- lspRootLifecycle
- lspProcessInventory
- compatibleWebExtensionsV1 for the approved internal API probe;
- vsixInspectV1 only after explicit VSIX graduation;
- workspaceFileWatchV1
- notificationV1

At invoke_handler around main.rs:5162-5234 register every named command exactly once. Add a
test that compares the capability list with wrapper/command registration for all new
capabilities. A capability with no command or a wrapper with no capability fails.

Main.rs remains wiring. Validation/query/registry logic stays in its module.

### 20.2 Integrate frontend adapters in one pass

In tauriSource.ts add thin wrappers only. Put domain models in their domain files rather than
making tauriSource a second type registry. Every wrapper:

- returns null when not in Tauri;
- uses the exact Rust payload field names;
- has a focused payload-shape test;
- does no retry, state mutation, or error remapping.

In backendCapabilities.ts retain the one-read cache. Services test the named capability before
calling a new wrapper.

In settingsStore:

- merge old persisted records with defaults;
- add notifications, GitHub recipe defaults, browser profile behavior, resource display, and
  selected-extension policies through versioned nested keys;
- add database profiles only when the user explicitly promotes TSK-769; otherwise add no DB
  settings key, wrapper, handler, script, or panel;
- never persist tokens, cookies, passwords, full environments, or confirmation IDs.

In SettingsDialog:

- Notifications section.
- Source Control and Hosted Review section with per-action recipes and three budget cards.
- Browser section for profile/data/devtools policy.
- Resources section for display and LSP root budget.
- Extensions and Themes section shows the rendered API comparison and the selected compatible
  API/declarative paths. VSIX preview/import controls appear only after explicit graduation.
- Database Console section only when TSK-769 is promoted.

All section metadata meets Work Package 1 contrast.

### 20.3 Integrate /next once

2026-08-03 dirty-snapshot `/next` anchors; Work Package 0 must refresh them before editing:

- imports: lines 25-107;
- shell/service controls: lines 156-219;
- workspace capture/restore: 377-473;
- session/run lifecycle: 477-813;
- snippets around Sessions 971, Session 1006, and Browser 1035;
- top bar around 1046;
- ShellFrame map around 1051-1060.

Controller changes:

1. Import new center/tool components and domain services.
2. Build CenterPanelRegistration[] for Session, Editor, Browser, Diff, Git Graph, Pull
   Requests, and Markdown.
3. Mount TopBarDestinationSwitch and preserve the existing RunButton.
4. Capture/restore BrowserWorkspaceSnapshot inside existing SessionWorkspaceSnapshot.
5. Pass the complete ProjectSelection to panel activation.
6. Register notification policy on the existing natural-exit callback, not a second listener.
7. Register integrated-agent action/context handlers through the existing TSK-809/810
   conversation and terminal services. Register database console/run handlers only when the
   user explicitly promotes TSK-769; otherwise add none.
8. Route Open/Ask Agent/PR/Git/Markdown actions through existing openFileBus,
   commandRegistry, and ShellFrame controls.
9. Preserve the one `ShellOverlays -> SettingsHost -> SettingsDialog` chain certified in Work
   Package 2. Later settings sections extend that dialog and canonical store; they do not replace,
   remount, or bypass it. Re-run the settings reachability/persistence tests after adding every
   settings section.
10. Keep src/routes/+page.svelte, the legacy shell, frozen unless a compatibility test proves
   a shared renderer wrapper must change it. Do not port new work into the old route.
11. Mount one `WorkbenchActionFab` and `BrowserOverlayHost` through the existing `ShellOverlays`;
    provide context/command receipts rather than embedding business logic in `/next`.
12. Mount the certified TSK-809/810 `ConversationSurface` once and route Browser/Agent draft
    staging to its existing ownedId-keyed store; no second provider or composer.

### 20.4 Package scripts

Add focused scripts:

- test:contrast-tokens
- test:shell-primitives
- test:settings-store
- test:settings-reachability
- test:settings-application
- test:session-notifications
- test:editor-first-frame
- test:code-lens-peek-layout
- test:run-configurations
- test:build-output-parser
- test:git-graph-query
- test:git-diff-clipboard
- test:workspace-entries
- test:explorer-mutations
- test:github-service
- test:github-agent-recipes
- test:browser-model
- test:browser-bounds
- test:browser-annotations
- test:browser-backend
- test:action-surface-model
- test:agent-conversation-protocol
- test:agent-conversation-store
- test:conversation-session-isolation
- test:conversation-workspace-restore
- test:conversation-controls
- test:conversation-command-catalog
- test:conversation-message-safety
- test:resources
- test:markdown-renderer
- test:markdown-sanitizer
- test:markdown-scroll-sync
- test:agent-context
- test:agent-actions
- test:extension-api-compatibility
- test:extension-integration
- test:vsix-import and test:web-extension-host only after explicit VSIX graduation
- test:database-profiles, only when TSK-769 is promoted

Do not hide failures behind an always-successful aggregator. If adding test:tsk-808, implement
it as a sequential child-process runner that returns the first nonzero exit code and prints
the failing script name.

### 20.5 Serialized automated gate

Run focused lane tests first, then:

    cd /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-native-workbench/tauri-svelte-preview
    pnpm check:svelte
    pnpm check
    pnpm build
    RUST_TEST_THREADS=1 cargo test --manifest-path src-tauri/Cargo.toml
    cargo build --manifest-path src-tauri/Cargo.toml

Then:

    cd /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-native-workbench
    RUST_TEST_THREADS=1 cargo test --manifest-path core/Cargo.toml
    git diff --check
    dotnet build-server shutdown

No other heavy build runs during this gate.

### 20.6 Security review gate

Use one read-only security review after the full diff, focused on:

- child-webview capabilities and URL/profile isolation;
- browser inspector payload and no arbitrary eval;
- Git/GitHub literal argument vectors and credential redaction;
- one-time confirmation replay/stale-state behavior;
- workspace path traversal/symlink/overwrite/delete safety;
- process ownership and kill/restart revalidation;
- Markdown sanitization/local-path handling;
- VSIX traversal/size/hash/manifest validation, compatible browser-code staging, declared
  capability consent, worker/CSP limitations, typed broker isolation, and lifecycle cleanup;
- agent prompt-injection separation and audit redaction;
- notification and settings secret persistence.

Findings must identify the input source, canonical containment/allow-list evidence, argument
construction, and the real trust/privilege boundary. Literal argument vectors are necessary but
do not by themselves make repository, extension, hosted, or page-controlled input trusted.

### 20.7 Relevance and code review gate

Run one relevance review against this plan. Reject:

- new shell/session/editor/Git/worktree/agent abstractions duplicating existing ones;
- implementation of TSK-802 or unrelated task scope;
- extension paths outside the recorded Works now/Bounded adapter policy or an automatic
  marketplace surface not explicitly approved;
- DB grid/driver work before TSK-769 promotion;
- unapproved remote mutations;
- accidental changes to the Assembly compatibility identities in section 7A.4 or a blind
  MacCommandBar-to-Assembly replacement of repository/fixture/internal identifiers;
- changes to TSK-799 semantic count or Peek behavior without evidence;
- CodeLens Peek fixes that populate rows while the center editor tab strip, active file identity,
  or top-bar destination controls are hidden, clipped, covered, or replaced.

Then run one comprehensive milestone code review with explicit SOL-medium. Fix only concrete
in-contract issues. Route straightforward, fully specified fixes to Luna Max; route fixes that
still require meaningful choices or design judgment to SOL-medium. Use `agent_type: "default"`
when a fixed-model role would violate the required model/effort pairing.

### 20.8 Native acceptance gate

Use a rebuilt Tauri app, not a browser preview. Record cold/warm timing, process inventory,
CPU/RSS, screenshots, and a short screen recording.

The final-flow matrix is mandatory; every row records baseline/final duration, active ownedId/
workspace/project/file, process count/RSS, Roslyn/BuildHost count, screenshot/recording marker,
and pass/fail reason in the proof ledger:

| Flow | Exact interaction and invariant |
| --- | --- |
| Cold launch | Restore metadata only; no eager Browser/GitHub/Problems/resource/Roslyn work |
| Warm launch | Reopen after certified snapshot; exact layout/session/file state, no duplicate owner |
| Warm file switch | Readable text and semantic state return on the correct model/root |
| Top editor-file tab switch | Real nested Dockview tab changes without model/server duplication |
| Center destination switch | Session/Editor/Browser/Git/PR/Markdown preserve parked component state |
| Session switch | Exact owned conversation, worktree/cwd, browser, files, terminal, and controls restore |
| Workspace switch | Stale events rejected; Git/files/LSP/browser/resources bind to the new workspace only |
| Project switch | Canonical project/root changes once; local/hosted/resource facts map exactly |
| Page refresh | Snapshot/replay restores state and leaves no duplicate PTY/webview/worker/Roslyn process |

Core flow:

1. Assembly upgrade proof from Work Package 1B: public window/document/menu/runtime/artifact
   identity is Assembly while the same Settings, sessions, tabs, layout, drafts, ledger,
   permissions, and credential access remain available with no duplicate data namespace.
2. Cold launch restoring only inactive/saved sessions: no browser, GitHub, resource scan,
   Problems scan, or Roslyn. Explicit active-session start may warm exactly one canonical C#
   root, and the first C# file must reuse it.
3. Open file: readable first text, Monaco swap, syntax, format, diagnostics, CodeLens/Peek.
   The CodeLens Peek proof must include before/after screenshots and a short recording showing
   populated Peek rows while the center editor tab strip/top-bar destination controls and active
   file identity remain visible and usable.
4. Switch center tabs with top-bar buttons and shortcuts.
5. Start, resume, restart, move Working -> Done -> Settled -> Working, and switch session; exact
   workspace returns without deleting history/checkpoint/terminal receipt.
6. Run configuration starts/stops and Problems receives build output.
7. Minimize/restore sessions, tools, and dock without state loss.
8. Restore Settings and run the complete gear/palette/Cmd-or-Ctrl-comma, focus, persistence,
   reset, lazy-load failure/retry, contrast, and native-child-webview proof from Work Package 2.
9. Git graph/filter/page/detail/diff/explorer/worktree flow in disposable repo.
10. PR list/detail/check/files plus prepare/cancel and one explicitly approved fixture action.
11. Browser full proof from Work Package 8.
12. TSK-809/810 no-box composer, attachments, real controls, context telemetry, per-session state,
    and recursive read-only Claude/Codex child-transcript proof with one authoritative PTY.
13. Resources/Roslyn full proof from Work Package 9, including the baseline/final timing table.
14. Markdown full proof from Work Package 10.
15. Guarded-agent proposal/cancel/one safe confirmed action through the TSK-809/810 surface,
    including Git repair refusal, run-config proposal, summaries, browser fact hash, and audit fields.
16. Reconciled Houston + native DiffEditor + thin terminal/SCM/API-host proof and rendered
    extension comparison; apply the recorded Works now/Bounded adapter policy and do not require
    a generic VSIX installer.
17. Both themes, keyboard-only, touch/trackpad, VoiceOver, Reduced Motion, narrow window.
18. Quit and verify all task-owned Tauri, Roslyn, BuildHost, child-webview, browser, fixture,
    and helper processes exit.

### 20.9 Evidence-to-reference matrix

| Evidence refs in TSK-808 | Required proof                                                                                       |
| ------------------------ | ---------------------------------------------------------------------------------------------------- |
| 2, 12–14, 17–21        | Compact My Work, history/filter/grouping, explicit actions, branch/PR/port metadata, readable states |
| 1, 3–5, 15–16          | Native Rust-backed Git graph, Source Control grouping, native Monaco diff, real file tree/context menu, safe worktree detail |
| 6–10                    | Native browser tabs, annotation select/queue, profile/viewport, restore/shortcut                     |
| 11, 19, 22, 30           | CPU/RSS/process tree/ports/disk/provider/API budget pressure visibility                              |
| 18                       | Notification toggles, focus suppression, sound, and test notification                                |
| 23–30                   | PR queue/detail/check annotations/files/settings/recipes/rate buckets                                |
| 21                       | Markdown/editor readability and command tooltip density                                              |
| 31–32                    | Global bottom-right FAB/fan and latest Browser/annotation interaction references; Work Package 0 records the exact attachment filenames before implementation |
| User report, 2026-08-03 | Existing Settings screen is reachable, readable, persistent, modal, and not duplicated in Tauri      |
| `output/extension-integration-results.html`, 2026-08-03 | Official Houston remains stable; native Monaco DiffEditor and one-root read-only SCM projection reuse existing authorities; Git Graph/GitLens extensions are not embedded |
| `codex-clipboard-WBFL58.png`, 2026-08-03 | Populated CodeLens Peek leaves both center destination tabs and open-file tabs visible, ordered, selected, and usable in rebuilt Tauri |
| User request, 2026-08-03 | Public product identity is `Assembly`; compact tagline is `Build in parallel.`; longer approved variants remain available; compatibility identifiers and all existing user state survive the rename |

Do not copy the screenshots pixel-for-pixel. Use them as capability and information-density
evidence under the contrast contract.

### 20.10 Pull request and Notion closure

Branch: tsk-808-native-workbench-product-wave.

Before push:

    git status --short
    git diff --stat
    git diff --check
    git log -1 --oneline

Push and open a draft PR carrying TSK-808 in branch/title/body. The PR body links this plan,
the 37-task disposition ledger, test receipts, native proof, resource/process cleanup, and
remaining explicitly deferred items.

Immediately after the branch is pushed and the draft PR exists, verify the tree is clean,
remove the integration worktree, and prune. The branch and commits remain on origin. If review
later requires a fix, create a fresh task-owned worktree under the same mandated root, push
the fix, and remove that worktree immediately.

After merge:

1. Verify the GitHub-to-Notion integration changed TSK-808 to Done.
2. Verify every rapid-fire task that was actually completed is Done.
3. If integration did not close a completed task, run:

   /Users/blackcolours/.codex/skills/capture-task/close-notion-task.sh --task TSK-<number>

   The helper must print Done and verified.
4. Keep TSK-769 open if the user did not promote the later database lane.
5. Link the extension compatibility report and the user's selected paths. Do not claim
   unselected or elevated-host extensions are implemented.
6. Verify no task-owned worktree remains; do not recreate one merely to perform the closure
   checks.

### Done

- Required automated checks pass from a clean tree.
- Real-Tauri evidence covers every active acceptance row.
- Review/security findings are fixed or explicitly blocking.
- TSK-808 and completed linked tasks are verified closed.
- No task-owned worktree, browser tree, Tauri process, Roslyn/BuildHost, or fixture remains.

---

## 21. Parallel execution schedule

This schedule is retired as an independent dispatch authority. Use section 18, "Combined dependency
graph and execution schedule," in the 2026-08-04 amendment. That graph incorporates the still-valid
master packages in the required order and supplies A0-A12, A2-before-A3, A3-terminal, model routes,
R0-R6 reviews, user gates, heavy-runner limits, and controller integration.

The detailed work-package sections in this file remain scope/acceptance evidence. They do not
authorize the former B1/B2 rounds, older 10A/10B runtime split, or any parallel packet that
conflicts with an amendment lease. When a master package and an amendment packet touch the same
product authority, the amendment packet replaces that implementation lane while inheriting the
master acceptance rows.

---

## 22. Standard lane receipt

The controller maintains `docs/superpowers/evidence/tsk-808/proof-ledger.md` as execution evidence,
not a second plan. It contains one row per acceptance ID:

    Task ID | Work package/packet | Acceptance ID | Dependency state | Status
    Source Notion block/attachment | Exact test command/result | Native step/result
    Baseline metric | Final metric | Screenshot/recording path | Reviewer/model
    Cleanup receipt | Notion closure URL/state

Rows begin Pending, become Proven only from current test/native evidence, and become Blocked with
the exact failed condition. They are never marked Proven from overlap, compilation, an older
screenshot, or an agent summary. Every SOL-medium milestone review reads the ledger and records
accepted/rejected row IDs. Work Package 14 requires the ledger artifact and links it from the PR.

Every implementation agent returns:

    Lane:
    Model/effort actually used:
    Owned files changed:
    Shared-seam receipt:
    Focused tests run:
    Exact result:
    Acceptance rows proven:
    Baseline/final metrics artifact:
    Screenshot/recording artifact:
    Evidence/Notion attachment references:
    Dependency status:
    No database touched OR generated SQL inspected + sibling sweep result:
    Build/test processes stopped:
    Browser cleanup:
    Worktree cleanup: not applicable - controller owns the shared integration worktree
    Known limitation:
    Stop condition reached:

“Done” is invalid without exact test output and cleanup state. The controller verifies
git status, git worktree list, and any named browser/native process tree after every agent.

---

## 23. Global stop conditions

Stop implementation and ask for a decision when:

- the explicitly routed Luna Max or SOL-medium model/effort cannot be set for an implementation,
  fix, or milestone-review agent;
- TSK-808 is already closed or the user declines revival;
- direct/null-safe Notion status cannot prove TSK-808 is open;
- the approved `5884139` checkpoint or a reviewed equivalent is absent from the selected
  integration base;
- a lane needs an unlisted shared file while another lane owns it;
- the controller cannot attribute the shared-worktree diff to the dispatched exact path list;
- more than two heavy runners would overlap;
- Browser child-view clipping/capability isolation fails;
- strict persistent browser profiles would require silently shared cookies;
- GitHub mutation cannot be represented as prepare plus confirmed execute;
- filesystem mutation cannot prove canonical containment or dirty-buffer reconciliation;
- process ownership cannot be revalidated;
- Markdown sanitization or local-path containment is uncertain;
- CodeLens count/Peek behavior changes without a trace proving why;
- the implemented extension/DiffEditor/SCM/language checkpoint cannot be attributed and
  reconciled from commit `5884139` before broad TSK-808 work;
- a newly proposed extension/API path has not been classified or falls outside the recorded Works
  now/Bounded adapter policy;
- a selected extension needs Node, LocalProcess, remote host, native code, or broad unbrokered
  access without a separately approved elevated-host plan;
- DB work would materialize then filter/group/sort/page rows;
- native proof cannot be run or cleaned up.

---

## 24. First implementation action

Do not start implementation now. The product-source base is `main`/`origin/main` at `beebda6`,
which already contains the former `5884139` checkpoint through PR #14. The current tree is dirty
only with the three planning documents from this audit; no product implementation or task worktree
exists.

After the user explicitly authorizes the next action, refresh and re-anchor Work Package 0, create
the mandated clean TSK-808 worktree from refreshed `origin/main`, and run only the Work Package
12 API-first real-Tauri probe/comparison. Render the HTML capability/metrics report and run R0.
R0 releases Works now/Bounded adapter and skips all other new rows without pausing the wave. Do
not run 18.3–18.5 without a new exact third-party package/hash choice.

After R0, the first broad UI implementation is Work Package 1 contrast and
shared primitives. Work Package 1B then rebrands the public app and artifacts to Assembly while
preserving every compatibility identity and existing user state; Work Package 2 follows with the
shared shell contracts. Straightforward packets default to Luna Max; only unresolved
high-judgment choices and every milestone code review use SOL-medium.
