<script lang="ts">
  /**
   * CommitTimeline.svelte — the commits under the changed files.
   *
   * One row per commit: its subject, who wrote it, and when. Opening a row reads
   * what that commit changed and lists it; picking one of those files shows its
   * changes in the middle of the shell. Hovering the subject shows it in full,
   * because a narrow panel cuts most subjects off and two commits cut off at the
   * same word are impossible to tell apart.
   *
   * WHAT IS SHOWN IS WHAT THE APP RETURNS. The commit list carries a subject, an
   * author, a date and the commit id — not the message body — so the hover card
   * shows those and never pads them out with anything it did not read.
   *
   * No IO of its own: reading a commit's files goes through
   * `gitCommitFilesService`, reading more commits through `gitService`, and both
   * only ever from a click.
   */
  import { Button } from '$lib/components/ui/button/index.js';
  import { Chip } from '$lib/components/ui/chip/index.js';
  import { ListRow } from '$lib/components/ui/list-row/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import type { GitCommitFileChange } from '$lib/shell/git/gitBackendExtra';
  import type { GitCommitFilesService } from '$lib/shell/git/gitCommitFilesService';
  import {
    commitFilesEntry,
    describeCommitFiles,
    isCommitExpanded,
    splitRepositoryPath,
    summarizeCommitFiles,
    type GitCommitFilesState
  } from '$lib/shell/git/gitCommitFilesStore.svelte';
  import {
    canLoadMoreGitHistory,
    describeGitHistoryCount,
    describeGitHistoryFooter,
    type GitPanelState
  } from '$lib/shell/git/gitPanelStore.svelte';
  import type { GitService } from '$lib/shell/git/gitService';
  import { formatLastActivity } from '$lib/shell/relativeTime';
  import { showCenterTab } from '$lib/shell/workbenchNavigation';

  interface Props {
    panel: GitPanelState;
    service: GitService;
    commitFiles: GitCommitFilesService;
    commitFilesState: GitCommitFilesState;
  }
  let { panel, service, commitFiles, commitFilesState }: Props = $props();

  const countLabel = $derived(describeGitHistoryCount(panel));
  const footer = $derived(describeGitHistoryFooter(panel));
  const canLoadMore = $derived(canLoadMoreGitHistory(panel));

  function whenCommitted(committedAt: string): string {
    return formatLastActivity(committedAt, new Date());
  }

  function toggle(sha: string, parentCount: number): void {
    commitFiles.activate(panel.root);
    void commitFiles.toggleCommit(sha, parentCount > 1);
  }

  function pickFile(sha: string, file: GitCommitFileChange): void {
    commitFiles.activate(panel.root);
    void commitFiles.selectCommitFile(sha, file);
    showCenterTab('diff');
  }

  /** Everything the app told us about this commit, one fact per line. */
  function commitDetail(commit: {
    subject: string;
    author: string;
    committedAt: string;
    shortSha: string;
    refs: string;
  }): string[] {
    return [
      commit.subject,
      `${commit.author} · ${whenCommitted(commit.committedAt)}`,
      commit.refs.trim() === '' ? `Commit ${commit.shortSha}` : `Commit ${commit.shortSha} · ${commit.refs}`
    ];
  }
</script>

<section class="flex min-h-0 flex-col gap-1 px-1 pb-2" aria-label="Commits">
  <div class="flex items-center gap-2 px-2 pt-2">
    <h3 class="text-sm font-medium tracking-wide text-muted-foreground uppercase">Commits</h3>
    {#if countLabel !== ''}
      <Chip tone="count">{countLabel}</Chip>
    {/if}
  </div>

  {#if panel.historyLoading}
    <p class="px-2 py-1 text-sm text-muted-foreground">Reading the history…</p>
  {:else if panel.historyError !== ''}
    <p class="px-2 py-1 text-sm text-[var(--color-bad)]">{panel.historyError}</p>
  {:else if panel.history.length === 0}
    <p class="px-2 py-1 text-sm text-muted-foreground">This repository has no commits yet.</p>
  {:else}
    <ol class="flex min-w-0 flex-col">
      {#each panel.history as commit (commit.sha)}
        {@const expanded = isCommitExpanded(commitFilesState, commit.sha)}
        {@const entry = commitFilesEntry(commitFilesState, commit.sha)}
        {@const sentence = describeCommitFiles(entry, commit.parentCount > 1)}
        <li class="min-w-0">
          <Tooltip.Provider delayDuration={500}>
            <Tooltip.Root>
              <Tooltip.Trigger>
                {#snippet child({ props })}
                  <div {...props} class="min-w-0">
                    <ListRow
                      onclick={() => toggle(commit.sha, commit.parentCount)}
                      data-testid={`source-control-commit-${commit.shortSha}`}
                    >
                      <ChevronRight
                        class={`chevron size-3.5 shrink-0 text-muted-foreground ${
                          expanded ? 'is-open' : ''
                        }`}
                        aria-hidden="true"
                      />
                      <span class="min-w-0 flex-1 truncate">{commit.subject}</span>
                      <span class="shrink-0 text-sm text-muted-foreground">
                        {commit.author} · {whenCommitted(commit.committedAt)}
                      </span>
                    </ListRow>
                  </div>
                {/snippet}
              </Tooltip.Trigger>
              <Tooltip.Content side="left" sideOffset={6} class="max-w-[320px]">
                <span class="flex flex-col gap-1 text-left">
                  {#each commitDetail(commit) as line, index (index)}
                    <span class={index === 0 ? 'text-[13px] leading-snug' : 'text-sm text-muted-foreground'}>
                      {line}
                    </span>
                  {/each}
                </span>
              </Tooltip.Content>
            </Tooltip.Root>
          </Tooltip.Provider>

          {#if expanded}
            <div class="flex flex-col pb-1 pl-5">
              {#if sentence !== ''}
                <p class="px-2 py-1 text-sm text-muted-foreground">{sentence}</p>
              {:else}
                <p class="px-2 py-1 text-sm text-muted-foreground">
                  {summarizeCommitFiles(entry.files)}
                </p>
                {#each entry.files as file (file.relativePath)}
                  {@const parts = splitRepositoryPath(file.relativePath)}
                  <ListRow
                    onclick={() => pickFile(commit.sha, file)}
                    selected={commitFilesState.selectedCommitSha === commit.sha &&
                      commitFilesState.selectedRelativePath === file.relativePath}
                    data-testid={`source-control-commit-file-${file.relativePath}`}
                  >
                    <span
                      class="w-3 shrink-0 text-center font-mono text-sm leading-none text-muted-foreground"
                      aria-hidden="true">{file.badge}</span
                    >
                    <span class="min-w-0 flex-1 truncate" title={file.relativePath}>
                      {parts.name}
                      {#if parts.folder !== ''}
                        <span class="text-sm text-muted-foreground">{parts.folder}</span>
                      {/if}
                    </span>
                  </ListRow>
                {/each}
              {/if}
            </div>
          {/if}
        </li>
      {/each}
    </ol>

    {#if footer !== ''}
      <p class="px-2 pt-1 text-sm text-muted-foreground">{footer}</p>
    {/if}

    {#if canLoadMore}
      <div class="px-2 pt-1">
        <Button
          variant="ghost"
          size="sm"
          onclick={() => void service.loadMoreHistory()}
          disabled={panel.historyLoadingMore}
        >
          {panel.historyLoadingMore ? 'Reading older commits…' : 'Load more'}
        </Button>
      </div>
    {/if}
  {/if}
</section>

<style>
  /* The chevron turns to point down while its commit is open, and stops there. */
  section :global(.chevron) {
    transition: transform 120ms ease-out;
  }
  section :global(.chevron.is-open) {
    transform: rotate(90deg);
  }
  @media (prefers-reduced-motion: reduce) {
    section :global(.chevron) {
      transition: none;
    }
  }
</style>
