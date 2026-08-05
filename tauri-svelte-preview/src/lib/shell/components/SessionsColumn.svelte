<script lang="ts">
  /**
   * SessionsColumn.svelte — the /next shell's left column.
   *
   * The primary Working/Done/Settled layout and the resumable-session finder
   * are owned by SessionsPaneview. This wrapper keeps the column-width strip,
   * the existing New session affordance, and the remove confirmation while
   * forwarding every session intent to the page-owned rail authorities.
   */
  import Bot from '@lucide/svelte/icons/bot';
  import Gem from '@lucide/svelte/icons/gem';
  import PanelLeftClose from '@lucide/svelte/icons/panel-left-close';
  import PanelLeftOpen from '@lucide/svelte/icons/panel-left-open';
  import Plus from '@lucide/svelte/icons/plus';
  import SquareCode from '@lucide/svelte/icons/square-code';
  import SquareTerminal from '@lucide/svelte/icons/square-terminal';
  import Terminal from '@lucide/svelte/icons/terminal';

  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import { buttonVariants } from '$lib/components/ui/button/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import type { AgentSession } from '$lib/tauriSource';
  import type { AgentKind, OwnedSession } from '$lib/shell/ownedSessions';
  import SessionsPaneview from './SessionsPaneview.svelte';
  import { sessionLabel, stripCells } from '$lib/shell/sessionStrip';
  import { cn } from '$lib/utils';

  interface Props {
    owned: OwnedSession[];
    available: AgentSession[];
    activeOwnedId: string | null;
    scanning: boolean;
    collapsed: boolean;
    onSelect(ownedId: string): void;
    onAdopt(session: AgentSession): void;
    onClose(ownedId: string): void;
    onRestart(ownedId: string): void;
    onComplete(ownedId: string): void;
    onReopen(ownedId: string): void;
    onSettle(ownedId: string): void;
    onUnsettle(ownedId: string): void;
    onRemove(ownedId: string): void;
    onRescan(): void;
    onNewSession(): void;
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
    onSettle,
    onUnsettle,
    onRemove,
    onRescan,
    onNewSession,
    onCollapse
  }: Props = $props();

  let paneview = $state<{ openFinder(): void } | null>(null);
  const cells = $derived(stripCells(owned, activeOwnedId));

  /** The session the remove confirmation is about, or null while it is shut. */
  let removing = $state<OwnedSession | null>(null);
  let removeOpen = $state(false);

  export function openFinder(): void {
    paneview?.openFinder();
  }

  function askAboutRemoving(ownedId: string): void {
    removing = owned.find((session) => session.ownedId === ownedId) ?? null;
    removeOpen = removing !== null;
  }

  function removeNow(): void {
    const session = removing;
    removeOpen = false;
    removing = null;
    if (session) onRemove(session.ownedId);
  }

  function removeQuestion(session: OwnedSession): string {
    return session.state !== 'exited'
      ? 'Its terminal is still running and will be closed. The transcript stays on disk.'
      : 'The transcript stays on disk.';
  }

  const AGENT_ICONS: Record<AgentKind, typeof Bot> = {
    claude: Bot,
    codex: SquareCode,
    gemini: Gem,
    opencode: SquareTerminal,
    other: Terminal
  };
  const ACTION_CLASS =
    'text-[var(--color-text-2)] hover:text-foreground hover:bg-[var(--color-elevated)]';
  const TOOLTIP_CLASS =
    'bg-[var(--color-surface)] text-foreground ring-1 ring-[var(--color-border)] ' +
    'shadow-[var(--shadow-md)] text-[12px] px-2 py-1';
  const TOOLTIP_ARROW_CLASS = 'bg-[var(--color-surface)] fill-[var(--color-surface)]';
</script>

{#snippet action(label: string, tip: string, Icon: typeof Bot, run: () => void)}
  <Tooltip.Root>
    <Tooltip.Trigger
      class={cn(buttonVariants({ variant: 'ghost', size: 'icon-xs' }), ACTION_CLASS)}
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

<Tooltip.Provider delayDuration={250}>
  {#if collapsed}
    <div class="flex h-full w-full flex-col items-center gap-1 overflow-hidden bg-[var(--color-bg)] py-2">
      {@render action('open the sessions column', 'Open the sessions column', PanelLeftOpen, () => onCollapse(false))}

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
              <span class="state-dot absolute right-1 bottom-1" data-state={cell.state} aria-hidden="true"></span>
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
      <header class="flex shrink-0 items-center gap-2 border-b border-[var(--color-border)] px-2.5 py-2">
        <h2 class="text-[12px] font-semibold tracking-[0.09em] text-[var(--color-text-2)] uppercase">My work</h2>
        <span class="text-[12px] text-[var(--color-text-2)]">{owned.length}</span>
        <div class="ml-auto flex items-center gap-1">
          {@render action('start a new session', 'New session', Plus, onNewSession)}
          {@render action('fold the sessions column up', 'Fold this column up', PanelLeftClose, () => onCollapse(true))}
        </div>
      </header>

      <div class="min-h-0 flex-1 overflow-hidden">
        <SessionsPaneview
          bind:this={paneview}
          sessions={owned}
          {available}
          {scanning}
          activeOwnedId={activeOwnedId}
          onSelect={onSelect}
          onAdopt={onAdopt}
          onRescan={onRescan}
          onRestart={onRestart}
          onComplete={onComplete}
          onReopen={onReopen}
          onSettle={onSettle}
          onUnsettle={onUnsettle}
          onClose={onClose}
          onAskRemove={askAboutRemoving}
        />
      </div>
    </div>
  {/if}

  <AlertDialog.Root bind:open={removeOpen}>
    <AlertDialog.Content class="rounded-lg bg-background text-foreground ring-[var(--color-border)] shadow-[var(--shadow-lg)]">
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
        <AlertDialog.Action size="sm" variant="destructive" class="text-[13px]" onclick={removeNow}>
          Remove
        </AlertDialog.Action>
      </AlertDialog.Footer>
    </AlertDialog.Content>
  </AlertDialog.Root>
</Tooltip.Provider>

<style>
  .state-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--color-text-3);
  }
  .state-dot[data-state='live'],
  .state-dot[data-state='background'] { background: var(--color-live); }
  .state-dot[data-state='exited'] { background: transparent; box-shadow: inset 0 0 0 1px var(--color-text-2); }
</style>
