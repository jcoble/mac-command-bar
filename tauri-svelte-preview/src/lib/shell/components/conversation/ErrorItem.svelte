<script lang="ts">
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  let { item }: { item: Extract<ConversationDisplayItem, { kind: 'error' }> } = $props();
</script>

<aside class="timeline-event error-event" data-testid="timeline-error-item"><strong>Conversation error</strong><p>{item.text}</p></aside>

<style>.timeline-event{padding:10px 12px;border-left:2px solid #d77b84;background:color-mix(in srgb,#d77b84 8%,transparent)}p{margin:5px 0;color:var(--color-text-2)}</style>
