# Explorer cached-root activation lane report

Status: blocked by the required Svelte gate. The scoped implementation and regression test are present, but this lane is not complete while that gate exits 1.

## Implementation receipts

- `activate(root)` reads the existing per-root source-record getter before resetting a changed root: `tauri-svelte-preview/src/lib/shell/explorer/explorerService.ts:132-134`.
- A non-empty cached root supersedes any active scan, restores the normal begin/apply/end state shape, and returns without `scanRoot`: `tauri-svelte-preview/src/lib/shell/explorer/explorerService.ts:135-147`.
- Cached truncation is conservatively re-derived as `cachedRecords.length >= EXPLORER_SCAN_LIMIT`; no cache metadata was added: `tauri-svelte-preview/src/lib/shell/explorer/explorerService.ts:140-144`.
- A root without cached records falls through to the existing fresh scan: `tauri-svelte-preview/src/lib/shell/explorer/explorerService.ts:148-149`.
- Explicit refresh still calls `scanRoot` directly: `tauri-svelte-preview/src/lib/shell/explorer/explorerService.ts:152-155`.
- The existing `projectSourceRecords` getter was sufficient, so `projectSourceIndex.ts` was not changed: `tauri-svelte-preview/src/lib/shell/projectSourceIndex.ts:24-30`.
- The regression test scans root A, scans root B, reactivates root A, then asserts only two scanner calls and root A state: `tauri-svelte-preview/scripts/projectSourceIndex.test.mjs:67-105`.
- Production diff size is 25 additions and 8 deletions in `explorerService.ts`; `projectSourceIndex.ts` has no diff. The test adds 42 lines to the existing test file. Receipt command: `git diff --numstat -- tauri-svelte-preview/src/lib/shell/explorer/explorerService.ts tauri-svelte-preview/src/lib/shell/projectSourceIndex.ts tauri-svelte-preview/scripts/projectSourceIndex.test.mjs`.
- No commit was created.

## Test-first red receipt

Command, from `tauri-svelte-preview`:

```text
node --experimental-strip-types --test scripts/projectSourceIndex.test.mjs
```

Output before implementation, exit 1:

```text
✖ activate_serves_cached_root_without_rescan (45.086959ms)
ℹ tests 1
ℹ suites 0
ℹ pass 0
ℹ fail 1
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 188.830084

✖ failing tests:

test at scripts/projectSourceIndex.test.mjs:67:1
✖ activate_serves_cached_root_without_rescan (45.086959ms)
  AssertionError [ERR_ASSERTION]: Expected values to be strictly deep-equal:
  + actual - expected
  
    [
      '/root-a',
      '/root-b',
  +   '/root-a'
    ]
  
      at TestContext.<anonymous> (file:///Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/scripts/projectSourceIndex.test.mjs:103:10)
      at async Test.run (node:internal/test_runner/test:1208:7)
      at async startSubtestAfterBootstrap (node:internal/test_runner/harness:385:3) {
    generatedMessage: true,
    code: 'ERR_ASSERTION',
    actual: [ '/root-a', '/root-b', '/root-a' ],
    expected: [ '/root-a', '/root-b' ],
    operator: 'deepStrictEqual',
    diff: 'simple'
  }
```

## Green regression receipt

Command, from `tauri-svelte-preview`:

```text
node --experimental-strip-types --test scripts/projectSourceIndex.test.mjs
```

Output after implementation, exit 0:

```text
✔ activate_serves_cached_root_without_rescan (80.199083ms)
ℹ tests 1
ℹ suites 0
ℹ pass 1
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 328.896709
```

## TypeScript gate receipt

Command, from `tauri-svelte-preview`:

```text
npx tsc --noEmit
```

Exit 0 with no output.

## Required Svelte gate failure

Command, from `tauri-svelte-preview`:

```text
pnpm run check:svelte
```

Output, exit 1:

```text
> mac-command-bar-tauri-svelte-preview@0.1.0 check:svelte /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview
> svelte-kit sync && node scripts/checkSvelteNext.mjs

warning src/lib/shell/components/EditorPanel.svelte:137:7  `codeEditor` is updated, but is not declared with `$state(...)`. Changing its value will not correctly trigger updates
warning src/routes/next/+page.svelte:270:7  `conversationSurface` is updated, but is not declared with `$state(...)`. Changing its value will not correctly trigger updates
ERROR   src/routes/next/+page.svelte:896:20  'selected' is possibly 'undefined'.
ERROR   src/routes/next/+page.svelte:896:44  'selected' is possibly 'undefined'.
ERROR   src/routes/next/+page.svelte:897:64  'selected' is possibly 'undefined'.

Files the /next shell owns: 3 error(s), 2 warning(s).
Elsewhere in the project (not checked by this gate): 16 error(s) — the old shell's own backlog.

Checking the /next shell failed. Fix the problems listed above.
 ELIFECYCLE  Command failed with exit code 1.
```

The blocking errors are in the explicitly protected `tauri-svelte-preview/src/routes/next/+page.svelte`; this lane did not modify that file.
