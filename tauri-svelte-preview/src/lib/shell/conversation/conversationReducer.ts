import type {
  AgentEvent,
  AgentConversationEvent,
  AgentConversationProvider,
  ConversationSessionState,
  ConversationTimelineEntry
} from './conversationTypes.ts';

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
    timeline: []
  };
}

function replaceOrAppend(
  timeline: ConversationTimelineEntry[],
  itemId: string,
  create: (current: ConversationTimelineEntry | undefined) => ConversationTimelineEntry
): ConversationTimelineEntry[] {
  const index = timeline.findIndex((entry) => entry.itemId === itemId);
  if (index < 0) return [...timeline, create(undefined)];
  const next = timeline.slice();
  next[index] = create(timeline[index]);
  return next;
}

export function importConversationHistory(
  state: ConversationSessionState,
  history: readonly ConversationTimelineEntry[]
): ConversationSessionState {
  if (history.length === 0) return state;

  const existingIds = new Set(state.timeline.map((entry) => entry.itemId));
  const additions = history.filter((entry) => !existingIds.has(entry.itemId));
  if (additions.length === 0) return state;
  return { ...state, timeline: [...state.timeline, ...additions] };
}

export function applyConversationEvent(
  state: ConversationSessionState,
  event: AgentConversationEvent
): ConversationSessionState {
  if (event.ownedId !== state.ownedId || event.provider !== state.provider) return state;
  if (event.generation < state.generation) return state;

  const isNewGeneration = event.generation > state.generation;
  const priorSequence = isNewGeneration ? 0 : state.lastSequence;
  if (!isNewGeneration && event.sequence <= priorSequence) return state;

  const hasGap = event.sequence !== priorSequence + 1;
  let next: ConversationSessionState = {
    ...state,
    generation: event.generation,
    lastSequence: event.sequence,
    desynchronized: isNewGeneration ? hasGap : state.desynchronized || hasGap
  };

  // Applying a delta after a missing event would fabricate a transcript. Keep
  // the acknowledged position so duplicates remain rejected, preserve every
  // valid item already rendered, and let the service request a fresh snapshot.
  if (hasGap) return next;

  const { payload } = event;
  switch (payload.kind) {
    case 'connection':
      return {
        ...next,
        connectionState: payload.state,
        nativeSessionId: payload.nativeSessionId ?? next.nativeSessionId
      };

    case 'userMessage':
      return {
        ...next,
        timeline: replaceOrAppend(next.timeline, payload.itemId, () => ({
          kind: 'user',
          itemId: payload.itemId,
          text: payload.text,
          completed: payload.completed,
          timestampMs: event.timestampMs
        }))
      };

    case 'assistantDelta':
      return {
        ...next,
        timeline: replaceOrAppend(next.timeline, payload.itemId, (current) => ({
          kind: 'assistant',
          itemId: payload.itemId,
          text: current?.kind === 'assistant' ? `${current.text}${payload.delta}` : payload.delta,
          completed: false,
          timestampMs: current?.timestampMs ?? event.timestampMs
        }))
      };

    case 'assistantMessage':
      return {
        ...next,
        timeline: replaceOrAppend(next.timeline, payload.itemId, (current) => ({
          kind: 'assistant',
          itemId: payload.itemId,
          text: payload.text,
          completed: true,
          timestampMs: current?.timestampMs ?? event.timestampMs
        }))
      };

    case 'tool':
      return {
        ...next,
        timeline: replaceOrAppend(next.timeline, payload.itemId, () => ({
          kind: 'tool',
          itemId: payload.itemId,
          name: payload.name,
          state: payload.state,
          summary: payload.summary,
          timestampMs: event.timestampMs
        }))
      };

    case 'approval': {
      const itemId = `approval:${payload.requestId}`;
      return {
        ...next,
        timeline: replaceOrAppend(next.timeline, itemId, (current) => ({
          kind: 'approval',
          itemId,
          requestId: payload.requestId,
          state: payload.state,
          summary: payload.summary,
          timestampMs: current?.timestampMs ?? event.timestampMs
        }))
      };
    }

    case 'plan': {
      const itemId = 'plan:' + event.generation;
      const items = payload.items ?? (payload.entries ?? []).map((entry) => ({
        text: entry.title ?? entry.text ?? entry.content ?? '',
        status: entry.status ?? 'pending'
      }));
      return {
        ...next,
        timeline: replaceOrAppend(next.timeline, itemId, (current) => ({
          kind: 'plan',
          itemId,
          items,
          timestampMs: current?.timestampMs ?? event.timestampMs
        }))
      };
    }

    case 'turn': {
      const itemId = `turn:${payload.turnId}`;
      return {
        ...next,
        activeTurnId: payload.state === 'started' ? payload.turnId : undefined,
        timeline: replaceOrAppend(next.timeline, itemId, (current) => ({
          kind: 'turn',
          itemId,
          turnId: payload.turnId,
          state: payload.state,
          timestampMs: current?.timestampMs ?? event.timestampMs
        }))
      };
    }

    case 'usage':
      return {
        ...next,
        usage: { inputTokens: payload.inputTokens, outputTokens: payload.outputTokens }
      };

    case 'error':
      return {
        ...next,
        activeTurnId: undefined,
        connectionState: payload.recoverable ? next.connectionState : 'failed',
        timeline: [
          ...next.timeline,
          {
            kind: 'error',
            itemId: `error:${event.generation}:${event.sequence}`,
            code: payload.code,
            message: payload.message,
            recoverable: payload.recoverable,
            timestampMs: event.timestampMs
          }
        ]
      };

    // Rich ACP bridge items are projected by conversationTimeline.ts. The
    // reducer still advances the generation/sequence checkpoint so live and
    // replayed streams share the same ordering and resync behavior.
    case 'agentThoughtChunk':
    case 'toolCall':
    case 'toolCallUpdate':
    case 'turnDiff':
    case 'permissionRequest':
    case 'availableCommandsUpdate':
    case 'agentMessageChunk':
    case 'userMessageChunk':
      return next;
  }
}
