<script lang="ts">
  import { refreshResources, resourceState } from './resourceStore.svelte.ts';
  import { resourceCanStop, resourceOwnerLabel, formatBytes } from './resourceViewModel.ts';
  import { resourceService } from './resourceService.ts';
  import type { ResourceDiskRoot, ResourceProcess } from './resourceTypes.ts';
  import ResourcesWorkspace from './ResourcesWorkspace.svelte';
  import WorkspaceSpaceWorkspace from './WorkspaceSpaceWorkspace.svelte';

  interface Props { spaceRoots?: ResourceDiskRoot[]; }
  let { spaceRoots = [] }: Props = $props();

  let open = $state(false);
  let stoppingPid = $state<number | null>(null);
  let fullOpen = $state(false);
  let spaceOpen = $state(false);

  async function stop(process: ResourceProcess): Promise<void> {
    if (!resourceCanStop(process) || !window.confirm(`Stop ${resourceOwnerLabel(process)}?`)) return;
    stoppingPid = process.pid;
    try {
      await resourceService.stop({
        pid: process.pid,
        expectedPgid: process.pgid,
        expectedGeneration: process.registryGeneration,
        ownerId: process.ownerId,
        expectedRoot: process.root
      });
      await refreshResources();
    } finally {
      stoppingPid = null;
    }
  }
</script>

<aside class="resource-popover" data-testid="resource-popover" aria-label="Resources">
  <button type="button" class="trigger" aria-expanded={open} onclick={() => (open = !open)}>
    Resources
    {#if resourceState.snapshot}<span>{resourceState.snapshot.processes.length}</span>{/if}
  </button>
  {#if open}
    <section class="card">
      <header><strong>Resource use</strong><button type="button" onclick={() => void refreshResources()} disabled={resourceState.loading}>{resourceState.loading ? 'Reading…' : 'Refresh'}</button></header>
      {#if resourceState.unavailableReason}<p class="muted">{resourceState.unavailableReason}</p>{/if}
      {#if resourceState.error}<p class="error">{resourceState.error}</p>{/if}
      {#if resourceState.snapshot}
        <p class="totals">CPU {resourceState.snapshot.totalCpuPercent.toFixed(1)}% · RSS {formatBytes(resourceState.snapshot.totalRssBytes)}</p>
        <ul>
          {#each resourceState.snapshot.processes.slice(0, 8) as process (process.pid)}
            <li><div><strong>{process.command || 'Unnamed process'}</strong><small>PID {process.pid} · {resourceOwnerLabel(process)}</small></div><button type="button" disabled={!resourceCanStop(process) || stoppingPid === process.pid} onclick={() => void stop(process)}>{stoppingPid === process.pid ? 'Stopping…' : resourceCanStop(process) ? 'Stop' : 'External'}</button></li>
          {/each}
        </ul>
      {:else if !resourceState.loading}<p class="muted">Refresh to inspect bounded process inventory.</p>{/if}
      <button type="button" class="details" onclick={() => (fullOpen = !fullOpen)}>{fullOpen ? 'Close details' : 'Open full resource view'}</button>
      {#if fullOpen}<ResourcesWorkspace />{/if}
      <button type="button" class="details" onclick={() => (spaceOpen = !spaceOpen)}>{spaceOpen ? 'Close workspace space' : 'Open workspace space'}</button>
      {#if spaceOpen}<WorkspaceSpaceWorkspace roots={spaceRoots} />{/if}
    </section>
  {/if}
</aside>

<style>
  .resource-popover { position: relative; z-index: 10; }
  .trigger, .card button { border: 1px solid var(--color-border, #858599); background: var(--color-surface, #17171d); color: var(--color-text, #eef0f9); border-radius: 6px; padding: 0.4rem 0.6rem; font: inherit; cursor: pointer; }
  .trigger span { margin-left: 0.35rem; color: var(--color-muted, #a6a7b8); }
  .card { position: absolute; right: 0; top: calc(100% + 0.4rem); width: min(420px, calc(100vw - 2rem)); padding: 0.8rem; border: 1px solid var(--color-border, #858599); border-radius: 8px; background: var(--color-surface, #17171d); box-shadow: 0 12px 32px rgb(0 0 0 / 35%); }
  header, li { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; }
  ul { display: grid; gap: 0.45rem; margin: 0.75rem 0 0; padding: 0; list-style: none; }
  li { border-top: 1px solid var(--color-border-subtle, #383844); padding-top: 0.45rem; }
  li div { min-width: 0; display: grid; gap: 0.15rem; }
  li strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  small, .muted { color: var(--color-muted, #a6a7b8); }
  .totals { margin: 0.6rem 0; color: var(--color-muted, #a6a7b8); }
  .error { color: var(--color-danger, #ff9b9b); }
  button:disabled { cursor: not-allowed; opacity: 0.55; }
</style>
