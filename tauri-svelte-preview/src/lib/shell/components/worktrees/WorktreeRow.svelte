<script lang="ts">
  /**
   * WorktreeRow.svelte — one worktree in the manager list.
   *
   * Draws only. Every value it shows was worked out by
   * `worktrees/worktreeManagerRows.ts`, and every button hands straight back to
   * the pane; nothing here reads the machine or decides anything.
   *
   * A CLOSED row is one line and stays one line: the branch, the short status
   * words, and the buttons. It used to carry the folder name, the age, the
   * remote standing and the session count on that same row with "·" between
   * them, which in a narrow pane wrapped into a four-line stack — and when those
   * fields were empty, into a stack of separators with nothing between them.
   * Those facts now live in the open body, where there is room to label them
   * (see `worktreeFacts`), and a fact with nothing in it is simply left out.
   *
   * The buttons carry plain `title` sentences rather than the shell's hover
   * cards on purpose: some of them spend most of their life switched off, and a
   * switched-off button never fires the pointer events a hover card listens for
   * — so the one explanation a person most needs to read would be the one they
   * could not reach.
   *
   * A row whose folder is already gone has nothing to back up and nothing to
   * delete, so it gets one action of its own instead of three that would not
   * mean anything.
   */
  import Archive from '@lucide/svelte/icons/archive';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import FolderGit2 from '@lucide/svelte/icons/folder-git-2';
  import OctagonAlert from '@lucide/svelte/icons/octagon-alert';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import type { Snippet } from 'svelte';

  import { Badge } from '$lib/components/ui/badge/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
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
    /** Ask the everyday remove question. The row never removes anything itself. */
    onAskRemove(): void;
    onArchive(): void;
    /** Ask the "clear git's record" question, for a row whose folder is gone. */
    onAskClear(): void;
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
    onAskRemove,
    onArchive,
    onAskClear,
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
    row.blockedReason
      ? `${row.blockedReason} Back it up, or use Delete anyway.`
      : 'Remove this worktree: deletes its folder and takes it out of the repository. You are asked first.'
  );

  /** Why the destructive button is switched off, or what it will do. The
   * 'unknown' state is real screen time — the capability probe runs after the
   * pane opens — and claiming the app is out of date before it has answered
   * would be a guess stated as a fact. */
  const forceTitle = $derived(
    forceSupport === 'available'
      ? 'Delete this worktree and everything in it that was never saved anywhere else. You are asked first.'
      : forceSupport === 'unknown'
        ? 'Still asking the app whether it can do this.'
        : FORCE_REMOVE_UNAVAILABLE_TOOLTIP
  );
</script>

<li
  class={cn(
    'rounded-md ring-1 ring-transparent transition-colors',
    expanded && 'bg-[var(--color-elevated)] ring-[var(--color-border)]'
  )}
>
  <!-- The closed row: one line, and it stays one line however narrow the pane
       gets. The branch is the part allowed to shrink; everything else keeps its
       size, because a half-drawn status word says less than none. -->
  <div class="flex w-full min-w-0 items-center gap-1 px-1.5 py-1">
    <button
      type="button"
      class="flex h-7 min-w-0 flex-1 items-center gap-1.5 overflow-hidden rounded-md px-1
             text-left hover:bg-[var(--color-elevated)]"
      aria-expanded={expanded}
      title={row.path}
      onclick={onToggle}
    >
      <ChevronRight
        class={cn(
          'size-3.5 shrink-0 text-[var(--color-text-3)] transition-transform',
          expanded && 'rotate-90'
        )}
        aria-hidden="true"
      />
      <FolderGit2 class="size-3.5 shrink-0 text-[var(--color-text-3)]" aria-hidden="true" />
      <span class="min-w-0 flex-1 truncate text-[13px] leading-[1.35] font-medium text-[var(--color-text)]">
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
          variant="secondary"
          class="h-5 shrink-0 px-1.5 text-[12px] font-normal text-[var(--color-text-2)]"
        >
          {row.taskId}
        </Badge>
      {/if}
      {#each row.chips as chip (chip.id)}
        <span
          class={cn(
            'shrink-0 rounded-[6px] px-1.5 py-[1px] text-[12px] leading-[1.4]',
            CHIP_TONE[chip.tone]
          )}
          title={chip.title}
        >
          {chip.short}
        </span>
      {/each}
      {#if busy}
        <span class="shrink-0 text-[12px] leading-[1.4] text-[var(--color-text-2)]">Working…</span>
      {/if}
    </button>

    <div class="flex shrink-0 items-center gap-0.5">
      {#if row.isPrimary}
        <!-- The repository itself. There is no action here to offer, and a row
             of switched-off buttons would only invite a click. -->
      {:else if row.folderGone}
        <span title="The folder is already gone. This clears git’s record of it, and touches nothing on disk. You are asked first.">
          <Button
            size="xs"
            variant="ghost"
            class="text-[12px] text-[var(--color-text-2)] hover:text-foreground"
            disabled={paneBusy}
            onclick={onAskClear}
          >
            Clear this entry
          </Button>
        </span>
      {:else}
        <span title="Copy everything in this worktree somewhere safe first — changes, commits, and files git was never told about.">
          <Button
            size="icon-xs"
            variant="ghost"
            aria-label="Back up this worktree"
            class="text-[var(--color-text-2)] hover:text-foreground"
            disabled={paneBusy}
            onclick={onArchive}
          >
            <Archive aria-hidden="true" />
          </Button>
        </span>

        <span title={removeTitle}>
          <Button
            size="icon-xs"
            variant="ghost"
            aria-label="Remove this worktree"
            class="text-[var(--color-text-2)] hover:text-foreground"
            disabled={!row.canRemove || paneBusy}
            onclick={onAskRemove}
          >
            <Trash2 aria-hidden="true" />
          </Button>
        </span>

        <span title={forceTitle}>
          <Button
            size="icon-xs"
            variant="ghost"
            aria-label="Delete this worktree anyway, along with anything left in it"
            class="text-[var(--color-text-2)] hover:text-destructive"
            disabled={forceSupport !== 'available' || paneBusy}
            onclick={onAskForceRemove}
          >
            <OctagonAlert aria-hidden="true" />
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
