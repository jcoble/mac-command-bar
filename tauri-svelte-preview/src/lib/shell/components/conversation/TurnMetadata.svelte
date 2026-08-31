<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import Copy from '@lucide/svelte/icons/copy';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import { formatWeekdayTime, formatClockTime } from '$lib/shell/dateFormat.ts';

  interface Props {
    /** The text the copy action puts on the clipboard. */
    text: string;
    timestampMs: number;
    /** `end` puts the line under a right-aligned bubble. */
    align?: 'start' | 'end';
    onBranch?(): void;
  }

  let { text, timestampMs, align = 'start', onBranch }: Props = $props();

  const time = $derived.by(() => {
    if (timestampMs <= 0) return '';
    const date = new Date(timestampMs);
    const now = Date.now();
    // If within 7 days, show weekday time (e.g. "Friday 1:42 AM"), else clock time
    if (now - timestampMs < 7 * 24 * 3600 * 1000) {
      return formatWeekdayTime(date);
    }
    return formatClockTime(date);
  });

  let copied = $state(false);
  let confirmationTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => () => {
    if (confirmationTimer) clearTimeout(confirmationTimer);
  });

  async function copyTurn(): Promise<void> {
    if (typeof navigator === 'undefined' || !navigator.clipboard) return;
    await navigator.clipboard.writeText(text);
    copied = true;
    if (confirmationTimer) clearTimeout(confirmationTimer);
    confirmationTimer = setTimeout(() => (copied = false), 1400);
  }
</script>

<div class="turn-meta" class:end={align === 'end'} data-testid="conversation-turn-meta">
  <button
    class="action-btn"
    type="button"
    aria-label={copied ? 'Copied' : 'Copy message'}
    title={copied ? 'Copied' : 'Copy message'}
    data-testid="copy-conversation-turn"
    onclick={() => void copyTurn()}
  >
    {#if copied}<Check size={13} />{:else}<Copy size={13} />{/if}
  </button>

  {#if align === 'start'}
    <button
      class="action-btn"
      type="button"
      aria-label="Fork chat"
      title="Fork chat"
      onclick={onBranch}
    >
      <GitBranch size={13} />
    </button>
  {/if}

  {#if time}
    <span class="turn-time" data-testid="conversation-turn-time">{time}</span>
  {/if}
</div>

<style>
  .turn-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 24px;
    margin-top: 6px;
    color: var(--color-text-3);
    font-size: 12px;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.12s ease;
  }
  :global(.message-wrap:hover) .turn-meta,
  :global(.message-wrap:focus-within) .turn-meta,
  .turn-meta:hover,
  .turn-meta:focus-within {
    opacity: 1;
    pointer-events: auto;
  }
  .turn-meta.end {
    justify-content: flex-end;
  }
  .turn-time {
    margin-left: 6px;
    color: var(--color-text-3);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .action-btn {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--color-text-3);
    cursor: pointer;
    transition: background 0.12s ease, color 0.12s ease;
  }
  .action-btn:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }
  .action-btn:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
</style>
