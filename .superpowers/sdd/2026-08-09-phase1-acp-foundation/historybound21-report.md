# historybound21 — bounded Session History render

## Outcome

Verified complete in `/Users/blackcolours/dev/work/worktrees/mac-command-bar/historybound21` on branch `lane/historybound21`. No commit, staging, or push was performed.

The Session History view now renders a 25-row newest-first window per worktree, discloses 25 more rows at a time, searches the full record set, resets disclosure on filter changes, mounts only after the center surface is first shown, and runs its 60-second age clock only while that surface is visible.

## DOM count receipt

- Before: approximately 4,000 row elements from the owner-provided live profile. This baseline was not independently rerun against pre-change code in this lane.
- After, verified with 4,000 deterministic records at `1710x990` on strict port `5182`:
  - inactive startup: `0` history row elements and `1` cheap placeholder;
  - first shown: `25` history row elements;
  - disclosure label: `Show more — 3975 older`;
  - after one disclosure: `50` history row elements;
  - browser console errors: `0`.
- Screenshot: `tauri-svelte-preview/output/playwright/historybound21-bounded-history.png` visibly includes the bounded list bottom and the Show more control.
- Receipt command: `node --experimental-strip-types scripts/sessionHistoryDomReceipt.ts`.

## Implementation receipts

- `tauri-svelte-preview/src/lib/shell/history/sessionHistoryViewModel.ts:9` names `SESSION_HISTORY_ROW_WINDOW = 25`.
- `tauri-svelte-preview/src/lib/shell/history/sessionHistoryViewModel.ts:75-111` owns disclosure state, 25-row extension, and filter-reset behavior.
- `tauri-svelte-preview/src/lib/shell/history/sessionHistoryViewModel.ts:250-306` filters and sorts the full record set, bypasses the browsing window for text search, and slices each worktree before returning rows to the component.
- `tauri-svelte-preview/src/lib/shell/sessionLibrary/SessionLibraryWorkspace.svelte:105-121` latches first visibility and scopes the 60-second interval to current visibility.
- `tauri-svelte-preview/src/lib/shell/sessionLibrary/SessionLibraryWorkspace.svelte:388-406` uses the kit ghost button and renders the cheap pre-activation placeholder.
- `tauri-svelte-preview/src/routes/next/+page.svelte:1441` passes the existing `activeCenterPanelId` signal; no parallel activation mechanism was added.
- The unused global `paginateSessionLibrary` helper and `SessionLibraryPage` type were deleted from `sessionLibraryModel.ts`. The per-worktree view-model window is the one remaining history pagination mechanism.

## Fail-first and green tests

Fail-first receipt:

```text
SyntaxError: The requested module '../src/lib/shell/history/sessionHistoryViewModel.ts'
does not provide an export named 'SESSION_HISTORY_ROW_WINDOW'
```

Fresh green receipt:

```text
sessionHistoryWindow: all tests passed
sessionHistoryViewModel: all tests passed
sessionLibrary: all tests passed
```

Commands:

```text
node --experimental-strip-types scripts/sessionHistoryWindow.test.ts
node --experimental-strip-types scripts/sessionHistoryViewModel.test.mjs
node --experimental-strip-types scripts/sessionLibrary.test.ts
```

The 4,000-record assertions are in `tauri-svelte-preview/scripts/sessionHistoryWindow.test.ts:37-77`: 25 initial rows, 3,975 older, 50 after disclosure, full search bypass, and filter reset.

## Svelte gate

Command: `pnpm run check:svelte`

```text
Files the /next shell owns: 0 error(s), 0 warning(s).
Elsewhere in the project (not checked by this gate): 16 error(s) — the old shell's own backlog.
```

## Performance harness note

`scripts/perfHarness.ts:7` hardcodes port `5181`. That port is reserved by another lane under this lane's contract, so the harness was skipped rather than competing for it. The strict-port 5182 DOM receipt above is the primary performance evidence requested by the task.

## Cleanup

```text
Browser cleanup: stopped historybound21-chrome (Chrome helper tree exited)
Browser cleanup: stopped historybound21-vite (5182 process tree exited)
```

Final process checks found no listener on port `5182` and no process tagged `historybound21-chrome`. The supplied worktree remains in place because this lane did not create it. The pre-existing untracked `tauri-svelte-preview/node_modules` directory was preserved untouched.
