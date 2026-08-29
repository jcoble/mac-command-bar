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

  import type { OwnedSession } from '$lib/shell/ownedSessions';
  import { deriveOwnedLibraryState } from '$lib/shell/sessionLibrary/sessionLibraryModel';
  import { buildMyWorkGroups, type MyWorkViewOptions } from './myWorkViewOptions.ts';
  import {
    railElapsedCadenceFor,
    watchRailElapsed,
    type RailElapsedCadence
  } from './railElapsedTicker.ts';
  import WorktreeAgentRow from './WorktreeAgentRow.svelte';
  import SessionRowContextMenu from './SessionRowContextMenu.svelte';
  import { sessionRowJump, type SessionRowSurface } from './sessionRowJump.ts';
  import {
    sessionRowMenuItems,
    type SessionRowMenuAction
  } from './sessionRowMenu.ts';

  interface Props {
    sessions: OwnedSession[];
    options: MyWorkViewOptions;
    activeOwnedId?: string | null;
    onSelectSession?(ownedId: string): void | Promise<void>;
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
    onSelectSession,
    onComplete,
    onReopen,
    onSettle,
    onUnsettle,
    onAskRemove
  }: Props = $props();

  const groups = $derived(buildMyWorkGroups(sessions, options));
  let collapsedGroups = $state<Record<string, boolean>>({});
  let visualActiveOwnedId = $state<string | null>(null);
  let nowMs = $state(Date.now());
  let contextMenu = $state<{ session: OwnedSession; x: number; y: number } | null>(null);
  const contextMenuItems = $derived(contextMenu
    ? sessionRowMenuItems({
        status: deriveOwnedLibraryState(contextMenu.session),
        sessionId: contextMenu.session.nativeSessionId || contextMenu.session.ownedId,
        worktreePath: contextMenu.session.cwd || contextMenu.session.projectPath || null
      })
    : []);

  $effect(() => {
    if (activeOwnedId !== null) visualActiveOwnedId = activeOwnedId;
  });

  function sessionStartedAtMs(session: OwnedSession): number | null {
    if (session.startedAtMs !== null && session.startedAtMs !== undefined) {
      return session.startedAtMs;
    }
    if (!session.lastActivity) return null;
    const parsed = Date.parse(session.lastActivity);
    return Number.isFinite(parsed) ? parsed : null;
  }

  const elapsedCadence = $derived.by<RailElapsedCadence>(() => {
    for (const session of sessions) {
      const startedAtMs = sessionStartedAtMs(session);
      if (startedAtMs === null) continue;
      const working = (session.activeTurnId !== null && session.activeTurnId !== undefined)
        || session.runtimeState === 'starting'
        || session.runtimeState === 'working'
        || session.runtimeState === 'interrupting';
      if (railElapsedCadenceFor(Math.max(0, nowMs - startedAtMs), working) === 'second') {
        return 'second';
      }
    }
    return 'minute';
  });

  // The rail owns one clock subscription. Rows receive the same timestamp as
  // data and therefore create no timers, watcher closures, or cleanup effects.
  $effect(() => {
    if (sessions.length === 0) return;
    const cadence = elapsedCadence;
    return watchRailElapsed((tick) => {
      nowMs = tick;
    }, cadence);
  });

  function sessionNeedsYou(session: OwnedSession): boolean {
    return session.pendingPermission === true
      || session.pendingInput === true
      || session.runtimeState === 'waiting-approval'
      || session.runtimeState === 'waiting-input';
  }

  function isOpen(key: string): boolean {
    return collapsedGroups[key] !== true;
  }

  function toggleGroup(key: string): void {
    collapsedGroups = { ...collapsedGroups, [key]: isOpen(key) };
  }

  async function selectRow(ownedId: string): Promise<void> {
    visualActiveOwnedId = ownedId;
    await onSelectSession?.(ownedId);
  }

  async function jumpTo(session: OwnedSession, surface: SessionRowSurface): Promise<void> {
    visualActiveOwnedId = session.ownedId;
    if (!await sessionRowJump(session.ownedId, surface)) await onSelectSession?.(session.ownedId);
  }

  function openContextMenu(event: MouseEvent, session: OwnedSession): void {
    event.preventDefault();
    event.stopPropagation();
    contextMenu = { session, x: event.clientX, y: event.clientY };
  }

  function copyText(value: string | null): void {
    if (!value || !navigator.clipboard?.writeText) return;
    void navigator.clipboard.writeText(value);
  }

  function runContextAction(action: SessionRowMenuAction): void {
    const session = contextMenu?.session;
    contextMenu = null;
    if (!session) return;
    if (action === 'mark-done') onComplete?.(session.ownedId);
    else if (action === 'reopen') onReopen?.(session.ownedId);
    else if (action === 'archive') onSettle?.(session.ownedId);
    else if (action === 'unsettle') onUnsettle?.(session.ownedId);
    else if (action === 'copy-session-id') copyText(session.nativeSessionId || session.ownedId);
    else if (action === 'copy-worktree-path') copyText(session.cwd || session.projectPath || null);
    else if (action === 'open-in-editor') void jumpTo(session, 'editor');
    else if (action === 'open-source-control') void jumpTo(session, 'source-control');
    else if (action === 'delete') onAskRemove?.(session.ownedId);
  }
</script>

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
            >
              <span aria-hidden="true"></span>{needsYouCount}
            </span>
          {/if}
        </button>
      {/if}

      {#if isOpen(group.key)}
        <ul class="rows">
          {#each group.sessions as session (session.ownedId)}
            <WorktreeAgentRow
              {session}
              {nowMs}
              active={session.ownedId === visualActiveOwnedId}
              onSelect={() => void selectRow(session.ownedId)}
              onOpenSession={() => void jumpTo(session, 'session')}
              onOpenEditor={() => void jumpTo(session, 'editor')}
              onOpenSourceControl={() => void jumpTo(session, 'source-control')}
              onContextMenu={(event) => openContextMenu(event, session)}
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

{#if contextMenu}
  <SessionRowContextMenu
    x={contextMenu.x}
    y={contextMenu.y}
    items={contextMenuItems}
    onSelect={runContextAction}
    onClose={() => { contextMenu = null; }}
  />
{/if}

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
