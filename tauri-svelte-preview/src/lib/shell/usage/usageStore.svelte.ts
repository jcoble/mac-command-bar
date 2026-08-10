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

export const USAGE_REQUEST_TIMEOUT_MS = 15_000;

export function settleUsageRequest<T>(request: Promise<T>, timeoutMs = USAGE_REQUEST_TIMEOUT_MS): Promise<T> {
  let timeoutId: ReturnType<typeof setTimeout> | undefined;
  const timeout = new Promise<T>((_resolve, reject) => {
    timeoutId = setTimeout(() => reject(new Error('Usage request timed out.')), timeoutMs);
  });
  return Promise.race([request, timeout]).finally(() => {
    if (timeoutId) clearTimeout(timeoutId);
  });
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
    const results = await Promise.allSettled(
      providers.map((name) => settleUsageRequest(usageService.readCurrent(name, instanceId ?? 'local')))
    );
    const snapshots = results.flatMap((result) => result.status === 'fulfilled' ? [result.value] : []);
    if (snapshots.length === 0) {
      const failure = results.find((result) => result.status === 'rejected');
      if (failure?.status === 'rejected') throw failure.reason;
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
  const request = (async () => {
    const query = usageRangeQuery(range);
    const heatmapQuery = usageHeatmapQuery(range);
    const rollupRanges = ['today', 'yesterday', '30-days'] as const;
    const rollupRequests = rollupRanges.map((rollupRange) =>
      usageService.readProviderSummary({ ...usageRangeQuery(rollupRange), limit: 20, offset: 0 })
    );
    const selectedProviderSummary = range === 'all'
      ? usageService.readProviderSummary({ ...query, limit: 20, offset: 0 })
      : rollupRequests[rollupRanges.indexOf(range)];
    // Every displayed metric is returned as a bounded DB-side aggregate. No
    // event or turn rows cross IPC, and no displayed totals are summed in JS.
    const [summary, providerSummary, dailyTotals, providerDailyTotals, ...rollups] = await settleUsageRequest(
      Promise.all([
        usageService.readSummary(query),
        selectedProviderSummary,
        usageService.readDailyTotals({ ...heatmapQuery, limit: 42, offset: 0 }),
        usageService.readProviderDailyTotals({ ...usageRangeQuery('30-days'), limit: 200, offset: 0 }),
        ...rollupRequests
      ])
    );
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
  })().catch((error) => {
    usageState.error = describeUsageError(error);
    return null;
  }).finally(() => {
    usageState.historyLoading = false;
    historyLoadRequest = null;
  });
  historyLoadRequest = request;
  return request;
}

function refreshUsageIndexInBackground(): void {
  if (historyRefreshRequest) return;
  usageState.historyRefreshing = true;
  historyRefreshRequest = usageService.refreshHistory()
    .then(async () => {
      await loadUsageHistory(usageState.range);
    })
    .catch((error) => {
      usageState.error = describeUsageError(error);
    })
    .finally(() => {
      usageState.historyRefreshing = false;
      historyRefreshRequest = null;
    });
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
