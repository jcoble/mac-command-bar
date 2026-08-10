<script lang="ts">
  /**
   * CenterActivityDock.svelte — the center Dockview's permanent navigation.
   * Dockview remains the layout and activation state machine; this component
   * only presents that state as an icon rail on the far-right window edge.
   */
  import { onMount } from 'svelte';
  import {
    Bot,
    ChartNoAxesCombined,
    Cpu,
    FileCode2,
    GitCompareArrows,
    Globe2,
    History,
    MessagesSquare
  } from '@lucide/svelte';

  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import ResourcePopover from '$lib/shell/resources/ResourcePopover.svelte';
  import UsagePopover from '$lib/shell/usage/UsagePopover.svelte';
  import type { CenterPanelId } from '$lib/shell/layout/centerDock';

  interface Props {
    activeId: CenterPanelId;
    onSelect(id: CenterPanelId): void;
  }

  let { activeId, onSelect }: Props = $props();

  const surfaces: Array<{
    id: CenterPanelId;
    label: string;
    icon: typeof MessagesSquare;
  }> = [
    { id: 'session', label: 'Session', icon: MessagesSquare },
    { id: 'editor', label: 'Editor', icon: FileCode2 },
    { id: 'browser', label: 'Browser', icon: Globe2 },
    { id: 'diff', label: 'Diff', icon: GitCompareArrows },
    { id: 'session-library', label: 'Session History', icon: History },
    { id: 'agents', label: 'Agents', icon: Bot }
  ];

  let resourceMount: HTMLDivElement;
  let usageMount: HTMLDivElement;

  /**
   * ResourcePopover and UsagePopover retain ownership of their open state and
   * overlay content. Their legacy text triggers stay mounted as private event
   * endpoints while the kit IconButton is the only visible/focusable control.
   */
  function popoverTrigger(host: HTMLDivElement): HTMLButtonElement | null {
    return host.querySelector<HTMLButtonElement>('.trigger');
  }

  function activatePopover(host: HTMLDivElement): void {
    popoverTrigger(host)?.click();
  }

  onMount(() => {
    for (const host of [resourceMount, usageMount]) {
      const trigger = popoverTrigger(host);
      if (!trigger) continue;
      trigger.tabIndex = -1;
      trigger.setAttribute('aria-hidden', 'true');
    }
  });
</script>

<nav class="center-activity-dock" aria-label="Center surfaces" data-testid="center-activity-dock">
  <div class="surface-group">
    {#each surfaces as surface (surface.id)}
      {@const Icon = surface.icon}
      <span
        class="surface-action"
        class:active={surface.id === activeId}
        data-surface={surface.id}
      >
        <IconButton
          label={surface.label}
          size="sm"
          side="left"
          class={surface.id === activeId
            ? 'surface-button bg-secondary text-foreground'
            : 'surface-button text-muted-foreground'}
          onclick={() => onSelect(surface.id)}
        >
          <Icon class="size-4" strokeWidth={1.7} aria-hidden="true" />
        </IconButton>
      </span>
    {/each}
  </div>

  <div class="utility-group" aria-label="Workspace meters">
    <div class="utility-popover" bind:this={resourceMount}>
      <IconButton
        label="Resources"
        size="sm"
        side="left"
        class="utility-button text-muted-foreground"
        onclick={() => activatePopover(resourceMount)}
      >
        <Cpu class="size-4" strokeWidth={1.7} aria-hidden="true" />
      </IconButton>
      <ResourcePopover />
    </div>
    <div class="utility-popover" bind:this={usageMount}>
      <IconButton
        label="Usage and Stats"
        size="sm"
        side="left"
        class="utility-button text-muted-foreground"
        onclick={() => activatePopover(usageMount)}
      >
        <ChartNoAxesCombined class="size-4" strokeWidth={1.7} aria-hidden="true" />
      </IconButton>
      <UsagePopover />
    </div>
  </div>
</nav>

<style>
  .center-activity-dock {
    position: relative;
    z-index: 60;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    width: 44px;
    height: 100%;
    padding: 8px;
    border-left: 1px solid var(--border);
    background: var(--background);
    color: var(--muted-foreground);
    user-select: none;
  }

  .surface-group,
  .utility-group {
    display: grid;
    gap: 2px;
  }

  .utility-group {
    padding-top: 8px;
    border-top: 1px solid color-mix(in srgb, var(--border) 58%, transparent);
  }

  .surface-action,
  .utility-popover {
    position: relative;
    display: grid;
    width: 28px;
    height: 28px;
    place-items: center;
  }

  .surface-action.active::before {
    position: absolute;
    top: 6px;
    bottom: 6px;
    left: -8px;
    width: 2px;
    border-radius: 0 2px 2px 0;
    background: var(--primary);
    content: '';
  }

  .utility-popover:has(:global(.trigger[aria-expanded='true'])) :global(.utility-button) {
    background: var(--secondary);
    color: var(--foreground);
  }

  /* The popovers keep their own state machine, but their old text triggers are
     no longer controls in the rail. IconButton supplies the 28px target,
     tooltip, focus ring, and accessible name and forwards activation here. */
  .utility-popover :global(aside) {
    position: absolute;
    inset: 0;
    z-index: 2;
    width: 28px;
    height: 28px;
    pointer-events: none;
  }

  .utility-popover :global(.trigger) {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    padding: 0;
    opacity: 0;
    pointer-events: none;
  }

  .utility-popover :global(.usage-popover .card),
  .utility-popover :global(.usage-popover .modal-backdrop) {
    pointer-events: auto;
  }

  /* Open quota details inward and upward from the rail entry. ShellFrame lets
     this anchored card cross the fixed 44px rail cell without clipping it. */
  .utility-popover :global(.usage-popover .card) {
    top: auto;
    right: calc(100% + 12px);
    bottom: 0;
  }
</style>
