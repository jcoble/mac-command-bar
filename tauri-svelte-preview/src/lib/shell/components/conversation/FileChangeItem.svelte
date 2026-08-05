<script lang="ts">
  import ConversationMessage from './ConversationMessage.svelte';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  let { item, onFileLink }: { item: Extract<ConversationDisplayItem, { kind: 'file' }>; onFileLink?(path: string): void } = $props();
</script>

<aside class="timeline-event file-event" data-testid="timeline-file-change-item"><span class="event-label">File change</span><ConversationMessage text={item.text} role="assistant" label="File change" itemId={item.itemId} completed={item.completed} {onFileLink} /></aside>

<style>.timeline-event{padding:10px 12px;border-left:2px solid color-mix(in srgb,var(--color-accent) 50%,var(--color-border));background:color-mix(in srgb,var(--color-surface) 45%,transparent)}.event-label{display:block;margin-bottom:6px;color:var(--color-text-2);font-size:12px;font-weight:650;letter-spacing:.04em;text-transform:uppercase}</style>
