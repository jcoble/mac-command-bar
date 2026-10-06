<script lang="ts">
  /**
   * RightPanelTabs.svelte — the tab row across the top of the right drawer.
   *
   * One rounded island of icon tabs, Codex style: four permanent ones (Files,
   * Source control, Agents, Tasks), then the one ⋯-menu tab last picked
   * (Worktrees, Run, Context or History; the next pick replaces it); outside
   * the island, the ⋯ menu and the drawer's close button. Tabs are
   * not closable; only the drawer closes.
   * PRESENTATIONAL ONLY: the selected tab is handed in and every click is
   * handed back out.
   */
  import BotMessageSquare from '@lucide/svelte/icons/bot-message-square';
  import CirclePlay from '@lucide/svelte/icons/circle-play';
  import Ellipsis from '@lucide/svelte/icons/ellipsis';
  import FolderTree from '@lucide/svelte/icons/folder-tree';
  import Gauge from '@lucide/svelte/icons/gauge';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import GitFork from '@lucide/svelte/icons/git-fork';
  import History from '@lucide/svelte/icons/history';
  import ListChecks from '@lucide/svelte/icons/list-checks';
  import X from '@lucide/svelte/icons/x';

  import { Button, buttonVariants } from '$lib/components/ui/button/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import * as Tabs from '$lib/components/ui/tabs/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import type { RightExtraTabId } from '$lib/shell/controllers/workbenchController.svelte';
  import type { RightTabId } from '$lib/shell/workbenchNavigation';
  import { cn } from '$lib/utils.js';

  interface Props {
    activeId: RightTabId;
    extraId: RightExtraTabId | null;
    onSelect(id: RightTabId): void;
    onClose(): void;
  }
  let { activeId, extraId, onSelect, onClose }: Props = $props();

  const PERMANENT = [
    { id: 'files', label: 'Files', icon: FolderTree },
    { id: 'source-control', label: 'Source control', icon: GitBranch },
    { id: 'agents', label: 'Agents', icon: BotMessageSquare },
    { id: 'tasks', label: 'Tasks', icon: ListChecks }
  ] as const;

  const EXTRA = [
    { id: 'worktrees', label: 'Worktrees', icon: GitFork },
    { id: 'run', label: 'Run', icon: CirclePlay },
    { id: 'context', label: 'Context', icon: Gauge },
    { id: 'history', label: 'History', icon: History }
  ] as const;

  const ROUND = 'size-7 rounded-full p-0 text-muted-foreground [&_svg]:size-4';
  // The active tab is a lighter filled pill inside the island; no underline.
  const PILL =
    'size-7 flex-none rounded-full border-transparent p-0 text-muted-foreground hover:bg-foreground/6 [&_svg]:size-4 data-active:bg-foreground/12 data-active:shadow-sm dark:data-active:border-transparent dark:data-active:bg-foreground/12';

  const shown = $derived([...PERMANENT, ...EXTRA.filter((tab) => tab.id === extraId)]);
</script>

<div class="flex h-10 min-w-0 flex-none items-center gap-1 px-1.5">
  <Tabs.Root value={activeId} onValueChange={(id) => onSelect(id as RightTabId)} class="min-w-0 flex-1">
    <Tabs.List class="gap-0.5 rounded-full bg-foreground/7 p-1 group-data-[orientation=horizontal]/tabs:h-9" aria-label="Side panel">
      {#each shown as tab (tab.id)}
        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <Tabs.Trigger {...props} value={tab.id} aria-label={tab.label} class={PILL}>
                <tab.icon strokeWidth={1.75} aria-hidden="true" />
              </Tabs.Trigger>
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Content side="bottom" align="start">{tab.label}</Tooltip.Content>
        </Tooltip.Root>
      {/each}
    </Tabs.List>
  </Tabs.Root>

  <!-- The menu opens down and to the left, inside the drawer: anything drawn
       past its edge would sit under the live browser page beside it. -->
  <DropdownMenu.Root>
    <DropdownMenu.Trigger class={cn(buttonVariants({ variant: 'ghost' }), ROUND)} aria-label="More panels">
      <Ellipsis strokeWidth={1.75} aria-hidden="true" />
    </DropdownMenu.Trigger>
    <DropdownMenu.Content side="bottom" align="end" class="w-44">
      {#each EXTRA as tab (tab.id)}
        <DropdownMenu.Item onSelect={() => onSelect(tab.id)}>
          <tab.icon strokeWidth={1.75} aria-hidden="true" />
          {tab.label}
        </DropdownMenu.Item>
      {/each}
    </DropdownMenu.Content>
  </DropdownMenu.Root>
  <Button variant="ghost" class={ROUND} aria-label="Close side panel" onclick={onClose}>
    <X strokeWidth={1.75} aria-hidden="true" />
  </Button>
</div>
