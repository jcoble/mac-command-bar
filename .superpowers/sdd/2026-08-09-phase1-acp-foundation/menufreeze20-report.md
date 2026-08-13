# Lane menufreeze20 report

Date: 2026-08-13  
Branch: `lane/menufreeze20`  
Scope: commit-menu freeze plus every menu, popover, dropdown, and native select open path under `tauri-svelte-preview/src/lib/shell`.

## Outcome

The commit-menu freeze architecture is removed. Commit and changed-file rows now open one shared menu whose target, actions, and placement anchor are immutable snapshots; the menu performs one hidden measurement and does no native invoke on open.

The systemic sweep found and fixed nine additional live-coupled or pre-paint-I/O open paths. Static/local surfaces were verified clean. One hover popover remains a scoped follow-up because both files needed to change are expressly owned by the concurrent styling lane.

## Commit context-menu freeze: root cause and fix

### Root cause

The regression came from commit `8f34db4` (`feat: right-click context menus for source control rows`). `git show 8f34db4^..8f34db4 -- .../GraphPane.svelte .../ChangesPane.svelte` shows that it put a complete Bits UI `ContextMenu.Root` inside every rendered commit row and every rendered changed-file row. On the commit path, menu content also read the live `expanded` value derived from the mutable file store. Opening one row therefore activated a floating-layer owner embedded in the keyed live graph while the same graph supplied its content state. This is the same failure class as perf report defect 3: a transient menu was coupled to a deep, mutating owner tree rather than an open-time snapshot.

Source receipts before the fix (`git show HEAD:<path>`):

- `GraphPane.svelte`: commit iteration at old line 247, per-row `ContextMenu.Root` at old line 257, and live `sourceControlCommitContextMenuItems(expanded)` content at old lines 326-327.
- `ChangesPane.svelte`: file iteration at old line 472, per-row `ContextMenu.Root` at old line 474, and live `fileContextItems(group, file)` content at old lines 560-561.
- No Tauri invoke existed in the commit-menu open handler. The defect was frontend ownership/reactivity and floating placement, not synchronous backend I/O.

Runtime reproduction was intentionally not performed because this lane forbids running the real app. The causal claim is source-confirmed and regression-tested; it is not presented as a profiler trace.

### Fix

- `components/git/GraphPane.svelte:172-177,286,454-459` snapshots the selected commit, expanded state, action list, and pointer anchor, then renders one menu outside the graph iteration.
- `components/git/ChangesPane.svelte:191-199,490,576-581` applies the same one-owner snapshot path to changed files.
- `components/git/sourceControlContextMenu.ts:83-101,178-212` owns pure clamped placement and immutable commit/file snapshot construction.
- `components/git/SourceControlContextMenu.svelte:21-39` mounts hidden, measures exactly once with `getBoundingClientRect()`, clamps, then reveals. It has no resize observer, repeated placement loop, store subscription, or invoke.
- The old per-row Bits UI context-menu mechanism was deleted; there is one path only.

Complexity after the fix is O(menu items) on open. It is independent of total commit/file-store size except for the already-rendered row that receives the browser event.

## Systemic `/next` menu-open sweep

Discovery receipt:

```text
rg -l "<(DropdownMenu|ContextMenu|Popover|Select)\\.Root|<select\\b|role=\"menu\"" \
  tauri-svelte-preview/src/lib/shell --glob '*.svelte'
```

Custom rail overlays and the slash-command menu were also traced from their triggers even though they do not use a `*.Root` component.

| Surface | Verdict | Open-path evidence |
| --- | --- | --- |
| Git commit context menu | **Fixed** | `GraphPane.svelte:172-177,286,454-459`; immutable target/items and one shared menu. |
| Git changed-file context menu | **Fixed** | `ChangesPane.svelte:191-199,490,576-581`; immutable group/file/items and one shared menu. |
| Git branch dropdown | **Fixed** | `BranchMenu.svelte:82-96,132`; local busy/change state is captured on open, cached rows paint immediately, then branch/stash invokes begin through `afterFloatingSurfacePaint`. |
| Run dropdown | **Fixed** | `RunButton.svelte:52-62,104-119,186`; rows, groups, root, loading state, and current selection are copied when the menu opens instead of following the live stack store. |
| Usage popover and stats dialog | **Fixed** | `usage/UsagePopover.svelte:17-31,47-65`; cached provider snapshots paint first, live quota/history I/O is deferred until after paint, and the compact popover no longer refreshes full history. |
| Resource popover | **Fixed** | `resources/ResourcePopover.svelte:10-15`; the panel opens first and sampling I/O begins after paint. |
| Full composer config menus | **Fixed** | `ComposerConfigMenu.svelte:81-88,98,162`; provider/config/pending state is copied on open. |
| Compact composer config menu | **Fixed** | `CompactComposerControlsMenu.svelte:22-32`; same open-time config snapshot contract. |
| Conversation slash-command menu | **Fixed** | `ConversationComposer.svelte:83-98,148-154` and `conversation/composerSlashCommands.ts:22-24`; the catalog is copied on the closed-to-open transition and filtering uses that snapshot. |
| Pull-request base-branch dropdown | **Fixed** | `PullRequestPanel.svelte:177-186,223`; choices and selected base are copied on open. |
| Session Library context menu | **Fixed** | Existing target/item snapshots remain at `SessionLibraryWorkspace.svelte:218-238`; `sessionLibrary/SessionContextMenu.svelte:31-47` now uses one hidden measurement and removed resize-driven remeasurement. |
| New-session provider/model/effort/access/worktree dropdowns | **Clean** | `newSession/NewSessionThread.svelte:255-438`; these read bounded draft-owned choices/config copied into the new-session host, with no invoke in an open callback. Prior deep provider derivation was already removed by perf defect 3. |
| Sessions view-options dropdown | **Clean** | `SessionsColumn.svelte:234-317`; bounded local view-option state only, no backend read or streaming-store derivation on open. |
| Settings selects | **Clean** | `SettingsDialog.svelte:531-692`; each select iterates a small static option list and performs no open-time I/O. |
| Workflow status select | **Clean** | `components/workflows/WorkflowRunList.svelte:98-109`; native select over the static filter options. |
| Resource-manager kind/protection selects | **Clean** | `resources/WorkspaceSpaceWorkspace.svelte:82`; native selects with five/six literal options and no open callback. |
| Worktree-agent hover popover | **Follow-up** | `WorktreeAgentRow.svelte:84-86,389-407` reads a live conversation-derived surface while the hover card is open. Fixing the ownership boundary requires `WorktreeAgentRow.svelte` and likely `SessionHoverCard.svelte`; both are explicitly excluded from this lane because a concurrent styling lane owns them. Recommended fix: capture a shallow hover-card view model on the closed-to-open transition and render only that snapshot. |

No other `/next` menu/popover/dropdown owners were returned by the structural sweep. Tooltip-only primitives were excluded: they expose labels, not mutable action/data menus.

## Shared open-path contract

- `floatingSurface.ts:4-8` supplies the double-animation-frame after-paint boundary used by surfaces that must refresh native data.
- `conversation/conversationConfig.ts:32-47`, `conversation/composerSlashCommands.ts:22-24`, and `components/git/sourceControlContextMenu.ts:178-212` provide typed copy helpers for the three snapshot shapes.
- No `any` was added.
- No Rust, backend, visual-system, protected styling-lane component, or `src/lib/components/ui/*` file was changed.
- No compatibility fallback remains beside the replacement commit/file menu.

## Tests and verification receipts

Focused verification command:

```text
pnpm run test:source-control-context-menu
pnpm run test:floating-surface
pnpm run test:menu-open-sweep
pnpm run test:composer-config-menu
pnpm run test:composer-slash-commands
node --experimental-strip-types scripts/sessionContextMenuPlacement.test.ts
git diff --check
```

Observed results:

```text
sourceControlContextMenu tests passed
floating surface scheduling tests passed
menu open sweep tests passed
composerConfigMenu.test.ts passed
composerSlashCommands.test.ts passed
session context menu placement tests passed
git diff --check: exit 0, no output
```

Coverage added/changed:

- `scripts/sourceControlContextMenu.test.ts` proves commit/file snapshots do not change when the source objects mutate, verifies clamped placement, and statically prevents per-row context-menu roots from returning.
- `scripts/floatingSurface.test.ts` proves native work cannot run before two animation-frame boundaries.
- `scripts/menuOpenSweep.test.ts` enforces the snapshot/after-paint contracts across all fixed surfaces.
- `scripts/composerConfigMenu.test.ts` and `scripts/composerSlashCommands.test.ts` cover the new typed snapshot helpers.
- Touched `.mjs` tests were renamed to `.ts`, and package scripts now execute the TypeScript files directly.

Required Svelte ownership gate:

```text
pnpm run check:svelte
Files the /next shell owns: 0 error(s), 0 warning(s).
Elsewhere in the project (not checked by this gate): 16 error(s) — the old shell's own backlog.
```

The final verification commands were rerun after the report was completed; their fresh output is the completion receipt for this lane.

## Files changed

- Commit/file menu: `GraphPane.svelte`, `ChangesPane.svelte`, `SourceControlContextMenu.svelte`, `sourceControlContextMenu.ts`.
- Menu snapshots/deferred refresh: `BranchMenu.svelte`, `RunButton.svelte`, `UsagePopover.svelte`, `ResourcePopover.svelte`, `ComposerConfigMenu.svelte`, `CompactComposerControlsMenu.svelte`, `ConversationComposer.svelte`, `PullRequestPanel.svelte`, `SessionContextMenu.svelte`, `conversationConfig.ts`, `composerSlashCommands.ts`, `floatingSurface.ts`.
- Tests/tooling: `package.json`, three `.mjs` to `.ts` test renames, `floatingSurface.test.ts`, `menuOpenSweep.test.ts`, and the updated TypeScript tests listed above.

## Verified versus assumed

**Verified:** source-level regression origin; absence of a commit-menu invoke; removal of every per-row source-control context-menu owner; immutable open snapshots; one-pass placement; deferred native refresh; structural `/next` sweep; focused tests; diff whitespace check; Svelte ownership gate; no prohibited file edits; no Rust changes; no `any` additions.

**Not runtime-verified:** exact wall-clock freeze duration, CPU profile, and native interaction latency. Running the real app was prohibited, so no runtime claim is made.

**Follow-up:** the worktree-agent hover-card snapshot boundary at `WorktreeAgentRow.svelte:84-86,389-407`, held for its owning styling lane.

No files were staged or committed, and port 5177 was never bound.
