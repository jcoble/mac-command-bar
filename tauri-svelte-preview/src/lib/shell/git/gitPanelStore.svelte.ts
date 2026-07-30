/**
 * gitPanelStore.svelte.ts — runes state for the /next source-control panel.
 *
 * Started from `src/lib/stores/gitStore.svelte.ts` and kept to the same two
 * rules that store was audited clean against:
 *
 * 1. **No backend, ever.** This module performs no IO of any kind. Every Tauri
 *    call lives in `gitService.ts` and lands here as a plain assignment.
 * 2. **No `$effect`.** `$effect` is illegal in a `.svelte.ts` module and against
 *    the shell constitution. Nothing in here reacts; it is a value bag.
 *
 * Two things live here that the old store kept in the page, because they are
 * pure functions of the status value and belong next to it: the changed-file
 * grouping (staged / unstaged / untracked) and the short description helpers the
 * panel renders. They take their input as arguments and touch no state, so the
 * node test can exercise them directly.
 *
 * The exported `gitPanel` is a lane-scoped singleton: `GitPanel.svelte` has no
 * required props (the shell's panel contract), so the component reaches the
 * state through this module rather than through the page. Creating it costs
 * nothing at import — `createGitPanelState()` allocates a plain object.
 */

import type {
  GitCommitHistoryEntry,
  ProjectGitFileStatus,
  ProjectGitStatus,
  SourceGitDiff
} from '../../tauriSource.ts';

/** The working-tree action currently running, or '' when nothing is running. */
export type GitActionKind = 'stage' | 'unstage' | 'commit' | 'fetch' | 'pull' | 'push';

export interface GitPanelState {
  // ── which repository the panel is pointed at ─────────────────────────────
  /** Absolute path of the project the panel is showing, or null before it is
   *  pointed at one. Set only by `gitService.activate(root)`. */
  root: string | null;
  /** True once `activate()` has run for the current root. Before that the panel
   *  renders an inert empty state and issues no backend calls. */
  activated: boolean;
  /** True when the backend said "not running in the desktop app". The panel then
   *  says so plainly and shows nothing — it never invents stand-in data. */
  desktopOnly: boolean;

  // ── working-tree status ───────────────────────────────────────────────────
  status: ProjectGitStatus | null;
  statusLoading: boolean;
  statusError: string;

  // ── commit history ────────────────────────────────────────────────────────
  history: GitCommitHistoryEntry[];
  historyLoading: boolean;
  historyError: string;
  /** How many commits the last history read asked the app for. 0 before any read.
   *  The backend always answers with the whole list from the top, so this grows
   *  and the list is replaced — never appended to — which keeps the graph's
   *  columns correct across pages. */
  historyRequested: number;
  /** True while "Load more" is running. Separate from `historyLoading` so the
   *  list stays on screen instead of flipping back to "Reading the history…". */
  historyLoadingMore: boolean;
  /** True once a read came back with fewer commits than it asked for. Asking the
   *  same way again would return the same list, so there is nothing more to load. */
  historyComplete: boolean;
  /** True once "Load more" has been used for this repository. */
  historyPaged: boolean;
  /** True once we have asked for the most this app will ask for in one go. */
  historyCeiling: boolean;

  // ── the file whose diff is on screen ──────────────────────────────────────
  /** Repository-relative path of the selected file, or '' when none is selected. */
  selectedPath: string;
  selectedDiff: SourceGitDiff | null;
  diffLoading: boolean;
  diffError: string;

  // ── working-tree actions ──────────────────────────────────────────────────
  /** Draft commit message (bound to the commit box). */
  commitMessage: string;
  /** Which action is running right now, or ''. */
  actionBusy: GitActionKind | '';
  /** Plain-English result of the last action. */
  actionStatus: string;
  /** Plain-English failure of the last action. */
  actionError: string;
}

/** A fresh, empty panel state. Plain object: no runes, safe to call anywhere. */
export function createGitPanelState(): GitPanelState {
  return {
    root: null,
    activated: false,
    desktopOnly: false,
    status: null,
    statusLoading: false,
    statusError: '',
    history: [],
    historyLoading: false,
    historyError: '',
    historyRequested: 0,
    historyLoadingMore: false,
    historyComplete: false,
    historyPaged: false,
    historyCeiling: false,
    selectedPath: '',
    selectedDiff: null,
    diffLoading: false,
    diffError: '',
    commitMessage: '',
    actionBusy: '',
    actionStatus: '',
    actionError: ''
  };
}

/**
 * Wipe everything the previous repository put on screen and point the state at
 * `root`. Keeps the object identity so the runes proxy stays the same.
 */
export function resetGitPanelState(state: GitPanelState, root: string | null): void {
  const fresh = createGitPanelState();
  Object.assign(state, fresh, { root });
}

/** Forget the file whose diff is on screen. */
export function clearSelectedGitFile(state: GitPanelState): void {
  state.selectedPath = '';
  state.selectedDiff = null;
  state.diffLoading = false;
  state.diffError = '';
}

/** The reactive panel state the /next source-control panel reads and writes. */
export const gitPanel = $state<GitPanelState>(createGitPanelState());

// ── pure view-model helpers ─────────────────────────────────────────────────
// Ported from `src/routes/+page.svelte` (`buildGitStatusFileGroups` and friends,
// around :5140). The backend pre-computes `status` and `badge` per file, so
// nothing here parses git porcelain output — it only groups and describes.

export type GitStatusGroupId = 'staged' | 'unstaged' | 'untracked';

export interface GitStatusFileGroup {
  id: GitStatusGroupId;
  label: string;
  files: ProjectGitFileStatus[];
  /** What the group's bulk button does. */
  action: 'stage' | 'unstage';
}

/** The file has something in the index that is not just "untracked". */
export function isGitFileStaged(file: ProjectGitFileStatus | null | undefined): boolean {
  return Boolean(file?.indexStatus && file.badge !== '?');
}

export function isGitFileUntracked(file: ProjectGitFileStatus): boolean {
  return file.badge === '?' || file.indexStatus === '?' || file.worktreeStatus === '?';
}

export function hasGitFileUnstagedChanges(file: ProjectGitFileStatus): boolean {
  return isGitFileUntracked(file) || Boolean(file.worktreeStatus);
}

/** True when the file is gone from disk, so there is no diff to read for it. */
export function isGitFileDeleted(file: ProjectGitFileStatus): boolean {
  return file.badge === 'D' || file.worktreeStatus === 'deleted';
}

/**
 * Split the changed files into the three groups the panel shows. A file with
 * both staged and unstaged work appears in two groups — that is the truth, and
 * it is how the old shell showed it.
 */
export function buildGitStatusFileGroups(files: ProjectGitFileStatus[]): GitStatusFileGroup[] {
  const staged = files.filter(isGitFileStaged);
  const unstaged = files.filter(
    (file) => hasGitFileUnstagedChanges(file) && !isGitFileUntracked(file)
  );
  const untracked = files.filter(isGitFileUntracked);

  const groups: GitStatusFileGroup[] = [
    { id: 'staged', label: 'Staged', files: staged, action: 'unstage' },
    { id: 'unstaged', label: 'Changed', files: unstaged, action: 'stage' },
    { id: 'untracked', label: 'New files', files: untracked, action: 'stage' }
  ];
  return groups.filter((group) => group.files.length > 0);
}

/** "Staged 2 · Changed 5", or a plain sentence when there is nothing. */
export function describeGitStatusGroups(groups: GitStatusFileGroup[]): string {
  if (groups.length === 0) return 'No changed files';
  return groups.map((group) => `${group.label} ${group.files.length}`).join(' · ');
}

export function gitStatusGroupActionLabel(group: GitStatusFileGroup): string {
  return group.action === 'unstage' ? 'Unstage all' : 'Stage all';
}

/** The hover text for a changed-file row. */
export function gitFileTitle(file: ProjectGitFileStatus): string {
  const states = [
    file.indexStatus ? `Staged: ${file.indexStatus}` : '',
    file.worktreeStatus ? `On disk: ${file.worktreeStatus}` : ''
  ].filter(Boolean);
  return states.length > 0 ? `${file.relativePath}\n${states.join('\n')}` : file.relativePath;
}

/** The small grey line under a changed-file row. */
export function describeGitFileChange(file: ProjectGitFileStatus): string {
  if (file.indexStatus && file.worktreeStatus) {
    return `${file.indexStatus} + ${file.worktreeStatus}`;
  }
  return file.status || 'changed';
}

// ── how much of the history is on screen ────────────────────────────────────
// The history is read in pages: a short first list, then more on request. These
// four functions are the whole of the panel's honesty about that — they are
// pure over the state above so a node test can check every wording, including
// the ones that only appear against an app build that reads fewer commits than
// this panel asks for.

/**
 * A read that came back with fewer commits than it asked for has nothing more to
 * give: asking again the same way returns the same list. That is true whether
 * the repository ran out of commits or the app build stopped early, which is why
 * the sentences below never claim to know which.
 */
export function isGitHistoryComplete(requested: number, received: number): boolean {
  return received < requested;
}

/** Should the "Load more" button be offered right now? */
export function canLoadMoreGitHistory(state: GitPanelState): boolean {
  return (
    state.activated &&
    !state.desktopOnly &&
    state.history.length > 0 &&
    !state.historyComplete &&
    !state.historyCeiling &&
    !state.historyLoading &&
    !state.historyLoadingMore
  );
}

/**
 * The short count beside the "Commits" heading. "so far" is the whole point:
 * a bare number next to a list of 24 out of thousands reads as the total.
 */
export function describeGitHistoryCount(state: GitPanelState): string {
  const shown = state.history.length;
  if (shown === 0) return '';
  if (state.historyComplete || state.historyCeiling) return String(shown);
  return `${shown} so far`;
}

/**
 * The sentence under the list, and the hover text on the count. It says what
 * actually came back rather than what was asked for, so an older app build that
 * returns fewer commits than this panel requests cannot look like a repository
 * that has run out of history.
 */
export function describeGitHistoryFooter(state: GitPanelState): string {
  const shown = state.history.length;
  if (shown === 0) return '';
  if (state.historyCeiling) {
    return `Showing ${shown} commits — the most this app reads at one time. Older commits are not in this list.`;
  }
  if (!state.historyComplete) return `Showing ${shown} so far.`;
  if (!state.historyPaged) return `Showing all ${shown} commits.`;
  return (
    `Showing ${shown} commits. We asked for ${state.historyRequested} and this is all that came ` +
    `back, so it is everything this app will show for this repository.`
  );
}

/** "main · 2 ahead · 1 behind", or why there is no branch line to show. */
export function describeGitBranch(status: ProjectGitStatus | null): string {
  if (!status) return 'No repository loaded';
  const parts: string[] = [status.branch ?? 'no branch checked out'];
  if (!status.hasUpstream) {
    parts.push('no upstream branch');
  } else {
    if (status.ahead > 0) parts.push(`${status.ahead} to push`);
    if (status.behind > 0) parts.push(`${status.behind} to pull`);
    if (status.ahead === 0 && status.behind === 0) parts.push('up to date');
  }
  return parts.join(' · ');
}

/**
 * The hover text for the branch line. A branch name like
 * `codex/outbound-rule-generation` does not fit a narrow panel, and a name cut
 * off at `outbound-rule-generat…` is worse than no name at all — you cannot tell
 * two long branches apart. So the full name always exists somewhere you can
 * read it, with the ahead/behind counts spelled out as sentences rather than
 * arrows.
 */
export function describeGitBranchTitle(status: ProjectGitStatus | null): string {
  if (!status) return 'No repository loaded';
  const branch = status.branch ?? '';
  const lines: string[] = [branch === '' ? 'No branch is checked out.' : `Branch: ${branch}`];

  if (!status.hasUpstream) {
    lines.push('This branch has no matching branch on the remote yet.');
  } else if (status.ahead === 0 && status.behind === 0) {
    lines.push('Up to date with the remote.');
  } else {
    if (status.ahead > 0) {
      lines.push(
        `${status.ahead} commit${status.ahead === 1 ? '' : 's'} here that the remote does not have.`
      );
    }
    if (status.behind > 0) {
      lines.push(
        `${status.behind} commit${status.behind === 1 ? '' : 's'} on the remote that you do not have.`
      );
    }
  }
  return lines.join('\n');
}

/** The last path segment of an absolute path, for the panel's title line. */
export function repositoryLabel(root: string | null): string {
  if (!root) return '';
  const trimmed = root.replace(/\/+$/, '');
  const index = trimmed.lastIndexOf('/');
  return index >= 0 ? trimmed.slice(index + 1) : trimmed;
}

/** True when there is at least one staged file, so a commit could succeed. */
export function hasStagedChanges(status: ProjectGitStatus | null): boolean {
  return (status?.files ?? []).some(isGitFileStaged);
}

/**
 * Is this failure just "there is no repository in this folder"?
 *
 * That is an ordinary thing for a folder to be, not a fault, and git reports it
 * the same way it reports real breakage — on stderr, starting with `fatal:`. So
 * the panel has to tell the two apart to avoid showing a plain folder a red
 * error it can do nothing about. Everything this does NOT match stays an error
 * and keeps its own message, which is what a genuine failure needs.
 */
export function isNotARepositoryError(message: string): boolean {
  // Deliberately narrow. A phrase matched too eagerly here would HIDE a real
  // failure behind a calm empty state, which is worse than showing a message
  // that is hard to read — so only git's own wording for this counts.
  return /not a git repository/i.test(message);
}
