<script lang="ts">
  import ConversationMessage from './ConversationMessage.svelte';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  let { item, label = 'Assistant', onFileLink }: { item: Extract<ConversationDisplayItem, { kind: 'assistant' }>; label?: string; onFileLink?(path: string): void } = $props();
</script>

<div data-testid="timeline-assistant-message"><ConversationMessage text={item.text} role="assistant" {label} itemId={item.itemId} completed={item.completed} {onFileLink} /></div>
