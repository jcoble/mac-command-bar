<script lang="ts">
  import { tick, untrack, setContext } from 'svelte';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import ArrowDown from '@lucide/svelte/icons/arrow-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import type { AgentConversationTurnFacts } from '$lib/shell/conversation/conversationTypes.ts';
  import { conversationTurnGroups, type ConversationTurnGroup, type ConversationDisplayItem, type ConversationFileLinkProvenance } from '$lib/shell/conversation/conversationTimeline.ts';
  import { USER_SEND_ANCHOR_OFFSET_PX, type ConversationSendAnchorRequest } from '$lib/shell/conversation/conversationScrollAnchor.ts';
  import { conversationDisclosureContext, type ConversationDisclosureContext } from '$lib/shell/conversation/conversationChatUI.ts';
  import type { ConversationViewState } from '$lib/shell/sessionWorkspaces.ts';
  import { conversationItemHasVisibleContent } from '$lib/shell/conversation/conversationItemVisibility.ts';
  import { setConversationTimelineDiagnostics } from '$lib/shell/resourceDiagnostics.svelte';
  import TimelineItem from './TimelineItem.svelte';
  import TurnFileCard from './TurnFileCard.svelte';
  import PendingFirstMessage from './PendingFirstMessage.svelte';
  import WorkingSpinner from './WorkingSpinner.svelte';
  import ConversationTurnElapsed from './ConversationTurnElapsed.svelte';

  interface Props {
    items: readonly ConversationDisplayItem[];
    conversationId: string;
    historyOwnedId: string;
    renderWindowId?: string;
    timelineRevision: number;
    viewState: ConversationViewState;
    onViewChange(ownedId: string, historyOwnedId: string, view: ConversationViewState): void;
    itemFirstSequence(itemId: string): number | undefined;
    anchorRequest?: ConversationSendAnchorRequest | null;
    activeTurnId?: string | null;
    turnFacts?: readonly AgentConversationTurnFacts[];
    localTurnActive?: boolean;
    activityLabel?: string | null;
    composerHeight?: number;
    assistantLabel?: string;
    emptyText?: string;
    pendingFirstMessage?: string | null;
    hasOlder?: boolean;
    loadingOlder?: boolean;
    onLoadOlder?(): void;
    hasNewer?: boolean;
    loadingNewer?: boolean;
    onLoadNewer?(): void;
    oldestSequence?: number;
    newestSequence?: number;
    onJumpToLatest?(): void | Promise<void>;
    onApprovalDecision?(requestId: string, decision: string): void;
    onFileLink?(path: string, provenance?: ConversationFileLinkProvenance): void;
    onPlanOpen?(): void;
  }
  let {
    items, conversationId, historyOwnedId, renderWindowId = conversationId, timelineRevision,
    viewState, onViewChange, itemFirstSequence, anchorRequest = null, activeTurnId = null,
    turnFacts = [], localTurnActive = false, activityLabel = null, composerHeight = 0,
    assistantLabel = 'Assistant', emptyText = 'Start the conversation below.', pendingFirstMessage = null,
    hasOlder = false, loadingOlder = false, onLoadOlder, hasNewer = false, loadingNewer = false,
    onLoadNewer, oldestSequence = 0, newestSequence = 0, onJumpToLatest, onApprovalDecision,
    onFileLink, onPlanOpen
  }: Props = $props();

  let host = $state<HTMLDivElement | null>(null);
  let follow = $state(true);
  let expandedTurns = $state<Record<string, boolean>>({});
  let disclosures = $state<Record<string, boolean>>({});
  let openedWindow = '';
  let openedOwnedId = '';
  let openedHistoryId = '';
  let restoring = $state(true);
  let savedAnchor: ConversationViewState['anchor'];
  let jumping = $state(false);
  let seenSendRequest = 0;
  let anchoredSendItemId = $state<string | null>(null);
  let userDirection: 'older' | 'newer' | null = null;
  let dragging = false;
  let lastScrollTop = 0;
  let pendingSendAnchor = $state<ConversationSendAnchorRequest | null>(null);
  let pageRequest: { older: boolean; edge: number; windowId: string } | null = null;
  const renderedItems = $derived(items.filter(conversationItemHasVisibleContent));
  const groups = $derived(conversationTurnGroups(renderedItems, activeTurnId, turnFacts));

  function turnExpanded(group: ConversationTurnGroup): boolean {
    return !group.turnId || (expandedTurns[group.turnId] ?? group.running);
  }
  type Row = {
    key: string;
    item?: ConversationDisplayItem;
    group?: ConversationTurnGroup;
    heading?: boolean;
    edit?: { path: string; added: number; removed: number };
    activity?: boolean;
  };
  const rows = $derived.by((): Row[] => {
    const result: Row[] = [];
    for (const group of groups) {
      const expanded = turnExpanded(group);
      const workIds = new Set(group.workItemIds);
      const firstWork = group.items.find((item) => workIds.has(item.itemId));
      for (const item of group.items) {
        const work = workIds.has(item.itemId);
        const heading = item === firstWork && !!group.turnId;
        if (!work || expanded || heading) result.push({
          key: item.itemId, item: !work || expanded ? item : undefined, group, heading
        });
      }
      if (group.completed && expanded) for (const edit of getTurnFileEdits(group)) {
        result.push({ key: `turn-file:${group.turnId}:${edit.path}`, group, edit });
      }
    }
    if (activityLabel) result.push({ key: 'activity', activity: true });
    return result;
  });
  const virtualizer = createVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: 0, getScrollElement: () => host, estimateSize: () => 90, overscan: 3,
    anchorTo: 'end', followOnAppend: false, scrollEndThreshold: -1, gap: 12
  });
  const virtualRows = $derived($virtualizer.getVirtualItems());
  const totalSize = $derived($virtualizer.getTotalSize());

  setContext<ConversationDisclosureContext>(conversationDisclosureContext, {
    get(key) { return disclosures[key]; },
    set(key, open) { disclosures = { ...disclosures, [key]: open }; saveView(); }
  });

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

  function captureAnchor(): ConversationViewState['anchor'] {
    if (!host) return savedAnchor;
    const viewportTop = host.getBoundingClientRect().top;
    const candidates = [...host.querySelectorAll<HTMLElement>('[data-anchor-item-id]')];
    const node = candidates.find((candidate) => candidate.getBoundingClientRect().bottom > viewportTop);
    const itemId = node?.dataset.anchorItemId;
    const firstSequence = itemId ? itemFirstSequence(itemId) : undefined;
    return node && itemId && firstSequence !== undefined
      ? { itemId, firstSequence, offsetPx: node.getBoundingClientRect().top - viewportTop }
      : savedAnchor;
  }

  function saveView(): void {
    if (restoring || !openedOwnedId || openedWindow !== renderWindowId) return;
    savedAnchor = captureAnchor();
    const effective = { ...expandedTurns };
    for (const group of groups) if (group.turnId && group.running) effective[group.turnId] = turnExpanded(group);
    onViewChange(openedOwnedId, openedHistoryId, {
      followLatest: follow, ...(savedAnchor ? { anchor: savedAnchor } : {}),
      expandedTurns: effective, ...(Object.keys(disclosures).length ? { disclosures: { ...disclosures } } : {})
    });
  }

  $effect(() => {
    const windowId = renderWindowId;
    if (openedWindow === windowId) return;
    openedWindow = windowId;
    openedOwnedId = conversationId;
    openedHistoryId = historyOwnedId;
    const saved = untrack(() => viewState);
    follow = saved.followLatest;
    expandedTurns = { ...saved.expandedTurns };
    disclosures = { ...saved.disclosures };
    savedAnchor = saved.anchor;
    restoring = true;
    pendingSendAnchor = null;
    anchoredSendItemId = null;
    pageRequest = null;
    jumping = false;
    seenSendRequest = anchorRequest?.requestId ?? 0;
    $virtualizer.measure();
  });

  $effect(() => {
    const currentRows = rows;
    const element = host;
    const following = follow && !hasNewer && !restoring;
    const topInset = element ? Number.parseFloat(getComputedStyle(element).getPropertyValue('--center-head-height')) || 0 : 0;
    const footer = Math.max(composerHeight, 120) + 60;
    const paddingEnd = anchoredSendItemId ? Math.max(element?.clientHeight ?? 0, footer) : footer;
    untrack(() => $virtualizer.setOptions({
      count: currentRows.length,
      // Each options snapshot retains its own key list for prepend/trim comparison.
      getItemKey: (index) => currentRows[index]?.key ?? index,
      paddingStart: topInset, paddingEnd,
      scrollPaddingStart: topInset + (anchoredSendItemId ? USER_SEND_ANCHOR_OFFSET_PX : 0),
      followOnAppend: following, scrollEndThreshold: following ? 80 : -1
    }));
    // Measurements belong to this bounded selected window, not every visited page.
    const keys = new Set(currentRows.map((row) => row.key));
    untrack(() => {
      for (const key of $virtualizer.itemSizeCache.keys()) if (!keys.has(String(key))) $virtualizer.itemSizeCache.delete(key);
    });
  });

  $effect(() => {
    void composerHeight;
    if (!follow || restoring || !host) return;
    const windowId = renderWindowId;
    void tick().then(() => {
      if (windowId === renderWindowId && follow && !hasNewer && !restoring) $virtualizer.scrollToEnd();
    });
  });

  function restoreAnchor(anchor: NonNullable<ConversationViewState['anchor']>): boolean {
    const index = rows.findIndex((row) => row.key === anchor.itemId);
    if (index < 0) return false;
    const item = $virtualizer.getMeasurements()[index];
    if (!item) return false;
    $virtualizer.scrollToOffset(item.start - anchor.offsetPx);
    return true;
  }

  $effect(() => {
    if (!restoring || !host || rows.length === 0) return;
    const windowId = renderWindowId;
    void tick().then(() => {
      if (windowId !== renderWindowId || !restoring) return;
      if (follow) $virtualizer.scrollToEnd();
      else if (savedAnchor && !restoreAnchor(savedAnchor)) return;
      // Measure the admitted row before the final saved item-offset correction.
      void tick().then(() => {
        if (windowId !== renderWindowId || !restoring) return;
        if (follow) $virtualizer.scrollToEnd();
        else if (savedAnchor) restoreAnchor(savedAnchor);
        restoring = false;
        saveView();
      });
    });
  });

  function measureRow(node: HTMLDivElement) {
    $virtualizer.measureElement(node);
    return { destroy() { $virtualizer.measureElement(null); } };
  }

  function toggleTurn(group: ConversationTurnGroup): void {
    if (!group.turnId) return;
    const itemId = group.workItemIds[0];
    const node = itemId && host?.querySelector<HTMLElement>(`[data-anchor-item-id="${CSS.escape(itemId)}"]`);
    const offsetPx = node && host ? node.getBoundingClientRect().top - host.getBoundingClientRect().top : null;
    expandedTurns = { ...expandedTurns, [group.turnId]: !turnExpanded(group) };
    const windowId = renderWindowId;
    void tick().then(() => {
      if (windowId !== renderWindowId) return;
      const index = rows.findIndex((row) => row.key === itemId);
      const measurement = $virtualizer.getMeasurements()[index];
      if (measurement && offsetPx !== null) $virtualizer.scrollToOffset(measurement.start - offsetPx);
      saveView();
    });
  }

  $effect(() => {
    const running = groups.filter((group) => group.running && group.turnId && expandedTurns[group.turnId] === undefined);
    if (running.length) expandedTurns = { ...expandedTurns, ...Object.fromEntries(running.map((group) => [group.turnId!, true])) };
  });

  function requestPage(older: boolean): void {
    if (!host || restoring || jumping || pageRequest) return;
    if (older ? !hasOlder || loadingOlder || host.scrollTop > 80 : !hasNewer || loadingNewer || !$virtualizer.isAtEnd(80)) return;
    pageRequest = { older, edge: older ? oldestSequence : newestSequence, windowId: renderWindowId };
    if (older) onLoadOlder?.(); else onLoadNewer?.();
  }

  $effect(() => {
    if (loadingOlder || loadingNewer || !pageRequest) return;
    const request = pageRequest;
    const landed = request.windowId === renderWindowId && (request.older ? oldestSequence < request.edge : newestSequence > request.edge);
    pageRequest = null;
    // Collapsed work can add no visible height: keep reading until a visible page arrives.
    if (landed) void tick().then(() => {
      if (request.windowId === renderWindowId) requestPage(request.older);
    });
  });

  function handleScroll(): void {
    if (!host || restoring || jumping) return;
    const direction = dragging ? (host.scrollTop < lastScrollTop ? 'older' : 'newer') : userDirection;
    lastScrollTop = host.scrollTop;
    if (direction === 'older') follow = false;
    else if (direction === 'newer' && !hasNewer && $virtualizer.isAtEnd(80)) follow = true;
    userDirection = null;
    saveView();
    if (direction) requestPage(direction === 'older');
  }
  function handleWheel(event: WheelEvent): void {
    anchoredSendItemId = null;
    userDirection = event.deltaY < 0 ? 'older' : 'newer';
    if (event.deltaY < 0) {
      follow = false;
      saveView();
      requestPage(true);
    } else if (event.deltaY > 0) {
      if (!hasNewer && $virtualizer.isAtEnd(80)) follow = true;
      saveView();
      requestPage(false);
    }
  }

  function readerInput(node: HTMLDivElement) {
    const pointerDown = (event: PointerEvent) => {
      if (event.target !== node) return;
      dragging = true;
      lastScrollTop = node.scrollTop;
      anchoredSendItemId = null;
      follow = false;
    };
    const pointerUp = () => { dragging = false; };
    const touchStart = () => {
      dragging = true;
      lastScrollTop = node.scrollTop;
      anchoredSendItemId = null;
      follow = false;
    };
    const keydown = (event: KeyboardEvent) => {
      if (event.target instanceof Element && event.target.closest('input, textarea, [contenteditable]')) return;
      const older = ['ArrowUp', 'PageUp', 'Home'].includes(event.key) || (event.key === ' ' && event.shiftKey);
      const newer = ['ArrowDown', 'PageDown', 'End'].includes(event.key) || (event.key === ' ' && !event.shiftKey);
      if (!older && !newer) return;
      anchoredSendItemId = null;
      userDirection = older ? 'older' : 'newer';
      if (older) follow = false;
      saveView();
      requestPage(older);
    };
    node.addEventListener('pointerdown', pointerDown);
    node.addEventListener('touchstart', touchStart, { passive: true });
    window.addEventListener('pointerup', pointerUp);
    window.addEventListener('touchend', pointerUp);
    window.addEventListener('keydown', keydown);
    return { destroy() {
      node.removeEventListener('pointerdown', pointerDown);
      node.removeEventListener('touchstart', touchStart);
      window.removeEventListener('pointerup', pointerUp);
      window.removeEventListener('touchend', pointerUp);
      window.removeEventListener('keydown', keydown);
    } };
  }

  async function jumpToLatest(): Promise<void> {
    if (jumping) return;
    const windowId = renderWindowId;
    jumping = true;
    anchoredSendItemId = null;
    pageRequest = null;
    try {
      if (hasNewer) await onJumpToLatest?.();
      if (windowId !== renderWindowId) return;
      follow = true;
      savedAnchor = undefined;
      await tick();
      if (windowId !== renderWindowId) return;
      $virtualizer.scrollToEnd();
    } finally {
      if (windowId === renderWindowId) { jumping = false; saveView(); }
    }
  }

  $effect(() => {
    if (!anchorRequest || anchorRequest.conversationId !== conversationId || anchorRequest.requestId === seenSendRequest) return;
    seenSendRequest = anchorRequest.requestId;
    pendingSendAnchor = anchorRequest;
    follow = false;
  });
  $effect(() => {
    void timelineRevision;
    if (!pendingSendAnchor) return;
    const itemId = pendingSendAnchor.userItemId;
    if (!rows.some((row) => row.key === itemId)) return;
    pendingSendAnchor = null;
    anchoredSendItemId = itemId;
    const windowId = renderWindowId;
    void tick().then(() => {
      if (windowId !== renderWindowId || anchoredSendItemId !== itemId) return;
      const index = rows.findIndex((row) => row.key === itemId);
      if (index < 0) return;
      $virtualizer.scrollToIndex(index, { align: 'start' });
      saveView();
    });
  });

  $effect(() => {
    void timelineRevision;
    if (!restoring) untrack(saveView);
  });
  $effect(() => {
    setConversationTimelineDiagnostics(rows.length, virtualRows.length, $virtualizer.elementsCache.size, $virtualizer.itemSizeCache.size);
    return () => setConversationTimelineDiagnostics(0, 0, 0, 0);
  });
</script>

<div class="timeline-wrap" data-testid="conversation-timeline-wrap" style={`--composer-height:${composerHeight}px`}>
  {#if loadingOlder}<p class="older-loading" data-testid="conversation-older-loading" role="status"><span class="older-spinner" aria-hidden="true"></span>Loading earlier messages</p>{/if}
  <div class="timeline-scroll" data-testid="conversation-timeline-scroll" bind:this={host} onscroll={handleScroll} onwheel={handleWheel} use:readerInput>
    {#if renderedItems.length === 0}
      {#if pendingFirstMessage}<PendingFirstMessage text={pendingFirstMessage} />{:else}<p class="empty" data-testid="conversation-timeline-empty">{emptyText}</p>{/if}
    {/if}
    <div class="timeline-list" data-testid="conversation-timeline-list" style:height={`${totalSize}px`}>
      {#each virtualRows as virtualRow (virtualRow.key)}
        {@const row = rows[virtualRow.index]}
        {#if row}
          <div class="turn-row" data-index={virtualRow.index} data-anchor-item-id={row.group && !row.edit ? row.key : undefined} data-turn-id={row.group?.turnId} data-testid="conversation-timeline-row" style:transform={`translateY(${virtualRow.start}px)`} use:measureRow>
            {#if row.heading && row.group}
              <button class="turn-fold" data-testid="conversation-turn-fold" type="button" aria-expanded={turnExpanded(row.group)} onclick={() => toggleTurn(row.group!)}>
                <ConversationTurnElapsed running={row.group.running} completed={row.group.completed} startedAtMs={row.group.startedAtMs} elapsedMs={row.group.elapsedMs} />
                <span class="turn-fold-chevron" class:open={turnExpanded(row.group)} aria-hidden="true"><ChevronRight size={14} strokeWidth={1.8} /></span>
              </button>
            {/if}
            {#if row.item}<TimelineItem item={row.item} {assistantLabel} {onApprovalDecision} {onFileLink} {onPlanOpen} />{/if}
            {#if row.edit}<TurnFileCard path={row.edit.path} added={row.edit.added} removed={row.edit.removed} onReview={onFileLink} />{/if}
            {#if row.activity}<div class="working-row" data-testid="conversation-working-indicator" role="status"><WorkingSpinner seed={activeTurnId ?? renderWindowId} /><span>{activityLabel}…</span></div>{/if}
          </div>
        {/if}
      {/each}
    </div>
  </div>
  {#if (!follow || hasNewer) && renderedItems.length > 0}<button class="jump-latest" data-testid="conversation-jump-latest" type="button" aria-label="Jump to latest" onclick={() => void jumpToLatest()}><ArrowDown size={16} strokeWidth={2} aria-hidden="true" /></button>{/if}
</div>

<style>
  .timeline-wrap{position:relative;display:flex;flex-direction:column;width:100%;height:100%;flex:1 1 auto;min-height:0;overflow:hidden}
  .older-loading{position:absolute;top:calc(var(--center-head-height, 0px) + 6px);left:0;right:0;z-index:2;display:flex;align-items:center;justify-content:center;gap:6px;margin:0;font-size:12px;color:var(--color-text-2);pointer-events:none;animation:older-appear 1ms 300ms backwards}
  /* A page read from the local copy lands well inside 300 ms; only a slow one shows this. */
  @keyframes older-appear{from{opacity:0}}
  .older-spinner{width:11px;height:11px;border:1.5px solid color-mix(in srgb,var(--color-text-3) 45%,transparent);border-top-color:var(--color-text-2);border-radius:50%;animation:older-spin 700ms linear infinite}
  @keyframes older-spin{to{transform:rotate(360deg)}}
  @media (prefers-reduced-motion: reduce){.older-spinner{animation:none;border-top-color:color-mix(in srgb,var(--color-text-3) 45%,transparent)}}
  .timeline-scroll{box-sizing:border-box;display:flex;flex-direction:column;width:100%;height:100%;flex:1 1 auto;min-height:0;overflow-y:auto;overflow-x:hidden;overflow-anchor:none;padding:0 28px;scrollbar-width:thin;scrollbar-color:var(--scrollbar-thumb) transparent;overscroll-behavior:contain}
  .timeline-list{flex:none;position:relative;width:min(820px,100%);min-height:1px;margin:0 auto}
  .turn-row{position:absolute;top:0;left:0;display:flex;flex-direction:column;gap:12px;width:100%}
  .turn-fold{display:flex;width:100%;align-items:center;gap:5px;min-height:28px;padding:0 0 7px;border:0;border-bottom:1px solid var(--color-border);background:transparent;color:var(--color-text-2);font:inherit;font-size:13px;text-align:left;cursor:pointer}
  .turn-fold:hover{color:var(--color-text)}
  .turn-fold:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:2px}
  .turn-fold-chevron{display:grid;place-items:center;color:var(--color-text-3)}
  .turn-fold-chevron.open{transform:rotate(90deg)}
  .empty{display:grid;flex:1;place-items:center;min-height:100%;margin:0;color:var(--color-text-2);font-size:13px}
  .working-row{flex:none;display:flex;align-items:center;gap:8px;height:24px;overflow:hidden;white-space:nowrap;color:var(--color-text-3);font-size:13px}
  /* A disc under the middle of the transcript, holding one arrow. It sits over
     the column it scrolls rather than off in the corner, and it says what it
     does by pointing, so it stays out of the reading it is offering to move. */
  .jump-latest{position:absolute;left:50%;bottom:calc(max(var(--composer-height, 0px), 120px) + 20px);display:grid;place-items:center;width:32px;height:32px;padding:0;transform:translateX(-50%);border:1px solid color-mix(in srgb,var(--color-border) 68%,transparent);border-radius:999px;background:color-mix(in srgb,var(--color-elevated) 94%,var(--color-accent) 6%);color:var(--color-text);box-shadow:var(--shadow-sm);cursor:pointer}
  .jump-latest:hover{background:var(--color-hover)}
  .jump-latest:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:2px}
  @media (prefers-reduced-motion:no-preference){.turn-fold-chevron{transition:transform .14s ease}.jump-latest{transition:background .14s ease,box-shadow .14s ease}}
</style>
