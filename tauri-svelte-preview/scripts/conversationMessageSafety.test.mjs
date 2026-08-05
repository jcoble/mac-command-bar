import assert from 'node:assert/strict';
import {
  parseSafeMarkdown,
  sanitizeConversationHref,
  sanitizeConversationMarkdown
} from '../src/lib/shell/conversation/conversationMessageSafety.ts';

const source = '# Answer\n\nHere is **safe** [documentation](https://example.test/docs).\n\n```ts\nconst value = 1;';
const blocks = parseSafeMarkdown(source);
assert.equal(blocks[0].kind, 'heading');
assert.equal(blocks[1].kind, 'paragraph');
assert.equal(blocks[2].kind, 'code');
assert.equal(blocks[2].complete, false, 'partial fences remain code, not executable HTML');
assert.equal(sanitizeConversationHref('javascript:alert(1)'), null);
assert.equal(sanitizeConversationHref('https://example.test'), 'https://example.test');
assert.match(sanitizeConversationMarkdown('<script>alert(1)</script>'), /&lt;script&gt;/);
assert.doesNotMatch(sanitizeConversationMarkdown('<script>alert(1)</script>'), /<script>/);

const table = parseSafeMarkdown('| Name | State |\n| --- | --- |\n| A | done |');
assert.equal(table[0].kind, 'table');
assert.equal(table[0].rows.length, 1);
const tasks = parseSafeMarkdown('- [x] shipped\n- [ ] pending');
assert.equal(tasks[0].kind, 'list');
assert.equal(tasks[0].items[0].checked, true);
const fileLink = parseSafeMarkdown('[source](src/main.ts)');
assert.equal(fileLink[0].parts[0].kind, 'file-link');

console.log('conversationMessageSafety.test.mjs passed');
