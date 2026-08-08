# TSK-808 diff-layout bug receipt

Status: fixed in the shared worktree; verification complete.

This receipt records reproduction, root-cause evidence, the regression test, the fix, and verification for the Diff center-tab layout corruption.

## Reproduction

- Owner screenshot: `ui-feedback/current-diff-layout-breakage.png` shows the Diff tab's two editor panes escaping the center geometry and sitting under/over other shell surfaces.
- Headless baseline (before this lane's changes): `pnpm test:pane-layout`, `pnpm test:layout-storage`, and `pnpm test:panel-activation` all pass. Those suites do not exercise the center Dockview's DOM mount order.
- The preview server could not be started in this sandbox (`vite` listen on `127.0.0.1:5177` returned `EPERM`), so reproduction is pinned to the deterministic layout lifecycle in source rather than claimed as a fresh browser screenshot.

## Initial seam evidence (before the fix)

- `ShellFrame.svelte` creates the center Dockview at lines 107-131, then calls `frame.layout(...)` at line 132. The center Dockview therefore restores/builds while its Gridview parent has not yet received its first non-zero layout.
- `centerDock.ts` deliberately calls `layoutToContainer()` before v5/v4 restore/build at lines 246-260, but that call sees the just-created center host before the parent frame has been laid out. A Dockview/Monaco diff mounted during this window can capture zero/stale bounds; activation then exposes the stale absolute overlay geometry.

## Regression test (failing before fix)

- Added `tauri-svelte-preview/scripts/centerDock.test.mjs` to assert the parent Gridview layout call precedes `createCenterDock`.
- Baseline result: **failed** with `AssertionError: the parent Gridview must have real bounds before center Dockview restore/build`.

## Root cause and fix

**Verified source cause:** `centerDock.ts:246-260` measures the Dockview host before its restore/build path. The old `ShellFrame.svelte` order created that Dockview first and only then ran the explicit parent Gridview layout that settles the populated center region, so the center host could still be 0×0 while Dockview restored panels. With Dockview's always-rendered panels and the Diff editor's split layout, that stale geometry was exposed when the Diff panel became active.

**Fix:** `ShellFrame.svelte:107-137` now lays out the parent Gridview before calling `createCenterDock`. This satisfies the existing center-Dockview precondition instead of adding a Diff-specific overlay workaround. The change stays within the owned ShellFrame layout seam; `centerDock.ts` and `GitDiffView.svelte` did not need symptom-level changes.

## Suspect checks

- **v4-to-v5 migration re-adding panels:** not the root cause. `centerDock.ts:264-309` accepts only the exact five-panel v4 roster, calls Dockview's restore once, and adds only the missing `agents` panel inside the restored `session-library` group. The exact-roster cases are covered by `layoutStorage.test.mjs:196-223`; no duplicate Diff panel is introduced.
- **Diff mounting from the parking stage:** not a duplicate mount. `GitDiffView.svelte:84-92` renders one `NativeGitDiffEditor`, while `NativeGitDiffEditor.svelte:100-112` creates one Monaco diff editor in its bounded host. The parking stage is `display:none`, so parked content cannot contribute measured geometry.
- **Panel activation re-parenting:** not the initiating seam. `centerDock.ts:87-113` moves each Svelte element into its renderer once and returns it to its recorded parking parent only on disposal; activation itself only selects the existing panel.
- **Overlay bounds after activation:** confirmed as the failure mechanism. Dockview's always-rendered renderer is configured at `centerDock.ts:117-124`, and its overlay positions are computed from the reference and shell rectangles. The old ShellFrame order let the center Dockview take its initial dimensions before the parent Gridview layout; the fix removes that zero/stale measurement window.

## Verification

Verified serially in this worktree:

- `node --experimental-strip-types scripts/centerDock.test.mjs` failed before the fix, then `pnpm test:center-dock` passed after it.
- `pnpm test:pane-layout` — `paneLayout: all tests passed`.
- `pnpm test:layout-storage` — `layoutStorage: all tests passed`.
- `pnpm test:panel-activation` — `panelActivation: all tests passed`.
- `pnpm check:svelte` — `/next` shell gate reports `0 error(s), 0 warning(s)`; the command also reports 16 pre-existing errors outside `/next`, which are outside this lane.
- `pnpm build` — completed with `✓ built in 12.23s`; existing non-fatal warnings remain in the old shell and chunk diagnostics.
- `git diff --check` — clean.

**Assumed / not freshly browser-verified:** the supplied owner screenshot is the visual reproduction. A fresh preview could not be started in this sandbox because Vite's listener on `127.0.0.1:5177` returned `EPERM`; therefore no new native/browser screenshot is claimed here. The source-order failure and all requested headless/build gates are verified.

Status: fix and regression coverage complete in the dirty shared worktree. No commit created.
