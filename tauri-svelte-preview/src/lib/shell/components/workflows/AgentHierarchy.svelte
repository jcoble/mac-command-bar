<script lang="ts">
  import type { WorkflowAgentView, WorkflowRunView } from '$lib/shell/workflows/workflowTypes';

  interface Props {
    run: WorkflowRunView;
    providerNativeAgents?: WorkflowAgentView[];
    selectedOwnedId?: string | null;
    onSelect?(ownedId: string): void;
    onOpenConversation?(ownedId: string): void;
    onOpenWorktree?(ownedId: string): void;
    onPinToWorking?(ownedId: string): void;
  }

  let {
    run,
    providerNativeAgents = [],
    selectedOwnedId = null,
    onSelect,
    onOpenConversation,
    onOpenWorktree,
    onPinToWorking
  }: Props = $props();

  const workflowAgents = $derived(run.nodes.map((node) => node.agent));
  const agents = $derived([...workflowAgents, ...providerNativeAgents]);

  function readable(value: string): string {
    return value.replaceAll('-', ' ');
  }

  function agentTitle(agent: WorkflowAgentView): string {
    if (agent.roleId) return agent.roleId;
    return agent.provenance === 'provider-native' ? 'Provider-native child' : 'Workflow agent';
  }
</script>

<section data-testid="agent-hierarchy" class="panel" aria-label="Agent hierarchy">
  <header data-testid="agent-hierarchy-header" class="panel-header">
    <div data-testid="agent-hierarchy-title" class="panel-title">Agent hierarchy</div>
    <span data-testid="agent-hierarchy-count" class="panel-meta">{agents.length} agents</span>
  </header>

  <div data-testid="agent-hierarchy-tree" class="tree">
    <div data-testid="agent-hierarchy-root" class="root-row">
      <span data-testid="agent-hierarchy-root-label" class="root-label">Workflow</span>
      <span data-testid="agent-hierarchy-root-id" class="root-id">{run.workflowRunId}</span>
    </div>
    {#if agents.length === 0}
      <p data-testid="agent-hierarchy-empty" class="empty">No agents in this snapshot.</p>
    {:else}
      <ul data-testid="agent-hierarchy-agents" class="agent-list">
        {#each agents as agent (agent.ownedId)}
          <li data-testid={`agent-hierarchy-item-${agent.ownedId}`} class:selected={agent.ownedId === selectedOwnedId}>
            <div data-testid={`agent-hierarchy-row-${agent.ownedId}`} class="agent-row">
              <button data-testid={`agent-hierarchy-select-${agent.ownedId}`} class="agent-main" type="button" onclick={() => onSelect?.(agent.ownedId)}>
                <span data-testid={`agent-hierarchy-title-${agent.ownedId}`} class="agent-title">{agentTitle(agent)}</span>
                <span data-testid={`agent-hierarchy-owned-id-${agent.ownedId}`} class="agent-id">{agent.ownedId}</span>
                <span data-testid={`agent-hierarchy-provider-${agent.ownedId}`} class="agent-meta">{agent.provider} · {agent.provenance === 'workflow' ? 'Workflow' : 'Provider-native'}</span>
                <span data-testid={`agent-hierarchy-state-${agent.ownedId}`} class={`agent-state state-${agent.state}`}>{readable(agent.state)}</span>
              </button>
              <div data-testid={`agent-hierarchy-actions-${agent.ownedId}`} class="agent-actions">
                <button data-testid={`agent-open-conversation-${agent.ownedId}`} type="button" onclick={() => onOpenConversation?.(agent.ownedId)}>Conversation</button>
                <button data-testid={`agent-open-worktree-${agent.ownedId}`} type="button" disabled={!agent.leaseId} onclick={() => onOpenWorktree?.(agent.ownedId)}>Worktree</button>
                <button data-testid={`agent-pin-working-${agent.ownedId}`} type="button" onclick={() => onPinToWorking?.(agent.ownedId)}>Pin to Working</button>
              </div>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</section>

<style>
  .panel { display: flex; flex-direction: column; min-width: 0; min-height: 0; border: 1px solid var(--color-border); border-radius: 7px; background: var(--color-surface); color: var(--color-text); }
  .panel-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 9px 10px; border-bottom: 1px solid var(--color-border); }
  .panel-title { font-size: 13px; font-weight: 650; letter-spacing: .04em; text-transform: uppercase; }
  .panel-meta, .root-id, .agent-id, .agent-meta { color: var(--color-text-2); font-size: 12px; }
  .tree { min-height: 0; padding: 10px; overflow: auto; }
  .root-row { display: flex; align-items: center; gap: 7px; padding: 7px; border: 1px solid var(--color-selected-border); border-radius: 5px; background: var(--color-selected); }
  .root-label { color: var(--color-accent); font-size: 13px; font-weight: 600; }
  .root-id, .agent-id { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .agent-list { display: grid; gap: 6px; margin: 7px 0 0 16px; padding: 0 0 0 11px; border-left: 1px solid var(--color-border); list-style: none; }
  .agent-list li { min-width: 0; border-radius: 5px; }
  .agent-list li.selected { background: var(--color-selected); }
  .agent-row { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 6px; padding: 6px; }
  .agent-main { display: grid; min-width: 0; gap: 3px; padding: 0; border: 0; color: var(--color-text); background: transparent; text-align: left; cursor: pointer; }
  .agent-main:hover, .agent-main:focus-visible { color: var(--color-accent); outline: none; }
  .agent-title { overflow: hidden; font-size: 13px; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }
  .agent-id { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
  .agent-meta { color: var(--color-accent); }
  .agent-state { color: var(--color-text-2); font-size: 12px; text-transform: capitalize; }
  .state-running, .state-starting { color: var(--color-live); }
  .state-waiting-approval, .state-waiting-input { color: var(--color-attention); }
  .state-completed, .state-skipped { color: var(--color-good); }
  .state-failed, .state-cancelled { color: var(--color-bad); }
  .agent-actions { display: flex; align-items: flex-start; flex-wrap: wrap; justify-content: flex-end; gap: 4px; }
  .agent-actions button { min-height: 25px; padding: 0 5px; border: 0; border-radius: 4px; color: var(--color-text-2); background: var(--color-elevated); font-size: 12px; cursor: pointer; }
  .agent-actions button:hover, .agent-actions button:focus-visible { color: var(--color-text); background: var(--color-hover); outline: none; }
  .agent-actions button:disabled { cursor: default; opacity: .5; }
  .empty { margin: 8px 0 0; color: var(--color-text-2); font-size: 13px; }
</style>
