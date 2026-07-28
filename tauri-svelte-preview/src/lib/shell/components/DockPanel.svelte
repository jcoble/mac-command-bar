<script lang="ts">
  /** The bottom dock region: a named placeholder plus the frame's reset control. */
  import PanelPlaceholder from './PanelPlaceholder.svelte';

  interface Props {
    onReset(): void;
  }
  let { onReset }: Props = $props();
</script>

<div class="dock-slot">
  <PanelPlaceholder name="Dock" hint="Secondary terminals and logs will live here." />
  <button class="reset-layout" onclick={onReset}>Reset layout</button>
</div>

<style>
  .dock-slot {
    position: relative;
    height: 100%;
  }

  .reset-layout {
    position: absolute;
    top: 6px;
    right: 8px;
    background: transparent;
    border: 1px solid #22222c;
    border-radius: 5px;
    color: #6d6d7d;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 10px;
    padding: 2px 7px;
    cursor: pointer;
  }

  .reset-layout:hover {
    color: #d8d8e0;
    border-color: #3a3a48;
  }
</style>
