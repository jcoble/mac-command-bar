<script lang="ts">
  import type { WorkflowRunView } from '$lib/shell/workflows/workflowTypes';

  interface Props {
    run: WorkflowRunView;
    selectedNodeId?: string | null;
    onSelectNode?(nodeId: string): void;
  }

  let { run, selectedNodeId = null, onSelectNode }: Props = $props();

  const lanes = $derived(
    run.definition.roles
      .map((role) => ({
        role,
        nodes: run.nodes.filter((node) => node.roleId === role.id)
      }))
      .filter((lane) => lane.nodes.length > 0)
  );

  function readable(value: string): string {
    return value.replaceAll('-', ' ');
  }
</script>

<section data-testid="workflow-lane-board" class="board" aria-label="Workflow lanes">
  <header data-testid="workflow-lane-board-header" class="board-header">
    <div data-testid="workflow-lane-board-title" class="board-title">Role lanes</div>
    <span data-testid="workflow-lane-board-copy" class="board-copy">One lane per configured role</span>
  </header>

  {#if lanes.length === 0}
    <p data-testid="workflow-lane-board-empty" class="empty">No agent lanes are configured.</p>
  {:else}
    <div data-testid="workflow-lanes" class="lanes">
      {#each lanes as lane (lane.role.id)}
        <article data-testid={`workflow-lane-${lane.role.id}`} class="lane">
          <header data-testid={`workflow-lane-header-${lane.role.id}`} class="lane-header">
            <span data-testid={`workflow-lane-name-${lane.role.id}`} class="lane-name">{lane.role.name}</span>
            <span data-testid={`workflow-lane-count-${lane.role.id}`} class="lane-count">{lane.nodes.length}</span>
          </header>
          <p data-testid={`workflow-lane-purpose-${lane.role.id}`} class="lane-purpose">{lane.role.purpose || 'No role description.'}</p>
          <div data-testid={`workflow-lane-nodes-${lane.role.id}`} class="lane-nodes">
            {#each lane.nodes as node (node.nodeId)}
              <button
                data-testid={`workflow-lane-node-${node.nodeId}`}
                class:selected={node.nodeId === selectedNodeId}
                class="lane-node"
                type="button"
                onclick={() => onSelectNode?.(node.nodeId)}
              >
                <span data-testid={`workflow-lane-node-title-${node.nodeId}`} class="node-title">{run.definition.nodes.find((definition) => definition.id === node.nodeId)?.title ?? node.nodeId}</span>
                <span data-testid={`workflow-lane-node-provider-${node.nodeId}`} class="node-meta">{node.provider} · {node.provenance === 'workflow' ? 'Workflow' : 'Provider-native'}</span>
                <span data-testid={`workflow-lane-node-state-${node.nodeId}`} class={`node-state state-${node.state}`}>{readable(node.state)}</span>
              </button>
            {/each}
          </div>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .board { display: flex; flex-direction: column; min-width: 0; min-height: 0; border: 1px solid var(--color-border); border-radius: 7px; background: var(--color-surface); color: var(--color-text); }
  .board-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 9px 10px; border-bottom: 1px solid var(--color-border); }
  .board-title { font-size: 13px; font-weight: 650; letter-spacing: .04em; text-transform: uppercase; }
  .board-copy { color: var(--color-text-2); font-size: 12px; }
  .lanes { display: grid; grid-template-columns: repeat(auto-fit, minmax(190px, 1fr)); gap: 9px; padding: 10px; overflow: auto; }
  .lane { min-width: 0; padding: 8px; border: 1px solid var(--color-border); border-radius: 6px; background: var(--color-bg); }
  .lane-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .lane-name { overflow: hidden; font-size: 13px; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }
  .lane-count { color: var(--color-text-3); font-size: 12px; }
  .lane-purpose { min-height: 30px; margin: 5px 0 8px; color: var(--color-text-2); font-size: 12px; }
  .lane-nodes { display: grid; gap: 5px; }
  .lane-node { display: grid; gap: 4px; min-width: 0; padding: 7px; border: 1px solid var(--color-border); border-radius: 5px; color: var(--color-text); background: var(--color-surface); text-align: left; cursor: pointer; }
  .lane-node:hover, .lane-node:focus-visible, .lane-node.selected { border-color: var(--color-focus-solid); background: var(--color-selected); outline: none; }
  .node-title { overflow: hidden; font-size: 13px; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }
  .node-meta, .node-state { color: var(--color-text-2); font-size: 12px; }
  .node-state { text-transform: capitalize; }
  .state-running, .state-starting { color: var(--color-live); }
  .state-waiting-approval, .state-waiting-input { color: var(--color-attention); }
  .state-completed, .state-skipped { color: var(--color-good); }
  .state-failed, .state-cancelled { color: var(--color-bad); }
  .empty { margin: 0; padding: 14px; color: var(--color-text-2); font-size: 13px; }
</style>
