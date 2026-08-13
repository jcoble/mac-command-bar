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
  import Folder from '@lucide/svelte/icons/folder';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import Lock from '@lucide/svelte/icons/lock';
  import MessageCircle from '@lucide/svelte/icons/message-circle';
  import Play from '@lucide/svelte/icons/play';

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
  import { observeRailRowVisibility } from './railRowVisibility.ts';
  import { sessionRowMenuItems, type SessionRowMenuAction } from './sessionRowMenu.ts';
  import SessionHoverCard from './SessionHoverCard.svelte';
  import { sessionRowJump } from './sessionRowJump';

  interface Props {
    session: OwnedSession;
    active?: boolean;
    onSelect?(): void;
    onRestart?(): void;
    onComplete?(): void;
    onReopen?(): void;
    onSettle?(): void;
    onUnsettle?(): void;
    onAskRemove?(): void;
  }

  type RowPresence = 'working' | 'attention' | 'idle' | 'stopped' | 'done' | 'failed';
  type SessionRecordExtras = OwnedSession & { hostname?: string | null; machine?: string | null };

  let {
    session,
    active = false,
    onSelect,
    onRestart,
    onComplete,
    onReopen,
    onSettle,
    onUnsettle,
    onAskRemove
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
  const pendingApprovalCount = $derived(session.pendingPermission ? 1 : 0);
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
    return observeRailRowVisibility(element, (visible) => {
      onScreen = visible;
    });
  });

  const hasAge = $derived(startedAtMs !== null);
  /** Only two possible values, so this effect re-subscribes at the one-minute
   * mark rather than on every tick. */
  const cadence = $derived(railElapsedCadenceFor(ageMs ?? 0, isWorking));

  // A row off screen needs no clock at all; one on screen asks for seconds only
  // while it is working or still in its first minute, and minutes after that.
  $effect(() => {
    if (!onScreen || !hasAge) return;
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
  class="row group"
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
          <span class="line">
            <Folder class="glyph" aria-hidden="true" />
            <span data-testid="worktree-agent-meta" class="project">{project}</span>

            {#if presence === 'attention'}
              <span data-testid="worktree-agent-status" class="attention">
                <Lock aria-hidden="true" />Permission waiting
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

            <!-- The age stays put whatever the session is doing; while it works
                 the spinner joins it, and nothing else moves. -->
            <span
              data-testid="worktree-agent-age"
              class="status"
              class:working={isWorking}
              title={presenceDetail}
            >
              {#if isWorking}
                <span class="spinner" class:spinning aria-hidden="true"></span>
              {/if}
              {ageText ?? ''}
            </span>
          </span>

          <span class="line">
            <span data-testid="worktree-agent-title" class="session-title">{label}</span>
            <ProviderIcon
              data-testid="worktree-agent-provider"
              class="provider-icon"
              aria-label={providerName}
            />
          </span>

          <span class="line">
            <GitBranch class="glyph" aria-hidden="true" />
            <span class="branch">{session.branch ?? worktree}</span>
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

  <!-- The kit cluster: bare buttons over the metadata, always in the page, so
       revealing them never rebuilds a subtree or moves the title. -->
  <HoverActions
    data-testid="worktree-agent-overlay"
    label="Session actions"
    class="absolute top-[27px] right-[9px] z-[2]"
  >
    {#if presenceIsRestart}
      <span data-testid="worktree-agent-start" class="contents">
        <HoverActionButton label="Start session" tone="primary" onclick={startSession}>
          <Play aria-hidden="true" />
        </HoverActionButton>
      </span>
    {/if}

    <span data-testid="worktree-agent-jump" class="contents">
      <span data-testid="worktree-agent-jump-session" class="contents">
        <HoverActionButton
          label="Open session"
          tone="primary"
          onclick={(event) => jump(event, 'session')}
        >
          <MessageCircle aria-hidden="true" />
        </HoverActionButton>
      </span>
      <span data-testid="worktree-agent-jump-editor" class="contents">
        <HoverActionButton
          label="Open editor"
          tone="info"
          onclick={(event) => jump(event, 'editor')}
        >
          <FileCode2 aria-hidden="true" />
        </HoverActionButton>
      </span>
      <span data-testid="worktree-agent-jump-source-control" class="contents">
        <HoverActionButton
          label="Open source control"
          tone="success"
          onclick={(event) => jump(event, 'source-control')}
        >
          <GitBranch aria-hidden="true" />
        </HoverActionButton>
      </span>
    </span>
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
    position: relative;
    display: block;
    box-sizing: border-box;
    min-width: 0;
    list-style: none;
    color: var(--color-text);
    font-size: 13px;
    line-height: 1.4;
    content-visibility: auto;
    contain-intrinsic-size: auto 75px;
  }

  .session-row {
    position: relative;
    display: block;
    width: 100%;
    min-height: 75px;
    padding: var(--rail-row-content-inset);
    border: 0;
    border-bottom: 1px solid color-mix(in srgb, var(--color-text) 5.5%, transparent);
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    outline: none;
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
    gap: 6px;
  }

  .line + .line { margin-top: 4px; }

  .project {
    min-width: 0;
    overflow: hidden;
    color: var(--color-text-3);
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.glyph) {
    width: 14px;
    height: 14px;
    flex: 0 0 auto;
    color: var(--color-text-3);
  }

  .status,
  .attention,
  .idle-label,
  .failed { margin-left: auto; }

  .status {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 5px;
    color: var(--color-text-3);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  /* Working brings the age forward a tier; it never changes place. */
  .status.working { color: var(--color-text-2); }

  .spinner {
    width: 11px;
    height: 11px;
    border: 1.5px solid color-mix(in srgb, var(--color-live) 32%, transparent);
    border-top-color: var(--color-live);
    border-radius: 50%;
  }

  /* The only looping motion in the rail, and it runs only while this row is
     working and on screen. */
  .spinner.spinning { animation: spin 900ms linear infinite; }

  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .spinner.spinning { animation: none; } }

  .attention {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 5px;
    height: 22px;
    padding: 0 7px;
    border: 1px solid color-mix(in srgb, var(--color-attention) 30%, transparent);
    border-radius: var(--radius-sm);
    background: var(--color-attention-bg);
    color: var(--color-attention);
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
  }

  .attention :global(svg) { width: 12px; height: 12px; }

  .failed {
    min-width: 0;
    flex: 0 1 auto;
    overflow: hidden;
    color: var(--color-bad);
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .idle-label {
    flex: 0 0 auto;
    color: var(--color-idle);
    font-size: 13px;
    white-space: nowrap;
  }

  .session-title {
    min-width: 0;
    flex: 1 1 auto;
    overflow: hidden;
    color: var(--color-text);
    font-size: 13px;
    font-weight: 570;
    letter-spacing: -0.005em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.provider-icon) {
    width: 14px;
    height: 14px;
    flex: 0 0 auto;
    margin-left: auto;
    color: var(--color-text-3);
  }

  .branch {
    min-width: 0;
    overflow: hidden;
    color: var(--color-text-3);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* The cluster itself is the kit's; the row only says where it sits and which
     metadata steps aside for it — by fading, never by moving. */
  .row:hover .status,
  .row:hover .idle-label,
  .row:hover .attention,
  .row:hover :global(.provider-icon),
  .row:focus-within .status,
  .row:focus-within .idle-label,
  .row:focus-within .attention,
  .row:focus-within :global(.provider-icon) { opacity: 0; }

  .row :global([data-slot='hover-actions'] svg) { width: 14px; height: 14px; }

  .row[data-presence='stopped'] .project { opacity: 0.72; }
  .row[data-presence='stopped']:hover .project { opacity: 1; }

  .hover-popover {
    position: fixed;
    z-index: 60;
    pointer-events: none;
  }

  /* Interaction motion: every one of these ends. The row's fill and its accent
     bar answer a pointer or a selection and then stop; only the working spinner
     loops, and only while a real turn is running on screen. */
  @media (prefers-reduced-motion: no-preference) {
    .status,
    .idle-label,
    .attention,
    :global(.provider-icon) { transition: opacity 120ms ease; }

    .session-row { transition: background-color 140ms ease; }
    .session-row::before { transition: transform 180ms cubic-bezier(0.2, 0, 0, 1); }
    .hover-popover { animation: card-in 160ms ease-out; }
  }

  @keyframes card-in {
    from { opacity: 0; transform: translateX(-4px); }
    to { opacity: 1; transform: none; }
  }
</style>
