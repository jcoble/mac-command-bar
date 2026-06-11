import assert from 'node:assert/strict';
import {
  cleanupPasteText,
  formatPasteCleanupStats,
  pasteCleanupModes
} from '../src/lib/pasteCleanup.ts';

const messy = '  First line  \r\n\r\n\r\n\tSecond   line  \n\n\n';

assert.deepEqual(pasteCleanupModes, ['plain', 'compact', 'prompt', 'reply']);
assert.equal(cleanupPasteText(messy, 'plain'), 'First line\n\nSecond   line');
assert.equal(cleanupPasteText(messy, 'compact'), 'First line Second line');
assert.equal(
  cleanupPasteText('```txt\n  Here is the answer.  \n\n\nThanks.  \n```', 'prompt'),
  'Here is the answer.\n\nThanks.'
);
assert.equal(
  cleanupPasteText(
    '> Here is the answer.  \r\n> \r\nCopy code\n```ts\nconst x = 1;\n```\n\n\nThanks.  ',
    'reply'
  ),
  'Here is the answer.\n\n```ts\nconst x = 1;\n```\n\nThanks.'
);
assert.equal(cleanupPasteText('A\u00a0B\r\nC', 'plain'), 'A B\nC');
assert.equal(formatPasteCleanupStats('', ''), '0 chars');
assert.equal(formatPasteCleanupStats('  abc  ', 'abc'), '3 chars - trimmed 4');
