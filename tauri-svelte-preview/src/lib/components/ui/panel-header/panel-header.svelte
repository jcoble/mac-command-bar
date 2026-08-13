<!--
  panel-header.svelte — the top line of a panel.

  Every panel in the right column opens the same way: its name on the left, how
  many things are in it beside that, and its actions on the right. Doing that by
  hand is how eight panels ended up with eight different headers, so it lives
  here instead.

  Usage:
    <PanelHeader title="Worktrees" count={rows.length}>
      {#snippet actions()}
        <IconButton label="Refresh" onclick={refresh}><RefreshCw /></IconButton>
      {/snippet}
      {branchName}
    </PanelHeader>
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import { Chip } from '$lib/components/ui/chip/index.js';
  import { cn } from '$lib/utils.js';

  interface Props {
    title: string;
    /** Renders a `count`-toned Chip beside the title when not null. */
    count?: number | null;
    /** Right-aligned action slot — put IconButtons or a DropdownMenu trigger here. */
    actions?: Snippet;
    /** Optional second line under the title, for a branch name or a path. */
    children?: Snippet;
    class?: string;
    'data-testid'?: string;
  }

  let {
    title,
    count = null,
    actions,
    children,
    class: className,
    'data-testid': dataTestId
  }: Props = $props();
</script>

<header
  data-slot="panel-header"
  data-testid={dataTestId}
  class={cn('flex flex-col gap-1 border-b px-3 py-2', className)}
>
  <div class="flex min-h-7 min-w-0 items-center gap-2">
    <h2 class="min-w-0 flex-1 truncate text-[13px] leading-tight font-medium text-foreground">
      {title}
    </h2>
    {#if count !== null}
      <Chip tone="count">{count}</Chip>
    {/if}
    {#if actions}
      <div class="flex shrink-0 items-center gap-1">
        {@render actions()}
      </div>
    {/if}
  </div>
  {#if children}
    <p class="min-w-0 truncate text-sm leading-tight text-muted-foreground">
      {@render children()}
    </p>
  {/if}
</header>
