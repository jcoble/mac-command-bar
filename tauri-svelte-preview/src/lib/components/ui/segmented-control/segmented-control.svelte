<!--
  segmented-control.svelte — a row of two to four mutually exclusive choices,
  shown all at once.

  Use it instead of a Select when the options are few, short, and worth reading
  side by side ("Bottom / Right / Hidden"). Use a Select when there are more
  than four, or the labels are long.

  It is written by hand rather than vendored: the registry has no segmented
  control, and the two primitives that come close both bring the wrong
  behaviour — Tabs owns panels of content, and a radio group stacks. What is
  here is the standard pattern for one: a `radiogroup` with roving focus, so
  Tab reaches the control once and the arrow keys move between choices.
-->
<script lang="ts">
  import { cn } from '$lib/utils.js';

  import type { SegmentedControlItem } from './types.js';

  interface Props {
    items: readonly SegmentedControlItem[];
    value: string;
    /** `sm` is 24px tall, the floor for a hit target; the default is 28px. */
    size?: 'sm' | 'default';
    disabled?: boolean;
    /** Names the control for a screen reader when no visible label sits beside it. */
    'aria-label'?: string;
    class?: string;
    onValueChange?: (value: string) => void;
  }

  let {
    items,
    value = $bindable(''),
    size = 'default',
    disabled = false,
    'aria-label': ariaLabel,
    class: className,
    onValueChange
  }: Props = $props();

  let buttons: (HTMLButtonElement | null)[] = $state([]);

  function choose(next: string): void {
    if (disabled || next === value) return;
    value = next;
    onValueChange?.(next);
  }

  /** Arrow keys move to the next or previous choice and select it, wrapping
   *  around; Home and End jump to the ends. This is what a radio group does. */
  function onKeyDown(event: KeyboardEvent, index: number): void {
    const last = items.length - 1;
    let target: number | null = null;
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') target = index === last ? 0 : index + 1;
    else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') target = index === 0 ? last : index - 1;
    else if (event.key === 'Home') target = 0;
    else if (event.key === 'End') target = last;
    if (target === null) return;
    event.preventDefault();
    choose(items[target].value);
    buttons[target]?.focus();
  }
</script>

<div
  data-slot="segmented-control"
  data-size={size}
  role="radiogroup"
  aria-label={ariaLabel}
  class={cn(
    'bg-background/60 ring-border inline-flex w-fit items-center gap-0.5 rounded-lg p-[3px] ring-1',
    'data-[size=default]:h-8 data-[size=sm]:h-7',
    disabled && 'pointer-events-none opacity-50',
    className
  )}
>
  {#each items as item, index (item.value)}
    {@const selected = item.value === value}
    {@const Icon = item.icon}
    <button
      bind:this={buttons[index]}
      type="button"
      role="radio"
      data-slot="segmented-control-item"
      aria-checked={selected}
      tabindex={selected || (!items.some((candidate) => candidate.value === value) && index === 0)
        ? 0
        : -1}
      {disabled}
      class={cn(
        'focus-visible:ring-ring/50 focus-visible:border-ring inline-flex h-full min-w-14 items-center justify-center gap-1.5 rounded-md border border-transparent px-2.5 text-[13px] font-medium whitespace-nowrap transition-colors outline-none select-none focus-visible:ring-3',
        "[&_svg:not([class*='size-'])]:size-3.5 [&_svg]:pointer-events-none [&_svg]:shrink-0",
        selected
          ? 'bg-secondary text-foreground shadow-[var(--shadow-sm)]'
          : 'text-muted-foreground hover:bg-accent/60 hover:text-foreground'
      )}
      onclick={() => choose(item.value)}
      onkeydown={(event) => onKeyDown(event, index)}
    >
      {#if Icon}<Icon aria-hidden="true" />{/if}
      {item.label}
    </button>
  {/each}
</div>
