import assert from 'node:assert/strict';
import {
  typedConversationTimeline,
  type ConversationDisplayItem
} from '../src/lib/shell/conversation/conversationTimeline.ts';
import type { AgentItem } from '../src/lib/shell/conversation/conversationTypes.ts';

const items: AgentItem[] = Array.from({ length: 2_000 }, (_, index) => ({
  id: `item-${index}`,
  type: 'assistant-message',
  content: [{ channel: 'assistant', text: `Message ${index}` }],
  providerMetadata: { completed: index < 1_999, startedAtMs: index }
}));

const before = typedConversationTimeline(items);
const nextItems = items.slice();
nextItems[1_999] = {
  ...items[1_999],
  content: [{ channel: 'assistant', text: 'Message 1999 plus one delta' }]
};
const after = typedConversationTimeline(nextItems, [], {}, before);

const changed = after.filter((item: ConversationDisplayItem, index: number) => item !== before[index]);
assert.equal(changed.length, 1, 'one delta changes only the touched display item object');
assert.equal(changed[0].itemId, 'item-1999');
for (let index = 0; index < 1_999; index += 1) {
  assert.equal(after[index], before[index], `display item ${index} keeps object identity`);
}

console.log('conversation timeline identity tests passed');
