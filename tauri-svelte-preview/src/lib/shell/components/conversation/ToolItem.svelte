<script lang="ts">
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  let { item }: { item: Extract<ConversationDisplayItem, { kind: 'tool' }> } = $props();
</script>

<aside class="timeline-event" data-testid="timeline-tool-item"><div class="event-heading"><strong>{item.name}</strong><span>{item.state}</span></div>{#if item.summary}<p>{item.summary}</p>{/if}</aside>

<style>.timeline-event{padding:10px 12px;border-left:2px solid color-mix(in srgb,var(--color-accent) 50%,var(--color-border));background:color-mix(in srgb,var(--color-surface) 45%,transparent)}.event-heading{display:flex;justify-content:space-between;gap:12px}.event-heading span,p{color:var(--color-text-2);font-size:12px}p{margin:6px 0 0}</style>
