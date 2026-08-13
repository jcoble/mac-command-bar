<script lang="ts">
  import { tick } from 'svelte';
  import type { AgentConfigValue } from '$lib/shell/conversation/conversationTypes.ts';
  import {
    type ConversationDisplayItem
  } from '$lib/shell/conversation/conversationTimeline.ts';
  import {
    decideConversationScroll,
    initialConversationScrollAnchorState,
    type ConversationScrollAction,
    type ConversationScrollAnchorState,
    type ConversationScrollMotion,
    type ConversationSendAnchorRequest
  } from '$lib/shell/conversation/conversationScrollAnchor.ts';
  import TimelineItem from './TimelineItem.svelte';

  interface Props {
    items: readonly ConversationDisplayItem[];
    conversationId: string;
    timelineRevision: number;
    anchorRequest?: ConversationSendAnchorRequest | null;
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
    conversationId,
    timelineRevision,
    anchorRequest = null,
    assistantLabel = 'Assistant',
    savedScrollTop = 0,
    emptyText = 'Start the conversation below.',
    onScroll,
    onApprovalDecision,
    onInputSubmit,
    onFileLink
  }: Props = $props();

  let host = $state<HTMLDivElement | null>(null);
  let follow = $state(true);
  let scrollState = $state<ConversationScrollAnchorState>(initialConversationScrollAnchorState);
  let animationFrame: number | null = null;
  let seenAnchorRequest = '';
  let lastContentRevision = -1;
  let lastItemCount = -1;
  let userItemIds = $state<string[]>([]);

  $effect(() => {
    const itemCount = items.length;
    if (itemCount === lastItemCount) return;
    lastItemCount = itemCount;
    userItemIds = items.filter((item) => item.kind === 'user').map((item) => item.itemId);
  });

  function prefersReducedMotion(): boolean {
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  }

  function finishAnimation(): void {
    animationFrame = null;
    scrollState = decideConversationScroll(scrollState, { type: 'animation-finished' }).state;
  }

  function cancelProgrammaticScroll(): void {
    if (animationFrame !== null) cancelAnimationFrame(animationFrame);
    animationFrame = null;
  }

  function animateTo(top: number, motion: ConversationScrollMotion, settleItemId?: string): void {
    if (!host) return;
    cancelProgrammaticScroll();
    const target = Math.max(0, top);
    if (motion === 'instant') {
      host.scrollTop = target;
      if (settleItemId) {
        const settledTop = itemTop(settleItemId);
        if (settledTop !== null) host.scrollTop = settledTop;
      }
      finishAnimation();
      return;
    }
    const start = host.scrollTop;
    const distance = target - start;
    if (Math.abs(distance) < 1) {
      host.scrollTop = target;
      finishAnimation();
      return;
    }
    const durationMs = 180;
    let startedAt: number | null = null;
    const step = (now: number): void => {
      if (!host) return finishAnimation();
      startedAt ??= now;
      const progress = Math.min(1, (now - startedAt) / durationMs);
      const eased = 1 - Math.pow(1 - progress, 3);
      host.scrollTop = start + distance * eased;
      if (progress < 1) animationFrame = requestAnimationFrame(step);
      else {
        if (settleItemId) {
          const settledTop = itemTop(settleItemId);
          if (settledTop !== null) host.scrollTop = settledTop;
        }
        finishAnimation();
      }
    };
    animationFrame = requestAnimationFrame(step);
  }

  function itemTop(itemId: string): number | null {
    if (!host) return null;
    const item = [...host.querySelectorAll<HTMLElement>('[data-item-id]')]
      .find((candidate) => candidate.dataset.itemId === itemId);
    if (!item) return null;
    const hostTop = host.getBoundingClientRect().top;
    const itemTop = item.getBoundingClientRect().top;
    const paddingTop = Number.parseFloat(getComputedStyle(host).paddingTop) || 0;
    return host.scrollTop + itemTop - hostTop - paddingTop;
  }

  function perform(action: ConversationScrollAction): void {
    if (!host || action.type === 'none') return;
    if (action.type === 'cancel-programmatic-scroll') return cancelProgrammaticScroll();
    if (action.type === 'scroll-to-latest') {
      animateTo(host.scrollHeight - host.clientHeight, action.motion);
      return;
    }
    const top = itemTop(action.itemId);
    if (top !== null) animateTo(top, action.motion, action.itemId);
  }

  function handleScroll(): void {
    if (!host) return;
    follow = host.scrollHeight - (host.scrollTop + host.clientHeight) <= 80;
    onScroll?.(host.scrollTop);
  }

  function jumpToLatest(): void {
    const decision = decideConversationScroll(scrollState, {
      type: 'jump-to-latest',
      reducedMotion: prefersReducedMotion()
    });
    scrollState = decision.state;
    follow = true;
    perform(decision.action);
  }

  $effect(() => {
    if (!anchorRequest || anchorRequest.conversationId !== conversationId) return;
    const key = `${anchorRequest.conversationId}:${anchorRequest.requestId}`;
    if (seenAnchorRequest === key) return;
    seenAnchorRequest = key;
    scrollState = decideConversationScroll(scrollState, {
      type: 'send',
      previousUserItemId: anchorRequest.previousUserItemId,
      reducedMotion: prefersReducedMotion()
    }).state;
  });

  $effect(() => {
    const decision = decideConversationScroll(scrollState, {
      type: 'user-items-changed',
      userItemIds
    });
    scrollState = decision.state;
    if (decision.action.type !== 'none') void tick().then(() => perform(decision.action));
  });

  $effect(() => {
    if (timelineRevision === lastContentRevision) return;
    lastContentRevision = timelineRevision;
    const decision = decideConversationScroll(scrollState, { type: 'stream-growth' });
    scrollState = decision.state;
    void tick().then(() => {
      perform(decision.action);
      if (host) follow = host.scrollHeight - (host.scrollTop + host.clientHeight) <= 80;
    });
  });

  $effect(() => {
    if (host && savedScrollTop > 0 && host.scrollTop === 0) {
      host.scrollTop = savedScrollTop;
    }
  });

  function handleUserInput(): void {
    const decision = decideConversationScroll(scrollState, { type: 'user-input' });
    scrollState = decision.state;
    perform(decision.action);
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (['ArrowUp', 'ArrowDown', 'PageUp', 'PageDown', 'Home', 'End', ' '].includes(event.key)) {
      handleUserInput();
    }
  }

  function userInputInterrupts(node: HTMLElement): { destroy(): void } {
    node.addEventListener('wheel', handleUserInput, { passive: true });
    node.addEventListener('touchstart', handleUserInput, { passive: true });
    window.addEventListener('keydown', handleKeydown);
    return {
      destroy(): void {
        node.removeEventListener('wheel', handleUserInput);
        node.removeEventListener('touchstart', handleUserInput);
        window.removeEventListener('keydown', handleKeydown);
        cancelProgrammaticScroll();
      }
    };
  }
</script>

<div class="timeline-wrap" data-testid="conversation-timeline-wrap">
  <div
    class="timeline-scroll"
    data-testid="conversation-timeline-scroll"
    bind:this={host}
    onscroll={handleScroll}
    use:userInputInterrupts
  >
    {#if items.length === 0}<p class="empty" data-testid="conversation-timeline-empty">{emptyText}</p>{/if}
    <div class="timeline-list" data-testid="conversation-timeline-list">
      {#each items as item (item.itemId)}
        <TimelineItem {item} {assistantLabel} onApprovalDecision={onApprovalDecision} onInputSubmit={onInputSubmit} {onFileLink} />
      {/each}
    </div>
  </div>
  {#if !follow && items.length > 0}<button class="jump-latest" data-testid="conversation-jump-latest" type="button" onclick={jumpToLatest}>Jump to latest</button>{/if}
</div>

<style>
  .timeline-wrap{position:relative;flex:1;min-height:0}
  .timeline-scroll{box-sizing:border-box;height:100%;overflow:auto;padding:30px 24px 206px;scrollbar-gutter:stable;overscroll-behavior:contain}
  .timeline-list{display:flex;flex-direction:column;gap:18px;width:min(820px,100%);min-height:1px;margin:0 auto}
  .empty{display:grid;place-items:center;min-height:100%;margin:0;color:var(--color-text-2);font-size:13px}
  .jump-latest{position:absolute;right:24px;bottom:188px;min-height:28px;padding:6px 12px;border:1px solid color-mix(in srgb,var(--color-border) 68%,transparent);border-radius:999px;background:color-mix(in srgb,var(--color-elevated) 94%,var(--color-accent) 6%);color:var(--color-text);font-size:13px;box-shadow:var(--shadow-sm);cursor:pointer}
  .jump-latest:hover{background:var(--color-hover)}
  .jump-latest:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:2px}
  @media (prefers-reduced-motion:no-preference){.jump-latest{transition:background .14s ease,box-shadow .14s ease}}
</style>
