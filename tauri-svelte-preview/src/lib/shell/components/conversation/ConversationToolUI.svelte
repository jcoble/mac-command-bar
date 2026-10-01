<script lang="ts">
  import { getContext, setContext, type Snippet } from 'svelte';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  import { createChatUI, UIMessage, UIProvider } from '@tanstack/ai-svelte/ui';
  import {
    conversationMessagesContext, conversationToolContext,
    type ConversationMessagesContext, type ConversationToolContext
  } from '$lib/shell/conversation/conversationChatUI.ts';
  import ConversationMessageParts from './ConversationMessageParts.svelte';
  import ConversationToolPart from './ConversationToolPart.svelte';

  let { itemId, onFileLink, children }: {
    itemId: string;
    onFileLink?(path: string): void;
    children?: Snippet<[Extract<ConversationDisplayItem, { kind: 'tool' }>]>;
  } = $props();
  const host = getContext<ConversationMessagesContext>(conversationMessagesContext);
  const message = $derived(host.messages.get(itemId));
  const name = $derived(message?.parts.find((part) => part.type === 'tool-call')?.name);
  // UIProvider captures registered names on mount. Each tool registers its actual
  // name, including tools first encountered after the conversation was opened.
  const ui = $derived(createChatUI({}, {
    components: { layout: ConversationMessageParts, message: ConversationMessageParts },
    partsComponents: {},
    toolsComponents: name ? { [name]: ConversationToolPart } : {}
  }));
  setContext<ConversationToolContext>(conversationToolContext, {
    get message() { return message; },
    get renderTool() { return children; },
    openFile(path) { onFileLink?.(path); }
  });
  const chat = { get messages() { return message ? [message] : []; }, interrupts: [] };
</script>

{#if message && name}
  {#key name}
    <UIProvider {ui} {chat}>
      <UIMessage {ui} {message} />
    </UIProvider>
  {/key}
{/if}
