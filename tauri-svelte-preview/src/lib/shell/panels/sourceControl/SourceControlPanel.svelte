<script lang="ts">
  /**
   * SourceControlPanel.svelte — the Source control tab of the right column.
   *
   * What the working copy looks like, and the changes this version can make to
   * it: stage or unstage one file or all of them, throw one file's changes away,
   * and commit what is staged. Pushing, pulling, rebasing and pull requests are
   * deliberately absent — the header keeps a slot for them and says so rather
   * than pretending they are one press away.
   *
   * THIS PANEL IS THE ONLY PLACE A DISCARD IS ASKED ABOUT. A row can request
   * one; nothing reaches `gitService.discardPaths` until the dialog is answered,
   * because git keeps no copy of a discarded change.
   *
   * The diff is NOT drawn here. Picking a file asks the workbench to show that
   * file's changes, which land in the Diff tab in the middle of the shell, so
   * this panel never needs to know where that tab is.
   *
   * No IO of its own and no `$effect`: every git call goes through `gitService`
   * (the working copy) or `gitCommitFilesService` (a commit's files), and only
   * ever from a press. The shell points the service at a folder with
   * `gitService.activate(root)`; until that happens the panel says so and stays
   * quiet, which is what keeps a cold launch free of git commands.
   *
   * WRITING vs READING. In the desktop app both writes work. A browser tab can
   * read a repository through the dev server's bridge but deliberately cannot
   * change one, so Stage All and Commit are off there and say why: a page left
   * open on a repository must not be able to commit it by accident.
   */
  import { Button } from '$lib/components/ui/button/index.js';
  import { Chip } from '$lib/components/ui/chip/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { PanelHeader } from '$lib/components/ui/panel-header/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import { buttonVariants } from '$lib/components/ui/button/variants.js';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Ellipsis from '@lucide/svelte/icons/ellipsis';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import { canChangeRepository, READ_ONLY_IN_BROWSER_MESSAGE } from '$lib/shell/git/gitBackendExtra';
  import {
    gitCommitFilesService as defaultCommitFilesService,
    type GitCommitFilesService
  } from '$lib/shell/git/gitCommitFilesService';
  import {
    gitCommitFiles as defaultCommitFilesState,
    type GitCommitFilesState
  } from '$lib/shell/git/gitCommitFilesStore.svelte';
  import {
    describeGitBranch,
    describeGitBranchTitle,
    hasStagedChanges,
    isGitFileUntracked,
    isNotARepositoryError
  } from '$lib/shell/git/gitPanelStore.svelte';
  import { gitService as defaultService, type GitService } from '$lib/shell/git/gitService';
  import DiscardConfirmDialog from '$lib/shell/components/git/DiscardConfirmDialog.svelte';
  import {
    describeDiscardQuestion,
    type DiscardTarget
  } from '$lib/shell/components/git/discardConfirm';
  import type { ProjectGitFileStatus } from '$lib/tauriSource';
  import { cn } from '$lib/utils';

  import ChangedFileRow from './ChangedFileRow.svelte';
  import CommitTimeline from './CommitTimeline.svelte';
  import { sourceControlDiffstat, sourceControlSections } from './sourceControlSections.ts';

  interface Props {
    /** True while this is the tab in front. */
    visible: boolean;
    /** The active session's folder. The shell activates the service with it. */
    root: string;
    /** The active session, when there is one. Unused: this panel is per folder. */
    ownedId: string | null;
    /** The working-copy service. The shell's singleton unless a test says otherwise. */
    service?: GitService;
    /** The per-commit file service, likewise. */
    commitFiles?: GitCommitFilesService;
    commitFilesState?: GitCommitFilesState;
    /** Can this page change the repository? Defaults to "only in the desktop app". */
    canWrite?: boolean;
  }
  let {
    root,
    service = defaultService,
    commitFiles = defaultCommitFilesService,
    commitFilesState = defaultCommitFilesState,
    canWrite = canChangeRepository()
  }: Props = $props();

  /** What the header's more-actions slot is holding space for. */
  const MORE_ACTIONS_HINT = 'Push, pull, and pull requests arrive in a later update.';

  const panel = $derived(service.state);
  const folder = $derived(panel.root ?? root ?? '');

  /** A folder with no repository in it is an ordinary thing to be looking at,
   * not a fault — so it gets a plain sentence instead of git's `fatal:` line. */
  const notARepository = $derived(
    isNotARepositoryError(panel.statusError) || isNotARepositoryError(panel.historyError)
  );

  const sections = $derived(sourceControlSections(panel.status));
  const diffstat = $derived(sourceControlDiffstat(panel.status));
  const staged = $derived(hasStagedChanges(panel.status));
  const busy = $derived(panel.actionBusy !== '');

  /** Which sections are folded away. Open until somebody folds one. */
  let collapsed = $state<Record<string, boolean>>({});

  const stageablePaths = $derived(
    (panel.status?.files ?? [])
      .filter((file) => file.worktreeStatus !== '' || file.badge === '?')
      .map((file) => file.relativePath)
  );
  const canStageAll = $derived(canWrite && !busy && stageablePaths.length > 0);
  const canCommit = $derived(canWrite && !busy && staged && panel.commitMessage.trim() !== '');

  const stageHint = $derived.by(() => {
    if (!canWrite) return READ_ONLY_IN_BROWSER_MESSAGE;
    if (busy) return 'Wait for the current source-control action to finish.';
    if (stageablePaths.length === 0) return 'Everything is already staged.';
    return `Stage ${stageablePaths.length === 1 ? 'the one changed file' : `all ${stageablePaths.length} changed files`}`;
  });

  const commitHint = $derived.by(() => {
    if (!canWrite) return READ_ONLY_IN_BROWSER_MESSAGE;
    if (busy) return 'Wait for the current source-control action to finish.';
    if (!staged) return 'Stage something first — a commit takes what is staged.';
    if (panel.commitMessage.trim() === '') return 'Type a commit message first.';
    return 'Commit the staged files';
  });

  function stageAll(): void {
    if (!canStageAll) return;
    void service.stagePaths(stageablePaths);
  }

  function commit(): void {
    if (!canCommit) return;
    void service.commit();
  }

  /** Cmd/Ctrl+Enter commits, which is what the box's placeholder promises. */
  function commitOnShortcut(event: KeyboardEvent): void {
    if (event.key !== 'Enter' || !(event.metaKey || event.ctrlKey)) return;
    event.preventDefault();
    commit();
  }

  function toggleSection(id: string): void {
    collapsed = { ...collapsed, [id]: !collapsed[id] };
  }

  function stageOne(file: ProjectGitFileStatus): void {
    if (!canWrite || busy) return;
    void service.stagePaths([file.relativePath]);
  }

  function unstageOne(file: ProjectGitFileStatus): void {
    if (!canWrite || busy) return;
    void service.unstagePaths([file.relativePath]);
  }

  /** The file whose changes are being asked about, or null while nothing is. */
  let discarding = $state<DiscardTarget | null>(null);
  const discardQuestion = $derived(
    discarding === null ? null : describeDiscardQuestion({ scope: 'file', targets: [discarding] })
  );

  function askToDiscard(file: ProjectGitFileStatus): void {
    if (!canWrite || busy) return;
    discarding = { relativePath: file.relativePath, untracked: isGitFileUntracked(file) };
  }

  /** The one route to a discard in this panel, and only from the dialog. */
  function confirmDiscard(): void {
    const target = discarding;
    discarding = null;
    if (!target || !canWrite) return;
    void service.discardPaths([target.relativePath]);
  }
</script>

<div
  class="flex h-full min-h-0 w-full flex-col bg-card text-foreground"
  aria-label="Source control"
>
  <PanelHeader title="Source control" count={diffstat.filesChanged} data-testid="source-control-header">
    {#snippet actions()}
      <!-- The slot the rest of source control lands in later. It is off, and it
           says what it is waiting for — a hint on a disabled button is easy to
           miss, so the reason sits on the wrapper the pointer actually reaches. -->
      <span title={MORE_ACTIONS_HINT} class="flex">
        <DropdownMenu.Root>
          <DropdownMenu.Trigger
            class={cn(buttonVariants({ variant: 'ghost', size: 'icon-sm' }))}
            disabled
            aria-label="More source-control actions"
            data-testid="source-control-more-actions"
          >
            <Ellipsis aria-hidden="true" />
          </DropdownMenu.Trigger>
          <DropdownMenu.Content align="end">
            <DropdownMenu.Item disabled>{MORE_ACTIONS_HINT}</DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </span>
    {/snippet}

    <span class="flex min-w-0 items-center gap-1" title={describeGitBranchTitle(panel.status)}>
      <GitBranch class="size-3.5 shrink-0" aria-hidden="true" />
      <span class="min-w-0 truncate">{describeGitBranch(panel.status)}</span>
    </span>
  </PanelHeader>

  {#if !panel.activated}
    <EmptyState
      title="No project selected yet"
      body="Pick a session and this panel shows that folder's source control."
    >
      {#snippet icon()}<GitBranch />{/snippet}
    </EmptyState>
  {:else if panel.desktopOnly}
    <EmptyState
      title="Source control runs in the desktop app"
      body="Nothing is loaded here. Open this project in the desktop app to see its changes."
    >
      {#snippet icon()}<GitBranch />{/snippet}
    </EmptyState>
  {:else if notARepository}
    <EmptyState
      title="This folder is not a git repository"
      body="There is no repository here to read, so there is nothing to show."
    >
      {#snippet icon()}<GitBranch />{/snippet}
    </EmptyState>
  {:else}
    <ScrollArea class="min-h-0 flex-1">
      <div class="flex flex-col gap-2 p-2">
        <label class="flex flex-col gap-1">
          <span class="sr-only">Commit message</span>
          <textarea
            bind:value={panel.commitMessage}
            rows="3"
            data-testid="source-control-commit-message"
            placeholder="Message. Press Command-Enter to commit."
            disabled={!canWrite}
            onkeydown={commitOnShortcut}
            class="dark:bg-input/30 border-input focus-visible:border-ring focus-visible:ring-ring/50
                   w-full min-w-0 resize-y rounded-lg border bg-transparent px-2.5 py-1.5 text-[13px]
                   leading-snug outline-none transition-colors placeholder:text-muted-foreground
                   focus-visible:ring-3 disabled:pointer-events-none disabled:opacity-50"
          ></textarea>
        </label>

        <div class="flex items-center gap-2">
          <Button
            variant="secondary"
            size="sm"
            class="flex-1"
            disabled={!canStageAll}
            title={stageHint}
            data-testid="source-control-stage-all"
            onclick={stageAll}>Stage All</Button
          >
          <Button
            size="sm"
            class="flex-1"
            disabled={!canCommit}
            title={commitHint}
            data-testid="source-control-commit"
            onclick={commit}>Commit</Button
          >
        </div>

        {#if panel.actionError !== ''}
          <p class="text-sm text-[var(--color-bad)]">{panel.actionError}</p>
        {:else if panel.actionStatus !== ''}
          <p class="text-sm text-muted-foreground">{panel.actionStatus}</p>
        {/if}

        {#if panel.statusError !== ''}
          <p class="text-sm text-[var(--color-bad)]">{panel.statusError}</p>
        {/if}

        {#each sections as section (section.id)}
          <section class="flex min-w-0 flex-col">
            <button
              type="button"
              class="flex min-h-6 items-center gap-1 rounded-md px-2 text-left text-sm
                     font-medium tracking-wide text-muted-foreground uppercase outline-none
                     transition-colors hover:text-foreground focus-visible:ring-3
                     focus-visible:ring-ring/50"
              aria-expanded={!collapsed[section.id]}
              data-testid={`source-control-section-${section.id}`}
              onclick={() => toggleSection(section.id)}
            >
              <ChevronRight
                class={`chevron size-3.5 ${collapsed[section.id] ? '' : 'is-open'}`}
                aria-hidden="true"
              />
              <span>{section.label}</span>
              <Chip tone="count">{section.files.length}</Chip>
            </button>

            {#if !collapsed[section.id]}
              {#if section.files.length === 0}
                <p class="px-2 py-1 text-sm text-muted-foreground">
                  {section.id === 'untracked'
                    ? 'No new files.'
                    : 'No changes to files git already knows about.'}
                </p>
              {:else}
                {#each section.files as file (file.relativePath)}
                  <ChangedFileRow
                    {file}
                    root={folder}
                    selected={panel.selectedPath === file.relativePath}
                    {canWrite}
                    readOnlyReason={READ_ONLY_IN_BROWSER_MESSAGE}
                    {busy}
                    onStage={stageOne}
                    onUnstage={unstageOne}
                    onRequestDiscard={askToDiscard}
                  />
                {/each}
              {/if}
            {/if}
          </section>
        {/each}

        <CommitTimeline {panel} {service} {commitFiles} {commitFilesState} />
      </div>
    </ScrollArea>
  {/if}
</div>

<DiscardConfirmDialog
  question={discardQuestion}
  open={discarding !== null}
  onOpenChange={(next) => {
    if (!next) discarding = null;
  }}
  onConfirm={confirmDiscard}
/>

<style>
  /* The chevron turns to point down while its section is open, and stops there. */
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
