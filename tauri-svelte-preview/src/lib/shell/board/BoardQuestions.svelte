<!-- The Board's Questions panel: what agents asked the owner in chat, answered here or there. -->
<script lang="ts">
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import ExternalLink from '@lucide/svelte/icons/external-link';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { projectMachineLabel } from '$lib/shell/projects/projects.ts';
  import { rail } from '$lib/shell/stores/sessionRailStore.svelte.ts';
  import { cn } from '$lib/utils.js';
  import { answerQuestion, questionState } from '../workflows/ownerInboxActions.ts';
  import type { BoardQuestion, MachineBoard } from './boardSource.ts';

  interface Props {
    boards: MachineBoard[];
    /** Answered questions show only under All. */
    filter: 'all' | 'needs_you';
    /** Aborted when the board closes. */
    signal: AbortSignal;
    onError(text: string): void;
    onOpenSession(ownedId: string): void;
  }
  let { boards, filter, signal, onError, onOpenSession }: Props = $props();

  type Row = { id: string; machine: string; q: BoardQuestion };
  const rows = $derived(boards.flatMap((board) =>
    (board.value.questions ?? []).map((q) => ({ id: `${board.machine}:${q.id}`, machine: board.machine, q }))));
  const open = $derived(rows.filter((row) => row.q.answered_at_ms === null));
  const answered = $derived(rows.filter((row) => row.q.answered_at_ms !== null));
  const showAnswered = $derived(filter === 'all' && answered.length > 0);
  const ended = (row: Row): boolean => row.q.session_state === 'closed' || row.q.session_state === 'failed';
  const liveOpen = $derived(open.filter((row) => !ended(row)));

  /** Rows the owner opened or shut; an open question starts expanded. */
  let toggled = $state<Record<string, boolean>>({});
  let drafts = $state<Record<string, string>>({});
  let answeredOpen = $state(false);

  const isOpen = (row: Row): boolean => toggled[row.id] ?? row.q.answered_at_ms === null;
  const machineLabel = (machine: string): string => projectMachineLabel(machine === 'mac' ? 'local' : machine, rail.remoteProfiles);

  function when(ms: number): string {
    const date = new Date(ms);
    return date.toDateString() === new Date().toDateString()
      ? date.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })
      : date.toLocaleDateString([], { month: 'short', day: 'numeric' });
  }

  /** Sends "Q<n>: <text>" to the question's session; `record` false leaves it open. */
  async function send(row: Row, text: string, record = true): Promise<void> {
    onError('');
    try {
      await answerQuestion({ machine: row.machine, id: row.q.id, ownedId: row.q.session_owned_id }, text, signal, record);
      if (record) drafts[row.id] = '';
    } catch (cause) {
      if (!signal.aborted) onError(`Q${row.q.id}: ${cause instanceof Error ? cause.message : String(cause)}`);
    }
  }

  function keydown(event: KeyboardEvent, row: Row): void {
    const text = drafts[row.id]?.trim();
    if (event.key === 'Enter' && text) void send(row, text);
  }
</script>

{#snippet questionRow(row: Row)}
  {@const q = row.q}
  {@const expanded = isOpen(row)}
  {@const status = questionState(row.id)}
  {@const locked = status.busy || ended(row)}
  <li class="card">
    <button type="button" class="chead" aria-expanded={expanded} onclick={() => (toggled[row.id] = !expanded)}>
      <ChevronRight class={cn('size-3.5 shrink-0 text-(--color-text-3) transition-transform', expanded && 'rotate-90')} aria-hidden="true" />
      <span class="key">Q{q.id}</span>
      <span class="ctitle">{q.title}</span>
      <span class="ctime">{q.answered_at_ms === null ? when(q.asked_at_ms) : `Answered ${when(q.answered_at_ms)}`}</span>
    </button>
    <p class="sum one">{[q.session_name || 'Session', boards.length > 1 ? machineLabel(row.machine) : '', ended(row) ? 'Session ended' : ''].filter(Boolean).join(' · ')}</p>
    {#if q.answered_at_ms !== null}<p class="sum one">You answered: {q.answer}</p>{/if}
    {#if expanded}
      <div class="detail">
        {#if q.body}<p class="sum">{q.body}</p>{/if}
        {#if q.answered_at_ms === null}
          {#if q.recommended}<div class="well"><span>Recommended</span><p>{q.recommended}</p></div>{/if}
          {#if status.unrecorded}
            <p class="sum">Sent, but not recorded.</p>
            <div class="actions">
              <Button size="sm" variant="secondary" disabled={status.busy} onclick={() => void send(row, status.unrecorded ?? '')}>Retry</Button>
              <Button size="sm" variant="secondary" onclick={() => onOpenSession(q.session_owned_id)}><ExternalLink />Open</Button>
            </div>
          {:else}
            <Input
              bind:value={drafts[row.id]}
              maxlength={600}
              placeholder="Type an answer…"
              aria-label={`Answer Q${q.id}`}
              disabled={locked}
              onkeydown={(event) => keydown(event, row)}
              class="h-[30px] rounded-(--radius-sm) border-0 bg-[color-mix(in_srgb,var(--color-bg)_44%,transparent)] text-[13px] dark:bg-[color-mix(in_srgb,var(--color-bg)_44%,transparent)]"
            />
            <div class="actions">
              {#if q.recommended}
                {@const recommended = q.recommended}
                <Button size="sm" variant="secondary" disabled={locked} onclick={() => void send(row, recommended)}>As recommended</Button>
              {/if}
              <Button size="sm" variant="secondary" disabled={locked} onclick={() => void send(row, 'no')}>No</Button>
              <Button size="sm" variant="secondary" disabled={locked} onclick={() => void send(row, 'explain more', false)}>Explain more</Button>
              <Button size="sm" variant="secondary" onclick={() => onOpenSession(q.session_owned_id)}><ExternalLink />Open</Button>
            </div>
          {/if}
        {:else}
          <div class="actions">
            <Button size="sm" variant="secondary" onclick={() => onOpenSession(q.session_owned_id)}><ExternalLink />Open</Button>
          </div>
        {/if}
      </div>
    {/if}
  </li>
{/snippet}

{#if open.length || showAnswered}
  <section class="panel" data-testid="board-questions">
    <div class="ph"><h3><i class="dot bg-(--color-attention)"></i>Questions</h3><span class="muted">Agents are waiting on these</span><strong>{liveOpen.length}</strong></div>
    {#if open.length}
      <ul class="list">
        {#each open as row (row.id)}{@render questionRow(row)}{/each}
      </ul>
    {/if}
    {#if showAnswered}
      <div class="flex items-center gap-2 pt-1.5">
        <h3 class="min-w-0 flex-1">
          <button type="button" class="done-toggle" aria-expanded={answeredOpen} onclick={() => (answeredOpen = !answeredOpen)}>
            <i class="dot bg-(--color-good)"></i>Answered today
            <span class="ml-auto font-normal text-(--color-text-2) tabular-nums">{answered.length}</span>
            <ChevronDown class={cn('size-3.5 text-(--color-text-2) transition-transform', answeredOpen && 'rotate-180')} aria-hidden="true" />
          </button>
        </h3>
      </div>
      {#if answeredOpen}
        <ul class="list pt-1">
          {#each answered as row (row.id)}{@render questionRow(row)}{/each}
        </ul>
      {/if}
    {/if}
  </section>
{/if}

<style>
  /* Copied from BoardCards.svelte so the Questions panel matches the card sections. */
  .panel { flex: none; padding: 8px; border: 1px solid color-mix(in srgb, var(--color-border) 36%, transparent); border-radius: var(--radius-lg); background: color-mix(in srgb, var(--color-elevated) 38%, var(--color-surface)); }
  .ph { display: flex; align-items: baseline; gap: 8px; min-width: 0; padding: 1px 4px 7px; }
  h3 { margin: 0; display: inline-flex; align-items: center; gap: 7px; font-size: 0.88rem; line-height: 1.35; font-weight: 600; white-space: nowrap; }
  .ph strong { margin-left: auto; font-size: 0.88rem; font-variant-numeric: tabular-nums; }
  .muted { margin: 0; min-width: 0; overflow: hidden; color: var(--color-text-2); font-size: 0.76rem; line-height: 1.45; text-overflow: ellipsis; }
  .ph .muted { white-space: nowrap; }
  .dot { display: inline-block; flex: none; width: 8px; height: 8px; border-radius: 2px; }
  .list { display: flex; flex-direction: column; margin: 0; padding: 0; list-style: none; gap: 6px; }
  .card { min-width: 0; padding: 9px 12px; border-radius: var(--radius-md); background: color-mix(in srgb, var(--color-elevated) 52%, var(--color-surface)); }
  .chead, .done-toggle { display: flex; align-items: center; gap: 8px; width: 100%; min-width: 0; border-radius: var(--radius-sm); text-align: left; }
  .done-toggle { padding: 2px 4px; font: inherit; font-weight: 600; }
  .chead:focus-visible, .done-toggle:focus-visible { outline: none; box-shadow: var(--focus-ring); }
  .key { flex: none; padding: 1px 6px; border-radius: var(--radius-xs); background: color-mix(in srgb, var(--color-bg) 44%, transparent); color: var(--color-text-2); font-size: 12px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .ctitle { min-width: 0; overflow: hidden; font-size: 14px; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }
  .ctime { flex: none; margin-left: auto; color: var(--color-text-3); font-size: 0.75rem; font-variant-numeric: tabular-nums; }
  .sum { margin: 3px 0 0; color: var(--color-text-2); font-size: 13px; line-height: 1.45; white-space: pre-wrap; }
  .sum.one { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .detail { display: flex; flex-direction: column; gap: 8px; margin-top: 8px; }
  .actions { display: flex; flex-wrap: wrap; gap: 6px; }
  .well { display: grid; gap: 2px; padding: 7px 10px; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--color-bg) 48%, transparent); }
  .well > span { font-size: 0.75rem; font-weight: 600; }
  .well p { margin: 0; color: var(--color-text-2); font-size: 13px; line-height: 1.45; white-space: pre-wrap; }
</style>
