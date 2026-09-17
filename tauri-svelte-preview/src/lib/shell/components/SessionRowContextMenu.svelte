<script lang="ts">
  import { onMount } from 'svelte';
  import type { SessionRowMenuAction, SessionRowMenuItem } from './sessionRowMenu.ts';

  interface Props {
    x: number;
    y: number;
    items: SessionRowMenuItem[];
    onSelect(action: SessionRowMenuAction): void;
    onClose(): void;
  }

  let { x, y, items, onSelect, onClose }: Props = $props();
  let menu: HTMLDivElement | null = null;
  let left = $state(0);
  let top = $state(0);

  onMount(() => {
    left = x;
    top = y;
    const rect = menu?.getBoundingClientRect();
    if (rect) {
      left = Math.max(8, Math.min(x, window.innerWidth - rect.width - 8));
      top = Math.max(8, Math.min(y, window.innerHeight - rect.height - 8));
    }
    menu?.focus();
  });
</script>

<svelte:window
  onpointerdown={(event) => {
    if (menu && event.target instanceof Node && !menu.contains(event.target)) onClose();
  }}
  onkeydown={(event) => { if (event.key === 'Escape') onClose(); }}
/>

<div
  bind:this={menu}
  data-testid="session-row-context-menu"
  class="menu"
  role="menu"
  tabindex="-1"
  style={`left:${left}px;top:${top}px`}
  oncontextmenu={(event) => event.preventDefault()}
>
  {#each items as item (item.id)}
    {#if item.startsGroup}<span class="separator" aria-hidden="true"></span>{/if}
    <button
      type="button"
      role="menuitem"
      disabled={!item.enabled}
      class:destructive={item.destructive}
      title={item.enabled ? item.label : item.disabledReason ?? item.label}
      onclick={() => { if (item.enabled) onSelect(item.id); }}
    >{item.label}</button>
  {/each}
</div>

<style>
  .menu{position:fixed;z-index:1000;display:flex;width:224px;box-sizing:border-box;flex-direction:column;padding:5px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--color-bg);box-shadow:var(--shadow-lg);outline:none}
  button{width:100%;padding:7px 9px;border:0;border-radius:var(--radius-sm);background:transparent;color:var(--color-text-2);font:inherit;font-size:13px;font-weight:var(--weight-medium);text-align:left;cursor:pointer}
  button:hover:not(:disabled),button:focus-visible:not(:disabled){background:var(--color-surface);color:var(--color-text);outline:none}button:disabled{color:var(--color-disabled-text);cursor:not-allowed}.destructive{color:var(--color-bad)}.separator{height:1px;margin:4px;background:var(--color-border)}
</style>
