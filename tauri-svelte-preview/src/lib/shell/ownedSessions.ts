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
  };
}

export function serializeOwnedSessions(sessions: OwnedSession[]): string {
  return JSON.stringify(sessions);
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0;
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
