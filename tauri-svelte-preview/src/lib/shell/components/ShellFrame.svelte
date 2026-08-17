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
    createCenterDock,
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

  interface Props {
    /** The left column: the sessions list, and nothing else. */
    sessions: Snippet;
    center: {
      session: Snippet;
      editor: Snippet;
      diff: Snippet;
    };
    /** The small tabs in the center pane's upper-right corner. */
    centerTabs: Snippet;
    /** The right column: its tab strip, the open panel, and the bottom strip. */
    tools: Snippet;
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
      /** Put the bottom dock in the grid or take it out. A shut strip is
       * removed rather than shortened, because a region of zero height still
       * keeps its divider and the room around it. See `setDockPresent` in
       * `frame.ts`. */
      setDockPresent: (present: boolean) => void;
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
    centerTabs,
    tools,
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
  let dockSlot: HTMLElement;
  let sessionSlot: HTMLElement;
  let editorSlot: HTMLElement;
  let diffSlot: HTMLElement;

  let frame: Frame | null = null;
  let centerDock: CenterDock | null = null;
  let ready = $state(false);

  onMount(() => {
    let observer: ResizeObserver | null = null;
    try {
      frame = createShellFrame(gridHost, {
        storage: window.localStorage,
        regions: {
          sessions: sessionsSlot,
          center: centerRegionSlot,
          tools: toolsSlot,
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
        // All three stacked, one showing at a time, opening on the session. The
        // corner tabs are what move between them. Source control itself is not
        // here at all — it is a panel of the right column — but the changes it
        // shows are, because a diff wants the width of the middle.
        panels: [
          { id: 'session', title: 'Session', element: sessionSlot },
          { id: 'editor', title: 'Editor', element: editorSlot },
          {
            id: 'diff',
            title: 'Diff',
            element: diffSlot,
            renderer: 'onlyWhenVisible'
          }
        ],
        onPanelLayout: (id) => {
          if (id === 'session') onSessionPanelLayout?.();
        },
        onPanelActivated: (id) => {
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
        setDockPresent: (present) => frame?.setDockPresent(present),
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
    <div class="center-tabs">{@render centerTabs()}</div>
    <div class="center-dock-host" bind:this={centerSlot}></div>
  </div>
  <div class="slot tools-region" bind:this={toolsSlot}>{@render tools()}</div>
  <div class="slot" bind:this={dockSlot}>{@render dock()}</div>
  <div class="slot" bind:this={sessionSlot}>{@render center.session()}</div>
  <div class="slot" bind:this={editorSlot}>{@render center.editor()}</div>
  <div class="slot" bind:this={diffSlot}>{@render center.diff()}</div>
</div>

<style>
  .shell-frame {
    height: 100%;
    width: 100%;
    overflow: hidden;
    /* Matches the gutter each panel already keeps, so the cards sit inside the
       window rather than running off its edges. */
    padding: 3px;
    background: var(--color-bg);
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

  /* The pill tabs are laid OVER the pane, not in a row above it. A row charged
     every surface the same strip of height whether or not anyone was looking at
     it, and on the Editor it sat above a bar the editor already has. Floating
     costs nothing while the group is hidden, which is most of the time. */
  .center-region {
    position: relative;
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    /* Transparent on purpose: the card behind it paints the surface and its
       gradient, and an opaque fill here would cover both. */
    background: transparent;
    /* What the pill group reads to know whether it is wanted. Both are plain
       inherited values, which is how a rule here reaches a class inside a
       component this file cannot name. */
    --center-pills-reveal: 0;
    --center-pills-events: none;
  }

  /* The pointer anywhere in the centre pane asks for the group. HOVER ONLY.
     Keyboard focus used to be included here as well, and it was wrong: focus
     inside the pane means the composer or the editor, so the group stayed lit
     for as long as you were typing. The group answers for its own focus — see
     `.center-pills:focus-within` in `CenterCornerTabs`. */
  .center-region:hover {
    --center-pills-reveal: 1;
    --center-pills-events: auto;
  }

  .center-tabs {
    position: absolute;
    top: 0;
    right: 0;
    left: 0;
    /* Above the dock's own content, below anything a dialog puts on screen. */
    z-index: 12;
    display: flex;
    min-width: 0;
    justify-content: flex-end;
    /* The strip spans the pane only so the group can hang off its right edge;
       it must not swallow clicks meant for the surface underneath. The pills
       themselves take the pointer back when they are on screen. */
    pointer-events: none;
  }

  .tools-region {
    min-width: 0;
    min-height: 0;
    /* Same reason as `.center-region`. This also drops a stray `--background`,
       which is a different token from the `--color-*` set the shell uses. */
    background: transparent;
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
    --dv-border-radius: 8px;
    /* Transparent, not the surface colour: the card underneath already paints
       the surface and the gradient down its top edge, and an opaque group here
       covered both from the tab strip down. */
    --dv-group-view-background-color: transparent;
    --dv-tabs-and-actions-container-background-color: transparent;
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
    /* Panels read as cards laid on the backdrop rather than as regions cut out
       of one sheet: each section is pulled in a few pixels so the background
       shows through as a gutter, and the separator rules are dropped since the
       gap already does that work. */
    --dv-separator-border: transparent;
    --dv-active-sash-transition-delay: 0.1s;
    --dv-active-sash-transition-duration: 0.05s;
  }

  /* The gutter itself. The group is a plain flex column inside the splitview's
     absolutely placed box, so pulling it in on every side and taking the same
     amount off its height leaves the backdrop showing between sections. */
  /* ─── The cards ──────────────────────────────────────────────────────────
     THE one rule that shapes the shell's panels. `.shell-region-host` is the
     box each region's content is mounted into — sessions, centre, dock, tools
     — so rounding and insetting it here is what makes all four read as cards
     laid on the backdrop. Nothing else needs a radius: this clips its content.

     Not the dockview classes. `.dv-groupview` exists only inside the centre's
     own nested dock, so styling it shaped one panel out of four. */
  .shell-frame :global(.shell-region-host) {
    /* The inset is a pair of MAXIMUMS, not a width and a height. Dockview
       writes `width: 100%; height: 100%` on this element as an inline style,
       which outranks any rule here, so sizes set the ordinary way were thrown
       away while the margin still moved the box: every card overhung its cell
       by three pixels on the right and along the bottom, and the cell's
       `overflow: hidden` shaved those two edges off square — taking two of the
       four rounded corners with them. Nothing inline competes with a maximum,
       so this holds, and the gutter is the same six pixels on all four sides. */
    max-width: calc(100% - 6px);
    max-height: calc(100% - 6px);
    margin: 3px;
    border-radius: var(--radius-sm);
    overflow: hidden;
    /* A card is a lit surface, not a flat fill: each one carries a little more
       light along its top edge, falling off within the first couple of hundred
       pixels. Without it the panels read as holes cut in the backdrop. */
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.016), rgba(255, 255, 255, 0) 140px),
      var(--color-surface);
  }

  /* The middle is where the eye lands, so it gets the stronger lift and the
     side columns stay quieter. */
  .shell-frame :global(.shell-region-host-center) {
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.028), rgba(255, 255, 255, 0) 200px),
      var(--color-surface);
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

  /* Dockview still owns the three panels and their active state, while the
     center pane's corner tabs are their only visible navigation. */
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

  /* A grid cell never scrolls; the panel inside it does.
   *
   * dockview leaves `.dv-view` scrollable, and on a Mac set to show scroll bars
   * always, each one drew a horizontal bar along its bottom edge. Three cells
   * side by side read as one band above the status bar, and because the bar is
   * part of the cell, the panel inside — sized at `height: 100%` — stopped
   * seventeen pixels short of the status bar and took its own scrollbar with it.
   * Nothing in a grid cell is meant to be reached by scrolling the cell. */
  .shell-frame :global(.dv-view) {
    overflow: hidden;
  }
</style>
