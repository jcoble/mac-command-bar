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
</script>

<li
  data-testid="worktree-agent-row"
  class:active
  class="row group relative min-w-0 list-none border-b border-[var(--color-border)]/35"
  title={`${location} · ${providerLabel}`}
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
        <button
          data-testid="worktree-agent-jump-session"
          type="button"
          class="presence-jump"
          aria-label={`Open session ${label}`}
          title="Open session"
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
        </button>
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

    <span data-testid="worktree-agent-jump" class="row-jump" class:always-on={busy}>
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

    <span data-testid="worktree-agent-actions" class="row-actions">
      {#if shelf === 'working' && onComplete}
        <button
          data-testid="worktree-agent-mark-done"
          type="button"
          class="action-button"
          aria-label={`Mark ${label} done`}
          title="Mark done"
          onclick={(event) => stopPropagation(event, onComplete)}
        ><Check class="size-3" aria-hidden="true" /></button>
      {:else if shelf === 'done' && onReopen}
        <button
          data-testid="worktree-agent-reopen"
          type="button"
          class="action-button"
          aria-label={`Move ${label} back to Working`}
          title="Move back to Working"
          onclick={(event) => stopPropagation(event, onReopen)}
        ><Undo2 class="size-3" aria-hidden="true" /></button>
      {/if}
      {#if shelf === 'done' && onSettle}
        <button
          data-testid="worktree-agent-settle"
          type="button"
          class="action-button"
          aria-label={`Settle ${label}`}
          title="Move to Settled"
          onclick={(event) => stopPropagation(event, onSettle)}
        ><Archive class="size-3" aria-hidden="true" /></button>
      {:else if shelf === 'settled' && onUnsettle}
        <button
          data-testid="worktree-agent-unsettle"
          type="button"
          class="action-button"
          aria-label={`Unsettle ${label}`}
          title="Move back to Done"
          onclick={(event) => stopPropagation(event, onUnsettle)}
        ><RotateCcw class="size-3" aria-hidden="true" /></button>
      {/if}
      {#if shelf === 'done' && onAskRemove}
        <button
          data-testid="worktree-agent-remove"
          type="button"
          class="action-button"
          aria-label={`Remove ${label} from sessions`}
          title="Remove from sessions"
          onclick={(event) => stopPropagation(event, onAskRemove)}
        ><Trash2 class="size-3" aria-hidden="true" /></button>
      {/if}
      {#if onClose && session.state !== 'exited'}
        <button
          data-testid="worktree-agent-close"
          type="button"
          class="action-button"
          aria-label={`Close ${label} terminal`}
          title="Close terminal"
          onclick={(event) => stopPropagation(event, onClose)}
        ><Terminal class="size-3" aria-hidden="true" /></button>
      {/if}
    </span>

    {#if onToggle}
      <button
        data-testid="worktree-agent-expand"
        type="button"
        class="action-button shrink-0"
        aria-expanded={expanded}
        aria-label={expanded ? `Hide details for ${label}` : `Show details for ${label}`}
        title={expanded ? 'Hide details' : 'Show details'}
        onclick={(event) => stopPropagation(event, onToggle)}
      ><ChevronRight class={expanded ? 'size-3 rotate-90' : 'size-3'} aria-hidden="true" /></button>
    {/if}
  </div>

  <div data-testid="worktree-agent-hover-popover" class="hover-popover" role="tooltip">
    <strong>{label}</strong>
    <span>{project} · {location}</span>
    <span>{providerLabel} · {shelf}</span>
    {#if session.branch || session.taskId || session.pullRequest}
      <span>{[session.branch, session.taskId, session.pullRequest].filter(Boolean).join(' · ')}</span>
    {/if}
    {#if presentedError}<span data-testid="worktree-agent-error" class="error">{presentedError.summary}</span>{/if}
    {#if session.lastActivity}<span>Last activity {session.lastActivity}</span>{/if}
  </div>

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

  .presence-jump {
    display: inline-flex;
    align-items: center;
    border-radius: 4px;
    padding: 0 2px;
    outline: none;
  }

  .presence-jump:hover,
  .presence-jump:focus-visible {
    background: var(--color-elevated);
    box-shadow: 0 0 0 2px var(--color-focus);
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

  .row-jump,
  .row-actions {
    display: inline-flex;
    flex-shrink: 0;
    align-items: center;
    gap: 2px;
    opacity: 0;
  }

  /* A row that is doing something, or waiting on you, keeps its jumps on
     screen — those are the rows you reach for without aiming first. */
  .row-jump.always-on,
  .group:hover .row-jump,
  .group:hover .row-actions,
  .group:focus-within .row-jump,
  .group:focus-within .row-actions {
    opacity: 1;
  }

  @media (prefers-reduced-motion: no-preference) {
    .row {
      transition: background 140ms ease;
    }

    .row-jump,
    .row-actions {
      transition: opacity 140ms ease;
    }
  }

  .action-button {
    display: inline-flex;
    height: 24px;
    width: 24px;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--color-text-2);
    outline: none;
  }

  .action-button:hover,
  .action-button:focus-visible {
    background: var(--color-elevated);
    color: var(--color-text);
    box-shadow: 0 0 0 2px var(--color-focus);
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

  .hover-popover {
    position: absolute;
    z-index: 10;
    top: calc(100% - 2px);
    left: 8px;
    display: none;
    min-width: 220px;
    max-width: min(360px, calc(100vw - 24px));
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
  .group:hover .hover-popover,
  .group:focus-within .hover-popover { display: flex; }
</style>
