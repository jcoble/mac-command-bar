/**
 * projectRootsStore.svelte.ts — the project folders the new-session dialog offers.
 *
 * STATE ONLY, in the pattern `sessionRailStore.svelte.ts` set:
 *
 *  1. **No backend, ever.** The only IO here is `localStorage`. Listing
 *     worktrees, picking a folder with the system dialog and asking whether a
 *     folder is really a git repository all live in `newSessionBackend.ts`, and
 *     land here as plain mutations.
 *  2. **No `$effect`.** Persistence is an explicit `persist()` call at the end
 *     of every mutator that changes something worth keeping.
 *
 * Where the list comes from — three places, merged by `mergeKnownRoots`:
 *  - the folders built into this app (`defaultProjectRoots` in `sourceData.ts`,
 *    two hard-coded paths — see the note on `builtInRoots` below);
 *  - the folders the user added, kept here under this store's own key;
 *  - the folders the sessions on the rail are already running in, handed in by
 *    whoever opens the dialog via `setSessionRoots`.
 *
 * A storage failure never breaks the picker: the in-memory list keeps working
 * and the failure is reported on `projectRoots.error` rather than swallowed.
 */
import { defaultProjectRoots } from '../../sourceData.ts';
import {
  createCustomRoot,
  mergeKnownRoots,
  normalizeRootPath,
  parseStoredCustomRoots,
  serializeCustomRoots,
  type CustomRoot,
  type KnownRoot
} from './newSessionFlow.ts';

/** Where the folders the user added are kept. This lane's own key. */
export const CUSTOM_PROJECT_ROOTS_STORAGE_KEY = 'mac-command-bar.next.new-session-custom-roots';

/** Where the folder the dialog last started in is kept, so it opens there again. */
export const LAST_PROJECT_ROOT_STORAGE_KEY = 'mac-command-bar.next.new-session-last-root';

/** What the user is told when the added folders could not be written down. */
export const STORAGE_WRITE_FAILED_MESSAGE =
  'That folder could not be saved — browser storage is full, so it will be gone after a reload';

// ── Reactive state ────────────────────────────────────────────────────────────

export const projectRoots = $state<{
  /** Folders the user added by hand. Persisted. */
  custom: CustomRoot[];
  /** Folders the sessions on the rail run in. Never persisted — they are derived. */
  sessionPaths: string[];
  /** The folder the dialog should open on, when it is still one of the known ones. */
  lastUsedPath: string | null;
  /** Whether the stored folders have been read yet, so hydrate runs once. */
  hydrated: boolean;
  /** Last thing that went wrong here, in one sentence, or `null`. */
  error: string | null;
}>({
  custom: [],
  sessionPaths: [],
  lastUsedPath: null,
  hydrated: false,
  error: null
});

/**
 * The folders this app ships with.
 *
 * They are two hard-coded paths on one machine (`sourceData.ts`), which is a
 * poor list for anyone else — which is exactly why the picker also learns from
 * the sessions on the rail and lets the user add their own. Kept as a
 * starting point, not as the answer.
 */
export const builtInRoots = defaultProjectRoots;

// ── Reading the list ──────────────────────────────────────────────────────────

/**
 * Every folder the picker offers, in the order it offers them. Reads reactive
 * state, so a component reading it inside `$derived` re-runs when a folder is
 * added or the rail's folders change.
 */
export function knownRoots(): KnownRoot[] {
  return mergeKnownRoots({
    defaults: builtInRoots,
    custom: projectRoots.custom,
    sessionPaths: projectRoots.sessionPaths
  });
}

/**
 * The folder the dialog should open on: the one last started in when it is
 * still on the list, otherwise the first folder there, otherwise nothing.
 */
export function initialRootPath(): string | null {
  const roots = knownRoots();
  if (roots.length === 0) return null;
  const last = projectRoots.lastUsedPath;
  if (last && roots.some((root) => root.path === last)) return last;
  return roots[0].path;
}

// ── Persistence ───────────────────────────────────────────────────────────────

/** Write the added folders down; `true` when it landed. */
function persist(): boolean {
  if (typeof localStorage === 'undefined') return false;
  try {
    localStorage.setItem(
      CUSTOM_PROJECT_ROOTS_STORAGE_KEY,
      serializeCustomRoots($state.snapshot(projectRoots.custom) as CustomRoot[])
    );
    return true;
  } catch {
    projectRoots.error = STORAGE_WRITE_FAILED_MESSAGE;
    return false;
  }
}

/** Remember where the last session was started. Failing this is not worth a word. */
function persistLastUsed(): void {
  if (typeof localStorage === 'undefined') return;
  try {
    if (projectRoots.lastUsedPath) {
      localStorage.setItem(LAST_PROJECT_ROOT_STORAGE_KEY, projectRoots.lastUsedPath);
    } else {
      localStorage.removeItem(LAST_PROJECT_ROOT_STORAGE_KEY);
    }
  } catch {
    // Which folder the dialog opens on is a convenience. Losing it is not an
    // error anyone needs to read about.
  }
}

/**
 * Read the added folders and the last-used one back. Safe to call repeatedly —
 * it does its work once. Tolerant: unreadable storage means an empty list, not
 * a broken dialog.
 */
export function hydrate(): void {
  if (projectRoots.hydrated) return;
  projectRoots.hydrated = true;
  if (typeof localStorage === 'undefined') return;
  try {
    projectRoots.custom = parseStoredCustomRoots(
      localStorage.getItem(CUSTOM_PROJECT_ROOTS_STORAGE_KEY)
    );
    const last = normalizeRootPath(localStorage.getItem(LAST_PROJECT_ROOT_STORAGE_KEY) ?? '');
    projectRoots.lastUsedPath = last || null;
  } catch {
    projectRoots.custom = [];
    projectRoots.lastUsedPath = null;
  }
}

// ── Mutations ─────────────────────────────────────────────────────────────────

/**
 * Add a folder the user chose. Returns the folder as it was stored, or `null`
 * when the path is not one a session can run in.
 *
 * Adding a folder that is already known is not an error and not a second row —
 * the id is the folder itself, and the merge keeps one of each.
 */
export function addCustomRoot(path: string, name?: string): CustomRoot | null {
  const root = createCustomRoot(path, name);
  if (!root) return null;
  if (projectRoots.custom.some((existing) => existing.id === root.id)) return root;
  projectRoots.custom = [...projectRoots.custom, root];
  persist();
  return root;
}

/**
 * Forget a folder the user added. Only ever removes an added folder: the
 * built-in ones and the ones derived from running sessions are not this store's
 * to remove, and a session's folder would come straight back anyway.
 */
export function removeCustomRoot(id: string): void {
  const next = projectRoots.custom.filter((root) => root.id !== id);
  if (next.length === projectRoots.custom.length) return;
  projectRoots.custom = next;
  persist();
}

/**
 * Tell the store which folders the sessions on the rail are running in. Called
 * by whoever opens the dialog; the store never reaches into the rail itself.
 */
export function setSessionRoots(paths: string[]): void {
  projectRoots.sessionPaths = paths
    .map((path) => normalizeRootPath(path))
    .filter((path) => path.length > 0);
}

/** Remember the folder a session was just started in. */
export function rememberLastUsed(path: string): void {
  const normalized = normalizeRootPath(path);
  projectRoots.lastUsedPath = normalized || null;
  persistLastUsed();
}

/** Clear the one-line problem message. */
export function clearError(): void {
  projectRoots.error = null;
}

/** Say what went wrong, in a sentence a user can read. */
export function setError(message: string | null): void {
  projectRoots.error = message;
}
