<script lang="ts">
  /**
   * SessionsColumn.svelte — the /next shell's left column.
   *
   * The Working/Done/Settled list is owned by SessionRail. This wrapper keeps
   * the column-width strip, the header controls, and the remove confirmation
   * while forwarding every session intent to the page-owned rail authorities.
   */
  import Bot from '@lucide/svelte/icons/bot';
  import Archive from '@lucide/svelte/icons/archive';
  import Check from '@lucide/svelte/icons/check';
  import CircleDot from '@lucide/svelte/icons/circle-dot';
  import PanelLeftOpen from '@lucide/svelte/icons/panel-left-open';
  import Plus from '@lucide/svelte/icons/plus';
  import Search from '@lucide/svelte/icons/search';
  import List from '@lucide/svelte/icons/list';
  import { onMount } from 'svelte';

  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { buttonVariants } from '$lib/components/ui/button/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { SegmentedControl } from '$lib/components/ui/segmented-control/index.js';
  import * as Select from '$lib/components/ui/select/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import { resolveOwnedSessionProject, type OwnedSession } from '$lib/shell/ownedSessions';
  import { AGENT_ICONS } from '$lib/shell/agentIcons';
  import { registerSessionRestart } from '$lib/shell/conversation/sessionRestart.ts';
  import { openSessionLibrary } from '$lib/shell/sessionLibrary/sessionLibraryNavigation';
  import SessionRail from './SessionRail.svelte';
  import { sessionLabel, stripCells } from '$lib/shell/sessionStrip';
  import { cn } from '$lib/utils';
  import {
    DEFAULT_MY_WORK_VIEW_OPTIONS,
    MY_WORK_STATUSES,
    normalizeMyWorkViewOptions,
    readMyWorkViewOptions,
    writeMyWorkViewOptions,
    type MyWorkGrouping,
    type MyWorkSort,
    type MyWorkStatus,
    type MyWorkViewOptions
  } from './myWorkViewOptions';

  interface Props {
    owned: OwnedSession[];
    activeOwnedId: string | null;
    collapsed: boolean;
    onSelect(ownedId: string): void;
    onRestart(ownedId: string): void;
    onComplete(ownedId: string): void;
    onReopen(ownedId: string): void;
    onSettle(ownedId: string): void;
    onUnsettle(ownedId: string): void;
    onRemove(ownedId: string): void;
    onNewSession(): void;
    onCollapse(collapsed: boolean): void;
  }

  let {
    owned,
    activeOwnedId,
    collapsed,
    onSelect,
    onRestart,
    onComplete,
    onReopen,
    onSettle,
    onUnsettle,
    onRemove,
    onNewSession,
    onCollapse
  }: Props = $props();

  const cells = $derived(stripCells(owned, activeOwnedId));
  let viewOptions = $state<MyWorkViewOptions>({
    ...DEFAULT_MY_WORK_VIEW_OPTIONS,
    visibleStatuses: [...MY_WORK_STATUSES]
  });

  const GROUPING_ITEMS = [
    { value: 'none', label: 'None' },
    { value: 'status', label: 'Status' },
    { value: 'project', label: 'Project' }
  ] as const;

  const STATUS_CONTROLS = [
    { value: 'working', label: 'Working', icon: CircleDot },
    { value: 'done', label: 'Done', icon: Check },
    { value: 'settled', label: 'Settled', icon: Archive }
  ] as const;

  $effect(() => registerSessionRestart(onRestart, onSelect));

  onMount(() => {
    viewOptions = readMyWorkViewOptions(window.localStorage);
  });

  function setViewOptions(patch: Partial<MyWorkViewOptions>): void {
    viewOptions = normalizeMyWorkViewOptions({ ...viewOptions, ...patch });
    writeMyWorkViewOptions(window.localStorage, viewOptions);
  }

  function toggleStatus(status: MyWorkStatus): void {
    const visibleStatuses = viewOptions.visibleStatuses.includes(status)
      ? viewOptions.visibleStatuses.filter((candidate) => candidate !== status)
      : MY_WORK_STATUSES.filter(
          (candidate) => candidate === status || viewOptions.visibleStatuses.includes(candidate)
        );
    setViewOptions({ visibleStatuses });
  }

  /** The session the remove confirmation is about, or null while it is shut. */
  let removing = $state<OwnedSession | null>(null);
  let removeOpen = $state(false);

  /** Searching sessions means the full Session History tab, not a rail popover. */
  export function openFinder(): void {
    openSessionLibrary();
  }

  /** The rail's own filter: a strip under the header, open only while in use. */
  let filterOpen = $state(false);
  let filterText = $state('');
  let filterInput = $state<HTMLInputElement | null>(null);

  const filtered = $derived.by(() => {
    const needle = filterText.trim().toLowerCase();
    if (!needle) return owned;
    return owned.filter((session) => {
      const title = sessionLabel(session).toLowerCase();
      const project = resolveOwnedSessionProject(session).label.toLowerCase();
      return title.includes(needle) || project.includes(needle);
    });
  });

  function showFilter(): void {
    filterOpen = true;
    queueMicrotask(() => filterInput?.focus());
  }

  function hideFilter(): void {
    filterOpen = false;
    filterText = '';
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

  const ACTION_CLASS =
    'text-[var(--color-text-2)] hover:text-foreground hover:bg-[var(--color-elevated)]';
  const TOOLTIP_CLASS =
    'bg-[var(--color-surface)] text-foreground ring-1 ring-[var(--color-border)] ' +
    'shadow-[var(--shadow-md)] text-[12px] px-2 py-1';
  const TOOLTIP_ARROW_CLASS = 'bg-[var(--color-surface)] fill-[var(--color-surface)]';
</script>

<!-- Every icon-only control in this column is the same kit button at the same
     size, so they all say what they do on hover and light up the same way. -->
{#snippet action(tip: string, Icon: typeof Bot, run: () => void, extraClass = '')}
  <IconButton
    label={tip}
    size="sm"
    side="bottom"
    class={cn(ACTION_CLASS, extraClass)}
    onclick={(event: MouseEvent) => {
      event.stopPropagation();
      run();
    }}
  >
    <Icon class="size-4" aria-hidden="true" />
  </IconButton>
{/snippet}

<Tooltip.Provider delayDuration={250}>
  {#if collapsed}
    <div class="flex h-full w-full flex-col items-center gap-1 overflow-hidden bg-[var(--color-bg)] py-2">
      {@render action('Open the sessions column', PanelLeftOpen, () => onCollapse(false))}

      <div class="mt-1 flex min-h-0 w-full flex-1 flex-col items-center gap-1 overflow-y-auto">
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
    <div data-testid="sessions-column" class="sessions-column flex h-full min-h-0 flex-col bg-[var(--color-bg)] text-[var(--color-text)]">
      <header class="sessions-header">
        <h2 class="text-[14px] font-semibold text-[var(--color-text)]">Sessions</h2>
        <div class="header-actions ml-auto flex items-center gap-2">
          <IconButton
            label="Search sessions"
            size="xs"
            side="bottom"
            class={ACTION_CLASS}
            onclick={showFilter}
          >
            <Search class="size-3.5" aria-hidden="true" />
          </IconButton>
          <DropdownMenu.Root>
            <Tooltip.Root>
              <Tooltip.Trigger>
                {#snippet child({ props })}
                  <DropdownMenu.Trigger
                    {...props}
                    data-testid="my-work-view-options-trigger"
                    class={cn(buttonVariants({ variant: 'ghost', size: 'icon-sm' }), ACTION_CLASS)}
                    aria-label="View session options"
                  >
                    <List aria-hidden="true" />
                  </DropdownMenu.Trigger>
                {/snippet}
              </Tooltip.Trigger>
              <Tooltip.Content side="bottom" class={TOOLTIP_CLASS} arrowClasses={TOOLTIP_ARROW_CLASS}>
                View session options
              </Tooltip.Content>
            </Tooltip.Root>

            <DropdownMenu.Content
              align="end"
              sideOffset={7}
              class="w-[304px]! space-y-1.5 bg-popover p-2 text-foreground ring-border"
            >
              <div class="flex min-h-8 items-center justify-between gap-3 px-1">
                <span class="text-[13px] text-foreground">Show: Projects</span>
                <Switch
                  size="sm"
                  checked={viewOptions.groupBy === 'project'}
                  aria-label="Show projects"
                  onCheckedChange={(checked) => setViewOptions({ groupBy: checked ? 'project' : 'none' })}
                />
              </div>

              <div class="flex flex-col gap-1.5 px-1 py-1">
                <span class="text-[13px] text-foreground">Group by</span>
                <SegmentedControl
                  class="w-full"
                  size="sm"
                  items={GROUPING_ITEMS}
                  value={viewOptions.groupBy}
                  aria-label="Group My Work sessions"
                  onValueChange={(value) => setViewOptions({ groupBy: value as MyWorkGrouping })}
                />
              </div>

              <div class="flex min-h-8 items-center justify-between gap-3 px-1 py-1">
                <span class="text-[13px] text-foreground">Sort</span>
                <Select.Root
                  type="single"
                  value={viewOptions.sortBy}
                  onValueChange={(value) => setViewOptions({ sortBy: value as MyWorkSort })}
                >
                  <Select.Trigger size="sm" class="min-w-[132px]" aria-label="Sort My Work sessions">
                    {viewOptions.sortBy === 'recent' ? 'Recent activity' : 'Name'}
                  </Select.Trigger>
                  <Select.Content>
                    <Select.Item value="recent" label="Recent activity" />
                    <Select.Item value="name" label="Name" />
                  </Select.Content>
                </Select.Root>
              </div>

              <div class="flex flex-col gap-1.5 px-1 pt-1 pb-0.5">
                <span class="text-[13px] text-foreground">Status filters</span>
                <div class="grid grid-cols-3 gap-1" aria-label="Visible statuses">
                  {#each STATUS_CONTROLS as control (control.value)}
                    {@const StatusIcon = control.icon}
                    {@const pressed = viewOptions.visibleStatuses.includes(control.value)}
                    <Button
                      size="xs"
                      variant={pressed ? 'secondary' : 'ghost'}
                      class="min-w-0 gap-1 px-1 text-[13px] font-normal"
                      aria-pressed={pressed}
                      onclick={() => toggleStatus(control.value)}
                    >
                      <StatusIcon class="size-3" aria-hidden="true" />
                      <span class="truncate">{control.label}</span>
                    </Button>
                  {/each}
                </div>
              </div>
            </DropdownMenu.Content>
          </DropdownMenu.Root>
          {@render action('New session', Plus, onNewSession, 'text-[var(--color-accent)]')}
        </div>
      </header>

      {#if filterOpen}
        <div class="filter-strip">
          <Input
            bind:ref={filterInput}
            bind:value={filterText}
            data-testid="session-rail-filter"
            placeholder="Filter sessions"
            aria-label="Filter sessions by title or project"
            onkeydown={(event: KeyboardEvent) => { if (event.key === 'Escape') hideFilter(); }}
            onblur={hideFilter}
          />
        </div>
      {/if}

      <div class="min-h-0 flex-1 overflow-hidden">
        <SessionRail
          sessions={filtered}
          options={viewOptions}
          {activeOwnedId}
          {onSelect}
          {onRestart}
          {onComplete}
          {onReopen}
          {onSettle}
          {onUnsettle}
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

  .sessions-column {
    box-sizing: border-box;
    overflow: hidden;
    font-size: 13px;
    line-height: 19.5px;
  }

  .sessions-header {
    display: flex;
    flex: 0 0 52px;
    align-items: center;
    gap: 5px;
    padding: 0 9px 0 13px;
    font-size: 13px;
    line-height: 19.5px;
  }

  .sessions-header :global(h2) {
    flex: 1 1 auto;
    font-size: 14px;
    letter-spacing: -0.01em;
    line-height: 19.5px;
  }

  .sessions-header :global(button) {
    width: 29px;
    height: 29px;
    padding: 0;
    border-radius: 7px;
    font-size: 13.3333px;
    line-height: normal;
  }

  .sessions-header .header-actions { gap: 5px; }

  .filter-strip {
    flex: 0 0 auto;
    padding: 7px 9px 7px 13px;
  }

  .sessions-header :global(button svg) {
    width: 16px;
    height: 16px;
  }
</style>
