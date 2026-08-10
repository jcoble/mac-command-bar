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
  import type { CenterPanelId } from '$lib/shell/layout/centerDock';
  import {
    RAIL_UTILITY_REQUEST_EVENT,
    RAIL_UTILITY_STATE_EVENT,
    isRailUtilityState,
    railUtilityRequest,
    type RailUtilityId
  } from './railUtilityEvents';

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

  let utilityOpen = $state<Record<RailUtilityId, boolean>>({ resources: false, usage: false });

  /** The overlay owners live outside Dockview. Send them both the requested
   * utility and this exact button rectangle so the quota card opens beside the
   * rail instead of inside its clipped 44px Gridview cell. */
  function requestUtility(id: RailUtilityId, event: MouseEvent): void {
    if (!(event.currentTarget instanceof HTMLElement)) return;
    window.dispatchEvent(new CustomEvent(RAIL_UTILITY_REQUEST_EVENT, {
      detail: railUtilityRequest(id, event.currentTarget.getBoundingClientRect())
    }));
  }

  onMount(() => {
    const handleUtilityState = (event: Event): void => {
      if (!(event instanceof CustomEvent) || !isRailUtilityState(event.detail)) return;
      utilityOpen[event.detail.id] = event.detail.open;
    };
    window.addEventListener(RAIL_UTILITY_STATE_EVENT, handleUtilityState);
    return () => window.removeEventListener(RAIL_UTILITY_STATE_EVENT, handleUtilityState);
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
        aria-current={surface.id === activeId ? 'page' : undefined}
      >
        <IconButton
          label={surface.label}
          size="default"
          variant={surface.id === activeId ? 'secondary' : 'ghost'}
          side="left"
          class={surface.id === activeId
            ? 'surface-button text-foreground'
            : 'surface-button text-muted-foreground'}
          onclick={() => onSelect(surface.id)}
        >
          <Icon class="size-5" strokeWidth={1.7} aria-hidden="true" />
        </IconButton>
      </span>
    {/each}
  </div>

  <div class="utility-group" aria-label="Workspace meters">
    <div class="utility-action" class:active={utilityOpen.resources}>
      <IconButton
        label="Resources"
        size="default"
        variant={utilityOpen.resources ? 'secondary' : 'ghost'}
        side="left"
        class="utility-button text-muted-foreground"
        onclick={(event) => requestUtility('resources', event)}
      >
        <Cpu class="size-5" strokeWidth={1.7} aria-hidden="true" />
      </IconButton>
    </div>
    <div class="utility-action" class:active={utilityOpen.usage}>
      <IconButton
        label="Usage and Stats"
        size="default"
        variant={utilityOpen.usage ? 'secondary' : 'ghost'}
        side="left"
        class="utility-button text-muted-foreground"
        onclick={(event) => requestUtility('usage', event)}
      >
        <ChartNoAxesCombined class="size-5" strokeWidth={1.7} aria-hidden="true" />
      </IconButton>
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
    padding: 8px 6px;
    border-left: 1px solid var(--border);
    background: var(--background);
    color: var(--muted-foreground);
    user-select: none;
  }

  .surface-group,
  .utility-group {
    display: grid;
    gap: 8px;
  }

  .utility-group {
    padding-top: 8px;
    border-top: 1px solid color-mix(in srgb, var(--border) 58%, transparent);
  }

  .surface-action,
  .utility-action {
    position: relative;
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
  }

  .surface-action.active::before {
    position: absolute;
    top: 8px;
    bottom: 8px;
    left: -6px;
    width: 3px;
    border-radius: 0 2px 2px 0;
    background: var(--primary);
    content: '';
  }

  .surface-action.active :global(.surface-button),
  .utility-action.active :global(.utility-button) {
    color: var(--foreground);
  }
</style>
