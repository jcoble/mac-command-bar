<script lang="ts">
  import { getContext } from 'svelte';
  import type { ToolProps } from '@tanstack/ai-svelte/ui';
  import { conversationToolContext, type ConversationToolContext } from '$lib/shell/conversation/conversationChatUI.ts';
  import ToolItem from './ToolItem.svelte';

  let { part }: ToolProps<unknown> = $props();
  const context = getContext<ConversationToolContext>(conversationToolContext);
  // The timeline already built this call's row; reusing it skips parsing the stored output on every mount.
  const item = $derived(context.item);
</script>

{#if item}
  {@const tool = { ...item, state: part.state === 'error' ? 'failed' as const : part.state === 'complete' ? 'completed' as const : part.state === 'awaiting-input' ? 'pending' as const : 'running' as const }}
  {#if context.renderTool}
    {@render context.renderTool(tool)}
  {:else}
    <ToolItem item={tool} onFileLink={context.openFile} />
  {/if}
{/if}
