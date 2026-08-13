<script lang="ts">
  import { onMount } from 'svelte';

  import ResourceManagerPanel from './ResourceManagerPanel.svelte';
  import {
    formatResourceBytes,
    formatResourceCpu,
    formatResourceUpdatedAgo
  } from './resourceSampleViewModel';
  import {
    refreshResourceSample,
    resourceManagerState,
    resourceSampleState,
    toggleResourceManager
  } from './resourceSampleStore.svelte';

  let updatedLabel = $derived.by(() => {
    const generatedAtMs = resourceSampleState.sample?.generatedAtMs ?? null;
    if (generatedAtMs === null) return formatResourceUpdatedAgo(null, Date.now());
    return `sampled ${new Date(generatedAtMs).toLocaleTimeString([], {
      hour: 'numeric',
      minute: '2-digit',
      second: '2-digit'
    })}`;
  });

  onMount(() => {
    void refreshResourceSample();
    const refreshWhenVisible = (): void => {
      if (document.visibilityState === 'visible') void refreshResourceSample();
    };
    document.addEventListener('visibilitychange', refreshWhenVisible);
    return () => {
      document.removeEventListener('visibilitychange', refreshWhenVisible);
    };
  });

  // The history lines exist only while the manager is open. Sampling a hidden
  // panel wastes a full process-tree read and keeps the otherwise idle shell
  // waking every three seconds.
  $effect(() => {
    if (!resourceManagerState.open) return;
    void refreshResourceSample();
    const pollTimer = window.setInterval(() => void refreshResourceSample(), 3_000);
    return () => window.clearInterval(pollTimer);
  });
</script>

{#if resourceManagerState.open}
  <ResourceManagerPanel />
{/if}

<footer class="resource-bottom-bar" aria-label="Resource usage">
  <button
    type="button"
    class:open={resourceManagerState.open}
    aria-expanded={resourceManagerState.open}
    aria-controls="resource-manager-panel"
    onclick={toggleResourceManager}
  >
    <span class="label">Resources</span>
    {#if resourceSampleState.sample}
      <strong>{formatResourceBytes(resourceSampleState.sample.totals.rssBytes)}</strong>
      <span aria-hidden="true">·</span>
      <span>{formatResourceCpu(resourceSampleState.sample.totals.cpuPercent)} CPU</span>
      <span aria-hidden="true">·</span>
      <span>{resourceSampleState.sample.totals.processCount} {resourceSampleState.sample.totals.processCount === 1 ? 'proc' : 'procs'}</span>
    {:else if resourceSampleState.loading}
      <span>Reading process usage…</span>
    {:else}
      <span>{resourceSampleState.error ?? 'Desktop process usage unavailable'}</span>
    {/if}
    <span class="updated">{updatedLabel}</span>
  </button>
</footer>

<style>
  .resource-bottom-bar {
    z-index: 80;
    display: flex;
    flex: 0 0 28px;
    width: 100%;
    min-width: 0;
    border-top: 1px solid var(--color-border);
    background: var(--color-bg);
    color: var(--color-text-2);
  }

  button {
    display: flex;
    width: 100%;
    min-width: 0;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 12px;
    line-height: 1;
    text-align: left;
    cursor: pointer;
  }
  button:hover, button.open { background: var(--color-hover); color: var(--color-text); }
  button:focus-visible { outline: none; box-shadow: inset var(--focus-ring); }
  .label { color: var(--color-text); font-weight: 600; }
  strong { color: var(--color-text); font-weight: 600; font-variant-numeric: tabular-nums; }
  .updated { margin-left: auto; overflow: hidden; color: var(--color-text-3); text-overflow: ellipsis; white-space: nowrap; }

  @media (max-width: 540px) {
    .updated { display: none; }
  }
</style>
