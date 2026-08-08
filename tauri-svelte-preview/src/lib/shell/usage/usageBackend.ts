import {
  readCurrentProviderUsageFromTauri,
  readUsageBreakdownFromTauri,
  readUsageDailyFromTauri,
  readUsageSummaryFromTauri,
  refreshUsageHistoryFromTauri
} from '../../tauriSource.ts';
import type {
  ProviderUsageSnapshot,
  UsageBreakdownRow,
  UsageDailyRow,
  UsageHistoryQuery,
  UsageSummary
} from './usageTypes.ts';

export async function readCurrentProviderUsage(
  provider: string | null,
  instanceId: string | null
): Promise<ProviderUsageSnapshot | null> {
  return readCurrentProviderUsageFromTauri(provider, instanceId);
}

export async function readUsageSummary(query: UsageHistoryQuery = {}): Promise<UsageSummary | null> {
  return readUsageSummaryFromTauri(query);
}

export async function readUsageBreakdown(query: UsageHistoryQuery = {}): Promise<UsageBreakdownRow[] | null> {
  return readUsageBreakdownFromTauri(query);
}

export async function readUsageDaily(query: UsageHistoryQuery = {}): Promise<UsageDailyRow[] | null> {
  return readUsageDailyFromTauri(query);
}

export async function refreshUsageHistory(): Promise<number | null> {
  return refreshUsageHistoryFromTauri();
}
