<script lang="ts">
  import ConversationMessage from './ConversationMessage.svelte';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  let { item, onFileLink }: { item: Extract<ConversationDisplayItem, { kind: 'reasoning' }>; onFileLink?(path: string): void } = $props();
</script>

<details class="reasoning" data-testid="timeline-reasoning-item" open>
  <summary>Reasoning</summary>
  <ConversationMessage text={item.text} role="assistant" label="Reasoning" itemId={item.itemId} completed={item.completed} {onFileLink} />
</details>

<style>.reasoning{padding:10px 12px;border-left:2px solid color-mix(in srgb,var(--color-text-2) 65%,transparent);color:var(--color-text-2)}summary{cursor:pointer;margin-bottom:8px;font-size:12px;font-weight:650;letter-spacing:.04em;text-transform:uppercase}</style>
