/**
 * newSessionFlow.ts — the rules behind starting a new session, with no state.
 *
 * Everything here is a plain function over plain values: the list of things you
 * can launch, the command line the dialog shows before it runs anything, what
 * counts as a usable folder, how the list of known project folders is put
 * together, and the `git worktree add` line the dialog hands over instead of
 * ever making a worktree itself.
 *
 * NO imports beyond types, no backend, no storage, no Svelte. The store
 * (`projectRootsStore.svelte.ts`) and the dialog both read their answers from
 * here, which is why every rule in this file is covered by
 * `scripts/newSessionFlow.test.mjs` and none of them is covered by clicking.
 *
 * STANDING INVARIANT: nothing in this shell creates a git worktree. The picker
 * offers the checkouts that already exist; when the user wants a new one they
 * get the command to run themselves. See `worktreeAddCommand`.
 */
import type { AgentKind } from '../ownedSessions.ts';
import type { ProjectWorktree } from '../../tauriSource.ts';

// ── What you can start ────────────────────────────────────────────────────────

/** The three ways to start a session. `shell` is a terminal and nothing else. */
export type LaunchAgent = 'claude' | 'codex' | 'shell';

export type LaunchOption = {
  /** Which of the three this is. */
  agent: LaunchAgent;
  /**
   * The line typed into the fresh terminal, or `''` to type nothing at all.
   *
   * These are the bare interactive launches, taken from the tools themselves on
   * 2026-07-29: `claude --help` says "starts an interactive session by default",
   * and `codex --help` says "if no subcommand is specified, options will be
   * forwarded to the interactive CLI". So neither needs a flag, and neither gets
   * one — a flag nobody asked for is a behaviour nobody expects.
   */
  command: string;
  /** What the button says. */
  label: string;
  /** One line under the button explaining what happens. */
  hint: string;
};

/** The one value shared by the selected card, command field, and request. */
export type LaunchSelection = Pick<LaunchOption, 'agent' | 'command'>;

export const LAUNCH_CATALOG: LaunchOption[] = [
  {
    agent: 'claude',
    command: 'claude',
    label: 'Claude',
    hint: 'Start a conversation with Claude.'
  },
  {
    agent: 'codex',
    command: 'codex',
    label: 'Codex',
    hint: 'Start a conversation with Codex.'
  },
  {
    agent: 'shell',
    command: '',
    label: 'Terminal only',
    hint: 'Open a terminal without starting an agent.'
  }
];

/** The catalog entry for `agent`, or `null` when that is not one of the three. */
export function launchOptionFor(agent: string): LaunchOption | null {
  return LAUNCH_CATALOG.find((option) => option.agent === agent) ?? null;
}

/** Select an agent and its matching default command as one indivisible value. */
export function selectLaunchAgent(agent: LaunchAgent): LaunchSelection {
  return {
    agent,
    command: launchOptionFor(agent)?.command ?? ''
  };
}

/**
 * The rail's own word for who is running this session.
 *
 * `AgentKind` describes an agent conversation; a plain terminal is not one, so
 * it lands on `other` — exactly where `createFreshSession` already puts it.
 */
export function agentKindFor(agent: LaunchAgent): AgentKind {
  return agent === 'shell' ? 'other' : agent;
}

// ── The command preview ───────────────────────────────────────────────────────

/** Characters a shell passes through untouched; anything else gets quoted. */
const SAFE_SHELL_CHARACTERS = /^[A-Za-z0-9_@%+=:,./-]+$/;

/**
 * `value` written so a shell reads it as one word. Left alone when it is
 * already plain, single-quoted otherwise (with any single quote inside spliced
 * back in the way `sh` requires).
 */
export function quoteForShell(value: string): string {
  if (value.length > 0 && SAFE_SHELL_CHARACTERS.test(value)) return value;
  return `'${value.split("'").join("'\\''")}'`;
}

/**
 * The one line that says what is about to happen: move to the folder, then run
 * the command. Empty when there is no folder yet — there is nothing honest to
 * show before a project is picked — and stops at the folder when the command
 * box is empty, because then a terminal is the whole of it.
 *
 * This is what the user READS. What is actually done is a PTY opened in `cwd`
 * with `command` typed into it, which is the same thing said twice.
 */
export function buildCommandPreview(input: { cwd: string; command: string }): string {
  const cwd = normalizeRootPath(input.cwd);
  if (!cwd) return '';
  const command = input.command.trim();
  const move = `cd ${quoteForShell(cwd)}`;
  return command ? `${move} && ${command}` : move;
}

// ── Validation ────────────────────────────────────────────────────────────────

/** A draft of the session about to be started, as the dialog holds it. */
export type NewSessionDraft = {
  cwd: string;
  command: string;
  agent: LaunchAgent;
  title: string;
};

/** Something standing between the draft and the Start button. */
export type NewSessionProblem = {
  field: 'cwd' | 'command';
  message: string;
};

/**
 * Everything wrong with the draft, in the order the dialog shows the fields.
 * An empty list means Start can run.
 *
 * Deliberately NOT a check that the folder exists — that is a question for the
 * backend (`validate_project_root`), it costs a round trip, and it can change
 * between asking and starting. This function answers only what can be settled
 * from the text on screen.
 */
export function validateNewSession(draft: NewSessionDraft): NewSessionProblem[] {
  const problems: NewSessionProblem[] = [];
  const cwd = normalizeRootPath(draft.cwd);

  if (!cwd) {
    problems.push({ field: 'cwd', message: 'Choose where the session should work.' });
  } else if (!cwd.startsWith('/')) {
    problems.push({
      field: 'cwd',
      message: 'That folder needs to be a full path, starting with a slash.'
    });
  }

  // The command is typed into the terminal followed by Return. A second line
  // would be a second command, run without ever being shown in the preview.
  if (draft.command.includes('\n') || draft.command.includes('\r')) {
    problems.push({ field: 'command', message: 'Run one command, on one line.' });
  }

  return problems;
}

/** Shorthand for "nothing is stopping this". */
export function canStartNewSession(draft: NewSessionDraft): boolean {
  return validateNewSession(draft).length === 0;
}

// ── The handover ──────────────────────────────────────────────────────────────

/**
 * Everything the shell needs to actually start the session — and nothing else.
 *
 * This is the whole contract between the dialog and the page that owns the
 * session list: the folder to open a terminal in, the name for its row, which
 * agent is running there, and the line to type once the terminal is up.
 * `command` is `null` for a plain terminal, which is the same word the session
 * record already uses for "nothing to replay".
 */
export type NewSessionRequest = {
  cwd: string;
  title: string;
  agent: AgentKind;
  command: string | null;
};

/** Whether this request belongs to the structured conversation runtime. */
export function startsStructuredSession(
  request: Pick<NewSessionRequest, 'agent'> | null
): request is Pick<NewSessionRequest, 'agent'> & { agent: 'codex' | 'claude' } {
  return request?.agent === 'codex' || request?.agent === 'claude';
}

/**
 * The draft turned into the request, or `null` when the draft is not startable.
 *
 * Tidying happens HERE rather than at the caller: the folder is normalized, the
 * name falls back to the suggestion, and an empty command becomes `null`. So a
 * caller that only ever passes this straight on cannot start a session in a
 * folder spelled two different ways.
 */
export function buildNewSessionRequest(draft: NewSessionDraft): NewSessionRequest | null {
  if (validateNewSession(draft).length > 0) return null;
  const command = draft.command.trim();
  return {
    cwd: normalizeRootPath(draft.cwd),
    title: resolveSessionTitle({ cwd: draft.cwd, agent: draft.agent, title: draft.title }),
    agent: agentKindFor(draft.agent),
    command: command || null
  };
}

// ── Titles ────────────────────────────────────────────────────────────────────

const TITLE_WORD: Record<LaunchAgent, string> = {
  claude: 'Claude',
  codex: 'Codex',
  shell: 'Terminal'
};

/** The name the session row gets unless the user types their own. */
export function suggestSessionTitle(input: { cwd: string; agent: LaunchAgent }): string {
  const word = TITLE_WORD[input.agent] ?? 'Session';
  const folder = lastSegmentOf(normalizeRootPath(input.cwd));
  return folder ? `${word} in ${folder}` : word;
}

/** What the user typed, or the suggestion when they left the box empty. */
export function resolveSessionTitle(input: {
  cwd: string;
  agent: LaunchAgent;
  title: string;
}): string {
  const typed = input.title.trim();
  return typed || suggestSessionTitle(input);
}

// ── Known project folders ─────────────────────────────────────────────────────

/** Where a folder in the picker came from. */
export type KnownRootSource = 'default' | 'custom' | 'session';

export type KnownRoot = {
  id: string;
  name: string;
  path: string;
  source: KnownRootSource;
};

/** A folder the user added by hand, as it is stored and read back. */
export type CustomRoot = {
  id: string;
  name: string;
  path: string;
};

/** One spelling per folder: trimmed, with any trailing slashes removed. */
export function normalizeRootPath(path: string): string {
  const trimmed = (path ?? '').trim();
  if (trimmed === '/') return '/';
  return trimmed.replace(/\/+$/, '');
}

function lastSegmentOf(path: string): string {
  return path.split('/').filter(Boolean).at(-1) ?? '';
}

function isUsableRootPath(path: string): boolean {
  return path.startsWith('/') && path.length > 1;
}

/**
 * The folders the picker offers, in the order it offers them: the ones built
 * in, then the ones the user added, then the ones the sessions already on the
 * rail are running in.
 *
 * One folder gets one row however many places it came from — the first spelling
 * wins, so a built-in name is never replaced by a folder name derived from a
 * session's working directory. Anything that is not a full path is dropped
 * rather than shown as a row that cannot be started.
 */
export function mergeKnownRoots(input: {
  defaults: { id: string; name: string; path: string }[];
  custom: { id: string; name: string; path: string }[];
  sessionPaths: string[];
}): KnownRoot[] {
  const merged: KnownRoot[] = [];
  const seen = new Set<string>();

  const add = (candidate: KnownRoot): void => {
    if (!isUsableRootPath(candidate.path) || seen.has(candidate.path)) return;
    seen.add(candidate.path);
    merged.push(candidate);
  };

  for (const root of input.defaults) {
    const path = normalizeRootPath(root.path);
    add({ id: root.id, name: root.name.trim() || lastSegmentOf(path), path, source: 'default' });
  }
  for (const root of input.custom) {
    const path = normalizeRootPath(root.path);
    add({ id: root.id, name: root.name.trim() || lastSegmentOf(path), path, source: 'custom' });
  }

  // Sessions arrive in whatever order the rail holds them; a picker that
  // reshuffles itself as sessions come and go is a picker you cannot aim at.
  const sessionRoots = input.sessionPaths
    .map((path) => normalizeRootPath(path))
    .filter((path) => isUsableRootPath(path))
    .map((path) => ({ path, name: lastSegmentOf(path) }))
    .sort((left, right) => left.name.localeCompare(right.name, undefined, { sensitivity: 'base' }));

  for (const root of sessionRoots) {
    add({ id: `session:${root.path}`, name: root.name, path: root.path, source: 'session' });
  }

  return merged;
}

/**
 * A folder the user added, ready to store. `null` when the path is not one this
 * shell can start a session in.
 *
 * The id is the folder itself, so adding the same folder twice is the same row
 * twice — which the merge above then collapses to one.
 */
export function createCustomRoot(path: string, name?: string): CustomRoot | null {
  const normalized = normalizeRootPath(path);
  if (!isUsableRootPath(normalized)) return null;
  return {
    id: `custom:${normalized}`,
    name: (name ?? '').trim() || lastSegmentOf(normalized),
    path: normalized
  };
}

/** The added folders as one string for storage. */
export function serializeCustomRoots(roots: CustomRoot[]): string {
  return JSON.stringify(roots);
}

/**
 * The added folders read back. Tolerant on purpose: a payload from an older
 * build, a half-written value, or plain nonsense reads back as "none added
 * yet", which is a working picker rather than a broken dialog.
 */
export function parseStoredCustomRoots(raw: string | null): CustomRoot[] {
  if (!raw) return [];
  let data: unknown;
  try {
    data = JSON.parse(raw);
  } catch {
    return [];
  }
  if (!Array.isArray(data)) return [];

  const roots: CustomRoot[] = [];
  const seen = new Set<string>();
  for (const entry of data) {
    if (typeof entry !== 'object' || entry === null) continue;
    const candidate = entry as Record<string, unknown>;
    const path = typeof candidate.path === 'string' ? candidate.path : '';
    const name = typeof candidate.name === 'string' ? candidate.name : '';
    const root = createCustomRoot(path, name);
    if (!root || seen.has(root.id)) continue;
    seen.add(root.id);
    roots.push(root);
  }
  return roots;
}

// ── Which checkout to start in ────────────────────────────────────────────────

export type WorktreeChoice = {
  /** The folder the session would run in. */
  path: string;
  /** What the row says. */
  label: string;
  /** The branch checked out there, when git said. */
  branch: string | null;
  /** Whether this is the project's own checkout rather than a worktree of it. */
  isPrimary: boolean;
  /** A short warning worth reading before starting here, or `null`. */
  note: string | null;
};

/**
 * The checkouts the user can start in: the project's own folder first, then
 * every worktree git listed, in git's order.
 *
 * The project folder is offered even when `worktrees` is `null` — that is what
 * a wrapper returns when the desktop app is not there to ask, and "we could not
 * list worktrees" is no reason to refuse to start a session in the project
 * itself.
 */
export function worktreeChoicesFor(input: {
  projectRoot: string;
  worktrees: ProjectWorktree[] | null;
}): WorktreeChoice[] {
  const root = normalizeRootPath(input.projectRoot);
  if (!isUsableRootPath(root)) return [];

  const listed = input.worktrees ?? [];
  const primaryEntry = listed.find((worktree) => normalizeRootPath(worktree.path) === root) ?? null;

  const choices: WorktreeChoice[] = [
    {
      path: root,
      label: `${lastSegmentOf(root)} — main folder`,
      branch: primaryEntry?.branch ?? null,
      isPrimary: true,
      note: primaryEntry ? noteFor(primaryEntry) : null
    }
  ];

  const seen = new Set<string>([root]);
  for (const worktree of listed) {
    const path = normalizeRootPath(worktree.path);
    if (!isUsableRootPath(path) || seen.has(path)) continue;
    seen.add(path);
    choices.push({
      path,
      label: worktree.branch ? `${worktree.branch} — ${lastSegmentOf(path)}` : lastSegmentOf(path),
      branch: worktree.branch ?? null,
      isPrimary: false,
      note: noteFor(worktree)
    });
  }

  return choices;
}

/** The one thing worth saying about a checkout before you start work in it. */
function noteFor(worktree: ProjectWorktree): string | null {
  if (worktree.isLocked) return 'locked';
  if (worktree.isDirty) return 'has uncommitted changes';
  return null;
}

// ── Making a worktree is never something this dialog does ─────────────────────

/** A branch name git will take: lowercase, words joined by dashes. */
function branchSlug(name: string): string {
  const slug = name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._/-]+/g, '-')
    .replace(/^-+|-+$/g, '');
  return slug || 'new-branch';
}

/**
 * Where a new worktree for `repoRoot` should go: in a `worktrees` folder beside
 * the repository, one directory per repository, one per branch. Beside it, not
 * inside it — a worktree under the repository shows up in its own file lists,
 * its own searches, and its own `git status`.
 */
export function suggestedWorktreePath(repoRoot: string, name: string): string {
  const root = normalizeRootPath(repoRoot);
  if (!isUsableRootPath(root)) return '';
  const repoName = lastSegmentOf(root);
  const parent = root.slice(0, root.length - repoName.length - 1) || '';
  return `${parent}/worktrees/${repoName}/${branchSlug(name)}`;
}

/**
 * The command that makes the worktree — for the user to run, not for this
 * shell to run.
 *
 * The dialog shows this line and nothing else happens. Creating a worktree
 * writes to a repository the user is working in and can only be undone by hand,
 * so it stays a thing a person types. Empty when there is no project to make
 * one from.
 */
export function worktreeAddCommand(repoRoot: string, name: string): string {
  const root = normalizeRootPath(repoRoot);
  const path = suggestedWorktreePath(root, name);
  if (!path) return '';
  return `git -C ${quoteForShell(root)} worktree add ${quoteForShell(path)} -b ${branchSlug(name)}`;
}
