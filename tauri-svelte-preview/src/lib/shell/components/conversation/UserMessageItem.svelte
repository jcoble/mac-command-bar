<script lang="ts">
  import ConversationMessage from './ConversationMessage.svelte';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  let { item, onFileLink }: { item: Extract<ConversationDisplayItem, { kind: 'user' }>; onFileLink?(path: string): void } = $props();
</script>

<div data-testid="timeline-user-message"><ConversationMessage text={item.text} role="user" label="You" itemId={item.itemId} completed={item.completed} {onFileLink} /></div>
