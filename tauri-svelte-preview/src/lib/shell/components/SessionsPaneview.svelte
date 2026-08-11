<script lang="ts">
  /**
   * The single left-side Paneview host. It owns no session data and performs no
   * service calls; all rows, finder data, and actions are props. The three
   * stable pane ids are the only primary left-rail ids: working, done, settled.
   */
  import { onMount } from 'svelte';
  import type { Component } from 'svelte';

  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import type { AgentSession } from '$lib/tauriSource';
  import type { LayoutStorage } from '$lib/shell/layout/layoutStorage';
  import { createPaneStack, type PaneStack } from '$lib/shell/layout/paneStack';
  import {
    sidePaneRegistrationsToPaneSpecs,
    type SidePaneRegistration
  } from '$lib/shell/layout/sidePaneRegistry';
  import type { OwnedSession } from '$lib/shell/ownedSessions';
  import { deriveOwnedLibraryState } from '$lib/shell/sessionLibrary/sessionLibraryModel';
  import { openSessionLibrary } from '$lib/shell/sessionLibrary/sessionLibraryNavigation';
  import MyWorkSessionList from './MyWorkSessionList.svelte';
  import {
    prepareMyWorkSessions,
    type MyWorkViewOptions
  } from './myWorkViewOptions';

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
    viewOptions: MyWorkViewOptions;
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
    viewOptions,
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
  const visibleSessions = $derived(prepareMyWorkSessions(sessions, viewOptions));
  const statusContentSignature = $derived(
    visibleSessions.map((session) => `${session.ownedId}:${deriveOwnedLibraryState(session)}`).join('|')
  );

  /** The pane body owns its header and rows, so measure that whole body when
   * it is mounted. The fallback only covers the first layout before Svelte has
   * painted the rows; the reactive refit below replaces it with scrollHeight.
   */
  function paneContentSize(id: string): number {
    const count = visibleSessions.filter((session) => deriveOwnedLibraryState(session) === id).length;
    const fallback = 30 + Math.max(1, count) * 56;
    const measured = typeof document !== 'undefined'
      ? document.querySelector<HTMLElement>(`[data-testid="${id}-pane"]`)?.scrollHeight ?? 0
      : 0;
    return Math.max(74, measured || fallback);
  }

  /** The context panel and the old row now activate the full center tab. */
  export function openFinder(): void {
    openSessionLibrary();
  }

  onMount(() => {
    let observer: ResizeObserver | null = null;
    let refitFrame: number | null = null;
    try {
      const injectedStore =
        layoutStore ??
        (typeof window !== 'undefined' ? (window.localStorage as LayoutStorage) : undefined);
      const paneElements = new Map([
        ['working', workingSlot],
        ['done', doneSlot],
        ['settled', settledSlot]
      ]);
      const paneSpecs = sidePaneRegistrationsToPaneSpecs(LEFT_PANE_REGISTRATIONS, paneElements)
        .map((pane) => ({ ...pane, contentSize: () => paneContentSize(pane.id) }));
      stack = createPaneStack(host, {
        layoutStore: injectedStore,
        storageKey,
        layoutId: 'left-rail',
        panes: paneSpecs
      });
      const refitNow = (): void => {
        if (!host) return;
        const width = host.clientWidth;
        const height = host.clientHeight;
        if (width > 0 && height > 0) stack?.layout(width, height);
      };
      const scheduleRefit = (): void => {
        if (refitFrame !== null) return;
        refitFrame = requestAnimationFrame(() => {
          refitFrame = null;
          refitNow();
        });
      };
      refitNow();
      observer = new ResizeObserver(scheduleRefit);
      observer.observe(host);
      for (const pane of paneElements.values()) {
        const paneBody = pane.parentElement;
        if (paneBody) observer.observe(paneBody);
        const content = pane.querySelector<HTMLElement>('.pane-section');
        if (content) observer.observe(content);
      }
      onReady?.({ resetLayout: () => stack?.resetLayout() });
    } catch (error) {
      onError?.(error instanceof Error ? error.message : String(error));
    }

    return () => {
      observer?.disconnect();
      if (refitFrame !== null) cancelAnimationFrame(refitFrame);
      stack?.dispose();
      stack = null;
    };
  });

  $effect(() => {
    // Keep this dependency tied to both membership and status changes. The
    // pane children paint first; the microtask then measures their full body.
    statusContentSignature;
    if (viewOptions.groupBy !== 'status' || !stack) return;
    queueMicrotask(() => stack?.fitContent());
  });
</script>

<div data-testid="sessions-paneview" class="sessions-paneview">
  <div
    data-testid="sessions-paneview-stack"
    class="pane-stack-host"
    class:hidden={viewOptions.groupBy !== 'status'}
    bind:this={host}
  ></div>

  {#if viewOptions.groupBy !== 'status'}
    <div class="min-h-0 flex-1 overflow-hidden">
      <MyWorkSessionList
        sessions={visibleSessions}
        options={{ ...viewOptions, visibleStatuses: ['working', 'done', 'settled'] }}
        {activeOwnedId}
        {onSelect}
        {onRestart}
        {onComplete}
        {onReopen}
        {onSettle}
        {onUnsettle}
        {onClose}
        {onAskRemove}
      />
    </div>
  {/if}

  <div data-testid="session-finder" class="finder shrink-0 border-t border-[var(--color-border)]">
    <div class="flex items-center gap-1 px-2 py-1.5">
      <button
        data-testid="session-finder-open"
        type="button"
        class="flex min-w-0 flex-1 items-center gap-1.5 rounded-md px-1 py-1 text-left
               text-[var(--color-text-2)] transition-colors hover:bg-[var(--color-elevated)]
               hover:text-[var(--color-text)] focus-visible:ring-3 focus-visible:ring-ring/50
               outline-none"
        aria-label="Open Session History"
        title="Open Session History"
        onclick={openFinder}
      >
        <ChevronRight class="size-3.5 shrink-0" aria-hidden="true" />
        <span class="text-[13px]">Find a session</span>
        <span class="ml-auto pl-2 text-[12px] text-[var(--color-text-2)]">{resumable.length}</span>
      </button>
      <span data-testid="session-finder-rescan">
        <IconButton
          label={scanning ? 'Looking for agent sessions' : 'Look for agent sessions again'}
          size="xs"
          side="top"
          class="text-[var(--color-text-2)]"
          onclick={() => onRescan?.()}
        >
          <RefreshCw class={scanning ? 'size-3.5 animate-spin' : 'size-3.5'} aria-hidden="true" />
        </IconButton>
      </span>
    </div>
  </div>
</div>

<!-- Svelte owns these parking slots. paneStack only moves the slots into its
     pane hosts and returns them here when a panel is removed. -->
<div data-testid="sessions-paneview-parking" class="parking-stage" aria-hidden="true">
  <div data-testid="working-pane-slot" class="pane-slot" bind:this={workingSlot}>
    <WorkingPane
      sessions={visibleSessions}
      {activeOwnedId}
      onSelect={onSelect}
      onRestart={onRestart}
      onComplete={onComplete}
      onClose={onClose}
    />
  </div>
  <div data-testid="done-pane-slot" class="pane-slot" bind:this={doneSlot}>
    <DonePane
      sessions={visibleSessions}
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
      sessions={visibleSessions}
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
  /* `min-height`, never `height` + `overflow: hidden`: this slot is the only
     child of the scrollable pane body, so a fixed, clipped slot swallows every
     row past the pane's own height and leaves the body with nothing to scroll.
     Filling the pane and then growing past it is what lets the body scroll. */
  .pane-slot { width: 100%; min-width: 0; min-height: 100%; }
  .sessions-paneview :global(.pane-body-host) { position: absolute; inset: 0; overflow: auto; }
</style>
