<script lang="ts">
  import ChevronRight from '@lucide/svelte/icons/chevron-right';

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
          type="button"
          class="flex min-h-7 w-full items-center gap-1.5 px-2 py-1 text-left text-[13px]
            font-medium text-[var(--color-text-2)] outline-none hover:bg-[var(--color-elevated)]
            hover:text-[var(--color-text)] focus-visible:ring-2 focus-visible:ring-[var(--color-focus)]"
          aria-expanded={collapsedGroups[group.key] !== true}
          onclick={() => toggleGroup(group.key)}
        >
          <ChevronRight
            class={collapsedGroups[group.key] === true ? 'size-3.5' : 'size-3.5 rotate-90'}
            aria-hidden="true"
          />
          <span class="min-w-0 flex-1 truncate">{group.label}</span>
          <span class="text-[12px] font-normal text-[var(--color-text-3)]">{group.sessions.length}</span>
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
    <p class="m-0 px-2 py-3 text-[13px] text-[var(--color-text-3)]">No work matches these filters</p>
  {/if}
</div>
