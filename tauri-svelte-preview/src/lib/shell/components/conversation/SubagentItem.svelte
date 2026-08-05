<script lang="ts">
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  let { item }: { item: Extract<ConversationDisplayItem, { kind: 'subagent' }> } = $props();
</script>

<aside class="timeline-event" data-testid="timeline-subagent-item"><div class="event-heading"><strong>{item.label}</strong><span>{item.state}</span></div><small>Read-only child · {item.childId}</small></aside>

<style>.timeline-event{padding:10px 12px;border-left:2px solid color-mix(in srgb,var(--color-accent) 50%,var(--color-border));background:color-mix(in srgb,var(--color-surface) 45%,transparent)}.event-heading{display:flex;justify-content:space-between;gap:12px}.event-heading span,small{color:var(--color-text-2);font-size:12px}small{display:block;margin-top:5px}</style>
