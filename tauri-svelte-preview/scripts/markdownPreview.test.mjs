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
  markdownPreviewDefault,
  splitFrontmatter
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

// --- front matter stays out of the rich editor ------------------------------

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
  /import SourceMarkdownPreview from '\$lib\/SourceMarkdownPreview\.svelte'/,
  'the rendered view reuses the app markdown viewer instead of a new one'
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
