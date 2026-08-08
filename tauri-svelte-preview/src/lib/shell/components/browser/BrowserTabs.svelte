<script lang="ts">
  import Globe2 from '@lucide/svelte/icons/globe-2';
  import Plus from '@lucide/svelte/icons/plus';
  import X from '@lucide/svelte/icons/x';

  import type { BrowserTabState, BrowserWorkspaceState } from '$lib/shell/browser/browserTypes.ts';
  import { browserWorkspace } from '$lib/shell/browser/browserStore.svelte';

  interface Props {
    workspace?: BrowserWorkspaceState;
    onSelect?: (tabId: string) => void;
    onClose?: (tabId: string) => void;
    onCreate?: () => void;
  }

  let {
    workspace = browserWorkspace,
    onSelect,
    onClose,
    onCreate
  }: Props = $props();

  const tabs = $derived(
    workspace.tabOrder
      .map((id) => workspace.tabs[id])
      .filter((tab): tab is BrowserTabState => Boolean(tab))
  );
</script>

<nav class="browser-tabs" aria-label="Browser tabs" data-testid="browser-tabs">
  <div class="browser-tab-list" role="tablist">
    {#if tabs.length === 0}
      <span class="browser-tab-empty">No page open</span>
    {/if}
    {#each tabs as tab (tab.id)}
      <div class:active={tab.id === workspace.activeTabId} class="browser-tab" role="presentation">
        <button
          class="browser-tab-select"
          class:active={tab.id === workspace.activeTabId}
          type="button"
          role="tab"
          aria-selected={tab.id === workspace.activeTabId}
          aria-label={`Show ${tab.title || 'browser tab'}`}
          data-testid={`browser-tab-${tab.id}`}
          onclick={() => onSelect?.(tab.id)}
        >
          <span class="browser-tab-favicon" aria-hidden="true">
            <Globe2 size={14} strokeWidth={1.8} />
          </span>
          <span class="browser-tab-title">{tab.title || tab.url || 'New tab'}</span>
        </button>
        <button
          class="browser-tab-close"
          type="button"
          aria-label={`Close ${tab.title || 'browser tab'}`}
          title="Close tab"
          data-testid={`browser-tab-close-${tab.id}`}
          onclick={() => onClose?.(tab.id)}
        >
          <X size={14} strokeWidth={1.8} aria-hidden="true" />
        </button>
      </div>
    {/each}
  </div>
  <button
    class="browser-tab-new"
    type="button"
    aria-label="New browser tab"
    title="New browser tab"
    data-testid="browser-tab-new"
    onclick={() => onCreate?.()}
  >
    <Plus size={16} strokeWidth={1.8} aria-hidden="true" />
  </button>
</nav>

<style>
  .browser-tabs {
    display: flex;
    align-items: stretch;
    min-width: 0;
    min-height: 38px;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-text);
  }

  .browser-tab-list {
    display: flex;
    flex: 1 1 auto;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: thin;
  }

  .browser-tab,
  .browser-tab-select,
  .browser-tab-new,
  .browser-tab-close {
    font: inherit;
  }

  .browser-tab {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    min-width: 0;
    max-width: 260px;
    border-right: 1px solid var(--color-border);
  }

  .browser-tab-select,
  .browser-tab-new,
  .browser-tab-close {
    border: 0;
    background: transparent;
    color: var(--color-text-2);
    cursor: pointer;
  }

  .browser-tab-select {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    height: 38px;
    padding: 0 7px 0 12px;
    font-size: 12px;
  }

  .browser-tab-select:hover,
  .browser-tab-select.active {
    background: var(--color-elevated);
    color: var(--color-text);
  }

  .browser-tab-select.active {
    box-shadow: inset 0 -2px 0 var(--color-accent);
  }

  .browser-tab-favicon {
    display: inline-flex;
    flex: 0 0 auto;
    color: var(--color-text-3);
  }

  .browser-tab.active .browser-tab-favicon {
    color: var(--color-accent);
  }

  .browser-tab-title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .browser-tab-close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 32px;
    border-radius: 6px;
  }

  .browser-tab-close:hover,
  .browser-tab-new:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .browser-tab-new {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 38px;
    height: 38px;
    flex: 0 0 auto;
    border-left: 1px solid var(--color-border);
  }

  .browser-tab-empty {
    display: inline-flex;
    align-items: center;
    padding: 0 13px;
    color: var(--color-text-3);
    font-size: 12px;
  }
</style>
