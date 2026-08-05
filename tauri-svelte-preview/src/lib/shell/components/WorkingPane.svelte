<script lang="ts">
  import WorktreeAgentRow from './WorktreeAgentRow.svelte';
  import type { OwnedSession } from '$lib/shell/ownedSessions';
  import { deriveOwnedLibraryState } from '$lib/shell/sessionLibrary/sessionLibraryModel';

  interface Props {
    sessions: OwnedSession[];
    activeOwnedId?: string | null;
    onSelect?(ownedId: string): void;
    onRestart?(ownedId: string): void;
    onComplete?(ownedId: string): void;
    onClose?(ownedId: string): void;
  }
  let {
    sessions,
    activeOwnedId = null,
    onSelect,
    onRestart,
    onComplete,
    onClose
  }: Props = $props();

  const working = $derived(
    sessions.filter((session) => deriveOwnedLibraryState(session) === 'working')
  );
  let expanded = $state<Record<string, boolean>>({});

  function toggle(ownedId: string): void {
    expanded = { ...expanded, [ownedId]: expanded[ownedId] !== true };
  }
</script>

<section data-testid="working-pane" class="pane-section">
  <header data-testid="working-pane-header" class="pane-heading">
    <span class="pane-heading-label"><span class="pane-heading-dot" aria-hidden="true"></span>Working</span>
    <span data-testid="working-pane-count" class="pane-count">{working.length}</span>
  </header>
  <ul data-testid="working-pane-list" class="pane-list">
    {#each working as session (session.ownedId)}
      <WorktreeAgentRow
        {session}
        active={session.ownedId === activeOwnedId}
        expanded={expanded[session.ownedId] === true}
        onSelect={() => onSelect?.(session.ownedId)}
        onToggle={() => toggle(session.ownedId)}
        onRestart={() => onRestart?.(session.ownedId)}
        onComplete={() => onComplete?.(session.ownedId)}
        onClose={() => onClose?.(session.ownedId)}
      />
    {/each}
    {#if working.length === 0}
      <li data-testid="working-pane-empty" class="pane-empty">No active work</li>
    {/if}
  </ul>
</section>

<style>
  .pane-section { display: flex; min-height: 0; flex-direction: column; }
  .pane-heading {
    display: flex; min-height: 30px; align-items: center; justify-content: space-between;
    padding: 4px 8px; color: var(--color-text-2); font-size: 12px; font-weight: 600;
    letter-spacing: 0.08em; text-transform: uppercase;
  }
  .pane-heading-label { display: inline-flex; align-items: center; gap: 6px; }
  .pane-heading-dot { width: 7px; height: 7px; border-radius: 999px; background: var(--color-live); }
  .pane-count { color: var(--color-text-3); font-weight: 400; }
  .pane-list { min-width: 0; margin: 0; padding: 0; list-style: none; }
  .pane-empty { padding: 8px; color: var(--color-text-3); font-size: 13px; }
</style>
