<script lang="ts">
  /**
   * The session list: one scroll area, one row height, sticky section headings.
   *
   * Working, Done and Settled are sections of the same list rather than three
   * resizable panes, so nothing has to be dragged to see the sessions further
   * down and no splitter can hide a section. The heading names and counts come
   * from the view options, which is also how a person groups by project instead.
   */
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import { onMount } from 'svelte';

  import type { OwnedSession } from '$lib/shell/ownedSessions';
  import {
    readAssemblySettingFromTauri,
    writeAssemblySettingFromTauri
  } from '$lib/tauriSource';
  import { buildMyWorkGroups, type MyWorkViewOptions } from './myWorkViewOptions.ts';
  import WorktreeAgentRow from './WorktreeAgentRow.svelte';

  interface Props {
    sessions: OwnedSession[];
    options: MyWorkViewOptions;
    activeOwnedId?: string | null;
    onSelect?(ownedId: string): void;
    onComplete?(ownedId: string): void;
    onReopen?(ownedId: string): void;
    onSettle?(ownedId: string): void;
    onUnsettle?(ownedId: string): void;
    onAskRemove?(ownedId: string): void;
  }

  let {
    sessions,
    options,
    activeOwnedId = null,
    onSelect,
    onComplete,
    onReopen,
    onSettle,
    onUnsettle,
    onAskRemove
  }: Props = $props();

  const groups = $derived(buildMyWorkGroups(sessions, options));
  let collapsedGroups = $state<Record<string, boolean>>({});
  let groupOrders = $state<Record<string, string[]>>({});
  let dragState = $state<{ groupKey: string; ownedId: string } | null>(null);
  let dropTarget = $state<{
    groupKey: string;
    ownedId: string;
    position: 'before' | 'after';
  } | null>(null);
  const loadedOrderKeys = new Set<string>();
  const groupOrderVersions = new Map<string, number>();
  let destroyed = false;

  function groupOrderSettingKey(groupKey: string): string {
    return `rail.group-order.${groupKey}`;
  }

  function normalizeGroupOrder(value: unknown): string[] {
    if (!Array.isArray(value)) return [];
    return value.filter((ownedId): ownedId is string => typeof ownedId === 'string');
  }

  onMount(() => {
    return () => {
      destroyed = true;
    };
  });

  $effect(() => {
    for (const group of groups) {
      if (loadedOrderKeys.has(group.key)) continue;
      loadedOrderKeys.add(group.key);
      const restoreVersion = groupOrderVersions.get(group.key) ?? 0;
      void readAssemblySettingFromTauri(groupOrderSettingKey(group.key))
        .then((stored) => {
          if (!destroyed && (groupOrderVersions.get(group.key) ?? 0) === restoreVersion) {
            groupOrders = { ...groupOrders, [group.key]: normalizeGroupOrder(stored) };
          }
        })
        .catch(() => undefined);
    }
  });

  function orderedSessions(groupKey: string, defaultSessions: OwnedSession[]): OwnedSession[] {
    const byId = new Map(defaultSessions.map((session) => [session.ownedId, session]));
    const savedIds = groupOrders[groupKey] ?? [];
    const ordered = savedIds.flatMap((ownedId) => {
      const session = byId.get(ownedId);
      if (!session) return [];
      byId.delete(ownedId);
      return [session];
    });
    return [...ordered, ...byId.values()];
  }

  function sessionNeedsYou(session: OwnedSession): boolean {
    return session.pendingPermission === true
      || session.pendingInput === true
      || session.runtimeState === 'waiting-approval'
      || session.runtimeState === 'waiting-input';
  }

  function persistOrder(groupKey: string, ownedIds: string[]): void {
    groupOrderVersions.set(groupKey, (groupOrderVersions.get(groupKey) ?? 0) + 1);
    groupOrders = { ...groupOrders, [groupKey]: ownedIds };
    void writeAssemblySettingFromTauri(groupOrderSettingKey(groupKey), ownedIds).catch(
      () => undefined
    );
  }

  function clearDrag(): void {
    dragState = null;
    dropTarget = null;
  }

  function handleDragStart(event: DragEvent, groupKey: string, ownedId: string): void {
    const target = event.target;
    if (target instanceof Element && target.closest("[data-slot='icon-button']")) {
      event.preventDefault();
      return;
    }
    if (!event.dataTransfer) return;
    dragState = { groupKey, ownedId };
    dropTarget = null;
    event.dataTransfer.effectAllowed = 'move';
    event.dataTransfer.setData('text/plain', ownedId);
    const quietImage = new Image(1, 1);
    quietImage.src = 'data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';
    event.dataTransfer.setDragImage(quietImage, 0, 0);
  }

  function handleDragOver(event: DragEvent, groupKey: string, ownedId: string): void {
    if (!dragState) return;
    if (dragState.groupKey !== groupKey || dragState.ownedId === ownedId) {
      if (dropTarget !== null) dropTarget = null;
      return;
    }
    const row = event.currentTarget;
    if (!(row instanceof HTMLElement)) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
    const bounds = row.getBoundingClientRect();
    dropTarget = {
      groupKey,
      ownedId,
      position: event.clientY < bounds.top + bounds.height / 2 ? 'before' : 'after'
    };
  }

  function handleDrop(event: DragEvent, groupKey: string, ownedId: string): void {
    const dragged = dragState;
    if (!dragged || !dropTarget || dragged.groupKey !== groupKey || dropTarget.ownedId !== ownedId) {
      clearDrag();
      return;
    }
    event.preventDefault();
    const group = groups.find((candidate) => candidate.key === groupKey);
    if (!group) {
      clearDrag();
      return;
    }
    const reordered = orderedSessions(groupKey, group.sessions)
      .map((session) => session.ownedId)
      .filter((candidate) => candidate !== dragged.ownedId);
    const targetIndex = reordered.indexOf(ownedId);
    const insertAt = dropTarget.position === 'after' ? targetIndex + 1 : targetIndex;
    reordered.splice(insertAt, 0, dragged.ownedId);
    persistOrder(groupKey, reordered);
    clearDrag();
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && dragState) clearDrag();
  }

  function isOpen(key: string): boolean {
    return collapsedGroups[key] !== true;
  }

  function toggleGroup(key: string): void {
    collapsedGroups = { ...collapsedGroups, [key]: isOpen(key) };
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div data-testid="session-rail" class="session-scroll">
  {#each groups as group (group.key)}
    {@const needsYouCount = group.sessions.filter(sessionNeedsYou).length}
    <section data-testid="session-rail-section" data-group-key={group.key} class:collapsed={!isOpen(group.key)}>
      {#if group.label}
        <button
          data-testid="session-rail-section-toggle"
          type="button"
          class="section-heading"
          aria-expanded={isOpen(group.key)}
          onclick={() => toggleGroup(group.key)}
        >
          <!-- One chevron that turns, rather than two that swap: the quarter
               turn is what tells a person the section answered them. -->
          <ChevronRight class="chevron" aria-hidden="true" />
          <span class="name">{group.label}</span>
          <span data-testid="session-rail-section-count" class="count">{group.sessions.length}</span>
          {#if needsYouCount > 0}
            <span
              data-testid="session-rail-needs-you-count"
              class="needs-count"
              aria-label={`${needsYouCount} ${needsYouCount === 1 ? 'session needs' : 'sessions need'} you`}
              title={`${needsYouCount} ${needsYouCount === 1 ? 'session needs' : 'sessions need'} you`}
            >
              <span aria-hidden="true"></span>{needsYouCount}
            </span>
          {/if}
        </button>
      {/if}

      {#if isOpen(group.key)}
        <ul class="rows">
          {#each orderedSessions(group.key, group.sessions) as session (session.ownedId)}
            <WorktreeAgentRow
              {session}
              active={session.ownedId === activeOwnedId}
              dragging={dragState?.ownedId === session.ownedId && dragState?.groupKey === group.key}
              dropPosition={dropTarget?.ownedId === session.ownedId && dropTarget?.groupKey === group.key
                ? dropTarget.position
                : null}
              onSelect={() => onSelect?.(session.ownedId)}
              onComplete={() => onComplete?.(session.ownedId)}
              onReopen={() => onReopen?.(session.ownedId)}
              onSettle={() => onSettle?.(session.ownedId)}
              onUnsettle={() => onUnsettle?.(session.ownedId)}
              onAskRemove={() => onAskRemove?.(session.ownedId)}
              onDragStart={(event) => handleDragStart(event, group.key, session.ownedId)}
              onDragOver={(event) => handleDragOver(event, group.key, session.ownedId)}
              onDrop={(event) => handleDrop(event, group.key, session.ownedId)}
              onDragEnd={clearDrag}
            />
          {/each}
        </ul>
      {/if}
    </section>
  {/each}

  {#if groups.every((group) => group.sessions.length === 0)}
    <p class="empty-state">No work matches these filters</p>
  {/if}

  <div class="scroll-spacer" aria-hidden="true"></div>
</div>

<style>
  /* One scroll area for the whole list, with the scrollbar in its own gutter so
     it never sits on top of a row or a heading. */
  .session-scroll {
    min-height: 0;
    height: 100%;
    overflow-y: scroll;
    scrollbar-gutter: stable;
    scrollbar-width: thin;
    scrollbar-color: var(--color-text-3) var(--color-surface);
    overscroll-behavior: contain;
  }

  .session-scroll::-webkit-scrollbar { width: 9px; }
  .session-scroll::-webkit-scrollbar-track { background: var(--color-surface); }
  .session-scroll::-webkit-scrollbar-thumb {
    min-height: 48px;
    border: 2px solid var(--color-surface);
    border-radius: var(--radius-sm);
    background: var(--color-text-3);
  }

  .section-heading {
    position: sticky;
    top: 0;
    z-index: 3;
    display: flex;
    width: 100%;
    min-height: 33px;
    align-items: center;
    gap: 7px;
    padding: 7px 12px;
    border: 0;
    background: var(--color-surface);
    color: var(--color-text-2);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }

  .section-heading:hover { color: var(--color-text); }
  .section-heading:focus-visible { outline: 2px solid var(--color-focus); outline-offset: -2px; }
  .section-heading :global(.chevron) { width: 13px; height: 13px; flex: 0 0 auto; }
  .section-heading[aria-expanded='true'] :global(.chevron) { transform: rotate(90deg); }
  .section-heading .name { min-width: 0; overflow: hidden; font-weight: 620; text-overflow: ellipsis; white-space: nowrap; }
  .section-heading .count {
    margin-left: auto;
    color: var(--color-text-3);
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .needs-count {
    min-width: 24px;
    height: 20px;
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 0 6px;
    border: 1px solid color-mix(in srgb, var(--color-attention) 34%, transparent);
    border-radius: var(--radius-sm);
    background: var(--color-attention-bg);
    color: var(--color-attention);
    font-size: 12px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
  }

  .needs-count > span {
    width: 6px;
    height: 6px;
    flex: 0 0 auto;
    border-radius: 50%;
    background: var(--color-attention);
  }

  /* A shut section has nothing to stick to, so its heading joins the list. */
  .collapsed .section-heading {
    position: relative;
    top: auto;
    border-top: 1px solid var(--color-border);
  }

  .rows { min-width: 0; margin: 0; padding: 0; list-style: none; }

  /* Opening a section is a short settle, not a height animation: the rows keep
     their real height from the first frame, so nothing above or below them
     moves twice. Both of these end; neither loops. */
  @media (prefers-reduced-motion: no-preference) {
    .section-heading :global(.chevron) { transition: transform 160ms cubic-bezier(0.2, 0, 0, 1); }
    .section-heading { transition: color 120ms ease; }
    .rows { animation: section-in 160ms ease-out; }
  }

  @keyframes section-in {
    from { opacity: 0; transform: translateY(-4px); }
    to { opacity: 1; transform: none; }
  }
  .empty-state { margin: 0; padding: 10px 12px; color: var(--color-text-3); font-size: 13px; }
  .scroll-spacer { height: 64px; }
</style>
