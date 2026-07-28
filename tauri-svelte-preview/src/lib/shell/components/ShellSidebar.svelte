<script lang="ts">
  /**
   * ShellSidebar.svelte — the left column, as a stack of collapsible sections
   * (one dockview Paneview): Sessions, Files, Source control, Worktrees. Drag a
   * divider to resize a section, click a header to fold it away; both are
   * remembered across launches under this component's own storage key.
   *
   * Same teleport contract as ShellFrame: every section's body is authored in
   * the hidden parking stage below and MOVED into a dockview-owned host, so
   * dockview never renders or destroys app DOM. If the stack fails to build,
   * the bodies simply stay parked and this component says so in its place —
   * nothing crashes and nothing is lost.
   *
   * No backend IO here, and this component never loads anything itself. What it
   * does do is REPORT: it hands the page a way to reset the column, and it says
   * whenever the Source control section is opened or folded away, because that
   * is what decides when source control is allowed to read the repository (see
   * `panelActivation.ts`).
   */
  import 'dockview-core/dist/styles/dockview.css';
  import { onMount, type ComponentProps } from 'svelte';
  // dockview re-exports its disposable under a prefixed name to avoid clashing
  // with the one most codebases already have; the plain `IDisposable` is not
  // part of its public surface.
  import type { DockviewIDisposable } from 'dockview-core';

  import { createPaneStack, type PaneStack } from '$lib/shell/layout/paneStack';

  import ExplorerPanel from './ExplorerPanel.svelte';
  import GitPanel from './GitPanel.svelte';
  import PanelPlaceholder from './PanelPlaceholder.svelte';
  import SessionRail from './SessionRail.svelte';

  /** The section whose open/folded state gates a backend loader. */
  const SOURCE_CONTROL = 'source-control';

  interface Props extends ComponentProps<typeof SessionRail> {
    /** The column's controls, handed over once the stack is built. Never called
     * if building failed — there would be nothing behind them. */
    onReady?: (controls: { resetLayout(): void; expandSourceControl(): void }) => void;
    /** The Source control section's state: once when the stack is built (a
     * remembered layout may have left it open), then on every change. */
    onSourceControlExpanded?: (expanded: boolean) => void;
  }
  let { onReady, onSourceControlExpanded, ...railProps }: Props = $props();

  let host: HTMLElement;
  let sessionsSlot: HTMLElement;
  let filesSlot: HTMLElement;
  let sourceControlSlot: HTMLElement;
  let worktreesSlot: HTMLElement;

  let ready = $state(false);
  let stackError = $state<string | null>(null);

  onMount(() => {
    let stack: PaneStack | null = null;
    /** Subscription to the Source control section's own expansion event. */
    let expansionListener: DockviewIDisposable | null = null;

    /**
     * Re-subscribe to the Source control section and report where it stands.
     *
     * Called again after a reset on purpose: a reset removes every pane and
     * builds new ones, so the panel this was listening to no longer exists and
     * the new one starts folded again. Listening to the stack as a whole would
     * not do — `onDidLayoutChange` fires for every drag of every divider, and
     * the page would re-ask the same question dozens of times per resize.
     */
    const watchSourceControl = (): void => {
      expansionListener?.dispose();
      expansionListener = null;
      const pane = stack?.api.getPanel(SOURCE_CONTROL);
      if (!pane) {
        onSourceControlExpanded?.(false);
        return;
      }
      expansionListener = pane.api.onDidExpansionChange((event) =>
        onSourceControlExpanded?.(event.isExpanded)
      );
      onSourceControlExpanded?.(pane.api.isExpanded);
    };

    /**
     * Build only once the column has a real size.
     *
     * This component mounts while it is still parked inside ShellFrame's own
     * hidden stage (a child mounts before its parent), so at `onMount` the host
     * measures 0 x 0 — and a Paneview restored at 0 x 0 lays every remembered
     * section out at nothing. Waiting for the first real measurement is what
     * makes a remembered layout come back at the sizes it was saved with.
     */
    const build = (): void => {
      if (stack) return;
      try {
        stack = createPaneStack(host, {
          storage: window.localStorage,
          storageKey: 'mac-command-bar.next.left-panes',
          panes: [
            { id: 'sessions', title: 'Sessions', element: sessionsSlot, size: 260 },
            { id: 'files', title: 'Files', element: filesSlot, size: 240 },
            {
              id: SOURCE_CONTROL,
              title: 'Source control',
              element: sourceControlSlot,
              size: 220,
              expanded: false
            },
            {
              id: 'worktrees',
              title: 'Worktrees',
              element: worktreesSlot,
              size: 120,
              expanded: false
            }
          ]
        });
        ready = true;
        watchSourceControl();
        onReady?.({
          resetLayout: (): void => {
            stack?.resetLayout();
            // The panes are new objects now, and the section is folded again.
            watchSourceControl();
          },
          expandSourceControl: (): void => {
            // Unfolding fires the section's own expansion event, so the page
            // hears about this exactly the way it hears about a header click.
            stack?.api.getPanel(SOURCE_CONTROL)?.api.setExpanded(true);
          }
        });
      } catch (error) {
        stackError = error instanceof Error ? error.message : String(error);
      }
    };

    // dockview watches the host itself once the stack exists; this observer is
    // what starts it, and keeps the stack sized while the column is dragged.
    const observer = new ResizeObserver(() => {
      const width = host.clientWidth;
      const height = host.clientHeight;
      if (width <= 0 || height <= 0) return;
      if (!stack) build();
      else stack.layout(width, height);
    });
    observer.observe(host);
    if (host.clientWidth > 0 && host.clientHeight > 0) build();

    return () => {
      observer.disconnect();
      expansionListener?.dispose();
      expansionListener = null;
      stack?.dispose();
      stack = null;
    };
  });
</script>

<div class="sidebar">
  <div class="pane-stack-host" class:ready bind:this={host}></div>
  {#if stackError}
    <p class="stack-error">The left column could not be built: {stackError}</p>
  {/if}
</div>

<!-- Parking stage: the four section bodies live here until the stack claims
     them, and again if a section is ever removed. `display: none`, so nothing
     parked here can be measured — see the same note in ShellFrame. -->
<div class="parking-stage" aria-hidden="true">
  <div class="slot" bind:this={sessionsSlot}><SessionRail {...railProps} /></div>
  <div class="slot" bind:this={filesSlot}><ExplorerPanel /></div>
  <div class="slot" bind:this={sourceControlSlot}><GitPanel /></div>
  <div class="slot" bind:this={worktreesSlot}>
    <PanelPlaceholder name="Worktrees" hint="Worktree health and cleanup will live here." />
  </div>
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

  .pane-stack-host {
    flex: 1 1 auto;
    min-height: 0;
    width: 100%;
    overflow: hidden;
    visibility: hidden; /* anti-flash: revealed when the stack is built */
  }
  .pane-stack-host.ready {
    visibility: visible;
  }

  .stack-error {
    flex: 0 0 auto;
    margin: 0;
    padding: 8px 10px;
    color: #ff8f8f;
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
    font-size: 11px;
  }

  .parking-stage {
    display: none;
  }

  /* A teleported body fills the pane it lands in. */
  .slot,
  .pane-stack-host :global(.pane-body-host) {
    height: 100%;
    width: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  /* ---- paneview overrides ----
     dockview ships no default values for these: they only exist inside one of
     its theme classes, and this stack is built without a theme. Every variable
     the paneview stylesheet reads has to be named here or it resolves to
     nothing — which is how you get an invisible header on a black column. */
  .pane-stack-host :global(.shell-pane-stack) {
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
  .pane-stack-host :global(.dv-default-header) {
    align-items: center;
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.09em;
    text-transform: uppercase;
  }

  .pane-stack-host :global(.dv-default-header:hover) {
    color: #d8d8e0;
  }

  .pane-stack-host :global(.dv-pane-body) {
    background: #101014;
    min-height: 0;
    /* The bodies scroll themselves; a second scrollbar out here would be the
       paneview's, on content that never overflows it. */
    overflow: hidden;
  }
</style>
