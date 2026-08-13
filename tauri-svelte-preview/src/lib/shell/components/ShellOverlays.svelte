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

  import ThreadStartHost from '$lib/shell/newSession/ThreadStartHost.svelte';
  import PalettePanel from './PalettePanel.svelte';
  import SettingsHost from './SettingsHost.svelte';
  import SessionBrowserOverlay from '$lib/shell/browser/SessionBrowserOverlay.svelte';
  import AssistanceHost from '$lib/shell/assistance/AssistanceHost.svelte';
  import { invokeCounts } from '$lib/shell/devInvokeCounter.svelte';
  import ResourcePopover from '$lib/shell/resources/ResourcePopover.svelte';
  import UsagePopover from '$lib/shell/usage/UsagePopover.svelte';
  import type { ProblemsLocation } from '$lib/settingsStore.svelte';
  import type {
    ThreadStartProviderConfig,
    ThreadStartRequest
  } from '$lib/shell/newSession/threadStartFlow.ts';
  import {
    RAIL_UTILITY_REQUEST_EVENT,
    RAIL_UTILITY_STATE_EVENT,
    isRailUtilityRequest,
    railUtilityAnchorStyle,
    type RailUtilityAnchor,
    type RailUtilityId,
    type RailUtilityRequest
  } from './railUtilityEvents';

  interface Props {
    /** Put every panel back where it started. */
    onResetLayout: () => void;
    /** Look for agent sessions again. */
    onRescanSessions: () => void | Promise<void>;
    /** Start the session described by the thread-first draft. */
    onStartNewSession: (request: ThreadStartRequest) => void | Promise<void>;
    /** Read configuration only when a new-session draft opens. */
    newSessionProviderConfigs: () => ThreadStartProviderConfig[];
    /** The folders the sessions on the rail are running in, so the project
     * picker knows about projects nobody added by hand. */
    newSessionRoots: string[];
    /** One line describing whatever has gone wrong, or null when all is well. */
    message: string | null;
    /** The user moved the Problems list from the settings dialog, which lives
     * here; the page is what opens or closes the strip along the bottom. */
    onProblemsLocationChange?: (location: ProblemsLocation) => void;
    onSessionBrowserClose?: () => void;
  }
  let {
    onResetLayout,
    onRescanSessions,
    onStartNewSession,
    newSessionProviderConfigs,
    newSessionRoots,
    message,
    onProblemsLocationChange,
    onSessionBrowserClose
  }: Props = $props();

  let settingsHost: { open: () => void; close: () => void } | null = null;
  let newSessionHost: {
    openNewSession: (input?: {
      sessionRoots?: string[];
      providerConfigs?: ThreadStartProviderConfig[];
      projectPath?: string;
    }) => void;
    close(): void;
  } | null = null;
  let resourcePopoverHost: HTMLDivElement | null = null;
  let usagePopoverHost: HTMLDivElement | null = null;
  let resourceAnchor = $state<RailUtilityAnchor | null>(null);
  let usageAnchor = $state<RailUtilityAnchor | null>(null);

  function popoverHost(id: RailUtilityId): HTMLDivElement | null {
    return id === 'resources' ? resourcePopoverHost : usagePopoverHost;
  }

  function popoverTrigger(id: RailUtilityId): HTMLButtonElement | null {
    return popoverHost(id)?.querySelector<HTMLButtonElement>('.trigger') ?? null;
  }

  function publishUtilityState(id: RailUtilityId, trigger: HTMLButtonElement): void {
    window.dispatchEvent(new CustomEvent(RAIL_UTILITY_STATE_EVENT, {
      detail: { id, open: trigger.getAttribute('aria-expanded') === 'true' }
    }));
  }

  function observeUtilityState(id: RailUtilityId): () => void {
    const trigger = popoverTrigger(id);
    if (!trigger) return () => {};
    trigger.tabIndex = -1;
    trigger.setAttribute('aria-hidden', 'true');
    const observer = new MutationObserver(() => publishUtilityState(id, trigger));
    observer.observe(trigger, { attributes: true, attributeFilter: ['aria-expanded'] });
    publishUtilityState(id, trigger);
    return () => observer.disconnect();
  }

  /** Rail controls and overlay owners live in separate shell regions. Anchor
   * the owner first, then toggle its existing trigger after Svelte has applied
   * the fixed screen rectangle. */
  async function openRailUtility(request: RailUtilityRequest): Promise<void> {
    if (request.id === 'resources') resourceAnchor = request.anchor;
    else usageAnchor = request.anchor;
    await tick();
    popoverTrigger(request.id)?.click();
  }

  onMount(() => {
    const handleRailUtilityRequest = (event: Event): void => {
      if (!(event instanceof CustomEvent) || !isRailUtilityRequest(event.detail)) return;
      void openRailUtility(event.detail);
    };
    window.addEventListener(RAIL_UTILITY_REQUEST_EVENT, handleRailUtilityRequest);
    const stopObservingResources = observeUtilityState('resources');
    const stopObservingUsage = observeUtilityState('usage');
    return () => {
      window.removeEventListener(RAIL_UTILITY_REQUEST_EVENT, handleRailUtilityRequest);
      stopObservingResources();
      stopObservingUsage();
    };
  });

  /** Open the settings dialog from outside — the gear on the activity bar is
   * over in the left column, and the dialog lives here. Same shape as
   * `SettingsHost`'s own `open()`, one layer out. */
  export function openSettings(): void {
    settingsHost?.open();
  }

  /** Open the thread-first new-session pane from outside. Both ways in reach the same
   * instance: the "New session" button in the sessions column, and the palette
   * command the page registers. */
  export function openNewSession(projectPath?: string): void {
    newSessionHost?.openNewSession({
      sessionRoots: newSessionRoots,
      providerConfigs: newSessionProviderConfigs(),
      projectPath
    });
  }
</script>

<PalettePanel
  {onResetLayout}
  {onRescanSessions}
  onOpenSettings={() => settingsHost?.open()}
/>
<SettingsHost bind:this={settingsHost} {onProblemsLocationChange} />
<ThreadStartHost bind:this={newSessionHost} onStart={onStartNewSession} />
<!-- The session's own browser, over the whole window including the sessions
     column. It reads the active session itself and takes no props. -->
<SessionBrowserOverlay onClose={onSessionBrowserClose} />
<AssistanceHost />
<div
  bind:this={resourcePopoverHost}
  class="rail-popover-host"
  style={resourceAnchor ? railUtilityAnchorStyle(resourceAnchor) : undefined}
>
  <ResourcePopover />
</div>
<div
  bind:this={usagePopoverHost}
  class="rail-popover-host"
  style={usageAnchor ? railUtilityAnchorStyle(usageAnchor) : undefined}
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
     the rail's fixed-width Gridview cell. The rail supplies the exact button
     rectangle before each toggle. */
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
    bottom: 8px;
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
