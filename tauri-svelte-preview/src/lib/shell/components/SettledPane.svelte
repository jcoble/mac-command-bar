<script lang="ts">
  import WorktreeAgentRow from './WorktreeAgentRow.svelte';
  import type { OwnedSession } from '$lib/shell/ownedSessions';
  import { deriveOwnedLibraryState } from '$lib/shell/sessionLibrary/sessionLibraryModel';

  interface Props {
    sessions: OwnedSession[];
    activeOwnedId?: string | null;
    onSelect?(ownedId: string): void;
    onUnsettle?(ownedId: string): void;
    onRestart?(ownedId: string): void;
    onClose?(ownedId: string): void;
  }
  let {
    sessions,
    activeOwnedId = null,
    onSelect,
    onUnsettle,
    onRestart,
    onClose
  }: Props = $props();

  // Settled is explicit: only settledAt moves a record here. Age, title and
  // terminal state are intentionally not consulted.
  const settled = $derived(
    sessions.filter((session) => deriveOwnedLibraryState(session) === 'settled')
  );
  let expanded = $state<Record<string, boolean>>({});

  function toggle(ownedId: string): void {
    expanded = { ...expanded, [ownedId]: expanded[ownedId] !== true };
  }
</script>

<section data-testid="settled-pane" class="pane-section">
  <header data-testid="settled-pane-header" class="pane-heading">
    <span class="pane-heading-label"><span class="pane-heading-dot" aria-hidden="true"></span>Settled</span>
    <span data-testid="settled-pane-count" class="pane-count">{settled.length}</span>
  </header>
  <ul data-testid="settled-pane-list" class="pane-list">
    {#each settled as session (session.ownedId)}
      <WorktreeAgentRow
        {session}
        active={session.ownedId === activeOwnedId}
        expanded={expanded[session.ownedId] === true}
        onSelect={() => onSelect?.(session.ownedId)}
        onToggle={() => toggle(session.ownedId)}
        onRestart={() => onRestart?.(session.ownedId)}
        onUnsettle={() => onUnsettle?.(session.ownedId)}
        onClose={() => onClose?.(session.ownedId)}
      />
    {/each}
    {#if settled.length === 0}
      <li data-testid="settled-pane-empty" class="pane-empty">No settled references</li>
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
  .pane-heading-dot { width: 7px; height: 7px; border-radius: 999px; background: var(--color-text-3); }
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
