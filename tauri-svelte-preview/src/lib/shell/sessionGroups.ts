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
 */
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
