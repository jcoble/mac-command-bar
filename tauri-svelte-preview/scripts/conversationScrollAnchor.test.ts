import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  decideConversationScroll,
  initialConversationScrollAnchorState,
  USER_SEND_ANCHOR_OFFSET_PX
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
assert.deepEqual(result.action, {
  type: 'anchor-user',
  itemId: 'user-2',
  motion: 'smooth',
  offsetPx: USER_SEND_ANCHOR_OFFSET_PX
});

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
assert.deepEqual(result.action, {
  type: 'anchor-user',
  itemId: 'first-user',
  motion: 'instant',
  offsetPx: USER_SEND_ANCHOR_OFFSET_PX
});
assert.equal(result.state.programmaticMotion, 'idle', 'reduced motion does not start an animation');

// The transcript keeps a screen-tall spacer under the newest user message so that
// message can sit at the top of the screen. If the spacer went away when the turn
// finished, the page would get shorter under the reader and the browser would drag
// the view down to the new bottom. The spacer therefore has to depend on the
// anchored message, not on whether a turn is still running.
const timelineSource = readFileSync(
  new URL('../src/lib/shell/components/conversation/ConversationTimeline.svelte', import.meta.url),
  'utf8'
);
const tailLine = timelineSource
  .split('\n')
  .find((line) => line.includes('const showActiveTurnTail'));
assert.ok(tailLine, 'the transcript still declares the trailing spacer');
assert.ok(
  !tailLine.includes('localTurnActive'),
  'the trailing spacer must outlive the turn so the end of a turn cannot shorten the page and drag the view to the bottom'
);
assert.ok(
  tailLine.includes('anchoredUserIndex'),
  'the trailing spacer belongs to the anchored user message'
);

// Following the newest writing has to stop where the writing stops. The scroll box
// is one screen taller than its writing because of that spacer, so aiming at the
// very bottom of the box would sail past the last line into blank screen and leave
// the reader chasing text that has scrolled off above the prompt.
assert.ok(
  !/scrollHeight - host\.clientHeight/.test(timelineSource),
  'following the newest writing must not aim at the bottom of the scroll box, which includes the trailing spacer'
);
assert.ok(
  /scrollHeight - \(tail\?\.offsetHeight \?\? 0\) - host\.clientHeight/.test(timelineSource),
  'the follow target subtracts the trailing spacer so the last line lands just above the prompt box'
);

console.log('conversationScrollAnchor.test.ts passed');
