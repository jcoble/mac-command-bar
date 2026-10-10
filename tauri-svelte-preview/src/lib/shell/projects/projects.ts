/**
 * Plain rules for the project registry as the new-session screen shows it.
 * The records themselves live in SQLite; see `projectRegistry.svelte.ts`.
 */

import type { ProjectRecord } from '../../tauriSource.ts';

/**
 * Two letters for a project: the first letters of its first and last word, or
 * the first and last letter of a single word (mac-command-bar → MB,
 * EdiPlatform → EM).
 */
export function projectBadge(title: string): string {
  const words = title.split(/[^A-Za-z0-9]+/).filter(Boolean);
  if (words.length > 1) return `${words[0][0]}${words.at(-1)![0]}`.toUpperCase();
  const word = words[0] ?? '';
  return (word.length > 1 ? `${word[0]}${word.at(-1)}` : word).toUpperCase();
}

/** Projects on this Mac or a saved remote machine, in registry order. */
export function visibleProjects(
  projects: readonly ProjectRecord[],
  savedProfileIds: readonly string[]
): ProjectRecord[] {
  return projects.filter((project) => project.machine === 'local' || savedProfileIds.includes(project.machine));
}

/** Projects on `machine` (all machines when null) whose name or folder contains `query`, ignoring case. */
export function filterProjects(
  projects: readonly ProjectRecord[],
  machine: string | null,
  query: string
): ProjectRecord[] {
  const needle = query.trim().toLowerCase();
  return projects.filter((project) => (machine === null || project.machine === machine)
    && (!needle || project.title.toLowerCase().includes(needle) || project.rootPath.toLowerCase().includes(needle)));
}

/** How many projects each machine has, keyed by machine id. */
export function projectCountsByMachine(projects: readonly ProjectRecord[]): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const project of projects) counts[project.machine] = (counts[project.machine] ?? 0) + 1;
  return counts;
}

/** A folder under a home directory, shown from `~`. */
export function shortProjectPath(path: string): string {
  return path.replace(/^\/(?:home|Users)\/[^/]+(?=\/|$)/, '~');
}

type DraftOwnedSession = { ownedId: string; projectId: string | null; lastActivity: string | null };

/**
 * The project a new draft starts in: the active session's, else the most
 * recently active session's, else none. Only projects in `projects` count.
 */
export function defaultDraftProjectId(
  projects: readonly ProjectRecord[],
  owned: readonly DraftOwnedSession[],
  activeOwnedId: string | null
): string | null {
  const visible = (id: string | null): id is string => Boolean(id) && projects.some((project) => project.id === id);
  const active = owned.find((session) => session.ownedId === activeOwnedId);
  if (active && visible(active.projectId)) return active.projectId;
  const recent = owned
    .filter((session) => visible(session.projectId))
    .sort((a, b) => (Date.parse(b.lastActivity ?? '') || 0) - (Date.parse(a.lastActivity ?? '') || 0));
  return recent[0]?.projectId ?? null;
}

export function projectMachineLabel(
  machine: string,
  profiles: readonly { id: string; name: string }[]
): string {
  if (machine === 'local') return 'This Mac';
  return profiles.find((profile) => profile.id === machine)?.name ?? 'Remote machine';
}

/**
 * Whether the Add project path names a folder to create: its parent is the
 * listed folder and nothing of that name is listed there.
 */
export function isNewFolderPath(
  path: string,
  listing: { path: string; directories: readonly string[] } | null
): boolean {
  const trimmed = path.trim().replace(/\/+$/, '');
  const slash = trimmed.lastIndexOf('/');
  const name = trimmed.slice(slash + 1);
  const parent = trimmed.slice(0, slash) || '/';
  return Boolean(listing) && slash >= 0 && parent === listing!.path
    && name !== '' && name !== '.' && name !== '..' && !listing!.directories.includes(name);
}
