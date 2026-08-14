<script lang="ts">
  /**
   * ShellOverlays.svelte — everything that floats above the /next shell rather
   * than living in a region: the command palette, the settings dialog, the
   * message strip, and the development-only backend call counter.
   *
   * No backend call and no state of its own beyond the settings handle. The
   * palette is mounted exactly once (two of them would answer the same keyboard
   * shortcut and would fight over the seeded actions), which is why it lives
   * here and not inside a panel.
   */
  import { onMount, tick } from 'svelte';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

  import PalettePanel from './PalettePanel.svelte';
  import SettingsHost from './SettingsHost.svelte';
  import AssistanceHost from '$lib/shell/assistance/AssistanceHost.svelte';
  import { invokeCounts } from '$lib/shell/devInvokeCounter.svelte';
  import ResourceManagerPanel from '$lib/shell/resources/ResourceManagerPanel.svelte';
  import ResourcePopover from '$lib/shell/resources/ResourcePopover.svelte';
  import { resourceManagerState } from '$lib/shell/resources/resourceSampleStore.svelte';
  import UsagePopover from '$lib/shell/usage/UsagePopover.svelte';
  import type { ProblemsLocation } from '$lib/settingsStore.svelte';
  import {
    utilityAnchorStyle,
    type UtilityAnchor,
    type UtilityId
  } from './utilityStrip';

  interface Props {
    /** Put every panel back where it started. */
    onResetLayout: () => void;
    /** Look for agent sessions again. */
    onRescanSessions: () => void | Promise<void>;
    /** One line describing whatever has gone wrong, or null when all is well. */
    message: string | null;
    /** The user moved the Problems list from the settings dialog, which lives
     * here; the page is what opens or closes the strip along the bottom. */
    onProblemsLocationChange?: (location: ProblemsLocation) => void;
    /** One of the two bottom-strip surfaces opened or closed. The strip draws
     * its own button as on or off from this. */
    onUtilityStateChange?: (id: UtilityId, open: boolean) => void;
  }
  let {
    onResetLayout,
    onRescanSessions,
    message,
    onProblemsLocationChange,
    onUtilityStateChange
  }: Props = $props();

  let settingsHost: { open: () => void; close: () => void } | null = null;
  let resourcePopoverHost: HTMLDivElement | null = null;
  let usagePopoverHost: HTMLDivElement | null = null;
  let resourceAnchor = $state<UtilityAnchor | null>(null);
  let usageAnchor = $state<UtilityAnchor | null>(null);

  function popoverHost(id: UtilityId): HTMLDivElement | null {
    return id === 'resources' ? resourcePopoverHost : usagePopoverHost;
  }

  function popoverTrigger(id: UtilityId): HTMLButtonElement | null {
    return popoverHost(id)?.querySelector<HTMLButtonElement>('.trigger') ?? null;
  }

  /** Watch one surface's own trigger and report whether it is open, so the
   * bottom strip's button can read as on without owning that state itself. */
  function observeUtilityState(id: UtilityId): () => void {
    const trigger = popoverTrigger(id);
    if (!trigger) return () => {};
    trigger.tabIndex = -1;
    trigger.setAttribute('aria-hidden', 'true');
    const report = (): void =>
      onUtilityStateChange?.(id, trigger.getAttribute('aria-expanded') === 'true');
    const observer = new MutationObserver(report);
    observer.observe(trigger, { attributes: true, attributeFilter: ['aria-expanded'] });
    report();
    return () => observer.disconnect();
  }

  onMount(() => {
    const stopObservingResources = observeUtilityState('resources');
    const stopObservingUsage = observeUtilityState('usage');
    return () => {
      stopObservingResources();
      stopObservingUsage();
    };
  });

  /**
   * Open one of the two bottom-strip surfaces from outside.
   *
   * The strip is inside the layout and these surfaces are mounted out here,
   * above it, so a card cannot be clipped by the column it was opened from.
   * The anchor is applied FIRST and the surface's own trigger clicked after
   * Svelte has written the rectangle, or the card opens where the last one was.
   */
  export async function openUtility(id: UtilityId, anchor: UtilityAnchor): Promise<void> {
    if (id === 'resources') resourceAnchor = anchor;
    else usageAnchor = anchor;
    await tick();
    popoverTrigger(id)?.click();
  }

  /** Open the settings dialog from outside — the gear on the activity bar is
   * over in the left column, and the dialog lives here. Same shape as
   * `SettingsHost`'s own `open()`, one layer out. */
  export function openSettings(): void {
    settingsHost?.open();
  }
</script>

<PalettePanel
  {onResetLayout}
  {onRescanSessions}
  onOpenSettings={() => settingsHost?.open()}
/>
<SettingsHost bind:this={settingsHost} {onProblemsLocationChange} />
<!-- The session's own browser, over the whole window including the sessions
     column. It reads the active session itself and takes no props. -->
<AssistanceHost />
<div
  bind:this={resourcePopoverHost}
  class="rail-popover-host"
  style={resourceAnchor ? utilityAnchorStyle(resourceAnchor) : undefined}
>
  <ResourcePopover />
</div>
<!-- The Resource Manager is a window-anchored sheet rather than a card beside
     the button, so it is mounted plainly rather than in a popover host. -->
{#if resourceManagerState.open}
  <ResourceManagerPanel />
{/if}
<div
  bind:this={usagePopoverHost}
  class="rail-popover-host"
  style={usageAnchor ? utilityAnchorStyle(usageAnchor) : undefined}
>
  <UsagePopover />
</div>
{#if message}
  <!-- Something went wrong, said once, along the bottom edge. Announced to
       screen readers, and see-through to the mouse so it can never swallow a
       click meant for the shell underneath. -->
  <footer
    role="alert"
    class="pointer-events-none absolute bottom-2 left-1/2 flex -translate-x-1/2 items-center
           gap-2 rounded-lg bg-destructive/10 px-3 py-2 shadow-[var(--shadow-sm)]
           text-[13px] leading-[1.4] text-destructive"
  >
    <TriangleAlert class="size-3.5 shrink-0" aria-hidden="true" />
    <span>{message}</span>
  </footer>
{/if}

{#if import.meta.env.DEV}
  <footer class="invoke-counter">invokes: {invokeCounts.total} (+{invokeCounts.input} input)</footer>
{/if}

<style>
  /* Resource and quota state stay with their existing components, but the
     mounts live above Dockview so inward-opening content cannot be clipped by
     the right column's Gridview cell. The bottom strip supplies the exact
     button rectangle before each toggle. */
  .rail-popover-host {
    position: fixed;
    z-index: 70;
    width: 32px;
    height: 32px;
    pointer-events: none;
  }

  .rail-popover-host :global(aside) {
    position: absolute;
    inset: 0;
    width: 32px;
    height: 32px;
    pointer-events: none;
  }

  .rail-popover-host :global(.trigger) {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    padding: 0;
    opacity: 0;
    pointer-events: none;
  }

  .rail-popover-host :global(.usage-popover .card),
  .rail-popover-host :global(.modal-backdrop),
  .rail-popover-host :global(.modal-surface) {
    pointer-events: auto;
  }

  .rail-popover-host :global(.usage-popover .card) {
    top: auto;
    right: calc(100% + 12px);
    bottom: 0;
  }

  /* Development-only readout of how many backend calls the shell has made.
     Deliberately not part of the shared component set: it is a debugging
     instrument, not chrome, and it never ships to a user. */
  .invoke-counter {
    position: absolute;
    /* Clear of the Resources and Usage strip along the bottom of the right
       column, which is 28px tall — otherwise this sits on top of it and spoils
       every screenshot taken in development. */
    bottom: 36px;
    right: 10px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--color-surface) 92%, var(--color-bg));
    color: var(--color-text-3);
    font-family: ui-monospace, Menlo, monospace;
    font-size: 12px;
    padding: 3px 8px;
    pointer-events: none;
  }
</style>
