<script lang="ts">
  /**
   * The single left-side Paneview host. It owns no session data and performs no
   * service calls; all rows, finder data, and actions are props. The three
   * stable pane ids are the only primary left-rail ids: working, done, settled.
   */
  import { onMount, tick } from 'svelte';
  import type { Component } from 'svelte';

  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Search from '@lucide/svelte/icons/search';

  import * as Collapsible from '$lib/components/ui/collapsible/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import type { AgentSession } from '$lib/tauriSource';
  import type { LayoutStorage } from '$lib/shell/layout/layoutStorage';
  import { createPaneStack, type PaneStack } from '$lib/shell/layout/paneStack';
  import {
    sidePaneRegistrationsToPaneSpecs,
    type SidePaneRegistration
  } from '$lib/shell/layout/sidePaneRegistry';
  import type { OwnedSession } from '$lib/shell/ownedSessions';
  import {
    groupSessions,
    groupToggleKey,
    isGroupExpanded,
    readGroupExpansion,
    rememberGroupToggle,
    RESUME_GROUP_ROW_CAP,
    sessionGroupPath,
    visibleGroupItems,
    writeGroupExpansion,
    type GroupExpansion
  } from '$lib/shell/sessionGroups';

  import WorkingPane from './WorkingPane.svelte';
  import DonePane from './DonePane.svelte';
  import SettledPane from './SettledPane.svelte';

  const LEFT_PANE_REGISTRATIONS: readonly SidePaneRegistration[] = [
    {
      id: 'working',
      title: 'Working',
      region: 'left',
      component: WorkingPane as unknown as Component,
      minimumSize: 74,
      preferredSize: 260,
      maximumSize: null,
      defaultExpanded: true,
      persistent: true,
      order: 0
    },
    {
      id: 'done',
      title: 'Done',
      region: 'left',
      component: DonePane as unknown as Component,
      minimumSize: 74,
      preferredSize: 220,
      maximumSize: null,
      defaultExpanded: true,
      persistent: true,
      order: 1
    },
    {
      id: 'settled',
      title: 'Settled',
      region: 'left',
      component: SettledPane as unknown as Component,
      minimumSize: 74,
      preferredSize: 180,
      maximumSize: null,
      defaultExpanded: false,
      persistent: true,
      order: 2
    }
  ];

  interface Props {
    sessions: OwnedSession[];
    available?: AgentSession[];
    scanning?: boolean;
    activeOwnedId?: string | null;
    layoutStore?: LayoutStorage;
    /** Compatibility only; new hosts use the injected store's v1 map. */
    storageKey?: string;
    onSelect?(ownedId: string): void;
    onAdopt?(session: AgentSession): void;
    onRescan?(): void;
    onRestart?(ownedId: string): void;
    onComplete?(ownedId: string): void;
    onReopen?(ownedId: string): void;
    onSettle?(ownedId: string): void;
    onUnsettle?(ownedId: string): void;
    onClose?(ownedId: string): void;
    onAskRemove?(ownedId: string): void;
    onReady?(controls: { resetLayout(): void }): void;
    onError?(message: string): void;
  }

  let {
    sessions,
    available = [],
    scanning = false,
    activeOwnedId = null,
    layoutStore,
    storageKey,
    onSelect,
    onAdopt,
    onRescan,
    onRestart,
    onComplete,
    onReopen,
    onSettle,
    onUnsettle,
    onClose,
    onAskRemove,
    onReady,
    onError
  }: Props = $props();

  let host: HTMLElement;
  let workingSlot: HTMLElement;
  let doneSlot: HTMLElement;
  let settledSlot: HTMLElement;
  let stack: PaneStack | null = null;

  const adoptedIds = $derived(
    new Set(
      sessions
        .map((session) => session.nativeSessionId)
        .filter((id): id is string => typeof id === 'string' && id.length > 0)
    )
  );
  const resumable = $derived(
    available.filter(
      (session) => !adoptedIds.has(session.id) && !adoptedIds.has(`${session.provider}:${session.id}`)
    )
  );

  /** Finder state is local UI state; opening it never scans or loads anything. */
  let findOpen = $state(false);
  let query = $state('');
  let findInput = $state<HTMLInputElement | null>(null);
  let remembered = $state<GroupExpansion>(
    typeof window === 'undefined' ? {} : readGroupExpansion(window.localStorage)
  );
  let expandedRows = $state<Record<string, boolean>>({});
  const searching = $derived(query.trim().length > 0);
  const grouped = $derived(groupSessions([], resumable, query));
  const activeProjectPath = $derived.by(() => {
    const active = sessions.find((session) => session.ownedId === activeOwnedId);
    return active ? sessionGroupPath(active.projectPath ?? active.cwd) : null;
  });

  /** The context panel calls through SessionsColumn to this imperative focus. */
  export function openFinder(): void {
    findOpen = true;
    void tick().then(() => findInput?.focus());
  }

  function expanded(path: string): boolean {
    return isGroupExpanded({
      list: 'resume',
      path,
      remembered,
      activeProjectPath,
      searching
    });
  }

  function toggleGroup(path: string): void {
    const settled = (from: GroupExpansion) =>
      isGroupExpanded({
        list: 'resume',
        path,
        remembered: from,
        activeProjectPath,
        searching: false
      });
    remembered = rememberGroupToggle(
      remembered,
      groupToggleKey('resume', path),
      !settled(remembered),
      settled({})
    );
    if (typeof window !== 'undefined') writeGroupExpansion(window.localStorage, remembered);
  }

  onMount(() => {
    let observer: ResizeObserver | null = null;
    try {
      const injectedStore =
        layoutStore ??
        (typeof window !== 'undefined' ? (window.localStorage as LayoutStorage) : undefined);
      const paneElements = new Map([
        ['working', workingSlot],
        ['done', doneSlot],
        ['settled', settledSlot]
      ]);
      stack = createPaneStack(host, {
        layoutStore: injectedStore,
        storageKey,
        layoutId: 'left-rail',
        panes: sidePaneRegistrationsToPaneSpecs(LEFT_PANE_REGISTRATIONS, paneElements)
      });
      const refit = (): void => {
        if (!host) return;
        const width = host.clientWidth;
        const height = host.clientHeight;
        if (width > 0 && height > 0) stack?.layout(width, height);
      };
      refit();
      observer = new ResizeObserver(refit);
      observer.observe(host);
      onReady?.({ resetLayout: () => stack?.resetLayout() });
    } catch (error) {
      onError?.(error instanceof Error ? error.message : String(error));
    }

    return () => {
      observer?.disconnect();
      stack?.dispose();
      stack = null;
    };
  });
</script>

<div data-testid="sessions-paneview" class="sessions-paneview">
  <div data-testid="sessions-paneview-stack" class="pane-stack-host" bind:this={host}></div>

  <Collapsible.Root
    bind:open={findOpen}
    data-testid="session-finder"
    class="finder shrink-0 border-t border-[var(--color-border)]"
  >
    <div class="flex items-center gap-1 px-2 py-1.5">
      <Collapsible.Trigger
        class="flex min-w-0 flex-1 items-center gap-1.5 rounded-md px-1 py-1 text-left
               text-[var(--color-text-2)] transition-colors hover:bg-[var(--color-elevated)]
               hover:text-[var(--color-text)] focus-visible:ring-3 focus-visible:ring-ring/50
               outline-none"
      >
        {#if findOpen}
          <ChevronDown class="size-3.5 shrink-0" aria-hidden="true" />
        {:else}
          <ChevronRight class="size-3.5 shrink-0" aria-hidden="true" />
        {/if}
        <span class="text-[13px]">Find a session</span>
        <span class="ml-auto pl-2 text-[12px] text-[var(--color-text-2)]">{resumable.length}</span>
      </Collapsible.Trigger>
      <button
        data-testid="session-finder-rescan"
        type="button"
        class="finder-action"
        aria-label="look for agent sessions again"
        title={scanning ? 'Looking…' : 'Look again'}
        onclick={() => onRescan?.()}
      >
        <RefreshCw class={scanning ? 'size-3.5 animate-spin' : 'size-3.5'} aria-hidden="true" />
      </button>
    </div>

    <Collapsible.Content class="max-h-[45vh] overflow-y-auto">
      <div class="flex flex-col gap-2 px-2 pt-1 pb-2">
        <div class="relative">
          <Search
            class="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2
                   text-[var(--color-text-3)]"
            aria-hidden="true"
          />
          <Input
            data-testid="session-finder-search"
            type="search"
            placeholder="Search sessions to resume"
            aria-label="Search sessions to resume"
            bind:ref={findInput}
            bind:value={query}
            class="h-7 rounded-md bg-[var(--color-surface)] pl-7 text-[13px] md:text-[13px]"
          />
        </div>

        {#if resumable.length === 0}
          <p class="px-1 text-[13px] text-[var(--color-text-2)]">
            {scanning ? 'Looking for agent sessions…' : 'Nothing to resume.'}
          </p>
        {:else if grouped.available.length === 0}
          <p class="px-1 text-[13px] text-[var(--color-text-2)]">
            Nothing to resume matches “{query.trim()}”.
          </p>
        {:else}
          {#each grouped.available as group (group.path)}
            {@const visible = visibleGroupItems(
              group.items,
              RESUME_GROUP_ROW_CAP,
              expandedRows[group.path] === true
            )}
            <div class="flex flex-col">
              <button
                type="button"
                class="flex w-full items-center gap-1.5 rounded-md px-1 py-1 text-left
                       text-[12px] text-[var(--color-text-2)] transition-colors
                       hover:bg-[var(--color-elevated)] hover:text-[var(--color-text)]"
                aria-expanded={expanded(group.path)}
                title={group.path || 'Sessions with no project folder'}
                onclick={() => toggleGroup(group.path)}
              >
                {#if expanded(group.path)}
                  <ChevronDown class="size-3 shrink-0" aria-hidden="true" />
                {:else}
                  <ChevronRight class="size-3 shrink-0" aria-hidden="true" />
                {/if}
                <span class="truncate">{group.name}</span>
                {#if group.parentProject}
                  <span class="truncate text-[var(--color-text-2)]">· {group.parentProject}</span>
                {/if}
                <span class="ml-auto shrink-0 pl-2 text-[var(--color-text-2)]">{group.items.length}</span>
              </button>
              {#if expanded(group.path)}
                <div class="flex flex-col">
                  {#each visible.shown as session (`${session.provider}:${session.id}`)}
                    <button
                      data-testid="session-finder-row"
                      type="button"
                      class="flex w-full min-w-0 flex-col items-start gap-1 rounded-md px-2 py-1.5
                             text-left transition-colors hover:bg-[var(--color-surface)]"
                      title={session.projectPath ?? session.title}
                      onclick={() => onAdopt?.(session)}
                    >
                      <span class="flex w-full min-w-0 items-center gap-2">
                        <span class="finder-dot" aria-hidden="true"></span>
                        <span class="truncate text-[13px] leading-[1.35] text-[var(--color-text)]">
                          {session.title || session.id}
                        </span>
                        <span class="ml-auto shrink-0 text-[12px] text-primary">Resume</span>
                      </span>
                      {#if session.projectPath}
                        <span class="w-full truncate pl-4 text-[12px] text-[var(--color-text-2)]">
                          {session.projectPath}
                        </span>
                      {/if}
                    </button>
                  {/each}
                  {#if visible.hiddenCount > 0}
                    <button
                      type="button"
                      class="rounded-md px-2 py-1 pl-6 text-left text-[12px]
                             text-[var(--color-text-2)] transition-colors
                             hover:bg-[var(--color-elevated)] hover:text-[var(--color-text)]"
                      onclick={() => (expandedRows = { ...expandedRows, [group.path]: true })}
                    >
                      Show {visible.hiddenCount} more
                    </button>
                  {/if}
                </div>
              {/if}
            </div>
          {/each}
        {/if}
      </div>
    </Collapsible.Content>
  </Collapsible.Root>
</div>

<!-- Svelte owns these parking slots. paneStack only moves the slots into its
     pane hosts and returns them here when a panel is removed. -->
<div data-testid="sessions-paneview-parking" class="parking-stage" aria-hidden="true">
  <div data-testid="working-pane-slot" class="pane-slot" bind:this={workingSlot}>
    <WorkingPane
      {sessions}
      {activeOwnedId}
      onSelect={onSelect}
      onRestart={onRestart}
      onComplete={onComplete}
      onClose={onClose}
    />
  </div>
  <div data-testid="done-pane-slot" class="pane-slot" bind:this={doneSlot}>
    <DonePane
      {sessions}
      {activeOwnedId}
      onSelect={onSelect}
      onRestart={onRestart}
      onReopen={onReopen}
      onSettle={onSettle}
      onClose={onClose}
      onAskRemove={(ownedId) => onAskRemove?.(ownedId)}
    />
  </div>
  <div data-testid="settled-pane-slot" class="pane-slot" bind:this={settledSlot}>
    <SettledPane
      {sessions}
      {activeOwnedId}
      onSelect={onSelect}
      onRestart={onRestart}
      onUnsettle={onUnsettle}
      onClose={onClose}
    />
  </div>
</div>

<style>
  .sessions-paneview {
    display: flex;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
    overflow: hidden;
    background: var(--color-bg);
  }
  .pane-stack-host { min-width: 0; min-height: 0; flex: 1 1 auto; overflow: hidden; }
  .parking-stage { display: none; }
  .pane-slot { width: 100%; height: 100%; min-width: 0; min-height: 0; overflow: hidden; }
  .sessions-paneview :global(.pane-body-host) { position: absolute; inset: 0; overflow: auto; }
  .finder-action {
    display: inline-flex;
    height: 24px;
    width: 24px;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--color-text-2);
  }
  .finder-action:hover,
  .finder-action:focus-visible { background: var(--color-elevated); color: var(--color-text); }
  .finder-dot {
    width: 7px;
    height: 7px;
    flex: 0 0 auto;
    border-radius: 50%;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--color-text-2);
  }
</style>
