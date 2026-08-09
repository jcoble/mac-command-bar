import {
  readCurrentProviderUsage,
  readUsageBreakdown,
  readUsageCostInputs,
  readUsageDaily,
  readUsageDailyTotals,
  readUsageProviderDailyTotals,
  readUsageProviderSummary,
  readUsageSummary,
  readUsageTokenBreakdown,
  refreshUsageHistory
} from './usageBackend.ts';
import type {
  ProviderUsageSnapshot,
  UsageBreakdownRow,
  UsageCostInputRow,
  UsageDailyRow,
  UsageDailyTotalsRow,
  UsageHistoryQuery,
  UsageProviderDailyTotalsRow,
  UsageProviderSummaryRow,
  UsageSummary,
  UsageTokenBreakdown
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
  readProviderSummary(query?: UsageHistoryQuery): Promise<UsageProviderSummaryRow[] | null> {
    return readUsageProviderSummary(query);
  },
  readDaily(query?: UsageHistoryQuery): Promise<UsageDailyRow[] | null> {
    return readUsageDaily(query);
  },
  readDailyTotals(query?: UsageHistoryQuery): Promise<UsageDailyTotalsRow[] | null> {
    return readUsageDailyTotals(query);
  },
  readTokenBreakdown(query?: UsageHistoryQuery): Promise<UsageTokenBreakdown | null> {
    return readUsageTokenBreakdown(query);
  },
  readProviderDailyTotals(query?: UsageHistoryQuery): Promise<UsageProviderDailyTotalsRow[] | null> {
    return readUsageProviderDailyTotals(query);
  },
  readCostInputs(query?: UsageHistoryQuery): Promise<UsageCostInputRow[] | null> {
    return readUsageCostInputs(query);
  },
  refreshHistory(): Promise<number | null> {
    return refreshUsageHistory();
  }
};
