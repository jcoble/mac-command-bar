<script lang="ts">
  import type { WorkflowRunView } from '$lib/shell/workflows/workflowTypes';

  interface Props {
    run: WorkflowRunView;
    selectedNodeId?: string | null;
    onSelectNode?(nodeId: string): void;
  }

  let { run, selectedNodeId = null, onSelectNode }: Props = $props();

  function stateFor(nodeId: string): string {
    return run.nodes.find((node) => node.nodeId === nodeId)?.state ?? 'blocked';
  }

  function roleName(roleId: string): string {
    return run.definition.roles.find((role) => role.id === roleId)?.name ?? roleId;
  }

  function dependencies(nodeId: string): string[] {
    const definition = run.definition.nodes.find((node) => node.id === nodeId);
    const explicit = definition?.dependsOn ?? [];
    const edges = run.definition.edges.filter((edge) => edge.to === nodeId).map((edge) => edge.from);
    return [...new Set([...explicit, ...edges])];
  }

  function readable(value: string): string {
    return value.replaceAll('-', ' ');
  }
</script>

<section data-testid="workflow-graph" class="panel" aria-label="Workflow graph">
  <header data-testid="workflow-graph-header" class="panel-header">
    <div data-testid="workflow-graph-title" class="panel-title">DAG</div>
    <span data-testid="workflow-graph-phase" class="panel-meta">Engine phase: {readable(run.phase)}</span>
  </header>

  <div data-testid="workflow-graph-canvas" class="graph-canvas">
    {#if run.definition.nodes.length === 0}
      <p data-testid="workflow-graph-empty" class="empty">This template has no nodes.</p>
    {:else}
      {#each run.definition.nodes as definition (definition.id)}
        {@const runtime = run.nodes.find((node) => node.nodeId === definition.id)}
        {@const deps = dependencies(definition.id)}
        <button
          data-testid={`workflow-graph-node-${definition.id}`}
          class:selected={definition.id === selectedNodeId}
          class={`graph-node state-${runtime?.state ?? 'blocked'}`}
          type="button"
          onclick={() => onSelectNode?.(definition.id)}
        >
          <span data-testid={`workflow-graph-node-title-${definition.id}`} class="node-title">{definition.title}</span>
          <span data-testid={`workflow-graph-node-role-${definition.id}`} class="node-role">{roleName(definition.roleId)}</span>
          <span data-testid={`workflow-graph-node-state-${definition.id}`} class="node-state">{readable(runtime?.state ?? 'blocked')}</span>
          {#if deps.length > 0}
            <span data-testid={`workflow-graph-node-dependencies-${definition.id}`} class="node-dependencies">after {deps.join(', ')}</span>
          {:else}
            <span data-testid={`workflow-graph-node-dependencies-${definition.id}`} class="node-dependencies">starts first</span>
          {/if}
          {#if definition.approvalGate}
            <span data-testid={`workflow-graph-node-gate-${definition.id}`} class="node-gate">Approval gate</span>
          {/if}
        </button>
      {/each}
    {/if}
  </div>

  <footer data-testid="workflow-graph-legend" class="legend">
    <span data-testid="workflow-graph-legend-running" class="legend-item live">Running</span>
    <span data-testid="workflow-graph-legend-waiting" class="legend-item waiting">Waiting</span>
    <span data-testid="workflow-graph-legend-done" class="legend-item done">Done</span>
    <span data-testid="workflow-graph-legend-failed" class="legend-item failed">Failed</span>
  </footer>
</section>

<style>
  .panel { display: flex; flex-direction: column; min-width: 0; min-height: 0; border: 1px solid var(--color-border); border-radius: 7px; background: var(--color-surface); color: var(--color-text); }
  .panel-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 9px 10px; border-bottom: 1px solid var(--color-border); }
  .panel-title { font-size: 13px; font-weight: 650; letter-spacing: .04em; text-transform: uppercase; }
  .panel-meta { color: var(--color-text-2); font-size: 12px; text-transform: capitalize; }
  .graph-canvas { display: grid; grid-template-columns: repeat(auto-fit, minmax(165px, 1fr)); gap: 8px; min-height: 120px; padding: 10px; overflow: auto; }
  .graph-node { display: grid; align-content: start; gap: 5px; min-height: 102px; padding: 9px; border: 1px solid var(--color-border); border-left: 3px solid var(--color-idle); border-radius: 6px; color: var(--color-text); background: var(--color-bg); text-align: left; cursor: pointer; }
  .graph-node:hover, .graph-node:focus-visible, .graph-node.selected { border-color: var(--color-focus-solid); background: var(--color-selected); outline: none; }
  .graph-node.state-running, .graph-node.state-starting { border-left-color: var(--color-live); }
  .graph-node.state-waiting-approval, .graph-node.state-waiting-input { border-left-color: var(--color-attention); }
  .graph-node.state-completed, .graph-node.state-skipped { border-left-color: var(--color-good); }
  .graph-node.state-failed, .graph-node.state-cancelled { border-left-color: var(--color-bad); }
  .node-title { overflow: hidden; font-size: 13px; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }
  .node-role, .node-dependencies { overflow: hidden; color: var(--color-text-2); font-size: 12px; text-overflow: ellipsis; white-space: nowrap; }
  .node-state { width: fit-content; padding: 2px 4px; border-radius: 4px; color: var(--color-text-2); background: var(--color-elevated); font-size: 12px; text-transform: capitalize; }
  .node-gate { color: var(--color-attention); font-size: 12px; }
  .legend { display: flex; flex-wrap: wrap; gap: 9px; padding: 8px 10px; border-top: 1px solid var(--color-border); color: var(--color-text-2); font-size: 12px; }
  .legend-item::before { display: inline-block; width: 7px; height: 7px; margin-right: 4px; border-radius: 50%; background: var(--color-idle); content: ''; }
  .legend-item.live::before { background: var(--color-live); }
  .legend-item.waiting::before { background: var(--color-attention); }
  .legend-item.done::before { background: var(--color-good); }
  .legend-item.failed::before { background: var(--color-bad); }
  .empty { margin: 0; padding: 14px; color: var(--color-text-2); font-size: 13px; }
</style>
