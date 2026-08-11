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
  let lastItemRevision = '';
  const rowEstimate = 96;
  const range = $derived(visibleConversationRange(items.length, currentScrollTop, viewportHeight, rowEstimate));
  const visibleItems = $derived(items.slice(range.start, range.end));
  const latestRevision = $derived(items.length ? JSON.stringify(items[items.length - 1]) : '');

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
    if (!latestRevision || latestRevision === lastItemRevision) return;
    lastItemRevision = latestRevision;
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

<style>
  .timeline-wrap{position:relative;flex:1;min-height:0}
  .timeline-scroll{height:100%;overflow:auto;padding:34px max(28px,calc((100% - 820px)/2)) 220px;scrollbar-gutter:stable}
  /* 16px between items; a new turn adds 8px of its own (see TimelineItem). */
  .timeline-list{display:flex;flex-direction:column;gap:16px;min-height:1px}
  .virtual-spacer{flex:none}
  .empty{display:grid;place-items:center;min-height:100%;margin:0;color:var(--color-text-2);font-size:13px}
  .jump-latest{position:absolute;right:22px;bottom:192px;border:1px solid color-mix(in srgb,var(--color-border) 60%,transparent);border-radius:999px;background:color-mix(in srgb,var(--color-elevated) 92%,var(--color-accent) 8%);color:inherit;padding:7px 12px;font-size:13px;box-shadow:var(--shadow-sm)}
  .jump-latest:hover{background:var(--color-hover)}
  .jump-latest:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:2px}
  @media (prefers-reduced-motion:no-preference){
    .jump-latest{transition:background .14s ease,box-shadow .14s ease}
  }
</style>
