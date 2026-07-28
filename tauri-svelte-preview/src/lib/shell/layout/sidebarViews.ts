/**
 * sidebarViews.ts — which views the left column offers, and which one it opens.
 *
 * PURE: no DOM, no dockview, no backend call. The activity bar and the column
 * itself both read the roster from here so an id can never mean two different
 * things in the two places.
 *
 * Each view holds its own stack of collapsible panes and remembers it under its
 * own key, so opening Explorer cannot disturb how Sessions was arranged.
 *
 * The single shared stack these replaced wrote to
 * `mac-command-bar.next.left-panes`. That key is deliberately never read again:
 * it describes one stack of four sections, a shape the column no longer builds.
 * It is left where it is, harmless and unread — the same treatment the center
 * dock's pre-v2 layout key got.
 */
import { clearLayout, loadLayout, saveLayout, type LayoutStorage } from './layoutStorage.ts';

export type SidebarViewId = 'sessions' | 'explorer' | 'source-control' | 'worktrees';

export interface SidebarView {
  id: SidebarViewId;
  /** What the icon's tooltip says. */
  title: string;
}

/** Top to bottom, in the order the activity bar draws them. */
export const SIDEBAR_VIEWS: readonly SidebarView[] = [
  { id: 'sessions', title: 'Sessions' },
  { id: 'explorer', title: 'Explorer' },
  { id: 'source-control', title: 'Source control' },
  { id: 'worktrees', title: 'Worktrees' }
];

/** The view a first launch opens on, and the one a reset goes back to. */
export const DEFAULT_SIDEBAR_VIEW: SidebarViewId = 'sessions';

export const ACTIVE_VIEW_KEY = 'mac-command-bar.next.active-view';

export function isSidebarViewId(value: unknown): value is SidebarViewId {
  return typeof value === 'string' && SIDEBAR_VIEWS.some((view) => view.id === value);
}

/** Where one view's pane sizes and folded/open states are remembered. */
export function viewPanesKey(id: SidebarViewId): string {
  return `mac-command-bar.next.view-${id}-panes`;
}

/**
 * The view to open. Anything unreadable, corrupt, or naming a view that no
 * longer exists comes back as the default — a stored id must never be able to
 * open the column on nothing.
 */
export function readActiveView(storage: LayoutStorage): SidebarViewId {
  const stored = loadLayout<unknown>(storage, ACTIVE_VIEW_KEY);
  return isSidebarViewId(stored) ? stored : DEFAULT_SIDEBAR_VIEW;
}

/** Remember the open view. False means the write was refused (a full storage);
 * the column carries on regardless — the cost is only that the next launch
 * opens somewhere else. */
export function writeActiveView(storage: LayoutStorage, id: SidebarViewId): boolean {
  return saveLayout(storage, ACTIVE_VIEW_KEY, id);
}

export function clearActiveView(storage: LayoutStorage): void {
  clearLayout(storage, ACTIVE_VIEW_KEY);
}
