<script lang="ts">
  /**
   * SourceControlPanel.svelte — the Source control tab of the right column.
   *
   * What the working copy looks like, and the changes this version can make to
   * it: stage or unstage one file or all of them, throw one file's changes away,
   * commit what is staged, and fetch, pull or push from the header's more-actions
   * menu. Rebasing and pull requests are still absent.
   *
   * The picker at the top says which folder is being read. It defaults to the
   * session's own and can be pointed at any other checkout of the repository,
   * which is READ-ONLY: on somebody else's worktree the commit box, the two
   * buttons, the remote actions and the per-file write actions are all gone,
   * and a line says which folder is on screen. Selecting another checkout is
   * always read-only; Codex Assembly sessions get a separate action to make
   * that folder their durable session checkout.
   *
   * THIS PANEL IS THE ONLY PLACE A DISCARD IS ASKED ABOUT. A row can request
   * one; nothing reaches `gitService.discardPaths` until the dialog is answered,
   * because git keeps no copy of a discarded change.
   *
   * The diff is NOT drawn here. Picking a file asks the workbench to show that
   * file's changes, which land in the Diff tab in the middle of the shell, so
   * this panel never needs to know where that tab is.
   *
   * No IO of its own: every git call goes through `gitService`
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
  import * as Select from '$lib/components/ui/select/index.js';
  import { buttonVariants } from '$lib/components/ui/button/variants.js';
  import { Switch } from '$lib/components/ui/switch/index.js';
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
    describeGitBranchTitle,
    hasStagedChanges,
    isGitFileUntracked,
    isNotARepositoryError
  } from '$lib/shell/git/gitPanelStore.svelte';
  import { gitService as defaultService, type GitService } from '$lib/shell/git/gitService';
  import BranchMenu from '$lib/shell/components/git/BranchMenu.svelte';
  import DiscardConfirmDialog from '$lib/shell/components/git/DiscardConfirmDialog.svelte';
  import {
    describeDiscardQuestion,
    type DiscardQuestion,
    type DiscardTarget
  } from '$lib/shell/components/git/discardConfirm';
  import { worktreeManager } from '$lib/shell/worktrees/worktreeManagerStore.svelte';
  import { getConversationSession } from '$lib/shell/conversation/conversationStore.svelte';
  import { rail } from '$lib/shell/stores/sessionRailStore.svelte';
  import type { ProjectGitFileStatus } from '$lib/tauriSource';
  import { cn } from '$lib/utils';

  import ChangedFileRow from './ChangedFileRow.svelte';
  import CommitTimeline from './CommitTimeline.svelte';
  import {
    describeSourceControlScope,
    isSourceControlScopeReadOnly,
    sourceControlRemoteActions,
    sourceControlScopeOptions,
    type SourceControlRemoteActionId
  } from './sourceControlPanelActions.ts';
  import { sourceControlDiffstat, sourceControlSections } from './sourceControlSections.ts';

  interface Props {
    /** True while this is the tab in front. */
    visible: boolean;
    /** The active session's folder. The shell activates the service with it. */
    root: string;
    /** The active session, when there is one. Unused: this panel is per folder. */
    ownedId: string | null;
    /** False when the active session's checkout has disappeared. */
    rootAvailable?: boolean;
    /** The working-copy service. The shell's singleton unless a test says otherwise. */
    service?: GitService;
    /** The per-commit file service, likewise. */
    commitFiles?: GitCommitFilesService;
    commitFilesState?: GitCommitFilesState;
    /** Can this page change the repository? Defaults to "only in the desktop app". */
    canWrite?: boolean;
    /** Applies a selected linked worktree as the active session checkout. */
    onUseSessionCheckout?(root: string): void | Promise<void>;
  }
  let {
    visible,
    root,
    rootAvailable = true,
    service = defaultService,
    commitFiles = defaultCommitFilesService,
    commitFilesState = defaultCommitFilesState,
    canWrite = canChangeRepository(),
    onUseSessionCheckout
  }: Props = $props();

  /** Why nothing can be changed while the panel is on another checkout. */
  const READ_ONLY_SCOPE_MESSAGE =
    'This folder is open for reading only — switch back to the session folder to change it.';

  const panel = $derived(service.state);
  const folder = $derived(panel.root ?? root ?? '');

  /** The session's own folder — the one the shell activates the service with. */
  const sessionRoot = $derived((root ?? '').trim());
  /** The checkout the reader picked, or '' while the session's own is showing. */
  let scopeRoot = $state('');

  const scopeOptions = $derived(sourceControlScopeOptions(sessionRoot, worktreeManager.worktrees));
  const readOnlyScope = $derived(isSourceControlScopeReadOnly(sessionRoot, scopeRoot));
  const scopeValue = $derived(scopeRoot === '' ? sessionRoot : scopeRoot);
  const scopeLabel = $derived(
    scopeOptions.find((option) => option.path === scopeValue)?.label ?? 'Session folder'
  );

  const ownedSession = $derived(
    ownedId === null ? null : rail.owned.find((session) => session.ownedId === ownedId) ?? null
  );
  const conversation = $derived(ownedId === null ? null : getConversationSession(ownedId));
  const activeTool = $derived(
    conversation?.timeline.some(
      (entry) => entry.kind === 'tool' && (entry.state === 'started' || entry.state === 'updated')
    ) ?? false
  );
  let checkoutBusy = $state(false);
  const checkoutDisabledReason = $derived.by(() => {
    if (!readOnlyScope) return null;
    if (!onUseSessionCheckout) return 'Checkout changes are unavailable from this surface.';
    if (rail.activeOwnedId !== ownedId) return 'Select this session before changing its checkout.';
    if (ownedSession?.agent === 'claude') {
      return 'Claude Code sessions cannot change checkout; this action is Codex-only.';
    }
    if (ownedSession?.agent === 'antigravity') {
      return 'Antigravity (agy) sessions cannot change checkout; this action is Codex-only.';
    }
    if (ownedSession?.agent !== 'codex' || ownedSession.origin !== 'app') {
      return 'Only Codex Assembly sessions can change the session checkout.';
    }
    if (!conversation || conversation.provider !== 'codex') {
      return 'The Codex Assembly session is not connected yet.';
    }
    if (ownedSession.executionOwner !== 'structured') {
      return 'Checkout changes require the structured Assembly writer.';
    }
    if (conversation.sending || conversation.activeTurnId || ownedSession.activeTurnId) {
      return 'Finish the active turn before changing checkout.';
    }
    if (Object.keys(conversation.pendingApprovals).length > 0 || ownedSession.pendingPermission) {
      return 'Resolve the pending permission before changing checkout.';
    }
    if (Object.keys(conversation.pendingInputs).length > 0 || ownedSession.pendingInput) {
      return 'Complete the pending input before changing checkout.';
    }
    if (activeTool) return 'Wait for the active tool operation to finish before changing checkout.';
    if (
      ownedSession.runtimeState !== undefined
      && !['ready', 'suspended'].includes(ownedSession.runtimeState)
    ) {
      return 'Wait for the Codex session to become quiescent before changing checkout.';
    }
    return null;
  });
  const canUseSessionCheckout = $derived(
    readOnlyScope && !checkoutBusy && checkoutDisabledReason === null
  );
  const checkoutHint = $derived(
    checkoutBusy
      ? 'Changing the Codex session checkout…'
      : checkoutDisabledReason ?? 'Use this linked worktree as the Codex session checkout.'
  );

  /** A different session means a different repository: the old pick is dropped. */
  let scopedSessionRoot = '';
  $effect(() => {
    if (sessionRoot === scopedSessionRoot) return;
    scopedSessionRoot = sessionRoot;
    scopeRoot = '';
  });

  /** The one place a scope becomes a real read. `activate` ignores a repeat. */
  $effect(() => {
    const target = scopeRoot === '' ? sessionRoot : scopeRoot;
    if (target === '') {
      commitFiles.release();
      return;
    }
    service.activate(target);
    commitFiles.activate(target);
  });

  /** A folder with no repository in it is an ordinary thing to be looking at,
   * not a fault — so it gets a plain sentence instead of git's `fatal:` line. */
  const notARepository = $derived(
    isNotARepositoryError(panel.statusError) || isNotARepositoryError(panel.historyError)
  );

  const sections = $derived(sourceControlSections(panel.status));
  const diffstat = $derived(sourceControlDiffstat(panel.status));
  const staged = $derived(hasStagedChanges(panel.status));
  const busy = $derived(panel.actionBusy !== '');
  const files = $derived(panel.status?.files ?? []);
  let amend = $state(false);

  /** Can this panel change what it is looking at? Both answers have a sentence. */
  const canChange = $derived(rootAvailable && canWrite && !readOnlyScope);
  const cannotChangeReason = $derived(
    !rootAvailable
      ? 'Checkout/Worktree deleted.'
      : readOnlyScope
        ? READ_ONLY_SCOPE_MESSAGE
        : READ_ONLY_IN_BROWSER_MESSAGE
  );

  const remoteActions = $derived(
    sourceControlRemoteActions(panel.status, folder, {
      canWrite: canChange,
      readOnlyReason: cannotChangeReason,
      busy
    })
  );

  function runRemote(id: SourceControlRemoteActionId): void {
    void service.runRemoteAction(id);
  }

  async function useSessionCheckout(): Promise<void> {
    if (!canUseSessionCheckout || !onUseSessionCheckout) return;
    checkoutBusy = true;
    try {
      await onUseSessionCheckout(scopeValue);
    } finally {
      checkoutBusy = false;
    }
  }

  /** Which file sections the reader has opened. Starting collapsed keeps a
   * large working copy from mounting every changed-file row in one pass. */
  let openSections = $state<Record<string, boolean>>({});

  /** The list DOM is rebuilt on return, but the reader stays at the same place. */
  let scrollViewport = $state<HTMLElement | null>(null);
  let savedScrollTop = 0;

  $effect(() => {
    const viewport = scrollViewport;
    if (!viewport) return;
    viewport.scrollTop = savedScrollTop;
    const rememberScroll = () => {
      savedScrollTop = viewport.scrollTop;
    };
    viewport.addEventListener('scroll', rememberScroll, { passive: true });
    return () => {
      savedScrollTop = viewport.scrollTop;
      viewport.removeEventListener('scroll', rememberScroll);
    };
  });

  const stageablePaths = $derived(
    (panel.status?.files ?? [])
      .filter((file) => file.worktreeStatus !== '' || file.badge === '?')
      .map((file) => file.relativePath)
  );
  const canStageAll = $derived(canChange && !busy && stageablePaths.length > 0);
  const canCommit = $derived(canChange && !busy && staged && panel.commitMessage.trim() !== '');
  const canAmend = $derived(canChange && !busy && panel.commitMessage.trim() !== '');
  const canDiscardAll = $derived(canChange && !busy && files.length > 0);

  const stageHint = $derived.by(() => {
    if (!canChange) return cannotChangeReason;
    if (busy) return 'Wait for the current source-control action to finish.';
    if (stageablePaths.length === 0) return 'Everything is already staged.';
    return `Stage ${stageablePaths.length === 1 ? 'the one changed file' : `all ${stageablePaths.length} changed files`}`;
  });

  const commitHint = $derived.by(() => {
    if (!canChange) return cannotChangeReason;
    if (busy) return 'Wait for the current source-control action to finish.';
    if (amend) {
      if (panel.commitMessage.trim() === '') return 'Type the amended commit message first.';
      return 'Rewrite the last commit. Do not amend a commit that is already pushed and shared.';
    }
    if (!staged) return 'Stage something first — a commit takes what is staged.';
    if (panel.commitMessage.trim() === '') return 'Type a commit message first.';
    return 'Commit the staged files';
  });

  function stageAll(): void {
    if (!canStageAll) return;
    void service.stagePaths(stageablePaths);
  }

  function commit(): void {
    if (amend) {
      if (!canAmend) return;
      void service.amendCommit();
      return;
    }
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
    openSections = { ...openSections, [id]: !openSections[id] };
  }

  function stageOne(file: ProjectGitFileStatus): void {
    if (!canChange || busy) return;
    void service.stagePaths([file.relativePath]);
  }

  function unstageOne(file: ProjectGitFileStatus): void {
    if (!canChange || busy) return;
    void service.unstagePaths([file.relativePath]);
  }

  /** The changes being asked about, or null while nothing is. */
  let pendingDiscard = $state<{ question: DiscardQuestion; run: () => void } | null>(null);

  function askToDiscard(file: ProjectGitFileStatus): void {
    if (!canChange || busy) return;
    const target: DiscardTarget = {
      relativePath: file.relativePath,
      untracked: isGitFileUntracked(file)
    };
    pendingDiscard = {
      question: describeDiscardQuestion({ scope: 'file', targets: [target] }),
      run: () => void service.discardPaths([target.relativePath])
    };
  }

  function askToDiscardAll(): void {
    if (!canDiscardAll) return;
    const untrackedCount = files.filter(isGitFileUntracked).length;
    pendingDiscard = {
      question: describeDiscardQuestion({ scope: 'all', targets: [], untrackedCount }),
      run: () => void service.discardAll(untrackedCount > 0)
    };
  }

  /** The one route to a discard in this panel, and only from the dialog. */
  function confirmDiscard(): void {
    const pending = pendingDiscard;
    pendingDiscard = null;
    if (!pending || !canChange) return;
    pending.run();
  }
</script>

<div
  class="flex h-full min-h-0 w-full flex-col text-foreground"
  aria-label="Source control"
>
  <PanelHeader title="Source control" count={diffstat.filesChanged} data-testid="source-control-header">
    {#snippet actions()}
      <!-- Refresh, discard all, fetch, pull and push. Each item that cannot run
           carries the sentence saying why — usually a read-only folder or a
           branch with no upstream. -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger
          class={cn(buttonVariants({ variant: 'ghost', size: 'icon-sm' }))}
          aria-label="More source-control actions"
          data-testid="source-control-more-actions"
        >
          <Ellipsis aria-hidden="true" />
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="end">
          <DropdownMenu.Item
            data-testid="source-control-refresh"
            disabled={panel.statusLoading || panel.historyLoading}
            onSelect={() => void service.refresh()}
          >
            Refresh
          </DropdownMenu.Item>
          <DropdownMenu.Item
            data-testid="source-control-discard-all"
            disabled={!canDiscardAll}
            title={canDiscardAll
              ? undefined
              : canChange
                ? 'There are no changes to discard.'
                : cannotChangeReason}
            onSelect={() => askToDiscardAll()}
          >
            Discard all…
          </DropdownMenu.Item>
          <DropdownMenu.Separator />
          {#each remoteActions as item (item.id)}
            <DropdownMenu.Item
              data-testid={`source-control-remote-${item.id}`}
              disabled={!item.enabled}
              title={item.enabled ? undefined : item.disabledReason}
              onSelect={() => runRemote(item.id)}>{item.label}</DropdownMenu.Item
            >
          {/each}
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    {/snippet}

    <span class="min-w-0" title={describeGitBranchTitle(panel.status)}>
      <BranchMenu {panel} {service} canWrite={canChange} readOnlyReason={cannotChangeReason} />
    </span>
  </PanelHeader>

  {#if !panel.activated && rootAvailable}
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
    <!-- This body only exists in the document while its tab is showing. WebKit
         re-checks sibling styling after every inserted node, so mounting a long
         hidden file list grows quadratically and can freeze the whole app. The
         selection and open-section state live above this gate and survive. -->
    {#if visible}
    <ScrollArea bind:viewportRef={scrollViewport} class="min-h-0 flex-1">
      <div class="flex flex-col gap-2 p-2">
        {#if !rootAvailable}
          <p class="text-sm text-muted-foreground" data-testid="source-control-checkout-deleted">
            Checkout/Worktree deleted.
          </p>
        {/if}

        {#if scopeOptions.length > 1 || (!rootAvailable && scopeOptions.length > 0)}
          <Select.Root
            type="single"
            value={scopeValue}
            onValueChange={(value) => (scopeRoot = value === sessionRoot ? '' : value)}
          >
            <Select.Trigger
              size="sm"
              class="w-full min-w-0"
              aria-label="Folder this panel reads"
              data-testid="source-control-scope"
            >
              <span class="min-w-0 truncate">{scopeLabel}</span>
            </Select.Trigger>
            <Select.Content>
              {#each scopeOptions as option (option.path)}
                <Select.Item value={option.path} label={option.label} />
              {/each}
            </Select.Content>
          </Select.Root>
        {/if}

        {#if readOnlyScope}
          <p class="text-sm text-muted-foreground" data-testid="source-control-scope-note">
            {describeSourceControlScope(scopeRoot)}
          </p>
          <Button
            variant="outline"
            size="sm"
            class="w-full"
            disabled={!canUseSessionCheckout}
            title={checkoutHint}
            aria-label="Use as session checkout"
            data-testid="source-control-use-session-checkout"
            onclick={useSessionCheckout}
          >{checkoutBusy ? 'Changing checkout…' : 'Use as session checkout'}</Button>
          <p class="text-sm text-muted-foreground" data-testid="source-control-checkout-hint">
            {checkoutHint}
          </p>
        {:else}
          <label class="flex flex-col gap-1">
            <span class="sr-only">Commit message</span>
            <textarea
              bind:value={panel.commitMessage}
              rows="3"
              data-testid="source-control-commit-message"
              placeholder="Message. Press Command-Enter to commit."
              disabled={!canChange}
              onkeydown={commitOnShortcut}
              class="dark:bg-input/30 border-input focus-visible:border-(color:--focus-border)
                     w-full min-w-0 resize-y rounded-lg border bg-transparent px-2.5 py-1.5 text-[13px]
                     leading-snug outline-none transition-colors placeholder:text-muted-foreground
                     disabled:pointer-events-none disabled:opacity-50"
            ></textarea>
          </label>

          <div class="flex items-center gap-2">
            <label
              class="flex shrink-0 items-center gap-1.5 text-sm text-muted-foreground"
              title="Rewrite the last commit instead of adding one. Never do this to a commit that is already pushed and shared."
            >
              <Switch
                size="sm"
                disabled={!canChange}
                bind:checked={amend}
                data-testid="source-control-amend-toggle"
              />
              Amend
            </label>
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
              disabled={amend ? !canAmend : !canCommit}
              title={commitHint}
              data-testid="source-control-commit"
              onclick={commit}>{amend ? 'Amend' : 'Commit'}</Button
            >
          </div>
        {/if}

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
              aria-expanded={Boolean(openSections[section.id])}
              data-testid={`source-control-section-${section.id}`}
              onclick={() => toggleSection(section.id)}
            >
              <ChevronRight
                class={`chevron size-3.5 ${openSections[section.id] ? 'is-open' : ''}`}
                aria-hidden="true"
              />
              <span>{section.label}</span>
              <Chip tone="count">{section.files.length}</Chip>
            </button>

            {#if openSections[section.id]}
              {#if section.files.length === 0}
                <p class="px-2 py-1 text-sm text-muted-foreground">
                  {section.id === 'staged'
                    ? 'Nothing is staged yet.'
                    : section.id === 'untracked'
                      ? 'No new files.'
                      : 'No changes to files git already knows about.'}
                </p>
              {:else}
                {#each section.files as file (file.relativePath)}
                  <ChangedFileRow
                    {file}
                    root={folder}
                    selected={panel.selectedPath === file.relativePath}
                    canWrite={canChange}
                    readOnlyReason={cannotChangeReason}
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
  {/if}
</div>

<DiscardConfirmDialog
  question={pendingDiscard?.question ?? null}
  open={pendingDiscard !== null}
  onOpenChange={(next) => {
    if (!next) pendingDiscard = null;
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
