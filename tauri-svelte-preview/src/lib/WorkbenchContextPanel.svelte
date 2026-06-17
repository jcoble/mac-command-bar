<script lang="ts">
  import { Activity, Network, Braces, FolderGit2, GitBranch } from '@lucide/svelte';
  import CollapsibleSection from '$lib/components/CollapsibleSection.svelte';
  import Chip from '$lib/components/Chip.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import type { Tone } from '$lib/components/tone.js';

  // Minimal structural shapes — only the fields this panel renders. The page passes its existing
  // derived values (which are structurally compatible supersets of these).
  type RunRow = {
    id: string;
    title: string;
    status: string;
    phase?: string;
    progress?: number;
    taskID?: string | null;
  };
  type RuntimeRow = {
    pid: number | string;
    port: number | string;
    command?: string;
    rootLabel?: string;
    cwd?: string;
  };
  type AgentRow = {
    title: string;
    provider?: string;
  };
  type WorktreeRow = {
    path: string;
    branch: string;
    repo?: string;
    taskID?: string | null;
    isDirty?: boolean;
  };
  type RepoRow = {
    projectID?: string;
    path: string;
    isDirty?: boolean;
    error?: string | null;
    unstagedCount?: number;
    stagedCount?: number;
    untrackedCount?: number;
    ahead?: number;
    behind?: number;
    hasUpstream?: boolean;
  };

  type Props = {
    runs?: RunRow[];
    runsSummary?: string;
    runsLoading?: boolean;

    runtimeContexts?: RuntimeRow[];
    runtimeSummary?: string;
    runtimeLoading?: boolean;

    agents?: AgentRow[];
    agentsSummary?: string;
    agentsLoading?: boolean;

    worktrees?: WorktreeRow[];
    worktreesSummary?: string;
    worktreesSafetyStats?: string;
    worktreesLoading?: boolean;

    repos?: RepoRow[];
    reposSummary?: string;
    reposLoading?: boolean;

    /** Optional label for the active project, used in empty states. */
    projectName?: string;
  };

  let {
    runs = [],
    runsSummary = '',
    runsLoading = false,
    runtimeContexts = [],
    runtimeSummary = '',
    runtimeLoading = false,
    agents = [],
    agentsSummary = '',
    agentsLoading = false,
    worktrees = [],
    worktreesSummary = '',
    worktreesSafetyStats = '',
    worktreesLoading = false,
    repos = [],
    reposSummary = '',
    reposLoading = false,
    projectName = ''
  }: Props = $props();

  function runStatusTone(status: string): Tone {
    const value = status.toLowerCase();
    if (value.includes('fail') || value.includes('error') || value.includes('block')) return 'bad';
    if (value.includes('wait') || value.includes('review') || value.includes('attention') || value.includes('approval'))
      return 'attention';
    if (value.includes('run') || value.includes('active') || value.includes('progress') || value.includes('live'))
      return 'live';
    if (value.includes('done') || value.includes('complete') || value.includes('success') || value.includes('merged'))
      return 'good';
    return 'neutral';
  }

  function repoSyncLabel(repo: RepoRow): string {
    if (!repo.hasUpstream) return 'no upstream';
    const parts: string[] = [];
    if (repo.ahead) parts.push(`↑${repo.ahead}`);
    if (repo.behind) parts.push(`↓${repo.behind}`);
    return parts.length > 0 ? parts.join(' ') : 'in sync';
  }

  function repoDirtyCount(repo: RepoRow): number {
    return (repo.unstagedCount ?? 0) + (repo.stagedCount ?? 0) + (repo.untrackedCount ?? 0);
  }

  function repoName(repo: RepoRow): string {
    const segments = repo.path.split('/').filter(Boolean);
    return segments[segments.length - 1] ?? repo.path;
  }
</script>

<div class="workbench-context-panel" aria-label="Context">
  <CollapsibleSection title="Runs" icon={Activity} badge={runs.length} expanded={true}>
    <div class="wcp-section-body">
      {#if runsSummary}<p class="wcp-summary">{runsSummary}</p>{/if}
      {#if runs.length > 0}
        <ul class="wcp-list">
          {#each runs.slice(0, 6) as run (run.id)}
            <li class="wcp-row">
              <div class="wcp-row-main">
                <Chip tone={runStatusTone(run.status)} size="xs">{run.status}</Chip>
                <span class="wcp-row-title" title={run.title}>{run.title}</span>
              </div>
              <div class="wcp-row-meta">
                {#if run.phase}<span>{run.phase}</span>{/if}
                {#if typeof run.progress === 'number'}<span>{run.progress}%</span>{/if}
                {#if run.taskID}<span class="wcp-mono">{run.taskID}</span>{/if}
              </div>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="wcp-empty">{runsLoading ? 'Loading runs…' : 'No orchestration runs'}</p>
      {/if}
    </div>
  </CollapsibleSection>

  <CollapsibleSection title="Runtime" icon={Network} badge={runtimeContexts.length} expanded={true}>
    <div class="wcp-section-body">
      {#if runtimeSummary}<p class="wcp-summary">{runtimeSummary}</p>{/if}
      {#if runtimeContexts.length > 0}
        <ul class="wcp-list">
          {#each runtimeContexts.slice(0, 6) as ctx (`${ctx.pid}:${ctx.port}`)}
            <li class="wcp-row">
              <div class="wcp-row-main">
                <Badge tone="live">:{ctx.port}</Badge>
                <span class="wcp-row-title" title={ctx.command}>{ctx.command ?? 'process'}</span>
              </div>
              <div class="wcp-row-meta">
                {#if ctx.rootLabel}<span>{ctx.rootLabel}</span>{/if}
                {#if ctx.cwd}<small class="wcp-mono" title={ctx.cwd}>{ctx.cwd}</small>{/if}
              </div>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="wcp-empty">{runtimeLoading ? 'Scanning runtime…' : 'No runtime contexts'}</p>
      {/if}
    </div>
  </CollapsibleSection>

  <CollapsibleSection title="Agents" icon={Braces} badge={agents.length} expanded={true}>
    <div class="wcp-section-body">
      {#if agentsSummary}<p class="wcp-summary">{agentsSummary}</p>{/if}
      {#if agents.length > 0}
        <ul class="wcp-list">
          {#each agents.slice(0, 6) as agent, index (`${agent.title}:${index}`)}
            <li class="wcp-row">
              <div class="wcp-row-main">
                {#if agent.provider}<Chip tone="neutral" size="xs">{agent.provider}</Chip>{/if}
                <span class="wcp-row-title" title={agent.title}>{agent.title}</span>
              </div>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="wcp-empty">{agentsLoading ? 'Loading agents…' : 'No agent sessions'}</p>
      {/if}
    </div>
  </CollapsibleSection>

  <CollapsibleSection title="Worktrees" icon={FolderGit2} badge={worktrees.length} expanded={true}>
    <div class="wcp-section-body">
      {#if worktreesSummary}<p class="wcp-summary">{worktreesSummary}</p>{/if}
      {#if worktreesSafetyStats}<p class="wcp-subtle">{worktreesSafetyStats}</p>{/if}
      {#if worktrees.length > 0}
        <ul class="wcp-list">
          {#each worktrees.slice(0, 6) as worktree (worktree.path)}
            <li class="wcp-row">
              <div class="wcp-row-main">
                <span class="wcp-row-title" title={worktree.branch}>{worktree.branch}</span>
                {#if worktree.isDirty}<Chip tone="attention" size="xs">dirty</Chip>{/if}
              </div>
              <div class="wcp-row-meta">
                {#if worktree.repo}<span>{worktree.repo}</span>{/if}
                {#if worktree.taskID}<span class="wcp-mono">{worktree.taskID}</span>{/if}
                <small class="wcp-mono" title={worktree.path}>{worktree.path}</small>
              </div>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="wcp-empty">{worktreesLoading ? 'Scanning worktrees…' : 'No worktrees'}</p>
      {/if}
    </div>
  </CollapsibleSection>

  <CollapsibleSection title="Git" icon={GitBranch} badge={repos.length} expanded={true}>
    <div class="wcp-section-body">
      {#if reposSummary}<p class="wcp-summary">{reposSummary}</p>{/if}
      {#if repos.length > 0}
        <ul class="wcp-list">
          {#each repos.slice(0, 6) as repo (repo.path)}
            <li class="wcp-row">
              <div class="wcp-row-main">
                <span class="wcp-row-title" title={repo.path}>{repoName(repo)}</span>
                {#if repo.error}
                  <Chip tone="bad" size="xs">error</Chip>
                {:else if repo.isDirty}
                  <Chip tone="attention" size="xs">{repoDirtyCount(repo)} changed</Chip>
                {:else}
                  <Chip tone="good" size="xs">clean</Chip>
                {/if}
              </div>
              <div class="wcp-row-meta">
                <span>{repoSyncLabel(repo)}</span>
              </div>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="wcp-empty">{reposLoading ? 'Scanning repositories…' : 'No repository summaries'}</p>
      {/if}
    </div>
  </CollapsibleSection>
</div>

<style>
  .workbench-context-panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-1, 4px);
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding: var(--space-2, 8px);
    background: var(--color-bg, #191a21);
    scrollbar-width: thin;
  }

  .wcp-section-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-2, 8px);
    padding: var(--space-1, 4px) var(--space-3, 12px) var(--space-3, 12px) var(--space-5, 28px);
  }

  .wcp-summary {
    margin: 0;
    font-size: var(--text-sm, 12px);
    color: var(--color-text-2, #aab6b2);
  }

  .wcp-subtle {
    margin: 0;
    font-size: var(--text-xs, 11px);
    color: var(--color-text-3, #7f8b88);
  }

  .wcp-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2, 8px);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .wcp-row {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: var(--space-2, 8px) var(--space-3, 12px);
    border-radius: var(--radius-sm, 6px);
    background: var(--color-surface, rgba(255, 255, 255, 0.045));
  }

  .wcp-row-main {
    display: flex;
    align-items: center;
    gap: var(--space-2, 8px);
    min-width: 0;
  }

  .wcp-row-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: var(--text-sm, 12px);
    font-weight: var(--weight-semibold, 650);
    color: var(--color-text, #dffdf8);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .wcp-row-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2, 8px);
    min-width: 0;
    font-size: var(--text-xs, 11px);
    color: var(--color-text-3, #7f8b88);
  }

  .wcp-row-meta small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .wcp-mono {
    font-family: var(--font-mono, ui-monospace, monospace);
  }

  .wcp-empty {
    margin: 0;
    font-size: var(--text-sm, 12px);
    color: var(--color-text-3, #7f8b88);
  }
</style>
