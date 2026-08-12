<!--
  icon-button.svelte — a square button carrying an icon and nothing else, with
  the label it needs in order to be usable.

  An icon on its own is a guess. This component makes the two things that fix
  that impossible to forget: `label` becomes both the accessible name and the
  tooltip text, so every icon button in the shell says what it does on hover
  and to a screen reader. Sizes start at 24px, the smallest square a pointer
  can reliably hit; 28px is the default and what the chrome should use.

  Usage:
    <IconButton label="Close panel" onclick={close}><X /></IconButton>
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import type { ButtonVariant } from '$lib/components/ui/button/variants.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';

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
    /** `xs` is 24px, `sm` 28px (the default for shell chrome), `default` 32px. */
    size?: 'xs' | 'sm' | 'default';
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
            class={className}
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
    class={className}
  >
    {@render children()}
  </Button>
{/if}
