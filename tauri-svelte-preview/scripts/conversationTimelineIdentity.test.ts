import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  type ConversationDisplayItem
} from '../src/lib/shell/conversation/conversationTimeline.ts';
import { conversationChunksFromEvent, conversationDisplayItems, conversationMessagesAfterCustom } from '../src/lib/shell/conversation/conversationMessages.ts';
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

console.log('conversation timeline identity tests passed');
