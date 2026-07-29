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

/**
 * Bumped to `-v2` when the shell's regions were rearranged: the sessions list
 * took a column of its own down the left, every tool view moved to a column on
 * the right, and the icon strip moved to the far right edge. A layout saved
 * under the old key describes four regions that no longer exist by those names,
 * with the sizes and the left-to-right order of the old arrangement. There is
 * no honest way to translate one into the other — the old layout says nothing
 * about how wide the user wants two columns that were never there — so it is
 * not migrated. It is left where it is, harmless and unread, and the new
 * arrangement starts from its defaults.
 */
export const GRID_LAYOUT_KEY = 'mac-command-bar.next.grid-layout-v2';
/**
 * Bumped to `-v2` when the center dock stopped being one group of tabs and
 * became a conversation group beside a display group, and to `-v3` when the
 * Diff tab joined the display group. A stored layout is only restored when its
 * tabs are exactly the tabs the shell now builds, so without the bump every
 * existing install would keep its three-tab arrangement and the Diff tab would
 * never appear. Layouts saved under either older key are left where they are,
 * harmless and unread.
 */
export const CENTER_LAYOUT_KEY = 'mac-command-bar.next.center-layout-v3';

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

/**
 * Pane ids inside a `SerializedPaneview` (dockview-core 6.6.1 shape:
 * `{ size, views: [{ size, expanded?, data: { id, component, title, … } }] }`).
 * Collapsed panes are in `views` exactly like open ones, so this returns the
 * whole stack either way. Malformed input yields an empty set — the caller then
 * rebuilds from defaults.
 */
export function paneviewPanelIds(serialized: unknown): Set<string> {
  const ids = new Set<string>();
  const views = (serialized as { views?: unknown } | null)?.views;
  if (!Array.isArray(views)) return ids;
  for (const view of views) {
    if (!view || typeof view !== 'object') continue;
    const data = (view as { data?: unknown }).data;
    if (!data || typeof data !== 'object') continue;
    const id = (data as { id?: unknown }).id;
    if (typeof id === 'string') ids.add(id);
  }
  return ids;
}

/** True when `ids` is EXACTLY `expected` (both directions, order-free). */
export function panelSetMatches(ids: Iterable<string>, expected: Iterable<string>): boolean {
  const a = new Set(ids);
  const b = new Set(expected);
  if (a.size !== b.size) return false;
  for (const id of a) if (!b.has(id)) return false;
  return true;
}
