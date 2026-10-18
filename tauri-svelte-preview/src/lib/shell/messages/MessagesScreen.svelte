<!--
  MessagesScreen.svelte — every agent-to-agent message on every machine, over the
  whole window like the Projects screen. Back or Escape closes it, and so does a
  chat actually opening from the reader.
-->
<script lang="ts">
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';

  import { Button } from '$lib/components/ui/button/index.js';
  import AgentMessages from '$lib/shell/panels/agents/AgentMessages.svelte';

  interface Props {
    runId: string | null;
    runLabel: string;
    onBack: () => void;
    onClearRun: () => void;
    onShowRun: (runId: string, label: string) => void;
  }
  let { runId, runLabel, onBack, onClearRun, onShowRun }: Props = $props();
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="messages-screen"
  role="region"
  aria-label="Messages"
  data-testid="messages-screen"
  onkeydown={(event) => {
    if (event.key === 'Escape') onBack();
  }}
>
  <header class="flex items-center gap-2">
    <Button variant="ghost" size="sm" onclick={onBack}><ArrowLeft />Back</Button>
    <h1 class="title">Messages</h1>
  </header>
  <div class="min-h-0 flex-1">
    <AgentMessages visible={true} layout="screen" session={null} {runId} {runLabel} {onClearRun} {onShowRun} onOpened={onBack} />
  </div>
</div>

<style>
  .messages-screen {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-5) var(--space-5);
    background: var(--color-bg);
  }

  .title {
    margin: 0;
    font-size: var(--text-heading);
    font-weight: var(--text-heading-weight);
  }
</style>
