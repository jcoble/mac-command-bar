/**
 * Owned-session identity and lifecycle records for the /next shell.
 *
 * ownedId (CommandBar-minted) is the PRIMARY key across rail, manager,
 * persistence, and active-tracking. Provider ids are mutable (a resume mints a
 * new one), so they are carried but never used as keys.
 */
import type {
  AgentConversationSessionMeta,
  AgentConversationSessionRecord,
  AgentSession,
  ExecutionEnvironment,
  TerminalSessionInfo
} from '../tauriSource';

export type AgentKind = 'codex' | 'claude' | 'antigravity' | 'gemini' | 'opencode' | 'other';
export type OwnedSessionState = 'live' | 'background' | 'exited';

export type AgentExecutionOwner =
  | 'structured'
  | 'terminal'
  | 'transitioning-to-structured'
  | 'transitioning-to-terminal'
  | 'stopped';

export type AgentRuntimeState =
  | 'starting'
  | 'ready'
  | 'working'
  | 'waiting-approval'
  | 'waiting-input'
  | 'interrupting'
  | 'suspended'
  | 'failed'
  | 'closed';

export interface OwnedAgentRuntimeFields {
  executionOwner: AgentExecutionOwner;
  runtimeState: AgentRuntimeState;
  providerInstanceId: string | null;
  nativeSessionId: string | null;
  activeTurnId: string | null;
  pendingPermission?: boolean;
  pendingInput?: boolean;
  capabilityRevision: number;
  lastRuntimeError: string | null;
}

export type OwnedSession = Omit<Partial<OwnedAgentRuntimeFields>, 'nativeSessionId'> & {
  ownedId: string;
  executionEnvironment: ExecutionEnvironment;
  remoteProfileId?: string | null;
  agent: AgentKind;
  /** App-created agent sessions use ACP; adopted sessions stay PTY projections. */
  origin?: 'app' | 'external';
  viaCmux: boolean;
  source: 'scanned' | 'fresh';
  title: string;
  /** Model reported by the session scanner, when one was available. */
  model?: string | null;
  projectPath: string | null;
  cwd: string;
  resumeCommand: string | null;
  nativeSessionId: string | null;
  ptySessionId: string | null;
  state: OwnedSessionState;
  /** The most recent app-owned ACP start failure, retained for an in-rail retry. */
  lastError?: string | null;
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
   * When the user archived this session out of the Done list into Settled, as
   * an ISO stamp; `null` while it still belongs in Working or Done. Written
   * only by the user's explicit "Settle"/"Unsettle" actions — never inferred
   * from age, title, or process state. Old records without the field load as
   * `null`.
   */
  settledAt: string | null;
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
  /**
   * When this session was started, in milliseconds since the epoch, as the
   * stored record reports it. The rail shows the row's age from it, which is
   * why it survives a turn ending. `null` for a session the app has not
   * persisted yet, and for one adopted off disk where nothing recorded a start.
   */
  startedAtMs?: number | null;
};

export interface OwnedSessionProject {
  /** The best path we have for this session, used by detail views. */
  path: string;
  /** A compact label that is always useful in the rail. */
  label: string;
}

const KNOWN_AGENTS: AgentKind[] = ['codex', 'claude', 'antigravity', 'gemini', 'opencode'];
const KNOWN_STATES: OwnedSessionState[] = ['live', 'background', 'exited'];
const KNOWN_EXECUTION_OWNERS: AgentExecutionOwner[] = [
  'structured',
  'terminal',
  'transitioning-to-structured',
  'transitioning-to-terminal',
  'stopped'
];
const KNOWN_RUNTIME_STATES: AgentRuntimeState[] = [
  'starting',
  'ready',
  'working',
  'waiting-approval',
  'waiting-input',
  'interrupting',
  'suspended',
  'failed',
  'closed'
];

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

function cleanSessionPath(value: string | null | undefined): string {
  const cleaned = (value ?? '').trim().replaceAll('\\', '/').replace(/\/+$/, '');
  const normalized = cleaned.toLowerCase();
  return normalized === 'no project recorded' || normalized === 'no project' ? '' : cleaned;
}

function folderName(value: string): string {
  const parts = value.split('/').filter(Boolean);
  return parts.at(-1) ?? '';
}

function providerFallback(agent: AgentKind | null | undefined, viaCmux: boolean): string {
  const name = (agent ?? '').trim();
  if (!name) return 'Session';
  return viaCmux ? `${name} session` : name;
}

/**
 * Resolve the project identity once, at the session-record boundary.
 *
 * The scanner's recorded project wins. If it has no project, a worktree/cwd
 * folder is still more useful than a placeholder. A provider label is the
 * final truthful fallback when neither path exists.
 */
export function resolveOwnedSessionProject(
  session: Pick<OwnedSession, 'projectPath' | 'cwd'> &
    Partial<Pick<OwnedSession, 'agent' | 'viaCmux'>>
): OwnedSessionProject {
  const recorded = cleanSessionPath(session.projectPath);
  const worktree = cleanSessionPath(session.cwd);
  const path = recorded || worktree;
  const label = folderName(path) || providerFallback(session.agent, session.viaCmux === true);
  return { path, label };
}

export function adoptAgentSession(record: AgentSession, mintId: () => string = defaultMintId): OwnedSession {
  const { agent, viaCmux } = normalizeProvider(record.provider);
  return {
    ownedId: mintId(),
    executionEnvironment: 'local',
    remoteProfileId: null,
    agent,
    origin: 'external',
    viaCmux,
    source: 'scanned',
    title: record.title,
    model: isNonEmptyString(record.model) ? record.model : null,
    projectPath: record.projectPath,
    cwd: record.projectPath ?? '',
    resumeCommand: record.resumeCommands[0] ?? null,
    nativeSessionId: record.id,
    ptySessionId: null,
    state: 'background',
    lastError: null,
    executionOwner: 'stopped',
    runtimeState: 'closed',
    providerInstanceId: null,
    activeTurnId: null,
    capabilityRevision: 0,
    lastRuntimeError: null,
    completedAt: null,
    settledAt: null,
    branch: isNonEmptyString(record.branchHint) ? record.branchHint : null,
    taskId: isNonEmptyString(record.taskId) ? record.taskId : null,
    pullRequest: isNonEmptyString(record.pullRequestHint) ? record.pullRequestHint : null,
    messageCount: isTurnCount(record.messageCount) ? record.messageCount : null,
    latestTurnPreview: isNonEmptyString(record.latestTurnPreview) ? record.latestTurnPreview : null,
    lastActivity: isNonEmptyString(record.lastActivity) ? record.lastActivity : null,
  };
}

export function createFreshSession(
  opts: { cwd: string; title?: string; executionEnvironment?: ExecutionEnvironment; remoteProfileId?: string | null },
  mintId: () => string = defaultMintId
): OwnedSession {
  const segments = opts.cwd.split('/').filter((segment) => segment.length > 0);
  const defaultTitle = segments.length > 0 ? segments[segments.length - 1] : opts.cwd;
  return {
    ownedId: mintId(),
    executionEnvironment: opts.executionEnvironment ?? 'local',
    remoteProfileId: opts.remoteProfileId ?? null,
    agent: 'other',
    origin: 'external',
    viaCmux: false,
    source: 'fresh',
    title: opts.title ?? defaultTitle,
    model: null,
    projectPath: null,
    cwd: opts.cwd,
    resumeCommand: null,
    nativeSessionId: null,
    ptySessionId: null,
    state: 'background',
    lastError: null,
    executionOwner: 'stopped',
    runtimeState: 'closed',
    providerInstanceId: null,
    activeTurnId: null,
    capabilityRevision: 0,
    lastRuntimeError: null,
    completedAt: null,
    settledAt: null,
    branch: null,
    taskId: null,
    pullRequest: null,
    messageCount: null,
    latestTurnPreview: null,
    lastActivity: new Date().toISOString(),
  };
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0;
}

export function ownedSessionFromBackend(record: AgentConversationSessionRecord): OwnedSession {
  const { agent, viaCmux: providerViaCmux } = normalizeProvider(record.provider);
  const terminalOwned = record.origin === 'external' && record.ptySessionId !== null;
  return {
    ownedId: record.ownedId,
    executionEnvironment: record.executionEnvironment,
    remoteProfileId: record.remoteProfileId,
    agent,
    origin: record.origin ?? 'app',
    viaCmux: record.viaCmux || providerViaCmux,
    source: record.source ?? 'fresh',
    title: record.title ?? '',
    model: record.model,
    projectPath: record.project,
    cwd: record.cwd,
    resumeCommand: record.resumeCommand,
    nativeSessionId: record.nativeSessionId,
    ptySessionId: record.ptySessionId,
    state: record.activeTurnId === null ? 'background' : 'live',
    lastError: null,
    executionOwner: terminalOwned ? 'terminal' : 'structured',
    runtimeState: record.suspended ? 'suspended' : record.state,
    providerInstanceId: null,
    activeTurnId: record.activeTurnId,
    pendingPermission: record.pendingPermission,
    pendingInput: record.pendingInput,
    capabilityRevision: 0,
    lastRuntimeError: null,
    completedAt: record.completedAt,
    settledAt: record.settledAt,
    // The rail prints the branch straight into the row, so anything that is not
    // a name has to stop here rather than reach the row as "[object Object]".
    // The two constructors below already read it this way.
    branch: isNonEmptyString(record.branch) ? record.branch : null,
    taskId: record.taskId,
    pullRequest: record.pullRequest,
    messageCount: record.messageCount,
    latestTurnPreview: record.latestTurnPreview,
    lastActivity: record.scannedLastActivity,
    startedAtMs: Number.isFinite(record.createdAtMs) ? record.createdAtMs : null
  };
}

export function ownedSessionMetaForBackend(session: OwnedSession): AgentConversationSessionMeta {
  return {
    worktree: session.cwd || null,
    branch: session.branch,
    title: session.title || null,
    project: session.projectPath,
    ptySessionId: session.ptySessionId,
    origin: session.origin ?? null,
    source: session.source,
    viaCmux: session.viaCmux,
    resumeCommand: session.resumeCommand,
    completedAt: session.completedAt,
    settledAt: session.settledAt,
    taskId: session.taskId,
    pullRequest: session.pullRequest,
    messageCount: session.messageCount,
    latestTurnPreview: session.latestTurnPreview,
    scannedLastActivity: session.lastActivity
  };
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
    const origin = candidate.origin === 'app' ? 'app' : 'external';
    const ptySessionId = isNonEmptyString(candidate.ptySessionId) ? candidate.ptySessionId : null;
    const executionOwner = (KNOWN_EXECUTION_OWNERS as string[]).includes(
      candidate.executionOwner as string
    )
      ? (candidate.executionOwner as AgentExecutionOwner)
      : ptySessionId && state !== 'exited'
        ? 'terminal'
        : 'stopped';
    const runtimeState = (KNOWN_RUNTIME_STATES as string[]).includes(candidate.runtimeState as string)
      ? (candidate.runtimeState as AgentRuntimeState)
      : executionOwner === 'terminal'
        ? 'ready'
        : 'closed';
    result.push({
      ownedId: candidate.ownedId,
      executionEnvironment:
        candidate.executionEnvironment === 'remote' ? 'remote' : 'local',
      remoteProfileId: isNonEmptyString(candidate.remoteProfileId) ? candidate.remoteProfileId : null,
      agent,
      origin,
      viaCmux: candidate.viaCmux === true,
      source: candidate.source === 'fresh' ? 'fresh' : 'scanned',
      title: isNonEmptyString(candidate.title) ? candidate.title : '',
      model: isNonEmptyString(candidate.model) ? candidate.model : null,
      projectPath: isNonEmptyString(candidate.projectPath) ? candidate.projectPath : null,
      cwd: candidate.cwd,
      resumeCommand: isNonEmptyString(candidate.resumeCommand) ? candidate.resumeCommand : null,
      nativeSessionId: isNonEmptyString(candidate.nativeSessionId) ? candidate.nativeSessionId : null,
      ptySessionId,
      state,
      lastError: isNonEmptyString(candidate.lastError) ? candidate.lastError : null,
      executionOwner,
      runtimeState,
      providerInstanceId: isNonEmptyString(candidate.providerInstanceId)
        ? candidate.providerInstanceId
        : null,
      activeTurnId: isNonEmptyString(candidate.activeTurnId) ? candidate.activeTurnId : null,
      capabilityRevision: nonNegativeInteger(candidate.capabilityRevision) ?? 0,
      lastRuntimeError: isNonEmptyString(candidate.lastRuntimeError)
        ? candidate.lastRuntimeError
        : null,
      // Sessions saved before this field existed have no value here at all, and
      // that reads exactly like "not done" — which is the right answer for them.
      completedAt: isNonEmptyString(candidate.completedAt) ? candidate.completedAt : null,
      // Same "not there means not settled" reading for records saved before
      // the Settled shelf existed.
      settledAt: isNonEmptyString(candidate.settledAt) ? candidate.settledAt : null,
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

function nonNegativeInteger(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isInteger(value) && value >= 0 ? value : undefined;
}

export function reconcileOwnedSessions(
  stored: OwnedSession[],
  live: Pick<TerminalSessionInfo, 'sessionId' | 'exited'>[]
): { owned: OwnedSession[]; reattachable: OwnedSession[] } {
  const liveById = new Map(live.map((entry) => [entry.sessionId, entry]));
  const owned: OwnedSession[] = [];
  const reattachable: OwnedSession[] = [];

  for (const session of stored) {
    // Structured sessions do not own a PTY. A reload must leave their native
    // record resumable instead of treating the absent terminal as proof that
    // the session stopped; otherwise the row paints the stopped affordance and
    // a later restart keeps the stale stopped owner.
    if (session.ptySessionId === null) {
      const failed = session.state === 'exited' && (session.runtimeState === 'failed' || session.lastError !== null);
      if (failed) {
        owned.push({ ...session, state: 'exited', executionOwner: 'stopped', runtimeState: 'failed' });
      } else {
        owned.push({ ...session, state: 'background', executionOwner: 'structured', runtimeState: 'closed' });
      }
      continue;
    }

    const match = session.ptySessionId ? liveById.get(session.ptySessionId) : undefined;
    if (match && !match.exited) {
      const next: OwnedSession = {
        ...session,
        state: 'background',
        executionOwner: 'terminal',
        runtimeState: 'ready'
      };
      owned.push(next);
      reattachable.push(next);
    } else if (match && match.exited) {
      owned.push({ ...session, state: 'exited', executionOwner: 'stopped', runtimeState: 'closed' });
    } else {
      // Keep the terminal id so a failed migration can be retried or diagnosed.
      owned.push({ ...session, state: 'exited', executionOwner: 'stopped', runtimeState: 'closed' });
    }
  }

  return { owned, reattachable };
}
