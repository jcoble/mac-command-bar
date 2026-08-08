import type { UsageHistoryQuery, UsageSummary } from './usageTypes.ts';

export function usageSummaryLabel(summary: Pick<UsageSummary, 'inputTokens' | 'outputTokens'>): string {
  const total = summary.inputTokens + summary.outputTokens;
  return `${total.toLocaleString('en-US')} tokens`;
}

/** A read-only SQL shape used in the review contract; the desktop backend runs it. */
export function buildUsageBreakdownQuery(query: UsageHistoryQuery = {}): string {
  const limit = Math.min(Math.max(query.limit ?? 20, 1), 200);
  const offset = Math.min(Math.max(query.offset ?? 0, 0), 100_000);
  const predicates = [
    query.provider ? `provider = '${query.provider.replaceAll("'", "''")}'` : null,
    query.model ? `model = '${query.model.replaceAll("'", "''")}'` : null,
    query.projectId ? `project_id = '${query.projectId.replaceAll("'", "''")}'` : null,
    query.workflowId ? `workflow_id = '${query.workflowId.replaceAll("'", "''")}'` : null
  ].filter((value): value is string => value !== null);
  const where = predicates.length > 0 ? predicates.join(' AND ') : '1 = 1';
  return `WITH filtered AS (SELECT provider, model, project_id, owned_id, turn_id, workflow_id, input_tokens, output_tokens FROM usage_events WHERE ${where}) SELECT provider, model, project_id, COUNT(*) AS event_count, COUNT(DISTINCT owned_id) AS session_count, COUNT(DISTINCT turn_id) AS turn_count, COUNT(DISTINCT workflow_id) AS workflow_count, SUM(input_tokens) AS input_tokens, SUM(output_tokens) AS output_tokens, COUNT(*) OVER () AS total_count FROM filtered GROUP BY provider, model, project_id ORDER BY provider ASC, model ASC LIMIT ${limit} OFFSET ${offset}`;
}
