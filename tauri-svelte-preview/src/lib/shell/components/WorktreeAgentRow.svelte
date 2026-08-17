<script lang="ts">
  /**
   * One session in the rail: project and status, title and model, branch.
   *
   * The row is presentational. Selecting, jumping, and every menu action arrive
   * through props. Hover only reveals controls that are already in the page and
   * a read-only detail card; nothing about hover moves the title or the branch.
   *
   * Motion is deliberate: the working indicator spins only while this row is
   * genuinely working AND on screen, and the elapsed clock is the rail's one
   * shared interval rather than a timer per row.
   */
  import { onDestroy } from 'svelte';

  import Archive from '@lucide/svelte/icons/archive';
  import ArchiveRestore from '@lucide/svelte/icons/archive-restore';
  import CornerDownLeft from '@lucide/svelte/icons/corner-down-left';
  import FileCode2 from '@lucide/svelte/icons/file-code-2';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import MessageCircle from '@lucide/svelte/icons/message-circle';

  import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
  import { HoverActionButton, HoverActions } from '$lib/components/ui/hover-actions/index.js';
  import { AGENT_ICONS, agentDisplayName } from '$lib/shell/agentIcons.ts';
  import { modelLabel } from '$lib/shell/conversation/agentConfigLabels.ts';
  import { conversationSessions } from '$lib/shell/conversation/conversationStore.svelte.ts';
  import {
    deriveSessionPresence,
    EMPTY_SESSION_PRESENCE_HISTORY,
    sessionPresenceHistory
  } from '$lib/shell/conversation/sessionPresence.ts';
  import { requestSessionRestart } from '$lib/shell/conversation/sessionRestart.ts';
  import { presentAgentError } from '$lib/shell/errorPresentation';
  import { canonicalCwd, deriveOwnedLibraryState } from '$lib/shell/sessionLibrary/sessionLibraryModel';
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
  import { observeElementVisibility } from '$lib/shell/elementVisibility.ts';
  import { sessionRowMenuItems, type SessionRowMenuAction } from './sessionRowMenu.ts';
  import SessionHoverCard from './SessionHoverCard.svelte';
  import { sessionRowJump } from './sessionRowJump';

  interface Props {
    session: OwnedSession;
    active?: boolean;
    dragging?: boolean;
    dropPosition?: 'before' | 'after' | null;
    onSelect?(): void;
    onRestart?(): void;
    onComplete?(): void;
    onReopen?(): void;
    onSettle?(): void;
    onUnsettle?(): void;
    onAskRemove?(): void;
    onDragStart?(event: DragEvent): void;
    onDragOver?(event: DragEvent): void;
    onDrop?(event: DragEvent): void;
    onDragEnd?(event: DragEvent): void;
  }

  type RowPresence = 'working' | 'attention' | 'idle' | 'stopped' | 'done' | 'failed';
  type SessionRecordExtras = OwnedSession & { hostname?: string | null; machine?: string | null };

  let {
    session,
    active = false,
    dragging = false,
    dropPosition = null,
    onSelect,
    onRestart,
    onComplete,
    onReopen,
    onSettle,
    onUnsettle,
    onAskRemove,
    onDragStart,
    onDragOver,
    onDrop,
    onDragEnd
  }: Props = $props();

  const label = $derived(sessionLabel(session));
  const shelf = $derived(deriveOwnedLibraryState(session));
  const projectInfo = $derived(resolveOwnedSessionProject(session));
  const project = $derived(projectInfo.label);
  const worktree = $derived(canonicalCwd(session.cwd || session.projectPath) || projectInfo.path || project);
  const conversation = $derived(
    session.state === 'exited' ? null : conversationSessions[session.ownedId] ?? null
  );
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
    session.state === 'exited'
      ? 'stopped'
      : deriveSessionPresence(
          {
            terminalState: session.state,
            suspended,
            activeTurnId,
            sending: conversation?.sending,
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
          : session.state === 'exited' || session.executionOwner === 'stopped' || presenceSignals === 'disconnected'
          ? 'stopped'
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
      stopped: 'Stopped',
      done: 'Finished',
      failed: 'Error'
    }[presence]
  );
  const presenceDetail = $derived(suspended ? 'Idle — resumes on send' : presenceLabel);
  const presenceIsRestart = $derived(presence === 'stopped');
  const modelValue = $derived(conversation?.metadata.model ?? session.model ?? null);
  const modelText = $derived(modelValue ? modelLabel(modelValue) : null);
  // The row marks the provider with its glyph; the name and the model belong to
  // the hover card, where there is room to read them.
  const ProviderIcon = $derived(AGENT_ICONS[session.agent]);
  const providerName = $derived(agentDisplayName(session.agent, session.viaCmux));
  const machine = $derived(
    (session as SessionRecordExtras).hostname?.trim()
      || (session as SessionRecordExtras).machine?.trim()
      || null
  );
  const activity = $derived(formatActivity(session.lastActivity));
  const usage = $derived(formatUsage(conversation?.metadata.usedTokens, conversation?.usage));

  // ── The age, and the one bit of motion in the rail ─────────────────────────
  let rowElement = $state<HTMLLIElement | null>(null);
  let onScreen = $state(true);
  let nowMs = $state(Date.now());

  const isWorking = $derived(presence === 'working');
  /** Spin only for a working row a person can actually see. */
  const spinning = $derived(isWorking && onScreen);

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

  // Diagnostic A/B (owner request): keep the snapshot popout on while the ticker stays isolated.
  const POPOUT_DIAG_DISABLED = false;
  const TICKER_DIAG_DISABLED = true;

  // A row off screen needs no clock at all; one on screen asks for seconds only
  // while it is working or still in its first minute, and minutes after that.
  $effect(() => {
    if (TICKER_DIAG_DISABLED || !onScreen || !hasAge) return;
    const wanted = cadence;
    nowMs = Date.now();
    return watchRailElapsed((tick) => {
      nowMs = tick;
    }, wanted);
  });

  // ── The read-only detail card ──────────────────────────────────────────────
  /**
   * Everything the card shows, read once when it opens.
   *
   * The card is a still picture of the row at the moment a person paused on it.
   * Reading the live conversation while it is open would repaint a floating
   * surface on every transcript event, so nothing here is a store read: the
   * snapshot is plain values, and the card renders only from them.
   */
  interface HoverCardView {
    title: string;
    statusLabel: string;
    statusDetail: string;
    project: string;
    worktree: string;
    machine: string | null;
    branch: string | null;
    provider: string;
    model: string | null;
    lastActivity: string | null;
    usage: string | null;
    error: string | null;
    statusTone: RowPresence;
  }

  let cardPlacement = $state<{ top: number; left: number } | null>(null);
  let cardView = $state<HoverCardView | null>(null);
  let cardTimer: ReturnType<typeof setTimeout> | null = null;
  const CARD_WIDTH = 336;
  const CARD_HEIGHT = 300;

  function clearCardTimer(): void {
    if (cardTimer !== null) {
      clearTimeout(cardTimer);
      cardTimer = null;
    }
  }

  /** Read the row's current values into plain data, once, on the way open. */
  function takeCardView(): HoverCardView {
    return {
      title: label,
      statusLabel: presenceLabel,
      statusDetail: presenceDetail,
      project,
      worktree,
      machine,
      branch: session.branch,
      provider: providerName,
      model: modelText,
      lastActivity: activity,
      usage,
      error: presentedError?.summary ?? null,
      statusTone: presence
    };
  }

  function placeCard(row: HTMLElement): void {
    const rect = row.getBoundingClientRect();
    const rightRoom = window.innerWidth - rect.right - 8;
    cardView = takeCardView();
    cardPlacement = {
      top: Math.max(8, Math.min(rect.top, window.innerHeight - CARD_HEIGHT - 8)),
      left: rightRoom >= CARD_WIDTH ? rect.right + 8 : Math.max(8, rect.left - CARD_WIDTH - 8)
    };
  }

  function bodyPortal(node: HTMLElement): { destroy(): void } {
    document.body.appendChild(node);
    return {
      destroy(): void {
        node.remove();
      }
    };
  }

  function showOverlay(event: { currentTarget: EventTarget | null }): void {
    if (POPOUT_DIAG_DISABLED) return;
    const row = event.currentTarget;
    if (!(row instanceof HTMLElement)) return;
    clearCardTimer();
    cardTimer = setTimeout(() => {
      cardTimer = null;
      placeCard(row);
    }, 160);
  }

  function hideOverlay(): void {
    clearCardTimer();
    cardPlacement = null;
    cardView = null;
  }

  function handleFocusOut(event: FocusEvent): void {
    const next = event.relatedTarget;
    if (
      next instanceof Node
      && event.currentTarget instanceof Node
      && event.currentTarget.contains(next)
    ) return;
    hideOverlay();
  }

  function selectRow(event: MouseEvent): void {
    onSelect?.();
    if (event.detail > 0 && event.currentTarget instanceof HTMLButtonElement) {
      event.currentTarget.blur();
    }
  }

  function startSession(event: MouseEvent): void {
    event.stopPropagation();
    const handled = requestSessionRestart(session.ownedId, session.state === 'exited');
    if (handled) return;
    (session.state === 'exited' ? onRestart : onSelect)?.();
  }

  function jump(event: MouseEvent, surface: 'session' | 'editor' | 'source-control'): void {
    event.stopPropagation();
    if (!sessionRowJump(session.ownedId, surface)) onSelect?.();
  }

  onDestroy(clearCardTimer);

  // ── The right-click menu ───────────────────────────────────────────────────
  const sessionIdForCopy = $derived(session.nativeSessionId || session.ownedId);
  const menuItems = $derived(
    sessionRowMenuItems({
      status: shelf,
      sessionId: sessionIdForCopy,
      worktreePath: worktree || null
    })
  );

  function copyText(value: string | null): void {
    if (!value || typeof navigator === 'undefined' || !navigator.clipboard?.writeText) return;
    void navigator.clipboard.writeText(value);
  }

  function runMenuAction(action: SessionRowMenuAction): void {
    hideOverlay();
    if (action === 'mark-done') onComplete?.();
    else if (action === 'reopen') onReopen?.();
    else if (action === 'archive') onSettle?.();
    else if (action === 'unsettle') onUnsettle?.();
    else if (action === 'copy-session-id') copyText(sessionIdForCopy);
    else if (action === 'copy-worktree-path') copyText(worktree || null);
    else if (action === 'open-in-editor') {
      if (!sessionRowJump(session.ownedId, 'editor')) onSelect?.();
    } else if (action === 'delete') onAskRemove?.();
  }

  function formatActivity(value: string | null): string | null {
    if (!value) return null;
    const timestamp = Date.parse(value);
    if (!Number.isFinite(timestamp)) return value;
    const seconds = Math.max(0, Math.floor((Date.now() - timestamp) / 1000));
    if (seconds < 60) return 'now';
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h`;
    const days = Math.floor(hours / 24);
    return `${days}d`;
  }

  function formatUsage(
    metadataTokens: number | null | undefined,
    eventUsage: { inputTokens?: number; outputTokens?: number } | undefined
  ): string | null {
    const total = metadataTokens
      ?? ((eventUsage?.inputTokens ?? 0) + (eventUsage?.outputTokens ?? 0) || null);
    return total && total > 0 ? `${total.toLocaleString()} tokens` : null;
  }
</script>

<li
  bind:this={rowElement}
  data-testid="worktree-agent-row"
  data-presence={presence}
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
  onmouseenter={showOverlay}
  onmouseleave={hideOverlay}
  onfocusin={showOverlay}
  onfocusout={handleFocusOut}
>
  <ContextMenu.Root>
    <ContextMenu.Trigger>
      {#snippet child({ props })}
        <button
          {...props}
          data-testid="worktree-agent-select"
          type="button"
          class="session-row"
          aria-current={active ? 'true' : undefined}
          aria-label={`Open session: ${label}`}
          onclick={selectRow}
        >
          <!-- The mark, at the height of the three lines beside it. It carries
               the provider and whether this session is working, and nothing
               else: no action is ever drawn on top of it. -->
          <span
            data-testid="worktree-agent-provider"
            class="thumb"
            role="img"
            aria-label={providerName}
          >
            <ProviderIcon class="thumb-mark" aria-hidden="true" />
            {#if isWorking}
              <span class="spinner" class:spinning aria-hidden="true"></span>
            {/if}
          </span>

          <span class="lines">
            <!-- Line one is the title and only the title. The actions live in
                 the gutter to its right, which is reserved on every row whether
                 or not it is hovered, so this never shortens under the pointer. -->
            <span class="line">
              <span data-testid="worktree-agent-title" class="session-title">{label}</span>
            </span>

            <span class="line">
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
                  title={presentedError?.detail ?? presentedError?.summary}
                >{presentedError?.summary ?? presenceLabel}</span>
              {:else if suspended || presence === 'stopped'}
                <span data-testid="worktree-agent-status" class="idle-label" title={presenceDetail}>
                  {suspended ? 'Suspended' : presenceLabel}
                </span>
              {/if}
            </span>

            <!-- The path is cut from the LEFT: the tail is the part that says
                 which worktree this is. `direction: rtl` puts the ellipsis on
                 the near edge, and the `bdi` keeps the path itself reading
                 left to right inside it. -->
            <span class="line">
              <span class="worktree-path" title={worktree}><bdi>{worktree}</bdi></span>
              <span data-testid="worktree-agent-age" class="age" title={presenceDetail}>
                <span class="age-text">{ageText ?? ''}</span>
              </span>
            </span>
          </span>
        </button>
      {/snippet}
    </ContextMenu.Trigger>

    <ContextMenu.Content
      data-testid="worktree-agent-context-menu"
      class="w-[216px] bg-popover text-foreground"
      aria-label="Session actions"
    >
      {#each menuItems as item (item.id)}
        {#if item.startsGroup}
          <ContextMenu.Separator />
        {/if}
        <ContextMenu.Item
          data-testid={`session-row-menu-${item.id}`}
          disabled={!item.enabled}
          variant={item.destructive ? 'destructive' : 'default'}
          title={item.enabled ? item.label : item.disabledReason}
          onSelect={() => runMenuAction(item.id)}
        >{item.label}</ContextMenu.Item>
      {/each}
    </ContextMenu.Content>
  </ContextMenu.Root>

  <!-- The kit cluster is always in the page and only fades. It sits in the
       gutter every row reserves for it, level with the title, so revealing it
       covers no text and moves nothing: the row reads the same width whether
       the pointer is on it or three rows further down. -->
  <HoverActions
    data-testid="worktree-agent-overlay"
    label="Session actions"
    class="absolute top-[9px] right-[17px] z-[2]"
  >
    <!-- A stopped session's one extra move, at the head of the cluster so the
         four standing actions keep their places against the right edge. -->
    {#if presenceIsRestart}
      <span data-testid="worktree-agent-resume" class="contents">
        <HoverActionButton label="Resume session" tone="primary" size="sm" onclick={startSession}>
          <CornerDownLeft aria-hidden="true" />
        </HoverActionButton>
      </span>
    {/if}

    <span data-testid="worktree-agent-jump" class="contents">
      <span data-testid="worktree-agent-jump-session" class="contents">
        <HoverActionButton
          label="Open session"
          tone="primary"
          size="sm"
          onclick={(event) => jump(event, 'session')}
        >
          <MessageCircle aria-hidden="true" />
        </HoverActionButton>
      </span>
      <span data-testid="worktree-agent-jump-editor" class="contents">
        <HoverActionButton
          label="Open editor"
          tone="info"
          size="sm"
          onclick={(event) => jump(event, 'editor')}
        >
          <FileCode2 aria-hidden="true" />
        </HoverActionButton>
      </span>
      <span data-testid="worktree-agent-jump-source-control" class="contents">
        <HoverActionButton
          label="Open source control"
          tone="success"
          size="sm"
          onclick={(event) => jump(event, 'source-control')}
        >
          <GitBranch aria-hidden="true" />
        </HoverActionButton>
      </span>
    </span>

    <!-- Where the age sits at rest: the lifecycle move this session can make
         next, which is settling it, or putting a settled one back on Done. -->
    {#if shelf === 'settled'}
      <HoverActionButton
        data-testid="worktree-agent-unsettle"
        label="Revert to Done"
        tone="attention"
        size="sm"
        onclick={(event) => { event.stopPropagation(); onUnsettle?.(); }}
      >
        <ArchiveRestore aria-hidden="true" />
      </HoverActionButton>
    {:else}
      <HoverActionButton
        data-testid="worktree-agent-settle"
        label="Settle"
        tone="attention"
        size="sm"
        onclick={(event) => { event.stopPropagation(); onSettle?.(); }}
      >
        <Archive aria-hidden="true" />
      </HoverActionButton>
    {/if}
  </HoverActions>

  <!-- Nothing here reads a store: the card renders the snapshot taken when it
       opened, so a transcript event cannot repaint an open floating surface. -->
  {#if cardPlacement && cardView}
    <div
      use:bodyPortal
      data-testid="worktree-agent-hover-popover"
      class="hover-popover"
      role="tooltip"
      style="top: {cardPlacement.top}px; left: {cardPlacement.left}px"
    >
      <SessionHoverCard {...cardView} />
    </div>
  {/if}
</li>

<style>
  .row {
    /* The empty column every row keeps on its right for the action cluster:
       four 25px buttons and the 4px gaps between them. It is reserved whether
       or not the row is hovered, which is the whole point — the title and the
       two lines under it truncate to the same width at rest as they do with
       the buttons showing, so nothing shifts as the pointer runs down the
       list. A stopped row reserves one button more, and reserves it always. */
    --rail-action-gutter: 112px;
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
    content-visibility: auto;
    contain-intrinsic-size: auto 70px;
  }

  .row[data-presence='stopped'] { --rail-action-gutter: 141px; }

  /* The mark, then the three lines, then the reserved gutter — which is the
     row's own right padding, so every line inside stops short of it. */
  .session-row {
    position: relative;
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr);
    column-gap: 10px;
    align-items: center;
    width: 100%;
    min-height: 70px;
    padding: var(--rail-row-content-inset);
    padding-right: calc(var(--rail-action-gutter) + 11px);
    border: 0;
    /* No rule between rows: the gap and the rounded highlight carry the
       separation, which reads calmer than a stack of hairlines. */
    border-radius: var(--radius-sm);
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    /* Keeps the selected accent bar inside the rounded corner. */
    overflow: hidden;
    outline: none;
  }

  /* The provider mark, at the height of the text beside it. Nothing is ever
     drawn over this square: it says who is running the session and whether the
     session is working, and those are the only two things it says. */
  .thumb {
    position: relative;
    display: flex;
    width: 56px;
    height: 56px;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--color-elevated) 68%, var(--color-surface));
    color: var(--color-text-2);
  }

  :global(.thumb-mark) {
    width: 26px;
    height: 26px;
    flex: 0 0 auto;
  }

  .row[data-presence='stopped'] .thumb { opacity: 0.6; }
  .row:hover .thumb { opacity: 1; }

  .lines {
    display: flex;
    min-width: 0;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
  }

  .row:hover .session-row { background: var(--color-hover); }
  .session-row:focus-visible { box-shadow: inset 0 0 0 2px var(--color-focus); }

  .active .session-row { background: var(--color-selected); }

  /* The accent bar is always there and sweeps up from the row's top edge when
     the row becomes the selected one. Transform only — the row's box never
     changes, so nothing around it reflows. */
  .session-row::before {
    position: absolute;
    inset: 0 auto 0 0;
    width: 2px;
    background: var(--color-selected-border);
    content: '';
    transform: scaleY(0);
    transform-origin: top;
  }

  .active .session-row::before { transform: scaleY(1); }

  .line {
    position: relative;
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 5px;
  }

  .project {
    min-width: 0;
    flex: 0 1 auto;
    overflow: hidden;
    color: var(--color-text-2);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sep {
    flex: 0 0 auto;
    color: var(--color-text-3);
    font-size: 11.5px;
  }

  .idle-label,
  .failed,
  .needs-you { margin-left: auto; }

  /* The path, cut from the left. Everything about this is on the box rather
     than the string: `direction: rtl` moves the overflow — and so the ellipsis
     — to the near edge, `text-align: left` keeps a path that DOES fit sitting
     where the eye expects it, and the `bdi` in the markup stops the trailing
     slash of a directory being reordered to the wrong end. */
  .worktree-path {
    min-width: 0;
    flex: 1 1 auto;
    overflow: hidden;
    direction: rtl;
    color: var(--color-text-3);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Last activity, dim, at the end of the same line. It is a fixed corner of
     the row now rather than a slot that hover empties. */
  .age {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    padding-left: 6px;
    color: var(--color-text-3);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .row[data-presence='working'] .age { color: var(--color-text-2); }

  /* The working indicator: a small badge on the corner of the mark, which is
     the only place a session's own state is drawn. A disc rather than a ring
     around the whole square — a rotating rounded square reads as a wobble,
     while a circle is what a turning thing is supposed to look like. */
  .spinner {
    position: absolute;
    right: -4px;
    bottom: -4px;
    box-sizing: border-box;
    width: 15px;
    height: 15px;
    border: 2px solid color-mix(in srgb, var(--color-live) 30%, var(--color-surface));
    border-top-color: var(--color-live);
    border-radius: 50%;
    background: var(--color-surface);
  }

  /* The only looping motion in the rail, and it runs only while this row is
     working and on screen. */
  .spinner.spinning { animation: spin 900ms linear infinite; }

  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .spinner.spinning { animation: none; } }

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
    font-size: 13.5px;
    font-weight: 600;
    letter-spacing: -0.01em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .branch {
    min-width: 0;
    overflow: hidden;
    color: var(--color-text-3);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* A 16px glyph inside a 25px button: the disc that appears on hover needs
     the margin around the icon in order to read as a disc. */
  .row :global([data-slot='hover-actions'] svg) { width: 16px; height: 16px; }

  .row[data-presence='stopped'] .project { opacity: 0.72; }
  .row[data-presence='stopped']:hover .project { opacity: 1; }

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

  .hover-popover {
    position: fixed;
    z-index: 60;
    pointer-events: none;
  }

  /* Interaction motion: every one of these ends. The row's fill and its accent
     bar answer a pointer or a selection and then stop; only the working spinner
     loops, and only while a real turn is running on screen. */
  @media (prefers-reduced-motion: no-preference) {
    .idle-label,
    .thumb { transition: opacity 120ms ease; }

    .session-row { transition: background-color 140ms ease; }
    .session-row::before { transition: transform 180ms cubic-bezier(0.2, 0, 0, 1); }
    .hover-popover { animation: card-in 160ms ease-out; }
  }

  @keyframes card-in {
    from { opacity: 0; transform: translateX(-4px); }
    to { opacity: 1; transform: none; }
  }
</style>
