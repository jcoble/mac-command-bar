/**
 * newSessionBackend.ts — the ONLY place the new-session thread pane talks to the
 * outside world.
 *
 * Three questions, three functions: which folder did the user choose in the
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
  path: string
): Promise<BackendAnswer<ProjectRootValidationResult>> {
  const trimmed = path.trim();
  if (!trimmed) {
    return { status: 'failed', message: 'Type a folder path first.' };
  }
  try {
    countInvoke('validate_project_root');
    const result = await validateProjectRootFromTauri(trimmed);
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

/** Every local branch, newest commit first, with checkout locations attached. */
export async function listGitRefs(root: string): Promise<BackendAnswer<ProjectGitRef[]>> {
  const trimmed = root.trim();
  if (!trimmed) {
    return { status: 'failed', message: 'Choose a project folder first.' };
  }
  if (!isNativeTauriRuntime()) {
    return { status: 'unavailable', message: DESKTOP_ONLY_MESSAGE };
  }
  try {
    countInvoke('list_project_git_refs');
    const { invoke } = await import('@tauri-apps/api/core');
    return {
      status: 'ok',
      value: await invoke<ProjectGitRef[]>('list_project_git_refs', { root: trimmed })
    };
  } catch (error) {
    return {
      status: 'failed',
      message: `The branches for this project could not be listed: ${describeError(error)}`
    };
  }
}
