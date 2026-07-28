# Slice 2 — Walking Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `/next` its permanent frame — a single Gridview-rooted layout (left rail / center tabs / right context / bottom dock) with the live terminal surface as a center tab and shallow placeholder slots for editor, browser, context, and dock — so every later feature lands as a panel on a stable frame.

**Architecture:** One `createGridview` root owns four regions. The center region hosts one `createDockview` for tabs. All panel content is authored by Svelte in a hidden off-screen parking stage and **teleported** (node moved, identity preserved) into dockview-owned host divs — dockview never owns, renders, or destroys app DOM. Layout persists to localStorage with a guarded restore (panel-set equality, try/catch, clear+rebuild fallback). The Slice 1 orchestrator keeps its exact session logic; only its markup moves into a `ShellFrame` component.

**Tech Stack:** Svelte 5 (runes, snippets), dockview-core 6.6.1 (`createGridview` + `createDockview`), existing Slice 1 shell modules (`terminalService`, `sessionRailStore`, `TerminalSurface`, `SessionRail`).

## Global Constraints

- **Old shell is frozen.** `src/routes/+page.svelte` and everything it exclusively owns: read-only, 0 lines changed. Copy patterns out; never import from it.
- **Constitution:** orchestrator `src/routes/next/+page.svelte` stays ≤ 300 lines; effects never do IO; no backend call at layout creation, restore, or sash drag; nothing hydrates at launch except the session rail (unchanged from Slice 1).
- **Terminal DOM is sacred:** an xterm host is never destroyed or re-created by layout changes. `TerminalSurface` keeps every host mounted for the life of the page; the skeleton may only *move* its wrapper (`replaceChildren` teleport — node identity preserved, xterm survives re-parenting).
- **Center Dockview uses `defaultRenderer: 'always'`** — `'onlyWhenVisible'` detaches panel DOM on tab switch and would kill the terminal rendering. Non-negotiable.
- **No `src-tauri/` or `core/` edits in this slice.** Frontend only; the running `tauri dev` must never be restarted by a build watch trigger.
- **Perf gates:** dragging any sash or switching any center tab adds **zero** invokes except at most one gated `resize_terminal_session` when the terminal's grid actually changed size. The dev invoke counter is the proof.
- localStorage keys are namespaced `mac-command-bar.next.*`; every write is quota-safe (try/catch, never throws into callers).
- Commits: plain messages, **no Co-Authored-By trailer** (user's global rule).
- dockview-core 6.6.1 API facts used below were verified against `node_modules/dockview-core/dist` typings (recon 2026-07-28): Gridview's `createComponent` must return a **`GridviewPanel` subclass instance** (abstract `getComponent(): IFrameworkPart`; the DOM node is the base class's `this.element`); `GridviewOptions` has **no `theme` field** (set `--dv-*` vars via container class yourself); `GridviewApi` has **no `setVisible`** (rail collapse is out of scope); `Orientation` is a runtime enum (value import); Dockview `IContentRenderer` needs only `{ element, init }`. If an implementer hits a type mismatch, check the installed `.d.ts` before changing design.

## File Structure

| File | Responsibility |
|---|---|
| Create `src/lib/shell/layout/layoutStorage.ts` | Pure, testable: load/save/validate serialized layouts against an expected panel-id set. No DOM, no dockview import. |
| Create `scripts/layoutStorage.test.mjs` | Node test for the pure module. |
| Modify `src/lib/shell/terminalService.ts` | Add `refit()` — refit the active view; resize gate already prevents storms. |
| Modify `scripts/nextTerminalService.test.mjs` | Tests for `refit()`. |
| Create `src/lib/shell/layout/frame.ts` | Gridview root: `TeleportGridPanel` subclass, four-region construction, guarded restore, persistence wiring, dispose. DOM-only, zero backend IO. |
| Create `src/lib/shell/layout/centerDock.ts` | Center Dockview: teleport content renderer, fixed panel roster, guarded restore, persistence wiring, dispose. DOM-only, zero backend IO. |
| Create `src/lib/shell/components/ShellFrame.svelte` | Svelte wrapper: parking stage + snippets, mounts frame + center dock, ResizeObserver, layout-change callback, reset-layout, all dockview CSS overrides. |
| Create `src/lib/shell/components/PanelPlaceholder.svelte` | Shallow slot content (name + hint line). |
| Modify `src/routes/next/+page.svelte` | Swap the two-column markup for `<ShellFrame>` + snippets; wire `refit`. Stays ≤ 300 lines. |
| Modify `tauri-svelte-preview/package.json` | Add `test:layout-storage` script. |

Seams frozen for the parallel wave: panel content enters ONLY as a `ShellFrame` snippet or a center-dock roster entry; `frame.ts` / `centerDock.ts` interfaces are the contract.

---

### Task 1: `layoutStorage.ts` — pure persistence guard + tests

**Files:**
- Create: `tauri-svelte-preview/src/lib/shell/layout/layoutStorage.ts`
- Create: `tauri-svelte-preview/scripts/layoutStorage.test.mjs`
- Modify: `tauri-svelte-preview/package.json` (scripts block)

**Interfaces:**
- Consumes: nothing (pure).
- Produces: `loadLayout(storage, key)`, `saveLayout(storage, key, layout): boolean`, `clearLayout(storage, key)`, `gridPanelIds(serialized)`, `dockPanelIds(serialized)`, `panelSetMatches(ids, expected)` — used verbatim by Tasks 3 and 4.

- [ ] **Step 1: Write the failing test**

Create `tauri-svelte-preview/scripts/layoutStorage.test.mjs` (same harness style as `scripts/ownedSessions.test.mjs` — plain node asserts, no framework):

```js
import assert from 'node:assert/strict';
import {
  loadLayout,
  saveLayout,
  clearLayout,
  gridPanelIds,
  dockPanelIds,
  panelSetMatches
} from '../src/lib/shell/layout/layoutStorage.ts';

function memoryStorage(initial = {}) {
  const map = new Map(Object.entries(initial));
  return {
    getItem: (k) => (map.has(k) ? map.get(k) : null),
    setItem: (k, v) => map.set(k, v),
    removeItem: (k) => map.delete(k),
    _map: map
  };
}

// loadLayout: absent, corrupt, and round-trip
{
  const storage = memoryStorage();
  assert.equal(loadLayout(storage, 'k'), null, 'absent key -> null');
  storage.setItem('k', '{not json');
  assert.equal(loadLayout(storage, 'k'), null, 'corrupt json -> null, no throw');
  storage.setItem('k', JSON.stringify({ a: 1 }));
  assert.deepEqual(loadLayout(storage, 'k'), { a: 1 }, 'round-trip');
}

// saveLayout: happy path true, quota error false (never throws)
{
  const storage = memoryStorage();
  assert.equal(saveLayout(storage, 'k', { a: 1 }), true);
  assert.deepEqual(JSON.parse(storage.getItem('k')), { a: 1 });
  const failing = {
    getItem: () => null,
    setItem: () => {
      throw new DOMException('quota', 'QuotaExceededError');
    },
    removeItem: () => {}
  };
  assert.equal(saveLayout(failing, 'k', { a: 1 }), false, 'quota -> false, no throw');
}

// clearLayout tolerates a throwing storage
{
  const throwing = {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {
      throw new Error('nope');
    }
  };
  assert.doesNotThrow(() => clearLayout(throwing, 'k'));
}

// gridPanelIds walks the SerializedGridviewComponent tree (branch/leaf shape)
{
  const grid = {
    grid: {
      root: {
        type: 'branch',
        data: [
          { type: 'leaf', data: { id: 'rail' } },
          {
            type: 'branch',
            data: [
              { type: 'leaf', data: { id: 'center' } },
              { type: 'leaf', data: { id: 'dock' } }
            ]
          },
          { type: 'leaf', data: { id: 'context' } }
        ]
      },
      width: 1200,
      height: 800,
      orientation: 'HORIZONTAL'
    }
  };
  assert.deepEqual([...gridPanelIds(grid)].sort(), ['center', 'context', 'dock', 'rail']);
  assert.deepEqual([...gridPanelIds({})], [], 'malformed -> empty, no throw');
}

// dockPanelIds reads SerializedDockview.panels keys
{
  const dock = { panels: { session: {}, editor: {}, browser: {} }, grid: {} };
  assert.deepEqual([...dockPanelIds(dock)].sort(), ['browser', 'editor', 'session']);
  assert.deepEqual([...dockPanelIds({ grid: {} })], [], 'missing panels -> empty');
}

// panelSetMatches: exact set equality, order-independent
{
  assert.equal(panelSetMatches(['a', 'b'], ['b', 'a']), true);
  assert.equal(panelSetMatches(['a'], ['a', 'b']), false);
  assert.equal(panelSetMatches(['a', 'b', 'c'], ['a', 'b']), false);
  assert.equal(panelSetMatches([], []), true);
}

console.log('layoutStorage: all tests passed');
```

Add to `tauri-svelte-preview/package.json` scripts, next to `test:owned-sessions` (match the exact runner invocation used by the sibling `test:*` scripts in that file — copy its flags verbatim, only changing the test path):

```json
"test:layout-storage": "node --experimental-strip-types scripts/layoutStorage.test.mjs"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd tauri-svelte-preview && pnpm run test:layout-storage`
Expected: FAIL — cannot find module `layoutStorage.ts`.

- [ ] **Step 3: Write the implementation**

Create `tauri-svelte-preview/src/lib/shell/layout/layoutStorage.ts`:

```ts
/**
 * layoutStorage.ts — pure persistence helpers for the /next shell layout.
 *
 * No DOM, no dockview import, no IO of its own: callers hand in a Storage-like
 * object so node tests can run without a browser. Every function is total —
 * corrupt JSON, quota errors, and malformed trees come back as null / false /
 * empty, never as a throw into the shell.
 */

/** The subset of `Storage` the shell needs (injectable for tests). */
export interface LayoutStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

export const GRID_LAYOUT_KEY = 'mac-command-bar.next.grid-layout';
export const CENTER_LAYOUT_KEY = 'mac-command-bar.next.center-layout';

export function loadLayout<T>(storage: LayoutStorage, key: string): T | null {
  try {
    const raw = storage.getItem(key);
    if (!raw) return null;
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

/** Quota-safe write. Returns false instead of throwing (layout loss is benign). */
export function saveLayout(storage: LayoutStorage, key: string, layout: unknown): boolean {
  try {
    storage.setItem(key, JSON.stringify(layout));
    return true;
  } catch {
    return false;
  }
}

export function clearLayout(storage: LayoutStorage, key: string): void {
  try {
    storage.removeItem(key);
  } catch {
    // A storage that cannot even remove is a storage we ignore.
  }
}

/**
 * Panel ids inside a `SerializedGridviewComponent` (dockview-core 6.6.1 shape:
 * `{ grid: { root: <branch|leaf tree> } }`, leaves carry `data.id`). Malformed
 * input yields an empty set — the caller then rebuilds from defaults.
 */
export function gridPanelIds(serialized: unknown): Set<string> {
  const ids = new Set<string>();
  const root = (serialized as { grid?: { root?: unknown } } | null)?.grid?.root;
  const walk = (node: unknown): void => {
    if (!node || typeof node !== 'object') return;
    const { type, data } = node as { type?: string; data?: unknown };
    if (type === 'branch' && Array.isArray(data)) {
      for (const child of data) walk(child);
      return;
    }
    if (type === 'leaf' && data && typeof data === 'object') {
      const id = (data as { id?: unknown }).id;
      if (typeof id === 'string') ids.add(id);
    }
  };
  walk(root);
  return ids;
}

/** Panel ids inside a `SerializedDockview` (`{ panels: Record<id, …> }`). */
export function dockPanelIds(serialized: unknown): Set<string> {
  const panels = (serialized as { panels?: unknown } | null)?.panels;
  if (!panels || typeof panels !== 'object') return new Set();
  return new Set(Object.keys(panels));
}

/** True when `ids` is EXACTLY `expected` (both directions, order-free). */
export function panelSetMatches(ids: Iterable<string>, expected: Iterable<string>): boolean {
  const a = new Set(ids);
  const b = new Set(expected);
  if (a.size !== b.size) return false;
  for (const id of a) if (!b.has(id)) return false;
  return true;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd tauri-svelte-preview && pnpm run test:layout-storage`
Expected: PASS — `layoutStorage: all tests passed`. Also run `pnpm run check` (tsc) — clean.

- [ ] **Step 5: Commit**

```bash
git add tauri-svelte-preview/src/lib/shell/layout/layoutStorage.ts tauri-svelte-preview/scripts/layoutStorage.test.mjs tauri-svelte-preview/package.json
git commit -m "feat(next): pure layout persistence helpers with guarded parse"
```

---

### Task 2: `terminalService.refit()` — the skeleton's only service change

**Files:**
- Modify: `tauri-svelte-preview/src/lib/shell/terminalService.ts`
- Modify: `tauri-svelte-preview/scripts/nextTerminalService.test.mjs`

**Interfaces:**
- Consumes: existing service internals — `manager.activeKey()`, the view registry, `view.fit()`, the `lastSizeByPty` gate.
- Produces: `refit(): void` on the object returned by `createTerminalService` (alongside `attach/startOwned/adoptExisting/show/closeOwned/dispose`). Task 6's ResizeObserver calls it; it is safe to call at any time, including before attach and with no active session.

**Why:** `/next` currently refits a terminal only inside `show()`. When the skeleton's sashes resize the session panel, the visible xterm must refit — and ONLY refit. `view.fit()` already reports real geometry through the view's `onResize` hook into `resizePty`, whose `lastSizeByPty` string-compare swallows no-op sizes, so `refit()` needs no gating of its own and can never storm the backend.

- [ ] **Step 1: Write the failing tests**

In `tauri-svelte-preview/scripts/nextTerminalService.test.mjs`, add (following the file's existing mock-backend + fake-view harness conventions — reuse its existing helpers for building a service with a started session):

```js
// refit(): no active view -> no fit, no backend call, no throw
{
  const { service, backendCalls } = makeServiceHarness(); // existing helper pattern
  assert.doesNotThrow(() => service.refit());
  assert.equal(backendCalls.filter((c) => c.op === 'resize').length, 0);
}

// refit(): fits ONLY the active view; unchanged geometry sends nothing
{
  const h = await makeTwoSessionHarness(); // active: A, hidden: B (existing pattern)
  h.views.A.fitCalls = 0;
  h.views.B.fitCalls = 0;
  h.service.refit();
  assert.equal(h.views.A.fitCalls, 1, 'active view refits');
  assert.equal(h.views.B.fitCalls, 0, 'hidden view untouched');
  // fake fit reports the SAME cols/rows the PTY already has:
  assert.equal(h.backendCalls.filter((c) => c.op === 'resize').length, 0,
    'unchanged size -> gate swallows the resize');
}

// refit(): changed geometry funnels exactly one gated backend resize
{
  const h = await makeTwoSessionHarness();
  h.views.A.nextFitSize = { cols: 100, rows: 40 }; // differs from seeded size
  h.service.refit();
  h.service.refit(); // second refit at same size: swallowed
  assert.equal(h.backendCalls.filter((c) => c.op === 'resize').length, 1);
}
```

(Adapter note for the implementer: the file already has fake views whose `fit()` invokes the `onResize` hook with a configurable size, and a recorded mock backend — extend those, do not build a parallel harness. If helper names differ, keep the assertions identical and use the real names.)

- [ ] **Step 2: Run to verify the new cases fail**

Run: `cd tauri-svelte-preview && pnpm run test:next-terminal-service`
Expected: FAIL — `service.refit is not a function`.

- [ ] **Step 3: Implement `refit()`**

In `createTerminalService` (`terminalService.ts`), next to `show()`:

```ts
/**
 * Re-measure the ACTIVE view against its host and, when the grid really
 * changed, push exactly one gated resize to the PTY. The layout frame calls
 * this from a ResizeObserver; hidden views are left alone (their grids are
 * corrected on their next `show()`), and `fit()` on the fake/hidden path is
 * already a no-op inside the view. Safe pre-attach and with no session.
 */
function refit(): void {
  if (disposed) return;
  const active = manager.activeKey();
  if (active == null) return;
  const view = manager.viewFor?.(active) ?? null;
  view?.fit();
}
```

and add `refit` to the returned object. If the view manager has no public lookup for an existing view (check its actual surface — `ensureView` creates, so it must NOT be used here), add the minimal read-only accessor to it (e.g. `viewFor(key): TerminalView | null`) rather than reaching into private maps from the service.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd tauri-svelte-preview && pnpm run test:next-terminal-service`
Expected: PASS (all pre-existing cases too). Run `pnpm run check` — clean.

- [ ] **Step 5: Commit**

```bash
git add tauri-svelte-preview/src/lib/shell/terminalService.ts tauri-svelte-preview/scripts/nextTerminalService.test.mjs
git commit -m "feat(next): terminalService.refit() for layout-driven terminal refits"
```

---

### Task 3: `frame.ts` — the Gridview root with teleport panels

**Files:**
- Create: `tauri-svelte-preview/src/lib/shell/layout/frame.ts`

**Interfaces:**
- Consumes: Task 1's `loadLayout/saveLayout/clearLayout/gridPanelIds/panelSetMatches`, `GRID_LAYOUT_KEY`; dockview-core `createGridview`, `GridviewPanel`, `Orientation` (VALUE imports), types `GridviewApi`, `IFrameworkPart`.
- Produces: `createShellFrame(container, options): ShellFrame` where

```ts
export type ShellRegionId = 'rail' | 'center' | 'context' | 'dock';
export interface ShellFrameOptions {
  storage: LayoutStorage;
  /** Svelte-owned region elements, teleported into the grid. */
  regions: Record<ShellRegionId, HTMLElement>;
  /** Fired (already debounced) after any user-driven layout change was persisted. */
  onLayoutPersisted?: (ok: boolean) => void;
}
export interface ShellFrame {
  api: GridviewApi;
  /** Wipe stored grid layout and rebuild the default arrangement in place. */
  resetLayout(): void;
  layout(width: number, height: number): void;
  dispose(): void;
}
```

Task 5 is the only consumer.

**Design constraints (from the 6.6.1 recon — do not fight them):**
- `createComponent` must return a `GridviewPanel` subclass instance; the base class owns `this.element`; the only abstract member is `getComponent(): IFrameworkPart`.
- No `theme` option exists on Gridview: the caller's container carries the `--dv-*` CSS vars (Task 5).
- No `setVisible` on `GridviewApi`: no collapse/hide in this slice.
- All four regions are teleport targets; the module never creates content, only moves the provided `regions[id]` node via `host.replaceChildren(node)` — node identity preserved (xterm-safe).

- [ ] **Step 1: Implement the module**

Create `tauri-svelte-preview/src/lib/shell/layout/frame.ts`:

```ts
/**
 * frame.ts — the /next shell's Gridview root (left rail / center / right
 * context / bottom dock). DOM-only: zero backend IO, zero Svelte imports.
 *
 * Teleport contract: every region's content is a Svelte-owned element that
 * this module MOVES into a dockview-owned host div. dockview never renders or
 * disposes app DOM; disposing the frame moves nothing (Svelte still owns the
 * nodes and the page is unmounting anyway).
 */
import {
  createGridview,
  GridviewPanel,
  Orientation,
  type GridviewApi,
  type IFrameworkPart
} from 'dockview-core';

import {
  clearLayout,
  GRID_LAYOUT_KEY,
  gridPanelIds,
  loadLayout,
  panelSetMatches,
  saveLayout,
  type LayoutStorage
} from './layoutStorage';

export type ShellRegionId = 'rail' | 'center' | 'context' | 'dock';

const REGION_IDS: readonly ShellRegionId[] = ['rail', 'center', 'context', 'dock'];
const COMPONENT = 'shell-region';
const PERSIST_DEBOUNCE_MS = 250;

export interface ShellFrameOptions {
  storage: LayoutStorage;
  regions: Record<ShellRegionId, HTMLElement>;
  onLayoutPersisted?: (ok: boolean) => void;
}

export interface ShellFrame {
  api: GridviewApi;
  resetLayout(): void;
  layout(width: number, height: number): void;
  dispose(): void;
}

/** Gridview panel that adopts a Svelte-owned element into `this.element`. */
class TeleportGridPanel extends GridviewPanel {
  private readonly adopt: (id: string, host: HTMLElement) => void;

  constructor(id: string, component: string, adopt: (id: string, host: HTMLElement) => void) {
    super(id, component);
    this.adopt = adopt;
  }

  protected getComponent(): IFrameworkPart {
    this.adopt(this.id, this.element);
    return {
      update: () => {},
      dispose: () => {}
    };
  }
}

export function createShellFrame(container: HTMLElement, options: ShellFrameOptions): ShellFrame {
  let synchronizing = false;
  let persistTimer: ReturnType<typeof setTimeout> | null = null;
  let disposed = false;

  const adopt = (id: string, host: HTMLElement): void => {
    const region = options.regions[id as ShellRegionId];
    if (!region) return;
    host.classList.add('shell-region-host', `shell-region-host-${id}`);
    if (region.parentElement !== host) host.replaceChildren(region);
  };

  const api = createGridview(container, {
    orientation: Orientation.HORIZONTAL,
    proportionalLayout: true,
    hideBorders: true,
    className: 'shell-grid',
    createComponent: ({ id, name }) => new TeleportGridPanel(id, name, adopt)
  });

  /** The default arrangement; also the fallback whenever restore is unusable. */
  const buildDefault = (): void => {
    api.addPanel({ id: 'center', component: COMPONENT });
    api.addPanel({
      id: 'rail',
      component: COMPONENT,
      position: { direction: 'left', referencePanel: 'center' },
      size: 264,
      minimumWidth: 200,
      maximumWidth: 480
    });
    api.addPanel({
      id: 'context',
      component: COMPONENT,
      position: { direction: 'right', referencePanel: 'center' },
      size: 300,
      minimumWidth: 220,
      maximumWidth: 560
    });
    // Below CENTER only: the dock spans the middle column, not the rails.
    api.addPanel({
      id: 'dock',
      component: COMPONENT,
      position: { direction: 'below', referencePanel: 'center' },
      size: 180,
      minimumHeight: 96
    });
  };

  const runSynchronized = (fn: () => void): void => {
    synchronizing = true;
    try {
      fn();
    } finally {
      synchronizing = false;
    }
  };

  runSynchronized(() => {
    const stored = loadLayout<object>(options.storage, GRID_LAYOUT_KEY);
    if (stored && panelSetMatches(gridPanelIds(stored), REGION_IDS)) {
      try {
        api.fromJSON(stored as never);
        return;
      } catch {
        try {
          api.clear();
        } catch {
          // fall through to a plain rebuild on a fresh container
        }
      }
    }
    buildDefault();
  });

  const persistSoon = (): void => {
    if (synchronizing || disposed) return;
    if (persistTimer !== null) clearTimeout(persistTimer);
    persistTimer = setTimeout(() => {
      persistTimer = null;
      if (disposed) return;
      const ok = saveLayout(options.storage, GRID_LAYOUT_KEY, api.toJSON());
      options.onLayoutPersisted?.(ok);
    }, PERSIST_DEBOUNCE_MS);
  };

  const changeListener = api.onDidLayoutChange(persistSoon);

  return {
    api,
    resetLayout(): void {
      clearLayout(options.storage, GRID_LAYOUT_KEY);
      runSynchronized(() => {
        api.clear();
        buildDefault();
      });
      persistSoon();
    },
    layout(width: number, height: number): void {
      api.layout(width, height);
    },
    dispose(): void {
      disposed = true;
      if (persistTimer !== null) clearTimeout(persistTimer);
      changeListener.dispose();
      api.dispose();
    }
  };
}
```

- [ ] **Step 2: Type-check**

Run: `cd tauri-svelte-preview && pnpm run check`
Expected: clean. If `addPanel`'s option names differ (e.g. constraint fields not accepted there), consult `node_modules/dockview-core/dist/esm/gridview/gridviewComponent.d.ts:14-24` — constraints may alternatively be passed via the `TeleportGridPanel` constructor's third `super` argument (`gridviewPanel.d.ts:58-63`); keep the SAME numbers.

- [ ] **Step 3: Commit**

```bash
git add tauri-svelte-preview/src/lib/shell/layout/frame.ts
git commit -m "feat(next): Gridview shell frame with teleported regions and guarded restore"
```

---

### Task 4: `centerDock.ts` — center tabs with teleport renderers

**Files:**
- Create: `tauri-svelte-preview/src/lib/shell/layout/centerDock.ts`

**Interfaces:**
- Consumes: Task 1 helpers + `CENTER_LAYOUT_KEY`; dockview-core `createDockview`, `themeDracula` (values), types `DockviewApi`, `IContentRenderer`, `GroupPanelPartInitParameters`.
- Produces:

```ts
export interface CenterPanelSpec {
  id: string;          // 'session' | 'editor' | 'browser' in Slice 2
  title: string;       // tab label
  element: HTMLElement; // Svelte-owned content, teleported
}
export interface CenterDock {
  api: DockviewApi;
  activatePanel(id: string): void;
  /** Wipe stored center layout and rebuild the roster's default tab order. */
  resetLayout(): void;
  /** Called with a panel id every time dockview lays that panel out (resize/tab churn). */
  dispose(): void;
}
export function createCenterDock(
  container: HTMLElement,
  options: {
    storage: LayoutStorage;
    panels: CenterPanelSpec[];
    onPanelLayout?: (id: string) => void;
    onLayoutPersisted?: (ok: boolean) => void;
  }
): CenterDock;
```

Task 5 is the only consumer; `onPanelLayout('session')` is how terminal refits get scheduled.

**Design constraints:**
- `defaultRenderer: 'always'` (terminal DOM stays attached across tab switches — Global Constraints).
- The content renderer is a dumb empty div; the Svelte element is moved in with `replaceChildren` (identity preserved). On renderer `dispose` (panel closed by the user), the element is RETURNED to its parking parent — never left to be garbage-collected with dockview's DOM. The renderer takes the parking node as part of the spec.
- Panel identity travels in `params.params.panelId`, exactly like the old shell's `panelID` pattern.
- Guarded restore identical in shape to Task 3 (`dockPanelIds` + `panelSetMatches` + try/catch + clear + default build). Note the consequence: a tab the user closed reappears on next launch — accepted for the skeleton, recorded in the deferred list.

- [ ] **Step 1: Implement the module**

Create `tauri-svelte-preview/src/lib/shell/layout/centerDock.ts`:

```ts
/**
 * centerDock.ts — the /next center tab area (one Dockview). DOM-only: zero
 * backend IO, zero Svelte imports. Same teleport contract as frame.ts, plus
 * an explicit "return to parking" on panel close so Svelte-owned content
 * (the terminal surface!) is never destroyed with a dockview renderer.
 */
import {
  createDockview,
  themeDracula,
  type DockviewApi,
  type GroupPanelPartInitParameters,
  type IContentRenderer
} from 'dockview-core';

import {
  CENTER_LAYOUT_KEY,
  clearLayout,
  dockPanelIds,
  loadLayout,
  panelSetMatches,
  saveLayout,
  type LayoutStorage
} from './layoutStorage';

export interface CenterPanelSpec {
  id: string;
  title: string;
  element: HTMLElement;
}

export interface CenterDockOptions {
  storage: LayoutStorage;
  panels: CenterPanelSpec[];
  onPanelLayout?: (id: string) => void;
  onLayoutPersisted?: (ok: boolean) => void;
}

export interface CenterDock {
  api: DockviewApi;
  activatePanel(id: string): void;
  resetLayout(): void;
  dispose(): void;
}

const COMPONENT = 'center-panel';
const PERSIST_DEBOUNCE_MS = 250;

export function createCenterDock(container: HTMLElement, options: CenterDockOptions): CenterDock {
  const specs = new Map(options.panels.map((panel) => [panel.id, panel]));
  /** Where each element goes back to when its dockview panel dies. */
  const parking = new Map(
    options.panels.map((panel) => [panel.id, panel.element.parentElement as HTMLElement | null])
  );
  let synchronizing = false;
  let persistTimer: ReturnType<typeof setTimeout> | null = null;
  let disposed = false;

  const createRenderer = (): IContentRenderer => {
    const element = document.createElement('div');
    element.className = 'center-panel-host';
    let panelId: string | null = null;
    return {
      element,
      init(parameters: GroupPanelPartInitParameters): void {
        const candidate = parameters.params?.panelId;
        if (typeof candidate !== 'string' || !specs.has(candidate)) {
          element.textContent = `Unknown panel: ${String(candidate)}`;
          return;
        }
        panelId = candidate;
        const content = specs.get(candidate)!.element;
        if (content.parentElement !== element) element.replaceChildren(content);
      },
      layout(): void {
        if (panelId) options.onPanelLayout?.(panelId);
      },
      dispose(): void {
        if (disposed || !panelId) return;
        // Panel closed while the shell lives on: hand the content back to its
        // Svelte-owned parking node so nothing app-owned dies with dockview.
        const content = specs.get(panelId)?.element;
        const home = parking.get(panelId) ?? null;
        if (content && home && content.parentElement !== home) home.appendChild(content);
      }
    };
  };

  const api = createDockview(container, {
    className: 'shell-center-dock',
    defaultRenderer: 'always',
    dndStrategy: 'pointer',
    hideBorders: true,
    scrollbars: 'native',
    theme: { ...themeDracula, gap: 0 },
    createComponent: createRenderer
  });

  const buildDefault = (): void => {
    for (const panel of options.panels) {
      api.addPanel({
        id: panel.id,
        title: panel.title,
        component: COMPONENT,
        params: { panelId: panel.id }
      });
    }
    const first = options.panels[0];
    if (first) api.getPanel(first.id)?.api.setActive();
  };

  const runSynchronized = (fn: () => void): void => {
    synchronizing = true;
    try {
      fn();
    } finally {
      synchronizing = false;
    }
  };

  runSynchronized(() => {
    const stored = loadLayout<object>(options.storage, CENTER_LAYOUT_KEY);
    if (stored && panelSetMatches(dockPanelIds(stored), specs.keys())) {
      try {
        api.fromJSON(stored as never);
        return;
      } catch {
        try {
          api.clear();
        } catch {
          // fall through
        }
      }
    }
    buildDefault();
  });

  const persistSoon = (): void => {
    if (synchronizing || disposed) return;
    if (persistTimer !== null) clearTimeout(persistTimer);
    persistTimer = setTimeout(() => {
      persistTimer = null;
      if (disposed) return;
      const ok = saveLayout(options.storage, CENTER_LAYOUT_KEY, api.toJSON());
      options.onLayoutPersisted?.(ok);
    }, PERSIST_DEBOUNCE_MS);
  };

  const listeners = [api.onDidLayoutChange(persistSoon)];

  return {
    api,
    activatePanel(id: string): void {
      api.getPanel(id)?.api.setActive();
    },
    resetLayout(): void {
      clearLayout(options.storage, CENTER_LAYOUT_KEY);
      runSynchronized(() => {
        api.clear();
        buildDefault();
      });
      persistSoon();
    },
    dispose(): void {
      disposed = true; // renderer dispose() no-ops: page teardown owns the DOM now
      if (persistTimer !== null) clearTimeout(persistTimer);
      for (const listener of listeners) listener.dispose();
      api.dispose();
    }
  };
}
```

- [ ] **Step 2: Type-check**

Run: `cd tauri-svelte-preview && pnpm run check`
Expected: clean. (`params` on `GroupPanelPartInitParameters` is `Record<string, any>`-ish in 6.6.1 — if tsc complains about the `panelId` access, mirror the old shell's cast pattern at `sourceDockviewWorkspace.ts:1310-1315`, which faced the same shape.)

- [ ] **Step 3: Commit**

```bash
git add tauri-svelte-preview/src/lib/shell/layout/centerDock.ts
git commit -m "feat(next): center Dockview with teleport renderers and always-attached panels"
```

---

### Task 5: `ShellFrame.svelte` + `PanelPlaceholder.svelte` — the Svelte wrapper and the CSS

**Files:**
- Create: `tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte`
- Create: `tauri-svelte-preview/src/lib/shell/components/PanelPlaceholder.svelte`

**Interfaces:**
- Consumes: Tasks 3+4 factories; `dockview-core/dist/styles/dockview.css`.
- Produces: `<ShellFrame rail center context dock onSessionPanelLayout onReady />` where `rail`/`context`/`dock` are Svelte 5 **snippets**, `center` is `{ session: Snippet; editor: Snippet; browser: Snippet }`, `onSessionPanelLayout(): void` fires on any session-panel geometry change (Task 6 debounces + refits), and `onReady(controls: { resetLayout(): void })` hands the page reset control. The component renders ALL snippet content into a hidden parking stage, mounts the grid + dock in `onMount` (DOM-only — allowed; it is not backend IO), and disposes both on unmount.

- [ ] **Step 1: Create `PanelPlaceholder.svelte`**

```svelte
<script lang="ts">
  /** A shallow slot: names the future panel, does nothing, loads nothing. */
  interface Props {
    name: string;
    hint?: string;
  }
  let { name, hint = 'Coming in the parallel wave.' }: Props = $props();
</script>

<div class="placeholder">
  <p class="name">{name}</p>
  <p class="hint">{hint}</p>
</div>

<style>
  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    height: 100%;
    width: 100%;
    background: #101014;
    color: #6d6d7d;
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
    font-size: 12px;
    user-select: none;
  }
  .placeholder p {
    margin: 0;
  }
  .hint {
    color: #4c4c5a;
    font-size: 11px;
  }
</style>
```

- [ ] **Step 2: Create `ShellFrame.svelte`**

```svelte
<script lang="ts">
  /**
   * ShellFrame.svelte — mounts the Gridview root + center Dockview and
   * teleports Svelte-owned content into them. All content is authored in the
   * hidden parking stage below, so dockview NEVER owns app DOM; if the frame
   * fails to mount, content simply stays parked (invisible) and the page's
   * error rail reports it. No backend IO anywhere in this component.
   */
  import 'dockview-core/dist/styles/dockview.css';
  import { onMount, type Snippet } from 'svelte';

  import { createCenterDock, type CenterDock } from '$lib/shell/layout/centerDock';
  import { createShellFrame, type ShellFrame as Frame } from '$lib/shell/layout/frame';

  interface Props {
    rail: Snippet;
    center: { session: Snippet; editor: Snippet; browser: Snippet };
    context: Snippet;
    dock: Snippet;
    onSessionPanelLayout?: () => void;
    onReady?: (controls: { resetLayout: () => void }) => void;
    onError?: (message: string) => void;
  }
  let { rail, center, context, dock, onSessionPanelLayout, onReady, onError }: Props = $props();

  let gridHost: HTMLElement;
  let railSlot: HTMLElement;
  let centerSlot: HTMLElement; // holds the center Dockview's own container
  let contextSlot: HTMLElement;
  let dockSlot: HTMLElement;
  let sessionSlot: HTMLElement;
  let editorSlot: HTMLElement;
  let browserSlot: HTMLElement;

  let frame: Frame | null = null;
  let centerDock: CenterDock | null = null;
  let ready = $state(false);

  onMount(() => {
    let observer: ResizeObserver | null = null;
    try {
      frame = createShellFrame(gridHost, {
        storage: window.localStorage,
        regions: { rail: railSlot, center: centerSlot, context: contextSlot, dock: dockSlot }
      });
      centerDock = createCenterDock(centerSlot, {
        storage: window.localStorage,
        panels: [
          { id: 'session', title: 'Session', element: sessionSlot },
          { id: 'editor', title: 'Editor', element: editorSlot },
          { id: 'browser', title: 'Browser', element: browserSlot }
        ],
        onPanelLayout: (id) => {
          if (id === 'session') onSessionPanelLayout?.();
        }
      });
      frame.layout(gridHost.clientWidth, gridHost.clientHeight);
      observer = new ResizeObserver(() => {
        frame?.layout(gridHost.clientWidth, gridHost.clientHeight);
      });
      observer.observe(gridHost);
      ready = true;
      onReady?.({
        resetLayout: () => {
          frame?.resetLayout();
          centerDock?.resetLayout();
        }
      });
    } catch (error) {
      onError?.(error instanceof Error ? error.message : String(error));
    }

    return () => {
      observer?.disconnect();
      centerDock?.dispose();
      centerDock = null;
      frame?.dispose();
      frame = null;
    };
  });
</script>

<div class="shell-frame" class:ready bind:this={gridHost}></div>

<!-- Parking stage: content lives here until (and unless) dockview claims it.
     1px off-screen, hidden, layout-contained — the old shell's proven pattern. -->
<div class="parking-stage" aria-hidden="true">
  <div class="slot" bind:this={railSlot}>{@render rail()}</div>
  <div class="slot slot-center-dock" bind:this={centerSlot}></div>
  <div class="slot" bind:this={contextSlot}>{@render context()}</div>
  <div class="slot" bind:this={dockSlot}>{@render dock()}</div>
  <div class="slot" bind:this={sessionSlot}>{@render center.session()}</div>
  <div class="slot" bind:this={editorSlot}>{@render center.editor()}</div>
  <div class="slot" bind:this={browserSlot}>{@render center.browser()}</div>
</div>

<style>
  .shell-frame {
    height: 100%;
    width: 100%;
    overflow: hidden;
    visibility: hidden; /* anti-flash: revealed when ready */
  }
  .shell-frame.ready {
    visibility: visible;
  }

  .parking-stage {
    position: fixed;
    left: -1px;
    bottom: -1px;
    width: 1px;
    height: 1px;
    overflow: hidden;
    visibility: hidden;
    contain: layout paint;
  }

  /* Teleported slots and hosts must fill whatever cell they land in. */
  .slot,
  :global(.shell-region-host),
  :global(.center-panel-host) {
    height: 100%;
    width: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  /* ---- dockview overrides (ported from the old shell's audited CSS) ----
     BOTH theme classes must be targeted where dracula reasserts values.   */
  .shell-frame :global(.shell-grid),
  .shell-frame :global(.shell-center-dock) {
    /* token map: translate dockview vars onto the /next palette */
    --dv-group-view-background-color: #101014;
    --dv-tabs-and-actions-container-background-color: #101014;
    --dv-activegroup-visiblepanel-tab-background-color: #17171d;
    --dv-activegroup-hiddenpanel-tab-background-color: #101014;
    --dv-inactivegroup-visiblepanel-tab-background-color: #14141a;
    --dv-inactivegroup-hiddenpanel-tab-background-color: #101014;
    --dv-tab-divider-color: transparent;
    --dv-activegroup-visiblepanel-tab-color: #d8d8e0;
    --dv-activegroup-hiddenpanel-tab-color: #6d6d7d;
    --dv-inactivegroup-visiblepanel-tab-color: #9a9aa8;
    --dv-inactivegroup-hiddenpanel-tab-color: #6d6d7d;
    --dv-separator-border: #22222c;
    --dv-paneview-active-outline-color: transparent;
    /* NEUTRAL wash only — the teal accent here caused the green drag-flash. */
    --dv-drag-over-background-color: rgba(255, 255, 255, 0.05);
    --dv-drag-over-border-color: transparent;
  }

  /* seam flattening */
  .shell-frame :global(.dv-groupview),
  .shell-frame :global(.dv-tabs-and-actions-container),
  .shell-frame :global(.dv-content-container),
  .shell-frame :global(.dv-tabs-container),
  .shell-frame :global(.dv-tab) {
    border-color: transparent;
    box-shadow: none;
  }

  /* sizing chain — without this, dockview panels collapse in flex/grid parents */
  .shell-frame :global(.dv-dockview),
  .shell-frame :global(.dv-gridview),
  .shell-frame :global(.dv-grid-view),
  .shell-frame :global(.dv-branch-node),
  .shell-frame :global(.dv-view-container),
  .shell-frame :global(.dv-view) {
    min-width: 0;
    min-height: 0;
  }
</style>
```

- [ ] **Step 3: Type-check + dead-render smoke**

Run: `cd tauri-svelte-preview && pnpm run check`
Expected: clean. (Live rendering is proven in Task 6 — this component has no consumer yet.)

- [ ] **Step 4: Commit**

```bash
git add tauri-svelte-preview/src/lib/shell/components/ShellFrame.svelte tauri-svelte-preview/src/lib/shell/components/PanelPlaceholder.svelte
git commit -m "feat(next): ShellFrame component — parking stage, teleport wiring, dockview skin"
```

---

### Task 6: Rewire `/next` onto the skeleton

**Files:**
- Modify: `tauri-svelte-preview/src/routes/next/+page.svelte`

**Interfaces:**
- Consumes: `<ShellFrame>` (Task 5), `service.refit()` (Task 2). All session logic (scan, adopt, reattach, close, select) is UNCHANGED.
- Produces: the live walking skeleton. The page's markup section becomes: `<ShellFrame>` with `rail` → existing `<SessionRail …/>`, `center.session` → existing `<TerminalSurface …/>`, `center.editor`/`center.browser`/`context` → `<PanelPlaceholder …/>`, `dock` → a `<PanelPlaceholder name="Dock">` plus a small "Reset layout" button; error footer and invoke counter overlays stay.

**Hard limit:** the file stays ≤ 300 lines. The old two-column `<main>` grid CSS goes away (ShellFrame owns geometry); that pays for the new lines.

- [ ] **Step 1: Rewire the markup + refit**

Script-side additions (complete):

```ts
import PanelPlaceholder from '$lib/shell/components/PanelPlaceholder.svelte';
import ShellFrame from '$lib/shell/components/ShellFrame.svelte';

let frameControls: { resetLayout: () => void } | null = null;
let refitScheduled = false;

/** Coalesce dockview's layout bursts into one refit per frame. */
function scheduleRefit(): void {
  if (refitScheduled) return;
  refitScheduled = true;
  requestAnimationFrame(() => {
    refitScheduled = false;
    service?.refit();
  });
}
```

Markup (replaces the current `<main class="next-shell">` two-column body; error/counter footers stay as siblings inside the main wrapper).

**Naming trap, do not "simplify" this:** the page imports the rune store named `rail` — an implicit child snippet `{#snippet rail()}` would shadow it and break every `rail.owned` access. Therefore ALL snippets are declared at the top level of the template with `Area`-suffixed names and passed to `<ShellFrame>` explicitly:

```svelte
{#snippet railArea()}
  <SessionRail
    owned={rail.owned}
    available={rail.available}
    activeOwnedId={rail.activeOwnedId}
    scanning={rail.scanning}
    onSelect={selectOwned}
    onAdopt={adopt}
    onClose={closeOwned}
    onRescan={scanRail}
  />
{/snippet}
{#snippet contextArea()}
  <PanelPlaceholder name="Context" hint="Runs · agents · worktrees · git — parallel wave." />
{/snippet}
{#snippet dockArea()}
  <div class="dock-slot">
    <PanelPlaceholder name="Dock" hint="Secondary terminals & logs — parallel wave." />
    <button class="reset-layout" onclick={() => frameControls?.resetLayout()}>
      Reset layout
    </button>
  </div>
{/snippet}
{#snippet sessionArea()}
  <TerminalSurface owned={rail.owned} activeOwnedId={rail.activeOwnedId} {registerHost} />
{/snippet}
{#snippet editorArea()}
  <PanelPlaceholder name="Editor" hint="Monaco + LSP code reading — parallel wave." />
{/snippet}
{#snippet browserArea()}
  <PanelPlaceholder name="Browser" />
{/snippet}

<main class="next-shell">
  <ShellFrame
    rail={railArea}
    context={contextArea}
    dock={dockArea}
    center={{ session: sessionArea, editor: editorArea, browser: browserArea }}
    onSessionPanelLayout={scheduleRefit}
    onReady={(controls) => (frameControls = controls)}
    onError={(message) => (rail.error = `layout failed: ${message}`)}
  />

  {#if rail.error}
    <footer class="next-error">{rail.error}</footer>
  {/if}
  {#if import.meta.env.DEV}
    <footer class="invoke-counter">invokes: {invokeCounts.total} (+{invokeCounts.input} input)</footer>
  {/if}
</main>
```

CSS changes in the page: DELETE the grid-template rules from `.next-shell` (keep `position: relative; height: 100vh; width: 100vw; overflow: hidden; background; color`), DELETE `.next-rail` / `.next-main` blocks entirely, ADD:

```css
.dock-slot {
  position: relative;
  height: 100%;
}
.reset-layout {
  position: absolute;
  top: 6px;
  right: 8px;
  background: transparent;
  border: 1px solid #22222c;
  border-radius: 5px;
  color: #6d6d7d;
  font-family: ui-monospace, Menlo, monospace;
  font-size: 10px;
  padding: 2px 7px;
  cursor: pointer;
}
.reset-layout:hover {
  color: #d8d8e0;
  border-color: #3a3a48;
}
```

- [ ] **Step 2: Line-count + type-check + tests**

Run: `wc -l tauri-svelte-preview/src/routes/next/+page.svelte` → must print ≤ 300.
Run: `cd tauri-svelte-preview && pnpm run check && pnpm run test:owned-sessions && pnpm run test:next-terminal-service && pnpm run test:layout-storage`
Expected: all clean/pass.

- [ ] **Step 3: Live smoke (implementer-level, dev server)**

With the existing `pnpm run tauri:dev:next` app running (do NOT restart it — verify via the browser at `http://127.0.0.1:5177/next` if the Tauri window is in use):
- Four regions render; center shows Session/Editor/Browser tabs; placeholders name themselves.
- Adopt or start a session: terminal renders inside the Session tab, input works.
- Switch to Editor tab and back: terminal intact, no re-spawn (invoke counter unmoved except gated resize).
- Drag every sash; watch the invoke counter: only `resize_terminal_session` when the terminal grid truly changed, nothing else.
- Reload the page: layout arrangement restored; sessions re-attach exactly as before Slice 2.
- Click "Reset layout": default arrangement returns without a reload; terminal still live.

- [ ] **Step 4: Commit**

```bash
git add tauri-svelte-preview/src/routes/next/+page.svelte
git commit -m "feat(next): mount the walking skeleton — grid frame, center tabs, live session panel"
```

---

### Task 7: Milestone — user live verification + whole-branch review

(Per the standing directive: code reviews at milestones only — this is the milestone.)

- [ ] **Step 1: Hand to the user for live verification** against the same checklist as Task 6 Step 3, plus their own multi-session workflow (2–3 live agents, switching, hidden progress).
- [ ] **Step 2: Dispatch the final whole-branch reviewer** (superpowers requesting-code-review flow) over the full Slice 2 diff range, carrying the Global Constraints block and the deferred-items list below for triage.
- [ ] **Step 3: Fix wave if needed** (one fix dispatch + scoped re-review), then append the verified-live note + any newly deferred items to THIS plan file and commit.

---

## Deferred out of Slice 2 (recorded up front; final review triages)

- Rail/context collapse-toggle (GridviewApi exposes no `setVisible`; needs remove/re-add or a component-level reach-in — design in the parallel wave's settings/layout lane).
- A center tab closed by the user reappears on next launch (the panel-set equality guard rebuilds). Fine for a skeleton with 3 fixed tabs.
- Bottom dock is a placeholder Gridview cell, not its own Dockview; it becomes one when secondary terminals/logs land.
- Custom tab renderers (lucide icon + overflow menu), Bits UI, and the tokens.css design system — parallel wave (settings/theming lane).
- Per-conversation center layouts (each session owning its own tab arrangement) — parallel wave, after the editor exists to make it meaningful.
