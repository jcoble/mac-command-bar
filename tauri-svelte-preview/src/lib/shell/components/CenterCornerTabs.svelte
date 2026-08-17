<script lang="ts">
  /**
   * CenterCornerTabs.svelte — the three small tabs in the center pane's
   * upper-right corner.
   *
   * Session is what you talk to, Editor is the code you have open, Diff is the
   * changes to one file. They sit in a shared track and read as one control,
   * the same way the range picker on the usage screen does, so which surface
   * you are on is legible without reading the labels.
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
         which is below the floor for anything a person reads. Rest, hover and
         selected are painted below, off `aria-current`, so the one attribute
         that tells a screen reader which tab this is also tells the eye. -->
    <Button
      size="sm"
      variant="ghost"
      aria-current={tab.id === activeId ? 'page' : undefined}
      class="text-sm"
      data-testid={`center-tab-${tab.id}`}
      onclick={() => onSelect(tab.id)}
    >
      <Icon class="size-4" strokeWidth={1.6} aria-hidden="true" />
      {tab.label}
    </Button>
  {/each}
</nav>

<style>
  /* The tabs sit flat on the card. They used to live in a recessed track, but
     its darker fill ran the card's full width and its lower edge read as a rule
     under the tab bar; only the active tab carries a fill now. */
  .center-corner-tabs {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 2px;
    margin: 4px 6px;
    padding: 3px;
    border-radius: var(--radius-md, 9px);
    background: transparent;
    user-select: none;
  }

  /* 30px tall with real side padding: a tab you can hit without aiming. */
  .center-corner-tabs :global(button) {
    min-height: 30px;
    gap: 6px;
    padding: 5px 11px;
    border-radius: var(--radius-sm, 7px);
    background: transparent;
    color: var(--color-text-2);
  }

  .center-corner-tabs :global(button:hover) {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .center-corner-tabs :global(button[aria-current='page']) {
    background: var(--color-elevated);
    color: var(--color-text);
  }
</style>
