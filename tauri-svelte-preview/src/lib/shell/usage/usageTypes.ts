export type ProviderUsageState = 'available' | 'unavailable' | 'error';

export type ProviderUsageWindow = {
  name: string;
  semantics: string;
  percentConsumed: number | null;
  percentRemaining: number | null;
  resetAt: string | null;
};

export type ProviderUsageSnapshot = {
  provider: string;
  account: string | null;
  instanceId: string;
  state: ProviderUsageState;
  windows: ProviderUsageWindow[];
  capturedAt: number;
  source: string | null;
  sourceVersion: string | null;
  unavailableReason: string | null;
};

export type UsageHistoryQuery = {
  provider?: string | null;
  model?: string | null;
  projectId?: string | null;
  workflowId?: string | null;
  startMicros?: number | null;
  endMicros?: number | null;
  limit?: number | null;
  offset?: number | null;
};

export type UsageSummary = {
  eventCount: number;
  providerCount: number;
  activeDays: number;
  sessionCount: number;
  turnCount: number;
  workflowCount: number;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheWriteTokens: number;
  reasoningTokens: number;
  estimatedCostMicros: number | null;
};

export type UsageBreakdownRow = {
  provider: string;
  model: string;
  projectId: string | null;
  eventCount: number;
  sessionCount: number;
  turnCount: number;
  workflowCount: number;
  inputTokens: number;
  outputTokens: number;
  totalCount: number;
};

export type UsageDailyRow = {
  day: string;
  provider: string;
  model: string;
  projectId: string | null;
  eventCount: number;
  sessionCount: number;
  turnCount: number;
  workflowCount: number;
  inputTokens: number;
  outputTokens: number;
  estimatedCostMicros: number | null;
};
