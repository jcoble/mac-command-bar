<script lang="ts">
  /**
   * CenterActivityDock.svelte — the center Dockview's permanent navigation.
   * Dockview remains the layout and activation state machine; this component
   * only presents that state as a compact rail on the center region's right.
   */
  import {
    Bot,
    ChartNoAxesCombined,
    Cpu,
    FileCode2,
    GitCompareArrows,
    Globe2,
    History,
    SquareTerminal
  } from '@lucide/svelte';

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
    icon: typeof SquareTerminal;
    shortcut?: string;
  }> = [
    { id: 'session', label: 'Session', icon: SquareTerminal },
    { id: 'editor', label: 'Editor', icon: FileCode2 },
    { id: 'browser', label: 'Browser', icon: Globe2 },
    { id: 'diff', label: 'Diff', icon: GitCompareArrows },
    { id: 'session-library', label: 'Session History', icon: History },
    { id: 'agents', label: 'Agents', icon: Bot }
  ];

  const tooltip = (label: string, shortcut?: string): string =>
    shortcut ? `${label} (${shortcut})` : label;
</script>

<nav class="center-activity-dock" aria-label="Center surfaces" data-testid="center-activity-dock">
  <div class="surface-group">
    {#each surfaces as surface (surface.id)}
      {@const Icon = surface.icon}
      <button
        type="button"
        class="dock-button"
        class:active={surface.id === activeId}
        data-surface={surface.id}
        data-tooltip={tooltip(surface.label, surface.shortcut)}
        title={tooltip(surface.label, surface.shortcut)}
        aria-label={tooltip(surface.label, surface.shortcut)}
        aria-current={surface.id === activeId ? 'page' : undefined}
        onclick={() => onSelect(surface.id)}
      >
        <Icon size={19} strokeWidth={1.6} aria-hidden="true" />
      </button>
    {/each}
  </div>

  <div class="utility-group" aria-label="Workspace meters">
    <div class="utility-action" data-tooltip="Resources">
      <Cpu size={19} strokeWidth={1.6} aria-hidden="true" />
      <ResourcePopover />
    </div>
    <div class="utility-action" data-tooltip="Usage">
      <ChartNoAxesCombined size={19} strokeWidth={1.6} aria-hidden="true" />
      <UsagePopover />
    </div>
  </div>
</nav>

<style>
  .center-activity-dock {
    position: relative;
    z-index: 4;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    width: 48px;
    height: 100%;
    padding: 8px 4px;
    border-left: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
    background: var(--color-surface);
    color: var(--color-text-3);
    user-select: none;
  }

  .surface-group,
  .utility-group {
    display: grid;
    gap: 4px;
  }

  .utility-group {
    padding-top: 8px;
    border-top: 1px solid color-mix(in srgb, var(--color-border) 58%, transparent);
  }

  .dock-button,
  .utility-action {
    position: relative;
    display: grid;
    width: 40px;
    height: 40px;
    place-items: center;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text-3);
  }

  .dock-button {
    padding: 0;
    font: inherit;
    cursor: pointer;
    transition:
      color 140ms ease,
      background-color 140ms ease,
      transform 140ms ease;
  }

  .dock-button:hover,
  .utility-action:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .dock-button:active {
    transform: scale(0.96);
  }

  .dock-button.active {
    background: var(--color-selected);
    color: var(--color-text);
  }

  .dock-button.active::before {
    position: absolute;
    top: 9px;
    bottom: 9px;
    left: -4px;
    width: 2px;
    border-radius: 0 var(--radius-pill) var(--radius-pill) 0;
    background: var(--color-accent);
    content: '';
  }

  .dock-button:focus-visible,
  .utility-action:has(:global(.trigger:focus-visible)) {
    outline: 2px solid var(--color-focus-solid);
    outline-offset: -3px;
  }

  .dock-button::after,
  .utility-action::after {
    position: absolute;
    top: 50%;
    right: calc(100% + 10px);
    z-index: 12;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    background: var(--color-elevated);
    box-shadow: var(--shadow-sm);
    color: var(--color-text);
    content: attr(data-tooltip);
    font-size: 12px;
    line-height: 1.2;
    opacity: 0;
    pointer-events: none;
    transform: translate(4px, -50%);
    transition:
      opacity 120ms ease,
      transform 120ms ease;
    white-space: nowrap;
  }

  .dock-button:hover::after,
  .dock-button:focus-visible::after,
  .utility-action:hover::after,
  .utility-action:focus-within::after {
    opacity: 1;
    transform: translate(0, -50%);
  }

  .utility-action :global(aside) {
    position: absolute;
    inset: 0;
    z-index: 2;
    width: 40px;
    height: 40px;
  }

  .utility-action :global(.trigger) {
    position: absolute;
    inset: 0;
    width: 40px;
    height: 40px;
    padding: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: transparent;
    font-size: 0;
  }

  .utility-action :global(.trigger span) {
    display: none;
  }

  .utility-action:has(:global(.trigger[aria-expanded='true'])) {
    background: var(--color-selected);
    color: var(--color-text);
  }

  /* Usage used to open downward from the window's top-right. In the dock it
     opens inward and upward, staying inside the center region. */
  .utility-action :global(.usage-popover .card) {
    top: auto;
    right: calc(100% + 10px);
    bottom: 0;
  }

  @media (prefers-reduced-motion: reduce) {
    .dock-button,
    .dock-button::after,
    .utility-action::after {
      transition: none;
    }
  }
</style>
