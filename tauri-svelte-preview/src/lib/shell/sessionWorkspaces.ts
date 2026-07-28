/**
 * sessionWorkspaces.ts — what each session had open, kept per session.
 *
 * PURE: no store, no DOM, no backend call. It only turns "this is what the
 * editor and the file tree look like right now" into a small record, and reads
 * those records back out of a storage the caller hands in.
 *
 * The problem it exists for: the editor and the file tree are one of each for
 * the whole shell, so two sessions working in the same repository were sharing
 * one set of tabs. Switching between them lost your place both ways. Giving
 * every session its own record means switching can put back what that session
 * had, and the two stop overwriting each other.
 *
 * What is NOT here: any decision about when to save or restore. The page owns
 * that (see `+page.svelte`), so nothing in this file can fire on its own.
 */
import { loadLayout, saveLayout, type LayoutStorage } from './layout/layoutStorage.ts';

/** What one session had open, small enough to store for every session at once. */
export interface SessionWorkspaceSnapshot {
  /** Editor tabs in strip order, newest at the end. */
  openPaths: string[];
  /** The tab that was showing. */
  activePath: string | null;
  /** Ids of the folders open in the file tree. */
  expandedFolderIds: string[];
  /** The highlighted file in the tree. */
  selectedPath: string | null;
  /** Scroll offset of the tree, in pixels. */
  scrollTop: number;
}

export const SESSION_WORKSPACES_STORAGE_KEY = 'mac-command-bar.next.session-workspaces';

/**
 * How many tabs a session's record keeps. Twelve is more than anyone has open
 * at once and far less than the hundreds a long session can accumulate — the
 * point of the limit is that every session on the rail is stored together, so
 * one busy session must not be able to fill the browser's storage on its own.
 */
export const OPEN_PATHS_CAP = 12;

/** The strip, trimmed to the cap: the most recent tabs, which are the ones at
 * the end. The tab that was showing is always kept, even when it is older than
 * all of them — coming back to a session and not finding the file you left on
 * screen is the one thing nobody would forgive. It keeps its place in the
 * strip; the oldest of the recent tabs makes room for it. */
function cappedPaths(paths: string[], activePath: string | null): string[] {
  const unique: string[] = [];
  for (const path of paths) {
    if (typeof path === 'string' && path && !unique.includes(path)) unique.push(path);
  }
  if (unique.length <= OPEN_PATHS_CAP) return unique;

  const recent = unique.slice(unique.length - OPEN_PATHS_CAP);
  if (!activePath || recent.includes(activePath) || !unique.includes(activePath)) return recent;
  return [activePath, ...recent.slice(1)];
}

/** Turn what is on screen into a record. Nothing is read from anywhere: the
 * caller passes the editor's and the tree's own state straight in. */
export function captureWorkspace(input: {
  openFiles: { path: string }[];
  activePath: string | null;
  expandedFolderIds: Set<string>;
  selectedPath: string | null;
  scrollTop: number;
}): SessionWorkspaceSnapshot {
  const activePath = input.activePath ?? null;
  return {
    openPaths: cappedPaths(
      input.openFiles.map((file) => file.path),
      activePath
    ),
    activePath,
    expandedFolderIds: [...input.expandedFolderIds],
    selectedPath: input.selectedPath ?? null,
    scrollTop: Math.max(0, input.scrollTop)
  };
}

function stringsOf(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value.filter((entry): entry is string => typeof entry === 'string' && entry.length > 0);
}

function pathOf(value: unknown): string | null {
  return typeof value === 'string' && value.length > 0 ? value : null;
}

/** One stored entry, or null if it is not a record at all. Fields of the wrong
 * type become their empty version rather than sinking the whole entry: a
 * half-corrupt record still restores most of a session. */
function snapshotOf(value: unknown): SessionWorkspaceSnapshot | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;

  const entry = value as Record<string, unknown>;
  const activePath = pathOf(entry.activePath);
  const scrollTop = typeof entry.scrollTop === 'number' && entry.scrollTop > 0 ? entry.scrollTop : 0;
  return {
    openPaths: cappedPaths(stringsOf(entry.openPaths), activePath),
    activePath,
    expandedFolderIds: stringsOf(entry.expandedFolderIds),
    selectedPath: pathOf(entry.selectedPath),
    scrollTop
  };
}

/** Every session's record, by owned id. Anything unreadable — no value, broken
 * JSON, a value that is not a map — comes back as "no session has one yet". */
export function readWorkspaces(storage: LayoutStorage): Record<string, SessionWorkspaceSnapshot> {
  const stored = loadLayout<unknown>(storage, SESSION_WORKSPACES_STORAGE_KEY);
  if (!stored || typeof stored !== 'object' || Array.isArray(stored)) return {};

  const all: Record<string, SessionWorkspaceSnapshot> = {};
  for (const [ownedId, value] of Object.entries(stored)) {
    const snapshot = snapshotOf(value);
    if (snapshot) all[ownedId] = snapshot;
  }
  return all;
}

/** False means the write was refused (a full storage). Nothing is retried: the
 * cost is one session coming back to an empty editor after a reload. */
export function writeWorkspaces(
  storage: LayoutStorage,
  all: Record<string, SessionWorkspaceSnapshot>
): boolean {
  return saveLayout(storage, SESSION_WORKSPACES_STORAGE_KEY, all);
}

/** Drop the records of sessions the rail no longer has, so a removed session
 * takes its workspace with it and the store cannot grow forever. */
export function pruneWorkspaces(
  all: Record<string, SessionWorkspaceSnapshot>,
  keepOwnedIds: string[]
): Record<string, SessionWorkspaceSnapshot> {
  const keep = new Set(keepOwnedIds);
  const kept: Record<string, SessionWorkspaceSnapshot> = {};
  for (const [ownedId, snapshot] of Object.entries(all)) {
    if (keep.has(ownedId)) kept[ownedId] = snapshot;
  }
  return kept;
}
