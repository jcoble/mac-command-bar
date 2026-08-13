<script lang="ts">
  /**
   * SessionCard.svelte — one session you own, on the sessions column.
   *
   * PRESENTATIONAL ONLY: no IO, no stores, no `$effect`. Every intent goes back
   * to the column through a callback, and every word it shows comes from
   * `sessionCardModel.ts`.
   *
   * The card is two things stacked:
   *
   *   THE ROW      what it is called, whether its terminal is running, what is
   *                running it, which branch and folder it lives in, how big the
   *                conversation is, and how long ago anything happened. Clicking
   *                it puts the session on screen.
   *
   *   THE DETAIL   opened by the chevron, closed by default. The same facts
   *                written out in full — the whole folder path, the exact times,
   *                the command that starts it back up, the last thing said with
   *                no truncation — and the buttons that DO something to the
   *                session, as words rather than a row of icons in the corner.
   *
   * The buttons live down there on purpose. They used to hover over the top
   * right of every card, which meant the title had to stop short of them, the
   * amount it stopped short by depended on which buttons the card had, and the
   * one truly destructive action sat a stray click away from the one that just
   * puts a session on screen.
   *
   * A card NEVER shrinks below its own content (`shrink-0`). The column it sits
   * in is a flex column, and without that a tall card being read squeezes every
   * other card into a sliver — which is the bug this card was rebuilt to fix.
   */
  import Check from '@lucide/svelte/icons/check';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Folder from '@lucide/svelte/icons/folder';
  import Play from '@lucide/svelte/icons/play';
  import Power from '@lucide/svelte/icons/power';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Undo2 from '@lucide/svelte/icons/undo-2';
  import type { Snippet } from 'svelte';

  import { Button } from '$lib/components/ui/button/index.js';
  import * as Card from '$lib/components/ui/card/index.js';
  import { presentAgentError } from '$lib/shell/errorPresentation';
  import { projectLabel } from '$lib/shell/sessionGroups';
  import { formatLastActivity, exactLocalTime } from '$lib/shell/relativeTime';
  import type { AgentKind, OwnedSession } from '$lib/shell/ownedSessions';
  import { sessionLabel } from '$lib/shell/sessionStrip';
  import { cn } from '$lib/utils';

  import {
    messageCountLabel,
    sessionFacts,
    sessionStatus,
    type SessionStatusTone
  } from './sessionCardModel';

  interface Props {
    session: OwnedSession;
    /** The session is on the Done list — the user's answer, not the process's. */
    isDone: boolean;
    /** This is the session showing in the main pane. */
    active: boolean;
    /** Its detail block is open. The column remembers which, so a redraw does
     * not close what someone was reading. */
    expanded: boolean;
    /** The agent badge and the branch / task / pull request chips, drawn by the
     * column so a card and a resume row wear exactly the same ones. */
    meta: Snippet<
      [AgentKind, boolean, { branch?: string | null; taskId?: string | null; pullRequest?: string | null }]
    >;
    onToggle(): void;
    onSelect(): void;
    onRestart(): void;
    onComplete(): void;
    onReopen(): void;
    onClose(): void;
    /** Ask the column's "remove this?" question. The card removes nothing. */
    onAskRemove(): void;
  }

  let {
    session,
    isDone,
    active,
    expanded,
    meta,
    onToggle,
    onSelect,
    onRestart,
    onComplete,
    onReopen,
    onClose,
    onAskRemove
  }: Props = $props();

  const label = $derived(sessionLabel(session));
  const status = $derived(sessionStatus(session));
  const project = $derived(projectLabel(session.projectPath ?? session.cwd));
  const messages = $derived(messageCountLabel(session.messageCount));

  /** The clock is read per redraw rather than held in state: the column redraws
   * on every scan, adopt and keystroke, so the stamps stay honest with no timer
   * running behind them. */
  const activityStamp = $derived(formatLastActivity(session.lastActivity, new Date()));
  const facts = $derived(sessionFacts(session, new Date()));
  const presentedError = $derived(session.lastError ? presentAgentError(session.lastError) : null);

  /** The status word wears the same colour as its dot. Tokens only — a hex here
   * would be the one colour in the shell a theme could not move. */
  const TONE_TEXT: Record<SessionStatusTone, string> = {
    running: 'text-[var(--color-live)]',
    done: 'text-[var(--color-good)]',
    stopped: 'text-[var(--color-text-2)]'
  };

  const DETAIL_BUTTON =
    'text-[12px] text-[var(--color-text-2)] hover:text-foreground hover:bg-[var(--color-elevated)]';
</script>

<Card.Root
  size="sm"
  class={cn(
    'relative shrink-0 rounded-md ring-[var(--color-border)] transition-colors [--card-spacing:0px]',
    'hover:bg-[var(--color-elevated)]',
    active && 'bg-[var(--color-elevated)] ring-primary/45'
  )}
>
  {#if active}
    <span class="absolute inset-y-0 left-0 w-[2px] bg-primary" aria-hidden="true"></span>
  {/if}

  <div class="flex w-full min-w-0 items-start">
    <button
      type="button"
      class="flex min-w-0 flex-1 flex-col items-start gap-1.5 px-2.5 py-2.5 text-left"
      title={session.cwd || label}
      onclick={onSelect}
    >
      <span class="flex w-full min-w-0 items-center gap-2">
        <span class="dot" data-tone={status.tone} data-attached={session.state === 'live'} aria-hidden="true"
        ></span>
        <span class="truncate text-[13px] leading-[1.4] font-medium text-[var(--color-text)]">
          {label}
        </span>
        <span class={cn('ml-auto shrink-0 text-[12px]', TONE_TEXT[status.tone])} title={status.hint}>
          {status.word}
        </span>
      </span>

      <span class="flex w-full min-w-0 flex-wrap items-center gap-1">
        {@render meta(session.agent, session.viaCmux, session)}
      </span>

      <!-- Where the work lives, how much of it there is, and when anything last
           happened. Every piece is left out when there is nothing to say — a
           session the scanner counted no turns for simply has a shorter card. -->
      <span
        class="flex w-full min-w-0 items-center gap-1.5 text-[12px] text-[var(--color-text-2)]"
      >
        <Folder class="size-3 shrink-0 text-[var(--color-text-2)]" aria-hidden="true" />
        <span class="truncate">
          {project.name}{project.parentProject ? ` · ${project.parentProject}` : ''}
        </span>
        {#if messages}
          <span class="shrink-0 text-[var(--color-text-3)]" aria-hidden="true">·</span>
          <span class="shrink-0">{messages}</span>
        {/if}
        {#if activityStamp}
          <span class="shrink-0 text-[var(--color-text-3)]" aria-hidden="true">·</span>
          <span class="shrink-0" title={exactLocalTime(session.lastActivity)}>{activityStamp}</span>
        {/if}
      </span>

      {#if session.latestTurnPreview}
        <span class="w-full min-w-0 truncate text-[13px] text-[var(--color-text-2)]">
          {session.latestTurnPreview}
        </span>
      {/if}
      {#if presentedError}
        <span
          data-testid="session-card-error"
          class="w-full min-w-0 text-[12px] leading-[1.4] break-words text-destructive"
          title={presentedError.detail ?? presentedError.summary}
        >
          {presentedError.summary}
        </span>
      {/if}
    </button>

    <!-- Always drawn, never on hover: a control that appears when you point at
         it is a control nobody knows is there. -->
    <button
      type="button"
      class="mt-2 mr-1.5 flex size-6 shrink-0 items-center justify-center rounded-md
             text-[var(--color-text-2)] transition-colors hover:bg-[var(--color-elevated)]
             hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 outline-none"
      aria-expanded={expanded}
      aria-label={expanded ? `hide the details for ${label}` : `show the details for ${label}`}
      title={expanded ? 'Hide the details' : 'Show the details and what you can do'}
      onclick={onToggle}
    >
      <ChevronRight
        class={cn('size-3.5 transition-transform', expanded && 'rotate-90')}
        aria-hidden="true"
      />
    </button>
  </div>

  {#if expanded}
    <div class="flex flex-col gap-2.5 border-t border-[var(--color-border)] px-2.5 py-2.5">
      <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1 text-[12px]">
        {#each facts as fact (fact.label)}
          <dt class="text-[var(--color-text-2)]">{fact.label}</dt>
          <dd class="min-w-0 break-words text-[var(--color-text)]" title={fact.exact}>
            {fact.value}
          </dd>
        {/each}
      </dl>

      {#if session.latestTurnPreview}
        <div class="flex flex-col gap-1">
          <span class="text-[12px] text-[var(--color-text-2)]">Last thing said</span>
          <p class="text-[13px] leading-[1.5] break-words whitespace-pre-wrap text-[var(--color-text)]">
            {session.latestTurnPreview}
          </p>
        </div>
      {/if}

      {#if presentedError?.detail}
        <details class="text-[12px] text-[var(--color-text-2)]">
          <summary class="cursor-pointer">Technical error details</summary>
          <pre
            data-testid="session-card-error-detail"
            class="mt-1 min-w-0 [overflow-wrap:anywhere] whitespace-pre-wrap font-[inherit]"
          >{presentedError.detail}</pre>
        </details>
      {/if}

      <div class="flex flex-wrap items-center gap-1">
        <!-- A finished session can be picked back up from either list, and it is
             the FIRST button because it is the one thing you cannot do to it
             otherwise. Starting a done session again does not move it off Done:
             which list a session is on is the user's answer, not the terminal's. -->
        {#if session.state === 'exited'}
          <Button size="xs" variant="ghost" class={DETAIL_BUTTON} onclick={onRestart}>
            <Play aria-hidden="true" />
            Start it again
          </Button>
        {/if}

        {#if isDone}
          <Button size="xs" variant="ghost" class={DETAIL_BUTTON} onclick={onReopen}>
            <Undo2 aria-hidden="true" />
            Move back to Working
          </Button>
          <Button
            size="xs"
            variant="ghost"
            class={cn(DETAIL_BUTTON, 'hover:text-destructive')}
            onclick={onAskRemove}
          >
            <Trash2 aria-hidden="true" />
            Remove from this list
          </Button>
        {:else}
          <Button size="xs" variant="ghost" class={DETAIL_BUTTON} onclick={onComplete}>
            <Check aria-hidden="true" />
            Mark done
          </Button>
          <!-- Nothing left to close once the process has ended, and the card's
               hollow dot already says so. -->
          {#if session.state !== 'exited'}
            <Button
              size="xs"
              variant="ghost"
              class={cn(DETAIL_BUTTON, 'hover:text-destructive')}
              onclick={onClose}
            >
              <Power aria-hidden="true" />
              Close the terminal
            </Button>
          {/if}
        {/if}
      </div>
    </div>
  {/if}
</Card.Root>

<style>
  /* The state dot: running is the live colour, and it pulses only while this
     app is the one attached to the terminal; done is the good colour; a
     terminal that has ended is hollow. */
  .dot {
    flex: 0 0 auto;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--color-text-3);
  }

  .dot[data-tone='running'] {
    background: var(--color-live);
  }

  .dot[data-tone='running'][data-attached='true'] {
    animation: pulse 1.9s ease-in-out infinite;
  }

  .dot[data-tone='done'] {
    background: var(--color-good);
  }

  .dot[data-tone='stopped'] {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--color-text-2);
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

  @media (prefers-reduced-motion: reduce) {
    .dot[data-tone='running'][data-attached='true'] {
      animation: none;
    }
  }
</style>
