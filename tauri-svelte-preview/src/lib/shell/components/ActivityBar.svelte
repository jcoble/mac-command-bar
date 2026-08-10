<script lang="ts">
  /**
   * ActivityBar.svelte — the horizontal line-tab picker above the right tool
   * pane. One icon per view; clicking one opens that view directly below it.
   * The gear is not a view — it opens the settings dialog.
   *
   * PRESENTATIONAL ONLY: no state, no IO, no knowledge of what a view contains.
   * The roster comes from `sidebarViews.ts` so the ids here and the ids the
   * column builds hosts for cannot drift apart.
   */
  import {
    Activity,
    Bot,
    Files,
    GitBranch,
    Layers,
    Play,
    Settings,
    TriangleAlert
  } from '@lucide/svelte';

  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import * as Tabs from '$lib/components/ui/tabs/index.js';
  import { settings } from '$lib/settingsStore.svelte';
  import {
    PROBLEMS_VIEW_ID,
    SIDEBAR_VIEWS,
    isSidebarViewId,
    type SidebarViewId
  } from '$lib/shell/layout/sidebarViews';

  interface Props {
    activeId: SidebarViewId;
    onSelect(id: SidebarViewId): void;
    onOpenSettings(): void;
  }
  let { activeId, onSelect, onOpenSettings }: Props = $props();

  /** Icon per view. Keyed by id so adding a view to the roster without an icon
   * is a type error rather than a blank tab. */
  const ICONS: Record<SidebarViewId, typeof Files> = {
    explorer: Files,
    'source-control': GitBranch,
    worktrees: Layers,
    stacks: Play,
    context: Activity,
    agents: Bot,
    problems: TriangleAlert
  };

  /** Problems is always mounted by ShellSidebar, but it is offered here only
   * when settings place it in the right pane rather than the bottom dock. */
  const shownViews = $derived(
    SIDEBAR_VIEWS.filter(
      (view) => view.id !== PROBLEMS_VIEW_ID || settings.panels.problemsLocation === 'right'
    )
  );

  function selectView(value: string): void {
    if (isSidebarViewId(value)) onSelect(value);
  }
</script>

<nav class="activity-tabs" aria-label="Right pane">
  <Tabs.Root value={activeId} onValueChange={selectView} class="right-pane-tabs">
    <Tabs.List variant="line" class="right-pane-tab-list" aria-label="Right pane views">
      {#each shownViews as view (view.id)}
        {@const Icon = ICONS[view.id]}
        <Tabs.Trigger
          value={view.id}
          class="right-pane-tab"
          title={view.title}
          aria-label={view.title}
        >
          <Icon class="size-4" strokeWidth={1.6} aria-hidden="true" />
        </Tabs.Trigger>
      {/each}
    </Tabs.List>
  </Tabs.Root>

  <span class="settings-action">
    <IconButton
      label="Settings"
      size="sm"
      side="bottom"
      class="text-muted-foreground"
      onclick={() => onOpenSettings()}
    >
      <Settings class="size-4" strokeWidth={1.6} aria-hidden="true" />
    </IconButton>
  </span>
</nav>

<style>
  .activity-tabs {
    display: flex;
    width: 100%;
    height: 40px;
    min-width: 0;
    align-items: center;
    gap: 4px;
    padding: 4px 6px 4px 4px;
    border-bottom: 1px solid var(--border);
    background: var(--background);
    user-select: none;
  }

  :global(.right-pane-tabs) {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
  }

  :global(.right-pane-tab-list) {
    width: 100%;
    height: 32px;
    min-width: 0;
    justify-content: flex-start;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
  }

  :global(.right-pane-tab-list::-webkit-scrollbar) {
    display: none;
  }

  :global(.right-pane-tab) {
    flex: 0 0 30px;
    width: 30px;
    min-width: 30px;
    padding-right: 6px;
    padding-left: 6px;
  }

  .settings-action {
    display: grid;
    flex: 0 0 32px;
    width: 32px;
    height: 28px;
    padding-left: 4px;
    border-left: 1px solid color-mix(in srgb, var(--border) 58%, transparent);
    place-items: center end;
  }
</style>
