/**
 * projectRootsStore.svelte.ts — the project folders the new-session thread pane offers.
 *
 * STATE ONLY, in the pattern `sessionRailStore.svelte.ts` set:
 *
 *  1. **SQLite owns durable state.** This store reads and writes only the two
 *     global Assembly settings it owns. Listing worktrees, picking a folder
 *     with the system dialog and asking whether a folder is really a git
 *     repository all live in `newSessionBackend.ts`, and land here as plain
 *     mutations.
 *  2. **No `$effect`.** Persistence is an explicit write at the end of every
 *     mutator that changes something worth keeping.
 *
 * Where the list comes from — three places, merged by `mergeKnownRoots`:
 *  - the folders built into this app (`defaultProjectRoots` in `sourceData.ts`,
 *    two hard-coded paths — see the note on `builtInRoots` below);
 *  - the folders the user added, kept here under this store's own key;
 *  - the folders the sessions on the rail are already running in, handed in by
 *    whoever opens the pane via `setSessionRoots`.
 *
 * A storage failure never breaks the picker: the in-memory list keeps working
 * and the failure is reported on `projectRoots.error` rather than swallowed.
 */
import { defaultProjectRoots } from '../../sourceData.ts';
import {
  readAssemblySettingFromTauri,
  writeAssemblySettingFromTauri
} from '../../tauriSource.ts';
import {
  createCustomRoot,
  mergeKnownRoots,
  normalizeRootPath,
  parseStoredCustomRoots,
  type CustomRoot,
  type KnownRoot
} from './newSessionFlow.ts';

/** The global SQLite setting that owns folders added through New Session. */
export const CUSTOM_PROJECT_ROOTS_SETTING_KEY = 'new-session.custom-project-roots';

/** The global SQLite setting that owns the folder New Session last used. */
export const LAST_PROJECT_ROOT_SETTING_KEY = 'new-session.last-project-root';

/** What the user is told when the added folders could not be written down. */
export const PERSISTENCE_WRITE_FAILED_MESSAGE =
  'That folder could not be saved, so it will be gone after a reload';

// ── Reactive state ────────────────────────────────────────────────────────────

export const projectRoots = $state<{
  /** Folders the user added by hand. Persisted. */
  custom: CustomRoot[];
  /** Folders the sessions on the rail run in. Never persisted — they are derived. */
  sessionPaths: string[];
  /** The folder the pane should open on, when it is still one of the known ones. */
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
 * The folder the pane should open on: the one last started in when it is
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

let customVersion = 0;
let lastUsedVersion = 0;
let hydrationPromise: Promise<void> | null = null;

/** Write the added folders to their SQLite setting. */
function persistCustom(): void {
  const snapshot = $state.snapshot(projectRoots.custom) as CustomRoot[];
  void persistCustomRoots(snapshot);
}

/** Remember where the last session was started. Failing this is not worth a word. */
function persistLastUsed(): void {
  void persistLastUsedRoot(projectRoots.lastUsedPath);
}

async function persistCustomRoots(snapshot: CustomRoot[]): Promise<void> {
  try {
    await writeAssemblySettingFromTauri(CUSTOM_PROJECT_ROOTS_SETTING_KEY, snapshot);
  } catch {
    projectRoots.error = PERSISTENCE_WRITE_FAILED_MESSAGE;
  }
}

async function persistLastUsedRoot(path: string | null): Promise<void> {
  try {
    await writeAssemblySettingFromTauri(LAST_PROJECT_ROOT_SETTING_KEY, path);
  } catch {
    // Best effort preference.
  }
}

/**
 * Read the added folders and the last-used one back. Safe to call repeatedly —
 * it does its work once. Tolerant: an unreadable setting means an empty value,
 * not a broken dialog. A user mutation made while the read is in flight wins.
 */
export async function hydrate(): Promise<void> {
  if (projectRoots.hydrated) return;
  if (hydrationPromise) {
    await hydrationPromise;
    return;
  }
  const customVersionAtStart = customVersion;
  const lastUsedVersionAtStart = lastUsedVersion;

  hydrationPromise = hydrateProjectRoots(customVersionAtStart, lastUsedVersionAtStart);

  await hydrationPromise;
}

async function hydrateProjectRoots(
  customVersionAtStart: number,
  lastUsedVersionAtStart: number
): Promise<void> {
  try {
    const [storedCustom, storedLastUsed] = await Promise.all([
      readAssemblySettingFromTauri(CUSTOM_PROJECT_ROOTS_SETTING_KEY),
      readAssemblySettingFromTauri(LAST_PROJECT_ROOT_SETTING_KEY)
    ]);
      if (customVersion === customVersionAtStart) {
        projectRoots.custom = parseStoredCustomRoots(
          storedCustom === null ? null : JSON.stringify(storedCustom)
        );
      }
      if (lastUsedVersion === lastUsedVersionAtStart) {
        const last = normalizeRootPath(typeof storedLastUsed === 'string' ? storedLastUsed : '');
        projectRoots.lastUsedPath = last || null;
      }
  } catch {
    if (customVersion === customVersionAtStart) projectRoots.custom = [];
    if (lastUsedVersion === lastUsedVersionAtStart) projectRoots.lastUsedPath = null;
  } finally {
    projectRoots.hydrated = true;
    hydrationPromise = null;
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
  customVersion += 1;
  projectRoots.custom = [...projectRoots.custom, root];
  persistCustom();
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
  customVersion += 1;
  projectRoots.custom = next;
  persistCustom();
}

/**
 * Tell the store which folders the sessions on the rail are running in. Called
 * by whoever opens the pane; the store never reaches into the rail itself.
 */
export function setSessionRoots(paths: string[]): void {
  projectRoots.sessionPaths = paths
    .map((path) => normalizeRootPath(path))
    .filter((path) => path.length > 0);
}

/** Remember the folder a session was just started in. */
export function rememberLastUsed(path: string): void {
  const normalized = normalizeRootPath(path);
  lastUsedVersion += 1;
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
