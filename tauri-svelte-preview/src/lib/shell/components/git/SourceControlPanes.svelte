<script lang="ts">
  /**
   * SourceControlPanes.svelte — the whole source-control surface: the
   * repository strip, the changed files with the commit box, and the commit
   * history drawn as a graph.
   *
   * It replaces the body of `GitPanel.svelte` and keeps that panel's contract:
   * no required props, no IO of its own, no `$effect`. Every git call is made
   * by `gitService` (the working copy) or `gitCommitFilesService` (a commit's
   * files), and only ever from a click. The integrator points the panel at a
   * repository with `gitService.activate(root)`, exactly as it does today.
   *
   * THE DIFF IS NOT DRAWN HERE. Clicking a file — whether a changed file or a
   * file inside a commit — writes it into the state `GitDiffView` already
   * reads, so the changes show up wherever that view is mounted. This round
   * that is meant to be a panel of its own in the middle of the shell; see
   * `_(git)-INTEGRATION.md` next to this file.
   *
   * WRITING vs READING. In the desktop app everything works. In a browser the
   * dev server can READ the repository (see `src/lib/server/gitBridge.ts`) but
   * deliberately cannot change it, so committing, staging and the remote
   * buttons are switched off there and say why. A browser tab left open on a
   * repository with work in progress must not be able to commit it by accident.
   */
  import { canChangeRepository, READ_ONLY_IN_BROWSER_MESSAGE } from '$lib/shell/git/gitBackendExtra';
  import {
    gitCommitFilesService as defaultCommitFilesService,
    type GitCommitFilesService
  } from '$lib/shell/git/gitCommitFilesService';
  import {
    gitCommitFiles as defaultCommitFilesState,
    type GitCommitFilesState
  } from '$lib/shell/git/gitCommitFilesStore.svelte';
  import { isNotARepositoryError } from '$lib/shell/git/gitPanelStore.svelte';
  import { gitService as defaultService, type GitService } from '$lib/shell/git/gitService';

  import ChangesPane from './ChangesPane.svelte';
  import GraphPane from './GraphPane.svelte';
  import RepoPane from './RepoPane.svelte';

  interface Props {
    /** The working-copy service. The shell's singleton unless a page says otherwise. */
    service?: GitService;
    /** The per-commit file service, likewise. */
    commitFiles?: GitCommitFilesService;
    /** The state that service writes, read by the graph's open rows. */
    commitFilesState?: GitCommitFilesState;
    /** Can this page change the repository? Defaults to "only in the desktop app". */
    canWrite?: boolean;
  }
  let {
    service = defaultService,
    commitFiles = defaultCommitFilesService,
    commitFilesState = defaultCommitFilesState,
    canWrite = canChangeRepository()
  }: Props = $props();

  const panel = $derived(service.state);

  /** A folder with no repository in it is an ordinary thing to be looking at,
   * not a fault — so it gets a plain sentence instead of git's `fatal:` line.
   * Any other failure keeps its own message. */
  const notARepository = $derived(
    isNotARepositoryError(panel.statusError) || isNotARepositoryError(panel.historyError)
  );
</script>

<div
  class="flex h-full min-h-0 w-full flex-col bg-[var(--color-bg)] text-[var(--color-text)]"
  aria-label="Source control"
>
  <RepoPane {panel} {service} {canWrite} readOnlyReason={READ_ONLY_IN_BROWSER_MESSAGE} />

  {#if !panel.activated}
    <p class="px-2 py-2 text-[13px] leading-[18px] text-[var(--color-text-2)]">
      No project selected yet. Pick a session and this panel will show that folder's source
      control.
    </p>
  {:else if panel.desktopOnly}
    <p class="px-2 py-2 text-[13px] leading-[18px] text-[var(--color-text-2)]">
      Source control runs in the desktop app only. Nothing is loaded here.
    </p>
  {:else if notARepository}
    <p class="px-2 py-2 text-[13px] leading-[18px] text-[var(--color-text-2)]">
      This folder is not a git repository.
    </p>
  {:else}
    <ChangesPane {panel} {service} {canWrite} readOnlyReason={READ_ONLY_IN_BROWSER_MESSAGE} />
    <GraphPane {panel} {service} {commitFiles} files={commitFilesState} />
  {/if}
</div>
