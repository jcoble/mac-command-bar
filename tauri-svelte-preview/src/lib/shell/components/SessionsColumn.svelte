<script lang="ts">
  /**
   * SessionsColumn.svelte — the /next shell's left column. PRESENTATIONAL ONLY.
   *
   * No IO, no stores, no `$effect`: it draws two surfaces and reports every
   * intent through callbacks. The page owns all state and all backend calls.
   *
   *   MY WORK        the room. Every session CommandBar owns, as a card:
   *                  what it is called, what is running it, where it lives,
   *                  and what was last said in it. Split into Working and
   *                  Done, and which side a session sits on is the USER's
   *                  answer (`completedAt`), never the process's — an agent
   *                  that stopped running is still work in progress until it
   *                  is marked done, and a session marked done may still have
   *                  a terminal running. That is also why a card has two
   *                  separate destructive-looking buttons: closing the
   *                  terminal ends the process and keeps the card, removing
   *                  takes the card away — and why removing is only offered
   *                  once a session is done.
   *
   *   FIND A SESSION a drawer, closed until you want it. Everything on this
   *                  machine that could be resumed, under one heading per
   *                  project folder, with a search box over it. Closed by
   *                  default because it is a cupboard, not a room: several
   *                  hundred sessions can be in there and none of them is
   *                  what you are working on right now.
   *
   * The column also folds up into a strip of one cell per session, which is
   * the same list in the same order with everything but the icon taken away.
   * Folding is a WIDTH, so it goes through the frame (`onCollapse`) rather
   * than being faked with CSS: a column that still occupies 300px while
   * pretending to be 52 wide is a column the layout cannot reason about.
   *
   * What is grouped, what is capped, how long ago something was, and which
   * sessions are working versus done are all decided by pure modules with
   * their own tests (`sessionGroups.ts`, `relativeTime.ts`, `sessionStrip.ts`).
   * This file draws what they return.
   *
   * Which project headings the user has opened or closed is read from and
   * written to `localStorage` here, in the click handler that changed it. That
   * is the one thing this file touches outside its props, and it is
   * deliberately not an `$effect`: nothing is watched, nothing reloads, and a
   * storage that refuses the write costs only the arrangement next launch.
   */
  import Bot from '@lucide/svelte/icons/bot';
  import Check from '@lucide/svelte/icons/check';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Folder from '@lucide/svelte/icons/folder';
  import Gem from '@lucide/svelte/icons/gem';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import GitPullRequest from '@lucide/svelte/icons/git-pull-request';
  import Hash from '@lucide/svelte/icons/hash';
  import PanelLeftClose from '@lucide/svelte/icons/panel-left-close';
  import PanelLeftOpen from '@lucide/svelte/icons/panel-left-open';
  import Plus from '@lucide/svelte/icons/plus';
  import Play from '@lucide/svelte/icons/play';
  import Power from '@lucide/svelte/icons/power';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Search from '@lucide/svelte/icons/search';
  import SquareCode from '@lucide/svelte/icons/square-code';
  import SquareTerminal from '@lucide/svelte/icons/square-terminal';
  import Terminal from '@lucide/svelte/icons/terminal';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Undo2 from '@lucide/svelte/icons/undo-2';

  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import { Badge } from '$lib/components/ui/badge/index.js';
  import { buttonVariants } from '$lib/components/ui/button/index.js';
  import * as Card from '$lib/components/ui/card/index.js';
  import * as Collapsible from '$lib/components/ui/collapsible/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import { normalizeProvider, type AgentKind, type OwnedSession } from '$lib/shell/ownedSessions';
  import { exactLocalTime, formatLastActivity } from '$lib/shell/relativeTime';
  import {
    groupSessions,
    groupToggleKey,
    isGroupExpanded,
    projectLabel,
    readGroupExpansion,
    rememberGroupToggle,
    RESUME_GROUP_ROW_CAP,
    sessionGroupPath,
    visibleGroupItems,
    writeGroupExpansion,
    type GroupExpansion,
    type SessionList
  } from '$lib/shell/sessionGroups';
  import { sessionLabel, splitOwnedSessions, stripCells } from '$lib/shell/sessionStrip';
  import { cn } from '$lib/utils';
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
    /** The column is folded up to a strip. The PAGE owns this, because the
     * width it implies belongs to the frame. */
    collapsed: boolean;
    /** Focus an owned session. */
    onSelect(ownedId: string): void;
    /** Resume a scanned session (adopt + start a PTY). */
    onAdopt(session: AgentSession): void;
    /** End a session's terminal process. The session itself stays on the list. */
    onClose(ownedId: string): void;
    /** Give a finished session a new terminal, without changing anything else
     * about it. Offered on both lists: a done session can be picked back up
     * without being moved off Done first. */
    onRestart(ownedId: string): void;
    /** Move a session to Done. */
    onComplete(ownedId: string): void;
    /** Move a done session back to Working. */
    onReopen(ownedId: string): void;
    /** Take a session off the list for good. Only reachable from a done card. */
    onRemove(ownedId: string): void;
    /** Re-run the agent-session scan. */
    onRescan(): void;
    /** Open the new-session dialog. */
    onNewSession(): void;
    /** Fold the column up, or open it out again. */
    onCollapse(collapsed: boolean): void;
  }

  let {
    owned,
    available,
    activeOwnedId,
    scanning,
    collapsed,
    onSelect,
    onAdopt,
    onClose,
    onRestart,
    onComplete,
    onReopen,
    onRemove,
    onRescan,
    onNewSession,
    onCollapse
  }: Props = $props();

  /** Every provider id the column already owns, in both accepted forms — a
   * scanned session that has been adopted must not be offered again, and the
   * two adopt paths stored the id differently. */
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

  /** Your own sessions, in the two lists My work draws. */
  const split = $derived(splitOwnedSessions(owned));
  /** The same list again with everything but the icon taken away. */
  const cells = $derived(stripCells(owned, activeOwnedId));

  /** What the drawer's search box holds. Deliberately the column's own state
   * and NOT reported to the page: narrowing what you can see loads nothing. */
  let query = $state('');
  const searching = $derived(query.trim().length > 0);

  /** Is the drawer open? Closed to start: it is a cupboard, not a room. */
  let findOpen = $state(false);

  const grouped = $derived(groupSessions([], resumable, query));

  /** Headings the user has opened or closed by hand, restored from last time. */
  let remembered = $state<GroupExpansion>(
    typeof window === 'undefined' ? {} : readGroupExpansion(window.localStorage)
  );

  /** Headings whose "Show N more" has been clicked. Not remembered: every visit
   * starts short again, which is the point of the cap. */
  let expandedRows = $state<Record<string, boolean>>({});

  /** The project folder of the session on screen — the one heading in the
   * drawer that opens on its own. Grouped the same way its rows are, so the
   * two keys are comparable. */
  const activeProjectPath = $derived.by(() => {
    const active = owned.find((session) => session.ownedId === activeOwnedId);
    if (!active) return null;
    return sessionGroupPath(active.projectPath ?? active.cwd);
  });

  const RESUME_LIST: SessionList = 'resume';

  function expanded(path: string): boolean {
    return isGroupExpanded({ list: RESUME_LIST, path, remembered, activeProjectPath, searching });
  }

  /** A click flips how the heading sits with no search running. While a search
   * is open every heading is open regardless, so toggling against what is on
   * screen would record the opposite of what the user meant. */
  function toggleGroup(path: string) {
    const settled = (from: GroupExpansion) =>
      isGroupExpanded({
        list: RESUME_LIST,
        path,
        remembered: from,
        activeProjectPath,
        searching: false
      });

    remembered = rememberGroupToggle(
      remembered,
      groupToggleKey(RESUME_LIST, path),
      !settled(remembered),
      settled({})
    );
    if (typeof window !== 'undefined') writeGroupExpansion(window.localStorage, remembered);
  }

  /** The icon that stands for whatever is running a session. */
  const AGENT_ICONS: Record<AgentKind, typeof Bot> = {
    claude: Bot,
    codex: SquareCode,
    gemini: Gem,
    opencode: SquareTerminal,
    other: Terminal
  };

  /** `claude` / `codex` / `cmux · claude` when the session runs through cmux. */
  function agentLabel(agent: AgentKind, viaCmux: boolean): string {
    return viaCmux ? `cmux · ${agent}` : agent;
  }

  /** The branch, task and pull request the scanner worked out for a row, in the
   * order they answer "where is this work?". Anything the scanner did not find
   * is left out rather than drawn empty — a chip is only worth its space when
   * it says something. */
  function chips(values: {
    branch?: string | null;
    taskId?: string | null;
    pullRequest?: string | null;
  }): { icon: typeof Bot; text: string }[] {
    return [
      { icon: GitBranch, text: values.branch },
      { icon: Hash, text: values.taskId },
      { icon: GitPullRequest, text: values.pullRequest }
    ].filter((chip): chip is { icon: typeof Bot; text: string } => {
      return typeof chip.text === 'string' && chip.text.length > 0;
    });
  }

  /** "12 messages" — a plain count of the turns the scanner saw. It is a floor
   * rather than a total (the scanner reads a bounded window of a transcript),
   * which is why it is not dressed up as a total with a "+". */
  function messageCountLabel(count: number | null | undefined): string | null {
    if (typeof count !== 'number' || count <= 0) return null;
    return count === 1 ? '1 message' : `${count} messages`;
  }

  /** The clock is read per row rather than held in state on purpose: the column
   * redraws on every scan, adopt and keystroke, so the stamps stay honest with
   * no timer running behind them. */
  function stamp(when: string | null | undefined): string {
    return formatLastActivity(when, new Date());
  }

  /** The session the "remove this?" question is about, or null when nothing is
   * being asked. Only ever one: the question is a dialog, so a second one
   * cannot be opened behind it. */
  let removing = $state<OwnedSession | null>(null);
  let removeOpen = $state(false);

  function askAboutRemoving(session: OwnedSession): void {
    removing = session;
    removeOpen = true;
  }

  function removeNow(): void {
    const session = removing;
    removeOpen = false;
    removing = null;
    if (session) onRemove(session.ownedId);
  }

  /** Removing is the one action nothing undoes, so it is asked about first. The
   * sentence says what survives, because the worry it answers is losing the
   * conversation rather than the row — and it says what does NOT survive when
   * that is true, because removing a session whose process is still going ends
   * it, and this is the only warning the user gets. */
  function removeQuestion(session: OwnedSession): string {
    return session.state !== 'exited'
      ? 'Its terminal is still running and will be closed. The transcript stays on disk.'
      : 'The transcript stays on disk.';
  }

  /** Tooltips are the shell's own dark sheet rather than the library's inverted
   * white one, which on a near-black shell reads as a flashbulb. */
  const TOOLTIP_CLASS =
    'bg-[var(--color-surface)] text-foreground ring-1 ring-[var(--color-border)] ' +
    'shadow-[var(--shadow-md)] text-[12px] px-2 py-1';
  /** The arrow is part of the same sheet, so it is recoloured with it. */
  const TOOLTIP_ARROW_CLASS = 'bg-[var(--color-surface)] fill-[var(--color-surface)]';

  /** A card's own buttons: quiet until the card is pointed at, and never
   * bigger than the title beside them. */
  const ACTION_CLASS =
    'text-[var(--color-text-2)] hover:text-foreground hover:bg-[var(--color-elevated)]';
</script>

<!-- One icon button with a tooltip. `run` is called on click; the click never
     reaches the card underneath, which would select the session as a
     side-effect of closing its terminal. -->
{#snippet action(
  label: string,
  tip: string,
  Icon: typeof Bot,
  danger: boolean,
  run: () => void
)}
  <Tooltip.Root>
    <Tooltip.Trigger
      class={cn(
        buttonVariants({ variant: 'ghost', size: 'icon-xs' }),
        ACTION_CLASS,
        danger && 'hover:text-destructive'
      )}
      aria-label={label}
      onclick={(event: MouseEvent) => {
        event.stopPropagation();
        run();
      }}
    >
      <Icon aria-hidden="true" />
    </Tooltip.Trigger>
    <Tooltip.Content side="top" class={TOOLTIP_CLASS} arrowClasses={TOOLTIP_ARROW_CLASS}>
      {tip}
    </Tooltip.Content>
  </Tooltip.Root>
{/snippet}

<!-- The badge for whatever is running a session, and the branch / task / pull
     request behind it. A long branch name is cut short on screen and kept whole
     in the tooltip, so a card stays the width of the column however it was
     named. -->
{#snippet metaBadges(
  agent: AgentKind,
  viaCmux: boolean,
  values: { branch?: string | null; taskId?: string | null; pullRequest?: string | null }
)}
  {@const AgentIcon = AGENT_ICONS[agent]}
  <Badge
    variant="secondary"
    class="h-5 gap-1 px-1.5 text-[12px] font-normal text-[var(--color-text-2)]"
  >
    <AgentIcon aria-hidden="true" />
    {agentLabel(agent, viaCmux)}
  </Badge>
  <!-- Keyed by position, not by text: these are three fixed slots, and a branch
       named after its task puts the same word in two of them — a duplicate key
       there would throw and take the whole column down. -->
  {#each chips(values) as chip, slot (slot)}
    {@const ChipIcon = chip.icon}
    <Badge
      variant="outline"
      title={chip.text}
      class="h-5 max-w-[20ch] gap-1 px-1.5 text-[12px] font-normal text-[var(--color-text-2)]"
    >
      <ChipIcon aria-hidden="true" />
      <span class="truncate">{chip.text}</span>
    </Badge>
  {/each}
{/snippet}

<!-- "Working" / "Done", and the drawer's project headings: the same quiet
     lettering, so a division inside a list never competes with the list. -->
{#snippet sectionHead(label: string, count: number)}
  <div class="flex items-center justify-between gap-2 px-1 pt-1 pb-0.5">
    <h3
      class="text-[12px] font-semibold tracking-[0.09em] text-[var(--color-text-2)] uppercase"
    >
      {label}
    </h3>
    <span class="text-[12px] text-[var(--color-text-3)]">{count}</span>
  </div>
{/snippet}

<!-- One session you own. `isDone` is the only difference between the two
     sections: it decides which pair of buttons the card offers. -->
{#snippet workCard(session: OwnedSession, isDone: boolean)}
  {@const active = session.ownedId === activeOwnedId}
  {@const project = projectLabel(session.projectPath ?? session.cwd)}
  {@const count = messageCountLabel(session.messageCount)}
  {@const finishedWhen = isDone ? stamp(session.completedAt) : ''}
  <!-- When the scanner last saw something happen in the conversation. Working
       rows only: a done row already says when it was finished, and two times on
       one line is a line nobody reads. Sessions started here have no stamp and
       the line is simply shorter. -->
  {@const busyWhen = isDone ? '' : stamp(session.lastActivity)}
  <Card.Root
    size="sm"
    class={cn(
      'relative rounded-md ring-[var(--color-border)] transition-colors [--card-spacing:0px]',
      'hover:bg-[var(--color-elevated)]',
      active && 'bg-[var(--color-elevated)] ring-primary/45'
    )}
  >
    {#if active}
      <span class="absolute inset-y-0 left-0 w-[2px] bg-primary" aria-hidden="true"></span>
    {/if}
    <button
      type="button"
      class="flex w-full min-w-0 flex-col items-start gap-1 px-2.5 py-2 text-left"
      title={session.cwd || sessionLabel(session)}
      onclick={() => onSelect(session.ownedId)}
    >
      <!-- The title stops short of the buttons in the corner rather than running
           under them. A card carries two, except a finished one on Done, which
           also carries Start again. -->
      <span
        class={cn(
          'flex w-full min-w-0 items-center gap-2',
          isDone && session.state === 'exited' ? 'pr-[76px]' : 'pr-11'
        )}
      >
        <span class="dot" data-state={session.state} aria-hidden="true"></span>
        <span class="truncate text-[14px] leading-[1.35] font-medium text-[#e6e6ee]">
          {sessionLabel(session)}
        </span>
      </span>

      <span class="flex w-full min-w-0 flex-wrap items-center gap-1">
        {@render metaBadges(session.agent, session.viaCmux, session)}
      </span>

      <!-- Where the work lives, how much of it there is, and when it finished.
           Every piece is left out when there is nothing to say — a session the
           scanner counted no turns for simply has a shorter card. -->
      <span
        class="flex w-full min-w-0 items-center gap-1.5 text-[12px] text-[var(--color-text-2)]"
      >
        <Folder class="size-3 shrink-0 text-[var(--color-text-3)]" aria-hidden="true" />
        <span class="truncate">
          {project.name}{project.parentProject ? ` · ${project.parentProject}` : ''}
        </span>
        {#if count}
          <span class="shrink-0 text-[var(--color-text-3)]">·</span>
          <span class="shrink-0">{count}</span>
        {/if}
        {#if busyWhen}
          <span class="shrink-0 text-[var(--color-text-3)]">·</span>
          <span class="shrink-0" title={exactLocalTime(session.lastActivity)}>
            last active {busyWhen}
          </span>
        {/if}
        {#if finishedWhen}
          <span class="shrink-0 text-[var(--color-text-3)]">·</span>
          <span class="shrink-0" title={exactLocalTime(session.completedAt)}>
            done {finishedWhen}
          </span>
        {/if}
      </span>

      {#if session.latestTurnPreview}
        <span class="w-full min-w-0 truncate text-[13px] text-[var(--color-text-2)]">
          {session.latestTurnPreview}
        </span>
      {/if}
    </button>

    <!-- Out of sight until the card is pointed at or a button is tabbed to, so
         a column of twenty sessions is a list of titles rather than a wall of
         icons. -->
    <div
      class="absolute top-1.5 right-1.5 flex items-center gap-0.5 opacity-0 transition-opacity
             group-hover/card:opacity-100 focus-within:opacity-100"
    >
      <!-- A finished session can be picked back up from either list, and it is
           the FIRST button on the card because it is the one thing you cannot
           do to it otherwise. Starting a done session again does not move it
           off Done: which list a session is on is the user's answer, not the
           terminal's. -->
      {#if session.state === 'exited'}
        {@render action(
          `start ${sessionLabel(session)} again`,
          'Start this session again',
          Play,
          false,
          () => onRestart(session.ownedId)
        )}
      {/if}
      {#if isDone}
        {@render action(
          `reopen ${sessionLabel(session)} — put it back under Working`,
          'Reopen',
          Undo2,
          false,
          () => onReopen(session.ownedId)
        )}
        {@render action(
          `remove ${sessionLabel(session)} from this list`,
          'Remove from this list',
          Trash2,
          true,
          () => askAboutRemoving(session)
        )}
      {:else}
        {@render action(
          `mark ${sessionLabel(session)} done`,
          'Mark done',
          Check,
          false,
          () => onComplete(session.ownedId)
        )}
        <!-- Nothing left to close once the process has ended, and the card's
             hollow dot already says so. -->
        {#if session.state !== 'exited'}
          {@render action(
            `close the terminal for ${sessionLabel(session)}`,
            'Close the terminal',
            Power,
            true,
            () => onClose(session.ownedId)
          )}
        {/if}
      {/if}
    </div>
  </Card.Root>
{/snippet}

<!-- One session in the drawer: everything the scanner knows, and a click
     resumes it. -->
{#snippet resumeRow(session: AgentSession)}
  {@const provider = normalizeProvider(session.provider)}
  {@const when = stamp(session.lastActivity)}
  {@const count = messageCountLabel(session.messageCount)}
  <button
    type="button"
    class="group/resume flex w-full min-w-0 flex-col items-start gap-1 rounded-md px-2 py-1.5
           text-left transition-colors hover:bg-[var(--color-surface)]"
    title={session.projectPath ?? session.title}
    onclick={() => onAdopt(session)}
  >
    <span class="flex w-full min-w-0 items-center gap-2">
      <span class="dot" data-state="available" aria-hidden="true"></span>
      <span class="truncate text-[13px] leading-[1.35] text-[var(--color-text)]">
        {session.title || session.id}
      </span>
      <span
        class="ml-auto shrink-0 text-[12px] text-primary opacity-0 transition-opacity
               group-hover/resume:opacity-100 group-focus-visible/resume:opacity-100"
      >
        Resume
      </span>
    </span>
    <span class="flex w-full min-w-0 flex-wrap items-center gap-1 pl-4">
      {@render metaBadges(provider.agent, provider.viaCmux, {
        branch: session.branchHint,
        taskId: session.taskId,
        pullRequest: session.pullRequestHint
      })}
      {#if when}
        <span class="text-[12px] text-[var(--color-text-3)]" title={exactLocalTime(session.lastActivity)}>
          {when}
        </span>
      {/if}
      {#if count}
        <span class="text-[12px] text-[var(--color-text-3)]">{count}</span>
      {/if}
    </span>
    {#if session.latestTurnPreview}
      <span class="w-full min-w-0 truncate pl-4 text-[13px] text-[var(--color-text-2)]">
        {session.latestTurnPreview}
      </span>
    {/if}
  </button>
{/snippet}

<Tooltip.Provider delayDuration={250}>
  {#if collapsed}
    <!-- Folded up: the same sessions in the same order, one cell each. -->
    <div class="flex h-full w-full flex-col items-center gap-1 overflow-hidden bg-[var(--color-bg)] py-2">
      <Tooltip.Root>
        <Tooltip.Trigger
          class={cn(buttonVariants({ variant: 'ghost', size: 'icon-sm' }), ACTION_CLASS)}
          aria-label="open the sessions column"
          onclick={() => onCollapse(false)}
        >
          <PanelLeftOpen aria-hidden="true" />
        </Tooltip.Trigger>
        <Tooltip.Content side="right" class={TOOLTIP_CLASS} arrowClasses={TOOLTIP_ARROW_CLASS}>
          Open the sessions column
        </Tooltip.Content>
      </Tooltip.Root>

      <div class="mt-1 flex w-full flex-1 flex-col items-center gap-1 overflow-y-auto">
        {#each cells as cell (cell.ownedId)}
          {@const CellIcon = AGENT_ICONS[cell.agent]}
          <Tooltip.Root>
            <Tooltip.Trigger
              class={cn(
                'relative flex size-9 shrink-0 items-center justify-center rounded-md border',
                'border-transparent text-[var(--color-text-2)] transition-colors',
                'hover:bg-[var(--color-elevated)] hover:text-foreground',
                'focus-visible:ring-3 focus-visible:ring-ring/50 outline-none',
                cell.done && 'opacity-55',
                cell.active &&
                  'border-primary/45 bg-[var(--color-elevated)] text-foreground opacity-100'
              )}
              aria-label={cell.label}
              onclick={() => onSelect(cell.ownedId)}
            >
              <CellIcon class="size-4" aria-hidden="true" />
              <span
                class="dot absolute right-1 bottom-1"
                data-state={cell.state}
                aria-hidden="true"
              ></span>
            </Tooltip.Trigger>
            <Tooltip.Content side="right" class={TOOLTIP_CLASS} arrowClasses={TOOLTIP_ARROW_CLASS}>
              {cell.label}{cell.done ? ' · done' : ''}
            </Tooltip.Content>
          </Tooltip.Root>
        {/each}
      </div>
    </div>
  {:else}
    <div class="flex h-full min-h-0 flex-col bg-[var(--color-bg)] text-[var(--color-text)]">
      <header
        class="flex shrink-0 items-center gap-2 border-b border-[var(--color-border)] px-2.5 py-2"
      >
        <h2 class="text-[12px] font-semibold tracking-[0.09em] text-[var(--color-text-2)] uppercase">
          My work
        </h2>
        <span class="text-[12px] text-[var(--color-text-3)]">{owned.length}</span>
        <div class="ml-auto flex items-center gap-1">
          {@render action('start a new session', 'New session', Plus, false, () => onNewSession())}
          {@render action(
            'fold the sessions column up',
            'Fold this column up',
            PanelLeftClose,
            false,
            () => onCollapse(true)
          )}
        </div>
      </header>

      <div class="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto px-2 py-2">
        {#if owned.length === 0}
          <p class="px-1 py-1 text-[13px] text-[var(--color-text-2)]">
            No sessions yet — find one to resume below.
          </p>
        {:else}
          {#if split.working.length > 0}
            {@render sectionHead('Working', split.working.length)}
            {#each split.working as session (session.ownedId)}
              {@render workCard(session, false)}
            {/each}
          {/if}
          <!-- Nobody needs to be told they have finished nothing yet. -->
          {#if split.done.length > 0}
            <div class={split.working.length > 0 ? 'pt-2' : ''}>
              {@render sectionHead('Done', split.done.length)}
            </div>
            {#each split.done as session (session.ownedId)}
              {@render workCard(session, true)}
            {/each}
          {/if}
        {/if}
      </div>

      <Collapsible.Root
        bind:open={findOpen}
        class="shrink-0 border-t border-[var(--color-border)]"
      >
        <div class="flex items-center gap-1 px-2 py-1.5">
          <Collapsible.Trigger
            class="flex min-w-0 flex-1 items-center gap-1.5 rounded-md px-1 py-1 text-left
                   text-[var(--color-text-2)] transition-colors hover:bg-[var(--color-elevated)]
                   hover:text-[var(--color-text)] focus-visible:ring-3 focus-visible:ring-ring/50
                   outline-none"
          >
            {#if findOpen}
              <ChevronDown class="size-3.5 shrink-0" aria-hidden="true" />
            {:else}
              <ChevronRight class="size-3.5 shrink-0" aria-hidden="true" />
            {/if}
            <span class="text-[13px]">Find a session</span>
            <span class="ml-auto pl-2 text-[12px] text-[var(--color-text-3)]">
              {resumable.length}
            </span>
          </Collapsible.Trigger>
          {@render action(
            'look for agent sessions again',
            scanning ? 'Looking…' : 'Look again',
            RefreshCw,
            false,
            () => onRescan()
          )}
        </div>

        <!-- NOT `flex` on the content itself: a closed collapsible is hidden by
             the `hidden` attribute, which is a display rule the browser only
             applies by default — any display class of ours would beat it and
             the drawer would never close. The layout lives on the div inside. -->
        <Collapsible.Content class="max-h-[45vh] overflow-y-auto">
          <div class="flex flex-col gap-2 px-2 pt-1 pb-2">
            <div class="relative">
              <Search
                class="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2
                       text-[var(--color-text-3)]"
                aria-hidden="true"
              />
              <Input
                type="search"
                placeholder="Search sessions to resume"
                aria-label="Search sessions to resume"
                bind:value={query}
                class="h-7 rounded-md bg-[var(--color-surface)] pl-7 text-[13px] md:text-[13px]"
              />
            </div>

            {#if resumable.length === 0}
              <p class="px-1 text-[13px] text-[var(--color-text-2)]">
                {scanning ? 'Looking for agent sessions…' : 'Nothing to resume.'}
              </p>
            {:else if grouped.available.length === 0}
              <p class="px-1 text-[13px] text-[var(--color-text-2)]">
                Nothing to resume matches “{query.trim()}”.
              </p>
            {:else}
              {#each grouped.available as group (group.path)}
                {@const visible = visibleGroupItems(
                  group.items,
                  RESUME_GROUP_ROW_CAP,
                  expandedRows[group.path] === true
                )}
                <div class="flex flex-col">
                  <button
                    type="button"
                    class="flex w-full items-center gap-1.5 rounded-md px-1 py-1 text-left
                           text-[12px] text-[var(--color-text-2)] transition-colors
                           hover:bg-[var(--color-elevated)] hover:text-[var(--color-text)]"
                    aria-expanded={expanded(group.path)}
                    title={group.path || 'Sessions with no project folder'}
                    onclick={() => toggleGroup(group.path)}
                  >
                    {#if expanded(group.path)}
                      <ChevronDown class="size-3 shrink-0" aria-hidden="true" />
                    {:else}
                      <ChevronRight class="size-3 shrink-0" aria-hidden="true" />
                    {/if}
                    <span class="truncate">{group.name}</span>
                    {#if group.parentProject}
                      <!-- A worktree is named for its task, so the repository it
                           belongs to is said here, and it is the first thing to
                           go when the column is narrow. -->
                      <span class="truncate text-[var(--color-text-3)]">
                        · {group.parentProject}
                      </span>
                    {/if}
                    <span class="ml-auto shrink-0 pl-2 text-[var(--color-text-3)]">
                      {group.items.length}
                    </span>
                  </button>
                  {#if expanded(group.path)}
                    <div class="flex flex-col">
                      {#each visible.shown as session (`${session.provider}:${session.id}`)}
                        {@render resumeRow(session)}
                      {/each}
                      {#if visible.hiddenCount > 0}
                        <button
                          type="button"
                          class="rounded-md px-2 py-1 pl-6 text-left text-[12px]
                                 text-[var(--color-text-2)] transition-colors
                                 hover:bg-[var(--color-elevated)] hover:text-[var(--color-text)]"
                          onclick={() => (expandedRows = { ...expandedRows, [group.path]: true })}
                        >
                          Show {visible.hiddenCount} more
                        </button>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            {/if}
          </div>
        </Collapsible.Content>
      </Collapsible.Root>
    </div>
  {/if}

  <!-- The one question this column asks. It used to be `window.confirm`, which
       the desktop webview does not have: there the dialog never appeared, the
       call answered "no" without asking anyone, and the only place the warning
       ever showed was a browser. -->
  <AlertDialog.Root bind:open={removeOpen}>
    <AlertDialog.Content
      class="rounded-lg bg-background text-foreground ring-[var(--color-border)]
             shadow-[var(--shadow-lg)]"
    >
      <AlertDialog.Header>
        <AlertDialog.Title class="text-[14px] leading-[1.4] font-semibold">
          Remove “{removing ? sessionLabel(removing) : ''}” from your sessions?
        </AlertDialog.Title>
        <AlertDialog.Description class="text-[13px] leading-[1.5] text-[var(--color-text-2)]">
          {removing ? removeQuestion(removing) : ''}
        </AlertDialog.Description>
      </AlertDialog.Header>
      <AlertDialog.Footer class="bg-transparent">
        <AlertDialog.Cancel size="sm" class="text-[13px]">Keep</AlertDialog.Cancel>
        <AlertDialog.Action
          size="sm"
          variant="destructive"
          class="text-[13px]"
          onclick={removeNow}
        >
          Remove
        </AlertDialog.Action>
      </AlertDialog.Footer>
    </AlertDialog.Content>
  </AlertDialog.Root>
</Tooltip.Provider>

<style>
  /* The state dot, unchanged from the rail this column replaces:
     live = accent pulse, background = solid, exited = hollow, and a scanned
     session waiting to be resumed = hollow in the resume colour. */
  .dot {
    flex: 0 0 auto;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #4c4c5a;
  }

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

  @media (prefers-reduced-motion: reduce) {
    .dot[data-state='live'] {
      animation: none;
    }
  }
</style>
