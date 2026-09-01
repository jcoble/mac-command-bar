<!--
  hover-action-button.svelte — one button inside a hover-action cluster.

  It is an IconButton, so it still owes its label as tooltip and accessible
  name. Its hover is the one thing it draws differently: a translucent disc the
  size of the button itself, in the destination's own colour, with the glyph
  taking that colour too — the way a social app lights a row action. No outer
  glow: the ring the shell's other icon buttons wear made the target read as
  much larger than it is, and these sit shoulder to shoulder in a cluster.

  Translucent is safe here even though the shell's default disc is opaque: the
  cluster only ever covers the elapsed time, which has already faded out by the
  time a button can be hovered.

  `tone` says which kind of destination the action leads to, not which color to
  use: `primary` for the session itself, `info` for a file or editor surface,
  `success` for source control, `attention` for putting something away.
  Anything else stays `default`.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { cn } from '$lib/utils.js';

  type HoverActionTone = 'default' | 'primary' | 'info' | 'success' | 'attention';

  interface Props {
    label: string;
    children: Snippet;
    tone?: HoverActionTone;
    /** `sm` (28px) is the default, matching the approved mockup; `xs` (24px) is for tight rows. */
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
    size = 'sm',
    disabled = false,
    class: className,
    'data-testid': dataTestId,
    onclick
  }: Props = $props();

  /**
   * A destination maps to one of the shell's three signal colors. The tone
   * still picks WHICH colour; only how it is painted differs from the shared
   * button.
   */
  const TONES: Record<HoverActionTone, 'accent' | 'live' | 'attention'> = {
    default: 'accent',
    primary: 'accent',
    info: 'live',
    success: 'accent',
    attention: 'attention'
  };

  /**
   * The disc, written out in full for each colour: Tailwind reads this file as
   * text, so a class it never sees written down is a class it never generates.
   * `shadow-none` is the important half — it removes the 4px ring the shared
   * button adds, which is what made these look bigger than they are.
   */
  const DISC: Record<'accent' | 'live' | 'attention', string> = {
    accent:
      'hover:bg-[color-mix(in_srgb,var(--color-accent)_16%,transparent)] hover:shadow-none',
    live:
      'hover:bg-[color-mix(in_srgb,var(--color-live)_16%,transparent)] hover:shadow-none',
    attention:
      'hover:bg-[color-mix(in_srgb,var(--color-attention)_16%,transparent)] hover:shadow-none'
  };
</script>

<IconButton
  {label}
  {size}
  {disabled}
  {onclick}
  tone={TONES[tone]}
  side="bottom"
  data-testid={dataTestId}
  class={cn('rounded-full bg-transparent', DISC[TONES[tone]], className)}
>
  {@render children()}
</IconButton>
