/**
 * Pins the rail's one clock: it starts when the first visible row asks for it,
 * every listener hears the same tick, it runs no faster than the rows need,
 * and it stops the moment the last row lets go — an idle rail must not wake
 * the app up once a second.
 *
 * Run: node --experimental-strip-types scripts/railElapsedTicker.test.ts
 */
import assert from 'node:assert/strict';

import {
  formatRailElapsed,
  railElapsedCadenceFor,
  railElapsedTickerInterval,
  railElapsedTickerRunning,
  railElapsedWatcherCount,
  RAIL_ELAPSED_MINUTE_MS,
  RAIL_ELAPSED_SECOND_MS,
  watchRailElapsed
} from '../src/lib/shell/components/railElapsedTicker.ts';

// --- the lifecycle ---------------------------------------------------------

assert.equal(railElapsedTickerRunning(), false, 'a rail nobody is watching has no timer');
assert.equal(railElapsedWatcherCount(), 0);
assert.equal(railElapsedTickerInterval(), null);

const firstTicks: number[] = [];
const releaseFirst = watchRailElapsed((nowMs: number) => firstTicks.push(nowMs));
assert.equal(railElapsedTickerRunning(), true, 'the first visible row starts the clock');

const secondTicks: number[] = [];
const releaseSecond = watchRailElapsed((nowMs: number) => secondTicks.push(nowMs));
assert.equal(railElapsedWatcherCount(), 2, 'a second row joins the same clock');

await new Promise((resolve) => setTimeout(resolve, RAIL_ELAPSED_SECOND_MS + 120));

assert.ok(firstTicks.length >= 1, 'the first row heard a tick');
assert.deepEqual(secondTicks, firstTicks, 'both rows are told the same time by the same timer');

releaseFirst();
assert.equal(railElapsedTickerRunning(), true, 'one row left, one row still counting');

releaseSecond();
assert.equal(railElapsedTickerRunning(), false, 'the last row to leave stops the clock');
assert.equal(railElapsedWatcherCount(), 0);
assert.equal(railElapsedTickerInterval(), null);

releaseSecond();
assert.equal(railElapsedTickerRunning(), false, 'letting go twice is harmless');

// --- the cadence -----------------------------------------------------------

const releaseSlow = watchRailElapsed(() => {}, 'minute');
assert.equal(
  railElapsedTickerInterval(),
  RAIL_ELAPSED_MINUTE_MS,
  'with nothing in the seconds range the clock drops to once a minute'
);

const releaseFast = watchRailElapsed(() => {}, 'second');
assert.equal(
  railElapsedTickerInterval(),
  RAIL_ELAPSED_SECOND_MS,
  'one row counting seconds speeds the shared clock back up'
);

releaseFast();
assert.equal(
  railElapsedTickerInterval(),
  RAIL_ELAPSED_MINUTE_MS,
  'and it slows down again the moment that row is done'
);

releaseSlow();
assert.equal(railElapsedTickerRunning(), false);

assert.equal(railElapsedCadenceFor(5_000, false), 'second', 'a young row counts in seconds');
assert.equal(railElapsedCadenceFor(59_999, false), 'second');
assert.equal(railElapsedCadenceFor(60_000, false), 'minute', 'past a minute the seconds stop mattering');
assert.equal(
  railElapsedCadenceFor(4_000_000, true),
  'second',
  'a working row keeps the fast cadence however old it is'
);

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

console.log('railElapsedTicker: ok');
