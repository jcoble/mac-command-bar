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

  import { createCenterDock, type CenterDock } from '$lib/shell/layout/centerDock';
  import { createShellFrame, type ShellFrame as Frame } from '$lib/shell/layout/frame';

  interface Props {
    rail: Snippet;
    center: { session: Snippet; editor: Snippet; git: Snippet; browser: Snippet };
    context: Snippet;
    dock: Snippet;
    onSessionPanelLayout?: () => void;
    /** A center tab came to the front. Fires for the dock's own start-up
     * announcements too — see the note in `centerDock.ts`. */
    onCenterPanelShown?: (id: string) => void;
    onReady?: (controls: {
      resetLayout: () => void;
      showCenterPanel: (id: string) => void;
    }) => void;
    onError?: (message: string) => void;
  }
  let {
    rail,
    center,
    context,
    dock,
    onSessionPanelLayout,
    onCenterPanelShown,
    onReady,
    onError
  }: Props = $props();

  let gridHost: HTMLElement;
  let railSlot: HTMLElement;
  let centerSlot: HTMLElement; // holds the center Dockview's own container
  let contextSlot: HTMLElement;
  let dockSlot: HTMLElement;
  let sessionSlot: HTMLElement;
  let editorSlot: HTMLElement;
  let gitSlot: HTMLElement;
  let browserSlot: HTMLElement;

  let frame: Frame | null = null;
  let centerDock: CenterDock | null = null;
  let ready = $state(false);

  onMount(() => {
    let observer: ResizeObserver | null = null;
    try {
      frame = createShellFrame(gridHost, {
        storage: window.localStorage,
        regions: { rail: railSlot, center: centerSlot, context: contextSlot, dock: dockSlot }
      });
      centerDock = createCenterDock(centerSlot, {
        storage: window.localStorage,
        panels: [
          { id: 'session', title: 'Session', element: sessionSlot },
          { id: 'editor', title: 'Editor', element: editorSlot },
          { id: 'git', title: 'Source control', element: gitSlot },
          { id: 'browser', title: 'Browser', element: browserSlot }
        ],
        onPanelLayout: (id) => {
          if (id === 'session') onSessionPanelLayout?.();
        },
        onPanelActivated: (id) => onCenterPanelShown?.(id)
      });
      frame.layout(gridHost.clientWidth, gridHost.clientHeight);
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
        showCenterPanel: (id: string) => centerDock?.activatePanel(id)
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
  <div class="slot" bind:this={railSlot}>{@render rail()}</div>
  <div class="slot slot-center-dock" bind:this={centerSlot}></div>
  <div class="slot" bind:this={contextSlot}>{@render context()}</div>
  <div class="slot" bind:this={dockSlot}>{@render dock()}</div>
  <div class="slot" bind:this={sessionSlot}>{@render center.session()}</div>
  <div class="slot" bind:this={editorSlot}>{@render center.editor()}</div>
  <div class="slot" bind:this={gitSlot}>{@render center.git()}</div>
  <div class="slot" bind:this={browserSlot}>{@render center.browser()}</div>
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

  /* ---- dockview overrides (ported from the old shell's audited CSS) ----
     BOTH theme classes must be targeted where dracula reasserts values.   */
  .shell-frame :global(.shell-grid),
  .shell-frame :global(.shell-center-dock) {
    /* token map: translate dockview vars onto the /next palette */
    --dv-group-view-background-color: #101014;
    --dv-tabs-and-actions-container-background-color: #101014;
    --dv-activegroup-visiblepanel-tab-background-color: #17171d;
    --dv-activegroup-hiddenpanel-tab-background-color: #101014;
    --dv-inactivegroup-visiblepanel-tab-background-color: #14141a;
    --dv-inactivegroup-hiddenpanel-tab-background-color: #101014;
    --dv-tab-divider-color: transparent;
    --dv-activegroup-visiblepanel-tab-color: #d8d8e0;
    --dv-activegroup-hiddenpanel-tab-color: #6d6d7d;
    --dv-inactivegroup-visiblepanel-tab-color: #9a9aa8;
    --dv-inactivegroup-hiddenpanel-tab-color: #6d6d7d;
    --dv-separator-border: #22222c;
    --dv-paneview-active-outline-color: transparent;
    /* NEUTRAL wash only — the teal accent here caused the green drag-flash. */
    --dv-drag-over-background-color: rgba(255, 255, 255, 0.05);
    --dv-drag-over-border-color: transparent;
    /* VS Code-style resize feedback: the divider lights up teal on hover/drag.
       Safe to color, unlike the drag-over wash above — this paints only the
       sash line itself, never the panel content. Delay 0 so it appears the
       moment you grab it, not half a second in. */
    --dv-active-sash-color: #4bf3c8;
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
