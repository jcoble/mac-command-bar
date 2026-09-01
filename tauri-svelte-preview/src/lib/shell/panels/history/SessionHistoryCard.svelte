<script lang="ts">
  /**
   * SessionHistoryCard.svelte — one past session in the History panel.
   *
   * Every card draws the same zones in the same order, whether the scan found
   * a model, a folder, turns, agents or none of them: the row that names the
   * session, five labelled facts, the turns it kept, the agents it ran, what
   * can be done about it, and one line saying why. A value the scan did not
   * have is an em dash in the muted colour and its zone stays where it was —
   * a card that drops a block reads as a different design rather than as the
   * same card with less in it, which is why forty sessions used to look like
   * forty different layouts.
   *
   * The card decides nothing. Which actions a session supports, and what to
   * say about the ones it does not, comes from `sessionHistoryActions`;
   * running them belongs to the panel, which owns the service and the confirm
   * dialog. That split is what lets the roster be tested without a browser.
   */
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import ChevronUp from '@lucide/svelte/icons/chevron-up';
  import Copy from '@lucide/svelte/icons/copy';
  import Ellipsis from '@lucide/svelte/icons/ellipsis';
  import FileCode2 from '@lucide/svelte/icons/file-code-2';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import Play from '@lucide/svelte/icons/play';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import type { Component } from 'svelte';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Card } from '$lib/components/ui/card/index.js';
  import { Chip } from '$lib/components/ui/chip/index.js';
  import * as Collapsible from '$lib/components/ui/collapsible/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { HoverActionButton } from '$lib/components/ui/hover-actions/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { ListRow } from '$lib/components/ui/list-row/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import { Separator } from '$lib/components/ui/separator/index.js';
  import { AGENT_ICONS, agentDisplayName } from '$lib/shell/agentIcons.ts';
  import { normalizeProvider } from '$lib/shell/ownedSessions.ts';
  import { exactLocalTime, formatLastActivity } from '$lib/shell/relativeTime.ts';
  import type {
    SessionLibraryRecord,
    SessionLibraryTurn
  } from '$lib/shell/sessionLibrary/sessionLibraryModel.ts';
  import { cn } from '$lib/utils.js';

  import {
    SESSION_HISTORY_MENU_ACTION_IDS,
    sessionHistoryActions,
    type SessionHistoryAction,
    type SessionHistoryActionId
  } from './sessionHistoryActions.ts';

  interface Props {
    record: SessionLibraryRecord;
    /** The title shown, worked out by the history view model. */
    title: string;
    /** One line of what was last said, or the session's first prompt. */
    excerpt: string;
    expanded: boolean;
    detail?: Pick<SessionLibraryRecord, 'firstPrompt' | 'latestTurns'> | null;
    detailsLoading?: boolean;
    /** The clock, passed down so forty cards do not keep forty of them. */
    now: Date;
    onToggle(): void;
    onAction(id: SessionHistoryActionId): void;
    /** Put a block of the transcript on the clipboard. */
    onCopyText(value: string): void;
    /** Open one of a session's agents' own transcripts. */
    onOpenLog(path: string): void;
  }

  let {
    record,
    title,
    excerpt,
    expanded,
    detail = null,
    detailsLoading = false,
    now,
    onToggle,
    onAction,
    onCopyText,
    onOpenLog
  }: Props = $props();

  let menuOpen = $state(false);
  /** The turns are behind a disclosure and start closed, the way "Why" does. */
  let turnsOpen = $state(false);

  /** What an unknown value reads as, everywhere on the card. */
  const DASH = '—';

  /** The two lines of small-caps section label, so they cannot drift apart. */
  const SECTION_LABEL =
    'm-0 text-xs leading-normal font-semibold tracking-wide uppercase text-muted-foreground';

  /** A quiet sentence: the closing line, and every "nothing here" line. */
  const QUIET_LINE = 'm-0 text-(length:--text-quiet) leading-normal text-muted-foreground';

  const actions = $derived(sessionHistoryActions(record));
  const byId = $derived(new Map(actions.map((item) => [item.id, item])));

  function action(id: SessionHistoryActionId): SessionHistoryAction {
    // Every id in the union is in the roster, so this only guards a typo.
    return byId.get(id) ?? { id, label: id, enabled: false, disabledReason: null, destructive: false };
  }

  const menuActions = $derived(SESSION_HISTORY_MENU_ACTION_IDS.map(action));

  const agent = $derived(normalizeProvider(record.provider));
  const ProviderIcon = $derived(AGENT_ICONS[agent.agent]);
  const providerName = $derived(agentDisplayName(agent.agent, agent.viaCmux));
  const age = $derived(formatLastActivity(record.updatedAt, now));
  const modelLabel = $derived(record.model?.trim() || DASH);
  const turnsLabel = $derived(
    record.messageCount && record.messageCount > 0 ? String(record.messageCount) : DASH
  );
  const firstPrompt = $derived(detail?.firstPrompt?.trim() ?? '');
  const latestTurns = $derived(detail?.latestTurns ?? []);

  /**
   * A folder path, shortened from the middle so both ends survive: the repo is
   * at the front and the folder's own name is at the back, and it is the
   * stretch between them nobody reads. The whole path stays on the title.
   */
  function shortenPath(value: string): string {
    const budget = 42;
    if (value.length <= budget) return value;
    const head = Math.ceil((budget - 1) / 2);
    const tail = budget - 1 - head;
    return `${value.slice(0, head)}…${value.slice(value.length - tail)}`;
  }

  /** The five labelled facts, in this order on every card, dash when unknown. */
  const facts = $derived.by(() => {
    const folder = record.canonicalCwd.trim();
    return [
      {
        label: 'Folder',
        value: folder ? shortenPath(folder) : DASH,
        title: folder || undefined
      },
      {
        label: 'Last activity',
        value: age || DASH,
        title: record.updatedAt ? exactLocalTime(record.updatedAt) : undefined
      },
      { label: 'Model', value: modelLabel, title: record.model ?? undefined },
      { label: 'Turns', value: turnsLabel, title: undefined },
      {
        label: 'Subagents',
        value: record.subagents.length > 0 ? String(record.subagents.length) : DASH,
        title: undefined
      }
    ];
  });

  /**
   * Everything the scan kept of the conversation, in order: the first prompt,
   * then the window of latest turns behind it. The first prompt is dropped
   * when the window already opens with it, so it is never shown twice.
   */
  const turns = $derived.by(() => {
    const first = firstPrompt;
    const list: SessionLibraryTurn[] = first ? [{ speaker: 'user', text: first }] : [];
    for (const turn of latestTurns) {
      const text = turn.text.trim();
      if (!text || text === first) continue;
      list.push({ speaker: turn.speaker, text });
    }
    return list;
  });

  /** The first turn and the last two. Fewer than four means all of them. */
  const shownTurns = $derived(turns.length < 4 ? turns : [turns[0], ...turns.slice(-2)]);
  /** How many sit between the first turn and the final pair. */
  const elidedTurns = $derived(turns.length < 4 ? 0 : turns.length - 3);

  /** The four actions offered as words, in the order a person reads them. */
  const CLUSTER: readonly { id: SessionHistoryActionId; icon: Component }[] = [
    { id: 'resume-assembly', icon: Play },
    { id: 'view-log', icon: FileCode2 },
    { id: 'open-working-directory', icon: FolderOpen },
    { id: 'delete', icon: Trash2 }
  ];

  /**
   * One sentence at the foot of every card. It says why a button is off when
   * one is, and otherwise says what this card is not showing — so the line is
   * always there and always about this session.
   */
  const closingLine = $derived.by(() => {
    const off = CLUSTER.map((item) => action(item.id)).find(
      (item) => !item.enabled && item.disabledReason
    );
    if (off?.disabledReason) return off.disabledReason;
    if (detailsLoading) return 'Loading session details…';
    if (turns.length === 0) return 'No turns were stored for this session.';
    return `Started with ${providerName}. Everything the scan kept is shown above.`;
  });

  function run(id: SessionHistoryActionId): void {
    if (!action(id).enabled) return;
    onAction(id);
  }
</script>

<article
  class="session-card px-(--space-2) pt-(--space-4) last:pb-(--space-4)"
  data-testid="session-history-card"
>
  <!-- A card is told apart from the panel by its own fill and a shadow. It
       carries no outline: a border here would be signalling state, which the
       kit reserves for sheets that float over the page. -->
  <Card
    class={cn(
      'gap-(--space-3) border-0 bg-foreground/5 py-(--space-2) shadow-sm',
      expanded && 'bg-foreground/10',
      menuOpen && 'menu-open'
    )}
  >
    <!-- The row that names the session. The title is what the reader scans
         for, so it is the one thing that never gives way: it takes the row
         and wraps to a second line before anything is allowed to hide it.
         Chips are context, not identity - one with nothing to say is left
         out here entirely (the facts grid below is where absence is shown). -->
    <ListRow onclick={onToggle} selected={expanded} actionsLabel="Session actions" class="pr-24">
      <ChevronRight
        class={cn(
          'size-4 shrink-0 text-muted-foreground transition-transform',
          expanded && 'rotate-90'
        )}
        aria-hidden="true"
      />
      <ProviderIcon class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
      <span class="line-clamp-2 min-w-0 flex-1 font-medium leading-snug break-words" title={title}
        >{title}</span
      >
      <span class="flex shrink-0 items-center gap-(--space-1)">
        {#if record.model?.trim()}
          <Chip class="h-6 max-w-32 px-2 font-normal" data-testid="session-history-model">
            <span class="truncate" title={record.model}>{modelLabel}</span>
          </Chip>
        {/if}
        {#if turnsLabel !== DASH}
          <Chip tone="count" class="h-6 px-2 font-normal">{turnsLabel}</Chip>
        {/if}
      </span>

      {#snippet actions()}
        <HoverActionButton
          label={action('resume-assembly').enabled
            ? 'Resume as Assembly Session'
            : (action('resume-assembly').disabledReason ?? 'Resume as Assembly Session')}
          tone="primary"
          disabled={!action('resume-assembly').enabled}
          onclick={() => run('resume-assembly')}
        >
          <Play />
        </HoverActionButton>
        <HoverActionButton label={expanded ? 'Hide details' : 'Show details'} onclick={onToggle}>
          {#if expanded}<ChevronUp />{:else}<ChevronDown />{/if}
        </HoverActionButton>

        <DropdownMenu.Root bind:open={menuOpen}>
          <DropdownMenu.Trigger>
            {#snippet child({ props })}
              <IconButton
                {...props}
                label="More actions"
                side="bottom"
                class="rounded-md bg-transparent text-[var(--color-text)] shadow-none
                       hover:bg-accent/60 hover:text-foreground"
              >
                <Ellipsis />
              </IconButton>
            {/snippet}
          </DropdownMenu.Trigger>
          <DropdownMenu.Content class="w-56" align="end">
            {#each menuActions as item (item.id)}
              <DropdownMenu.Item
                disabled={!item.enabled}
                title={item.disabledReason ?? undefined}
                class={item.destructive ? 'text-[var(--color-bad)]' : undefined}
                onSelect={() => run(item.id)}
              >
                {item.label}
              </DropdownMenu.Item>
            {/each}
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      {/snippet}
    </ListRow>

    {#if !expanded}
      <p class="truncate px-(--space-3) text-(length:--text-quiet) text-muted-foreground">
        {excerpt || DASH}
      </p>
    {/if}

    <!-- The body is written into the page only while the card is open. A
         history tab that keeps every session's transcript mounted is what used
         to freeze the shell, so this stays a real removal rather than a hidden
         subtree. -->
    {#if expanded}
      <!-- The plain facts, one labelled line each, on a hairline the eye can
           run along. Every row is here on every card; a value the scan did not
           have is a dash rather than a missing line. -->
      <dl class="m-0 grid grid-cols-[auto_minmax(0,1fr)] px-(--space-3)">
        {#each facts as fact, index (fact.label)}
          {@const ruled = index < facts.length - 1 && 'border-b border-border'}
          {@const unknown = fact.value === DASH}
          <dt
            class={cn(
              'm-0 flex items-center py-(--space-2) pr-(--space-2) text-(length:--text-quiet) leading-normal text-muted-foreground',
              ruled
            )}
          >
            {fact.label}
          </dt>
          <dd
            class={cn(
              'm-0 flex min-w-0 items-center justify-end py-(--space-2) text-(length:--text-body) leading-normal',
              unknown ? 'text-muted-foreground' : 'text-foreground',
              ruled
            )}
            title={fact.title}
          >
            <span class="truncate">{fact.value}</span>
          </dd>
        {/each}
      </dl>

      <section class="flex flex-col gap-(--space-2) px-(--space-3)">
        <div class="flex items-center justify-between">
          <h3 class={SECTION_LABEL}>First prompt</h3>
          <IconButton
            label="Copy first prompt"
            size="xs"
            disabled={!firstPrompt}
            onclick={() => onCopyText(firstPrompt)}
          >
            <Copy />
          </IconButton>
        </div>
        {#if detailsLoading}
          <p class={QUIET_LINE}>Loading session details…</p>
        {:else if firstPrompt}
          <ScrollArea>
            <p class="m-0 max-h-48 overflow-y-auto whitespace-pre-wrap text-(length:--text-quiet) leading-relaxed">
              {firstPrompt}
            </p>
          </ScrollArea>
        {:else}
          <p class={QUIET_LINE}>First prompt was not stored for this session.</p>
        {/if}
      </section>

      <!-- The conversation, behind a disclosure: the first turn, then the last
           two. The block is here even when the scan kept nothing, because a
           card that sometimes has a turns section and sometimes does not is
           the thing this rebuild exists to end. -->
      <div class="flex flex-col px-(--space-3)">
        <Collapsible.Root bind:open={turnsOpen}>
          <Collapsible.Trigger
            class="flex w-fit items-center gap-1 rounded-lg py-(--space-1)
                   text-(length:--text-quiet) leading-normal text-muted-foreground
                   transition-colors outline-none hover:text-foreground
                   focus-visible:ring-3 focus-visible:ring-ring/50"
          >
            <ChevronRight
              class={cn('size-3.5 transition-transform', turnsOpen && 'rotate-90')}
              aria-hidden="true"
            />
            Turns ({turns.length})
          </Collapsible.Trigger>
          <Collapsible.Content>
            <div class="flex flex-col gap-(--space-2) pt-(--space-2)">
              {#if turns.length === 0}
                <p class={QUIET_LINE}>No turns were stored for this session.</p>
              {:else}
                {#each shownTurns as turn, index (index)}
                  {#if index === 1 && elidedTurns > 0}
                    <p class={QUIET_LINE}>… {elidedTurns} earlier turns</p>
                  {/if}
                  <!--
                    Each turn scrolls inside its own box. An agent's last answer
                    runs to thousands of characters, and letting one of them
                    grow to its full height pushed the next session off the
                    panel entirely.
                  -->
                  <div class="turn-block overflow-hidden rounded-lg bg-background">
                    <div
                      class="flex items-center gap-(--space-2) border-b px-(--space-2) py-1"
                      class:speaker-user={turn.speaker === 'user'}
                      class:speaker-agent={turn.speaker !== 'user'}
                    >
                      <p class="m-0 flex-1 text-xs font-semibold tracking-wide uppercase">
                        {turn.speaker === 'user' ? 'You' : 'Agent'}
                      </p>
                      <IconButton
                        label="Copy this turn"
                        size="xs"
                        onclick={() => onCopyText(turn.text)}
                      >
                        <Copy />
                      </IconButton>
                    </div>
                    <ScrollArea>
                      <p
                        class="m-0 px-(--space-2) py-(--space-2) text-(length:--text-quiet)
                               leading-relaxed whitespace-pre-wrap text-foreground"
                      >
                        {turn.text}
                      </p>
                    </ScrollArea>
                  </div>
                {/each}
              {/if}
            </div>
          </Collapsible.Content>
        </Collapsible.Root>
      </div>

      <!-- Who the session ran for itself. Named, because "2 subagents" in the
           facts above is not enough to open one of their transcripts. -->
      <section class="flex flex-col gap-(--space-2) px-(--space-3)">
        <h3 class={SECTION_LABEL}>Agents this session ran</h3>
        {#if record.subagents.length === 0}
          <p class={QUIET_LINE}>No agents were recorded for this session.</p>
        {:else}
          {#each record.subagents as subagent, index (index)}
            <ListRow class="bg-background" actionsLabel="Agent actions">
              <span class="min-w-0 flex-1 truncate">{subagent.name}</span>
              {#if subagent.kind}<Chip class="h-6 px-2 font-normal">{subagent.kind}</Chip>{/if}
              {#if subagent.messageCount}
                <Chip tone="count" class="h-6 px-2 font-normal">{subagent.messageCount}</Chip>
              {/if}
              {#snippet actions()}
                <HoverActionButton
                  label={subagent.logPath
                    ? 'View log'
                    : 'No transcript file was found for this agent.'}
                  tone="info"
                  disabled={!subagent.logPath}
                  onclick={() => subagent.logPath && onOpenLog(subagent.logPath)}
                >
                  <FileCode2 />
                </HoverActionButton>
              {/snippet}
            </ListRow>
          {/each}
        {/if}
      </section>

      <Separator />

      <!-- What can be done about this session. Resume is the one filled
           button; the rest are offered as words. An action that is not backed
           by something real is switched off and dimmed rather than removed,
           and the line underneath says which one and why. -->
      <section class="flex flex-col gap-(--space-2) px-(--space-3)">
        <h3 class={SECTION_LABEL}>What you can do</h3>
        <div class="flex flex-wrap items-center gap-(--space-1)">
          {#each CLUSTER as item (item.id)}
            {@const entry = action(item.id)}
            {@const Icon = item.icon}
            <Button
              variant={item.id === 'resume-assembly' ? 'default' : 'ghost'}
              class={cn(
                'rounded-full text-(length:--text-quiet)',
                item.id === 'resume-assembly'
                  ? 'px-4'
                  : 'px-3 text-muted-foreground hover:text-foreground',
                entry.destructive && 'hover:text-[var(--color-bad)]'
              )}
              disabled={!entry.enabled}
              title={entry.disabledReason ?? undefined}
              onclick={() => run(item.id)}
            >
              <Icon aria-hidden="true" />
              {entry.label}
            </Button>
          {/each}
        </div>
        <p class={QUIET_LINE}>{closingLine}</p>
      </section>
    {/if}
  </Card>
</article>

<style>
  /*
    The kit's row is sized for a dense file list. A session row is the thing a
    decision is made about, so it takes the shell's body size and the same
    12 by 16 inset the cards use, and its hover cluster moves in to match.
  */
  .session-card :global([data-slot='list-row'] > button) {
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    font-size: var(--text-body);
  }
  .session-card :global([data-slot='hover-actions']) {
    right: var(--space-3);
  }

  /*
    While the more-actions menu is open the pointer is over the menu, not the
    row, so the row's hover cluster would fade out and take its own trigger with
    it. Hold it in place until the menu closes. Nothing moves and nothing
    rebuilds — this is the same rest state the hover already ends at.
  */
  .session-card.menu-open :global([data-slot='hover-actions']) {
    opacity: 1;
    pointer-events: auto;
    transform: none;
  }

  /*
    Who said it, told apart at a glance. Both headers were the same muted grey,
    so two stacked turns read as one block of text and the whole point of
    showing a pair — seeing the question against the answer — was lost.
  */
  .turn-block .speaker-user {
    background: color-mix(in srgb, var(--color-accent) 22%, transparent);
    color: var(--color-text);
  }

  .turn-block .speaker-agent {
    background: color-mix(in srgb, var(--color-text) 7%, transparent);
    color: var(--color-text-2, var(--color-text));
  }

  /*
    The height limit belongs on the element that scrolls.

    Setting it on the scroll area's outer box did nothing: that box is only a
    positioning parent, and the viewport inside it is `height: 100%`. A
    percentage height measured against a parent whose own height is `auto` — a
    max-height does not change that — resolves to `auto` as well, so the viewport
    grew to the full height of the turn, never overflowed itself, and never had
    anything to scroll. The text simply ran past the box and was cut off by an
    ancestor, with no scrollbar anywhere.

    Bounding the viewport instead gives it something to scroll, and leaves a
    short turn its natural height rather than a fixed box with empty space under
    it.
  */
  .turn-block :global([data-slot='scroll-area-viewport']) {
    height: auto;
    max-height: 11rem;
  }
</style>
