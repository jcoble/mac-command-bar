/**
 * workbenchTabs.ts — which tab each session was left on.
 *
 * The center pane and right panel remember their selected tabs in the active
 * session's SQLite workspace snapshot. This pure module owns only the valid
 * values and defaults; it has no persistence path of its own.
 */
import {
  CENTER_TAB_IDS,
  RIGHT_TAB_IDS,
  type CenterTabId,
  type RightTabId
} from '../workbenchNavigation.ts';

export const DEFAULT_CENTER_TAB: CenterTabId = 'session';
export const DEFAULT_RIGHT_TAB: RightTabId = 'files';

export function isCenterTabId(value: unknown): value is CenterTabId {
  return typeof value === 'string' && CENTER_TAB_IDS.some((id) => id === value);
}

export function isRightTabId(value: unknown): value is RightTabId {
  return typeof value === 'string' && RIGHT_TAB_IDS.some((id) => id === value);
}
