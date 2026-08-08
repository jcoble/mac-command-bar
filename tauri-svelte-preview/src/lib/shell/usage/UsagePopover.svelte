<script lang="ts">
  import { refreshCurrentUsage, refreshUsageHistory, usageState } from './usageStore.svelte.ts';
  import { usagePercent, usageProviderLabel, usageResetLabel } from './usageCurrent.ts';
  import UsageWorkspace from './UsageWorkspace.svelte';

  interface Props { provider?: string | null; instanceId?: string | null; }
  let { provider = null, instanceId = null }: Props = $props();
  let open = $state(false);
  let fullOpen = $state(false);
  let snapshots = $derived(Object.values(usageState.currentByProvider));

  function toggleOpen(): void {
    open = !open;
    if (open) {
      void refreshCurrentUsage(provider, instanceId);
      void refreshUsageHistory();
    }
  }

  async function refresh(): Promise<void> {
    await Promise.all([refreshCurrentUsage(provider, instanceId), refreshUsageHistory()]);
  }
</script>

<aside class="usage-popover" data-testid="usage-popover" aria-label="Provider usage">
  <button type="button" class="trigger" aria-expanded={open} onclick={toggleOpen}>Usage</button>
  {#if open}
    <section class="card">
      <header class="card-header"><div><p class="eyebrow">Live quota</p><h2>Usage</h2></div><button type="button" onclick={() => void refresh()} disabled={usageState.loading}>{usageState.loading ? 'Refreshing…' : 'Refresh'}</button></header>
      <div class="mode-toggle" role="group" aria-label="Usage density"><button type="button" class:active={usageState.viewMode === 'detailed'} onclick={() => (usageState.viewMode = 'detailed')}>Detailed</button><button type="button" class:active={usageState.viewMode === 'compact'} onclick={() => (usageState.viewMode = 'compact')}>Compact</button></div>
      {#if snapshots.length === 0}<p class="muted">Quota data has not been read yet.</p>{/if}
      <div class:compact={usageState.viewMode === 'compact'} class="provider-list">
        {#each snapshots as snapshot (snapshot.provider)}
          <article class="provider-card">
            <div class="provider-heading"><div><strong>{usageProviderLabel(snapshot.provider)}</strong>{#if snapshot.account}<small>{snapshot.account}</small>{/if}</div><span class:available={snapshot.state === 'available'}>{snapshot.state === 'available' ? 'Available' : 'Unavailable'}</span></div>
            {#if snapshot.state === 'available' && snapshot.windows.length > 0}
              {#each snapshot.windows as window (window.name)}<div class="quota-row"><div><span>{window.name}</span><small>{usageResetLabel(window.resetAt)}</small></div><strong>{Math.round(usagePercent(window))}% used</strong><div class="quota-bar"><i style={`width:${usagePercent(window)}%`}></i></div>{#if usageState.viewMode === 'detailed'}<small>{window.semantics}</small>{/if}</div>{/each}
            {:else}<p class="muted unavailable">{snapshot.unavailableReason ?? 'Quota is unavailable locally.'}</p>{/if}
          </article>
        {/each}
      </div>
      <button type="button" class="details" onclick={() => { fullOpen = !fullOpen; if (fullOpen) void refreshUsageHistory(); }}>{fullOpen ? 'Close Stats & Usage' : 'Open Stats & Usage'}</button>
      {#if fullOpen}<UsageWorkspace />{/if}
    </section>
  {/if}
</aside>

<style>
  .usage-popover { position: relative; z-index: 10; } .trigger, .card button { border: 0; background: var(--color-elevated); color: var(--color-text); border-radius: 0.45rem; padding: 0.5rem 0.75rem; font: inherit; cursor: pointer; } .trigger:hover:not(:disabled), .card button:hover:not(:disabled) { background: var(--color-hover); } .trigger:focus-visible, .card button:focus-visible { outline: none; box-shadow: var(--focus-ring); } .trigger { white-space: nowrap; }
  .card { position: static; margin-top: 0.55rem; width: min(30rem, 34vw); max-height: min(78vh, 52rem); overflow: auto; padding: 1rem; border: 1px solid var(--color-border); border-radius: 0.7rem; background: var(--color-elevated); box-shadow: var(--shadow-lg); } .card-header { display: flex; align-items: start; justify-content: space-between; gap: 0.7rem; } h2, p { margin: 0; } h2 { font-size: 1.15rem; } .eyebrow { margin-bottom: 0.15rem; color: var(--color-text-2); text-transform: uppercase; letter-spacing: 0.09em; font-size: 0.68rem; } .muted, small { color: var(--color-text-2); }
  .mode-toggle { display: grid; grid-template-columns: 1fr 1fr; gap: 0.2rem; margin: 0.9rem 0; padding: 0.2rem; border: 1px solid var(--color-border); border-radius: 0.5rem; } .mode-toggle button { border: 0; background: transparent; color: var(--color-text-2); } .mode-toggle button:hover:not(.active):not(:disabled) { background: var(--color-hover); color: var(--color-text); } .mode-toggle button.active { background: var(--color-selected); color: var(--color-text); }
  .provider-list { display: grid; gap: 0.6rem; } .provider-card { display: grid; gap: 0.65rem; padding: 0.75rem; border: 1px solid var(--color-border); border-radius: 0.55rem; } .provider-heading { display: flex; justify-content: space-between; gap: 0.5rem; } .provider-heading > div { display: grid; gap: 0.15rem; } .provider-heading span { color: var(--color-text-2); font-size: 0.75rem; } .provider-heading span.available { color: var(--color-good); }
  .quota-row { display: grid; grid-template-columns: 1fr auto; gap: 0.3rem 0.6rem; align-items: center; } .quota-row > div:first-child { display: flex; justify-content: space-between; gap: 0.5rem; min-width: 0; } .quota-row > div:first-child span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; } .quota-row > strong { font-size: 0.8rem; font-variant-numeric: tabular-nums; } .quota-bar { grid-column: 1 / -1; height: 0.45rem; overflow: hidden; border-radius: 9rem; background: var(--color-border); } .quota-bar i { display: block; height: 100%; border-radius: inherit; background: var(--color-accent); } .quota-row > small:last-child { grid-column: 1 / -1; } .provider-list.compact .quota-row > small:last-child { display: none; }
  .unavailable { padding: 0.2rem 0; } .details { width: 100%; margin-top: 0.8rem; } button:disabled { cursor: not-allowed; opacity: 0.55; }
</style>
