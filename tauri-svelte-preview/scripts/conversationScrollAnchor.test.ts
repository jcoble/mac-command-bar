import assert from 'node:assert/strict';
import {
  decideConversationScroll,
  initialConversationScrollAnchorState
} from '../src/lib/shell/conversation/conversationScrollAnchor.ts';

let result = decideConversationScroll(initialConversationScrollAnchorState, {
  type: 'send',
  previousUserItemId: 'user-1',
  reducedMotion: false
});
result = decideConversationScroll(result.state, {
  type: 'user-items-changed',
  userItemIds: ['user-1', 'user-2']
});
assert.deepEqual(result.action, { type: 'anchor-user', itemId: 'user-2', motion: 'smooth' });

const anchoredState = result.state;
result = decideConversationScroll(anchoredState, { type: 'stream-growth' });
assert.equal(result.action.type, 'none', 'assistant stream growth never moves the viewport');
assert.deepEqual(result.state, anchoredState, 'stream growth preserves the user-turn anchor state');

result = decideConversationScroll(anchoredState, { type: 'user-input' });
assert.equal(result.action.type, 'cancel-programmatic-scroll', 'wheel/touch/keyboard cancels a smooth anchor');
assert.equal(result.state.programmaticMotion, 'idle');

result = decideConversationScroll(result.state, { type: 'jump-to-latest', reducedMotion: false });
assert.deepEqual(result.action, { type: 'scroll-to-latest', motion: 'smooth' });
assert.equal(result.state.pinnedToBottom, true, 'Jump to latest explicitly pins the bottom');
result = decideConversationScroll(result.state, { type: 'stream-growth' });
assert.deepEqual(result.action, { type: 'scroll-to-latest', motion: 'instant' }, 'explicit bottom pin follows later stream growth');

result = decideConversationScroll(initialConversationScrollAnchorState, {
  type: 'send',
  previousUserItemId: null,
  reducedMotion: true
});
result = decideConversationScroll(result.state, {
  type: 'user-items-changed',
  userItemIds: ['first-user']
});
assert.deepEqual(result.action, { type: 'anchor-user', itemId: 'first-user', motion: 'instant' });
assert.equal(result.state.programmaticMotion, 'idle', 'reduced motion does not start an animation');

console.log('conversationScrollAnchor.test.ts passed');
