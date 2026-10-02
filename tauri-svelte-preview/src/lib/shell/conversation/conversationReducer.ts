import { StreamProcessor } from '@tanstack/ai/client';
import { applyMessageEvent, displayEventFrom, finishMessageReasoning } from './conversationMessages.ts';
import type {
  AgentEvent,
  AgentConversationEvent,
  AgentConversationProvider,
  ConversationSessionState
} from './conversationTypes.ts';

/** Below this the window is not full enough for a fall to mean a compaction;
 * agents report small numbers for other reasons. */
const COMPACTION_FLOOR_TOKENS = 32_000;
/** How much of the old total has to survive for the fall to be ordinary. */
const COMPACTION_DROP_RATIO = 0.6;

/**
 * Whether a reported occupancy falling from `previous` to `used` can only be a
 * compaction. Claude says nothing when it throws the older part of a
 * conversation away — the number dropping off a cliff is the only sign there
 * is. A modest decline is ordinary, an agent reporting its last request rather
 * than the whole session, and means nothing.
 */
export function usageDropIsCompaction(
  previous: number | undefined,
  used: number | undefined
): boolean {
  return previous !== undefined
    && used !== undefined
    && previous >= COMPACTION_FLOOR_TOKENS
    && used <= previous * COMPACTION_DROP_RATIO;
}

export function shouldClearConversationSending(
  event: AgentConversationEvent | AgentEvent
): boolean {
  if ('type' in event) {
    return ['turn.completed', 'turn.interrupted', 'runtime.error'].includes(event.type);
  }
  return event.payload.kind === 'error'
    || (event.payload.kind === 'turn' && event.payload.state !== 'started');
}

export function createConversationState(
  ownedId: string,
  provider: AgentConversationProvider
): ConversationSessionState {
  return {
    ownedId,
    provider,
    generation: 0,
    lastSequence: 0,
    desynchronized: false,
    connectionState: 'disconnected',
    suspended: false,
    timelineRevision: 0,
    transcript: new StreamProcessor()
  };
}

/** One event's sequence, connection, turn and usage bookkeeping, without the
 * transcript. A dropped event returns `state` itself. */
export function reduceConversationEvent(
  state: ConversationSessionState,
  event: AgentConversationEvent
): ConversationSessionState {
  if (event.ownedId !== state.ownedId || event.provider !== state.provider || event.generation < state.generation) return state;
  const newGeneration = event.generation > state.generation;
  const previousSequence = newGeneration ? 0 : state.lastSequence;
  if (!newGeneration && event.sequence <= previousSequence) return state;
  const gap = event.sequence !== previousSequence + 1;
  const next: ConversationSessionState = {
    ownedId: state.ownedId, provider: state.provider,
    generation: event.generation, lastSequence: event.sequence,
    desynchronized: newGeneration ? gap : state.desynchronized || gap,
    connectionState: state.connectionState, suspended: state.suspended,
    timelineRevision: state.timelineRevision, nativeSessionId: state.nativeSessionId,
    activeTurnId: state.activeTurnId, usage: state.usage, transcript: state.transcript
  };
  if (next.desynchronized) return next;
  const payload = event.payload;
  if (payload.kind === 'connection') {
    next.connectionState = payload.state;
    next.nativeSessionId = payload.nativeSessionId ?? next.nativeSessionId;
  } else if (payload.kind === 'turn') {
    next.activeTurnId = payload.state === 'started' ? payload.turnId : undefined;
  } else if (payload.kind === 'error') {
    next.activeTurnId = undefined;
    if (!payload.recoverable) next.connectionState = 'failed';
  } else if (payload.kind === 'usage') {
    next.usage = {
      inputTokens: payload.inputTokens ?? next.usage?.inputTokens,
      outputTokens: payload.outputTokens ?? next.usage?.outputTokens,
      usedTokens: payload.usedTokens ?? next.usage?.usedTokens,
      contextWindow: payload.contextWindow ?? next.usage?.contextWindow,
      totalTokens: payload.totalTokens ?? next.usage?.totalTokens
    };
  }
  return next;
}

export function applyConversationEvent(
  state: ConversationSessionState,
  event: AgentConversationEvent
): ConversationSessionState {
  const next = reduceConversationEvent(state, event);
  if (next === state || next.desynchronized) return next;
  const payload = event.payload;
  if (payload.kind === 'usage') {
    const previous = state.usage?.usedTokens;
    const last = next.transcript.getMessages().at(-1);
    if (usageDropIsCompaction(previous, payload.usedTokens) && last?.metadata?.itemType !== 'context-compaction') {
      applyMessageEvent(next.transcript, { ...event, payload: { kind: 'contextCompaction', preTokens: previous, postTokens: payload.usedTokens } });
      next.timelineRevision += 1;
    }
  }
  const normalized = displayEventFrom(event);
  if (applyMessageEvent(next.transcript, normalized)) next.timelineRevision += 1;
  if (finishMessageReasoning(next.transcript, normalized)) next.timelineRevision += 1;
  return next;
}
