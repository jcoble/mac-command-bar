<script lang="ts">
  import type { OwnedSession } from '$lib/shell/ownedSessions';
  import { buildMyWorkGroups, type MyWorkViewOptions } from './myWorkViewOptions';
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
    onClose?(ownedId: string): void;
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
    onClose,
    onAskRemove
  }: Props = $props();

  const groups = $derived(buildMyWorkGroups(sessions, options));
  let collapsedGroups = $state<Record<string, boolean>>({});
  let expandedRows = $state<Record<string, boolean>>({});

  function toggleGroup(key: string): void {
    collapsedGroups = { ...collapsedGroups, [key]: collapsedGroups[key] !== true };
  }

  function toggleRow(ownedId: string): void {
    expandedRows = { ...expandedRows, [ownedId]: expandedRows[ownedId] !== true };
  }
</script>

<div data-testid="my-work-session-list" class="h-full min-h-0 overflow-y-auto">
  {#each groups as group (group.key)}
    <section data-testid="my-work-group" data-group-key={group.key}>
      {#if options.groupBy !== 'none'}
        <button
          data-testid="my-work-group-toggle"
          type="button"
          class="group-head"
          aria-expanded={collapsedGroups[group.key] !== true}
          onclick={() => toggleGroup(group.key)}
        >
          <span class="min-w-0 truncate">{group.label}</span>
          <span class="group-count">{group.sessions.length}</span>
        </button>
      {/if}

      {#if collapsedGroups[group.key] !== true}
        <ul class="m-0 min-w-0 list-none p-0">
          {#each group.sessions as session (session.ownedId)}
            <WorktreeAgentRow
              {session}
              active={session.ownedId === activeOwnedId}
              expanded={expandedRows[session.ownedId] === true}
              onSelect={() => onSelect?.(session.ownedId)}
              onToggle={() => toggleRow(session.ownedId)}
              onRestart={() => onRestart?.(session.ownedId)}
              onComplete={() => onComplete?.(session.ownedId)}
              onReopen={() => onReopen?.(session.ownedId)}
              onSettle={() => onSettle?.(session.ownedId)}
              onUnsettle={() => onUnsettle?.(session.ownedId)}
              onClose={() => onClose?.(session.ownedId)}
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
</div>

<style>
  .group-head {
    display: flex;
    width: 100%;
    min-height: 35px;
    align-items: center;
    gap: 7px;
    padding: 10px 12px 6px;
    border: 0;
    color: var(--color-text-3);
    background: transparent;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-align: left;
    text-transform: uppercase;
    outline: none;
  }
  .group-head:hover,
  .group-head:focus-visible { color: var(--color-text-2); background: var(--color-hover); }
  .group-count {
    padding: 1px 6px;
    border-radius: 999px;
    color: var(--color-text-3);
    background: var(--color-elevated);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0;
  }
  .empty-state { margin: 0; padding: 10px 12px; color: var(--color-text-3); font-size: 13px; }
</style>
