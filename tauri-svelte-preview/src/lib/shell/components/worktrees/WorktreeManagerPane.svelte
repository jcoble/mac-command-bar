<script lang="ts">
  /**
   * WorktreeManagerPane.svelte — the Worktrees view of the tool column.
   *
   * Self-contained by the shell's panel contract: no required props and no IO in
   * this file. It reads `worktreeManager` and calls `worktreeManagerService`
   * from user events only. Nothing loads until the shell calls
   * `worktreeManagerService.activate(...)` — which it does when this view comes
   * into view, the same rule source control and the context cards follow — so a
   * cold launch reads no repositories at all.
   *
   * What this pane is FOR: deciding what to do with the folders a repository has
   * scattered around the disk. Every worktree is somebody's unfinished work
   * until proven otherwise, so the pane is built around not losing any:
   *
   *  - the everyday Remove is the desktop app's careful one, which refuses
   *    anything with changes, unpushed commits, or a lock on it;
   *  - Back up copies everything out first, including the files git was never
   *    told about;
   *  - Delete anyway is the only button that can lose work, and it goes through
   *    a dialog that lists what dies and asks for the folder name typed out.
   *
   * EVERY one of those, and the quiet "Clear this entry" on a row whose folder
   * is already gone, is asked about first. That last one is why: it used to act
   * on the first click, and on an app build that clears folder-gone rows
   * together it took another row with it, which nobody had been told.
   *
   * The rows themselves are worked out by `worktreeManagerRows.ts` (pure and
   * tested); this file arranges them and nothing else.
   */
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import {
    buildWorktreeManagerRows,
    describeRemovalQuestion,
    filterWorktreeRows,
    otherFolderGoneBranches,
    summarizeWorktreeManager,
    type WorktreeManagerRow,
    type WorktreeRemovalKind
  } from '$lib/shell/worktrees/worktreeManagerRows';
  import {
    toggleWorktreeDetail,
    worktreeManager
  } from '$lib/shell/worktrees/worktreeManagerStore.svelte';
  import {
    archiveWorktree,
    clearWorktreeEntry,
    forceRemoveWorktree,
    refresh,
    removeWorktree
  } from '$lib/shell/worktrees/worktreeManagerService';

  import RemoveWorktreeDialog from './RemoveWorktreeDialog.svelte';
  import WorktreeDetail from './WorktreeDetail.svelte';
  import WorktreeRow from './WorktreeRow.svelte';

  interface Props {
    /**
     * Focus one of the shell's sessions. Left out, the sessions listed under a
     * worktree are plain text rather than links — which is what the pane does
     * before the shell has wired this up.
     */
    onOpenSession?: (ownedId: string) => void;
  }
  let { onOpenSession }: Props = $props();

  /** The row a removal question is open about, and which question it is. */
  let asking = $state<{ row: WorktreeManagerRow; kind: WorktreeRemovalKind } | null>(null);

  const rows = $derived(
    buildWorktreeManagerRows({
      worktrees: worktreeManager.worktrees,
      repositories: worktreeManager.repositories,
      sessions: worktreeManager.sessions,
      primaryPath: worktreeManager.root
    })
  );
  const shown = $derived(filterWorktreeRows(rows, worktreeManager.filter));
  const summary = $derived(summarizeWorktreeManager(rows));
  const paneBusy = $derived(worktreeManager.busyAction !== '');
  /** The filter is hiding rows, and saying so beats an unexplained short list. */
  const hiddenByFilter = $derived(rows.length - shown.length);

  /**
   * The open question, written out by the pure function so the exact wording —
   * including the sentence about an older app build clearing more than one row
   * — can be read in a test.
   */
  const question = $derived(
    asking
      ? describeRemovalQuestion(asking.row, {
          kind: asking.kind,
          pruneSingleRow: worktreeManager.pruneSingleRowSupport,
          otherFolderGoneBranches: otherFolderGoneBranches(rows, asking.row)
        })
      : null
  );

  function ask(row: WorktreeManagerRow, kind: WorktreeRemovalKind): void {
    asking = { row, kind };
  }

  /** Do the thing that was asked about, whichever of the three it was. */
  function confirmAsked(): void {
    const open = asking;
    asking = null;
    if (!open) return;
    if (open.kind === 'clear') void clearWorktreeEntry(open.row.path, open.row.branch);
    else if (open.kind === 'force') void forceRemoveWorktree(open.row.path);
    else void removeWorktree(open.row.path);
  }
</script>

<div class="flex h-full w-full min-w-0 flex-col bg-[var(--color-bg)] text-[var(--color-text)]">
  <header
    class="flex shrink-0 items-center justify-between gap-2 border-b border-[var(--color-border)] px-2.5 py-2"
  >
    <div class="min-w-0">
      <p class="m-0 truncate text-[13px] leading-[1.35] text-[var(--color-text)]">
        {worktreeManager.projectName || 'Worktrees'}
      </p>
      <p class="m-0 truncate text-[12px] leading-[1.4] text-[var(--color-text-2)]">
        {worktreeManager.activated ? summary : 'Not looked yet'}
      </p>
    </div>
    <Button
      size="xs"
      variant="ghost"
      class="shrink-0 text-[12px] text-[var(--color-text-2)] hover:text-foreground"
      disabled={!worktreeManager.activated || worktreeManager.loading}
      onclick={() => void refresh()}
    >
      <RefreshCw aria-hidden="true" />
      {worktreeManager.loading ? 'Reading…' : 'Refresh'}
    </Button>
  </header>

  {#if !worktreeManager.activated}
    <p class="m-0 px-2.5 py-3 text-[13px] leading-[1.5] text-[var(--color-text-2)]">
      Pick a session and this will show the worktrees of the repository it belongs to.
    </p>
  {:else if worktreeManager.unavailableReason}
    <p class="m-0 px-2.5 py-3 text-[13px] leading-[1.5] text-[var(--color-text-2)]">
      {worktreeManager.unavailableReason}
    </p>
  {:else if worktreeManager.error}
    <div class="flex flex-col items-start gap-1.5 px-2.5 py-3">
      <p class="m-0 text-[13px] leading-[1.5] text-[var(--color-bad)]">{worktreeManager.error}</p>
      <Button
        size="xs"
        variant="ghost"
        class="text-[12px] text-[var(--color-text-2)] hover:text-foreground"
        onclick={() => void refresh()}
      >
        Try again
      </Button>
    </div>
  {:else}
    <div class="shrink-0 px-2.5 py-2">
      <Input
        class="h-7 text-[13px]"
        placeholder="Filter by branch, task, or folder"
        autocomplete="off"
        spellcheck="false"
        bind:value={worktreeManager.filter}
      />
    </div>

    {#if worktreeManager.actionError}
      <p class="m-0 px-2.5 pb-1.5 text-[12px] leading-[1.5] text-[var(--color-bad)]">
        {worktreeManager.actionError}
      </p>
    {:else if worktreeManager.actionMessage}
      <p class="m-0 px-2.5 pb-1.5 text-[12px] leading-[1.5] text-[var(--color-text-2)]">
        {worktreeManager.actionMessage}
      </p>
    {/if}

    <div class="min-h-0 flex-1 overflow-y-auto px-1.5 pb-2">
      {#if rows.length === 0}
        <p class="m-0 px-1 py-2 text-[13px] leading-[1.5] text-[var(--color-text-2)]">
          {worktreeManager.loading
            ? 'Looking for worktrees…'
            : 'This repository has no worktrees beside its main checkout.'}
        </p>
      {:else}
        <ul class="m-0 flex list-none flex-col gap-0.5 p-0">
          {#each shown as row (row.path)}
            <WorktreeRow
              {row}
              expanded={worktreeManager.selectedPath === row.path}
              busy={worktreeManager.busyPath === row.path}
              {paneBusy}
              forceSupport={worktreeManager.forceRemoveSupport}
              onToggle={() => toggleWorktreeDetail(row.path)}
              onAskRemove={() => ask(row, 'remove')}
              onArchive={() => void archiveWorktree(row.path)}
              onAskClear={() => ask(row, 'clear')}
              onAskForceRemove={() => ask(row, 'force')}
            >
              {#snippet detail()}
                <WorktreeDetail {row} {onOpenSession} />
              {/snippet}
            </WorktreeRow>
          {/each}
        </ul>
        {#if hiddenByFilter > 0}
          <p class="m-0 px-1 pt-1.5 text-[12px] leading-[1.5] text-[var(--color-text-2)]">
            {hiddenByFilter}
            {hiddenByFilter === 1 ? 'worktree is' : 'worktrees are'} hidden by the filter.
          </p>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<RemoveWorktreeDialog
  {question}
  folderName={asking?.row.folderName ?? ''}
  open={asking !== null}
  onOpenChange={(open) => {
    if (!open) asking = null;
  }}
  onConfirm={confirmAsked}
/>
