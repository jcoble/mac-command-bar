import assert from 'node:assert/strict';
import {
  CONVERSATION_RENDER_WINDOW,
  conversationRenderWindow,
  discloseEarlierConversationItems
} from '../src/lib/shell/conversation/conversationTimeline.ts';

const projection = Array.from({ length: 2_000 }, (_, index) => ({
  itemId: `item-${index}`,
  index
}));

const initial = conversationRenderWindow(projection, 'conversation-one');
assert.equal(CONVERSATION_RENDER_WINDOW, 120);
assert.equal(initial.items.length, 120, 'the initial render contains only the newest window');
assert.equal(initial.hiddenCount, 1_880);
assert.equal(initial.items[0], projection[1_880], 'windowing preserves projection object identity');
assert.equal(initial.items.at(-1), projection.at(-1), 'the newest projection object is reused');

const disclosed = discloseEarlierConversationItems(projection, 'conversation-one', initial.state);
assert.equal(disclosed.items.length, 240, 'one disclosure adds one render window');
assert.equal(disclosed.hiddenCount, 1_760);
for (let index = 0; index < disclosed.items.length; index += 1) {
  assert.equal(disclosed.items[index], projection[1_760 + index], `disclosed item ${index} keeps object identity`);
}

const appendedProjection = [...projection, { itemId: 'item-2000', index: 2_000 }];
const withLiveItem = conversationRenderWindow(appendedProjection, 'conversation-one', disclosed.state);
assert.equal(withLiveItem.items.length, 241, 'live items join the newest window without hiding disclosed items');
assert.equal(withLiveItem.items[0], projection[1_760], 'the first disclosed object remains rendered after append');
assert.equal(withLiveItem.items.at(-1), appendedProjection.at(-1), 'the appended object enters the window');

const reset = conversationRenderWindow(projection, 'conversation-two', disclosed.state);
assert.equal(reset.items.length, 120, 'switching conversations resets disclosure state');
assert.equal(reset.hiddenCount, 1_880);
assert.equal(reset.state.disclosedItems, 0);

console.log('conversation render window tests passed');
