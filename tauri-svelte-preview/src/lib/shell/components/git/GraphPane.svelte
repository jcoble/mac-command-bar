<script lang="ts">
  /**
   * GraphPane.svelte — the commit history, drawn as the branching picture it
   * actually is.
   *
   * Three pieces do the work, and none of them is in this file:
   *
   *  - `gitGraphLanes.ts` says which column each commit sits in and which line
   *    segments belong in its row. This file turns those numbers into an SVG.
   *  - `gitGraphViewModel.ts` (already built and tested, previously used only by
   *    the old shell) turns a commit into the branch and tag names to show, the
   *    task it belongs to, and what kind of commit it is.
   *  - `gitCommitFilesService.ts` reads what a commit changed when its row is
   *    opened, and puts a file's changes on screen when one is clicked.
   *
   * A row opens IN PLACE rather than replacing the list, so the line of history
   * either side of it stays visible — which is why the columns carry on being
   * drawn down the left of an open row's file list.
   *
   * The collapsible's content is hidden with the `hidden` attribute, which any
   * display rule of ours would beat, so the layout classes live on the div
   * inside it and never on the content itself.
   *
   * HOW MUCH HISTORY IS HERE. The list starts at the newest 24 commits and
   * grows on request. The count beside the heading says "so far" until a read
   * comes back short, and the line under the list says what actually came back
   * — never "that is all of them" — because an app build that reads fewer
   * commits than this panel asks for looks exactly like a repository that has
   * run out of history, and the two must not be confused.
   */
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';

  import { buttonVariants } from '$lib/components/ui/button/index.js';
  import * as Collapsible from '$lib/components/ui/collapsible/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { buildGitCommitGraphRows } from '$lib/gitGraphViewModel';
  import {
    assignGitGraphLanes,
    gitGraphLaneWidth,
    gitGraphLaneX,
    gitGraphRowLanesBelow,
    type GitGraphLaneRow
  } from '$lib/shell/git/gitGraphLanes';
  import {
    commitFilesEntry,
    describeCommitFiles,
    isCommitExpanded,
    isUnreadableGitPath,
    splitRepositoryPath,
    summarizeCommitFiles,
    type GitCommitFilesState
  } from '$lib/shell/git/gitCommitFilesStore.svelte';
  import type { GitCommitFilesService } from '$lib/shell/git/gitCommitFilesService';
  import {
    canLoadMoreGitHistory,
    describeGitHistoryCount,
    describeGitHistoryFooter,
    type GitPanelState
  } from '$lib/shell/git/gitPanelStore.svelte';
  import { COMMIT_HISTORY_LIMIT, type GitService } from '$lib/shell/git/gitService';
  import { formatLastActivity } from '$lib/shell/relativeTime';
  import { cn } from '$lib/utils';
  import {
    snapshotSourceControlCommitMenu,
    sourceControlContextMenuAnchor,
    type SourceControlCommitMenuSnapshot,
    type SourceControlContextMenuAction
  } from './sourceControlContextMenu';
  import SourceControlContextMenu from './SourceControlContextMenu.svelte';
  import type { GitCommitFileChange } from '$lib/shell/git/gitBackendExtra';

  interface Props {
    panel: GitPanelState;
    service: GitService;
    commitFiles: GitCommitFilesService;
    /** The commit-file state the service writes, read here for the open rows. */
    files: GitCommitFilesState;
    /** A file inside a commit was picked, so the diff should come to the front. */
    onShowDiff?: () => void;
  }
  let { panel, service, commitFiles, files, onShowDiff }: Props = $props();

  let open = $state(true);
  let contextMenu = $state<SourceControlCommitMenuSnapshot | null>(null);

  /** One row of the graph is exactly this tall, so the SVG can be drawn to size. */
  const ROW_HEIGHT = 40;
  /** Distance between two columns, and the gap either side of them. */
  const LANE_SPACING = 12;
  const LANE_OFFSET = 9;
  /** Past this the graph would eat the panel; extra columns fold into the last one. */
  const MAX_DRAWN_LANES = 6;

  /**
   * A colour per column, from the shell's own tokens, so a theme change carries
   * the graph with it. Six is enough to tell neighbouring branches apart; the
   * seventh column starts the list again.
   */
  const LANE_COLORS = [
    'var(--color-accent)',
    'var(--color-good)',
    'var(--color-attention)',
    'var(--color-bad)',
    'var(--color-live)',
    'var(--color-text-2)'
  ];

  const commits = $derived(buildGitCommitGraphRows(panel.history));
  // The lane assignment reads every commit's parents, so it is always handed the
  // whole accumulated list — which is why loading more replaces the list rather
  // than appending a page to it.
  const layout = $derived(assignGitGraphLanes(panel.history));
  const historyCount = $derived(describeGitHistoryCount(panel));
  const historyFooter = $derived(describeGitHistoryFooter(panel));
  const canLoadMore = $derived(canLoadMoreGitHistory(panel));
  const drawnLanes = $derived(Math.min(layout.laneCount, MAX_DRAWN_LANES));
  const graphWidth = $derived(
    Math.max(gitGraphLaneWidth(drawnLanes, LANE_SPACING, LANE_OFFSET), LANE_OFFSET * 2)
  );

  function laneColor(lane: number): string {
    return LANE_COLORS[lane % LANE_COLORS.length];
  }

  /** Columns past the drawn width are pinned to the last one rather than cut off. */
  function laneX(lane: number): number {
    return gitGraphLaneX(Math.min(lane, drawnLanes - 1), LANE_SPACING, LANE_OFFSET);
  }

  /** One line segment of a row, as an SVG path. */
  function edgePath(
    kind: 'child' | 'parent' | 'passing',
    fromLane: number,
    toLane: number
  ): string {
    const x1 = laneX(fromLane);
    const x2 = laneX(toLane);
    const middle = ROW_HEIGHT / 2;

    if (kind === 'passing') return `M ${x1} 0 L ${x1} ${ROW_HEIGHT}`;
    if (kind === 'child') return `M ${x1} 0 L ${x2} ${middle}`;
    if (x1 === x2) return `M ${x1} ${middle} L ${x1} ${ROW_HEIGHT}`;
    // A line changing column bends rather than cutting the corner, so which two
    // columns it joins stays readable when several do it at once.
    return `M ${x1} ${middle} C ${x1} ${middle + 10}, ${x2} ${ROW_HEIGHT - 12}, ${x2} ${ROW_HEIGHT}`;
  }

  function whenCommitted(committedAt: string): string {
    return formatLastActivity(committedAt, new Date());
  }

  function toggleCommit(row: Pick<GitGraphLaneRow, 'sha' | 'isMerge'>): void {
    commitFiles.activate(panel.root);
    void commitFiles.toggleCommit(row.sha, row.isMerge);
  }

  function copyCommitHash(sha: string): void {
    if (typeof navigator === 'undefined' || !navigator.clipboard) return;
    void navigator.clipboard.writeText(sha);
  }

  /** Keep the context menu on the same expansion path as a left click. */
  function openCommitContextMenu(
    row: GitGraphLaneRow,
    expanded: boolean,
    event: MouseEvent
  ): void {
    event.preventDefault();
    event.stopPropagation();
    contextMenu = snapshotSourceControlCommitMenu({
      sha: row.sha,
      isMerge: row.isMerge,
      expanded,
      anchor: sourceControlContextMenuAnchor(event)
    });
  }

  function runCommitContextAction(action: SourceControlContextMenuAction): void {
    const current = contextMenu;
    contextMenu = null;
    if (!current) return;
    if (action === 'toggle-commit') toggleCommit(current.target);
    else if (action === 'copy-hash') copyCommitHash(current.target.sha);
  }

  function pickFile(sha: string, file: GitCommitFileChange): void {
    commitFiles.activate(panel.root);
    void commitFiles.selectCommitFile(sha, file);
    onShowDiff?.();
  }

  /**
   * Branch names worth a pill of their own. The checked-out branch already
   * appears inside its `HEAD -> …` pill, so showing it twice would just eat the
   * room the commit's subject needs.
   */
  function branchPills(refs: { headLabels: string[]; branchLabels: string[] }): string[] {
    return refs.branchLabels.filter(
      (label) => !refs.headLabels.some((head) => head.includes(label))
    );
  }

  function badgeTone(badge: string): string {
    if (badge === 'A') return 'text-[var(--color-good)]';
    if (badge === 'D') return 'text-[var(--color-bad)]';
    if (badge === 'R' || badge === 'C') return 'text-[var(--color-accent)]';
    return 'text-[var(--color-attention)]';
  }

  /** A branch or tag name. Capped so a long branch name cannot push the commit
   * subject — the thing you are actually reading — off the end of the row. */
  const PILL =
    'inline-flex max-w-[45%] shrink-0 items-center truncate rounded-full border px-1.5 ' +
    'text-[12px] leading-[16px] font-medium';

  /** The same card, eyebrow and count pill the Stats & Usage screen is built from. */
  const CARD =
    'rounded-[var(--radius-md)] border ' +
    'border-[color-mix(in_srgb,var(--color-border)_36%,transparent)] ' +
    'bg-[color-mix(in_srgb,var(--color-elevated)_38%,var(--color-surface))]';
  const TILE =
    'rounded-[var(--radius-sm)] ' +
    'bg-[color-mix(in_srgb,var(--color-elevated)_56%,var(--color-surface))]';
  const EYEBROW =
    'text-[12px] leading-[16px] [font-weight:680] tracking-[0.085em] uppercase ' +
    'text-[var(--color-text-3)]';
  const COUNT_PILL =
    'inline-flex items-center justify-center rounded-full px-1.5 py-px text-[12px] ' +
    'leading-[16px] font-medium tabular-nums text-[var(--color-text-2)] ' +
    'bg-[color-mix(in_srgb,var(--color-elevated)_82%,transparent)]';
  const SECONDARY_ACTION =
    'bg-[color-mix(in_srgb,var(--color-elevated)_72%,transparent)] text-[var(--color-text-2)] ' +
    'hover:bg-[var(--color-hover)] hover:text-[var(--color-text)] ' +
    'disabled:cursor-not-allowed disabled:opacity-[0.52]';
</script>

<section
  class={cn('flex min-h-0 flex-col overflow-hidden', CARD, open ? 'flex-1' : 'shrink-0')}
>
  <div class="flex shrink-0 items-center gap-1.5 px-2.5 py-2">
    <button
      type="button"
      class={cn(
        'flex min-w-0 flex-1 items-center gap-1.5 text-left transition-colors',
        'hover:text-[var(--color-text)] focus-visible:ring-3 focus-visible:ring-ring/50 outline-none',
        EYEBROW
      )}
      aria-expanded={open}
      onclick={() => (open = !open)}
    >
      {#if open}
        <ChevronDown class="size-3 shrink-0" aria-hidden="true" />
      {:else}
        <ChevronRight class="size-3 shrink-0" aria-hidden="true" />
      {/if}
      <span>Commits</span>
      {#if historyCount}
        <span class={cn('ml-auto normal-case tracking-normal', COUNT_PILL)} title={historyFooter}>
          {historyCount}
        </span>
      {/if}
    </button>
    <IconButton
      label="Read the commit history again"
      size="sm"
      side="bottom"
      class="shrink-0 text-[var(--color-text-2)] hover:text-[var(--color-text)]"
      disabled={!panel.activated || panel.historyLoading}
      onclick={() => void service.refreshHistory()}
    >
      <RefreshCw class={cn('size-3.5', panel.historyLoading && 'animate-spin')} aria-hidden="true" />
    </IconButton>
  </div>

  {#if open}
    <div class="flex min-h-0 flex-1 flex-col overflow-y-auto pb-2">
      {#if panel.historyError && commits.length === 0}
        <div class={cn('mx-2.5 flex flex-col gap-1 px-2.5 py-4 text-center', TILE)}>
          <strong class="text-[13px] leading-[18px] font-medium text-[var(--color-bad)]">
            The history could not be read
          </strong>
          <p class="text-[12px] leading-[16px] text-[var(--color-text-2)]">{panel.historyError}</p>
        </div>
      {:else if panel.historyLoading && commits.length === 0}
        <div class={cn('mx-2.5 flex flex-col gap-1 px-2.5 py-4 text-center', TILE)}>
          <strong class="text-[13px] leading-[18px] font-medium text-[var(--color-text)]">
            Reading the commit history…
          </strong>
          <p class="text-[12px] leading-[16px] text-[var(--color-text-2)]">
            Asking git for the newest commits on this branch.
          </p>
        </div>
      {:else if commits.length === 0}
        <div class={cn('mx-2.5 flex flex-col gap-1 px-2.5 py-4 text-center', TILE)}>
          <strong class="text-[13px] leading-[18px] font-medium text-[var(--color-text)]">
            No commits yet.
          </strong>
          <p class="text-[12px] leading-[16px] text-[var(--color-text-2)]">
            The first commit you make will appear here.
          </p>
        </div>
      {:else}
        {#each commits as commit, index (commit.sha)}
          {@const row = layout.rows[index]}
          {@const entry = commitFilesEntry(files, commit.sha)}
          {@const expanded = isCommitExpanded(files, commit.sha)}
          {@const sentence = describeCommitFiles(entry, row?.isMerge ?? false)}
          <Collapsible.Root
            open={expanded}
            onOpenChange={() => row && toggleCommit(row)}
            class="border-b border-[var(--color-border)]/45 last:border-b-0"
          >
                <Collapsible.Trigger
                  class={cn(
                    'flex w-full items-stretch gap-1.5 pr-2.5 text-left transition-colors',
                    'hover:bg-[var(--color-hover)] focus-visible:ring-3 focus-visible:ring-ring/50',
                    'outline-none',
                    expanded &&
                      'bg-[color-mix(in_srgb,var(--color-accent)_12%,var(--color-elevated))]'
                  )}
                  title={commit.detailLabel}
                  oncontextmenu={(event) => row && openCommitContextMenu(row, expanded, event)}
                >
                  <svg
                    class="shrink-0"
                    width={graphWidth}
                    height={ROW_HEIGHT}
                    viewBox="0 0 {graphWidth} {ROW_HEIGHT}"
                    aria-hidden="true"
                  >
                    {#if row}
                      {#each row.edges as edge, edgeIndex (edgeIndex)}
                        <path
                          d={edgePath(edge.kind, edge.fromLane, edge.toLane)}
                          fill="none"
                          stroke={laneColor(edge.kind === 'child' ? edge.toLane : edge.fromLane)}
                          stroke-width="1.5"
                          stroke-linecap="round"
                          opacity="0.75"
                        />
                      {/each}
                      <circle
                        cx={laneX(row.lane)}
                        cy={ROW_HEIGHT / 2}
                        r={row.isMerge ? 3 : 3.5}
                        fill={row.isMerge ? 'var(--color-bg)' : laneColor(row.lane)}
                        stroke={laneColor(row.lane)}
                        stroke-width="1.5"
                      />
                    {/if}
                  </svg>

                  <span class="flex min-w-0 flex-1 flex-col justify-center py-[3px]">
                    <span class="flex min-w-0 items-center gap-1">
                      <!-- The branch you are on is mint; every other ref is blue,
                           and tags keep amber, so the checked-out branch is the
                           one chip the eye finds first. -->
                      {#each commit.refs.headLabels as label (label)}
                        <span
                          class="{PILL} border-[color-mix(in_srgb,var(--color-accent)_38%,transparent)]
                                 bg-[color-mix(in_srgb,var(--color-accent)_16%,transparent)]
                                 text-[var(--color-accent)]"
                        >
                          {label}
                        </span>
                      {/each}
                      {#each branchPills(commit.refs) as label (label)}
                        <span
                          class="{PILL} border-[color-mix(in_srgb,var(--color-live)_32%,transparent)]
                                 bg-[var(--color-live-bg)] text-[var(--color-live)]"
                        >
                          {label}
                        </span>
                      {/each}
                      {#each commit.refs.tagLabels as label (label)}
                        <span
                          class="{PILL} border-[color-mix(in_srgb,var(--color-attention)_32%,transparent)]
                                 bg-[var(--color-attention-bg)] text-[var(--color-attention)]"
                        >
                          {label}
                        </span>
                      {/each}
                      <span class="min-w-0 truncate text-[13px] leading-[17px] text-[var(--color-text)]">
                        {commit.subject || '(no message)'}
                      </span>
                    </span>
                    <span
                      class="truncate text-[12px] leading-[16px] tabular-nums text-[var(--color-text-2)]"
                    >
                      {commit.shortSha} · {commit.author} · {whenCommitted(commit.committedAt)}
                      {#if row?.isMerge}· merge{/if}
                    </span>
                  </span>
                </Collapsible.Trigger>

            <!-- Layout classes go on the div INSIDE: a closed collapsible is
                 hidden by the `hidden` attribute, which any display rule of
                 ours would beat, and the list would never close. -->
            <Collapsible.Content>
              <div class="relative flex flex-col pb-1">
                <!-- The columns carry on behind an open row, so the history
                     either side of it still reads as one picture. -->
                {#if row}
                  {#each gitGraphRowLanesBelow(row) as lane (lane)}
                    <span
                      class="pointer-events-none absolute top-0 bottom-0 w-px opacity-75"
                      style="left: {laneX(lane) - 0.5}px; background: {laneColor(lane)}"
                      aria-hidden="true"
                    ></span>
                  {/each}
                {/if}

                <div class="flex flex-col" style="padding-left: {graphWidth + 6}px">
                  {#if sentence}
                    <p class="py-1 pr-2 text-[12px] leading-[16px] text-[var(--color-text-2)]">
                      {sentence}
                    </p>
                  {:else}
                    <p class="py-0.5 pr-2 text-[12px] leading-[16px] text-[var(--color-text-3)]">
                      {summarizeCommitFiles(entry.files)} in this commit
                    </p>
                    {#each entry.files as file (file.relativePath)}
                      {@const parts = splitRepositoryPath(file.relativePath)}
                      {@const unreadable = isUnreadableGitPath(file.relativePath)}
                      {@const chosen =
                        files.selectedCommitSha === commit.sha &&
                        files.selectedRelativePath === file.relativePath}
                      <button
                        type="button"
                        class={cn(
                          'flex w-full items-center gap-1.5 rounded-[var(--radius-sm)] py-[3px] pr-1.5 pl-1',
                          'text-left transition-colors hover:bg-[var(--color-hover)]',
                          'focus-visible:ring-3 focus-visible:ring-ring/50 outline-none',
                          chosen && 'bg-[color-mix(in_srgb,var(--color-accent)_14%,var(--color-elevated))]',
                          unreadable && 'opacity-70'
                        )}
                        title={unreadable
                          ? 'This file has a name git could not print in plain letters.'
                          : `${file.relativePath} — ${file.status}`}
                        onclick={() => pickFile(commit.sha, file)}
                      >
                        <span class="shrink-0 truncate text-[13px] leading-[17px]">
                          {parts.name}
                        </span>
                        {#if parts.folder}
                          <span
                            class="min-w-0 truncate text-[12px] leading-[16px] text-[var(--color-text-3)]"
                          >
                            {parts.folder}
                          </span>
                        {/if}
                        <span
                          class={cn(
                            'ml-auto w-3 shrink-0 text-center text-[12px] leading-[16px] font-medium',
                            badgeTone(file.badge)
                          )}
                        >
                          {file.badge || '·'}
                        </span>
                      </button>
                    {/each}
                  {/if}
                </div>
              </div>
            </Collapsible.Content>
          </Collapsible.Root>
        {/each}

        <!-- How much of the history is on screen, and how to get more of it.
             Inside the scrolling list on purpose: it belongs to the end of the
             list, the way the bottom of a page belongs to the page. -->
        <div class="flex shrink-0 flex-col gap-1.5 px-2.5 pt-2 pb-0.5">
          {#if panel.historyError}
            <p class="text-[12px] leading-[16px] text-[var(--color-bad)]">
              {panel.historyError}
            </p>
          {/if}
          <p class="text-[12px] leading-[16px] tabular-nums text-[var(--color-text-3)]">
            {historyFooter}
          </p>
          {#if canLoadMore || panel.historyLoadingMore}
            <button
              type="button"
              class={cn(
                buttonVariants({ variant: 'secondary', size: 'xs' }),
                'w-full text-[12px] font-normal',
                SECONDARY_ACTION
              )}
              disabled={!canLoadMore}
              title="Read another {COMMIT_HISTORY_LIMIT} commits further back in this branch's history"
              onclick={() => void service.loadMoreHistory()}
            >
              {panel.historyLoadingMore
                ? 'Reading older commits…'
                : `Load ${COMMIT_HISTORY_LIMIT} more`}
            </button>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</section>

{#if contextMenu}
  {#key contextMenu.key}
    <SourceControlContextMenu
      anchor={contextMenu.anchor}
      items={contextMenu.items}
      onSelect={runCommitContextAction}
      onClose={() => (contextMenu = null)}
    />
  {/key}
{/if}
