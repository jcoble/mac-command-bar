<script lang="ts">
  import WorktreeAgentRow from './WorktreeAgentRow.svelte';
  import type { OwnedSession } from '$lib/shell/ownedSessions';
  import { deriveOwnedLibraryState } from '$lib/shell/sessionLibrary/sessionLibraryModel';

  interface Props {
    sessions: OwnedSession[];
    activeOwnedId?: string | null;
    onSelect?(ownedId: string): void;
    onRestart?(ownedId: string): void;
    onReopen?(ownedId: string): void;
    onSettle?(ownedId: string): void;
    onClose?(ownedId: string): void;
    onAskRemove?(ownedId: string): void;
  }
  let {
    sessions,
    activeOwnedId = null,
    onSelect,
    onRestart,
    onReopen,
    onSettle,
    onClose,
    onAskRemove
  }: Props = $props();

  const done = $derived(
    sessions.filter((session) => deriveOwnedLibraryState(session) === 'done')
  );
  let expanded = $state<Record<string, boolean>>({});

  function toggle(ownedId: string): void {
    expanded = { ...expanded, [ownedId]: expanded[ownedId] !== true };
  }
</script>

<section data-testid="done-pane" class="pane-section">
  <header data-testid="done-pane-header" class="pane-heading">
    <span class="pane-heading-label"><span class="pane-heading-dot" aria-hidden="true"></span>Done</span>
    <span data-testid="done-pane-count" class="pane-count">{done.length}</span>
  </header>
  <ul data-testid="done-pane-list" class="pane-list">
    {#each done as session (session.ownedId)}
      <WorktreeAgentRow
        {session}
        active={session.ownedId === activeOwnedId}
        expanded={expanded[session.ownedId] === true}
        onSelect={() => onSelect?.(session.ownedId)}
        onToggle={() => toggle(session.ownedId)}
        onRestart={() => onRestart?.(session.ownedId)}
        onReopen={() => onReopen?.(session.ownedId)}
        onSettle={() => onSettle?.(session.ownedId)}
        onClose={() => onClose?.(session.ownedId)}
        onAskRemove={() => onAskRemove?.(session.ownedId)}
      />
    {/each}
    {#if done.length === 0}
      <li data-testid="done-pane-empty" class="pane-empty">Nothing marked done</li>
    {/if}
  </ul>
</section>

<style>
  .pane-section { display: flex; min-height: 0; flex-direction: column; }
  .pane-heading {
    display: flex; min-height: 35px; align-items: center; justify-content: space-between;
    padding: 10px 12px 6px; color: var(--color-text-3); font-size: 12px; font-weight: 600;
    letter-spacing: 0.08em; text-transform: uppercase;
  }
  .pane-heading-label { display: inline-flex; align-items: center; gap: 6px; }
  .pane-heading-dot { width: 7px; height: 7px; border-radius: 999px; background: var(--color-good); }
  .pane-count {
    padding: 1px 6px;
    border-radius: 999px;
    color: var(--color-text-3);
    background: var(--color-elevated);
    font-weight: 600;
    letter-spacing: 0;
  }
  .pane-list { min-width: 0; margin: 0; padding: 0; list-style: none; }
  .pane-empty { padding: 8px; color: var(--color-text-3); font-size: 13px; }
</style>
