<script lang="ts">
  import {
    formatResourceBytes,
    formatResourceCpu,
    resourceSessionKindLabel,
    shapeResourceSample
  } from './resourceSampleViewModel';
  import {
    refreshResourceSample,
    resourceSampleState,
    setResourceManagerOpen
  } from './resourceSampleStore.svelte';

  let collapsed = $state<Record<string, boolean>>({});
  let view = $derived(
    resourceSampleState.sample ? shapeResourceSample(resourceSampleState.sample) : null
  );

  function toggle(id: string): void {
    collapsed = { ...collapsed, [id]: !collapsed[id] };
  }

  function closeOnEscape(event: KeyboardEvent): void {
    if (event.key === 'Escape') setResourceManagerOpen(false);
  }
</script>

<svelte:window onkeydown={closeOnEscape} />

<dialog id="resource-manager-panel" open class="resource-manager" aria-modal="false" aria-labelledby="resource-manager-title">
  <header class="manager-header">
    <div>
      <p class="context">Live process footprint</p>
      <h2 id="resource-manager-title">Resource Manager</h2>
    </div>
    <div class="header-actions">
      {#if resourceSampleState.sample}
        <p class="headline-totals" aria-label="Combined CPU and memory usage">
          <span>{formatResourceCpu(resourceSampleState.sample.totals.cpuPercent)}</span>
          <span aria-hidden="true">·</span>
          <strong>{formatResourceBytes(resourceSampleState.sample.totals.rssBytes)} Σ RSS</strong>
        </p>
      {/if}
      <button
        type="button"
        class="quiet-action"
        onclick={() => void refreshResourceSample()}
        disabled={resourceSampleState.loading}
      >
        {resourceSampleState.loading ? 'Reading…' : 'Refresh'}
      </button>
      <button
        type="button"
        class="close-action"
        aria-label="Close Resource Manager"
        onclick={() => setResourceManagerOpen(false)}
      >×</button>
    </div>
  </header>

  <div class="column-headings" aria-hidden="true">
    <span>Workspace / session / process</span>
    <span>CPU%</span>
    <span>RSS</span>
  </div>

  <div class="manager-body">
    {#if resourceSampleState.error}
      <p class="message error">{resourceSampleState.error}</p>
    {/if}

    {#if view}
      {#if view.groups.length === 0}
        <p class="message empty">No terminal or conversation processes are running.</p>
      {:else}
        <div class="workspace-list" data-testid="resource-manager-tree">
          {#each view.groups as group (group.id)}
            <section class="workspace-group">
              <button
                type="button"
                class="tree-row workspace-row"
                aria-expanded={!collapsed[group.id]}
                onclick={() => toggle(group.id)}
              >
                <span class="tree-label">
                  <span class:collapsed={collapsed[group.id]} class="disclosure" aria-hidden="true">▾</span>
                  <span><small>Workspace</small><strong>{group.workspace}</strong></span>
                </span>
                <span class="metric">{formatResourceCpu(group.totals.cpuPercent)}</span>
                <span class="metric">{formatResourceBytes(group.totals.rssBytes)}</span>
              </button>

              {#if !collapsed[group.id]}
                {#each group.sessions as session (session.id)}
                  <div class="session-group">
                    <button
                      type="button"
                      class="tree-row session-row"
                      aria-expanded={!collapsed[session.id]}
                      onclick={() => toggle(session.id)}
                    >
                      <span class="tree-label">
                        <span class:collapsed={collapsed[session.id]} class="disclosure" aria-hidden="true">▾</span>
                        <span>
                          <strong>{session.label}</strong>
                          <small>{resourceSessionKindLabel(session.kind)} · {session.totals.processCount} {session.totals.processCount === 1 ? 'process' : 'processes'}</small>
                        </span>
                      </span>
                      <span class="metric">{formatResourceCpu(session.totals.cpuPercent)}</span>
                      <span class="metric">{formatResourceBytes(session.totals.rssBytes)}</span>
                    </button>

                    {#if !collapsed[session.id]}
                      {#each session.processes as process (process.pid)}
                        <div class="tree-row process-row">
                          <span class="process-name">
                            <strong>{process.name}</strong>
                            <small>PID {process.pid}</small>
                          </span>
                          <span class="metric">{formatResourceCpu(process.cpuPercent)}</span>
                          <span class="metric">{formatResourceBytes(process.rssBytes)}</span>
                        </div>
                      {/each}
                    {/if}
                  </div>
                {/each}
              {/if}
            </section>
          {/each}
        </div>
      {/if}

      <section class="app-group" aria-labelledby="this-app-title">
        <div class="tree-row app-row">
          <span class="tree-label">
            <span class="app-mark" aria-hidden="true"></span>
            <span><small>Application</small><strong id="this-app-title">This app</strong></span>
          </span>
          <span class="metric">{formatResourceCpu(view.appTotals.cpuPercent)}</span>
          <span class="metric">{formatResourceBytes(view.appTotals.rssBytes)}</span>
        </div>
        {#if view.appParts.length === 0}
          <p class="message app-empty">The app process tree is not available in this sample.</p>
        {:else}
          {#each view.appParts as part (part.pid)}
            <div class="tree-row process-row app-part">
              <span class="process-name"><strong>{part.label}</strong><small>PID {part.pid}</small></span>
              <span class="metric">{formatResourceCpu(part.cpuPercent)}</span>
              <span class="metric">{formatResourceBytes(part.rssBytes)}</span>
            </div>
          {/each}
        {/if}
      </section>
    {:else if resourceSampleState.loading}
      <p class="message">Reading the live process tree…</p>
    {:else}
      <p class="message">Resource usage is available in the desktop app.</p>
    {/if}
  </div>
</dialog>

<style>
  .resource-manager {
    position: fixed;
    right: 8px;
    bottom: 34px;
    z-index: 140;
    display: grid;
    width: min(760px, calc(100vw - 16px));
    max-height: min(74vh, 720px);
    grid-template-rows: auto auto minmax(0, 1fr);
    overflow: hidden;
    border: 1px solid var(--color-border);
    border-radius: 7px;
    background: var(--color-surface);
    color: var(--color-text);
    box-shadow: var(--shadow-lg);
    font-size: 12px;
    margin: 0;
  }

  .manager-header {
    display: flex;
    min-height: 54px;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding: 8px 10px 8px 14px;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg);
  }

  h2, p { margin: 0; }
  h2 { font-size: 16px; line-height: 1.2; font-weight: 650; }
  .context, small { color: var(--color-text-2); }
  .context { margin-bottom: 2px; font-size: 12px; letter-spacing: 0.035em; }
  .header-actions { display: flex; align-items: center; gap: 8px; }
  .headline-totals { display: flex; align-items: baseline; gap: 6px; white-space: nowrap; font-variant-numeric: tabular-nums; }
  .headline-totals span { color: var(--color-text-2); }

  button { font: inherit; }
  .quiet-action, .close-action {
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--color-text-2);
    cursor: pointer;
  }
  .quiet-action { padding: 5px 7px; }
  .close-action { width: 26px; height: 26px; padding: 0; font-size: 19px; line-height: 1; }
  .quiet-action:hover:not(:disabled), .close-action:hover { background: var(--color-hover); color: var(--color-text); }
  button:focus-visible { outline: none; box-shadow: var(--focus-ring); }
  button:disabled { cursor: wait; opacity: 0.55; }

  .column-headings, .tree-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 74px 92px;
    align-items: center;
  }
  .column-headings {
    min-height: 25px;
    padding: 0 12px;
    border-bottom: 1px solid var(--color-border);
    color: var(--color-text-3);
    background: var(--color-elevated);
    font-size: 12px;
  }
  .column-headings span:not(:first-child) { text-align: right; }
  .manager-body { min-height: 0; overflow: auto; overscroll-behavior: contain; }

  .workspace-group + .workspace-group, .app-group { border-top: 1px solid var(--color-border); }
  .tree-row { box-sizing: border-box; min-height: 34px; width: 100%; padding: 5px 12px; }
  button.tree-row { border: 0; text-align: left; color: inherit; cursor: pointer; }
  button.tree-row:hover { background: var(--color-hover); }
  .workspace-row { background: var(--color-surface); }
  .session-row { padding-left: 28px; background: color-mix(in srgb, var(--color-surface) 74%, var(--color-bg)); }
  .process-row { min-height: 31px; padding-left: 58px; border-top: 1px solid color-mix(in srgb, var(--color-border) 58%, transparent); background: var(--color-bg); }
  .app-row { background: var(--color-surface); }
  .app-part { padding-left: 38px; }

  .tree-label, .process-name { display: flex; min-width: 0; align-items: center; gap: 8px; }
  .tree-label > span:last-child, .process-name { min-width: 0; }
  .tree-label > span:last-child, .process-name { display: grid; gap: 1px; }
  .tree-label strong, .process-name strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 560; }
  .disclosure { display: inline-grid; width: 10px; place-items: center; color: var(--color-text-3); transition: transform 120ms ease; }
  .disclosure.collapsed { transform: rotate(-90deg); }
  .app-mark { width: 6px; height: 14px; border-radius: 2px; background: var(--color-accent); }
  .metric { text-align: right; color: var(--color-text-2); font-variant-numeric: tabular-nums; }
  .message { padding: 18px 14px; color: var(--color-text-2); }
  .error { color: var(--color-bad); border-bottom: 1px solid var(--color-border); }
  .empty { border-bottom: 1px solid var(--color-border); }
  .app-empty { padding-left: 38px; background: var(--color-bg); }

  @media (max-width: 620px) {
    .manager-header { align-items: flex-start; gap: 10px; }
    .header-actions { gap: 3px; }
    .headline-totals { display: none; }
    .column-headings, .tree-row { grid-template-columns: minmax(0, 1fr) 58px 76px; }
    .session-row { padding-left: 20px; }
    .process-row { padding-left: 38px; }
  }

  @media (prefers-reduced-motion: reduce) {
    .disclosure { transition: none; }
  }
</style>
