# editorfix21 — language-server controls in the editor title row

Status: done. HEAD remains `7f4163ff6bc3f15f3299de05b3cc6812541169a4`, the index is clean,
and no commit or push was performed (verified by `git rev-parse HEAD`, `git diff --cached --quiet`,
and the command history for this lane).

## What moved and what was deleted

- Verified — `tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:855-921` now has one
  `.editor-header` row containing the open-file tabs, `LanguageServerStatusChip`, the existing
  Markdown kit `SegmentedControl` when applicable, and the existing language-intelligence kit
  `Switch` block.
- Verified — `tauri-svelte-preview/src/lib/shell/components/EditorPanel.svelte:1025-1037` retains
  the title row's existing `gap` and `padding`; this lane added no replacement spacing rule.
- Verified — the separate `.editor-status-bar` markup, `.status-slot` wrapper, and both CSS rules
  introduced by `83ec542` are deleted. `rg -n "editor-status-bar|status-slot"` has no application
  match, so there is no empty spacer.
- Verified — `tauri-svelte-preview/src/lib/shell/components/editor/editorPanelLanguageServer.test.ts:43-53`
  pins the single-row contract and absence of the deleted row/wrapper.

## Verification receipts

- Verified — `pnpm run check:svelte` exited 0 and printed:
  `Files the /next shell owns: 0 error(s), 0 warning(s).`
- Verified — `pnpm test:editor-panel-language-server`: 12 tests, 12 passed, 0 failed.
- Verified — `pnpm test:editor-modes-wiring`: 7 tests, 7 passed, 0 failed.
- Verified — `rg -n "editor-status-bar|status-slot|editor-header|markdown-view-toggle" scripts`
  found only `scripts/markdownPreview.test.mjs:77`, the unchanged Markdown toggle testid pin;
  `pnpm test:markdown-preview` printed `markdownPreview: ok`.

## Browser receipt

- Verified — screenshot:
  `tauri-svelte-preview/output/playwright/editorfix21-editor-title-row-1710x990.png`.
- Verified — `window.innerWidth` / `window.innerHeight`: `1710` / `990`; `sips` reports the PNG as
  `1710 x 990` pixels.
- Verified — browser DOM receipt: file tab `EditorPanel.svelte`; chip `Svelte: ready`;
  `.editor-header .server-chip` present; `.editor-status-bar` count `0`.
- Verified — the screenshot used Vite at `http://127.0.0.1:5181/next` with `--strictPort`. A
  controlled desktop-command mock supplied the truthful `ready` state because a plain browser has
  no language-server process; the rendered chip and row are the application's real components.
- Verified — visual inspection shows the file tab, ready chip, and language-intelligence controls
  sharing the one editor title row, with no second row above it.

## Cleanup

Browser cleanup: stopped editorfix21 (daemon + Chrome helper tree).

Browser cleanup: stopped editorfix21 Vite 5181 server (wrapper + child).

- Verified — the temporary Vite filesystem allowance used for this worktree's pre-provisioned
  `node_modules` symlink was removed; `git diff -- vite.config.ts` is empty.
- Verified — final process search found no `cliDaemon.js editorfix21`, no session-scoped browser
  helper tree, no `playwright-cli --session editorfix21`, and no editorfix21 Vite process. A
  separately owned `centerfix21` lane started its own strict-5181 server after this lane's server
  stopped; it was identified by `--config scripts/centerfix21.vite.config.ts` and left untouched.
- Verified — the provided worktree was used directly; no worktree was created or removed.
