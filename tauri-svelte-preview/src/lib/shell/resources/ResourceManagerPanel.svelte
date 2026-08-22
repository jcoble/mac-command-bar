<script lang="ts">
  /**
   * ResourceManagerPanel.svelte — what is running, what it has been doing, and
   * where the disk went.
   *
   * Three things share one panel because they answer one question. The tree
   * says what is running now. The line beside each row says what it has been
   * doing for the last few minutes, which is the difference between a spike and
   * a leak. The disk section at the bottom says which folders grew while nobody
   * was looking.
   *
   * Two of those rows can act, and both act only through a dialog that names
   * the exact thing: the process ids about to be signalled, or the folder about
   * to be removed. Nothing here ever stops or removes anything on its own.
   *
   * The Playwright card sits at the bottom for the same reason: browsers left
   * behind by an interrupted test run are machine-wide, look exactly like the
   * user's own Chrome in Activity Monitor, and are the thing people come to this
   * panel to find.
   */
  import Square from '@lucide/svelte/icons/square';
  import Trash2 from '@lucide/svelte/icons/trash-2';

  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { resourceDiagnostics } from '$lib/shell/resourceDiagnostics.svelte';

  import PlaywrightCard from '$lib/shell/components/processes/PlaywrightCard.svelte';

  import ResourceConfirmDialog from './ResourceConfirmDialog.svelte';
  import Sparkline from './Sparkline.svelte';
  import {
    buildReclaimRequest,
    describeReclaimQuestion,
    formatDiskMeasuredAgo,
    shapeDiskReport
  } from './resourceDiskViewModel';
  import {
    askToReclaimDiskEntry,
    cancelReclaimDiskEntry,
    confirmReclaimDiskEntry,
    loadResourceDiskUsage,
    resourceDiskState,
    resourceReclaimState
  } from './resourceDiskStore.svelte';
  import { describeStopQuestion, type ResourceStopTarget } from './resourceStopModel';
  import {
    formatResourceBytes,
    formatResourceCpu,
    resourceSessionKindLabel,
    shapeResourceSample
  } from './resourceSampleViewModel';
  import {
    askToStopResource,
    cancelStopResource,
    confirmStopResource,
    refreshResourceSample,
    resourceSampleState,
    resourceStopState,
    setResourceManagerOpen
  } from './resourceSampleStore.svelte';
  import type { DiskUsageEntry } from './resourceDiskTypes';
  import type { ResourceSessionView } from './resourceSampleViewModel';

  let collapsed = $state<Record<string, boolean>>({});
  let diskOpen = $state(false);
  let nowMs = $state(Date.now());

  let view = $derived(
    resourceSampleState.sample ? shapeResourceSample(resourceSampleState.sample) : null
  );
  let diskView = $derived(
    resourceDiskState.report ? shapeDiskReport(resourceDiskState.report) : null
  );
  let stopQuestion = $derived(
    resourceStopState.target ? describeStopQuestion(resourceStopState.target) : null
  );
  let reclaimQuestion = $derived(
    resourceReclaimState.entry ? describeReclaimQuestion(resourceReclaimState.entry) : null
  );

  function toggle(id: string): void {
    collapsed = { ...collapsed, [id]: !collapsed[id] };
  }

  /** Measuring a workspace is slow, so it happens when the section is opened. */
  function toggleDisk(): void {
    diskOpen = !diskOpen;
    nowMs = Date.now();
    if (diskOpen && !resourceDiskState.report && !resourceDiskState.loading) {
      void loadResourceDiskUsage();
    }
  }

  function stopSession(workspace: string, session: ResourceSessionView): void {
    const target: ResourceStopTarget = {
      scope: 'session',
      label: `${session.label} in ${workspace}`,
      ownedId: session.ownedId,
      rootPid: session.rootPid,
      pids: session.processes.map((process) => process.pid)
    };
    askToStopResource(target);
  }

  function stopProcess(session: ResourceSessionView, pid: number, name: string): void {
    askToStopResource({
      scope: 'process',
      label: `${name} (PID ${pid})`,
      ownedId: session.ownedId,
      rootPid: pid,
      pids: [pid]
    });
  }

  function reclaim(entry: DiskUsageEntry): void {
    askToReclaimDiskEntry(entry);
  }

  function closeOnEscape(event: KeyboardEvent): void {
    if (resourceStopState.target || resourceReclaimState.entry) return;
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
        <p class="headline-totals" aria-label="Combined CPU, physical footprint and resident RSS usage">
          <span>{formatResourceCpu(resourceSampleState.sample.totals.cpuPercent)}</span>
          <span aria-hidden="true">·</span>
          <strong>{formatResourceBytes(resourceSampleState.sample.totals.physicalFootprintBytes)} Σ Physical footprint</strong>
          <span>RSS {formatResourceBytes(resourceSampleState.sample.totals.rssBytes)}</span>
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
    <span class="trend">Trend</span>
    <span>CPU%</span>
    <span>Physical footprint</span>
    <span></span>
  </div>

  <div class="manager-body">
    {#if resourceSampleState.error}
      <p class="message error">{resourceSampleState.error}</p>
    {/if}
    {#if resourceStopState.error}
      <p class="message error">{resourceStopState.error}</p>
    {:else if resourceStopState.receipt}
      <p class="message receipt">{resourceStopState.receipt}</p>
    {/if}
    {#if resourceSampleState.sample}
      <section class="diagnostic-strip" aria-label="Assembly physical process footprint">
        {#each resourceSampleState.sample.processCategories as category (category.label)}
          <span><strong>{formatResourceBytes(category.physicalFootprintBytes)}</strong> {category.label}</span>
        {/each}
      </section>
      <section class="diagnostic-strip" aria-label="Assembly lifecycle counts">
        <span><strong>{resourceSampleState.sample.diagnostics.conversations.durableSessionRows}</strong> SQLite sessions</span>
        <span><strong>{resourceSampleState.sample.diagnostics.conversations.liveSessionOverlays}</strong> overlays</span>
        <span><strong>{resourceSampleState.sample.diagnostics.conversations.liveRuntimeHandles}</strong> ACP runtimes</span>
        <span><strong>{resourceSampleState.sample.diagnostics.conversations.sidecarProcesses}</strong> sidecars</span>
        <span><strong>{resourceSampleState.sample.diagnostics.conversations.activeTurns}</strong> active turns</span>
        <span><strong>{resourceSampleState.sample.diagnostics.database.sessionStoreOpenHandles}</strong> DB handles</span>
        <span><strong>{resourceSampleState.sample.diagnostics.database.sessionStoreActiveOperations}</strong> DB active</span>
        <span><strong>{resourceSampleState.sample.diagnostics.database.sessionStoreOperations}</strong> DB ops</span>
        <span><strong>{resourceSampleState.sample.diagnostics.terminals.userPtys}</strong> user PTYs</span>
        <span><strong>{resourceSampleState.sample.diagnostics.terminals.agentToolPtys}</strong> tool PTYs</span>
        <span><strong>{resourceSampleState.sample.diagnostics.terminals.transcriptProjections}</strong> transcript projections</span>
        <span><strong>{resourceSampleState.sample.diagnostics.streams.workers}</strong> stream workers</span>
        <span><strong>{resourceSampleState.sample.diagnostics.streams.channels}</strong> projection channels</span>
        <span><strong>{resourceSampleState.sample.diagnostics.streams.queuedFrames}</strong> queued frames</span>
        <span><strong>{resourceSampleState.sample.diagnostics.streams.queuedBytes}</strong> queued bytes</span>
        <span><strong>{resourceSampleState.sample.diagnostics.languageServers.runningProcesses}</strong> LSPs</span>
        <span><strong>{resourceSampleState.sample.diagnostics.browser.nativeViews}</strong> browser views</span>
      </section>
    {/if}
    {#if import.meta.env.DEV}
      <section class="diagnostic-strip" aria-label="Frontend lifecycle counts">
        <span><strong>{resourceDiagnostics.loadedConversationProjections}</strong> loaded conversations</span>
        <span><strong>{resourceDiagnostics.loadedConversationEventBytes}</strong> conversation bytes</span>
        <span><strong>{resourceDiagnostics.loadedChildTranscriptBytes}</strong> child transcript bytes</span>
        <span><strong>{resourceDiagnostics.tauriRootListeners}</strong> Tauri listeners</span>
        <span><strong>{resourceDiagnostics.tauriEventSubscribers}</strong> JS event subscribers</span>
        <span><strong>{resourceDiagnostics.tauriChannels}</strong> Tauri Channels</span>
        <span><strong>{resourceDiagnostics.fileWatchers}</strong> file watchers</span>
        <span><strong>{resourceDiagnostics.objectUrls}</strong> object URLs</span>
        <span><strong>{resourceDiagnostics.attachmentObjectUrls}</strong> attachment URLs</span>
        <span><strong>{resourceDiagnostics.animationFrames}</strong> animation frames</span>
        <span><strong>{resourceDiagnostics.loadedTreeNodes}</strong> tree nodes</span>
        <span><strong>{resourceDiagnostics.loadedGitHistoryRows}</strong> Git rows</span>
        <span><strong>{resourceDiagnostics.activeDiffs}</strong> active diffs</span>
        <span><strong>{resourceDiagnostics.xtermViews}</strong> xterm views</span>
        <span><strong>{resourceDiagnostics.codeMirrorEditorViews}</strong> mounted EditorViews</span>
        <span><strong>{resourceDiagnostics.codeMirrorEditorStates}</strong> retained EditorStates</span>
        <span><strong>{resourceDiagnostics.codeMirrorDocBytes}</strong> editor bytes</span>
        <span><strong>{resourceDiagnostics.codeMirrorUndoDepth}</strong> active undo depth</span>
        <span><strong>{resourceDiagnostics.openTabDocumentBytes}</strong> active open-tab document bytes</span>
        <span><strong>{resourceDiagnostics.mergeViews}</strong> mounted MergeViews</span>
        <span><strong>{resourceDiagnostics.mergeDocBytes}</strong> diff bytes</span>
        <span><strong>{resourceDiagnostics.elementVisibilityWatchers}</strong> visibility watchers</span>
        <span><strong>{resourceDiagnostics.railElapsedWatchers}</strong> rail watchers</span>
        <span><strong>{resourceDiagnostics.semanticWaitingSpots}</strong> semantic waits</span>
        <span><strong>{resourceDiagnostics.semanticSchedulerInFlight}</strong> semantic inflight</span>
        <span><strong>{resourceDiagnostics.semanticRetryTimers}</strong> semantic retries</span>
      </section>
    {/if}

    {#if view}
      {#if view.groups.length === 0}
        <p class="message empty">No terminal or conversation processes are running.</p>
      {:else}
        <div class="workspace-list" data-testid="resource-manager-tree">
          {#each view.groups as group (group.id)}
            <section class="workspace-group">
              <div class="row-shell">
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
                  <span class="trend"><Sparkline values={group.history.physicalFootprintBytes} unit="memory" /></span>
                  <span class="metric">{formatResourceCpu(group.totals.cpuPercent)}</span>
                  <span class="metric">{formatResourceBytes(group.totals.physicalFootprintBytes)}</span>
                </button>
              </div>

              {#if !collapsed[group.id]}
                {#each group.sessions as session (session.id)}
                  <div class="session-group">
                    <div class="row-shell">
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
                        <span class="trend"><Sparkline values={session.history.cpuPercent} unit="cpu" /></span>
                        <span class="metric">{formatResourceCpu(session.totals.cpuPercent)}</span>
                        <span class="metric">{formatResourceBytes(session.totals.physicalFootprintBytes)}</span>
                      </button>
                      <span class="row-action">
                        <IconButton
                          size="xs"
                          label={`Stop ${session.label} and its ${session.totals.processCount} ${session.totals.processCount === 1 ? 'process' : 'processes'}`}
                          onclick={() => stopSession(group.workspace, session)}
                        >
                          <Square size={12} />
                        </IconButton>
                      </span>
                    </div>

                    {#if !collapsed[session.id]}
                      {#each session.processes as process (process.pid)}
                        <div class="row-shell">
                          <div class="tree-row process-row">
                            <span class="process-name">
                              <strong>{process.name}</strong>
                              <small>PID {process.pid}</small>
                            </span>
                            <span class="trend"></span>
                            <span class="metric">{formatResourceCpu(process.cpuPercent)}</span>
                            <span class="metric">{formatResourceBytes(process.physicalFootprintBytes)}</span>
                          </div>
                          <span class="row-action">
                            <IconButton
                              size="xs"
                              label={`Stop ${process.name}, PID ${process.pid}`}
                              onclick={() => stopProcess(session, process.pid, process.name)}
                            >
                              <Square size={12} />
                            </IconButton>
                          </span>
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
          <span class="trend"><Sparkline values={view.appHistory.physicalFootprintBytes} unit="memory" /></span>
          <span class="metric">{formatResourceCpu(view.appTotals.cpuPercent)}</span>
          <span class="metric">{formatResourceBytes(view.appTotals.physicalFootprintBytes)}</span>
        </div>
        {#if view.appParts.length === 0}
          <p class="message app-empty">The app process tree is not available in this sample.</p>
        {:else}
          {#each view.appParts as part (part.pid)}
            <div class="tree-row process-row app-part">
              <span class="process-name"><strong>{part.label}</strong><small>PID {part.pid}</small></span>
              <span class="trend"></span>
              <span class="metric">{formatResourceCpu(part.cpuPercent)}</span>
              <span class="metric">{formatResourceBytes(part.physicalFootprintBytes)}</span>
            </div>
          {/each}
        {/if}
      </section>
    {:else if resourceSampleState.loading}
      <p class="message">Reading the live process tree…</p>
    {:else}
      <p class="message">Resource usage is available in the desktop app.</p>
    {/if}

    <section class="disk-group" aria-labelledby="disk-title">
      <div class="row-shell">
        <button type="button" class="tree-row disk-row" aria-expanded={diskOpen} onclick={toggleDisk}>
          <span class="tree-label">
            <span class:collapsed={!diskOpen} class="disclosure" aria-hidden="true">▾</span>
            <span>
              <small>Disk</small>
              <strong id="disk-title">Where the space went</strong>
            </span>
          </span>
          <span class="trend"></span>
          <span class="metric"></span>
          <span class="metric">{diskView ? diskView.totalLabel : ''}</span>
        </button>
      </div>

      {#if diskOpen}
        {#if resourceDiskState.loading && !diskView}
          <p class="message disk-message">Measuring the known large folders…</p>
        {/if}
        {#if resourceDiskState.error}
          <p class="message error disk-message">{resourceDiskState.error}</p>
        {/if}
        {#if resourceReclaimState.error}
          <p class="message error disk-message">{resourceReclaimState.error}</p>
        {:else if resourceReclaimState.receipt}
          <p class="message receipt disk-message">{resourceReclaimState.receipt}</p>
        {/if}

        {#if diskView}
          <p class="disk-summary">
            <span>{diskView.reclaimableLabel} of that is build output and installed dependencies.</span>
            <button
              type="button"
              class="quiet-action"
              onclick={() => void loadResourceDiskUsage(true)}
              disabled={resourceDiskState.loading}
            >
              {resourceDiskState.loading ? 'Measuring…' : 'Measure again'}
            </button>
          </p>
          {#each diskView.sections as section (section.id)}
            <div class="tree-row disk-section-row">
              <span class="tree-label">
                <span>
                  <strong>{section.label}</strong>
                  <small>{section.root || 'App data'} · {formatDiskMeasuredAgo(section.measuredAtMs, nowMs)}</small>
                </span>
              </span>
              <span class="trend"></span>
              <span class="metric"></span>
              <span class="metric">{section.sizeLabel}</span>
            </div>
            {#each section.entries as entry (entry.id)}
              <div class="row-shell">
                <div class="tree-row process-row disk-entry-row">
                  <span class="process-name">
                    <strong>{entry.label}</strong>
                    <small>{entry.categoryLabel}{entry.truncated ? ' · at least' : ''}</small>
                  </span>
                  <span class="trend"></span>
                  <span class="metric"></span>
                  <span class="metric">{entry.sizeLabel}</span>
                </div>
                {#if buildReclaimRequest(entry)}
                  <span class="row-action">
                    <IconButton
                      size="xs"
                      label={`Remove ${entry.path}, ${entry.sizeLabel}`}
                      onclick={() => reclaim(entry)}
                    >
                      <Trash2 size={12} />
                    </IconButton>
                  </span>
                {/if}
              </div>
            {/each}
          {/each}
        {/if}
      {/if}
    </section>

    <section class="playwright-group" aria-label="Playwright processes">
      <PlaywrightCard />
    </section>
  </div>
</dialog>

<ResourceConfirmDialog
  question={stopQuestion}
  open={resourceStopState.target !== null}
  busy={resourceStopState.busy}
  onCancel={cancelStopResource}
  onConfirm={() => void confirmStopResource()}
/>

<ResourceConfirmDialog
  question={reclaimQuestion}
  open={resourceReclaimState.entry !== null}
  busy={resourceReclaimState.busy}
  onCancel={cancelReclaimDiskEntry}
  onConfirm={() => void confirmReclaimDiskEntry()}
/>

<style>
  .resource-manager {
    position: fixed;
    right: 8px;
    bottom: 34px;
    z-index: 140;
    display: grid;
    width: min(860px, calc(100vw - 16px));
    max-height: min(74vh, 720px);
    grid-template-rows: auto auto minmax(0, 1fr);
    overflow: hidden;
    border: 1px solid var(--color-border);
    border-radius: 7px;
    background: var(--color-surface);
    color: var(--color-text);
    box-shadow: var(--shadow-lg);
    font-size: 13px;
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
  small { font-size: 12px; }
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
    grid-template-columns: minmax(0, 1fr) 70px 66px 88px;
    align-items: center;
  }
  .column-headings { grid-template-columns: minmax(0, 1fr) 70px 66px 88px 34px; }
  .column-headings {
    min-height: 25px;
    padding: 0 12px;
    border-bottom: 1px solid var(--color-border);
    color: var(--color-text-3);
    background: var(--color-elevated);
    font-size: 12px;
  }
  .column-headings span:not(:first-child) { text-align: right; }
  .column-headings .trend { text-align: left; padding-left: 4px; }
  .manager-body { min-height: 0; overflow: auto; overscroll-behavior: contain; }

  .workspace-group + .workspace-group, .app-group, .disk-group, .playwright-group { border-top: 1px solid var(--color-border); }
  .row-shell { position: relative; }
  .tree-row { box-sizing: border-box; min-height: 34px; width: 100%; padding: 5px 40px 5px 12px; }
  button.tree-row { border: 0; text-align: left; color: inherit; cursor: pointer; }
  button.tree-row:hover { background: var(--color-hover); }
  .workspace-row, .disk-row { background: var(--color-surface); }
  .session-row { padding-left: 28px; background: color-mix(in srgb, var(--color-surface) 74%, var(--color-bg)); }
  .process-row { min-height: 31px; padding-left: 58px; border-top: 1px solid color-mix(in srgb, var(--color-border) 58%, transparent); background: var(--color-bg); }
  .app-row { background: var(--color-surface); }
  .app-part { padding-left: 38px; }
  .disk-section-row { padding-left: 28px; background: color-mix(in srgb, var(--color-surface) 74%, var(--color-bg)); }
  .disk-entry-row { padding-left: 38px; }
  .disk-summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 12px 6px 28px;
    color: var(--color-text-2);
    background: var(--color-bg);
  }
  .disk-message { padding-left: 28px; background: var(--color-bg); }
  .diagnostic-strip {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg);
  }
  .diagnostic-strip span {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
    min-height: 24px;
    padding: 3px 7px;
    border: 1px solid var(--color-border);
    border-radius: 4px;
    color: var(--color-text-2);
    background: var(--color-elevated);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .diagnostic-strip strong { color: var(--color-text); font-weight: 650; }

  /*
    The action stays visible rather than appearing on hover: a control that is
    only there when the pointer is over it cannot be found by anybody reading
    the panel, and it is the one control on the row that ends real work.
  */
  .row-action {
    position: absolute;
    top: 50%;
    right: 6px;
    transform: translateY(-50%);
    opacity: 0.6;
    transition: opacity 120ms ease;
  }
  .row-shell:hover .row-action,
  .row-action:focus-within { opacity: 1; }

  .tree-label, .process-name { display: flex; min-width: 0; align-items: center; gap: 8px; }
  .tree-label > span:last-child, .process-name { min-width: 0; }
  .tree-label > span:last-child, .process-name { display: grid; gap: 1px; }
  .tree-label strong, .process-name strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 560; }
  .disclosure { display: inline-grid; width: 10px; place-items: center; color: var(--color-text-3); transition: transform 120ms ease; }
  .disclosure.collapsed { transform: rotate(-90deg); }
  .app-mark { width: 6px; height: 14px; border-radius: 2px; background: var(--color-accent); }
  .trend { display: flex; justify-content: flex-start; padding-left: 4px; }
  .metric { text-align: right; color: var(--color-text-2); font-variant-numeric: tabular-nums; }
  .message { padding: 18px 14px; color: var(--color-text-2); }
  .error { color: var(--color-bad); border-bottom: 1px solid var(--color-border); }
  .receipt { padding: 10px 14px; border-bottom: 1px solid var(--color-border); }
  .empty { border-bottom: 1px solid var(--color-border); }
  .app-empty { padding-left: 38px; background: var(--color-bg); }

  @media (max-width: 720px) {
    .manager-header { align-items: flex-start; gap: 10px; }
    .header-actions { gap: 3px; }
    .headline-totals { display: none; }
    .column-headings, .tree-row { grid-template-columns: minmax(0, 1fr) 58px 76px; }
    .column-headings { grid-template-columns: minmax(0, 1fr) 58px 76px 34px; }
    .trend { display: none; }
    .session-row, .disk-section-row { padding-left: 20px; }
    .process-row, .disk-entry-row { padding-left: 38px; }
  }

  @media (prefers-reduced-motion: reduce) {
    .disclosure, .row-action { transition: none; }
  }
</style>
