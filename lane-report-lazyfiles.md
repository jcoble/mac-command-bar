The Files tree now loads only while the Files right-panel tab is in view. Session picks made while Files is hidden do no explorer work; opening Files loads the selected root, repeated picks for that root are skipped, and a visible switch to a different root loads the new tree.

# Lazy Files panel activation lane report

## Implementation receipts

- `tauri-svelte-preview/src/lib/shell/panelActivation.ts:95-96` exposes `filesVisible(visible)` beside the source-control visibility report.
- `tauri-svelte-preview/src/lib/shell/panelActivation.ts:131-134` holds the mirrored `filesInView` and `filesLoadedFor` bookkeeping.
- `tauri-svelte-preview/src/lib/shell/panelActivation.ts:164-170` owns the single explorer activation path.
- `tauri-svelte-preview/src/lib/shell/panelActivation.ts:202-211` gates session-pick explorer loading on Files being in view and the root changing; the former unconditional call is gone.
- `tauri-svelte-preview/src/lib/shell/panelActivation.ts:256-262` loads the selected root when Files becomes visible and skips a root already loaded.
- `tauri-svelte-preview/src/lib/shell/panelActivation.ts:301-310` reports explorer as loaded only after an actual Files load.
- `tauri-svelte-preview/src/lib/shell/shellPanels.ts:122-141` exposes the returned visibility method through the existing `shellPanels` binding; its explorer service wiring is unchanged.
- `tauri-svelte-preview/src/routes/next/+page.svelte:370-376` reports Files visibility from the existing right-tab integration with `id === 'files'`.
- `tauri-svelte-preview/scripts/panelActivation.test.ts:6-35` covers the required hidden, first-visible, same-root, and changed-root sequence.

`explorerService` was not changed. No commit was created. Pre-existing untracked files were left alone.

## Red receipt

Command from `tauri-svelte-preview`:

```text
node --experimental-strip-types --test scripts/panelActivation.test.ts
```

Exit 1, verbatim output:

```text
✖ files_panel_hidden_skips_explorer_on_session_pick (5.391375ms)
ℹ tests 1
ℹ suites 0
ℹ pass 0
ℹ fail 1
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 211.674666

✖ failing tests:

test at scripts/panelActivation.test.ts:6:1
✖ files_panel_hidden_skips_explorer_on_session_pick (5.391375ms)
  AssertionError [ERR_ASSERTION]: a hidden Files panel costs nothing on a session pick
  + actual - expected
  
  + [
  +   '/repo/one'
  + ]
  - []
  
      at TestContext.<anonymous> (file:///Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/scripts/panelActivation.test.ts:24:10)
      at Test.runInAsyncScope (node:async_hooks:226:14)
      at Test.run (node:internal/test_runner/test:1201:25)
      at Test.start (node:internal/test_runner/test:1096:17)
      at startSubtestAfterBootstrap (node:internal/test_runner/harness:385:17) {
    generatedMessage: false,
    code: 'ERR_ASSERTION',
    actual: [ '/repo/one' ],
    expected: [],
    operator: 'deepStrictEqual',
    diff: 'simple'
  }
```

## Green receipts

Command:

```text
node --experimental-strip-types --test scripts/panelActivation.test.ts
```

Exit 0, verbatim output:

```text
✔ files_panel_hidden_skips_explorer_on_session_pick (5.07375ms)
ℹ tests 1
ℹ suites 0
ℹ pass 1
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 478.106958
```

There were no pre-existing `panelActivation` test files in this HEAD; the focused pure test above is the complete current suite.

Command:

```text
npx tsc --noEmit
```

Exit 0. Verbatim output: no output.

Command:

```text
pnpm run check:svelte
```

Exit 0, verbatim output:

```text

> mac-command-bar-tauri-svelte-preview@0.1.0 check:svelte /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview
> svelte-kit sync && node scripts/checkSvelteNext.mjs

warning src/lib/shell/components/EditorPanel.svelte:137:7  `codeEditor` is updated, but is not declared with `$state(...)`. Changing its value will not correctly trigger updates

Files the /next shell owns: 0 error(s), 1 warning(s).
Elsewhere in the project (not checked by this gate): 16 error(s) — the old shell's own backlog.
```

## Scope and diff receipt

`git diff --check` exited 0 with no output.

Changed implementation files plus the allowed test: 68 added lines and 16 removed lines, 84 changed lines total. This is within the requested 40–90-line budget.

```text
30	14	tauri-svelte-preview/src/lib/shell/panelActivation.ts
1	1	tauri-svelte-preview/src/lib/shell/shellPanels.ts
2	1	tauri-svelte-preview/src/routes/next/+page.svelte
35	0	tauri-svelte-preview/scripts/panelActivation.test.ts
```

<oai-mem-citation>
<citation_entries>
MEMORY.md:73-76|note=[worktree scope and verification context]
</citation_entries>
<rollout_ids>
</rollout_ids>
</oai-mem-citation>
