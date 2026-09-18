/**
 * gitBackendExtra.ts — the two git questions the shell could not ask before,
 * plus the browser's way of asking any of them.
 *
 * WHAT IS NEW HERE
 * The desktop app grew two commands this round:
 *   - `read_git_commit_files(root, sha)` — which files one commit touched.
 *   - `read_git_commit_file_diff(root, sha, relativePath)` — what changed
 *     inside one of those files, in that commit.
 * Everything else the source-control panel needs already had a wrapper in
 * `tauriSource.ts`; this file adds only the missing two, in the same style, so
 * the panel keeps one habit: a wrapper returns `null` when there is nothing to
 * ask, and the caller says so plainly instead of inventing data.
 *
 * ⚠ THE PATH TRAP. `read_git_commit_file_diff` takes a path RELATIVE to the
 * repository, while the older `read_source_git_diff` takes the whole path on
 * disk. They look like the same argument and they are not. Passing the wrong
 * one gets you "Git actions only accept relative repo paths" or "Source path is
 * outside Git root" — so each wrapper below names its argument for what it is.
 *
 * WHAT ELSE IS HERE: THE BROWSER
 * The Rust commands only exist inside the desktop app, so in a browser the
 * whole panel could previously only say "desktop app only" — which made it
 * impossible to look at while building it. The dev server now answers the same
 * questions by running git itself (`src/lib/server/gitBridge.ts`), and the
 * functions below try the desktop first and that bridge second. The bridge
 * READS ONLY: staging, committing, fetching, pulling and pushing stay in the
 * desktop app, and `bridgeGitBackend()` says exactly that if something asks.
 *
 * TOLERATING AN OLDER DESKTOP BUILD
 * A desktop app built before the two new commands existed answers "command not
 * found". That is not a failure worth a red error — it means "this build is
 * older than this panel". `isMissingCommandError` spots it so the caller can
 * say so in one plain sentence.
 */

import type { GitBackend } from './gitService.ts';
import { validateProjectRootFromTauri } from '../../tauriSource.ts';
import type {
  GitActionResult,
  GitHistoryPage,
  ProjectGitStatus,
  SourceGitDiff
} from '../../tauriSource.ts';

/** One file touched by one commit — the same three fields a changed-file row shows. */
export interface GitCommitFileChange {
  relativePath: string;
  status: string;
  badge: string;
}

/** Said whenever a write is attempted anywhere but the desktop app. */
export const READ_ONLY_IN_BROWSER_MESSAGE =
  'Changing the repository runs in the desktop app only. This page can read it, not commit to it.';

/** Said when the desktop app is older than this panel. */
export const MISSING_COMMAND_MESSAGE =
  'This desktop app was built before per-commit file lists existed. Restart it after the next update.';

export const GIT_DIFF_TIMEOUT_MESSAGE =
  'Reading these changes took too long. Try refreshing Source Control.';

/** A selected diff must always leave its loading state, even if an IPC read
 * never settles. The timer exists only while that user-triggered read is live. */
export async function withGitDiffTimeout<T>(
  read: Promise<T>,
  timeoutMs = 20_000
): Promise<T> {
  let timeout: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      read,
      new Promise<never>((_resolve, reject) => {
        timeout = setTimeout(() => reject(new Error(GIT_DIFF_TIMEOUT_MESSAGE)), timeoutMs);
      })
    ]);
  } finally {
    if (timeout !== undefined) clearTimeout(timeout);
  }
}

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/** True while the page can also reach the dev server's read-only git bridge. */
export function hasGitBridge(): boolean {
  return typeof window !== 'undefined' && typeof fetch === 'function';
}

/** Where this page's git answers come from, in words fit for the screen. */
export function gitDataSourceLabel(): string {
  if (isTauriRuntime()) return 'desktop app';
  if (hasGitBridge()) return 'dev server (reads only)';
  return 'nowhere';
}

/**
 * Does this error mean "the desktop app does not have that command"? Tauri
 * phrases it a few ways depending on version, so match the stable words.
 */
export function isMissingCommandError(error: unknown): boolean {
  const message = (error instanceof Error ? error.message : String(error ?? '')).toLowerCase();
  return (
    message.includes('not found') &&
    (message.includes('command') || message.includes('invoke')) &&
    !message.includes('not a git repository')
  );
}

// ── which folder to ask ─────────────────────────────────────────────────────

/**
 * The top of the repository a folder belongs to, or `null` when nothing can
 * work it out.
 *
 * WHY THIS MATTERS. `git show <sha> -- <path>` matches the path against the
 * folder git is run in, not against the top of the repository. Ask from a
 * subfolder with a repository-relative path and git matches nothing, prints
 * nothing, and reports success — a blank diff that reads exactly like "this
 * file did not change". The panel is pointed at whatever folder a session is
 * working in, and that is often a subfolder, so the top has to be worked out
 * rather than assumed.
 *
 * `validateProjectRootFromTauri` already answers this in both places: the
 * desktop app runs `git rev-parse --show-toplevel`, and in a browser the dev
 * server's source bridge does the same walk up the folders.
 */
export async function resolveRepositoryTop(root: string): Promise<string | null> {
  if (!root.trim()) return null;
  const validation = await validateProjectRootFromTauri(root);
  const top = validation?.gitRoot ?? null;
  return top && top.trim() !== '' ? top : null;
}

// ── the two new desktop commands ────────────────────────────────────────────

/** Mirrors `read_git_commit_files`. `null` means "not the desktop app". */
export async function readGitCommitFilesFromTauri(
  root: string,
  sha: string
): Promise<GitCommitFileChange[] | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('../../workspaceInvoke');
  return invoke<GitCommitFileChange[]>('read_git_commit_files', { root, sha });
}

/**
 * Mirrors `read_git_commit_file_diff`. `null` means "not the desktop app".
 * `relativePath` is relative to the repository — see the path trap at the top.
 */
export async function readGitCommitFileDiffFromTauri(
  root: string,
  sha: string,
  relativePath: string
): Promise<SourceGitDiff | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import('../../workspaceInvoke');
  return invoke<SourceGitDiff>('read_git_commit_file_diff', { root, sha, relativePath });
}

// ── the dev server's read-only bridge ───────────────────────────────────────

/**
 * Ask the dev server one git question. `null` means the bridge is not there —
 * a built page with no dev server behind it, for instance — which is a reason
 * to say "desktop app only", not an error to show.
 */
export async function postGitBridge<T>(
  route: string,
  payload: Record<string, unknown>
): Promise<T | null> {
  if (!hasGitBridge()) return null;

  let response: Response;
  try {
    response = await fetch(`/__mcb/git/${route}`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(payload)
    });
  } catch {
    return null;
  }

  if (!(response.headers.get('content-type') ?? '').includes('application/json')) return null;
  const text = await response.text();
  const body = text ? JSON.parse(text) : null;

  if (!response.ok) {
    const message =
      body && typeof body === 'object' && 'error' in body
        ? String((body as { error: unknown }).error)
        : `The git bridge answered with HTTP ${response.status}`;
    throw new Error(message);
  }

  return body as T;
}

// ── desktop first, bridge second ────────────────────────────────────────────

/**
 * Which files one commit touched. An empty list is a real answer: a merge
 * commit brings no changes of its own, so git lists nothing for it.
 */
export async function readCommitFiles(
  root: string,
  sha: string
): Promise<GitCommitFileChange[] | null> {
  const fromDesktop = await askDesktop(() => readGitCommitFilesFromTauri(root, sha));
  if (fromDesktop) return fromDesktop;
  return postGitBridge<GitCommitFileChange[]>('commit-files', { root, sha });
}

/**
 * Run a desktop call, turning "that command does not exist" into one plain
 * sentence. Every other failure keeps git's own words.
 */
async function askDesktop<T>(run: () => Promise<T | null>): Promise<T | null> {
  try {
    return await run();
  } catch (error) {
    if (isMissingCommandError(error)) throw new Error(MISSING_COMMAND_MESSAGE);
    throw error;
  }
}

/** What changed inside one file of one commit. `relativePath` is repository-relative. */
export async function readCommitFileDiff(
  root: string,
  sha: string,
  relativePath: string
): Promise<SourceGitDiff | null> {
  const fromDesktop = await askDesktop(() =>
    readGitCommitFileDiffFromTauri(root, sha, relativePath)
  );
  if (fromDesktop) return fromDesktop;
  return postGitBridge<SourceGitDiff>('commit-file-diff', { root, sha, relativePath });
}

/**
 * The panel's usual backend, answered by the dev server instead of the desktop
 * app, so the whole surface can be looked at in a browser.
 *
 * Reads work. Writes do not, and say so in one sentence rather than failing
 * with something only a programmer could read — the panel also disables those
 * buttons, so reaching this message means something got past the button.
 */
export function bridgeGitBackend(): GitBackend {
  const refuse = async (): Promise<GitActionResult | null> => {
    throw new Error(READ_ONLY_IN_BROWSER_MESSAGE);
  };

  return {
    readStatus: (root) => postGitBridge<ProjectGitStatus>('status', { root }),
    readDiff: (root, absolutePath) =>
      postGitBridge<SourceGitDiff>('file-diff', { root, path: absolutePath }),
    readHistory: (root, cursor, relativePath) =>
      postGitBridge<GitHistoryPage>('history', { root, cursor, relativePath }),
    stage: refuse,
    unstage: refuse,
    commit: refuse,
    amend: refuse,
    // Discarding, branching and stashing all change the repository, so the
    // browser refuses them for the same reason it refuses a commit — and a
    // discard in particular must never be reachable from a tab left open.
    discard: refuse,
    discardAll: refuse,
    createBranch: refuse,
    switchBranch: refuse,
    stash: refuse,
    popStash: refuse,
    listBranches: async () => null,
    listStashes: async () => null,
    fetch: refuse,
    pull: refuse,
    push: refuse
  };
}

/** True when this page can change the repository (i.e. it is the desktop app). */
export function canChangeRepository(): boolean {
  return isTauriRuntime();
}
