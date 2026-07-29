# Themes — wiring instructions for the integrator

This lane built the theme registry, the service that applies a theme, and the
drift tests. It edited no shared file. Everything below is the wiring, in the
order it makes sense to do it.

**The rule that governs all of it:** Houston is the theme the app ships with and
it is a value-for-value copy of what the app renders today, so after this wiring
the app must look exactly as it does now until somebody picks Dracula. If any
step below changes a color on first load, something went wrong.

## Files this lane added

| File | What it is |
|---|---|
| `src/lib/shell/themes/themeRegistry.ts` | The themes. Each one is a set of `--color-…` values, a Monaco theme, and an xterm theme. Plain data, no imports. |
| `src/lib/shell/themes/themeService.ts` | `apply(themeId)`, `applyStoredTheme()`, and the two places Monaco and xterm hand in their repaint functions. |
| `src/lib/shell/styles/themeChrome.css` | Four colors the dock needs that the shared token file has no name for. |
| `scripts/themeRegistry.test.mjs` | Completeness plus three drift checks: Houston against `nextTokens.css`, against `sourcePreviewAppearance.ts`, and against the terminal factory's palette. |

## 1. `package.json` — the test entry

```json
"test:theme-registry": "node --experimental-strip-types scripts/themeRegistry.test.mjs"
```

## 2. Load the extra chrome colors

`src/routes/next/+page.svelte`, right after the existing `nextTokens.css` import
(line 15):

```ts
import '$lib/shell/styles/themeChrome.css';
```

This has to be a stylesheet rather than something the service sets, because the
dock is painted on the first frame, before any theme has been applied. Without
it the four `--dv-…` edits in step 6 would resolve to nothing for one frame.

## 3. Apply the remembered theme once, at mount

`src/routes/next/+page.svelte`, inside the existing `onMount` (line 535) — first
statement, before the terminal work, because it is synchronous and touches only
the DOM:

```ts
import { applyStoredTheme } from '$lib/shell/themes/themeService';

onMount(() => {
  applyStoredTheme();
  disposed = false;
  ...
});
```

`applyStoredTheme()` finds the `.next-shell` element itself. It does not rewrite
the stored setting, so a settings file that still says `dark` (the value every
existing install has) renders as Houston and stays as it is until the user picks
something.

## 4. The Appearance select in `SettingsDialog.svelte`

Two edits. Replace the hardcoded item list at lines 39-43:

```ts
// was: const themeItems = [{ value: 'dark', ... }, { value: 'light', ... }, { value: 'houston', ... }];
import { apply as applyTheme, themeChoices } from '$lib/shell/themes/themeService';

const themeItems = themeChoices();
```

`themeChoices()` returns `{ value, label, description }` per theme, so the
existing `labelFor(themeItems, …)` helper and the `{#each}` keep working
unchanged.

Then make the select actually do something (lines 132-142). Today it only writes
the setting; it needs to apply as well. Keep the bound value so the trigger
label stays correct, and add the apply on change:

```svelte
<Select.Root
  type="single"
  value={themeItems.some((item) => item.value === settings.appearance.themeId)
    ? settings.appearance.themeId
    : 'houston'}
  onValueChange={(value) => applyTheme(value)}
>
  <Select.Trigger class="w-full">
    {labelFor(themeItems, settings.appearance.themeId)}
  </Select.Trigger>
  <Select.Content>
    {#each themeItems as item (item.value)}
      <Select.Item value={item.value} label={item.label} />
    {/each}
  </Select.Content>
</Select.Root>
```

Note the value expression: every existing install has `dark` stored, which is no
longer an option, and an unmatched value leaves the select showing nothing.
`applyTheme` writes the setting itself (through `updateSettings`), so
`bind:value` is not needed and would fight it.

Two more things about this dialog:

- **"Reset section" on Appearance** (footer, line 350) puts `themeId` back to
  `dark` via `resetSettings`. Call `applyTheme(settings.appearance.themeId)`
  after a reset so the screen follows, or the settings say one thing and the
  screen shows another. It resolves to Houston, which is the right answer.
- **The Terminal tab's own theme select** (lines 55-60, bound to
  `settings.terminal.theme`) is read by nothing, before or after this lane. The
  terminal now follows the app theme. Either remove that select or leave it as
  it is for a later slice — but do not wire it to the registry, because the two
  settings would then disagree about what the terminal looks like.

## 5. The code editor

`src/lib/MonacoSourceEditor.svelte`. Three sites name the theme today; all three
have to move to the registry, and one becomes the registration point.

**(a) `configureMonaco(monaco)`, line 380-386 — the registration point.** It
already receives the Monaco API and already calls `defineTheme` once. Replace
its body with a registration, so every theme is defined and every later switch
lands here too:

```ts
import { listThemes } from '$lib/shell/themes/themeRegistry';
import { registerMonacoApplier } from '$lib/shell/themes/themeService';

function configureMonaco(monaco: typeof Monaco) {
  for (const theme of listThemes()) {
    const { id, ...definition } = theme.monaco;
    monaco.editor.defineTheme(id, {
      ...definition,
      rules: definition.rules ?? [],
      colors: definition.colors ?? {}
    });
  }
  // Called at once with the theme in force, and again on every switch.
  disposeThemeApplier?.();
  disposeThemeApplier = registerMonacoApplier((theme) => {
    monaco.editor.setTheme(theme.monaco.id);
  });
}
```

Keep the returned unregister function in a module/component variable and call it
in `onDestroy`, or a torn-down editor stays in the set. `setTheme` is global to
Monaco, so registering once per editor instance is harmless but pointless —
registering in `configureMonaco` (which runs once per Monaco load) is the right
scope.

**(b) `applyAppearance()`, lines 1413 and 1415** — both say
`sourcePreviewAppearance.theme.id`, which is the string `'houston'`. This
function re-runs whenever the user changes the editor font, so leaving it would
silently switch the editor back to Houston mid-session. Both become:

```ts
theme: currentTheme().monaco.id,
// and
monacoApi.editor.setTheme(currentTheme().monaco.id);
```

**(c) editor creation, line 2086** — same substitution, `theme:
currentTheme().monaco.id`.

**(d) `editorBackground`, line 345** is a module constant read from Houston's
`editor.background` and written into the host element's style at line 2377. It
is the color behind the editor before Monaco paints. Make it follow too:

```ts
const editorBackground = $derived(
  currentTheme().monaco.colors['editor.background'] ?? '#17191e'
);
```

Left alone it is a one-frame flash of Houston's background on a Dracula editor —
small, but this is the kind of thing that reads as a bug.

**About `sourcePreviewAppearanceKey`** (`sourcePreviewAppearance.ts:148): it is a
JSON string built once at module load out of the frozen Houston values,
including the whole theme object. It can therefore never change, so it can never
signal a theme switch. Today nothing is harmed by that — the old shell passes it
as `appearanceKey` at `+page.svelte:16354` and `MonacoSourceEditor` declares no
such prop, so it goes nowhere. **Do not** try to make it theme-aware: any value
computed at module load has the same problem. The registration seam in (a) is
the live path. If a future change starts using that key to decide "appearance
unchanged, skip the update", the theme id has to be added to it as a separate
input, not merged into that constant.

## 6. The terminals

`src/lib/shell/xtermFactory.ts`. The factory builds one terminal per owned
session and the shell keeps them all alive, so a switch has to walk every one of
them, not just the visible one.

Delete the local `DRACULA_THEME` constant (lines 43-65) — the registry now holds
those exact values, and `scripts/themeRegistry.test.mjs` reads this file to
prove it, so **update that test's parsing if the constant moves** (it looks for
`const DRACULA_THEME = { … } as const;`). Simpler and safer: leave the constant
where it is, unused, or keep it and let the test keep reading it. Either way:

```ts
import { currentTheme, registerTerminalApplier } from './themes/themeService';

/** Every terminal this factory has built and not yet disposed. */
const liveTerminals = new Set<import('@xterm/xterm').Terminal>();

registerTerminalApplier((theme) => {
  for (const terminal of liveTerminals) {
    terminal.options.theme = { ...theme.terminal };
  }
});
```

then in `makeTerminalView`:

- construction (line 139): `theme: { ...currentTheme().terminal }`
- after `terminal.open(host)`: `liveTerminals.add(terminal);`
- in `dispose()` (line 233): `liveTerminals.delete(terminal);`

The WebGL renderer picks up an options change on its own; if a terminal that was
hidden during the switch comes back with the old colors, hiding and showing it
re-acquires the context and repaints. Worth a look during the Chrome walkthrough
with two sessions open.

**Note, and it is deliberate:** Houston's terminal colors ARE Dracula's. The
terminal in this app has been Dracula-colored since long before themes existed,
in both shells, so Houston keeps exactly that — otherwise turning themes on
would repaint every terminal on screen. The consequence is that switching
between the two themes changes the chrome and the editor but not the terminal.
Giving Houston a terminal palette of its own is a real design decision and a
visible change; it belongs in its own change, with the user looking.

## 7. The dock colors — the exact edit list

dockview paints itself from its own `--dv-…` names, and those are set to colors
typed straight into two component stylesheets, which is why the dock has never
followed anything. Every one below becomes a `var(--color-…)` reference. **Every
replacement is the same color it is today**, so this changes nothing on screen
until a theme is picked.

`scripts/themeRegistry.test.mjs` checks this list: while any of these literals
is still in a component, it must appear in this file; once this file is deleted
(as contracts are, after wiring) the test requires that none is left.

### `src/lib/shell/components/ShellFrame.svelte` (the rule at lines 182-208)

| Line | Current | Replacement |
|---|---|---|
| 185 | `--dv-group-view-background-color: #101014;` | `--dv-group-view-background-color: var(--color-bg);` |
| 186 | `--dv-tabs-and-actions-container-background-color: #101014;` | `--dv-tabs-and-actions-container-background-color: var(--color-bg);` |
| 187 | `--dv-activegroup-visiblepanel-tab-background-color: #17171d;` | `--dv-activegroup-visiblepanel-tab-background-color: var(--color-surface);` |
| 188 | `--dv-activegroup-hiddenpanel-tab-background-color: #101014;` | `--dv-activegroup-hiddenpanel-tab-background-color: var(--color-bg);` |
| 189 | `--dv-inactivegroup-visiblepanel-tab-background-color: #14141a;` | `--dv-inactivegroup-visiblepanel-tab-background-color: var(--color-tab-unfocused-surface);` |
| 190 | `--dv-inactivegroup-hiddenpanel-tab-background-color: #101014;` | `--dv-inactivegroup-hiddenpanel-tab-background-color: var(--color-bg);` |
| 192 | `--dv-activegroup-visiblepanel-tab-color: #d8d8e0;` | `--dv-activegroup-visiblepanel-tab-color: var(--color-text);` |
| 193 | `--dv-activegroup-hiddenpanel-tab-color: #6d6d7d;` | `--dv-activegroup-hiddenpanel-tab-color: var(--color-text-2);` |
| 194 | `--dv-inactivegroup-visiblepanel-tab-color: #9a9aa8;` | `--dv-inactivegroup-visiblepanel-tab-color: var(--color-tab-unfocused-text);` |
| 195 | `--dv-inactivegroup-hiddenpanel-tab-color: #6d6d7d;` | `--dv-inactivegroup-hiddenpanel-tab-color: var(--color-text-2);` |
| 196 | `--dv-separator-border: #22222c;` | `--dv-separator-border: var(--color-border);` |
| 205 | `--dv-active-sash-color: #4bf3c8;` | `--dv-active-sash-color: var(--color-accent);` |

Lines 191, 197, 199, 200, 206 and 207 stay exactly as they are: `transparent`,
the neutral white drag wash (deliberately not the accent — the comment at 198
explains why), and two durations. None of them is a theme color.

### `src/lib/shell/components/ShellSidebar.svelte` (the rule at lines 380-394)

| Line | Current | Replacement |
|---|---|---|
| 382 | `--dv-group-view-background-color: #101014;` | `--dv-group-view-background-color: var(--color-bg);` |
| 383 | `--dv-activegroup-visiblepanel-tab-color: #7b7b8c;` | `--dv-activegroup-visiblepanel-tab-color: var(--color-section-header-text);` |
| 385 | `--dv-paneview-header-border-color: #22222c;` | `--dv-paneview-header-border-color: var(--color-border);` |
| 386 | `--dv-separator-border: #22222c;` | `--dv-separator-border: var(--color-border);` |
| 388 | `--dv-paneview-active-outline-color: #bd93f9;` | `--dv-paneview-active-outline-color: var(--color-section-focus-ring);` |
| 390 | `--dv-active-sash-color: #4bf3c8;` | `--dv-active-sash-color: var(--color-accent);` |

One more line in the same rule, line 393, is not a `--dv-` variable and so is not
covered by the test, but it is the same color for the same reason and the sidebar
will not follow a theme without it:

```css
background: #101014;   /*  ->  background: var(--color-bg);  */
```

Nothing in `src/lib/SourceDockviewShell.svelte` needs touching: that is the old
shell's dock, it already uses `var(--color-…)`, and the old shell must keep
rendering exactly as it does.

## 8. Optional: a palette command

If the palette gets a theme entry this round, `themeChoices()` gives the list and
`apply(id)` does the work — one command per theme ("Theme: Houston", "Theme:
Dracula") is the least surprising shape, and no new store is needed because
`currentThemeId()` reports what is in force.

## Out of scope, on purpose

- The roughly 246 colors typed directly into `/next` components (panels, cards,
  the git view) still do not follow a theme. That sweep is a captured follow-up;
  this round covers the shared chrome, the dock, the editor and the terminal.
- Importing a VS Code `.vsix` theme is still queued — no code for it anywhere.
