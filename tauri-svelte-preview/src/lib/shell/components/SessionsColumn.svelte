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
   *                  a terminal running. That is also why a card offers two
   *                  separate destructive-looking actions: closing the
   *                  terminal ends the process and keeps the card, removing
   *                  takes the card away — and why removing is only offered
   *                  once a session is done.
   *
   *                  Working and Done are two panes that open and close on
   *                  their own, and each one scrolls its own list. That is a
   *                  fix, not a decoration: they used to be plain runs of
   *                  cards in one flex column, so a card growing — being
   *                  selected, being read — squeezed every other card down to
   *                  a sliver. Cards now refuse to shrink below their own
   *                  content and the pane around them scrolls instead.
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
  import { tick } from 'svelte';

  import Bot from '@lucide/svelte/icons/bot';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Gem from '@lucide/svelte/icons/gem';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import GitPullRequest from '@lucide/svelte/icons/git-pull-request';
  import Hash from '@lucide/svelte/icons/hash';
  import PanelLeftClose from '@lucide/svelte/icons/panel-left-close';
  import PanelLeftOpen from '@lucide/svelte/icons/panel-left-open';
  import Plus from '@lucide/svelte/icons/plus';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Search from '@lucide/svelte/icons/search';
  import SquareCode from '@lucide/svelte/icons/square-code';
  import SquareTerminal from '@lucide/svelte/icons/square-terminal';
  import Terminal from '@lucide/svelte/icons/terminal';

  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import { Badge } from '$lib/components/ui/badge/index.js';
  import { buttonVariants } from '$lib/components/ui/button/index.js';
  import * as Collapsible from '$lib/components/ui/collapsible/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import SessionCard from '$lib/shell/components/sessions/SessionCard.svelte';
  import { messageCountLabel } from '$lib/shell/components/sessions/sessionCardModel';
  import { normalizeProvider, type AgentKind, type OwnedSession } from '$lib/shell/ownedSessions';
  import { exactLocalTime, formatLastActivity } from '$lib/shell/relativeTime';
  import {
    groupSessions,
    groupToggleKey,
    isGroupExpanded,
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

  /** Are the Working and Done panes open? Both start open, and each closes on
   * its own — a column with fifteen finished sessions in it can be folded down
   * to the four being worked on without losing the count beside "Done". */
  let workingOpen = $state(true);
  let doneOpen = $state(true);

  /** Which cards have their detail block open, by owned id. Held here rather
   * than inside the cards so a redraw — a scan landing, a session starting —
   * does not close what somebody was reading. */
  let openCards = $state<Record<string, boolean>>({});

  function toggleCard(ownedId: string): void {
    openCards = { ...openCards, [ownedId]: openCards[ownedId] !== true };
  }

  /** What the drawer's search box holds. Deliberately the column's own state
   * and NOT reported to the page: narrowing what you can see loads nothing. */
  let query = $state('');
  const searching = $derived(query.trim().length > 0);

  /** Is the drawer open? Closed to start: it is a cupboard, not a room. */
  let findOpen = $state(false);

  /** The drawer's search box, once it is on screen. Null while the drawer is
   * closed — a closed collapsible is `hidden`, so there is no input to hold. */
  let findInput = $state<HTMLInputElement | null>(null);

  /**
   * Open the "Find a session" drawer from somewhere else in the shell, with the
   * cursor already in the search box.
   *
   * The context panel's "Search all sessions" link is on the far side of the
   * window from this drawer, so a link that only scrolled a closed drawer into
   * view would look like it had done nothing. The page reaches this through
   * `bind:this` on the component.
   *
   * The focus waits a tick because the box does not exist yet at the moment the
   * drawer is told to open — and if it still is not there, opening the drawer is
   * the part that mattered and the click is not wasted.
   */
  export function openFinder(): void {
    findOpen = true;
    void tick().then(() => findInput?.focus());
  }

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

<!-- "Working" or "Done": a heading that opens and closes, and under it that
     list's cards with their own scrollbar.

     The pane grows to fill what is left of the column while it is open
     (`flex-auto` + `min-h-0`) and takes only its heading's height while it is
     closed. Two open panes therefore share the column between them and each
     one scrolls its own cards, which is what stops a long list of finished
     sessions from pushing the ones being worked on off the screen. -->
{#snippet workPane(
  label: string,
  sessions: OwnedSession[],
  isDone: boolean,
  open: boolean,
  setOpen: (next: boolean) => void
)}
  <Collapsible.Root
    {open}
    onOpenChange={setOpen}
    class={cn('flex min-w-0 flex-col', open ? 'min-h-0 flex-auto' : 'shrink-0')}
  >
    <Collapsible.Trigger
      class="flex w-full shrink-0 items-center gap-1.5 rounded-md px-1 py-1.5 text-left
             transition-colors hover:bg-[var(--color-elevated)] focus-visible:ring-3
             focus-visible:ring-ring/50 outline-none"
    >
      <ChevronRight
        class={cn(
          'size-3 shrink-0 text-[var(--color-text-2)] transition-transform',
          open && 'rotate-90'
        )}
        aria-hidden="true"
      />
      <!-- A span rather than a heading: this is the label of a button, and a
           button may only contain phrasing content. -->
      <span class="text-[12px] font-semibold tracking-[0.09em] text-[var(--color-text-2)] uppercase">
        {label}
      </span>
      <span class="ml-auto pl-2 text-[12px] text-[var(--color-text-2)]">{sessions.length}</span>
    </Collapsible.Trigger>

    <!-- NOT `flex` on the content itself: a closed collapsible is hidden by the
         `hidden` attribute, which is a display rule the browser only applies by
         default — any display class of ours would beat it and the pane would
         never close. `flex-auto` is a sizing rule, not a display one, so it is
         safe here; the arrangement of the cards lives on the div inside. -->
    <Collapsible.Content class="min-h-0 flex-auto overflow-y-auto">
      <div class="flex flex-col gap-1.5 py-1">
        {#each sessions as session (session.ownedId)}
          <SessionCard
            {session}
            {isDone}
            active={session.ownedId === activeOwnedId}
            expanded={openCards[session.ownedId] === true}
            meta={metaBadges}
            onToggle={() => toggleCard(session.ownedId)}
            onSelect={() => onSelect(session.ownedId)}
            onRestart={() => onRestart(session.ownedId)}
            onComplete={() => onComplete(session.ownedId)}
            onReopen={() => onReopen(session.ownedId)}
            onClose={() => onClose(session.ownedId)}
            onAskRemove={() => askAboutRemoving(session)}
          />
        {/each}
      </div>
    </Collapsible.Content>
  </Collapsible.Root>
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
        <span class="text-[12px] text-[var(--color-text-2)]" title={exactLocalTime(session.lastActivity)}>
          {when}
        </span>
      {/if}
      {#if count}
        <span class="text-[12px] text-[var(--color-text-2)]">{count}</span>
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
        <span class="text-[12px] text-[var(--color-text-2)]">{owned.length}</span>
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

      <!-- The two panes and nothing else. `overflow-hidden` rather than a
           scrollbar of its own: each pane scrolls its own cards, so a scrollbar
           here would be a second one wrapped around the first. -->
      <div class="flex min-h-0 flex-1 flex-col gap-1 overflow-hidden px-2 py-2">
        {#if owned.length === 0}
          <p class="px-1 py-1 text-[13px] text-[var(--color-text-2)]">
            No sessions yet — find one to resume below.
          </p>
        {:else}
          {#if split.working.length > 0}
            {@render workPane(
              'Working',
              split.working,
              false,
              workingOpen,
              (next) => (workingOpen = next)
            )}
          {/if}
          <!-- Nobody needs to be told they have finished nothing yet. -->
          {#if split.done.length > 0}
            {@render workPane('Done', split.done, true, doneOpen, (next) => (doneOpen = next))}
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
            <span class="ml-auto pl-2 text-[12px] text-[var(--color-text-2)]">
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
                bind:ref={findInput}
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
                      <span class="truncate text-[var(--color-text-2)]">
                        · {group.parentProject}
                      </span>
                    {/if}
                    <span class="ml-auto shrink-0 pl-2 text-[var(--color-text-2)]">
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
  /* The state dot on the folded-up strip and on a drawer row: a running
     terminal this app is attached to pulses in the live colour, one running
     without us is the same colour standing still, a terminal that has ended is
     hollow, and a session on disk waiting to be resumed is hollow too — it has
     no terminal at all yet.

     Every colour is a token. The dot used to be five hard-coded hexes, which
     meant it was the one thing in the shell a theme could not move. The cards
     draw their own dot from the same tokens (SessionCard.svelte): Svelte scopes
     styles to the component, so the rule cannot be shared. */
  .dot {
    flex: 0 0 auto;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--color-text-3);
  }

  .dot[data-state='live'] {
    background: var(--color-live);
    animation: pulse 1.9s ease-in-out infinite;
  }

  .dot[data-state='background'] {
    background: var(--color-live);
  }

  .dot[data-state='exited'] {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--color-text-2);
  }

  .dot[data-state='available'] {
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
    .dot[data-state='live'] {
      animation: none;
    }
  }
</style>
