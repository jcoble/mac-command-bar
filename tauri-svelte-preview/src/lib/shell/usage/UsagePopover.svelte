<script lang="ts">
  import { refreshCurrentUsage, usageState } from './usageStore.svelte.ts';
  import { usageWindowLabel } from './usageCurrent.ts';
  import UsageWorkspace from './UsageWorkspace.svelte';

  interface Props { provider?: string | null; instanceId?: string | null; }
  let { provider = null, instanceId = null }: Props = $props();
  let open = $state(false);
  let fullOpen = $state(false);
</script>

<aside class="usage-popover" data-testid="usage-popover" aria-label="Provider usage">
  <button type="button" class="trigger" aria-expanded={open} onclick={() => (open = !open)}>Usage</button>
  {#if open}<section class="card"><header><strong>Current provider quota</strong><button type="button" onclick={() => void refreshCurrentUsage(provider, instanceId)} disabled={usageState.loading}>{usageState.loading ? 'Reading…' : 'Refresh'}</button></header>{#if usageState.current?.state === 'available'}<p>{usageState.current.provider} · {usageState.current.account ?? 'account unavailable'}</p><ul>{#each usageState.current.windows as window (window.name)}<li>{usageWindowLabel(window)}<small>{window.semantics}</small></li>{/each}</ul>{:else}<p class="muted">{usageState.current?.unavailableReason ?? usageState.unavailableReason ?? 'Quota is unavailable until the provider advertises it.'}</p>{/if}<button type="button" class="details" onclick={() => (fullOpen = !fullOpen)}>{fullOpen ? 'Close history' : 'Open usage history'}</button>{#if fullOpen}<UsageWorkspace />{/if}</section>{/if}
</aside>

<style>
  .usage-popover { position: relative; z-index: 10; } .trigger, .card button { border: 1px solid var(--color-border, #858599); background: var(--color-surface, #17171d); color: var(--color-text, #eef0f9); border-radius: 6px; padding: 0.4rem 0.6rem; font: inherit; cursor: pointer; } .card { position: absolute; right: 0; top: calc(100% + 0.4rem); width: min(360px, calc(100vw - 2rem)); padding: 0.8rem; border: 1px solid var(--color-border, #858599); border-radius: 8px; background: var(--color-surface, #17171d); box-shadow: 0 12px 32px rgb(0 0 0 / 35%); } header { display: flex; justify-content: space-between; gap: 0.5rem; } p { margin: 0.6rem 0; } .muted, small { color: var(--color-muted, #a6a7b8); } ul { display: grid; gap: 0.6rem; margin: 0; padding: 0; list-style: none; } li { display: grid; gap: 0.2rem; } button:disabled { opacity: 0.55; cursor: not-allowed; }
</style>
