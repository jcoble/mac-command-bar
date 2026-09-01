import type {
  ProviderUsageSnapshot,
  UsageDailyRow,
  UsageDailyTotalsRow,
  UsageProviderDailyTotalsRow,
  UsageProviderSummaryRow,
  UsageRange,
  UsageSummary
} from './usageTypes.ts';
import { usageHeatmapQuery, usageRangeQuery } from './usageAnalytics.ts';
import { usageService } from './usageService.ts';

export async function settleUsageRequest<T>(request: Promise<T>): Promise<T> {
  return request;
}

export const usageState = $state<{
  current: ProviderUsageSnapshot | null;
  currentByProvider: Record<string, ProviderUsageSnapshot>;
  summary: UsageSummary | null;
  providerSummary: UsageProviderSummaryRow[];
  providerRollups: Record<'today' | 'yesterday' | '30-days', UsageProviderSummaryRow[]>;
  providerDailyTotals: UsageProviderDailyTotalsRow[];
  daily: UsageDailyRow[];
  dailyTotals: UsageDailyTotalsRow[];
  readonly loading: boolean;
  currentLoading: boolean;
  historyLoading: boolean;
  historyRefreshing: boolean;
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
  daily: [],
  dailyTotals: [],
  get loading() { return this.currentLoading || this.historyLoading || this.historyRefreshing; },
  currentLoading: false,
  historyLoading: false,
  historyRefreshing: false,
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
    const snapshots: Array<ProviderUsageSnapshot | null> = [];
    let failure: unknown = null;
    for (const name of providers) {
      try {
        snapshots.push(await settleUsageRequest(usageService.readCurrent(name, instanceId ?? 'local')));
      } catch (error) {
        failure ??= error;
      }
    }
    if (snapshots.length === 0) {
      if (failure) throw failure;
    }
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

let historyLoadRequest: Promise<UsageSummary | null> | null = null;
let historyRefreshRequest: Promise<void> | null = null;

async function loadUsageHistory(range: UsageRange): Promise<UsageSummary | null> {
  if (historyLoadRequest) return historyLoadRequest;
  usageState.historyLoading = true;
  usageState.error = null;
  usageState.range = range;
  const request = loadUsageHistoryOnce(range);
  historyLoadRequest = request;
  return request;
}

async function loadUsageHistoryOnce(range: UsageRange): Promise<UsageSummary | null> {
  try {
    const query = usageRangeQuery(range);
    const heatmapQuery = usageHeatmapQuery(range);
    const rollupRanges = ['today', 'yesterday', '30-days'] as const;
    // Every displayed metric is returned as a bounded DB-side aggregate. No
    // event or turn rows cross IPC, and no displayed totals are summed in JS.
    const summary = await settleUsageRequest(usageService.readSummary(query));
    const rollups: UsageProviderSummaryRow[][] = [];
    for (const rollupRange of rollupRanges) {
      rollups.push(await usageService.readProviderSummary({ ...usageRangeQuery(rollupRange), limit: 20, offset: 0 }) ?? []);
    }
    const providerSummary = range === 'all'
      ? await usageService.readProviderSummary({ ...query, limit: 20, offset: 0 })
      : rollups[rollupRanges.indexOf(range)];
    const dailyTotals = await usageService.readDailyTotals({ ...heatmapQuery, limit: 42, offset: 0 });
    const providerDailyTotals = await usageService.readProviderDailyTotals({ ...usageRangeQuery('30-days'), limit: 200, offset: 0 });
    usageState.summary = summary;
    usageState.providerSummary = providerSummary ?? [];
    usageState.dailyTotals = dailyTotals ?? [];
    usageState.providerDailyTotals = providerDailyTotals ?? [];
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
    historyLoadRequest = null;
  }
}

function refreshUsageIndexInBackground(): void {
  if (historyRefreshRequest) return;
  usageState.historyRefreshing = true;
  historyRefreshRequest = refreshUsageIndexOnce();
}

async function refreshUsageIndexOnce(): Promise<void> {
  try {
    await usageService.refreshHistory();
    await loadUsageHistory(usageState.range);
  } catch (error) {
    usageState.error = describeUsageError(error);
  } finally {
    usageState.historyRefreshing = false;
    historyRefreshRequest = null;
  }
}

export async function refreshUsageHistory(): Promise<UsageSummary | null> {
  const cached = await loadUsageHistory(usageState.range);
  refreshUsageIndexInBackground();
  return cached;
}

export async function selectUsageRange(range: UsageRange): Promise<UsageSummary | null> {
  if (range === usageState.range && usageState.summary) return usageState.summary;
  return loadUsageHistory(range);
}
