<script lang="ts">
  /**
   * The single left-side Paneview host. It owns no session data and performs no
   * service calls; all rows and actions are props. The three stable pane ids are
   * the only primary left-rail ids: working, done, settled.
   */
  import { onMount } from 'svelte';
  import type { Component } from 'svelte';

  import type { LayoutStorage } from '$lib/shell/layout/layoutStorage';
  import { createPaneStack, type PaneStack } from '$lib/shell/layout/paneStack';
  import {
    sidePaneRegistrationsToPaneSpecs,
    type SidePaneRegistration
  } from '$lib/shell/layout/sidePaneRegistry';
  import type { OwnedSession } from '$lib/shell/ownedSessions';

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
    activeOwnedId?: string | null;
    layoutStore?: LayoutStorage;
    /** Compatibility only; new hosts use the injected store's v1 map. */
    storageKey?: string;
    onSelect?(ownedId: string): void;
    onRestart?(ownedId: string): void;
    onComplete?(ownedId: string): void;
    onReopen?(ownedId: string): void;
    onSettle?(ownedId: string): void;
    onUnsettle?(ownedId: string): void;
    onClose?(ownedId: string): void;
    onReady?(controls: { resetLayout(): void }): void;
    onError?(message: string): void;
  }

  let {
    sessions,
    activeOwnedId = null,
    layoutStore,
    storageKey,
    onSelect,
    onRestart,
    onComplete,
    onReopen,
    onSettle,
    onUnsettle,
    onClose,
    onReady,
    onError
  }: Props = $props();

  let host: HTMLElement;
  let workingSlot: HTMLElement;
  let doneSlot: HTMLElement;
  let settledSlot: HTMLElement;
  let stack: PaneStack | null = null;

  onMount(() => {
    let observer: ResizeObserver | null = null;
    try {
      const injectedStore =
        layoutStore ?? (typeof window !== 'undefined' ? (window.localStorage as LayoutStorage) : undefined);
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

<div data-testid="sessions-paneview" class="sessions-paneview" bind:this={host}></div>

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
  .sessions-paneview { width: 100%; height: 100%; min-width: 0; min-height: 0; overflow: hidden; }
  .parking-stage { display: none; }
  .pane-slot { width: 100%; height: 100%; min-width: 0; min-height: 0; overflow: hidden; }
  .sessions-paneview :global(.pane-body-host) { position: absolute; inset: 0; overflow: auto; }
</style>
