/**
 * Pins the rule the editor uses to decide whether a Markdown file opens as
 * rendered text or as raw source, and the wiring that makes the toggle real.
 *
 * Run: node --experimental-strip-types scripts/markdownPreview.test.mjs
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  isMarkdownFile,
  markdownImageTarget,
  markdownPreviewDefault,
  frontmatterRows,
  splitFrontmatter,
  taskCheckboxOffsets,
  toggleTaskAt,
  toggleTaskCheckbox
} from '../src/lib/shell/components/editor/markdownPreview.ts';

// --- which files are Markdown ---------------------------------------------

assert.equal(isMarkdownFile('README.md'), true);
assert.equal(isMarkdownFile('notes.MD'), true);
assert.equal(isMarkdownFile('CHANGELOG.markdown'), true);
assert.equal(isMarkdownFile('plan.mdx'), true);
assert.equal(isMarkdownFile('main.rs'), false);
assert.equal(isMarkdownFile('mdfile'), false, 'the extension has to be an extension');
assert.equal(isMarkdownFile(''), false);
assert.equal(isMarkdownFile(null), false);
assert.equal(isMarkdownFile(undefined), false);
assert.equal(isMarkdownFile('  README.md  '), true, 'surrounding space is ignored');

// --- what a newly opened file shows first ----------------------------------

assert.equal(
  markdownPreviewDefault('README.md', 'jump'),
  'rendered',
  'a Markdown file reached from a diff or a file jump opens rendered'
);
assert.equal(
  markdownPreviewDefault('README.md', 'strip'),
  'raw',
  'picking the file in the open-files strip keeps the editor on source'
);
assert.equal(
  markdownPreviewDefault('main.rs', 'jump'),
  'raw',
  'anything that is not Markdown has no rendered form here'
);
assert.equal(markdownPreviewDefault('main.rs', 'strip'), 'raw');

assert.equal(
  markdownPreviewDefault('README.md', 'jump'),
  markdownPreviewDefault('README.md', 'jump'),
  'the rule is pure: same input, same answer'
);

// --- front matter is shown apart from the document ----------------------------

{
  const file = '---\ntitle: Notes\ntags: [a, b]\n---\n\n# Notes\n\nBody.\n';
  const { frontmatter, body } = splitFrontmatter(file);
  assert.equal(frontmatter, '---\ntitle: Notes\ntags: [a, b]\n---\n\n', 'the block and the blank lines after it');
  assert.equal(body, '# Notes\n\nBody.\n');
  assert.equal(frontmatter + body, file, 'splitting never loses a byte');
}
assert.deepEqual(
  splitFrontmatter('---\r\ntitle: x\r\n...\r\nBody\r\n'),
  { frontmatter: '---\r\ntitle: x\r\n...\r\n', body: 'Body\r\n' },
  'CRLF files and a "..." closing line are front matter too'
);
assert.deepEqual(
  splitFrontmatter('# Title\n\n---\ntitle: x\n---\n'),
  { frontmatter: '', body: '# Title\n\n---\ntitle: x\n---\n' },
  'only a block at the very start of the file counts'
);
assert.deepEqual(
  splitFrontmatter('---\ntitle: never closed\n\nBody\n'),
  { frontmatter: '', body: '---\ntitle: never closed\n\nBody\n' },
  'an opening line with no closing line is ordinary Markdown'
);
assert.deepEqual(splitFrontmatter(''), { frontmatter: '', body: '' });

// --- front matter shows as a small table -------------------------------------

assert.deepEqual(
  frontmatterRows('---\ntitle: Notes\ntags: [a, b]\ndate: 2026-10-07\n---\n\n'),
  [['title', 'Notes'], ['tags', '[a, b]'], ['date', '2026-10-07']],
  'one row per top-level key, the value as written'
);
assert.deepEqual(
  frontmatterRows('---\r\nname: x\r\nauthors:\r\n  - Ann\r\n  - Bo\r\n# a comment\r\nurl: "https://a.b/c: d"\r\n...\r\n'),
  [['name', 'x'], ['authors', '- Ann\n- Bo'], ['url', '"https://a.b/c: d"']],
  'nested lines belong to the key above them; comments are skipped; the first colon splits'
);
assert.deepEqual(frontmatterRows(''), [], 'no front matter, no rows');

// --- task checkboxes map to one character in the file -------------------------

{
  const file = [
    '---',
    'todo: "- [ ] not a task"',
    '---',
    '- [ ] one',
    '- [x] two',
    '  - [X] nested',
    '1. [ ] ordered',
    '2) [x] ordered paren',
    '> - [ ] quoted',
    '> > * [ ] twice quoted',
    '+ [ ] plus',
    '```md',
    '- [ ] inside a fence',
    '```',
    '~~~',
    '- [x] inside a tilde fence',
    '~~~',
    '- [ ]no space after is not a task',
    '- [ ]   ',
    '- [y] not a box',
    'text - [ ] mid-line is not a task',
    '- [ ] last'
  ].join('\n');
  const offsets = taskCheckboxOffsets(file);
  const marks = offsets.map((offset) => file.slice(offset - 1, offset + 2));
  assert.deepEqual(
    marks,
    ['[ ]', '[x]', '[X]', '[ ]', '[x]', '[ ]', '[ ]', '[ ]', '[ ]'],
    'every task item in document order, and nothing in front matter, fences or non-task lines'
  );
  const lines = offsets.map((offset) => file.slice(file.lastIndexOf('\n', offset) + 1, file.indexOf('\n', offset) === -1 ? undefined : file.indexOf('\n', offset)));
  assert.deepEqual(lines, [
    '- [ ] one', '- [x] two', '  - [X] nested', '1. [ ] ordered', '2) [x] ordered paren',
    '> - [ ] quoted', '> > * [ ] twice quoted', '+ [ ] plus', '- [ ] last'
  ]);
}
{
  const crlf = '- [ ] a\r\n- [x] b\r\n';
  assert.deepEqual(taskCheckboxOffsets(crlf), [3, 12], 'CRLF line endings keep exact offsets');
}
{
  const nested = '- outer\n\n  ```\n  - [ ] fenced inside an item\n  ```\n- [ ] real\n';
  assert.equal(taskCheckboxOffsets(nested).length, 1, 'a fence indented inside a list item still hides its lines');
}

{
  const file = '# Plan\n\n- [ ] write\n- [x] test\n\nEnd.\n';
  const [first, second] = taskCheckboxOffsets(file);
  const checkedFirst = toggleTaskCheckbox(file, first);
  const changed = [...file].filter((char, index) => checkedFirst[index] !== char);
  assert.equal(checkedFirst.length, file.length, 'the file keeps its length');
  assert.deepEqual(changed, [' '], 'exactly one character changes');
  assert.equal(checkedFirst, '# Plan\n\n- [x] write\n- [x] test\n\nEnd.\n');
  assert.equal(toggleTaskCheckbox(file, second), '# Plan\n\n- [ ] write\n- [ ] test\n\nEnd.\n', 'x clears');
  assert.equal(toggleTaskCheckbox(toggleTaskCheckbox(file, first), first), file, 'two clicks give back the file byte for byte');
  assert.equal(toggleTaskCheckbox('- [X] a', 3), '- [ ] a', 'a capital X clears too');
  assert.throws(() => toggleTaskCheckbox(file, 0), /checkbox/, 'an offset that is not inside [ ] is refused');
}

// --- a click on a rendered box ----------------------------------------------

{
  const file = '- [ ] one\n- [x] two\n';
  assert.equal(toggleTaskAt(file, 0, 2, true), '- [x] one\n- [x] two\n', 'ticking the first box writes its x');
  assert.equal(toggleTaskAt(file, 1, 2, false), '- [ ] one\n- [ ] two\n', 'clearing the second box writes its space');
  assert.equal(toggleTaskAt(file, 0, 3, true), null, 'more boxes on screen than in the file: refused');
  assert.equal(toggleTaskAt(file, 2, 2, true), null, 'a box past the last one: refused');
  assert.equal(toggleTaskAt(file, 1, 2, true), null, 'the file already holds the state the box now shows: refused');
}

// --- where an image in a Markdown file comes from ------------------------------

const doc = '/Users/me/project/docs/guide.md';
assert.deepEqual(markdownImageTarget(doc, 'img/shot.png'), { kind: 'file', path: '/Users/me/project/docs/img/shot.png' });
assert.deepEqual(markdownImageTarget(doc, './img/shot.png'), { kind: 'file', path: '/Users/me/project/docs/img/shot.png' });
assert.deepEqual(markdownImageTarget(doc, '../assets/logo.svg'), { kind: 'file', path: '/Users/me/project/assets/logo.svg' });
assert.deepEqual(markdownImageTarget(doc, '/tmp/abs.png'), { kind: 'file', path: '/tmp/abs.png' }, 'an absolute path is used as it is');
assert.deepEqual(markdownImageTarget(doc, 'my%20shot.png?raw=1#top'), { kind: 'file', path: '/Users/me/project/docs/my shot.png' }, 'escapes decode; query and hash drop');
assert.deepEqual(markdownImageTarget(doc, 'https://example.com/a.png'), { kind: 'https', url: 'https://example.com/a.png' });
assert.deepEqual(markdownImageTarget(doc, 'HTTP://example.com/a.png'), { kind: 'http', url: 'HTTP://example.com/a.png' });
assert.equal(markdownImageTarget(doc, ''), null);
assert.equal(markdownImageTarget(doc, undefined), null, 'the sanitizer removed an unsafe URL');
assert.equal(markdownImageTarget(doc, 'data:image/png;base64,AAAA'), null);
assert.equal(markdownImageTarget(doc, '#anchor'), null);

// --- the wiring ------------------------------------------------------------

const panel = readFileSync(
  new URL('../src/lib/shell/components/EditorPanel.svelte', import.meta.url),
  'utf8'
);

assert.match(
  panel,
  /markdownPreviewDefault/,
  'the editor decides the first view with the shared rule'
);
assert.match(
  panel,
  /import\('\$lib\/SourceMarkdownPreview\.svelte'\)/,
  'the rendered view is the app markdown viewer, loaded the first time it is shown'
);
assert.match(
  panel,
  /SegmentedControl/,
  'the toggle is a kit control, sitting beside the language intelligence switch'
);
assert.match(
  panel,
  /data-testid="markdown-view-toggle"/,
  'the toggle can be found by a test and by a screenshot run'
);

console.log('markdownPreview: ok');
