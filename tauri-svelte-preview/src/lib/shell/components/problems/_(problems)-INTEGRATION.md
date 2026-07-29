# Problems panel — wiring instructions for the integrator

Lane 7 of the panels wave. Every file below that is NOT in `src/lib/shell/problems/` or
`src/lib/shell/components/problems/` is a shared file this lane was not allowed to touch, so the
edits are written out here instead. Line numbers are from the wave branch as of this commit.

New files this lane shipped:

| File | What it is |
|---|---|
| `src/lib/shell/problems/problemsStore.svelte.ts` | Runes state + every pure rule (grouping, severity order, counts, filtering, empty-state wording). Type-only imports — keep it that way or `scripts/problemsStore.test.mjs` needs a bundler. |
| `src/lib/shell/problems/problemsService.ts` | The only place the panel talks to the machine. `activate(root)` and `refresh()`. |
| `src/lib/shell/problems/problemsBackend.ts` | This lane's wrapper for `list_source_lsp_diagnostics_for_root`, with the graceful-absence check and the per-file fallback read. |
| `src/lib/shell/components/problems/ProblemsPanel.svelte` | The panel. No props. |
| `scripts/problemsStore.test.mjs` | 37 checks, plain node. |

---

## 1. `package.json` — the test entry (integrator-only, per the wave rule)

Add next to the other store tests (they sit around `package.json:49-54`):

```json
"test:problems-store": "node --experimental-strip-types scripts/problemsStore.test.mjs",
```

Run it with `node scripts/problemsStore.test.mjs`; it compiles the runes module itself.

---

## 2. `DockPanel.svelte` becomes the Problems host

`src/lib/shell/components/DockPanel.svelte` is 40 lines: a `.dock-slot` wrapper (`:11`), a
`PanelPlaceholder` whose hint reads "Secondary terminals and logs will live here." (`:12`), and
the frame's Reset-layout button (`:13`). Replace the placeholder, keep the button:

```svelte
<script lang="ts">
  /** The bottom dock region: the Problems panel plus the frame's reset control. */
  import ProblemsPanel from './problems/ProblemsPanel.svelte';

  interface Props {
    onReset(): void;
  }
  let { onReset }: Props = $props();
</script>

<div class="dock-slot">
  <ProblemsPanel />
  <button class="reset-layout" onclick={onReset}>Reset layout</button>
</div>
```

Notes:

- Drop the `PanelPlaceholder` import (`:3`) with it. `PanelPlaceholder` is still used elsewhere —
  do not delete the component.
- The `.dock-slot` / `.reset-layout` styles (`:16-39`) stay as they are. The reset button is
  absolutely positioned at `top:6px; right:8px`, which lands on top of the panel's header row; the
  header's filter box is capped at `45%` width and the Refresh button sits left of the reset
  button's corner, so they do not overlap at the dock's normal width. If the two ever crowd each
  other, move the reset button into the panel header rather than shrinking the filter box.
- The reset button's four hard-coded hex values (`:27`, `:29`, `:37-38`) are part of Task 6's
  `--dv-*`/hex sweep list, not this lane's.

Nothing else mounts the panel: `src/routes/next/+page.svelte:662` already renders
`<DockPanel onReset={resetLayout} />` as the `dockArea` snippet.

---

## 3. Activation gating — the panel must NEVER load at launch

This is the constitution rule, and the bottom dock makes it easy to get wrong: unlike source
control and the context cards, the dock region is **on screen from the moment the shell opens**
(`src/lib/shell/layout/frame.ts:216-222` adds it below `center` at 180px). So a load on mount
would be exactly the launch-time backend call the shell is built to avoid.

`ProblemsPanel.svelte` therefore loads **nothing** when it mounts. Its only self-started route is
the user pressing **Refresh**. Add the second route — a session pick while the panel is genuinely
in view — by mirroring the source-control / context rule.

### 3a. `panelActivation.ts`

Mirror `sourceControlVisible` (`:80-83` in the interface, `:173-184` in the implementation) and
`contextVisible` (`:83`, `:186-197`) exactly:

- `PanelActivators` (`:56-62`) gains `problems(root: string | null): void;`
- `PanelActivation` (`:64-86`) gains, worded like its two neighbours:

  ```ts
  /** The Problems panel came into view, or went out of it. Same contract as
   * source control above. */
  problemsVisible(visible: boolean): void;
  ```

- In `createPanelActivation` (`:96-207`), alongside `sourceControlInView` / `gitLoadedFor`
  (`:104-112`):

  ```ts
  /** Can the user see the Problems panel right now? */
  let problemsInView = false;
  /** The folder problems were last loaded for, or `null` if they never have been. */
  let problemsLoadedFor: string | null = null;
  ```

  a loader beside `loadSourceControl` (`:122-126`):

  ```ts
  /** Point the Problems panel at this folder. */
  const loadProblems = (selection: ProjectSelection): void => {
    const root = selection.root.trim();
    problemsLoadedFor = root;
    activators.problems(root || null);
  };
  ```

  a line in `loadSessionPanels` (`:137-142`), after the context line:

  ```ts
  if (problemsInView) loadProblems(selection);
  ```

  the method itself, copied from `sourceControlVisible` (`:173-184`) with the names swapped:

  ```ts
  problemsVisible(visible: boolean): void {
    problemsInView = visible;
    if (!visible || !sessionPanelsShown) return;
    const selection = readSelection();
    if (problemsLoadedFor === selection.root.trim()) return;
    loadProblems(selection);
  },
  ```

  and `'problems'` in `loadedPanels()` (`:199-205`) when `problemsLoadedFor !== null`.

`scripts/panelActivation.test.mjs` covers the two existing view-gated panels; the same cases
copied for `problems` are the cheapest way to keep this honest.

### 3b. `shellPanels.ts`

Add the activator to the map at `src/lib/shell/shellPanels.ts:50-63`:

```ts
import { activate as activateProblems } from './problems/problemsService.ts';
// …
problems: (root) => activateProblems(root),
```

### 3c. Who calls `problemsVisible(true)` — the important half

**Do not call it on mount.** Because the dock region is always on screen, "in view" has to mean a
user gesture. Pick whichever the dock ends up with:

- **If the dock gains tabs** (Problems / Terminal / Output later this round): report `true` when
  the Problems tab becomes the active one, `false` when another does — a click, exactly like the
  tool column's icon strip.
- **If Problems stays the dock's only content** (the state this lane ships against): report `true`
  when the user expands the dock region past its collapsed height, and `false` when it is
  collapsed. If the dock has no collapse gesture yet, **wire nothing** and leave the panel's
  Refresh button as the only route — that is correct behaviour, not a gap, and the panel's
  "Nothing loaded yet / Press Refresh…" state says so in plain words.

The one thing that must not happen is `problemsVisible(true)` being reported from
`onMount`/an effect, which would put a language-server read back at launch.

---

## 4. Palette command

In `src/lib/shell/shellCommands.ts`, alongside `context-refresh` (`:101-106`):

```ts
import { refresh as refreshProblems } from './problems/problemsService.ts';
// …
{
  id: 'problems-refresh',
  label: 'Look for problems again',
  detail: 'Ask the language server what is wrong with this project',
  perform: () => void refreshProblems()
},
```

No `showPanel` hook is needed — the dock is always on screen.

---

## 5. Backend contract this lane consumes

From Task 0 (frozen, camelCase over the bridge):

- `list_source_lsp_diagnostics_for_root({ root })` → `SourceLspDiagnostic[]`, empty when there is
  nothing. `SourceLspDiagnostic` now carries an optional `path`.
- Fallback: the existing `read_source_lsp_diagnostics` via
  `readSourceLspDiagnosticsFromTauri(preview, { root, line: 1, column: 1 })`
  (`src/lib/tauriSource.ts:1116`).

`problemsBackend.ts` answers three ways and the panel words each one differently: `null` (not the
desktop app), `{ unavailable: true }` (desktop app predates the command → per-open-file fallback,
and the panel's status line says *"This desktop app cannot list problems for a whole project yet,
so these come from the N files you have open."*), or the real list. Both calls are counted with
`countInvoke`.

If the integrator folds this wrapper into `tauriSource.ts`, keep the unknown-command check —
without it an older desktop app shows an error instead of the files it can actually read.

---

## 6. THE LIVE BUG: /next Monaco shows no squiggles at all on the desktop

Not fixed here (every file involved is shared). This is the whole fix, spelled out.

### What is broken

- `src/lib/MonacoSourceEditor.svelte:399-412` — when the app is running natively, the bundled
  TypeScript worker's diagnostics are switched **off** on purpose (`suppressWorkerDiagnostics =
  isNativeTauriRuntime()`), because the Rust language server is supposed to own them. They arrive
  as `mcb-lsp` markers through the `externalDiagnostics` prop (declared `:191`, defaulted `[]` at
  `:231`, applied to the model at `:1536-1552`).
- `src/lib/shell/components/EditorPanel.svelte:259-269` renders `<CodeEditor …>` and **never
  passes `externalDiagnostics`**. `src/lib/shell/editor/sourceIntelligence.ts` has no diagnostics
  code at all — its ten callbacks are definitions, references, hover, completions, highlights,
  signature help, inlay hints and semantic tokens.

So on the desktop: the worker is silenced and nothing replaces it → zero squiggles. In a browser
tab the worker still runs, which is why the web preview looks fine and the bug reads as
"desktop only".

### The reference wiring that works — the old shell

- `src/routes/+page.svelte:9996-10024` `loadSourceLspDiagnostics(preview, project)`: bails unless
  `sourceSupportsLanguageIntelligence(preview.language)`, calls
  `readSourceLspDiagnosticsFromTauri({ ...preview, content: draft }, { root: project.path, line: 1,
  column: 1 })` (`:10010`), and drops the answer if the open file or the project changed while it
  was in flight, then assigns `sourceLspDiagnostics`.
- Debounced by `scheduleSourceLspDiagnostics` (`:9962-9971`, 650ms), re-run on file switch
  (`:8759`, `:8770`) and after a scan (`:9800`), cleared on teardown (`:9945-9956`).
- Handed down as `lspDiagnostics={sourceLspDiagnostics}` at `src/routes/+page.svelte:16358`, which
  the old `EditorPanel` forwards into `MonacoSourceEditor`'s `externalDiagnostics`.

### The smallest correct landing in /next (two shared files)

**`sourceIntelligence.ts`** — add an eleventh lookup that is *pushed*, not pulled by Monaco. It
does not join `SourceIntelligenceCallbacks` (Monaco has no diagnostics callback); it is a method
on `SourceIntelligence` (interface at `:144-161`, implementation returned at `:498-533`):

```ts
/** What the language server says is wrong with the file on screen. Empty when
 *  there is no language server for it, or when the read fails. */
loadActiveFileDiagnostics(): Promise<SourceDiagnostic[]>;
```

implemented next to the other tier-1 lookups (`:220-248`), using the module's own
`previewWithDraft()` (`:212-214`) and `lookupRoot()` (`:216-218`) so it always reads the text on
screen, not the text on disk:

```ts
async function loadActiveFileDiagnostics(): Promise<SourceDiagnostic[]> {
  const preview = previewWithDraft();
  if (!preview || !languageIntelligenceAvailable()) return [];
  try {
    countInvoke('read_source_lsp_diagnostics');
    return (await readSourceLspDiagnosticsFromTauri(preview, {
      root: lookupRoot(),
      line: 1,
      column: 1
    })) ?? [];
  } catch {
    return [];
  }
}
```

(`readSourceLspDiagnosticsFromTauri` joins the import list at `:48-61`; `SourceDiagnostic` joins
the type imports at `:34-47`.)

**`EditorPanel.svelte`** — hold the answers per file and pass the active file's:

```ts
let diagnosticsByPath = $state<Record<string, SourceDiagnostic[]>>({});

/** Ask what is wrong with the file on screen, and remember it against that file. */
async function loadDiagnosticsForActiveFile(): Promise<void> {
  const path = editorState.activePath;
  if (!path) return;
  const diagnostics = await sourceIntelligence.loadActiveFileDiagnostics();
  if (destroyed || editorState.activePath !== path) return;   // superseded → drop it
  diagnosticsByPath = { ...diagnosticsByPath, [path]: diagnostics };
}
```

Call it from the two places the panel already syncs the lookup service:

- `readFileIntoEditor`'s `finally`, right after `syncIntelligenceWithActiveFile()`
  (`EditorPanel.svelte:135`) — a file that has just been read;
- `selectOpenFile` (`:165`) — switching tabs.

and pass it to Monaco in the render block (`:259-269`):

```svelte
externalDiagnostics={diagnosticsByPath[activeFile.path] ?? []}
```

Drop a file's entry in `closeOpenFileAt` (`:174`) so a closed file stops holding memory.

**Timing note (do not skip):** the language server answers a file it has only just opened with an
empty list, so the first call after a file opens often returns nothing and the squiggles appear
only on the next one. The old shell hid this behind a 650ms debounce
(`src/routes/+page.svelte:9967`, `scheduleSourceLspDiagnostics`). Copy that: run the call once
when the file lands and once more after ~650ms, or wire it to the same debounce shape. Without
it, "it worked when I clicked around" is the bug report.

**Not in scope for this fix:** the editor is read-only in /next (`editable={false}`,
`EditorPanel.svelte:263`), so there is no keystroke to re-run on. When editing lands, the debounce
above is the hook.

### If the squiggle fix is too big to land in the integration pass

Report it as its own follow-up — the Problems panel is complete and useful without it (they read
the same data by different routes: the panel asks about the whole project, the editor about one
file). Do not half-land it: passing `externalDiagnostics` without the debounce gives a file that
shows squiggles only sometimes, which is worse than none.

### A cheaper variant, for later

Once the editor holds per-file diagnostics, the Problems panel's fallback route (the one that
walks the open files) could read them straight from the editor instead of re-invoking. Nice, not
necessary — and it must stay a *fallback*: the whole-project command is the real source, and only
it can list a file nobody has opened.

---

## 7. What this lane deliberately did not do

- No mounting, so no visual check yet — first render happens in the integration pass. Look at:
  header counts and the filter box at the dock's default 180px height (the header wraps rather
  than clipping), a file heading with a long path, and a collapsed file (`Collapsible.Content`
  has no display class of its own; the layout is on the div inside, so folding actually works).
- No push events: `list_source_lsp_diagnostics_for_root` is a pull. The panel shows what the
  language server had reported at the moment you asked. That is why Refresh is prominent and why
  the empty state mentions a server that may still be starting.
- Build errors (`tsc`, `cargo`, `svelte-check`) are not in this panel — nothing in the app runs a
  build with its output captured. That is a second slice, and the store's `ProblemsSource` union
  is where it would join.
