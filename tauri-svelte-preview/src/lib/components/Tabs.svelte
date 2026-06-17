<!--
  Tabs.svelte — bits-ui v2 Tabs wrapper
  Props:
    value        — $bindable string, active tab value
    tabs         — { value: string; label: string; icon?: Component }[]
    panel        — snippet(value: string) rendering content for the active tab
                   OR use children snippet for full layout control

  Aesthetic: borderless, spacious, active tab underline in --color-accent.
-->
<script lang="ts">
  import type { Component, Snippet } from 'svelte';
  import { Tabs } from 'bits-ui';

  interface TabDef {
    value: string;
    label: string;
    icon?: Component;
  }

  interface Props {
    value?: string;
    tabs: TabDef[];
    /** Render the panel content for the active tab. Receives the active `value`. */
    panel?: Snippet<[string]>;
    /** Alternative: full children snippet — you control Tabs.Content yourself. */
    children?: Snippet;
  }

  let {
    value = $bindable(undefined),
    tabs,
    panel,
    children,
  }: Props = $props();
</script>

<Tabs.Root bind:value class="tabs-root">
  <Tabs.List class="tabs-list">
    {#each tabs as tab (tab.value)}
      <Tabs.Trigger value={tab.value} class="tabs-trigger">
        {#if tab.icon}
          <span class="tabs-trigger__icon" aria-hidden="true">
            <tab.icon />
          </span>
        {/if}
        <span class="tabs-trigger__label">{tab.label}</span>
      </Tabs.Trigger>
    {/each}
  </Tabs.List>

  {#if panel}
    {#each tabs as tab (tab.value)}
      <Tabs.Content value={tab.value} class="tabs-content">
        {@render panel(tab.value)}
      </Tabs.Content>
    {/each}
  {:else if children}
    {@render children()}
  {/if}
</Tabs.Root>

<style>
  /* NOTE: .tabs-root / .tabs-list / .tabs-trigger / .tabs-content are classes
     forwarded to bits-ui-rendered DOM elements, so they must be targeted with
     :global(...) — Svelte's scope hash is never added to a child component's
     markup. Only .tabs-trigger__label / __icon are local elements in this file. */
  :global(.tabs-root) {
    display: flex;
    flex-direction: column;
    width: 100%;
  }

  /* ── Tab list (pill row, borderless) ─────────────────── */
  :global(.tabs-list) {
    display: flex;
    flex-direction: row;
    gap: var(--space-1);
    padding-block-end: var(--space-1);
    /* subtle separator from content — one pixel, transparent-ish */
    border-bottom: 1px solid var(--color-border);
  }

  /* ── Individual trigger ──────────────────────────────── */
  :global(.tabs-trigger) {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px; /* sit on top of the list border-bottom */
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
    font-family: inherit;
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    color: var(--color-text-3);
    cursor: pointer;
    outline: none;
    user-select: none;
    white-space: nowrap;
    transition:
      color 130ms ease,
      border-color 130ms ease,
      background-color 130ms ease;
  }

  :global(.tabs-trigger:hover:not([data-disabled])) {
    color: var(--color-text-2);
    background-color: var(--color-surface);
  }

  /* Active state via bits-ui data attribute */
  :global(.tabs-trigger[data-state='active']) {
    color: var(--color-text);
    border-bottom-color: var(--color-accent);
    font-weight: var(--weight-semibold);
  }

  :global(.tabs-trigger:focus-visible) {
    box-shadow: var(--focus-ring);
  }

  :global(.tabs-trigger[data-disabled]) {
    opacity: 0.38;
    cursor: not-allowed;
    pointer-events: none;
  }

  /* ── Icon inside trigger ─────────────────────────────── */
  .tabs-trigger__icon {
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
  }

  .tabs-trigger__icon :global(svg) {
    width: 14px;
    height: 14px;
  }

  /* ── Content panel ───────────────────────────────────── */
  :global(.tabs-content) {
    padding-block-start: var(--space-4);
    outline: none;
  }

  :global(.tabs-content:focus-visible) {
    box-shadow: var(--focus-ring);
  }
</style>
