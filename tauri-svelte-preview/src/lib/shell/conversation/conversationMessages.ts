import { EventType, StreamProcessor, type UIMessage } from '@tanstack/ai/client';
import type { AgentConfigValue, AgentEvent, AgentConversationEvent, AgentItem, ConversationAttachment, ConversationTranscriptMessage } from './conversationTypes.ts';
import { agentItemFromEvent, conversationEventAppendsItemContent, displayItemFromAgentItem, displayItemFromApproval, permissionRequestFromEvent, reuseConversationDisplayItems, withoutRepeatedPlans, type ConversationDisplayItem } from './conversationTimeline.ts';

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
  const rows = messages.map(conversationMessageDisplayItem).map((item) => {
    const attachments = item.kind === 'user' ? sentAttachments[item.itemId] : undefined;
    return attachments?.length ? { ...item, attachments } : item;
  }).sort((a, b) => a.timestampMs - b.timestampMs);
  return reuseConversationDisplayItems(withoutRepeatedPlans(rows), previous);
}

export function conversationHasRunningTool(messages: readonly UIMessage[]): boolean {
  return messages.some((message) => message.parts.some((part) => part.type === 'tool-call'
    && part.state !== 'complete' && part.state !== 'error'));
}

export function restoreProcessor(processor: StreamProcessor, messages: readonly UIMessage[]): void {
  processor.reset();
  for (const message of messages) {
    // Seed each message on its own so every chunk scans one message, not all.
    processor.setMessages([]);
    processor.processChunk({ type: EventType.TEXT_MESSAGE_START, messageId: message.id, role: message.role, metadata: message.metadata });
    for (const part of message.parts) {
      if (part.type === 'text') {
        processor.processChunk({ type: EventType.TEXT_MESSAGE_CONTENT, messageId: message.id, delta: part.content });
      } else if (part.type === 'tool-call') {
        processor.processChunk({ type: EventType.TOOL_CALL_START, toolCallId: part.id, parentMessageId: message.id, toolCallName: part.name, metadata: part.metadata as Record<string, unknown> | undefined });
        processor.processChunk({ type: EventType.TOOL_CALL_ARGS, toolCallId: part.id, delta: part.arguments });
        processor.processChunk({ type: EventType.TOOL_CALL_END, toolCallId: part.id });
      }
    }
  }
  // Preserve the exact canonical parts, including running native tool output
  // and results (a result chunk changes only the parts, so none is replayed).
  // The event replay above seeds the processor's internal segment state.
  processor.setMessages([...messages]);
}

/** Native events carry authoritative full messages as well as text deltas. */
export function applyMessageEvent(processor: StreamProcessor, event: AgentConversationEvent | AgentEvent): boolean {
  const request = permissionRequestFromEvent(event);
  if (request) {
    const row = displayItemFromApproval(request, event.timestampMs);
    processor.processChunk({ type: EventType.TEXT_MESSAGE_START, messageId: row.itemId, role: 'assistant', metadata: { displayItem: row } });
    return true;
  }
  const item = agentItemFromEvent(event);
  if (!item) return false;
  const existing = processor.getMessages().find((message) => message.id === item.id);
  const prior = metadataOf(existing);
  // A row belongs to the turn it was first written in, since it keeps that
  // place. A background task can finish during a later turn (or start between
  // turns); taking that turn's id cut both turns in two around the row.
  const metadata: Record<string, AgentConfigValue> = {
    ...prior, ...item.providerMetadata, itemType: item.type,
    turnId: existing ? prior.turnId ?? null : item.turnId ?? null,
    startedAtMs: prior.startedAtMs ?? item.providerMetadata?.startedAtMs ?? event.timestampMs
  };
  // Tool output has one owner: the paired canonical tool-call/result parts.
  delete metadata.output;
  const text = item.content.map((part) => part.text).join('');
  const append = conversationEventAppendsItemContent(event);
  const display = displayItemFromAgentItem(item, event.timestampMs);
  processor.processChunk({ type: EventType.TEXT_MESSAGE_START, messageId: item.id, role: item.type === 'user-message' ? 'user' : 'assistant', metadata });
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
      processor.processChunk({ type: EventType.TOOL_CALL_START, toolCallId: item.id, parentMessageId: item.id, toolCallName: String(item.providerMetadata?.name ?? display.title) });
      const input = item.providerMetadata?.rawInput ?? {};
      processor.processChunk({ type: EventType.TOOL_CALL_ARGS, toolCallId: item.id, delta: typeof input === 'string' ? input : JSON.stringify(input) });
      processor.processChunk({ type: EventType.TOOL_CALL_END, toolCallId: item.id });
    }
    if (display.state === 'completed' || display.state === 'failed') {
      processor.processChunk({ type: EventType.TOOL_CALL_RESULT, toolCallId: item.id, messageId: item.id,
        content: JSON.stringify(output), role: 'tool',
        metadata: display.state === 'failed' ? { tanstack: { state: 'output-error' } } : undefined });
    } else {
      // Native providers stream tool output, whereas AG-UI results are terminal.
      // Keep progress in the canonical call until its real result arrives.
      processor.setMessages(processor.getMessages().map((message) => message.id === item.id
        ? { ...message, parts: message.parts.map((part) => part.type === 'tool-call'
          ? { ...part, output, state: 'input-complete' as const } : part) }
        : message));
    }
    return true;
  }
  const previousText = textOf(existing);
  if (append) {
    if (!(item.providerMetadata?.replay === true && previousText.endsWith(text))) {
      processor.processChunk({ type: EventType.TEXT_MESSAGE_CONTENT, messageId: item.id, delta: text });
    }
  } else if (text.startsWith(previousText)) {
    const delta = text.slice(previousText.length);
    if (delta) processor.processChunk({ type: EventType.TEXT_MESSAGE_CONTENT, messageId: item.id, delta });
  } else {
    // Full snapshots can replace streamed text. Re-seed the library's internal
    // segments too, so a later delta cannot resurrect the replaced prefix.
    restoreProcessor(processor, processor.getMessages().map((message) => message.id === item.id
      ? { ...message, parts: [{ type: 'text', content: text }] }
      : message));
  }
  if (metadata.completed === true) processor.processChunk({ type: EventType.TEXT_MESSAGE_END, messageId: item.id });
  return true;
}

export function finishMessageReasoning(processor: StreamProcessor, event: AgentConversationEvent | AgentEvent): boolean {
  const payload = event.payload as Record<string, unknown>;
  const kind = typeof payload.kind === 'string' ? payload.kind.replace(/_([a-z])/g, (_, letter: string) => letter.toUpperCase()) : '';
  const eventType = 'type' in event ? event.type : '';
  if (!(kind === 'turn' && payload.state !== 'started')
    && !['assistantDelta', 'assistantMessage', 'agentMessageChunk'].includes(kind)
    && !['turn.completed', 'turn.interrupted'].includes(eventType)
    && !(eventType === 'content.delta' && payload.channel === 'assistant')) return false;
  const turnId = 'turnId' in event ? event.turnId : payload.turnId;
  let changed = false;
  for (const message of processor.getMessages()) {
    const metadata = metadataOf(message);
    if (metadata.itemType !== 'reasoning' || metadata.completed === true) continue;
    if (turnId && metadata.turnId && turnId !== metadata.turnId) continue;
    processor.processChunk({ type: EventType.TEXT_MESSAGE_END, messageId: message.id, metadata: { completed: true, streaming: false } });
    changed = true;
  }
  return changed;
}

export function transcriptMessages(messages: readonly ConversationTranscriptMessage[], prefix = ''): StreamProcessor {
  const processor = new StreamProcessor();
  processor.setMessages(messages.map((message) => ({
    id: prefix + message.itemId, role: message.role,
    parts: [{ type: 'text', content: message.text }],
    metadata: { itemType: message.role === 'user' ? 'user-message' : 'assistant-message', completed: true, startedAtMs: message.timestampMs }
  })));
  return processor;
}

export function displayItemsFromConversationEvents(events: readonly (AgentEvent | AgentConversationEvent)[]): ConversationDisplayItem[] {
  const processor = new StreamProcessor();
  for (const event of events) {
    const normalized = 'type' in event ? event : displayEventFrom(event);
    applyMessageEvent(processor, normalized);
    finishMessageReasoning(processor, normalized);
  }
  return conversationDisplayItems(processor.getMessages());
}
