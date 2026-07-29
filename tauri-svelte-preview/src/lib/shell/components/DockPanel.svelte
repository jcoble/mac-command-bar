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
  <ProblemsPanel>
    {#snippet headerEnd()}
      <button class="reset-layout" onclick={onReset}>Reset layout</button>
    {/snippet}
  </ProblemsPanel>
</div>

<style>
  .dock-slot {
    position: relative;
    height: 100%;
  }

  /* On the end of the panel's own header row. It used to float in that corner,
     which put it straight on top of the panel's Refresh button. */
  .reset-layout {
    flex-shrink: 0;
    background: transparent;
    border: 1px solid #22222c;
    border-radius: 5px;
    color: #6d6d7d;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 12px;
    padding: 2px 7px;
    cursor: pointer;
  }

  .reset-layout:hover {
    color: #d8d8e0;
    border-color: #3a3a48;
  }
</style>
