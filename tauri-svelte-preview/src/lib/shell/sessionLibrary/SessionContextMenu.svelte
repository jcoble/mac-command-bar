<script lang="ts">
  import type {
    SessionContextMenuAction,
    SessionContextMenuItem
  } from './sessionLibraryContextMenu';

  interface Props {
    x: number;
    y: number;
    items: SessionContextMenuItem[];
    onSelect(action: SessionContextMenuAction): void;
    onClose(): void;
  }

  let { x, y, items, onSelect, onClose }: Props = $props();

  const MENU_WIDTH = 224;
  const MENU_HEIGHT = 284;
  const MENU_GAP = 8;
  const viewportWidth = $derived(typeof window === 'undefined' ? x + MENU_WIDTH : window.innerWidth);
  const viewportHeight = $derived(typeof window === 'undefined' ? y + MENU_HEIGHT : window.innerHeight);
  const menuLeft = $derived(
    Math.max(8, Math.min(x, viewportWidth - MENU_WIDTH - 8))
  );
  const menuTop = $derived(
    Math.max(
      8,
      Math.min(
        y + MENU_GAP + MENU_HEIGHT <= viewportHeight - 8
          ? y + MENU_GAP
          : y - MENU_HEIGHT - MENU_GAP,
        viewportHeight - MENU_HEIGHT - 8
      )
    )
  );
</script>

<div
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
