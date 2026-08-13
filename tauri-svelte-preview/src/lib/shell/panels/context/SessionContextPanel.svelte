<script lang="ts">
  /**
   * SessionContextPanel.svelte — the Context tab of the right column.
   *
   * Today it hosts the existing activity dashboard unchanged. The per-session
   * context readout that replaces it lands in this directory.
   */
  import ContextPanel from '$lib/shell/components/ContextPanel.svelte';

  interface Props {
    visible: boolean;
    root: string;
    ownedId: string | null;
  }
  let {}: Props = $props();
</script>

<ContextPanel />
