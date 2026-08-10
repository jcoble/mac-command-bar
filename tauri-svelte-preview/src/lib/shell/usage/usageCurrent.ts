import type { ProviderUsageSnapshot } from './usageTypes.ts';

type ProviderUsageWindowInput = {
  label?: string;
  name?: string;
  usedPercent?: number | null;
  percentConsumed?: number | null;
  percentRemaining?: number | null;
  resetsAt?: string | null;
  resetAt?: string | null;
  windowMinutes?: number | null;
  semantics?: string;
};

type ProviderUsageInput = Omit<Partial<ProviderUsageSnapshot>, 'windows'> & {
  windows?: ProviderUsageWindowInput[];
  inputTokens?: number;
};

export function usageWindowDurationLabel(windowMinutes: number | null | undefined): string {
  if (!Number.isFinite(windowMinutes) || !windowMinutes || windowMinutes <= 0) return 'Provider window';
  if (windowMinutes >= 6 * 24 * 60 && windowMinutes <= 8 * 24 * 60) return 'Weekly';
  if (windowMinutes < 60) return `${windowMinutes}-minute`;
  const hours = windowMinutes / 60;
  return `${Number.isInteger(hours) ? hours : hours.toFixed(1)}-hour`;
}

export function usageQuotaWindowLabel(window: { label: string; windowMinutes?: number | null }): string {
  const minutes = window.windowMinutes;
  if (minutes && minutes >= 6 * 24 * 60 && minutes <= 8 * 24 * 60) return 'Weekly (7-day) window';
  if (minutes === 5 * 60) return 'Session (5-hour) window';
  const label = window.label.trim() || 'Provider';
  return /window$/i.test(label) ? label : `${label} window`;
}

function reportedWindowMinutes(window: ProviderUsageWindowInput): number | null {
  if (Number.isFinite(window.windowMinutes) && window.windowMinutes && window.windowMinutes > 0) return window.windowMinutes;
  const semanticsMatch = window.semantics?.match(/\b(\d+)-minute\b/);
  return semanticsMatch ? Number(semanticsMatch[1]) : null;
}

function normalizeProviderWindow(window: ProviderUsageWindowInput) {
  const windowMinutes = reportedWindowMinutes(window);
  const usedPercent = window.usedPercent ?? window.percentConsumed ?? (window.percentRemaining == null ? 0 : 100 - window.percentRemaining);
  return {
    label: windowMinutes == null ? window.label?.trim() || window.name?.trim() || 'Provider window' : usageWindowDurationLabel(windowMinutes),
    usedPercent: Math.max(0, Math.min(100, usedPercent)),
    resetsAt: window.resetsAt ?? window.resetAt ?? null,
    windowMinutes
  };
}

export function normalizeProviderUsage(input: ProviderUsageInput): ProviderUsageSnapshot {
  const state = input.state === 'available' && (input.windows?.length ?? 0) > 0 ? 'available' : input.state === 'error' ? 'error' : 'unavailable';
  return {
    provider: input.provider?.trim() || 'unknown',
    account: input.account ?? null,
    instanceId: input.instanceId?.trim() || 'unknown',
    state,
    windows: state === 'available' ? (input.windows ?? []).map(normalizeProviderWindow) : [],
    capturedAt: input.capturedAt ?? Date.now(),
    source: input.source ?? null,
    sourceVersion: input.sourceVersion ?? null,
    unavailableReason: state === 'available' ? null : input.unavailableReason ?? 'No provider quota data is available.'
  };
}

export function usageWindowLabel(window: { label: string; usedPercent: number }): string {
  return `${window.label}: ${Math.round(window.usedPercent)}% used`;
}

export function usageProviderLabel(provider: string): string {
  return provider.trim() ? provider.trim().replace(/[-_]+/g, ' ').replace(/\b\w/g, (character) => character.toUpperCase()) : 'Provider';
}

export function usageResetLabel(resetsAt: string | null): string {
  if (!resetsAt) return 'Reset unavailable';
  const parsed = Number(resetsAt);
  const target = Number.isFinite(parsed) ? (parsed < 10_000_000_000 ? parsed * 1000 : parsed) : Date.parse(resetsAt);
  if (!Number.isFinite(target)) return 'Reset unavailable';
  const delta = Math.max(0, target - Date.now());
  const minutes = Math.round(delta / 60_000);
  if (minutes < 60) return `Resets in ${minutes}m`;
  const hours = Math.floor(minutes / 60);
  const remainder = minutes % 60;
  return `Resets in ${hours}h${remainder ? ` ${remainder}m` : ''}`;
}

export function usagePercent(window: { usedPercent: number }): number {
  return Math.max(0, Math.min(100, window.usedPercent));
}
