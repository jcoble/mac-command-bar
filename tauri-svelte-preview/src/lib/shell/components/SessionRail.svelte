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
   * The sessions CommandBar owns are split into Working and Done, and which side
   * a session sits on is the USER's answer (`completedAt`), never the process's:
   * an agent that stopped running is still work in progress until it is marked
   * done, and a session marked done may still have a terminal running. That is
   * also why a row has two separate destructive-looking buttons — closing the
   * terminal ends the process and keeps the row, removing takes the row away —
   * and why removing is only offered once a session is done.
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
    type SessionGroup,
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
    /** End a session's terminal process. The session itself stays on the list. */
    onClose(ownedId: string): void;
    /** Move a session to Done. */
    onComplete(ownedId: string): void;
    /** Move a done session back to Working. */
    onReopen(ownedId: string): void;
    /** Take a session off the list for good. Only reachable from a done row. */
    onRemove(ownedId: string): void;
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
    onComplete,
    onReopen,
    onRemove,
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

  /** Sessions the user has not marked done yet. */
  const working = $derived(owned.filter((session) => session.completedAt === null));

  /** Sessions the user marked done, most recently finished first. `filter` has
   * already made a new array, so sorting it in place disturbs nothing. */
  const done = $derived(
    owned
      .filter((session) => session.completedAt !== null)
      .sort((a, b) => (b.completedAt ?? '').localeCompare(a.completedAt ?? ''))
  );

  // Grouped separately so a project heading appears under whichever subsection
  // actually has rows for it. The scanned list is grouped with the Working pass
  // only — there is one Resume section and it belongs to neither subsection.
  const grouped = $derived(groupSessions(working, resumable, query));
  const groupedDone = $derived(groupSessions(done, [], query));
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

  /** What a row is called in a button's label and in the confirmation. */
  function rowName(session: OwnedSession): string {
    return session.title || session.ownedId.slice(0, 8);
  }

  /** Removing is the one action nothing undoes, so it is asked about first. The
   * sentence says what survives, because the worry it answers is losing the
   * conversation rather than the row — and it says what does NOT survive when
   * that is true, because removing a session whose process is still going ends
   * it, and this is the only warning the user gets. */
  function confirmRemove(session: OwnedSession): void {
    const running = session.state !== 'exited';
    const question = running
      ? `Remove "${rowName(session)}" from your sessions? Its terminal is still running and will be closed. The transcript stays on disk.`
      : `Remove "${rowName(session)}" from your sessions? The transcript stays on disk.`;
    if (typeof window !== 'undefined' && !window.confirm(question)) return;
    onRemove(session.ownedId);
  }

  /** The branch, task and pull request the scanner worked out for a row, in the
   * order they answer "where is this work?": which branch, which task, which
   * pull request. Anything the scanner did not find is left out rather than
   * drawn empty — a chip is only worth its space when it says something. */
  function chips(
    values: { branch?: string | null; taskId?: string | null; pullRequest?: string | null }
  ): string[] {
    return [values.branch, values.taskId, values.pullRequest].filter(
      (value): value is string => typeof value === 'string' && value.length > 0
    );
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

<!-- The branch, task and pull request behind a row, drawn the same way for both
     lists. A long branch name is cut short on screen and kept whole in the
     tooltip, so a row stays one line however it was named. -->
{#snippet metaChips(values: {
  branch?: string | null;
  taskId?: string | null;
  pullRequest?: string | null;
})}
  <!-- Keyed by position, not by text: these are three fixed slots, and a branch
       named after its task puts the same word in two of them — a duplicate key
       there would throw and take the whole rail down. -->
  {#each chips(values) as chip, slot (slot)}
    <span class="badge chip" title={chip}>{chip}</span>
  {/each}
{/snippet}

<!-- "Working" / "Done". Reads like the section heading above it, set in from the
     edge so it is plainly a division WITHIN Sessions rather than a rival to it. -->
{#snippet subsectionHead(label: string, count: number)}
  <div class="subsection-head">
    <h3>{label}</h3>
    <span class="count">{count}</span>
  </div>
{/snippet}

<!-- The rows of one subsection, under one heading per project folder. `isDone`
     is the only difference between the two: it decides which pair of buttons a
     row offers. -->
{#snippet ownedProjects(groups: SessionGroup<OwnedSession>[], isDone: boolean)}
  {#each groups as group (group.path)}
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
                  <span class="row-title">{rowName(session)}</span>
                  <span class="row-meta">
                    <span class="badge">{agentLabel(session)}</span>
                    {@render metaChips(session)}
                    {#if session.state === 'exited'}
                      <span class="finished">{stateLabel(session.state)}</span>
                    {/if}
                  </span>
                </span>
              </button>
              {#if isDone}
                <button
                  type="button"
                  class="row-act"
                  aria-label={`reopen ${rowName(session)} — put it back under Working`}
                  title="Reopen"
                  onclick={() => onReopen(session.ownedId)}
                >
                  ↩
                </button>
                <button
                  type="button"
                  class="row-act danger"
                  aria-label={`remove ${rowName(session)} from this list`}
                  title="Remove from this list"
                  onclick={() => confirmRemove(session)}
                >
                  ✕
                </button>
              {:else}
                <button
                  type="button"
                  class="row-act"
                  aria-label={`mark ${rowName(session)} done`}
                  title="Mark done"
                  onclick={() => onComplete(session.ownedId)}
                >
                  ✓
                </button>
                <!-- Nothing left to close once the process has ended, and the
                     row says so with its finished badge. -->
                {#if session.state !== 'exited'}
                  <button
                    type="button"
                    class="row-act danger"
                    aria-label={`close the terminal for ${rowName(session)}`}
                    title="Close the terminal"
                    onclick={() => onClose(session.ownedId)}
                  >
                    ✕
                  </button>
                {/if}
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/each}
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
      <span class="count">{countIn(grouped.owned) + countIn(groupedDone.owned)}</span>
    </header>

    {#if owned.length === 0}
      <p class="empty">No sessions yet — resume one below.</p>
    {:else if grouped.owned.length === 0 && groupedDone.owned.length === 0}
      <p class="empty">No session matches “{query.trim()}”.</p>
    {:else}
      {#if grouped.owned.length > 0}
        {@render subsectionHead('Working', countIn(grouped.owned))}
        {@render ownedProjects(grouped.owned, false)}
      {/if}
      <!-- Nobody needs to be told they have finished nothing yet. -->
      {#if groupedDone.owned.length > 0}
        {@render subsectionHead('Done', countIn(groupedDone.owned))}
        {@render ownedProjects(groupedDone.owned, true)}
      {/if}
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
                        {@render metaChips({
                          branch: session.branchHint,
                          taskId: session.taskId,
                          pullRequest: session.pullRequestHint
                        })}
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

  h2,
  h3 {
    margin: 0;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: #7b7b8c;
  }

  /* Dimmer than "Sessions" and set in from the edge: Working and Done divide
     that section, they do not compete with it. */
  .subsection-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 0 4px 4px 8px;
  }

  .subsection-head h3 {
    color: #5d5d6b;
  }

  /* The gap a subsection heading needs above it once rows have already been
     drawn. The first one sits directly under "Sessions" and needs none. */
  .subsection-head:not(:first-of-type) {
    margin-top: 14px;
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
    /* More detail than a narrow rail can hold is cut off at the row's edge
       rather than widening it. */
    overflow: hidden;
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

  /* The branch, task and pull request. Same size and shape as the agent badge
     beside them — these are the things the user scans a rail for, so they are
     not allowed to shrink — and outlined rather than filled so the row still
     reads as one badge followed by its details. A long branch name gives way
     first and is cut with an ellipsis; the tooltip still has all of it. */
  .chip {
    flex: 0 1 auto;
    min-width: 0;
    border: 1px solid #33333f;
    background: transparent;
    color: #8a8a9c;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 24ch;
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

  /* The row's own buttons. Out of sight until the row is pointed at or a
     button is tabbed to, so a rail of twenty sessions is a list of titles
     rather than a wall of icons. */
  .row-act {
    flex: 0 0 auto;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: #5d5d6b;
    font-size: 11px;
    padding: 0 6px;
    cursor: pointer;
    opacity: 0;
  }

  .row:hover .row-act,
  .row-act:focus-visible {
    opacity: 1;
  }

  .row-act:hover {
    color: #d8d8e0;
  }

  /* Ending a process and dropping a row both cost something that is not coming
     back on its own, so both say so before they are clicked. */
  .row-act.danger:hover {
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
