<script lang="ts">
  /**
   * ShellSidebar.svelte — the tool column on the right of the shell: whichever
   * one view the icon strip on the far right edge has open. Each view holds its
   * own stack of collapsible panes (one dockview Paneview per view) and
   * remembers its own sizes, so arranging Explorer cannot disturb how Worktrees
   * was left.
   *
   * The icon strip itself is NOT in here — it is a region of its own on the
   * outer edge, so it stays exactly one icon wide however this column is
   * resized. Which view is open is still decided here, and reported out so the
   * strip can light up the icon for it.
   *
   * Every view's container stays mounted for the whole session; switching views
   * only flips which one is displayed. Nothing re-parents, so a terminal or a
   * scroll position inside a view survives being looked away from.
   *
   * A view's pane stack is built the first time that view is shown AND its
   * container has a real size — a Paneview restored into a 0 x 0 container lays
   * every remembered pane out at nothing, and a container that is not being
   * displayed measures exactly that. So the same ResizeObserver that keeps a
   * view sized is also what starts it.
   *
   * Same teleport contract as ShellFrame: every pane body is authored in the
   * hidden parking stage below and MOVED into a dockview-owned host, so dockview
   * never renders or destroys app DOM.
   *
   * No backend IO here, and this component never loads anything itself. What it
   * does do is REPORT: it hands the page controls for the column, and it says
   * whenever source control or the context cards become visible or stop being
   * visible, which is what decides when either may read anything (see
   * `panelActivation.ts`). "Visible" means both things that have to be true —
   * that view is the open one, and its pane is not folded away.
   */
  import 'dockview-core/dist/styles/dockview.css';
  import { onMount } from 'svelte';
  // dockview re-exports its disposable under a prefixed name to avoid clashing
  // with the one most codebases already have; the plain `IDisposable` is not
  // part of its public surface.
  import type { DockviewIDisposable } from 'dockview-core';

  import { createPaneStack, type PaneStack } from '$lib/shell/layout/paneStack';
  import {
    DEFAULT_SIDEBAR_VIEW,
    SIDEBAR_VIEWS,
    clearActiveView,
    readActiveView,
    viewPanesKey,
    writeActiveView,
    type SidebarViewId
  } from '$lib/shell/layout/sidebarViews';

  import ContextPanel from './ContextPanel.svelte';
  import ExplorerPanel from './ExplorerPanel.svelte';
  import GitPanel from './GitPanel.svelte';
  import PanelPlaceholder from './PanelPlaceholder.svelte';

  /** The view "Show source control" opens. */
  const SOURCE_CONTROL: SidebarViewId = 'source-control';

  /** The views whose visibility gates a loader: each reads what it shows only
   * while the user can actually see it. */
  const GATED_VIEWS: readonly SidebarViewId[] = ['source-control', 'context'];

  /** The panes each view opens with. One apiece today; the stack is what lets a
   * view grow a second section (an Outline under Files, a graph under Source
   * control) without any of this changing shape. */
  const PANES: Record<SidebarViewId, { id: string; title: string }> = {
    explorer: { id: 'files', title: 'Files' },
    'source-control': { id: 'source-control', title: 'Source control' },
    worktrees: { id: 'worktrees', title: 'Worktrees' },
    context: { id: 'context', title: 'Context' }
  };

  /** Height a pane opens at when the view has not been measured yet. Normally
   * the view's own height is used instead — see `buildView`. */
  const FALLBACK_PANE_SIZE = 400;

  interface Props {
    /** The column's controls, handed over as soon as it is mounted. They work
     * whether or not any view has been built yet. */
    onReady?: (controls: {
      resetLayout(): void;
      expandSourceControl(): void;
      selectView(id: SidebarViewId): void;
    }) => void;
    /** Which view is open now — so the icon strip, which lives outside this
     * component, can light up the right icon. Reported once at start-up (the
     * remembered view) and on every change after that. */
    onActiveViewChange?: (id: SidebarViewId) => void;
    /** Can the user see source control right now? Reported when it changes, and
     * once at start-up — a remembered view may have left it open. */
    onSourceControlVisible?: (visible: boolean) => void;
    /** Same question for the context cards. */
    onContextVisible?: (visible: boolean) => void;
  }
  let {
    onReady,
    onActiveViewChange,
    onSourceControlVisible,
    onContextVisible
  }: Props = $props();

  let activeView = $state<SidebarViewId>(DEFAULT_SIDEBAR_VIEW);
  let stackError = $state<string | null>(null);

  /** One container per view, and one authored body per view, by id. */
  let hosts: Partial<Record<SidebarViewId, HTMLElement>> = {};
  let bodies: Partial<Record<SidebarViewId, HTMLElement>> = {};

  /** Only the views that have actually been opened appear here. */
  const stacks = new Map<SidebarViewId, PaneStack>();
  /** Subscription to each gated view's own fold/unfold event. */
  const expansionListeners = new Map<SidebarViewId, DockviewIDisposable>();
  /** Last thing we told the page per gated view, so an unchanged answer is not
   * repeated. */
  const lastReported = new Map<SidebarViewId, boolean>();
  /** "Show source control" arrived before its view had been built; unfold it as
   * soon as it is. */
  let unfoldOnBuild = false;

  const paneFor = (id: SidebarViewId) => stacks.get(id)?.api.getPanel(PANES[id].id);

  const sourceControlPane = () => paneFor(SOURCE_CONTROL);

  /** Both conditions: this is the open view, and its pane is not folded away. A
   * stack that has never been built shows nothing. */
  function isViewVisible(id: SidebarViewId): boolean {
    if (activeView !== id) return false;
    return paneFor(id)?.api.isExpanded ?? false;
  }

  /** Tell the page where one gated view stands, if the answer has changed. */
  function reportView(id: SidebarViewId): void {
    const visible = isViewVisible(id);
    if (visible === lastReported.get(id)) return;
    lastReported.set(id, visible);
    if (id === SOURCE_CONTROL) onSourceControlVisible?.(visible);
    else if (id === 'context') onContextVisible?.(visible);
  }

  /** Say where every gated view stands. Called whenever the open view changes:
   * one of them may have just gone out of sight and another come into it. */
  function reportGatedViews(): void {
    for (const id of GATED_VIEWS) reportView(id);
  }

  /**
   * Re-subscribe to one gated view's pane and say where it stands.
   *
   * Called again after a reset on purpose: a reset removes every pane and builds
   * new ones, so the pane this was listening to no longer exists. Listening to
   * the stack as a whole would not do — `onDidLayoutChange` fires for every drag
   * of every divider, and the page would re-ask the same question dozens of
   * times per resize.
   */
  function watchView(id: SidebarViewId): void {
    expansionListeners.get(id)?.dispose();
    expansionListeners.delete(id);
    const pane = paneFor(id);
    if (pane) expansionListeners.set(id, pane.api.onDidExpansionChange(() => reportView(id)));
    reportView(id);
  }

  /**
   * Build one view's pane stack. Safe to call again; it only ever builds once.
   *
   * The stack is asked for at the view's real height and then laid out again
   * straight away, and BOTH are load-bearing. `createPaneStack` sizes the empty
   * stack before its panes exist and never lays out afterwards, so the pane it
   * adds keeps the height it was ASKED for rather than the height it HAS — and
   * nothing arrives later to correct it, because the resize that triggered this
   * build has already been delivered and dockview only re-lays-out when the
   * size CHANGES. A pane left taller than its view hangs its content below the
   * bottom edge, which is invisible on a panel that scrolls from the top and
   * very visible on one that centres itself.
   */
  function buildView(id: SidebarViewId): void {
    const host = hosts[id];
    const element = bodies[id];
    if (stacks.has(id) || !host || !element) return;
    const pane = PANES[id];
    const width = host.clientWidth;
    const height = host.clientHeight;
    try {
      const stack = createPaneStack(host, {
        storage: window.localStorage,
        storageKey: viewPanesKey(id),
        panes: [
          { id: pane.id, title: pane.title, element, size: height || FALLBACK_PANE_SIZE }
        ]
      });
      stacks.set(id, stack);
      if (width > 0 && height > 0) stack.layout(width, height);
    } catch (error) {
      stackError = error instanceof Error ? error.message : String(error);
      return;
    }
    if (!GATED_VIEWS.includes(id)) return;
    if (id === SOURCE_CONTROL && unfoldOnBuild) {
      unfoldOnBuild = false;
      sourceControlPane()?.api.setExpanded(true);
    }
    watchView(id);
  }

  /** Open a view. The container that becomes visible gets a size for the first
   * time here, and its own observer is what builds it. */
  function selectView(id: SidebarViewId): void {
    if (id !== activeView) {
      activeView = id;
      writeActiveView(window.localStorage, id);
      onActiveViewChange?.(id);
    }
    reportGatedViews();
  }

  onMount(() => {
    activeView = readActiveView(window.localStorage);
    onActiveViewChange?.(activeView);

    const observers = SIDEBAR_VIEWS.map((view) => {
      const observer = new ResizeObserver(() => {
        const host = hosts[view.id];
        if (!host) return;
        const width = host.clientWidth;
        const height = host.clientHeight;
        // Zero means this view is not the open one, or the whole column is
        // still parked inside ShellFrame's hidden stage. Either way: not a
        // measurement, and not a moment to build anything.
        if (width <= 0 || height <= 0) return;
        const stack = stacks.get(view.id);
        if (!stack) buildView(view.id);
        else stack.layout(width, height);
      });
      const host = hosts[view.id];
      if (host) observer.observe(host);
      return observer;
    });

    onReady?.({
      resetLayout: (): void => {
        for (const stack of stacks.values()) stack.resetLayout();
        clearActiveView(window.localStorage);
        activeView = DEFAULT_SIDEBAR_VIEW;
        onActiveViewChange?.(activeView);
        // Every pane is a new object after a reset, including the ones being
        // listened to.
        for (const id of GATED_VIEWS) watchView(id);
      },
      expandSourceControl: (): void => {
        selectView(SOURCE_CONTROL);
        const pane = sourceControlPane();
        // Unfolding fires the pane's own event, so the page hears about this
        // exactly the way it hears about a header click. A view that has never
        // been opened has no pane yet — `buildView` finishes the job.
        if (pane) pane.api.setExpanded(true);
        else unfoldOnBuild = true;
      },
      selectView
    });

    return () => {
      for (const observer of observers) observer.disconnect();
      for (const listener of expansionListeners.values()) listener.dispose();
      expansionListeners.clear();
      for (const stack of stacks.values()) stack.dispose();
      stacks.clear();
    };
  });
</script>

<div class="sidebar">
  <div class="views">
    {#each SIDEBAR_VIEWS as view (view.id)}
      <div
        class="view-host"
        class:showing={view.id === activeView}
        bind:this={hosts[view.id]}
      ></div>
    {/each}
    {#if stackError}
      <p class="stack-error">The tool column could not be built: {stackError}</p>
    {/if}
  </div>
</div>

<!-- Parking stage: each view's pane body lives here until its stack claims it,
     and again if a pane is ever removed. `display: none`, so nothing parked here
     can be measured — see the same note in ShellFrame. -->
<div class="parking-stage" aria-hidden="true">
  <div class="slot" bind:this={bodies.explorer}><ExplorerPanel /></div>
  <div class="slot" bind:this={bodies['source-control']}><GitPanel /></div>
  <div class="slot" bind:this={bodies.worktrees}>
    <PanelPlaceholder name="Worktrees" hint="Worktree health and cleanup will live here." />
  </div>
  <div class="slot" bind:this={bodies.context}><ContextPanel /></div>
</div>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    overflow: hidden;
    background: #101014;
  }

  /* Every view stacks in the same space; only the open one is displayed.
     `position: relative` is what the view containers below are absolutely
     positioned against, and `contain: paint` is the belt to `overflow`'s
     braces: it makes this box a containing block for EVERY positioned
     descendant and clips their painting to it. Without it, one positioned
     element inside a view that finds a different ancestor to anchor to gets
     drawn at that ancestor's size — which is how a strip of one view ends up
     painted outside the column, over the middle of the shell. */
  .views {
    position: relative;
    flex: 1 1 auto;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    contain: paint;
  }

  .view-host {
    position: absolute;
    inset: 0;
    display: none;
    overflow: hidden;
  }
  .view-host.showing {
    display: block;
  }

  .stack-error {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    margin: 0;
    padding: 8px 10px;
    color: #ff8f8f;
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
    font-size: 11px;
  }

  .parking-stage {
    display: none;
  }

  /* A teleported body fills the pane it lands in.
     The host is positioned rather than sized at `height: 100%` on purpose.
     dockview gives the pane body its height from `flex-grow: 1` and never
     writes a height on it (`.dv-pane-container .dv-pane .dv-pane-body` in
     dockview.css: `overflow-y: auto; flex-grow: 1; position: relative`), so a
     percentage height here is threaded through a flex-grow chain to resolve —
     the fragile case. That same rule makes the body `position: relative`, so
     `inset: 0` gives us the body's exact box with nothing to resolve. */
  .view-host :global(.pane-body-host) {
    position: absolute;
    inset: 0;
    overflow: hidden;
  }

  .slot {
    height: 100%;
    width: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  /* ---- paneview overrides ----
     dockview ships no default values for these: they only exist inside one of
     its theme classes, and these stacks are built without a theme. Every
     variable the paneview stylesheet reads has to be named here or it resolves
     to nothing — which is how you get an invisible header on a black column. */
  .view-host :global(.shell-pane-stack) {
    /* header background and header text */
    --dv-group-view-background-color: #101014;
    --dv-activegroup-visiblepanel-tab-color: #7b7b8c;
    /* the hairline between two sections */
    --dv-paneview-header-border-color: #22222c;
    --dv-separator-border: #22222c;
    /* the focus ring dockview draws around a header or body */
    --dv-paneview-active-outline-color: #bd93f9;
    /* VS Code-style resize feedback: the divider lights up teal as you grab it */
    --dv-active-sash-color: #4bf3c8;
    --dv-active-sash-transition-delay: 0.1s;
    --dv-active-sash-transition-duration: 0.05s;
    background: #101014;
  }

  /* Section headers read like the rail's own group headings. */
  .view-host :global(.dv-default-header) {
    align-items: center;
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.09em;
    text-transform: uppercase;
  }

  .view-host :global(.dv-default-header:hover) {
    color: #d8d8e0;
  }

  .view-host :global(.dv-pane-body) {
    background: #101014;
    min-height: 0;
    /* The bodies scroll themselves; a second scrollbar out here would be the
       paneview's, on content that never overflows it. */
    overflow: hidden;
  }
</style>
