import { EventType, StreamProcessor, type StreamChunk, type UIMessage } from '@tanstack/ai/client';
import type { AgentConfigValue, AgentEvent, AgentConversationEvent, AgentItem, ConversationAttachment } from './conversationTypes.ts';
import { agentItemFromEvent, conversationEventAppendsItemContent, displayItemFromAgentItem, displayItemFromApproval, permissionRequestFromEvent, reuseConversationDisplayItems, withoutRepeatedPlans, type ConversationDisplayItem } from './conversationTimeline.ts';
import { usageDropIsCompaction } from './conversationReducer.ts';

/** Unwrap stored provider projections before the one live/replay mapper. */
export function displayEventFrom(event: AgentConversationEvent): AgentConversationEvent | AgentEvent {
  if (event.payload.kind !== 'terminalProjection') return event;
  return {
    type: event.payload.eventType, ownedId: event.ownedId, provider: event.provider,
    providerInstanceId: event.payload.providerInstanceId, generation: event.generation,
    sequence: event.sequence, timestampMs: event.payload.timestampMs ?? event.timestampMs,
    nativeSessionId: event.payload.nativeSessionId, itemId: event.payload.itemId ?? undefined,
    payload: event.payload.payload, providerMetadata: event.payload.providerMetadata,
    rawFrameReference: event.payload.rawFrameReference
  };
}

function metadataOf(message: UIMessage | undefined): Record<string, AgentConfigValue> {
  return (message?.metadata ?? {}) as Record<string, AgentConfigValue>;
}

function textOf(message: UIMessage | undefined): string {
  return message?.parts.map((part) => part.type === 'text' ? part.content : part.type === 'thinking' ? part.content : '').join('') ?? '';
}

export function conversationMessagesFromEvents(
  events: readonly AgentConversationEvent[]
): UIMessage[] {
  const processor = new StreamProcessor();
  let usedTokens: number | undefined;
  for (const event of events) {
    if (event.payload.kind === 'usage') {
      if (usageDropIsCompaction(usedTokens, event.payload.usedTokens)) {
        applyMessageEvent(processor, {
          ...event,
          payload: {
            kind: 'contextCompaction',
            preTokens: usedTokens,
            postTokens: event.payload.usedTokens
          }
        });
      }
      usedTokens = event.payload.usedTokens ?? usedTokens;
    }
    applyMessageEvent(processor, displayEventFrom(event));
  }
  return processor.getMessages();
}

export function conversationMessageDisplayItem(message: UIMessage): ConversationDisplayItem {
  const metadata = metadataOf(message);
  const direct = metadata.displayItem;
  if (direct && typeof direct === 'object') return direct as unknown as ConversationDisplayItem;
  const call = message.parts.find((part) => part.type === 'tool-call');
  const result = message.parts.find((part) => part.type === 'tool-result' && part.toolCallId === call?.id);
  const output = call?.output && typeof call.output === 'object'
    ? call.output as Record<string, AgentConfigValue>
    : result?.type === 'tool-result' && typeof result.content === 'string'
      ? JSON.parse(result.content) as Record<string, AgentConfigValue> : undefined;
  const facts = output ?? metadata;
  return displayItemFromAgentItem({
    id: message.id,
    type: metadata.itemType as AgentItem['type'],
    turnId: typeof metadata.turnId === 'string' ? metadata.turnId : undefined,
    content: [{ channel: metadata.itemType === 'reasoning' ? 'reasoning' : 'assistant', text: output ? String(output.output ?? '') : textOf(message) }],
    providerMetadata: { ...facts, startedAtMs: metadata.startedAtMs }
  }, Number(metadata.startedAtMs ?? 0));
}

export function conversationDisplayItems(
  messages: readonly UIMessage[],
  previous: readonly ConversationDisplayItem[] = [],
  sentAttachments: Readonly<Record<string, readonly ConversationAttachment[]>> = {}
): ConversationDisplayItem[] {
  const rows = messages.map((message) => {
    const item = conversationMessageDisplayItem(message);
    // A message still on its way carries its own screenshots until the backend's copy claims them.
    const attachments = item.kind === 'user'
      ? sentAttachments[item.itemId] ?? message.metadata?.attachments as readonly ConversationAttachment[] | undefined
      : undefined;
    return attachments?.length ? { ...item, attachments } : item;
  }).sort((a, b) => a.timestampMs - b.timestampMs);
  return reuseConversationDisplayItems(withoutRepeatedPlans(rows), previous);
}

export function conversationHasRunningTool(messages: readonly UIMessage[]): boolean {
  return messages.some((message) => message.parts.some((part) => part.type === 'tool-call'
    && part.state !== 'complete' && part.state !== 'error'));
}

function nativeCustomChunk(name: string, value: unknown): StreamChunk {
  return { type: EventType.CUSTOM, name, value } as StreamChunk;
}

/** Reset the SDK stream once, then restore native running-tool authority in one
 * update. Per-tool replacements repeatedly rebuilt the whole retained history. */
export function conversationSnapshotChunks(messages: readonly UIMessage[]): StreamChunk[] {
  const running = messages.flatMap((message) => {
    const toolCalls = message.parts.filter((part) => part.type === 'tool-call'
      && part.state !== 'complete' && part.state !== 'error');
    return toolCalls.length ? [{ itemId: message.id, toolCalls, metadata: message.metadata }] : [];
  });
  return [{ type: EventType.MESSAGES_SNAPSHOT, messages: [...messages] } as StreamChunk,
    ...(running.length ? [nativeCustomChunk('assembly:running-tools', running)] : [])];
}

/** Translate one native event into the standard chunks owned by TanStack. */
export function conversationChunksFromEvent(
  messages: readonly UIMessage[],
  source: AgentConversationEvent | AgentEvent
): StreamChunk[] {
  const event = 'type' in source ? source : displayEventFrom(source);
  const finishChunks = reasoningEndChunks(messages, event);
  const request = permissionRequestFromEvent(event);
  if (request) {
    const row = displayItemFromApproval(request, event.timestampMs);
    return [{ type: EventType.TEXT_MESSAGE_START, messageId: row.itemId, role: 'assistant', metadata: { displayItem: row } }, ...finishChunks];
  }
  const item = agentItemFromEvent(event);
  if (!item) return finishChunks;
  const existing = messages.find((message) => message.id === item.id);
  const prior = metadataOf(existing);
  const metadata: Record<string, AgentConfigValue> = {
    ...prior, ...item.providerMetadata, itemType: item.type,
    // Like its start, an item's turn is set by the event that began it, even
    // when that was between turns; a background task updated during later
    // turns must not move into them. A message sent from here has no turn
    // until it is admitted.
    turnId: 'turnId' in prior ? prior.turnId : item.turnId ?? null,
    startedAtMs: prior.startedAtMs ?? item.providerMetadata?.startedAtMs ?? event.timestampMs,
    firstSequence: prior.firstSequence ?? item.providerMetadata?.firstSequence ?? event.sequence,
    lastSequence: event.sequence,
    positionKnown: item.providerMetadata?.positionKnown ?? prior.positionKnown ?? true
  };
  // Tool output has one owner: the paired canonical tool-call/result parts.
  delete metadata.output;
  const text = item.content.map((part) => part.text).join('');
  const append = conversationEventAppendsItemContent(event);
  const display = displayItemFromAgentItem(item, event.timestampMs);
  const chunks: StreamChunk[] = existing
    ? [nativeCustomChunk('assembly:message-metadata', { itemId: item.id, metadata })]
    : [{
        type: EventType.TEXT_MESSAGE_START,
        messageId: item.id,
        role: item.type === 'user-message' ? 'user' : 'assistant',
        metadata
      }];
  if (display.kind === 'tool') {
    const previousCall = existing?.parts.find((part) => part.type === 'tool-call');
    const previousOutput = previousCall?.output && typeof previousCall.output === 'object' ? previousCall.output as Record<string, AgentConfigValue> : {};
    const previousText = String(previousOutput.output ?? '');
    const duplicateReplay = item.providerMetadata?.replay === true && previousText.endsWith(text);
    const output = {
      ...previousOutput, ...item.providerMetadata,
      startedAtMs: metadata.startedAtMs,
      output: append && !duplicateReplay ? previousText + text : text && !duplicateReplay ? text : previousText
    };
    if (!previousCall) {
      chunks.push({ type: EventType.TOOL_CALL_START, toolCallId: item.id, parentMessageId: item.id, toolCallName: String(item.providerMetadata?.name ?? display.title) });
      const input = item.providerMetadata?.rawInput ?? {};
      chunks.push({ type: EventType.TOOL_CALL_ARGS, toolCallId: item.id, delta: typeof input === 'string' ? input : JSON.stringify(input) });
      chunks.push({ type: EventType.TOOL_CALL_END, toolCallId: item.id });
    }
    if (display.state === 'completed' || display.state === 'failed') {
      chunks.push({ type: EventType.TOOL_CALL_RESULT, toolCallId: item.id, messageId: item.id,
        content: JSON.stringify(output), role: 'tool',
        metadata: display.state === 'failed' ? { tanstack: { state: 'output-error' } } : undefined });
    } else {
      chunks.push(nativeCustomChunk('assembly:running-tool', { itemId: item.id, output }));
    }
    return [...chunks, ...finishChunks];
  }
  const previousText = textOf(existing);
  if (append) {
    if (text && !(item.providerMetadata?.replay === true && previousText.endsWith(text))) {
      if (item.type === 'assistant-message' && existing?.metadata && 'blocks' in existing.metadata) {
        chunks.push(nativeCustomChunk('assembly:stale-markdown', { itemId: item.id }));
      }
      chunks.push({ type: EventType.TEXT_MESSAGE_CONTENT, messageId: item.id, delta: text });
    }
  } else if (text.startsWith(previousText)) {
    const delta = text.slice(previousText.length);
    if (delta) chunks.push({ type: EventType.TEXT_MESSAGE_CONTENT, messageId: item.id, delta });
  } else if (text !== previousText) {
    chunks.length = 0;
    chunks.push(...conversationSnapshotChunks(messages.map((message) => message.id === item.id ? {
        ...message,
        role: item.type === 'user-message' ? 'user' : 'assistant',
        metadata,
        parts: [{ type: item.type === 'reasoning' ? 'thinking' : 'text', content: text }]
      } : message)
    ));
  }
  if (metadata.completed === true) chunks.push({ type: EventType.TEXT_MESSAGE_END, messageId: item.id });
  return [...chunks, ...finishChunks];
}

function reasoningEndChunks(
  messages: readonly UIMessage[],
  event: AgentConversationEvent | AgentEvent
): StreamChunk[] {
  const payload = event.payload as Record<string, unknown>;
  const kind = typeof payload.kind === 'string'
    ? payload.kind.replace(/_([a-z])/g, (_, letter: string) => letter.toUpperCase())
    : '';
  const eventType = 'type' in event ? event.type : '';
  if (!(kind === 'turn' && payload.state !== 'started')
    && !['assistantDelta', 'assistantMessage', 'agentMessageChunk'].includes(kind)
    && !['turn.completed', 'turn.interrupted'].includes(eventType)
    && !(eventType === 'content.delta' && payload.channel === 'assistant')) return [];
  const turnId = 'turnId' in event ? event.turnId : payload.turnId;
  return messages.flatMap((message) => {
    const metadata = metadataOf(message);
    if (metadata.itemType !== 'reasoning' || metadata.completed === true) return [];
    if (turnId && metadata.turnId && turnId !== metadata.turnId) return [];
    return [{
      type: EventType.TEXT_MESSAGE_END,
      messageId: message.id,
      metadata: { completed: true, streaming: false }
    }];
  });
}

export function conversationMessagesAfterCustom(
  messages: readonly UIMessage[],
  name: string,
  value: unknown
): UIMessage[] {
  const patch = value as {
    itemId?: unknown;
    output?: unknown;
    toolCalls?: UIMessage['parts'];
    metadata?: UIMessage['metadata'];
  };
  if (name === 'assembly:running-tool' || name === 'assembly:running-tools') {
    const patches = new Map((name === 'assembly:running-tools' ? value as typeof patch[] : [patch])
      .map((entry) => [entry.itemId, entry]));
    return messages.map((message) => {
      const current = patches.get(message.id);
      if (!current) return message;
      return {
        ...message,
        ...(current.metadata ? { metadata: current.metadata } : {}),
        parts: message.parts.reduce<UIMessage['parts']>((parts, part) => {
          const callId = part.type === 'tool-result' ? part.toolCallId : part.type === 'tool-call' ? part.id : null;
          const authoritative = current.toolCalls?.find((call) => call.type === 'tool-call' && call.id === callId);
          if (part.type === 'tool-result' && authoritative) return parts;
          if (part.type !== 'tool-call') return [...parts, part];
          if (current.toolCalls && !authoritative) return [...parts, part];
          return [...parts, authoritative ?? { ...part, output: current.output, state: 'input-complete' as const }];
        }, [])
      };
    });
  }
  if (typeof patch.itemId !== 'string') return [...messages];
  if (name === 'assembly:message-metadata') {
    return messages.map((message) => message.id === patch.itemId
      ? { ...message, metadata: { ...message.metadata, ...patch.metadata } }
      : message);
  }
  if (name === 'assembly:stale-markdown') {
    return messages.map((message) => {
      if (message.id !== patch.itemId || !message.metadata) return message;
      const metadata = { ...message.metadata };
      delete metadata.blocks;
      return { ...message, metadata };
    });
  }
  return [...messages];
}

/** Bounded history replay uses the same native-to-TanStack mapping as live events. */
function applyMessageEvent(processor: StreamProcessor, event: AgentConversationEvent | AgentEvent): boolean {
  const chunks = conversationChunksFromEvent(processor.getMessages(), event);
  for (const chunk of chunks) {
    if (chunk.type === EventType.CUSTOM) {
      processor.setMessages(conversationMessagesAfterCustom(
        processor.getMessages(), chunk.name, chunk.value
      ));
    } else {
      processor.processChunk(chunk);
    }
  }
  return chunks.length > 0;
}

export function displayItemsFromConversationEvents(events: readonly (AgentEvent | AgentConversationEvent)[]): ConversationDisplayItem[] {
  const processor = new StreamProcessor();
  for (const event of events) {
    const normalized = 'type' in event ? event : displayEventFrom(event);
    applyMessageEvent(processor, normalized);
  }
  return conversationDisplayItems(processor.getMessages());
}
