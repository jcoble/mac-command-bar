<script lang="ts">
  import { refreshResources, resourceState } from './resourceStore.svelte.ts';
  import { formatBytes, resourceOwnerLabel, resourceCanStop } from './resourceViewModel.ts';
  import { resourceService } from './resourceService.ts';

  let stoppingPid = $state<number | null>(null);

  async function stop(pid: number): Promise<void> {
    const process = resourceState.snapshot?.processes.find((item) => item.pid === pid);
    if (!process || !resourceCanStop(process) || !window.confirm(`Stop ${resourceOwnerLabel(process)}?`)) return;
    stoppingPid = pid;
    try {
      await resourceService.stop({ pid, expectedPgid: process.pgid, expectedGeneration: process.registryGeneration, ownerId: process.ownerId, expectedRoot: process.root });
      await refreshResources();
    } finally {
      stoppingPid = null;
    }
  }
</script>

<section class="workspace" data-testid="resources-workspace" aria-label="Resources workspace">
  <header><div><p class="eyebrow">Resources</p><h2>Processes and ownership</h2><p class="muted">One bounded inventory, grouped by the ownership the app can prove.</p></div><button type="button" onclick={() => void refreshResources()} disabled={resourceState.loading}>{resourceState.loading ? 'Reading…' : 'Refresh'}</button></header>
  {#if resourceState.snapshot}
    <div class="summary"><span>{resourceState.snapshot.processes.length} processes</span><span>{resourceState.snapshot.totalCpuPercent.toFixed(1)}% CPU</span><span>{formatBytes(resourceState.snapshot.totalRssBytes)} RSS</span></div>
    <div class="table" role="table"><div class="row heading" role="row"><span>Process</span><span>Owner</span><span>Ports</span><span>Action</span></div>{#each resourceState.snapshot.processes as process (process.pid)}<div class="row" role="row"><span><strong>{process.command || 'Unnamed process'}</strong><small>PID {process.pid} · {process.user} · {process.elapsedSeconds}s</small></span><span>{resourceOwnerLabel(process)}</span><span>{process.listeningPorts.length ? process.listeningPorts.join(', ') : '—'}</span><span><button type="button" disabled={!resourceCanStop(process) || stoppingPid === process.pid} onclick={() => void stop(process.pid)}>{resourceCanStop(process) ? stoppingPid === process.pid ? 'Stopping…' : 'Stop' : 'External'}</button></span></div>{/each}</div>
  {:else}<p class="empty">{resourceState.error ?? resourceState.unavailableReason ?? 'Refresh to inspect resources.'}</p>{/if}
</section>

<style>
  .workspace { display: grid; gap: 1rem; padding: 1rem; color: var(--color-text, #eef0f9); }
  header { display: flex; align-items: start; justify-content: space-between; gap: 1rem; }
  h2, p { margin: 0; } .eyebrow { text-transform: uppercase; letter-spacing: 0.08em; font-size: 0.72rem; color: var(--color-muted, #a6a7b8); } .muted, small, .empty { color: var(--color-muted, #a6a7b8); }
  header button, .row button { border: 1px solid var(--color-border, #858599); background: var(--color-surface, #17171d); color: inherit; border-radius: 6px; padding: 0.45rem 0.65rem; font: inherit; cursor: pointer; } button:disabled { opacity: 0.55; cursor: not-allowed; }
  .summary { display: flex; gap: 1rem; flex-wrap: wrap; color: var(--color-muted, #a6a7b8); }
  .table { overflow: auto; border: 1px solid var(--color-border-subtle, #383844); border-radius: 8px; } .row { display: grid; grid-template-columns: minmax(14rem, 2fr) minmax(9rem, 1fr) 5rem 6rem; gap: 0.8rem; align-items: center; padding: 0.65rem 0.8rem; border-top: 1px solid var(--color-border-subtle, #383844); min-width: 42rem; } .row:first-child { border-top: 0; } .heading { color: var(--color-muted, #a6a7b8); font-size: 0.78rem; } .row span:first-child { display: grid; gap: 0.2rem; min-width: 0; } strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
