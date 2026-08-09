import type {
  ProviderUsageSnapshot,
  UsageDailyRow,
  UsageDailyTotalsRow,
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
  daily: UsageDailyRow[];
  dailyTotals: UsageDailyTotalsRow[];
  loading: boolean;
  currentLoading: boolean;
  historyLoading: boolean;
  error: string | null;
  unavailableReason: string | null;
  viewMode: 'detailed' | 'compact';
  range: UsageRange;
}>({
  current: null,
  currentByProvider: {},
  summary: null,
  providerSummary: [],
  daily: [],
  dailyTotals: [],
  loading: false,
  currentLoading: false,
  historyLoading: false,
  error: null,
  unavailableReason: null,
  viewMode: 'detailed',
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
  usageState.loading = true;
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
    usageState.loading = usageState.historyLoading;
  }
}

async function loadUsageHistory(range: UsageRange, refreshIndex: boolean): Promise<UsageSummary | null> {
  if (usageState.historyLoading) return usageState.summary;
  usageState.historyLoading = true;
  usageState.loading = true;
  usageState.error = null;
  usageState.range = range;
  try {
    if (refreshIndex) await usageService.refreshHistory();
    const query = usageRangeQuery(range);
    const heatmapQuery = usageHeatmapQuery(range);
    // One dashboard refresh is exactly three DB-side aggregate queries.
    const [summary, providerSummary, dailyTotals] = await Promise.all([
      usageService.readSummary(query),
      usageService.readProviderSummary({ ...query, limit: 20, offset: 0 }),
      usageService.readDailyTotals({ ...heatmapQuery, limit: 42, offset: 0 })
    ]);
    usageState.summary = summary;
    usageState.providerSummary = providerSummary ?? [];
    usageState.dailyTotals = dailyTotals ?? [];
    return summary;
  } catch (error) {
    usageState.error = describeUsageError(error);
    return null;
  } finally {
    usageState.historyLoading = false;
    usageState.loading = usageState.currentLoading;
  }
}

export async function refreshUsageHistory(): Promise<UsageSummary | null> {
  return loadUsageHistory(usageState.range, true);
}

export async function selectUsageRange(range: UsageRange): Promise<UsageSummary | null> {
  if (range === usageState.range && usageState.summary) return usageState.summary;
  return loadUsageHistory(range, false);
}
