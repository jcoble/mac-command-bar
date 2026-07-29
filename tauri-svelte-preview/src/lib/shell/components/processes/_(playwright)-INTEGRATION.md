# Playwright process card — wiring contract (Task 4)

Everything in this lane is new. Nothing shared was edited. Three small wiring
edits are left for the integrator; all three are listed below with exact
locations.

## Files this lane added

| File | What it is |
|---|---|
| `src/lib/shell/processes/playwrightStore.svelte.ts` | Runes state + the pure derivations (grouping by process group, plain-English kind names, age). No backend, no `$effect`. |
| `src/lib/shell/processes/playwrightService.ts` | The only place this card talks to the backend. `activate()`, `refresh()`, `stopSession(pgid)`, `stopAll()`, `resetPlaywrightActivation()`. |
| `src/lib/shell/processes/playwrightBackend.ts` | Own wrappers for `list_playwright_sessions`, `kill_playwright_sessions`, `kill_playwright_session` (`null` when not in the desktop app), plus `isMissingCommandError`. |
| `src/lib/shell/components/processes/PlaywrightCard.svelte` | The card. No props, no IO at mount. |
| `scripts/playwrightStore.test.mjs` | 22 assertions over grouping, kind labels, age, summary, stop-result wording, and the store's ticket/stop state. |

## 1. Mount the card in the context view

`ContextPanel.svelte` renders its five cards as five sibling
`<section class="card">` blocks inside `<div class="context-panel">`
(`src/lib/shell/components/ContextPanel.svelte:122-437`, in the order runs,
running processes, agent sessions, worktrees, repositories). The whole card list
sits inside the `{:else}` branch of the `{#if !contextState.activated}` guard at
line 118.

Slot the Playwright card **after "Running processes"** — that is, between the
`</section>` that closes the runtime card (line 242) and the agent-sessions
comment (line 244). It is the same family of thing, and it should be visible
without scrolling to the bottom.

```svelte
import PlaywrightCard from './processes/PlaywrightCard.svelte';
...
<PlaywrightCard />
```

Two notes:

- The card renders its own `<section class="card">` with its own scoped styles,
  copied from `ContextPanel`'s card chrome so it is visually indistinguishable —
  same head, chevron, count pill, row rhythm. It uses `var(--color-*)` tokens
  rather than repeating hex values, so Task 6's theme reaches it. If the
  integrator later restyles `ContextPanel` to tokens, nothing here changes.
- The card does **not** hide itself when `contextState.activated` is false — it
  has its own "nothing read yet" line. Placing it inside the existing `{:else}`
  branch is still correct and keeps the panel's single empty state.

## 2. Load it when the context view becomes visible (refresh policy)

**Never at launch.** Loads happen exactly twice: when the context view becomes
visible, and when the user asks.

`shellPanels.ts:53-58` has the context activator:

```ts
context: (selection) =>
  activateContextCards({ projects: ..., activeRoot: ... })
```

Add the Playwright load in the same activator (it is the callback
`panelActivation.ts` fires when the context view is both open and unfolded):

```ts
import { activate as activatePlaywright } from './processes/playwrightService.ts';
...
context: (selection) => {
  activateContextCards({ ... });   // unchanged
  activatePlaywright();
}
```

`activatePlaywright()` is idempotent: the first call reads, later calls cost
nothing. It takes no project — the process list is machine-wide, so it does not
reload when the selected project changes.

Optional, integrator's call: `shellCommands.ts:14` imports
`refreshAll as refreshContextCards`, and `ContextPanel`'s refresh button calls
`refreshAll()`. Adding `void refreshPlaywright()` next to those two makes the
panel-wide refresh cover this card too. The card already has its own refresh
button, so skipping this loses nothing.

If the shell ever tears the context region down, call
`resetPlaywrightActivation()` alongside `resetContextActivation()`.

## 3. Test script entry (integrator adds all lanes' entries at once)

```json
"test:playwright-store": "node --experimental-strip-types scripts/playwrightStore.test.mjs"
```

## Optional: palette command

If the integrator is adding palette commands anyway, a good one is
**"Show leftover Playwright processes"** → open the context view and call
`activatePlaywright()` / `refresh()`. Deliberately NOT "stop all from the
palette": stopping is always behind the card's confirmation dialog.

## Backend commands this card uses

| Command | Origin | Absence handling |
|---|---|---|
| `list_playwright_sessions` | already shipped | `null` outside the desktop app → the card says "This runs in the desktop app only." |
| `kill_playwright_sessions` | already shipped | same |
| `kill_playwright_session({ pgid })` | **Task 0** | If the running desktop build predates it, the per-session button disappears and one plain line explains that only stop-everything works here. |

How absence is detected, so nobody has to re-derive it: after a successful read,
the service asks the desktop app once with `PROBE_PGID = -1`. Process group ids
are never negative and the command refuses any group it did not just list, so
the probe cannot stop anything. A build that HAS the command answers "No
Playwright session is running in process group -1"; a build that does not
answers that the command was not found. The answer is remembered for the
session.

Two facts about Task 0's result shape that cost time to find:

- `PlaywrightCleanupResult.sessions` is the list of sessions the command was
  **asked** to stop, not what survived. It must not be used as the new list —
  the service re-reads after every stop.
- `terminatedPids` counts processes that were signalled, and `failedPgids`
  carries a per-process message; both are folded into one plain sentence by
  `describeCleanupOutcome`.

## Verification run in this lane

- `pnpm run check` — clean.
- `node scripts/checkSvelteNext.mjs` — 0 errors, 0 warnings in owned files.
- `node --experimental-strip-types scripts/playwrightStore.test.mjs` — 22 passed
  (red first: the store did not exist).
- `pnpm build` — green.

Not verified here (no mounting in a parallel lane): how the card looks on screen,
and a real stop against real Playwright processes. Both belong to the
integrator's Chrome walkthrough and the end-of-round desktop pass. The desktop
pass is the only place the per-session stop can be exercised, because it needs
Task 0 merged and the app restarted.
