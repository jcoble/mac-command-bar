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
   * The rows themselves are worked out by `worktreeManagerRows.ts` (pure and
   * tested); this file arranges them and nothing else.
   */
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import {
    buildWorktreeManagerRows,
    filterWorktreeRows,
    summarizeWorktreeManager,
    type WorktreeManagerRow
  } from '$lib/shell/worktrees/worktreeManagerRows';
  import {
    toggleWorktreeDetail,
    worktreeManager
  } from '$lib/shell/worktrees/worktreeManagerStore.svelte';
  import {
    archiveWorktree,
    forceRemoveWorktree,
    refresh,
    removeWorktree
  } from '$lib/shell/worktrees/worktreeManagerService';

  import ForceRemoveDialog from './ForceRemoveDialog.svelte';
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

  /** The worktree the "delete it anyway" question is being asked about. */
  let asking = $state<WorktreeManagerRow | null>(null);

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

  function askForceRemove(row: WorktreeManagerRow): void {
    asking = row;
  }

  function confirmForceRemove(): void {
    const row = asking;
    asking = null;
    if (row) void forceRemoveWorktree(row.path);
  }
</script>

<div class="flex h-full w-full min-w-0 flex-col bg-[var(--color-bg)] text-[var(--color-text)]">
  <header
    class="flex shrink-0 items-center justify-between gap-2 border-b border-[var(--color-border)] px-2.5 py-2"
  >
    <div class="min-w-0">
      <p class="m-0 truncate text-[13px] leading-[1.35] text-[#e6e6ee]">
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
              onRemove={() => void removeWorktree(row.path)}
              onArchive={() => void archiveWorktree(row.path)}
              onAskForceRemove={() => askForceRemove(row)}
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

<ForceRemoveDialog
  row={asking}
  open={asking !== null}
  onOpenChange={(open) => {
    if (!open) asking = null;
  }}
  onConfirm={confirmForceRemove}
/>
