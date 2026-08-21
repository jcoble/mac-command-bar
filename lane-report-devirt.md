The plain timeline experiment is in place, and both required verification gates pass. Only the requested component and this report were changed in the lane's existing uncommitted work; no commit was made.

# Timeline devirtualization experiment

## What was bypassed

- `PLAIN_RENDER = true` selects the experiment and leaves the original virtual path available when flipped to `false`: `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:4`.
- The enabled path does not call `createVirtualizer`, calculate row estimates, publish virtualizer options, scroll to a virtual index, measure rows, or run the virtual hydration effect: `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:144`, `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:149`, `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:162`, `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:372`, `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:425`, `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:439`.
- Every turn group is mounted with a plain keyed `{#each}` in document flow. The row key remains the index used by TanStack's default keying, while timeline items remain keyed by `item.itemId`: `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:610`.
- The plain-render CSS restores normal document flow for turns and the active-turn tail: `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationTimeline.svelte:735`.
- Existing session reset, follow-scroll behavior, and fold/expand handlers remain in place.

No commit was made. Pre-existing experiment edits in other files were not touched or reverted.

## Verification

From `tauri-svelte-preview`:

### `npx tsc --noEmit`

Exit code: 0

Verbatim output (empty):

```text
```

### `pnpm run check:svelte`

Exit code: 0

Verbatim output:

```text

> mac-command-bar-tauri-svelte-preview@0.1.0 check:svelte /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave/tauri-svelte-preview
> svelte-kit sync && node scripts/checkSvelteNext.mjs

warning src/lib/shell/components/EditorPanel.svelte:137:7  `codeEditor` is updated, but is not declared with `$state(...)`. Changing its value will not correctly trigger updates
warning src/routes/next/+page.svelte:238:7  `editorPanel` is updated, but is not declared with `$state(...)`. Changing its value will not correctly trigger updates
warning src/routes/next/+page.svelte:270:7  `conversationSurface` is updated, but is not declared with `$state(...)`. Changing its value will not correctly trigger updates

Files the /next shell owns: 0 error(s), 3 warning(s).
Elsewhere in the project (not checked by this gate): 16 error(s) — the old shell's own backlog.
```
