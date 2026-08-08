<script lang="ts">
  import { refreshUsageHistory, usageState } from './usageStore.svelte.ts';
  import { usageSummaryLabel } from './usageAnalytics.ts';

  function costLabel(value: number | null): string { return value == null ? 'Estimate unavailable' : `Estimated cost ${(value / 1_000_000).toFixed(4)}`; }
</script>

<section class="workspace" data-testid="usage-workspace" aria-label="Usage history workspace">
  <header><div><p class="eyebrow">Usage history</p><h2>Provider activity</h2><p class="muted">History is indexed incrementally and filtered in SQLite before it reaches this view.</p></div><button type="button" onclick={() => void refreshUsageHistory()} disabled={usageState.loading}>Refresh</button></header>
  {#if usageState.summary}<div class="summary"><span>{usageSummaryLabel(usageState.summary)}</span><span>{usageState.summary.eventCount} events</span><span>{usageState.summary.activeDays} active days</span><span>{usageState.summary.sessionCount} sessions</span><span>{usageState.summary.turnCount} turns</span><span>{usageState.summary.workflowCount} workflows</span><span>{costLabel(usageState.summary.estimatedCostMicros)}</span></div><div class="table" role="table"><div class="row heading"><span>Provider</span><span>Model</span><span>Events</span><span>Sessions</span><span>Tokens</span></div>{#each usageState.breakdown as row (`${row.provider}:${row.model}:${row.projectId ?? ''}`)}<div class="row"><span>{row.provider}</span><span>{row.model}</span><span>{row.eventCount}</span><span>{row.sessionCount}</span><span>{(row.inputTokens + row.outputTokens).toLocaleString('en-US')}</span></div>{/each}</div>{:else}<p class="empty">{usageState.error ?? 'Refresh to load usage history.'}</p>{/if}
</section>

<style>
  .workspace { display: grid; gap: 1rem; padding: 1rem; color: var(--color-text, #eef0f9); } header { display: flex; justify-content: space-between; align-items: start; gap: 1rem; } h2, p { margin: 0; } .eyebrow { text-transform: uppercase; letter-spacing: 0.08em; font-size: 0.72rem; color: var(--color-muted, #a6a7b8); } .muted, .empty { color: var(--color-muted, #a6a7b8); } button { border: 1px solid var(--color-border, #858599); background: var(--color-surface, #17171d); color: inherit; border-radius: 6px; padding: 0.45rem 0.65rem; font: inherit; cursor: pointer; } button:disabled { opacity: 0.55; cursor: not-allowed; } .summary { display: flex; flex-wrap: wrap; gap: 1rem; color: var(--color-muted, #a6a7b8); } .table { border: 1px solid var(--color-border-subtle, #383844); border-radius: 8px; overflow: auto; } .row { display: grid; grid-template-columns: 1fr 1fr 6rem 6rem 8rem; gap: 0.8rem; padding: 0.65rem 0.8rem; border-top: 1px solid var(--color-border-subtle, #383844); min-width: 34rem; } .row:first-child { border-top: 0; } .heading { color: var(--color-muted, #a6a7b8); font-size: 0.78rem; }
</style>
