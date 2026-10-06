<script lang="ts">
  import { getContext, setContext, type Snippet } from 'svelte';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  import { UIMessage } from '@tanstack/ai-svelte/ui';
  import {
    conversationMessagesContext, conversationToolContext,
    type ConversationMessagesContext, type ConversationToolContext
  } from '$lib/shell/conversation/conversationChatUI.ts';

  let { itemId, onFileLink, children }: {
    itemId: string;
    onFileLink?(path: string): void;
    children?: Snippet<[Extract<ConversationDisplayItem, { kind: 'tool' }>]>;
  } = $props();
  const host = getContext<ConversationMessagesContext>(conversationMessagesContext);
  const message = $derived(host.messages.get(itemId));
  setContext<ConversationToolContext>(conversationToolContext, {
    get message() { return message; },
    get renderTool() { return children; },
    openFile(path) { onFileLink?.(path); }
  });
</script>

{#if message}
  <UIMessage ui={host.ui} {message} />
{/if}
