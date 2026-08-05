# TSK-808 shell-mounting receipt

Date: 2026-08-04  
Repository: `mac-command-bar`  
Branch: `tsk-808-assembly-wave`  
Base: `4857674`

## Outcome

The `/next` shell now mounts the existing Working/Done/Settled Paneview in the left column and the existing Session Library workspace as a center Dockview destination. The v3 center layout is migrated exactly when its four-panel roster matches; malformed, partial, or unrelated payloads fall back to the current defaults. No commit was created.

## Changed files

- `tauri-svelte-preview/src/lib/shell/components/SessionsColumn.svelte`
  - Reduced the left-column wrapper to the existing width strip, New session affordance, collapse control, and remove confirmation.
  - Replaced the old Working/Done Collapsibles and resumable archive drawer with the `SessionsPaneview` host at lines 173–190.
  - Forwarded select, resume, complete/reopen, settle/unsettle, close, rescan, and remove intents.

- `tauri-svelte-preview/src/lib/shell/components/SessionsPaneview.svelte`
  - Mounted one injected `createPaneStack` host with stable `working`, `done`, and `settled` registrations at lines 42–79; Settled is collapsed by default at line 75.
  - Moved the existing resumable finder/search UI into the host and kept it prop-driven.
  - Creates/disposes the Paneview only in `onMount` at lines 192–227; no service or backend call runs when the host is constructed.
  - Uses the existing `sidePaneRegistry`/`paneStack` `left-rail` layout entry, with no new storage-key family.

- `tauri-svelte-preview/src/lib/shell/components/DonePane.svelte`
  - Added the optional remove-confirmation callback and forwarded it to the row.

- `tauri-svelte-preview/src/lib/shell/components/WorktreeAgentRow.svelte`
  - Added the existing Done-shelf remove affordance while retaining the mark-done, reopen, settle, unsettle, restart, and close actions.

- `tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte`
  - Added `sessionLibrary` to the center snippet contract and the parked Svelte slot.
  - Added the `session-library` panel spec at lines 112–123. No `agents` panel was registered.

- `tauri-svelte-preview/src/lib/shell/layout/centerDock.ts`
  - Added exact v3 roster recognition for `session`, `editor`, `browser`, and `diff`.
  - Restores a valid v4 layout unchanged; otherwise restores a valid v3 layout, adds `session-library` inside the existing `diff` group, restores the prior active panel, and saves the complete result under v4.
  - Any roster mismatch or Dockview restore failure falls through to the existing safe default builder.
  - The center dock remains DOM-neutral; `ShellFrame.svelte` and the page supply the `SessionLibraryWorkspace` element mapping through the existing panel-spec contract.

- `tauri-svelte-preview/src/lib/shell/layout/layoutStorage.ts`
  - Kept the previous key as `CENTER_LAYOUT_KEY_V3` and bumped the authority-owned `CENTER_LAYOUT_KEY` to `mac-command-bar.next.center-layout-v4`.

- `tauri-svelte-preview/src/lib/shell/shellCommands.ts`
  - Added `show-session-library`, which routes through the existing `showPanel('session-library')` command pattern at lines 80–83.

- `tauri-svelte-preview/src/routes/next/+page.svelte`
  - Added explicit `settledAt` transitions through `updateOwnedSession` at lines 202–210.
  - Built an inert-at-construction Session Library service adapter around the existing rail actions at lines 212–245.
  - Mounted `SessionLibraryWorkspace` only in the center snippet at lines 1117–1124 and supplied it as the fifth center destination at lines 1137–1143.

- `tauri-svelte-preview/scripts/layoutStorage.test.mjs`
  - Added the center roster key and exact-set migration-source assertions.

- `docs/superpowers/evidence/tsk-808/shell-mounting-receipt.md`
  - This complete implementation, migration, unmount, and verification receipt.

Unchanged by design: `ShellSidebar.svelte`, `sidebarViews.ts`, `panelActivation.ts`, and `shellPanels.ts`. The library is not a right-column view, and `panelActivation.ts` continues to load only `editor` and `browser`; therefore showing the new center tab does not start panel IO.

## CENTER_LAYOUT_KEY migration

`CENTER_LAYOUT_KEY` is now `mac-command-bar.next.center-layout-v4`; `CENTER_LAYOUT_KEY_V3` remains the readable legacy key. On startup, `centerDock.ts` first requires the stored v4 panel set to equal the live roster exactly. If that fails, it requires the v3 set to equal exactly the four known legacy IDs, restores that Dockview tree, adds `session-library` through the public Dockview API, restores the previous active panel, and writes the completed tree under v4. The v3 record is left untouched. A malformed, partial, extra, or unrelated roster, or a Dockview exception, clears the attempted restore and uses the existing safe defaults.

## Activation and unmount evidence

- No required shell surface is unmountable.
- The old components were not deleted; only their old `SessionsColumn` mount was removed. The new single left host is evidenced by `SessionsColumn.svelte:173–190`.
- The left registry contains only `working`, `done`, and `settled` (`SessionsPaneview.svelte:42–79`); `session-library` is absent from that registry.
- The center roster contains `session-library` and no `agents` (`ShellFrame.svelte:112–123`).
- The library service is constructed with getters and callbacks only; `SessionLibraryWorkspace` receives the service at `+page.svelte:1117–1124`. Its refresh/action work remains imperative.
- The existing activation gate ignores `session-library` because `panelActivation.ts`'s loadable set remains limited to `editor` and `browser`.
- The optional A8 Session Library fork button remains inert because no fork action exists in the page-owned rail authorities (`SessionLibraryWorkspace.svelte:93–94,235`; the page adapter has no `onFork` at `+page.svelte:214–245`). The task-authorized shell actions were open/resume/complete/reopen/settle/unsettle, so no invented fork or second authority was added.

## Verification receipts

All commands were run serially from the repository root or the `tauri-svelte-preview` package directory for direct script invocations.

| Command | Result |
| --- | --- |
| `pnpm --dir tauri-svelte-preview test:pane-layout` | PASS — `paneLayout: all tests passed` |
| `pnpm --dir tauri-svelte-preview test:layout-storage` | PASS — `layoutStorage: all tests passed` |
| `pnpm --dir tauri-svelte-preview test:sidebar-views` | PASS — `sidebarViews: all tests passed` |
| `pnpm --dir tauri-svelte-preview test:session-strip` | PASS — `sessionStrip: all assertions passed` |
| `pnpm --dir tauri-svelte-preview test:session-groups` | PASS — `sessionGroups: all tests passed` |
| `pnpm --dir tauri-svelte-preview test:owned-sessions` | PASS — `ownedSessions tests passed` |
| `node --experimental-strip-types scripts/sidePaneRegistry.test.mjs` | PASS — `sidePaneRegistry: all tests passed` |
| `node --experimental-strip-types scripts/sessionLibrary.test.mjs` | PASS — `sessionLibrary: all tests passed` |
| `pnpm --dir tauri-svelte-preview check:svelte` | PASS — `/next` owns 0 errors and 0 warnings; the command reports 16 pre-existing errors outside the gate in the old shell |
| `pnpm --dir tauri-svelte-preview build` | PASS — Vite/Svelte build completed, adapter-static wrote `build` |
| `git diff --check` | PASS |

The build still prints existing warnings from the old shell (`src/routes/+page.svelte`, `ActivityGitPanel.svelte`, and `ActivityWorktreesPanel.svelte`) plus normal chunk-size/dynamic-import notices; it exits successfully and reports no `/next` failure.

## Deviations and preserved work

- Required persistence-owner seam: `layoutStorage.ts` is outside the enumerated shell-file list, but it is the existing authority that owns `CENTER_LAYOUT_KEY`. Editing that owner was necessary to perform the requested v3→v4 bump without introducing a duplicate key family.
- The direct instruction not to register `agents` was followed even though the broader plan names that future surface; no component exists to mount.
- The existing dirty/untracked Rust work was preserved and not touched by this packet: `src-tauri/src/agent_conversation/manager.rs`, `git_diff_models.rs`, `lsp.rs`, `main.rs`, `orchestration.rs`, and untracked `src-tauri/src/workflow.rs`.
- No browsers, persistent external processes, commits, or new worktrees were created.
