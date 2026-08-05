<script lang="ts">
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  interface Props { item: Extract<ConversationDisplayItem, { kind: 'approval' }>; onDecision?(requestId: string, decision: string): void; }
  let { item, onDecision }: Props = $props();
</script>

<aside class="timeline-event approval-event" data-testid="timeline-approval-item"><strong>{item.title}</strong><p>{item.summary}</p>{#if item.state === 'requested'}<div class="approval-actions">{#each item.options as option (option)}<button data-testid={`approval-${option}-button`} type="button" onclick={() => onDecision?.(item.requestId, option)}>{option}</button>{/each}</div>{:else}<small>{item.state}</small>{/if}</aside>

<style>.timeline-event{padding:10px 12px;border-left:2px solid color-mix(in srgb,var(--color-accent) 50%,var(--color-border));background:color-mix(in srgb,var(--color-accent) 7%,transparent)}p{margin:5px 0;color:var(--color-text-2)}small{color:var(--color-text-2);font-size:12px}.approval-actions{display:flex;gap:7px;margin-top:9px}.approval-actions button{border:1px solid var(--color-border);border-radius:7px;background:var(--color-surface);color:inherit;padding:6px 9px}</style>
