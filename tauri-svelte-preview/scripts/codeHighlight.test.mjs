/**
 * codeHighlight.test.mjs — the two pure decisions behind transcript code
 * coloring: which language a fence means, and which color class a token gets.
 *
 * Run: node --experimental-strip-types scripts/codeHighlight.test.mjs
 */
import assert from 'node:assert/strict';
import {
  codeTokenClass,
  monacoLanguageForFence,
  plainHighlightedLines
} from '../src/lib/shell/components/conversation/codeHighlight.ts';

// Common fence spellings reach the editor's language ids.
assert.equal(monacoLanguageForFence('ts'), 'typescript');
assert.equal(monacoLanguageForFence('TSX'), 'typescript');
assert.equal(monacoLanguageForFence('js'), 'javascript');
assert.equal(monacoLanguageForFence('py'), 'python');
assert.equal(monacoLanguageForFence('bash'), 'shell');
assert.equal(monacoLanguageForFence('sh'), 'shell');
assert.equal(monacoLanguageForFence('rs'), 'rust');
assert.equal(monacoLanguageForFence('cs'), 'csharp');
assert.equal(monacoLanguageForFence('yml'), 'yaml');
assert.equal(monacoLanguageForFence('json'), 'json');
assert.equal(monacoLanguageForFence('rust'), 'rust');

// A fence can carry more than the language ("ts title=foo"); the first word wins.
assert.equal(monacoLanguageForFence('ts title=example.ts'), 'typescript');

// Anything unknown or missing falls back to plain text rather than guessing.
assert.equal(monacoLanguageForFence(''), 'plaintext');
assert.equal(monacoLanguageForFence('   '), 'plaintext');
assert.equal(monacoLanguageForFence('not-a-language'), 'plaintext');

// Token types arrive from the editor with a language suffix.
assert.equal(codeTokenClass('keyword.ts'), 'keyword');
assert.equal(codeTokenClass('keyword.control.rust'), 'keyword');
assert.equal(codeTokenClass('string.quoted.double.js'), 'string');
assert.equal(codeTokenClass('comment.line.python'), 'comment');
assert.equal(codeTokenClass('number.hex.rust'), 'number');
assert.equal(codeTokenClass('type.identifier.ts'), 'type');
assert.equal(codeTokenClass('identifier.ts'), 'plain');
assert.equal(codeTokenClass(''), 'plain');
assert.equal(codeTokenClass('source.ts'), 'plain');

// The fallback rendering keeps every line, uncolored, so a failed load still
// shows the code.
assert.deepEqual(plainHighlightedLines('a\nb'), [
  [{ value: 'a', className: 'plain' }],
  [{ value: 'b', className: 'plain' }]
]);
assert.deepEqual(plainHighlightedLines(''), [[{ value: '', className: 'plain' }]]);

console.log('codeHighlight.test.mjs passed');
