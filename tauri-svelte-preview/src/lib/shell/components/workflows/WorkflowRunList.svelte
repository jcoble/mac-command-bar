<script lang="ts">
  import type {
    WorkflowFilter,
    WorkflowRunView
  } from '$lib/shell/workflows/workflowTypes';

  interface Props {
    runs: WorkflowRunView[];
    selectedWorkflowRunId?: string | null;
    query?: string;
    filter?: WorkflowFilter;
    loading?: boolean;
    onSelect?(workflowRunId: string): void;
    onQueryChange?(query: string): void;
    onFilterChange?(filter: WorkflowFilter): void;
    onRefresh?(): void;
    onNewWorkflow?(): void;
  }

  let {
    runs,
    selectedWorkflowRunId = null,
    query = '',
    filter = 'all',
    loading = false,
    onSelect,
    onQueryChange,
    onFilterChange,
    onRefresh,
    onNewWorkflow
  }: Props = $props();

  const filterOptions: readonly { value: WorkflowFilter; label: string }[] = [
    { value: 'all', label: 'All runs' },
    { value: 'active', label: 'Active' },
    { value: 'waiting', label: 'Waiting' },
    { value: 'failed', label: 'Failed' },
    { value: 'completed', label: 'Completed' }
  ];

  function readable(value: string): string {
    return value.replaceAll('-', ' ');
  }

  function age(ms: number): string {
    const seconds = Math.max(0, Math.floor((Date.now() - ms) / 1000));
    if (seconds < 60) return `${seconds}s ago`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    return `${Math.floor(hours / 24)}d ago`;
  }
</script>

<section data-testid="workflow-run-list" class="run-list" aria-label="Workflow runs">
  <header data-testid="workflow-run-list-header" class="list-header">
    <div data-testid="workflow-run-list-title" class="title-block">
      <span data-testid="workflow-run-list-kicker" class="kicker">Agent Control Center</span>
      <h2 data-testid="workflow-run-list-heading">Workflows</h2>
    </div>
    <div data-testid="workflow-run-list-actions" class="header-actions">
      <button
        data-testid="workflow-run-refresh"
        class="quiet-button"
        type="button"
        disabled={loading}
        onclick={() => onRefresh?.()}
        title="Read workflow snapshots again"
      >
        {loading ? 'Reading…' : 'Refresh'}
      </button>
      <button
        data-testid="workflow-run-new"
        class="accent-button"
        type="button"
        onclick={() => onNewWorkflow?.()}
      >
        New workflow
      </button>
    </div>
  </header>

  <div data-testid="workflow-run-filters" class="filters">
    <label data-testid="workflow-run-search-label" class="search-label">
      <span data-testid="workflow-run-search-copy">Find a run</span>
      <input
        data-testid="workflow-run-search"
        type="search"
        value={query}
        placeholder="Name or workflow id"
        aria-label="Find a workflow run"
        oninput={(event) => onQueryChange?.((event.currentTarget as HTMLInputElement).value)}
      />
    </label>
    <label data-testid="workflow-run-filter-label" class="filter-label">
      <span data-testid="workflow-run-filter-copy">Show</span>
      <select
        data-testid="workflow-run-filter"
        value={filter}
        aria-label="Filter workflow runs"
        onchange={(event) => onFilterChange?.((event.currentTarget as HTMLSelectElement).value as WorkflowFilter)}
      >
        {#each filterOptions as option (option.value)}
          <option data-testid={`workflow-run-filter-option-${option.value}`} value={option.value}>{option.label}</option>
        {/each}
      </select>
    </label>
  </div>

  {#if runs.length === 0}
    <div data-testid="workflow-run-list-empty" class="empty-list">
      <p data-testid="workflow-run-list-empty-title">No workflow runs yet.</p>
      <p data-testid="workflow-run-list-empty-copy">Start from a template when you are ready to delegate work.</p>
    </div>
  {:else}
    <ul data-testid="workflow-run-items" class="run-items">
      {#each runs as run (run.workflowRunId)}
        <li data-testid={`workflow-run-item-${run.workflowRunId}`} class:selected={run.workflowRunId === selectedWorkflowRunId}>
          <button
            data-testid={`workflow-run-select-${run.workflowRunId}`}
            class="run-button"
            type="button"
            aria-current={run.workflowRunId === selectedWorkflowRunId ? 'true' : undefined}
            onclick={() => onSelect?.(run.workflowRunId)}
          >
            <span data-testid={`workflow-run-name-${run.workflowRunId}`} class="run-name">{run.definition.name}</span>
            <span data-testid={`workflow-run-state-${run.workflowRunId}`} class={`state state-${run.state}`}>{readable(run.state)}</span>
            <span data-testid={`workflow-run-phase-${run.workflowRunId}`} class="run-meta">
              {readable(run.phase)} · {age(run.updatedAtMs)}
            </span>
            <span data-testid={`workflow-run-provenance-${run.workflowRunId}`} class="provenance">{run.provenance === 'workflow' ? 'Workflow' : 'Provider-native'}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .run-list { display: flex; flex-direction: column; min-width: 220px; min-height: 0; height: 100%; background: var(--color-surface); border-right: 1px solid var(--color-border); color: var(--color-text); }
  .list-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; padding: 14px 12px 10px; border-bottom: 1px solid var(--color-border); }
  .title-block { min-width: 0; }
  .kicker { display: block; margin-bottom: 3px; color: var(--color-accent); font-size: 12px; letter-spacing: .08em; text-transform: uppercase; }
  h2 { margin: 0; font-size: 18px; font-weight: 650; }
  .header-actions { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 5px; }
  button, input, select { font: inherit; }
  button { cursor: pointer; }
  .quiet-button, .accent-button { min-height: 28px; padding: 0 8px; border: 1px solid var(--color-border); border-radius: 5px; color: var(--color-text-2); background: transparent; font-size: 12px; }
  .accent-button { border-color: var(--color-selected-border); color: var(--color-on-accent); background: var(--color-accent); }
  .quiet-button:hover, .quiet-button:focus-visible { color: var(--color-text); background: var(--color-hover); }
  .accent-button:hover, .accent-button:focus-visible { filter: brightness(1.08); }
  button:disabled { cursor: default; opacity: .55; }
  .filters { display: grid; gap: 8px; padding: 10px 12px; border-bottom: 1px solid var(--color-border); }
  .search-label, .filter-label { display: grid; gap: 4px; color: var(--color-text-2); font-size: 12px; }
  input, select { width: 100%; min-height: 30px; box-sizing: border-box; padding: 0 8px; border: 1px solid var(--color-border); border-radius: 5px; color: var(--color-text); background: var(--color-bg); font-size: 13px; }
  input:focus-visible, select:focus-visible, button:focus-visible { outline: 1px solid var(--color-focus-solid); outline-offset: 1px; }
  .run-items { min-height: 0; overflow: auto; margin: 0; padding: 6px; list-style: none; }
  .run-items li { margin: 0 0 4px; }
  .run-items li.selected { border-radius: 6px; background: var(--color-selected); }
  .run-button { display: grid; width: 100%; gap: 5px; padding: 9px 8px; border: 0; border-radius: 6px; color: inherit; background: transparent; text-align: left; }
  .run-button:hover { background: var(--color-hover); }
  .run-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; font-weight: 600; }
  .state { width: fit-content; padding: 2px 5px; border-radius: 4px; color: var(--color-text-2); background: var(--color-elevated); font-size: 12px; text-transform: capitalize; }
  .state-running, .state-queued { color: var(--color-live); background: var(--color-live-bg); }
  .state-completed { color: var(--color-good); background: var(--color-good-bg); }
  .state-failed, .state-cancelled { color: var(--color-bad); background: var(--color-bad-bg); }
  .state-waiting-approval, .state-waiting-input, .state-paused { color: var(--color-attention); background: var(--color-attention-bg); }
  .run-meta, .provenance { color: var(--color-text-3); font-size: 12px; }
  .provenance { color: var(--color-accent); }
  .empty-list { display: grid; gap: 5px; padding: 24px 14px; color: var(--color-text-2); }
  .empty-list p { margin: 0; }
  .empty-list p:first-child { color: var(--color-text); font-weight: 600; }
</style>
