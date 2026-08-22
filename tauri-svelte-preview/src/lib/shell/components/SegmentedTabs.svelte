<!--
  SegmentedTabs.svelte — one row of tabs inside a single container, with one
  pill that slides to whichever tab is chosen.

  The shape comes from Material's segmented control: a quiet container a step
  above the surface it sits on, segments that touch each other, and a filled
  pill marking the choice. Only one pill exists, so choosing a different tab
  moves it rather than swapping one highlight for another — that movement is
  what says the tabs belong to the same set.

  PRESENTATIONAL ONLY: no state beyond its own measurements, no IO, and no
  knowledge of what a tab leads to. The choice is handed in and every click is
  handed back out.
-->
<script lang="ts">
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import {
    cancelTrackedAnimationFrame,
    requestTrackedAnimationFrame
  } from '$lib/shell/resourceDiagnostics.svelte';

  import { indicatorFrame, type SegmentedTabItem } from './segmentedTabs';

  interface Props {
    items: readonly SegmentedTabItem[];
    value: string;
    /** Names the strip for a screen reader; no visible heading sits beside it. */
    label: string;
    onChange(id: string): void;
  }

  let { items, value, label, onChange }: Props = $props();

  let strip = $state<HTMLDivElement | null>(null);
  let segments = $state<(HTMLButtonElement | null)[]>([]);
  let widths = $state<number[]>([]);
  /** False until the pill has been placed once, so it does not fly in from
   *  nothing on the first paint. */
  let placed = $state(false);

  const selected = $derived(items.findIndex((item) => item.id === value));
  const frame = $derived(indicatorFrame(widths, selected));

  /** Fractional widths, not rounded ones: eight segments sharing a panel each
   *  land on a fraction of a pixel, and rounding them drifts the pill. */
  function measure(): void {
    widths = items.map((_, index) => segments[index]?.getBoundingClientRect().width ?? 0);
  }

  $effect(() => {
    const element = strip;
    if (!element) return;
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  });

  $effect(() => {
    if (widths.length === 0 || placed) return;
    const frameId = requestTrackedAnimationFrame(() => (placed = true));
    return () => cancelTrackedAnimationFrame(frameId);
  });

  /** Left and right walk the strip and choose as they go, wrapping at the ends. */
  function onKeyDown(event: KeyboardEvent): void {
    const last = items.length - 1;
    if (last < 0) return;
    let target: number | null = null;
    if (event.key === 'ArrowRight') target = selected >= last ? 0 : selected + 1;
    else if (event.key === 'ArrowLeft') target = selected <= 0 ? last : selected - 1;
    if (target === null) return;
    event.preventDefault();
    onChange(items[target].id);
    segments[target]?.focus();
  }
</script>

<Tooltip.Provider delayDuration={400}>
  <div
    bind:this={strip}
    class="segmented-tabs"
    class:placed
    role="tablist"
    aria-label={label}
    style="--indicator-left: {frame.left}px; --indicator-width: {frame.width}px"
    onkeydown={onKeyDown}
    tabindex={-1}
  >
    <span class="indicator" aria-hidden="true"></span>
    {#each items as item, index (item.id)}
      {@const Icon = item.icon}
      {@const chosen = item.id === value}
      <Tooltip.Root>
        <Tooltip.Trigger>
          {#snippet child({ props })}
            <button
              {...props}
              bind:this={segments[index]}
              type="button"
              role="tab"
              class="segment"
              aria-selected={chosen}
              aria-label={item.label}
              tabindex={chosen ? 0 : -1}
              data-testid={item.testId}
              onclick={() => onChange(item.id)}
            >
              {#if Icon}
                <Icon class="size-[20px]" strokeWidth={chosen ? 1.9 : 1.6} aria-hidden="true" />
              {:else}
                <span class="text">{item.label}</span>
              {/if}
            </button>
          {/snippet}
        </Tooltip.Trigger>
        <Tooltip.Content side="bottom" sideOffset={6}>{item.label}</Tooltip.Content>
      </Tooltip.Root>
    {/each}
  </div>
</Tooltip.Provider>

<style>
  .segmented-tabs {
    position: relative;
    display: flex;
    width: 100%;
    min-width: 0;
    /* The segments touch: the container's corners are the only round ones
       until the pill lands on one of them. */
    gap: 0;
    height: 38px;
    padding: 3px;
    border-radius: var(--radius-pill);
    background: var(--color-elevated);
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
    user-select: none;
  }

  .segmented-tabs::-webkit-scrollbar {
    display: none;
  }

  /* The one moving part. It is laid out once at the first segment's corner and
     after that only translated, so the move stays on the compositor. */
  .indicator {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 3px;
    width: var(--indicator-width);
    transform: translateX(var(--indicator-left));
    border-radius: var(--radius-pill);
    /* Material's muted lavender-grey: light enough on the container to read as
       filled, quiet enough not to compete with the mint accent. */
    background: #535367;
    pointer-events: none;
  }

  .placed .indicator {
    transition:
      transform 220ms cubic-bezier(0.2, 0, 0, 1),
      width 220ms cubic-bezier(0.2, 0, 0, 1);
  }

  .segment {
    position: relative;
    display: flex;
    flex: 1 1 0;
    min-width: 40px;
    align-items: center;
    justify-content: center;
    border: 0;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text-2);
    cursor: pointer;
    transition: color 160ms ease;
    outline: none;
  }

  /* Only the tabs the pill is not on light up, so pointing at the chosen tab
     changes nothing. */
  .segment[aria-selected='false']:hover {
    color: var(--color-text);
    background: color-mix(in srgb, var(--color-text) 6%, transparent);
  }

  .segment[aria-selected='true'] {
    color: var(--color-text);
  }

  .segment:focus-visible {
    box-shadow: var(--focus-ring);
  }

  .text {
    padding: 0 12px;
    font-size: 13px;
    white-space: nowrap;
  }

  .segment[aria-selected='true'] .text {
    font-weight: 600;
  }

  @media (prefers-reduced-motion: reduce) {
    .placed .indicator {
      transition: none;
    }
  }
</style>
