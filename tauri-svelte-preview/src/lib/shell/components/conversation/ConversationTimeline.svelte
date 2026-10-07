<script lang="ts">
  import { tick, untrack, setContext } from 'svelte';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import ArrowDown from '@lucide/svelte/icons/arrow-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import BookOpen from '@lucide/svelte/icons/book-open';
  import Pencil from '@lucide/svelte/icons/pencil';
  import Terminal from '@lucide/svelte/icons/terminal';
  import Search from '@lucide/svelte/icons/search';
  import Sparkles from '@lucide/svelte/icons/sparkles';
  import type { AgentConversationTurnFacts } from '$lib/shell/conversation/conversationTypes.ts';
  import { conversationTurnGroups, foldToolRuns, type ConversationTurnGroup, type ConversationDisplayItem, type ConversationFileLinkProvenance } from '$lib/shell/conversation/conversationTimeline.ts';
  import { USER_SEND_ANCHOR_OFFSET_PX, type ConversationSendAnchorRequest } from '$lib/shell/conversation/conversationScrollAnchor.ts';
  import { conversationDisclosureContext, type ConversationDisclosureContext } from '$lib/shell/conversation/conversationChatUI.ts';
  import type { ConversationViewState } from '$lib/shell/sessionWorkspaces.ts';
  import { conversationItemHasVisibleContent } from '$lib/shell/conversation/conversationItemVisibility.ts';
  import { setConversationTimelineDiagnostics } from '$lib/shell/resourceDiagnostics.svelte';
  import TimelineItem from './TimelineItem.svelte';
  import TurnFileCard from './TurnFileCard.svelte';
  import PendingFirstMessage from './PendingFirstMessage.svelte';
  import WorkingSpinner from './WorkingSpinner.svelte';

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
    pageError?: string;
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
    hasOlder = false, loadingOlder = false, pageError = '', onLoadOlder, hasNewer = false, loadingNewer = false,
    onLoadNewer, oldestSequence = 0, newestSequence = 0, onJumpToLatest, onApprovalDecision,
    onFileLink, onPlanOpen
  }: Props = $props();

  let host = $state<HTMLDivElement | null>(null);
  let follow = $state(true);
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
  let pageRequest: { older: boolean; edge: number; windowId: string; visibleKeys: string } | null = null;
  const renderedItems = $derived(items.filter(conversationItemHasVisibleContent));
  const groups = $derived(conversationTurnGroups(renderedItems, activeTurnId, turnFacts));

  type Row = {
    key: string;
    anchorItemId?: string;
    item?: ConversationDisplayItem;
    group?: ConversationTurnGroup;
    run?: Extract<ConversationDisplayItem, { kind: 'toolRun' }>;
    edit?: { path: string; added: number; removed: number };
    activity?: boolean;
  };
  // Only the preceding bounded window's native IDs are needed to retain run identity.
  let previousRunWindow = '';
  let previousRuns: { id: string; nativeItemIds: readonly string[] }[] = [];
  function runOpen(run: Extract<ConversationDisplayItem, { kind: 'toolRun' }>): boolean {
    return disclosures[`${run.itemId}:run`]
      ?? disclosures[`tool-run:${run.items[0]?.itemId}:run`] ?? !run.completed;
  }
  const rows = $derived.by((): Row[] => {
    if (previousRunWindow !== renderWindowId) {
      previousRunWindow = renderWindowId;
      previousRuns = [];
    }
    const result: Row[] = [];
    const nextRuns: typeof previousRuns = [];
    const claimed = new Set<string>();
    const foldedGroups = groups.map((group) => ({ group, items: foldToolRuns(group.items) }));
    const seeds = new Set(foldedGroups.flatMap(({ items }) =>
      items.filter((item) => item.kind === 'toolRun').map((item) => item.itemId)));
    for (const { group, items: groupedItems } of foldedGroups) {
      for (const item of groupedItems) {
        let run: Row['run'];
        if (item.kind === 'toolRun') {
          const nativeItemIds = item.items.map((child) => child.itemId);
          const nativeIds = new Set(nativeItemIds);
          const previous = previousRuns.find((candidate) => !claimed.has(candidate.id)
            && (candidate.id === item.itemId || !seeds.has(candidate.id))
            && candidate.nativeItemIds.some((id) => nativeIds.has(id)));
          const id = previous?.id ?? item.itemId;
          claimed.add(id);
          nextRuns.push({ id, nativeItemIds });
          run = { ...item, itemId: id };
        }
        if (run) {
          const open = runOpen(run);
          result.push({ key: run.itemId, group, run,
            anchorItemId: open ? undefined : run.items[0]?.itemId });
          if (open) for (const child of run.items) {
            result.push({ key: child.itemId, anchorItemId: child.itemId, group, item: child });
          }
        } else {
          result.push({ key: item.itemId, anchorItemId: item.itemId, group, item });
        }
      }
      if (group.completed) for (const edit of getTurnFileEdits(group)) {
        result.push({ key: `turn-file:${group.turnId}:${edit.path}`, group, edit });
      }
    }
    previousRuns = nextRuns;
    if (activityLabel) result.push({ key: 'activity', activity: true });
    return result;
  });
  const virtualizer = createVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: 0, getScrollElement: () => host, estimateSize: () => 90, overscan: 3,
    anchorTo: 'end', followOnAppend: false, scrollEndThreshold: -1
  });
  const virtualRows = $derived($virtualizer.getVirtualItems());
  const totalSize = $derived($virtualizer.getTotalSize());
  const viewportHeight = $derived($virtualizer.scrollRect?.height ?? 0);

  $effect.pre(() => {
    const currentRows = rows;
    const element = host;
    const following = follow && !hasNewer && !restoring && !jumping;
    const topInset = element ? Number.parseFloat(getComputedStyle(element).getPropertyValue('--center-head-height')) || 0 : 0;
    const footer = Math.max(composerHeight, 120) + 60;
    const paddingEnd = anchoredSendItemId ? Math.max(viewportHeight, footer) : footer;
    untrack(() => {
      $virtualizer.setOptions({
        count: currentRows.length, getScrollElement: () => element,
        getItemKey: (index) => currentRows[index].key,
        estimateSize: (index) => currentRows[index].run || currentRows[index].item?.kind === 'tool' ? 36 : 90,
        paddingStart: topInset, paddingEnd,
        scrollEndThreshold: following ? 80 : -1
      });
      const keys = new Set(currentRows.map((row) => row.key));
      for (const key of $virtualizer.itemSizeCache.keys()) {
        if (!keys.has(String(key))) $virtualizer.itemSizeCache.delete(key);
      }
    });
  });

  function atEnd(): boolean { return $virtualizer.isAtEnd(80); }
  function scrollToEnd(): void { $virtualizer.scrollToEnd(); }
  function positionRow(key: string, offsetPx: number): boolean {
    const index = rows.findIndex((row) => row.key === key);
    if (index < 0) return false;
    $virtualizer.setOptions({ scrollPaddingStart: offsetPx });
    $virtualizer.scrollToIndex(index, { align: 'start' });
    return true;
  }
  function measureRow(node: HTMLDivElement) {
    $virtualizer.measureElement(node);
    return { destroy() { $virtualizer.measureElement(null); } };
  }

  setContext<ConversationDisclosureContext>(conversationDisclosureContext, {
    get(key) { return disclosures[key]; },
    set(key, open) { follow = false; disclosures = { ...disclosures, [key]: open }; saveView(); }
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
    onViewChange(openedOwnedId, openedHistoryId, {
      followLatest: follow, ...(savedAnchor ? { anchor: savedAnchor } : {}),
      expandedTurns: {}, ...(Object.keys(disclosures).length ? { disclosures: { ...disclosures } } : {})
    });
  }

  $effect(() => {
    const windowId = renderWindowId;
    if (openedWindow === windowId) return;
    openedWindow = windowId;
    openedOwnedId = conversationId;
    openedHistoryId = historyOwnedId;
    userDirection = null;
    dragging = false;
    lastScrollTop = 0;
    const saved = untrack(() => viewState);
    follow = saved.followLatest;
    disclosures = { ...saved.disclosures };
    savedAnchor = saved.anchor;
    restoring = true;
    pendingSendAnchor = null;
    anchoredSendItemId = null;
    pageRequest = null;
    jumping = false;
    seenSendRequest = anchorRequest?.requestId ?? 0;
    untrack(() => $virtualizer.measure());
  });

  $effect(() => {
    void rows;
    void totalSize;
    void composerHeight;
    if (!follow || hasNewer || restoring || jumping || !host) return;
    const windowId = renderWindowId;
    void tick().then(() => {
      if (windowId === renderWindowId && follow && !hasNewer && !restoring && !jumping) scrollToEnd();
    });
  });

  function restoreAnchor(anchor: NonNullable<ConversationViewState['anchor']>): boolean {
    const index = rows.findIndex((row) => row.anchorItemId === anchor.itemId
      || (row.run && !runOpen(row.run) && row.run.items.some((item) => item.itemId === anchor.itemId)));
    if (index < 0) return false;
    return positionRow(rows[index].key, anchor.offsetPx);
  }

  $effect(() => {
    if (!restoring || !host || rows.length === 0) return;
    const windowId = renderWindowId;
    void tick().then(() => {
      if (windowId !== renderWindowId || !restoring) return;
      if (follow) scrollToEnd();
      else if (savedAnchor && !restoreAnchor(savedAnchor)) return;
      // Restore after the admitted content has mounted.
      void tick().then(() => {
        if (windowId !== renderWindowId || !restoring) return;
        if (follow) scrollToEnd();
        else if (savedAnchor) restoreAnchor(savedAnchor);
        restoring = false;
        saveView();
      });
    });
  });

  function toggleRun(run: NonNullable<Row['run']>): void {
    const key = run.itemId;
    const node = host?.querySelector<HTMLElement>(`[data-run-id="${CSS.escape(key)}"]`);
    const offsetPx = node && host ? node.getBoundingClientRect().top - host.getBoundingClientRect().top : null;
    follow = false;
    disclosures = { ...disclosures, [`${key}:run`]: !runOpen(run) };
    const windowId = renderWindowId;
    void tick().then(() => {
      if (windowId !== renderWindowId) return;
      const index = rows.findIndex((row) => row.key === key);
      if (index >= 0 && offsetPx !== null) positionRow(rows[index].key, offsetPx);
      saveView();
    });
  }

  $effect(() => {
    const initial = rows.flatMap((row) => {
      if (!row.run) return [];
      const open = runOpen(row.run);
      return [`${row.run.itemId}:run`, `tool-run:${row.run.items[0]?.itemId}:run`]
        .filter((key) => disclosures[key] !== open).map((key) => [key, open] as const);
    });
    if (initial.length) disclosures = { ...disclosures, ...Object.fromEntries(initial) };
  });

  function requestPage(older: boolean): void {
    if (!host || restoring || jumping || pageRequest) return;
    if (older ? !hasOlder || loadingOlder || host.scrollTop > 80 : !hasNewer || loadingNewer || !atEnd()) return;
    pageRequest = { older, edge: older ? oldestSequence : newestSequence, windowId: renderWindowId,
      visibleKeys: rows.map((row) => row.key).join('\0') };
    if (older) onLoadOlder?.(); else onLoadNewer?.();
  }

  $effect(() => {
    if (loadingOlder || loadingNewer || !pageRequest) return;
    const request = pageRequest;
    const landed = request.windowId === renderWindowId && (request.older ? oldestSequence < request.edge : newestSequence > request.edge);
    pageRequest = null;
    // Collapsed work can add no visible height: keep reading until a visible page arrives.
    if (landed) void tick().then(() => {
      if (request.windowId === renderWindowId
        && request.visibleKeys === rows.map((row) => row.key).join('\0')) requestPage(request.older);
    });
  });

  function handleScroll(): void {
    if (!host || restoring || jumping) return;
    const direction = dragging ? (host.scrollTop < lastScrollTop ? 'older' : 'newer') : userDirection;
    lastScrollTop = host.scrollTop;
    if (direction === 'older') follow = false;
    else if (direction === 'newer' && !hasNewer && atEnd()) follow = true;
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
      if (!hasNewer && atEnd()) follow = true;
      saveView();
      requestPage(false);
    }
  }

  function readerInput(node: HTMLDivElement) {
    const interactive = 'input, textarea, select, button, a, [contenteditable], [role="button"]';
    const pointerDown = (event: PointerEvent) => {
      if (!(event.target instanceof Element && event.target.closest(interactive))) {
        node.focus({ preventScroll: true });
      }
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
      if (event.target instanceof Element && event.target.closest(interactive)) return;
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
    node.addEventListener('keydown', keydown);
    return { destroy() {
      node.removeEventListener('pointerdown', pointerDown);
      node.removeEventListener('touchstart', touchStart);
      window.removeEventListener('pointerup', pointerUp);
      window.removeEventListener('touchend', pointerUp);
      node.removeEventListener('keydown', keydown);
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
      scrollToEnd();
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
      const topInset = host ? Number.parseFloat(getComputedStyle(host).getPropertyValue('--center-head-height')) || 0 : 0;
      positionRow(itemId, topInset + USER_SEND_ANCHOR_OFFSET_PX);
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
  {#if pageError}<p role="alert" class="px-2 py-2 text-sm text-muted-foreground">{pageError}</p>{/if}
  {#if loadingOlder}<p class="older-loading" data-testid="conversation-older-loading" role="status"><span class="older-spinner" aria-hidden="true"></span>Loading earlier messages</p>{/if}
  <div class="timeline-scroll" tabindex="0" data-testid="conversation-timeline-scroll" bind:this={host} onscroll={handleScroll} onwheel={handleWheel} use:readerInput>
    {#if renderedItems.length === 0}
      {#if pendingFirstMessage}<PendingFirstMessage text={pendingFirstMessage} />{:else}<p class="empty" data-testid="conversation-timeline-empty">{emptyText}</p>{/if}
    {/if}
    <div class="timeline-list" data-testid="conversation-timeline-list" style:height={`${totalSize}px`}>
      {#each virtualRows as virtualRow (virtualRow.key)}
        {@const row = rows[virtualRow.index]}
          <div class="turn-row" data-index={virtualRow.index} style:transform={`translateY(${virtualRow.start}px)`} use:measureRow class:compact-tool={row.item?.kind === 'tool' || !!row.run} data-row-key={row.key} data-anchor-item-id={row.anchorItemId} data-turn-id={row.group?.turnId} data-testid="conversation-timeline-row">
            {#if row.run}
              <button class="run-header" data-run-id={row.run.itemId} type="button" aria-expanded={runOpen(row.run)} onclick={() => toggleRun(row.run!)}>
                <span class="run-icon" aria-hidden="true">
                  {#if row.run.icon === 'pencil'}<Pencil size={13} />
                  {:else if row.run.icon === 'book'}<BookOpen size={13} />
                  {:else if row.run.icon === 'terminal'}<Terminal size={13} />
                  {:else if row.run.icon === 'search'}<Search size={13} />
                  {:else}<Sparkles size={13} />{/if}
                </span>
                <span class="run-summary">{row.run.summary}</span>
                <span class="turn-fold-chevron" class:open={runOpen(row.run)} aria-hidden="true"><ChevronRight size={13} /></span>
              </button>
            {/if}
            {#if row.item}<TimelineItem item={row.item} {assistantLabel} {onApprovalDecision} {onFileLink} {onPlanOpen} />{/if}
            {#if row.edit}<TurnFileCard path={row.edit.path} added={row.edit.added} removed={row.edit.removed} onReview={onFileLink} />{/if}
            {#if row.activity}<div class="working-row" data-testid="conversation-working-indicator" role="status"><WorkingSpinner /><span>{activityLabel}…</span></div>{/if}
          </div>
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
  .turn-row{position:absolute;top:0;left:0;display:flex;flex-direction:column;gap:12px;width:100%;padding-bottom:12px}
  .turn-row.compact-tool{padding-bottom:4px}
  .turn-fold-chevron{display:grid;place-items:center;color:var(--color-text-3)}
  .turn-fold-chevron.open{transform:rotate(90deg)}
  .run-header{display:flex;align-items:center;gap:8px;min-height:30px;padding:3px 6px;border:0;border-radius:8px;background:transparent;color:var(--color-text-2);font-size:13px;text-align:left;cursor:pointer}
  .run-header:hover{background:color-mix(in srgb,var(--color-hover) 50%,transparent)}
  .run-header:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:2px}
  .run-icon{display:grid;place-items:center;flex:none;color:var(--color-text-3)}
  .run-summary{flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
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
