<script lang="ts">
  /**
   * HistoryPanel.svelte — the History tab of the right column.
   *
   * Every session this machine has run, grouped by the project it ran in, most
   * recent first. A project with hundreds of sessions in it opens showing the
   * newest twenty-five per checkout and offers the rest on request, because the
   * alternative is a panel that takes a second to draw every time it is shown.
   *
   * The panel owns running the actions, since it is the only thing here holding
   * the session service and the confirm dialog. Deciding WHICH actions a
   * session supports is `sessionHistoryActions`'s job, and drawing a card is
   * `SessionHistoryCard`'s.
   */
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import History from '@lucide/svelte/icons/history';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Chip } from '$lib/components/ui/chip/index.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { PanelHeader } from '$lib/components/ui/panel-header/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import {
    buildSessionHistoryViewModel,
    createSessionHistoryCollapseState,
    createSessionHistoryWindowState,
    extendSessionHistoryWindow,
    isSessionHistoryGroupOpen,
    resetSessionHistoryWindowOnFilterChange,
    toggleSessionHistoryGroup,
    type SessionHistoryRow
  } from '$lib/shell/history/sessionHistoryViewModel.ts';
  import {
    buildSessionLibrary,
    type SessionLibraryRecord
  } from '$lib/shell/sessionLibrary/sessionLibraryModel.ts';
  import { sessionLibraryHost } from '$lib/shell/sessionLibrary/sessionLibraryService.ts';
  import { rail } from '$lib/shell/stores/sessionRailStore.svelte.ts';
  import { openPathFromTauri, revealPathFromTauri } from '$lib/tauriSource.ts';
  import {
    openFileInEditor,
    startWorkbenchSession
  } from '$lib/shell/workbenchNavigation.ts';

  import SessionHistoryCard from './SessionHistoryCard.svelte';
  import {
    sessionHistoryActions,
    sessionHistoryIdentity,
    sessionHistoryStartRequest,
    sessionResumeCommand,
    type SessionHistoryActionId
  } from './sessionHistoryActions.ts';

  interface Props {
    /** True while this panel's tab is the selected one. */
    visible: boolean;
    /** The active session's working folder, or '' when nothing is selected. */
    root: string;
    /** The active session's ownedId, or null. */
    ownedId: string | null;
  }
  let { visible }: Props = $props();

  // Read once, at init: the page registers its actions in its own component
  // body, which runs before this panel is created.
  const host = sessionLibraryHost();

  let query = $state('');
  let expandedKey = $state<string | null>(null);
  let collapseState = $state(createSessionHistoryCollapseState());
  let windowState = $state(createSessionHistoryWindowState());
  let pendingDelete = $state<SessionLibraryRecord | null>(null);
  /** Redrawn only while the panel is on screen, so ages do not go stale in it. */
  let now = $state(new Date());

  const records = $derived(buildSessionLibrary(rail.owned, rail.available));
  const viewModel = $derived(
    buildSessionHistoryViewModel(records, { query, windowState })
  );

  /** A new search starts every checkout back at its first page of cards. */
  function search(value: string): void {
    query = value;
    windowState = resetSessionHistoryWindowOnFilterChange(windowState, { query });
  }

  $effect(() => {
    if (!visible) return;
    now = new Date();
    const timer = setInterval(() => (now = new Date()), 60_000);
    return () => clearInterval(timer);
  });

  function toggleGroup(level: 'project' | 'worktree', key: string): void {
    collapseState = toggleSessionHistoryGroup(collapseState, level, key);
  }

  function showOlder(worktreeKey: string): void {
    windowState = extendSessionHistoryWindow(windowState, worktreeKey);
  }

  async function copyText(value: string | null): Promise<void> {
    if (!value || typeof navigator === 'undefined' || !navigator.clipboard?.writeText) return;
    await navigator.clipboard.writeText(value);
  }

  /**
   * Run one of a card's actions. Nothing here re-checks whether the action is
   * allowed beyond the roster's own answer, and nothing here fails quietly: an
   * action the record cannot support never becomes pressable in the first place.
   */
  async function runAction(record: SessionLibraryRecord, id: SessionHistoryActionId): Promise<void> {
    const allowed = sessionHistoryActions(record).find((action) => action.id === id);
    if (!allowed?.enabled) return;

    switch (id) {
      case 'resume-worktree':
        await host.service.resume(record);
        return;
      case 'continue-new-session':
        await startWorkbenchSession(sessionHistoryStartRequest(record));
        return;
      case 'view-log':
        if (record.logPath) openFileInEditor({ path: record.logPath });
        return;
      case 'open-log':
        if (record.logPath) await openPathFromTauri(record.logPath);
        return;
      case 'reveal-log':
        if (record.logPath) await revealPathFromTauri(record.logPath);
        return;
      case 'open-working-directory':
        await openPathFromTauri(record.canonicalCwd);
        return;
      case 'copy-resume-command':
        await copyText(sessionResumeCommand(record));
        return;
      case 'copy-session-id':
        await copyText(sessionHistoryIdentity(record));
        return;
      case 'copy-log-path':
        await copyText(record.logPath);
        return;
      case 'delete':
        // Nothing is removed until the question below is answered.
        pendingDelete = record;
        return;
    }
  }

  async function confirmDelete(): Promise<void> {
    const record = pendingDelete;
    pendingDelete = null;
    if (record) await host.service.delete(record);
  }

  function toggleCard(row: SessionHistoryRow): void {
    expandedKey = expandedKey === row.record.key ? null : row.record.key;
  }
</script>

<div class="flex h-full min-h-0 flex-col bg-card">
  <PanelHeader title="History" count={viewModel.totalCount}>
    {#snippet actions()}
      <IconButton
        label="Look for sessions again"
        disabled={rail.scanning}
        onclick={() => host.rescan?.()}
      >
        <RefreshCw />
      </IconButton>
    {/snippet}
  </PanelHeader>

  <div class="border-b px-3 py-2">
    <Input
      class="h-7"
      placeholder="Search sessions"
      autocomplete="off"
      spellcheck="false"
      value={query}
      oninput={(event) => search(event.currentTarget.value)}
    />
  </div>

  <!-- The list only exists in the document while its tab is showing. A hidden
       tab used to keep every session card mounted under display:none, and the
       browser engine walks that invisible text character by character during
       mouse hit-tests and caret queries — over a hundred thousand hidden
       characters froze the whole app for seconds at a time. The panel's own
       state (search, open groups, paging) lives above and survives; reopening
       the tab rebuilds the cards from it instantly. -->
  {#if visible}
  <ScrollArea class="min-h-0 flex-1">
    {#if viewModel.projects.length === 0}
      <EmptyState
        title={query ? 'Nothing matches that search' : 'No past sessions yet'}
        body={query
          ? 'Try a shorter search, or part of a project name.'
          : 'Sessions you have run appear here, grouped by project, with everything needed to pick one back up.'}
      >
        {#snippet icon()}<History />{/snippet}
      </EmptyState>
    {:else}
      {#each viewModel.projects as project (project.key)}
        {@const open = isSessionHistoryGroupOpen(collapseState, 'project', project.key)}
        <section>
          <h2 class="sticky top-0 z-10 bg-card">
            <button
              type="button"
              class="flex min-h-7 w-full items-center gap-2 border-b px-3 py-1.5 text-left
                     text-[13px] leading-tight font-medium text-foreground outline-none
                     transition-colors hover:bg-accent/60 focus-visible:ring-3
                     focus-visible:ring-ring/50"
              aria-expanded={open}
              onclick={() => toggleGroup('project', project.key)}
            >
              <ChevronRight
                class={`size-3.5 shrink-0 transition-transform duration-150 ${open ? 'rotate-90' : ''}`}
                aria-hidden="true"
              />
              <span class="min-w-0 flex-1 truncate">{project.name}</span>
              <Chip tone="count">{project.count}</Chip>
            </button>
          </h2>

          {#if open}
            {#each project.worktrees as worktree (worktree.key)}
              {@const worktreeOpen =
                project.singleCheckout
                || isSessionHistoryGroupOpen(collapseState, 'worktree', worktree.key)}
              {#if !project.singleCheckout}
                <button
                  type="button"
                  class="flex min-h-6 w-full items-center gap-2 px-3 py-1 text-left text-sm
                         leading-tight text-muted-foreground outline-none transition-colors
                         hover:bg-accent/60 focus-visible:ring-3 focus-visible:ring-ring/50"
                  aria-expanded={worktreeOpen}
                  onclick={() => toggleGroup('worktree', worktree.key)}
                >
                  <ChevronRight
                    class={`size-3 shrink-0 transition-transform duration-150 ${worktreeOpen ? 'rotate-90' : ''}`}
                    aria-hidden="true"
                  />
                  <span class="min-w-0 flex-1 truncate">{worktree.name}</span>
                  <Chip tone="count">{worktree.count}</Chip>
                </button>
              {/if}

              {#if worktreeOpen}
                {#each worktree.rows as row (row.record.key)}
                  <SessionHistoryCard
                    record={row.record}
                    title={row.displayTitle}
                    excerpt={row.excerpt}
                    expanded={expandedKey === row.record.key}
                    {now}
                    onToggle={() => toggleCard(row)}
                    onAction={(id) => void runAction(row.record, id)}
                    onCopyText={(value) => void copyText(value)}
                    onOpenLog={(path) => openFileInEditor({ path })}
                  />
                {/each}
                {#if worktree.olderCount > 0}
                  <div class="px-3 py-1.5">
                    <Button size="sm" variant="ghost" onclick={() => showOlder(worktree.key)}>
                      Show {worktree.olderCount} older
                    </Button>
                  </div>
                {/if}
              {/if}
            {/each}
          {/if}
        </section>
      {/each}
    {/if}
  </ScrollArea>
  {/if}
</div>

<AlertDialog.Root
  open={pendingDelete !== null}
  onOpenChange={(open) => {
    if (!open) pendingDelete = null;
  }}
>
  <AlertDialog.Content data-testid="session-history-delete-confirm">
    <AlertDialog.Header>
      <AlertDialog.Title class="flex items-center gap-1.5 text-[14px] leading-snug font-semibold">
        <TriangleAlert class="size-4 shrink-0 text-[var(--color-bad)]" aria-hidden="true" />
        Delete this session?
      </AlertDialog.Title>
      <AlertDialog.Description class="text-[13px] leading-relaxed text-muted-foreground">
        “{pendingDelete?.title ?? ''}” is removed from this app. The transcript file on disk is
        left alone, and this cannot be undone from here.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer class="bg-transparent">
      <AlertDialog.Cancel size="sm" class="text-[13px]">Keep it</AlertDialog.Cancel>
      <AlertDialog.Action
        size="sm"
        variant="destructive"
        class="text-[13px]"
        onclick={() => void confirmDelete()}
      >
        Delete session
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
