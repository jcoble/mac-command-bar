<script lang="ts">
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import FileText from '@lucide/svelte/icons/file-text';
  import List from '@lucide/svelte/icons/list';
  import ListTodo from '@lucide/svelte/icons/list-todo';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Search from '@lucide/svelte/icons/search';
  import Settings2 from '@lucide/svelte/icons/settings-2';
  import { onDestroy, onMount, tick, untrack } from 'svelte';

  import { Button } from '$lib/components/ui/button/index.js';
  import { buttonVariants } from '$lib/components/ui/button/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { FilterPills } from '$lib/components/ui/filter-pills/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { PanelHeader } from '$lib/components/ui/panel-header/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import * as Select from '$lib/components/ui/select/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import {
    clearNotionTaskSettings,
    listNotionTasks,
    readNotionTaskSettings,
    refreshNotionTasks,
    saveNotionTaskSettings,
    type NotionTaskRow,
    type NotionTaskSettings
  } from '$lib/shell/notionTasks.ts';
  import { connectNotion } from '$lib/shell/notionOAuth.ts';
  import { matchNotionProject } from '$lib/shell/notionProjectMatch.ts';
  import { cn } from '$lib/utils.js';
  import NotionTaskViewer from './NotionTaskViewer.svelte';

  const PAGE_SIZE = 25;
  const TASK_ROW_HEIGHT = 48;
  const TASK_ROW_OVERSCAN = 5;
  /* The Sessions rail's header buttons: quiet glyphs that turn to the text colour on hover. */
  const ACTION_CLASS =
    'text-[var(--color-text-2)] hover:text-foreground hover:bg-[var(--pill-surface-hover)]';

  function taskStatusIconClass(status: string): string {
    switch (status.trim().toLowerCase()) {
      case 'done':
      case 'complete':
      case 'completed':
        return 'text-[var(--color-good)]';
      case 'doing':
      case 'in progress':
      case 'working':
        return 'text-[var(--color-live)]';
      case 'blocked':
        return 'text-[var(--color-bad)]';
      case 'todo':
      case 'to do':
      case 'not started':
        return 'text-[var(--color-attention)]';
      default:
        return 'text-[var(--color-idle)]';
    }
  }

  /** The active session's folder; the list follows the Notion project it maps to. */
  let { root = '' }: { root?: string } = $props();

  let settings = $state<NotionTaskSettings | null>(null);
  let tasks = $state<NotionTaskRow[]>([]);
  let projects = $state<string[]>([]);
  let statuses = $state<string[]>([]);
  let priorities = $state<string[]>([]);
  /** Selected pill values per group id: `status` and `priority`. Empty means unfiltered. */
  let filters = $state<Record<string, string[]>>({});
  let searchDraft = $state('');
  let search = $state('');
  let projectFilter = $state('');
  let sortBy = $state('taskNumber');
  let sortDirection = $state('desc');
  let searchOpen = $state(false);
  let searchInput = $state<HTMLInputElement | null>(null);
  let loading = $state(true);
  let refreshing = $state(false);
  let saving = $state(false);
  let error = $state('');
  let hasMore = $state(false);
  let showSetup = $state(false);
  let dataSourceId = $state('');
  let token = $state('');
  let oauthController: AbortController | null = null;
  let selectedTask = $state<NotionTaskRow | null>(null);
  let taskViewport = $state<HTMLElement | null>(null);
  let taskLoadSentinel = $state<HTMLElement | null>(null);
  let taskScrollTop = $state(0);
  let taskViewportHeight = $state(800);
  let generation = 0;
  let readGeneration = 0;

  const firstTaskIndex = $derived(
    Math.max(0, Math.floor(taskScrollTop / TASK_ROW_HEIGHT) - TASK_ROW_OVERSCAN)
  );
  const lastTaskIndex = $derived(
    Math.min(
      tasks.length,
      Math.ceil((taskScrollTop + taskViewportHeight) / TASK_ROW_HEIGHT) + TASK_ROW_OVERSCAN
    )
  );
  const visibleTasks = $derived(tasks.slice(firstTaskIndex, lastTaskIndex));
  const sessionProject = $derived(matchNotionProject(root, projects));
  const filterGroups = $derived(
    [
      { id: 'status', label: 'Status', options: statuses.map((value) => ({ value, label: value })) },
      { id: 'priority', label: 'Priority', options: priorities.map((value) => ({ value, label: value })) }
    ].filter((group) => group.options.length > 0)
  );
  const filtered = $derived(
    Boolean(search || projectFilter || filters.status?.length || filters.priority?.length)
  );

  async function loadCached(owner: number, append = false): Promise<void> {
    const readOwner = append ? readGeneration : ++readGeneration;
    const offset = append ? tasks.length : 0;
    const page = await listNotionTasks(
      offset,
      PAGE_SIZE,
      search,
      projectFilter,
      filters.status ?? [],
      filters.priority ?? [],
      sortBy,
      sortDirection
    );
    if (owner !== generation || readOwner !== readGeneration) return;
    tasks = append ? [...tasks, ...page.tasks] : page.tasks;
    projects = page.projects;
    statuses = page.statuses;
    priorities = page.priorities;
    hasMore = page.hasMore;
  }

  async function reloadCached(): Promise<void> {
    const owner = generation;
    loading = true;
    error = '';
    try {
      await loadCached(owner);
    } catch (cause) {
      if (owner === generation) error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      if (owner === generation) loading = false;
    }
  }

  function applySearch(event: SubmitEvent): void {
    event.preventDefault();
    search = searchDraft.trim();
    void reloadCached();
  }

  async function toggleSearch(): Promise<void> {
    searchOpen = !searchOpen;
    if (!searchOpen) return;
    await tick();
    searchInput?.focus();
  }

  function directionLabel(direction: string): string {
    if (sortBy === 'taskNumber') return direction === 'asc' ? 'Lowest first' : 'Highest first';
    return direction === 'asc' ? 'A to Z' : 'Z to A';
  }

  async function refresh(owner = generation): Promise<void> {
    refreshing = true;
    error = '';
    try {
      await refreshNotionTasks();
      if (owner !== generation) return;
      await loadCached(owner);
    } catch (cause) {
      if (owner === generation) error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      if (owner === generation) refreshing = false;
    }
  }

  async function initialize(owner: number): Promise<void> {
    try {
      settings = await readNotionTaskSettings();
      if (owner !== generation) return;
      dataSourceId = settings.dataSourceId;
      showSetup = !settings.dataSourceId || !settings.hasToken;
      await loadCached(owner);
      if (!showSetup) await refresh(owner);
    } catch (cause) {
      if (owner === generation) error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      if (owner === generation) loading = false;
    }
  }

  async function saveSetup(): Promise<void> {
    const owner = generation;
    saving = true;
    error = '';
    try {
      settings = await saveNotionTaskSettings(dataSourceId, token);
      if (owner !== generation) return;
      token = '';
      showSetup = false;
      await refresh(owner);
    } catch (cause) {
      if (owner === generation) error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      if (owner === generation) saving = false;
    }
  }

  async function connectWorkspace(): Promise<void> {
    oauthController?.abort();
    const controller = new AbortController();
    oauthController = controller;
    saving = true;
    error = '';
    try {
      settings = await connectNotion(controller.signal);
      if (controller.signal.aborted || oauthController !== controller) return;
      dataSourceId = settings.dataSourceId;
      showSetup = false;
      await refresh(generation);
    } catch (cause) {
      if (!controller.signal.aborted && oauthController === controller) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    } finally {
      if (oauthController === controller) {
        oauthController = null;
        saving = false;
      }
    }
  }

  async function openNotionUrl(url: string): Promise<void> {
    error = '';
    try {
      const { openUrl } = await import('@tauri-apps/plugin-opener');
      await openUrl(url);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function disconnect(): Promise<void> {
    const owner = generation;
    saving = true;
    error = '';
    try {
      await clearNotionTaskSettings();
      if (owner !== generation) return;
      settings = { dataSourceId: '', hasToken: false };
      dataSourceId = '';
      token = '';
      showSetup = true;
    } catch (cause) {
      if (owner === generation) error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      if (owner === generation) saving = false;
    }
  }

  async function loadMore(): Promise<void> {
    if (loading || !hasMore) return;
    const owner = generation;
    loading = true;
    try {
      await loadCached(owner, true);
    } catch (cause) {
      if (owner === generation) error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      if (owner === generation) loading = false;
    }
  }

  onMount(() => {
    const owner = ++generation;
    void initialize(owner);
  });
  $effect(() => {
    const next = sessionProject;
    untrack(() => {
      if (next === projectFilter) return;
      projectFilter = next;
      void reloadCached();
    });
  });
  $effect(() => {
    const viewport = taskViewport;
    if (!viewport) return;
    const onScroll = (): void => {
      taskScrollTop = viewport.scrollTop;
      taskViewportHeight = viewport.clientHeight;
    };
    onScroll();
    viewport.addEventListener('scroll', onScroll, { passive: true });
    return () => viewport.removeEventListener('scroll', onScroll);
  });
  $effect(() => {
    const root = taskViewport;
    const target = taskLoadSentinel;
    if (!root || !target || !hasMore) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) void loadMore();
      },
      { root, rootMargin: `${TASK_ROW_HEIGHT * 3}px 0px` }
    );
    observer.observe(target);
    return () => observer.disconnect();
  });
  onDestroy(() => {
    oauthController?.abort();
    oauthController = null;
    generation += 1;
    readGeneration += 1;
    tasks = [];
    projects = [];
    statuses = [];
    priorities = [];
    token = '';
    selectedTask = null;
    taskViewport = null;
    taskLoadSentinel = null;
    searchInput = null;
  });
</script>

<div class="relative h-full min-h-0 overflow-hidden">
<section
  class="absolute inset-0 flex min-h-0 flex-col transition-transform duration-200 motion-reduce:transition-none {selectedTask ? '-translate-x-full' : 'translate-x-0'}"
  aria-hidden={selectedTask !== null}
  inert={selectedTask !== null}
>
  <PanelHeader title="Tasks" count={tasks.length}>
    {#snippet actions()}
      <div class="tasks-actions flex items-center">
        <IconButton label="Search tasks" size="xs" side="bottom" class={ACTION_CLASS} onclick={() => void toggleSearch()}>
          <Search class="size-4" aria-hidden="true" />
        </IconButton>
        <DropdownMenu.Root>
          <Tooltip.Root>
            <Tooltip.Trigger>
              {#snippet child({ props })}
                <DropdownMenu.Trigger
                  {...props}
                  class={cn(
                    buttonVariants({ variant: 'ghost', size: 'icon-sm' }),
                    ACTION_CLASS,
                    projectFilter && 'text-[var(--color-accent)]'
                  )}
                  aria-label="View task options"
                >
                  <List class="size-4" aria-hidden="true" />
                </DropdownMenu.Trigger>
              {/snippet}
            </Tooltip.Trigger>
            <Tooltip.Content side="bottom">View task options</Tooltip.Content>
          </Tooltip.Root>
          <DropdownMenu.Content
            align="end"
            sideOffset={7}
            class="flex w-[304px]! flex-col gap-[var(--space-3)] p-[var(--space-4)] text-foreground"
          >
            <div class="flex min-h-8 items-center justify-between gap-3">
              <span class="text-[13px] text-foreground">Sort</span>
              <Select.Root
                type="single"
                value={sortBy}
                onValueChange={(value) => {
                  sortBy = value;
                  void reloadCached();
                }}
              >
                <Select.Trigger size="sm" class="min-w-[132px]" aria-label="Sort tasks">
                  {sortBy === 'taskNumber' ? 'Task number' : 'Title'}
                </Select.Trigger>
                <Select.Content>
                  <Select.Item value="taskNumber" label="Task number" />
                  <Select.Item value="title" label="Title" />
                </Select.Content>
              </Select.Root>
            </div>
            <div class="flex min-h-8 items-center justify-between gap-3">
              <span class="text-[13px] text-foreground">Direction</span>
              <Select.Root
                type="single"
                value={sortDirection}
                onValueChange={(value) => {
                  sortDirection = value;
                  void reloadCached();
                }}
              >
                <Select.Trigger size="sm" class="min-w-[132px]" aria-label="Sort direction">
                  {directionLabel(sortDirection)}
                </Select.Trigger>
                <Select.Content>
                  <Select.Item value="asc" label={directionLabel('asc')} />
                  <Select.Item value="desc" label={directionLabel('desc')} />
                </Select.Content>
              </Select.Root>
            </div>
            <div class="flex min-h-8 items-center justify-between gap-3">
              <span class="text-[13px] text-foreground">Project</span>
              <Select.Root
                type="single"
                value={projectFilter || 'all'}
                onValueChange={(value) => {
                  projectFilter = value === 'all' ? '' : value;
                  void reloadCached();
                }}
              >
                <Select.Trigger size="sm" class="min-w-[132px]" aria-label="Filter tasks by project">
                  {projectFilter || 'All projects'}
                </Select.Trigger>
                <Select.Content>
                  <Select.Item value="all" label="All projects" />
                  {#each projects as project}<Select.Item value={project} label={project} />{/each}
                </Select.Content>
              </Select.Root>
            </div>
          </DropdownMenu.Content>
        </DropdownMenu.Root>
        <IconButton label="Task settings" size="xs" side="bottom" class={ACTION_CLASS} onclick={() => (showSetup = !showSetup)}>
          <Settings2 class="size-4" aria-hidden="true" />
        </IconButton>
        <IconButton
          label="Refresh tasks"
          size="xs"
          side="bottom"
          class={ACTION_CLASS}
          disabled={refreshing || showSetup}
          onclick={() => void refresh()}
        >
          <RefreshCw class="size-4" aria-hidden="true" />
        </IconButton>
      </div>
    {/snippet}
    {projectFilter || 'All projects'} · Latest successful Notion snapshot
  </PanelHeader>

  {#if filterGroups.length > 0}
    <div class="tasks-pills px-(--space-4) pb-(--space-2)">
      <FilterPills
        label="Filter tasks"
        groups={filterGroups}
        value={filters}
        onChange={(next) => {
          filters = next;
          void reloadCached();
        }}
      />
    </div>
  {/if}

  {#if projects.length > 0 && root && !sessionProject && !projectFilter}
    <p class="mx-3 mb-2 text-xs text-muted-foreground">No Notion project matches this session. Showing all projects.</p>
  {/if}

  {#if showSetup}
    <div class="mx-3 mb-3 grid gap-2 rounded-lg border border-border bg-card p-3">
      <p class="text-sm font-medium text-foreground">Notion workspace</p>
      <p class="text-xs leading-snug text-muted-foreground">
        Sign in through your default browser, choose the pages Assembly can read, then return here.
      </p>
      <Button disabled={saving} onclick={() => void connectWorkspace()}>
        {saving ? 'Waiting for Notion…' : 'Connect Notion'}
        {#if !saving}<ExternalLink class="size-3.5" />{/if}
      </Button>
      <details class="grid gap-2 text-xs text-muted-foreground">
        <summary class="cursor-pointer select-none py-1">Developer token setup</summary>
        <div class="grid gap-2 pt-1">
          <p class="leading-snug">
            For local development only. Production users will not need a token or data source ID.
          </p>
          <Button
            variant="secondary"
            onclick={() => void openNotionUrl('https://developers.notion.com/guides/get-started/personal-access-tokens#create-a-pat')}
          >
            Open developer token setup <ExternalLink class="size-3.5" />
          </Button>
          <Input bind:value={token} type="password" placeholder={settings?.hasToken ? 'Token already stored' : 'Paste developer token'} aria-label="Notion developer token" />
          <Input bind:value={dataSourceId} placeholder="Optional data source ID" aria-label="Notion data source ID" />
          <div class="flex justify-end gap-2">
            {#if settings?.hasToken}
              <Button variant="ghost" disabled={saving} onclick={() => void disconnect()}>Disconnect</Button>
            {/if}
            <Button disabled={saving || (!settings?.hasToken && !token.trim())} onclick={() => void saveSetup()}>
              {saving ? 'Connecting…' : 'Connect developer token'}
            </Button>
          </div>
        </div>
      </details>
    </div>
  {/if}

  {#if error}
    <p class="mx-3 mb-2 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive">{error}</p>
  {/if}

  {#if searchOpen}
    <form class="px-(--space-4) pb-(--space-2)" onsubmit={applySearch}>
      <Input bind:ref={searchInput} bind:value={searchDraft} placeholder="Search tasks" aria-label="Search tasks" />
    </form>
  {/if}

  <ScrollArea class="min-h-0 flex-1" bind:viewportRef={taskViewport}>
    {#if loading && tasks.length === 0}
      <EmptyState title="Loading tasks…" body="Reading the last successful local snapshot." />
    {:else if tasks.length === 0}
      <EmptyState
        title={showSetup
          ? 'Connect Notion to see tasks'
          : filtered
            ? 'No matching tasks'
            : 'No tasks in the snapshot'}
        body={showSetup
          ? 'Configuration stays local to this Mac.'
          : filtered
            ? 'Try a different search or filter.'
            : 'Refresh to ask Notion for the latest task list.'}
      >
        {#snippet icon()}<ListTodo />{/snippet}
      </EmptyState>
    {:else}
      <div class="relative mx-2" style={`height: ${tasks.length * TASK_ROW_HEIGHT}px`}>
        {#each visibleTasks as task, index (task.sourceTaskId)}
          <button
            type="button"
            class="absolute inset-x-0 grid h-[44px] w-full grid-cols-[1rem_minmax(0,1fr)] items-center gap-(--space-3) rounded-(--radius-md) px-(--space-2) text-left hover:bg-[var(--menu-row-hover)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            style={`transform: translateY(${(firstTaskIndex + index) * TASK_ROW_HEIGHT}px)`}
            onclick={() => (selectedTask = task)}
          >
            <FileText class={`size-4 stroke-[1.8] ${taskStatusIconClass(task.status)}`} aria-hidden="true" />
            <span class="min-w-0">
              <span class="block truncate text-[13px] leading-[18px] font-medium text-foreground">{task.title}</span>
              <span class="block truncate text-sm leading-4 text-muted-foreground">{[task.project || 'Unspecified project', task.status || 'Unspecified', task.priority].filter(Boolean).join(' · ')}</span>
            </span>
          </button>
        {/each}
        <span bind:this={taskLoadSentinel} class="absolute right-0 bottom-0 size-px" aria-hidden="true"></span>
      </div>
    {/if}
  </ScrollArea>
</section>

<section
  class="absolute inset-0 transition-transform duration-200 motion-reduce:transition-none {selectedTask ? 'translate-x-0' : 'translate-x-full'}"
  aria-hidden={selectedTask === null}
>
  {#if selectedTask}
    {#key selectedTask.sourceTaskId}
      <NotionTaskViewer
        task={selectedTask}
        {statuses}
        onBack={() => (selectedTask = null)}
        onOpenExternal={(url) => void openNotionUrl(url)}
        onStatusUpdated={async (status) => {
          selectedTask = selectedTask ? { ...selectedTask, status } : null;
          await reloadCached();
        }}
      />
    {/key}
  {/if}
</section>
</div>

<style>
  /* This panel sits on the elevated card, the same layer the kit's secondary and
     accent slots name, so a pill filled with either would vanish into it. Both
     controls take the shell's pill surface instead, one step above the card. */
  .tasks-pills {
    --secondary: var(--pill-surface);
    --accent: var(--pill-surface-hover);
  }

  /* The Sessions rail's header group: 29px round buttons in one pill. */
  .tasks-actions {
    gap: 1px;
    padding: 3px;
    border-radius: var(--radius-pill);
    background: var(--pill-surface);
  }
  .tasks-actions :global(button) {
    width: 29px;
    height: 29px;
    padding: 0;
    border-radius: var(--radius-pill);
  }
</style>
