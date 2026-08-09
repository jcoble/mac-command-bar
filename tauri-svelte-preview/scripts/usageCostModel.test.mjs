import assert from 'node:assert/strict';
import {
  estimateUsageCostMicros,
  USAGE_COST_RATES,
  USAGE_COST_RATE_VERSION,
  usageRatesForModel
} from '../src/lib/shell/usage/usageCostModel.ts';

assert.match(USAGE_COST_RATE_VERSION, /^published-api-rates-/);
assert.ok(USAGE_COST_RATES.length >= 5);
assert.equal(usageRatesForModel('gpt-5.6-sol')?.outputUsdPerMillion, 30);
assert.equal(usageRatesForModel('claude-opus-4-8-20260801')?.cacheReadUsdPerMillion, 0.5);
assert.equal(usageRatesForModel('unknown-model'), null);

assert.equal(
  estimateUsageCostMicros([
    {
      provider: 'codex',
      model: 'gpt-5.6-sol',
      inputTokens: 1_000_000,
      outputTokens: 1_000_000,
      cacheReadTokens: 1_000_000,
      cacheWriteTokens: 1_000_000,
      reasoningTokens: 9_000_000
    }
  ]),
  41_750_000
);
assert.equal(
  estimateUsageCostMicros([
    {
      provider: 'local',
      model: 'unknown-model',
      inputTokens: 1,
      outputTokens: 0,
      cacheReadTokens: 0,
      cacheWriteTokens: 0,
      reasoningTokens: 0
    }
  ]),
  null
);
assert.equal(estimateUsageCostMicros([]), 0);

console.log('usage cost model tests passed');
