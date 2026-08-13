<!--
  hover-action-button.svelte — one button inside a hover-action cluster.

  It is an IconButton, so it still owes its label as tooltip and accessible
  name. What it adds is the shared look for row actions: quiet at rest, and on
  hover it lights up on its own rather than as part of a block — each button
  answers for itself.

  `tone` says which kind of destination the action leads to, not which color to
  use: `primary` for the session itself, `info` for a file or editor surface,
  `success` for source control. Anything else stays `default`.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { cn } from '$lib/utils.js';

  type HoverActionTone = 'default' | 'primary' | 'info' | 'success';

  interface Props {
    label: string;
    children: Snippet;
    tone?: HoverActionTone;
    /** `xs` is 24px and the default here; a roomier row may use `sm` (28px). */
    size?: 'xs' | 'sm';
    disabled?: boolean;
    class?: string;
    'data-testid'?: string;
    onclick?: (event: MouseEvent) => void;
  }

  let {
    label,
    children,
    tone = 'default',
    size = 'xs',
    disabled = false,
    class: className,
    'data-testid': dataTestId,
    onclick
  }: Props = $props();

  /**
   * `info` and `success` reach past the shadcn slots on purpose: the registry
   * has no name for "a live surface" or "source control", and those two tones
   * are the shell's status colors, the same ones the rows use.
   */
  const TONES: Record<HoverActionTone, string> = {
    default: 'hover:bg-accent/60 hover:text-foreground',
    primary: 'hover:bg-primary/15 hover:text-primary',
    info: 'hover:bg-[var(--color-live-bg)] hover:text-[var(--color-live)]',
    success: 'hover:bg-[var(--color-good-bg)] hover:text-[var(--color-good)]'
  };
</script>

<IconButton
  {label}
  {size}
  {disabled}
  {onclick}
  side="bottom"
  data-testid={dataTestId}
  class={cn(
    'text-[var(--color-text-2)] rounded-md bg-transparent shadow-none',
    TONES[tone],
    className
  )}
>
  {@render children()}
</IconButton>
