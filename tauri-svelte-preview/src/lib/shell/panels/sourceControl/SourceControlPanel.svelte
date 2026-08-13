<script lang="ts">
  /**
   * SourceControlPanel.svelte — the Source control tab of the right column.
   *
   * Today it hosts the existing source control pane unchanged. Picking a file
   * goes through the workbench navigation module rather than a prop, so no
   * panel has to know where the diff is drawn.
   */
  import GitPanel from '$lib/shell/components/GitPanel.svelte';
  import { showCenterTab } from '$lib/shell/workbenchNavigation';

  interface Props {
    visible: boolean;
    root: string;
    ownedId: string | null;
  }
  let {}: Props = $props();
</script>

<GitPanel onShowDiff={() => showCenterTab('diff')} />
