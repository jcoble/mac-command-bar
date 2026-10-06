<script lang="ts" module>
  import type { TopTabKind } from '$lib/shell/layout/topTabsOps';

  /** One tab as the row draws it. Built by the page from the owning stores. */
  export interface TopTabView {
    key: string;
    kind: TopTabKind;
    label: string;
    /** Editor tabs only: the file's absolute path. */
    path?: string;
    dirty?: boolean;
    /** Editor tabs only: a preview file, replaced by the next one opened. */
    preview?: boolean;
  }
</script>

<script lang="ts">
  /**
   * TopTabRow.svelte — the window chrome's tab row (Codex style).
   *
   * Custom because shadcn has no closable, scrollable row of mixed tabs; every
   * control inside it is stock shadcn. The Chat tab is always first. While the
   * pane sits beside the chat it is the title over the chat column; while the
   * pane is expanded it is the first tab, and clicking it collapses the pane.
   * After it: one tab per open file, browser page, Changes, History and Pull
   * requests, then `+` (new browser tab), expand, and the drawer toggle.
   *
   * PRESENTATIONAL ONLY: the tabs and the active key are handed in, and every
   * click is handed back out.
   */
  import { untrack } from 'svelte';
  import FileDiff from '@lucide/svelte/icons/file-diff';
  import GitCommitHorizontal from '@lucide/svelte/icons/git-commit-horizontal';
  import GitPullRequest from '@lucide/svelte/icons/git-pull-request';
  import Globe from '@lucide/svelte/icons/globe';
  import Maximize2 from '@lucide/svelte/icons/maximize-2';
  import MessageCircle from '@lucide/svelte/icons/message-circle';
  import Minimize2 from '@lucide/svelte/icons/minimize-2';
  import PanelRight from '@lucide/svelte/icons/panel-right';
  import Plus from '@lucide/svelte/icons/plus';
  import X from '@lucide/svelte/icons/x';

  import { Button, buttonVariants } from '$lib/components/ui/button/index.js';
  import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
  import { Separator } from '$lib/components/ui/separator/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import { cn } from '$lib/utils.js';
  import FileIcon from './explorer/FileIcon.svelte';
  import { revealTab, scrollTabStrip } from './tabScrolling';

  interface Props {
    tabs: TopTabView[];
    activeKey: string | null;
    expanded: boolean;
    /** The tab pane is in the frame (at least one tab is open). */
    paneOpen: boolean;
    chatTitle: string;
    drawerOpen: boolean;
    canOpenBrowser: boolean;
    onSelect(key: string): void;
    onClose(key: string): void;
    onSelectChat(): void;
    onNewBrowserTab(): void;
    onToggleExpanded(): void;
    onToggleDrawer(): void;
    editorActions: {
      pin(path: string): void;
      closeOthers(path: string): void;
      closeSaved(): void;
      timeline(path: string): void;
    };
  }
  let {
    tabs,
    activeKey,
    expanded,
    paneOpen,
    chatTitle,
    drawerOpen,
    canOpenBrowser,
    onSelect,
    onClose,
    onSelectChat,
    onNewBrowserTab,
    onToggleExpanded,
    onToggleDrawer,
    editorActions
  }: Props = $props();

  const ICONS = {
    browser: Globe,
    diff: FileDiff,
    'git-history': GitCommitHorizontal,
    'pull-requests': GitPullRequest
  } as const;

  let track = $state<HTMLElement | null>(null);

  // Collapsed, the chat column's title sits over the chat and the tabs over
  // the pane (and over the drawer beside it, when open); expanded, the Chat
  // tab leads the one row.
  const columns = $derived(
    expanded
      ? 'auto minmax(0, 1fr)'
      : `minmax(0, 1fr) ${paneOpen ? 'calc(var(--tools-rail-width) + var(--drawer-width, 0px))' : 'auto'}`
  );

  $effect(() => {
    activeKey;
    untrack(() => revealTab(track, track?.querySelector('[data-active="true"]') ?? null));
  });
</script>

{#snippet tabChip(tab: TopTabView, props: Record<string, unknown>)}
  {@const active = tab.key === activeKey}
  <div
    {...props}
    class={cn(
      'group flex h-8 w-38 min-w-24 shrink items-center gap-1.5 rounded-lg pr-1 pl-2.5 text-sm font-medium transition-colors',
      active ? 'bg-accent text-accent-foreground' : 'text-muted-foreground hover:bg-accent/50 hover:text-foreground'
    )}
    data-active={active}
  >
    <button
      type="button"
      role="tab"
      aria-selected={active}
      class="flex h-full min-w-0 flex-1 items-center gap-1.5 text-left outline-none"
      onclick={() => onSelect(tab.key)}
    >
      {#if tab.kind === 'editor'}
        <FileIcon fileName={tab.label} size={16} />
      {:else}
        {@const Icon = ICONS[tab.kind]}
        <Icon class="size-4 shrink-0" aria-hidden="true" />
      {/if}
      <span class={cn('truncate', tab.preview && 'italic')}>{tab.label}</span>
    </button>
    <Button
      variant="ghost"
      size="icon-xs"
      aria-label={`Close ${tab.label}${tab.dirty ? ' (unsaved)' : ''}`}
      class={cn(
        'shrink-0',
        active || tab.dirty ? 'opacity-100' : 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'
      )}
      onclick={() => onClose(tab.key)}
    >
      {#if tab.dirty}
        <span class="group-hover:hidden" aria-hidden="true">•</span>
        <X class="hidden group-hover:block" aria-hidden="true" />
      {:else}
        <X aria-hidden="true" />
      {/if}
    </Button>
  </div>
{/snippet}

<div
  class="grid h-full min-w-0 items-center"
  style:grid-template-columns={columns}
  data-tauri-drag-region
>
  <div class={cn('flex h-full min-w-0 items-center gap-1', expanded ? 'pl-2' : 'px-2')} data-tauri-drag-region>
    <button
      type="button"
      class={cn(
        'flex h-8 min-w-0 items-center gap-1.5 rounded-lg px-2.5 text-sm font-semibold text-foreground transition-colors',
        expanded && 'max-w-56 hover:bg-accent/50'
      )}
      aria-label={expanded ? `Chat: ${chatTitle}. Collapse the pane` : `Chat: ${chatTitle}`}
      onclick={onSelectChat}
    >
      {#if expanded}<MessageCircle class="size-4 shrink-0" aria-hidden="true" />{/if}
      <span class="truncate">{chatTitle}</span>
    </button>
    {#if expanded}<Separator orientation="vertical" class="h-4" />{/if}
  </div>

  <div
    class={cn('flex h-full min-w-0 items-center gap-1 px-2', paneOpen && !expanded && 'border-l border-border')}
    data-tauri-drag-region
  >
    <div
      bind:this={track}
      class="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto [scrollbar-width:none]"
      role="tablist"
      aria-label="Open tabs"
      use:scrollTabStrip
    >
      {#each tabs as tab (tab.key)}
        {#if tab.kind === 'editor' && tab.path}
          {@const path = tab.path}
          <!-- Opening the menu activates its tab first, so the menu never
               floats over a live browser page (a native view no DOM can cover). -->
          <ContextMenu.Root onOpenChange={(open) => { if (open) onSelect(tab.key); }}>
            <ContextMenu.Trigger>
              {#snippet child({ props })}
                {@render tabChip(tab, props)}
              {/snippet}
            </ContextMenu.Trigger>
            <ContextMenu.Content class="w-[220px]" aria-label={`Actions for ${tab.label}`}>
              {#if tab.preview}
                <ContextMenu.Item onSelect={() => editorActions.pin(path)}>Pin Tab</ContextMenu.Item>
              {/if}
              <ContextMenu.Item onSelect={() => onClose(tab.key)}>Close</ContextMenu.Item>
              <ContextMenu.Item
                disabled={!tabs.some((other) => other.kind === 'editor' && other.key !== tab.key && !other.dirty)}
                onSelect={() => editorActions.closeOthers(path)}
              >Close other clean files</ContextMenu.Item>
              <ContextMenu.Item
                disabled={!tabs.some((other) => other.kind === 'editor' && !other.dirty)}
                onSelect={editorActions.closeSaved}
              >Close saved files</ContextMenu.Item>
              <ContextMenu.Item onSelect={() => editorActions.timeline(path)}>File Timeline</ContextMenu.Item>
            </ContextMenu.Content>
          </ContextMenu.Root>
        {:else}
          {@render tabChip(tab, {})}
        {/if}
      {/each}
    </div>

    <div class="flex flex-none items-center gap-1">
      <Tooltip.Root>
        <Tooltip.Trigger
          class={buttonVariants({ variant: 'ghost', size: 'icon-sm' })}
          aria-label="New browser tab"
          disabled={!canOpenBrowser}
          onclick={onNewBrowserTab}
        >
          <Plus aria-hidden="true" />
        </Tooltip.Trigger>
        <Tooltip.Content side="bottom">New browser tab</Tooltip.Content>
      </Tooltip.Root>
      {#if paneOpen}
        <Tooltip.Root>
          <Tooltip.Trigger
            class={buttonVariants({ variant: 'ghost', size: 'icon-sm' })}
            aria-label={expanded ? 'Collapse the pane' : 'Expand the pane'}
            onclick={onToggleExpanded}
          >
            {#if expanded}<Minimize2 aria-hidden="true" />{:else}<Maximize2 aria-hidden="true" />{/if}
          </Tooltip.Trigger>
          <Tooltip.Content side="bottom">{expanded ? 'Collapse' : 'Expand'}</Tooltip.Content>
        </Tooltip.Root>
      {/if}
      <Tooltip.Root>
        <Tooltip.Trigger
          class={cn(buttonVariants({ variant: 'ghost', size: 'icon-sm' }), drawerOpen && 'bg-accent text-accent-foreground')}
          aria-label={drawerOpen ? 'Close side panel' : 'Open side panel'}
          aria-pressed={drawerOpen}
          onclick={onToggleDrawer}
        >
          <PanelRight aria-hidden="true" />
        </Tooltip.Trigger>
        <Tooltip.Content side="bottom">{drawerOpen ? 'Close side panel' : 'Open side panel'}</Tooltip.Content>
      </Tooltip.Root>
    </div>
  </div>
</div>
