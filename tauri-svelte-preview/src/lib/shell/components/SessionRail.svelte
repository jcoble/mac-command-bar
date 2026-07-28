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
   * Rows are arranged under one heading per project folder, the search box at
   * the top narrows both lists at once, and each heading opens and closes. All
   * of that is decided by `sessionGroups.ts`, which is pure and has its own
   * test; this file only draws what it returns. The search text is the rail's
   * own state and is deliberately NOT reported to the page — narrowing what you
   * can see costs nothing and loads nothing.
   *
   * Which headings the user has opened or closed is read from and written to
   * `localStorage` here, in the click handler that changed it. That is the one
   * thing this file touches outside its own props, and it is deliberately not
   * an `$effect`: nothing is watched, nothing reloads, and a storage that
   * refuses the write costs only the arrangement on the next launch.
   */
  import type { OwnedSession, OwnedSessionState } from '$lib/shell/ownedSessions';
  import { exactLocalTime, formatLastActivity } from '$lib/shell/relativeTime';
  import {
    groupSessions,
    groupToggleKey,
    isGroupExpanded,
    readGroupExpansion,
    rememberGroupToggle,
    RESUME_GROUP_ROW_CAP,
    sessionGroupPath,
    visibleGroupItems,
    writeGroupExpansion,
    type GroupExpansion,
    type SessionList
  } from '$lib/shell/sessionGroups';
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
  const searching = $derived(query.trim().length > 0);

  /** Headings the user has opened or closed by hand, restored from last time. */
  let remembered = $state<GroupExpansion>(
    typeof window === 'undefined' ? {} : readGroupExpansion(window.localStorage)
  );

  /** Headings whose "Show N more" has been clicked. Not remembered: every visit
   * starts short again, which is the point of the cap. */
  let expandedRows = $state<Record<string, boolean>>({});

  /** The project folder of the session on screen — the one heading the Resume
   * list opens on its own. Grouped the same way its rows are, so the two keys
   * are comparable. */
  const activeProjectPath = $derived.by(() => {
    const active = owned.find((session) => session.ownedId === activeOwnedId);
    if (!active) return null;
    return sessionGroupPath(active.projectPath ?? active.cwd);
  });

  function expanded(list: SessionList, path: string): boolean {
    return isGroupExpanded({ list, path, remembered, activeProjectPath, searching });
  }

  /** A click flips how the heading sits with no search running. While a search
   * is open every heading is open regardless, so toggling against what is on
   * screen would record the opposite of what the user meant. */
  function toggleGroup(list: SessionList, path: string) {
    const settled = (from: GroupExpansion) =>
      isGroupExpanded({ list, path, remembered: from, activeProjectPath, searching: false });

    remembered = rememberGroupToggle(
      remembered,
      groupToggleKey(list, path),
      !settled(remembered),
      settled({})
    );
    if (typeof window !== 'undefined') writeGroupExpansion(window.localStorage, remembered);
  }

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

  /** The clock is read per row rather than held in state on purpose: the rail
   * redraws on every scan, adopt and keystroke, so the stamps stay honest with
   * no timer running behind them. */
  function stamp(lastActivity: string | null | undefined): string {
    return formatLastActivity(lastActivity, new Date());
  }
</script>

<!-- One heading per project, drawn the same way for both lists. Clicking it
     opens or closes the rows underneath. -->
{#snippet projectHead(
  list: SessionList,
  name: string,
  parentProject: string | null,
  path: string,
  count: number
)}
  <button
    type="button"
    class="project-head"
    aria-expanded={expanded(list, path)}
    title={path || 'Sessions with no project folder'}
    onclick={() => toggleGroup(list, path)}
  >
    <span class="chevron" aria-hidden="true">{expanded(list, path) ? '▾' : '▸'}</span>
    <span class="project-name">{name}</span>
    {#if parentProject}
      <span class="project-parent">· {parentProject}</span>
    {/if}
    <span class="project-count">{count}</span>
  </button>
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
          {@render projectHead(
            'owned',
            group.name,
            group.parentProject,
            group.path,
            group.items.length
          )}
          {#if expanded('owned', group.path)}
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
          {/if}
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
        {@const visible = visibleGroupItems(
          group.items,
          RESUME_GROUP_ROW_CAP,
          expandedRows[group.path] === true
        )}
        <div class="project">
          {@render projectHead(
            'resume',
            group.name,
            group.parentProject,
            group.path,
            group.items.length
          )}
          {#if expanded('resume', group.path)}
            <ul class="rows">
              {#each visible.shown as session (`${session.provider}:${session.id}`)}
                {@const when = stamp(session.lastActivity)}
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
                        {#if when}
                          <span class="stamp" title={exactLocalTime(session.lastActivity)}>
                            {when}
                          </span>
                        {/if}
                      </span>
                    </span>
                  </button>
                </li>
              {/each}
              {#if visible.hiddenCount > 0}
                <li class="row more">
                  <button
                    type="button"
                    class="row-main more-main"
                    onclick={() => (expandedRows = { ...expandedRows, [group.path]: true })}
                  >
                    Show {visible.hiddenCount} more
                  </button>
                </li>
              {/if}
            </ul>
          {/if}
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
     list, not a second heading competing with "Sessions". It is a button
     because clicking it opens and closes the rows, but it must not look like
     one sitting among them. */
  .project-head {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px 4px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: #6d6d7d;
    font: inherit;
    font-size: 10px;
    text-align: left;
    cursor: pointer;
  }

  .project-head:hover {
    background: #17171d;
    color: #9a9aad;
  }

  .chevron {
    flex: 0 0 auto;
    width: 8px;
    color: #4c4c5a;
  }

  .project-head:hover .chevron {
    color: #9a9aad;
  }

  .project-name {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* A worktree is named for its task, so the repository it belongs to is said
     here — quieter than the name, and the first thing to be dropped when the
     rail is narrow. */
  .project-parent {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #4c4c5a;
  }

  /* Was 10px and unreadable. This is the size of the section count above it,
     which is what it is really a smaller version of. */
  .project-count {
    flex: 0 0 auto;
    margin-left: auto;
    padding-left: 8px;
    color: #5d5d6b;
    font-size: 11px;
  }

  /* The row that offers the rest of a long project. Reads as a row so it lands
     where the eye already is, but carries no dot and no badge. */
  .more-main {
    color: #7b7b8c;
    font-size: 11px;
    padding-left: 23px;
  }

  .row.more:hover .more-main {
    color: #d8d8e0;
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
    font-size: 11px;
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
