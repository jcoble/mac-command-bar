<!--
  The Board drop-down: cards agents write on the left, live state on the right.
  Mounted only while open; closing it drops everything it read.
-->
<script lang="ts">
  import { untrack } from 'svelte';

  import * as Popover from '$lib/components/ui/popover/index.js';
  import { readNotionTaskSettings } from '$lib/shell/notionTasks.ts';
  import type { OwnedSession } from '$lib/shell/ownedSessions.ts';
  import { openInboxSession } from '$lib/shell/workflows/ownerInboxActions.ts';
  import BoardCards from './BoardCards.svelte';
  import BoardLive from './BoardLive.svelte';
  import { readBoard, type MachineBoard } from './boardSource.ts';

  interface Props {
    /** The button's badge number: what needs the owner. */
    badge: number;
    /** Sessions waiting on the owner, from the rail. */
    sessions: OwnedSession[];
    /** Changes whenever the board may have changed. */
    version: number;
    onClose(): void;
  }
  let { badge, sessions, version, onClose }: Props = $props();

  let boards = $state<MachineBoard[]>([]);
  let loadedAt = $state<number | null>(null);
  let loadError = $state('');
  /** The last card or step action that failed; the next action clears it. */
  let actionError = $state('');
  let notion = $state(false);
  // Lives while the board is open; an action that finishes after it closes changes nothing.
  const shown = new AbortController();
  let loader: AbortController | null = null;
  // A newer click makes a slow session lookup stale.
  let opening = 0;

  $effect(() => {
    void readNotion();
    return () => {
      shown.abort();
      loader?.abort();
    };
  });

  $effect(() => {
    version;
    untrack(() => void load());
  });

  function message(cause: unknown): string {
    return cause instanceof Error ? cause.message : String(cause);
  }

  /** Task keys and Notion links show only when Notion Tasks has both a data source and a token. */
  async function readNotion(): Promise<void> {
    try {
      const settings = await readNotionTaskSettings();
      if (!shown.signal.aborted) notion = Boolean(settings.dataSourceId && settings.hasToken);
    } catch {
      // Not connected: the board shows no task keys.
    }
  }

  /** Opens a session at its latest message, as the Inbox does, and closes the board. */
  async function openSession(ownedId: string): Promise<void> {
    const attempt = ++opening;
    try {
      await openInboxSession(ownedId, shown.signal, () => attempt === opening && !shown.signal.aborted);
      if (attempt === opening) onClose();
    } catch (cause) {
      if (attempt === opening && !shown.signal.aborted) actionError = `Could not open session: ${message(cause)}`;
    }
  }

  async function load(): Promise<void> {
    loader?.abort();
    const controller = (loader = new AbortController());
    const now = new Date();
    const midnight = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
    try {
      const found = await readBoard(midnight, now.getTime(), controller.signal);
      if (!found || controller.signal.aborted) return;
      boards = found;
      loadedAt = Date.now();
      loadError = '';
    } catch (cause) {
      if (!controller.signal.aborted) loadError = `Could not load the board: ${message(cause)}`;
    }
  }

  /** Counted in SQL on each machine, so cards past the 40-card limit are included. */
  const count = (state: 'running' | 'in_review' | 'done'): number =>
    boards.reduce((sum, board) => sum + (board.value.counts[state] ?? 0), 0);
  const tally = $derived([
    { label: 'need you', count: badge, dot: 'bg-(--color-attention)' },
    { label: 'running', count: count('running'), dot: 'bg-(--color-live)' },
    { label: 'in review', count: count('in_review'), dot: 'bg-(--color-merged)' },
    { label: 'done today', count: count('done'), dot: 'bg-(--color-good)' }
  ]);
  const time = (ms: number): string => new Date(ms).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
</script>

<Popover.Content
  role="dialog"
  aria-label="Board"
  side="bottom"
  align="end"
  collisionPadding={{ bottom: 36, right: 8, left: 8 }}
  trapFocus={false}
  class="max-h-(--bits-popover-content-available-height) w-[min(900px,calc(100vw-16px))] gap-0 overflow-hidden p-0"
  data-testid="board"
>
  <header class="flex flex-none items-baseline gap-3 border-b border-border px-4 py-3">
    <h2 class="text-(length:--text-heading) font-semibold">Board</h2>
    <span class="truncate text-(length:--text-quiet) text-muted-foreground">
      {new Date().toLocaleDateString([], { weekday: 'long', month: 'short', day: 'numeric' })}{loadedAt ? ` · updated ${time(loadedAt)}` : ''}
    </span>
    <span class="ml-auto flex flex-none gap-3 text-(length:--text-quiet) text-muted-foreground">
      {#each tally as item (item.label)}
        <span class="inline-flex items-center gap-1.5"><span class={`size-1.5 rounded-full ${item.dot}`}></span>{item.count} {item.label}</span>
      {/each}
    </span>
  </header>
  {#if actionError || loadError}<p role="alert" class="flex-none px-4 py-2 text-[13px] text-destructive">{actionError || loadError}</p>{/if}
  <div class="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_minmax(0,320px)]">
    <BoardCards {boards} {notion} signal={shown.signal} onError={(text) => (actionError = text)} {onClose} onOpenSession={openSession} />
    <BoardLive {boards} {sessions} {notion} signal={shown.signal} onError={(text) => (actionError = text)} onOpenSession={openSession} />
  </div>
</Popover.Content>
