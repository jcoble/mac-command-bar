<script lang="ts">
  import { tick } from 'svelte';
  import ArrowDown from '@lucide/svelte/icons/arrow-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import type { AgentConfigValue } from '$lib/shell/conversation/conversationTypes.ts';
  import {
    conversationTurnGroups,
    formatWorkedFor,
    type ConversationTurnGroup,
    type ConversationDisplayItem,
    type ConversationFileLinkProvenance
  } from '$lib/shell/conversation/conversationTimeline.ts';
  import {
    decideConversationScroll,
    initialConversationScrollAnchorState,
    nextWritingFollowScrollTop,
    type ConversationScrollAction,
    type ConversationScrollAnchorState,
    type ConversationScrollMotion,
    type ConversationSendAnchorRequest
  } from '$lib/shell/conversation/conversationScrollAnchor.ts';
  import { conversationItemHasVisibleContent } from '$lib/shell/conversation/conversationItemVisibility.ts';
  import {
    cancelTrackedAnimationFrame,
    requestTrackedAnimationFrame,
    setConversationTimelineDiagnostics
  } from '$lib/shell/resourceDiagnostics.svelte';
  import TimelineItem from './TimelineItem.svelte';
  import WorkingSpinner from './WorkingSpinner.svelte';

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
    renderWindowId = conversationId,
    timelineRevision,
    anchorRequest = null,
    activeTurnId = null,
    localTurnActive = false,
    composerHeight = 0,
    assistantLabel = 'Assistant',
    savedScrollTop = 0,
    emptyText = 'Start the conversation below.',
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

  /** Where an already-read conversation is waiting to be put back to. */
  let restoringScrollTop = $state<number | null>(null);
  let host = $state<HTMLDivElement | null>(null);
  let list = $state<HTMLDivElement | null>(null);
  let tail = $state<HTMLDivElement | null>(null);
  let follow = $state(true);
  let scrollState = $state<ConversationScrollAnchorState>(initialConversationScrollAnchorState);
  let animationFrame: number | null = null;
  let hydrationFrame: number | null = null;
  let seenAnchorRequest = '';
  let anchoredUserItemId = $state<string | null>(null);
  let viewportHeight = $state(0);
  let lastContentRevision = -1;
  let lastComposerHeight = -1;
  let lastItemCount = -1;
  let userItemIds = $state<string[]>([]);
  let openedConversationId = $state('');
  let foldConversationId = $state('');
  let expandedTurns = $state<Map<string, boolean>>(new Map());
  /** Every item the conversation holds. A stored row is drawn, never offered. */
  const renderedItems = $derived(items);
  const effectiveActiveTurnId = $derived(activeTurnId ?? (localTurnActive
    ? renderedItems.findLast((item) => item.turnId)?.turnId ?? null
    : null));
  const renderedGroups = $derived(conversationTurnGroups(renderedItems, effectiveActiveTurnId));

  /*
   * One turn is one virtual row. Turns are the unit the transcript already
   * groups by, so a row is a whole exchange rather than a fragment of one, and
   * a turn is never split across the boundary of what is mounted.
   *
   * Heights are measured, not declared: a turn holding one line and a turn
   * holding a diff are nothing alike, and an estimate that guessed would make
   * the scrollbar lie. The estimate below is only what an unmeasured row is
   * assumed to be until it has been on screen once.
   */
  /*
   * A row is estimated from how many characters it holds, not from a fixed
   * guess. Text length is the only thing known before a row has ever been on
   * screen that actually tracks its height: a row is as tall as its writing
   * wraps. A flat number is wrong in both directions at once — it inflates the
   * scroll height for the rows that draw nothing (a stored transcript is mostly
   * usage and config records, which render no text at all) and understates the
   * ones carrying a long reply or a tool dump.
   *
   * These are only estimates for rows not yet measured; `measureElement`
   * replaces each with its real height the first time it paints.
   */
  const ROW_CHARS_PER_LINE = 72;
  const ROW_LINE_HEIGHT = 20;
  const ROW_CHROME = 32;
  const ROW_MIN_HEIGHT = 44;

  function rowHeightEstimate(group: ConversationTurnGroup): number {
    let characters = 0;
    for (const item of group.items) {
      const record = item as { text?: string; title?: string; output?: string };
      characters += (record.text?.length ?? 0)
        + (record.title?.length ?? 0)
        + (record.output?.length ?? 0);
    }
    if (characters === 0) return ROW_MIN_HEIGHT;
    return Math.ceil(characters / ROW_CHARS_PER_LINE) * ROW_LINE_HEIGHT + ROW_CHROME;
  }

  function rowKey(group: ConversationTurnGroup | undefined, index: number): string {
    const groupId = group?.turnId ?? group?.items[0]?.itemId ?? `row:${index}`;
    return `${renderWindowId}:${groupId}`;
  }

  let rowEstimates: number[] = [];
  $effect(() => {
    rowEstimates = renderedGroups.map(rowHeightEstimate);
  });

  const virtualizer = createVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: 0,
    getScrollElement: () => host,
    getItemKey: (index) => rowKey(renderedGroups[index], index),
    estimateSize: (index) => rowEstimates[index] ?? ROW_MIN_HEIGHT,
    overscan: 6,
    useAnimationFrameWithResizeObserver: true
  });

  let timelineMounted = false;
  function publishTimelineDiagnostics(): void {
    if (!timelineMounted) return;
    setConversationTimelineDiagnostics(
      renderedGroups.length,
      $virtualizer.getVirtualItems().length,
      $virtualizer.elementsCache.size,
      $virtualizer.itemSizeCache.size
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

  let appliedRowCount = -1;
  let appliedHost: HTMLDivElement | null = null;
  let appliedRenderWindowId = '';
  $effect(() => {
    // Guarded because setOptions publishes the store, and this effect reads it:
    // without the guard the two would drive each other in a loop.
    const count = renderedGroups.length;
    const element = host;
    const windowId = renderWindowId;
    if (count === appliedRowCount && element === appliedHost && windowId === appliedRenderWindowId) return;
    const windowChanged = windowId !== appliedRenderWindowId;
    appliedRowCount = count;
    appliedHost = element;
    appliedRenderWindowId = windowId;
    $virtualizer.setOptions({
      count,
      getScrollElement: () => element,
      getItemKey: (index) => rowKey(renderedGroups[index], index),
      estimateSize: (index) => rowEstimates[index] ?? ROW_MIN_HEIGHT,
      overscan: 6,
      useAnimationFrameWithResizeObserver: true
    });
    if (windowChanged) {
      // Row keys include the session/child window. TanStack does not prune old
      // measured sizes merely because getItemKey starts returning new keys.
      $virtualizer.measure();
      void tick().then(() => {
        // Session replacement removes the old rows during this Svelte flush.
        // Sweep only nodes that are now detached; current rows stay observed.
        $virtualizer.measureElement(null);
        publishTimelineDiagnostics();
      });
    }
  });
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
    let frame: number | null = null;
    const publish = (): void => {
      frame = null;
      const nextHeight = host?.clientHeight ?? 0;
      if (viewportHeight !== nextHeight) viewportHeight = nextHeight;
    };
    const observer = new ResizeObserver(() => {
      if (frame === null) frame = requestTrackedAnimationFrame(publish);
    });
    observer.observe(host);
    publish();
    return () => {
      observer.disconnect();
      if (frame !== null) cancelTrackedAnimationFrame(frame);
    };
  });

  $effect(() => {
    if (openedConversationId === renderWindowId) return;
    openedConversationId = renderWindowId;
    pageAnchor = null;
    anchoredUserItemId = null;
    foldConversationId = renderWindowId;
    expandedTurns = new Map();
    // Coming back to a conversation returns to the place it was left. Only one
    // being opened for the first time lands on its newest turn — and that is
    // the only case with nowhere else to land. Both wait for messages to be on
    // screen before anything moves.
    if (savedScrollTop > 0) restoringScrollTop = savedScrollTop;
    else scrollState = decideConversationScroll(scrollState, { type: 'opened' }).state;
  });

  $effect(() => {
    // Put the view back where it was. Rows are measured as they draw, so the
    // page is still growing under this: it is set once the rows exist and again
    // on the next frame, by which point the heights above the reader are real.
    if (restoringScrollTop === null || renderedItems.length === 0) return;
    const target = restoringScrollTop;
    restoringScrollTop = null;
    void tick().then(() => {
      if (!host) return;
      host.scrollTop = target;
      requestTrackedAnimationFrame(() => {
        if (host) host.scrollTop = target;
      });
    });
  });

  $effect(() => {
    // Land on the newest writing once the messages are actually on screen. Stored
    // messages arrive in batches, so this runs again on each batch and keeps the
    // view at the end until the reader scrolls, types or sends, any of which drops
    // the opening state and hands the view back to them. The move is immediate
    // rather than animated: nobody asked to watch a transcript they have not read
    // scroll past.
    if (renderedItems.length === 0 || !scrollState.openingToLatest) return;
    let cancelled = false;
    let frame: number | null = null;
    let framesLeft = 4;
    const settleAtLatest = (): void => {
      frame = null;
      if (cancelled || !host || !scrollState.openingToLatest) return;
      const target = latestWritingScrollTop();
      if (Math.abs(host.scrollTop - target) > 1) host.scrollTop = target;
      follow = true;
      framesLeft -= 1;
      if (framesLeft > 0) frame = requestTrackedAnimationFrame(settleAtLatest);
    };
    void tick().then(() => {
      if (!cancelled) frame = requestTrackedAnimationFrame(settleAtLatest);
    });
    return () => {
      cancelled = true;
      if (frame !== null) cancelTrackedAnimationFrame(frame);
    };
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
    if (animationFrame !== null) cancelTrackedAnimationFrame(animationFrame);
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
      if (progress < 1) animationFrame = requestTrackedAnimationFrame(step);
      else {
        if (settleItemId) {
          const settledTop = itemTop(settleItemId, settleOffsetPx ?? 0);
          if (settledTop !== null) host.scrollTop = settledTop;
        }
        finishAnimation();
      }
    };
    animationFrame = requestTrackedAnimationFrame(step);
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
    return host.scrollHeight - (tail?.offsetHeight ?? 0) - host.clientHeight;
  }

  /** How far the reader is above the end of the writing. Zero means they are
   * level with the prompt box and reading the newest line. */
  function distanceBelowReader(): number {
    return host ? latestWritingScrollTop() - host.scrollTop : 0;
  }

  function anchorUser(itemId: string, motion: ConversationScrollMotion, offsetPx: number, framesLeft = 8): void {
    const top = itemTop(itemId, offsetPx);
    if (top !== null) {
      animateTo(top, motion, itemId, offsetPx);
      return;
    }
    if (framesLeft === 0) return finishAnimation();
    if (framesLeft === 8) {
      const rowIndex = renderedGroups.findIndex((group) => group.items.some((item) => item.itemId === itemId));
      if (rowIndex >= 0) $virtualizer.scrollToIndex(rowIndex, { align: 'start' });
    }
    animationFrame = requestTrackedAnimationFrame(() => {
      animationFrame = null;
      anchorUser(itemId, motion, offsetPx, framesLeft - 1);
    });
  }

  function perform(action: ConversationScrollAction): void {
    if (!host || action.type === 'none') return;
    if (action.type === 'cancel-programmatic-scroll') return cancelProgrammaticScroll();
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
    if (!host || !hasOlder || loadingOlder || !onLoadOlder) return;
    // One viewport of warning, so the page arrives before the reader hits the top.
    if (host.scrollTop > Math.max(viewportHeight, 1)) return;
    captureViewportAnchor();
    onLoadOlder();
  }

  function requestNewerHistory(force = false): void {
    if (!host || !hasNewer || loadingNewer || !onLoadNewer) return;
    if (!force && distanceBelowReader() > Math.max(viewportHeight, 1)) return;
    captureViewportAnchor();
    onLoadNewer();
  }

  // A restored page can mount with estimates that make its bounded tail look
  // shorter than the viewport. Give those rows the same bounded frame window
  // used by send anchoring, then let the normal scroll-position gate backfill.
  function hydrateVisibleWindow(framesLeft = 8): void {
    hydrationFrame = null;
    if (!host || !list || renderedItems.length === 0) return;
    for (const row of list.querySelectorAll<HTMLDivElement>('.turn-row')) {
      $virtualizer.measureElement(row);
    }
    requestOlderHistory();
    if (framesLeft === 0 || loadingOlder || !hasOlder) return;
    hydrationFrame = requestTrackedAnimationFrame(() => hydrateVisibleWindow(framesLeft - 1));
  }

  let hydratedRevision = -1;
  let hydratedWindowId = '';
  $effect(() => {
    const revision = timelineRevision;
    const windowId = renderWindowId;
    if (!host || renderedGroups.length === 0) return;
    if (revision === hydratedRevision && windowId === hydratedWindowId) return;
    hydratedRevision = revision;
    hydratedWindowId = windowId;
    if (hydrationFrame !== null) cancelTrackedAnimationFrame(hydrationFrame);
    hydrationFrame = requestTrackedAnimationFrame(() => hydrateVisibleWindow());
  });

  function restoreViewportAnchor(anchor: PageAnchor): void {
    void tick().then(() => {
      if (!host) return;
      const rowIndex = renderedGroups.findIndex((group) =>
        group.items.some((item) => item.itemId === anchor.itemId)
      );
      if (rowIndex >= 0) $virtualizer.scrollToIndex(rowIndex, { align: 'start' });
      requestTrackedAnimationFrame(() => {
        if (!host) return;
        const item = host.querySelector<HTMLElement>(`[data-item-id="${CSS.escape(anchor.itemId)}"]`);
        if (item) host.scrollTop += item.getBoundingClientRect().top - anchor.viewportTop;
        if (follow) pageAnchor = null;
        else captureViewportAnchor();
      });
    });
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
    follow = distanceBelowReader() <= 80;
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
    if (hasNewer && onJumpToLatest) {
      jumpingToLatest = true;
      // A direct tail reload replaces the paging operation. Do not let its
      // pending viewport correction pull the reader back from the newest row.
      pageAnchor = null;
      try {
        await onJumpToLatest();
        await tick();
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
    if (decision.action.type !== 'none') void tick().then(() => perform(decision.action));
  });

  $effect(() => {
    if (timelineRevision === lastContentRevision) return;
    lastContentRevision = timelineRevision;
    const decision = decideConversationScroll(scrollState, { type: 'stream-growth' });
    scrollState = decision.state;
    void tick().then(() => {
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
  });

  $effect(() => {
    if (composerHeight === lastComposerHeight) return;
    lastComposerHeight = composerHeight;
    if (!scrollState.pinnedToBottom) return;
    void tick().then(() => {
      if (host) animateTo(latestWritingScrollTop(), 'instant');
    });
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

  /** Hands each mounted turn to the virtualizer, which reads its real height
   * from `data-index` and re-lays the rows below it. A turn that grows while it
   * is on screen — a reply streaming in, a fold opening — is re-measured by the
   * observer the virtualizer keeps on the element. */
  function measureRow(node: HTMLDivElement): { destroy(): void } {
    $virtualizer.measureElement(node);
    publishTimelineDiagnostics();
    return {
      destroy(): void {
        // Svelte destroys the action before removing its element. TanStack's
        // null cleanup only evicts elements that are already disconnected, so
        // running it synchronously leaves this row and its message subtree in
        // elementsCache. Wait until Svelte has detached the row, then sweep it.
        queueMicrotask(() => {
          $virtualizer.measureElement(null);
          publishTimelineDiagnostics();
        });
      }
    };
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
        if (hydrationFrame !== null) cancelTrackedAnimationFrame(hydrationFrame);
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
    {#if renderedItems.length === 0}<p class="empty" data-testid="conversation-timeline-empty">{emptyText}</p>{/if}
    <div
      class="timeline-list"
      data-testid="conversation-timeline-list"
      bind:this={list}
      style={`height:${$virtualizer.getTotalSize() + (showActiveTurnTail ? viewportHeight : 0)}px`}
    >
      {#each $virtualizer.getVirtualItems() as row (row.key)}
          {@const group = renderedGroups[row.index]}
          {#if group}
            {@const expanded = turnExpanded(group)}
            {@const firstWorkItemId = group.workItemIds[0]}
            <div
              class="turn-row"
              data-index={row.index}
              use:measureRow
              style={`transform:translateY(${row.start}px)`}
            >
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
            </div>
          {/if}
      {/each}
      {#if showActiveTurnTail}
        <!-- Inside the list and directly after the last row, which is where it
             sat before the transcript was virtualized. Outside it, this became a
             screen-tall blank between the newest message and the prompt, because
             it stacked with the scroll box's own bottom padding. -->
        <div
          class="active-turn-tail"
          bind:this={tail}
          style={`transform:translateY(${$virtualizer.getTotalSize()}px);height:${viewportHeight}px`}
          aria-hidden="true"
        ></div>
      {/if}
    </div>
  </div>
  {#if !follow && renderedItems.length > 0}<button class="jump-latest" data-testid="conversation-jump-latest" type="button" aria-label="Jump to latest" onclick={() => void jumpToLatest()}><ArrowDown size={16} strokeWidth={2} aria-hidden="true" /></button>{/if}
</div>

<style>
  .timeline-wrap{position:relative;flex:1;min-height:0}
  /* Laid over the top of the transcript rather than placed in it: a row in the
     scroll flow would change its height while a page is arriving, and the
     scroll position is being corrected against exactly that height. */
  /* A pill rather than a line of small grey text: reaching the top of a long
     transcript takes several reads, and the reader has to be able to tell the
     difference between one running and nothing happening. */
  .older-loading{position:absolute;top:calc(var(--center-head-height) + 6px);left:0;right:0;z-index:2;display:flex;align-items:center;justify-content:center;gap:6px;margin:0;font-size:12px;color:var(--color-text-2);pointer-events:none}
  /* The turn is the only animation in the app that repeats, and it exists only
     while a read is actually running — the element is removed when it ends. */
  .older-spinner{width:11px;height:11px;border:1.5px solid color-mix(in srgb,var(--color-text-3) 45%,transparent);border-top-color:var(--color-text-2);border-radius:50%;animation:older-spin 700ms linear infinite}
  @keyframes older-spin{to{transform:rotate(360deg)}}
  @media (prefers-reduced-motion: reduce){.older-spinner{animation:none;border-top-color:color-mix(in srgb,var(--color-text-3) 45%,transparent)}}
  /* scrollbar-width/-color are set here rather than on the shell root: they are
     inherited, and declaring them globally turns every overlay scrollbar in the
     app into a permanent one, including horizontal bars nobody asked for. Code
     blocks and tables carry their own overflow, so this pane never scrolls sideways. */
  /* The top inset is the centre pane's head, because that is what is laid over
     the top of this box. Whatever comes to rest at the top — the first message
     of a short conversation, or the one just sent — lands below the controls
     rather than behind them. It is an inset, not a gap: the reading still
     scrolls up through it. */
  .timeline-scroll{box-sizing:border-box;display:flex;flex-direction:column;height:100%;overflow:auto;overflow-x:hidden;overflow-anchor:none;padding:var(--center-head-height) 24px calc(var(--composer-height) + 16px);scrollbar-width:thin;scrollbar-color:var(--scrollbar-thumb) transparent;scrollbar-gutter:stable;overscroll-behavior:contain}
  /* The list is now a measured column of absolutely placed turns, so its own
     height is what the virtualizer reports rather than what its children add up
     to. The 16px that used to be `gap` lives on each row instead: absolute
     children are not flex items and gap would not reach them. */
  /* Top-aligned on purpose. A conversation starts at the top of the pane and
     grows downward; when it reaches the prompt the scroll box takes over and the
     newest writing stays at the bottom. Bottom-anchoring this (margin-top:auto)
     leaves a short conversation stranded against the prompt with the empty pane
     above it, which is not how a conversation reads. */
  .timeline-list{position:relative;flex:none;width:min(820px,100%);min-height:1px;margin:0 auto}
  .turn-row{position:absolute;top:0;left:0;display:flex;flex-direction:column;gap:16px;width:100%;padding-bottom:16px}
  .turn-fold{display:flex;width:100%;align-items:center;gap:5px;min-height:28px;padding:0 0 7px;border:0;border-bottom:1px solid var(--color-border);background:transparent;color:var(--color-text-2);font:inherit;font-size:13px;text-align:left;cursor:pointer}
  .turn-fold:hover{color:var(--color-text)}
  .turn-fold:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:2px}
  .turn-fold-chevron{display:grid;place-items:center;color:var(--color-text-3)}
  .turn-fold-chevron.open{transform:rotate(90deg)}
  .empty{display:grid;flex:1;place-items:center;min-height:100%;margin:0;color:var(--color-text-2);font-size:13px}
  .working-row{display:flex;align-items:center;gap:8px;min-height:20px;color:var(--color-text-3);font-size:13px}
  .active-turn-tail{position:absolute;top:0;left:0;width:100%;pointer-events:none}
  /* A disc under the middle of the transcript, holding one arrow. It sits over
     the column it scrolls rather than off in the corner, and it says what it
     does by pointing, so it stays out of the reading it is offering to move. */
  .jump-latest{position:absolute;left:50%;bottom:calc(var(--composer-height) + 16px);display:grid;place-items:center;width:32px;height:32px;padding:0;transform:translateX(-50%);border:1px solid color-mix(in srgb,var(--color-border) 68%,transparent);border-radius:999px;background:color-mix(in srgb,var(--color-elevated) 94%,var(--color-accent) 6%);color:var(--color-text);box-shadow:var(--shadow-sm);cursor:pointer}
  .jump-latest:hover{background:var(--color-hover)}
  .jump-latest:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:2px}
  @media (prefers-reduced-motion:no-preference){.turn-fold-chevron{transition:transform .14s ease}.jump-latest{transition:background .14s ease,box-shadow .14s ease}}
</style>
