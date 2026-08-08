import {
  readCurrentProviderUsage,
  readUsageBreakdown,
  readUsageDaily,
  readUsageSummary,
  refreshUsageHistory
} from './usageBackend.ts';
import type {
  ProviderUsageSnapshot,
  UsageBreakdownRow,
  UsageDailyRow,
  UsageHistoryQuery,
  UsageSummary
} from './usageTypes.ts';

const currentInFlight = new Map<string, Promise<ProviderUsageSnapshot | null>>();

export const usageService = {
  readCurrent(provider: string | null, instanceId: string | null): Promise<ProviderUsageSnapshot | null> {
    const key = `${provider ?? 'unknown'}:${instanceId ?? 'unknown'}`;
    const existing = currentInFlight.get(key);
    if (existing) return existing;
    const request = readCurrentProviderUsage(provider, instanceId).finally(() => {
      currentInFlight.delete(key);
    });
    currentInFlight.set(key, request);
    return request;
  },
  readSummary(query?: UsageHistoryQuery): Promise<UsageSummary | null> {
    return readUsageSummary(query);
  },
  readBreakdown(query?: UsageHistoryQuery): Promise<UsageBreakdownRow[] | null> {
    return readUsageBreakdown(query);
  },
  readDaily(query?: UsageHistoryQuery): Promise<UsageDailyRow[] | null> {
    return readUsageDaily(query);
  },
  refreshHistory(): Promise<number | null> {
    return refreshUsageHistory();
  }
};
