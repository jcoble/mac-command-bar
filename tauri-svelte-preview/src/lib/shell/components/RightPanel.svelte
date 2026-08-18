<script lang="ts">
  /**
   * RightPanel.svelte — the right column of the workbench.
   *
   * Two rows: the icon tab strip along the top and one panel body filling the
   * rest. The Resources/Usage strip now runs the full width of the window as
   * the shell's status bar, so it no longer lives here. Exactly one panel
   * is on screen at a time, but every one of the eight stays MOUNTED for the
   * whole session — switching tabs only changes which is displayed. Nothing is
   * torn down, so a scroll position, a typed filter or a half-expanded tree
   * survives being looked away from.
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
    /** The active session's ownedId, or null. */
    ownedId: string | null;
  }
  let { activeId, onSelect, root, ownedId }: Props = $props();
</script>

<div class="right-panel">
  <RightPanelTabs {activeId} {onSelect} />

  <div class="panel-bodies">
    <div class="panel-body" class:showing={activeId === 'files'}>
      <FilesPanel visible={activeId === 'files'} {root} {ownedId} />
    </div>
    <div class="panel-body" class:showing={activeId === 'source-control'}>
      <SourceControlPanel visible={activeId === 'source-control'} {root} {ownedId} />
    </div>
    <div class="panel-body" class:showing={activeId === 'worktrees'}>
      <WorktreesPanel visible={activeId === 'worktrees'} {root} {ownedId} />
    </div>
    <div class="panel-body" class:showing={activeId === 'run'}>
      <RunPanel visible={activeId === 'run'} {root} {ownedId} />
    </div>
    <div class="panel-body" class:showing={activeId === 'context'}>
      <SessionContextPanel visible={activeId === 'context'} {root} {ownedId} />
    </div>
    <div class="panel-body" class:showing={activeId === 'agents'}>
      <AgentsPanel visible={activeId === 'agents'} {root} {ownedId} />
    </div>
    <div class="panel-body" class:showing={activeId === 'browser'}>
      <BrowserPanel visible={activeId === 'browser'} {root} {ownedId} />
    </div>
    <div class="panel-body" class:showing={activeId === 'history'}>
      <HistoryPanel visible={activeId === 'history'} {root} {ownedId} />
    </div>
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
