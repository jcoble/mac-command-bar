<script lang="ts">
  /**
   * RightPanel.svelte — the right column of the workbench.
   *
   * Two rows: the icon tab strip along the top and one panel body filling the
   * rest. The Resources/Usage strip now runs the full width of the window as
   * the shell's status bar, so it no longer lives here. Exactly one panel is
   * mounted at a time; panel stores keep durable state while component-owned
   * observers and DOM leave WebKit as soon as the tab is closed.
   *
   * No backend IO and no state of its own beyond the layout. Which tab is open
   * is decided by the page (it is remembered per session) and handed in; every
   * click goes straight back out.
   */
  import type { RightTabId } from '$lib/shell/workbenchNavigation';

  import AgentsPanel from '$lib/shell/panels/agents/AgentsPanel.svelte';
  import BrowserPanel from '$lib/shell/panels/browser/BrowserPanel.svelte';
  import FilesPanel from '$lib/shell/panels/files/FilesPanel.svelte';
  import HistoryPanel from '$lib/shell/panels/history/HistoryPanel.svelte';
  import RunPanel from '$lib/shell/panels/run/RunPanel.svelte';
  import SessionContextPanel from '$lib/shell/panels/context/SessionContextPanel.svelte';
  import SourceControlPanel from '$lib/shell/panels/sourceControl/SourceControlPanel.svelte';
  import WorktreesPanel from '$lib/shell/panels/worktrees/WorktreesPanel.svelte';

  import RightPanelTabs from './RightPanelTabs.svelte';

  interface Props {
    /** The tab on screen. */
    activeId: RightTabId;
    onSelect(id: RightTabId): void;
    /** The active session's working folder, or '' when nothing is selected. */
    root: string;
    /** False when the active session's checkout has disappeared. */
    rootAvailable?: boolean;
    /** The active session's ownedId, or null. */
    ownedId: string | null;
    onRootUnavailable?(root: string): void | Promise<void>;
    expandedPathsByRoot?: Readonly<Record<string, readonly string[]>>;
    onExpandedPathsChange?(root: string, paths: readonly string[]): void;
    onUseSessionCheckout?(root: string): void | Promise<void>;
  }
  let {
    activeId,
    onSelect,
    root,
    rootAvailable = true,
    ownedId,
    onRootUnavailable,
    expandedPathsByRoot,
    onExpandedPathsChange,
    onUseSessionCheckout
  }: Props = $props();
</script>

<div class="right-panel">
  <RightPanelTabs {activeId} {onSelect} />

  <div class="panel-bodies">
    {#if activeId === 'files'}
      <div class="panel-body showing">
        <FilesPanel
          visible={true}
          {root}
          {ownedId}
          {onRootUnavailable}
          {expandedPathsByRoot}
          {onExpandedPathsChange}
        />
      </div>
    {:else if activeId === 'source-control'}
      <div class="panel-body showing">
        <SourceControlPanel
          visible={true}
          {root}
          {ownedId}
          {rootAvailable}
          {onUseSessionCheckout}
        />
      </div>
    {:else if activeId === 'worktrees'}
      <div class="panel-body showing"><WorktreesPanel visible={true} {root} {ownedId} /></div>
    {:else if activeId === 'run'}
      <div class="panel-body showing"><RunPanel visible={true} {root} {ownedId} /></div>
    {:else if activeId === 'context'}
      <div class="panel-body showing"><SessionContextPanel visible={true} {root} {ownedId} /></div>
    {:else if activeId === 'agents'}
      <div class="panel-body showing"><AgentsPanel visible={true} {root} {ownedId} /></div>
    {:else if activeId === 'browser'}
      <div class="panel-body showing"><BrowserPanel visible={true} {root} {ownedId} /></div>
    {:else if activeId === 'history'}
      <div class="panel-body showing"><HistoryPanel visible={true} {root} {ownedId} /></div>
    {/if}
  </div>
</div>

<style>
  .right-panel {
    display: grid;
    height: 100%;
    width: 100%;
    min-width: 0;
    min-height: 0;
    grid-template-rows: auto minmax(0, 1fr);
    overflow: hidden;
    /* The card behind this paints the surface and its gradient. */
    background: transparent;
  }

  /* Every panel stacks in the same space; only the open one is displayed.
     `contain: paint` is the belt to `overflow`'s braces: it makes this box a
     containing block for every positioned descendant and clips their painting
     to it, so a panel's own floating strip cannot be drawn over the middle of
     the shell. Same reasoning as the column this replaced. */
  .panel-bodies {
    position: relative;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    contain: paint;
  }

  .panel-body {
    position: absolute;
    inset: 0;
    display: none;
    overflow: hidden;
  }

  .panel-body.showing {
    display: block;
  }
</style>
