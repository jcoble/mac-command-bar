/**
 * Owned-session identity and lifecycle records for the /next shell.
 *
 * ownedId (CommandBar-minted) is the PRIMARY key across rail, manager,
 * persistence, and active-tracking. Provider ids are mutable (a resume mints a
 * new one), so they are carried but never used as keys.
 */
import type { AgentSession, TerminalSessionInfo } from '../tauriSource';

export type AgentKind = 'codex' | 'claude' | 'gemini' | 'opencode' | 'other';
export type OwnedSessionState = 'live' | 'background' | 'exited';

export type OwnedSession = {
  ownedId: string;
  agent: AgentKind;
  viaCmux: boolean;
  source: 'scanned' | 'fresh';
  title: string;
  projectPath: string | null;
  cwd: string;
  resumeCommand: string | null;
  nativeSessionId: string | null;
  ptySessionId: string | null;
  state: OwnedSessionState;
  /**
   * When the user marked this session done, as an ISO stamp; `null` while it is
   * still being worked on.
   *
   * Deliberately NOT touched by anything that happens to the terminal. A process
   * exiting says the terminal is over, not that the work is; a session whose
   * agent has finished is still on the list until the user says otherwise, and a
   * session marked done can still be running. Only the user's "Mark done" and
   * "Reopen" write this field.
   */
  completedAt: string | null;
  /**
   * What the scanner worked out about the session — the branch it is on, the
   * task it belongs to, the pull request it opened. Copied off the scanned
   * record when the session is adopted so a row keeps showing them afterwards,
   * and `null` whenever the scanner found nothing or the session was started
   * here rather than found on disk.
   */
  branch: string | null;
  taskId: string | null;
  pullRequest: string | null;
  /**
   * How many turns of the conversation the scanner saw, and the last one of them
   * already written as the row shows it (`You: …` / `Agent: …`). Copied off the
   * scanned record when the session is adopted, so the row keeps showing them
   * afterwards.
   *
   * The count is a floor, not a total — the scanner reads a bounded window of
   * each transcript. `null` whenever the scanner found no conversation, or the
   * session was started here rather than found on disk.
   */
  messageCount: number | null;
  latestTurnPreview: string | null;
  /**
   * When the scanner last saw anything happen in this conversation, as the
   * stamp it reported. Copied off the scanned record when the session is
   * adopted, so a row can say how old the work is.
   *
   * It is a reading of the transcript on disk taken at adopt time and nothing
   * moves it afterwards — a session started here has none at all. `null`
   * whenever the scanner had nothing to say, and the row then shows no stamp.
   */
  lastActivity: string | null;
};

const KNOWN_AGENTS: AgentKind[] = ['codex', 'claude', 'gemini', 'opencode'];
const KNOWN_STATES: OwnedSessionState[] = ['live', 'background', 'exited'];

function defaultMintId(): string {
  return globalThis.crypto.randomUUID();
}

export function normalizeProvider(provider: string): { agent: AgentKind; viaCmux: boolean } {
  const value = (provider ?? '').trim().toLowerCase();
  const viaCmux = value.startsWith('cmux-');
  const base = viaCmux ? value.slice('cmux-'.length) : value;
  const agent = (KNOWN_AGENTS as string[]).includes(base) ? (base as AgentKind) : 'other';
  return { agent, viaCmux };
}

export function adoptAgentSession(record: AgentSession, mintId: () => string = defaultMintId): OwnedSession {
  const { agent, viaCmux } = normalizeProvider(record.provider);
  return {
    ownedId: mintId(),
    agent,
    viaCmux,
    source: 'scanned',
    title: record.title,
    projectPath: record.projectPath,
    cwd: record.projectPath ?? '',
    resumeCommand: record.resumeCommands[0] ?? null,
    nativeSessionId: record.id,
    ptySessionId: null,
    state: 'background',
    completedAt: null,
    branch: isNonEmptyString(record.branchHint) ? record.branchHint : null,
    taskId: isNonEmptyString(record.taskId) ? record.taskId : null,
    pullRequest: isNonEmptyString(record.pullRequestHint) ? record.pullRequestHint : null,
    messageCount: isTurnCount(record.messageCount) ? record.messageCount : null,
    latestTurnPreview: isNonEmptyString(record.latestTurnPreview) ? record.latestTurnPreview : null,
    lastActivity: isNonEmptyString(record.lastActivity) ? record.lastActivity : null,
  };
}

export function createFreshSession(
  opts: { cwd: string; title?: string },
  mintId: () => string = defaultMintId
): OwnedSession {
  const segments = opts.cwd.split('/').filter((segment) => segment.length > 0);
  const defaultTitle = segments.length > 0 ? segments[segments.length - 1] : opts.cwd;
  return {
    ownedId: mintId(),
    agent: 'other',
    viaCmux: false,
    source: 'fresh',
    title: opts.title ?? defaultTitle,
    projectPath: null,
    cwd: opts.cwd,
    resumeCommand: null,
    nativeSessionId: null,
    ptySessionId: null,
    state: 'background',
    completedAt: null,
    branch: null,
    taskId: null,
    pullRequest: null,
    messageCount: null,
    latestTurnPreview: null,
    // Deliberately not "now": this field is what the scanner read out of a
    // conversation on disk, and a shell started here has no conversation yet.
    // Its row simply shows no stamp, like every other scanner field above.
    lastActivity: null,
  };
}

export function serializeOwnedSessions(sessions: OwnedSession[]): string {
  return JSON.stringify(sessions);
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0;
}

/**
 * A count of turns worth showing: a whole number, at least one. Zero is nothing
 * to say rather than something to print, so a row shows no count at all instead
 * of "0 messages".
 */
function isTurnCount(value: unknown): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value > 0;
}

export function parseStoredOwnedSessions(raw: string | null): OwnedSession[] {
  if (!raw) {
    return [];
  }
  let data: unknown;
  try {
    data = JSON.parse(raw);
  } catch {
    return [];
  }
  if (!Array.isArray(data)) {
    return [];
  }
  const result: OwnedSession[] = [];
  for (const entry of data) {
    if (typeof entry !== 'object' || entry === null) {
      continue;
    }
    const candidate = entry as Record<string, unknown>;
    if (!isNonEmptyString(candidate.ownedId) || !isNonEmptyString(candidate.cwd)) {
      continue;
    }
    const agent = (KNOWN_AGENTS as string[]).includes(candidate.agent as string)
      ? (candidate.agent as AgentKind)
      : 'other';
    const state = (KNOWN_STATES as string[]).includes(candidate.state as string)
      ? (candidate.state as OwnedSessionState)
      : 'exited';
    result.push({
      ownedId: candidate.ownedId,
      agent,
      viaCmux: candidate.viaCmux === true,
      source: candidate.source === 'fresh' ? 'fresh' : 'scanned',
      title: isNonEmptyString(candidate.title) ? candidate.title : '',
      projectPath: isNonEmptyString(candidate.projectPath) ? candidate.projectPath : null,
      cwd: candidate.cwd,
      resumeCommand: isNonEmptyString(candidate.resumeCommand) ? candidate.resumeCommand : null,
      nativeSessionId: isNonEmptyString(candidate.nativeSessionId) ? candidate.nativeSessionId : null,
      ptySessionId: isNonEmptyString(candidate.ptySessionId) ? candidate.ptySessionId : null,
      state,
      // Sessions saved before this field existed have no value here at all, and
      // that reads exactly like "not done" — which is the right answer for them.
      completedAt: isNonEmptyString(candidate.completedAt) ? candidate.completedAt : null,
      // Same story here: a session saved before the scanner sent these, or one
      // the scanner had nothing to say about, simply shows no chips.
      branch: isNonEmptyString(candidate.branch) ? candidate.branch : null,
      taskId: isNonEmptyString(candidate.taskId) ? candidate.taskId : null,
      pullRequest: isNonEmptyString(candidate.pullRequest) ? candidate.pullRequest : null,
      messageCount: isTurnCount(candidate.messageCount) ? candidate.messageCount : null,
      latestTurnPreview: isNonEmptyString(candidate.latestTurnPreview)
        ? candidate.latestTurnPreview
        : null,
      lastActivity: isNonEmptyString(candidate.lastActivity) ? candidate.lastActivity : null,
    });
  }
  return result;
}

export function reconcileOwnedSessions(
  stored: OwnedSession[],
  live: Pick<TerminalSessionInfo, 'sessionId' | 'exited'>[]
): { owned: OwnedSession[]; reattachable: OwnedSession[] } {
  const liveById = new Map(live.map((entry) => [entry.sessionId, entry]));
  const owned: OwnedSession[] = [];
  const reattachable: OwnedSession[] = [];

  for (const session of stored) {
    const match = session.ptySessionId ? liveById.get(session.ptySessionId) : undefined;
    if (match && !match.exited) {
      const next: OwnedSession = { ...session, state: 'background' };
      owned.push(next);
      reattachable.push(next);
    } else if (match && match.exited) {
      owned.push({ ...session, state: 'exited' });
    } else {
      owned.push({ ...session, state: 'exited', ptySessionId: null });
    }
  }

  return { owned, reattachable };
}
