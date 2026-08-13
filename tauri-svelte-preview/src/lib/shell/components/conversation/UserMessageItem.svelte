<script lang="ts">
  import ConversationMessage from './ConversationMessage.svelte';
  import TurnMetadata from './TurnMetadata.svelte';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  let { item, onFileLink }: { item: Extract<ConversationDisplayItem, { kind: 'user' }>; onFileLink?(path: string): void } = $props();
</script>

<div class="group" data-testid="timeline-user-message">
  <ConversationMessage text={item.text} role="user" itemId={item.itemId} completed={item.completed} {onFileLink} />
  <TurnMetadata text={item.text} timestampMs={item.timestampMs} align="end" />
</div>
