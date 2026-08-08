<script lang="ts">
  import type { WorkflowAgentView, WorkflowNodeView } from '$lib/shell/workflows/workflowTypes';

  interface Props {
    agent: WorkflowAgentView | null;
    node?: WorkflowNodeView | null;
    usage?: { tokensUsed?: number | null; toolTerminalsUsed?: number | null } | null;
    conversationSummary?: string | null;
    tools?: string[];
    plan?: string[];
    tasks?: string[];
    onOpenConversation?(ownedId: string): void;
    onOpenWorktree?(ownedId: string): void;
    onPinToWorking?(ownedId: string): void;
  }

  let {
    agent,
    node = null,
    usage = null,
    conversationSummary = null,
    tools = [],
    plan = [],
    tasks = [],
    onOpenConversation,
    onOpenWorktree,
    onPinToWorking
  }: Props = $props();

  function readable(value: string): string {
    return value.replaceAll('-', ' ');
  }

  function outputPreview(value: unknown): string {
    if (value === null || value === undefined) return 'No structured output yet.';
    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return 'Structured output could not be displayed.';
    }
  }
</script>

<section data-testid="agent-runtime-inspector" class="panel" aria-label="Agent runtime inspector">
  <header data-testid="agent-runtime-inspector-header" class="panel-header">
    <div data-testid="agent-runtime-inspector-title" class="panel-title">Agent inspector</div>
    {#if agent}
      <span data-testid="agent-runtime-inspector-provenance" class="provenance">{agent.provenance === 'workflow' ? 'Workflow' : 'Provider-native'}</span>
    {/if}
  </header>

  {#if !agent}
    <p data-testid="agent-runtime-inspector-empty" class="empty">Select an agent to inspect its runtime identity.</p>
  {:else}
    <div data-testid="agent-runtime-inspector-body" class="body">
      <div data-testid="agent-runtime-identity" class="identity">
        <span data-testid="agent-runtime-provider" class="identity-title">{agent.provider}</span>
        <span data-testid="agent-runtime-owned-id" class="mono">ownedId: {agent.ownedId}</span>
        {#if agent.workflowRunId}
          <span data-testid="agent-runtime-workflow-id" class="mono">workflowRunId: {agent.workflowRunId}</span>
        {/if}
        {#if agent.nodeRunId}
          <span data-testid="agent-runtime-node-run-id" class="mono">nodeRunId: {agent.nodeRunId}</span>
        {/if}
      </div>

      <dl data-testid="agent-runtime-facts" class="facts">
        <div data-testid="agent-runtime-role"><dt>Role</dt><dd>{agent.roleId ?? 'Provider child'}</dd></div>
        <div data-testid="agent-runtime-state"><dt>State</dt><dd>{readable(agent.state)}</dd></div>
        <div data-testid="agent-runtime-attempt"><dt>Attempt</dt><dd>{agent.attempt ?? '—'}</dd></div>
        <div data-testid="agent-runtime-depth"><dt>Depth</dt><dd>{agent.depth ?? '—'}</dd></div>
        <div data-testid="agent-runtime-contract"><dt>Output contract</dt><dd>{agent.outputContract ?? 'Provider-native'}</dd></div>
        <div data-testid="agent-runtime-provider-instance"><dt>Provider instance</dt><dd>{agent.providerInstanceId ?? 'Not assigned'}</dd></div>
        <div data-testid="agent-runtime-lease"><dt>Worktree lease</dt><dd>{agent.leaseId ?? 'None'}</dd></div>
      </dl>

      <div data-testid="agent-runtime-actions" class="actions">
        <button data-testid="agent-runtime-open-conversation" type="button" onclick={() => onOpenConversation?.(agent.ownedId)}>Open conversation</button>
        <button data-testid="agent-runtime-open-worktree" type="button" disabled={!agent.leaseId} onclick={() => onOpenWorktree?.(agent.ownedId)}>Open worktree</button>
        <button data-testid="agent-runtime-pin-working" type="button" onclick={() => onPinToWorking?.(agent.ownedId)}>Pin to Working</button>
      </div>

      {#if conversationSummary}
        <section data-testid="agent-runtime-conversation" class="subsection">
          <h3 data-testid="agent-runtime-conversation-heading">Conversation</h3>
          <p data-testid="agent-runtime-conversation-copy">{conversationSummary}</p>
        </section>
      {/if}

      <div data-testid="agent-runtime-inspection-columns" class="inspection-columns">
        <section data-testid="agent-runtime-plan" class="subsection">
          <h3 data-testid="agent-runtime-plan-heading">Plan</h3>
          {#if plan.length === 0}<p data-testid="agent-runtime-plan-empty" class="muted">Not available in this snapshot.</p>{:else}<ul data-testid="agent-runtime-plan-items">{#each plan as item, index}<li data-testid={`agent-runtime-plan-item-${index}`}>{item}</li>{/each}</ul>{/if}
        </section>
        <section data-testid="agent-runtime-tasks" class="subsection">
          <h3 data-testid="agent-runtime-tasks-heading">Tasks</h3>
          {#if tasks.length === 0}<p data-testid="agent-runtime-tasks-empty" class="muted">Not available in this snapshot.</p>{:else}<ul data-testid="agent-runtime-task-items">{#each tasks as item, index}<li data-testid={`agent-runtime-task-item-${index}`}>{item}</li>{/each}</ul>{/if}
        </section>
        <section data-testid="agent-runtime-tools" class="subsection">
          <h3 data-testid="agent-runtime-tools-heading">Tools</h3>
          {#if tools.length === 0}<p data-testid="agent-runtime-tools-empty" class="muted">Not available in this snapshot.</p>{:else}<ul data-testid="agent-runtime-tool-items">{#each tools as item, index}<li data-testid={`agent-runtime-tool-item-${index}`}>{item}</li>{/each}</ul>{/if}
        </section>
      </div>

      <section data-testid="agent-runtime-output" class="subsection">
        <h3 data-testid="agent-runtime-output-heading">Structured output</h3>
        <pre data-testid="agent-runtime-output-value">{outputPreview(node?.structuredOutput)}</pre>
      </section>

      <section data-testid="agent-runtime-usage" class="subsection">
        <h3 data-testid="agent-runtime-usage-heading">Usage where available</h3>
        <p data-testid="agent-runtime-usage-copy" class="muted">{usage?.tokensUsed ?? '—'} tokens · {usage?.toolTerminalsUsed ?? '—'} tool terminals</p>
      </section>
    </div>
  {/if}
</section>

<style>
  .panel { display: flex; flex-direction: column; min-width: 0; min-height: 0; border: 1px solid var(--color-border); border-radius: 7px; background: var(--color-surface); color: var(--color-text); }
  .panel-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 9px 10px; border-bottom: 1px solid var(--color-border); }
  .panel-title { font-size: 13px; font-weight: 650; letter-spacing: .04em; text-transform: uppercase; }
  .provenance { padding: 3px 5px; border-radius: 4px; color: var(--color-accent); background: var(--color-selected); font-size: 12px; }
  .body { display: grid; gap: 10px; min-height: 0; padding: 10px; overflow: auto; }
  .identity { display: grid; gap: 3px; padding-bottom: 8px; border-bottom: 1px solid var(--color-border); }
  .identity-title { font-size: 15px; font-weight: 650; }
  .mono { overflow: hidden; color: var(--color-text-2); font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 12px; text-overflow: ellipsis; white-space: nowrap; }
  .facts { display: grid; grid-template-columns: repeat(auto-fit, minmax(130px, 1fr)); gap: 7px; margin: 0; }
  .facts div { min-width: 0; padding: 6px; border: 1px solid var(--color-border); border-radius: 5px; background: var(--color-bg); }
  dt { color: var(--color-text-3); font-size: 12px; }
  dd { margin: 3px 0 0; overflow: hidden; font-size: 13px; text-overflow: ellipsis; white-space: nowrap; }
  .actions { display: flex; flex-wrap: wrap; gap: 5px; }
  .actions button { min-height: 28px; padding: 0 7px; border: 0; border-radius: 5px; color: var(--color-text-2); background: var(--color-elevated); font-size: 12px; cursor: pointer; }
  .actions button:hover, .actions button:focus-visible { color: var(--color-text); background: var(--color-hover); outline: none; }
  .actions button:disabled { cursor: default; opacity: .5; }
  .subsection { min-width: 0; padding-top: 7px; border-top: 1px solid var(--color-border); }
  h3 { margin: 0 0 5px; font-size: 13px; font-weight: 600; }
  .subsection p { margin: 0; color: var(--color-text-2); font-size: 13px; }
  ul { display: grid; gap: 3px; margin: 0; padding-left: 16px; color: var(--color-text-2); font-size: 12px; }
  pre { max-height: 180px; margin: 0; padding: 7px; overflow: auto; border: 1px solid var(--color-border); border-radius: 5px; color: var(--color-text-2); background: var(--color-bg); font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 12px; white-space: pre-wrap; word-break: break-word; }
  .muted { color: var(--color-text-3) !important; }
  .empty { margin: 0; padding: 14px; color: var(--color-text-2); font-size: 13px; }
</style>
