/**
 * newSessionBackend.ts — the ONLY place the new-session thread pane talks to the
 * outside world.
 *
 * Three questions and one action: which folder did the user choose in the
 * system dialog, is a typed-in folder really a project, and which checkouts
 * does this project have. Nothing here runs at import, nothing polls, nothing
 * runs from an `$effect`; the thread pane calls them when the user does something.
 *
 * Every call is counted with `countInvoke('<command name>')` so the development
 * call counter stays honest.
 *
 * Each function answers with a small result rather than throwing, because all
 * three have a perfectly ordinary "not available here" answer: the web build
 * has no system folder dialog and no git commands behind it, and saying so is
 * the pane's job. A `null` from a `…FromTauri` wrapper means nothing was
 * invoked at all — see `tauriSource.ts`.
 */
import { countInvoke } from '../devInvokeCounter.svelte.ts';
import {
  isNativeTauriRuntime,
  listProjectWorktreesFromTauri,
  validateProjectRootFromTauri,
  type ProjectRootValidationResult,
  type ProjectWorktree
} from '../../tauriSource.ts';

export type ProjectGitRef = {
  name: string;
  isDefault: boolean;
  isCurrent: boolean;
  checkoutPath: string | null;
  lastCommitMs: number | null;
};

/**
 * What came back: an answer, "this only works in the desktop app", or a failure
 * with something readable to show.
 */
export type BackendAnswer<T> =
  | { status: 'ok'; value: T }
  | { status: 'unavailable'; message: string }
  | { status: 'failed'; message: string };

/** Shown wherever a step of this pane needs the desktop app to work. */
export const DESKTOP_ONLY_MESSAGE = 'This works in the desktop app only.';

function describeError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** Whether the system "choose a folder" dialog can be opened at all. */
export function canPickFolder(): boolean {
  return isNativeTauriRuntime();
}

/**
 * Ask the system for a folder.
 *
 * `null` means the user closed the dialog without choosing, which is not a
 * failure and gets no message. In the browser there is no system dialog to
 * open, so the caller is told to type a path instead.
 */
export async function pickProjectFolder(): Promise<BackendAnswer<string | null>> {
  if (!canPickFolder()) {
    return {
      status: 'unavailable',
      message: 'The folder picker works in the desktop app only. Enter the full path instead.'
    };
  }
  try {
    countInvoke('plugin:dialog|open');
    const { open } = await import('@tauri-apps/plugin-dialog');
    const chosen = await open({
      directory: true,
      multiple: false,
      title: 'Choose where the agent should work'
    });
    const folder = Array.isArray(chosen) ? chosen[0] : chosen;
    if (typeof folder !== 'string' || folder.trim().length === 0) {
      return { status: 'ok', value: null };
    }
    return { status: 'ok', value: folder };
  } catch (error) {
    return { status: 'failed', message: `The folder chooser could not open: ${describeError(error)}` };
  }
}

/**
 * Ask whether `path` is a folder on this machine, and whether it is a git
 * repository. Used on a typed-in path before it joins the picker, so a typo
 * fails while the user is looking at it rather than when a terminal opens
 * somewhere unexpected.
 */
export async function validateProjectRoot(
  path: string,
  signal?: AbortSignal
): Promise<BackendAnswer<ProjectRootValidationResult>> {
  const trimmed = path.trim();
  if (!trimmed) {
    return { status: 'failed', message: 'Type a folder path first.' };
  }
  try {
    countInvoke('validate_project_root');
    const result = await validateProjectRootFromTauri(trimmed, signal);
    if (!result) {
      return { status: 'unavailable', message: DESKTOP_ONLY_MESSAGE };
    }
    return { status: 'ok', value: result };
  } catch (error) {
    return { status: 'failed', message: `That folder could not be checked: ${describeError(error)}` };
  }
}

/**
 * The checkouts git knows about for `root` — the project itself plus any
 * worktrees of it.
 *
 * Only ever READS. There is no command here that makes a worktree, and this
 * lane never adds one: the pane offers what already exists and hands over the
 * `git worktree add` line for anything else.
 */
export async function listWorktrees(root: string): Promise<BackendAnswer<ProjectWorktree[]>> {
  const trimmed = root.trim();
  if (!trimmed) {
    return { status: 'failed', message: 'Choose a project folder first.' };
  }
  try {
    countInvoke('list_project_worktrees');
    const worktrees = await listProjectWorktreesFromTauri(trimmed);
    if (!worktrees) {
      return { status: 'unavailable', message: DESKTOP_ONLY_MESSAGE };
    }
    return { status: 'ok', value: worktrees };
  } catch (error) {
    return {
      status: 'failed',
      message: `The working copies for this project could not be listed: ${describeError(error)}`
    };
  }
}

/** Every local branch on the project's machine, newest commit first, with
 * checkout locations attached. */
export async function listGitRefs(machine: string, root: string): Promise<BackendAnswer<ProjectGitRef[]>> {
  const trimmed = root.trim();
  if (!trimmed) {
    return { status: 'failed', message: 'Choose a project folder first.' };
  }
  if (!isNativeTauriRuntime()) {
    return { status: 'unavailable', message: DESKTOP_ONLY_MESSAGE };
  }
  try {
    countInvoke('list_project_git_refs');
    const refs = await invokeOn<ProjectGitRef[] | null>(machine, 'list_project_git_refs', { root: trimmed });
    // A folder with no git repository behind it answers with nothing rather
    // than an empty list. That is "no branches", not a list, and handing the
    // non-list straight to the branch picker throws while the picker is opening
    // — which leaves the picker stuck open with no content to dismiss.
    return { status: 'ok', value: Array.isArray(refs) ? refs : [] };
  } catch (error) {
    return {
      status: 'failed',
      message: `The branches for this project could not be listed: ${describeError(error)}`
    };
  }
}

/** `git switch <name>` in `root` on the project's machine. The only write this
 * pane makes, and only at the first send. A failure carries git's own words. */
export async function switchBranch(machine: string, root: string, name: string): Promise<BackendAnswer<null>> {
  if (!isNativeTauriRuntime()) {
    return { status: 'unavailable', message: DESKTOP_ONLY_MESSAGE };
  }
  try {
    countInvoke('switch_git_branch');
    await invokeOn(machine, 'switch_git_branch', { root, name });
    return { status: 'ok', value: null };
  } catch (error) {
    return { status: 'failed', message: describeError(error) };
  }
}

export type CreatedWorktree = { path: string; branch: string };

/** `git worktree add -b assembly-<hex> <path> <base>` on the project's machine,
 * at the first send of a New worktree draft. A failure carries git's own words. */
export async function createWorktree(machine: string, root: string, base: string): Promise<BackendAnswer<CreatedWorktree>> {
  if (!isNativeTauriRuntime()) {
    return { status: 'unavailable', message: DESKTOP_ONLY_MESSAGE };
  }
  try {
    countInvoke('create_project_worktree');
    return { status: 'ok', value: await invokeOn<CreatedWorktree>(machine, 'create_project_worktree', { root, base }) };
  } catch (error) {
    return { status: 'failed', message: describeError(error) };
  }
}

/** This Mac runs `command` itself; a remote machine runs it through its server. */
async function invokeOn<T>(machine: string, command: string, args: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  if (machine === 'local') return invoke<T>(command, args);
  return invoke<T>('remote_workspace', { profileId: machine, operation: command, args });
}
