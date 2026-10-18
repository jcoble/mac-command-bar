<!--
  WorkflowPipelineStrip.svelte — a run's steps as one strip of segments, in
  order. A segment's fill is the step's state; a step that ran more than once
  shows its attempt count. Each segment opens that step's details.
-->
<script lang="ts">
  import type { ChipTone } from '$lib/components/ui/chip/index.js';

  interface Segment {
    key: string;
    title: string;
    tone: ChipTone;
    /** The step's state in words, for the tooltip and screen readers. */
    label: string;
    attempt: number;
  }

  let { segments, onSelect }: { segments: Segment[]; onSelect: (key: string) => void } = $props();

  // The chip's status fills, so a segment and its step's chip agree.
  const fill: Record<ChipTone, string> = {
    good: 'bg-[var(--color-good-bg)] text-[var(--color-good)]',
    bad: 'bg-[var(--color-bad-bg)] text-[var(--color-bad)]',
    live: 'bg-[var(--color-live-bg)] text-[var(--color-live)]',
    attention: 'bg-[var(--color-attention-bg)] text-[var(--color-attention)]',
    neutral: 'bg-muted-foreground/30 text-foreground',
    count: 'bg-secondary text-muted-foreground'
  };
</script>

<ol class="m-0 flex min-w-0 list-none gap-1 p-0" aria-label="Steps">
  {#each segments as segment, index (segment.key)}
    <li class="flex min-w-0 flex-1">
      <button
        type="button"
        class="flex h-6 min-w-0 flex-1 items-center justify-center rounded-md text-sm leading-none font-medium tabular-nums outline-none transition-[filter] hover:brightness-125 focus-visible:ring-3 focus-visible:ring-ring/50 {fill[segment.tone]}"
        title={`${index + 1}. ${segment.title}: ${segment.label}${segment.attempt > 1 ? `, ${segment.attempt} attempts` : ''}`}
        aria-label={`${segment.title}: ${segment.label}${segment.attempt > 1 ? `, ${segment.attempt} attempts` : ''}`}
        onclick={() => onSelect(segment.key)}
      >
        {segment.attempt > 1 ? `×${segment.attempt}` : ''}
      </button>
    </li>
  {/each}
</ol>
