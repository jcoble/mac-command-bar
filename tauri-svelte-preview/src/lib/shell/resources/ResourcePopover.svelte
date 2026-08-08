<script lang="ts">
  import { refreshResources, resourceState } from './resourceStore.svelte.ts';
  import { formatBytes, groupResourceProcesses, resourceCanStop, resourceOwnerLabel } from './resourceViewModel.ts';
  import { resourceService } from './resourceService.ts';
  import type { ResourceDiskRoot, ResourceProcess } from './resourceTypes.ts';
  import ResourcesWorkspace from './ResourcesWorkspace.svelte';
  import WorkspaceSpaceWorkspace from './WorkspaceSpaceWorkspace.svelte';

  interface Props { spaceRoots?: ResourceDiskRoot[]; }
  let { spaceRoots = [] }: Props = $props();
  let open = $state(false);
  let fullOpen = $state(false);
  let spaceOpen = $state(false);
  let stoppingPid = $state<number | null>(null);
  let groups = $derived(resourceState.snapshot ? groupResourceProcesses(resourceState.snapshot.processes) : []);

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
      <header class="card-header">
        <div><p class="eyebrow">Owned resources</p><h2>Resource Manager</h2></div>
        <button type="button" onclick={() => void refreshResources()} disabled={resourceState.loading}>{resourceState.loading ? 'Refreshing…' : 'Refresh'}</button>
      </header>
      {#if resourceState.error}<p class="error">{resourceState.error}</p>{/if}
      {#if resourceState.snapshot}
        <div class="totals"><strong>{resourceState.snapshot.processes.length}</strong><span>owned processes</span><strong>{resourceState.snapshot.totalCpuPercent.toFixed(1)}%</strong><span>CPU</span><strong>{formatBytes(resourceState.snapshot.totalRssBytes)}</strong><span>RSS</span></div>
        {#if groups.length === 0}
          <p class="muted empty">No owned terminal or agent processes are running.</p>
        {:else}
          <div class="tree" data-testid="resource-tree">
            {#each groups as project (project.id)}
              <section class="project">
                <div class="group-row project-row"><span class="chevron">⌄</span><strong>{project.label}</strong><span>{project.totalCpuPercent.toFixed(1)}%</span><span>{formatBytes(project.totalRssBytes)}</span></div>
                {#each project.workspaces as workspace (workspace.id)}
                  <div class="group-row workspace-row"><span class="chevron">⌄</span><strong>{workspace.label}</strong><span>{workspace.totalCpuPercent.toFixed(1)}%</span><span>{formatBytes(workspace.totalRssBytes)}</span></div>
                  {#each workspace.sessions as session (session.id)}
                    <div class="session-block">
                      <div class="group-row session-row"><span class="chevron">⌄</span><strong>{session.label}</strong><span>{session.totalCpuPercent.toFixed(1)}%</span><span>{formatBytes(session.totalRssBytes)}</span></div>
                      {#each session.processes as process (process.pid)}
                        <div class="process-row">
                          <div class="process-name"><span class="status-dot"></span><span><strong>{process.command || 'Unnamed process'}</strong><small>PID {process.pid} · {process.user}</small></span></div>
                          <span>{process.cpuPercent.toFixed(1)}%</span><span>{formatBytes(process.rssBytes)}</span>
                          <button type="button" disabled={!resourceCanStop(process) || stoppingPid === process.pid} onclick={() => void stop(process)}>{stoppingPid === process.pid ? 'Stopping…' : 'Stop'}</button>
                        </div>
                      {/each}
                    </div>
                  {/each}
                {/each}
              </section>
            {/each}
          </div>
        {/if}
      {:else if !resourceState.loading}
        <p class="muted empty">Refresh to inspect the processes owned by this app.</p>
      {/if}
      <div class="actions">
        <button type="button" class="details" onclick={() => (fullOpen = !fullOpen)}>{fullOpen ? 'Close full resource view' : 'Open full resource view'}</button>
        <button type="button" class="details" onclick={() => (spaceOpen = !spaceOpen)}>{spaceOpen ? 'Close Space' : 'Open Space'}</button>
      </div>
      {#if fullOpen}<ResourcesWorkspace />{/if}
      {#if spaceOpen}<WorkspaceSpaceWorkspace roots={spaceRoots} />{/if}
    </section>
  {/if}
</aside>

<style>
  .resource-popover { position: relative; z-index: 10; }
  .trigger, .card button { border: 1px solid var(--color-border); background: var(--color-surface); color: var(--color-text); border-radius: 0.45rem; padding: 0.5rem 0.75rem; font: inherit; cursor: pointer; }
  .trigger span { margin-left: 0.35rem; color: var(--color-muted); }
  .card { position: static; margin-top: 0.55rem; width: min(43rem, 48vw); max-height: min(75vh, 48rem); overflow: auto; padding: 1rem; border: 1px solid var(--color-border); border-radius: 0.7rem; background: var(--color-elevated); box-shadow: var(--shadow-lg); }
  .card-header, .group-row, .process-row { display: grid; align-items: center; gap: 0.65rem; }
  .card-header { grid-template-columns: 1fr auto; }
  h2, p { margin: 0; } h2 { font-size: 1.15rem; } .eyebrow { margin-bottom: 0.15rem; color: var(--color-muted); text-transform: uppercase; letter-spacing: 0.09em; font-size: 0.68rem; } .muted, small { color: var(--color-muted); } .error { margin-top: 0.65rem; color: var(--color-danger); }
  .totals { display: flex; align-items: baseline; flex-wrap: wrap; gap: 0.35rem 0.65rem; padding: 0.85rem 0; color: var(--color-muted); border-bottom: 1px solid var(--color-border-subtle); } .totals strong { color: var(--color-text); font-size: 1.05rem; }
  .tree { display: grid; gap: 0.1rem; padding: 0.5rem 0; } .project + .project { border-top: 1px solid var(--color-border-subtle); margin-top: 0.45rem; padding-top: 0.45rem; }
  .group-row { grid-template-columns: 1rem minmax(0, 1fr) 4.3rem 5.6rem; min-height: 2.2rem; } .project-row { font-size: 0.9rem; } .workspace-row { padding-left: 1rem; color: var(--color-muted); } .session-row { padding-left: 2rem; color: var(--color-text); font-size: 0.85rem; } .group-row span:nth-last-child(-n+2) { text-align: right; font-variant-numeric: tabular-nums; color: var(--color-muted); }
  .chevron { color: var(--color-muted); } .session-block { display: grid; gap: 0.1rem; } .process-row { grid-template-columns: minmax(0, 1fr) 4.3rem 5.6rem auto; padding: 0.45rem 0.25rem 0.45rem 3.15rem; border-top: 1px solid var(--color-border-subtle); font-size: 0.78rem; } .process-row > span { text-align: right; color: var(--color-muted); font-variant-numeric: tabular-nums; } .process-name { display: flex; align-items: center; gap: 0.45rem; min-width: 0; } .process-name > span:last-child { display: grid; min-width: 0; gap: 0.15rem; } .process-name strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; } .status-dot { width: 0.45rem; height: 0.45rem; flex: 0 0 auto; border-radius: 50%; background: var(--color-success); }
  .process-row button { padding: 0.3rem 0.5rem; font-size: 0.75rem; } .actions { display: flex; flex-wrap: wrap; gap: 0.5rem; border-top: 1px solid var(--color-border-subtle); padding-top: 0.8rem; margin-top: 0.5rem; } .details { flex: 1 1 12rem; } button:disabled { cursor: not-allowed; opacity: 0.55; } .empty { padding: 1rem 0; }
</style>
