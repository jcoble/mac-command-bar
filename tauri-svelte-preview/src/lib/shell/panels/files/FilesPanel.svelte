<script lang="ts">
  /**
   * FilesPanel.svelte — the Files tab of the right column.
   *
   * Today it hosts the existing file tree unchanged, so nothing is lost while
   * the workbench is being rearranged. The rebuilt tree lands in this directory.
   */
  import ExplorerPanel from '$lib/shell/components/ExplorerPanel.svelte';

  interface Props {
    /** True while this panel's tab is the selected one. */
    visible: boolean;
    /** The active session's working folder, or '' when nothing is selected. */
    root: string;
    /** The active session's ownedId, or null. */
    ownedId: string | null;
  }
  // The file tree reads the folder it is pointed at from its own service; the
  // three props above are the contract every panel in this column takes.
  let {}: Props = $props();
</script>

<ExplorerPanel />
