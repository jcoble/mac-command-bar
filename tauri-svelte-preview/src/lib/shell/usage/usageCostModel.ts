import type { UsageCostInputRow } from './usageTypes.ts';

export type UsageTokenRates = {
  id: string;
  models: readonly string[];
  inputUsdPerMillion: number;
  outputUsdPerMillion: number;
  cacheReadUsdPerMillion: number;
  cacheWriteUsdPerMillion: number;
};

export const USAGE_COST_RATE_VERSION = 'published-api-rates-2026-08-09';

export const USAGE_COST_SOURCES = [
  'https://openai.com/index/gpt-5-6/',
  'https://platform.claude.com/docs/en/about-claude/pricing'
] as const;

/**
 * Published standard API prices in USD per million tokens. The generic
 * `codex` model is retained for events indexed before transcript model capture
 * and uses the current Sol rate. These are directional estimates, not billing.
 */
export const USAGE_COST_RATES: readonly UsageTokenRates[] = [
  {
    id: 'gpt-5.6-sol',
    models: ['gpt-5.6-sol', 'codex'],
    inputUsdPerMillion: 5,
    outputUsdPerMillion: 30,
    cacheReadUsdPerMillion: 0.5,
    cacheWriteUsdPerMillion: 6.25
  },
  {
    id: 'gpt-5.6-terra',
    models: ['gpt-5.6-terra'],
    inputUsdPerMillion: 2.5,
    outputUsdPerMillion: 15,
    cacheReadUsdPerMillion: 0.25,
    cacheWriteUsdPerMillion: 3.125
  },
  {
    id: 'gpt-5.6-luna',
    models: ['gpt-5.6-luna'],
    inputUsdPerMillion: 1,
    outputUsdPerMillion: 6,
    cacheReadUsdPerMillion: 0.1,
    cacheWriteUsdPerMillion: 1.25
  },
  {
    id: 'gpt-5.5',
    models: ['gpt-5.5'],
    inputUsdPerMillion: 5,
    outputUsdPerMillion: 30,
    cacheReadUsdPerMillion: 0.5,
    cacheWriteUsdPerMillion: 6.25
  },
  {
    id: 'opus-current',
    models: [
      'claude-opus-5',
      'claude-opus-4-8',
      'claude-opus-4.8',
      'claude-opus-4-7',
      'claude-opus-4.7',
      'claude-opus-4-6',
      'claude-opus-4.6',
      'claude-opus-4-5',
      'claude-opus-4.5'
    ],
    inputUsdPerMillion: 5,
    outputUsdPerMillion: 25,
    cacheReadUsdPerMillion: 0.5,
    cacheWriteUsdPerMillion: 6.25
  },
  {
    id: 'sonnet-5-introductory',
    models: ['claude-sonnet-5'],
    inputUsdPerMillion: 2,
    outputUsdPerMillion: 10,
    cacheReadUsdPerMillion: 0.2,
    cacheWriteUsdPerMillion: 2.5
  },
  {
    id: 'sonnet-4.6',
    models: ['claude-sonnet-4-6', 'claude-sonnet-4.6'],
    inputUsdPerMillion: 3,
    outputUsdPerMillion: 15,
    cacheReadUsdPerMillion: 0.3,
    cacheWriteUsdPerMillion: 3.75
  },
  {
    id: 'haiku-4.5',
    models: ['claude-haiku-4-5', 'claude-haiku-4.5'],
    inputUsdPerMillion: 1,
    outputUsdPerMillion: 5,
    cacheReadUsdPerMillion: 0.1,
    cacheWriteUsdPerMillion: 1.25
  }
] as const;

export function usageRatesForModel(model: string): UsageTokenRates | null {
  const normalized = model.trim().toLowerCase();
  return USAGE_COST_RATES.find((rate) =>
    rate.models.some((candidate) => normalized === candidate || normalized.startsWith(`${candidate}-`))
  ) ?? null;
}

export function estimateUsageCostMicros(rows: readonly UsageCostInputRow[]): number | null {
  let totalMicros = 0;
  for (const row of rows) {
    const rates = usageRatesForModel(row.model);
    const billableTokens = row.inputTokens + row.outputTokens + row.cacheReadTokens + row.cacheWriteTokens;
    if (!rates) {
      if (billableTokens > 0) return null;
      continue;
    }
    totalMicros += row.inputTokens * rates.inputUsdPerMillion;
    totalMicros += row.outputTokens * rates.outputUsdPerMillion;
    totalMicros += row.cacheReadTokens * rates.cacheReadUsdPerMillion;
    totalMicros += row.cacheWriteTokens * rates.cacheWriteUsdPerMillion;
  }
  return Math.round(totalMicros);
}
