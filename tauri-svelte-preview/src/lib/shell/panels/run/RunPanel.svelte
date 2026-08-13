<script lang="ts">
  /**
   * RunPanel.svelte — the Run tab of the right column.
   *
   * Today it hosts the existing run configurations pane unchanged. The project
   * actions that replace the old top-bar play button land in this directory.
   */
  import StacksPane from '$lib/shell/components/stacks/StacksPane.svelte';

  interface Props {
    visible: boolean;
    root: string;
    ownedId: string | null;
  }
  let {}: Props = $props();
</script>

<StacksPane />
