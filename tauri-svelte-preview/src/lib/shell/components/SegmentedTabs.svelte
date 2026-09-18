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
    placed = true;
  });

  // A narrow strip keeps the same tabs and the same hit targets. Selection is
  // what brings an off-screen item back, matching direct trackpad scrolling.
  $effect(() => {
    value;
    const target = segments[selected];
    if (!target || !placed) return;
    target.scrollIntoView({ block: 'nearest', inline: 'nearest' });
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

  /** A mouse wheel has no horizontal axis. Use its vertical movement on this
   *  one-axis strip; trackpad sideways gestures keep their native behavior. */
  function onWheel(event: WheelEvent): void {
    if (!strip || Math.abs(event.deltaX) >= Math.abs(event.deltaY)) return;
    const limit = strip.scrollWidth - strip.clientWidth;
    if (limit <= 0) return;
    const next = Math.max(0, Math.min(limit, strip.scrollLeft + event.deltaY));
    if (next === strip.scrollLeft) return;
    event.preventDefault();
    strip.scrollLeft = next;
  }
</script>

<div
  bind:this={strip}
  class="segmented-tabs"
  class:placed
  role="tablist"
  aria-label={label}
  style:--indicator-left={`${frame.left}px`}
  style:--indicator-width={`${frame.width}px`}
  onkeydown={onKeyDown}
  onwheel={onWheel}
  tabindex={-1}
>
  <span class="indicator" aria-hidden="true"></span>
  {#each items as item, index (item.id)}
    {@const Icon = item.icon}
    {@const chosen = item.id === value}
    <button
      bind:this={segments[index]}
      type="button"
      role="tab"
      class="segment"
      aria-selected={chosen}
      aria-label={item.label}
      title={item.label}
      tabindex={chosen ? 0 : -1}
      data-testid={item.testId}
      onclick={() => onChange(item.id)}
    >
      {#if Icon}
        <Icon class="size-[22px]" strokeWidth={chosen ? 1.9 : 1.6} aria-hidden="true" />
      {:else}
        <span class="text">{item.label}</span>
      {/if}
    </button>
  {/each}
</div>

<style>
  .segmented-tabs {
    position: relative;
    display: flex;
    width: 100%;
    min-width: 0;
    /* The segments touch: the container's corners are the only round ones
       until the pill lands on one of them. */
    gap: 0;
    height: 42px;
    padding: 3px;
    border-radius: var(--radius-pill);
    background: var(--segmented-tab-surface);
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
    scroll-behavior: smooth;
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
    background: var(--segmented-tab-active);
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
    .segmented-tabs {
      scroll-behavior: auto;
    }

    .placed .indicator {
      transition: none;
    }
  }
</style>
