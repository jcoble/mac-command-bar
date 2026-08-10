<script lang="ts">
  /**
   * CenterActivityDock.svelte — the center Dockview's permanent navigation.
   * Dockview remains the layout and activation state machine; this component
   * only presents that state as a compact horizontal strip above the center.
   */
  import {
    Bot,
    FileCode2,
    GitCompareArrows,
    Globe2,
    History,
    SquareTerminal
  } from '@lucide/svelte';

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
        <Icon size={16} strokeWidth={1.7} aria-hidden="true" />
        <span class="dock-label">{surface.label}</span>
      </button>
    {/each}
  </div>
</nav>

<style>
  .center-activity-dock {
    position: relative;
    z-index: 4;
    display: flex;
    align-items: center;
    width: 100%;
    height: 44px;
    min-width: 0;
    padding: 4px 8px;
    border-bottom: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
    background: var(--color-surface);
    color: var(--color-text-3);
    user-select: none;
  }

  .surface-group {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .surface-group::-webkit-scrollbar {
    display: none;
  }

  .dock-button {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    flex: 0 0 auto;
    height: 36px;
    padding: 0 11px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text-3);
    font: inherit;
    cursor: pointer;
    transition:
      color 140ms ease,
      background-color 140ms ease;
  }

  .dock-button:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .dock-button.active {
    background: var(--color-selected);
    color: var(--color-text);
  }

  .dock-button.active::after {
    position: absolute;
    right: 8px;
    bottom: 0;
    left: 8px;
    height: 2px;
    border-radius: var(--radius-pill) var(--radius-pill) 0 0;
    background: var(--color-accent);
    content: '';
  }

  .dock-label {
    font-size: 12px;
    font-weight: 540;
    letter-spacing: 0.005em;
    line-height: 1;
    white-space: nowrap;
  }

  .dock-button:focus-visible {
    outline: 2px solid var(--color-focus-solid);
    outline-offset: -3px;
  }

  @media (prefers-reduced-motion: reduce) {
    .dock-button {
      transition: none;
    }
  }
</style>
