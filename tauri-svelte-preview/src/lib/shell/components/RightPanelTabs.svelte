<script lang="ts">
  /**
   * RightPanelTabs.svelte — the tab row across the top of the right drawer.
   *
   * Four permanent icon tabs (Files, Source control, Agents, Tasks), then the
   * one ⋯-menu tab last picked (Worktrees, Run, Context or History; the next
   * pick replaces it), then the ⋯ menu and the drawer's close button. Tabs are
   * not closable; only the drawer closes.
   * PRESENTATIONAL ONLY: the selected tab is handed in and every click is
   * handed back out.
   */
  import Activity from '@lucide/svelte/icons/activity';
  import Bot from '@lucide/svelte/icons/bot';
  import Ellipsis from '@lucide/svelte/icons/ellipsis';
  import Files from '@lucide/svelte/icons/files';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import History from '@lucide/svelte/icons/history';
  import Layers from '@lucide/svelte/icons/layers';
  import ListTodo from '@lucide/svelte/icons/list-todo';
  import Play from '@lucide/svelte/icons/play';
  import X from '@lucide/svelte/icons/x';

  import { Button, buttonVariants } from '$lib/components/ui/button/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import * as Tabs from '$lib/components/ui/tabs/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import type { RightExtraTabId } from '$lib/shell/controllers/workbenchController.svelte';
  import type { RightTabId } from '$lib/shell/workbenchNavigation';

  interface Props {
    activeId: RightTabId;
    extraId: RightExtraTabId | null;
    onSelect(id: RightTabId): void;
    onClose(): void;
  }
  let { activeId, extraId, onSelect, onClose }: Props = $props();

  const PERMANENT = [
    { id: 'files', label: 'Files', icon: Files },
    { id: 'source-control', label: 'Source control', icon: GitBranch },
    { id: 'agents', label: 'Agents', icon: Bot },
    { id: 'tasks', label: 'Tasks', icon: ListTodo }
  ] as const;

  const EXTRA = [
    { id: 'worktrees', label: 'Worktrees', icon: Layers },
    { id: 'run', label: 'Run', icon: Play },
    { id: 'context', label: 'Context', icon: Activity },
    { id: 'history', label: 'History', icon: History }
  ] as const;

  const shown = $derived([...PERMANENT, ...EXTRA.filter((tab) => tab.id === extraId)]);
</script>

<div class="flex h-11 min-w-0 flex-none items-center gap-1 px-2">
  <Tabs.Root value={activeId} onValueChange={(id) => onSelect(id as RightTabId)} class="min-w-0 flex-1">
    <Tabs.List variant="line" aria-label="Side panel">
      {#each shown as tab (tab.id)}
        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <Tabs.Trigger {...props} value={tab.id} aria-label={tab.label} class="flex-none px-2">
                <tab.icon aria-hidden="true" />
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
    <DropdownMenu.Trigger class={buttonVariants({ variant: 'ghost', size: 'icon-sm' })} aria-label="More panels">
      <Ellipsis aria-hidden="true" />
    </DropdownMenu.Trigger>
    <DropdownMenu.Content side="bottom" align="end" class="w-44">
      {#each EXTRA as tab (tab.id)}
        <DropdownMenu.Item onSelect={() => onSelect(tab.id)}>
          <tab.icon aria-hidden="true" />
          {tab.label}
        </DropdownMenu.Item>
      {/each}
    </DropdownMenu.Content>
  </DropdownMenu.Root>
  <Button variant="ghost" size="icon-sm" aria-label="Close side panel" onclick={onClose}>
    <X aria-hidden="true" />
  </Button>
</div>
