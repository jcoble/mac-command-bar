/**
 * sessionGroups.ts — how the /next session rail is grouped and filtered.
 *
 * PURE: no store, no DOM, no backend call. It takes the rail's two lists and a
 * search box's text and returns the same sessions arranged under one heading
 * per project folder, so a rail holding twenty sessions across four projects
 * reads as four short lists instead of one long one.
 *
 * Nothing here decides what a session IS — the records arrive already ordered
 * by the stores, and that order is preserved inside every group.
 *
 * It also decides which headings open, and how many rows a heading shows before
 * it stops. A machine that has been running agents for months can offer several
 * hundred sessions to resume; drawn all at once they are not a list anybody can
 * read. Both decisions are here rather than in the component so they can be
 * tested without a browser.
 */
import { loadLayout, saveLayout, type LayoutStorage } from './layout/layoutStorage.ts';
import type { OwnedSession } from './ownedSessions';
import type { AgentSession } from '$lib/tauriSource';

export interface SessionGroup<T> {
  /** Project folder name, e.g. "mac-command-bar"; "Other" when no path is known. */
  name: string;
  /** Full path used as the stable group key ('' for Other). */
  path: string;
  items: T[];
}

function projectOf(path: string | null | undefined): { name: string; path: string } {
  const trimmed = (path ?? '').trim();
  if (!trimmed) return { name: 'Other', path: '' };
  const parts = trimmed.split('/').filter(Boolean);
  return { name: parts[parts.length - 1] ?? trimmed, path: trimmed };
}

function matches(query: string, ...fields: Array<string | null | undefined>): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  return fields.some((field) => (field ?? '').toLowerCase().includes(needle));
}

/** Group + filter the rail's two lists. Groups sort by most items first, "Other" last;
 * items keep their incoming order (the stores already order by recency). */
export function groupSessions(
  owned: OwnedSession[],
  available: AgentSession[],
  query: string
): { owned: SessionGroup<OwnedSession>[]; available: SessionGroup<AgentSession>[] } {
  const groupBy = <T>(
    items: T[],
    pathOf: (item: T) => string | null | undefined,
    hit: (item: T) => boolean
  ) => {
    const groups = new Map<string, SessionGroup<T>>();
    for (const item of items) {
      if (!hit(item)) continue;
      const { name, path } = projectOf(pathOf(item));
      const existing = groups.get(path) ?? { name, path, items: [] };
      existing.items.push(item);
      groups.set(path, existing);
    }
    return [...groups.values()].sort((a, b) =>
      a.path === '' ? 1 : b.path === '' ? -1 : b.items.length - a.items.length
    );
  };
  return {
    owned: groupBy(
      owned,
      (s) => s.projectPath ?? s.cwd,
      (s) => matches(query, s.title, s.projectPath, s.cwd)
    ),
    available: groupBy(
      available,
      (s) => s.projectPath,
      (s) => matches(query, s.title, s.projectPath, s.description)
    )
  };
}

/** The key a session's path groups under — '' for sessions with no folder. */
export function sessionGroupPath(path: string | null | undefined): string {
  return projectOf(path).path;
}

/** Which of the rail's two lists a heading belongs to. */
export type SessionList = 'owned' | 'resume';

/**
 * How many rows an open Resume heading draws before it stops and offers the
 * rest. Eight is about a screen's worth beside the list above it — enough to
 * recognise the session you came for, few enough that four projects still fit.
 */
export const RESUME_GROUP_ROW_CAP = 8;

export const RAIL_GROUPS_STORAGE_KEY = 'mac-command-bar.next.rail-groups';

/** Headings the user has opened or closed by hand, by `groupToggleKey`. */
export type GroupExpansion = Record<string, boolean>;

/** One heading in one list. The two lists can hold the same project folder. */
export function groupToggleKey(list: SessionList, path: string): string {
  return `${list}:${path}`;
}

/**
 * Whether a heading is open.
 *
 * The user's own choice always wins. Failing that: everything the rail already
 * owns is open, because those are the sessions being worked on right now and
 * there are rarely many. Everything offered for resume is closed, except the
 * project the session on screen belongs to — that is the one folder the user is
 * plainly in the middle of.
 *
 * A search overrides all of it. Rows hidden under a closed heading would make
 * the box look broken: you type a title you can see is there and get nothing.
 */
export function isGroupExpanded(options: {
  list: SessionList;
  path: string;
  remembered: GroupExpansion;
  activeProjectPath: string | null;
  searching: boolean;
}): boolean {
  if (options.searching) return true;

  const chosen = options.remembered[groupToggleKey(options.list, options.path)];
  if (typeof chosen === 'boolean') return chosen;

  if (options.list === 'owned') return true;
  return options.activeProjectPath !== null && options.path === options.activeProjectPath;
}

/**
 * Record a heading the user opened or closed. A choice that matches what the
 * rail would have done anyway is forgotten rather than stored, so the remembered
 * set stays as small as the number of headings actually argued with.
 */
export function rememberGroupToggle(
  remembered: GroupExpansion,
  key: string,
  expanded: boolean,
  fallback: boolean
): GroupExpansion {
  const next = { ...remembered };
  if (expanded === fallback) delete next[key];
  else next[key] = expanded;
  return next;
}

/**
 * The rows an open heading draws, and how many it is holding back. `showAll` is
 * the "Show N more" row having been clicked — deliberately not remembered, so
 * every visit starts short again.
 */
export function visibleGroupItems<T>(
  items: T[],
  cap: number,
  showAll: boolean
): { shown: T[]; hiddenCount: number } {
  if (showAll || cap <= 0 || items.length <= cap) return { shown: items, hiddenCount: 0 };
  return { shown: items.slice(0, cap), hiddenCount: items.length - cap };
}

/** Anything unreadable or not a map of booleans comes back as no choices made. */
export function readGroupExpansion(storage: LayoutStorage): GroupExpansion {
  const stored = loadLayout<unknown>(storage, RAIL_GROUPS_STORAGE_KEY);
  if (!stored || typeof stored !== 'object' || Array.isArray(stored)) return {};

  const expansion: GroupExpansion = {};
  for (const [key, value] of Object.entries(stored)) {
    if (typeof value === 'boolean') expansion[key] = value;
  }
  return expansion;
}

/** False means the write was refused (a full storage). The rail carries on: the
 * cost is only that the next launch opens the headings it would have anyway. */
export function writeGroupExpansion(storage: LayoutStorage, expansion: GroupExpansion): boolean {
  return saveLayout(storage, RAIL_GROUPS_STORAGE_KEY, expansion);
}
