<script lang="ts">
  import type { Action } from 'svelte/action';
  import type { SerializedDockview } from 'dockview-core';
  import {
    createSourceWorkbench,
    sourceWorkbenchStorageKey,
    type SourceWorkbench
  } from '$lib/sourceDockviewWorkspace';
  import {
    createDefaultSourceDockLayout,
    normalizeSourceDockLayout,
    type SourceDockLayout,
    type SourceDockPanelID
  } from '$lib/sourceDockLayout';

  type SetPanelElement = (panelID: SourceDockPanelID, element: HTMLElement | null) => void;

  type Props = {
    /** Layout the workbench is seeded with when there is no compatible stored layout. */
    layout?: SourceDockLayout;
    /** Persist/restore the workbench layout under this key. Pass null to disable persistence. */
    storageKey?: string | null;
    /**
     * Bindable handle the page uses to route existing panel content roots into the workbench
     * via the IContentRenderer bridge. It is a no-op until the workbench has mounted.
     */
    setPanelElement?: SetPanelElement;
    /** Bindable readiness flag flipped true once the Dockview has mounted. */
    ready?: boolean;
    /** Bindable error message populated when the workbench fails to mount. */
    error?: string;
    /** Notified after each panel-ownership change so the host can re-route content. */
    onPanelOwnershipChange?: () => void;
  };

  let {
    layout = createDefaultSourceDockLayout(),
    storageKey = sourceWorkbenchStorageKey,
    setPanelElement = $bindable(() => {}),
    ready = $bindable(false),
    error = $bindable(''),
    onPanelOwnershipChange
  }: Props = $props();

  let workbench: SourceWorkbench | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let hostToken = 0;
  let hostNode: HTMLElement | null = null;
  let mountedPanelSignature = '';

  // The factory builds a fixed plan from the visible panels at creation time, so when the set
  // of visible panels changes (e.g. the user shows the terminal) we rebuild the workbench so the
  // new panel becomes a real Dockview tab. Pure drag/resize rearrangements keep the same visible
  // set, so they never trigger a rebuild and the user's arrangement is preserved.
  function panelSignature(value: SourceDockLayout): string {
    const normalized = normalizeSourceDockLayout(value);
    return normalized.groups
      .map((group) => `${group.id}:${group.panelIDs.join(',')}`)
      .join('|');
  }

  const currentPanelSignature = $derived(panelSignature(layout));

  function loadStoredLayout(): SerializedDockview | null {
    if (!storageKey || typeof window === 'undefined') return null;
    try {
      const raw = window.localStorage.getItem(storageKey);
      if (!raw) return null;
      return JSON.parse(raw) as SerializedDockview;
    } catch {
      return null;
    }
  }

  function persistLayout(serialized: SerializedDockview) {
    if (!storageKey || typeof window === 'undefined') return;
    try {
      window.localStorage.setItem(storageKey, JSON.stringify(serialized));
    } catch {
      // Ignore persistence failures (private mode / quota); the live layout still works.
    }
  }

  function layoutToContainer(node: HTMLElement) {
    if (!workbench) return;
    const width = Math.max(1, Math.round(node.clientWidth));
    const height = Math.max(1, Math.round(node.clientHeight));
    workbench.api.layout(width, height, true);
  }

  function disposeWorkbench() {
    resizeObserver?.disconnect();
    resizeObserver = null;
    workbench?.dispose();
    workbench = null;
    setPanelElement = () => {};
    ready = false;
  }

  async function initialize(node: HTMLElement, token: number) {
    ready = false;
    error = '';

    disposeWorkbench();

    try {
      const instance = await createSourceWorkbench(node, {
        layout,
        storedLayout: loadStoredLayout(),
        onDidLayoutChange: (serialized) => persistLayout(serialized),
        onDidPanelMove: (_panelID, serialized) => persistLayout(serialized),
        onDidActivePanelChange: () => onPanelOwnershipChange?.(),
        onDidPanelClose: () => onPanelOwnershipChange?.()
      });

      if (token !== hostToken) {
        instance.dispose();
        return;
      }

      workbench = instance;
      mountedPanelSignature = panelSignature(layout);
      setPanelElement = (panelID, element) => instance.setPanelElement(panelID, element);

      resizeObserver = new ResizeObserver(() => layoutToContainer(node));
      resizeObserver.observe(node);

      ready = true;
      layoutToContainer(node);
      onPanelOwnershipChange?.();
    } catch (workbenchError) {
      error =
        workbenchError instanceof Error ? workbenchError.message : 'Unified workbench unavailable';
      ready = false;
      disposeWorkbench();
    }
  }

  const workbenchAction: Action<HTMLElement> = (node) => {
    hostNode = node;
    const token = ++hostToken;
    void initialize(node, token);

    return {
      destroy() {
        if (hostToken === token) {
          hostToken += 1;
          hostNode = null;
          disposeWorkbench();
        }
      }
    };
  };

  // Rebuild the Dockview when the visible-panel set changes so newly-shown panels appear as tabs.
  $effect(() => {
    const signature = currentPanelSignature;
    if (!workbench || !hostNode || signature === mountedPanelSignature) return;
    mountedPanelSignature = signature;
    void initialize(hostNode, ++hostToken);
  });
</script>

<!--
  Mirror the proven SourceDockviewShell structure: an outer `.source-dockview-workbench-shell`
  wrapper (so all existing :global workbench-shell panel/context CSS applies to the teleported
  content) and an inner `.source-dockview-workbench-host` that the factory mounts the Dockview
  into. The wrapper is reused verbatim so no panel-sizing CSS has to be duplicated.
-->
<div
  class="source-workbench source-dockview-workbench-shell"
  class:source-workbench-ready={ready}
  class:source-workbench-error={Boolean(error)}
>
  <div
    class="source-dockview-workbench-host source-dockview-host"
    use:workbenchAction
  ></div>

  {#if error}
    <div class="source-workbench-error-banner" role="status">{error}</div>
  {/if}
</div>

<style>
  .source-workbench {
    position: absolute;
    inset: 0;
    z-index: 1;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: var(--color-bg);
    contain: layout paint;
  }

  /*
   * The shared `.source-dockview-workbench-host` global rule (in SourceDockviewShell) keeps the
   * host `visibility: hidden` until a `.dockview-ready` ancestor flips it visible. This component
   * mounts the Dockview directly, so force the host visible here without depending on that class.
   */
  .source-workbench .source-dockview-workbench-host {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    visibility: visible;
  }

  .source-workbench-error-banner {
    position: absolute;
    right: var(--space-2);
    bottom: var(--space-2);
    z-index: 4;
    max-width: calc(100% - 16px);
    overflow: hidden;
    color: var(--color-bad);
    border: 1px solid var(--color-bad-bg-strong);
    border-radius: var(--radius-sm);
    background: var(--color-bad-bg);
    padding: var(--space-1) var(--space-2);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
