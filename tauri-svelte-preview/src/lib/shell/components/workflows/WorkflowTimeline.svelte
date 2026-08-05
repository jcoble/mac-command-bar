<script lang="ts">
  import type { WorkflowNodeView, WorkflowRunView } from '$lib/shell/workflows/workflowTypes';

  interface Props {
    run: WorkflowRunView;
    selectedNodeId?: string | null;
    onSelectNode?(nodeId: string): void;
  }

  let { run, selectedNodeId = null, onSelectNode }: Props = $props();

  const entries = $derived(
    [...run.nodes].sort((left, right) => {
      const leftTime = left.startedAtMs ?? left.finishedAtMs ?? Number.MAX_SAFE_INTEGER;
      const rightTime = right.startedAtMs ?? right.finishedAtMs ?? Number.MAX_SAFE_INTEGER;
      return leftTime - rightTime || left.nodeId.localeCompare(right.nodeId);
    })
  );

  function readable(value: string): string {
    return value.replaceAll('-', ' ');
  }

  function timing(node: WorkflowNodeView): string {
    if (node.startedAtMs === null && node.finishedAtMs === null) return 'Not started';
    if (node.finishedAtMs === null) return 'In progress';
    if (node.startedAtMs === null) return 'Finished';
    const seconds = Math.max(0, Math.round((node.finishedAtMs - node.startedAtMs) / 1000));
    return seconds < 60 ? `${seconds}s` : `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  }
</script>

<section data-testid="workflow-timeline" class="panel" aria-label="Workflow timeline">
  <header data-testid="workflow-timeline-header" class="panel-header">
    <div data-testid="workflow-timeline-title" class="panel-title">Timeline</div>
    <span data-testid="workflow-timeline-sequence" class="panel-meta">Ledger receipt {run.lastSequence}</span>
  </header>

  {#if entries.length === 0}
    <p data-testid="workflow-timeline-empty" class="empty">No node activity in this snapshot.</p>
  {:else}
    <ol data-testid="workflow-timeline-entries" class="entries">
      {#each entries as node (node.id)}
        <li data-testid={`workflow-timeline-entry-${node.id}`} class:selected={node.nodeId === selectedNodeId}>
          <button data-testid={`workflow-timeline-select-${node.id}`} class="entry" type="button" onclick={() => onSelectNode?.(node.nodeId)}>
            <span data-testid={`workflow-timeline-marker-${node.id}`} class={`marker state-${node.state}`} aria-hidden="true"></span>
            <span data-testid={`workflow-timeline-node-${node.id}`} class="entry-node">{node.nodeId}</span>
            <span data-testid={`workflow-timeline-state-${node.id}`} class="entry-state">{readable(node.state)}</span>
            <span data-testid={`workflow-timeline-timing-${node.id}`} class="entry-meta">{timing(node)} · attempt {node.attempt}</span>
            <span data-testid={`workflow-timeline-contract-${node.id}`} class="entry-meta">{node.outputContract}</span>
            {#if node.failure}
              <span data-testid={`workflow-timeline-failure-${node.id}`} class="entry-failure">{node.failure.code}: {node.failure.message}</span>
            {/if}
          </button>
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .panel { display: flex; flex-direction: column; min-width: 0; min-height: 0; border: 1px solid var(--color-border); border-radius: 7px; background: var(--color-surface); color: var(--color-text); }
  .panel-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 9px 10px; border-bottom: 1px solid var(--color-border); }
  .panel-title { font-size: 13px; font-weight: 650; letter-spacing: .04em; text-transform: uppercase; }
  .panel-meta, .entry-meta { color: var(--color-text-2); font-size: 12px; }
  .entries { display: grid; gap: 4px; margin: 0; padding: 10px 10px 10px 19px; overflow: auto; list-style: none; }
  .entries li { position: relative; min-width: 0; border-radius: 5px; }
  .entries li::before { position: absolute; top: 0; bottom: -4px; left: -12px; width: 1px; background: var(--color-border); content: ''; }
  .entries li:last-child::before { bottom: 50%; }
  .entries li.selected { background: var(--color-selected); }
  .entry { position: relative; display: grid; grid-template-columns: minmax(80px, auto) auto minmax(0, 1fr); gap: 4px 8px; width: 100%; padding: 7px; border: 0; color: var(--color-text); background: transparent; text-align: left; cursor: pointer; }
  .entry:hover, .entry:focus-visible { background: var(--color-hover); outline: none; }
  .marker { position: absolute; top: 10px; left: -16px; width: 8px; height: 8px; border: 1px solid var(--color-border); border-radius: 50%; background: var(--color-idle); }
  .marker.state-running, .marker.state-starting { background: var(--color-live); border-color: var(--color-live); }
  .marker.state-waiting-approval, .marker.state-waiting-input { background: var(--color-attention); border-color: var(--color-attention); }
  .marker.state-completed, .marker.state-skipped { background: var(--color-good); border-color: var(--color-good); }
  .marker.state-failed, .marker.state-cancelled { background: var(--color-bad); border-color: var(--color-bad); }
  .entry-node { font-size: 13px; font-weight: 600; }
  .entry-state { color: var(--color-text-2); font-size: 12px; text-transform: capitalize; }
  .entry-failure { grid-column: 1 / -1; color: var(--color-bad); font-size: 12px; }
  .empty { margin: 0; padding: 14px; color: var(--color-text-2); font-size: 13px; }
</style>
