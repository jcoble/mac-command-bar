# Timer and lease leak-fix lane report

## Outcome

Blocked at the first required verification gate. The two scoped production edits are present but the lane is not complete because TypeScript compilation failed. Per instruction, no follow-up edit or later gate was run, and no commit was created.

## Partial implementation receipts

- `tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:567-615` adds a `Set<number>` for semantic retry handles, removes each handle when its callback fires, and retains the existing retry behavior while the request remains current.
- `tauri-svelte-preview/src/lib/shell/editor/sourceIntelligence.ts:951-960` cancels and clears every tracked retry in the shared reset path used by project-root and active-preview changes.
- `tauri-svelte-preview/src/lib/shell/editor/extensionApiProbeController.ts:221-230` disposes a newly created terminal lease when the post-await ownership check throws, then rethrows the same error.
- `tauri-svelte-preview/src/lib/shell/editor/extensionApiProbeController.ts:272-292` applies the same disposal-and-rethrow behavior to a newly acquired SCM lease.
- Scoped production diff before the gate: 20 insertions and 4 deletions across the two named production files.

## Test seams

No test changes were retained.

- `scripts/sourceCodeLensKeys.test.mjs` has a source-level seam, but its mandated command fails earlier at `scripts/sourceCodeLensKeys.test.mjs:159` on a pre-existing assertion against the unrelated dirty `MonacoSourceEditor.svelte` experiment, before a focused retry-lifecycle assertion can execute.
- `scripts/extensionApiProbeController.test.mjs` has a behavioral seam, but its mandated command fails during module loading before any test executes:

```text
Error [ERR_MODULE_NOT_FOUND]: Cannot find module '/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/productIdentity' imported from /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview/src/lib/shell/editor/fixtures/mcbExtensionApiProbe.ts
```

Changing either unrelated dependency was outside this lane's allowed files.

## Required gate output

Command, from `tauri-svelte-preview`:

```text
npx tsc --noEmit
```

Exit code: `2`

Verbatim output:

```text
src/lib/shell/editor/sourceIntelligence.ts(611,11): error TS2322: Type 'Timeout' is not assignable to type 'number'.
```

`pnpm run check:svelte` was not run because the instruction required stopping when a required command failed.

## Scope preservation

Only the two named production files and this required report were changed by this lane. Existing experiment edits and untracked reports were left untouched. No commit was created.
