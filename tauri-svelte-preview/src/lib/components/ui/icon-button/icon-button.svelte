<!--
  icon-button.svelte — a square button carrying an icon and nothing else, with
  the label it needs in order to be usable.

  An icon on its own is a guess. This component makes the two things that fix
  that impossible to forget: `label` becomes both the accessible name and the
  tooltip text, so every icon button in the shell says what it does on hover
  and to a screen reader. Every icon button is at least 28px square, whichever
  size it asks for, because a smaller square is one a pointer misses.

  At rest it is only the glyph. Pointing at it fills a tinted disc behind the
  icon and puts a soft ring around that — the fill is mixed into the elevated
  surface rather than laid over it, so it is opaque and the row's own text
  cannot read through it. `tone` picks which of the shell's three signal
  colours that fill is drawn in.

  Usage:
    <IconButton label="Close panel" onclick={close}><X /></IconButton>
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import type { ButtonVariant } from '$lib/components/ui/button/variants.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import { cn } from '$lib/utils.js';

  type IconButtonTone = 'accent' | 'live' | 'attention';

  /**
   * The props are listed one by one rather than inherited from
   * `HTMLButtonAttributes`: every DOM attribute plus the button's own union of
   * variants is a type large enough that the checker gives up on it, and an
   * icon button only ever needs these. Anything more elaborate should use
   * `Button` directly.
   */
  interface Props {
    /** What the button does, in plain words. Used as the tooltip and the accessible name. */
    label: string;
    children: Snippet;
    /** How much room the glyph gets: `sm` (the default for shell chrome) and
     * `default` are 28px and 32px. `xs` asks for 24px and still gets a 28px
     * hit target, because the floor applies to every size. */
    size?: 'xs' | 'sm' | 'default';
    /** Which colour the hover fill is drawn in. Mint unless there is a reason. */
    tone?: IconButtonTone;
    variant?: ButtonVariant;
    side?: 'top' | 'bottom' | 'left' | 'right';
    /** Turn the tooltip off where the surrounding text already says the same thing. */
    tooltip?: boolean;
    disabled?: boolean;
    class?: string;
    "data-testid"?: string;
    onclick?: (event: MouseEvent) => void;
  }

  let {
    label,
    children,
    size = 'sm',
    tone = 'accent',
    variant = 'ghost',
    side = 'top',
    tooltip = true,
    disabled = false,
    class: className,
    "data-testid": dataTestId,
    onclick
  }: Props = $props();

  const buttonSize = $derived(
    size === 'xs' ? ('icon-xs' as const) : size === 'sm' ? ('icon-sm' as const) : ('icon' as const)
  );

  /**
   * The three recipes are written out in full rather than built from a colour
   * name: Tailwind reads this file as text, and a class it never sees written
   * down is a class it never generates.
   *
   * The caller's own `class` is applied after these, so a button that already
   * declares its own hover colour keeps it.
   */
  const TONE_CLASSES: Record<IconButtonTone, string> = {
    accent:
      'min-h-7 min-w-7 text-[var(--color-text-3)] duration-[120ms] motion-reduce:transition-none '
      + 'hover:bg-[color-mix(in_srgb,var(--color-accent)_16%,var(--color-elevated))] '
      + 'hover:text-[var(--color-accent)] '
      + 'hover:shadow-[0_0_0_4px_color-mix(in_srgb,var(--color-accent)_10%,transparent)] '
      + 'focus-visible:shadow-[var(--focus-ring)]',
    live:
      'min-h-7 min-w-7 text-[var(--color-text-3)] duration-[120ms] motion-reduce:transition-none '
      + 'hover:bg-[color-mix(in_srgb,var(--color-live)_16%,var(--color-elevated))] '
      + 'hover:text-[var(--color-live)] '
      + 'hover:shadow-[0_0_0_4px_color-mix(in_srgb,var(--color-live)_10%,transparent)] '
      + 'focus-visible:shadow-[var(--focus-ring)]',
    attention:
      'min-h-7 min-w-7 text-[var(--color-text-3)] duration-[120ms] motion-reduce:transition-none '
      + 'hover:bg-[color-mix(in_srgb,var(--color-attention)_16%,var(--color-elevated))] '
      + 'hover:text-[var(--color-attention)] '
      + 'hover:shadow-[0_0_0_4px_color-mix(in_srgb,var(--color-attention)_10%,transparent)] '
      + 'focus-visible:shadow-[var(--focus-ring)]'
  };

  /* Round, so an icon on its own reads as a control rather than as a small
     panel. The caller's class still wins if it names a different radius. */
  const buttonClass = $derived(cn('rounded-full', TONE_CLASSES[tone], className));
</script>

{#if tooltip}
  <Tooltip.Provider delayDuration={400}>
    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <Button
            {...props}
            data-slot="icon-button"
            size={buttonSize}
            {variant}
            {disabled}
            {onclick}
            data-testid={dataTestId}
            aria-label={label}
            class={buttonClass}
          >
            {@render children()}
          </Button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content {side} sideOffset={6}>{label}</Tooltip.Content>
    </Tooltip.Root>
  </Tooltip.Provider>
{:else}
  <Button
    data-slot="icon-button"
    size={buttonSize}
    {variant}
    {disabled}
    {onclick}
    data-testid={dataTestId}
    aria-label={label}
    class={buttonClass}
  >
    {@render children()}
  </Button>
{/if}
