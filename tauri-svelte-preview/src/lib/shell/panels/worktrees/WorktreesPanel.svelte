<script lang="ts">
  /**
   * WorktreesPanel.svelte — the Worktrees tab of the right column.
   *
   * Today it hosts the existing worktree manager unchanged. Opening one of a
   * worktree's sessions goes through the rail's own jump, which is the one
   * thing in the shell that can both select a session and show its surface.
   */
  import WorktreeManagerPane from '$lib/shell/components/worktrees/WorktreeManagerPane.svelte';
  import { sessionRowJump } from '$lib/shell/components/sessionRowJump';

  interface Props {
    visible: boolean;
    root: string;
    ownedId: string | null;
  }
  let {}: Props = $props();
</script>

<WorktreeManagerPane onOpenSession={(id) => void sessionRowJump(id, 'session')} />
