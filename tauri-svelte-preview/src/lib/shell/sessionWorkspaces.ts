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
  /**
   * The file the Diff tab was showing, as a path relative to the repository, or
   * null when it was showing nothing.
   */
  diffPath: string | null;
  /**
   * The project folder `diffPath` is inside, or null when there is no diff.
   *
   * Stored next to the path because a path on its own cannot be checked: the
   * Diff tab is one tab for the whole shell, and "src/lib/index.ts" names a real
   * file in most projects. Keeping the folder it came from is what lets
   * {@link diffPathFor} tell "this is your file" from "this is the last
   * project's file that happens to have the same name".
   */
  diffRoot: string | null;
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
  /** What the Diff tab is showing, if the caller passes it. Optional so the
   * page can start recording it separately from the rest of this record; a
   * caller that says nothing gets "no diff", which is what an older stored
   * record means too. */
  diffPath?: string | null;
  /** The project folder that diff came from. See `diffRoot` on the record. */
  diffRoot?: string | null;
}): SessionWorkspaceSnapshot {
  const activePath = input.activePath ?? null;
  // A path with no folder cannot be checked against the session being restored,
  // and the safe reading of an unknown owner is "not this session's" — so half
  // an answer is stored as no answer rather than as a diff we would then show
  // to the wrong project.
  const diffPath = pathOf(input.diffPath);
  const diffRoot = pathOf(input.diffRoot);
  const bothKnown = diffPath !== null && diffRoot !== null;
  return {
    openPaths: cappedPaths(
      input.openFiles.map((file) => file.path),
      activePath
    ),
    activePath,
    expandedFolderIds: [...input.expandedFolderIds],
    selectedPath: input.selectedPath ?? null,
    scrollTop: Math.max(0, input.scrollTop),
    diffPath: bothKnown ? diffPath : null,
    diffRoot: bothKnown ? diffRoot : null
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
  // Records written before the Diff tab was remembered have neither field, and
  // they read back as "this session was not looking at a diff" — which is the
  // right answer for them and the safe one in general.
  const diffPath = pathOf(entry.diffPath);
  const diffRoot = pathOf(entry.diffRoot);
  const bothKnown = diffPath !== null && diffRoot !== null;
  return {
    openPaths: cappedPaths(stringsOf(entry.openPaths), activePath),
    activePath,
    expandedFolderIds: stringsOf(entry.expandedFolderIds),
    selectedPath: pathOf(entry.selectedPath),
    scrollTop,
    diffPath: bothKnown ? diffPath : null,
    diffRoot: bothKnown ? diffRoot : null
  };
}

/** Trailing slashes make two spellings of one folder look different. */
function sameFolder(left: string, right: string): boolean {
  return left.replace(/\/+$/, '') === right.replace(/\/+$/, '');
}

/**
 * The file the Diff tab should show for a session whose project folder is
 * `root`, or null when it should show nothing.
 *
 * This is the answer to the bug where switching session left the Diff tab
 * showing a file from the project you had just left. There are three ways to get
 * null, and all of them mean "clear the tab": the session has no stored record
 * at all, it was not looking at a diff, or the diff it was looking at belongs to
 * a different project. Only a diff from this session's own project comes back.
 *
 * A session with no folder (`root` empty) can own no diff, so it always clears.
 */
export function diffPathFor(
  snapshot: SessionWorkspaceSnapshot | null | undefined,
  root: string | null | undefined
): string | null {
  const folder = (root ?? '').trim();
  if (!snapshot || !folder || !snapshot.diffPath || !snapshot.diffRoot) return null;
  return sameFolder(snapshot.diffRoot, folder) ? snapshot.diffPath : null;
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
