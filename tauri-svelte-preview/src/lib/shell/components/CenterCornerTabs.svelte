<script lang="ts">
  /**
   * CenterCornerTabs.svelte — the centre surface picker in the window chrome.
   *
   * Session is what you talk to, Editor is the code you have open, Diff is the
   * changes to one file. Each control used to carry its own
   * fill and its own shadow, and four separate discs at the top of the pane read
   * as things dropped there rather than as the head of a column. Sharing a
   * surface is what makes them one control, and the head one thing.
   *
   * PRESENTATIONAL ONLY: no state beyond that hold, no IO. The selected tab is
   * handed in and every click is handed back out.
   */
  import FileCode2 from '@lucide/svelte/icons/file-code-2';
  import GitCompareArrows from '@lucide/svelte/icons/git-compare-arrows';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import MessagesSquare from '@lucide/svelte/icons/messages-square';
  import PanelRightClose from '@lucide/svelte/icons/panel-right-close';
  import PanelRightOpen from '@lucide/svelte/icons/panel-right-open';

  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import type { CenterTabId } from '$lib/shell/workbenchNavigation';

  interface Props {
    activeId: CenterTabId;
    onSelect(id: CenterTabId): void;
    rightPanelOpen: boolean;
    onToggleRightPanel(): void;
  }
  let { activeId, onSelect, rightPanelOpen, onToggleRightPanel }: Props = $props();

  const TABS: ReadonlyArray<{ id: CenterTabId; label: string; icon: typeof FileCode2 }> = [
    { id: 'session', label: 'Session', icon: MessagesSquare },
    { id: 'editor', label: 'Editor', icon: FileCode2 },
    { id: 'diff', label: 'Diff', icon: GitCompareArrows },
    { id: 'git-history', label: 'Git History', icon: GitBranch }
  ];

  function choose(id: CenterTabId): void {
    onSelect(id);
  }
</script>

<nav class="center-pills" aria-label="Center surfaces" data-tauri-drag-region>
  {#each TABS as tab (tab.id)}
    {@const Icon = tab.icon}
    <!-- Icon only. The word lives on `label`, which `IconButton` makes both the
         accessible name and the tooltip, so nothing is lost to a screen reader
         or to anyone who pauses on one. `aria-current` sits on the wrapper
         because the kit button does not take it — the same arrangement the
         right panel's tab strip uses — and it is what the fill is painted from,
         so the one attribute that names the current surface also shows it. -->
    <span class="tab" aria-current={tab.id === activeId ? 'page' : undefined}>
      <IconButton
        label={tab.label}
        size="sm"
        side="bottom"
        variant="ghost"
        data-testid={`center-tab-${tab.id}`}
        onclick={() => choose(tab.id)}
      >
        <Icon class="size-5" strokeWidth={tab.id === activeId ? 1.9 : 1.7} aria-hidden="true" />
      </IconButton>
    </span>
  {/each}
  <span class="panel-divider" aria-hidden="true"></span>
  <IconButton
    label={rightPanelOpen ? 'Close right panel' : 'Open right panel'}
    size="sm"
    side="bottom"
    variant="ghost"
    data-testid="toggle-right-panel"
    onclick={onToggleRightPanel}
  >
    {#if rightPanelOpen}
      <PanelRightClose class="size-5" strokeWidth={1.7} aria-hidden="true" />
    {:else}
      <PanelRightOpen class="size-5" strokeWidth={1.7} aria-hidden="true" />
    {/if}
  </IconButton>
</nav>

<style>
  /* One compact surface switch inside the draggable titlebar. The surrounding
     chrome owns the row; this capsule owns only its controls. */
  .center-pills {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    /* Tight, because the capsule's own edge is what separates the group from
       the pane now; the controls inside it only need to stay apart. */
    gap: 2px;
    height: var(--center-head-row-height);
    padding: 2px;
    margin: 0;
    border-radius: var(--radius-pill);
    background: var(--pill-surface);
    box-shadow: none;
    user-select: none;
  }

  .tab {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
  }

  .panel-divider {
    width: 1px;
    height: 18px;
    margin: 0 2px;
    background: var(--color-border);
  }

  /* Marks on the capsule's own surface: no fill and no shadow of their own, so
     the only filled disc in the group is the one that means something. 28px is
     the kit's floor for an icon button — a smaller square is one a pointer
     misses — and the capsule is built around it rather than the other way
     round. Scoped through `.tab` so the language switch, which is a button too,
     is not shaped like a surface tab. */
  .tab :global(button),
  .center-pills > :global(button) {
    width: 34px;
    height: 34px;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text-2);
  }

  .tab :global(button:hover),
  .center-pills > :global(button:hover) {
    background: var(--pill-surface-hover);
    color: var(--color-text);
    /* The kit draws a soft 4px ring around a hovered icon button. On a button
       standing on its own that reads well; inside a capsule with 2px of room
       around it, the ring lands on the capsule's edge and blurs it. */
    box-shadow: none;
  }

  /* The one that is filled is the one you are on. It shares the right-panel
     selector colour so green remains a status colour, not a navigation state. */
  .tab[aria-current='page'] :global(button) {
    background: var(--pill-surface-active);
    color: var(--pill-text-active);
  }

  .tab[aria-current='page'] :global(button:hover) {
    background: var(--pill-surface-active);
    color: var(--pill-text-active);
  }
</style>
