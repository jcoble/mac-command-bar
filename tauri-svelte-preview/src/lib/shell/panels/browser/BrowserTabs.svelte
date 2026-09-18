<script lang="ts">
  import { tick } from 'svelte';
  import Globe2 from '@lucide/svelte/icons/globe-2';
  import Plus from '@lucide/svelte/icons/plus';
  import X from '@lucide/svelte/icons/x';

  import type { BrowserTabState } from '$lib/shell/browser/browserTypes.ts';

  interface Props {
    tabs: readonly BrowserTabState[];
    activeTabId: string | null;
    disabled?: boolean;
    onSelect(id: string): void;
    onClose(id: string): void;
    onNew(): void;
  }

  let {
    tabs,
    activeTabId,
    disabled = false,
    onSelect,
    onClose,
    onNew
  }: Props = $props();

  let track = $state<HTMLDivElement | null>(null);

  $effect(() => {
    activeTabId;
    const host = track;
    if (!host || !activeTabId) return;
    void tick().then(() => {
      const active = host.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]');
      active?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    });
  });

  function chooseByKeyboard(event: KeyboardEvent, currentId: string): void {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const current = tabs.findIndex((tab) => tab.id === currentId);
    const next = event.key === 'Home'
      ? 0
      : event.key === 'End'
        ? tabs.length - 1
        : event.key === 'ArrowLeft'
          ? Math.max(0, current - 1)
          : Math.min(tabs.length - 1, current + 1);
    const tab = tabs[next];
    if (!tab) return;
    onSelect(tab.id);
    void tick().then(() => {
      track?.querySelector<HTMLElement>(`[data-browser-tab-id="${CSS.escape(tab.id)}"]`)?.focus();
    });
  }

  /** A mouse wheel has no horizontal axis. Use its vertical movement on this
   *  one-axis strip; trackpad sideways gestures keep their native behavior. */
  function scrollTabs(event: WheelEvent): void {
    if (!track || Math.abs(event.deltaX) >= Math.abs(event.deltaY)) return;
    const limit = track.scrollWidth - track.clientWidth;
    if (limit <= 0) return;
    const next = Math.max(0, Math.min(limit, track.scrollLeft + event.deltaY));
    if (next === track.scrollLeft) return;
    event.preventDefault();
    track.scrollLeft = next;
  }
</script>

<div class="browser-tabs" aria-label="Browser pages">
  <div
    class="tab-track"
    bind:this={track}
    role="tablist"
    aria-label="Open browser pages"
    onwheel={scrollTabs}
  >
    {#each tabs as tab (tab.id)}
      <div class="tab-shell" class:active={tab.id === activeTabId}>
        <button
          type="button"
          class="tab"
          role="tab"
          aria-selected={tab.id === activeTabId}
          tabindex={tab.id === activeTabId ? 0 : -1}
          data-browser-tab-id={tab.id}
          title={tab.title || tab.url || 'New browser tab'}
          onclick={() => onSelect(tab.id)}
          onkeydown={(event) => chooseByKeyboard(event, tab.id)}
        >
          <Globe2 aria-hidden="true" />
          <span>{tab.title || tab.url || 'New tab'}</span>
        </button>
        <button
          type="button"
          class="close"
          aria-label={`Close ${tab.title || tab.url || 'browser tab'}`}
          title="Close tab"
          onclick={() => onClose(tab.id)}
        >
          <X aria-hidden="true" />
        </button>
      </div>
    {/each}
  </div>
  <button
    type="button"
    class="new-tab"
    aria-label="New browser tab"
    title="New browser tab"
    {disabled}
    onclick={onNew}
  >
    <Plus aria-hidden="true" />
  </button>
</div>

<style>
  .browser-tabs {
    display: flex;
    height: 44px;
    min-width: 0;
    align-items: center;
    gap: 4px;
    border-bottom: 1px solid var(--color-border);
    padding: 0 8px;
    background: transparent;
  }

  .tab-track {
    display: flex;
    min-width: 0;
    flex: 1 1 auto;
    gap: 4px;
    overflow-x: auto;
    overflow-y: hidden;
    scroll-behavior: smooth;
    scrollbar-width: none;
  }

  .tab-track::-webkit-scrollbar {
    display: none;
  }

  .tab-shell {
    display: flex;
    flex: 0 0 auto;
    width: clamp(104px, 38%, 184px);
    height: 32px;
    min-width: 0;
    align-items: center;
    border: 1px solid transparent;
    border-radius: 9px;
    color: var(--color-text-2);
  }

  .tab-shell:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .tab-shell.active {
    border-color: var(--color-border);
    background: var(--color-elevated);
    color: var(--color-text);
  }

  .tab,
  .close,
  .new-tab {
    appearance: none;
    border: 0;
    padding: 0;
    background: transparent;
    color: inherit;
    font: inherit;
  }

  .tab {
    display: flex;
    height: 100%;
    min-width: 0;
    flex: 1 1 auto;
    align-items: center;
    gap: 7px;
    padding: 0 4px 0 9px;
    text-align: left;
  }

  .tab :global(svg) {
    width: 15px;
    height: 15px;
    flex: none;
  }

  .tab span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    font-weight: 550;
  }

  .close,
  .new-tab {
    display: grid;
    width: 26px;
    height: 26px;
    flex: none;
    place-items: center;
    border-radius: 7px;
  }

  .close {
    margin-right: 2px;
    color: var(--color-text-3);
  }

  .close:hover,
  .new-tab:hover:not(:disabled) {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .close :global(svg),
  .new-tab :global(svg) {
    width: 15px;
    height: 15px;
  }

  .new-tab {
    color: var(--color-text-2);
  }

  .new-tab:disabled {
    opacity: 0.45;
  }

  .tab:focus-visible,
  .close:focus-visible,
  .new-tab:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: -2px;
  }

  @media (prefers-reduced-motion: reduce) {
    .tab-track {
      scroll-behavior: auto;
    }
  }
</style>
