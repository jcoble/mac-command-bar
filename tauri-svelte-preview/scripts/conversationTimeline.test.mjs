import assert from 'node:assert/strict';
import {
  conversationScrollShouldFollow,
  displayItemFromAgentItem,
  typedConversationTimeline,
  visibleConversationRange
} from '../src/lib/shell/conversation/conversationTimeline.ts';

const typed = typedConversationTimeline([
  { id: 'assistant-1', type: 'assistant-message', content: [{ channel: 'assistant', text: 'Answer' }] },
  { id: 'tool-1', type: 'mcp-tool', content: [], providerMetadata: { name: 'shell', state: 'completed' } },
  { id: 'plan-1', type: 'plan', content: [], providerMetadata: { title: 'Ship', steps: [{ id: 'step-1', title: 'Test', state: 'in-progress' }] } }
], [{ kind: 'user', itemId: 'user-1', text: 'Question', completed: true, timestampMs: 1 }]);

assert.deepEqual(typed.map((item) => item.kind), ['user', 'assistant', 'tool', 'plan']);
assert.equal(displayItemFromAgentItem({ id: 'reason-1', type: 'reasoning', content: [{ channel: 'reasoning', text: 'Think' }] }, 2).kind, 'reasoning');
assert.equal(conversationScrollShouldFollow(500, 400, 950), true);
assert.equal(conversationScrollShouldFollow(300, 400, 950), false);
assert.deepEqual(visibleConversationRange(100, 960, 480, 96, 2), { start: 8, end: 17, offsetTop: 768 });
assert.deepEqual(visibleConversationRange(0, 0, 480), { start: 0, end: 0, offsetTop: 0 });

console.log('conversationTimeline.test.mjs passed');
