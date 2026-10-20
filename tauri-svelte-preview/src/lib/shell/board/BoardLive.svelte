<!-- The Board's right column, built from live state: what waits on the owner, workflow runs and busy sessions. -->
<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import CircleStop from '@lucide/svelte/icons/circle-stop';
  import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Chip } from '$lib/components/ui/chip/index.js';
  import { HoverActionButton, HoverActions } from '$lib/components/ui/hover-actions/index.js';
  import { sessionWaitsOnYou, type OwnedSession } from '$lib/shell/ownedSessions.ts';
  import { projectMachineLabel } from '$lib/shell/projects/projects.ts';
  import { rail } from '$lib/shell/stores/sessionRailStore.svelte.ts';
  import { answerInboxStep, waitingOn, type InboxDecision } from '$lib/shell/workflows/ownerInboxActions.ts';
  import BoardNotes from './BoardNotes.svelte';
  import type { BoardNote, BoardStep, MachineBoard } from './boardSource.ts';

  interface Props {
    boards: MachineBoard[];
    /** Sessions waiting on the owner, from the rail. */
    sessions: OwnedSession[];
    /** Notion Tasks is connected, so task keys show. */
    notion: boolean;
    /** Aborted when the board closes. */
    signal: AbortSignal;
    onError(text: string): void;
    onOpenSession(ownedId: string): void;
  }
  let { boards, sessions, notion, signal, onError, onOpenSession }: Props = $props();

  const HEADING = 'flex items-center gap-2 px-1 text-[13px] font-semibold';
  const ROW = 'flex min-w-0 flex-col gap-1 rounded-lg bg-muted/50 px-3 py-2';
  let busy = $state(false);
  /** The live row whose note field is showing. */
  let adding = $state<string | null>(null);

  const machineName = (machine: string): string => projectMachineLabel(machine, rail.remoteProfiles);
  const sessionMachine = (session: OwnedSession): string =>
    machineName(session.executionEnvironment === 'remote' ? (session.remoteProfileId ?? '') : 'local');
  const steps = $derived(boards.flatMap((board) => board.value.steps.map((step) => ({ machine: board.machine, step }))));
  const runs = $derived(boards.flatMap((board) => board.value.runs.map((run) => ({ machine: board.machine, run }))));
  /** Top-level sessions at work or waiting, each with its helpers not yet marked done, under their machine. */
  const machines = $derived.by(() => {
    const groups = new Map<string, { session: OwnedSession; helpers: OwnedSession[] }[]>();
    for (const session of rail.owned) {
      if (session.helperOf || session.runId) continue;
      if (!sessionWaitsOnYou(session) && session.runtimeState !== 'working' && session.runtimeState !== 'interrupting') continue;
      const helpers = rail.owned.filter((helper) => helper.helperOf === session.ownedId && !helper.completedAt && !helper.settledAt);
      const machine = sessionMachine(session);
      groups.set(machine, [...(groups.get(machine) ?? []), { session, helpers }]);
    }
    return [...groups];
  });

  /** Notes on a session live on the machine that runs it. */
  const sessionBoard = (session: OwnedSession): string =>
    session.executionEnvironment === 'remote' ? (session.remoteProfileId ?? '') : 'mac';
  const sessionNotes = (machine: string, target: string): BoardNote[] =>
    boards.find((board) => board.machine === machine)?.value.session_notes.find((item) => item.target === target)?.notes ?? [];

  function status(session: OwnedSession): string {
    return session.runtimeState === 'working' || session.runtimeState === 'interrupting' ? 'Working' : 'Idle';
  }

  function message(cause: unknown): string {
    return cause instanceof Error ? cause.message : String(cause);
  }

  async function act(label: string, machine: string, step: BoardStep, decision: InboxDecision): Promise<void> {
    busy = true;
    onError('');
    try {
      await answerInboxStep({ machine, runId: step.run_id, nodeId: step.node_id }, decision);
    } catch (cause) {
      // The step may have been answered elsewhere; the change signal shows what is waiting now.
      if (!signal.aborted) onError(`${label}: ${message(cause)}`);
    }
    if (!signal.aborted) busy = false;
  }
</script>

{#snippet addNote(target: string)}
  <HoverActions class="absolute top-1 right-1.5" label="Note actions">
    <HoverActionButton label="Add note" size="xs" onclick={() => (adding = target)}><MessageSquarePlus /></HoverActionButton>
  </HoverActions>
{/snippet}

{#snippet notesFor(notes: BoardNote[], machine: string, target: string)}
  {#if notes.length || adding === target}
    <div class="flex min-w-0 flex-col gap-1 pt-1">
      <BoardNotes {notes} {machine} {target} {signal} adding={adding === target} {onError} onDone={() => (adding = null)} />
    </div>
  {/if}
{/snippet}

{#snippet sessionRow(session: OwnedSession)}
  {@const machine = sessionBoard(session)}
  {@const target = `session:${session.ownedId}`}
  <div class="group relative">
    <button
      type="button"
      class="flex w-full min-w-0 items-center gap-2 rounded-md px-2 py-1.5 text-left hover:bg-accent/60 focus-visible:ring-3 focus-visible:ring-ring/50"
      onclick={() => onOpenSession(session.ownedId)}
    >
      <span class="min-w-0 flex-1 truncate text-[13px] font-medium">{session.title}</span>
      {#if sessionWaitsOnYou(session)}
        <Chip tone="attention">{waitingOn(session)}</Chip>
      {:else}
        <span class="flex-none text-xs text-muted-foreground">{status(session)}</span>
      {/if}
    </button>
    {@render addNote(target)}
  </div>
  <div class="px-2">{@render notesFor(sessionNotes(machine, target), machine, target)}</div>
{/snippet}

<div class="flex min-h-0 min-w-0 flex-col gap-4 overflow-y-auto border-l border-border p-3" data-testid="board-live">
  <section class="flex min-w-0 flex-col gap-2">
    <h3 class={HEADING}>Messages for you <span class="font-normal text-muted-foreground tabular-nums">{steps.length + sessions.length}</span></h3>
    {#if steps.length === 0 && sessions.length === 0}
      <p class="px-1 text-[13px] text-muted-foreground">Nothing is waiting for you.</p>
    {/if}
    <ul class="flex flex-col gap-2">
      {#each steps as { machine, step } (`${machine}:${step.run_id}:${step.node_id}`)}
        <li class={`${ROW} group relative`}>
          {@render addNote(`step:${step.run_id}/${step.node_id}`)}
          <span class="flex min-w-0 items-center gap-2 text-sm">
            <span class="min-w-0 flex-1 truncate font-medium">
              {[notion ? step.task_key : null, step.workflow_name].filter(Boolean).join(' · ') || step.run_id}
            </span>
            <Chip tone="attention">{step.state === 'waiting-approval' ? 'Approval' : 'Answer'}</Chip>
          </span>
          <span class="truncate text-[13px] text-muted-foreground">{step.node_id} · {machineName(machine === 'mac' ? 'local' : machine)}</span>
          {#if step.reason}<span class="line-clamp-3 text-[13px]" title={step.reason}>{step.reason}</span>{/if}
          <span class="flex flex-wrap gap-1.5 pt-1">
            {#if step.state === 'waiting-approval'}
              <Button size="sm" disabled={busy} onclick={() => void act('Approve', machine, step, 'approve')}><Check />Approve</Button>
            {:else}
              <Button size="sm" disabled={busy} onclick={() => void act('Another round', machine, step, 'another-round')}><RotateCcw />Another round</Button>
              <Button size="sm" variant="outline" disabled={busy} onclick={() => void act('Accept', machine, step, 'accept')}><Check />Accept</Button>
              <Button size="sm" variant="ghost" disabled={busy} onclick={() => void act('Stop', machine, step, 'stop')}><CircleStop />Stop</Button>
            {/if}
            {#if step.owned_id}
              {@const ownedId = step.owned_id}
              <Button size="sm" variant="ghost" onclick={() => onOpenSession(ownedId)}>Open</Button>
            {/if}
          </span>
          {@render notesFor(step.notes, machine, `step:${step.run_id}/${step.node_id}`)}
        </li>
      {/each}
      {#each sessions as session (session.ownedId)}
        <li class="rounded-lg bg-muted/50 py-0.5">{@render sessionRow(session)}</li>
      {/each}
    </ul>
  </section>

  {#if runs.length}
    <section class="flex min-w-0 flex-col gap-2">
      <h3 class={HEADING}>Workflow runs <span class="font-normal text-muted-foreground tabular-nums">{runs.length}</span></h3>
      <ul class="flex flex-col gap-2">
        {#each runs as { machine, run } (`${machine}:${run.id}`)}
          <li class={`${ROW} group relative`}>
            {@render addNote(`run:${run.id}`)}
            <span class="flex min-w-0 items-center gap-2 text-sm">
              <span class="min-w-0 truncate font-medium">{run.name ?? run.id}</span>
              {#if notion && run.task_key}<span class="flex-none text-[13px] text-muted-foreground">{run.task_key}</span>{/if}
              <span class="ml-auto flex-none text-[13px] text-muted-foreground tabular-nums">{run.steps_done} / {run.steps_total}</span>
            </span>
            <span class="truncate text-[13px] text-muted-foreground">
              {[run.current_step ? `Step ${run.current_step}` : null, run.state, run.round !== null ? `round ${run.round}` : null,
                run.findings !== null ? `${run.findings} found` : null, machineName(machine === 'mac' ? 'local' : machine)].filter(Boolean).join(' · ')}
            </span>
            {@render notesFor(run.notes, machine, `run:${run.id}`)}
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if machines.length}
    <section class="flex min-w-0 flex-col gap-1">
      <h3 class={HEADING}>Sessions <span class="font-normal text-muted-foreground tabular-nums">{machines.reduce((total, [, rows]) => total + rows.length, 0)}</span></h3>
      {#each machines as [machine, rows] (machine)}
        <h4 class="px-1 pt-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">{machine}</h4>
        <ul class="flex flex-col">
          {#each rows as { session, helpers } (session.ownedId)}
            <li>
              {@render sessionRow(session)}
              {#if helpers.length}
                <ul class="ml-3 border-l border-border pl-2">
                  {#each helpers as helper (helper.ownedId)}<li>{@render sessionRow(helper)}</li>{/each}
                </ul>
              {/if}
            </li>
          {/each}
        </ul>
      {/each}
    </section>
  {/if}
</div>
