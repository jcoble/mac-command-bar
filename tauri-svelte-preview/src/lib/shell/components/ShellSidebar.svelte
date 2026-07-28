<script lang="ts">
  /**
   * ShellSidebar.svelte — the left column: the session rail above, this
   * project's files below. Presentational only; every prop is forwarded to
   * SessionRail unchanged, and the file tree takes none.
   *
   * The two halves each scroll on their own (`min-height: 0` is what makes that
   * work inside a flex column), so a long session list never pushes the file
   * tree out of the shell.
   */
  import type { ComponentProps } from 'svelte';

  import ExplorerPanel from './ExplorerPanel.svelte';
  import SessionRail from './SessionRail.svelte';

  let railProps: ComponentProps<typeof SessionRail> = $props();
</script>

<div class="sidebar">
  <div class="sidebar-part sidebar-sessions">
    <SessionRail {...railProps} />
  </div>
  <div class="sidebar-part sidebar-files">
    <ExplorerPanel />
  </div>
</div>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    overflow: hidden;
    background: #101014;
  }

  .sidebar-part {
    min-height: 0;
    overflow: hidden;
  }

  .sidebar-sessions {
    flex: 1 1 45%;
  }

  .sidebar-files {
    flex: 1 1 55%;
    border-top: 1px solid #22222c;
  }
</style>
