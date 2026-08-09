import assert from 'node:assert/strict';
import fs from 'node:fs';
import { usagePercent, usageProviderLabel, usageResetLabel } from '../src/lib/shell/usage/usageCurrent.ts';
import { usageSummaryLabel } from '../src/lib/shell/usage/usageAnalytics.ts';

globalThis.$state = (value) => value;
const { describeUsageError } = await import('../src/lib/shell/usage/usageStore.svelte.ts');

function testUsageErrorsPreserveStringRejections() {
  // describeUsageError must pass Tauri string rejections through untouched.
  assert.equal(describeUsageError('SQLite is unavailable (sqlite3): spawn failed'), 'SQLite is unavailable (sqlite3): spawn failed');
  assert.equal(describeUsageError(new Error('boom')), 'boom');
  assert.equal(describeUsageError({ weird: true }), 'Usage history could not be read.');
}

testUsageErrorsPreserveStringRejections();

assert.equal(usageProviderLabel('codex'), 'Codex');
assert.equal(usagePercent({ percentConsumed: null, percentRemaining: 25 }), 75);
assert.equal(usagePercent({ percentConsumed: 125, percentRemaining: null }), 100);
assert.match(usageResetLabel(String(Date.now() + 3_600_000)), /^Resets in 1h/);
assert.equal(usageSummaryLabel({ inputTokens: 12, outputTokens: 8 }), '20 tokens');

const usageWorkspaceSource = fs.readFileSync(new URL('../src/lib/shell/usage/UsageWorkspace.svelte', import.meta.url), 'utf8');
const usageStoreSource = fs.readFileSync(new URL('../src/lib/shell/usage/usageStore.svelte.ts', import.meta.url), 'utf8');
const tauriSource = fs.readFileSync(new URL('../src/lib/tauriSource.ts', import.meta.url), 'utf8');
assert.match(usageWorkspaceSource, /usageState\.providerSummary/);
assert.match(usageWorkspaceSource, /card\.sessionCount/);
assert.match(usageWorkspaceSource, /usageState\.dailyTotals\.slice\(\)\.reverse\(\) as row \(row\.day\)/);
assert.match(usageWorkspaceSource, /usageState\.dailyTotals\.map/);
assert.doesNotMatch(usageWorkspaceSource, /usageState\.daily\.slice\(\)\.reverse\(\)/);
assert.doesNotMatch(usageWorkspaceSource, /\.reduce\(/);
assert.match(usageStoreSource, /dailyTotals/);
assert.match(usageStoreSource, /readDailyTotals/);
assert.match(tauriSource, /read_usage_daily_totals/);
console.log('usage view-model tests passed');
