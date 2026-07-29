# Source control — how to wire it in

Scratch note for the integrator. Delete it once the wiring below has landed.

Everything in this lane is new files. Nothing shared was edited, so the panel on
screen today is unchanged until you do the six things in "The wiring" below.

---

## What this lane built

| File | What it is |
|---|---|
| `src/lib/shell/git/gitGraphLanes.ts` | Pure column layout for the commit graph: which column each commit sits in and which line segments its row draws. No state, no IO. |
| `src/lib/shell/git/gitBackendExtra.ts` | Wrappers for the two new desktop commands (`read_git_commit_files`, `read_git_commit_file_diff`), the repository-top resolver, the read-only browser backend, and the "this desktop build is older than this panel" detection. |
| `src/lib/shell/git/gitCommitFilesStore.svelte.ts` | Which commits are open, what each one changed, and every sentence those rows can say. No IO, no `$effect`. |
| `src/lib/shell/git/gitCommitFilesService.ts` | The git calls behind an open commit. Injectable for tests; exports the singleton `gitCommitFilesService`. |
| `src/lib/shell/components/git/SourceControlPanes.svelte` | The whole surface: `RepoPane` + `ChangesPane` + `GraphPane`. No required props. |
| `src/lib/server/gitBridge.ts` | The dev server's **read-only** git bridge, so the panel works in a browser. Exports `gitBridgePlugin()` for `vite.config.ts`. |
| `scripts/gitGraphLanes.test.mjs`, `scripts/gitBridge.test.mjs`, `scripts/gitCommitFiles.test.mjs` | Node tests. The bridge one runs against this real repository. |

Scratch, for looking at the panes in a browser while they were built — **delete
both** once the wiring lands: `src/routes/git-preview/+page.svelte` and
`vite.config.gitpreview.ts`.

Left alone on purpose: `gitPanelStore.svelte.ts`, `gitService.ts`,
`parseUnifiedDiff.ts`, `GitDiffView.svelte`. They were sound and are used as
they are.

---

## The wiring

### 1. The panel body

`ShellSidebar.svelte:293` parks `<GitPanel />` as the `source-control` view.
Replace the body of `GitPanel.svelte` with the new panes:

```svelte
<script lang="ts">
  import SourceControlPanes from './git/SourceControlPanes.svelte';
</script>

<SourceControlPanes />
```

`GitPanel.svelte`'s `showDiff` prop and its whole hand-written stylesheet go
with the old body. Nothing passes that prop today (`ShellSidebar` mounts the
panel propless), so nothing else changes.

`SourceControlPanes` takes no required props: it uses the `gitService` and
`gitCommitFilesService` singletons and works out for itself whether this page
can change the repository. `gitService.activate(root)` from `shellPanels.ts:53`
still drives it, unchanged.

### 2. The diff as a fourth centre tab

The diff is no longer drawn inside the panel. `GitDiffView` reads `gitPanel`
directly and takes no props, so it can simply be mounted in the centre dock.

- `ShellFrame.svelte:93-96` — add the fourth roster entry:
  ```ts
  { id: 'diff', title: 'Diff', element: diffSlot, group: 'display' }
  ```
  plus `let diffSlot: HTMLElement;` (:67) and the parking slot at :145:
  ```svelte
  <div class="slot" bind:this={diffSlot}>{@render center.diff()}</div>
  ```
- `ShellFrame.svelte:23` — widen the snippet contract:
  `center: { session: Snippet; editor: Snippet; browser: Snippet; diff: Snippet }`.
- `src/routes/next/+page.svelte:676` area — add
  `{#snippet diffArea()}<GitDiffView />{/snippet}` and pass it at :681.
- **`CENTER_LAYOUT_KEY` must be bumped** (`layoutStorage.ts:35`,
  `…center-layout-v2` → `-v3`). `centerDock.ts:252` only restores a stored
  layout when its panel ids match the roster exactly; without the bump every
  existing user keeps the three-tab layout and the Diff tab never appears.
- The activation hop: when a file is picked, bring the Diff tab to the front.
  The page already has the call — `frameControls?.showCenterPanel('diff')`, the
  same shape as `:672` for the editor. Put it in the click path, not in
  `gitService.selectFile` (the service must stay free of layout).
  `SourceControlPanes` does not do this itself: it has no access to the frame.
  If you would rather not touch the click path, the alternative is
  `onCenterPanelShown`-style plumbing — but the one-line call is the smaller
  change.

Both kinds of file selection land in the same place: a changed file goes through
`gitService.selectFile`, and a file inside a commit through
`gitCommitFilesService.selectCommitFile`, which writes into the same `gitPanel`
fields after letting go of whatever diff was on screen.

### 3. The git bridge in `vite.config.ts`

Two lines, so a browser can read a repository:

```ts
import { gitBridgePlugin } from './src/lib/server/gitBridge';
// …
plugins: [gitBridgePlugin(), localSourceBridgePlugin(), tailwindcss(), sveltekit()],
```

It answers `POST /__mcb/git/status | history | commit-files | commit-file-diff |
file-diff` and nothing else. **It cannot change a repository**, deliberately: a
browser tab left open on a checkout with work in progress must not be able to
commit it by accident. In a browser the panes therefore disable the commit box,
the stage buttons and the remote buttons, and say why in their hover text.

The desktop app never reaches the bridge — the wrappers try Tauri first.

### 4. `package.json` test entries

```json
"test:git-graph-lanes": "node --experimental-strip-types scripts/gitGraphLanes.test.mjs",
"test:git-bridge": "node --experimental-strip-types scripts/gitBridge.test.mjs",
"test:git-commit-files": "node --experimental-strip-types scripts/gitCommitFiles.test.mjs",
```

`test:git-bridge` runs git against this checkout, reading only.

### 5. Palette commands

Nothing new is needed: `shellCommands.ts:63-84` already registers refresh,
fetch, pull and push. If you want one for the new tab, "Show the diff" →
`showCenterPanel('diff')` is the obvious addition.

### 6. Per-commit files: built, not contracted

The plan left open whether the per-commit file list should be folded into
`gitService`/`gitPanelStore`. **It is built** as its own store and service
(`gitCommitFilesStore` + `gitCommitFilesService`) so those two tested files
stayed untouched. Fold it in later if you want one git service; nothing in the
shell depends on where it lives.

---

## Two things worth knowing

**A merge lists no files.** `read_git_commit_files` returns an empty list for a
merge commit, because a merge carries no changes of its own. An opened merge row
says so in a sentence; it never spins.

**Two path traps, both handled here.**

1. `read_git_commit_file_diff` takes a path RELATIVE to the repository, while
   the older `read_source_git_diff` takes the whole path on disk.
2. Both of them, and the file list, must be given the **top** of the repository.
   `git show <sha> -- <path>` matches the path against the folder git runs in,
   so asking from a subfolder returns nothing and reports success — a blank diff
   that reads exactly like "this file did not change". The panel is pointed at a
   session's working folder, which is often not the top, so
   `gitCommitFilesService` resolves the top once per repository (through
   `validate_project_root`, which works in the desktop app and in a browser) and
   every call uses it. The bridge does its own `rev-parse --show-toplevel`.

**Known limitation to carry forward (not fixed here).** The desktop commands
return a non-ASCII file name as octal escapes (`"\303\251.md"`), because
`src-tauri` does not pass `-c core.quotepath=false` to git. Handing that name
back to git matches nothing, so the diff comes back blank and looks clean. Until
the Rust side sets that option, such a row is shown dimmed and says "This file
has a name git could not print in plain letters, so its changes cannot be shown
here yet." rather than showing an empty diff. The bridge already passes the
option, so the browser side is correct. Suggested follow-up: add `-c
core.quotepath=false` to `run_git_text` and `project_git_status_sync` in
`src-tauri/src/main.rs`.
