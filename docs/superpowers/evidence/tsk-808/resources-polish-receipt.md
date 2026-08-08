# TSK-808 resources-polish receipt

Scope: close only the four resources-polish gaps from the checkpoint review. No commit.

## Work log

- Receipt created before repository inspection, per lane instructions.
- `tsk-808-assembly-wave` is the active worktree; unrelated session-library/browser files are already dirty and remain untouched.
- Prior resource implementation is present in the shared worktree; this lane will limit edits to the named resource files, tests, Rust resource code/tests, and this receipt.
- `git show e794abc:tauri-svelte-preview/scripts/resourceViewModel.test.mjs` verified the pre-existing `resourceOwnerLabel` and `resourceCanStop` assertions that must be restored.
- `resources.rs:184-191` currently uses only the cwd leaf and parent leaf; the replacement will cache Git-root/common-dir identity and retain that heuristic only for non-repositories.

## Verified milestones

- [verified] Restored owner-label and stop-safety assertions alongside grouping/ring tests in `tauri-svelte-preview/scripts/resourceViewModel.test.mjs`; `node --experimental-strip-types scripts/resourceViewModel.test.mjs` passed.
- [verified] Added bounded CPU sample history, inline SVG sparklines, inactive-workspace review rows, and the Space workspace-name filter in the owned resource modules; `node --experimental-strip-types scripts/spaceViewModel.test.mjs` passed.
- [verified] `RUST_TEST_THREADS=1 cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml resources` passed; the local fixture test covered primary nested, linked-worktree nested, and non-Git fallback identity.
- [verified] `pnpm check:svelte` passed the `/next` gate with 0 errors / 0 warnings; it reported 16 errors in the old shell backlog, outside this lane and not changed.
- [verified] `pnpm build` completed successfully; existing old-shell accessibility/chunk warnings were emitted, with no resource-lane build error.
- [verified] `git diff --check` is clean for the owned resource files, tests, Rust module, and receipt.

## Claims

- [verified] `tauri-svelte-preview/scripts/resourceViewModel.test.mjs:12-14` restores `resourceOwnerLabel` and both `resourceCanStop` safety assertions; `:22-36` covers grouping, bounded CPU samples, and inactive-workspace counting.
- [verified] `tauri-svelte-preview/src/lib/shell/resources/resourceViewModel.ts:3-98` defines the bounded CPU ring, sparkline points, and inactive-workspace count; `resourceStore.svelte.ts:5-36` records each successful snapshot in that ring.
- [verified] `tauri-svelte-preview/src/lib/shell/resources/ResourcePopover.svelte:56-84` and `ResourcesWorkspace.svelte:33-47` render per-level CPU sparklines and the inactive-workspace review row.
- [verified] `tauri-svelte-preview/src/lib/shell/resources/WorkspaceSpaceWorkspace.svelte:21-26,78-79` and `workspaceSpaceViewModel.ts:21-25` provide and apply the workspace-name filter; `spaceViewModel.test.mjs:7-12` covers it.
- [verified] `tauri-svelte-preview/src-tauri/src/resources.rs:186-242` runs Git root/common-dir probes from the cwd with a process-wide cache and heuristic fallback; `:553-611` exercises primary nested, linked worktree nested, and non-Git fixtures locally.
- [verified] The requested JavaScript, Rust, Svelte `/next`, production build, and diff checks were rerun sequentially after the final resource edits; all passed. The 16 old-shell `check:svelte` errors remain outside this lane and were not changed.

## Final verification

- [verified] `node --experimental-strip-types scripts/resourceViewModel.test.mjs` and `node --experimental-strip-types scripts/spaceViewModel.test.mjs` both passed.
- [verified] `RUST_TEST_THREADS=1 cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml resources` passed 1 resource identity test with 0 failures.
- [verified] `pnpm check:svelte` reported 0 `/next` errors and 0 `/next` warnings; `pnpm build` completed and wrote the static site.
- [verified] No commit was created; unrelated shared-worktree changes remain present and untouched.
