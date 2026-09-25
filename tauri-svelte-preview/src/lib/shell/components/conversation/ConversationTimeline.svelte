<script lang="ts">
  import ArrowDown from '@lucide/svelte/icons/arrow-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import type { AgentConfigValue } from '$lib/shell/conversation/conversationTypes.ts';
  import {
    conversationTurnGroups,
    foldFileEdits,
    foldToolRuns,
    formatWorkedFor,
    type ConversationTurnGroup,
    type ConversationDisplayItem,
    type ConversationFileLinkProvenance
  } from '$lib/shell/conversation/conversationTimeline.ts';
  import {
    decideConversationScroll,
    initialConversationScrollAnchorState,
    nextWritingFollowScrollTop,
    USER_SEND_ANCHOR_OFFSET_PX,
    type ConversationScrollAction,
    type ConversationScrollAnchorState,
    type ConversationScrollMotion,
    type ConversationSendAnchorRequest
  } from '$lib/shell/conversation/conversationScrollAnchor.ts';
  import { conversationItemHasVisibleContent } from '$lib/shell/conversation/conversationItemVisibility.ts';
  import { setConversationTimelineDiagnostics } from '$lib/shell/resourceDiagnostics.svelte';
  import TimelineItem from './TimelineItem.svelte';
  import TurnFileCard from './TurnFileCard.svelte';
  import PendingFirstMessage from './PendingFirstMessage.svelte';
  import WorkingSpinner from './WorkingSpinner.svelte';

  interface Props {
    items: readonly ConversationDisplayItem[];
    conversationId: string;
    showing: boolean;
    renderWindowId?: string;
    timelineRevision: number;
    anchorRequest?: ConversationSendAnchorRequest | null;
    activeTurnId?: string | null;
    localTurnActive?: boolean;
    composerHeight?: number;
    assistantLabel?: string;
    emptyText?: string;
    pendingFirstMessage?: string | null;
    /** Older history exists behind the first row on screen. */
    hasOlder?: boolean;
    loadingOlder?: boolean;
    onLoadOlder?(): void;
    /** Newer history exists beyond a window trimmed while reading upward. */
    hasNewer?: boolean;
    loadingNewer?: boolean;
    onLoadNewer?(): void;
    onJumpToLatest?(): void | Promise<void>;
    onScroll?(scrollTop: number): void;
    onApprovalDecision?(requestId: string, decision: string): void;
    onInputSubmit?(requestId: string, values: Record<string, AgentConfigValue>, cancelled?: boolean): void;
    onFileLink?(path: string, provenance?: ConversationFileLinkProvenance): void;
    /** Opens the plan chip above the composer, for the transcript's plan line. */
    onPlanOpen?(): void;
  }

  let {
    items,
    conversationId,
    showing,
    renderWindowId = conversationId,
    timelineRevision,
    anchorRequest = null,
    activeTurnId = null,
    localTurnActive = false,
    composerHeight = 0,
    assistantLabel = 'Assistant',
    emptyText = 'Start the conversation below.',
    pendingFirstMessage = null,
    hasOlder = false,
    loadingOlder = false,
    onLoadOlder,
    hasNewer = false,
    loadingNewer = false,
    onLoadNewer,
    onJumpToLatest,
    onScroll,
    onApprovalDecision,
    onInputSubmit,
    onFileLink,
    onPlanOpen
  }: Props = $props();

  let host = $state<HTMLDivElement | null>(null);
  let list = $state<HTMLDivElement | null>(null);
  let follow = $state(true);
  let scrollState = $state<ConversationScrollAnchorState>(initialConversationScrollAnchorState);
  let seenAnchorRequest = '';
  let anchoredUserItemId = $state<string | null>(null);
  let lastContentRevision = -1;
  let lastComposerHeight = -1;
  let lastItemCount = -1;
  let turnWasActive = false;
  let userItemIds = $state<string[]>([]);
  let openedConversationId = $state('');
  let wasShowing = false;
  let expandedTurns = $state<Map<string, boolean>>(new Map());
  /** Every item the conversation holds. A stored row is drawn, never offered. */
  const renderedItems = $derived(items.filter(conversationItemHasVisibleContent));
  const effectiveActiveTurnId = $derived(activeTurnId ?? (localTurnActive
    ? renderedItems.findLast((item) => item.turnId)?.turnId ?? null
    : null));
  const renderedGroups = $derived(conversationTurnGroups(renderedItems, effectiveActiveTurnId));

  function countDiffLines(diffText: string): { added: number; removed: number } {
    let added = 0;
    let removed = 0;
    let lineStart = 0;
    const len = diffText.length;
    for (let i = 0; i <= len; i++) {
      if (i === len || diffText.charCodeAt(i) === 10) {
        if (i > lineStart) {
          const first = diffText.charCodeAt(lineStart);
          const second = i > lineStart + 1 ? diffText.charCodeAt(lineStart + 1) : 0;
          if (first === 43 && second !== 43) added++;
          else if (first === 45 && second !== 45) removed++;
        }
        lineStart = i + 1;
      }
    }
    return { added, removed };
  }

  function getTurnFileEdits(group: ConversationTurnGroup): { path: string; added: number; removed: number }[] {
    const edits: { path: string; added: number; removed: number }[] = [];
    const seenPaths = new Set<string>();

    for (const item of group.items) {
      if (item.kind === 'fileEdits') {
        for (const edit of item.edits) {
          if (!seenPaths.has(edit.path)) {
            seenPaths.add(edit.path);
            edits.push({ path: edit.path, added: edit.added, removed: edit.removed });
          }
        }
      } else if (item.kind === 'file') {
        const p = typeof item.metadata?.path === 'string' ? item.metadata.path : '';
        if (p && !seenPaths.has(p)) {
          seenPaths.add(p);
          const d = typeof item.metadata?.diff === 'string' ? item.metadata.diff : item.text;
          const { added, removed } = countDiffLines(d);
          edits.push({ path: p, added, removed });
        }
      } else if (item.kind === 'toolRun') {
        for (const sub of item.items) {
          if (sub.kind === 'fileEdits') {
            for (const edit of sub.edits) {
              if (!seenPaths.has(edit.path)) {
                seenPaths.add(edit.path);
                edits.push({ path: edit.path, added: edit.added, removed: edit.removed });
              }
            }
          } else if (sub.kind === 'file') {
            const p = typeof sub.metadata?.path === 'string' ? sub.metadata.path : '';
            if (p && !seenPaths.has(p)) {
              seenPaths.add(p);
              const d = typeof sub.metadata?.diff === 'string' ? sub.metadata.diff : sub.text;
              const { added, removed } = countDiffLines(d);
              edits.push({ path: p, added, removed });
            }
          }
        }
      }
    }
    return edits;
  }

  function rowKey(group: ConversationTurnGroup | undefined, index: number): string {
    const groupId = group?.turnId ?? group?.items[0]?.itemId ?? `row:${index}`;
    return `${renderWindowId}:${groupId}`;
  }


  let timelineMounted = false;
  function publishTimelineDiagnostics(): void {
    if (!timelineMounted) return;
    setConversationTimelineDiagnostics(
      renderedGroups.length,
      renderedGroups.length,
      0,
      0
    );
  }

  $effect(() => {
    timelineMounted = true;
    publishTimelineDiagnostics();
    return () => {
      timelineMounted = false;
      setConversationTimelineDiagnostics(0, 0, 0, 0);
    };
  });
  const anchoredUserIndex = $derived(anchoredUserItemId
    ? renderedItems.findIndex((item) => item.itemId === anchoredUserItemId)
    : -1);
  const showWorking = $derived(
    localTurnActive
      && anchoredUserIndex >= 0
      && renderedItems.slice(anchoredUserIndex + 1).every((item) => !conversationItemHasVisibleContent(item))
  );

  $effect(() => {
    if (!showing) {
      wasShowing = false;
      return;
    }
    if (wasShowing && openedConversationId === renderWindowId) return;
    wasShowing = true;
    openedConversationId = renderWindowId;
    pageAnchor = null;
    anchoredUserItemId = null;
    expandedTurns = new Map();
    // Every open starts at the newest message; the reader controls scrolling
    // after that, including while new writing arrives.
    follow = true;
    scrollState = decideConversationScroll(scrollState, { type: 'opened' }).state;
  });

  $effect(() => {
    // Offscreen rows can gain their real height after the first frames. Keep
    // the newest message in view as the list settles, until the reader scrolls.
    if (!showing || renderedItems.length === 0 || !scrollState.openingToLatest || !host || !list) return;
    const viewport = host;
    const scrollLatest = () => {
      if (showing && scrollState.openingToLatest && viewport.clientHeight > 0) {
        viewport.scrollTop = viewport.scrollHeight - viewport.clientHeight;
      }
    };
    const observer = new ResizeObserver(scrollLatest);
    observer.observe(viewport);
    observer.observe(list);
    scrollLatest();
    return () => observer.disconnect();
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
    scrollState = decideConversationScroll(scrollState, { type: 'animation-finished' }).state;
  }

  function animateTo(top: number, motion: ConversationScrollMotion, settleItemId?: string, settleOffsetPx?: number): void {
    if (!host) return;
    const target = Math.max(0, top);
    void motion;
    host.scrollTop = target;
    if (settleItemId) {
      const settledTop = itemTop(settleItemId, settleOffsetPx ?? 0);
      if (settledTop !== null) host.scrollTop = settledTop;
    }
    finishAnimation();
  }

  function itemTop(itemId: string, offsetPx: number): number | null {
    if (!host) return null;
    const item = [...host.querySelectorAll<HTMLElement>('[data-item-id]')]
      .find((candidate) => candidate.dataset.itemId === itemId);
    if (!item) return null;
    // Measured from where the transcript BEGINS, not from the box's outer edge.
    // The centre pane's controls are laid over the top of this box, and the top
    // inset below is what keeps the reading clear of them; measuring from the
    // outer edge parked a just-sent message underneath those controls, which
    // reads as a message that scrolled away.
    const hostTop = host.getBoundingClientRect().top + host.clientTop
      + Number.parseFloat(getComputedStyle(host).paddingTop);
    const itemTop = item.getBoundingClientRect().top;
    return host.scrollTop + itemTop - hostTop - offsetPx;
  }

  /** Where to stop when following the newest writing. The empty space under the
   * newest user message is not writing, so it is left out of the sum: following
   * the reply means stopping where the reply stops, not sailing on into blank
   * screen. What is left below the last line is the scroll box's bottom padding,
   * which is exactly the height of the prompt box, so the last line comes to rest
   * just above the prompt instead of hiding behind it. */
  function latestWritingScrollTop(): number {
    if (!host) return 0;
    const item = [...host.querySelectorAll<HTMLElement>('[data-item-id]')].at(-1);
    if (!item) return 0;
    const hostTop = host.getBoundingClientRect().top + host.clientTop;
    const writingBottom = host.scrollTop + item.getBoundingClientRect().bottom - hostTop;
    const composerClearance = Math.max(composerHeight, 120) + 60;
    return Math.max(0, writingBottom - host.clientHeight + composerClearance);
  }

  /** How far the reader is above the end of the writing. Zero means they are
   * level with the prompt box and reading the newest line. */
  function distanceBelowReader(): number {
    return host ? latestWritingScrollTop() - host.scrollTop : 0;
  }

  function anchorUser(itemId: string, motion: ConversationScrollMotion, offsetPx: number): void {
    const top = itemTop(itemId, offsetPx);
    if (top !== null) {
      animateTo(top, motion, itemId, offsetPx);
      return;
    }
    finishAnimation();
  }

  function perform(action: ConversationScrollAction): void {
    if (!host || action.type === 'none') return;
    if (action.type === 'cancel-programmatic-scroll') return finishAnimation();
    if (action.type === 'scroll-to-latest') {
      animateTo(latestWritingScrollTop(), action.motion);
      return;
    }
    anchoredUserItemId = action.itemId;
    anchorUser(action.itemId, action.motion, action.offsetPx);
  }

  /*
   * Reading older history moves everything already on screen down. Remember the
   * first existing item and restore its viewport offset after replay; anchoring
   * an item rather than total height also survives the newest rows being trimmed.
   */
  type PageAnchor = { viewportTop: number; itemId: string; timelineRevision: number; conversationId: string };

  let pageAnchor: PageAnchor | null = null;

  function captureViewportAnchor(): void {
    if (!host) return;
    const hostTop = host.getBoundingClientRect().top;
    const candidates = [...host.querySelectorAll<HTMLElement>('[data-item-id]')];
    const item = candidates.find((candidate) => candidate.getBoundingClientRect().bottom > hostTop)
      ?? candidates[0];
    pageAnchor = item
      ? {
          viewportTop: item.getBoundingClientRect().top,
          itemId: item.dataset.itemId ?? '',
          timelineRevision,
          conversationId
        }
      : null;
  }

  function requestOlderHistory(): void {
    if (!host || !hasOlder || loadingOlder || !onLoadOlder || scrollState.openingToLatest) return;
    if (host.scrollTop > 80) return;
    captureViewportAnchor();
    onLoadOlder();
  }

  function requestNewerHistory(force = false): void {
    if (!host || !hasNewer || loadingNewer || !onLoadNewer || scrollState.openingToLatest) return;
    if (!force && distanceBelowReader() > 80) return;
    captureViewportAnchor();
    onLoadNewer();
  }

  function restoreViewportAnchor(anchor: PageAnchor): void {
    if (!host) return;
    const item = host.querySelector<HTMLElement>(`[data-item-id="${CSS.escape(anchor.itemId)}"]`);
    if (item) host.scrollTop += item.getBoundingClientRect().top - anchor.viewportTop;
    if (follow) pageAnchor = null;
    else captureViewportAnchor();
  }

  $effect(() => {
    const revision = timelineRevision;
    const anchor = pageAnchor;
    if (!anchor) return;
    if (anchor.conversationId !== conversationId) {
      pageAnchor = null;
      return;
    }
    if (revision <= anchor.timelineRevision) return;
    pageAnchor = null;
    restoreViewportAnchor(anchor);
  });

  let paging = false;
  $effect(() => {
    const now = loadingOlder || loadingNewer;
    if (paging && !now) pageAnchor = null;
    paging = now;
  });

  function handleScroll(): void {
    if (!host) return;
    const maxScroll = Math.max(0, host.scrollHeight - host.clientHeight);
    if (host.scrollTop > maxScroll) {
      host.scrollTop = maxScroll;
    }
    follow = anchoredUserItemId === null && distanceBelowReader() <= 80;
    // Keep the anchor fresh while reading so a live append that trims the top
    // can restore the same visible item instead of moving the reader.
    if (follow) pageAnchor = null;
    else captureViewportAnchor();
    onScroll?.(host.scrollTop);
    requestOlderHistory();
    requestNewerHistory();
  }

  let jumpingToLatest = false;
  async function jumpToLatest(): Promise<void> {
    if (jumpingToLatest) return;
    anchoredUserItemId = null;
    if (hasNewer && onJumpToLatest) {
      jumpingToLatest = true;
      // A direct tail reload replaces the paging operation. Do not let its
      // pending viewport correction pull the reader back from the newest row.
      pageAnchor = null;
      try {
        await onJumpToLatest();
      } finally {
        jumpingToLatest = false;
      }
    }
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
    // The message just sent goes to the top and stays there. Following the
    // writing as well meant the reply pushed that message off the top of the
    // screen the moment it ran longer than one, so the reader was returned to
    // the bottom of something they had not read the beginning of. Jump to
    // latest is how following starts again.
    follow = false;
  });

  $effect(() => {
    const decision = decideConversationScroll(scrollState, {
      type: 'user-items-changed',
      userItemIds
    });
    scrollState = decision.state;
    if (decision.action.type !== 'none') perform(decision.action);
  });

  $effect(() => {
    if (timelineRevision === lastContentRevision) return;
    lastContentRevision = timelineRevision;
    if (anchoredUserItemId && host) {
      const top = itemTop(anchoredUserItemId, USER_SEND_ANCHOR_OFFSET_PX);
      if (top !== null) host.scrollTop = top;
      return;
    }
    const decision = decideConversationScroll(scrollState, { type: 'stream-growth' });
    scrollState = decision.state;
    perform(decision.action);
    if (!host) return;
    if (follow && decision.action.type === 'none') {
      animateTo(nextWritingFollowScrollTop(host.scrollTop, latestWritingScrollTop()), 'instant');
    }
    // Whether the view follows is the reader's to decide — by scrolling to
    // the bottom, or by asking for the latest. It used to be recomputed here
    // as well, from how close the writing had grown to where they were
    // sitting, which turned a reply catching up with the reader into
    // permission to take the view from them.
  });

  $effect(() => {
    const turnIsActive = localTurnActive;
    const turnJustFinished = turnWasActive && !turnIsActive;
    turnWasActive = turnIsActive;
    if (!turnJustFinished || !anchoredUserItemId || !host) return;
    const top = itemTop(anchoredUserItemId, USER_SEND_ANCHOR_OFFSET_PX);
    if (top !== null) host.scrollTop = top;
  });

  $effect(() => {
    if (composerHeight === lastComposerHeight) return;
    lastComposerHeight = composerHeight;
    if (follow || scrollState.pinnedToBottom || scrollState.openingToLatest) {
      if (host) animateTo(latestWritingScrollTop(), 'instant');
    }
  });



  function handleUserInput(): void {
    anchoredUserItemId = null;
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
    node.addEventListener('pointerdown', handleUserInput, { passive: true });
    window.addEventListener('keydown', handleKeydown);
    return {
      destroy(): void {
        node.removeEventListener('wheel', handleUserInput);
        node.removeEventListener('touchstart', handleUserInput);
        node.removeEventListener('pointerdown', handleUserInput);
        window.removeEventListener('keydown', handleKeydown);
        finishAnimation();
      }
    };
  }
</script>

<div class="timeline-wrap" data-testid="conversation-timeline-wrap" style={`--composer-height:${composerHeight}px`}>
  {#if loadingOlder}
    <p class="older-loading" data-testid="conversation-older-loading" role="status">
      <span class="older-spinner" aria-hidden="true"></span>
      Loading earlier messages
    </p>
  {/if}
  <div
    class="timeline-scroll"
    data-testid="conversation-timeline-scroll"
    bind:this={host}
    onscroll={handleScroll}
    use:userInputInterrupts
  >
    {#if renderedItems.length === 0}
      {#if pendingFirstMessage}<PendingFirstMessage text={pendingFirstMessage} />
      {:else}<p class="empty" data-testid="conversation-timeline-empty">{emptyText}</p>{/if}
    {/if}
    <div
      class="timeline-list"
      data-testid="conversation-timeline-list"
      bind:this={list}
    >
      {#each renderedGroups as group, index (rowKey(group, index))}
        {@const expanded = turnExpanded(group)}
        {@const foldedItems = foldToolRuns(foldFileEdits(group.items))}
        {@const firstWorkItemId = foldedItems.find((item) => item.kind === 'toolRun' || item.kind === 'fileEdits' || group.workItemIds.includes(item.itemId))?.itemId}
        <div
          class="turn-row"
          data-index={index}
          data-turn-id={group.turnId}
          data-testid="conversation-timeline-row"
        >
          {#each foldedItems as item (item.itemId)}
            {@const workItem = item.kind === 'toolRun' || item.kind === 'fileEdits' || group.workItemIds.includes(item.itemId)}
            {#if group.completed && item.itemId === firstWorkItemId}
              <button class="turn-fold" data-testid="conversation-turn-fold" type="button" aria-expanded={expanded} onclick={() => toggleTurn(group)}>
                <span>{group.elapsedMs === null ? 'Worked' : `Worked for ${formatWorkedFor(group.elapsedMs)}`}</span>
                <span class="turn-fold-chevron" class:open={expanded} aria-hidden="true"><ChevronRight size={14} strokeWidth={1.8} /></span>
              </button>
            {/if}
            {#if !workItem || expanded}
              <TimelineItem {item} {assistantLabel} onApprovalDecision={onApprovalDecision} onInputSubmit={onInputSubmit} {onFileLink} {onPlanOpen} />
              {#if showWorking && item.itemId === anchoredUserItemId}
                <div class="working-row" data-testid="conversation-working-indicator" role="status">
                  <WorkingSpinner seed={group.turnId ?? item.itemId} />
                  <span>Working…</span>
                </div>
              {/if}
            {/if}
          {/each}
          {#if group.completed && expanded}
            {#each getTurnFileEdits(group) as edit (edit.path)}
              <TurnFileCard path={edit.path} added={edit.added} removed={edit.removed} onReview={onFileLink} />
            {/each}
          {/if}
        </div>
      {/each}
      <div class:send-anchor-space={anchoredUserItemId !== null} class="timeline-bottom-spacer" aria-hidden="true"></div>
    </div>
  </div>
  {#if !follow && renderedItems.length > 0}<button class="jump-latest" data-testid="conversation-jump-latest" type="button" aria-label="Jump to latest" onclick={() => void jumpToLatest()}><ArrowDown size={16} strokeWidth={2} aria-hidden="true" /></button>{/if}
</div>

<style>
  .timeline-wrap{position:relative;display:flex;flex-direction:column;width:100%;height:100%;flex:1 1 auto;min-height:0;overflow:hidden}
  .older-loading{position:absolute;top:calc(var(--center-head-height, 0px) + 6px);left:0;right:0;z-index:2;display:flex;align-items:center;justify-content:center;gap:6px;margin:0;font-size:12px;color:var(--color-text-2);pointer-events:none}
  .older-spinner{width:11px;height:11px;border:1.5px solid color-mix(in srgb,var(--color-text-3) 45%,transparent);border-top-color:var(--color-text-2);border-radius:50%;animation:older-spin 700ms linear infinite}
  @keyframes older-spin{to{transform:rotate(360deg)}}
  @media (prefers-reduced-motion: reduce){.older-spinner{animation:none;border-top-color:color-mix(in srgb,var(--color-text-3) 45%,transparent)}}
  .timeline-scroll{box-sizing:border-box;display:flex;flex-direction:column;width:100%;height:100%;flex:1 1 auto;min-height:0;overflow-y:auto;overflow-x:hidden;overflow-anchor:none;padding:var(--center-head-height, 0px) 28px 0;scrollbar-width:thin;scrollbar-color:var(--scrollbar-thumb) transparent;overscroll-behavior:contain}
  /* Keep the observed list at its content height. Shrinking it to the viewport
     hides delayed row-height changes from the opening ResizeObserver. */
  .timeline-list{position:relative;flex:none;display:flex;flex-direction:column;gap:18px;width:min(820px,100%);min-height:1px;margin:0 auto}
  .timeline-bottom-spacer{flex:none;height:calc(max(var(--composer-height, 0px), 120px) + 60px);pointer-events:none}
  .timeline-bottom-spacer.send-anchor-space{height:max(calc(max(var(--composer-height, 0px), 120px) + 60px),100vh)}
  .turn-row{position:relative;display:flex;flex-direction:column;gap:12px;width:100%;content-visibility:auto;contain-intrinsic-size:auto 120px}
  .turn-fold{display:flex;width:100%;align-items:center;gap:5px;min-height:28px;padding:0 0 7px;border:0;border-bottom:1px solid var(--color-border);background:transparent;color:var(--color-text-2);font:inherit;font-size:13px;text-align:left;cursor:pointer}
  .turn-fold:hover{color:var(--color-text)}
  .turn-fold:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:2px}
  .turn-fold-chevron{display:grid;place-items:center;color:var(--color-text-3)}
  .turn-fold-chevron.open{transform:rotate(90deg)}
  .empty{display:grid;flex:1;place-items:center;min-height:100%;margin:0;color:var(--color-text-2);font-size:13px}
  .working-row{display:flex;align-items:center;gap:8px;min-height:24px;color:var(--color-text-3);font-size:13px}
  /* A disc under the middle of the transcript, holding one arrow. It sits over
     the column it scrolls rather than off in the corner, and it says what it
     does by pointing, so it stays out of the reading it is offering to move. */
  .jump-latest{position:absolute;left:50%;bottom:calc(max(var(--composer-height, 0px), 120px) + 20px);display:grid;place-items:center;width:32px;height:32px;padding:0;transform:translateX(-50%);border:1px solid color-mix(in srgb,var(--color-border) 68%,transparent);border-radius:999px;background:color-mix(in srgb,var(--color-elevated) 94%,var(--color-accent) 6%);color:var(--color-text);box-shadow:var(--shadow-sm);cursor:pointer}
  .jump-latest:hover{background:var(--color-hover)}
  .jump-latest:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:2px}
  @media (prefers-reduced-motion:no-preference){.turn-fold-chevron{transition:transform .14s ease}.jump-latest{transition:background .14s ease,box-shadow .14s ease}}
</style>
