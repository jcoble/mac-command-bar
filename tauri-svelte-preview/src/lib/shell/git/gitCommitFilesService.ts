/**
 * gitCommitFilesService.ts — the git calls behind an open commit in the graph.
 *
 * The same rules `gitService.ts` is built on, for the same reasons:
 *
 *  - **Nothing loads at import, and nothing reacts.** No `$effect`. A commit's
 *    file list is read when somebody opens that commit, and never otherwise.
 *  - **Every question goes to the top of the repository.** `git show <sha> --
 *    <path>` matches the path against the folder git is run in, so asking from
 *    a subfolder matches nothing and reports success — a blank answer that
 *    reads as "this file did not change". The panel is often pointed at a
 *    session's working folder, which is frequently NOT the top, so the top is
 *    worked out once per repository and every call uses it.
 *  - **A slow answer never overwrites a newer one.** Same monotonic request id
 *    as the rest of the panel.
 *  - **No stand-in data.** When neither the desktop app nor the dev server can
 *    answer, the row says so in one sentence.
 *
 * The diff itself is shown by the existing `GitDiffView`, which reads
 * `gitPanel`. So picking a file inside a commit writes into that same state —
 * after letting go of whatever working-copy diff was on screen, so the two
 * cannot race each other into the view.
 */

import {
  readCommitFileDiff,
  readCommitFiles,
  resolveRepositoryTop,
  type GitCommitFileChange
} from './gitBackendExtra.ts';
import {
  COMMIT_FILES_DESKTOP_ONLY_MESSAGE,
  UNREADABLE_PATH_MESSAGE,
  commitFilesEntry,
  createGitCommitFilesEntry,
  gitCommitFiles,
  isCommitExpanded,
  isUnreadableGitPath,
  resetGitCommitFilesState,
  type GitCommitFilesState
} from './gitCommitFilesStore.svelte.ts';
import { gitPanel } from './gitPanelStore.svelte.ts';
import { gitService } from './gitService.ts';
import type { SourceGitDiff } from '../../tauriSource.ts';

/** The part of the panel state a commit's diff is written into. */
export interface CommitDiffTarget {
  root: string | null;
  selectedPath: string;
  selectedDiff: SourceGitDiff | null;
  diffLoading: boolean;
  diffError: string;
}

export interface GitCommitFilesServiceOptions {
  state?: GitCommitFilesState;
  panel?: CommitDiffTarget;
  /** The top of the repository a folder belongs to, or null when unknown. */
  resolveTop?(root: string): Promise<string | null>;
  readFiles?(root: string, sha: string): Promise<GitCommitFileChange[] | null>;
  readDiff?(
    root: string,
    sha: string,
    relativePath: string
  ): Promise<SourceGitDiff | null>;
  /** Let go of the working-copy diff on screen before showing a commit's one. */
  clearPanelSelection?(): void;
}

export interface GitCommitFilesService {
  readonly state: GitCommitFilesState;
  /** Point at a repository. Calling it again with the same folder does nothing. */
  activate(root: string | null): void;
  /** Release expanded commit rows and their loaded file payloads. */
  release(): void;
  /** Open or close one commit, reading its file list the first time it opens. */
  toggleCommit(sha: string, isMerge: boolean): Promise<void>;
  /** Read one commit's file list, whether or not the row is open. */
  loadCommitFiles(sha: string, isMerge: boolean): Promise<void>;
  /** Show what one file of one commit changed. */
  selectCommitFile(sha: string, file: GitCommitFileChange): Promise<void>;
  /** Stop showing a commit's file. */
  clearSelection(): void;
}

function describeError(error: unknown, fallback: string): string {
  const message = error instanceof Error ? error.message : String(error ?? '');
  return message.trim() === '' ? fallback : message;
}

export function createGitCommitFilesService(
  options: GitCommitFilesServiceOptions = {}
): GitCommitFilesService {
  const state = options.state ?? gitCommitFiles;
  const panel = options.panel ?? gitPanel;
  const resolveTop = options.resolveTop ?? resolveRepositoryTop;
  const readFiles = options.readFiles ?? readCommitFiles;
  const readDiff = options.readDiff ?? readCommitFileDiff;
  const clearPanelSelection = options.clearPanelSelection ?? (() => gitService.clearSelection());

  /** The one in-flight look-up of the repository top, shared by every caller. */
  let topRequest: Promise<string> | null = null;
  /** Newest file-list request per commit, so a slow one cannot win. */
  const fileRequests = new Map<string, number>();
  let diffRequest = 0;

  function currentRoot(): string | null {
    return state.root ?? panel.root;
  }

  async function repositoryTopFor(root: string): Promise<string> {
    if (state.repositoryTop) return state.repositoryTop;
    if (!topRequest) {
      topRequest = (async () => {
        let resolved: string | null = null;
        try {
          resolved = await resolveTop(root);
        } catch {
          // Not knowing the top is not a failure worth showing: fall back to
          // the folder we were given, which is the top in the usual case.
          resolved = null;
        }
        const top = (resolved ?? '').trim() === '' ? root : (resolved as string).trim();
        if (state.root === root) state.repositoryTop = top;
        return top;
      })();
    }
    return topRequest;
  }

  function writeEntry(
    sha: string,
    changes: Partial<ReturnType<typeof createGitCommitFilesEntry>>
  ): void {
    state.byCommit[sha] = { ...commitFilesEntry(state, sha), ...changes };
  }

  async function loadCommitFiles(sha: string): Promise<void> {
    const root = currentRoot();
    if (!root || sha.trim() === '') return;

    const id = (fileRequests.get(sha) ?? 0) + 1;
    fileRequests.set(sha, id);
    writeEntry(sha, { loading: true, error: '' });

    const stillCurrent = () => fileRequests.get(sha) === id && state.root === root;

    try {
      const top = await repositoryTopFor(root);
      const files = await readFiles(top, sha);
      if (!stillCurrent()) return;
      if (!files) {
        writeEntry(sha, { files: [], loaded: false, error: COMMIT_FILES_DESKTOP_ONLY_MESSAGE });
        return;
      }
      writeEntry(sha, { files, loaded: true, error: '' });
    } catch (error) {
      if (!stillCurrent()) return;
      writeEntry(sha, {
        files: [],
        loaded: false,
        error: describeError(error, 'Could not read what this commit changed.')
      });
    } finally {
      if (stillCurrent()) writeEntry(sha, { loading: false });
    }
  }

  async function selectCommitFile(sha: string, file: GitCommitFileChange): Promise<void> {
    const root = currentRoot();
    if (!root) return;

    // Whatever the working-copy side was showing is let go of first, so its own
    // in-flight read cannot land on top of this one.
    clearPanelSelection();
    state.selectedCommitSha = sha;
    state.selectedRelativePath = file.relativePath;
    panel.selectedPath = file.relativePath;
    panel.selectedDiff = null;
    panel.diffError = '';

    if (isUnreadableGitPath(file.relativePath)) {
      diffRequest += 1;
      panel.diffLoading = false;
      panel.diffError = UNREADABLE_PATH_MESSAGE;
      return;
    }

    diffRequest += 1;
    const id = diffRequest;
    panel.diffLoading = true;
    const stillCurrent = () => diffRequest === id && state.root === root;

    try {
      const top = await repositoryTopFor(root);
      const diff = await readDiff(top, sha, file.relativePath);
      if (!stillCurrent()) return;
      if (!diff) {
        panel.diffError = COMMIT_FILES_DESKTOP_ONLY_MESSAGE;
        return;
      }
      panel.selectedDiff = diff;
    } catch (error) {
      if (!stillCurrent()) return;
      panel.diffError = describeError(error, 'Could not read the changes for this file.');
    } finally {
      if (stillCurrent()) panel.diffLoading = false;
    }
  }

  return {
    state,

    activate(root: string | null): void {
      if (root === state.root) return;
      topRequest = null;
      fileRequests.clear();
      diffRequest += 1;
      resetGitCommitFilesState(state, root);
    },

    release(): void {
      topRequest = null;
      fileRequests.clear();
      diffRequest += 1;
      resetGitCommitFilesState(state, null);
    },

    async toggleCommit(sha: string, isMerge: boolean): Promise<void> {
      if (isCommitExpanded(state, sha)) {
        state.expanded[sha] = false;
        return;
      }
      state.expanded[sha] = true;
      const entry = commitFilesEntry(state, sha);
      if (entry.loaded || entry.loading) return;
      await loadCommitFiles(sha);
      void isMerge; // the merge wording is chosen when the row is drawn
    },

    loadCommitFiles(sha: string, isMerge: boolean): Promise<void> {
      void isMerge;
      return loadCommitFiles(sha);
    },

    selectCommitFile,

    clearSelection(): void {
      diffRequest += 1;
      state.selectedCommitSha = '';
      state.selectedRelativePath = '';
    }
  };
}

/**
 * The service the commit graph uses. A lane-scoped singleton bound to
 * `gitCommitFiles` and `gitPanel`, because the panel components take no
 * required props. Constructing it performs no IO.
 */
export const gitCommitFilesService = createGitCommitFilesService();
