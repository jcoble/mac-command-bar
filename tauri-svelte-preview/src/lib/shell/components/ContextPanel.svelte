<script lang="ts">
  /**
   * ContextPanel.svelte — the /next context region: six cards showing what is
   * going on around the active project (recorded agent runs, running processes,
   * Playwright's leftover browsers, agent sessions, worktrees, repositories).
   *
   * Quarried from `src/lib/WorkbenchContextPanel.svelte` (row shapes, the
   * six-row cap with a "+N more" line, the tone rules) and restyled to the
   * /next palette. Two differences from the old panel:
   *
   *  - it reads `contextStore` directly instead of taking twenty props, and
   *  - the collapsible sections, chips and badges are inlined here, so the
   *    panel carries no dependency on the old shell's component library.
   *
   * NO props, NO IO at mount, NO `$effect`. It renders an inert "nothing loaded
   * yet" line until the shell calls `activate()` on `contextService`; after that
   * the only things that reach the backend are the Refresh button and the stop
   * button on a running process, which asks first.
   *
   * WHAT A CAPPED LIST DOES AT THE BOTTOM. Every card shows six rows and then
   * says how many more there are. That count used to be a dead sentence, which
   * is the worst of both worlds: it tells you there are 368 sessions and gives
   * you no way to see one. Now each "+N more" does something. Most cards unfold
   * the next 25 rows in place. The worktrees card instead opens the Worktrees
   * view, because a worktree list you can act on already exists there and a
   * second half-copy of it in a glance panel would only disagree with it.
   *
   * Colours come from the `--color-*` tokens rather than the hex values this
   * file was written with, so changing the theme reaches this panel too.
   */
  import {
    Activity,
    Braces,
    ChevronRight,
    FolderGit2,
    GitBranch,
    Network,
    Search
  } from '@lucide/svelte';

  import PlaywrightCard from './processes/PlaywrightCard.svelte';
  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import { refreshAll, refreshCard, stopProcess } from '$lib/shell/context/contextService';
  import { contextPanelHooks } from '$lib/shell/context/contextPanelHooks.svelte';
  import {
    contextState,
    filterAgentRows,
    folderName,
    processStopUnavailableReason,
    runStatusGroup,
    summarizeAgents,
    summarizeRepositories,
    summarizeRuns,
    summarizeRuntime,
    summarizeWorktrees,
    type ContextCardKey
  } from '$lib/shell/context/contextStore.svelte';
  import { playwrightState } from '$lib/shell/processes/playwrightStore.svelte';
  import type { RuntimeContext } from '$lib/tauriSource';

  /** How many rows a card shows before it defers to a "+N more" line. */
  const ROW_LIMIT = 6;

  /** How many further rows one press of "+N more" unfolds. */
  const PAGE_SIZE = 25;

  /**
   * Which sections are open.
   *
   * Recorded agent runs starts CLOSED. On most machines it is empty — nothing
   * in this shell records a run — so an open card spends its height saying
   * nothing. Its heading still carries the count, so a machine that does have
   * runs shows that at a glance and one click opens them.
   */
  let expanded = $state<Record<ContextCardKey, boolean>>({
    runs: false,
    runtime: true,
    agents: true,
    worktrees: true,
    repositories: true
  });

  /** How many rows each card is currently showing. Bumped by "+N more". */
  let shown = $state<Record<ContextCardKey, number>>({
    runs: ROW_LIMIT,
    runtime: ROW_LIMIT,
    agents: ROW_LIMIT,
    worktrees: ROW_LIMIT,
    repositories: ROW_LIMIT
  });

  /** What the agent-session search box holds. */
  let agentQuery = $state('');

  /** The running process the "stop it?" question is being asked about. */
  let confirmingProcess = $state<RuntimeContext | null>(null);
  let confirmStopOpen = $state(false);

  function toggle(key: ContextCardKey): void {
    expanded[key] = !expanded[key];
  }

  function showMore(key: ContextCardKey): void {
    shown[key] += PAGE_SIZE;
  }

  function showFewer(key: ContextCardKey): void {
    shown[key] = ROW_LIMIT;
  }

  /**
   * The refresh button covers every card in the panel, including the Playwright
   * card next door, so "reading…" has to mean all of them are busy — otherwise
   * the button goes live again while a card is still loading.
   */
  const anyLoading = $derived(
    contextState.runs.loading ||
      contextState.runtime.loading ||
      contextState.agents.loading ||
      contextState.worktrees.loading ||
      contextState.repositories.loading ||
      playwrightState.loading
  );

  /** The agent sessions matching the search box — the whole list, not the page. */
  const agentMatches = $derived(filterAgentRows(contextState.agents.rows, agentQuery));
  const agentSearching = $derived(agentQuery.trim().length > 0);

  /** Why the stop button is off, or null when it can be pressed. */
  const stopUnavailable = $derived(processStopUnavailableReason(contextState.processKill));

  /** Trailing tail of a path, so the meaningful end stays visible: `…/b/c`. */
  function tailPath(path: string, segments = 2): string {
    const parts = path.split('/').filter(Boolean);
    if (parts.length <= segments) return path;
    return `…/${parts.slice(-segments).join('/')}`;
  }

  /** Empty-state sentence that names the project when we know it. */
  function emptyLabel(noun: string): string {
    return contextState.projectName ? `No ${noun} in ${contextState.projectName}` : `No ${noun}`;
  }

  function askStopProcess(job: RuntimeContext): void {
    confirmingProcess = job;
    confirmStopOpen = true;
  }

  function stopConfirmedProcess(): void {
    const job = confirmingProcess;
    confirmingProcess = null;
    if (job) void stopProcess(job.pid);
  }

  function repoSyncLabel(repo: { hasUpstream: boolean; ahead: number; behind: number }): string {
    if (!repo.hasUpstream) return 'no remote branch';
    const parts: string[] = [];
    if (repo.ahead) parts.push(`${repo.ahead} to push`);
    if (repo.behind) parts.push(`${repo.behind} to pull`);
    return parts.length > 0 ? parts.join(' · ') : 'in sync';
  }

  function repoSyncTone(repo: {
    hasUpstream: boolean;
    ahead: number;
    behind: number;
  }): 'good' | 'live' | 'attention' | 'muted' {
    if (!repo.hasUpstream) return 'muted';
    if (repo.behind) return 'attention';
    if (repo.ahead) return 'live';
    return 'good';
  }

  function repoChangedCount(repo: {
    stagedCount: number;
    unstagedCount: number;
    untrackedCount: number;
  }): number {
    return repo.stagedCount + repo.unstagedCount + repo.untrackedCount;
  }
</script>

<div class="context-panel" aria-label="Context">
  <header class="toolbar">
    <span class="toolbar-title">
      {contextState.projectName ? contextState.projectName : 'Context'}
    </span>
    <button
      type="button"
      class="refresh"
      disabled={anyLoading}
      onclick={() => void refreshAll()}
      title="Read every card again — runs, processes, Playwright, sessions, worktrees, repositories"
    >
      {anyLoading ? 'reading…' : 'refresh'}
    </button>
  </header>

  {#if !contextState.activated}
    <p class="state">Nothing loaded yet — press refresh to look around this project.</p>
  {:else}
    <!-- ── Recorded agent runs ──────────────────────────────────────────── -->
    <section class="card">
      <button
        type="button"
        class="card-head"
        aria-expanded={expanded.runs}
        onclick={() => toggle('runs')}
      >
        <span class="chevron" class:open={expanded.runs} aria-hidden="true">
          <ChevronRight size={13} />
        </span>
        <span class="card-icon" aria-hidden="true"><Activity size={13} /></span>
        <span class="card-title">Recorded agent runs</span>
        <span class="card-count">{contextState.runs.rows.length}</span>
      </button>

      {#if expanded.runs}
        <div class="card-body">
          <p class="explainer">
            A run is a piece of work an agent reported step by step as it went — which stage it
            reached, how far along it is, what it produced. Nothing shows up here unless an agent
            records one, so an empty list is the normal answer.
          </p>

          {#if contextState.runs.error}
            <p class="state error">
              {contextState.runs.error}
              <button type="button" class="retry" onclick={() => void refreshCard('runs')}>
                try again
              </button>
            </p>
          {:else if contextState.runs.unavailableReason}
            <p class="state">{contextState.runs.unavailableReason}</p>
          {:else if contextState.runs.rows.length > 0}
            {@const summary = summarizeRuns(contextState.runs.rows)}
            {#if summary}<p class="summary">{summary}</p>{/if}
            <ul class="rows">
              {#each contextState.runs.rows.slice(0, shown.runs) as run (run.id)}
                <li class="row">
                  <div class="row-main">
                    {#if runStatusGroup(run.status) === 'running'}
                      <span class="pulse" aria-hidden="true"></span>
                    {/if}
                    <span class="row-title" title={run.title}>{run.title}</span>
                    <span class="chip" data-tone={runStatusGroup(run.status)}>{run.status}</span>
                  </div>
                  {#if run.phase || typeof run.progress === 'number' || run.taskID}
                    <div class="row-meta">
                      {#if run.phase}<span class="meta-strong">{run.phase}</span>{/if}
                      {#if typeof run.progress === 'number'}
                        <span class="num">{run.progress}%</span>
                      {/if}
                      {#if run.taskID}<span class="mono meta-id">{run.taskID}</span>{/if}
                    </div>
                  {/if}
                </li>
              {/each}
            </ul>
            {#if contextState.runs.rows.length > shown.runs}
              <button type="button" class="more" onclick={() => showMore('runs')}>
                Show {Math.min(PAGE_SIZE, contextState.runs.rows.length - shown.runs)} more of
                {contextState.runs.rows.length - shown.runs}
              </button>
            {:else if shown.runs > ROW_LIMIT}
              <button type="button" class="more" onclick={() => showFewer('runs')}>
                Show fewer
              </button>
            {/if}
          {:else if contextState.runs.loading}
            <p class="state loading">Reading recorded agent runs…</p>
          {:else}
            <p class="state">{emptyLabel('recorded agent runs')}</p>
          {/if}
        </div>
      {/if}
    </section>

    <!-- ── Running processes ────────────────────────────────────────────── -->
    <section class="card">
      <button
        type="button"
        class="card-head"
        aria-expanded={expanded.runtime}
        onclick={() => toggle('runtime')}
      >
        <span class="chevron" class:open={expanded.runtime} aria-hidden="true">
          <ChevronRight size={13} />
        </span>
        <span class="card-icon" aria-hidden="true"><Network size={13} /></span>
        <span class="card-title">Running processes</span>
        <span class="card-count">{contextState.runtime.rows.length}</span>
      </button>

      {#if expanded.runtime}
        <div class="card-body">
          {#if contextState.runtime.error}
            <p class="state error">
              {contextState.runtime.error}
              <button type="button" class="retry" onclick={() => void refreshCard('runtime')}>
                try again
              </button>
            </p>
          {:else if contextState.runtime.unavailableReason}
            <p class="state">{contextState.runtime.unavailableReason}</p>
          {:else if contextState.runtime.rows.length > 0}
            {@const summary = summarizeRuntime(contextState.runtime.rows)}
            {#if summary}<p class="summary">{summary}</p>{/if}
            <ul class="rows">
              {#each contextState.runtime.rows.slice(0, shown.runtime) as job (`${job.pid}:${job.port}`)}
                <li class="row">
                  <div class="row-main">
                    <span class="badge">:{job.port}</span>
                    <span class="row-title" title={job.command}>{job.command}</span>
                    <span class="mono meta-id">pid {job.pid}</span>
                  </div>
                  <div class="row-meta">
                    {#if job.rootLabel}<span class="meta-strong">{job.rootLabel}</span>{/if}
                    {#if job.cwd}
                      <span class="mono truncate" title={job.cwd}>{tailPath(job.cwd)}</span>
                    {/if}
                    <button
                      type="button"
                      class="stop"
                      disabled={stopUnavailable !== null || contextState.stoppingPid !== null}
                      title={stopUnavailable ??
                        `Ask process ${job.pid} on port ${job.port} to shut down`}
                      onclick={() => askStopProcess(job)}
                    >
                      {contextState.stoppingPid === job.pid ? 'stopping…' : 'Stop'}
                    </button>
                  </div>
                </li>
              {/each}
            </ul>
            {#if contextState.runtime.rows.length > shown.runtime}
              <button type="button" class="more" onclick={() => showMore('runtime')}>
                Show {Math.min(PAGE_SIZE, contextState.runtime.rows.length - shown.runtime)} more of
                {contextState.runtime.rows.length - shown.runtime}
              </button>
            {:else if shown.runtime > ROW_LIMIT}
              <button type="button" class="more" onclick={() => showFewer('runtime')}>
                Show fewer
              </button>
            {/if}
            {#if contextState.lastProcessMessage}
              <p class="result">{contextState.lastProcessMessage}</p>
            {/if}
            {#if stopUnavailable && contextState.processKill === 'unavailable'}
              <p class="note">
                This build of the app cannot stop a process yet — restart the desktop app after
                updating.
              </p>
            {/if}
          {:else if contextState.runtime.loading}
            <p class="state loading">Looking for running processes…</p>
          {:else}
            <p class="state">{emptyLabel('running processes')}</p>
          {/if}
        </div>
      {/if}
    </section>

    <!-- ── Playwright's leftover browsers ───────────────────────────────── -->
    <!-- Its own card, right after the running processes: same family of thing,
         and it must be visible without scrolling to the bottom. It brings its
         own card chrome and its own "nothing read yet" line. -->
    <PlaywrightCard />

    <!-- ── Agent sessions ───────────────────────────────────────────────── -->
    <section class="card">
      <button
        type="button"
        class="card-head"
        aria-expanded={expanded.agents}
        onclick={() => toggle('agents')}
      >
        <span class="chevron" class:open={expanded.agents} aria-hidden="true">
          <ChevronRight size={13} />
        </span>
        <span class="card-icon" aria-hidden="true"><Braces size={13} /></span>
        <span class="card-title">Agent sessions</span>
        <span class="card-count">{contextState.agents.rows.length}</span>
      </button>

      {#if expanded.agents}
        <div class="card-body">
          {#if contextState.agents.error}
            <p class="state error">
              {contextState.agents.error}
              <button type="button" class="retry" onclick={() => void refreshCard('agents')}>
                try again
              </button>
            </p>
          {:else if contextState.agents.unavailableReason}
            <p class="state">{contextState.agents.unavailableReason}</p>
          {:else if contextState.agents.rows.length > 0}
            {@const summary = summarizeAgents(contextState.agents.rows)}
            {#if summary}<p class="summary">{summary}</p>{/if}

            <!-- Searches every session on the machine, not just the rows on
                 screen — which is the only reason a box over six rows is worth
                 having. -->
            <label class="search">
              <span class="search-icon" aria-hidden="true"><Search size={12} /></span>
              <input
                type="search"
                bind:value={agentQuery}
                placeholder="Search all {contextState.agents.rows.length} sessions"
                aria-label="Search all agent sessions"
              />
            </label>

            {#if agentSearching}
              <p class="summary">
                {agentMatches.length}
                {agentMatches.length === 1 ? 'session matches' : 'sessions match'} “{agentQuery.trim()}”
              </p>
            {/if}

            <ul class="rows">
              {#each agentMatches.slice(0, shown.agents) as agent (`${agent.provider}:${agent.id}`)}
                <li class="row">
                  <div class="row-main">
                    <span class="row-title" title={agent.title}>{agent.title || agent.id}</span>
                    <span class="chip" data-tone="neutral">{agent.provider}</span>
                  </div>
                  {#if agent.projectPath || agent.lastActivity}
                    <div class="row-meta">
                      {#if agent.projectPath}
                        <span class="meta-strong" title={agent.projectPath}>
                          {folderName(agent.projectPath)}
                        </span>
                      {/if}
                      {#if agent.lastActivity}<span class="stamp">{agent.lastActivity}</span>{/if}
                    </div>
                  {/if}
                </li>
              {:else}
                <li class="row">
                  <span class="state">Nothing matches “{agentQuery.trim()}”.</span>
                </li>
              {/each}
            </ul>

            {#if agentMatches.length > shown.agents}
              <button type="button" class="more" onclick={() => showMore('agents')}>
                Show {Math.min(PAGE_SIZE, agentMatches.length - shown.agents)} more of
                {agentMatches.length - shown.agents}
              </button>
            {:else if shown.agents > ROW_LIMIT}
              <button type="button" class="more" onclick={() => showFewer('agents')}>
                Show fewer
              </button>
            {/if}

            {#if contextPanelHooks.openSessionFinder}
              <button
                type="button"
                class="link"
                onclick={() => contextPanelHooks.openSessionFinder?.()}
              >
                Open “Find a session” to take one over
              </button>
            {/if}
          {:else if contextState.agents.loading}
            <p class="state loading">Looking for agent sessions…</p>
          {:else}
            <p class="state">No agent sessions found</p>
          {/if}
        </div>
      {/if}
    </section>

    <!-- ── Worktrees ────────────────────────────────────────────────────── -->
    <section class="card">
      <button
        type="button"
        class="card-head"
        aria-expanded={expanded.worktrees}
        onclick={() => toggle('worktrees')}
      >
        <span class="chevron" class:open={expanded.worktrees} aria-hidden="true">
          <ChevronRight size={13} />
        </span>
        <span class="card-icon" aria-hidden="true"><FolderGit2 size={13} /></span>
        <span class="card-title">Worktrees</span>
        <span class="card-count">{contextState.worktrees.rows.length}</span>
      </button>

      {#if expanded.worktrees}
        <div class="card-body">
          {#if contextPanelHooks.showWorktreesView}
            <button
              type="button"
              class="link"
              onclick={() => contextPanelHooks.showWorktreesView?.()}
            >
              Open the Worktrees view to remove or back one up
            </button>
          {/if}

          {#if contextState.worktrees.error}
            <p class="state error">
              {contextState.worktrees.error}
              <button type="button" class="retry" onclick={() => void refreshCard('worktrees')}>
                try again
              </button>
            </p>
          {:else if contextState.worktrees.unavailableReason}
            <p class="state">{contextState.worktrees.unavailableReason}</p>
          {:else if contextState.worktrees.rows.length > 0}
            {@const summary = summarizeWorktrees(contextState.worktrees.rows)}
            {#if summary}<p class="summary">{summary}</p>{/if}
            <ul class="rows">
              {#each contextState.worktrees.rows.slice(0, shown.worktrees) as worktree (worktree.path)}
                <li class="row">
                  <div class="row-main">
                    <span class="row-title" title={worktree.branch}>{worktree.branch}</span>
                    {#if worktree.isDirty}
                      <span class="chip" data-tone="attention">uncommitted changes</span>
                    {:else}
                      <span class="chip" data-tone="good">clean</span>
                    {/if}
                  </div>
                  <div class="row-meta">
                    {#if worktree.repo}<span class="meta-strong">{worktree.repo}</span>{/if}
                    {#if worktree.taskID}<span class="mono meta-id">{worktree.taskID}</span>{/if}
                    <span class="mono truncate" title={worktree.path}>
                      {tailPath(worktree.path)}
                    </span>
                  </div>
                </li>
              {/each}
            </ul>
            {#if contextState.worktrees.rows.length > shown.worktrees}
              {#if contextPanelHooks.showWorktreesView}
                <!-- The Worktrees view is the place with the safety checks and
                     the remove buttons, so the rest of the list opens there
                     rather than growing a second, weaker copy here. -->
                <button
                  type="button"
                  class="more"
                  onclick={() => contextPanelHooks.showWorktreesView?.()}
                >
                  See all {contextState.worktrees.rows.length} in the Worktrees view
                </button>
              {:else}
                <button type="button" class="more" onclick={() => showMore('worktrees')}>
                  Show {Math.min(
                    PAGE_SIZE,
                    contextState.worktrees.rows.length - shown.worktrees
                  )} more of {contextState.worktrees.rows.length - shown.worktrees}
                </button>
              {/if}
            {:else if shown.worktrees > ROW_LIMIT}
              <button type="button" class="more" onclick={() => showFewer('worktrees')}>
                Show fewer
              </button>
            {/if}
          {:else if contextState.worktrees.loading}
            <p class="state loading">Looking for worktrees…</p>
          {:else}
            <p class="state">{emptyLabel('worktrees')}</p>
          {/if}
        </div>
      {/if}
    </section>

    <!-- ── Repositories ─────────────────────────────────────────────────── -->
    <section class="card">
      <button
        type="button"
        class="card-head"
        aria-expanded={expanded.repositories}
        onclick={() => toggle('repositories')}
      >
        <span class="chevron" class:open={expanded.repositories} aria-hidden="true">
          <ChevronRight size={13} />
        </span>
        <span class="card-icon" aria-hidden="true"><GitBranch size={13} /></span>
        <span class="card-title">Repositories</span>
        <span class="card-count">{contextState.repositories.rows.length}</span>
      </button>

      {#if expanded.repositories}
        <div class="card-body">
          {#if contextState.repositories.error}
            <p class="state error">
              {contextState.repositories.error}
              <button type="button" class="retry" onclick={() => void refreshCard('repositories')}>
                try again
              </button>
            </p>
          {:else if contextState.repositories.unavailableReason}
            <p class="state">{contextState.repositories.unavailableReason}</p>
          {:else if contextState.repositories.rows.length > 0}
            {@const summary = summarizeRepositories(contextState.repositories.rows)}
            {#if summary}<p class="summary">{summary}</p>{/if}
            <ul class="rows">
              {#each contextState.repositories.rows.slice(0, shown.repositories) as repo (repo.path)}
                <li class="row">
                  <div class="row-main">
                    <span class="row-title" title={repo.path}>
                      {repo.rootLabel || folderName(repo.path)}
                    </span>
                    {#if repo.error}
                      <span class="chip" data-tone="failed">could not read</span>
                    {:else if repo.isDirty}
                      <span class="chip" data-tone="attention">
                        {repoChangedCount(repo)} changed
                      </span>
                    {:else}
                      <span class="chip" data-tone="good">clean</span>
                    {/if}
                  </div>
                  <div class="row-meta">
                    {#if repo.error}
                      <span class="meta-error" title={repo.error}>{repo.error}</span>
                    {:else}
                      <span class="meta-strong">{repo.branch}</span>
                      <span class="sync" data-tone={repoSyncTone(repo)}>{repoSyncLabel(repo)}</span>
                    {/if}
                  </div>
                </li>
              {/each}
            </ul>
            {#if contextState.repositories.rows.length > shown.repositories}
              <button type="button" class="more" onclick={() => showMore('repositories')}>
                Show {Math.min(
                  PAGE_SIZE,
                  contextState.repositories.rows.length - shown.repositories
                )} more of {contextState.repositories.rows.length - shown.repositories}
              </button>
            {:else if shown.repositories > ROW_LIMIT}
              <button type="button" class="more" onclick={() => showFewer('repositories')}>
                Show fewer
              </button>
            {/if}
          {:else if contextState.repositories.loading}
            <p class="state loading">Reading repositories…</p>
          {:else}
            <p class="state">{emptyLabel('repositories')}</p>
          {/if}
        </div>
      {/if}
    </section>
  {/if}
</div>

<!-- Stopping a process is not undoable and the row is one line of text, so the
     question names the command, the port, the process id and the folder — enough
     to recognise it without going and looking it up somewhere else. -->
<AlertDialog.Root bind:open={confirmStopOpen}>
  <AlertDialog.Content
    class="rounded-lg bg-background text-foreground ring-[var(--color-border)]
           shadow-[var(--shadow-lg)]"
  >
    <AlertDialog.Header>
      <AlertDialog.Title class="text-[14px] leading-[1.4] font-semibold">
        Stop this process?
      </AlertDialog.Title>
      <AlertDialog.Description class="text-[13px] leading-[1.5] text-[var(--color-text-2)]">
        {#if confirmingProcess}
          <span class="font-medium">{confirmingProcess.command}</span> is holding port
          {confirmingProcess.port} as process {confirmingProcess.pid}{confirmingProcess.cwd
            ? `, started in ${confirmingProcess.cwd}`
            : ''}. It is asked to shut down cleanly, so whatever it was serving — a dev server, a
          test run, a database — stops, and anything it had not written out is lost. Nothing else on
          your machine is touched.
        {/if}
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer class="bg-transparent">
      <AlertDialog.Cancel size="sm" class="text-[13px]">Leave it running</AlertDialog.Cancel>
      <AlertDialog.Action
        size="sm"
        variant="destructive"
        class="text-[13px]"
        onclick={stopConfirmedProcess}
      >
        Stop it
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>

<style>
  .context-panel {
    display: flex;
    flex-direction: column;
    gap: 2px;
    height: 100%;
    width: 100%;
    min-width: 0;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding: 8px 8px 16px;
    background: var(--color-bg);
    color: var(--color-text);
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
    font-size: 12px;
    scrollbar-width: thin;
  }

  /* ── Toolbar ───────────────────────────────────────────────────────── */
  .toolbar {
    position: sticky;
    top: -8px;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin: -8px -8px 4px;
    padding: 8px;
    background: var(--color-bg);
  }

  .toolbar-title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: var(--color-section-header-text);
  }

  .refresh {
    flex: 0 0 auto;
    border: 1px solid var(--color-border);
    border-radius: 5px;
    background: transparent;
    color: var(--color-text-2);
    font: inherit;
    font-size: 12px;
    padding: 2px 7px;
    cursor: pointer;
  }

  .refresh:hover:not(:disabled) {
    border-color: var(--color-accent);
    color: var(--color-text);
  }

  .refresh:disabled {
    opacity: 0.5;
    cursor: default;
  }

  /* ── Card ──────────────────────────────────────────────────────────── */
  .card {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--color-border);
  }

  .card:first-of-type {
    border-top: 0;
  }

  .card-head {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 7px 6px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--color-text-2);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .card-head:hover {
    background: var(--color-surface);
    color: var(--color-text);
  }

  .chevron {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    color: var(--color-text-2);
    transition: transform 130ms ease;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .card-icon {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    color: var(--color-text-2);
  }

  .card-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .card-count {
    flex: 0 0 auto;
    border-radius: 999px;
    background: var(--color-elevated);
    color: var(--color-text-2);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    line-height: 1;
    padding: 3px 6px;
  }

  .card-body {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 0 4px 8px 22px;
  }

  .explainer,
  .summary,
  .note,
  .result {
    margin: 0;
    padding: 0 6px;
    color: var(--color-text-2);
    font-size: 12px;
    line-height: 1.45;
  }

  .note {
    color: var(--color-attention);
  }

  .result {
    padding-top: 4px;
  }

  /* ── The agent-session search box ──────────────────────────────────── */
  .search {
    display: flex;
    align-items: center;
    gap: 5px;
    margin: 2px 6px 4px;
    padding: 3px 7px;
    border: 1px solid var(--color-border);
    border-radius: 5px;
    background: var(--color-surface);
  }

  .search:focus-within {
    border-color: var(--color-accent);
  }

  .search-icon {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    color: var(--color-text-2);
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: 12px;
    line-height: 1.4;
    outline: none;
  }

  .search input::placeholder {
    color: var(--color-text-2);
  }

  /* ── Rows ──────────────────────────────────────────────────────────── */
  .rows {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 6px;
    border-radius: 6px;
    transition: background-color 120ms ease;
  }

  .row:hover {
    background: var(--color-surface);
  }

  .row + .row {
    position: relative;
  }

  .row + .row::before {
    content: '';
    position: absolute;
    top: 0;
    left: 6px;
    right: 6px;
    height: 1px;
    background: var(--color-border);
  }

  .row:hover::before,
  .row:hover + .row::before {
    opacity: 0;
  }

  .row-main {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .row-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text);
    font-size: 13px;
    line-height: 1.35;
  }

  .row-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 8px;
    min-width: 0;
    color: var(--color-text-2);
    font-size: 12px;
    line-height: 1.3;
    font-variant-numeric: tabular-nums;
  }

  .meta-strong {
    color: var(--color-text-2);
  }

  .num {
    color: var(--color-text-2);
    font-variant-numeric: tabular-nums;
  }

  .meta-id,
  .stamp {
    flex-shrink: 0;
    color: var(--color-text-2);
  }

  .mono {
    font-family: ui-monospace, Menlo, monospace;
  }

  .truncate {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta-error {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--color-bad);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ── Chips and badges ──────────────────────────────────────────────── */
  .chip {
    flex: 0 0 auto;
    max-width: 45%;
    overflow: hidden;
    border-radius: 4px;
    background: var(--color-elevated);
    color: var(--color-text-2);
    font-size: 12px;
    letter-spacing: 0.04em;
    padding: 1px 5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip[data-tone='running'] {
    background: var(--color-live-bg);
    color: var(--color-live);
  }

  .chip[data-tone='good'] {
    background: var(--color-good-bg);
    color: var(--color-good);
  }

  .chip[data-tone='attention'] {
    background: var(--color-attention-bg);
    color: var(--color-attention);
  }

  .chip[data-tone='failed'] {
    background: var(--color-bad-bg);
    color: var(--color-bad);
  }

  .chip[data-tone='finished'] {
    background: var(--color-good-bg);
    color: var(--color-good);
  }

  .badge {
    flex: 0 0 auto;
    border-radius: 4px;
    background: var(--color-live-bg);
    color: var(--color-live);
    font-family: ui-monospace, Menlo, monospace;
    font-size: 12px;
    padding: 1px 5px;
    white-space: nowrap;
  }

  .sync {
    letter-spacing: 0.01em;
  }

  .sync[data-tone='good'] {
    color: var(--color-good);
  }

  .sync[data-tone='live'] {
    color: var(--color-live);
  }

  .sync[data-tone='attention'] {
    color: var(--color-attention);
  }

  .sync[data-tone='muted'] {
    color: var(--color-text-2);
  }

  /* ── Live pulse ────────────────────────────────────────────────────── */
  .pulse {
    flex: 0 0 auto;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-live);
    animation: context-pulse 1.9s ease-in-out infinite;
  }

  @keyframes context-pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.35;
    }
  }

  /* ── Buttons ───────────────────────────────────────────────────────── */
  /* "+N more" is a button now, not a sentence: every capped list has somewhere
     to go. It keeps the quiet look the old dead line had so the panel does not
     turn into a wall of links. */
  .more,
  .link {
    align-self: flex-start;
    margin: 0;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--color-text-2);
    font: inherit;
    font-size: 12px;
    padding: 4px 6px 0;
    cursor: pointer;
    text-align: left;
    text-decoration: underline;
  }

  .more:hover,
  .link:hover {
    color: var(--color-text);
  }

  .stop {
    flex: 0 0 auto;
    margin-left: auto;
    border: 1px solid var(--color-border);
    border-radius: 5px;
    background: transparent;
    color: var(--color-text-2);
    font: inherit;
    font-size: 12px;
    line-height: 1.3;
    padding: 1px 8px;
    cursor: pointer;
  }

  .stop:hover:not(:disabled) {
    border-color: var(--color-bad);
    color: var(--color-bad);
  }

  .stop:disabled {
    opacity: 0.5;
    cursor: default;
  }

  /* ── States ────────────────────────────────────────────────────────── */
  .state {
    margin: 0;
    padding: 8px 6px;
    color: var(--color-text-2);
    font-size: 12px;
    line-height: 1.45;
  }

  .state.error {
    color: var(--color-bad);
  }

  .state.loading {
    animation: context-fade 1.4s ease-in-out infinite;
  }

  @keyframes context-fade {
    0%,
    100% {
      opacity: 0.5;
    }
    50% {
      opacity: 0.9;
    }
  }

  .retry {
    margin-left: 6px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--color-accent);
    font: inherit;
    font-size: 12px;
    padding: 0 2px;
    cursor: pointer;
    text-decoration: underline;
  }

  button:focus-visible {
    outline: 1px solid var(--color-accent);
    outline-offset: -1px;
  }

  @media (prefers-reduced-motion: reduce) {
    .pulse,
    .state.loading {
      animation: none;
    }
    .chevron {
      transition: none;
    }
  }
</style>
