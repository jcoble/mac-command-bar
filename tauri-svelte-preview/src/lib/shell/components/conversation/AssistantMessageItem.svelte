<script lang="ts">
  import ConversationMessage from './ConversationMessage.svelte';
  import TurnMetadata from './TurnMetadata.svelte';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  let { item, onFileLink }: { item: Extract<ConversationDisplayItem, { kind: 'assistant' }>; onFileLink?(path: string): void } = $props();
</script>

<!-- Text in flight stays in the plain reading plane. The time and copy action
     only appear once the message has settled. -->
<div class="group" data-testid="timeline-assistant-message">
  <ConversationMessage text={item.text} role="assistant" itemId={item.itemId} completed={item.completed} {onFileLink} />
  {#if item.completed}<TurnMetadata text={item.text} timestampMs={item.timestampMs} />{/if}
</div>

<style>.group{position:relative}</style>
