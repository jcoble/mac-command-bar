# TSK-808 Session History receipt

Started: 2026-08-08

## Scope

Implement the session-library center tab as a dedicated Session History experience, including grouping/filtering/search, inline expansion, session actions, reusable context menus, and the left-column Find a session entry point. Preserve panel id `session-library`; do not touch the excluded browser, conversation, resource, usage, frame, or Tauri areas.

## Evidence log

- 2026-08-08: Receipt created before implementation. Remaining claims will be appended as they are verified.
- 2026-08-08: Implemented the dedicated center-tab view, shared projection state, left-rail opener, reusable context menu, and model coverage. Scoped `git diff --check` and the new Session History stylesheet token scan returned no findings.
- 2026-08-08: `node --experimental-strip-types scripts/sessionLibrary.test.mjs` exited 0 with `sessionLibrary: all tests passed`.
- 2026-08-08: The requested full script sweep reached and passed `sessionLibrary.test.mjs`, then stopped on the unrelated `sourceCodeLensKeys.test.mjs` assertion in the parallel editor lane. That external failure was not changed.
- 2026-08-08: `pnpm check:svelte` exited 0: `/next` owned files reported `0 error(s), 0 warning(s)`; the command still reports 16 pre-existing old-shell errors outside this gate.
- 2026-08-08: `pnpm build` exited 0 and wrote the static site. Its warnings are existing old-shell accessibility/unused-selector, chunk-size, and ineffective dynamic-import warnings; no Session History error was emitted.
- 2026-08-08: After the final group-count/footer adjustment, the focused test, `pnpm check:svelte`, and `pnpm build` were rerun; all three exited 0 with the same external old-shell/editor warnings only.

## Claims

| Claim | Status | Evidence |
| --- | --- | --- |
| `SessionLibraryWorkspace.svelte` provides the dedicated Session History header, segmented scope, search/filter controls, project groups, row metadata, paging, and token-only styling | VERIFIED | `tauri-svelte-preview/src/lib/shell/sessionLibrary/SessionLibraryWorkspace.svelte`; fresh `pnpm check:svelte` and `pnpm build` both exited 0. |
| `sessionLibraryModel.ts` provides exact workspace/project/all filtering, grouping, and one-row expansion state | VERIFIED | `tauri-svelte-preview/src/lib/shell/sessionLibrary/sessionLibraryModel.ts`; `sessionLibrary.test.mjs` assertions passed. |
| Inline details contain Resume in Worktree, Continue in New Session, View Log, FIRST PROMPT, Copy, and LATEST TURNS surfaces | VERIFIED | `SessionLibraryWorkspace.svelte`; unsupported transcript/action data is visibly unavailable or disabled rather than fabricated. |
| `SessionContextMenu.svelte` is reusable for row and center-tab rosters | VERIFIED | `tauri-svelte-preview/src/lib/shell/sessionLibrary/SessionContextMenu.svelte` and `sessionLibraryContextMenu.ts`; roster test passed. |
| The left Find a session row opens the existing `session-library` center panel and has no popover | VERIFIED | `SessionsPaneview.svelte`, `sessionLibraryNavigation.ts`, and the session-library snippet in `src/routes/next/+page.svelte`. |
| The persisted center panel id remains `session-library` and its visible tab title is Session History, including restored layouts | VERIFIED | `tauri-svelte-preview/src/lib/shell/layout/centerDock.ts`; migration branches remain unchanged. |
| Session Library view-model tests | VERIFIED | `node --experimental-strip-types scripts/sessionLibrary.test.mjs` → `sessionLibrary: all tests passed`. |
| `pnpm check:svelte` | VERIFIED | `/next` shell gate → `0 error(s), 0 warning(s)`; 16 old-shell errors are outside this gate. |
| `pnpm build` | VERIFIED | `vite build` exited 0 and wrote the static site; warnings are external/pre-existing build noise noted above. |

## FUTURE-NATIVE gaps

- `resume-worktree` uses the existing rail resume/adopt callback, but there is no native action that creates or restores a dedicated worktree from an arbitrary history row. The UI does not claim that missing native behavior is complete.
- Complete first-prompt and transcript-turn retrieval is not present in the current session index. The details card shows the available latest preview and an explicit unavailable message for missing prompt/turn data; Copy is disabled when the prompt is absent.
- Continue in a new session and View Log have no backend handlers. Both actions are rendered disabled with FUTURE-NATIVE tooltips in the inline card and context roster.
- Provider-row Archive and Delete have no backend mutation support. They stay visible but disabled with a FUTURE-NATIVE tooltip; owned-row archive/delete use the existing rail callbacks.
- A center-tab context menu has no session-row target, so only Copy ID is enabled there; session actions are disabled with a FUTURE-NATIVE explanation.
