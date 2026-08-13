<!--
  TurnMetadata.svelte — the quiet line under a settled turn.

  A turn shows nothing about itself until someone reaches for it: the time and
  the copy action sit at zero opacity and fade in when the turn is hovered or
  holds keyboard focus. The row is always in the page, so revealing it never
  moves the text above it.

  The host turn supplies the trigger by carrying `class="group"`.
-->
<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import Copy from '@lucide/svelte/icons/copy';
  import { HoverActionButton, HoverActions } from '$lib/components/ui/hover-actions/index.js';

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
      ? new Date(timestampMs).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })
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
    <span
      class="turn-time opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 motion-safe:transition-opacity motion-safe:duration-150 motion-safe:ease-out"
      data-testid="conversation-turn-time">{time}</span
    >
  {/if}
  <HoverActions label="Turn actions">
    <HoverActionButton
      label={copied ? 'Copied' : 'Copy message'}
      size="xs"
      data-testid="copy-conversation-turn"
      onclick={() => void copyTurn()}
    >
      {#if copied}<Check />{:else}<Copy />{/if}
    </HoverActionButton>
  </HoverActions>
</div>

<style>
  .turn-meta{position:absolute;top:100%;left:0;z-index:1;display:flex;align-items:center;gap:6px;min-height:16px;color:var(--color-text-3);font-size:12px}
  .turn-meta.end{right:0;left:auto;justify-content:flex-end}
  .turn-time{font-variant-numeric:tabular-nums}
</style>
