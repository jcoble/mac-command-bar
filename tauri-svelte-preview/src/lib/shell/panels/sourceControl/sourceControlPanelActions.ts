/**
 * sourceControlPanelActions.ts — the two panel-wide decisions the Source
 * control header makes: which remote actions can run, and which folder the
 * panel is reading.
 *
 * `sourceControlFileMenu.ts` answers the same question for one file; this file
 * answers it for the panel itself, and for the same reason: an action that is
 * off without a sentence teaches nobody anything.
 *
 * Fetch, pull and push all need a folder, a page allowed to change a
 * repository, nothing else already running, and a branch that has an upstream
 * — without one there is no remote to talk to, which is a plain fact about the
 * branch rather than a fault.
 *
 * The scope picker lists the checkouts of the same repository so the panel can
 * be pointed at one and read it. Any folder other than the session's own is
 * read-only: the panel hides the actions that write while it is on one, so the
 * "read-only" here is a statement about the panel, not a lock on git.
 *
 * Pure: no DOM, no backend, no git. The panel maps these onto the real work.
 */

import type { ProjectGitStatus } from '../../../tauriSource.ts';

export type SourceControlRemoteActionId = 'fetch' | 'pull' | 'push';

export interface SourceControlRemoteAction {
  id: SourceControlRemoteActionId;
  label: string;
  enabled: boolean;
  /** Why the action is off, in plain words. Null while it is available. */
  disabledReason: string | null;
}

/** What the panel knows that the branch itself does not. */
export interface SourceControlRemoteContext {
  /** False in a browser, which can read a repository but never change one. */
  canWrite: boolean;
  /** Why not, in the words the panel already uses for its buttons. */
  readOnlyReason: string;
  /** True while another source-control action is still running. */
  busy: boolean;
}

const NO_REPOSITORY = 'This panel is not pointed at a repository folder yet.';
const NO_UPSTREAM =
  'This branch has no upstream branch, so there is no remote to fetch from or push to.';
const STILL_RUNNING = 'Wait for the current source-control action to finish.';

/**
 * Fetch, pull and push, in the order the menu draws them. `root` is the
 * repository folder the panel is pointed at; an empty one means it has none.
 */
export function sourceControlRemoteActions(
  status: ProjectGitStatus | null,
  root: string,
  context: SourceControlRemoteContext
): SourceControlRemoteAction[] {
  const reason =
    root.trim() === ''
      ? NO_REPOSITORY
      : !context.canWrite
        ? context.readOnlyReason
        : context.busy
          ? STILL_RUNNING
          : status?.hasUpstream
            ? null
            : NO_UPSTREAM;

  return (
    [
      ['fetch', 'Fetch'],
      ['pull', 'Pull'],
      ['push', 'Push']
    ] as const
  ).map(([id, label]) => ({ id, label, enabled: reason === null, disabledReason: reason }));
}

export interface SourceControlScopeOption {
  /** The folder this option reads. */
  path: string;
  /** What the trigger and the list show. */
  label: string;
}

/** The last segment of a path, which is what a person calls the folder. */
function folderName(path: string): string {
  const trimmed = path.replace(/\/+$/, '');
  return trimmed.slice(trimmed.lastIndexOf('/') + 1) || trimmed;
}

/**
 * The folders the panel can be pointed at: the session's own first, then every
 * other checkout, each named by its folder and the branch it has out. The
 * session folder is never listed twice, because it is usually a worktree too.
 */
export function sourceControlScopeOptions(
  sessionRoot: string,
  worktrees: readonly { path: string; branch: string }[]
): SourceControlScopeOption[] {
  const session = sessionRoot.trim();
  const options: SourceControlScopeOption[] =
    session === '' ? [] : [{ path: session, label: `${folderName(session)} (session)` }];

  for (const worktree of worktrees) {
    const path = worktree.path.trim();
    if (path === '' || path === session) continue;
    const branch = (worktree.branch ?? '').trim();
    options.push({
      path,
      label: branch === '' ? folderName(path) : `${folderName(path)} — ${branch}`
    });
  }
  return options;
}

/** True while the panel is reading a folder other than the session's own. */
export function isSourceControlScopeReadOnly(sessionRoot: string, scopeRoot: string): boolean {
  const scope = scopeRoot.trim();
  return scope !== '' && scope !== sessionRoot.trim();
}

/** The one line the panel shows while it is on somebody else's checkout. */
export function describeSourceControlScope(scopeRoot: string): string {
  return `Viewing ${scopeRoot.trim()} (read-only)`;
}
