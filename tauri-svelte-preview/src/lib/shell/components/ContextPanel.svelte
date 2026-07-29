<script lang="ts">
  /**
   * ContextPanel.svelte — the /next context region: five cards showing what is
   * going on around the active project (runs, running processes, agent
   * sessions, worktrees, repositories).
   *
   * Quarried from `src/lib/WorkbenchContextPanel.svelte` (row shapes, the
   * six-row cap with a "+N more" line, the tone rules) and restyled to the
   * /next palette. Two differences from the old panel:
   *
   *  - it reads `contextStore` directly instead of taking twenty props, and
   *  - the collapsible sections, chips and badges are inlined here, so the
   *    panel carries no dependency on the old shell's component library or its
   *    (different) colour tokens.
   *
   * NO props, NO IO at mount, NO `$effect`. It renders an inert "nothing loaded
   * yet" line until the shell calls `activate()` on `contextService`; after that
   * the only thing that starts a backend call is the Refresh button.
   *
   * Worktree rows are read-only in this slice: removing or archiving a worktree
   * runs a separate safety check that is not part of this panel yet.
   */
  import { Activity, Braces, ChevronRight, FolderGit2, GitBranch, Network } from '@lucide/svelte';

  import PlaywrightCard from './processes/PlaywrightCard.svelte';
  import { refreshAll, refreshCard } from '$lib/shell/context/contextService';
  import {
    contextState,
    folderName,
    runStatusGroup,
    summarizeAgents,
    summarizeRepositories,
    summarizeRuns,
    summarizeRuntime,
    summarizeWorktrees,
    type ContextCardKey
  } from '$lib/shell/context/contextStore.svelte';

  /** How many rows a card shows before it defers to a "+N more" line. */
  const ROW_LIMIT = 6;

  /** Which sections are open. All five start open, like the old panel. */
  let expanded = $state<Record<ContextCardKey, boolean>>({
    runs: true,
    runtime: true,
    agents: true,
    worktrees: true,
    repositories: true
  });

  function toggle(key: ContextCardKey): void {
    expanded[key] = !expanded[key];
  }

  const anyLoading = $derived(
    contextState.runs.loading ||
      contextState.runtime.loading ||
      contextState.agents.loading ||
      contextState.worktrees.loading ||
      contextState.repositories.loading
  );

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
      title="Read runs, processes, agent sessions, worktrees and repositories again"
    >
      {anyLoading ? 'reading…' : 'refresh'}
    </button>
  </header>

  {#if !contextState.activated}
    <p class="state">Nothing loaded yet — press refresh to look around this project.</p>
  {:else}
    <!-- ── Runs ─────────────────────────────────────────────────────────── -->
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
        <span class="card-title">Runs</span>
        <span class="card-count">{contextState.runs.rows.length}</span>
      </button>

      {#if expanded.runs}
        <div class="card-body">
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
              {#each contextState.runs.rows.slice(0, ROW_LIMIT) as run (run.id)}
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
            {#if contextState.runs.rows.length > ROW_LIMIT}
              <p class="more">+{contextState.runs.rows.length - ROW_LIMIT} more</p>
            {/if}
          {:else if contextState.runs.loading}
            <p class="state loading">Reading runs…</p>
          {:else}
            <p class="state">{emptyLabel('runs')}</p>
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
              {#each contextState.runtime.rows.slice(0, ROW_LIMIT) as job (`${job.pid}:${job.port}`)}
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
                  </div>
                </li>
              {/each}
            </ul>
            {#if contextState.runtime.rows.length > ROW_LIMIT}
              <p class="more">+{contextState.runtime.rows.length - ROW_LIMIT} more</p>
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
            <ul class="rows">
              {#each contextState.agents.rows.slice(0, ROW_LIMIT) as agent (`${agent.provider}:${agent.id}`)}
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
              {/each}
            </ul>
            {#if contextState.agents.rows.length > ROW_LIMIT}
              <p class="more">+{contextState.agents.rows.length - ROW_LIMIT} more</p>
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
              {#each contextState.worktrees.rows.slice(0, ROW_LIMIT) as worktree (worktree.path)}
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
            {#if contextState.worktrees.rows.length > ROW_LIMIT}
              <p class="more">+{contextState.worktrees.rows.length - ROW_LIMIT} more</p>
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
              {#each contextState.repositories.rows.slice(0, ROW_LIMIT) as repo (repo.path)}
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
            {#if contextState.repositories.rows.length > ROW_LIMIT}
              <p class="more">+{contextState.repositories.rows.length - ROW_LIMIT} more</p>
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
    background: #101014;
    color: #d8d8e0;
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
    background: #101014;
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
    color: #7b7b8c;
  }

  .refresh {
    flex: 0 0 auto;
    border: 1px solid #2a2a34;
    border-radius: 5px;
    background: transparent;
    color: #9a9aad;
    font: inherit;
    font-size: 12px;
    padding: 2px 7px;
    cursor: pointer;
  }

  .refresh:hover:not(:disabled) {
    border-color: #3d3d4a;
    color: #d8d8e0;
  }

  .refresh:disabled {
    opacity: 0.5;
    cursor: default;
  }

  /* ── Card ──────────────────────────────────────────────────────────── */
  .card {
    display: flex;
    flex-direction: column;
    border-top: 1px solid #22222c;
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
    color: #9a9aad;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .card-head:hover {
    background: #17171d;
    color: #d8d8e0;
  }

  .chevron {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    color: #6d6d7d;
    transition: transform 130ms ease;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .card-icon {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    color: #6d6d7d;
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
    background: #1c1c24;
    color: #6d6d7d;
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

  .summary {
    margin: 0;
    padding: 0 6px;
    color: #8a8a9c;
    font-size: 12px;
    line-height: 1.45;
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
    background: #17171d;
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
    background: #22222c;
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
    color: #e6e6ee;
    font-size: 12px;
    line-height: 1.35;
  }

  .row-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 8px;
    min-width: 0;
    color: #6d6d7d;
    font-size: 12px;
    line-height: 1.3;
    font-variant-numeric: tabular-nums;
  }

  .meta-strong {
    color: #8a8a9c;
  }

  .num {
    color: #8a8a9c;
    font-variant-numeric: tabular-nums;
  }

  .meta-id,
  .stamp {
    flex-shrink: 0;
    color: #5d5d6b;
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
    color: #ff9d9d;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ── Chips and badges ──────────────────────────────────────────────── */
  .chip {
    flex: 0 0 auto;
    max-width: 45%;
    overflow: hidden;
    border-radius: 4px;
    background: #24242f;
    color: #9a9aad;
    font-size: 12px;
    letter-spacing: 0.04em;
    padding: 1px 5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip[data-tone='running'] {
    background: rgba(80, 250, 123, 0.12);
    color: #50fa7b;
  }

  .chip[data-tone='good'] {
    background: rgba(80, 250, 123, 0.1);
    color: #7ee39a;
  }

  .chip[data-tone='attention'] {
    background: rgba(241, 250, 140, 0.12);
    color: #f1fa8c;
  }

  .chip[data-tone='failed'] {
    background: rgba(255, 85, 85, 0.14);
    color: #ff8888;
  }

  .chip[data-tone='finished'] {
    background: rgba(139, 233, 253, 0.1);
    color: #8be9fd;
  }

  .badge {
    flex: 0 0 auto;
    border-radius: 4px;
    background: rgba(139, 233, 253, 0.1);
    color: #8be9fd;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 12px;
    padding: 1px 5px;
    white-space: nowrap;
  }

  .sync {
    letter-spacing: 0.01em;
  }

  .sync[data-tone='good'] {
    color: #7ee39a;
  }

  .sync[data-tone='live'] {
    color: #8be9fd;
  }

  .sync[data-tone='attention'] {
    color: #f1fa8c;
  }

  .sync[data-tone='muted'] {
    color: #5d5d6b;
  }

  /* ── Live pulse ────────────────────────────────────────────────────── */
  .pulse {
    flex: 0 0 auto;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #50fa7b;
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

  /* ── States ────────────────────────────────────────────────────────── */
  .more {
    margin: 0;
    padding: 4px 6px 0;
    color: #5d5d6b;
    font-size: 12px;
  }

  .state {
    margin: 0;
    padding: 8px 6px;
    color: #6d6d7d;
    font-size: 12px;
    line-height: 1.45;
  }

  .state.error {
    color: #ff9d9d;
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
    color: #bd93f9;
    font: inherit;
    font-size: 12px;
    padding: 0 2px;
    cursor: pointer;
    text-decoration: underline;
  }

  button:focus-visible {
    outline: 1px solid #bd93f9;
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
