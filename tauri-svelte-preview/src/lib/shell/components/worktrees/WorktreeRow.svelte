<script lang="ts">
  /**
   * WorktreeRow.svelte — one worktree in the manager list.
   *
   * Draws only. Every value it shows was worked out by
   * `worktrees/worktreeManagerRows.ts`, and every button hands straight back to
   * the pane; nothing here reads the machine or decides anything.
   *
   * The row says four things, in the order a person needs them: what branch it
   * is, what is in the way of removing it, how long since anything happened,
   * and who is still working in it. A worktree with nothing in the way wears no
   * chips at all, which is what makes the ones that do wear them stand out.
   *
   * The three buttons carry plain `title` sentences rather than the shell's
   * hover cards on purpose: two of them spend most of their life switched off,
   * and a switched-off button never fires the pointer events a hover card
   * listens for — so the one explanation a person most needs to read would be
   * the one they could not reach.
   */
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import FolderGit2 from '@lucide/svelte/icons/folder-git-2';
  import type { Snippet } from 'svelte';

  import { Badge } from '$lib/components/ui/badge/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { exactLocalTime } from '$lib/shell/relativeTime';
  import type { WorktreeManagerRow } from '$lib/shell/worktrees/worktreeManagerRows';
  import {
    FORCE_REMOVE_UNAVAILABLE_TOOLTIP,
    type ForceRemoveSupport
  } from '$lib/shell/worktrees/worktreeManagerStore.svelte';
  import { cn } from '$lib/utils';

  interface Props {
    row: WorktreeManagerRow;
    /** This row's details are open below it. */
    expanded: boolean;
    /** An action is running on this row. */
    busy: boolean;
    /** An action is running somewhere in the pane; every button waits. */
    paneBusy: boolean;
    /** Whether this build of the desktop app can delete a worktree with work in it. */
    forceSupport: ForceRemoveSupport;
    onToggle(): void;
    onRemove(): void;
    onArchive(): void;
    /** Ask the "delete it anyway" question. The row never deletes anything. */
    onAskForceRemove(): void;
    /** The details, drawn inside this row while it is open. */
    detail?: Snippet;
  }
  let {
    row,
    expanded,
    busy,
    paneBusy,
    forceSupport,
    onToggle,
    onRemove,
    onArchive,
    onAskForceRemove,
    detail
  }: Props = $props();

  const CHIP_TONE: Record<string, string> = {
    danger: 'text-[var(--color-bad)] bg-[var(--color-bad-bg)]',
    warn: 'text-[var(--color-attention)] bg-[var(--color-attention-bg)]',
    info: 'text-[var(--color-text-2)] bg-[var(--color-elevated)]'
  };

  /** Why the everyday Remove button is switched off, or what it will do. */
  const removeTitle = $derived(
    row.isPrimary
      ? 'This is the repository itself, not a worktree of it. It is never removed from here.'
      : row.blockedReason
        ? `${row.blockedReason} Back it up, or use Delete anyway.`
        : 'Remove this worktree from the repository.'
  );

  /** Why the destructive button is switched off, or what it will do. */
  const forceTitle = $derived(
    forceSupport === 'available'
      ? 'Delete this worktree and everything in it that was never saved anywhere else.'
      : FORCE_REMOVE_UNAVAILABLE_TOOLTIP
  );
</script>

<li
  class={cn(
    'rounded-md ring-1 ring-transparent transition-colors',
    expanded && 'bg-[var(--color-elevated)] ring-[var(--color-border)]'
  )}
>
  <div class="flex w-full min-w-0 items-start gap-1.5 px-1.5 py-1.5">
    <button
      type="button"
      class="flex min-w-0 flex-1 flex-col items-start gap-1 rounded-md px-1 py-0.5 text-left
             hover:bg-[var(--color-elevated)]"
      aria-expanded={expanded}
      title={row.path}
      onclick={onToggle}
    >
      <span class="flex w-full min-w-0 items-center gap-1.5">
        <ChevronRight
          class={cn(
            'size-3.5 shrink-0 text-[var(--color-text-3)] transition-transform',
            expanded && 'rotate-90'
          )}
          aria-hidden="true"
        />
        <FolderGit2 class="size-3.5 shrink-0 text-[var(--color-text-3)]" aria-hidden="true" />
        <span class="truncate text-[13px] leading-[1.35] font-medium text-[#e6e6ee]">
          {row.branch}
        </span>
        {#if row.isPrimary}
          <Badge
            variant="secondary"
            class="h-5 shrink-0 px-1.5 text-[12px] font-normal text-[var(--color-text-2)]"
          >
            main checkout
          </Badge>
        {/if}
        {#if row.taskId}
          <Badge
            variant="outline"
            class="h-5 shrink-0 px-1.5 text-[12px] font-normal text-[var(--color-text-2)]"
          >
            {row.taskId}
          </Badge>
        {/if}
      </span>

      {#if row.chips.length > 0}
        <span class="flex w-full min-w-0 flex-wrap items-center gap-1 pl-5">
          {#each row.chips as chip (chip.id)}
            <span
              class={cn(
                'rounded-[6px] px-1.5 py-[1px] text-[12px] leading-[1.4]',
                CHIP_TONE[chip.tone]
              )}
              title={chip.title}
            >
              {chip.label}
            </span>
          {/each}
        </span>
      {/if}

      <span
        class="flex w-full min-w-0 flex-wrap items-center gap-x-1.5 gap-y-0.5 pl-5
               text-[12px] text-[var(--color-text-2)]"
      >
        <span class="truncate" title={row.path}>{row.folderName}</span>
        <span class="text-[var(--color-text-3)]">·</span>
        <span title={exactLocalTime(row.lastActivity)}>{row.age}</span>
        {#if row.aheadBehindLabel}
          <span class="text-[var(--color-text-3)]">·</span>
          <span>{row.aheadBehindLabel}</span>
        {/if}
        <span class="text-[var(--color-text-3)]">·</span>
        <span>{row.sessionsLabel}</span>
      </span>
    </button>

    <div class="flex shrink-0 items-center gap-1 pt-0.5">
      <span title={removeTitle}>
        <Button
          size="xs"
          variant="ghost"
          class="text-[12px] text-[var(--color-text-2)] hover:text-foreground"
          disabled={!row.canRemove || paneBusy}
          onclick={onRemove}
        >
          {busy ? 'Working…' : 'Remove'}
        </Button>
      </span>

      {#if !row.isPrimary}
        <span
          title="Copy everything in this worktree somewhere safe first — changes, commits, and files git was never told about."
        >
          <Button
            size="xs"
            variant="ghost"
            class="text-[12px] text-[var(--color-text-2)] hover:text-foreground"
            disabled={paneBusy}
            onclick={onArchive}
          >
            Back up
          </Button>
        </span>

        <span title={forceTitle}>
          <Button
            size="xs"
            variant="ghost"
            class="text-[12px] text-[var(--color-text-2)] hover:text-destructive"
            disabled={forceSupport !== 'available' || paneBusy}
            onclick={onAskForceRemove}
          >
            Delete anyway
          </Button>
        </span>
      {/if}
    </div>
  </div>

  {#if expanded && detail}
    <div class="border-t border-[var(--color-border)] px-2.5 py-2">
      {@render detail()}
    </div>
  {/if}
</li>
