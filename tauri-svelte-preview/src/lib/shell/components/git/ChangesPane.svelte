<script lang="ts">
  /**
   * ChangesPane.svelte — what has changed in the working copy, and the box that
   * turns it into a commit.
   *
   * The rows are deliberately tight, the way an editor's source-control list
   * is: file name first, the folder it lives in dimmed beside it, and git's own
   * letter for what happened to it on the right. The tightness comes from
   * padding, never from shrinking the text — 13px for a row's name and 12px for
   * anything secondary is the floor everywhere in this shell.
   *
   * ⌘Enter in the message box commits, which is the shortcut the box's own
   * placeholder promises. Ctrl+Enter does the same for anyone on a keyboard
   * without a command key.
   *
   * Presentational: it reads the state it is handed and calls the service it is
   * handed. Nothing starts here.
   */
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import FileDiff from '@lucide/svelte/icons/file-diff';
  import Minus from '@lucide/svelte/icons/minus';
  import Plus from '@lucide/svelte/icons/plus';
  import SquareArrowOutUpRight from '@lucide/svelte/icons/square-arrow-out-up-right';
  import Sparkles from '@lucide/svelte/icons/sparkles';
  import LoaderCircle from '@lucide/svelte/icons/loader-circle';
  import Undo2 from '@lucide/svelte/icons/undo-2';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import WandSparkles from '@lucide/svelte/icons/wand-sparkles';

  import { buttonVariants } from '$lib/components/ui/button/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
  import {
    buildGitStatusFileGroups,
    describeGitStatusGroups,
    gitFileTitle,
    gitStatusGroupActionLabel,
    hasGitFileUnstagedChanges,
    hasStagedChanges,
    isGitFileDeleted,
    isGitFileUntracked,
    type GitPanelState,
    type GitStatusFileGroup
  } from '$lib/shell/git/gitPanelStore.svelte';
  import { splitRepositoryPath } from '$lib/shell/git/gitCommitFilesStore.svelte';
  import { absolutePathWithin, type GitService } from '$lib/shell/git/gitService';
  import { requestOpenFile } from '$lib/shell/openFileBus';
  import {
    sourceControlFileContextMenuItems,
    type SourceControlFileAction
  } from './sourceControlContextMenu';
  import { describeCommitSuggestion, suggestCommitMessage } from './commitSuggestion';
  import type { DiscardTarget } from './discardConfirm';
  import { cn } from '$lib/utils';
  import type { ProjectGitFileStatus } from '$lib/tauriSource';

  interface Props {
    panel: GitPanelState;
    service: GitService;
    /** False in a browser: it can read the repository but not change it. */
    canWrite: boolean;
    readOnlyReason: string;
    /** A changed file was picked, so the diff should be brought to the front. */
    onShowDiff?: () => void;
    /** Ask the active agent for a commit subject, without opening the chat UI. */
    onGenerateCommitMessage?: () => Promise<string>;
    /** Whether the current active session can service an agent action. */
    agentAvailable?: boolean;
    /** Plain explanation for a disabled generation action. */
    agentUnavailableReason?: string;
    /**
     * Ask for these files' changes to be thrown away. THE PANE NEVER DISCARDS
     * ANYTHING ITSELF — it hands the request up, the shell asks the person, and
     * only that answer reaches the service.
     */
    onRequestDiscard?: (targets: DiscardTarget[]) => void;
    /** Same, for the whole working copy. */
    onRequestDiscardAll?: () => void;
  }
  let {
    panel,
    service,
    canWrite,
    readOnlyReason,
    onShowDiff,
    onGenerateCommitMessage,
    agentAvailable = false,
    agentUnavailableReason = 'No active agent session is running. Start an agent conversation to generate this message.',
    onRequestDiscard,
    onRequestDiscardAll
  }: Props = $props();

  /** Pick a file's changes, and ask for wherever they are drawn to come forward. */
  function pickFile(file: ProjectGitFileStatus): void {
    void service.selectFile(file);
    onShowDiff?.();
  }

  let open = $state(true);
  let generatingCommitMessage = $state(false);
  let generationError = $state('');
  /**
   * Amend rewrites the last commit instead of adding one. It is off every time
   * the panel is looked at afresh, and it is a switch rather than a second
   * button so the label on the one Commit button always says what will happen.
   */
  let amend = $state(false);

  const groups = $derived(buildGitStatusFileGroups(panel.status?.files ?? []));
  const summary = $derived(describeGitStatusGroups(groups));
  const busy = $derived(panel.actionBusy !== '');
  const branch = $derived(panel.status?.branch ?? 'this branch');
  const staged = $derived(hasStagedChanges(panel.status));
  const files = $derived(panel.status?.files ?? []);
  /** Everything not yet staged, changed or brand new — what "Stage all" takes. */
  const unstagedPaths = $derived(
    files.filter(hasGitFileUnstagedChanges).map((file) => file.relativePath)
  );
  const untrackedCount = $derived(files.filter(isGitFileUntracked).length);
  const canCommit = $derived(
    canWrite && !busy && panel.commitMessage.trim() !== '' && staged
  );
  /**
   * An amend with nothing staged is still a real thing to do: it is how the
   * last commit's message gets fixed. So the only thing it insists on is a
   * message, and it says in its hover text that it rewrites the last commit.
   */
  const canAmend = $derived(canWrite && !busy && panel.commitMessage.trim() !== '');
  const canStageAll = $derived(canWrite && !busy && unstagedPaths.length > 0);
  const canDiscardAll = $derived(canWrite && !busy && files.length > 0);
  const suggestion = $derived(suggestCommitMessage(files));
  const suggestionHint = $derived(describeCommitSuggestion(files));

  /** Why the Commit button is off, in the words that fit this moment. */
  const commitHint = $derived(
    !canWrite
      ? readOnlyReason
      : amend
        ? panel.commitMessage.trim() === ''
          ? 'Say what the last commit should say, then amend it.'
          : `Rewrite the last commit on ${branch}. Do not amend a commit that is already pushed and shared.`
        : !staged
          ? 'Stage a file first — a commit records the staged files.'
          : panel.commitMessage.trim() === ''
            ? 'Say what you changed, then commit.'
            : `Commit the staged files on ${branch}`
  );

  /** The colour a status letter gets, so the eye can sort the list without reading it. */
  function badgeTone(badge: string): string {
    if (badge === 'A' || badge === '?') return 'text-[var(--color-good)]';
    if (badge === 'D') return 'text-[var(--color-bad)]';
    if (badge === 'R' || badge === 'C') return 'text-[var(--color-accent)]';
    return 'text-[var(--color-attention)]';
  }

  function runGroupAction(group: GitStatusFileGroup): void {
    const paths = group.files.map((file) => file.relativePath);
    if (group.action === 'stage') void service.stagePaths(paths);
    else void service.unstagePaths(paths);
  }

  function runFileAction(group: GitStatusFileGroup, file: ProjectGitFileStatus): void {
    if (group.action === 'stage') void service.stagePaths([file.relativePath]);
    else void service.unstagePaths([file.relativePath]);
  }

  /** Ask the editor to open this file. The editor lane listens on the same bus. */
  function openInEditor(file: ProjectGitFileStatus): void {
    if (!panel.root) return;
    requestOpenFile({ path: absolutePathWithin(panel.root, file.relativePath) });
  }

  /** Put the same repository-relative path shown by the row on the clipboard. */
  function copyPath(file: ProjectGitFileStatus): void {
    if (typeof navigator === 'undefined' || !navigator.clipboard) return;
    void navigator.clipboard.writeText(file.relativePath);
  }

  function fileContextItems(group: GitStatusFileGroup, file: ProjectGitFileStatus) {
    return sourceControlFileContextMenuItems({
      groupAction: group.action,
      canWrite,
      busy,
      deleted: isGitFileDeleted(file),
      hasRoot: Boolean(panel.root)
    });
  }

  /** Route context-menu choices through the row's existing button handlers. */
  function runFileContextAction(
    action: SourceControlFileAction,
    group: GitStatusFileGroup,
    file: ProjectGitFileStatus
  ): void {
    if (action === 'open-diff') pickFile(file);
    else if (action === 'stage' || action === 'unstage') runFileAction(group, file);
    else if (action === 'discard') askToDiscardFile(file);
    else if (action === 'open-file') openInEditor(file);
    else copyPath(file);
  }

  /** Stage every changed and new file in one press. */
  function stageAll(): void {
    if (!canStageAll) return;
    void service.stagePaths(unstagedPaths);
  }

  /** Run the one thing the Commit button currently promises. */
  function runCommit(): void {
    if (amend) {
      if (canAmend) void service.amendCommit();
      return;
    }
    if (canCommit) void service.commit();
  }

  /**
   * ⌘Enter commits, as the box's placeholder promises. Adding Shift stages
   * everything first, which is the pair of actions this box is used for over
   * and over.
   */
  async function commitOnShortcut(event: KeyboardEvent): Promise<void> {
    if (event.key !== 'Enter' || !(event.metaKey || event.ctrlKey)) return;
    event.preventDefault();
    if (event.shiftKey) {
      if (!canStageAll && !staged) return;
      if (canStageAll) await service.stagePaths(unstagedPaths);
      runCommit();
      return;
    }
    runCommit();
  }

  /** Put the cheap suggested subject in the box, leaving it there to be edited. */
  function useSuggestion(): void {
    if (suggestion === '') return;
    panel.commitMessage = suggestion;
  }

  function discardTarget(file: ProjectGitFileStatus): DiscardTarget {
    return { relativePath: file.relativePath, untracked: isGitFileUntracked(file) };
  }

  /** Ask the shell to ask the person. Nothing is thrown away from in here. */
  function askToDiscardFile(file: ProjectGitFileStatus): void {
    if (!canWrite || busy) return;
    onRequestDiscard?.([discardTarget(file)]);
  }

  function askToDiscardGroup(group: GitStatusFileGroup): void {
    if (!canWrite || busy) return;
    onRequestDiscard?.(group.files.map(discardTarget));
  }

  async function generateCommitMessage(): Promise<void> {
    if (!onGenerateCommitMessage || !agentAvailable || generatingCommitMessage) return;
    generatingCommitMessage = true;
    generationError = '';
    try {
      const message = await onGenerateCommitMessage();
      panel.commitMessage = message.trim();
    } catch (error) {
      generationError = error instanceof Error ? error.message : String(error);
    } finally {
      generatingCommitMessage = false;
    }
  }

  const ROW_ACTION =
    'text-[var(--color-text-2)] ' +
    'opacity-0 transition-colors group-hover:opacity-100 focus-visible:opacity-100 ' +
    'hover:bg-[var(--color-elevated)] hover:text-[var(--color-text)]';
  const DISCARD_ROW_ACTION =
    ROW_ACTION + ' hover:bg-[var(--color-bad-bg)] hover:text-[var(--color-bad)]';

  function rowActionHint(label: string): string {
    if (!canWrite) return readOnlyReason;
    if (busy) return 'Wait for the current source-control action to finish.';
    return label;
  }
</script>

<!-- Open, this section is as tall as what is in it and no taller, capped at
     three-fifths of the panel. Taking a fixed half of the panel left a band of
     empty space above the commit history whenever there was little to show. -->
<section class={cn('flex min-h-0 shrink-0 flex-col', open && 'max-h-[60%]')}>
  <div class="flex w-full shrink-0 items-center gap-1 pr-1.5 pl-2">
    <button
      type="button"
      class="flex min-w-0 flex-1 items-center gap-1 py-1 text-left
             text-[12px] tracking-[0.06em] text-[var(--color-text-2)] uppercase
             transition-colors hover:text-[var(--color-text)]
             focus-visible:ring-3 focus-visible:ring-ring/50 outline-none"
      aria-expanded={open}
      onclick={() => (open = !open)}
    >
      {#if open}
        <ChevronDown class="size-3 shrink-0" aria-hidden="true" />
      {:else}
        <ChevronRight class="size-3 shrink-0" aria-hidden="true" />
      {/if}
      <span>Changes</span>
      <span class="ml-auto normal-case text-[var(--color-text-3)]">{summary}</span>
    </button>

    <!-- Help with the commit message lives here, as two small buttons on the
         section that needs it, rather than in a floating panel of its own. -->
    {#if open}
      <span data-testid="suggest-commit-message">
        <IconButton
          label={`Suggest a commit message. ${suggestionHint}`}
          size="sm"
          side="bottom"
          class="text-[var(--color-text-2)] hover:bg-[var(--color-elevated)] hover:text-[var(--color-text)]"
          disabled={!canWrite || suggestion === ''}
          onclick={useSuggestion}
        >
          <WandSparkles class="size-3.5" aria-hidden="true" />
        </IconButton>
      </span>
      <span data-testid="generate-commit-message">
        <IconButton
          label="Ask the active agent to write the commit message"
          size="sm"
          side="bottom"
          class="text-[var(--color-text-2)] hover:bg-[var(--color-elevated)] hover:text-[var(--color-text)]"
          disabled={!canWrite || !agentAvailable || generatingCommitMessage}
          onclick={() => void generateCommitMessage()}
        >
          {#if generatingCommitMessage}
            <LoaderCircle class="size-3.5 animate-spin" aria-hidden="true" />
          {:else}
            <Sparkles class="size-3.5" aria-hidden="true" />
          {/if}
        </IconButton>
      </span>
    {/if}
  </div>

  {#if open}
    <div class="flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto px-2 pt-0.5 pb-2">
      <div class="flex shrink-0 flex-col gap-1">
        <textarea
          class="w-full resize-y rounded-[6px] border border-[var(--color-border)]
                 bg-[var(--color-surface)] px-2 py-1.5 text-[13px] leading-[18px]
                 text-[var(--color-text)] placeholder:text-[var(--color-text-3)]
                 focus-visible:border-[var(--color-accent)] focus-visible:ring-3
                 focus-visible:ring-ring/50 outline-none disabled:opacity-60"
          rows="2"
          placeholder="Message (⌘Enter commits on '{branch}', ⇧⌘Enter stages everything first)"
          aria-label="Commit message"
          disabled={!canWrite}
          title={canWrite ? '' : readOnlyReason}
          bind:value={panel.commitMessage}
          onkeydown={commitOnShortcut}
        ></textarea>
        {#if !agentAvailable && onGenerateCommitMessage}
          <p class="text-[12px] leading-[16px] text-[var(--color-text-3)]" data-testid="commit-agent-unavailable">
            {agentUnavailableReason}
          </p>
        {/if}
        {#if generationError}
          <p class="text-[12px] leading-[16px] text-[var(--color-bad)]" role="alert">{generationError}</p>
        {/if}
      </div>

      <label
        class="flex shrink-0 items-center gap-1.5 text-[12px] leading-[16px] text-[var(--color-text-2)]"
        title="Rewrite the last commit instead of adding one. Never do this to a commit that is already pushed and shared."
      >
        <Switch size="sm" disabled={!canWrite} bind:checked={amend} data-testid="amend-toggle" />
        Amend the last commit
      </label>

      <button
        type="button"
        class={cn(buttonVariants({ variant: 'default', size: 'xs' }), 'w-full shrink-0 text-[12px]')}
        disabled={amend ? !canAmend : !canCommit}
        title={commitHint}
        onclick={runCommit}
        data-testid="commit-button"
      >
        {#if amend}
          {panel.actionBusy === 'amend' ? 'Amending…' : 'Amend last commit'}
        {:else}
          {panel.actionBusy === 'commit' ? 'Committing…' : 'Commit'}
        {/if}
      </button>

      <div class="flex shrink-0 items-center gap-1">
        <button
          type="button"
          class={cn(buttonVariants({ variant: 'secondary', size: 'xs' }), 'flex-1 gap-1 text-[12px] font-normal')}
          disabled={!canStageAll}
          title={canWrite
            ? unstagedPaths.length > 0
              ? `Stage all ${unstagedPaths.length} changed and new files`
              : 'Everything is already staged'
            : readOnlyReason}
          onclick={stageAll}
          data-testid="stage-all"
        >
          <Plus class="size-3" aria-hidden="true" />
          Stage all
        </button>
        <button
          type="button"
          class={cn(
            buttonVariants({ variant: 'ghost', size: 'xs' }),
            'flex-1 gap-1 text-[12px] font-normal text-[var(--color-bad)] hover:text-[var(--color-bad)]'
          )}
          disabled={!canDiscardAll}
          title={canWrite
            ? 'Throw away every change in this working copy. You will be asked first.'
            : readOnlyReason}
          onclick={() => onRequestDiscardAll?.()}
          data-testid="discard-all"
        >
          <Trash2 class="size-3" aria-hidden="true" />
          Discard all
        </button>
      </div>

      {#if groups.length === 0}
        <p class="px-1 py-1 text-[13px] leading-[18px] text-[var(--color-text-2)]">
          {panel.statusLoading ? 'Reading the repository…' : 'Nothing has changed yet.'}
        </p>
      {/if}

      {#each groups as group (group.id)}
        <div class="flex flex-col">
          <div class="flex items-center gap-1 px-1 pt-1 pb-0.5">
            <span class="text-[12px] leading-[16px] text-[var(--color-text-2)]">
              {group.label}
            </span>
            <span class="text-[12px] leading-[16px] text-[var(--color-text-3)]">
              {group.files.length}
            </span>
            <button
              type="button"
              class="ml-auto rounded-[4px] px-1 py-px text-[12px] leading-[16px]
                     text-[var(--color-text-2)] transition-colors
                     hover:bg-[var(--color-elevated)] hover:text-[var(--color-text)]
                     focus-visible:ring-3 focus-visible:ring-ring/50 outline-none
                     disabled:opacity-40"
              disabled={!canWrite || busy}
              title={canWrite ? '' : readOnlyReason}
              onclick={() => runGroupAction(group)}
            >
              {gitStatusGroupActionLabel(group)}
            </button>
            <button
              type="button"
              class="rounded-[4px] px-1 py-px text-[12px] leading-[16px]
                     text-[var(--color-text-3)] transition-colors
                     hover:bg-[var(--color-elevated)] hover:text-[var(--color-bad)]
                     focus-visible:ring-3 focus-visible:ring-ring/50 outline-none
                     disabled:opacity-40"
              disabled={!canWrite || busy}
              title={canWrite
                ? `Throw away the changes in these ${group.files.length} files. You will be asked first.`
                : readOnlyReason}
              onclick={() => askToDiscardGroup(group)}
            >
              Discard
            </button>
          </div>

          {#each group.files as file (group.id + file.relativePath)}
            {@const parts = splitRepositoryPath(file.relativePath)}
            <ContextMenu.Root>
              <ContextMenu.Trigger class="block">
                <div
                  class={cn(
                    'group flex items-center gap-1 rounded-[4px] pr-1 transition-colors',
                    'hover:bg-[var(--color-elevated)]',
                    panel.selectedPath === file.relativePath && 'bg-[var(--color-elevated)]'
                  )}
                >
                  <button
                    type="button"
                    class="flex min-w-0 flex-1 items-center gap-1.5 rounded-[4px] py-[3px] pl-1.5
                           text-left focus-visible:ring-3 focus-visible:ring-ring/50 outline-none"
                    title={gitFileTitle(file)}
                    onclick={() => pickFile(file)}
                  >
                    <FileDiff
                      class="size-3.5 shrink-0 text-[var(--color-text-3)]"
                      aria-hidden="true"
                    />
                    <span class="shrink-0 truncate text-[13px] leading-[18px]">{parts.name}</span>
                    {#if parts.folder}
                      <span class="min-w-0 truncate text-[12px] leading-[16px] text-[var(--color-text-3)]">
                        {parts.folder}
                      </span>
                    {/if}
                  </button>

                  <IconButton
                    label={`Open ${parts.name} in the editor`}
                    size="xs"
                    side="left"
                    class={ROW_ACTION}
                    onclick={() => openInEditor(file)}
                  >
                    <SquareArrowOutUpRight class="size-3.5" aria-hidden="true" />
                  </IconButton>
                  <span
                    class="inline-flex"
                    title={rowActionHint(`${group.action === 'stage' ? 'Stage' : 'Unstage'} ${parts.name}`)}
                  >
                    <IconButton
                      label={`${group.action === 'stage' ? 'Stage' : 'Unstage'} ${parts.name}`}
                      size="xs"
                      side="left"
                      tooltip={false}
                      class={ROW_ACTION}
                      disabled={!canWrite || busy}
                      onclick={() => runFileAction(group, file)}
                    >
                      {#if group.action === 'stage'}
                        <Plus class="size-3.5" aria-hidden="true" />
                      {:else}
                        <Minus class="size-3.5" aria-hidden="true" />
                      {/if}
                    </IconButton>
                  </span>
                  <span
                    class="inline-flex"
                    title={rowActionHint(`Throw away ${parts.name}'s changes. You will be asked first.`)}
                  >
                    <IconButton
                      label={`Throw away ${parts.name}'s changes`}
                      size="xs"
                      side="left"
                      tooltip={false}
                      class={DISCARD_ROW_ACTION}
                      disabled={!canWrite || busy}
                      onclick={() => askToDiscardFile(file)}
                    >
                      <Undo2 class="size-3.5" aria-hidden="true" />
                    </IconButton>
                  </span>

                  <span
                    class={cn(
                      'w-3 shrink-0 text-center text-[12px] leading-[16px] font-medium',
                      badgeTone(file.badge)
                    )}
                    title={file.status}
                  >
                    {file.badge || '·'}
                  </span>
                </div>
              </ContextMenu.Trigger>

              <ContextMenu.Content>
                {#each fileContextItems(group, file) as item (item.id)}
                  {#if item.id === 'copy-path'}
                    <ContextMenu.Separator />
                  {/if}
                  <ContextMenu.Item
                    disabled={!item.enabled}
                    onSelect={() => runFileContextAction(item.id, group, file)}
                  >
                    {item.label}
                  </ContextMenu.Item>
                {/each}
              </ContextMenu.Content>
            </ContextMenu.Root>
          {/each}
        </div>
      {/each}
    </div>
  {/if}
</section>
