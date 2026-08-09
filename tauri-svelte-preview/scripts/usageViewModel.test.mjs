import assert from 'node:assert/strict';
import fs from 'node:fs';
import { usagePercent, usageProviderLabel, usageResetLabel } from '../src/lib/shell/usage/usageCurrent.ts';
import { usageSummaryLabel } from '../src/lib/shell/usage/usageAnalytics.ts';

globalThis.$state = (value) => value;
const { describeUsageError } = await import('../src/lib/shell/usage/usageStore.svelte.ts');

function testUsageErrorsPreserveStringRejections() {
  // describeUsageError must pass Tauri string rejections through untouched.
  assert.equal(describeUsageError('Usage history database could not open: disk failure'), 'Usage history database could not open: disk failure');
  assert.equal(describeUsageError(new Error('boom')), 'boom');
  assert.equal(describeUsageError({ weird: true }), 'Usage history could not be read.');
}

testUsageErrorsPreserveStringRejections();

assert.equal(usageProviderLabel('codex'), 'Codex');
assert.equal(usagePercent({ usedPercent: 75 }), 75);
assert.equal(usagePercent({ usedPercent: 125 }), 100);
assert.match(usageResetLabel(String(Date.now() + 3_600_000)), /^Resets in 1h/);
assert.equal(usageSummaryLabel({ inputTokens: 12, outputTokens: 8 }), '20 tokens');

const usageWorkspaceSource = fs.readFileSync(new URL('../src/lib/shell/usage/UsageWorkspace.svelte', import.meta.url), 'utf8');
const usageStoreSource = fs.readFileSync(new URL('../src/lib/shell/usage/usageStore.svelte.ts', import.meta.url), 'utf8');
const usageBackendSource = fs.readFileSync(new URL('../src/lib/shell/usage/usageBackend.ts', import.meta.url), 'utf8');
const usageDbSource = fs.readFileSync(new URL('../src-tauri/src/usage_db.rs', import.meta.url), 'utf8');
const mainSource = fs.readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const tauriSource = fs.readFileSync(new URL('../src/lib/tauriSource.ts', import.meta.url), 'utf8');
assert.match(usageWorkspaceSource, /usageState\.providerSummary/);
assert.match(usageWorkspaceSource, /card\.sessionCount/);
assert.match(usageWorkspaceSource, /usageRangeOptions/);
assert.match(usageWorkspaceSource, /Best: \{formatDay\(bestDay\)\}/);
assert.match(usageWorkspaceSource, /No activity in/);
assert.match(usageWorkspaceSource, /Costs are estimates, not billing totals/);
assert.doesNotMatch(usageWorkspaceSource, /usageState\.daily\.slice\(\)\.reverse\(\)/);
assert.doesNotMatch(usageWorkspaceSource, /\.reduce\(/);
assert.match(usageStoreSource, /dailyTotals/);
assert.match(usageStoreSource, /readDailyTotals/);
assert.match(usageStoreSource, /exactly three DB-side aggregate queries/);
assert.match(usageStoreSource, /selectUsageRange/);
assert.match(tauriSource, /read_usage_daily_totals/);
assert.match(usageBackendSource, /read_usage_token_breakdown/);
assert.match(usageBackendSource, /read_usage_provider_daily_totals/);
assert.match(usageBackendSource, /read_usage_cost_inputs/);
assert.match(usageDbSource, /MAX\(total_tokens\) OVER \(\)/);
assert.match(usageDbSource, /ROW_NUMBER\(\) OVER \(PARTITION BY provider/);
assert.match(mainSource, /read_usage_token_breakdown,/);
assert.match(mainSource, /read_usage_provider_daily_totals,/);
assert.match(mainSource, /read_usage_cost_inputs,/);
console.log('usage view-model tests passed');
