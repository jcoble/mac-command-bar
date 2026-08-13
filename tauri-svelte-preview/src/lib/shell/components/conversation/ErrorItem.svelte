<script lang="ts">
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  let { item }: { item: Extract<ConversationDisplayItem, { kind: 'error' }> } = $props();
</script>

<aside class="error-event" data-testid="timeline-error-item">
  <span class="event-label">Conversation error</span>
  <p>{item.text}</p>
</aside>

<style>
  .error-event{padding:12px;border:1px solid color-mix(in srgb,var(--color-bad) 34%,var(--color-border));border-radius:8px;background:var(--color-bad-bg)}
  .event-label{display:block;color:var(--color-bad);font-size:12px;letter-spacing:.06em;text-transform:uppercase}
  p{margin:8px 0 0;color:var(--color-text-2);font-size:13px;line-height:1.5;white-space:pre-wrap}
</style>
