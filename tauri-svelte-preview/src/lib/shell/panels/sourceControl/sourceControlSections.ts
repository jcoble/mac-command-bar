/**
 * sourceControlSections.ts — how the panel splits the working copy into the two
 * lists it draws, and the counts beside its title.
 *
 * The backend already decides what each file's state is (`badge`, `indexStatus`,
 * `worktreeStatus`), so nothing here reads git output. It sorts and groups, and
 * that is all — which is why both functions are pure over the status value and
 * can be checked without a browser.
 *
 * The panel shows Untracked and Changed rather than the older staged/unstaged
 * split: staging is a single "Stage All" press in this version, so a per-file
 * staged column would be a distinction nobody can act on.
 */

import type { ProjectGitFileStatus, ProjectGitStatus } from '../../../tauriSource.ts';
import { isGitFileUntracked } from '../../git/gitPanelStore.svelte.ts';

export interface SourceControlSection {
  id: 'untracked' | 'changed';
  label: string;
  files: ProjectGitFileStatus[];
}

export interface SourceControlDiffstat {
  /** How many files the working copy has changed in any way. */
  filesChanged: number;
  /** How many of those are new files git is not tracking yet. */
  untracked: number;
}

function byPath(left: ProjectGitFileStatus, right: ProjectGitFileStatus): number {
  return left.relativePath.localeCompare(right.relativePath);
}

/** Untracked files first, then everything else, each sorted by relativePath. */
export function sourceControlSections(status: ProjectGitStatus | null): SourceControlSection[] {
  const files = status?.files ?? [];
  return [
    {
      id: 'untracked',
      label: 'Untracked',
      files: files.filter(isGitFileUntracked).sort(byPath)
    },
    {
      id: 'changed',
      label: 'Changed',
      files: files.filter((file) => !isGitFileUntracked(file)).sort(byPath)
    }
  ];
}

/**
 * The counts beside the panel title. Line-by-line additions and deletions are
 * deliberately absent: the status command this panel reads reports a state per
 * file and no line counts, and a number nobody measured is worse than no number.
 */
export function sourceControlDiffstat(status: ProjectGitStatus | null): SourceControlDiffstat {
  const files = status?.files ?? [];
  return {
    filesChanged: files.length,
    untracked: files.filter(isGitFileUntracked).length
  };
}
