<script lang="ts">
  /**
   * SessionsColumn.svelte — the /next shell's left column.
   *
   * The Working/Done/Settled list is owned by SessionRail. This wrapper keeps
   * the column-width strip and the header controls. Session rows are currently
   * inert while their selection lifecycle is rebuilt one responsibility at a time.
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

  import { Button } from '$lib/components/ui/button/index.js';
  import { buttonVariants } from '$lib/components/ui/button/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import * as Select from '$lib/components/ui/select/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import { resolveOwnedSessionProject, type OwnedSession } from '$lib/shell/ownedSessions';
  import { AGENT_ICONS } from '$lib/shell/agentIcons';
  import { openSessionLibrary } from '$lib/shell/sessionLibrary/sessionLibraryNavigation';
  import SegmentedTabs from './SegmentedTabs.svelte';
  import SessionRail from './SessionRail.svelte';
  import { sessionLabel, stripCells } from '$lib/shell/sessionStrip';
  import {
    readAssemblySettingFromTauri,
    writeAssemblySettingFromTauri
  } from '$lib/tauriSource';
  import { cn } from '$lib/utils';
  import {
    DEFAULT_MY_WORK_VIEW_OPTIONS,
    MY_WORK_STATUSES,
    normalizeMyWorkViewOptions,
    type MyWorkGrouping,
    type MyWorkSort,
    type MyWorkSortDirection,
    NATURAL_SORT_DIRECTION,
    myWorkSortDirectionLabel,
    type MyWorkStatus,
    type MyWorkViewOptions
  } from './myWorkViewOptions';

  interface Props {
    owned: OwnedSession[];
    activeOwnedId: string | null;
    collapsed: boolean;
    onNewSession(): void;
    onCollapse(collapsed: boolean): void;
    onSelectSession?(ownedId: string): void | Promise<void>;
    onComplete?(ownedId: string): void;
    onReopen?(ownedId: string): void;
    onSettle?(ownedId: string): void;
    onUnsettle?(ownedId: string): void;
    onAskRemove?(ownedId: string): void;
  }

  let {
    owned,
    activeOwnedId,
    collapsed,
    onNewSession,
    onCollapse,
    onSelectSession,
    onComplete,
    onReopen,
    onSettle,
    onUnsettle,
    onAskRemove
  }: Props = $props();

  const cells = $derived(stripCells(owned, activeOwnedId));
  let viewOptions = $state<MyWorkViewOptions>({
    ...DEFAULT_MY_WORK_VIEW_OPTIONS,
    visibleStatuses: [...MY_WORK_STATUSES]
  });
  const MY_WORK_VIEW_OPTIONS_SETTING_KEY = 'rail.my-work-view-options';
  let viewOptionsVersion = 0;

  const GROUPING_ITEMS = [
    { id: 'none', label: 'None' },
    { id: 'status', label: 'Status' },
    { id: 'project', label: 'Project' }
  ] as const;

  const STATUS_CONTROLS = [
    { value: 'working', label: 'Working', icon: CircleDot },
    { value: 'done', label: 'Done', icon: Check },
    { value: 'settled', label: 'Settled', icon: Archive }
  ] as const;

  onMount(() => {
    const owner = { active: true };
    const restoreVersion = viewOptionsVersion;
    void restoreViewOptions(owner, restoreVersion);
    return () => {
      owner.active = false;
    };
  });

  async function restoreViewOptions(owner: { active: boolean }, restoreVersion: number): Promise<void> {
    try {
      const stored = await readAssemblySettingFromTauri(MY_WORK_VIEW_OPTIONS_SETTING_KEY);
      if (owner.active && viewOptionsVersion === restoreVersion) {
        viewOptions = normalizeMyWorkViewOptions(stored);
      }
    } catch {
      // View options fall back to defaults when local settings are unavailable.
    }
  }

  function setViewOptions(patch: Partial<MyWorkViewOptions>): void {
    viewOptionsVersion += 1;
    viewOptions = normalizeMyWorkViewOptions({ ...viewOptions, ...patch });
    void persistViewOptions(viewOptions, viewOptionsVersion);
  }

  async function persistViewOptions(options: MyWorkViewOptions, version: number): Promise<void> {
    try {
      await writeAssemblySettingFromTauri(MY_WORK_VIEW_OPTIONS_SETTING_KEY, options);
    } catch {
      if (version !== viewOptionsVersion) return;
    }
  }

  function toggleStatus(status: MyWorkStatus): void {
    const visibleStatuses = viewOptions.visibleStatuses.includes(status)
      ? viewOptions.visibleStatuses.filter((candidate) => candidate !== status)
      : MY_WORK_STATUSES.filter(
          (candidate) => candidate === status || viewOptions.visibleStatuses.includes(candidate)
        );
    setViewOptions({ visibleStatuses });
  }

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
    filterInput?.focus();
  }

  function hideFilter(): void {
    filterOpen = false;
    filterText = '';
  }

  async function selectFilteredSession(ownedId: string): Promise<void> {
    await onSelectSession?.(ownedId);
    hideFilter();
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

  {#if collapsed}
    <div class="flex h-full w-full flex-col items-center gap-1 overflow-hidden py-2">
      <Tooltip.Provider delayDuration={0}>
        {@render action('Open the sessions column', PanelLeftOpen, () => onCollapse(false))}
      </Tooltip.Provider>

      <div class="mt-1 flex min-h-0 w-full flex-1 flex-col items-center gap-1 overflow-y-auto">
        {#each cells as cell (cell.ownedId)}
          {@const CellIcon = AGENT_ICONS[cell.agent]}
          <div
            class={cn(
              'relative flex size-9 shrink-0 items-center justify-center rounded-md border',
              'border-transparent text-[var(--color-text-2)]',
              cell.done && 'opacity-55',
              cell.active &&
                'border-primary/45 bg-[var(--color-elevated)] text-foreground opacity-100'
            )}
            aria-label={cell.label}
          >
            <CellIcon class="size-4" aria-hidden="true" />
            <span class="state-dot absolute right-1 bottom-1" data-state={cell.state} aria-hidden="true"></span>
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <div data-testid="sessions-column" class="sessions-column flex h-full min-h-0 flex-col text-[var(--color-text)]">
      <Tooltip.Provider delayDuration={0}>
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

            <!-- The sheet keeps the shared menu surface rather than the popover
                 colour it used to name: that colour is the same grey the rail
                 behind it is painted in, so an open menu vanished into the
                 column and only its hairline gave it away. Padding and the gaps
                 between rows come from the shell's spacing steps. -->
            <DropdownMenu.Content
              align="end"
              sideOffset={7}
              class="flex w-[304px]! flex-col gap-[var(--space-3)] p-[var(--space-4)] text-foreground"
            >
              <div class="flex min-h-8 items-center justify-between gap-3">
                <span class="text-[13px] text-foreground">Show: Projects</span>
                <Switch
                  size="sm"
                  checked={viewOptions.groupBy === 'project'}
                  aria-label="Show projects"
                  onCheckedChange={(checked) => setViewOptions({ groupBy: checked ? 'project' : 'none' })}
                />
              </div>

              <div class="flex flex-col gap-[var(--space-2)]">
                <span class="text-[13px] text-foreground">Group by</span>
                <SegmentedTabs
                  items={GROUPING_ITEMS}
                  value={viewOptions.groupBy}
                  label="Group My Work sessions"
                  onChange={(id) => setViewOptions({ groupBy: id as MyWorkGrouping })}
                />
              </div>

              <div class="flex min-h-8 items-center justify-between gap-3">
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

              <div class="flex min-h-8 items-center justify-between gap-3">
                <span class="text-[13px] text-foreground">Direction</span>
                <Select.Root
                  type="single"
                  value={viewOptions.sortDirection}
                  onValueChange={(value) =>
                    setViewOptions({ sortDirection: value as MyWorkSortDirection })}
                >
                  <Select.Trigger size="sm" class="min-w-[132px]" aria-label="Sort direction">
                    {myWorkSortDirectionLabel(viewOptions.sortBy, viewOptions.sortDirection)}
                  </Select.Trigger>
                  <Select.Content>
                    <Select.Item
                      value={NATURAL_SORT_DIRECTION[viewOptions.sortBy]}
                      label={myWorkSortDirectionLabel(
                        viewOptions.sortBy,
                        NATURAL_SORT_DIRECTION[viewOptions.sortBy]
                      )}
                    />
                    <Select.Item
                      value={NATURAL_SORT_DIRECTION[viewOptions.sortBy] === 'desc' ? 'asc' : 'desc'}
                      label={myWorkSortDirectionLabel(
                        viewOptions.sortBy,
                        NATURAL_SORT_DIRECTION[viewOptions.sortBy] === 'desc' ? 'asc' : 'desc'
                      )}
                    />
                  </Select.Content>
                </Select.Root>
              </div>

              <div class="flex flex-col gap-[var(--space-2)]">
                <span class="text-[13px] text-foreground">Status filters</span>
                <div class="grid grid-cols-3 gap-1" aria-label="Visible statuses">
                  {#each STATUS_CONTROLS as control (control.value)}
                    {@const StatusIcon = control.icon}
                    {@const pressed = viewOptions.visibleStatuses.includes(control.value)}
                    <Button
                      size="sm"
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
      </Tooltip.Provider>

      {#if filterOpen}
        <div class="filter-strip">
          <Input
            bind:ref={filterInput}
            bind:value={filterText}
            data-testid="session-rail-filter"
            placeholder="Filter sessions"
            aria-label="Filter sessions by title or project"
            onkeydown={(event: KeyboardEvent) => { if (event.key === 'Escape') hideFilter(); }}
          />
        </div>
      {/if}

      <div class="min-h-0 flex-1 overflow-hidden">
        <SessionRail
          sessions={filtered}
          options={viewOptions}
          {activeOwnedId}
          onSelectSession={selectFilteredSession}
          {onComplete}
          {onReopen}
          {onSettle}
          {onUnsettle}
          {onAskRemove}
        />
      </div>
    </div>
  {/if}

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

  /* No background here, and none on the folded rail above either. The panel
     this column is mounted into already paints the shell's card — the surface
     colour and the light along its top edge — so a fill of our own only covers
     it up, which is what made this column read as a hole cut in the backdrop
     while the other two read as cards. */
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
    border-radius: var(--radius-pill);
    font-size: 13.3333px;
    line-height: normal;
  }

  /* The three controls sit in one container, in the same tone and shape as the
     right panel's tab strip, so the shell's grouped controls all look alike.
     There is no sliding pill here: search, view options and new session are
     three things you do, not one choice out of three. */
  .sessions-header .header-actions {
    gap: 1px;
    padding: 3px;
    border-radius: var(--radius-pill);
    background: var(--color-elevated);
  }

  .filter-strip {
    flex: 0 0 auto;
    padding: 7px 9px 7px 13px;
  }

  .sessions-header :global(button svg) {
    width: 16px;
    height: 16px;
  }
</style>
