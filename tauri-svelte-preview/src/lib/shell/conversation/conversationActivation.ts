import type {
  AgentConversationProvider,
  ConversationConnectionState
} from './conversationTypes.ts';
import type { AgentExecutionOwner, OwnedSessionState } from '../ownedSessions.ts';

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

export interface ConversationSendState {
  sessionState?: OwnedSessionState;
  executionOwner?: AgentExecutionOwner;
  connectionState?: ConversationConnectionState | null;
  generation: number;
}

const DEAD_CONNECTION_STATES: readonly ConversationConnectionState[] = [
  'disconnected',
  'failed',
  'closed'
];

/** A send must not be attempted against a stopped rail record or dead conversation connection. */
export function shouldReviveBeforeSend(state: ConversationSendState): boolean {
  return state.sessionState === 'exited'
    || state.executionOwner === 'stopped'
    || state.generation < 1
    || state.connectionState == null
    || DEAD_CONNECTION_STATES.includes(state.connectionState);
}

/** Select a usable activation generation that is not older than the pre-ensure state. */
export function generationForSend(previousGeneration: number, activationGeneration: number): number | null {
  if (!Number.isInteger(previousGeneration) || !Number.isInteger(activationGeneration)) return null;
  if (activationGeneration < 1 || activationGeneration < previousGeneration) return null;
  return activationGeneration;
}

/**
 * Validate the connection returned by ensure immediately before a send.
 *
 * The current state must still describe that same connected generation. A
 * later activation is therefore rejected rather than sending with a stale
 * generation, while an idempotent ensure remains valid.
 */
export function validateStructuredSendGeneration(
  previousGeneration: number,
  activation: { generation: number; state?: ConversationConnectionState; connectionState?: ConversationConnectionState } | null | undefined,
  current: Pick<ConversationSendState, 'connectionState' | 'generation'> | null | undefined
): number | null {
  const activatedState = activation?.state ?? activation?.connectionState;
  const nextGeneration = generationForSend(previousGeneration, activation?.generation ?? -1);
  if (nextGeneration === null || activatedState !== 'connected') return null;
  if (!current || current.connectionState !== 'connected' || current.generation !== nextGeneration) return null;
  return nextGeneration;
}

function connectedSessionMatches(
  session: ConversationActivationSession,
  connection: ConversationConnection | null
): boolean {
  const stopped = session.executionOwner === 'stopped' || session.state === 'exited';
  const state = connection?.state ?? connection?.connectionState;
  if (stopped || !connection || state !== 'connected' || connection.provider !== session.agent) {
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
    return { kind: 'structured', nativeSessionMode: 'load' };
  }

  if (connectedSessionMatches(session, connection)) return { kind: 'view' };
  return { kind: 'structured', nativeSessionMode: 'resume' };
}
