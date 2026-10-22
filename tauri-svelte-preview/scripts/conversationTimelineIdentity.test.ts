import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import {
  reuseConversationDisplayItems,
  type ConversationDisplayItem
} from '../src/lib/shell/conversation/conversationTimeline.ts';
import { conversationChunksFromEvent, conversationDisplayItems, conversationMessagesAfterCustom, takeStreamItem } from '../src/lib/shell/conversation/conversationMessages.ts';
import { EventType, StreamProcessor, type StreamChunk, type UIMessage } from '@tanstack/ai/client';

const items: UIMessage[] = Array.from({ length: 2_000 }, (_, index) => ({
  id: `item-${index}`,
  role: 'assistant',
  parts: [{ type: 'text', content: `Message ${index}` }],
  metadata: { itemType: 'assistant-message', completed: index < 1_999, startedAtMs: index }
}));

const before = conversationDisplayItems(items);
const nextItems = items.slice();
nextItems[1_999] = {
  ...items[1_999],
  parts: [{ type: 'text', content: 'Message 1999 plus one delta' }]
};
const after = conversationDisplayItems(nextItems, before);

const changed = after.filter((item: ConversationDisplayItem, index: number) => item !== before[index]);
assert.equal(changed.length, 1, 'one delta changes only the touched display item object');
assert.equal(changed[0].itemId, 'item-1999');
for (let index = 0; index < 1_999; index += 1) {
  assert.equal(after[index], before[index], `display item ${index} keeps object identity`);
}

test('a_text_delta_to_an_existing_message_sends_no_metadata_chunk_when_only_the_sequence_changed', () => {
  const processor = new StreamProcessor();
  const apply = (sequence: number, text: string): StreamChunk[] => {
    const chunks = conversationChunksFromEvent(processor.getMessages(), {
      type: 'content.delta', ownedId: 'owner', provider: 'codex', providerInstanceId: 'fixture',
      generation: 1, sequence, timestampMs: sequence * 10, nativeSessionId: 'native', itemId: 'assistant-1',
      payload: { kind: 'assistantDelta', itemId: 'assistant-1', text }, providerMetadata: {}
    } as never);
    for (const chunk of chunks) {
      if (chunk.type === EventType.CUSTOM) processor.setMessages(conversationMessagesAfterCustom(processor.getMessages(), chunk.name, chunk.value));
      else processor.processChunk(chunk);
    }
    return chunks;
  };
  apply(1, 'Hello');
  const chunks = apply(2, ' world');
  assert.equal(chunks.some((chunk) => chunk.type === EventType.CUSTOM && chunk.name === 'assembly:message-metadata'), false);
  assert.equal(processor.getMessages()[0]?.parts[0]?.type === 'text' && processor.getMessages()[0].parts[0].content, 'Hello world');
});

test('unchanged_messages_keep_their_display_items', () => {
  const first = conversationDisplayItems(items);
  const second = conversationDisplayItems(items);
  assert.ok(first.every((item, index) => item === second[index]));
});

test('a_plan_with_equal_steps_is_the_same_display_item', () => {
  const steps = () => [{ id: 'step-1', title: 'Read', detail: null, state: 'in-progress' as const, ownerAgentId: null, startedAt: null, completedAt: null }];
  const plan = { kind: 'plan' as const, itemId: 'plan-1', title: 'Plan', steps: steps(), timestampMs: 1 };
  const [reused] = reuseConversationDisplayItems([{ ...plan, steps: steps() }], [plan]);
  assert.equal(reused, plan);
});

test('streamed_text_deltas_in_one_frame_reach_the_chat_as_one_update', async () => {
  let frames = 0;
  Object.assign(globalThis, {
    requestAnimationFrame: (callback: () => void) => { frames += 1; return setTimeout(callback, 0); },
    cancelAnimationFrame: (frame: ReturnType<typeof setTimeout>) => clearTimeout(frame)
  });
  const delta = (sequence: number, itemId: string, text: string) => ({ nativeEvent: {
    ownedId: 'owner', provider: 'claude', generation: 1, sequence, timestampMs: sequence, turnId: 'turn-1',
    payload: { kind: 'assistantDelta', itemId, delta: text }
  } }) as never;
  const usage = { nativeEvent: { ownedId: 'owner', provider: 'claude', generation: 1, sequence: 4, timestampMs: 4, turnId: 'turn-1', payload: { kind: 'usage', usedTokens: 10 } } } as never;
  const queue = [delta(1, 'a', 'One'), delta(2, 'a', ' two'), delta(3, 'a', ' three'), usage, delta(5, 'a', ' four')];
  const signal = new AbortController().signal;
  const first = await takeStreamItem(queue, signal) as { nativeEvent: { sequence: number; timestampMs: number; payload: { itemId: string; delta: string } } };
  assert.equal(frames, 1);
  assert.equal(first.nativeEvent.payload.delta, 'One two three');
  assert.equal(first.nativeEvent.payload.itemId, 'a');
  // The joined delta holds the text up to the last delta, so it carries that delta's place.
  assert.equal(first.nativeEvent.sequence, 3);
  assert.equal(first.nativeEvent.timestampMs, 3);
  assert.equal(await takeStreamItem(queue, signal), usage);
  const last = await takeStreamItem(queue, signal) as { nativeEvent: { payload: { delta: string } } };
  assert.equal(last.nativeEvent.payload.delta, ' four');
  assert.equal(queue.length, 0);
});

test('a_push_while_the_stream_takes_from_an_empty_queue_is_delivered', async () => {
  const source = readFileSync(new URL('../src/lib/shell/conversation/conversationConnection.ts', import.meta.url), 'utf8');
  const block = source.slice(source.indexOf('function enqueueFactory('), source.indexOf('function resetReady('));
  const enqueueFactory = Function('conversationChunksFromEvent', 'recordLiveMessages', 'EventType', 'takeStreamItem',
    `${stripTypeScriptTypes(block, { mode: 'strip' })}\nreturn enqueueFactory;`
  )((_messages: unknown, event: { sequence: number }) => [{ type: 'probe', sequence: event.sequence }], () => undefined, EventType, takeStreamItem);
  const queue = enqueueFactory(new AbortController().signal, { chat: { messages: [] }, pendingSend: null });
  const next = queue.stream()[Symbol.asyncIterator]().next();
  queue.push({ nativeEvent: { ownedId: 'owner', provider: 'claude', generation: 1, sequence: 7, timestampMs: 7, payload: { kind: 'usage', usedTokens: 1 } } });
  const delivered = await Promise.race([next, new Promise((resolve) => setTimeout(() => resolve('still waiting'), 200))]);
  assert.deepEqual(delivered, { value: { type: 'probe', sequence: 7 }, done: false });
});

console.log('conversation timeline identity tests passed');
