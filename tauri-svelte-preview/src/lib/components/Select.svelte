<script lang="ts">
  import type { Component } from 'svelte';
  import { Select } from 'bits-ui';
  import { ChevronDown, Check } from '@lucide/svelte';

  interface SelectItem {
    value: string;
    label: string;
    /** Lucide icon component rendered before the label. */
    icon?: Component<{ size?: number | string; class?: string }>;
  }

  interface Props {
    value?: string;
    items: SelectItem[];
    placeholder?: string;
    disabled?: boolean;
    name?: string;
  }

  let {
    value = $bindable(''),
    items,
    placeholder = 'Select…',
    disabled = false,
    name,
  }: Props = $props();

  /** Label for the currently selected item, used in the trigger display. */
  const selectedLabel = $derived(
    items.find((i) => i.value === value)?.label ?? ''
  );
</script>

<Select.Root type="single" bind:value {disabled} {name}>
  <!-- Trigger -->
  <Select.Trigger class="select-trigger" aria-label={selectedLabel || placeholder}>
    <span class="select-trigger__value" class:placeholder={!selectedLabel}>
      {selectedLabel || placeholder}
    </span>
    <span class="select-trigger__chevron" aria-hidden="true">
      <ChevronDown size={14} />
    </span>
  </Select.Trigger>

  <!-- Dropdown -->
  <Select.Portal>
    <Select.Content class="select-content" sideOffset={4} align="start">
      <Select.ScrollUpButton class="select-scroll-btn">
        <!-- small up-caret affordance; no icon import needed — CSS arrow -->
        <span class="select-scroll-caret select-scroll-caret--up" aria-hidden="true"></span>
      </Select.ScrollUpButton>

      <Select.Viewport class="select-viewport">
        {#each items as item (item.value)}
          <Select.Item
            class="select-item"
            value={item.value}
            label={item.label}
          >
            {#snippet children({ selected })}
              <!-- Optional icon slot -->
              {#if item.icon}
                {@const Icon = item.icon}
                <span class="select-item__icon" aria-hidden="true">
                  <Icon size={14} />
                </span>
              {/if}

              <span class="select-item__label">{item.label}</span>

              {#if selected}
                <span class="select-item__check" aria-hidden="true">
                  <Check size={12} />
                </span>
              {/if}
            {/snippet}
          </Select.Item>
        {:else}
          <span class="select-empty">No options</span>
        {/each}
      </Select.Viewport>

      <Select.ScrollDownButton class="select-scroll-btn">
        <span class="select-scroll-caret select-scroll-caret--down" aria-hidden="true"></span>
      </Select.ScrollDownButton>
    </Select.Content>
  </Select.Portal>
</Select.Root>

<style>
  /* ── Trigger ──────────────────────────────────────────── */
  :global(.select-trigger) {
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    width: 100%;
    min-height: 32px;
    padding: var(--space-2) var(--space-3);
    background: var(--color-surface);
    border: none;
    border-radius: var(--radius-md);
    color: var(--color-text);
    font-size: var(--text-sm);
    font-family: inherit;
    font-weight: var(--weight-normal);
    cursor: pointer;
    outline: none;
    white-space: nowrap;
    transition: background 120ms ease, box-shadow 120ms ease;
    user-select: none;
  }

  :global(.select-trigger:hover:not([data-disabled])) {
    background: var(--color-elevated);
  }

  :global(.select-trigger:focus-visible) {
    box-shadow: var(--focus-ring);
  }

  :global(.select-trigger[data-disabled]) {
    opacity: 0.4;
    cursor: not-allowed;
    pointer-events: none;
  }

  :global(.select-trigger[data-state='open'] .select-trigger__chevron) {
    transform: rotate(180deg);
  }

  .select-trigger__value {
    flex: 1;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .select-trigger__value.placeholder {
    color: var(--color-text-3);
  }

  .select-trigger__chevron {
    display: inline-flex;
    align-items: center;
    color: var(--color-text-3);
    flex-shrink: 0;
    transition: transform 160ms ease;
  }

  /* ── Content panel ────────────────────────────────────── */
  :global(.select-content) {
    z-index: 200;
    min-width: var(--bits-select-anchor-width, 160px);
    max-height: min(320px, var(--bits-select-content-available-height, 320px));
    background: var(--color-bg);
    /* hairline border + elevation */
    box-shadow:
      0 0 0 1px var(--color-border),
      var(--shadow-md);
    border-radius: var(--radius-md);
    outline: none;
    overflow: hidden;

    animation: select-in 140ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  :global(.select-content[data-state='closed']) {
    animation: select-out 110ms ease forwards;
  }

  @keyframes select-in {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @keyframes select-out {
    from {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
    to {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
  }

  /* ── Viewport (scroll container) ─────────────────────── */
  :global(.select-viewport) {
    padding: var(--space-1);
    overflow-y: auto;
    max-height: inherit;
  }

  /* ── Items ────────────────────────────────────────────── */
  :global(.select-item) {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    font-family: inherit;
    color: var(--color-text);
    cursor: default;
    outline: none;
    user-select: none;
    transition: background 80ms ease;
  }

  :global(.select-item[data-highlighted]) {
    background: var(--color-elevated);
  }

  :global(.select-item[data-selected]) {
    color: var(--color-accent);
  }

  :global(.select-item[data-disabled]) {
    opacity: 0.4;
    pointer-events: none;
  }

  .select-item__icon {
    display: inline-flex;
    align-items: center;
    color: var(--color-text-3);
    flex-shrink: 0;
  }

  .select-item__label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .select-item__check {
    display: inline-flex;
    align-items: center;
    margin-left: auto;
    color: var(--color-accent);
    flex-shrink: 0;
  }

  /* ── Scroll buttons ───────────────────────────────────── */
  :global(.select-scroll-btn) {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 20px;
    cursor: default;
    color: var(--color-text-3);
  }

  .select-scroll-caret {
    display: inline-block;
    width: 0;
    height: 0;
  }

  .select-scroll-caret--up {
    border-left: 4px solid transparent;
    border-right: 4px solid transparent;
    border-bottom: 5px solid currentColor;
  }

  .select-scroll-caret--down {
    border-left: 4px solid transparent;
    border-right: 4px solid transparent;
    border-top: 5px solid currentColor;
  }

  /* ── Empty state ──────────────────────────────────────── */
  .select-empty {
    display: block;
    padding: var(--space-3) var(--space-4);
    font-size: var(--text-sm);
    color: var(--color-text-3);
    text-align: center;
  }
</style>
