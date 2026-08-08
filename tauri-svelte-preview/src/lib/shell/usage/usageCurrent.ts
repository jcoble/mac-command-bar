import type { ProviderUsageSnapshot } from './usageTypes.ts';

export function normalizeProviderUsage(input: Partial<ProviderUsageSnapshot> & { inputTokens?: number }): ProviderUsageSnapshot {
  const state = input.state === 'available' && (input.windows?.length ?? 0) > 0 ? 'available' : input.state === 'error' ? 'error' : 'unavailable';
  return {
    provider: input.provider?.trim() || 'unknown',
    account: input.account ?? null,
    instanceId: input.instanceId?.trim() || 'unknown',
    state,
    windows: state === 'available' ? input.windows ?? [] : [],
    capturedAt: input.capturedAt ?? Date.now(),
    source: input.source ?? null,
    sourceVersion: input.sourceVersion ?? null,
    unavailableReason: state === 'available' ? null : input.unavailableReason ?? 'No provider quota data is available.'
  };
}

export function usageWindowLabel(window: { name: string; percentRemaining: number | null; resetAt: string | null }): string {
  const remaining = window.percentRemaining == null ? 'remaining unavailable' : `${Math.round(window.percentRemaining)}% remaining`;
  return `${window.name}: ${remaining}`;
}
