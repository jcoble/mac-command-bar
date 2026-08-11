/**
 * collapseToolOutput.test.mjs — the rule that keeps a long tool output from
 * swallowing the transcript.
 *
 * Run: node --experimental-strip-types scripts/collapseToolOutput.test.mjs
 */
import assert from 'node:assert/strict';
import {
  COLLAPSED_LINE_COUNT,
  COLLAPSE_THRESHOLD_LINES,
  collapseToolOutput,
  toolOutputLines
} from '../src/lib/shell/components/conversation/collapseToolOutput.ts';

assert.equal(COLLAPSE_THRESHOLD_LINES, 12);
assert.equal(COLLAPSED_LINE_COUNT, 6);

// Short output stays whole.
const short = ['one', 'two', 'three'];
assert.deepEqual(collapseToolOutput(short), { visible: short, hiddenCount: 0 });

// Exactly at the threshold is still whole — the rule is "over 12 lines".
const twelve = Array.from({ length: 12 }, (_, index) => `line ${index + 1}`);
assert.deepEqual(collapseToolOutput(twelve), { visible: twelve, hiddenCount: 0 });

// One line over the threshold collapses to the first six.
const thirteen = Array.from({ length: 13 }, (_, index) => `line ${index + 1}`);
const collapsedThirteen = collapseToolOutput(thirteen);
assert.deepEqual(collapsedThirteen.visible, thirteen.slice(0, 6));
assert.equal(collapsedThirteen.hiddenCount, 7);

// The count in the expand control is the count of lines a person cannot see.
const forty = Array.from({ length: 40 }, (_, index) => `line ${index + 1}`);
const collapsedForty = collapseToolOutput(forty);
assert.equal(collapsedForty.visible.length, 6);
assert.equal(collapsedForty.hiddenCount, 34);

// Empty and single-line output are safe.
assert.deepEqual(collapseToolOutput([]), { visible: [], hiddenCount: 0 });
assert.deepEqual(collapseToolOutput(['only']), { visible: ['only'], hiddenCount: 0 });

// The helper never mutates what it was handed.
const source = Array.from({ length: 20 }, (_, index) => `line ${index + 1}`);
collapseToolOutput(source);
assert.equal(source.length, 20);

// Text is split on either newline convention, and a trailing newline does not
// invent an empty last line.
assert.deepEqual(toolOutputLines('a\r\nb\nc'), ['a', 'b', 'c']);
assert.deepEqual(toolOutputLines('a\nb\n'), ['a', 'b']);
assert.deepEqual(toolOutputLines(''), []);
assert.deepEqual(toolOutputLines('   '), []);

console.log('collapseToolOutput.test.mjs passed');
