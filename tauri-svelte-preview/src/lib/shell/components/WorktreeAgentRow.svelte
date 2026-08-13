<script lang="ts">
  /**
   * The compact, two-line session row used by every left-rail shelf.
   *
   * The row is intentionally presentational. Selection, restart, shelf
   * transitions and surface jumps arrive through props; hover only reveals
   * controls and a read-only detail card.
   */
  import { onDestroy } from 'svelte';

  import Bot from '@lucide/svelte/icons/bot';
  import Check from '@lucide/svelte/icons/check';
  import FileCode2 from '@lucide/svelte/icons/file-code-2';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import MessageCircle from '@lucide/svelte/icons/message-circle';
  import MoreHorizontal from '@lucide/svelte/icons/more-horizontal';
  import Play from '@lucide/svelte/icons/play';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Archive from '@lucide/svelte/icons/archive';
  import Sparkles from '@lucide/svelte/icons/sparkles';
  import Terminal from '@lucide/svelte/icons/terminal';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Undo2 from '@lucide/svelte/icons/undo-2';

  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { modelLabel } from '$lib/shell/conversation/agentConfigLabels.ts';
  import { conversationSessions } from '$lib/shell/conversation/conversationStore.svelte.ts';
  import {
    deriveSessionPresence,
    EMPTY_SESSION_PRESENCE_HISTORY,
    sessionPresenceHistory,
    synchronizeSessionPresenceWork
  } from '$lib/shell/conversation/sessionPresence.ts';
  import { requestSessionRestart } from '$lib/shell/conversation/sessionRestart.ts';
  import { presentAgentError } from '$lib/shell/errorPresentation';
  import { canonicalCwd, deriveOwnedLibraryState } from '$lib/shell/sessionLibrary/sessionLibraryModel';
  import {
    resolveOwnedSessionProject,
    type OwnedSession
  } from '$lib/shell/ownedSessions';
  import { sessionLabel } from '$lib/shell/sessionStrip';
  import SessionHoverCard from './SessionHoverCard.svelte';
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

  type RowPresence = 'working' | 'attention' | 'idle' | 'stopped' | 'done' | 'failed';
  type SessionRecordExtras = OwnedSession & { hostname?: string | null; machine?: string | null };

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
  const projectInfo = $derived(resolveOwnedSessionProject(session));
  const project = $derived(projectInfo.label);
  const worktree = $derived(canonicalCwd(session.cwd || session.projectPath) || projectInfo.path || project);
  const providerGlyph = $derived(session.viaCmux ? 'terminal' : 'agent');
  const conversation = $derived(
    session.state === 'exited' ? null : conversationSessions[session.ownedId] ?? null
  );
  const presentedError = $derived(session.lastError ? presentAgentError(session.lastError) : null);

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
  const suspended = $derived(conversation?.suspended === true);
  const presenceSignals = $derived(
    session.state === 'exited'
      ? 'stopped'
      : deriveSessionPresence(
          {
            terminalState: session.state,
            connectionState,
            suspended,
            activeTurnId,
            sending: conversation?.sending,
            pendingApprovalCount,
            runtimeState
          },
          $sessionPresenceHistory[session.ownedId] ?? EMPTY_SESSION_PRESENCE_HISTORY,
          0
        ).state
  );

  // Keep the shared history current without mounting the old wordy indicator.
  $effect(() => {
    if (session.state === 'exited') return;
    synchronizeSessionPresenceWork(
      session.ownedId,
      activeTurnId,
      conversation?.sending === true || runtimeState === 'working'
    );
  });

  const presence = $derived<RowPresence>(
    shelf === 'done'
      ? 'done'
      : session.lastError || runtimeState === 'failed'
        ? 'failed'
        : suspended
          ? 'idle'
          : session.state === 'exited' || session.executionOwner === 'stopped' || presenceSignals === 'disconnected'
          ? 'stopped'
          : pendingApprovalCount > 0
          || runtimeState === 'waiting-approval'
          || runtimeState === 'waiting-input'
          || presenceSignals === 'needs-attention'
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
  const presenceDetail = $derived(suspended ? 'Idle — resumes on click' : presenceLabel);
  const presenceIsRestart = $derived(presence === 'stopped');
  const modelValue = $derived(conversation?.metadata.model ?? session.model ?? null);
  const modelText = $derived(modelValue ? modelLabel(modelValue) : null);
  const machine = $derived(
    (session as SessionRecordExtras).hostname?.trim()
      || (session as SessionRecordExtras).machine?.trim()
      || null
  );
  const activity = $derived(formatActivity(session.lastActivity));
  const usage = $derived(formatUsage(conversation?.metadata.usedTokens, conversation?.usage));
  const metaProject = $derived(project);

  let cardPlacement = $state<{ top: number; left: number } | null>(null);
  let cardTimer: ReturnType<typeof setTimeout> | null = null;
  const CARD_WIDTH = 336;
  const CARD_HEIGHT = 300;

  function clearCardTimer(): void {
    if (cardTimer !== null) {
      clearTimeout(cardTimer);
      cardTimer = null;
    }
  }

  function placeCard(row: HTMLElement): void {
    const rect = row.getBoundingClientRect();
    const rightRoom = window.innerWidth - rect.right - 8;
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

  function showOverlay(event: {
    currentTarget: EventTarget | null;
  }): void {
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

  function stopPropagation(event: MouseEvent, action?: () => void): void {
    event.stopPropagation();
    action?.();
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
  data-testid="worktree-agent-row"
  data-presence={presence}
  class:active
  class="row"
  title={label}
  onmouseenter={showOverlay}
  onmouseleave={hideOverlay}
  onfocusin={showOverlay}
  onfocusout={handleFocusOut}
>
  <div class="row-body">
    <button
      data-testid="worktree-agent-select"
      type="button"
      class="row-select"
      aria-current={active ? 'true' : undefined}
      onclick={selectRow}
    >
      <span class="row-title-line">
        <span
          data-testid="worktree-agent-runtime"
          class="presence {presence}"
          class:suspended
          role="img"
          aria-label={presenceLabel}
          title={presenceDetail}
        >
          <span class="presence-dot" aria-hidden="true"></span>
        </span>
        {#if providerGlyph === 'terminal'}
          <Terminal data-testid="worktree-agent-provider-icon" class="provider-icon" aria-hidden="true" />
        {:else}
          <Bot data-testid="worktree-agent-provider-icon" class="provider-icon" aria-hidden="true" />
        {/if}
        <span data-testid="worktree-agent-title" class="row-title" title={label}>{label}</span>
      </span>

      <span data-testid="worktree-agent-meta-line" class="row-meta-line">
        <span data-testid="worktree-agent-meta" class="row-meta">
          <span>{metaProject}</span>
          {#if session.branch}
            <span class="meta-separator" aria-hidden="true">·</span>
            <span class="branch">{session.branch}</span>
          {/if}
        </span>
        <span class="row-right-slot">
          {#if modelText}
            <span data-testid="worktree-agent-model" class="model-chip" title={modelValue ?? undefined}>
              <Sparkles class="model-mark" aria-hidden="true" />{modelText}
            </span>
          {/if}
          {#if activity}
            <span data-testid="worktree-agent-activity" class="row-time">{activity}</span>
          {/if}
        </span>
      </span>
    </button>

    <span data-testid="worktree-agent-overlay" class="row-overlay">
      {#if presenceIsRestart}
        <span data-testid="worktree-agent-start">
          <IconButton
            label="Start session"
            size="xs"
            side="bottom"
            class="action action-session"
            onclick={startSession}
          >
            <Play class="action-icon" aria-hidden="true" />
          </IconButton>
        </span>
      {/if}

      <span data-testid="worktree-agent-jump" class="row-cluster">
        <span data-testid="worktree-agent-jump-session">
          <IconButton
            label="Open session"
            size="xs"
            side="bottom"
            class="action action-session"
            onclick={(event) => jump(event, 'session')}
          >
            <MessageCircle class="action-icon" aria-hidden="true" />
          </IconButton>
        </span>
        <span data-testid="worktree-agent-jump-editor">
          <IconButton
            label="Open editor"
            size="xs"
            side="bottom"
            class="action action-editor"
            onclick={(event) => jump(event, 'editor')}
          >
            <FileCode2 class="action-icon" aria-hidden="true" />
          </IconButton>
        </span>
        <span data-testid="worktree-agent-jump-source-control">
          <IconButton
            label="Open source control"
            size="xs"
            side="bottom"
            class="action action-git"
            onclick={(event) => jump(event, 'source-control')}
          >
            <GitBranch class="action-icon" aria-hidden="true" />
          </IconButton>
        </span>
      </span>

      {#if onToggle}
        <span data-testid="worktree-agent-details">
          <IconButton
            label="Details — session actions"
            size="xs"
            side="bottom"
            class="action action-details"
            onclick={(event) => stopPropagation(event, onToggle)}
          >
            <MoreHorizontal class="action-icon" aria-hidden="true" />
          </IconButton>
        </span>
      {/if}
    </span>
  </div>

  {#if cardPlacement}
    <div
      use:bodyPortal
      data-testid="worktree-agent-hover-popover"
      class="hover-popover"
      role="tooltip"
      style="top: {cardPlacement.top}px; left: {cardPlacement.left}px"
    >
      <SessionHoverCard
        title={label}
        statusLabel={presenceLabel}
        statusDetail={presenceDetail}
        {project}
        {worktree}
        {machine}
        branch={session.branch}
        model={modelText}
        lastActivity={activity}
        {usage}
        statusTone={presence}
      />
    </div>
  {/if}

  {#if expanded}
    <div data-testid="worktree-agent-detail" class="detail-grid">
      <span>Project</span><span class="truncate" title={project}>{project}</span>
      <span>Worktree</span><span class="truncate" title={worktree}>{worktree}</span>
      {#if session.branch}<span>Branch</span><span class="truncate mono">{session.branch}</span>{/if}
      {#if session.taskId}<span>Task</span><span class="truncate">{session.taskId}</span>{/if}
      {#if session.pullRequest}<span>Pull request</span><span class="truncate">{session.pullRequest}</span>{/if}
      {#if session.nativeSessionId}<span>Session</span><span class="truncate mono">{session.nativeSessionId}</span>{/if}
      {#if session.latestTurnPreview}<span>Last turn</span><span class="truncate" title={session.latestTurnPreview}>{session.latestTurnPreview}</span>{/if}
      {#if presentedError}
        <span>Error</span><span class="error">{presentedError.summary}</span>
        {#if presentedError.detail}
          <span></span>
          <details data-testid="worktree-agent-error-detail" class="error-detail">
            <summary>Technical details</summary>
            <pre>{presentedError.detail}</pre>
          </details>
        {/if}
      {/if}
      <div data-testid="worktree-agent-actions" class="detail-actions">
        {#if shelf === 'working' && onComplete}
          <span data-testid="worktree-agent-mark-done">
            <IconButton label="Mark done" size="xs" side="bottom" class="secondary-action" onclick={(event) => stopPropagation(event, onComplete)}>
              <Check class="action-icon" aria-hidden="true" />
            </IconButton>
          </span>
        {:else if shelf === 'done' && onReopen}
          <span data-testid="worktree-agent-reopen">
            <IconButton label="Move back to Working" size="xs" side="bottom" class="secondary-action" onclick={(event) => stopPropagation(event, onReopen)}>
              <Undo2 class="action-icon" aria-hidden="true" />
            </IconButton>
          </span>
        {/if}
        {#if shelf === 'done' && onSettle}
          <span data-testid="worktree-agent-settle">
            <IconButton label="Move to Settled" size="xs" side="bottom" class="secondary-action" onclick={(event) => stopPropagation(event, onSettle)}>
              <Archive class="action-icon" aria-hidden="true" />
            </IconButton>
          </span>
        {:else if shelf === 'settled' && onUnsettle}
          <span data-testid="worktree-agent-unsettle">
            <IconButton label="Move back to Done" size="xs" side="bottom" class="secondary-action" onclick={(event) => stopPropagation(event, onUnsettle)}>
              <RotateCcw class="action-icon" aria-hidden="true" />
            </IconButton>
          </span>
        {/if}
        {#if shelf === 'done' && onAskRemove}
          <span data-testid="worktree-agent-remove">
            <IconButton label="Remove from sessions" size="xs" side="bottom" class="secondary-action" onclick={(event) => stopPropagation(event, onAskRemove)}>
              <Trash2 class="action-icon" aria-hidden="true" />
            </IconButton>
          </span>
        {/if}
        {#if onClose && session.state !== 'exited' && session.ptySessionId}
          <span data-testid="worktree-agent-close">
            <IconButton label="Close terminal" size="xs" side="bottom" class="secondary-action" onclick={(event) => stopPropagation(event, onClose)}>
              <Terminal class="action-icon" aria-hidden="true" />
            </IconButton>
          </span>
        {/if}
      </div>
    </div>
  {/if}
</li>

<style>
  .row {
    position: relative;
    display: block;
    box-sizing: border-box;
    min-width: 0;
    list-style: none;
    padding: var(--rail-row-content-inset);
    background: transparent;
    color: var(--color-text);
    font-size: 13px;
    line-height: 19.5px;
    content-visibility: auto;
    contain-intrinsic-size: auto 57px;
    contain-intrinsic-block-size: auto 39.1875px;
  }

  .row:not(:first-child) { box-shadow: inset 0 1px 0 color-mix(in srgb, var(--color-text) 4.5%, transparent); }

  .row:hover,
  .row:focus-within { background: var(--row-hover); }

  .active { background: var(--row-selected); }
  .active::before {
    position: absolute;
    inset: 0 auto 0 0;
    width: 2px;
    background: var(--color-selected-border);
    content: '';
  }
  .active:hover,
  .active:focus-within { background: var(--row-active); }

  .row-body {
    position: relative;
    min-width: 0;
    padding: 0;
  }

  .row-select {
    display: flex;
    width: 100%;
    min-width: 0;
    flex-direction: column;
    align-items: stretch;
    gap: 0;
    padding: 0;
    border: 0;
    border-radius: 4px;
    color: inherit;
    background: transparent;
    text-align: left;
    outline: none;
  }

  .row-select:focus-visible { box-shadow: 0 0 0 2px var(--color-focus); }

  .row-title-line,
  .row-meta-line {
    display: flex;
    min-width: 0;
    align-items: center;
  }

  .row-title-line { gap: 8px; }
  .row-meta-line { gap: 6px; margin: 4px 0 0 22px; line-height: 17.25px; }

  :global(.provider-icon) {
    width: 14px;
    height: 14px;
    flex: 0 0 auto;
    color: var(--secondary-label);
  }

  .presence {
    position: relative;
    display: inline-flex;
    width: 14px;
    height: 14px;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
  }

  .presence-dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--color-idle);
  }

  .presence.working .presence-dot { background: var(--color-accent); }
  .presence.working::after {
    position: absolute;
    inset: 0;
    border: 1.5px solid var(--color-accent);
    border-radius: 999px;
    content: '';
    opacity: 0.35;
  }
  .presence.attention .presence-dot { background: var(--color-attention); }
  .presence.attention::after {
    position: absolute;
    inset: 1px;
    border: 1.5px solid var(--color-attention);
    border-radius: 999px;
    content: '';
    opacity: 0.35;
  }
  .presence.idle .presence-dot { opacity: 0.85; }
  .presence.idle.suspended .presence-dot { opacity: 0.6; }
  .presence.stopped .presence-dot {
    width: 7px;
    height: 7px;
    border: 1.5px solid var(--color-text-3);
    background: transparent;
    opacity: 0.6;
  }
  .presence.done .presence-dot { background: var(--color-good); }
  .presence.failed .presence-dot { background: var(--color-bad); }

  .row-title {
    min-width: 0;
    flex: 1 1 auto;
    overflow: hidden;
    color: var(--color-text);
    font-size: 13px;
    line-height: 1.38;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .active .row-title { font-weight: 600; }

  .row-meta {
    display: block;
    min-width: 0;
    flex: 1 1 auto;
    gap: 5px;
    overflow: hidden;
    color: var(--secondary-label);
    font-size: 12px;
    line-height: 1.35;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-meta > span:first-child { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta-separator { flex: 0 0 auto; padding: 0 1px; opacity: 0.55; }
  .branch {
    overflow: hidden;
    flex: 0 1 auto;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .model-chip {
    display: inline-flex;
    height: 17px;
    flex: 0 0 auto;
    align-items: center;
    gap: 4px;
    max-width: 118px;
    padding: 0 6px;
    overflow: hidden;
    border-radius: 5px;
    color: var(--color-text-2);
    background: var(--color-elevated);
    font-size: calc(12px - 1px);
    font-weight: 550;
    letter-spacing: 0.01em;
    line-height: 16.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  :global(.model-mark) { width: 11px; height: 11px; flex: 0 0 auto; color: var(--color-text-3); }
  .row-time {
    color: var(--secondary-label);
    font-size: 11.5px;
    line-height: 17.25px;
    opacity: 0.85;
    white-space: nowrap;
  }

  .row-right-slot {
    display: inline-flex;
    width: 111px;
    min-width: 111px;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
    overflow: hidden;
    opacity: 1;
  }

  .row[data-presence='stopped'] :global(.provider-icon) {
    filter: grayscale(1);
    opacity: 0.4;
  }
  .row[data-presence='stopped'] .row-meta > span:first-child { opacity: 0.72; }
  .row[data-presence='stopped']:hover :global(.provider-icon),
  .row[data-presence='stopped']:focus-within :global(.provider-icon),
  .row[data-presence='stopped']:hover .row-meta > span:first-child,
  .row[data-presence='stopped']:focus-within .row-meta > span:first-child {
    filter: none;
    opacity: 1;
  }

  .row-overlay {
    position: absolute;
    z-index: 3;
    top: 50%;
    right: 4px;
    display: inline-flex;
    align-items: center;
    gap: 1px;
    padding: 2px;
    border-radius: 9px;
    background: var(--color-elevated);
    box-shadow: var(--shadow-sm), inset 0 0 0 1px color-mix(in srgb, var(--color-text) 6%, transparent);
    isolation: isolate;
    transform: translateY(-50%);
    opacity: 0;
    pointer-events: none;
  }
  .row-overlay::before {
    position: absolute;
    z-index: -1;
    inset: 0 auto 0 -32px;
    width: 32px;
    background: linear-gradient(to right, transparent, var(--color-elevated) 78%);
    content: '';
  }
  .row-cluster { display: inline-flex; align-items: center; gap: 1px; }
  .row:hover .row-right-slot,
  .row:focus-within .row-right-slot { opacity: 0; }
  .row:hover .row-overlay,
  .row:focus-within .row-overlay,
  .row-overlay:focus-within {
    opacity: 1;
    pointer-events: auto;
  }
  .action,
  .secondary-action { color: var(--color-text-2); }
  .row-overlay :global(button.action) {
    width: 26px;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: 7px;
  }
  .action :global(svg),
  .secondary-action :global(svg) { width: 16px; height: 16px; }
  .action:hover { color: var(--color-text); background: color-mix(in srgb, var(--color-text) 10%, transparent); }
  .action-session:hover { color: var(--color-accent); background: color-mix(in srgb, var(--color-accent) 16%, transparent); }
  .action-editor:hover { color: var(--color-live); background: var(--color-live-bg); }
  .action-git:hover { color: var(--color-good); background: var(--color-good-bg); }
  .action-details:hover,
  .secondary-action:hover { color: var(--color-text); background: color-mix(in srgb, var(--color-text) 12%, transparent); }

  .hover-popover {
    position: fixed;
    z-index: 60;
    pointer-events: none;
  }

  .detail-grid {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 4px 10px;
    padding: 0 12px 10px 34px;
    color: var(--color-text-2);
    font-size: 12px;
  }
  .detail-grid > span:nth-child(odd) { color: var(--color-text-3); }
  .detail-actions { display: flex; grid-column: 1 / -1; gap: 2px; padding-top: 4px; }
  .truncate { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 11.5px; }
  .error { color: var(--color-bad); }
  .error-detail summary { cursor: pointer; color: var(--color-text-2); }
  .error-detail pre { margin: 4px 0 0; overflow-wrap: anywhere; white-space: pre-wrap; font: inherit; }

  @media (prefers-reduced-motion: no-preference) {
    .row-right-slot,
    .row-overlay,
    :global(.provider-icon),
    .row-meta > span:first-child { transition: opacity 120ms ease; }
  }
</style>
