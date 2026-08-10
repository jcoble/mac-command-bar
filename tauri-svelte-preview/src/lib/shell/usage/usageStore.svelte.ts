import type {
  ProviderUsageSnapshot,
  UsageCostInputRow,
  UsageDailyRow,
  UsageDailyTotalsRow,
  UsageProviderDailyTotalsRow,
  UsageProviderSummaryRow,
  UsageRange,
  UsageSummary
} from './usageTypes.ts';
import { usageHeatmapQuery, usageRangeQuery } from './usageAnalytics.ts';
import { usageService } from './usageService.ts';

export const usageState = $state<{
  current: ProviderUsageSnapshot | null;
  currentByProvider: Record<string, ProviderUsageSnapshot>;
  summary: UsageSummary | null;
  providerSummary: UsageProviderSummaryRow[];
  providerRollups: Record<'today' | 'yesterday' | '30-days', UsageProviderSummaryRow[]>;
  providerDailyTotals: UsageProviderDailyTotalsRow[];
  costInputs: UsageCostInputRow[];
  daily: UsageDailyRow[];
  dailyTotals: UsageDailyTotalsRow[];
  readonly loading: boolean;
  currentLoading: boolean;
  historyLoading: boolean;
  error: string | null;
  unavailableReason: string | null;
  range: UsageRange;
}>({
  current: null,
  currentByProvider: {},
  summary: null,
  providerSummary: [],
  providerRollups: { today: [], yesterday: [], '30-days': [] },
  providerDailyTotals: [],
  costInputs: [],
  daily: [],
  dailyTotals: [],
  get loading() { return this.currentLoading || this.historyLoading; },
  currentLoading: false,
  historyLoading: false,
  error: null,
  unavailableReason: null,
  range: '30-days'
});

export function describeUsageError(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  return 'Usage history could not be read.';
}

export async function refreshCurrentUsage(provider: string | null, instanceId: string | null): Promise<ProviderUsageSnapshot | null> {
  if (usageState.currentLoading) return usageState.current;
  usageState.currentLoading = true;
  usageState.error = null;
  try {
    const providers = provider ? [provider] : ['codex', 'claude'];
    const snapshots = await Promise.all(providers.map((name) => usageService.readCurrent(name, instanceId ?? 'local')));
    const current = snapshots.find((snapshot) => snapshot?.state === 'available') ?? snapshots[0] ?? null;
    for (const snapshot of snapshots) {
      if (snapshot) usageState.currentByProvider[snapshot.provider] = snapshot;
    }
    usageState.current = current;
    usageState.unavailableReason = current?.state === 'unavailable' ? current.unavailableReason : current ? null : 'Usage is available in the desktop app.';
    return current;
  } catch (error) {
    usageState.error = describeUsageError(error);
    return null;
  } finally {
    usageState.currentLoading = false;
  }
}

async function loadUsageHistory(range: UsageRange, refreshIndex: boolean): Promise<UsageSummary | null> {
  if (usageState.historyLoading) return usageState.summary;
  usageState.historyLoading = true;
  usageState.error = null;
  usageState.range = range;
  try {
    if (refreshIndex) await usageService.refreshHistory();
    const query = usageRangeQuery(range);
    const heatmapQuery = usageHeatmapQuery(range);
    const rollupRanges = ['today', 'yesterday', '30-days'] as const;
    const rollupRequests = rollupRanges.map((rollupRange) =>
      usageService.readProviderSummary({ ...usageRangeQuery(rollupRange), limit: 20, offset: 0 })
    );
    const selectedProviderSummary = range === 'all'
      ? usageService.readProviderSummary({ ...query, limit: 20, offset: 0 })
      : rollupRequests[rollupRanges.indexOf(range)];
    // The selected-range core remains exactly three DB-side aggregate queries;
    // fixed provider rollups, cost inputs, and daily trends are also DB-shaped.
    const [summary, providerSummary, dailyTotals, providerDailyTotals, costInputs, ...rollups] = await Promise.all([
      usageService.readSummary(query),
      selectedProviderSummary,
      usageService.readDailyTotals({ ...heatmapQuery, limit: 42, offset: 0 }),
      usageService.readProviderDailyTotals({ ...usageRangeQuery('30-days'), limit: 200, offset: 0 }),
      usageService.readCostInputs({ ...query, limit: 200, offset: 0 }),
      ...rollupRequests
    ]);
    usageState.summary = summary;
    usageState.providerSummary = providerSummary ?? [];
    usageState.dailyTotals = dailyTotals ?? [];
    usageState.providerDailyTotals = providerDailyTotals ?? [];
    usageState.costInputs = costInputs ?? [];
    usageState.providerRollups = {
      today: rollups[0] ?? [],
      yesterday: rollups[1] ?? [],
      '30-days': rollups[2] ?? []
    };
    return summary;
  } catch (error) {
    usageState.error = describeUsageError(error);
    return null;
  } finally {
    usageState.historyLoading = false;
  }
}

export async function refreshUsageHistory(): Promise<UsageSummary | null> {
  return loadUsageHistory(usageState.range, true);
}

export async function selectUsageRange(range: UsageRange): Promise<UsageSummary | null> {
  if (range === usageState.range && usageState.summary) return usageState.summary;
  return loadUsageHistory(range, false);
}
