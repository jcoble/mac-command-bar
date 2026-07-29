# Worktree manager — wiring instructions for the integrator

Everything below is a shared-file edit this lane was not allowed to make. Nothing
in the lane's own files needs changing; they compile and type-check as they
stand (`pnpm run check` and `node scripts/checkSvelteNext.mjs` are clean).

Delete this file once the wiring has landed — it is scratch, and the report in
`.superpowers/sdd/2026-07-29-panels-wave/task-2-report.md` keeps the record.

## What this lane shipped

| File | What it is |
|---|---|
| `src/lib/shell/worktrees/worktreeManagerRows.ts` | PURE. Joins worktrees + repository summaries + sessions into rows, and every sentence a row shows. Tested. |
| `src/lib/shell/worktrees/worktreeManagerStore.svelte.ts` | Runes value bag. No backend, no `$effect`, nothing persisted. |
| `src/lib/shell/worktrees/worktreeManagerService.ts` | Every desktop call, imperative, each one counted. |
| `src/lib/shell/worktrees/worktreesBackend.ts` | This lane's own `invoke` wrappers; `null` when not in the desktop app. |
| `src/lib/shell/components/worktrees/WorktreeManagerPane.svelte` | The view. No required props. |
| `src/lib/shell/components/worktrees/WorktreeRow.svelte` | One worktree. |
| `src/lib/shell/components/worktrees/WorktreeDetail.svelte` | The open row: verdict, sessions, three copyable commands. |
| `src/lib/shell/components/worktrees/ForceRemoveDialog.svelte` | The one dialog that can lose work. |
| `scripts/worktreeManager.test.mjs` | The pure joins and wordings. |

## 1. `package.json` — the test script (integrator adds all lanes' entries at once)

```json
"test:worktree-manager": "node --experimental-strip-types scripts/worktreeManager.test.mjs"
```

## 2. `ShellSidebar.svelte` — the view stops being a placeholder

The roster in `layout/sidebarViews.ts` needs **no change**: `{ id: 'worktrees',
title: 'Worktrees' }` is already the third entry, and `PANES.worktrees` already
exists. What changes is what gets parked into it, plus the gating below.

```diff
-  import PanelPlaceholder from './PanelPlaceholder.svelte';
+  import WorktreeManagerPane from './worktrees/WorktreeManagerPane.svelte';
```

(`PanelPlaceholder` keeps its other use if it has one; at the time of writing the
worktrees slot is its only one, so the import may go entirely.)

```diff
-  <div class="slot" bind:this={bodies.worktrees}>
-    <PanelPlaceholder name="Worktrees" hint="Worktree health and cleanup will live here." />
-  </div>
+  <div class="slot" bind:this={bodies.worktrees}>
+    <WorktreeManagerPane onOpenSession={(ownedId) => onOpenSession?.(ownedId)} />
+  </div>
```

`onOpenSession` is optional. Wired, the sessions listed under an open worktree
become buttons that focus that session; left out they are plain text and nothing
breaks. Add the prop to `ShellSidebar`'s `Props`:

```ts
/** Focus one of the shell's sessions, from a worktree's session list. */
onOpenSession?: (ownedId: string) => void;
```

and in `+page.svelte`: `onOpenSession={(ownedId) => void selectOwned(ownedId)}`.

## 3. Loading gate — the same route source control takes

The pane loads **only while it is on screen**, which is the third route described
in the header of `panelActivation.ts` ("Two panels are neither a tab nor always
on screen" — now three). Copy the `context` case exactly; it is the closer of the
two, because it takes a whole selection rather than a bare root.

### `panelActivation.ts`

```diff
 const GATED = ...            // (no such constant; the three below are fields)
 export interface PanelActivators {
   editor(root: string | null): void;
   git(root: string | null): void;
   browser(): void;
   explorer(root: string): void;
   context(selection: ProjectSelection): void;
+  worktrees(selection: ProjectSelection): void;
 }
```

```diff
 export interface PanelActivation {
   ...
   contextVisible(visible: boolean): void;
+  /** The worktree manager came into view, or went out of it. Same contract. */
+  worktreesVisible(visible: boolean): void;
   loadedPanels(): string[];
 }
```

Inside `createPanelActivation`, add the three lines that mirror the context
cards — a `worktreesInView` flag, a `worktreesLoadedFor` marker, a
`loadWorktrees(selection)` that sets the marker and calls the activator, a call
to it from `loadSessionPanels` (`if (worktreesInView) loadWorktrees(selection)`),
the `worktreesVisible` method itself (identical body to `contextVisible`), and
`if (worktreesLoadedFor !== null) loaded.push('worktrees')` in `loadedPanels`.

`scripts/panelActivation.test.mjs` will need the same three cases the context
cards have; this lane did not touch that file.

### `shellPanels.ts`

```diff
+import { activate as activateWorktrees } from './worktrees/worktreeManagerService.ts';
+import type { WorktreeSessionInput } from './worktrees/worktreeManagerRows.ts';
+
+/** The rail's sessions in the shape the worktree manager joins on. */
+function worktreeSessions(): WorktreeSessionInput[] {
+  return rail.owned.map((session) => ({
+    ownedId: session.ownedId,
+    title: session.title,
+    cwd: session.cwd,
+    projectPath: session.projectPath,
+    state: session.state
+  }));
+}
```

```diff
     context: (selection) =>
       activateContextCards({ ... }),
+    worktrees: (selection) =>
+      activateWorktrees({
+        root: selection.root.trim() || null,
+        projectName: folderName(selection.root.trim()),
+        sessions: worktreeSessions()
+      })
```

`activate` is idempotent: pointed at the same folder with the same sessions it
does no work at all, so calling it every time the view is shown is free.

### `ShellSidebar.svelte`

```diff
-  const GATED_VIEWS: readonly SidebarViewId[] = ['source-control', 'context'];
+  const GATED_VIEWS: readonly SidebarViewId[] = ['source-control', 'context', 'worktrees'];
```

```diff
     if (id === SOURCE_CONTROL) onSourceControlVisible?.(visible);
     else if (id === 'context') onContextVisible?.(visible);
+    else if (id === 'worktrees') onWorktreesVisible?.(visible);
```

plus the `onWorktreesVisible?: (visible: boolean) => void;` prop, destructured
with the others.

### `+page.svelte`

```diff
     onContextVisible={(visible) => shellPanels.contextVisible(visible)}
+    onWorktreesVisible={(visible) => shellPanels.worktreesVisible(visible)}
```

## 4. Palette command

In `shellCommands.ts`, add one hook and one command. The hook opens the view; the
command is the entry point somebody types when they have not got the tool column
open at all.

```diff
 export interface ShellCommandHooks {
   showPanel(id: string): void;
   expandSourceControl(): void;
+  /** Open the Worktrees view of the tool column. */
+  showWorktrees(): void;
 }
```

```diff
+    {
+      id: 'worktrees-clean-up',
+      label: 'Clean up worktrees',
+      detail: 'Open the worktree list and see which folders are safe to remove',
+      perform: () => hooks.showWorktrees()
+    },
```

and in `+page.svelte`'s `registerShellCommands({ … })`:

```diff
+    showWorktrees: () => sidebarControls?.selectView('worktrees')
```

`selectView` already reports the view change, which is what triggers the load —
no extra activate call is needed here.

## 5. Backend it depends on

- `list_project_worktrees`, `archive_project_worktree`, `list_git_repository_summaries` — all already shipped.
- `remove_project_worktree({ root, path, force? })` — the `force` half is the backend lane's (`tsk-344-763-764-770-backend`). Without it the everyday Remove still works; the destructive button stays switched off.
- `read_backend_capabilities()` → `string[]` — the backend lane is adding it. The destructive button is enabled **only** when that list contains `worktreeForceRemove`. A build that does not have the command answers with an error, which `worktreesBackend.ts` catches and reads as "it cannot". **Do not** replace this with a probe that tries the forced remove: an older build silently drops the unknown `force` field and performs an ordinary remove instead, so trying it cannot tell the two builds apart.

## 6. Things deliberately left for later

- The action buttons use plain `title` sentences rather than the shell's hover
  cards, because a switched-off button never fires the pointer events a hover
  card listens for — and the switched-off case is exactly when the explanation
  matters. Upgrading them to `Tooltip` around a wrapper element is fine, but it
  needs a browser check first (this lane could not mount anything).
- Disk size per worktree is not shown: the only code that measures it is
  `core/src/scanners/worktrees.rs`, which nothing calls.
- The pane reads on activation and on Refresh, never on a timer.
