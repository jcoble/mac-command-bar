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

export function usageProviderLabel(provider: string): string {
  return provider.trim() ? provider.trim().replace(/[-_]+/g, ' ').replace(/\b\w/g, (character) => character.toUpperCase()) : 'Provider';
}

export function usageResetLabel(resetAt: string | null): string {
  if (!resetAt) return 'Reset unavailable';
  const parsed = Number(resetAt);
  const target = Number.isFinite(parsed) ? (parsed < 10_000_000_000 ? parsed * 1000 : parsed) : Date.parse(resetAt);
  if (!Number.isFinite(target)) return 'Reset unavailable';
  const delta = Math.max(0, target - Date.now());
  const minutes = Math.round(delta / 60_000);
  if (minutes < 60) return `Resets in ${minutes}m`;
  const hours = Math.floor(minutes / 60);
  const remainder = minutes % 60;
  return `Resets in ${hours}h${remainder ? ` ${remainder}m` : ''}`;
}

export function usagePercent(window: { percentConsumed: number | null; percentRemaining: number | null }): number {
  return Math.max(0, Math.min(100, window.percentConsumed ?? (window.percentRemaining == null ? 0 : 100 - window.percentRemaining)));
}
