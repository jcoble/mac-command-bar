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
  markdownPreviewDefault
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
