/**
 * gitService.ts — every backend call the /next source-control panel makes.
 *
 * Constitution rules this file exists to keep:
 *
 * - **No `$effect` anywhere.** Nothing here reacts to state. The panel's UI
 *   events and the integrator's first-activation hook call these functions
 *   imperatively; that is the only way a git command ever runs.
 * - **Nothing loads at import.** Building the service allocates an object and
 *   touches no backend. The first call happens in `activate(root)`, which the
 *   integrator invokes when the panel is first shown.
 * - **Every backend call is counted.** `countInvoke('<command name>')` runs
 *   immediately before each one, so the dev invoke counter tells the truth about
 *   what this panel costs.
 * - **No stand-in data.** The `…FromTauri` wrappers return `null` when the app
 *   is not running under Tauri. That flips the panel to a plain "desktop app
 *   only" state; it never fabricates a repository.
 *
 * Superseded-request handling is the shape the old shell used
 * (`src/routes/+page.svelte` around :3776, `loadProjectGitStatus` +
 * `isCurrentProjectGitStatusRequest`): a monotonic request id per lane of work,
 * re-checked after every await, plus a check that the repository has not changed
 * underneath. A superseded read drops its result silently and does NOT clear the
 * loading flag the newer read owns.
 */

import { countInvoke } from '../devInvokeCounter.svelte.ts';
import {
  commitGitRepositoryFromTauri,
  fetchGitRepositoryFromTauri,
  pullGitRepositoryFromTauri,
  pushGitRepositoryFromTauri,
  readGitCommitHistoryFromTauri,
  readProjectGitStatusFromTauri,
  readSourceGitDiffFromTauri,
  stageGitPathsFromTauri,
  unstageGitPathsFromTauri,
  type GitActionResult,
  type GitCommitHistoryEntry,
  type ProjectGitFileStatus,
  type ProjectGitStatus,
  type SourceGitDiff
} from '../../tauriSource.ts';
import {
  clearSelectedGitFile,
  gitPanel,
  isGitFileDeleted,
  resetGitPanelState,
  type GitActionKind,
  type GitPanelState
} from './gitPanelStore.svelte.ts';

/** How many commits the history list asks for. The backend caps the limit. */
export const COMMIT_HISTORY_LIMIT = 24;

/** Shown whenever a wrapper returns `null` — i.e. we are not in the desktop app. */
export const DESKTOP_ONLY_MESSAGE =
  'Source control runs in the desktop app only. Nothing is loaded here.';

/**
 * The nine git commands this panel can issue. Named as an interface so the node
 * test can drive the service with a stub and prove the superseded-request
 * handling, exactly as `terminalService.ts` does with its terminal backend.
 */
export interface GitBackend {
  readStatus(root: string): Promise<ProjectGitStatus | null>;
  readDiff(root: string, absolutePath: string): Promise<SourceGitDiff | null>;
  readHistory(root: string, limit: number): Promise<GitCommitHistoryEntry[] | null>;
  stage(root: string, paths: string[]): Promise<GitActionResult | null>;
  unstage(root: string, paths: string[]): Promise<GitActionResult | null>;
  commit(root: string, message: string): Promise<GitActionResult | null>;
  fetch(root: string): Promise<GitActionResult | null>;
  pull(root: string): Promise<GitActionResult | null>;
  push(root: string): Promise<GitActionResult | null>;
}

/**
 * The real backend: the existing `tauriSource.ts` wrappers, each preceded by the
 * invoke count for the Tauri command it runs.
 */
export function tauriGitBackend(count: (command: string) => void = countInvoke): GitBackend {
  return {
    readStatus(root) {
      count('project_git_status');
      return readProjectGitStatusFromTauri(root);
    },
    readDiff(root, absolutePath) {
      count('read_source_git_diff');
      return readSourceGitDiffFromTauri(root, absolutePath);
    },
    readHistory(root, limit) {
      count('read_git_commit_history');
      return readGitCommitHistoryFromTauri(root, limit);
    },
    stage(root, paths) {
      count('stage_git_paths');
      return stageGitPathsFromTauri(root, paths);
    },
    unstage(root, paths) {
      count('unstage_git_paths');
      return unstageGitPathsFromTauri(root, paths);
    },
    commit(root, message) {
      count('commit_git_repository');
      return commitGitRepositoryFromTauri(root, message);
    },
    fetch(root) {
      count('fetch_git_repository');
      return fetchGitRepositoryFromTauri(root);
    },
    pull(root) {
      count('pull_git_repository');
      return pullGitRepositoryFromTauri(root);
    },
    push(root) {
      count('push_git_repository');
      return pushGitRepositoryFromTauri(root);
    }
  };
}

export interface RequestGuard {
  /** Claim the newest request id; every older one is now superseded. */
  next(): number;
  /** Is `id` still the newest claim? */
  isCurrent(id: number): boolean;
  /** Supersede everything in flight without claiming a new id. */
  invalidate(): void;
}

/** A monotonic request id, so a slow answer can never overwrite a fast newer one. */
export function createRequestGuard(): RequestGuard {
  let latest = 0;
  return {
    next: () => (latest += 1),
    isCurrent: (id: number) => id === latest,
    invalidate: () => {
      latest += 1;
    }
  };
}

/**
 * Join a repository root and a repository-relative path. The diff command wants
 * an absolute path on disk; git status hands us a relative one.
 */
export function absolutePathWithin(root: string, relativePath: string): string {
  const base = root.replace(/\/+$/, '');
  const tail = relativePath.replace(/^\/+/, '');
  return tail === '' ? base : `${base}/${tail}`;
}

function describeError(error: unknown, fallback: string): string {
  const message = error instanceof Error ? error.message : String(error ?? '');
  return message.trim() === '' ? fallback : message;
}

export interface GitService {
  /** The state this service writes. The panel reads the same object. */
  readonly state: GitPanelState;
  /**
   * Point the panel at a repository and do its first read. Idempotent: calling
   * it again with the same root does nothing, so the integrator can call it on
   * every activation. A different root wipes the panel and reloads.
   */
  activate(root: string | null): void;
  /** Re-read status and commit history for the current repository. */
  refresh(): Promise<void>;
  refreshStatus(): Promise<void>;
  refreshHistory(): Promise<void>;
  /** Show this file's diff. */
  selectFile(file: ProjectGitFileStatus): Promise<void>;
  /** Stop showing a diff. */
  clearSelection(): void;
  stagePaths(paths: string[]): Promise<void>;
  unstagePaths(paths: string[]): Promise<void>;
  /** Commit the staged changes using `state.commitMessage` unless one is given. */
  commit(message?: string): Promise<void>;
  runRemoteAction(action: 'fetch' | 'pull' | 'push'): Promise<void>;
}

export interface GitServiceOptions {
  backend?: GitBackend;
  state?: GitPanelState;
}

export function createGitService(options: GitServiceOptions = {}): GitService {
  const backend = options.backend ?? tauriGitBackend();
  const state = options.state ?? gitPanel;
  const statusGuard = createRequestGuard();
  const historyGuard = createRequestGuard();
  const diffGuard = createRequestGuard();

  /** The repository is unchanged AND this request is still the newest one. */
  function stillCurrent(guard: RequestGuard, id: number, root: string): boolean {
    return guard.isCurrent(id) && state.root === root;
  }

  /** A `null` result means "not the desktop app": say so and load nothing. */
  function markDesktopOnly(): void {
    state.desktopOnly = true;
    state.status = null;
    state.history = [];
  }

  async function refreshStatus(): Promise<void> {
    const root = state.root;
    if (!root) return;
    const id = statusGuard.next();
    state.statusLoading = true;
    state.statusError = '';

    try {
      const status = await backend.readStatus(root);
      if (!stillCurrent(statusGuard, id, root)) return;
      if (!status) {
        markDesktopOnly();
        return;
      }
      state.desktopOnly = false;
      state.status = status;
    } catch (error) {
      if (!stillCurrent(statusGuard, id, root)) return;
      state.status = null;
      state.statusError = describeError(error, 'Could not read the repository status.');
    } finally {
      if (stillCurrent(statusGuard, id, root)) state.statusLoading = false;
    }
  }

  async function refreshHistory(): Promise<void> {
    const root = state.root;
    if (!root) return;
    const id = historyGuard.next();
    state.historyLoading = true;
    state.historyError = '';

    try {
      const history = await backend.readHistory(root, COMMIT_HISTORY_LIMIT);
      if (!stillCurrent(historyGuard, id, root)) return;
      if (!history) {
        markDesktopOnly();
        return;
      }
      state.desktopOnly = false;
      state.history = history;
    } catch (error) {
      if (!stillCurrent(historyGuard, id, root)) return;
      state.history = [];
      state.historyError = describeError(error, 'Could not read the commit history.');
    } finally {
      if (stillCurrent(historyGuard, id, root)) state.historyLoading = false;
    }
  }

  async function refresh(): Promise<void> {
    await Promise.all([refreshStatus(), refreshHistory()]);
  }

  async function selectFile(file: ProjectGitFileStatus): Promise<void> {
    const root = state.root;
    if (!root) return;

    state.selectedPath = file.relativePath;
    state.selectedDiff = null;
    state.diffError = '';

    if (isGitFileDeleted(file)) {
      // The diff command reads the file off disk first, so a deleted file always
      // fails there. Say what happened instead of showing its error.
      diffGuard.invalidate();
      state.diffLoading = false;
      state.diffError = 'This file was deleted, so there is nothing on disk to compare.';
      return;
    }

    const id = diffGuard.next();
    state.diffLoading = true;

    try {
      const diff = await backend.readDiff(root, absolutePathWithin(root, file.relativePath));
      if (!stillCurrent(diffGuard, id, root)) return;
      if (!diff) {
        state.desktopOnly = true;
        state.diffError = DESKTOP_ONLY_MESSAGE;
        return;
      }
      state.selectedDiff = diff;
    } catch (error) {
      if (!stillCurrent(diffGuard, id, root)) return;
      state.diffError = describeError(error, 'Could not read the changes for this file.');
    } finally {
      if (stillCurrent(diffGuard, id, root)) state.diffLoading = false;
    }
  }

  function clearSelection(): void {
    diffGuard.invalidate();
    clearSelectedGitFile(state);
  }

  /** Re-read the diff on screen after an action changed the working tree. */
  function refreshSelectedDiff(): void {
    const selected = state.selectedPath;
    if (selected === '') return;
    const file = (state.status?.files ?? []).find((entry) => entry.relativePath === selected);
    if (!file) {
      clearSelection();
      return;
    }
    void selectFile(file);
  }

  /**
   * The one place a working-tree action runs. Every action returns the fresh
   * status alongside its message, so the panel never needs a follow-up status
   * read — it adopts the result and supersedes any status read still in flight.
   */
  async function runAction(
    kind: GitActionKind,
    run: (root: string) => Promise<GitActionResult | null>,
    options: { reloadHistory: boolean }
  ): Promise<void> {
    const root = state.root;
    if (!root || state.actionBusy !== '') return;

    state.actionBusy = kind;
    state.actionStatus = '';
    state.actionError = '';

    try {
      const result = await run(root);
      if (state.root !== root) return;
      if (!result) {
        state.desktopOnly = true;
        state.actionStatus = DESKTOP_ONLY_MESSAGE;
        return;
      }
      // The fresh status arrived with the result: take it, and make sure a
      // slower status read started earlier cannot overwrite it.
      statusGuard.invalidate();
      state.statusLoading = false;
      state.statusError = '';
      state.desktopOnly = false;
      state.status = result.status;
      state.actionStatus = result.message;
      refreshSelectedDiff();
      if (options.reloadHistory) void refreshHistory();
    } catch (error) {
      if (state.root !== root) return;
      state.actionError = describeError(error, 'The git command did not finish.');
    } finally {
      if (state.actionBusy === kind) state.actionBusy = '';
    }
  }

  function cleanPaths(paths: string[]): string[] {
    return paths.map((path) => path.trim()).filter((path) => path !== '');
  }

  return {
    state,

    activate(root: string | null): void {
      if (root === state.root && state.activated) return;
      statusGuard.invalidate();
      historyGuard.invalidate();
      diffGuard.invalidate();
      resetGitPanelState(state, root);
      if (!root) return;
      state.activated = true;
      void refresh();
    },

    refresh,
    refreshStatus,
    refreshHistory,
    selectFile,
    clearSelection,

    async stagePaths(paths: string[]): Promise<void> {
      const wanted = cleanPaths(paths);
      if (wanted.length === 0) return;
      await runAction('stage', (root) => backend.stage(root, wanted), { reloadHistory: false });
    },

    async unstagePaths(paths: string[]): Promise<void> {
      const wanted = cleanPaths(paths);
      if (wanted.length === 0) return;
      await runAction('unstage', (root) => backend.unstage(root, wanted), { reloadHistory: false });
    },

    async commit(message?: string): Promise<void> {
      const text = (message ?? state.commitMessage).trim();
      if (text === '') {
        state.actionError = 'Type a commit message first.';
        return;
      }
      const before = state.status;
      await runAction('commit', (root) => backend.commit(root, text), { reloadHistory: true });
      // Only clear the box on a commit that actually happened.
      if (state.actionError === '' && state.status !== before) state.commitMessage = '';
    },

    async runRemoteAction(action: 'fetch' | 'pull' | 'push'): Promise<void> {
      const run =
        action === 'fetch'
          ? (root: string) => backend.fetch(root)
          : action === 'pull'
            ? (root: string) => backend.pull(root)
            : (root: string) => backend.push(root);
      await runAction(action, run, { reloadHistory: action !== 'fetch' });
    }
  };
}

/**
 * The service the panel uses. A lane-scoped singleton bound to `gitPanel`,
 * because the panel component takes no props (the shell's panel contract).
 * Constructing it performs no IO.
 */
export const gitService = createGitService();
