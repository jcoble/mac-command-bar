import assert from 'node:assert/strict';
import {
  type ConversationDisplayItem
} from '../src/lib/shell/conversation/conversationTimeline.ts';
import { conversationDisplayItems } from '../src/lib/shell/conversation/conversationMessages.ts';
import type { UIMessage } from '@tanstack/ai/client';

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

console.log('conversation timeline identity tests passed');
