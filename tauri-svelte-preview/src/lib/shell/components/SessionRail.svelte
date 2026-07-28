<script lang="ts">
  /**
   * SessionRail.svelte — the /next shell's left rail. PRESENTATIONAL ONLY.
   *
   * No IO, no stores, no `$effect`: it renders the two groups (sessions
   * CommandBar owns, plus scanned sessions it could resume) and reports every
   * intent through callbacks. The page owns all state and all backend calls.
   *
   * The one piece of logic that lives here is the resume filter: a scanned
   * session already adopted must not be offered again, matched by the owned
   * record's `nativeSessionId` (both the bare provider id and a `provider:id`
   * composite are accepted, so the rail stays correct whichever form an adopt
   * path stored).
   *
   * Rows are arranged under one heading per project folder, and the search box
   * at the top narrows both lists at once. Both of those are decided by
   * `sessionGroups.ts`, which is pure and has its own test; this file only
   * draws what it returns. The search text is the rail's own state and is
   * deliberately NOT reported to the page — narrowing what you can see costs
   * nothing and loads nothing.
   */
  import type { OwnedSession, OwnedSessionState } from '$lib/shell/ownedSessions';
  import { groupSessions } from '$lib/shell/sessionGroups';
  import type { AgentSession } from '$lib/tauriSource';

  interface Props {
    /** Sessions CommandBar owns (rail.owned). */
    owned: OwnedSession[];
    /** Scanned agent sessions from the last rail scan (rail.available). */
    available: AgentSession[];
    /** `ownedId` of the session shown in the main pane, if any. */
    activeOwnedId: string | null;
    /** A rail scan is in flight — disables the rescan button. */
    scanning: boolean;
    /** Focus an owned session. */
    onSelect(ownedId: string): void;
    /** Resume a scanned session (adopt + start a PTY). */
    onAdopt(session: AgentSession): void;
    /** Close a live session's PTY, or dismiss an exited row. */
    onClose(ownedId: string): void;
    /** Re-run the agent-session scan. */
    onRescan(): void;
  }

  let {
    owned,
    available,
    activeOwnedId,
    scanning,
    onSelect,
    onAdopt,
    onClose,
    onRescan
  }: Props = $props();

  /** Every provider id the rail already owns, in both accepted forms. */
  const adoptedIds = $derived(
    new Set(
      owned
        .map((session) => session.nativeSessionId)
        .filter((id): id is string => typeof id === 'string' && id.length > 0)
    )
  );

  /** Scanned sessions that are NOT already adopted. */
  const resumable = $derived(
    available.filter(
      (session) => !adoptedIds.has(session.id) && !adoptedIds.has(`${session.provider}:${session.id}`)
    )
  );

  /** What the search box holds. Empty means "show everything". */
  let query = $state('');

  const grouped = $derived(groupSessions(owned, resumable, query));

  function countIn<T>(groups: { items: T[] }[]): number {
    return groups.reduce((total, group) => total + group.items.length, 0);
  }

  /** `claude` / `codex` / `cmux · claude` when the session runs through cmux. */
  function agentLabel(session: OwnedSession): string {
    return session.viaCmux ? `cmux · ${session.agent}` : session.agent;
  }

  function stateLabel(state: OwnedSessionState): string {
    if (state === 'exited') return 'finished';
    return state;
  }

  function providerLabel(session: AgentSession): string {
    const provider = session.provider.toLowerCase();
    return provider.startsWith('cmux-') ? `cmux · ${provider.slice('cmux-'.length)}` : provider;
  }
</script>

<!-- One heading per project, drawn the same way for both lists. -->
{#snippet projectHead(name: string, path: string, count: number)}
  <div class="project-head" title={path || 'Sessions with no project folder'}>
    <span class="project-name">{name}</span>
    <span class="project-count">{count}</span>
  </div>
{/snippet}

<div class="rail">
  <div class="search">
    <input
      type="search"
      placeholder="Filter sessions"
      aria-label="Filter sessions"
      bind:value={query}
    />
  </div>

  <section class="group">
    <header class="group-head">
      <h2>Sessions</h2>
      <span class="count">{countIn(grouped.owned)}</span>
    </header>

    {#if owned.length === 0}
      <p class="empty">No sessions yet — resume one below.</p>
    {:else if grouped.owned.length === 0}
      <p class="empty">No session matches “{query.trim()}”.</p>
    {:else}
      {#each grouped.owned as group (group.path)}
        <div class="project">
          {@render projectHead(group.name, group.path, group.items.length)}
          <ul class="rows">
            {#each group.items as session (session.ownedId)}
              <li class="row" class:active={session.ownedId === activeOwnedId}>
                <button
                  type="button"
                  class="row-main"
                  onclick={() => onSelect(session.ownedId)}
                  title={session.cwd || session.title}
                >
                  <span class="dot" data-state={session.state} aria-hidden="true"></span>
                  <span class="row-text">
                    <span class="row-title">{session.title || session.ownedId.slice(0, 8)}</span>
                    <span class="row-meta">
                      <span class="badge">{agentLabel(session)}</span>
                      {#if session.state === 'exited'}
                        <span class="finished">{stateLabel(session.state)}</span>
                      {/if}
                    </span>
                  </span>
                </button>
                <button
                  type="button"
                  class="row-close"
                  aria-label={session.state === 'exited'
                    ? `dismiss ${session.title}`
                    : `close ${session.title}`}
                  title={session.state === 'exited' ? 'dismiss' : 'close'}
                  onclick={() => onClose(session.ownedId)}
                >
                  ✕
                </button>
              </li>
            {/each}
          </ul>
        </div>
      {/each}
    {/if}
  </section>

  <section class="group">
    <header class="group-head">
      <h2>Resume</h2>
      <button type="button" class="rescan" disabled={scanning} onclick={() => onRescan()}>
        {scanning ? 'scanning…' : 'rescan'}
      </button>
    </header>

    {#if resumable.length === 0}
      <p class="empty">{scanning ? 'Scanning for agent sessions…' : 'Nothing to resume.'}</p>
    {:else if grouped.available.length === 0}
      <p class="empty">Nothing to resume matches “{query.trim()}”.</p>
    {:else}
      {#each grouped.available as group (group.path)}
        <div class="project">
          {@render projectHead(group.name, group.path, group.items.length)}
          <ul class="rows">
            {#each group.items as session (`${session.provider}:${session.id}`)}
              <li class="row">
                <button
                  type="button"
                  class="row-main"
                  onclick={() => onAdopt(session)}
                  title={session.projectPath ?? session.title}
                >
                  <span class="dot" data-state="available" aria-hidden="true"></span>
                  <span class="row-text">
                    <span class="row-title">{session.title || session.id}</span>
                    <span class="row-meta">
                      <span class="badge">{providerLabel(session)}</span>
                      {#if session.lastActivity}
                        <span class="stamp">{session.lastActivity}</span>
                      {/if}
                    </span>
                  </span>
                </button>
              </li>
            {/each}
          </ul>
        </div>
      {/each}
    {/if}
  </section>
</div>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    gap: 18px;
    height: 100%;
    overflow-y: auto;
    padding: 12px 10px;
    background: #101014;
    color: #d8d8e0;
    font-size: 12px;
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
  }

  /* The search box stays put while the two lists below it scroll. The negative
     margins pull it out to the rail's own padding so its background covers the
     full width of the rows passing underneath; the negative bottom one gives
     back the 8px of cover, leaving the rail's 18px gap as the only space
     between the box and the first heading. */
  .search {
    position: sticky;
    top: -12px; /* cancels the rail's own top padding */
    z-index: 1;
    margin: -12px -10px -8px;
    padding: 12px 10px 8px;
    background: #101014;
  }

  input[type='search'] {
    width: 100%;
    border: 1px solid #22222c;
    border-radius: 5px;
    background: #17171d;
    color: #d8d8e0;
    font: inherit;
    font-size: 11px;
    padding: 4px 7px;
  }

  input[type='search']::placeholder {
    color: #4c4c5a;
  }

  .group-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 0 4px 6px;
  }

  .project + .project {
    margin-top: 10px;
  }

  /* Quieter than the section heading above it: a project is a divider inside a
     list, not a second heading competing with "Sessions". */
  .project-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 3px 8px 4px;
    color: #6d6d7d;
    font-size: 10px;
  }

  .project-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .project-count {
    flex: 0 0 auto;
    color: #4c4c5a;
  }

  h2 {
    margin: 0;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: #7b7b8c;
  }

  .count {
    font-size: 10px;
    color: #5d5d6b;
  }

  .rescan {
    border: 1px solid #2a2a34;
    border-radius: 5px;
    background: transparent;
    color: #9a9aad;
    font-size: 10px;
    padding: 2px 7px;
    cursor: pointer;
  }

  .rescan:hover:not(:disabled) {
    border-color: #3d3d4a;
    color: #d8d8e0;
  }

  .rescan:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .empty {
    margin: 0;
    padding: 4px 6px;
    color: #5d5d6b;
    font-size: 11px;
  }

  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .row {
    display: flex;
    align-items: stretch;
    gap: 2px;
    border-radius: 6px;
  }

  .row:hover {
    background: #1a1a22;
  }

  .row.active {
    background: #22222c;
  }

  .row-main {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 8px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .row-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .row-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #e6e6ee;
  }

  .row-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .badge {
    border-radius: 4px;
    background: #24242f;
    color: #9a9aad;
    font-size: 9px;
    letter-spacing: 0.04em;
    padding: 1px 5px;
    white-space: nowrap;
  }

  .finished,
  .stamp {
    color: #5d5d6b;
    font-size: 9px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dot {
    flex: 0 0 auto;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #4c4c5a;
  }

  /* live = accent pulse, background = solid, exited = hollow */
  .dot[data-state='live'] {
    background: #50fa7b;
    animation: pulse 1.9s ease-in-out infinite;
  }

  .dot[data-state='background'] {
    background: #8a8a9c;
  }

  .dot[data-state='exited'] {
    background: transparent;
    box-shadow: inset 0 0 0 1px #4c4c5a;
  }

  .dot[data-state='available'] {
    background: transparent;
    box-shadow: inset 0 0 0 1px #bd93f9;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.35;
    }
  }

  .row-close {
    flex: 0 0 auto;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: #5d5d6b;
    font-size: 11px;
    padding: 0 8px;
    cursor: pointer;
    opacity: 0;
  }

  .row:hover .row-close,
  .row-close:focus-visible {
    opacity: 1;
  }

  .row-close:hover {
    color: #ff5555;
  }

  button:focus-visible,
  input:focus-visible {
    outline: 1px solid #bd93f9;
    outline-offset: -1px;
  }

  @media (prefers-reduced-motion: reduce) {
    .dot[data-state='live'] {
      animation: none;
    }
  }
</style>
