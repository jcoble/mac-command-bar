<script lang="ts">
  /**
   * RightPanelTabs.svelte — the tab strip across the top of the right panel.
   *
   * Eight tabs, one panel each, in a fixed order, drawn as one segmented
   * control so the strip reads as a single set with one choice in it. The
   * panel is narrow, so each tab is its glyph and says its name on hover.
   * PRESENTATIONAL ONLY: no state, no IO, and no knowledge of what any panel
   * contains. The selected tab is handed in and every click is handed back out.
   */
  import Activity from '@lucide/svelte/icons/activity';
  import Bot from '@lucide/svelte/icons/bot';
  import Files from '@lucide/svelte/icons/files';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import Globe2 from '@lucide/svelte/icons/globe-2';
  import History from '@lucide/svelte/icons/history';
  import Layers from '@lucide/svelte/icons/layers';
  import Play from '@lucide/svelte/icons/play';

  import type { RightTabId } from '$lib/shell/workbenchNavigation';
  import SegmentedTabs from './SegmentedTabs.svelte';
  import type { SegmentedTabItem } from './segmentedTabs';

  interface Props {
    activeId: RightTabId;
    onSelect(id: RightTabId): void;
  }
  let { activeId, onSelect }: Props = $props();

  /** Order, ids, labels and glyphs, settled in one place so the strip and the
   * panel host cannot drift apart. Each tab keeps the test id it carried while
   * the strip was eight separate buttons. */
  const TABS: ReadonlyArray<SegmentedTabItem> = (
    [
      { id: 'files', label: 'Files', icon: Files },
      { id: 'source-control', label: 'Source control', icon: GitBranch },
      { id: 'worktrees', label: 'Worktrees', icon: Layers },
      { id: 'run', label: 'Run', icon: Play },
      { id: 'context', label: 'Context', icon: Activity },
      { id: 'agents', label: 'Agents', icon: Bot },
      { id: 'browser', label: 'Browser', icon: Globe2 },
      { id: 'history', label: 'History', icon: History }
    ] as const satisfies ReadonlyArray<{ id: RightTabId; label: string; icon: typeof Files }>
  ).map((tab) => ({ ...tab, testId: `right-tab-${tab.id}` }));
</script>

<div class="right-panel-tabs">
  <SegmentedTabs
    label="Right panel"
    items={TABS}
    value={activeId}
    onChange={(id) => onSelect(id as RightTabId)}
  />
</div>

<style>
  .right-panel-tabs {
    display: flex;
    width: 100%;
    min-width: 0;
    height: 44px;
    align-items: center;
    /* Transparent, and no rule beneath it: the card's gradient runs behind the
       tabs, and a hairline here cut the strip off as its own band again. */
    padding: 0 6px;
    background: transparent;
  }
</style>
