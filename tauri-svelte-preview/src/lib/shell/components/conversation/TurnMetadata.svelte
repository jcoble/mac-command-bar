<!--
  TurnMetadata.svelte — the quiet line under a settled turn.

  The time and copy action stay visible. They do not subscribe to or style from
  hover on the much larger message subtree that owns this footer.
-->
<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import Copy from '@lucide/svelte/icons/copy';
  import { formatClockTime } from '$lib/shell/dateFormat.ts';

  interface Props {
    /** The text the copy action puts on the clipboard. */
    text: string;
    timestampMs: number;
    /** `end` puts the line under a right-aligned bubble. */
    align?: 'start' | 'end';
  }

  let { text, timestampMs, align = 'start' }: Props = $props();

  const time = $derived(
    timestampMs > 0
      ? formatClockTime(new Date(timestampMs))
      : ''
  );

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
  {#if time}
    <span class="turn-time" data-testid="conversation-turn-time">{time}</span>
  {/if}
  <button
    class="copy-turn"
    type="button"
    aria-label={copied ? 'Copied' : 'Copy message'}
    title={copied ? 'Copied' : 'Copy message'}
    data-testid="copy-conversation-turn"
    onclick={() => void copyTurn()}
  >
    {#if copied}<Check />{:else}<Copy />{/if}
  </button>
</div>

<style>
  .turn-meta{position:absolute;top:100%;left:0;z-index:1;display:flex;align-items:center;gap:6px;min-height:16px;color:var(--color-text-3);font-size:12px}
  .turn-meta.end{right:0;left:auto;justify-content:flex-end}
  .turn-time{font-variant-numeric:tabular-nums}
  .copy-turn{display:grid;width:24px;height:24px;padding:0;border:0;border-radius:var(--radius-pill);place-items:center;background:transparent;color:var(--color-text-3);cursor:pointer}
  .copy-turn:hover{background:var(--color-hover);color:var(--color-text)}
  .copy-turn:focus-visible{outline:none;box-shadow:var(--focus-ring)}
  .copy-turn :global(svg){width:14px;height:14px}
</style>
