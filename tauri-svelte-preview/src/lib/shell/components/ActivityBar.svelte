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
    Bot,
    ChartNoAxesCombined,
    Cpu,
    Files,
    GitBranch,
    Layers,
    Play,
    Settings,
    TriangleAlert
  } from '@lucide/svelte';

  import { settings } from '$lib/settingsStore.svelte';
  import ResourcePopover from '$lib/shell/resources/ResourcePopover.svelte';
  import UsagePopover from '$lib/shell/usage/UsagePopover.svelte';
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
    agents: Bot,
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

  <div class="group utility-group" aria-label="Workspace meters and settings">
    <div class="utility-action" title="Resources">
      <Cpu size={19} strokeWidth={1.6} aria-hidden="true" />
      <ResourcePopover />
    </div>
    <div class="utility-action" title="Stats and Usage">
      <ChartNoAxesCombined size={19} strokeWidth={1.6} aria-hidden="true" />
      <UsagePopover />
    </div>
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
    background: var(--color-bg);
    /* The strip is on the outer edge now, so its hairline faces the column it
       opens rather than the middle of the shell. */
    border-left: 1px solid var(--color-border);
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
    color: var(--color-text-3);
    cursor: pointer;
  }

  .icon-button:hover {
    color: var(--color-text);
  }

  .icon-button.active {
    color: var(--color-text);
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
    background: var(--color-accent);
  }

  .icon-button:focus-visible {
    outline: 2px solid var(--color-focus-solid);
    outline-offset: -3px;
  }

  .utility-group {
    padding-top: 6px;
    border-top: 1px solid color-mix(in srgb, var(--color-border) 58%, transparent);
  }

  .utility-action {
    position: relative;
    display: grid;
    width: 44px;
    height: 40px;
    place-items: center;
    color: var(--color-text-3);
  }

  .utility-action:hover,
  .utility-action:has(:global(.trigger[aria-expanded='true'])) {
    color: var(--color-text);
  }

  .utility-action :global(aside) {
    position: absolute;
    inset: 0;
    z-index: 2;
    width: 44px;
    height: 40px;
  }

  .utility-action :global(.trigger) {
    position: absolute;
    inset: 0;
    width: 44px;
    height: 40px;
    padding: 0;
    border-radius: 0;
    background: transparent;
    color: transparent;
    font-size: 0;
  }

  .utility-action :global(.trigger span) {
    display: none;
  }

  .utility-action:has(:global(.trigger:focus-visible)) {
    outline: 2px solid var(--color-focus-solid);
    outline-offset: -3px;
  }

  /* The far-right strip opens its compact stats card inward and upward, never
     beyond the window edge. The full stats workspace remains a fixed modal. */
  .utility-action :global(.usage-popover .card) {
    top: auto;
    right: calc(100% + 10px);
    bottom: 0;
  }
</style>
