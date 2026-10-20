<!-- The Board's left column: the cards agents write, in sections by state. -->
<script lang="ts">
  import AlarmClock from '@lucide/svelte/icons/alarm-clock';
  import Check from '@lucide/svelte/icons/check';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Trash2 from '@lucide/svelte/icons/trash-2';

  import { Button, buttonVariants } from '$lib/components/ui/button/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { projectMachineLabel } from '$lib/shell/projects/projects.ts';
  import { rail } from '$lib/shell/stores/sessionRailStore.svelte.ts';
  import { openUrlInBrowser } from '$lib/shell/workbenchNavigation.ts';
  import { cn } from '$lib/utils.js';
  import BoardNotes from './BoardNotes.svelte';
  import { changeBoardCard, type BoardCard, type BoardCardAction, type MachineBoard } from './boardSource.ts';

  interface Props {
    boards: MachineBoard[];
    /** Notion Tasks is connected, so task keys and links show. */
    notion: boolean;
    /** Aborted when the board closes. */
    signal: AbortSignal;
    onError(text: string): void;
    onClose(): void;
    onOpenSession(ownedId: string): void;
  }
  let { boards, notion, signal, onError, onClose, onOpenSession }: Props = $props();

  type Entry = { id: string; machine: string; card: BoardCard };
  const SECTIONS = [
    { state: 'needs_you', label: 'Needs you', dot: 'bg-(--color-attention)' },
    { state: 'running', label: 'Running', dot: 'bg-(--color-live)' },
    { state: 'in_review', label: 'In review', dot: 'bg-(--color-merged)' }
  ] as const;
  const HEADING = 'flex items-center gap-2 px-1 pt-1 text-[13px] font-semibold';
  const LABEL = 'mb-1.5 text-xs font-semibold tracking-wide text-muted-foreground uppercase';
  const LINK = 'min-w-0 truncate text-left underline underline-offset-2 hover:text-foreground';

  /** Cards dropped while the board is open; each stays as "Dropped · Undo" until it closes. */
  let dropped = $state<Entry[]>([]);
  /** Cards the owner opened or shut; a Needs you card starts open. */
  let toggled = $state<Record<string, boolean>>({});
  let doneOpen = $state(false);
  let busy = $state(false);

  const entries = $derived(boards.flatMap((board) =>
    board.value.cards.map((card) => ({ id: `${board.machine}:${card.key}`, machine: board.machine, card }))));
  const done = $derived(inState('done'));

  const snoozed = (card: BoardCard): boolean => card.snoozed_until_ms !== null && card.snoozed_until_ms > Date.now();
  const isOpen = (entry: Entry): boolean => toggled[entry.id] ?? entry.card.state === 'needs_you';
  const isDropped = (entry: Entry): boolean => dropped.some((other) => other.id === entry.id);
  const machineLabel = (machine: string): string => projectMachineLabel(machine === 'mac' ? 'local' : machine, rail.remoteProfiles);
  const sessionTitle = (ownedId: string): string => rail.owned.find((session) => session.ownedId === ownedId)?.title || 'Session';

  /** A section's cards across machines: snoozed ones last, then newest first. */
  function inState(state: BoardCard['state']): Entry[] {
    const live = entries.filter((entry) => entry.card.state === state && !isDropped(entry));
    return [...live, ...dropped.filter((entry) => entry.card.state === state)].sort((a, b) =>
      Number(snoozed(a.card)) - Number(snoozed(b.card)) || b.card.updated_at_ms - a.card.updated_at_ms);
  }

  /** A state's cards on every machine, past the 40-card limit, less those dropped before the reload. */
  function total(state: BoardCard['state']): number {
    return boards.reduce((sum, board) => sum + (board.value.counts[state] ?? 0), 0)
      - entries.filter((entry) => entry.card.state === state && isDropped(entry)).length;
  }

  /** Cards the machines counted but did not send. */
  function notShown(state: BoardCard['state'], list: Entry[]): number {
    return total(state) - list.filter((entry) => !isDropped(entry)).length;
  }

  function when(ms: number): string {
    const date = new Date(ms);
    return date.toDateString() === new Date().toDateString()
      ? date.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })
      : date.toLocaleDateString([], { month: 'short', day: 'numeric' });
  }

  function tomorrowAtNine(): number {
    const date = new Date();
    date.setDate(date.getDate() + 1);
    date.setHours(9, 0, 0, 0);
    return date.getTime();
  }

  function message(cause: unknown): string {
    return cause instanceof Error ? cause.message : String(cause);
  }

  /** Owner actions go to the machine that owns the card; the change signal reloads the board. */
  async function act(entry: Entry, action: BoardCardAction, at = Date.now()): Promise<void> {
    busy = true;
    onError('');
    try {
      const changed = await changeBoardCard(entry.machine, entry.card.key, action, at, signal);
      if (signal.aborted) return;
      if (action === 'drop' && changed) dropped = [...dropped, entry];
      if (action === 'undo') dropped = dropped.filter((other) => other.id !== entry.id);
    } catch (cause) {
      if (!signal.aborted) onError(`Could not change "${entry.card.title}": ${message(cause)}`);
    }
    if (!signal.aborted) busy = false;
  }

  async function clearDone(): Promise<void> {
    busy = true;
    onError('');
    try {
      for (const machine of new Set(done.map((entry) => entry.machine))) {
        await changeBoardCard(machine, '', 'clear_done', Date.now(), signal);
      }
    } catch (cause) {
      if (!signal.aborted) onError(`Could not clear done cards: ${message(cause)}`);
    }
    if (!signal.aborted) busy = false;
  }

  function follow(url: string): void {
    onClose();
    void openUrlInBrowser({ url });
  }
</script>

{#snippet heading(label: string, count: number, dot: string)}
  <span class={`size-1.5 flex-none rounded-full ${dot}`}></span>{label}
  <span class="font-normal text-muted-foreground tabular-nums">{count}</span>
{/snippet}

{#snippet more(count: number)}
  {#if count > 0}<p class="px-1 text-[13px] text-muted-foreground">{count} more not shown. Mark cards done or drop them to see the rest.</p>{/if}
{/snippet}

{#snippet cardRow(entry: Entry)}
  {@const card = entry.card}
  {#if isDropped(entry)}
    <li class="flex min-w-0 items-center gap-2 rounded-lg px-3 py-1.5 text-[13px] text-muted-foreground">
      <span class="min-w-0 flex-1 truncate">{card.title}</span>
      Dropped ·
      <Button size="xs" variant="ghost" disabled={busy} onclick={() => void act(entry, 'undo')}>Undo</Button>
    </li>
  {:else if snoozed(card)}
    <li class="flex min-w-0 items-center gap-2 rounded-lg px-3 py-2 text-[13px] text-muted-foreground">
      <span class="min-w-0 flex-1 truncate">{card.title}</span>
      Snoozed until {when(card.snoozed_until_ms ?? 0)}
    </li>
  {:else}
    {@const open = isOpen(entry)}
    <li class="flex min-w-0 flex-col gap-1 rounded-lg bg-muted/50 px-3 py-2.5">
      <button
        type="button"
        class="flex w-full min-w-0 items-center gap-2 rounded-md text-left focus-visible:ring-3 focus-visible:ring-ring/50"
        aria-expanded={open}
        onclick={() => (toggled[entry.id] = !open)}
      >
        <ChevronRight class={cn('size-4 shrink-0 text-muted-foreground transition-transform', open && 'rotate-90')} aria-hidden="true" />
        <span class="min-w-0 flex-1 truncate text-(length:--text-body) font-semibold">{card.title}</span>
        {#if notion && card.task_key}<span class="flex-none text-[13px] text-muted-foreground">{card.task_key}</span>{/if}
        <span class="flex-none text-[13px] text-muted-foreground">{when(card.updated_at_ms)}</span>
      </button>
      {#if card.summary}<p class={cn('pl-6 text-[13px] text-muted-foreground', !open && 'truncate')}>{card.summary}</p>{/if}
      {#if card.stale}<p class="pl-6 text-[13px] text-muted-foreground">No update since {when(card.updated_at_ms)}</p>{/if}
      {#if open}
        <div class="flex min-w-0 flex-col gap-3 pt-2 pl-6">
          {#if card.steps.length}
            <section>
              <h4 class={LABEL}>What to do</h4>
              <ol class="m-0 flex list-none flex-col gap-1.5 p-0">
                {#each card.steps as step, index (index)}
                  <li class="flex gap-2 text-[13px]">
                    <span class="mt-px inline-flex size-5 flex-none items-center justify-center rounded-full bg-secondary text-xs tabular-nums">{index + 1}</span>
                    <span class="min-w-0">{step}</span>
                  </li>
                {/each}
              </ol>
            </section>
          {/if}
          {#if card.note}
            <section>
              <h4 class={LABEL}>Note for you</h4>
              <p class="text-[13px] whitespace-pre-wrap">{card.note}</p>
            </section>
          {/if}
          <section class="flex flex-col gap-1.5">
            <h4 class={LABEL}>Your notes</h4>
            <BoardNotes notes={card.notes} machine={entry.machine} target={`card:${card.key}`} {signal} adding {onError} />
          </section>
          {#if card.pr_url || (notion && card.task_url) || card.run_id || card.session_owned_id}
            <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-1 text-[13px]">
              {#if card.pr_url}
                {@const url = card.pr_url}
                <dt class="text-muted-foreground">Pull request</dt>
                <dd class="flex min-w-0"><button type="button" class={LINK} onclick={() => follow(url)}>{url.replace(/^https?:\/\//, '')}</button></dd>
              {/if}
              {#if notion && card.task_url}
                {@const url = card.task_url}
                <dt class="text-muted-foreground">Task</dt>
                <dd class="flex min-w-0"><button type="button" class={LINK} onclick={() => follow(url)}>{card.task_key ?? url}</button></dd>
              {/if}
              {#if card.run_id}
                <dt class="text-muted-foreground">Workflow run</dt>
                <dd class="truncate">{card.run_state ?? 'Not found'}{card.run_step ? ` · ${card.run_step}` : ''}</dd>
              {/if}
              {#if card.session_owned_id}
                {@const ownedId = card.session_owned_id}
                <dt class="text-muted-foreground">Session</dt>
                <dd class="flex min-w-0 gap-1">
                  <button type="button" class={LINK} onclick={() => onOpenSession(ownedId)}>{sessionTitle(ownedId)}</button>
                  <span class="flex-none text-muted-foreground">· {card.session_state ?? 'Closed'}</span>
                </dd>
              {/if}
            </dl>
          {/if}
          <p class="truncate text-[13px] text-muted-foreground">Written by {card.writer_name ?? card.written_by} · {machineLabel(entry.machine)}</p>
          <div class="flex flex-wrap gap-2">
            {#if card.state !== 'done'}
              <Button size="sm" variant="outline" disabled={busy} onclick={() => void act(entry, 'done')}><Check />Mark done</Button>
            {/if}
            {#if card.state === 'needs_you'}
              <DropdownMenu.Root>
                <DropdownMenu.Trigger class={buttonVariants({ variant: 'ghost', size: 'sm' })} disabled={busy}><AlarmClock />Snooze</DropdownMenu.Trigger>
                <DropdownMenu.Content align="start" class="w-48">
                  <DropdownMenu.Item onSelect={() => void act(entry, 'snooze', Date.now() + 3_600_000)}>For 1 hour</DropdownMenu.Item>
                  <DropdownMenu.Item onSelect={() => void act(entry, 'snooze', tomorrowAtNine())}>Until tomorrow 9 AM</DropdownMenu.Item>
                </DropdownMenu.Content>
              </DropdownMenu.Root>
            {/if}
            <Button size="sm" variant="ghost" disabled={busy} onclick={() => void act(entry, 'drop')}><Trash2 />Drop</Button>
          </div>
        </div>
      {/if}
    </li>
  {/if}
{/snippet}

<div class="flex min-h-0 min-w-0 flex-col gap-4 overflow-y-auto p-3" data-testid="board-cards">
  {#if entries.length === 0 && dropped.length === 0}
    <p class="px-1 py-6 text-center text-[13px] text-muted-foreground">No cards yet. Agents post them with the <code>board_update</code> tool.</p>
  {/if}
  {#each SECTIONS as section (section.state)}
    {@const list = inState(section.state)}
    {#if list.length || total(section.state)}
      <section class="flex min-w-0 flex-col gap-2">
        <h3 class={HEADING}>{@render heading(section.label, total(section.state), section.dot)}</h3>
        <ul class="m-0 flex list-none flex-col p-0 gap-2">
          {#each list as entry (entry.id)}{@render cardRow(entry)}{/each}
        </ul>
        {@render more(notShown(section.state, list))}
      </section>
    {/if}
  {/each}
  {#if done.length || total('done')}
    <section class="flex min-w-0 flex-col gap-2">
      <div class="flex items-center gap-2">
        <h3 class="min-w-0 flex-1">
          <button
            type="button"
            class={cn(HEADING, 'rounded-md text-left focus-visible:ring-3 focus-visible:ring-ring/50')}
            aria-expanded={doneOpen}
            onclick={() => (doneOpen = !doneOpen)}
          >
            {@render heading('Done today', total('done'), 'bg-(--color-good)')}
            <ChevronRight class={cn('size-4 text-muted-foreground transition-transform', doneOpen && 'rotate-90')} aria-hidden="true" />
          </button>
        </h3>
        <Button size="xs" variant="ghost" disabled={busy} onclick={() => void clearDone()}>Clear</Button>
      </div>
      {#if doneOpen}
        <ul class="m-0 flex list-none flex-col p-0 gap-2">
          {#each done as entry (entry.id)}{@render cardRow(entry)}{/each}
        </ul>
        {@render more(notShown('done', done))}
      {/if}
    </section>
  {/if}
</div>
