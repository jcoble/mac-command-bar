import type {
  ProviderUsageSnapshot,
  UsageDailyRow,
  UsageProviderSummaryRow,
  UsageSummary
} from './usageTypes.ts';
import { usageService } from './usageService.ts';

export const usageState = $state<{
  current: ProviderUsageSnapshot | null;
  currentByProvider: Record<string, ProviderUsageSnapshot>;
  summary: UsageSummary | null;
  providerSummary: UsageProviderSummaryRow[];
  daily: UsageDailyRow[];
  loading: boolean;
  currentLoading: boolean;
  historyLoading: boolean;
  error: string | null;
  unavailableReason: string | null;
  viewMode: 'detailed' | 'compact';
}>({
  current: null,
  currentByProvider: {},
  summary: null,
  providerSummary: [],
  daily: [],
  loading: false,
  currentLoading: false,
  historyLoading: false,
  error: null,
  unavailableReason: null,
  viewMode: 'detailed'
});

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
    usageState.error = error instanceof Error ? error.message : 'Usage could not be read.';
    return null;
  } finally {
    usageState.currentLoading = false;
    usageState.loading = usageState.historyLoading;
  }
}

export async function refreshUsageHistory(): Promise<UsageSummary | null> {
  if (usageState.historyLoading) return usageState.summary;
  usageState.historyLoading = true;
  usageState.loading = true;
  usageState.error = null;
  try {
    await usageService.refreshHistory();
    const [summary, providerSummary, daily] = await Promise.all([
      usageService.readSummary(),
      usageService.readProviderSummary({ limit: 20, offset: 0 }),
      usageService.readDaily({ limit: 31, offset: 0 })
    ]);
    usageState.summary = summary;
    usageState.providerSummary = providerSummary ?? [];
    usageState.daily = daily ?? [];
    return summary;
  } catch (error) {
    usageState.error = error instanceof Error ? error.message : 'Usage history could not be read.';
    return null;
  } finally {
    usageState.historyLoading = false;
    usageState.loading = usageState.currentLoading;
  }
}
