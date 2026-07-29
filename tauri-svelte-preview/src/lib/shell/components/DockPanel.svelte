<script lang="ts">
  /**
   * The bottom dock region: the Problems panel, plus the frame's reset control.
   *
   * This region is on screen from the moment the shell opens, so the panel it
   * hosts must load nothing when it mounts — `ProblemsPanel` reads the language
   * server only when the user presses its Refresh button. See
   * `panelActivation.ts` for the rule and the route that is waiting for a
   * gesture to fire it.
   */
  import ProblemsPanel from './problems/ProblemsPanel.svelte';

  interface Props {
    onReset(): void;
  }
  let { onReset }: Props = $props();
</script>

<div class="dock-slot">
  <ProblemsPanel />
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
