<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import type { ConversationDisplayItem, ConversationFileLinkProvenance } from '$lib/shell/conversation/conversationTimeline.ts';
  import TimelineItem from './TimelineItem.svelte';

  let { items, active, assistantLabel, onApprovalDecision, onFileLink, onPlanOpen }: {
    items: readonly ConversationDisplayItem[];
    active: boolean;
    assistantLabel: string;
    onApprovalDecision?(requestId: string, decision: string): void;
    onFileLink?(path: string, provenance?: ConversationFileLinkProvenance): void;
    onPlanOpen?(): void;
  } = $props();
  let host = $state<HTMLDivElement>();
  let initialized = false;
  const virtualizer = createVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: 0, getScrollElement: () => host ?? null, estimateSize: () => 36,
    overscan: 2, anchorTo: 'end', followOnAppend: false
  });
  const virtualRows = $derived($virtualizer.getVirtualItems());
  const totalSize = $derived($virtualizer.getTotalSize());
  $effect.pre(() => {
    const currentItems = items;
    const element = host;
    const follow = active;
    untrack(() => $virtualizer.setOptions({
      count: currentItems.length, getScrollElement: () => element ?? null,
      getItemKey: (index) => currentItems[index].itemId,
      followOnAppend: follow, scrollEndThreshold: follow ? 16 : -1
    }));
  });
  $effect(() => {
    if (!host || initialized) return;
    initialized = true;
    if (active) void tick().then(() => $virtualizer.scrollToEnd());
  });
  function measure(node: HTMLDivElement) {
    $virtualizer.measureElement(node);
    return { destroy() { $virtualizer.measureElement(null); } };
  }
</script>

<div class="tool-scroll" data-tool-scroll tabindex="0" role="region" aria-label="Tool calls"
  bind:this={host} style:height={`${Math.min(180, totalSize)}px`}
>
  <div class="tool-list" style:height={`${totalSize}px`}>
    {#each virtualRows as row (row.key)}
      <div class="tool-row" data-index={row.index} style:transform={`translateY(${row.start}px)`} use:measure>
        <TimelineItem item={items[row.index]} {assistantLabel} {onApprovalDecision} {onFileLink} {onPlanOpen} />
      </div>
    {/each}
  </div>
</div>

<style>
  .tool-scroll{overflow:auto;overflow-anchor:none;overscroll-behavior:contain;scrollbar-width:thin;scrollbar-color:var(--scrollbar-thumb) transparent;margin-left:12px}
  .tool-scroll:focus-visible{outline:1px solid var(--color-focus-solid);outline-offset:2px;border-radius:8px}
  .tool-list{position:relative;width:100%}
  .tool-row{position:absolute;top:0;left:0;width:100%;min-height:36px}
</style>
