import assert from 'node:assert/strict';
import { normalizeProviderUsage, usageWindowDurationLabel } from '../src/lib/shell/usage/usageCurrent.ts';

const unavailable = normalizeProviderUsage({ provider: 'local', state: 'unavailable', reason: 'No quota data' });
assert.equal(unavailable.state, 'unavailable');
assert.equal(unavailable.windows.length, 0);
assert.equal(normalizeProviderUsage({ provider: 'local', inputTokens: 1000 }).state, 'unavailable');

const weeklyOnly = normalizeProviderUsage({
  provider: 'codex',
  instanceId: 'local',
  state: 'available',
  windows: [
    {
      name: '5-hour',
      semantics: 'Provider-defined 10080-minute quota window',
      percentConsumed: 42,
      resetAt: '1786160562'
    }
  ]
});
assert.deepEqual(weeklyOnly.windows.map((window) => window.label), ['Weekly']);
assert.equal(weeklyOnly.windows[0].windowMinutes, 10080);

const fiveHourAndWeekly = normalizeProviderUsage({
  provider: 'codex',
  instanceId: 'local',
  state: 'available',
  windows: [
    { name: 'primary', semantics: 'Provider-defined 300-minute quota window', percentConsumed: 12, resetAt: '1783741721' },
    { name: 'secondary', semantics: 'Provider-defined 10080-minute quota window', percentConsumed: 17, resetAt: '1784310518' }
  ]
});
assert.deepEqual(fiveHourAndWeekly.windows.map((window) => window.label), ['5-hour', 'Weekly']);
assert.equal(usageWindowDurationLabel(720), '12-hour');
console.log('current usage tests passed');
