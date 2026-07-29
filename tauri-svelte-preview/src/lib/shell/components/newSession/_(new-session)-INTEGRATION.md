# New session — wiring contract (Task 5, panels wave)

Everything in this lane is new files. Nothing shared was touched. This file is
the exact list of edits the integrator makes, and it is scratch — delete it once
the wiring lands.

## What this lane shipped

| File | What it is |
|---|---|
| `src/lib/shell/newSession/newSessionFlow.ts` | Pure rules: the launch catalog, the command preview, validation, the folder-list merge, the `git worktree add` line, and `buildNewSessionRequest`. No imports beyond types. |
| `src/lib/shell/newSession/projectRootsStore.svelte.ts` | Runes state for the project folders. localStorage only, no `$effect`, explicit `persist()`. |
| `src/lib/shell/newSession/newSessionBackend.ts` | The only backend calls: the system folder chooser, `validate_project_root`, `list_project_worktrees`. Each returns `ok` / `unavailable` / `failed`, each counted with `countInvoke`. |
| `src/lib/shell/components/newSession/NewSessionDialog.svelte` | The screen. Takes `bind:open` and `onStart`; exports `reset({ sessionRoots })`. |
| `src/lib/shell/components/newSession/NewSessionHost.svelte` | Lazy mount, same shape as `SettingsHost.svelte`. Exports `open({ sessionRoots })`, `close()`, `isOpen()`. |
| `scripts/newSessionFlow.test.mjs` | 20-odd assertion blocks over the pure module. |

Verified on this branch: `pnpm run check` clean, `node scripts/checkSvelteNext.mjs`
clean (0 errors, 0 warnings in owned files), `pnpm build` green, the Node test
green after being red first. The dialog has NOT been mounted or looked at in a
browser — that happens at integration, as the wave planned.

## 1. `package.json` — the test script (integrator-only file)

```json
"test:new-session-flow": "node --experimental-strip-types scripts/newSessionFlow.test.mjs",
```

## 2. Where the dialog lives: `ShellOverlays.svelte`

Precedent to copy exactly: `SettingsHost` in
`src/lib/shell/components/ShellOverlays.svelte:15,28-35,43`. The overlays file is
where a screen that must exist exactly once already lives, and both entry points
below (a button in the sessions column, a palette command) need to reach the same
instance.

```svelte
  import NewSessionHost from './newSession/NewSessionHost.svelte';
  import type { NewSessionRequest } from '$lib/shell/newSession/newSessionFlow';

  interface Props {
    …existing props…
    /** Start the session the new-session dialog described. */
    onStartNewSession: (request: NewSessionRequest) => void | Promise<void>;
    /** The folders the sessions on the rail are running in. */
    newSessionRoots: string[];
  }

  let newSessionHost: { open: (input?: { sessionRoots?: string[] }) => void } | null = null;

  /** Open the new-session dialog from outside — the button is in the sessions
   * column and the palette command is registered by the page. */
  export function openNewSession(): void {
    newSessionHost?.open({ sessionRoots: newSessionRoots });
  }
```

```svelte
<NewSessionHost bind:this={newSessionHost} onStart={onStartNewSession} />
```

## 3. The page handler — `src/routes/next/+page.svelte`

This is the wiring `createFreshSession` was written for; it has had zero callers
until now. Put it next to `adopt()` (`+page.svelte:334`), which it deliberately
mirrors step for step.

```ts
  import { createFreshSession } from '$lib/shell/ownedSessions';
  import type { NewSessionRequest } from '$lib/shell/newSession/newSessionFlow';

  /** EXPLICIT IO: start a session the user described in the new-session dialog. */
  async function startNewSession(request: NewSessionRequest): Promise<void> {
    if (!service || disposed) return;
    const owned = {
      ...createFreshSession({ cwd: request.cwd, title: request.title }),
      // createFreshSession makes a plain shell: agent 'other', no command to
      // replay. The dialog knows better on both counts, and BOTH must be set
      // before startOwned — that is the call that types the command in.
      agent: request.agent,
      resumeCommand: request.command
    };
    addOwnedSession(owned);
    const host = await hostFor(owned.ownedId);
    if (!host) {
      rail.error = `no terminal host for "${owned.title}"`;
      return;
    }
    const ptySessionId = await service.startOwned(owned, host);
    if (!ptySessionId) {
      updateOwnedSession(owned.ownedId, { state: 'exited' });
      rail.error = `failed to start a terminal for "${owned.title}"`;
      return;
    }
    // Persist the PTY id: reload re-attach reads it back out of localStorage.
    updateOwnedSession(owned.ownedId, { ptySessionId, state: 'live' });
    await selectOwned(owned.ownedId);
  }
```

`updateOwnedSession` and `selectOwned` are already imported in the page;
`createFreshSession` is not.

Mount props on `<ShellOverlays>` (`+page.svelte:720`):

```svelte
    onStartNewSession={startNewSession}
    newSessionRoots={rail.owned.map((session) => session.cwd)}
```

Three notes on the handover, all deliberate:

- `request.command` is `null` for "just a terminal", which is exactly the value
  `OwnedSession.resumeCommand` already uses for nothing-to-replay, so it goes
  straight in with no translation.
- `request.cwd` is already normalized (trimmed, no trailing slash) and
  `request.title` is already resolved to the shown name. Pass them through; do
  not re-derive.
- `projectPath` is left as `createFreshSession` leaves it (`null`). Owned rows do
  not group by it, and `shellPanels.readSelection()` derives projects from
  session `cwd`s. Set it to `request.cwd` only if something later wants it.

## 4. "+ New session" in the sessions column header

`SessionsColumn.svelte` is presentational, so it gets a callback prop and nothing
else. Add to `Props` (near `onRescan`, `:122`):

```ts
    /** Open the new-session dialog. */
    onNewSession(): void;
```

destructure it with the rest (`:140`), and add the button in the expanded
header (`:631-647`), left of the fold action, using the header's own `action`
snippet:

```svelte
        <div class="ml-auto flex items-center gap-1">
          {@render action('start a new session', 'New session', Plus, false, () => onNewSession())}
          {@render action(
            'fold the sessions column up',
            'Fold this column up',
            PanelLeftClose,
            false,
            () => onCollapse(true)
          )}
        </div>
```

`Plus` comes from `@lucide/svelte`, added to the existing icon import. The folded
strip (`:589`) can carry the same button if wanted; not required.

Page side (`+page.svelte:638`):

```svelte
    onNewSession={() => overlays?.openNewSession()}
```

## 5. Palette command

`shellCommands.ts` — one more hook and one more entry:

```ts
export interface ShellCommandHooks {
  …existing…
  /** Open the new-session dialog. */
  openNewSession(): void;
}
```

```ts
    {
      id: 'new-session',
      label: 'Start a new session',
      detail: 'Pick a project folder and start an agent in it',
      perform: () => hooks.openNewSession()
    },
```

and at the page's `registerShellCommands({…})` call (`+page.svelte:128`):

```ts
    openNewSession: () => overlays?.openNewSession()
```

`overlays` is already the page's `bind:this` handle on `ShellOverlays`; its type
annotation there needs `openNewSession` adding alongside `openSettings`.

## 6. Storage keys this lane owns

| Key | Holds |
|---|---|
| `mac-command-bar.next.new-session-custom-roots` | The project folders the user added by hand. |
| `mac-command-bar.next.new-session-last-root` | The folder the last session was started in, so the dialog opens there again. |

No existing key is read or written.

## 7. Things worth knowing

- **This lane never creates a worktree.** The checkout picker lists what
  `list_project_worktrees` already reports, with the project's own checkout
  first. "I need a new worktree" shows a copyable `git worktree add …` line and
  does nothing else. Keep it that way.
- **The system folder chooser works.** `@tauri-apps/plugin-dialog` is in
  `package.json` (`~2.7.1`, installed) and `dialog:default` is already in
  `src-tauri/capabilities/default.json`, so `open({ directory: true })` needs no
  new permission. In the browser the button is not rendered at all and the typed
  path box is the way in.
- **The built-in project list is two hard-coded paths** (`defaultProjectRoots`,
  `sourceData.ts:520`). That is why the picker also learns folders from the
  sessions on the rail and lets the user add their own — a cold /next with no
  sessions and nothing added shows an honest "No project folders yet" line
  rather than a broken picker.
- **Launch commands are the bare interactive ones**, taken from the tools on this
  machine on 2026-07-29: `claude --help` — "starts an interactive session by
  default"; `codex --help` — "if no subcommand is specified, options will be
  forwarded to the interactive CLI". No flags. The command box in the dialog is
  editable, so anything else is one keystroke away.
- The dialog closes itself as soon as `onStart` resolves, and shows whatever
  `onStart` throws without closing. So the handler above can be `void`-returning
  and still report failures on `rail.error` as it does today.

## 8. Not done here, for whoever wants it later

- A "recent folders" list, and remembering the last agent as well as the last
  folder.
- Grouping the picker's folders by where they came from (built in / added /
  from a session) — the source is on every row (`KnownRoot.source`), nothing
  reads it yet.
- Starting a session in a folder that is not a git repository is allowed on
  purpose; `validate_project_root` is used to catch typos, not to refuse.
