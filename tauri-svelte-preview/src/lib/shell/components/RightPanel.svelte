<script lang="ts">
  /**
   * RightPanel.svelte — the right column of the workbench.
   *
   * Two rows: the icon tab strip along the top and one panel body filling the
   * rest. The Resources/Usage strip now runs the full width of the window as
   * the shell's status bar, so it no longer lives here. Files keeps one bounded
   * panel instance for the active session because repeatedly reconstructing the
   * virtual tree makes WebKit retain allocator pages. Every other panel mounts
   * only while it is visible.
   *
   * No backend IO and no state of its own beyond the layout. Which tab is open
   * is decided by the page (it is remembered per session) and handed in; every
   * click goes straight back out.
   */
  import type { RightTabId } from '$lib/shell/workbenchNavigation';
  import type {
    CheckoutScope,
    SessionHistoryWorkspace,
    SessionSourceControlWorkspace
  } from '$lib/shell/sessionWorkspaces';

  import AgentsPanel from '$lib/shell/panels/agents/AgentsPanel.svelte';
  import BrowserPanel from '$lib/shell/panels/browser/BrowserPanel.svelte';
  import FilesPanel from '$lib/shell/panels/files/FilesPanel.svelte';
  import HistoryPanel from '$lib/shell/panels/history/HistoryPanel.svelte';
  import RunPanel from '$lib/shell/panels/run/RunPanel.svelte';
  import SessionContextPanel from '$lib/shell/panels/context/SessionContextPanel.svelte';
  import SourceControlPanel from '$lib/shell/panels/sourceControl/SourceControlPanel.svelte';
  import WorktreesPanel from '$lib/shell/panels/worktrees/WorktreesPanel.svelte';
  import TasksPanel from '$lib/shell/panels/tasks/TasksPanel.svelte';

  import RightPanelTabs from './RightPanelTabs.svelte';

  interface Props {
    /** False while the whole right grid region is closed. */
    visible: boolean;
    /** The tab on screen. */
    activeId: RightTabId;
    onSelect(id: RightTabId): void;
    /** The active session's working folder, or '' when nothing is selected. */
    root: string;
    /** False when the active session's checkout has disappeared. */
    rootAvailable?: boolean;
    /** Current route-owned checkout roots and stale-request generations. */
    checkoutScope?: CheckoutScope;
    /** The active session's ownedId, or null. */
    ownedId: string | null;
    /** Optional tree-only projection while rail selection is rebuilt in layers. */
    filesRoot?: string;
    filesOwnedId?: string | null;
    onRootUnavailable?(root: string): void | Promise<void>;
    expandedPathsByRoot?: Readonly<Record<string, readonly string[]>>;
    onExpandedPathsChange?(root: string, paths: readonly string[]): void;
    filesInspectionRoot?: string | null;
    checkoutDiscoveryRoots?: readonly string[];
    sourceControlInspectionRoot?: string | null;
    onFilesInspectionRootChange?(root: string | null): void;
    onSourceControlInspectionRootChange?(root: string | null): void;
    sourceControlWorkspace?: SessionSourceControlWorkspace;
    historyWorkspace?: SessionHistoryWorkspace;
    onSourceControlWorkspaceChange?(ownedId: string | null, state: SessionSourceControlWorkspace): void;
    onHistoryWorkspaceChange?(ownedId: string | null, state: SessionHistoryWorkspace): void;
    onUseSessionCheckout?(root: string): void | Promise<void>;
  }
  let {
    visible,
    activeId,
    onSelect,
    root,
    rootAvailable = true,
    checkoutScope,
    ownedId,
    filesRoot,
    filesOwnedId,
    onRootUnavailable,
    expandedPathsByRoot,
    onExpandedPathsChange,
    filesInspectionRoot,
    checkoutDiscoveryRoots,
    sourceControlInspectionRoot,
    onFilesInspectionRootChange,
    onSourceControlInspectionRootChange,
    sourceControlWorkspace,
    historyWorkspace,
    onSourceControlWorkspaceChange,
    onHistoryWorkspaceChange,
    onUseSessionCheckout
  }: Props = $props();
</script>

<div class="right-panel">
  <RightPanelTabs {activeId} {onSelect} />

  <div class="panel-bodies">
    <div
      class="panel-body"
      class:showing={visible && activeId === 'files'}
      aria-hidden={!visible || activeId !== 'files'}
    >
      <FilesPanel
        visible={visible && activeId === 'files'}
        root={filesRoot ?? root}
        ownedId={filesOwnedId === undefined ? ownedId : filesOwnedId}
        {onRootUnavailable}
        {expandedPathsByRoot}
        {onExpandedPathsChange}
        inspectionRoot={filesInspectionRoot}
        onInspectionRootChange={onFilesInspectionRootChange}
        {checkoutDiscoveryRoots}
        {onUseSessionCheckout}
      />
    </div>
    {#if visible && activeId === 'source-control'}
      <div class="panel-body showing">
        <SourceControlPanel
          visible={true}
          {root}
          {ownedId}
          {rootAvailable}
          {checkoutScope}
          inspectionRoot={sourceControlInspectionRoot}
          onInspectionRootChange={onSourceControlInspectionRootChange}
          workspaceState={sourceControlWorkspace}
          onWorkspaceStateChange={onSourceControlWorkspaceChange}
          {onUseSessionCheckout}
        />
      </div>
    {:else if visible && activeId === 'worktrees'}
      <div class="panel-body showing"><WorktreesPanel visible={true} {root} {ownedId} /></div>
    {:else if visible && activeId === 'run'}
      <div class="panel-body showing"><RunPanel visible={true} {root} {ownedId} /></div>
    {:else if visible && activeId === 'context'}
      <div class="panel-body showing"><SessionContextPanel visible={true} {root} {ownedId} /></div>
    {:else if visible && activeId === 'agents'}
      <div class="panel-body showing"><AgentsPanel visible={true} {root} {ownedId} /></div>
    {:else if visible && activeId === 'browser'}
      <div class="panel-body showing"><BrowserPanel visible={true} {root} {ownedId} /></div>
    {:else if visible && activeId === 'history'}
      <div class="panel-body showing">
        <HistoryPanel
          visible={true}
          {root}
          {ownedId}
          workspaceState={historyWorkspace}
          onWorkspaceStateChange={onHistoryWorkspaceChange}
        />
      </div>
    {:else if visible && activeId === 'tasks'}
      <div class="panel-body showing"><TasksPanel /></div>
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
