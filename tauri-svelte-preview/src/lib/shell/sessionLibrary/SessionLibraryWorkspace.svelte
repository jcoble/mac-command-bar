<script lang="ts">
  import { onMount } from 'svelte';

  import Activity from '@lucide/svelte/icons/activity';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import FolderGit2 from '@lucide/svelte/icons/folder-git-2';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import MoreHorizontal from '@lucide/svelte/icons/ellipsis';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Search from '@lucide/svelte/icons/search';

  import { Badge } from '$lib/components/ui/badge/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import * as Collapsible from '$lib/components/ui/collapsible/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import * as Select from '$lib/components/ui/select/index.js';
  import type { AgentSession } from '$lib/tauriSource';
  import type { OwnedSession } from '$lib/shell/ownedSessions';
  import { formatLastActivity } from '$lib/shell/relativeTime';
  import {
    buildSessionHistoryViewModel,
    createSessionHistoryCollapseState,
    createSessionHistoryWindowState,
    extendSessionHistoryWindow,
    isSessionHistoryGroupOpen,
    resetSessionHistoryWindowOnFilterChange,
    toggleSessionHistoryGroup,
    type SessionHistoryCollapseState,
    type SessionHistoryRow,
    type SessionHistoryWindowState,
    type SessionHistoryWorktreeGroup
  } from '$lib/shell/history/sessionHistoryViewModel';
  import {
    sessionContextMenuRoster,
    type SessionContextMenuAction,
    type SessionContextMenuItem
  } from './sessionLibraryContextMenu';
  import { buildSessionLibrary, type SessionLibraryRecord } from './sessionLibraryModel';
  import { registerSessionLibraryOpenHandler } from './sessionLibraryNavigation';
  import {
    inertSessionLibraryService,
    type SessionLibraryAction,
    type SessionLibraryService
  } from './sessionLibraryService';
  import { sessionLibraryState, type SessionLibraryUiState } from './sessionLibraryStore.svelte';
  import SessionContextMenu from './SessionContextMenu.svelte';

  const ALL_PROVIDERS = '__all_providers__';

  interface Props {
    owned: OwnedSession[];
    available: AgentSession[];
    service?: SessionLibraryService;
    store?: SessionLibraryUiState;
    workspacePath?: string | null;
    projectPath?: string | null;
    visible?: boolean;
    onOpenTab?(): void;
    onRefresh?(): void | Promise<void>;
  }

  type ContextMenuState = {
    target: 'session' | 'center-tab';
    record: SessionLibraryRecord | null;
    anchor: {
      left: number;
      right: number;
      top: number;
      bottom: number;
      containingBlockLeft?: number;
      containingBlockRight?: number;
      containingBlockTop?: number;
      containingBlockBottom?: number;
    };
    items: SessionContextMenuItem[];
  };

  let {
    owned,
    available,
    service = inertSessionLibraryService,
    store = sessionLibraryState,
    visible = true,
    onOpenTab,
    onRefresh
  }: Props = $props();

  let collapseState = $state<SessionHistoryCollapseState>(createSessionHistoryCollapseState());
  let windowState = $state<SessionHistoryWindowState>(createSessionHistoryWindowState());
  let hasRendered = $state(false);
  /** Rows the reader has opened to see everything the history holds on them. */
  let expandedRows = $state<string[]>([]);
  let contextMenu = $state<ContextMenuState | null>(null);
  let now = $state(new Date());

  const records = $derived(buildSessionLibrary(owned, available));
  const history = $derived(buildSessionHistoryViewModel(records, {
    query: store.query,
    provider: store.provider,
    windowState
  }));

  $effect(() => {
    if (visible) hasRendered = true;
  });

  $effect(() => {
    const reset = resetSessionHistoryWindowOnFilterChange(windowState, {
      query: store.query,
      provider: store.provider
    });
    if (reset !== windowState) windowState = reset;
  });

  $effect(() => {
    if (!visible) return;
    const timer = window.setInterval(() => (now = new Date()), 60_000);
    return () => window.clearInterval(timer);
  });

  function setQuery(value: string): void {
    store.query = value;
    store.page = 1;
  }

  function setProvider(value: unknown): void {
    store.provider = typeof value === 'string' && value !== ALL_PROVIDERS ? value : '';
    store.page = 1;
  }

  function clearFilters(): void {
    store.query = '';
    store.provider = '';
    store.page = 1;
  }

  function providerFilterLabel(): string {
    if (!store.provider) return 'All providers';
    return history.providers.find((provider) => provider.value === store.provider)?.label ?? store.provider;
  }

  function rowIsExpanded(key: string): boolean {
    return expandedRows.includes(key);
  }

  function toggleRow(key: string): void {
    expandedRows = rowIsExpanded(key)
      ? expandedRows.filter((candidate) => candidate !== key)
      : [...expandedRows, key];
  }

  function showMore(worktreeKey: string): void {
    windowState = extendSessionHistoryWindow(windowState, worktreeKey);
  }

  /**
   * Everything the history already knows about a session, in reading order.
   * Nothing here is fetched — a row that has no value for a line drops it.
   */
  function rowDetails(row: SessionHistoryRow): Array<{ label: string; value: string }> {
    const record = row.record;
    const lines: Array<{ label: string; value: string | null | undefined }> = [
      { label: 'Title', value: record.title || row.displayTitle },
      { label: 'Project', value: record.projectPath },
      { label: 'Worktree', value: record.canonicalCwd },
      { label: 'Branch', value: record.owned?.branch },
      { label: 'Provider', value: row.providerLabel },
      { label: 'Model', value: record.model },
      { label: 'Session id', value: sessionId(record) },
      { label: 'Turns', value: countLabel(record) },
      { label: 'Last activity', value: record.lastActivity || record.updatedAt },
      { label: 'First prompt', value: record.firstPrompt || row.excerpt }
    ];
    return lines
      .map((line) => ({ label: line.label, value: (line.value ?? '').trim() }))
      .filter((line) => line.value !== '');
  }

  function setGroupOpen(level: 'project' | 'worktree', key: string, open: boolean): void {
    if (isSessionHistoryGroupOpen(collapseState, level, key) === open) return;
    collapseState = toggleSessionHistoryGroup(collapseState, level, key);
  }

  async function runAction(action: SessionLibraryAction, record: SessionLibraryRecord): Promise<void> {
    store.selectedKey = record.key;
    await service[action](record);
  }

  async function openRow(record: SessionLibraryRecord): Promise<void> {
    await runAction('open', record);
  }

  function countLabel(record: SessionLibraryRecord): string {
    if (!record.messageCount || record.messageCount < 1) return 'No indexed turns';
    return record.messageCount === 1 ? '1 turn' : `${record.messageCount} turns`;
  }

  function ageLabel(record: SessionLibraryRecord): string {
    return formatLastActivity(record.updatedAt, now) || 'Activity unknown';
  }

  function sessionId(record: SessionLibraryRecord): string {
    return record.nativeSessionId || record.ownedId || record.key;
  }

  async function copyText(value: string): Promise<void> {
    if (!value || typeof navigator === 'undefined' || !navigator.clipboard?.writeText) return;
    await navigator.clipboard.writeText(value);
  }

  function menuPoint(event: MouseEvent): ContextMenuState['anchor'] {
    const target = event.currentTarget instanceof HTMLElement ? event.currentTarget : null;
    const anchor = target;
    const rect = anchor?.getBoundingClientRect();
    const containingBlock = target?.closest<HTMLElement>('.dv-render-overlay');
    const containingBlockRect = containingBlock?.getBoundingClientRect();
    const containingBlockLeft = containingBlockRect?.left ?? 0;
    const containingBlockRight = containingBlockRect?.right ?? window.innerWidth;
    const containingBlockTop = containingBlockRect?.top ?? 0;
    const containingBlockBottom = containingBlockRect?.bottom ?? window.innerHeight;
    return rect
      ? {
          left: rect.left,
          right: rect.right,
          top: rect.top,
          bottom: rect.bottom,
          containingBlockLeft,
          containingBlockRight,
          containingBlockTop,
          containingBlockBottom
        }
      : {
          left: event.clientX,
          right: event.clientX,
          top: event.clientY,
          bottom: event.clientY,
          containingBlockLeft,
          containingBlockRight,
          containingBlockTop,
          containingBlockBottom
        };
  }

  function openRowMenu(record: SessionLibraryRecord, event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();
    store.selectedKey = record.key;
    contextMenu = {
      target: 'session',
      record,
      anchor: menuPoint(event),
      items: sessionContextMenuRoster({ target: 'session', record })
    };
  }

  function openCenterTabMenu(event: MouseEvent): void {
    const target = event.target as HTMLElement | null;
    const tab = target?.closest('.dv-tab');
    if (!tab) return;
    const label = (tab.textContent ?? '').replace(/\s+/g, ' ').trim();
    if (!/Session History|Session Library/i.test(label)) return;
    event.preventDefault();
    contextMenu = {
      target: 'center-tab',
      record: null,
      anchor: menuPoint(event),
      items: sessionContextMenuRoster({ target: 'center-tab' })
    };
  }

  async function selectContextAction(action: SessionContextMenuAction): Promise<void> {
    const current = contextMenu;
    contextMenu = null;
    if (!current) return;
    const record = current.record;
    if (!record) {
      if (action === 'copy-id') await copyText('session-library');
      return;
    }
    if (action === 'resume-worktree') await runAction('resume', record);
    else if (action === 'copy-id') await copyText(sessionId(record));
    else if (action === 'archive') await runAction('archive', record);
    else if (action === 'delete') await runAction('delete', record);
  }

  function closeContextMenu(): void {
    contextMenu = null;
  }

  onMount(() => {
    const releaseOpenHandler = registerSessionLibraryOpenHandler(() => onOpenTab?.());
    const onContextMenu = (event: Event): void => {
      if (event instanceof MouseEvent) openCenterTabMenu(event);
    };
    const onPointerDown = (event: Event): void => {
      const target = event.target as HTMLElement | null;
      if (target?.closest('[data-testid="session-context-menu"]')) return;
      closeContextMenu();
    };
    document.addEventListener('contextmenu', onContextMenu, true);
    document.addEventListener('pointerdown', onPointerDown, true);
    return () => {
      releaseOpenHandler();
      document.removeEventListener('contextmenu', onContextMenu, true);
      document.removeEventListener('pointerdown', onPointerDown, true);
    };
  });
</script>

{#snippet sessionRow(row: SessionHistoryRow)}
  {@const open = rowIsExpanded(row.record.key)}
  <article
    data-testid="session-history-row"
    data-session-key={row.record.key}
    data-expanded={open ? 'true' : 'false'}
    class="group grid min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-1 rounded-lg
           border-t border-border/60 first:border-t-0 hover:bg-accent/45 focus-within:bg-accent/45"
    role="listitem"
    oncontextmenu={(event) => openRowMenu(row.record, event)}
  >
    <span data-testid="session-history-row-expand" class="ml-1 inline-flex">
      <IconButton
        label={open ? 'Hide session details' : 'Show session details'}
        size="sm"
        side="right"
        onclick={() => toggleRow(row.record.key)}
      >
        {#if open}
          <ChevronDown class="size-4" aria-hidden="true" />
        {:else}
          <ChevronRight class="size-4" aria-hidden="true" />
        {/if}
      </IconButton>
    </span>
    <Button
      data-testid="session-history-row-trigger"
      variant="ghost"
      class="h-auto min-w-0 justify-start rounded-lg px-2 py-2.5 text-left text-[13px] font-normal"
      onclick={() => void openRow(row.record)}
    >
      <span class="flex min-w-0 flex-1 flex-col gap-1.5">
        <span
          data-testid="session-history-row-title"
          class="block truncate text-[13px] font-medium text-foreground"
          title={row.displayTitle}
        >{row.displayTitle}</span>
        <span class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-[12px] text-muted-foreground">
          <Badge data-testid="session-history-provider-badge" variant="outline">{row.providerLabel}</Badge>
          <span data-testid="session-history-message-count">{countLabel(row.record)}</span>
          <span data-testid="session-history-age" title={row.record.updatedAt ?? undefined}>{ageLabel(row.record)}</span>
          {#if row.statusHint}
            <Badge data-testid="session-history-status" variant="secondary">
              <Activity class="size-3" aria-hidden="true" />{row.statusHint}
            </Badge>
          {/if}
        </span>
      </span>
    </Button>
    <span data-testid="session-history-row-menu" class="mr-1 inline-flex">
      <IconButton
        label={`More actions for ${row.displayTitle}`}
        size="sm"
        class="opacity-70 group-hover:opacity-100 group-focus-within:opacity-100"
        side="left"
        onclick={(event) => openRowMenu(row.record, event)}
      >
        <MoreHorizontal class="size-4" aria-hidden="true" />
      </IconButton>
    </span>

    {#if open}
      <dl
        data-testid="session-history-row-details"
        class="col-span-3 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1 px-3 pt-0 pb-2.5
               text-[13px] leading-5"
      >
        {#each rowDetails(row) as line (line.label)}
          <dt class="text-muted-foreground">{line.label}</dt>
          <dd class="min-w-0 break-words text-foreground">{line.value}</dd>
        {/each}
      </dl>
    {/if}
  </article>
{/snippet}

{#snippet showMoreRows(worktree: SessionHistoryWorktreeGroup)}
  {#if worktree.olderCount > 0}
    <Button
      data-testid="session-history-show-more"
      variant="ghost"
      size="sm"
      class="mt-1 w-full justify-center text-muted-foreground"
      onclick={() => showMore(worktree.key)}
    >Show more — {worktree.olderCount} older</Button>
  {/if}
{/snippet}

{#if !hasRendered}
  <div
    data-testid="session-history-placeholder"
    class="h-full min-h-0 bg-background"
    aria-hidden="true"
  ></div>
{:else}
<section
  data-testid="session-library-workspace"
  data-session-history-workspace="true"
  class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden bg-background text-foreground"
  aria-label="Agent Session History"
>
  <header data-testid="session-history-header" class="shrink-0 border-b px-4 py-3.5">
    <div class="flex items-start justify-between gap-4">
      <div class="min-w-0">
        <div class="flex items-center gap-2">
          <h2 data-testid="session-library-title" class="truncate text-[17px] font-semibold tracking-[-0.012em]">
            Session History
          </h2>
          <Badge data-testid="session-history-total-count" variant="secondary">{history.totalCount}</Badge>
        </div>
        <p data-testid="session-history-host-line" class="mt-1 text-[13px] text-muted-foreground">
          {history.totalCount} {history.totalCount === 1 ? 'session' : 'sessions'} indexed on this Mac
        </p>
      </div>
      <span data-testid="session-history-refresh" class="inline-flex">
        <IconButton
          label="Refresh session history"
          onclick={() => void onRefresh?.()}
        >
          <RefreshCw class="size-4" aria-hidden="true" />
        </IconButton>
      </span>
    </div>

    <div class="mt-3 grid grid-cols-[minmax(0,1fr)_minmax(148px,auto)] gap-2">
      <div class="relative min-w-0">
        <Search class="pointer-events-none absolute top-1/2 left-2.5 z-[1] size-4 -translate-y-1/2 text-muted-foreground" aria-hidden="true" />
        <Input
          data-testid="session-history-search"
          type="search"
          value={store.query}
          class="pl-8"
          placeholder="Search title, message, project, or worktree"
          aria-label="Search session history"
          oninput={(event) => setQuery(event.currentTarget.value)}
        />
      </div>

      <Select.Root
        type="single"
        value={store.provider || ALL_PROVIDERS}
        onValueChange={setProvider}
      >
        <Select.Trigger
          data-testid="session-history-provider"
          class="w-full min-w-[148px]"
          aria-label="Filter by provider"
        >
          {providerFilterLabel()}
        </Select.Trigger>
        <Select.Content align="end">
          <Select.Item value={ALL_PROVIDERS} label="All providers" />
          {#each history.providers as provider (provider.value)}
            <Select.Item value={provider.value} label={provider.label} />
          {/each}
        </Select.Content>
      </Select.Root>
    </div>
  </header>

  <div data-testid="session-history-list" class="min-h-0 flex-1 overflow-y-auto px-3 py-2" role="list">
    {#if history.projects.length === 0}
      <div data-testid="session-history-empty" class="mx-auto flex max-w-[440px] flex-col items-center px-5 py-12 text-center">
        <FolderGit2 class="mb-3 size-7 text-muted-foreground" aria-hidden="true" />
        {#if records.length === 0}
          <p class="text-[13px] font-medium text-foreground">No session history yet</p>
          <p class="mt-1 text-[13px] leading-5 text-muted-foreground">
            Refresh after starting or importing a session on this Mac.
          </p>
        {:else}
          <p class="text-[13px] font-medium text-foreground">No sessions match this view</p>
          <p class="mt-1 text-[13px] leading-5 text-muted-foreground">
            Clear the search or show every provider to see the full history.
          </p>
          <Button class="mt-3" size="sm" variant="secondary" onclick={clearFilters}>Clear filters</Button>
        {/if}
      </div>
    {:else}
      {#each history.projects as project (project.key)}
        {@const projectOpen = isSessionHistoryGroupOpen(collapseState, 'project', project.key)}
        <Collapsible.Root
          data-testid="session-history-project"
          data-project-path={project.path}
          open={projectOpen}
          onOpenChange={(open) => setGroupOpen('project', project.key, open)}
          class="border-b last:border-b-0"
        >
          <Collapsible.Trigger
            class="flex min-h-9 w-full min-w-0 items-center gap-2 rounded-lg px-2 text-left text-[13px]
                   text-foreground transition-colors hover:bg-accent/60 focus-visible:ring-3
                   focus-visible:ring-ring/50 outline-none"
            title={project.path}
          >
            {#if projectOpen}
              <ChevronDown class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
            {:else}
              <ChevronRight class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
            {/if}
            <FolderGit2 class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
            <span class="truncate font-medium">{project.name}</span>
            <Badge data-testid="session-history-project-count" variant="secondary" class="ml-auto">
              {project.count}
            </Badge>
          </Collapsible.Trigger>

          <Collapsible.Content>
            {#each project.worktrees as worktree (worktree.key)}
              {#if project.singleCheckout}
                <div data-testid="session-history-single-checkout" data-worktree-path={worktree.path} class="pb-2 pl-3">
                  {#each worktree.rows as row (row.record.key)}
                    {@render sessionRow(row)}
                  {/each}
                  {@render showMoreRows(worktree)}
                </div>
              {:else}
                {@const worktreeOpen = isSessionHistoryGroupOpen(collapseState, 'worktree', worktree.key)}
                <Collapsible.Root
                  data-testid="session-history-worktree"
                  data-worktree-path={worktree.path}
                  open={worktreeOpen}
                  onOpenChange={(open) => setGroupOpen('worktree', worktree.key, open)}
                  class="ml-3 border-l border-border/70 pl-2"
                >
                  <Collapsible.Trigger
                    class="flex min-h-8 w-full min-w-0 items-center gap-2 rounded-lg px-2 text-left text-[13px]
                           text-muted-foreground transition-colors hover:bg-accent/60 hover:text-foreground
                           focus-visible:ring-3 focus-visible:ring-ring/50 outline-none"
                    title={worktree.path}
                  >
                    {#if worktreeOpen}
                      <ChevronDown class="size-3.5 shrink-0" aria-hidden="true" />
                    {:else}
                      <ChevronRight class="size-3.5 shrink-0" aria-hidden="true" />
                    {/if}
                    <GitBranch class="size-3.5 shrink-0" aria-hidden="true" />
                    <span class="truncate">{worktree.name}</span>
                    <Badge data-testid="session-history-worktree-count" variant="outline" class="ml-auto">
                      {worktree.count}
                    </Badge>
                  </Collapsible.Trigger>

                  <Collapsible.Content class="pb-2 pl-3">
                    {#each worktree.rows as row (row.record.key)}
                      {@render sessionRow(row)}
                    {/each}
                    {@render showMoreRows(worktree)}
                  </Collapsible.Content>
                </Collapsible.Root>
              {/if}
            {/each}
          </Collapsible.Content>
        </Collapsible.Root>
      {/each}
    {/if}
  </div>
</section>

{#if contextMenu}
  <SessionContextMenu
    anchor={contextMenu.anchor}
    items={contextMenu.items}
    onSelect={(action) => void selectContextAction(action)}
    onClose={closeContextMenu}
  />
{/if}
{/if}
