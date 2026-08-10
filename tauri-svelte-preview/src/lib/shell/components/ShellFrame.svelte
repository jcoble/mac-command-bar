<script lang="ts">
  /**
   * ShellFrame.svelte — mounts the Gridview root + center Dockview and
   * teleports Svelte-owned content into them. All content is authored in the
   * hidden parking stage below, so dockview NEVER owns app DOM; if the frame
   * fails to mount, content simply stays parked (invisible) and the page's
   * error rail reports it. No backend IO anywhere in this component.
   */
  import 'dockview-core/dist/styles/dockview.css';
  import { onMount, type Snippet } from 'svelte';

  import {
    isCenterPanelId,
    createCenterDock,
    type CenterPanelId,
    type CenterDock,
    type CenterDockSnapshot
  } from '$lib/shell/layout/centerDock';
  import {
    createShellFrame,
    type RegionHeightLimits,
    type RegionWidthLimits,
    type ShellFrame as Frame,
    type ShellRegionId
  } from '$lib/shell/layout/frame';
  import CenterActivityDock from './CenterActivityDock.svelte';

  interface Props {
    /** The left column: the sessions list, and nothing else. */
    sessions: Snippet;
    center: {
      session: Snippet;
      editor: Snippet;
      browser: Snippet;
      diff: Snippet;
      sessionLibrary: Snippet;
      agents: Snippet;
    };
    /** The right column: whichever tool view the icon strip has open. */
    tools: Snippet;
    /** The icon strip on the far right edge, which picks that view. */
    activity: Snippet;
    dock: Snippet;
    onSessionPanelLayout?: () => void;
    /** A center surface came to the front. Fires for Dockview's own start-up
     * announcements too — see the note in `centerDock.ts`. */
    onCenterPanelShown?: (id: string) => void;
    onReady?: (controls: {
      resetLayout: () => void;
      showCenterPanel: (id: string) => void;
      captureCenterLayout: () => CenterDockSnapshot | null;
      restoreCenterLayout: (snapshot: CenterDockSnapshot | null | undefined) => void;
      /** Give one region a width — how a column asks to be folded up or
       * opened out. See `setRegionWidth` in `frame.ts`. */
      setRegionWidth: (id: ShellRegionId, width: number, limits?: RegionWidthLimits) => void;
      /** Give one region a height — how the bottom strip is closed when the
       * Problems list has been moved elsewhere or hidden. See
       * `setRegionHeight` in `frame.ts`. */
      setRegionHeight: (id: ShellRegionId, height: number, limits?: RegionHeightLimits) => void;
      /** Say what a region may be dragged to without moving it. See
       * `setRegionLimits` in `frame.ts`. */
      setRegionLimits: (id: ShellRegionId, limits: RegionWidthLimits) => void;
      /** How wide a region is right now, or null if the frame is gone. */
      regionWidth: (id: ShellRegionId) => number | null;
    }) => void;
    onError?: (message: string) => void;
  }
  let {
    sessions,
    center,
    tools,
    activity,
    dock,
    onSessionPanelLayout,
    onCenterPanelShown,
    onReady,
    onError
  }: Props = $props();

  let gridHost: HTMLElement;
  let sessionsSlot: HTMLElement;
  let centerRegionSlot: HTMLElement;
  let centerSlot: HTMLElement; // holds the center Dockview's own container
  let toolsSlot: HTMLElement;
  let activitySlot: HTMLElement;
  let dockSlot: HTMLElement;
  let sessionSlot: HTMLElement;
  let editorSlot: HTMLElement;
  let browserSlot: HTMLElement;
  let diffSlot: HTMLElement;
  let sessionLibrarySlot: HTMLElement;
  let agentsSlot: HTMLElement;

  let frame: Frame | null = null;
  let centerDock: CenterDock | null = null;
  let activeCenterPanel = $state<CenterPanelId>('session');
  let ready = $state(false);

  function selectCenterPanel(id: CenterPanelId): void {
    centerDock?.activatePanel(id);
  }

  onMount(() => {
    let observer: ResizeObserver | null = null;
    try {
      frame = createShellFrame(gridHost, {
        storage: window.localStorage,
        regions: {
          sessions: sessionsSlot,
          center: centerRegionSlot,
          tools: toolsSlot,
          activity: activitySlot,
          dock: dockSlot
        }
      });
      // Lay out the parent Gridview before the center Dockview restores or
      // builds its panels. Center Dockview's restore path deliberately measures
      // its host before calling `fromJSON`; doing that while the Gridview is
      // still at 0×0 leaves always-rendered panels with stale overlay bounds
      // until the next activation.
      frame.layout(gridHost.clientWidth, gridHost.clientHeight);
      centerDock = createCenterDock(centerSlot, {
        storage: window.localStorage,
        // The session is what you talk to, so it opens on its own on the left;
        // the editor, the browser and the diff are where you look at the
        // result, so they open stacked together on the right. Source control
        // itself is not here at all — it is a view of the tool column — but the
        // changes it shows are, because a diff wants the width of the middle.
        panels: [
          { id: 'session', title: 'Session', element: sessionSlot },
          { id: 'editor', title: 'Editor', element: editorSlot, group: 'display' },
          {
            id: 'browser',
            title: 'Browser',
            element: browserSlot,
            group: 'display',
            renderer: 'onlyWhenVisible'
          },
          {
            id: 'diff',
            title: 'Diff',
            element: diffSlot,
            group: 'display',
            renderer: 'onlyWhenVisible'
          },
          {
            id: 'session-library',
            title: 'Session Library',
            element: sessionLibrarySlot,
            group: 'display'
          },
          { id: 'agents', title: 'Agents', element: agentsSlot, group: 'display' }
        ],
        onPanelLayout: (id) => {
          if (id === 'session') onSessionPanelLayout?.();
        },
        onPanelActivated: (id) => {
          if (isCenterPanelId(id)) activeCenterPanel = id;
          onCenterPanelShown?.(id);
        }
      });
      observer = new ResizeObserver(() => {
        frame?.layout(gridHost.clientWidth, gridHost.clientHeight);
      });
      observer.observe(gridHost);
      ready = true;
      onReady?.({
        resetLayout: () => {
          frame?.resetLayout();
          centerDock?.resetLayout();
        },
        showCenterPanel: (id: string) => centerDock?.activatePanel(id),
        captureCenterLayout: () => centerDock?.captureLayout() ?? null,
        restoreCenterLayout: (snapshot) => centerDock?.restoreLayout(snapshot),
        setRegionWidth: (id, width, limits) => frame?.setRegionWidth(id, width, limits),
        setRegionHeight: (id, height, limits) => frame?.setRegionHeight(id, height, limits),
        setRegionLimits: (id, limits) => frame?.setRegionLimits(id, limits),
        regionWidth: (id) => frame?.regionWidth(id) ?? null
      });
    } catch (error) {
      onError?.(error instanceof Error ? error.message : String(error));
    }

    return () => {
      observer?.disconnect();
      centerDock?.dispose();
      centerDock = null;
      frame?.dispose();
      frame = null;
    };
  });
</script>

<div class="shell-frame" class:ready bind:this={gridHost}></div>

<!-- Parking stage: content lives here until (and unless) dockview claims it, and
     whenever a panel hands it back. Not rendered at all, so nothing parked here
     can be measured — see the note on `.parking-stage` in the styles below. -->
<div class="parking-stage" aria-hidden="true">
  <div class="slot" bind:this={sessionsSlot}>{@render sessions()}</div>
  <div class="slot center-region" bind:this={centerRegionSlot}>
    <CenterActivityDock activeId={activeCenterPanel} onSelect={selectCenterPanel} />
    <div class="center-dock-host" bind:this={centerSlot}></div>
  </div>
  <div class="slot" bind:this={toolsSlot}>{@render tools()}</div>
  <div class="slot" bind:this={activitySlot}>{@render activity()}</div>
  <div class="slot" bind:this={dockSlot}>{@render dock()}</div>
  <div class="slot" bind:this={sessionSlot}>{@render center.session()}</div>
  <div class="slot" bind:this={editorSlot}>{@render center.editor()}</div>
  <div class="slot" bind:this={browserSlot}>{@render center.browser()}</div>
  <div class="slot" bind:this={diffSlot}>{@render center.diff()}</div>
  <div class="slot" bind:this={sessionLibrarySlot}>{@render center.sessionLibrary()}</div>
  <div class="slot" bind:this={agentsSlot}>{@render center.agents()}</div>
</div>

<style>
  .shell-frame {
    height: 100%;
    width: 100%;
    overflow: hidden;
    visibility: hidden; /* anti-flash: revealed when ready */
  }
  .shell-frame.ready {
    visibility: visible;
  }

  /* `display: none`, NOT a 1px hidden box: parked content must be UNMEASURABLE.
     A 1px stage still has a size, and a terminal parked in one measures its host
     at zero — xterm's fit addon clamps that to 2 columns by 1 row and resizes the
     real PTY to it, reflowing the agent's screen. With `display: none` the
     computed height is `auto`, the fit addon proposes NaN and returns without
     touching the grid. Content renders normally the moment it is moved out. */
  .parking-stage {
    display: none;
  }

  /* Teleported slots and hosts must fill whatever cell they land in. */
  .slot,
  :global(.shell-region-host),
  :global(.center-panel-host) {
    height: 100%;
    width: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .center-region {
    display: grid;
    grid-template-rows: 44px minmax(0, 1fr);
    background: var(--color-bg);
  }

  .center-dock-host {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  /* ---- dockview overrides (ported from the old shell's audited CSS) ----
     BOTH theme classes must be targeted where dracula reasserts values.   */
  .shell-frame :global(.shell-grid),
  .shell-frame :global(.shell-center-dock) {
    /* token map: translate dockview vars onto the /next palette */
    --dv-group-view-background-color: var(--color-surface);
    --dv-tabs-and-actions-container-background-color: var(--color-surface);
    --dv-activegroup-visiblepanel-tab-background-color: var(--color-surface);
    --dv-activegroup-hiddenpanel-tab-background-color: var(--color-bg);
    --dv-inactivegroup-visiblepanel-tab-background-color: var(--color-tab-unfocused-surface);
    --dv-inactivegroup-hiddenpanel-tab-background-color: var(--color-bg);
    --dv-tab-divider-color: transparent;
    --dv-tabs-and-actions-container-height: 0px;
    --dv-activegroup-visiblepanel-tab-color: var(--color-text);
    --dv-activegroup-hiddenpanel-tab-color: var(--color-text-2);
    --dv-inactivegroup-visiblepanel-tab-color: var(--color-tab-unfocused-text);
    --dv-inactivegroup-hiddenpanel-tab-color: var(--color-text-2);
    --dv-separator-border: color-mix(in srgb, var(--color-border) 64%, transparent);
    --dv-paneview-active-outline-color: transparent;
    /* NEUTRAL wash only — the teal accent here caused the green drag-flash. */
    --dv-drag-over-background-color: rgba(255, 255, 255, 0.05);
    --dv-drag-over-border-color: transparent;
    /* VS Code-style resize feedback: the divider lights up teal on hover/drag.
       Safe to color, unlike the drag-over wash above — this paints only the
       sash line itself, never the panel content. Delay 0 so it appears the
       moment you grab it, not half a second in. */
    --dv-active-sash-color: var(--color-accent);
    --dv-active-sash-transition-delay: 0.1s;
    --dv-active-sash-transition-duration: 0.05s;
  }

  /* seam flattening */
  .shell-frame :global(.dv-groupview),
  .shell-frame :global(.dv-tabs-and-actions-container),
  .shell-frame :global(.dv-content-container),
  .shell-frame :global(.dv-tabs-container),
  .shell-frame :global(.dv-tab) {
    border-color: transparent;
    box-shadow: none;
  }

  /* Dockview still owns the six panels and their active state, while the
     horizontal surface strip is their only visible navigation. */
  .shell-frame :global(.shell-center-dock .dv-tabs-and-actions-container) {
    display: none;
  }

  /* Every center tab is permanent this slice (a close is undone on the next
     tick to protect the live terminal), so showing a close button would be a
     lie. Remove this rule when closable tabs become real. */
  .shell-frame :global(.shell-center-dock .dv-default-tab-action) {
    display: none;
  }

  /* sizing chain — without this, dockview panels collapse in flex/grid parents */
  .shell-frame :global(.dv-dockview),
  .shell-frame :global(.dv-gridview),
  .shell-frame :global(.dv-grid-view),
  .shell-frame :global(.dv-branch-node),
  .shell-frame :global(.dv-view-container),
  .shell-frame :global(.dv-view) {
    min-width: 0;
    min-height: 0;
  }
</style>
