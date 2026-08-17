<script lang="ts">
  /**
   * RightPanelTabs.svelte — the icon strip across the top of the right panel.
   *
   * Eight tabs, one panel each, in a fixed order. PRESENTATIONAL ONLY: no
   * state, no IO, and no knowledge of what any panel contains. The selected tab
   * is handed in and every click is handed back out.
   */
  import Activity from '@lucide/svelte/icons/activity';
  import Bot from '@lucide/svelte/icons/bot';
  import Files from '@lucide/svelte/icons/files';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import Globe2 from '@lucide/svelte/icons/globe-2';
  import History from '@lucide/svelte/icons/history';
  import Layers from '@lucide/svelte/icons/layers';
  import Play from '@lucide/svelte/icons/play';

  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import type { RightTabId } from '$lib/shell/workbenchNavigation';

  interface Props {
    activeId: RightTabId;
    onSelect(id: RightTabId): void;
  }
  let { activeId, onSelect }: Props = $props();

  /** Order, ids, labels and glyphs, settled in one place so the strip and the
   * panel host cannot drift apart. */
  const TABS: ReadonlyArray<{ id: RightTabId; label: string; icon: typeof Files }> = [
    { id: 'files', label: 'Files', icon: Files },
    { id: 'source-control', label: 'Source control', icon: GitBranch },
    { id: 'worktrees', label: 'Worktrees', icon: Layers },
    { id: 'run', label: 'Run', icon: Play },
    { id: 'context', label: 'Context', icon: Activity },
    { id: 'agents', label: 'Agents', icon: Bot },
    { id: 'browser', label: 'Browser', icon: Globe2 },
    { id: 'history', label: 'History', icon: History }
  ];
</script>

<nav class="right-panel-tabs" aria-label="Right panel">
  {#each TABS as tab (tab.id)}
    {@const Icon = tab.icon}
    <span
      class="tab"
      data-tab={tab.id}
      aria-current={tab.id === activeId ? 'page' : undefined}
    >
      <IconButton
        label={tab.label}
        size="sm"
        side="bottom"
        variant={tab.id === activeId ? 'secondary' : 'ghost'}
        class={tab.id === activeId ? 'text-foreground' : 'text-muted-foreground'}
        data-testid={`right-tab-${tab.id}`}
        onclick={() => onSelect(tab.id)}
      >
        <Icon class="size-[20px]" strokeWidth={1.6} aria-hidden="true" />
      </IconButton>
    </span>
  {/each}
</nav>

<style>
  .right-panel-tabs {
    display: flex;
    width: 100%;
    min-width: 0;
    height: 44px;
    align-items: center;
    gap: 4px;
    padding: 4px 6px;
    overflow-x: auto;
    overflow-y: hidden;
    /* Transparent, and no rule beneath it: the card's gradient runs behind the
       tabs, and a hairline here cut the strip off as its own band again. */
    background: transparent;
    scrollbar-width: none;
    user-select: none;
  }

  .right-panel-tabs::-webkit-scrollbar {
    display: none;
  }

  .tab {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
  }

  /* Round, and bigger than the small icon button's default: these are the
     panel's primary navigation, and at 28px they were hard to aim at. */
  .tab :global(button) {
    width: 34px;
    height: 34px;
    border-radius: 999px;
  }

  .tab[aria-current='page'] :global(button) {
    background: var(--color-elevated);
  }
</style>
