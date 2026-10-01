<script lang="ts">
  import { getContext } from 'svelte';
  import type { ToolProps } from '@tanstack/ai-svelte/ui';
  import { conversationToolContext, type ConversationToolContext } from '$lib/shell/conversation/conversationChatUI.ts';
  import { conversationMessageDisplayItem } from '$lib/shell/conversation/conversationMessages.ts';
  import ToolItem from './ToolItem.svelte';

  let { part, result }: ToolProps<unknown> = $props();
  const context = getContext<ConversationToolContext>(conversationToolContext);
  // UIMessage supplies the live call and its terminal paired result.
  const item = $derived(context.message ? conversationMessageDisplayItem({
    ...context.message,
    parts: result ? [part, result] : [part]
  }) : undefined);
</script>

{#if item?.kind === 'tool'}
  {@const tool = { ...item, state: part.state === 'error' ? 'failed' as const : part.state === 'complete' ? 'completed' as const : part.state === 'awaiting-input' ? 'pending' as const : 'running' as const }}
  {#if context.renderTool}
    {@render context.renderTool(tool)}
  {:else}
    <ToolItem item={tool} onFileLink={context.openFile} />
  {/if}
{/if}
