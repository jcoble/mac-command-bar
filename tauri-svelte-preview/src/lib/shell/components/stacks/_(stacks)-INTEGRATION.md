# Stack runner — wiring for the integrator

Lane 3 of the panels wave (TSK-767). Everything below is NEW; this lane edited no shared file.

## Files this lane added

| File | What it is |
|---|---|
| `src/lib/shell/stacks/stackStore.svelte.ts` | Runes state + the pure derivation and tolerant-parse functions. localStorage is its only IO. |
| `src/lib/shell/stacks/stackService.ts` | The one place that reads `list_runtime_contexts`, and the place the page's three handlers are registered. |
| `src/lib/shell/components/stacks/StacksPane.svelte` | The pane. No props, no IO at mount, no `$effect`. |
| `scripts/stackStore.test.mjs` | 23 assertions: the four states, the folder match, the sentences, tolerant parsing, the session tag. |

`package.json` entry for the integrator to add (lanes do not touch it):

```json
"test:stack-store": "node --experimental-strip-types scripts/stackStore.test.mjs"
```

## 1. Where the pane goes: ROSTER VIEW (the pick)

**A view of its own in the tool column, not a card in the context region.** Reasons, in order:

1. A stack is a thing you *operate* — start, stop, add, remove — and the context region is a
   read-only "what is going on" surface. Putting buttons that spawn and kill processes inside
   a card next to five read-only cards makes the destructive ones easy to hit by accident.
2. The pane owns a form (name + command) and a per-row confirm step. That needs the full width
   and height of a view; squeezed into a card it would push the other five cards off screen.
3. The context region already shows "Running processes" — every listening port on the machine.
   Stacks are the ones *you started here*. Keeping them in a separate view is what makes that
   difference visible instead of two lists of ports in one column.

### Exact edits

**`src/lib/shell/layout/sidebarViews.ts`**

```ts
export type SidebarViewId = 'explorer' | 'source-control' | 'worktrees' | 'stacks' | 'context';

export const SIDEBAR_VIEWS: readonly SidebarView[] = [
  { id: 'explorer', title: 'Explorer' },
  { id: 'source-control', title: 'Source control' },
  { id: 'worktrees', title: 'Worktrees' },
  { id: 'stacks', title: 'Stacks' },
  { id: 'context', title: 'Context' }
];
```

`scripts/sidebarViews.test.mjs` asserts the roster; it needs the new id added there too.

**`src/lib/shell/components/ShellSidebar.svelte`**

- `PANES`: `stacks: { id: 'stacks', title: 'Stacks' },`
- `GATED_VIEWS`: add `'stacks'` — the pane must read the machine only while it is in view.
- new prop `onStacksVisible?: (visible: boolean) => void;` and, in `reportView`,
  `else if (id === 'stacks') onStacksVisible?.(visible);`
- parking stage: `<div class="slot" bind:this={bodies.stacks}><StacksPane /></div>`
  (`import StacksPane from './stacks/StacksPane.svelte';`)

**`src/lib/shell/components/ActivityBar.svelte`** — `ICONS` needs a `stacks` entry.
Suggested lucide icon: `Server` (or `Play`); it is a keyed `Record<SidebarViewId, …>`, so the
build breaks until one is added, which is the intent.

### Activation (the pane must not load at launch)

Lowest-touch route — no `panelActivation.ts` change:

`src/routes/next/+page.svelte`, alongside `onContextVisible` (line 652):

```svelte
onStacksVisible={(visible) => {
  if (!visible || !shellPanels.loadsAllowed()) return;
  const selection = readSelection();          // already exported from shellPanels.ts
  activateStacks({ activeRoot: selection.root.trim() || null });
}}
```

`activateStacks` is idempotent: opening the view again on the same project does no backend work.
If you would rather have it symmetrical with the other gated views, add `stacks(selection)` to
`PanelActivators` + `stacksVisible(visible)` to `PanelActivation` and give it `contextVisible`'s
body verbatim — that also gets the "re-read when the session changes" behaviour for free, at the
cost of touching `panelActivation.ts` and its test.

Also call `activateStacks({ activeRoot })` again from wherever the shell notices the selected
session changed, if you took the low-touch route — otherwise the pane keeps showing the project
it was opened on until the user presses refresh. (It shows the folder name in its header, so it
is never lying about which project it is listing.)

### Palette command

In `shellCommands.ts`, next to `show-git`:

```ts
{
  id: 'show-stacks',
  label: 'Show stacks',
  detail: 'Open the stacks section of the tool column on the right',
  perform: () => hooks.selectView('stacks')     // ShellSidebar already exposes selectView
},
{
  id: 'stacks-refresh',
  label: 'Stacks: refresh',
  detail: 'Look again at which ports the stacks have open',
  perform: () => void refreshStacks()
}
```

`ShellCommandHooks` currently has `showPanel` and `expandSourceControl`; `selectView(id)` is
already handed to the page by `ShellSidebar`'s `onReady` — pass it through as a third hook.

## 2. The three page handlers (exact signatures)

The service NEVER spawns or kills anything itself — the page owns the rail, the terminal service
and the terminal hosts. Register once from the page's `onMount`, after `service` exists:

```ts
import { registerStackHandlers, noteTerminalExit, noteSessionRemoved } from '$lib/shell/stacks/stackService';

registerStackHandlers({
  onStartStack,     // (request: StackStartRequest) => Promise<string | null>
  onStopStack,      // (ownedId: string) => Promise<void>
  onSelectSession   // (ownedId: string) => void | Promise<void>
});
```

### `onStartStack({ stackId, cwd, script, title }) => Promise<string | null>`

```ts
export interface StackStartRequest {
  stackId: string;   // the saved stack; carry it if you want it, the service tags the session itself
  cwd: string;       // folder to run in
  script: string;    // the command line, exactly as the user typed it
  title: string;     // what the rail should call the session — the stack's name
}
```

Answer with the new session's `ownedId`, or `null` when no terminal could be opened (the pane
then says "Could not start "Web": no terminal opened." — it does not guess). Throwing is fine
too; the message is shown.

This is `createFreshSession`'s wiring (it has had zero callers). Body, mirroring `adopt`
(`+page.svelte:341-359`):

```ts
async function onStartStack(request: StackStartRequest): Promise<string | null> {
  if (!service || disposed) return null;
  const owned = { ...createFreshSession({ cwd: request.cwd, title: request.title }),
                  resumeCommand: request.script };
  addOwnedSession(owned);
  const host = await hostFor(owned.ownedId);
  if (!host) return null;
  const ptySessionId = await service.startOwned(owned, host);
  if (!ptySessionId) {
    updateOwnedSession(owned.ownedId, { state: 'exited' });
    return null;
  }
  updateOwnedSession(owned.ownedId, { ptySessionId, state: 'live' });
  await selectOwned(owned.ownedId);
  return owned.ownedId;
}
```

**Why `resumeCommand`:** `startOwned` types `${owned.resumeCommand}\r` into the fresh shell
(`terminalService.ts:541-543`). That is the existing write-command-into-shell path and it needs
zero Rust, which is why v1 uses it.

**The Task 0 upgrade, once its branch is merged.** `TerminalStartRequest` gains
`command?: string`, which spawns `shell -lc <command>` and therefore gives a REAL exit code
instead of a shell that stays alive after the command dies. Switching costs one line in
`terminalService.startOwned` — `backend.start({ cwd, ownedId, command })` — but that is a
SHARED file this lane may not touch, and the change affects every caller, so it is left to you.
Until it lands, "failed" only appears when the whole shell exits non-zero; a dev server that
crashes and drops the user back to a prompt reads as "started, no port yet" rather than failed.
**That is the one honesty gap in this lane and it is worth closing this round if Task 0 merges.**

### `onStopStack(ownedId: string) => Promise<void>`

Straight to the page's existing `closeTerminal(ownedId)` (`+page.svelte:487`). It never rejects,
it ends the PTY, and the session stays in the rail as a finished row — which is what the pane
wants: stopping a stack is not throwing the work away.

### `onSelectSession(ownedId: string) => void | Promise<void>`

The page's existing `selectOwned(ownedId)` (`+page.svelte:319`). This is the row click-through:
a running stack IS a session, so clicking it shows its terminal.

## 3. How a terminal exit reaches this service (no polling anywhere)

The path already exists end to end:

1. `src-tauri/src/terminal.rs` emits `terminal_output` with
   `{ sessionId, data, terminated, exitCode, signal }` (`TerminalOutputEvent`, terminal.rs:48-56).
2. `listenToTerminalOutput` (`src/lib/tauriSource.ts:478-488`) is the ONE listener.
3. `terminalService.ts:501` calls the `onExit(ownedId, payload)` hook it was built with
   (`createTerminalService` opts, `terminalService.ts:210`).
4. The page passes exactly one function there today (`+page.svelte:546`):
   `onExit: (ownedId) => updateOwnedSession(ownedId, { state: 'exited' })`.

**Fan it out — one added line, no shared-module change beyond the page:**

```ts
onExit: (ownedId, payload) => {
  updateOwnedSession(ownedId, { state: 'exited' });
  noteTerminalExit(ownedId, { exitCode: payload.exitCode, signal: payload.signal });
}
```

`noteTerminalExit` ignores every session that is not a stack's (it costs one map lookup), and for
one that is, it writes the exit down and re-reads the ports once. That is the third and last
moment this lane touches the backend: **pane opened, refresh pressed, terminal ended. No timers.**

Optional, same shape: in `removeSession` (`+page.svelte:521`) call
`noteSessionRemoved(ownedId)` so a removed session drops its stack tag instead of leaving a
saved tag pointing at a row that is gone. (Harmless if skipped — `hydrateStacks` drops tags whose
stack no longer exists, and a stale tag only means the row offers Stop until it is started again.)

## 4. The session tag (`kind: 'agent' | 'stack'`)

`OwnedSession` is shared, so the tag lives in this lane's store, keyed by `ownedId` and saved
under this lane's own key. To badge a stack session in the rail:

```ts
import { isStackSession, stackNameForOwnedId } from '$lib/shell/stacks/stackStore.svelte';
// in SessionsColumn: {#if isStackSession(session.ownedId)}<span class="badge">stack</span>{/if}
// tooltip: stackNameForOwnedId(session.ownedId)
```

If you decide to fold a real `kind` field into `OwnedSession` later, `stackIdForOwnedId` is the
single lookup that has to move; nothing else in this lane reads it.

## 5. Storage keys (both this lane's own)

| Key | Holds |
|---|---|
| `mac-command-bar.next.stacks.definitions` | The saved stacks: `{id, name, script, cwd}[]`. |
| `mac-command-bar.next.stacks.runs` | Which session belongs to which stack, and how it ended. |

Both are read once by `hydrateStacks()` (called by `activateStacks`) and tolerate anything:
unreadable JSON, a value that is not a list, rows missing fields, duplicate ids. A tag naming a
stack that no longer exists is dropped as it is read.

## 6. Things worth knowing before you wire it

- **`list_runtime_contexts` only sees LISTENING sockets**, and it drops any process whose folder
  is not underneath one of the folders it is given. The service builds that folder list from the
  saved stacks themselves (`projectsToScan`), so it does not depend on the rail's project list.
- **Ports are matched to a stack by folder, never by pid** — the backend never says which terminal
  spawned a process, and a stack's shell spawns children with their own pids. Two stacks saved in
  the SAME folder will both show that folder's ports. The row says the port is in this stack's
  folder; it does not claim more than that.
- **"started, no port yet" is a real resting state**, not a spinner: a stack that never opens a
  port (a watcher, a build) sits there for as long as it runs. The sentence says exactly that.
- On the web (no desktop app) the pane shows *"Running processes can only be read in the desktop
  app."* and still lets stacks be saved — the saved list is browser storage, not Tauri.
- No `window.confirm` anywhere: removing a saved stack is a two-press inline confirm
  ("Really remove?"), so no AlertDialog needs vendoring for this lane.

## 7. Verification this lane ran

- `pnpm run check` — clean.
- `node scripts/checkSvelteNext.mjs` — 0 errors, 0 warnings in the /next shell (checked by
  deliberately breaking a type in `StacksPane.svelte` first, to prove the gate actually sees the
  new file).
- `node --experimental-strip-types scripts/stackStore.test.mjs` — 23 passed, written red-first.
- `pnpm build` — green.
- Not mounted anywhere, so there are no visuals to check until you wire it. The pane is styled
  against `ContextPanel.svelte`'s palette so it lands in the same column looking like a neighbour.
