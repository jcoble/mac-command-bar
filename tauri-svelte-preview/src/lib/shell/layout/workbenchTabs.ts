/**
 * workbenchTabs.ts — which tab each session was left on.
 *
 * The center pane and the right panel each remember their selected tab PER
 * SESSION: coming back to a session brings back the arrangement it was being
 * worked in, and a different session is free to have been left somewhere else.
 * Both keys hold one record of `ownedId -> tab id`; a shell with no session
 * picked uses the `''` slot.
 *
 * PURE: no DOM, no dockview, no backend call. Storage is handed in, the same
 * way `sidebarViews.ts` and `layoutStorage.ts` take it, so node tests run
 * without a browser. Every read is total — unreadable, corrupt, or naming a tab
 * that no longer exists all come back as the default, because a stored id must
 * never be able to leave a column showing nothing.
 */
import {
  CENTER_TAB_IDS,
  RIGHT_TAB_IDS,
  type CenterTabId,
  type RightTabId
} from '../workbenchNavigation.ts';
import { clearLayout, loadLayout, saveLayout, type LayoutStorage } from './layoutStorage.ts';

export const CENTER_TAB_KEY = 'mac-command-bar.next.center-tab-v1';
export const RIGHT_TAB_KEY = 'mac-command-bar.next.right-tab-v1';

export const DEFAULT_CENTER_TAB: CenterTabId = 'session';
export const DEFAULT_RIGHT_TAB: RightTabId = 'files';

export function isCenterTabId(value: unknown): value is CenterTabId {
  return typeof value === 'string' && CENTER_TAB_IDS.some((id) => id === value);
}

export function isRightTabId(value: unknown): value is RightTabId {
  return typeof value === 'string' && RIGHT_TAB_IDS.some((id) => id === value);
}

/** The slot one session's answer is kept in. No session picked is its own slot
 * rather than a missing one, so the shell remembers where it was left too. */
function slotFor(ownedId: string | null): string {
  return ownedId ?? '';
}

/** Whatever is stored under `key`, as a plain record. Anything else — a string,
 * an array, unparseable text — comes back empty. */
function readRecord(storage: LayoutStorage, key: string): Record<string, unknown> {
  const stored = loadLayout<unknown>(storage, key);
  if (!stored || typeof stored !== 'object' || Array.isArray(stored)) return {};
  return stored as Record<string, unknown>;
}

function writeSlot(
  storage: LayoutStorage,
  key: string,
  ownedId: string | null,
  id: string
): boolean {
  const record = readRecord(storage, key);
  record[slotFor(ownedId)] = id;
  return saveLayout(storage, key, record);
}

export function readCenterTab(storage: LayoutStorage, ownedId: string | null): CenterTabId {
  const stored = readRecord(storage, CENTER_TAB_KEY)[slotFor(ownedId)];
  return isCenterTabId(stored) ? stored : DEFAULT_CENTER_TAB;
}

/** Remember the center tab. False means the write was refused (a full storage);
 * the shell carries on — the cost is only that the next launch opens on the
 * default. */
export function writeCenterTab(
  storage: LayoutStorage,
  ownedId: string | null,
  id: CenterTabId
): boolean {
  return writeSlot(storage, CENTER_TAB_KEY, ownedId, id);
}

export function readRightTab(storage: LayoutStorage, ownedId: string | null): RightTabId {
  const stored = readRecord(storage, RIGHT_TAB_KEY)[slotFor(ownedId)];
  return isRightTabId(stored) ? stored : DEFAULT_RIGHT_TAB;
}

/** Remember the right tab. Same contract as `writeCenterTab`. */
export function writeRightTab(
  storage: LayoutStorage,
  ownedId: string | null,
  id: RightTabId
): boolean {
  return writeSlot(storage, RIGHT_TAB_KEY, ownedId, id);
}

/** Forget every session's tabs — what "Reset layout" means for this pair. */
export function clearWorkbenchTabs(storage: LayoutStorage): void {
  clearLayout(storage, CENTER_TAB_KEY);
  clearLayout(storage, RIGHT_TAB_KEY);
}
