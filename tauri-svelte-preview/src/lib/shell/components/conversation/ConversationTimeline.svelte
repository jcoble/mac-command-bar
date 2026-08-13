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
  .timeline-scroll{height:100%;overflow:auto;padding:30px 24px 206px;scrollbar-gutter:stable}
  .timeline-list{display:flex;flex-direction:column;gap:18px;width:min(820px,100%);min-height:1px;margin:0 auto}
  .virtual-spacer{flex:none;width:min(820px,100%);margin:0 auto}
  .empty{display:grid;place-items:center;min-height:100%;margin:0;color:var(--color-text-2);font-size:13px}
  .jump-latest{position:absolute;right:24px;bottom:188px;min-height:28px;padding:6px 12px;border:1px solid color-mix(in srgb,var(--color-border) 68%,transparent);border-radius:999px;background:color-mix(in srgb,var(--color-elevated) 94%,var(--color-accent) 6%);color:var(--color-text);font-size:13px;box-shadow:var(--shadow-sm);cursor:pointer}
  .jump-latest:hover{background:var(--color-hover)}
  .jump-latest:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:2px}
  @media (prefers-reduced-motion:no-preference){.jump-latest{transition:background .14s ease,box-shadow .14s ease}}
</style>
