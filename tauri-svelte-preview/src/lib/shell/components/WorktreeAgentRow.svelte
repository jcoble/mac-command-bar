<script lang="ts">
  /**
   * One session in the rail: project and status, title and model, branch.
   *
   * The row is presentational. Selecting, jumping, and every menu action arrive
   * through props. Hover changes only the row highlight; secondary actions live
   * in the right-click menu instead of mounting buttons on every rail row.
   *
   * The working indicator is static and only shown while this row is genuinely
   * working and on screen; the elapsed clock is the rail's one shared interval
   * rather than a timer per row.
  */
  import FileCode2 from '@lucide/svelte/icons/file-code-2';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import MessageCircle from '@lucide/svelte/icons/message-circle';

  import { AGENT_ICONS, agentDisplayName } from '$lib/shell/agentIcons.ts';
  import WorkingSpinner from '$lib/shell/components/conversation/WorkingSpinner.svelte';
  import {
    deriveSessionPresence,
    EMPTY_SESSION_PRESENCE_HISTORY,
    sessionPresenceHistory
  } from '$lib/shell/conversation/sessionPresence.ts';
  import { presentAgentError } from '$lib/shell/errorPresentation';
  import { deriveOwnedLibraryState } from '$lib/shell/sessionLibrary/sessionLibraryModel';
  import {
    resolveOwnedSessionProject,
    type OwnedSession
  } from '$lib/shell/ownedSessions';
  import { sessionLabel } from '$lib/shell/sessionStrip';
  import {
    formatRailElapsed,
    railElapsedCadenceFor,
    watchRailElapsed
  } from './railElapsedTicker.ts';
  import { sessionRowJump } from './sessionRowJump';
  import { observeElementVisibility } from '$lib/shell/elementVisibility.ts';

  interface Props {
    session: OwnedSession;
    active?: boolean;
    dragging?: boolean;
    dropPosition?: 'before' | 'after' | null;
    onSelect?(): void;
    onContextMenu?(event: MouseEvent): void;
    onDragStart?(event: DragEvent): void;
    onDragOver?(event: DragEvent): void;
    onDrop?(event: DragEvent): void;
    onDragEnd?(event: DragEvent): void;
  }

  type RowPresence = 'working' | 'attention' | 'idle' | 'done' | 'failed';
  let {
    session,
    active = false,
    dragging = false,
    dropPosition = null,
    onSelect,
    onContextMenu,
    onDragStart,
    onDragOver,
    onDrop,
    onDragEnd
  }: Props = $props();

  const label = $derived(sessionLabel(session));
  const shelf = $derived(deriveOwnedLibraryState(session));
  const projectInfo = $derived(resolveOwnedSessionProject(session));
  const project = $derived(projectInfo.label);
  const presentedError = $derived(session.lastError ? presentAgentError(session.lastError) : null);

  const presenceHistory = $derived(
    $sessionPresenceHistory[session.ownedId] ?? EMPTY_SESSION_PRESENCE_HISTORY
  );
  // Presence is rail-record truth plus events received live. Loading a stored
  // transcript may populate `conversation`, but it must not repaint this row.
  const needsYou = $derived(
    session.pendingPermission === true
    || session.pendingInput === true
    || session.runtimeState === 'waiting-approval'
    || session.runtimeState === 'waiting-input'
  );
  const pendingApprovalCount = $derived(needsYou ? 1 : 0);
  const runtimeState = $derived(session.runtimeState);
  const activeTurnId = $derived(session.activeTurnId ?? presenceHistory.activeTurnId);
  const suspended = $derived(session.runtimeState === 'suspended');
  const presenceSignals = $derived(
    deriveSessionPresence(
      {
        terminalState: session.state,
        suspended,
        activeTurnId,
        sending: false,
        pendingApprovalCount,
        runtimeState
      },
      presenceHistory,
      0
    ).state
  );

  const presence = $derived<RowPresence>(
    shelf === 'done'
      ? 'done'
      : session.lastError || runtimeState === 'failed'
        ? 'failed'
        : suspended
          ? 'idle'
          : needsYou || presenceSignals === 'needs-attention'
            ? 'attention'
            : presenceSignals === 'working'
              ? 'working'
              : 'idle'
  );
  const presenceLabel = $derived(
    {
      working: 'Working',
      attention: 'Waiting on you',
      idle: 'Idle',
      done: 'Finished',
      failed: 'Error'
    }[presence]
  );
  const ProviderIcon = $derived(AGENT_ICONS[session.agent]);
  const providerName = $derived(agentDisplayName(session.agent, session.viaCmux));

  // ── The age, and the working indicator in the rail ─────────────────────────
  let rowElement = $state<HTMLLIElement | null>(null);
  let onScreen = $state(true);
  let nowMs = $state(Date.now());

  const isWorking = $derived(presence === 'working');
  /** Show the working indicator only for a row a person can actually see. */
  const showWorkingIndicator = $derived(isWorking && onScreen);

  /**
   * How old this session is, counted from when it started rather than from the
   * current turn — the number stays put when the agent stops working, which is
   * what makes it comparable between rows.
   */
  const startedAtMs = $derived(
    session.startedAtMs
      ?? (session.lastActivity && Number.isFinite(Date.parse(session.lastActivity))
        ? Date.parse(session.lastActivity)
        : null)
  );
  const ageMs = $derived(startedAtMs === null ? null : Math.max(0, nowMs - startedAtMs));
  const ageText = $derived(ageMs === null ? null : formatRailElapsed(ageMs));

  $effect(() => {
    if (!isWorking) {
      onScreen = true;
      return;
    }
    const element = rowElement;
    if (!element) return;
    return observeElementVisibility(element, (visible) => {
      onScreen = visible;
    });
  });

  const hasAge = $derived(startedAtMs !== null);
  /** Only two possible values, so this effect re-subscribes at the one-minute
   * mark rather than on every tick. */
  const cadence = $derived(railElapsedCadenceFor(ageMs ?? 0, isWorking));

  // Every row shares this one clock. Most rows ask for one update per minute;
  // only genuinely working or newly-created rows make it tick each second.
  $effect(() => {
    if (!hasAge) return;
    const wanted = cadence;
    nowMs = Date.now();
    return watchRailElapsed((tick) => {
      nowMs = tick;
    }, wanted);
  });

  function selectRow(event: MouseEvent): void {
    onSelect?.();
    if (event.detail > 0 && event.currentTarget instanceof HTMLButtonElement) {
      event.currentTarget.blur();
    }
  }

  function jump(event: MouseEvent, surface: 'session' | 'editor' | 'source-control'): void {
    event.stopPropagation();
    if (!sessionRowJump(session.ownedId, surface)) onSelect?.();
  }

</script>

<li
  bind:this={rowElement}
  data-testid="worktree-agent-row"
  data-presence={presence}
  data-shelf={shelf}
  class:active
  class:dragging
  class:drop-before={dropPosition === 'before'}
  class:drop-after={dropPosition === 'after'}
  class:needs-you-row={needsYou}
  class="row group"
  draggable="true"
  ondragstart={onDragStart}
  ondragover={onDragOver}
  ondrop={onDrop}
  ondragend={onDragEnd}
>
        <button
          data-testid="worktree-agent-select"
          type="button"
          class="session-row"
          aria-current={active ? 'true' : undefined}
          aria-label={`Open session: ${label}`}
          onclick={selectRow}
          oncontextmenu={onContextMenu}
        >
          <!-- The mark, at the height of the three lines beside it. It carries
               the provider and whether this session is working, and nothing
               else: no action is ever drawn on top of it. -->
          <span
            data-testid="worktree-agent-provider"
            class="thumb"
            data-agent={session.agent}
            role="img"
            aria-label={providerName}
          >
            <ProviderIcon class="thumb-mark" aria-hidden="true" />
          </span>

          <span class="lines">
            <!-- Line one is the title and a corner that always holds exactly one
                 thing: the last activity time at rest, the quick-jump buttons
                 while the pointer is on the row. The corner is in the same place
                 either way, so the buttons never land on the title — the title
                 simply clips a little earlier to make room for them. -->
            <span class="line line-title">
              <span data-testid="worktree-agent-title" class="session-title">{label}</span>
              <span data-testid="worktree-agent-age" class="age">
                {#if showWorkingIndicator}
                  <WorkingSpinner seed={session.activeTurnId ?? session.ownedId} size={12} />
                {/if}
                <span class="age-text">{ageText ?? ''}</span>
              </span>
            </span>

            <span class="line line-meta">
              <span data-testid="worktree-agent-meta" class="project">{project}</span>
              {#if session.branch}
                <span class="sep" aria-hidden="true">•</span>
                <span class="branch">{session.branch}</span>
              {/if}

              <!-- What the row still needs to say in words. The working state is
                   the mark's job, so only the states a colour cannot carry are
                   spelled out here. -->
              {#if needsYou}
                <span data-testid="worktree-agent-needs-you" class="needs-you">
                  <span class="attention-dot" aria-hidden="true"></span>
                  Needs you
                </span>
              {:else if presence === 'failed'}
                <span
                  data-testid="worktree-agent-status"
                  class="failed"
                >{presentedError?.summary ?? presenceLabel}</span>
              {:else if suspended}
                <span data-testid="worktree-agent-status" class="idle-label">
                  {suspended ? 'Suspended' : presenceLabel}
                </span>
              {/if}
            </span>
          </span>
        </button>

        <span class="row-actions" aria-label="Session shortcuts">
          <button data-slot="icon-button" type="button" title="Open session" aria-label="Open session" onclick={(event) => jump(event, 'session')}>
            <MessageCircle aria-hidden="true" />
          </button>
          <button data-slot="icon-button" type="button" title="Open editor" aria-label="Open editor" onclick={(event) => jump(event, 'editor')}>
            <FileCode2 aria-hidden="true" />
          </button>
          <button data-slot="icon-button" type="button" title="Open source control" aria-label="Open source control" onclick={(event) => jump(event, 'source-control')}>
            <GitBranch aria-hidden="true" />
          </button>
        </span>

</li>

<style>
  .row {
    position: relative;
    display: block;
    box-sizing: border-box;
    min-width: 0;
    /* The highlight block is inset from the rail's edges rather than bled to
       them, so a hovered row reads as a card in the list. */
    padding: 0 6px;
    list-style: none;
    color: var(--color-text);
    font-size: 13px;
    line-height: 1.4;
  }

  /* What else line one's corner is holding. A row that can step back carries a
     fourth button; a working row keeps an 18px hole so its spinner is never
     covered. A done row is never also working, so those two never both apply —
     except on a settled row, which can still be running. */
  .row[data-shelf='done'],
  .row[data-shelf='settled'] { --rail-action-gutter: 109px; }

  .row[data-presence='working'] { --rail-action-gutter: 99px; }
  .row[data-presence='working'][data-shelf='settled'] { --rail-action-gutter: 127px; }

  /* The mark, then the three lines. The row is 58px so a rail this narrow still
     shows a useful stack of sessions; the mark matches the height of the three
     lines beside it, which is the proportion the reference keeps. */
  .session-row {
    position: relative;
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr);
    column-gap: 10px;
    align-items: center;
    width: 100%;
    min-height: 58px;
    padding: var(--rail-row-content-inset);
    border: 0;
    /* No rule between rows: the gap and the rounded highlight carry the
       separation, which reads calmer than a stack of hairlines. */
    border-radius: var(--radius-sm);
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    /* Nothing inside may paint past the rounded corner. */
    overflow: hidden;
    outline: none;
  }

  /* The provider mark, at the height of the text beside it. Nothing is ever
     drawn over this square: it says who is running the session and whether the
     session is working, and those are the only two things it says. */
  .thumb {
    position: relative;
    display: flex;
    width: 44px;
    height: 44px;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--color-elevated) 68%, var(--color-surface));
    color: var(--color-text-2);
  }

  :global(.thumb-mark) {
    width: 22px;
    height: 22px;
    flex: 0 0 auto;
  }

  /* The two marks their vendors publish in a colour wear it here. The rest
     stay the theme's own, because inventing a brand colour for them would be
     a worse lie than a neutral glyph. */
  .thumb[data-agent='claude'] { color: var(--agent-mark-claude); }
  .thumb[data-agent='codex'] { color: var(--agent-mark-codex); }

  /* TWO fixed line boxes, not three. The heights are declared rather than left
     to the font so the row is the same height on every machine and the mark can
     be sized against it. */
  .lines {
    display: flex;
    min-width: 0;
    flex-direction: column;
    justify-content: center;
    gap: 0;
  }

  /* Selection is persistent state; hover is a local paint-only response. */
  .row:hover .session-row {
    background: var(--color-hover);
  }

  .active .session-row {
    background: color-mix(in srgb, var(--color-elevated) 88%, var(--color-text));
  }

  .row.active:hover .session-row {
    background: color-mix(in srgb, var(--color-elevated) 84%, var(--color-text));
  }

  .row-actions {
    position: absolute;
    top: 5px;
    right: 17px;
    z-index: 2;
    display: flex;
    gap: 3px;
    visibility: hidden;
  }

  .row:hover .row-actions,
  .row:focus-within .row-actions { visibility: visible; }

  .row:hover .line-title,
  .row:focus-within .line-title { padding-right: 82px; }

  .row:hover .age,
  .row:focus-within .age { visibility: hidden; }

  .row-actions button {
    display: grid;
    width: 25px;
    height: 25px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-pill);
    place-items: center;
    background: var(--color-elevated);
    color: var(--color-text-2);
    cursor: pointer;
  }

  .row-actions button:hover,
  .row-actions button:focus-visible {
    background: var(--color-hover);
    color: var(--color-text);
    outline: none;
  }

  .row-actions button:focus-visible { box-shadow: var(--focus-ring); }
  .row-actions :global(svg) { width: 15px; height: 15px; }

  .session-row:focus-visible { box-shadow: inset 0 0 0 2px var(--color-focus); }

  .line {
    position: relative;
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 5px;
  }

  .line-title {
    height: 21px;
    line-height: 21px;
  }

  .line-meta {
    height: 17px;
    line-height: 17px;
  }

  .project {
    min-width: 0;
    flex: 0 1 auto;
    overflow: hidden;
    color: var(--color-text-2);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* The branch, at the project's size but a step dimmer. Not monospaced: this
     line is read, not compared character by character, and a mono face at this
     size is both wider and harder to read in a rail this narrow. The project
     gives way first, because it repeats down the whole list and the branch is
     what tells one row from the next. */
  .sep {
    flex: 0 0 auto;
    color: var(--color-text-3);
    font-size: 12px;
  }

  .branch {
    min-width: 0;
    flex: 0 1 auto;
    overflow: hidden;
    color: var(--color-text-3);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .idle-label,
  .failed,
  .needs-you { margin-left: auto; }

  /* Last activity stays in line one's right corner while the row is hovered. */
  .age {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
    padding-left: 8px;
    color: var(--color-text-3);
    font-family: var(--font-mono);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  /* One button wide, always: the spinner beside it must sit the same distance
     from the row's right edge whether the time reads "now" or "12m", because
     the cluster's gap for it is measured from that edge. */
  .age-text {
    min-width: 28px;
    text-align: right;
  }

  .row[data-presence='working'] .age { color: var(--color-text-2); }

  .needs-you {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 5px;
    color: var(--color-attention);
    font-size: 11.5px;
    font-weight: 600;
    white-space: nowrap;
  }

  .attention-dot {
    width: 7px;
    height: 7px;
    flex: 0 0 auto;
    border-radius: 50%;
    background: var(--color-attention);
  }

  .failed {
    min-width: 0;
    flex: 0 1 auto;
    overflow: hidden;
    color: var(--color-bad);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .idle-label {
    flex: 0 0 auto;
    color: var(--color-idle);
    font-size: 11.5px;
    white-space: nowrap;
  }

  .session-title {
    min-width: 0;
    flex: 1 1 auto;
    overflow: hidden;
    color: var(--color-text);
    font-size: 14px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row.dragging { opacity: 0.48; }
  .row.drop-before::after,
  .row.drop-after::after {
    position: absolute;
    right: 6px;
    left: 6px;
    z-index: 5;
    height: 4px;
    border-radius: 2px;
    background: var(--color-accent);
    content: '';
    pointer-events: none;
  }
  .row.drop-before::after { top: -2px; }
  .row.drop-after::after { bottom: -2px; }

  /* Presence changes may fade their own small labels; pointer movement owns no
     transition or animation in a session row. */
  @media (prefers-reduced-motion: no-preference) {
    .age-text,
    .idle-label,
    .thumb { transition: opacity 120ms ease; }
  }
</style>
