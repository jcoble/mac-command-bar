<script lang="ts">
  import type { Component, Snippet } from 'svelte';
  import { ContextMenu } from 'bits-ui';

  // Re-use the same item shape as Menu.svelte
  export interface ContextMenuItem {
    id: string;
    label: string;
    /** Lucide icon component */
    icon?: Component<{ size?: number | string; class?: string }>;
    onselect?: () => void;
    danger?: boolean;
    separatorBefore?: boolean;
    disabled?: boolean;
  }

  interface Props {
    /** Array-driven API */
    items?: ContextMenuItem[];
    /** Snippet-based API: render custom panel content */
    content?: Snippet;
    /** The right-clickable area */
    children: Snippet;
  }

  let {
    items,
    content,
    children,
  }: Props = $props();
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger class="ctx-trigger">
    {@render children()}
  </ContextMenu.Trigger>

  <ContextMenu.Portal>
    <ContextMenu.Content class="menu-content" sideOffset={4}>
      {#if content}
        {@render content()}
      {:else if items}
        {#each items as item (item.id)}
          {#if item.separatorBefore}
            <ContextMenu.Separator class="menu-separator" />
          {/if}
          <ContextMenu.Item
            class="menu-item{item.danger ? ' menu-item--danger' : ''}"
            disabled={item.disabled}
            onSelect={item.onselect}
          >
            {#if item.icon}
              <span class="menu-item__icon" aria-hidden="true">
                <item.icon size={14} />
              </span>
            {/if}
            <span class="menu-item__label">{item.label}</span>
          </ContextMenu.Item>
        {/each}
      {/if}
    </ContextMenu.Content>
  </ContextMenu.Portal>
</ContextMenu.Root>

<style>
  /* ── Trigger wrapper: transparent, full area ───────────────────── */
  :global(.ctx-trigger) {
    display: contents;
  }

  /* ── Panel + items reuse the same tokens as Menu.svelte ─────────
     These class names (.menu-content, .menu-separator, .menu-item,
     .menu-item--danger, .menu-item__icon, .menu-item__label) are
     intentionally identical so both components share one visual
     language. In a future refactor they can move to a shared CSS
     module. For now they are duplicated so each file is standalone.
  ─────────────────────────────────────────────────────────────────── */

  :global(.menu-content) {
    min-width: 180px;
    max-width: 280px;
    padding: var(--space-1);
    background-color: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    outline: none;
    z-index: 9000;
    animation: menu-in 120ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes menu-in {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.97);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  :global(.menu-separator) {
    display: block;
    height: 1px;
    margin: var(--space-1) calc(-1 * var(--space-1));
    background-color: var(--color-border);
  }

  :global(.menu-item) {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-2);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    color: var(--color-text-2);
    cursor: default;
    user-select: none;
    outline: none;
    border: none;
    background: transparent;
    transition:
      background-color 80ms ease,
      color 80ms ease;
  }

  :global(.menu-item[data-highlighted]) {
    background-color: var(--color-surface);
    color: var(--color-text);
  }

  :global(.menu-item[data-disabled]) {
    opacity: 0.35;
    cursor: not-allowed;
    pointer-events: none;
  }

  :global(.menu-item--danger) {
    color: var(--color-bad);
  }

  :global(.menu-item--danger[data-highlighted]) {
    background-color: var(--color-bad-bg);
    color: var(--color-bad);
  }

  :global(.menu-item__icon) {
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
    opacity: 0.75;
  }

  :global(.menu-item[data-highlighted] .menu-item__icon) {
    opacity: 1;
  }

  :global(.menu-item__label) {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
