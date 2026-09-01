<script lang="ts">
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import ListTodo from '@lucide/svelte/icons/list-todo';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Settings2 from '@lucide/svelte/icons/settings-2';
  import { onDestroy, onMount } from 'svelte';

  import { Button } from '$lib/components/ui/button/index.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { PanelHeader } from '$lib/components/ui/panel-header/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import {
    clearNotionTaskSettings,
    listNotionTasks,
    readNotionTaskSettings,
    refreshNotionTasks,
    saveNotionTaskSettings,
    type NotionTaskRow,
    type NotionTaskSettings
  } from '$lib/shell/notionTasks.ts';
  import { openUrlInBrowser } from '$lib/shell/workbenchNavigation.ts';

  const PAGE_SIZE = 100;

  let settings = $state<NotionTaskSettings | null>(null);
  let tasks = $state<NotionTaskRow[]>([]);
  let projects = $state<string[]>([]);
  let statuses = $state<string[]>([]);
  let searchDraft = $state('');
  let search = $state('');
  let statusFilter = $state('');
  let projectFilter = $state('');
  let loading = $state(true);
  let refreshing = $state(false);
  let saving = $state(false);
  let error = $state('');
  let hasMore = $state(false);
  let showSetup = $state(false);
  let dataSourceId = $state('');
  let token = $state('');
  let generation = 0;
  let readGeneration = 0;

  async function loadCached(owner: number, append = false): Promise<void> {
    const readOwner = append ? readGeneration : ++readGeneration;
    const offset = append ? tasks.length : 0;
    const page = await listNotionTasks(
      offset,
      PAGE_SIZE,
      search,
      projectFilter,
      statusFilter
    );
    if (owner !== generation || readOwner !== readGeneration) return;
    tasks = append ? [...tasks, ...page.tasks] : page.tasks;
    projects = page.projects;
    statuses = page.statuses;
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
  onDestroy(() => {
    generation += 1;
    readGeneration += 1;
    tasks = [];
    projects = [];
    statuses = [];
    token = '';
  });
</script>

<div class="flex h-full min-h-0 flex-col">
  <PanelHeader title="Tasks" count={tasks.length}>
    {#snippet actions()}
      <IconButton label="Task settings" onclick={() => (showSetup = !showSetup)}><Settings2 /></IconButton>
      <IconButton label="Refresh tasks" disabled={refreshing || showSetup} onclick={() => void refresh()}>
        <RefreshCw />
      </IconButton>
    {/snippet}
    Latest successful Notion snapshot
  </PanelHeader>

  {#if showSetup}
    <div class="mx-3 mb-3 grid gap-2 rounded-lg border border-border bg-card p-3">
      <p class="text-sm font-medium text-foreground">Connect a read-only Notion integration</p>
      <p class="text-xs leading-snug text-muted-foreground">
        Share the task data source with the integration, then paste its data source ID and token.
        The token stays in macOS Keychain.
      </p>
      <Input bind:value={dataSourceId} placeholder="Data source ID" aria-label="Notion data source ID" />
      <Input bind:value={token} type="password" placeholder={settings?.hasToken ? 'Token already stored' : 'Integration token'} aria-label="Notion integration token" />
      <div class="flex justify-end gap-2">
        {#if settings?.hasToken}
          <Button variant="ghost" disabled={saving} onclick={() => void disconnect()}>Disconnect</Button>
        {/if}
        <Button disabled={saving || !dataSourceId.trim()} onclick={() => void saveSetup()}>
          {saving ? 'Saving…' : 'Save and refresh'}
        </Button>
      </div>
    </div>
  {/if}

  {#if error}
    <p class="mx-3 mb-2 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive">{error}</p>
  {/if}

  {#if projects.length > 0 || statuses.length > 0 || searchDraft || projectFilter || statusFilter}
    <form class="grid gap-2 px-3 pb-3" onsubmit={applySearch}>
      <div class="flex gap-2">
        <Input bind:value={searchDraft} placeholder="Search tasks" aria-label="Search tasks" />
        <Button type="submit" variant="ghost" disabled={loading}>Search</Button>
      </div>
      <div class="grid grid-cols-2 gap-2">
        <select bind:value={projectFilter} onchange={() => void reloadCached()} aria-label="Filter by project" class="h-8 min-w-0 rounded-md border border-border bg-background px-2 text-xs text-foreground">
          <option value="">All projects</option>
          {#each projects as project}<option value={project}>{project}</option>{/each}
        </select>
        <select bind:value={statusFilter} onchange={() => void reloadCached()} aria-label="Filter by status" class="h-8 min-w-0 rounded-md border border-border bg-background px-2 text-xs text-foreground">
          <option value="">All statuses</option>
          {#each statuses as status}<option value={status}>{status}</option>{/each}
        </select>
      </div>
    </form>
  {/if}

  <ScrollArea class="min-h-0 flex-1">
    {#if loading && tasks.length === 0}
      <EmptyState title="Loading tasks…" body="Reading the last successful local snapshot." />
    {:else if tasks.length === 0}
      <EmptyState
        title={showSetup
          ? 'Connect Notion to see tasks'
          : search || projectFilter || statusFilter
            ? 'No matching tasks'
            : 'No tasks in the snapshot'}
        body={showSetup
          ? 'Configuration stays local to this Mac.'
          : search || projectFilter || statusFilter
            ? 'Try a different search or filter.'
            : 'Refresh to ask Notion for the latest read-only task list.'}
      >
        {#snippet icon()}<ListTodo />{/snippet}
      </EmptyState>
    {:else}
      <div class="grid gap-px px-2 pb-3">
        {#each tasks as task (task.sourceTaskId)}
          <article class="group grid gap-1 rounded-md px-2 py-2 hover:bg-accent/60">
            <div class="flex min-w-0 items-start gap-2">
              <div class="min-w-0 flex-1">
                <p class="line-clamp-2 text-sm leading-snug font-medium text-foreground">{task.title}</p>
                <p class="mt-1 truncate text-xs text-muted-foreground">{task.project} · {task.status}</p>
              </div>
              <IconButton label="Open in Notion" onclick={() => void openUrlInBrowser({ url: task.sourceUrl })}>
                <ExternalLink />
              </IconButton>
            </div>
            {#if task.priority || task.assignee || task.dueDate}
              <p class="truncate text-[11px] text-muted-foreground">
                {[task.priority, task.assignee, task.dueDate].filter(Boolean).join(' · ')}
              </p>
            {/if}
          </article>
        {/each}
        {#if hasMore}
          <Button variant="ghost" disabled={loading} onclick={() => void loadMore()}>{loading ? 'Loading…' : 'Load more'}</Button>
        {/if}
      </div>
    {/if}
  </ScrollArea>
</div>
