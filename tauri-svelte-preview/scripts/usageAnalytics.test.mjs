import assert from 'node:assert/strict';
import { usageSummaryLabel, buildUsageBreakdownQuery } from '../src/lib/shell/usage/usageAnalytics.ts';

assert.equal(usageSummaryLabel({ inputTokens: 10, outputTokens: 4 }), '14 tokens');
const query = buildUsageBreakdownQuery({ provider: 'provider-a', limit: 20, offset: 0 });
assert.match(query, /GROUP BY/);
assert.match(query, /LIMIT/);
console.log('usage analytics tests passed');
