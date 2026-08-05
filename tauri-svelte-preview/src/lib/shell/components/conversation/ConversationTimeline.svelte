<script lang="ts">
  import { tick } from 'svelte';
  import type { AgentConfigValue } from '$lib/shell/conversation/conversationTypes.ts';
  import {
    conversationScrollShouldFollow,
    visibleConversationRange,
    type ConversationDisplayItem
  } from '$lib/shell/conversation/conversationTimeline.ts';
  import TimelineItem from './TimelineItem.svelte';

  interface Props {
    items: readonly ConversationDisplayItem[];
    assistantLabel?: string;
    savedScrollTop?: number;
    emptyText?: string;
    onScroll?(scrollTop: number): void;
    onApprovalDecision?(requestId: string, decision: string): void;
    onInputSubmit?(requestId: string, values: Record<string, AgentConfigValue>, cancelled?: boolean): void;
    onFileLink?(path: string): void;
  }

  let {
    items,
    assistantLabel = 'Assistant',
    savedScrollTop = 0,
    emptyText = 'Start the conversation below.',
    onScroll,
    onApprovalDecision,
    onInputSubmit,
    onFileLink
  }: Props = $props();

  let host = $state<HTMLDivElement | null>(null);
  let viewportHeight = $state(480);
  let currentScrollTop = $state(0);
  let follow = $state(true);
  let lastItemId = '';
  const rowEstimate = 96;
  const range = $derived(visibleConversationRange(items.length, currentScrollTop, viewportHeight, rowEstimate));
  const visibleItems = $derived(items.slice(range.start, range.end));

  function measure(): void {
    if (host) viewportHeight = Math.max(1, host.clientHeight);
  }

  function handleScroll(): void {
    if (!host) return;
    currentScrollTop = host.scrollTop;
    follow = conversationScrollShouldFollow(host.scrollTop, host.clientHeight, host.scrollHeight);
    onScroll?.(host.scrollTop);
  }

  function jumpToLatest(): void {
    if (!host) return;
    host.scrollTop = host.scrollHeight;
    currentScrollTop = host.scrollTop;
    follow = true;
  }

  $effect(() => {
    const latest = items[items.length - 1];
    const latestId = latest?.itemId ?? '';
    if (!latestId || latestId === lastItemId) return;
    lastItemId = latestId;
    if (!follow) return;
    void tick().then(() => jumpToLatest());
  });

  $effect(() => {
    if (host && savedScrollTop > 0 && host.scrollTop === 0) {
      host.scrollTop = savedScrollTop;
      currentScrollTop = savedScrollTop;
    }
  });
</script>

<div class="timeline-wrap" data-testid="conversation-timeline-wrap">
  <div class="timeline-scroll" data-testid="conversation-timeline-scroll" bind:this={host} onscroll={handleScroll} onresize={measure}>
    {#if items.length === 0}<p class="empty" data-testid="conversation-timeline-empty">{emptyText}</p>{/if}
    {#if items.length > 0}<div class="virtual-spacer" style={`height:${range.offsetTop}px`} aria-hidden="true"></div>{/if}
    <div class="timeline-list" data-testid="conversation-timeline-list">
      {#each visibleItems as item (item.itemId)}
        <TimelineItem {item} {assistantLabel} onApprovalDecision={onApprovalDecision} onInputSubmit={onInputSubmit} {onFileLink} />
      {/each}
    </div>
    {#if items.length > 0}<div class="virtual-spacer" style={`height:${Math.max(0, (items.length - range.end) * rowEstimate)}px`} aria-hidden="true"></div>{/if}
  </div>
  {#if !follow && items.length > 0}<button class="jump-latest" data-testid="conversation-jump-latest" type="button" onclick={jumpToLatest}>Jump to latest</button>{/if}
</div>

<style>.timeline-wrap{position:relative;flex:1;min-height:0}.timeline-scroll{height:100%;overflow:auto;padding:34px max(28px,calc((100% - 820px)/2)) 220px;scrollbar-gutter:stable}.timeline-list{display:flex;flex-direction:column;gap:24px;min-height:1px}.virtual-spacer{flex:none}.empty{display:grid;place-items:center;min-height:100%;margin:0;color:var(--color-text-2)}.jump-latest{position:absolute;right:22px;bottom:192px;border:1px solid var(--color-border);border-radius:999px;background:var(--color-surface);color:inherit;padding:7px 11px;box-shadow:0 5px 18px rgba(0,0,0,.22)}</style>
