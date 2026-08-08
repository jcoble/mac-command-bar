<script lang="ts">
  import type { WorkflowRunView } from '$lib/shell/workflows/workflowTypes';

  interface Props {
    run: WorkflowRunView;
    busy?: boolean;
    onStart?(): void;
    onPause?(): void;
    onResume?(): void;
    onCancel?(): void;
    onSaveAsTemplate?(): void;
  }

  let {
    run,
    busy = false,
    onStart,
    onPause,
    onResume,
    onCancel,
    onSaveAsTemplate
  }: Props = $props();

  function readable(value: string): string {
    return value.replaceAll('-', ' ');
  }

  const canCancel = $derived(
    ['draft', 'queued', 'running', 'waiting-approval', 'waiting-input', 'paused'].includes(run.state)
  );
  const canStart = $derived(run.state === 'draft' || run.state === 'queued');
  const canPause = $derived(run.state === 'running');
  const canResume = $derived(run.state === 'paused');
</script>

<header data-testid="workflow-run-header" class="run-header">
  <div data-testid="workflow-run-header-title" class="header-title">
    <div data-testid="workflow-run-header-kicker" class="kicker">Workflow run</div>
    <h1 data-testid="workflow-run-header-name">{run.definition.name}</h1>
    <p data-testid="workflow-run-header-id" class="run-id">{run.workflowRunId}</p>
  </div>

  <div data-testid="workflow-run-header-status" class="header-status">
    <span data-testid="workflow-run-header-state" class={`state state-${run.state}`}>{readable(run.state)}</span>
    <span data-testid="workflow-run-header-phase" class="phase">Loop phase: {readable(run.phase)}</span>
    <span data-testid="workflow-run-header-provenance" class="provenance">{run.provenance === 'workflow' ? 'Workflow' : 'Provider-native'}</span>
  </div>

  <div data-testid="workflow-run-header-metrics" class="metrics">
    <span data-testid="workflow-run-metric-agents">{run.nodes.length} agents</span>
    <span data-testid="workflow-run-metric-tokens">{run.tokensUsed} tokens</span>
    <span data-testid="workflow-run-metric-terminals">{run.toolTerminalsUsed} terminals</span>
    <span data-testid="workflow-run-metric-worktrees">{run.worktreesAllocated} worktrees</span>
    <span data-testid="workflow-run-metric-sequence">receipt {run.lastSequence}</span>
  </div>

  <div data-testid="workflow-run-header-actions" class="actions">
    {#if canStart}
      <button data-testid="workflow-run-start" class="action accent" type="button" disabled={busy} onclick={() => onStart?.()}>Start</button>
    {/if}
    {#if canPause}
      <button data-testid="workflow-run-pause" class="action" type="button" disabled={busy} onclick={() => onPause?.()}>Pause</button>
    {/if}
    {#if canResume}
      <button data-testid="workflow-run-resume" class="action" type="button" disabled={busy} onclick={() => onResume?.()}>Resume</button>
    {/if}
    {#if canCancel}
      <button data-testid="workflow-run-cancel" class="action danger" type="button" disabled={busy} onclick={() => onCancel?.()}>Cancel</button>
    {/if}
    <button data-testid="workflow-run-save-template" class="action" type="button" disabled={busy} onclick={() => onSaveAsTemplate?.()}>Save as template</button>
  </div>
</header>

<style>
  .run-header { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 10px 16px; padding: 15px 18px; border-bottom: 1px solid var(--color-border); background: var(--color-surface); color: var(--color-text); }
  .header-title, .header-status, .metrics, .actions { min-width: 0; }
  .kicker { color: var(--color-accent); font-size: 12px; letter-spacing: .08em; text-transform: uppercase; }
  h1 { margin: 2px 0 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 20px; font-weight: 650; }
  .run-id { margin: 3px 0 0; overflow: hidden; color: var(--color-text-3); font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 12px; text-overflow: ellipsis; white-space: nowrap; }
  .header-status { display: flex; align-items: flex-start; justify-content: flex-end; gap: 7px; flex-wrap: wrap; }
  .state, .provenance { height: fit-content; padding: 3px 6px; border-radius: 4px; font-size: 12px; text-transform: capitalize; }
  .state { color: var(--color-text-2); background: var(--color-elevated); }
  .state-running, .state-queued { color: var(--color-live); background: var(--color-live-bg); }
  .state-completed { color: var(--color-good); background: var(--color-good-bg); }
  .state-failed, .state-cancelled { color: var(--color-bad); background: var(--color-bad-bg); }
  .state-waiting-approval, .state-waiting-input, .state-paused { color: var(--color-attention); background: var(--color-attention-bg); }
  .phase { padding: 3px 0; color: var(--color-text-2); font-size: 12px; }
  .provenance { color: var(--color-accent); background: var(--color-selected); text-transform: none; }
  .metrics { display: flex; flex-wrap: wrap; gap: 6px 12px; color: var(--color-text-2); font-size: 12px; }
  .actions { display: flex; justify-content: flex-end; flex-wrap: wrap; gap: 6px; }
  .action { min-height: 29px; padding: 0 9px; border: 0; border-radius: 5px; color: var(--color-text-2); background: var(--color-elevated); font-size: 12px; cursor: pointer; }
  .action:hover, .action:focus-visible { color: var(--color-text); background: var(--color-hover); }
  .action.accent { color: var(--color-on-accent); background: var(--color-accent); }
  .action.danger { color: var(--color-bad); background: var(--color-bad-bg); }
  .action:disabled { cursor: default; opacity: .55; }
  @media (max-width: 680px) {
    .run-header { grid-template-columns: 1fr; }
    .header-status, .actions { justify-content: flex-start; }
  }
</style>
