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

  import FileCode2 from '@lucide/svelte/icons/file-code-2';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import MessageCircle from '@lucide/svelte/icons/message-circle';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';

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

  type RowPresence = 'working' | 'attention' | 'idle' | 'done' | 'failed';
  type SessionRecordExtras = OwnedSession & { hostname?: string | null; machine?: string | null };

  let {
    session,
    active = false,
    dragging = false,
    dropPosition = null,
    onSelect,
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
    deriveSessionPresence(
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
  const presenceDetail = $derived(suspended ? 'Idle — resumes on send' : presenceLabel);
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
    // The right-click menu is the thing being read while it is open. The card
    // would sit over it, and the pointer is inside the row the whole time it
    // is up, so this guard is what keeps it from coming straight back.
    if (menuOpen) return;
    const row = event.currentTarget;
    if (!(row instanceof HTMLElement)) return;
    clearCardTimer();
    cardTimer = setTimeout(() => {
      cardTimer = null;
      placeCard(row);
    }, 160);
  }

  /** Whether this row's right-click menu is up. */
  let menuOpen = $state(false);

  /**
   * Opening the menu takes the card down and keeps it down. Both surfaces
   * answer the same row, and two of them at once is one too many — the menu is
   * the one that was asked for, so it wins. Closing takes it down as well: the
   * menu gives focus back to the row on the way out, and without this the card
   * would arrive as an answer to that, with the pointer somewhere else
   * entirely.
   */
  function menuOpenChanged(open: boolean): void {
    menuOpen = open;
    hideOverlay();
  }

  function hideOverlay(): void {
    clearCardTimer();
    cardPlacement = null;
    cardView = null;
  }

  /**
   * The card on keyboard focus, and only keyboard focus.
   *
   * `:focus-visible` is the browser's own answer to "did a person Tab here, or
   * did something hand focus back?" — and handing focus back is exactly what a
   * closing menu does. Asking the platform is what keeps that from looking
   * like someone arriving at the row.
   */
  function showOverlayFromFocus(event: FocusEvent): void {
    const focused = event.target;
    if (!(focused instanceof HTMLElement) || !focused.matches(':focus-visible')) return;
    showOverlay(event);
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
    } else if (action === 'open-source-control') {
      if (!sessionRowJump(session.ownedId, 'source-control')) onSelect?.();
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
  onmouseenter={showOverlay}
  onmouseleave={hideOverlay}
  onfocusin={showOverlayFromFocus}
  onfocusout={handleFocusOut}
>
  <ContextMenu.Root onOpenChange={menuOpenChanged}>
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
                {#if isWorking}
                  <span class="spinner" class:spinning aria-hidden="true"></span>
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
      {/snippet}
    </ContextMenu.Trigger>

    <ContextMenu.Content
      data-testid="worktree-agent-context-menu"
      class="w-[216px]"
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
          title={item.enabled ? undefined : item.disabledReason}
          onSelect={() => runMenuAction(item.id)}
        >{item.label}</ContextMenu.Item>
      {/each}
    </ContextMenu.Content>
  </ContextMenu.Root>

  <!-- The kit cluster is always in the page and only fades. It sits in line
       one's right corner, over the space the time vacates as the pointer
       arrives, so it lands on no text and nothing in the row moves: the lines
       keep their places and only the title's clip width changes. -->
  <HoverActions
    data-testid="worktree-agent-overlay"
    label="Session actions"
    class="absolute top-[5px] right-[17px] z-[2]"
  >
    <!-- Three, not four. The three surfaces a session is worked in; settling
         stays on the row's right-click menu, because every button here costs
         the title width while the pointer is on the row. -->
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

    <!-- A hole the width of the spinner, so the buttons stop to its left and a
         running session keeps saying so while the pointer is on the row. -->
    {#if isWorking}
      <span class="spinner-gap" aria-hidden="true"></span>
    {/if}

    <!-- The step back, last in the cluster and therefore over the time itself:
         the corner the time was using is the corner this move lands in. Done
         goes back to Settled, Settled goes back to Working — one rung a click,
         which is why a settled row clears both stamps rather than one. -->
    {#if shelf === 'done'}
      <span data-testid="worktree-agent-unsettle" class="contents">
        <HoverActionButton
          label="Move back to Settled"
          size="sm"
          onclick={(event) => {
            event.stopPropagation();
            onSettle?.();
          }}
        >
          <RotateCcw aria-hidden="true" />
        </HoverActionButton>
      </span>
    {:else if shelf === 'settled'}
      <span data-testid="worktree-agent-reopen" class="contents">
        <HoverActionButton
          label="Move back to Working"
          size="sm"
          onclick={(event) => {
            event.stopPropagation();
            onUnsettle?.();
            onReopen?.();
          }}
        >
          <RotateCcw aria-hidden="true" />
        </HoverActionButton>
      </span>
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
    /* How much room the action cluster needs: three buttons and the 4px gaps
       between them, measured at 81px. It is claimed only while the pointer is
       on the row, out of line one's right corner, which the time occupies the
       rest of the time. Nothing is held empty at rest — the title runs all the
       way to the time — and nothing moves when the buttons arrive, because the
       corner they land in is the one the time just left. A row that can step
       back carries a fourth button and reserves 28px more for it. */
    --rail-action-gutter: 81px;
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
    contain-intrinsic-size: auto 58px;
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

  .row:hover .thumb { opacity: 1; }

  /* TWO fixed line boxes, not three. A third line meant every line had to be
     small enough to fit, which is the whole reason the rail read badly; the
     branch and the worktree path say what they have to say on the hover card
     instead. The heights are declared rather than left to the font so the row
     is the same height on every machine and the mark can be sized against it. */
  .lines {
    display: flex;
    min-width: 0;
    flex-direction: column;
    justify-content: center;
    gap: 0;
  }

  /* Four states, one neutral scale, each step brighter than the last: rest is
     the card showing through, hover answers the pointer, selected sits above
     both because it persists, and a selected row under the pointer lifts once
     more so hovering it still says something. No stripe down the rail's edge
     and no accent tint — accent means "this session is working", and a row
     that happens to be the one on screen has not earned that signal. */
  .row:hover .session-row { background: var(--color-hover); }

  .active .session-row {
    background: color-mix(in srgb, var(--color-elevated) 88%, var(--color-text));
  }

  .row.active:hover .session-row {
    background: color-mix(in srgb, var(--color-elevated) 84%, var(--color-text));
  }

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

  /* Last activity, dim, in line one's right corner — the corner the buttons
     take over while the pointer is on the row. The slot holds its contents
     against its right edge, so the time fades out exactly where it stood and
     the buttons fade in over the same spot; only the space to its left grows,
     which is space the title was using and gives back. */
  .age {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: flex-end;
    padding-left: 8px;
    color: var(--color-text-3);
    font-family: var(--font-mono);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  /* The cluster's width plus the gap that keeps the title clear of it. The
     row's own :hover and :focus-within are what the action cluster answers to,
     so the slot and the buttons open and close together. */
  .row:hover .age,
  .row:focus-within .age { min-width: calc(var(--rail-action-gutter) + 8px); }

  /* One button wide, always: the spinner beside it must sit the same distance
     from the row's right edge whether the time reads "now" or "12m", because
     the cluster's gap for it is measured from that edge. */
  .age-text {
    min-width: 28px;
    text-align: right;
  }

  .row:hover .age-text,
  .row:focus-within .age-text { opacity: 0; }

  .row[data-presence='working'] .age { color: var(--color-text-2); }

  /* The working indicator, immediately left of the time it belongs to. The
     buttons step around it rather than over it — see the gutter above — so a
     session that is running says so whether or not the pointer is on the row. */
  .spinner {
    flex: 0 0 auto;
    box-sizing: border-box;
    margin-right: 6px;
    width: 12px;
    height: 12px;
    border: 2px solid color-mix(in srgb, var(--color-live) 30%, var(--color-surface));
    border-top-color: var(--color-live);
    border-radius: 50%;
    background: var(--color-surface);
  }

  /* The only looping motion in the rail, and it runs only while this row is
     working and on screen. */
  .spinner.spinning { animation: spin 900ms linear infinite; }

  /* Exactly the spinner's footprint: 12px and the 6px that separates it from
     the time. It paints nothing — it only stops the buttons here. */
  .spinner-gap {
    flex: 0 0 auto;
    width: 18px;
  }

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
    font-size: 14px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* A 16px glyph inside a 25px button: the disc that appears on hover needs
     the margin around the icon in order to read as a disc. */
  .row :global([data-slot='hover-actions'] svg) { width: 16px; height: 16px; }


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

  /* Interaction motion: every one of these ends. The row's fill and the time in
     its corner answer a pointer or a selection and then stop; only the working
     spinner loops, and only while a real turn is running on screen. */
  @media (prefers-reduced-motion: no-preference) {
    .age-text,
    .idle-label,
    .thumb { transition: opacity 120ms ease; }

    .session-row { transition: background-color 140ms ease; }
    .hover-popover { animation: card-in 160ms ease-out; }
  }

  @keyframes card-in {
    from { opacity: 0; transform: translateX(-4px); }
    to { opacity: 1; transform: none; }
  }
</style>
