import type {
  ProviderUsageSnapshot,
  UsageBreakdownRow,
  UsageDailyRow,
  UsageSummary
} from './usageTypes.ts';
import { usageService } from './usageService.ts';

export const usageState = $state<{
  current: ProviderUsageSnapshot | null;
  summary: UsageSummary | null;
  breakdown: UsageBreakdownRow[];
  daily: UsageDailyRow[];
  loading: boolean;
  error: string | null;
  unavailableReason: string | null;
}>({
  current: null,
  summary: null,
  breakdown: [],
  daily: [],
  loading: false,
  error: null,
  unavailableReason: null
});

export async function refreshCurrentUsage(provider: string | null, instanceId: string | null): Promise<ProviderUsageSnapshot | null> {
  if (usageState.loading) return usageState.current;
  usageState.loading = true;
  usageState.error = null;
  try {
    const current = await usageService.readCurrent(provider, instanceId);
    usageState.current = current;
    usageState.unavailableReason = current?.state === 'unavailable' ? current.unavailableReason : current ? null : 'Usage is available in the desktop app.';
    return current;
  } catch (error) {
    usageState.error = error instanceof Error ? error.message : 'Usage could not be read.';
    return null;
  } finally {
    usageState.loading = false;
  }
}

export async function refreshUsageHistory(): Promise<UsageSummary | null> {
  try {
    const [summary, breakdown, daily] = await Promise.all([
      usageService.readSummary(),
      usageService.readBreakdown({ limit: 20, offset: 0 }),
      usageService.readDaily({ limit: 31, offset: 0 })
    ]);
    usageState.summary = summary;
    usageState.breakdown = breakdown ?? [];
    usageState.daily = daily ?? [];
    return summary;
  } catch (error) {
    usageState.error = error instanceof Error ? error.message : 'Usage history could not be read.';
    return null;
  }
}
