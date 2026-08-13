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
  import { buildMyWorkGroups, type MyWorkViewOptions } from './myWorkViewOptions.ts';
  import WorktreeAgentRow from './WorktreeAgentRow.svelte';

  interface Props {
    sessions: OwnedSession[];
    options: MyWorkViewOptions;
    activeOwnedId?: string | null;
    onSelect?(ownedId: string): void;
    onRestart?(ownedId: string): void;
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
    onRestart,
    onComplete,
    onReopen,
    onSettle,
    onUnsettle,
    onAskRemove
  }: Props = $props();

  const groups = $derived(buildMyWorkGroups(sessions, options));
  let collapsedGroups = $state<Record<string, boolean>>({});

  function isOpen(key: string): boolean {
    return collapsedGroups[key] !== true;
  }

  function toggleGroup(key: string): void {
    collapsedGroups = { ...collapsedGroups, [key]: isOpen(key) };
  }
</script>

<div data-testid="session-rail" class="session-scroll">
  {#each groups as group (group.key)}
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
        </button>
      {/if}

      {#if isOpen(group.key)}
        <ul class="rows">
          {#each group.sessions as session (session.ownedId)}
            <WorktreeAgentRow
              {session}
              active={session.ownedId === activeOwnedId}
              onSelect={() => onSelect?.(session.ownedId)}
              onRestart={() => onRestart?.(session.ownedId)}
              onComplete={() => onComplete?.(session.ownedId)}
              onReopen={() => onReopen?.(session.ownedId)}
              onSettle={() => onSettle?.(session.ownedId)}
              onUnsettle={() => onUnsettle?.(session.ownedId)}
              onAskRemove={() => onAskRemove?.(session.ownedId)}
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
    border-bottom: 1px solid color-mix(in srgb, var(--color-text) 5.5%, transparent);
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
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-variant-numeric: tabular-nums;
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
