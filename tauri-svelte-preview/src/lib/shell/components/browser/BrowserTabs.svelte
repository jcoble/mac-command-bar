<script lang="ts">
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
          <span class="browser-tab-status" aria-hidden="true"></span>
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
          <span aria-hidden="true">×</span>
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
    <span aria-hidden="true">+</span>
  </button>
</nav>

<style>
  .browser-tabs {
    display: flex;
    align-items: stretch;
    min-width: 0;
    min-height: 35px;
    border-bottom: 1px solid var(--color-border, #858599);
    background: var(--color-bg, #101014);
  }

  .browser-tab-list {
    display: flex;
    flex: 1 1 auto;
    min-width: 0;
    overflow-x: auto;
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
    max-width: 240px;
    border-right: 1px solid var(--color-border, #858599);
  }

  .browser-tab-select,
  .browser-tab-new,
  .browser-tab-close {
    color: var(--color-text-2, #a7a7b5);
    border: 0;
    background: transparent;
    cursor: pointer;
  }

  .browser-tab-select {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    height: 35px;
    padding: 0 8px 0 11px;
    font-size: 12px;
  }

  .browser-tab-select:hover,
  .browser-tab-select.active {
    color: var(--color-text, #eef0f9);
    background: var(--color-surface, #17171d);
  }

  .browser-tab-status {
    width: 6px;
    height: 6px;
    flex: 0 0 auto;
    border-radius: 50%;
    background: var(--color-text-3, #767687);
  }

  .browser-tab.active .browser-tab-status {
    background: var(--color-accent, #4bf3c8);
  }

  .browser-tab-title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .browser-tab-close {
    width: 28px;
    height: 35px;
    font-size: 16px;
    line-height: 1;
  }

  .browser-tab-close:hover,
  .browser-tab-new:hover {
    color: var(--color-text, #eef0f9);
    background: var(--color-hover, #25252e);
  }

  .browser-tab-new {
    width: 35px;
    height: 35px;
    flex: 0 0 auto;
    border-left: 1px solid var(--color-border, #858599);
    font-size: 18px;
  }

  .browser-tab-empty {
    display: inline-flex;
    align-items: center;
    padding: 0 12px;
    color: var(--color-text-3, #767687);
    font-size: 12px;
  }
</style>
