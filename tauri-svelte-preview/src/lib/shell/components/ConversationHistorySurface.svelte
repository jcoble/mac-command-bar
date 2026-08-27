<script lang="ts">
  import type { OwnedSession } from '$lib/shell/ownedSessions.ts';
  import { conversationSessions, setConversationScrollTop } from '$lib/shell/conversation/conversationStore.svelte.ts';
  import {
    typedConversationTimeline,
    type ConversationDisplayItem
  } from '$lib/shell/conversation/conversationTimeline.ts';
  import ConversationTimeline from './conversation/ConversationTimeline.svelte';

  interface Props {
    owned: OwnedSession[];
    activeOwnedId: string | null;
    composerHeight?: number;
  }

  let { owned, activeOwnedId, composerHeight = 0 }: Props = $props();
  const active = $derived(owned.find((session) => session.ownedId === activeOwnedId) ?? null);
  const conversation = $derived(activeOwnedId ? conversationSessions[activeOwnedId] ?? null : null);
  let previousTimelineKey = '';
  let previousItems: ConversationDisplayItem[] = [];
  const items = $derived.by(() => {
    if (!conversation) return [];
    if (conversation.ownedId !== previousTimelineKey) {
      previousTimelineKey = conversation.ownedId;
      previousItems = [];
    }
    previousItems = typedConversationTimeline(
      conversation.agentItems,
      conversation.timeline,
      {},
      previousItems,
      conversation.sentAttachments
    );
    return previousItems;
  });

  function handleScroll(scrollTop: number): void {
    if (active) setConversationScrollTop(active.ownedId, scrollTop);
  }
</script>

<div class="history-surface">
  <ConversationTimeline
    {items}
    conversationId={active?.ownedId ?? ''}
    renderWindowId={active?.ownedId ?? ''}
    timelineRevision={conversation?.timelineRevision ?? 0}
    activeTurnId={conversation?.activeTurnId ?? null}
    {composerHeight}
    assistantLabel={active?.agent ?? 'Assistant'}
    savedScrollTop={conversation?.scrollTop ?? 0}
    emptyText="No conversation history."
    onScroll={handleScroll}
  />
</div>

<style>
  .history-surface {
    flex: 1 1 auto;
    width: 100%;
    min-height: 0;
  }
</style>
