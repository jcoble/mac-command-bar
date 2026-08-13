/**
 * Pins the rail's age reading, and that the reading is all there is: the rail
 * holds NO clock. A live per-second ticker was tried and it progressively
 * slowed the whole app, so any reappearance of one is a regression.
 *
 * Run: node --experimental-strip-types scripts/railElapsedFormat.test.ts
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { formatRailElapsed } from '../src/lib/shell/components/railElapsedFormat.ts';

// --- the reading, coarsening as it ages ------------------------------------

assert.equal(formatRailElapsed(0), '0s');
assert.equal(formatRailElapsed(-5_000), '0s', 'a clock skew never prints a negative age');
assert.equal(formatRailElapsed(38_000), '38s');
assert.equal(formatRailElapsed(59_999), '59s');
assert.equal(formatRailElapsed(60_000), '1m', 'from one minute the seconds are dropped');
assert.equal(formatRailElapsed(134_000), '2m');
assert.equal(formatRailElapsed(701_000), '11m');
assert.equal(formatRailElapsed(3_599_000), '59m');
assert.equal(formatRailElapsed(3_600_000), '1h', 'from one hour the minutes are dropped');
assert.equal(formatRailElapsed(7_800_000), '2h');
assert.equal(formatRailElapsed(180_000_000), '50h');

// --- and no clock anywhere near it -----------------------------------------

const here = dirname(fileURLToPath(import.meta.url));
const componentsDir = join(here, '..', 'src', 'lib', 'shell', 'components');
for (const name of ['railElapsedFormat.ts', 'WorktreeAgentRow.svelte']) {
  const source = readFileSync(join(componentsDir, name), 'utf8');
  assert.doesNotMatch(
    source,
    /setInterval|watchRailElapsed|railElapsedTicker/,
    `${name} must not tick — the rail's age is read once at mount`
  );
}

console.log('railElapsedFormat: ok');
