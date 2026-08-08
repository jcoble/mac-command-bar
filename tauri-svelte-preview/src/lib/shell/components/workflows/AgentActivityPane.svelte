<script lang="ts">
  import {
    toWorkflowRunView,
    type WorkflowAgentView,
    type WorkflowRunView
  } from '$lib/shell/workflows/workflowTypes';
  import { workflowState } from '$lib/shell/workflows/workflowStore.svelte';

  interface Props {
    runs?: WorkflowRunView[];
    onOpenWorkflow?(workflowRunId: string): void;
    onOpenConversation?(ownedId: string): void;
    onPinToWorking?(ownedId: string): void;
  }

  let { runs, onOpenWorkflow, onOpenConversation, onPinToWorking }: Props = $props();

  const visibleRuns = $derived(
    runs ?? workflowState.runs.map((run) => toWorkflowRunView(run))
  );
  const activeAgents = $derived(
    visibleRuns.flatMap((run) => run.nodes.map((node) => ({ run, agent: node.agent })))
      .filter(({ agent }) => ['queued', 'starting', 'running'].includes(agent.state))
  );
  const waitingAgents = $derived(
    visibleRuns.flatMap((run) => run.nodes.map((node) => ({ run, node, agent: node.agent })))
      .filter(({ node }) => ['waiting-approval', 'waiting-input'].includes(node.state))
  );
  const failures = $derived(
    visibleRuns.flatMap((run) => run.nodes.map((node) => ({ run, node })))
      .filter(({ node }) => node.state === 'failed' || node.failure !== null)
  );

  function progress(run: WorkflowRunView): string {
    if (run.nodes.length === 0) return '0%';
    const complete = run.nodes.filter((node) => ['completed', 'skipped'].includes(node.state)).length;
    return `${Math.round((complete / run.nodes.length) * 100)}%`;
  }

  function agentLabel(agent: WorkflowAgentView): string {
    return agent.roleId ?? agent.ownedId;
  }
</script>

<aside data-testid="agent-activity-pane" class="pane" aria-label="Agent activity">
  <header data-testid="agent-activity-header" class="pane-header">
    <div data-testid="agent-activity-title" class="pane-title">Agent activity</div>
    <span data-testid="agent-activity-run-count" class="pane-meta">{visibleRuns.length} runs</span>
  </header>

  <div data-testid="agent-activity-body" class="body">
    <section data-testid="agent-activity-progress" class="section">
      <header data-testid="agent-activity-progress-header" class="section-header"><h2 data-testid="agent-activity-progress-title">Workflow progress</h2><span data-testid="agent-activity-progress-count">{visibleRuns.length}</span></header>
      {#if visibleRuns.length === 0}
        <p data-testid="agent-activity-progress-empty" class="empty">No workflow run selected.</p>
      {:else}
        <ul data-testid="agent-activity-progress-items" class="items">
          {#each visibleRuns as run (run.workflowRunId)}
            <li data-testid={`agent-activity-progress-${run.workflowRunId}`}>
              <button data-testid={`agent-activity-open-workflow-${run.workflowRunId}`} class="workflow-row" type="button" onclick={() => onOpenWorkflow?.(run.workflowRunId)}>
                <span data-testid={`agent-activity-progress-name-${run.workflowRunId}`} class="item-title">{run.definition.name}</span>
                <span data-testid={`agent-activity-progress-phase-${run.workflowRunId}`} class="item-meta">{run.phase} · {progress(run)}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section data-testid="agent-activity-active" class="section">
      <header data-testid="agent-activity-active-header" class="section-header"><h2 data-testid="agent-activity-active-title">Active agents</h2><span data-testid="agent-activity-active-count">{activeAgents.length}</span></header>
      {#if activeAgents.length === 0}<p data-testid="agent-activity-active-empty" class="empty">No agents are running.</p>{:else}<ul data-testid="agent-activity-active-items" class="items">{#each activeAgents as item (item.agent.ownedId)}<li data-testid={`agent-activity-active-${item.agent.ownedId}`}><button data-testid={`agent-activity-active-open-${item.agent.ownedId}`} class="agent-row" type="button" onclick={() => onOpenConversation?.(item.agent.ownedId)}><span data-testid={`agent-activity-active-name-${item.agent.ownedId}`} class="item-title">{agentLabel(item.agent)}</span><span data-testid={`agent-activity-active-meta-${item.agent.ownedId}`} class="item-meta">{item.agent.provider} · Workflow</span></button><button data-testid={`agent-activity-active-pin-${item.agent.ownedId}`} class="pin" type="button" onclick={() => onPinToWorking?.(item.agent.ownedId)}>Pin</button></li>{/each}</ul>{/if}
    </section>

    <section data-testid="agent-activity-waiting" class="section">
      <header data-testid="agent-activity-waiting-header" class="section-header"><h2 data-testid="agent-activity-waiting-title">Waiting approvals</h2><span data-testid="agent-activity-waiting-count">{waitingAgents.length}</span></header>
      {#if waitingAgents.length === 0}<p data-testid="agent-activity-waiting-empty" class="empty">Nothing is waiting for a decision.</p>{:else}<ul data-testid="agent-activity-waiting-items" class="items">{#each waitingAgents as item (item.node.id)}<li data-testid={`agent-activity-waiting-${item.node.id}`}><button data-testid={`agent-activity-waiting-open-${item.node.id}`} class="agent-row" type="button" onclick={() => onOpenWorkflow?.(item.run.workflowRunId)}><span data-testid={`agent-activity-waiting-name-${item.node.id}`} class="item-title">{item.node.nodeId}</span><span data-testid={`agent-activity-waiting-meta-${item.node.id}`} class="item-meta">{item.node.state.replaceAll('-', ' ')}</span></button></li>{/each}</ul>{/if}
    </section>

    <section data-testid="agent-activity-failures" class="section">
      <header data-testid="agent-activity-failures-header" class="section-header"><h2 data-testid="agent-activity-failures-title">Failures</h2><span data-testid="agent-activity-failures-count">{failures.length}</span></header>
      {#if failures.length === 0}<p data-testid="agent-activity-failures-empty" class="empty">No failed nodes.</p>{:else}<ul data-testid="agent-activity-failures-items" class="items">{#each failures as item (item.node.id)}<li data-testid={`agent-activity-failure-${item.node.id}`}><button data-testid={`agent-activity-failure-open-${item.node.id}`} class="agent-row" type="button" onclick={() => onOpenWorkflow?.(item.run.workflowRunId)}><span data-testid={`agent-activity-failure-name-${item.node.id}`} class="item-title">{item.node.nodeId}</span><span data-testid={`agent-activity-failure-meta-${item.node.id}`} class="item-meta">{item.node.failure?.code ?? 'Failed'}</span></button></li>{/each}</ul>{/if}
    </section>
  </div>
</aside>

<style>
  .pane { display: flex; flex-direction: column; min-width: 0; height: 100%; background: var(--color-surface); color: var(--color-text); }
  .pane-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 10px; border-bottom: 1px solid var(--color-border); }
  .pane-title { font-size: 13px; font-weight: 650; letter-spacing: .04em; text-transform: uppercase; }
  .pane-meta, .section-header span { color: var(--color-text-2); font-size: 12px; }
  .body { display: grid; gap: 9px; min-height: 0; padding: 8px; overflow: auto; }
  .section { min-width: 0; padding: 7px; border: 1px solid var(--color-border); border-radius: 6px; background: var(--color-bg); }
  .section-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding-bottom: 5px; border-bottom: 1px solid var(--color-border); }
  h2 { margin: 0; font-size: 13px; font-weight: 600; }
  .items { display: grid; gap: 4px; margin: 6px 0 0; padding: 0; list-style: none; }
  .items li { display: flex; align-items: stretch; gap: 4px; min-width: 0; }
  .workflow-row, .agent-row { display: grid; flex: 1; min-width: 0; gap: 3px; padding: 5px; border: 0; border-radius: 4px; color: var(--color-text); background: transparent; text-align: left; cursor: pointer; }
  .workflow-row:hover, .workflow-row:focus-visible, .agent-row:hover, .agent-row:focus-visible { background: var(--color-hover); outline: none; }
  .item-title { overflow: hidden; font-size: 13px; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }
  .item-meta { overflow: hidden; color: var(--color-text-2); font-size: 12px; text-overflow: ellipsis; white-space: nowrap; text-transform: capitalize; }
  .pin { align-self: center; min-height: 24px; padding: 0 5px; border: 0; border-radius: 4px; color: var(--color-text-2); background: var(--color-elevated); font-size: 12px; cursor: pointer; }
  .pin:hover, .pin:focus-visible { color: var(--color-text); background: var(--color-hover); outline: none; }
  .empty { margin: 6px 0 0; color: var(--color-text-3); font-size: 13px; }
</style>
