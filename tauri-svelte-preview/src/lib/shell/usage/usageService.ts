import {
  readCurrentProviderUsage,
  readUsageBreakdown,
  readUsageDaily,
  readUsageSummary
} from './usageBackend.ts';
import type {
  ProviderUsageSnapshot,
  UsageBreakdownRow,
  UsageDailyRow,
  UsageHistoryQuery,
  UsageSummary
} from './usageTypes.ts';

let currentInFlight: Promise<ProviderUsageSnapshot | null> | null = null;

export const usageService = {
  readCurrent(provider: string | null, instanceId: string | null): Promise<ProviderUsageSnapshot | null> {
    if (!currentInFlight) {
      currentInFlight = readCurrentProviderUsage(provider, instanceId).finally(() => {
        currentInFlight = null;
      });
    }
    return currentInFlight;
  },
  readSummary(query?: UsageHistoryQuery): Promise<UsageSummary | null> {
    return readUsageSummary(query);
  },
  readBreakdown(query?: UsageHistoryQuery): Promise<UsageBreakdownRow[] | null> {
    return readUsageBreakdown(query);
  },
  readDaily(query?: UsageHistoryQuery): Promise<UsageDailyRow[] | null> {
    return readUsageDaily(query);
  }
};
