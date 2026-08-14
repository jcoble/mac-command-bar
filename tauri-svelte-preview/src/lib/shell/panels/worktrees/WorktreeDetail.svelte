<script lang="ts">
  /**
   * WorktreeDetail.svelte — what is open under one worktree row.
   *
   * Three parts, and none of them is decided here. The plain facts about the
   * folder come from `worktreeFacts`; the verdict — what to do about this
   * worktree and why — comes from `worktreeSafety.ts` through the row; and the
   * three things that can be done about it come from `worktreeAgentPrompts.ts`.
   *
   * This used to end in three commands printed for the reader to copy into a
   * terminal themselves, because nothing in the app could be trusted to look
   * before it deleted. The same three commands are still what happens — they
   * are just carried now by a session that reads the folder first, says what it
   * found, warns about anything that would be lost, and only then acts. The
   * session shows up in the sessions list like any other, so the work is watched
   * rather than taken on trust.
   */
  import type { Component } from 'svelte';
  import Archive from '@lucide/svelte/icons/archive';
  import ScanSearch from '@lucide/svelte/icons/scan-search';
  import Trash2 from '@lucide/svelte/icons/trash-2';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Chip } from '$lib/components/ui/chip/index.js';
  import { exactLocalTime } from '$lib/shell/relativeTime';
  import { worktreeFacts, type WorktreeManagerRow } from '$lib/shell/worktrees/worktreeManagerRows';

  import type { WorktreeAgentAction, WorktreeAgentActionId } from './worktreeAgentPrompts.ts';

  interface Props {
    row: WorktreeManagerRow;
    /** The three agent actions for this row, already worked out. */
    actions: WorktreeAgentAction[];
    /** A session is being started for this row. */
    busy: boolean;
    /** Something is happening somewhere in the panel; every button waits. */
    panelBusy: boolean;
    onAction(id: WorktreeAgentActionId): void;
    /** Focus a session that worked here. Left out, the sessions are plain text. */
    onOpenSession?: (ownedId: string) => void;
  }
  let { row, actions, busy, panelBusy, onAction, onOpenSession }: Props = $props();

  const LANE_TONE: Record<string, 'bad' | 'attention' | 'live' | 'neutral' | 'good'> = {
    blocked: 'bad',
    backup: 'attention',
    cleanup: 'live',
    review: 'neutral',
    protected: 'good'
  };

  const ICONS: Record<WorktreeAgentActionId, Component> = {
    inspect: ScanSearch,
    'archive-and-remove': Archive,
    remove: Trash2
  };

  const facts = $derived(worktreeFacts(row));
</script>

<div class="flex flex-col gap-3">
  <!-- The plain facts, one labelled line each. Anything unknown is absent. -->
  <dl class="m-0 grid grid-cols-[auto_minmax(0,1fr)] gap-x-2 gap-y-0.5">
    {#each facts as fact (fact.label)}
      <dt class="m-0 text-sm leading-normal text-muted-foreground">{fact.label}</dt>
      <dd
        class="m-0 min-w-0 text-sm leading-normal break-words text-foreground"
        title={fact.stamp ? exactLocalTime(fact.stamp) : undefined}
      >
        {fact.value}
      </dd>
    {/each}
  </dl>

  <!-- The verdict: what to do, why, and what to check first. -->
  <div class="flex flex-col gap-1.5">
    <div class="flex flex-wrap items-center gap-1.5">
      <Chip tone={LANE_TONE[row.lane.tone] ?? 'neutral'}>{row.lane.label}</Chip>
      <span class="text-[13px] leading-normal text-foreground">{row.lane.detail}</span>
    </div>
    <p class="m-0 text-sm leading-normal text-muted-foreground">
      {row.safety.reason}. {row.safety.recommendation}
    </p>
    {#if row.safety.decisionChecklist.length > 0}
      <ul class="m-0 flex list-none flex-col gap-0.5 p-0">
        {#each row.safety.decisionChecklist as item, index (index)}
          <li class="flex gap-1.5 text-sm leading-normal text-muted-foreground">
            <span aria-hidden="true">•</span>
            <span>{item}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <!-- Who has worked in this folder. Named, because "2 sessions" is not enough
       to decide whether removing the folder would interrupt anybody. -->
  {#if row.sessions.length > 0}
    <div class="flex flex-col gap-1">
      <h4 class="m-0 text-sm leading-normal font-semibold text-muted-foreground">Sessions here</h4>
      <ul class="m-0 flex min-w-0 list-none flex-wrap gap-1 p-0">
        {#each row.sessions as link (link.ownedId)}
          {@const label = `${link.title.trim() || 'Untitled session'}${
            link.isRunning ? ' · running' : ''
          }`}
          <li class="min-w-0 max-w-full">
            {#if onOpenSession}
              <Button
                size="xs"
                variant="secondary"
                class="max-w-full min-w-0 text-sm"
                title={label}
                onclick={() => onOpenSession?.(link.ownedId)}
              >
                <span class="truncate">{label}</span>
              </Button>
            {:else}
              <Chip class="max-w-full min-w-0"><span class="truncate" title={label}>{label}</span></Chip>
            {/if}
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  <!-- The three things that can be done about this worktree. Each starts a
       session that looks first and says what it found before it acts. -->
  <div class="flex flex-col gap-1.5">
    <h4 class="m-0 text-sm leading-normal font-semibold text-muted-foreground">
      What to do about it
    </h4>
    <div class="flex flex-wrap gap-1.5">
      {#each actions as action (action.id)}
        {@const Icon = ICONS[action.id]}
        <Button
          size="xs"
          variant={action.destructive ? 'outline' : 'secondary'}
          class="text-[13px]"
          disabled={!action.enabled || panelBusy}
          onclick={() => onAction(action.id)}
        >
          <Icon aria-hidden="true" />
          {action.label}
        </Button>
      {/each}
    </div>
    {#if actions.some((action) => !action.enabled)}
      <p class="m-0 text-sm leading-normal text-muted-foreground">
        {actions.find((action) => !action.enabled)?.disabledReason}
      </p>
    {/if}
    <p class="m-0 text-sm leading-normal text-muted-foreground">
      {busy
        ? 'Starting a session…'
        : 'Each of these starts a session in this folder. It reads the worktree, explains what it found, warns about anything that would be lost, and only then does what you asked.'}
    </p>
  </div>
</div>
