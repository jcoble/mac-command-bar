<script lang="ts">
  import { RefreshCw } from '@lucide/svelte';
  import {
    buildUsageHeatmap,
    usageCacheShare,
    usageRangeLabel,
    usageRangeOptions,
    usageTotalTokens
  } from './usageAnalytics.ts';
  import { estimateUsageCostMicros, USAGE_COST_RATE_VERSION } from './usageCostModel.ts';
  import { usageProviderLabel } from './usageCurrent.ts';
  import { refreshUsageHistory, selectUsageRange, usageState } from './usageStore.svelte.ts';
  import type { UsageProviderSummaryRow } from './usageTypes.ts';

  const providerNames = ['codex', 'claude'] as const;
  const integerFormat = new Intl.NumberFormat('en-US');
  const compactCurrency = new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    maximumFractionDigits: 0
  });
  const preciseCurrency = new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });

  let totalTokens = $derived(usageState.summary ? usageTotalTokens(usageState.summary) : 0);
  let cacheShare = $derived(usageState.summary
    ? usageCacheShare({ ...usageState.summary, totalTokens })
    : 0);
  let heatmapDays = $derived(buildUsageHeatmap(usageState.dailyTotals, usageState.range));
  let estimatedCostMicros = $derived(estimateUsageCostMicros(
    usageState.providerSummary.flatMap((provider) => provider.costInputs)
  ));
  let bestDay = $derived(usageState.dailyTotals[0]?.bestDay ?? null);

  function visibleTokens(input: number): string {
    if (input >= 1_000_000_000) return `${(input / 1_000_000_000).toFixed(1)}B`;
    if (input >= 1_000_000) return `${(input / 1_000_000).toFixed(1)}M`;
    if (input >= 1_000) return `${(input / 1_000).toFixed(1)}K`;
    return integerFormat.format(input);
  }

  function costLabel(value: number | null): string {
    if (value == null) return 'Unavailable';
    const dollars = value / 1_000_000;
    return dollars >= 100 ? compactCurrency.format(dollars) : preciseCurrency.format(dollars);
  }

  function formatDay(day: string): string {
    const parsed = new Date(`${day}T00:00:00`);
    return Number.isNaN(parsed.getTime())
      ? day
      : parsed.toLocaleDateString([], { month: 'short', day: 'numeric' });
  }

  function formatLastSeen(value: number | null): string {
    if (value == null) return 'Last seen unavailable';
    return `Last seen ${new Date(value / 1_000).toLocaleDateString([], { month: 'short', day: 'numeric' })}`;
  }

  function providerCard(provider: string): UsageProviderSummaryRow | null {
    return usageState.providerSummary.find((card) => card.provider === provider) ?? null;
  }

  function providerCost(card: UsageProviderSummaryRow): number | null {
    return estimateUsageCostMicros(card.costInputs);
  }

  function providerShare(card: UsageProviderSummaryRow): number {
    return card.rangeTotalTokens === 0 ? 0 : Math.round((card.totalTokens / card.rangeTotalTokens) * 100);
  }

  function sessionLabel(card: UsageProviderSummaryRow): string {
    const shortId = card.lastSessionId && card.lastSessionId.length > 12
      ? `${card.lastSessionId.slice(0, 8)}…`
      : card.lastSessionId;
    if (card.lastProjectId && shortId) return `${card.lastProjectId} · ${shortId}`;
    return card.lastProjectId ?? shortId ?? 'Session unavailable';
  }

  function mixWidth(value: number): number {
    return totalTokens === 0 ? 0 : (value / totalTokens) * 100;
  }
</script>

<section class="workspace" data-testid="usage-workspace" aria-label="Stats and Usage" aria-busy={usageState.loading}>
  <header class="workspace-header">
    <div class="title-block">
      <p class="eyebrow">Stats &amp; Usage</p>
      <h2>Usage at a glance</h2>
      <p class="muted">Local session activity, indexed privately on this Mac.</p>
    </div>
    <div class="header-actions">
      <div class="range-selector" aria-label="Usage date range">
        {#each usageRangeOptions as option (option.value)}
          <button
            type="button"
            class:active={usageState.range === option.value}
            aria-pressed={usageState.range === option.value}
            disabled={usageState.loading}
            onclick={() => void selectUsageRange(option.value)}
          >{option.label}</button>
        {/each}
      </div>
      <button class="refresh" type="button" onclick={() => void refreshUsageHistory()} disabled={usageState.loading}>
        <RefreshCw size={14} class={usageState.loading ? 'spinning' : undefined} aria-hidden="true" />
        {usageState.loading ? 'Indexing' : 'Refresh'}
      </button>
    </div>
  </header>

  {#if usageState.summary}
    <div class="metric-grid" aria-label={`${usageRangeLabel(usageState.range)} summary`}>
      <article>
        <span>Total tokens</span>
        <strong>{visibleTokens(totalTokens)}</strong>
        <small>{integerFormat.format(totalTokens)} indexed</small>
      </article>
      <article>
        <span>Estimated cost</span>
        <strong>{costLabel(estimatedCostMicros)}</strong>
        <small>Published API token rates</small>
      </article>
      <article>
        <span>Active days</span>
        <strong>{integerFormat.format(usageState.summary.activeDays)}</strong>
        <small>{usageRangeLabel(usageState.range)}</small>
      </article>
      <article>
        <span>Cache share</span>
        <strong>{cacheShare}%</strong>
        <small>{visibleTokens(usageState.summary.cacheReadTokens + usageState.summary.cacheWriteTokens)} cached</small>
      </article>
    </div>

    <div class="analytics-grid">
      <section class="panel intensity-panel" aria-labelledby="daily-intensity-title">
        <div class="panel-heading">
          <div>
            <h3 id="daily-intensity-title">Daily intensity</h3>
            <p class="muted">Combined token activity by indexed day.</p>
          </div>
          {#if bestDay}
            <span class="data-chip">Best: {formatDay(bestDay)}</span>
          {/if}
        </div>
        <div class="heatmap-wrap">
          <div class="heatmap" role="img" aria-label={`Daily token intensity for ${usageRangeLabel(usageState.range)}`}>
            {#each heatmapDays as day (day.day)}
              <span
                class={`level-${day.intensity}`}
                title={`${formatDay(day.day)} · ${visibleTokens(day.totalTokens)} tokens`}
              ></span>
            {/each}
          </div>
        </div>
        <div class="heatmap-legend">
          <span>{heatmapDays[0] ? formatDay(heatmapDays[0].day) : 'No data'}</span>
          <span class="scale-label">Less</span>
          <i class="level-0"></i>
          <i class="level-1"></i>
          <i class="level-2"></i>
          <i class="level-3"></i>
          <i class="level-4"></i>
          <span class="scale-label">More</span>
          <span>{heatmapDays.at(-1) ? formatDay(heatmapDays.at(-1)!.day) : ''}</span>
        </div>
      </section>

      <section class="panel token-panel" aria-labelledby="token-mix-title">
        <div class="panel-heading">
          <div>
            <h3 id="token-mix-title">Token mix</h3>
            <p class="muted">New input, output, and cached context.</p>
          </div>
          {#if usageState.summary.reasoningTokens > 0}
            <span class="data-chip">{visibleTokens(usageState.summary.reasoningTokens)} reasoning</span>
          {/if}
        </div>
        <div class="mix-total">
          <strong>{visibleTokens(totalTokens)}</strong>
          <span>tokens</span>
        </div>
        <div class="mix-bar" aria-label="Token type share">
          <i class="input" style={`width: ${mixWidth(usageState.summary.inputTokens)}%`}></i>
          <i class="output" style={`width: ${mixWidth(usageState.summary.outputTokens)}%`}></i>
          <i class="cache" style={`width: ${mixWidth(usageState.summary.cacheReadTokens + usageState.summary.cacheWriteTokens)}%`}></i>
        </div>
        <div class="mix-legend">
          <span><i class="dot input"></i><b>New input</b><strong>{visibleTokens(usageState.summary.inputTokens)}</strong></span>
          <span><i class="dot output"></i><b>Output</b><strong>{visibleTokens(usageState.summary.outputTokens)}</strong></span>
          <span><i class="dot cache"></i><b>Cache</b><strong>{visibleTokens(usageState.summary.cacheReadTokens + usageState.summary.cacheWriteTokens)}</strong></span>
        </div>
      </section>
    </div>

    <section class="providers" aria-labelledby="providers-title">
      <div class="section-heading">
        <div>
          <h3 id="providers-title">Providers</h3>
          <p class="muted">Share and session detail for {usageRangeLabel(usageState.range).toLowerCase()}.</p>
        </div>
        <span class="event-count">{integerFormat.format(usageState.summary.eventCount)} events</span>
      </div>
      <div class="provider-grid">
        {#each providerNames as provider (provider)}
          {@const card = providerCard(provider)}
          {#if card}
            <article class="provider-card">
              <div class="provider-heading">
                <div>
                  <p class="provider-name">{usageProviderLabel(provider)}</p>
                  <span>{formatLastSeen(card.lastSeenAtMicros)}</span>
                </div>
                <strong>{providerShare(card)}%</strong>
              </div>
              <div class="provider-metrics">
                <span><strong>{visibleTokens(card.totalTokens)}</strong><small>tokens</small></span>
                <span><strong>{integerFormat.format(card.sessionCount)}</strong><small>sessions</small></span>
                <span><strong>{integerFormat.format(card.eventCount)} / {integerFormat.format(card.turnCount)}</strong><small>events / turns</small></span>
                <span><strong>{costLabel(providerCost(card))}</strong><small>estimated cost</small></span>
              </div>
              <div class="provider-context">
                <span>Latest</span>
                <strong>{card.lastModel}</strong>
                <small>{sessionLabel(card)}</small>
              </div>
              <div class="share-row">
                <span>Share of selected tokens</span>
                <div class="provider-bar"><i style={`width: ${providerShare(card)}%`}></i></div>
              </div>
            </article>
          {:else}
            <article class="provider-card empty-provider">
              <div class="provider-heading">
                <p class="provider-name">{usageProviderLabel(provider)}</p>
                <span>No data</span>
              </div>
              <div>
                <strong>No activity in {usageRangeLabel(usageState.range).toLowerCase()}</strong>
                <p class="muted">Refresh after this provider records a local session.</p>
              </div>
            </article>
          {/if}
        {/each}
      </div>
    </section>

    <p class="estimate-note">
      Costs are estimates, not billing totals · {USAGE_COST_RATE_VERSION.replace('published-api-rates-', 'rates updated ')}
    </p>
  {:else}
    <div class="empty">
      <strong>{usageState.error ? 'Usage history is unavailable' : 'Index local usage to see analytics'}</strong>
      <p>{usageState.error ?? 'Refresh to build the private SQLite index from local sessions.'}</p>
    </div>
  {/if}
</section>

<style>
  .workspace {
    display: grid;
    gap: 16px;
    min-height: 100%;
    padding: 18px;
    color: var(--color-text);
    background: var(--color-surface);
  }
  h2, h3, p { margin: 0; }
  h2 { font-size: 1.38rem; line-height: 1.2; letter-spacing: -0.025em; }
  h3 { font-size: 0.9rem; line-height: 1.35; }
  button { font: inherit; }
  .workspace-header, .header-actions, .panel-heading, .section-heading, .provider-heading {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
  }
  .title-block { display: grid; gap: 3px; }
  .eyebrow {
    color: var(--color-text-3);
    font-size: 0.75rem;
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .muted, small { color: var(--color-text-2); }
  .muted { font-size: 0.78rem; line-height: 1.45; }
  .header-actions { align-items: center; }
  .range-selector {
    display: flex;
    gap: 2px;
    padding: 3px;
    border: 1px solid var(--color-border);
    border-radius: 7px;
    background: color-mix(in srgb, var(--color-bg) 55%, var(--color-surface));
  }
  .range-selector button, .refresh {
    min-height: 30px;
    border: 0;
    border-radius: 5px;
    padding: 5px 9px;
    color: var(--color-text-2);
    background: transparent;
    cursor: pointer;
  }
  .range-selector button:hover:not(:disabled), .refresh:hover:not(:disabled) {
    color: var(--color-text);
    background: var(--color-hover);
  }
  .range-selector button.active {
    color: var(--color-text);
    background: var(--color-elevated);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-border) 70%, transparent);
  }
  button:focus-visible { outline: none; box-shadow: var(--focus-ring); }
  button:disabled { cursor: not-allowed; opacity: 0.55; }
  .refresh {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 1px solid var(--color-border);
    background: var(--color-elevated);
  }
  .spinning { animation: spin 0.9s linear infinite; }
  .metric-grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 8px; }
  .metric-grid article {
    display: grid;
    gap: 5px;
    min-width: 0;
    padding: 14px;
    border: 1px solid color-mix(in srgb, var(--color-border) 78%, transparent);
    border-radius: 8px;
    background: color-mix(in srgb, var(--color-elevated) 82%, var(--color-surface));
  }
  .metric-grid article > span {
    color: var(--color-text-2);
    font-size: 0.75rem;
    font-weight: 600;
  }
  .metric-grid article > strong {
    overflow: hidden;
    font-size: clamp(1.25rem, 2vw, 1.75rem);
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.035em;
    text-overflow: ellipsis;
  }
  .metric-grid small { overflow: hidden; font-size: 0.75rem; text-overflow: ellipsis; white-space: nowrap; }
  .analytics-grid { display: grid; grid-template-columns: minmax(0, 1.35fr) minmax(260px, 0.8fr); gap: 10px; }
  .panel, .providers {
    display: grid;
    gap: 14px;
    padding: 15px;
    border: 1px solid color-mix(in srgb, var(--color-border) 78%, transparent);
    border-radius: 8px;
    background: color-mix(in srgb, var(--color-bg) 45%, var(--color-surface));
  }
  .panel-heading > div, .section-heading > div { display: grid; gap: 3px; }
  .data-chip, .event-count {
    border: 1px solid var(--color-border);
    border-radius: 999px;
    padding: 3px 8px;
    color: var(--color-text-2);
    background: var(--color-surface);
    font-size: 0.75rem;
    white-space: nowrap;
  }
  .heatmap-wrap { overflow-x: auto; padding: 3px 0; }
  .heatmap {
    display: grid;
    grid-template-rows: repeat(7, 13px);
    grid-auto-flow: column;
    grid-auto-columns: 13px;
    gap: 4px;
    min-width: max-content;
    min-height: 115px;
    align-content: center;
  }
  .heatmap span, .heatmap-legend i {
    display: block;
    width: 13px;
    height: 13px;
    border: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
    border-radius: 3px;
  }
  .level-0 { background: var(--color-elevated); }
  .level-1 { background: color-mix(in srgb, var(--color-accent) 22%, var(--color-elevated)); }
  .level-2 { background: color-mix(in srgb, var(--color-accent) 42%, var(--color-elevated)); }
  .level-3 { background: color-mix(in srgb, var(--color-accent) 68%, var(--color-elevated)); }
  .level-4 { background: var(--color-accent); }
  .heatmap-legend {
    display: grid;
    grid-template-columns: minmax(70px, 1fr) auto repeat(5, 13px) auto minmax(70px, 1fr);
    gap: 4px;
    align-items: center;
    color: var(--color-text-2);
    font-size: 0.75rem;
  }
  .heatmap-legend > span:last-child { text-align: right; }
  .scale-label { padding: 0 3px; }
  .token-panel { grid-template-rows: auto 1fr auto auto; }
  .mix-total { display: flex; align-items: baseline; gap: 7px; padding-top: 6px; }
  .mix-total strong { font-size: 1.55rem; font-variant-numeric: tabular-nums; letter-spacing: -0.03em; }
  .mix-total span { color: var(--color-text-2); font-size: 0.75rem; }
  .mix-bar, .provider-bar {
    display: flex;
    height: 8px;
    overflow: hidden;
    border-radius: 4px;
    background: var(--color-elevated);
  }
  .mix-bar i, .provider-bar i { display: block; min-width: 0; }
  .input { background: var(--color-accent); }
  .output { background: var(--color-live); }
  .cache { background: var(--color-attention); }
  .mix-legend { display: grid; gap: 7px; }
  .mix-legend > span {
    display: grid;
    grid-template-columns: 9px 1fr auto;
    gap: 7px;
    align-items: center;
    color: var(--color-text-2);
    font-size: 0.75rem;
  }
  .mix-legend b { font-weight: 500; }
  .mix-legend strong { color: var(--color-text); font-variant-numeric: tabular-nums; }
  .dot { width: 8px; height: 8px; border-radius: 2px; }
  .provider-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
  .provider-card {
    display: grid;
    gap: 13px;
    min-width: 0;
    padding: 14px;
    border: 1px solid color-mix(in srgb, var(--color-border) 78%, transparent);
    border-radius: 8px;
    background: var(--color-surface);
  }
  .provider-name { font-size: 0.9rem; font-weight: 680; }
  .provider-heading > div { display: grid; gap: 2px; }
  .provider-heading span { color: var(--color-text-2); font-size: 0.75rem; }
  .provider-heading > strong { font-size: 1.15rem; font-variant-numeric: tabular-nums; }
  .provider-metrics { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 8px; }
  .provider-metrics > span { display: grid; gap: 2px; min-width: 0; }
  .provider-metrics strong { overflow: hidden; font-size: 0.88rem; font-variant-numeric: tabular-nums; text-overflow: ellipsis; }
  .provider-metrics small { font-size: 0.75rem; white-space: nowrap; }
  .provider-context {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 2px 9px;
    padding-top: 11px;
    border-top: 1px solid color-mix(in srgb, var(--color-border) 65%, transparent);
  }
  .provider-context > span { grid-row: span 2; color: var(--color-text-3); font-size: 0.75rem; }
  .provider-context strong, .provider-context small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .provider-context strong { font-size: 0.78rem; }
  .provider-context small { font-size: 0.75rem; }
  .share-row { display: grid; grid-template-columns: auto 1fr; gap: 10px; align-items: center; }
  .share-row > span { color: var(--color-text-2); font-size: 0.75rem; }
  .provider-bar i { background: var(--color-accent); }
  .empty-provider { align-content: start; min-height: 174px; border-style: dashed; }
  .empty-provider > div:last-child { display: grid; gap: 5px; padding-top: 22px; }
  .empty-provider > div:last-child strong { font-size: 0.85rem; }
  .estimate-note { color: var(--color-text-3); font-size: 0.75rem; text-align: right; }
  .empty {
    display: grid;
    gap: 5px;
    min-height: 190px;
    place-content: center;
    padding: 24px;
    border: 1px dashed var(--color-border);
    border-radius: 8px;
    color: var(--color-text-2);
    text-align: center;
  }
  .empty strong { color: var(--color-text); }
  .empty p { font-size: 0.78rem; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .spinning { animation: none; } }
  @media (max-width: 860px) {
    .workspace-header { align-items: stretch; flex-direction: column; }
    .header-actions { justify-content: space-between; }
    .metric-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .analytics-grid { grid-template-columns: 1fr; }
  }
  @media (max-width: 620px) {
    .workspace { padding: 14px; }
    .header-actions { align-items: stretch; flex-direction: column; }
    .range-selector { display: grid; grid-template-columns: repeat(4, 1fr); }
    .provider-grid, .provider-metrics { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
</style>
