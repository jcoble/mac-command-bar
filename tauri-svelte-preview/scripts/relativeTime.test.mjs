import assert from 'node:assert/strict';

import { exactLocalTime, formatLastActivity } from '../src/lib/shell/relativeTime.ts';

/**
 * These tests build every stamp in LOCAL time, on purpose. The whole point of
 * the module is that the rail stops showing UTC, so a test written in UTC would
 * pass in London and fail everywhere else.
 */

/** A local wall-clock moment, as the reader's own machine would see it. */
function local(year, month, day, hour = 0, minute = 0, second = 0) {
  return new Date(year, month - 1, day, hour, minute, second);
}

const now = local(2026, 7, 28, 14, 30);

// Under a minute has nothing useful to say, and "0m ago" reads like a bug.
assert.equal(formatLastActivity(now.toISOString(), now), 'just now');
assert.equal(formatLastActivity(local(2026, 7, 28, 14, 29, 30).toISOString(), now), 'just now');

// Minutes, then hours, both rounded down — "1h ago" until it really is two.
assert.equal(formatLastActivity(local(2026, 7, 28, 14, 25).toISOString(), now), '5m ago');
assert.equal(formatLastActivity(local(2026, 7, 28, 13, 31).toISOString(), now), '59m ago');
assert.equal(formatLastActivity(local(2026, 7, 28, 13, 30).toISOString(), now), '1h ago');
assert.equal(formatLastActivity(local(2026, 7, 28, 12, 30).toISOString(), now), '2h ago');
assert.equal(formatLastActivity(local(2026, 7, 27, 15, 0).toISOString(), now), '23h ago');

// Yesterday is a calendar day, not "24 hours ago": at 2am, work from 11pm the
// night before is still just three hours old and says so.
assert.equal(formatLastActivity(local(2026, 7, 27, 14, 0).toISOString(), now), 'yesterday');
assert.equal(formatLastActivity(local(2026, 7, 27, 0, 5).toISOString(), now), 'yesterday');
assert.equal(
  formatLastActivity(local(2026, 7, 27, 23, 0).toISOString(), local(2026, 7, 28, 2, 0)),
  '3h ago'
);

// Older than yesterday becomes a date. This year, that includes the time of
// day; a previous year drops it, because by then the hour has stopped
// mattering. The exact wording is the reader's locale, so the test asserts what
// is in it rather than one country's punctuation.
{
  const thisYear = formatLastActivity(local(2026, 7, 25, 21, 16).toISOString(), now);
  assert.match(thisYear, /Jul/, 'the month is named');
  assert.match(thisYear, /25/, 'the day is there');
  assert.doesNotMatch(thisYear, /2026/, 'this year is not worth saying');
  assert.match(thisYear, /\d:\d\d/, 'the time of day is there');

  const olderYear = formatLastActivity(local(2025, 7, 27, 21, 16).toISOString(), now);
  assert.match(olderYear, /Jul/);
  assert.match(olderYear, /27/);
  assert.match(olderYear, /2025/, 'a different year has to be said');
  assert.doesNotMatch(olderYear, /\d:\d\d/, 'the hour has stopped mattering by then');
}

// It reads the reader's clock, not UTC. A stamp written in UTC is shown at the
// hour it happened HERE — that is the bug this module exists to fix.
{
  const utcStamp = '2026-07-25T21:16:04.312Z';
  const expected = new Date(utcStamp).toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit'
  });
  assert.equal(formatLastActivity(utcStamp, now), expected);
  assert.doesNotMatch(formatLastActivity(utcStamp, now), /T|Z/, 'no raw ISO text survives');
}

// A stamp from the future means the clocks disagree, not that something is
// scheduled. Better to say the least misleading thing than "-3m ago".
assert.equal(formatLastActivity(local(2026, 7, 28, 15, 0).toISOString(), now), 'just now');
assert.equal(formatLastActivity(local(2027, 1, 1, 0, 0).toISOString(), now), 'just now');

// Nothing to show is shown as nothing. Printing the raw text back at the user
// is what this module replaced.
assert.equal(formatLastActivity(null, now), '');
assert.equal(formatLastActivity(undefined, now), '');
assert.equal(formatLastActivity('', now), '');
assert.equal(formatLastActivity('   ', now), '');
assert.equal(formatLastActivity('not a date', now), '');

// Codex writes microseconds and Claude writes milliseconds; both parse.
assert.equal(typeof formatLastActivity('2026-03-13T21:48:02.611673Z', now), 'string');
assert.notEqual(formatLastActivity('2026-03-13T21:48:02.611673Z', now), '');

// The exact moment stays one hover away from the shortened stamp, in local
// time and with nothing unparseable getting through.
{
  const utcStamp = '2026-07-25T21:16:04.312Z';
  assert.equal(exactLocalTime(utcStamp), new Date(utcStamp).toLocaleString());
  assert.doesNotMatch(exactLocalTime(utcStamp), /T\d\d:|Z$/, 'no raw ISO text survives');
  assert.equal(exactLocalTime(null), '');
  assert.equal(exactLocalTime(''), '');
  assert.equal(exactLocalTime('not a date'), '');
}

console.log('relativeTime: all tests passed');
