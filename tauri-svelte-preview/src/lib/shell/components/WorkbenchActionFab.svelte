<script lang="ts">
  import { onMount } from 'svelte';
  import CircleDot from '@lucide/svelte/icons/circle-dot';
  import Check from '@lucide/svelte/icons/check';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import X from '@lucide/svelte/icons/x';
  import {
    actionsForContext,
    INITIAL_ACTION_SURFACE_STATE,
    layoutActionFan,
    reduceActionSurface,
    type ActionFanLayout,
    type ActionSurfaceState,
    type WorkbenchAction,
    type WorkbenchActionContext
  } from '$lib/shell/overlay/actionSurfaceModel.ts';

  interface Props {
    actions: WorkbenchAction[];
    context: WorkbenchActionContext;
  }

  let { actions, context }: Props = $props();
  let surface = $state<ActionSurfaceState>({ ...INITIAL_ACTION_SURFACE_STATE });
  let viewport = $state({ width: 0, height: 0 });
  let root: HTMLDivElement | null = null;
  let fabButton: HTMLButtonElement | null = null;

  const contextualActions = $derived(actionsForContext(actions, context));
  const actionEntries = $derived(
    contextualActions.map((action) => ({ action, availability: action.enabled(context) }))
  );
  const anchor = $derived({
    x: Math.max(40, viewport.width - 40),
    y: Math.max(40, viewport.height - 40)
  });
  const actionLayout = $derived<ActionFanLayout>(
    layoutActionFan(
      { width: viewport.width, height: viewport.height },
      anchor,
      actionEntries.map(({ action }) => ({ id: action.id, width: 48, height: 48 }))
    )
  );
  const pendingEntry = $derived(
    actionEntries.find(({ action }) => action.id === surface.pendingActionId) ?? null
  );
  const isOpen = $derived(surface.mode !== 'collapsed');
  const fabLabel = $derived(
    `Open ${context.kind} actions${actionEntries.length ? ` (${actionEntries.length} available)` : ''}`
  );

  function measureViewport(): void {
    if (typeof window === 'undefined') return;
    viewport = { width: window.innerWidth, height: window.innerHeight };
  }

  function setSurface(event: Parameters<typeof reduceActionSurface>[1]): void {
    surface = reduceActionSurface(surface, event);
  }

  function toggle(): void {
    setSurface({ type: 'toggle' });
    if (surface.mode === 'collapsed') {
      queueMicrotask(() => fabButton?.focus());
    }
  }

  function close(): void {
    setSurface({ type: 'close' });
    queueMicrotask(() => fabButton?.focus());
  }

  function actionPosition(actionId: string): string {
    if (actionLayout.kind === 'bottom-sheet') return '';
    const item = actionLayout.items.find((candidate) => candidate.id === actionId);
    return item ? `left: ${item.x}px; top: ${item.y}px;` : 'visibility: hidden;';
  }

  async function execute(action: WorkbenchAction): Promise<void> {
    setSurface({ type: 'start-action', actionId: action.id });
    try {
      await action.run(context);
      setSurface({ type: 'finish-action' });
      queueMicrotask(() => fabButton?.focus());
    } catch (error) {
      setSurface({
        type: 'fail-action',
        message: error instanceof Error ? error.message : String(error)
      });
    }
  }

  function choose(entry: (typeof actionEntries)[number]): void {
    if (!entry.availability.enabled || surface.mode === 'action-pending') return;
    if (entry.action.confirmation === 'none') {
      void execute(entry.action);
    } else {
      setSurface({ type: 'start-action', actionId: entry.action.id });
    }
  }

  function confirmPending(): void {
    if (pendingEntry) void execute(pendingEntry.action);
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
    if (event.key !== 'Escape' || !isOpen) return;
    event.preventDefault();
    if (surface.mode === 'action-pending') setSurface({ type: 'open' });
    else close();
  }

  function handleWindowClick(event: MouseEvent): void {
    if (isOpen && root && event.target instanceof Node && !root.contains(event.target)) close();
  }

  onMount(() => {
    measureViewport();
  });
</script>

<svelte:window onresize={measureViewport} onkeydown={handleWindowKeydown} onclick={handleWindowClick} />

<div bind:this={root} class="workbench-action-fab" data-testid="workbench-action-fab" data-layout={actionLayout.kind}>
  {#if isOpen && actionLayout.kind !== 'bottom-sheet'}
    {#each actionEntries as entry (entry.action.id)}
      {@const Icon = entry.action.icon}
      <button
        class="action-item"
        type="button"
        style={actionPosition(entry.action.id)}
        disabled={!entry.availability.enabled || surface.mode === 'action-pending'}
        aria-label={entry.availability.enabled ? entry.action.label : `${entry.action.label}: ${entry.availability.reason ?? 'Unavailable'}`}
        title={entry.availability.enabled ? entry.action.label : entry.availability.reason ?? 'Unavailable'}
        data-testid={`workbench-action-${entry.action.id}`}
        onclick={() => choose(entry)}
      >
        <Icon aria-hidden="true" />
      </button>
    {/each}
  {/if}

  {#if isOpen && actionLayout.kind === 'bottom-sheet'}
    <div class="action-sheet" role="menu" aria-label={`${context.kind} actions`}>
      {#each actionEntries as entry (entry.action.id)}
        {@const Icon = entry.action.icon}
        <button
          type="button"
          role="menuitem"
          disabled={!entry.availability.enabled || surface.mode === 'action-pending'}
          title={entry.availability.reason ?? entry.action.label}
          data-testid={`workbench-action-${entry.action.id}`}
          onclick={() => choose(entry)}
        >
          <Icon aria-hidden="true" />
          <span>{entry.action.label}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if surface.mode === 'action-pending' && pendingEntry}
    <div class="action-confirmation" role="dialog" aria-label={`Confirm ${pendingEntry.action.label}`}>
      <strong>{pendingEntry.action.confirmation === 'destructive' ? 'Confirm this action' : 'Review this action'}</strong>
      <p>{pendingEntry.action.label}</p>
      <div class="confirmation-actions">
        <button type="button" onclick={confirmPending}><Check aria-hidden="true" />Confirm</button>
        <button type="button" onclick={() => setSurface({ type: 'open' })}><X aria-hidden="true" />Cancel</button>
      </div>
    </div>
  {/if}

  {#if surface.error}
    <p class="action-error" role="alert"><TriangleAlert aria-hidden="true" />{surface.error}</p>
  {/if}

  <div class="fab-anchor">
    <button bind:this={fabButton} class="fab-trigger" type="button" aria-expanded={isOpen} aria-label={fabLabel} title={fabLabel} onclick={toggle}>
      {#if isOpen}<X aria-hidden="true" />{:else}<CircleDot aria-hidden="true" />{/if}
    </button>
    {#if isOpen}
      <span class="context-label" aria-live="polite">{context.kind} · {actionEntries.length} actions</span>
    {/if}
  </div>
</div>

<style>
  .workbench-action-fab {
    position: fixed;
    z-index: 55;
    inset: 0;
    pointer-events: none;
    isolation: isolate;
  }

  .fab-anchor,
  .action-item,
  .action-sheet,
  .action-confirmation,
  .action-error {
    pointer-events: auto;
  }

  .fab-anchor {
    position: absolute;
    right: 16px;
    bottom: 16px;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 4px;
  }

  .fab-trigger,
  .action-item {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 48px;
    min-width: 48px;
    height: 48px;
    min-height: 48px;
    border: 0;
    border-radius: 50%;
    background: var(--color-surface);
    box-shadow: var(--shadow-md);
    color: var(--color-text);
    cursor: pointer;
  }

  .fab-trigger {
    background: var(--color-accent);
    color: var(--color-on-accent);
  }

  .fab-trigger:hover,
  .action-item:hover:not(:disabled) {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .action-item {
    position: fixed;
    transform: translate(-50%, -50%);
  }

  .fab-trigger:focus-visible,
  .action-item:focus-visible,
  .action-sheet button:focus-visible,
  .confirmation-actions button:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .action-item:disabled,
  .action-sheet button:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .context-label {
    padding: 2px 6px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--color-surface) 94%, var(--color-bg));
    color: var(--color-text-3);
    font-size: 12px;
  }

  .action-sheet,
  .action-confirmation,
  .action-error {
    position: fixed;
    right: 16px;
    bottom: 76px;
    width: min(300px, calc(100vw - 32px));
    padding: 9px;
    border: 1px solid var(--color-border);
    border-radius: 7px;
    background: var(--color-surface);
    box-shadow: var(--shadow-lg);
    color: var(--color-text);
  }

  .action-sheet {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: min(60vh, 420px);
    overflow: auto;
  }

  .action-sheet button,
  .confirmation-actions button {
    display: flex;
    align-items: center;
    gap: 7px;
    min-height: 40px;
    padding: 0 9px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--color-text-2);
    font: inherit;
    cursor: pointer;
    text-align: left;
  }

  .action-sheet button:hover:not(:disabled),
  .confirmation-actions button:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .action-confirmation {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .action-confirmation p {
    margin: 0;
    color: var(--color-text-2);
    font-size: 13px;
  }

  .confirmation-actions {
    display: flex;
    gap: 6px;
  }

  .confirmation-actions button {
    flex: 1;
    justify-content: center;
    min-height: 36px;
  }

  .action-error {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    bottom: 134px;
    color: var(--color-bad);
    font-size: 12px;
  }

  :global(.workbench-action-fab svg) {
    width: 17px;
    height: 17px;
  }
</style>
