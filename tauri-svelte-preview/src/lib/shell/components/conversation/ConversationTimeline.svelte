<script lang="ts">
  import { tick } from 'svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import type { AgentConfigValue } from '$lib/shell/conversation/conversationTypes.ts';
  import {
    conversationRenderWindow,
    conversationTurnGroups,
    discloseEarlierConversationItems,
    formatWorkedFor,
    type ConversationTurnGroup,
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
  import { conversationItemHasVisibleContent } from '$lib/shell/conversation/conversationItemVisibility.ts';
  import TimelineItem from './TimelineItem.svelte';

  interface Props {
    items: readonly ConversationDisplayItem[];
    conversationId: string;
    renderWindowId?: string;
    timelineRevision: number;
    anchorRequest?: ConversationSendAnchorRequest | null;
    activeTurnId?: string | null;
    localTurnActive?: boolean;
    composerHeight?: number;
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
    renderWindowId = conversationId,
    timelineRevision,
    anchorRequest = null,
    activeTurnId = null,
    localTurnActive = false,
    composerHeight = 0,
    assistantLabel = 'Assistant',
    savedScrollTop = 0,
    emptyText = 'Start the conversation below.',
    onScroll,
    onApprovalDecision,
    onInputSubmit,
    onFileLink
  }: Props = $props();

  let host = $state<HTMLDivElement | null>(null);
  let tail = $state<HTMLDivElement | null>(null);
  let follow = $state(true);
  let scrollState = $state<ConversationScrollAnchorState>(initialConversationScrollAnchorState);
  let animationFrame: number | null = null;
  let seenAnchorRequest = '';
  let anchoredUserItemId = $state<string | null>(null);
  let viewportHeight = $state(0);
  let lastContentRevision = -1;
  let lastComposerHeight = -1;
  let lastItemCount = -1;
  let userItemIds = $state<string[]>([]);
  let windowConversationId = $state('');
  let disclosedItems = $state(0);
  let disclosureAnchorItemId = $state<string | null>(null);
  let foldConversationId = $state('');
  let expandedTurns = $state<Map<string, boolean>>(new Map());
  const renderWindow = $derived(conversationRenderWindow(items, renderWindowId, {
    conversationId: windowConversationId,
    disclosedItems,
    disclosureAnchorItemId
  }));
  const renderedItems = $derived(renderWindow.items);
  const effectiveActiveTurnId = $derived(activeTurnId ?? (localTurnActive
    ? renderedItems.findLast((item) => item.turnId)?.turnId ?? null
    : null));
  const renderedGroups = $derived(conversationTurnGroups(renderedItems, effectiveActiveTurnId));
  const anchoredUserIndex = $derived(anchoredUserItemId
    ? renderedItems.findIndex((item) => item.itemId === anchoredUserItemId)
    : -1);
  const showWorking = $derived(
    localTurnActive
      && anchoredUserIndex >= 0
      && renderedItems.slice(anchoredUserIndex + 1).every((item) => !conversationItemHasVisibleContent(item))
  );
  // Empty space under the newest user message, one screen tall, so that message can
  // sit at the top of the screen after a send. It stays there once the reply is
  // finished: taking it away would make the page shorter than the reader's current
  // position, and the browser would answer by yanking the view down to the new
  // bottom. The space is dropped only when another conversation is opened, which
  // also clears the anchored message.
  const showActiveTurnTail = $derived(anchoredUserIndex >= 0);

  $effect(() => {
    if (!host) return;
    const publish = (): void => {
      viewportHeight = host?.clientHeight ?? 0;
    };
    const observer = new ResizeObserver(publish);
    observer.observe(host);
    publish();
    return () => observer.disconnect();
  });

  $effect(() => {
    if (windowConversationId === renderWindowId) return;
    windowConversationId = renderWindowId;
    disclosedItems = 0;
    disclosureAnchorItemId = null;
    anchoredUserItemId = null;
    foldConversationId = renderWindowId;
    expandedTurns = new Map();
    // A session opens on its newest turn rather than at the beginning of the
    // transcript. This only records the intent; the scrolling waits until there
    // are messages on screen to scroll to.
    scrollState = decideConversationScroll(scrollState, { type: 'opened' }).state;
  });

  $effect(() => {
    // Land on the newest writing once the messages are actually on screen. Stored
    // messages arrive in batches, so this runs again on each batch and keeps the
    // view at the end until the reader scrolls, types or sends, any of which drops
    // the opening state and hands the view back to them. The move is immediate
    // rather than animated: nobody asked to watch a transcript they have not read
    // scroll past.
    if (renderedItems.length === 0 || !scrollState.openingToLatest) return;
    void tick().then(() => {
      if (!host || !scrollState.openingToLatest) return;
      host.scrollTop = latestWritingScrollTop();
      follow = true;
    });
  });

  $effect(() => {
    if (foldConversationId !== renderWindowId) return;
    let next: Map<string, boolean> | null = null;
    for (const group of renderedGroups) {
      if (!group.turnId || group.completed || group.workItemIds.length === 0 || expandedTurns.has(group.turnId)) continue;
      next ??= new Map(expandedTurns);
      next.set(group.turnId, true);
    }
    if (next) expandedTurns = next;
  });

  $effect(() => {
    const itemCount = renderedItems.length;
    if (itemCount === lastItemCount) return;
    lastItemCount = itemCount;
    userItemIds = renderedItems.filter((item) => item.kind === 'user').map((item) => item.itemId);
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

  function animateTo(top: number, motion: ConversationScrollMotion, settleItemId?: string, settleOffsetPx?: number): void {
    if (!host) return;
    cancelProgrammaticScroll();
    const target = Math.max(0, top);
    if (motion === 'instant') {
      host.scrollTop = target;
      if (settleItemId) {
        const settledTop = itemTop(settleItemId, settleOffsetPx ?? 0);
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
          const settledTop = itemTop(settleItemId, settleOffsetPx ?? 0);
          if (settledTop !== null) host.scrollTop = settledTop;
        }
        finishAnimation();
      }
    };
    animationFrame = requestAnimationFrame(step);
  }

  function itemTop(itemId: string, offsetPx: number): number | null {
    if (!host) return null;
    const item = [...host.querySelectorAll<HTMLElement>('[data-item-id]')]
      .find((candidate) => candidate.dataset.itemId === itemId);
    if (!item) return null;
    const hostTop = host.getBoundingClientRect().top;
    const itemTop = item.getBoundingClientRect().top;
    return host.scrollTop + itemTop - hostTop - offsetPx;
  }

  function itemViewportTop(itemId: string): number | null {
    if (!host) return null;
    const item = [...host.querySelectorAll<HTMLElement>('[data-item-id]')]
      .find((candidate) => candidate.dataset.itemId === itemId);
    return item?.getBoundingClientRect().top ?? null;
  }

  async function showEarlier(): Promise<void> {
    if (!host || renderWindow.hiddenCount === 0 || renderedItems.length === 0) return;
    const firstItemId = renderedItems[0].itemId;
    const previousViewportTop = itemViewportTop(firstItemId);
    const next = discloseEarlierConversationItems(items, renderWindowId, renderWindow.state);
    windowConversationId = next.state.conversationId;
    disclosedItems = next.state.disclosedItems;
    disclosureAnchorItemId = next.state.disclosureAnchorItemId;
    await tick();
    const nextViewportTop = itemViewportTop(firstItemId);
    if (previousViewportTop !== null && nextViewportTop !== null) {
      host.scrollTop += nextViewportTop - previousViewportTop;
    }
  }

  /** Where to stop when following the newest writing. The empty space under the
   * newest user message is not writing, so it is left out of the sum: following
   * the reply means stopping where the reply stops, not sailing on into blank
   * screen. What is left below the last line is the scroll box's bottom padding,
   * which is exactly the height of the prompt box, so the last line comes to rest
   * just above the prompt instead of hiding behind it. */
  function latestWritingScrollTop(): number {
    if (!host) return 0;
    return host.scrollHeight - (tail?.offsetHeight ?? 0) - host.clientHeight;
  }

  /** How far the reader is above the end of the writing. Zero means they are
   * level with the prompt box and reading the newest line. */
  function distanceBelowReader(): number {
    return host ? latestWritingScrollTop() - host.scrollTop : 0;
  }

  function perform(action: ConversationScrollAction): void {
    if (!host || action.type === 'none') return;
    if (action.type === 'cancel-programmatic-scroll') return cancelProgrammaticScroll();
    if (action.type === 'scroll-to-latest') {
      animateTo(latestWritingScrollTop(), action.motion);
      return;
    }
    anchoredUserItemId = action.itemId;
    const top = itemTop(action.itemId, action.offsetPx);
    if (top !== null) animateTo(top, action.motion, action.itemId, action.offsetPx);
  }

  function handleScroll(): void {
    if (!host) return;
    follow = distanceBelowReader() <= 80;
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
      if (host) follow = distanceBelowReader() <= 80;
    });
  });

  $effect(() => {
    if (composerHeight === lastComposerHeight) return;
    lastComposerHeight = composerHeight;
    if (!scrollState.pinnedToBottom) return;
    void tick().then(() => {
      if (host) animateTo(latestWritingScrollTop(), 'instant');
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

  function turnExpanded(group: ConversationTurnGroup): boolean {
    if (!group.turnId || !group.completed || group.workItemIds.length === 0) return true;
    return expandedTurns.get(group.turnId) ?? false;
  }

  function toggleTurn(group: ConversationTurnGroup): void {
    if (!group.turnId) return;
    const next = new Map(expandedTurns);
    next.set(group.turnId, !turnExpanded(group));
    expandedTurns = next;
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

<div class="timeline-wrap" data-testid="conversation-timeline-wrap" style={`--composer-height:${composerHeight}px`}>
  <div
    class="timeline-scroll"
    data-testid="conversation-timeline-scroll"
    bind:this={host}
    onscroll={handleScroll}
    use:userInputInterrupts
  >
    {#if renderedItems.length === 0}<p class="empty" data-testid="conversation-timeline-empty">{emptyText}</p>{/if}
    <div class="timeline-list" data-testid="conversation-timeline-list">
      {#if renderWindow.hiddenCount > 0}
        <div class="earlier-row" data-testid="conversation-show-earlier-row">
          <Button
            variant="ghost"
            size="sm"
            data-testid="conversation-show-earlier"
            onclick={() => void showEarlier()}
          >Show earlier — {renderWindow.hiddenCount.toLocaleString()} more</Button>
        </div>
      {/if}
      {#each renderedGroups as group (group.turnId ?? group.items[0]?.itemId)}
        {@const expanded = turnExpanded(group)}
        {@const firstWorkItemId = group.workItemIds[0]}
        {#each group.items as item (item.itemId)}
          {@const workItem = group.workItemIds.includes(item.itemId)}
          {#if group.turnId !== null && item.itemId === firstWorkItemId && group.completed}
            <button
              class="turn-fold"
              data-testid="conversation-turn-fold"
              type="button"
              aria-expanded={expanded}
              onclick={() => toggleTurn(group)}
            >
              <span>{group.elapsedMs === null ? 'Worked' : `Worked for ${formatWorkedFor(group.elapsedMs)}`}</span>
              <span class="turn-fold-chevron" aria-hidden="true">{expanded ? '⌄' : '>'}</span>
            </button>
          {/if}
          {#if !workItem || expanded}
            <TimelineItem {item} {assistantLabel} onApprovalDecision={onApprovalDecision} onInputSubmit={onInputSubmit} {onFileLink} />
            {#if showWorking && item.itemId === anchoredUserItemId}
              <div class="working-row" data-testid="conversation-working-indicator" role="status">
                <span class="working-dot" aria-hidden="true"></span>
                <span>Working…</span>
              </div>
            {/if}
          {/if}
        {/each}
      {/each}
      {#if showActiveTurnTail}<div class="active-turn-tail" bind:this={tail} style={`height:${viewportHeight}px`} aria-hidden="true"></div>{/if}
    </div>
  </div>
  {#if !follow && renderedItems.length > 0}<button class="jump-latest" data-testid="conversation-jump-latest" type="button" onclick={jumpToLatest}>Jump to latest</button>{/if}
</div>

<style>
  .timeline-wrap{position:relative;flex:1;min-height:0}
  .timeline-scroll{box-sizing:border-box;height:100%;overflow:auto;padding:30px 24px calc(var(--composer-height) + 16px);scrollbar-gutter:stable;overscroll-behavior:contain}
  .timeline-list{display:flex;flex-direction:column;gap:16px;width:min(820px,100%);min-height:1px;margin:0 auto}
  .earlier-row{display:flex;justify-content:center;min-height:28px}
  .turn-fold{display:flex;width:100%;align-items:center;gap:5px;min-height:28px;padding:0 0 7px;border:0;border-bottom:1px solid var(--color-border);background:transparent;color:var(--color-text-2);font:inherit;font-size:13px;text-align:left;cursor:pointer}
  .turn-fold:hover{color:var(--color-text)}
  .turn-fold:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:2px}
  .turn-fold-chevron{display:inline-block;line-height:1;transform:translateY(-1px);transition:transform .14s ease}
  .empty{display:grid;place-items:center;min-height:100%;margin:0;color:var(--color-text-2);font-size:13px}
  .working-row{display:flex;align-items:center;gap:8px;min-height:20px;color:var(--color-text-3);font-size:13px}
  .working-dot{width:6px;height:6px;border-radius:999px;background:currentColor}
  .active-turn-tail{flex:none;margin-top:-16px;pointer-events:none}
  .jump-latest{position:absolute;right:24px;bottom:calc(var(--composer-height) + 16px);min-height:28px;padding:6px 12px;border:1px solid color-mix(in srgb,var(--color-border) 68%,transparent);border-radius:999px;background:color-mix(in srgb,var(--color-elevated) 94%,var(--color-accent) 6%);color:var(--color-text);font-size:13px;box-shadow:var(--shadow-sm);cursor:pointer}
  .jump-latest:hover{background:var(--color-hover)}
  .jump-latest:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:2px}
  @keyframes working-pulse{0%,100%{opacity:.38;transform:scale(.82)}50%{opacity:1;transform:scale(1)}}
  @media (prefers-reduced-motion:no-preference){.jump-latest{transition:background .14s ease,box-shadow .14s ease}.working-dot{animation:working-pulse 1.1s ease-in-out infinite}}
</style>
