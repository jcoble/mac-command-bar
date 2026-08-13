<script lang="ts">
  /**
   * CenterCornerTabs.svelte — the three small tabs in the center pane's
   * upper-right corner.
   *
   * Session is what you talk to, Editor is the code you have open, Diff is the
   * changes to one file. Small on purpose: they name the surface without taking
   * a row of the pane away from it.
   *
   * PRESENTATIONAL ONLY: no state, no IO. The selected tab is handed in and
   * every click is handed back out.
   */
  import FileCode2 from '@lucide/svelte/icons/file-code-2';
  import GitCompareArrows from '@lucide/svelte/icons/git-compare-arrows';
  import MessagesSquare from '@lucide/svelte/icons/messages-square';

  import { Button } from '$lib/components/ui/button/index.js';
  import type { CenterTabId } from '$lib/shell/workbenchNavigation';

  interface Props {
    activeId: CenterTabId;
    onSelect(id: CenterTabId): void;
  }
  let { activeId, onSelect }: Props = $props();

  const TABS: ReadonlyArray<{ id: CenterTabId; label: string; icon: typeof FileCode2 }> = [
    { id: 'session', label: 'Session', icon: MessagesSquare },
    { id: 'editor', label: 'Editor', icon: FileCode2 },
    { id: 'diff', label: 'Diff', icon: GitCompareArrows }
  ];
</script>

<nav class="center-corner-tabs" aria-label="Center surfaces">
  {#each TABS as tab (tab.id)}
    {@const Icon = tab.icon}
    <!-- `text-sm` is 12px in this shell and overrides the xs recipe's 11px,
         which is below the floor for anything a person reads. -->
    <Button
      size="xs"
      variant={tab.id === activeId ? 'secondary' : 'ghost'}
      aria-current={tab.id === activeId ? 'page' : undefined}
      class={`text-sm ${tab.id === activeId ? 'text-foreground' : 'text-muted-foreground'}`}
      data-testid={`center-tab-${tab.id}`}
      onclick={() => onSelect(tab.id)}
    >
      <Icon class="size-3.5" strokeWidth={1.6} aria-hidden="true" />
      {tab.label}
    </Button>
  {/each}
</nav>

<style>
  .center-corner-tabs {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 2px;
    padding: 4px 6px;
    user-select: none;
  }
</style>
