<script lang="ts">
  import { onMount } from 'svelte';

  import type {
    SessionContextMenuAction,
    SessionContextMenuItem
  } from './sessionLibraryContextMenu';

  export interface SessionContextMenuAnchor {
    left: number;
    right: number;
    top: number;
    bottom: number;
  }

  interface Props {
    anchor: SessionContextMenuAnchor;
    items: SessionContextMenuItem[];
    onSelect(action: SessionContextMenuAction): void;
    onClose(): void;
  }

  let { anchor, items, onSelect, onClose }: Props = $props();

  const MENU_GAP = 8;
  const VIEWPORT_PADDING = 8;
  let menuElement: HTMLDivElement | null = null;
  let measuredWidth = $state(0);
  let measuredHeight = $state(0);
  let viewportWidth = $state(typeof window === 'undefined' ? 0 : window.innerWidth);
  let viewportHeight = $state(typeof window === 'undefined' ? 0 : window.innerHeight);

  const menuLeft = $derived.by(() => {
    const width = measuredWidth;
    const availableWidth = viewportWidth || anchor.right + VIEWPORT_PADDING;
    const rightPosition = anchor.right + MENU_GAP;
    const leftPosition = anchor.left - width - MENU_GAP;
    if (width === 0 || rightPosition + width <= availableWidth - VIEWPORT_PADDING) {
      return Math.max(VIEWPORT_PADDING, rightPosition);
    }
    if (leftPosition >= VIEWPORT_PADDING) return leftPosition;
    return Math.max(
      VIEWPORT_PADDING,
      Math.min(rightPosition, availableWidth - width - VIEWPORT_PADDING)
    );
  });

  const menuTop = $derived.by(() => {
    const height = measuredHeight;
    const availableHeight = viewportHeight || anchor.bottom + VIEWPORT_PADDING;
    const belowPosition = anchor.bottom + MENU_GAP;
    const abovePosition = anchor.top - height - MENU_GAP;
    if (height === 0 || belowPosition + height <= availableHeight - VIEWPORT_PADDING) {
      return Math.max(VIEWPORT_PADDING, belowPosition);
    }
    if (abovePosition >= VIEWPORT_PADDING) return abovePosition;
    return Math.max(
      VIEWPORT_PADDING,
      Math.min(belowPosition, availableHeight - height - VIEWPORT_PADDING)
    );
  });

  onMount(() => {
    const measure = (): void => {
      const rect = menuElement?.getBoundingClientRect();
      if (!rect) return;
      measuredWidth = rect.width;
      measuredHeight = rect.height;
    };
    const updateViewport = (): void => {
      viewportWidth = window.innerWidth;
      viewportHeight = window.innerHeight;
      measure();
    };
    measure();
    window.addEventListener('resize', updateViewport);
    let observer: ResizeObserver | null = null;
    if (menuElement && typeof ResizeObserver !== 'undefined') {
      observer = new ResizeObserver(measure);
      observer.observe(menuElement);
    }
    return () => {
      observer?.disconnect();
      window.removeEventListener('resize', updateViewport);
    };
  });
</script>

<div
  bind:this={menuElement}
  data-testid="session-context-menu"
  class="session-context-menu"
  role="menu"
  tabindex="-1"
  aria-label="Session actions"
  style={`left: ${menuLeft}px; top: ${menuTop}px`}
  onclick={(event) => event.stopPropagation()}
  oncontextmenu={(event) => event.preventDefault()}
  onkeydown={(event) => {
    if (event.key === 'Escape') onClose();
  }}
>
  {#each items as item (item.id)}
    <button
      data-testid={`session-context-menu-${item.id}`}
      type="button"
      role="menuitem"
      class="menu-item"
      class:disabled={!item.enabled}
      disabled={!item.enabled}
      aria-disabled={!item.enabled}
      title={item.enabled ? item.label : item.disabledReason ?? item.label}
      onclick={() => {
        if (item.enabled) onSelect(item.id);
      }}
    >{item.label}</button>
  {/each}
</div>

<style>
  .session-context-menu { position: fixed; z-index: 1000; display: flex; min-width: 216px; flex-direction: column; gap: 2px; border: 1px solid var(--color-border); border-radius: var(--radius-md); padding: 5px; background: var(--color-surface); box-shadow: var(--shadow-lg); }
  .menu-item { width: 100%; border-radius: var(--radius-sm); padding: 8px 9px; color: var(--color-text); font-size: 13px; text-align: left; }
  .menu-item:hover:not(:disabled), .menu-item:focus-visible:not(:disabled) { background: var(--color-elevated); outline: none; box-shadow: var(--focus-ring); }
  .menu-item.disabled { cursor: not-allowed; color: var(--color-disabled-text); }
</style>
