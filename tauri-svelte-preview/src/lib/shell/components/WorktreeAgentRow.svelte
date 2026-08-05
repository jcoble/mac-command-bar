<script lang="ts">
  /**
   * Compact left-rail row for one owned session. It is presentational: all
   * state and actions arrive through props, so hover/focus never starts IO.
   */
  import Bot from '@lucide/svelte/icons/bot';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Check from '@lucide/svelte/icons/check';
  import FolderGit2 from '@lucide/svelte/icons/folder-git-2';
  import Play from '@lucide/svelte/icons/play';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Archive from '@lucide/svelte/icons/archive';
  import Undo2 from '@lucide/svelte/icons/undo-2';
  import Terminal from '@lucide/svelte/icons/terminal';

  import { sessionLabel } from '$lib/shell/sessionStrip';
  import { canonicalCwd, deriveOwnedLibraryState } from '$lib/shell/sessionLibrary/sessionLibraryModel';
  import type { OwnedSession } from '$lib/shell/ownedSessions';

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
    onClose
  }: Props = $props();

  const label = $derived(sessionLabel(session));
  const shelf = $derived(deriveOwnedLibraryState(session));
  const project = $derived(canonicalCwd(session.projectPath) || 'No project recorded');
  const location = $derived(canonicalCwd(session.cwd || session.projectPath) || 'No worktree recorded');
  const providerLabel = $derived(session.viaCmux ? `cmux · ${session.agent}` : session.agent);
  const runtimeWord = $derived(
    session.state === 'exited' ? 'Stopped' : session.runtimeState === 'working' ? 'Working' : 'Ready'
  );

  function stopPropagation(event: MouseEvent, action?: () => void): void {
    event.stopPropagation();
    action?.();
  }
</script>

<li
  data-testid="worktree-agent-row"
  class:active
  class="group relative min-w-0 list-none border-b border-[var(--color-border)]/35"
  title={`${location} · ${providerLabel}`}
>
  <div class="relative flex min-h-[32px] w-full min-w-0 items-center gap-1 px-1 py-0.5">
    <button
      data-testid="worktree-agent-select"
      type="button"
      class="flex min-w-0 flex-1 items-center gap-1.5 rounded px-1.5 py-1 text-left outline-none
        hover:bg-[var(--color-elevated)] focus-visible:bg-[var(--color-elevated)] focus-visible:ring-2
        focus-visible:ring-[var(--color-focus)]"
      aria-current={active ? 'true' : undefined}
      onclick={() => onSelect?.()}
    >
      <span
        data-testid="worktree-agent-state-dot"
        class="state-dot shrink-0"
        data-state={shelf}
        aria-hidden="true"
      ></span>
      {#if session.viaCmux}
        <Terminal data-testid="worktree-agent-provider-icon" class="size-3 shrink-0 text-[var(--color-text-2)]" aria-hidden="true" />
      {:else}
        <Bot data-testid="worktree-agent-provider-icon" class="size-3 shrink-0 text-[var(--color-text-2)]" aria-hidden="true" />
      {/if}
      <span data-testid="worktree-agent-title" class="min-w-0 flex-1 truncate text-[13px] text-[var(--color-text)]">
        {label}
      </span>
      <span data-testid="worktree-agent-runtime" class="shrink-0 text-[12px] text-[var(--color-text-2)]">
        {shelf === 'settled' ? 'Settled' : shelf === 'done' ? 'Done' : runtimeWord}
      </span>
    </button>

    <span
      data-testid="worktree-agent-actions"
      class="row-actions flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity
        group-hover:opacity-100 group-focus-within:opacity-100"
    >
      {#if session.state === 'exited' && onRestart}
        <button
          data-testid="worktree-agent-restart"
          type="button"
          class="action-button"
          aria-label={`Start ${label} again`}
          title="Start this session again"
          onclick={(event) => stopPropagation(event, onRestart)}
        ><Play class="size-3" aria-hidden="true" /></button>
      {/if}
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
    </div>
  {/if}
</li>

<style>
  .active {
    background: color-mix(in srgb, var(--color-selected) 70%, transparent);
  }

  .state-dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: var(--color-idle);
  }

  .state-dot[data-state='working'] { background: var(--color-live); }
  .state-dot[data-state='done'] { background: var(--color-good); }
  .state-dot[data-state='settled'] { background: var(--color-text-3); }

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
    box-shadow: 0 5px 18px rgb(0 0 0 / 22%);
    font-size: 12px;
    pointer-events: none;
  }

  .hover-popover strong { color: var(--color-text); font-size: 13px; font-weight: 600; }
  .hover-popover span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .group:hover .hover-popover,
  .group:focus-within .hover-popover { display: flex; }
</style>
