# editorchrome20 — language server bar above the file tabs

Status: done. Not committed, not staged (per lane rules).

## What changed

The language-server status chip and the "Language intelligence" switch used to render inline at
the right end of the file-tab strip, taking width away from the tabs and pushing them out of
view. They now sit on their own thin full-width row directly above the tab strip, right-aligned,
and the tab strip has the whole row to itself. The old inline placement is deleted — there is one
place these controls render, no flag and no fallback position.

The markdown Source/Preview segmented control was left where it was, in the tab row; the lane
brief covered only the status chip and the intelligence switch.

## Files touched

- `/Users/blackcolours/dev/work/worktrees/mac-command-bar/editorchrome20/tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte`
  (only file changed; `git diff --stat` = 1 file, 62 insertions, 29 deletions — verified)

## Receipts

- `EditorPanel.svelte:858-886` — new `.editor-status-bar` row holding the `.status-slot` wrapper
  around `<LanguageServerStatusChip>` and the `.intelligence` block with the kit `Switch`.
  Verified.
- `EditorPanel.svelte:887` — `.editor-header` now contains only the file strip and the markdown
  view toggle. Verified.
- Old placement gone: `grep -n "LanguageServerStatusChip\|class=\"intelligence\""` returns exactly
  one render site each (lines 864 and 873) plus the import at line 38. Verified.
- `EditorPanel.svelte:1034-1060` — `.editor-status-bar` (surface background, bottom border,
  `padding: 3px 8px`) and `.status-slot` (`flex: 1 1 auto`, `justify-content: flex-end`,
  `min-width: 0`, `overflow: hidden`). Verified.
- No layout shift when the status text changes: the chip sits in a flex-grow slot that is
  right-aligned against the switch, so a longer or shorter label grows into empty space to its
  left and nothing else on the row moves. `.intelligence-state` keeps its existing
  `min-width: 22px` for the on/off word (`EditorPanel.svelte:1131-1135`). Verified by reading the
  rules; not verified in a running app (the lane brief forbids launching it).
- No timers, no animations, no new dependencies added — the change is markup and CSS only.
  Verified from the diff.

## Gates

- `pnpm run check:svelte` in `tauri-svelte-preview`:
  `Files the /next shell owns: 0 error(s), 0 warning(s).`
  (It also prints `Elsewhere in the project (not checked by this gate): 16 error(s) — the old
  shell's own backlog`, which is pre-existing and outside this gate.) Verified.
- `node --experimental-strip-types src/lib/shell/components/editor/editorModesWiring.test.ts`:
  `tests 7 / pass 7 / fail 0`. Verified.

## Cleanup

No worktree created by this lane (worked in the one provided). No browser session opened.
