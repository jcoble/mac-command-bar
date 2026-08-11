<script lang="ts">
  /**
   * Compact left-rail row for one owned session. It is presentational: all
   * state and actions arrive through props, so hover/focus never starts IO.
   *
   * Two lines — title on top, meta underneath — with the presence marker
   * leading the title and the quick-jump buttons trailing it. The presence
   * marker is itself a button: it opens the session on the Session tab, which
   * is the one thing a working row is almost always clicked for. Editor and
   * source control sit beside it, so getting to the right surface never means a
   * click on the left edge followed by another on the far right.
   */
  import Bot from '@lucide/svelte/icons/bot';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Check from '@lucide/svelte/icons/check';
  import FileCode2 from '@lucide/svelte/icons/file-code-2';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Archive from '@lucide/svelte/icons/archive';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Undo2 from '@lucide/svelte/icons/undo-2';
  import Terminal from '@lucide/svelte/icons/terminal';

  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { presentAgentError } from '$lib/shell/errorPresentation';
  import { sessionLabel } from '$lib/shell/sessionStrip';
  import { canonicalCwd, deriveOwnedLibraryState } from '$lib/shell/sessionLibrary/sessionLibraryModel';
  import type { OwnedSession } from '$lib/shell/ownedSessions';
  import { conversationSessions } from '$lib/shell/conversation/conversationStore.svelte.ts';
  import {
    deriveSessionPresence,
    EMPTY_SESSION_PRESENCE_HISTORY,
    sessionPresenceHistory
  } from '$lib/shell/conversation/sessionPresence.ts';
  import SessionPresenceIndicator from './conversation/SessionPresenceIndicator.svelte';
  import { myWorkProject } from './myWorkViewOptions';
  import { sessionRowJump } from './sessionRowJump';

  interface Props {
    session: OwnedSession;
    active?: boolean;
    expanded?: boolean;
    onSelect?(): void;
    onToggle?(): void;
    onRestart?(): void;
    onComplete?(): void;
    onReopen?(): void;
    onSettle?(): void;
    onUnsettle?(): void;
    onClose?(): void;
    onAskRemove?(): void;
  }

  let {
    session,
    active = false,
    expanded = false,
    onSelect,
    onToggle,
    onRestart,
    onComplete,
    onReopen,
    onSettle,
    onUnsettle,
    onClose,
    onAskRemove
  }: Props = $props();

  const label = $derived(sessionLabel(session));
  const shelf = $derived(deriveOwnedLibraryState(session));
  const project = $derived(canonicalCwd(session.projectPath) || 'No project recorded');
  const location = $derived(canonicalCwd(session.cwd || session.projectPath) || 'No worktree recorded');
  const providerLabel = $derived(session.viaCmux ? `cmux · ${session.agent}` : session.agent);
  const conversation = $derived(conversationSessions[session.ownedId] ?? null);
  const presentedError = $derived(session.lastError ? presentAgentError(session.lastError) : null);

  /** The second line: where this session lives, in the fewest words. */
  const meta = $derived(
    [myWorkProject(session).label, session.branch, providerLabel].filter(Boolean).join(' · ')
  );

  const pendingApprovalCount = $derived(
    conversation
      ? Object.keys(conversation.pendingApprovals).length
        + conversation.timeline.filter((item) => item.kind === 'approval' && item.state === 'requested').length
      : 0
  );
  const runtimeState = $derived(
    conversation ? (session.runtimeState === 'starting' ? 'starting' : null) : session.runtimeState
  );
  const connectionState = $derived(session.origin === 'app' ? conversation?.connectionState ?? null : null);
  const activeTurnId = $derived(conversation?.activeTurnId ?? session.activeTurnId ?? null);

  /**
   * The same answer the indicator draws, worked out here for two decisions the
   * row owns: a disconnected session keeps the indicator's own restart button,
   * so the marker is only wrapped in a jump button when it is not one already;
   * and a row that is working or waiting keeps its jump buttons on screen
   * instead of hiding them until the pointer arrives.
   */
  const presence = $derived(
    deriveSessionPresence(
      {
        terminalState: session.state,
        connectionState,
        activeTurnId,
        sending: conversation?.sending,
        pendingApprovalCount,
        runtimeState
      },
      $sessionPresenceHistory[session.ownedId] ?? EMPTY_SESSION_PRESENCE_HISTORY,
      0
    ).state
  );
  const presenceIsRestart = $derived(presence === 'disconnected');
  const busy = $derived(presence === 'working' || presence === 'needs-attention');

  function stopPropagation(event: MouseEvent, action?: () => void): void {
    event.stopPropagation();
    action?.();
  }

  function jump(event: MouseEvent, surface: 'session' | 'editor' | 'source-control'): void {
    event.stopPropagation();
    // The rail is mounted by the page, which registers the one host that can do
    // both halves of a jump. A false answer means no host — the row click still
    // selects the session, so nothing is lost.
    if (!sessionRowJump(session.ownedId, surface)) onSelect?.();
  }

  /** Where the detail card sits while the pointer is on this row. */
  let cardPlacement = $state<{ top: number; left: number } | null>(null);
  const CARD_WIDTH = 320;
  const CARD_HEIGHT = 150;

  /**
   * The card is placed in window coordinates rather than inside the row.
   * The sessions column scrolls, which clips anything that reaches past its
   * own width, and this card is meant to leave the column entirely.
   */
  function placeCard(event: { currentTarget: EventTarget | null }): void {
    const row = event.currentTarget;
    if (!(row instanceof HTMLElement)) return;
    const rect = row.getBoundingClientRect();
    const roomOnTheRight = window.innerWidth - rect.right - 12;
    cardPlacement = {
      top: Math.max(8, Math.min(rect.top, window.innerHeight - CARD_HEIGHT - 8)),
      left:
        roomOnTheRight >= CARD_WIDTH
          ? rect.right + 6
          : Math.max(8, rect.left - CARD_WIDTH - 6)
    };
  }

  function hideCard(): void {
    cardPlacement = null;
  }
</script>

<li
  data-testid="worktree-agent-row"
  class:active
  class="row group relative min-w-0 list-none border-b border-[var(--color-border)]/35"
  title={`${location} · ${providerLabel}`}
  onmouseenter={placeCard}
  onmouseleave={hideCard}
  onfocusin={placeCard}
  onfocusout={hideCard}
>
  <div class="row-body">
    <span data-testid="worktree-agent-runtime" class="row-presence">
      {#if presenceIsRestart}
        <SessionPresenceIndicator
          ownedId={session.ownedId}
          terminalState={session.state}
          {connectionState}
          {activeTurnId}
          sending={conversation?.sending}
          {pendingApprovalCount}
          {runtimeState}
          onRestart={session.state === 'exited' ? onRestart : onSelect}
        />
      {:else}
        <span data-testid="worktree-agent-jump-session">
          <IconButton
            label="Open session"
            size="xs"
            side="bottom"
            class="text-[var(--color-text-2)]"
            onclick={(event) => jump(event, 'session')}
          >
            <SessionPresenceIndicator
              ownedId={session.ownedId}
              terminalState={session.state}
              {connectionState}
              {activeTurnId}
              sending={conversation?.sending}
              {pendingApprovalCount}
              {runtimeState}
            />
          </IconButton>
        </span>
      {/if}
    </span>

    <button
      data-testid="worktree-agent-select"
      type="button"
      class="row-select"
      aria-current={active ? 'true' : undefined}
      onclick={() => onSelect?.()}
    >
      <span class="row-title-line">
        {#if session.viaCmux}
          <Terminal data-testid="worktree-agent-provider-icon" class="size-3 shrink-0 text-[var(--color-text-2)]" aria-hidden="true" />
        {:else}
          <Bot data-testid="worktree-agent-provider-icon" class="size-3 shrink-0 text-[var(--color-text-2)]" aria-hidden="true" />
        {/if}
        <span data-testid="worktree-agent-title" class="row-title">{label}</span>
      </span>
      <span data-testid="worktree-agent-meta" class="row-meta">{meta}</span>
    </button>

    <!-- Every control the row offers, pinned to its right edge and lifted out
         of the flow. The title keeps the full width of the row until the
         pointer arrives, and then these slide over the end of it instead of
         reserving a permanent strip of empty space beside it. -->
    <span
      data-testid="worktree-agent-overlay"
      class="row-overlay"
      class:always-on={busy}
    >
      <span data-testid="worktree-agent-jump" class="row-cluster">
        <span data-testid="worktree-agent-jump-editor">
          <IconButton
            label="Open editor"
            size="xs"
            side="bottom"
            class="text-[var(--color-text-2)]"
            onclick={(event) => jump(event, 'editor')}
          >
            <FileCode2 class="size-3.5" aria-hidden="true" />
          </IconButton>
        </span>
        <span data-testid="worktree-agent-jump-source-control">
          <IconButton
            label="Open source control"
            size="xs"
            side="bottom"
            class="text-[var(--color-text-2)]"
            onclick={(event) => jump(event, 'source-control')}
          >
            <GitBranch class="size-3.5" aria-hidden="true" />
          </IconButton>
        </span>
      </span>

      <span data-testid="worktree-agent-actions" class="row-cluster">
        {#if shelf === 'working' && onComplete}
          <span data-testid="worktree-agent-mark-done">
            <IconButton
              label="Mark done"
              size="xs"
              side="bottom"
              class="text-[var(--color-text-2)]"
              onclick={(event) => stopPropagation(event, onComplete)}
            >
              <Check class="size-3.5" aria-hidden="true" />
            </IconButton>
          </span>
        {:else if shelf === 'done' && onReopen}
          <span data-testid="worktree-agent-reopen">
            <IconButton
              label="Move back to Working"
              size="xs"
              side="bottom"
              class="text-[var(--color-text-2)]"
              onclick={(event) => stopPropagation(event, onReopen)}
            >
              <Undo2 class="size-3.5" aria-hidden="true" />
            </IconButton>
          </span>
        {/if}
        {#if shelf === 'done' && onSettle}
          <span data-testid="worktree-agent-settle">
            <IconButton
              label="Move to Settled"
              size="xs"
              side="bottom"
              class="text-[var(--color-text-2)]"
              onclick={(event) => stopPropagation(event, onSettle)}
            >
              <Archive class="size-3.5" aria-hidden="true" />
            </IconButton>
          </span>
        {:else if shelf === 'settled' && onUnsettle}
          <span data-testid="worktree-agent-unsettle">
            <IconButton
              label="Move back to Done"
              size="xs"
              side="bottom"
              class="text-[var(--color-text-2)]"
              onclick={(event) => stopPropagation(event, onUnsettle)}
            >
              <RotateCcw class="size-3.5" aria-hidden="true" />
            </IconButton>
          </span>
        {/if}
        {#if shelf === 'done' && onAskRemove}
          <span data-testid="worktree-agent-remove">
            <IconButton
              label="Remove from sessions"
              size="xs"
              side="bottom"
              class="text-[var(--color-text-2)]"
              onclick={(event) => stopPropagation(event, onAskRemove)}
            >
              <Trash2 class="size-3.5" aria-hidden="true" />
            </IconButton>
          </span>
        {/if}
        {#if onClose && session.state !== 'exited'}
          <span data-testid="worktree-agent-close">
            <IconButton
              label="Close terminal"
              size="xs"
              side="bottom"
              class="text-[var(--color-text-2)]"
              onclick={(event) => stopPropagation(event, onClose)}
            >
              <Terminal class="size-3.5" aria-hidden="true" />
            </IconButton>
          </span>
        {/if}
        {#if onToggle}
          <span data-testid="worktree-agent-expand">
            <IconButton
              label={expanded ? 'Hide details' : 'Show details'}
              size="xs"
              side="bottom"
              class="text-[var(--color-text-2)]"
              onclick={(event) => stopPropagation(event, onToggle)}
            >
              <ChevronRight class={expanded ? 'size-3.5 rotate-90' : 'size-3.5'} aria-hidden="true" />
            </IconButton>
          </span>
        {/if}
      </span>
    </span>
  </div>

  {#if cardPlacement}
    <div
      data-testid="worktree-agent-hover-popover"
      class="hover-popover"
      role="tooltip"
      style="top: {cardPlacement.top}px; left: {cardPlacement.left}px"
    >
    <strong>{label}</strong>
    <span>{project} · {location}</span>
    <span>{providerLabel} · {shelf}</span>
    {#if session.branch || session.taskId || session.pullRequest}
      <span>{[session.branch, session.taskId, session.pullRequest].filter(Boolean).join(' · ')}</span>
    {/if}
    {#if presentedError}<span data-testid="worktree-agent-error" class="error">{presentedError.summary}</span>{/if}
    {#if session.lastActivity}<span>Last activity {session.lastActivity}</span>{/if}
    </div>
  {/if}

  {#if expanded}
    <div data-testid="worktree-agent-detail" class="detail-grid px-8 pb-2 text-[12px] text-[var(--color-text-2)]">
      <span>Worktree</span><span class="truncate" title={location}>{location}</span>
      <span>Project</span><span class="truncate" title={project}>{project}</span>
      {#if session.branch}<span>Branch</span><span class="truncate">{session.branch}</span>{/if}
      {#if session.taskId}<span>Task</span><span class="truncate">{session.taskId}</span>{/if}
      {#if session.pullRequest}<span>Pull request</span><span class="truncate">{session.pullRequest}</span>{/if}
      {#if session.nativeSessionId}<span>Native session</span><span class="truncate">{session.nativeSessionId}</span>{/if}
      {#if session.latestTurnPreview}<span>Last turn</span><span class="truncate" title={session.latestTurnPreview}>{session.latestTurnPreview}</span>{/if}
      {#if presentedError}
        <span>Error</span><span class="error">{presentedError.summary}</span>
        {#if presentedError.detail}
          <span></span>
          <details data-testid="worktree-agent-error-detail" class="error-detail min-w-0">
            <summary>Technical details</summary>
            <pre>{presentedError.detail}</pre>
          </details>
        {/if}
      {/if}
    </div>
  {/if}
</li>

<style>
  .row {
    position: relative;
    background: transparent;
  }

  .row:hover {
    background: var(--color-hover);
  }

  .active {
    background: var(--color-selected);
  }

  /* The selected row says so twice — a filled surface and a bar down its left
     edge — so it stays obvious against a hovered neighbour. */
  .active::before {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 2px;
    background: var(--color-selected-border);
    content: '';
  }

  .active:hover {
    background: color-mix(in srgb, var(--color-selected) 82%, var(--color-hover));
  }

  .row-body {
    display: flex;
    min-width: 0;
    align-items: flex-start;
    gap: 8px;
    padding: 8px;
  }

  .row-presence {
    display: inline-flex;
    min-height: 18px;
    flex-shrink: 0;
    align-items: center;
  }

  .row-select {
    display: flex;
    min-width: 0;
    flex: 1 1 auto;
    flex-direction: column;
    gap: 2px;
    border-radius: 4px;
    text-align: left;
    outline: none;
  }

  .row-select:focus-visible {
    box-shadow: 0 0 0 2px var(--color-focus);
  }

  .row-title-line {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 6px;
  }

  .row-title {
    min-width: 0;
    flex: 1 1 auto;
    overflow: hidden;
    color: var(--color-text);
    font-size: 13px;
    line-height: 1.35;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .active .row-title {
    font-weight: 600;
  }

  .row-meta {
    overflow: hidden;
    color: var(--color-text-3);
    font-size: 12px;
    line-height: 1.3;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-meta:empty {
    display: none;
  }

  /* The controls sit over the end of the title rather than beside it. The
     chip carries its own background so whatever is underneath stops at its
     edge, and the strip to its left fades that edge out instead of cutting
     the title with a hard line. */
  .row-overlay {
    position: absolute;
    z-index: 2;
    top: 4px;
    right: 4px;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    border-radius: 6px;
    padding: 0 2px;
    background: var(--color-elevated);
    box-shadow: var(--shadow-sm);
    opacity: 0;
    pointer-events: none;
  }

  .row-overlay::before {
    position: absolute;
    top: 0;
    bottom: 0;
    left: -20px;
    width: 20px;
    background: linear-gradient(to right, transparent, var(--color-elevated));
    content: '';
  }

  .row-cluster {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }

  /* A row that is doing something, or waiting on you, keeps its controls on
     screen — those are the rows you reach for without aiming first. */
  .row-overlay.always-on,
  .group:hover .row-overlay,
  .group:focus-within .row-overlay {
    opacity: 1;
    pointer-events: auto;
  }

  @media (prefers-reduced-motion: no-preference) {
    .row {
      transition: background 140ms ease;
    }

    .row-overlay {
      transition: opacity 140ms ease;
    }
  }

  .detail-grid {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 3px 10px;
  }

  .error-detail summary {
    cursor: pointer;
    color: var(--color-text-2);
  }

  .error-detail pre {
    margin: 4px 0 0;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
    color: var(--color-text-2);
    font: inherit;
  }

  /* Out to the right of the rail, not down over the row underneath: a card
     dropped below this row covers the next one, which is exactly the row you
     are usually comparing it against. */
  .hover-popover {
    position: fixed;
    z-index: 60;
    display: flex;
    width: 320px;
    max-width: calc(100vw - 24px);
    flex-direction: column;
    gap: 3px;
    border: 1px solid var(--color-border);
    border-radius: 5px;
    padding: 7px 8px;
    color: var(--color-text-2);
    background: var(--color-surface);
    box-shadow: var(--shadow-md);
    font-size: 12px;
    pointer-events: none;
  }

  .hover-popover strong { color: var(--color-text); font-size: 13px; font-weight: 600; }
  .hover-popover span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
