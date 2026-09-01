# Editor session-switch teardown — dispose models, keep the UX

2026-08-20. Owner-measured: editors are the worst memory offender (LSP was
OFF — this is pure WebView/Monaco growth). Direction owner-approved: the ONE
shared Monaco editor stays alive (its ~250 MB first-open cost is paid once);
the DEPARTING session's per-file state is disposed at switch; tabs come back
as descriptors and rehydrate instantly.

## Hard constraints (from the reverted attempt `375dab3`/`2e56483`)

- NO component remount, NO `{#key}`, NO effect that writes `editorState` as
  part of teardown. Teardown is DIRECT code in the switch path.
- The safe boundary is the `if (switching)` block in `selectOwned`,
  immediately before `restoreWorkspace(ownedId)`
  (`tauri-svelte-preview/src/routes/next/+page.svelte:921-929`; verify —
  merged commits may have shifted lines). At that point the old session's
  draft is flushed and its workspace snapshotted (`:907-910`).
- Acceptance is a PLATEAU, not an instant RSS drop (WebKit allocator retains
  pages): open 10 files in session A, switch A→B→A five times — Activity
  Monitor must stay flat across cycles, not stair-step. Owner runs this in
  the desktop app (a plain browser cannot start the code services).

## What teardown must do (departing session), in order

1. For each open tab with a live model: capture Monaco `saveViewState()`
   (cursor/scroll/folding) keyed by path, and the unsaved `draftContent`.
   Store both in the session's workspace record via the existing
   `snapshotWorkspace` path (`+page.svelte:790-812`) — the record already
   persists up to 12 paths + active path + scroll
   (`src/lib/shell/sessionWorkspaces.ts:65-106`). Add a per-path
   `viewState` field there (JSON-serializable; cap stays 12 paths).
2. Dispose ALL of the departing session's Monaco models: tab models via the
   existing `disposeAllTabModels` (force) (`src/lib/MonacoSourceEditor.svelte:1391-1406`),
   plus owned/lazy external target models and the CodeLens/symbol caches the
   component clears in its `onDestroy` (`:2950-3030`) — factor those cache
   clears into a callable `releaseSessionResources()` used by BOTH onDestroy
   and the switch path. The editor instance itself is NOT disposed; call
   `editor.setModel(null)` before disposing the active model.
3. Clear EditorPanel's per-path maps (diagnostics/read-only/read-in-flight,
   `EditorPanel.svelte:145-171`) for the departing session's paths.
4. DELETE the retainedTabs full-text retention: `RetainedTab.preview` keeps
   full file text for 3 sessions (`sessionWorkspaces.ts:407-436,484-496`).
   Descriptors + viewState + drafts replace it. Remove the mechanism, its
   cap constant, and its restore path — replay reads from disk instead.
   Keep `draftContent` (unsaved edits) in the persisted record — drafts must
   never be lost.

## Rehydrate (arriving session) — must feel instant

1. Tab strip and active-tab selection rebuild synchronously from the
   workspace record (existing replay, `+page.svelte:865-880`).
2. The ACTIVE tab's file is read from disk and its model created immediately;
   apply `restoreViewState()` after the model is set. Other tabs get models
   lazily on first click (the existing `requestOpenFile` path already loads
   on demand — verify the replay does not eagerly read every path; if it
   does, restrict the eager read to the active path).
3. A file changed on disk since the descriptor was saved simply shows the new
   content (correct behavior); a draft always wins over disk content.

## Files (verify anchors before editing)

`src/lib/MonacoSourceEditor.svelte`, `src/lib/shell/components/EditorPanel.svelte`,
`src/routes/next/+page.svelte`, `src/lib/shell/sessionWorkspaces.ts`,
`src/lib/shell/editor/editorStore.svelte.ts` (all under tauri-svelte-preview).
No other files; no new files except none. Expected diff ~200-350 lines
(retainedTabs deletion is negative lines).

## Test-first

`sessionWorkspaces` has pure logic — write failing unit tests first (mirror
existing test style in the repo, e.g. scripts/ or *.test.* files):
- `workspace_record_round_trips_view_state_per_path`
- `snapshot_stores_drafts_but_no_file_preview_text` (asserts the full-text
  retention is gone)
- `restore_plan_marks_only_active_path_for_eager_read`

## Verification, verbatim

- `npx tsc --noEmit` and `pnpm run check:svelte` → "Files the /next shell
  owns: 0 error(s)" (run in the main worktree post-merge; the lane runs unit
  tests only).
- Owner protocol in the desktop app: (a) plateau test above; (b) switch away
  and back — same tabs, same order, same active file, cursor/scroll restored,
  unsaved draft intact; (c) no `effect_update_depth_exceeded` in the console.

## Done means

- Teardown runs only in the switch path at the stated boundary; no remount,
  no new effects; all three unit tests pass; gates green; retainedTabs
  full-text mechanism deleted.

KISS/YAGNI binding: smallest diff that meets the contract; no new
abstractions/options/shims beyond `releaseSessionResources()`; no drive-by
refactors; ~2x the diff estimate or an unlisted file means STOP and send a
one-paragraph proposal instead.
