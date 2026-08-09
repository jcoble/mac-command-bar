import type {
  AgentConversationProvider,
  ConversationConnectionState
} from './conversationTypes.ts';
import type { AgentExecutionOwner } from '../ownedSessions.ts';

export type ConversationActivationDecision =
  | { kind: 'view' }
  | { kind: 'terminal' }
  | { kind: 'structured'; nativeSessionMode: 'resume' | 'load' };

interface ConversationActivationSession {
  agent: string;
  origin?: 'app' | 'external';
  state: 'live' | 'background' | 'exited';
  executionOwner?: AgentExecutionOwner;
  ptySessionId: string | null;
  nativeSessionId: string | null;
}

interface ConversationConnection {
  provider: AgentConversationProvider;
  state?: ConversationConnectionState;
  connectionState?: ConversationConnectionState;
  nativeSessionId?: string | null;
}

function connectedSessionMatches(
  session: ConversationActivationSession,
  connection: ConversationConnection | null
): boolean {
  const state = connection?.state ?? connection?.connectionState;
  if (!connection || state !== 'connected' || connection.provider !== session.agent) {
    return false;
  }
  return !session.nativeSessionId || connection.nativeSessionId === session.nativeSessionId;
}

/**
 * Decide whether a rail selection is view-only, terminal-owned, or needs ACP.
 * Running external sessions are checked first: an old structured connection
 * never overrides their live CLI writer. A stopped external Codex session may
 * use session/load only when its native thread id is known.
 */
export function decideConversationActivation(
  session: ConversationActivationSession,
  connection: ConversationConnection | null
): ConversationActivationDecision {
  if (session.origin !== 'app') {
    const terminalOwned = session.executionOwner === 'terminal'
      || session.state === 'live'
      || Boolean(session.ptySessionId);
    const stopped = session.executionOwner === 'stopped' || session.state === 'exited';
    if (terminalOwned || !stopped) return { kind: 'terminal' };
    if (session.agent !== 'codex' || !session.nativeSessionId) return { kind: 'terminal' };
    if (connectedSessionMatches(session, connection)) return { kind: 'view' };
    return { kind: 'structured', nativeSessionMode: 'load' };
  }

  if (connectedSessionMatches(session, connection)) return { kind: 'view' };
  return { kind: 'structured', nativeSessionMode: 'resume' };
}
