<script lang="ts">
  /**
   * WorktreeDetail.svelte — what is open under one worktree row.
   *
   * Two halves, and both come from `$lib/worktreeSafety.ts` rather than from
   * anything decided in this folder: the verdict on the left (what to do about
   * this worktree and why), and the three commands on the right (look at it,
   * back it up, remove it) written out ready to paste into a terminal.
   *
   * The commands are here for the cases the buttons cannot cover — a worktree
   * with a lock somebody else set, or work only its owner can judge. Copying one
   * changes nothing on its own, which is the point: the reader takes it away,
   * reads it, and runs it themselves.
   */
  import Check from '@lucide/svelte/icons/check';
  import Copy from '@lucide/svelte/icons/copy';

  import { Button } from '$lib/components/ui/button/index.js';
  import type { WorktreeManagerRow } from '$lib/shell/worktrees/worktreeManagerRows';
  import { cn } from '$lib/utils';

  interface Props {
    row: WorktreeManagerRow;
    /** Focus a session that worked here. Left out, the sessions are plain text. */
    onOpenSession?: (ownedId: string) => void;
  }
  let { row, onOpenSession }: Props = $props();

  /** Which command was copied a moment ago, so its button can say so. */
  let copied = $state('');
  /** Why a copy did not work, when the browser refuses the clipboard. */
  let copyError = $state('');
  let copiedTimer: ReturnType<typeof setTimeout> | null = null;

  const LANE_TONE: Record<string, string> = {
    blocked: 'text-[var(--color-bad)] bg-[var(--color-bad-bg)]',
    backup: 'text-[var(--color-attention)] bg-[var(--color-attention-bg)]',
    cleanup: 'text-[var(--color-live)] bg-[var(--color-live-bg)]',
    review: 'text-[var(--color-text-2)] bg-[var(--color-elevated)]',
    protected: 'text-[var(--color-good)] bg-[var(--color-good-bg)]'
  };

  const commands = $derived([
    {
      id: 'audit',
      label: 'Look at what is in here',
      command: row.commands.audit
    },
    {
      id: 'backup',
      label: 'Copy the work somewhere safe',
      command: row.commands.backup
    },
    {
      id: 'cleanup',
      label: 'Remove it from the repository',
      command: row.commands.cleanup
    }
  ]);

  async function copy(id: string, command: string): Promise<void> {
    copyError = '';
    try {
      await navigator.clipboard.writeText(command);
      copied = id;
      if (copiedTimer) clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => (copied = ''), 1600);
    } catch {
      copied = '';
      copyError = 'The clipboard is not available here. Select the command and copy it by hand.';
    }
  }
</script>

<div class="flex flex-col gap-2.5">
  <!-- The verdict, in three lines: what to do, why, and what to check first. -->
  <div class="flex flex-col gap-1.5">
    <div class="flex flex-wrap items-center gap-1.5">
      <span
        class={cn('rounded-[6px] px-1.5 py-[1px] text-[12px] leading-[1.4]', LANE_TONE[row.lane.tone])}
      >
        {row.lane.label}
      </span>
      <span class="text-[13px] leading-[1.5] text-[var(--color-text)]">{row.lane.detail}</span>
    </div>
    <p class="m-0 text-[12px] leading-[1.5] text-[var(--color-text-2)]">
      {row.safety.reason}. {row.safety.recommendation}
    </p>
    {#if row.safety.decisionChecklist.length > 0}
      <ul class="m-0 flex list-none flex-col gap-0.5 p-0">
        {#each row.safety.decisionChecklist as item, index (index)}
          <li class="flex gap-1.5 text-[12px] leading-[1.5] text-[var(--color-text-2)]">
            <span class="text-[var(--color-text-3)]" aria-hidden="true">•</span>
            <span>{item}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <!-- Who has worked in this folder. Named, because "2 sessions" is not enough
       to decide whether deleting the folder would interrupt anybody. -->
  {#if row.sessions.length > 0}
    <div class="flex flex-col gap-1">
      <h4 class="m-0 text-[12px] font-semibold tracking-[0.09em] text-[var(--color-text-2)] uppercase">
        Sessions here
      </h4>
      <ul class="m-0 flex list-none flex-wrap gap-1 p-0">
        {#each row.sessions as link (link.ownedId)}
          <li>
            {#if onOpenSession}
              <button
                type="button"
                class="rounded-[6px] bg-[var(--color-surface)] px-1.5 py-[2px] text-[12px]
                       text-[var(--color-text-2)] hover:text-foreground"
                title="Show this session"
                onclick={() => onOpenSession?.(link.ownedId)}
              >
                {link.title}{link.isRunning ? ' · running' : ''}
              </button>
            {:else}
              <span
                class="rounded-[6px] bg-[var(--color-surface)] px-1.5 py-[2px] text-[12px]
                       text-[var(--color-text-2)]"
              >
                {link.title}{link.isRunning ? ' · running' : ''}
              </span>
            {/if}
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  <!-- The three commands, to take away and run yourself. -->
  <div class="flex flex-col gap-1.5">
    <h4 class="m-0 text-[12px] font-semibold tracking-[0.09em] text-[var(--color-text-2)] uppercase">
      Commands to run yourself
    </h4>
    {#each commands as entry (entry.id)}
      <div class="flex flex-col gap-1">
        <div class="flex items-center justify-between gap-2">
          <span class="text-[12px] text-[var(--color-text-2)]">{entry.label}</span>
          <Button
            size="xs"
            variant="ghost"
            class="text-[12px] text-[var(--color-text-2)] hover:text-foreground"
            onclick={() => void copy(entry.id, entry.command)}
          >
            {#if copied === entry.id}
              <Check aria-hidden="true" /> Copied
            {:else}
              <Copy aria-hidden="true" /> Copy
            {/if}
          </Button>
        </div>
        <pre
          class="m-0 max-h-24 overflow-auto rounded-md bg-[var(--color-bg)] px-2 py-1.5
                 text-[12px] leading-[1.5] text-[var(--color-text-2)] whitespace-pre-wrap
                 ring-1 ring-[var(--color-border)]">{entry.command}</pre>
      </div>
    {/each}
    {#if copyError}
      <p class="m-0 text-[12px] leading-[1.5] text-[var(--color-bad)]">{copyError}</p>
    {/if}
  </div>
</div>
