<script lang="ts">
  import ConversationMessage from './ConversationMessage.svelte';
  import TurnMetadata from './TurnMetadata.svelte';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  let { item, onFileLink }: { item: Extract<ConversationDisplayItem, { kind: 'user' }>; onFileLink?(path: string): void } = $props();
</script>

<div class="group" data-testid="timeline-user-message">
  {#if item.attachments?.length}
    <div class="sent-images" data-testid="timeline-user-attachments">
      {#each item.attachments as attachment (attachment.id)}
        <figure>
          <img src={attachment.previewUrl} alt={attachment.name} />
          <figcaption>{attachment.name}</figcaption>
        </figure>
      {/each}
    </div>
  {/if}
  <ConversationMessage text={item.text} role="user" itemId={item.itemId} completed={item.completed} {onFileLink} />
  <TurnMetadata text={item.text} timestampMs={item.timestampMs} align="end" />
</div>

<style>
  .group{position:relative}
  .sent-images { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 8px; margin-bottom: 6px; }
  .sent-images figure { display: grid; gap: 4px; max-width: 220px; margin: 0; }
  .sent-images img { max-width: 220px; max-height: 160px; border: 1px solid color-mix(in srgb, var(--color-border) 76%, transparent); border-radius: 10px; object-fit: cover; }
  .sent-images figcaption { overflow: hidden; color: var(--color-text-2); font-size: 12px; text-align: right; text-overflow: ellipsis; white-space: nowrap; }
</style>
