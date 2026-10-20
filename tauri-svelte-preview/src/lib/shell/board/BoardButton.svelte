<!-- The top bar's Board button: a badge with what needs the owner, and the board drop-down. -->
<script lang="ts">
  import Columns2 from '@lucide/svelte/icons/columns-2';

  import { buttonVariants } from '$lib/components/ui/button/index.js';
  import * as Popover from '$lib/components/ui/popover/index.js';
  import { sessionWaitsOnYou } from '$lib/shell/ownedSessions.ts';
  import { rail } from '$lib/shell/stores/sessionRailStore.svelte.ts';
  import { listenAgentMessagesChanged } from '$lib/tauriSource.ts';
  import { cn } from '$lib/utils.js';
  import Board from './Board.svelte';
  import { countBoard } from './boardSource.ts';

  let open = $state(false);
  /** Needs-you cards that are not snoozed, plus workflow steps waiting, on every machine. */
  let counted = $state(0);
  /** Bumped on every change, so an open board reads again. */
  let version = $state(0);
  const waiting = $derived(rail.owned.filter(sessionWaitsOnYou));
  const badge = $derived(counted + waiting.length);

  $effect(() => {
    const life = new AbortController();
    let request: AbortController | null = null;
    let timer = 0;
    // A recount aborts the one before it. The one timer brings a snoozed card back into the count.
    async function recount(): Promise<void> {
      request?.abort();
      window.clearTimeout(timer);
      const current = (request = new AbortController());
      try {
        const result = await countBoard(Date.now(), current.signal);
        if (!result || current.signal.aborted) return;
        counted = result.count;
        if (result.snoozed_until_ms) timer = window.setTimeout(() => void recount(), Math.max(0, result.snoozed_until_ms - Date.now()));
      } catch {
        // The next change counts again; until then the badge keeps its last number.
      }
    }
    void recount();
    void listenAgentMessagesChanged(() => {
      version += 1;
      void recount();
    }, life.signal);
    return () => {
      life.abort();
      request?.abort();
      window.clearTimeout(timer);
    };
  });
</script>

<Popover.Root bind:open>
  <Popover.Trigger
    class={cn(buttonVariants({ variant: 'ghost', size: 'sm' }), 'flex-none gap-1.5 rounded-full px-2.5', open && 'bg-accent')}
    aria-label={badge > 0 ? `Board, ${badge} need you` : 'Board'}
    data-testid="board-button"
  >
    <Columns2 aria-hidden="true" />
    Board
    {#if badge > 0}
      <span
        class="inline-flex h-5 min-w-6 items-center justify-center gap-1 rounded-(--radius-sm) border border-[color-mix(in_srgb,var(--color-attention)_34%,transparent)] bg-(--color-attention-bg) px-1.5 text-xs font-semibold text-(--color-attention) tabular-nums"
        aria-hidden="true"
      ><span class="size-1.5 rounded-full bg-(--color-attention)"></span>{badge > 99 ? '99+' : badge}</span>
    {/if}
  </Popover.Trigger>
  {#if open}
    <Board {badge} sessions={waiting} {version} onClose={() => (open = false)} />
  {/if}
</Popover.Root>
