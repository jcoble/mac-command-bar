<script lang="ts">
  /**
   * ActivityBar.svelte — the slim icon strip down the far right edge of the
   * shell. One icon per view; clicking one opens that view in the tool column
   * to its left. The gear at the bottom is not a view — it opens the settings
   * dialog.
   *
   * PRESENTATIONAL ONLY: no state, no IO, no knowledge of what a view contains.
   * The roster comes from `sidebarViews.ts` so the ids here and the ids the
   * column builds hosts for cannot drift apart.
   */
  import {
    Activity,
    Files,
    GitBranch,
    Layers,
    Play,
    Settings,
    TriangleAlert
  } from '@lucide/svelte';

  import { settings } from '$lib/settingsStore.svelte';
  import {
    PROBLEMS_VIEW_ID,
    SIDEBAR_VIEWS,
    type SidebarViewId
  } from '$lib/shell/layout/sidebarViews';

  interface Props {
    activeId: SidebarViewId;
    onSelect(id: SidebarViewId): void;
    onOpenSettings(): void;
  }
  let { activeId, onSelect, onOpenSettings }: Props = $props();

  /** Icon per view. Keyed by id so adding a view to the roster without an icon
   * is a type error rather than a blank button. */
  const ICONS: Record<SidebarViewId, typeof Files> = {
    explorer: Files,
    'source-control': GitBranch,
    worktrees: Layers,
    stacks: Play,
    context: Activity,
    problems: TriangleAlert
  };

  /** The icons to draw. Problems has a container in the tool column at all
   * times, but it only earns an icon here when the user has asked for it in
   * this column — while it is in the strip along the bottom, a second way in
   * would be two doors onto one list. */
  const shownViews = $derived(
    SIDEBAR_VIEWS.filter(
      (view) => view.id !== PROBLEMS_VIEW_ID || settings.panels.problemsLocation === 'right'
    )
  );
</script>

<nav class="activity-bar" aria-label="Views">
  <div class="group">
    {#each shownViews as view (view.id)}
      {@const Icon = ICONS[view.id]}
      <button
        type="button"
        class="icon-button"
        class:active={view.id === activeId}
        title={view.title}
        aria-label={view.title}
        aria-current={view.id === activeId ? 'true' : undefined}
        onclick={() => onSelect(view.id)}
      >
        <Icon size={19} strokeWidth={1.6} />
      </button>
    {/each}
  </div>

  <div class="group">
    <button
      type="button"
      class="icon-button"
      title="Settings"
      aria-label="Settings"
      onclick={() => onOpenSettings()}
    >
      <Settings size={19} strokeWidth={1.6} />
    </button>
  </div>
</nav>

<style>
  .activity-bar {
    display: flex;
    flex-direction: column;
    justify-content: space-between; /* views at the top, the gear at the bottom */
    flex: 0 0 44px;
    width: 44px;
    height: 100%;
    padding: 6px 0;
    background: #0c0c10;
    /* The strip is on the outer edge now, so its hairline faces the column it
       opens rather than the middle of the shell. */
    border-left: 1px solid #22222c;
    user-select: none;
  }

  .group {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .icon-button {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 40px;
    border: 0;
    background: transparent;
    color: #6d6d7d;
    cursor: pointer;
  }

  .icon-button:hover {
    color: #d8d8e0;
  }

  .icon-button.active {
    color: #e6e6ee;
  }

  /* The teal edge marking the open view — the same accent the pane dividers
     use. On the outer edge, so it does not sit on top of the hairline that
     separates the strip from the column it opens. */
  .icon-button.active::before {
    content: '';
    position: absolute;
    right: 0;
    top: 6px;
    bottom: 6px;
    width: 2px;
    border-radius: 2px 0 0 2px;
    background: #4bf3c8;
  }

  .icon-button:focus-visible {
    outline: 1px solid #bd93f9;
    outline-offset: -3px;
  }
</style>
