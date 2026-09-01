/**
 * Legacy pure helpers for grouping owned and scanned sessions by project.
 *
 * The current My Work rail uses `buildMyWorkGroups`; this module retains the
 * older grouping model and the worktree-parent label used by `SessionCard`.
 * It has no store, DOM, backend call, or persistence owner.
 */
import type { OwnedSession } from './ownedSessions';
import type { AgentSession } from '$lib/tauriSource';

export interface SessionGroup<T> {
  /** Project folder name, e.g. "mac-command-bar"; "Other" when no path is known. */
  name: string;
  /** The repository a worktree belongs to, when the path says so. Null for an
   * ordinary folder. Drawn after the name, dimmed. */
  parentProject: string | null;
  /** Full path used as the stable group key ('' for Other). */
  path: string;
  items: T[];
}

/**
 * The repository a worktree checkout belongs to.
 *
 * A worktree is named for the task, not the project — `tsk-670-role-experience`
 * says nothing about which repository it is a checkout of, and the rail can end
 * up showing several of them from different projects with no way to tell. The
 * layout convention `.../worktrees/<project>/<worktree>` carries the answer, so
 * where a path has that shape the project name is pulled out of it.
 *
 * Null whenever the path does not say — a folder literally called `worktrees`
 * with checkouts directly inside it names no project, and guessing one would be
 * worse than staying quiet.
 */
export function worktreeParentProject(path: string | null | undefined): string | null {
  const parts = (path ?? '').trim().split('/').filter(Boolean);
  const marker = parts.lastIndexOf('worktrees');
  // A project folder AND a worktree folder have to follow the marker.
  if (marker < 0 || parts.length - marker < 3) return null;

  return parts[marker + 1] ?? null;
}

function projectOf(path: string | null | undefined): {
  name: string;
  parentProject: string | null;
  path: string;
} {
  const trimmed = (path ?? '').trim();
  if (!trimmed) return { name: 'Other', parentProject: null, path: '' };

  const parts = trimmed.split('/').filter(Boolean);
  const name = parts[parts.length - 1] ?? trimmed;
  const parent = worktreeParentProject(trimmed);
  // "foo · foo" is noise, not context.
  return { name, parentProject: parent === name ? null : parent, path: trimmed };
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
      const { name, parentProject, path } = projectOf(pathOf(item));
      const existing = groups.get(path) ?? { name, parentProject, path, items: [] };
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

/**
 * What a project folder is CALLED — the same name a heading would show, for a
 * row that is not under one.
 *
 * Your own sessions are listed as cards under Working and Done rather than
 * under a heading per folder, so each card says which project it belongs to
 * itself. It is the same answer the headings use, from the same function, so
 * the two lists never disagree about what a folder is called.
 */
export function projectLabel(path: string | null | undefined): {
  name: string;
  parentProject: string | null;
} {
  const { name, parentProject } = projectOf(path);
  return { name, parentProject };
}

/** Which of the rail's two lists a heading belongs to. */
export type SessionList = 'owned' | 'resume';

/**
 * How many rows an open Resume heading draws before it stops and offers the
 * rest. Eight is about a screen's worth beside the list above it — enough to
 * recognise the session you came for, few enough that four projects still fit.
 */
export const RESUME_GROUP_ROW_CAP = 8;

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
