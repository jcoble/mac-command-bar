<script lang="ts">
  /**
   * SessionHistoryCard.svelte — one past session in the History panel.
   *
   * At rest a card is two lines: what the session was called, and one line of
   * facts about it. Everything else is behind hover or the expand control, so a
   * project with forty sessions in it can still be read down.
   *
   * The card decides nothing. Which actions a session supports, and what to say
   * about the ones it does not, comes from `sessionHistoryActions`; running them
   * belongs to the panel, which owns the service and the confirm dialog. That
   * split is what lets the roster be tested without a browser.
   */
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import ChevronUp from '@lucide/svelte/icons/chevron-up';
  import Copy from '@lucide/svelte/icons/copy';
  import Ellipsis from '@lucide/svelte/icons/ellipsis';
  import FileCode2 from '@lucide/svelte/icons/file-code-2';
  import MessageSquare from '@lucide/svelte/icons/message-square';
  import Play from '@lucide/svelte/icons/play';
  import TextCursorInput from '@lucide/svelte/icons/text-cursor-input';
  import Users from '@lucide/svelte/icons/users';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Chip } from '$lib/components/ui/chip/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { HoverActionButton } from '$lib/components/ui/hover-actions/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { ListRow } from '$lib/components/ui/list-row/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import { AGENT_ICONS, agentDisplayName } from '$lib/shell/agentIcons.ts';
  import { normalizeProvider } from '$lib/shell/ownedSessions.ts';
  import { exactLocalTime, formatLastActivity } from '$lib/shell/relativeTime.ts';
  import type { SessionLibraryRecord } from '$lib/shell/sessionLibrary/sessionLibraryModel.ts';
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
    /** The clock, passed down so forty cards do not keep forty of them. */
    now: Date;
    onToggle(): void;
    onAction(id: SessionHistoryActionId): void;
    /** Put a block of the transcript on the clipboard. */
    onCopyText(value: string): void;
    /** Open one of a session's agents' own transcripts. */
    onOpenLog(path: string): void;
  }

  let { record, title, excerpt, expanded, now, onToggle, onAction, onCopyText, onOpenLog }: Props =
    $props();

  let menuOpen = $state(false);

  const actions = $derived(sessionHistoryActions(record));
  const byId = $derived(new Map(actions.map((action) => [action.id, action])));

  function action(id: SessionHistoryActionId): SessionHistoryAction {
    // Every id in the union is in the roster, so this only guards a typo.
    return byId.get(id) ?? { id, label: id, enabled: false, disabledReason: null, destructive: false };
  }

  const menuActions = $derived(SESSION_HISTORY_MENU_ACTION_IDS.map(action));

  const agent = $derived(normalizeProvider(record.provider));
  const ProviderIcon = $derived(AGENT_ICONS[agent.agent]);
  const providerName = $derived(agentDisplayName(agent.agent, agent.viaCmux));
  const age = $derived(formatLastActivity(record.updatedAt, now));
  const messageLabel = $derived(
    record.messageCount && record.messageCount > 0 ? `${record.messageCount} msgs` : ''
  );
  const subagentLabel = $derived(
    record.subagents.length > 0 ? `${record.subagents.length} subagents` : ''
  );
  const userTurns = $derived(record.latestTurns.filter((turn) => turn.text.trim().length > 0));

  function run(id: SessionHistoryActionId): void {
    if (!action(id).enabled) return;
    onAction(id);
  }
</script>

<article
  class={cn('session-card border-b px-1 py-1.5 last:border-b-0', menuOpen && 'menu-open')}
  data-testid="session-history-card"
>
  <ListRow
    onclick={onToggle}
    selected={expanded}
    actionsLabel="Session actions"
    class="pr-24"
  >
    <span class="min-w-0 flex-1 truncate font-medium">{title}</span>
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

  <p class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5 px-2 pt-0.5 text-sm text-muted-foreground">
    <ProviderIcon class="size-3.5 shrink-0" aria-hidden="true" />
    <span class="shrink-0">{providerName}</span>
    {#if messageLabel}<Chip tone="count">{messageLabel}</Chip>{/if}
    {#if subagentLabel}<Chip tone="count">{subagentLabel}</Chip>{/if}
    {#if age}<span class="shrink-0" title={exactLocalTime(record.updatedAt)}>{age}</span>{/if}
    {#if record.model}<span class="min-w-0 truncate">{record.model}</span>{/if}
  </p>

  {#if !expanded && excerpt}
    <p class="truncate px-2 pt-0.5 text-sm text-muted-foreground">{excerpt}</p>
  {/if}

  {#if expanded}
    <div class="flex flex-col gap-3 px-2 pt-2 pb-1">
      <div class="flex flex-wrap items-center gap-2">
        <Button
          size="sm"
          variant="secondary"
          disabled={!action('resume-assembly').enabled}
          title={action('resume-assembly').disabledReason ?? undefined}
          onclick={() => run('resume-assembly')}
        >
          <Play aria-hidden="true" />
          Resume as Assembly Session
        </Button>
        <Button
          size="sm"
          variant="ghost"
          disabled={!action('view-log').enabled}
          title={action('view-log').disabledReason ?? undefined}
          onclick={() => run('view-log')}
        >
          <FileCode2 aria-hidden="true" />
          View Log
        </Button>
      </div>

      {#if record.firstPrompt}
        <section class="flex flex-col gap-1">
          <h3 class="flex items-center gap-1.5 text-xs font-medium tracking-wide text-muted-foreground uppercase">
            <TextCursorInput class="size-3" aria-hidden="true" />
            First prompt
          </h3>
          <div class="rounded-lg border bg-background p-2">
            <div class="flex items-start gap-2">
              <ScrollArea class="first-prompt min-w-0 flex-1">
                <p class="text-[13px] leading-relaxed whitespace-pre-wrap text-foreground">
                  {record.firstPrompt}
                </p>
              </ScrollArea>
              <Button
                size="xs"
                variant="ghost"
                onclick={() => onCopyText(record.firstPrompt ?? '')}
              >
                <Copy aria-hidden="true" />
                Copy
              </Button>
            </div>
          </div>
        </section>
      {/if}

      {#if userTurns.length > 0}
        <section class="flex flex-col gap-2">
          <h3 class="flex items-center gap-1.5 text-xs font-medium tracking-wide text-muted-foreground uppercase">
            <MessageSquare class="size-3" aria-hidden="true" />
            Latest turns
          </h3>
          {#each userTurns as turn, index (index)}
            <!--
              Each turn scrolls inside its own box. An agent's last answer runs
              to thousands of characters, and letting one of them grow to its
              full height pushed the next session off the panel entirely — the
              two turns are here to be compared, so both have to stay on screen.
            -->
            <div class="turn-block overflow-hidden rounded-lg border bg-background">
              <p
                class="flex items-center gap-1.5 border-b px-2 py-1 text-[11px] font-semibold
                       tracking-wide uppercase"
                class:speaker-user={turn.speaker === 'user'}
                class:speaker-agent={turn.speaker !== 'user'}
              >
                {turn.speaker === 'user' ? 'You' : 'Agent'}
              </p>
              <ScrollArea>
                <p class="px-2 py-1.5 text-[13px] leading-relaxed whitespace-pre-wrap text-foreground">
                  {turn.text}
                </p>
              </ScrollArea>
            </div>
          {/each}
        </section>
      {/if}

      {#if record.subagents.length > 0}
        <section class="flex flex-col gap-1">
          <h3 class="flex items-center gap-1.5 text-xs font-medium tracking-wide text-muted-foreground uppercase">
            <Users class="size-3" aria-hidden="true" />
            Subagents ({record.subagents.length})
          </h3>
          {#each record.subagents as subagent, index (index)}
            <ListRow class="bg-background" actionsLabel="Agent actions">
              <span class="min-w-0 flex-1 truncate">{subagent.name}</span>
              {#if subagent.kind}<Chip>{subagent.kind}</Chip>{/if}
              {#if subagent.messageCount}
                <span class="shrink-0 text-sm text-muted-foreground">{subagent.messageCount} msgs</span>
              {/if}
              {#snippet actions()}
                <HoverActionButton
                  label={subagent.logPath ? 'View log' : 'No transcript file was found for this agent.'}
                  tone="info"
                  disabled={!subagent.logPath}
                  onclick={() => subagent.logPath && onOpenLog(subagent.logPath)}
                >
                  <FileCode2 />
                </HoverActionButton>
              {/snippet}
            </ListRow>
          {/each}
        </section>
      {/if}
    </div>
  {/if}
</article>

<style>
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
    /* The viewport ships as `height: 100%`, and its parent has no height of its
       own, so it measured zero and there was nothing to scroll. Letting it size
       to the turn and capping it there is what gives the scrollbar something to
       do. */
    height: auto;
    max-height: 11rem;
  }

  :global(.first-prompt) :global([data-slot='scroll-area-viewport']) {
    height: auto;
    max-height: 10rem;
  }
</style>
