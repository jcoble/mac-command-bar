import assert from 'node:assert/strict';
import { usagePercent, usageProviderLabel, usageResetLabel } from '../src/lib/shell/usage/usageCurrent.ts';
import { usageSummaryLabel } from '../src/lib/shell/usage/usageAnalytics.ts';

assert.equal(usageProviderLabel('codex'), 'Codex');
assert.equal(usagePercent({ percentConsumed: null, percentRemaining: 25 }), 75);
assert.equal(usagePercent({ percentConsumed: 125, percentRemaining: null }), 100);
assert.match(usageResetLabel(String(Date.now() + 3_600_000)), /^Resets in 1h/);
assert.equal(usageSummaryLabel({ inputTokens: 12, outputTokens: 8 }), '20 tokens');
console.log('usage view-model tests passed');
