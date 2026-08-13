<script lang="ts">
  /**
   * AgentsPanel.svelte — the Agents tab of the right column.
   *
   * Today it hosts the existing agent activity pane unchanged. Opening one of
   * the listed conversations goes through the rail's own jump, the one thing in
   * the shell that can select a session and show its surface together.
   */
  import AgentActivityPane from '$lib/shell/components/workflows/AgentActivityPane.svelte';
  import { sessionRowJump } from '$lib/shell/components/sessionRowJump';

  interface Props {
    visible: boolean;
    root: string;
    ownedId: string | null;
  }
  let {}: Props = $props();
</script>

<AgentActivityPane onOpenConversation={(id) => void sessionRowJump(id, 'session')} />
