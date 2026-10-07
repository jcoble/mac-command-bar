<script lang="ts">
  /**
   * GitDiffView.svelte — the Changes tab.
   *
   * Shows every uncommitted change in the session's working copy, file under
   * file, with the changed-file tree beside it (`MultiFileDiff`). A file
   * picked in Source Control is the same view, scrolled to that file. A file
   * opened from a commit or a pull request is shown on its own.
   *
   * Reads go through `gitService`; the tab is only in the page while it is the
   * tab in front, so everything read here is let go of when it is not.
   */
  import { untrack } from 'svelte';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { SegmentedControl } from '$lib/components/ui/segmented-control/index.js';
  import MultiFileDiff from '$lib/shell/components/git/MultiFileDiff.svelte';
  import { diffTextOf, parsedDiffOf } from '$lib/shell/git/diffRows';
  import { gitCommitFiles, gitCommitFilesView } from '$lib/shell/git/gitCommitFilesStore.svelte';
  import { gitPanel } from '$lib/shell/git/gitPanelStore.svelte';
  import { gitService } from '$lib/shell/git/gitService';
  import { requestOpenFile } from '$lib/shell/openFileBus';
  import { isDiffMode, type DiffMode } from '$lib/shell/sessionWorkspaces';
  import { showCenterTab, type OpenPullRequestDiffRequest } from '$lib/shell/workbenchNavigation';
  import type { SourceGitDiff } from '$lib/tauriSource';

  interface Props {
    rootAvailable?: boolean;
    /** The session's folder, used when Source Control has not pointed the panel anywhere yet. */
    sessionRoot?: string;
    mode?: DiffMode;
    onModeChange?: (mode: DiffMode) => void;
    pullRequestDiff?: OpenPullRequestDiffRequest | null;
  }
  let { rootAvailable = true, sessionRoot = '', mode = 'unified', onModeChange, pullRequestDiff = null }: Props = $props();

  /** Files read at once. Each read is a few short git processes. */
  const READS_AT_ONCE = 4;
  const MODES = [
    { value: 'unified', label: 'Unified' },
    { value: 'side-by-side', label: 'Side by side' }
  ];

  const commitSha = $derived(
    gitPanel.diffOwner ? gitCommitFilesView(gitCommitFiles, gitPanel.diffOwner).selectedCommitSha : ''
  );
  /** A commit's or a pull request's file is shown on its own, not as part of the working copy. */
  const singleMode = $derived(pullRequestDiff !== null || commitSha !== '');
  const single = $derived<SourceGitDiff | null>(pullRequestDiff?.diff ?? (commitSha ? gitPanel.selectedDiff : null));

  let working = $state.raw<SourceGitDiff[]>([]);
  let unreadable = $state.raw<string[]>([]);
  let pending = $state(0);
  let generation = 0;

  const files = $derived(singleMode ? (single ? [single] : []) : working);
  const totals = $derived(
    files.reduce(
      (sum, file) => {
        const parsed = parsedDiffOf(file);
        return { added: sum.added + parsed.addedCount, removed: sum.removed + parsed.removedCount };
      },
      { added: 0, removed: 0 }
    )
  );
  const scope = $derived(
    pullRequestDiff
      ? `${pullRequestDiff.repository} #${pullRequestDiff.number}`
      : commitSha
        ? `Commit ${commitSha.slice(0, 7)}`
        : 'Uncommitted'
  );

  function put(diff: SourceGitDiff): void {
    working = [...working.filter((file) => file.relativePath !== diff.relativePath), diff];
  }

  /** Read every changed file's diff, a few at a time, the picked one first.
   * Full file texts are dropped once the diff text is taken from them. */
  async function readAll(paths: string[]): Promise<void> {
    const id = ++generation;
    const wanted = new Set(paths);
    working = working.filter((file) => wanted.has(file.relativePath));
    // A folder of new files is listed by git as `folder/`; there is no single file to read.
    unreadable = paths.filter((path) => path.endsWith('/'));
    const queue = paths.filter((path) => !path.endsWith('/'));
    const first = queue.indexOf(gitPanel.selectedPath);
    if (first > 0) queue.unshift(...queue.splice(first, 1));
    pending = queue.length;

    const worker = async (): Promise<void> => {
      for (let path = queue.shift(); path !== undefined; path = queue.shift()) {
        try {
          const diff = await gitService.readWorkingDiff(path);
          if (id !== generation) return;
          if (diff) put({ ...diff, diff: diffTextOf(diff), originalContent: null, modifiedContent: null });
          else unreadable = [...unreadable, path];
        } catch {
          if (id !== generation) return;
          unreadable = [...unreadable, path];
        } finally {
          if (id === generation) pending -= 1;
        }
      }
    };
    await Promise.all(Array.from({ length: READS_AT_ONCE }, worker));
  }

  /** Re-read one file keeping its current text, so a collapsed run of lines can open. */
  async function loadFullText(path: string): Promise<void> {
    const id = generation;
    const diff = await gitService.readWorkingDiff(path).catch(() => null);
    if (diff && id === generation) put({ ...diff, diff: diffTextOf(diff), originalContent: null });
  }

  // The working copy follows the repository status: a new status (after a
  // commit, a stage, a refresh) means the diffs are read again.
  $effect(() => {
    if (singleMode || !rootAvailable) return;
    const root = gitPanel.root;
    const status = gitPanel.status;
    untrack(() => {
      if (!root) {
        if (sessionRoot) gitService.activate(sessionRoot);
      } else if (!status) {
        if (!gitPanel.statusLoading && !gitPanel.statusError && !gitPanel.desktopOnly) void gitService.refreshStatus();
      } else {
        void readAll(status.files.map((file) => file.relativePath));
      }
    });
  });

  function openAt(relativePath: string, line: number | null): void {
    const root = gitPanel.root;
    if (!rootAvailable || !root) return;
    requestOpenFile({
      path: `${root.replace(/\/+$/, '')}/${relativePath.replace(/^\/+/, '')}`,
      projectRoot: root,
      line: line && line > 0 ? line : undefined
    });
  }
</script>

<div class="diff-view">
  <header class="bar">
    <span class="scope">{scope}</span>
    <span class="counts"><em>+{totals.added}</em> <del>-{totals.removed}</del></span>
    {#if !singleMode && pending > 0}<span class="quiet">Reading {pending} more {pending === 1 ? 'file' : 'files'}…</span>{/if}
    <span class="spacer"></span>
    {#if pullRequestDiff}
      <button type="button" class="back" onclick={() => showCenterTab('pull-requests')}>Back to PR</button>
    {/if}
    <SegmentedControl
      size="sm"
      items={MODES}
      value={mode}
      aria-label="Diff layout"
      onValueChange={(value) => {
        if (isDiffMode(value)) onModeChange?.(value);
      }}
    />
    {#if !singleMode}
      <IconButton label="Refresh changes" size="xs" disabled={gitPanel.statusLoading} onclick={() => void gitService.refreshStatus()}>
        <RefreshCw size={14} />
      </IconButton>
    {/if}
  </header>

  {#if !rootAvailable}
    <p class="notice">Checkout/Worktree deleted.</p>
  {:else if singleMode && !pullRequestDiff && gitPanel.diffLoading}
    <p class="notice">Reading the changes…</p>
  {:else if singleMode && !pullRequestDiff && gitPanel.diffError}
    <p class="notice error">{gitPanel.diffError}</p>
  {:else if !singleMode && gitPanel.statusError}
    <p class="notice error">{gitPanel.statusError}</p>
  {:else if !singleMode && gitPanel.desktopOnly}
    <p class="notice">Reading the working copy needs the desktop app.</p>
  {:else if !singleMode && !gitPanel.status}
    <p class="notice">Reading the working copy…</p>
  {:else}
    {#if unreadable.length > 0}
      <p class="notice">Not shown here: {unreadable.join(', ')}</p>
    {/if}
    <MultiFileDiff
      {files}
      {mode}
      focusPath={singleMode ? '' : gitPanel.selectedPath}
      onOpenLine={pullRequestDiff ? undefined : openAt}
      onLoadFullText={singleMode ? undefined : (path) => void loadFullText(path)}
    />
  {/if}
</div>

<style>
  .diff-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    min-height: 0;
    background: var(--color-bg);
    color: var(--color-text-2);
    font-size: 13px;
  }

  .bar {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 6px 8px 6px 12px;
    border-bottom: 1px solid var(--color-border);
  }

  .scope {
    color: var(--color-text);
    white-space: nowrap;
  }

  .counts {
    font: 12px var(--font-mono);
    white-space: nowrap;
  }

  .counts em {
    color: var(--color-good);
    font-style: normal;
  }

  .counts del {
    color: var(--color-bad);
    text-decoration: none;
  }

  .quiet {
    color: var(--color-text-3);
    white-space: nowrap;
  }

  .spacer {
    flex: 1 1 auto;
  }

  .back {
    min-height: 28px;
    padding: 0 12px;
    border: 0;
    border-radius: 999px;
    background: var(--color-surface);
    color: var(--color-text-2);
    cursor: pointer;
    font: inherit;
  }

  .back:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .back:focus-visible {
    outline: 2px solid var(--color-focus-solid);
    outline-offset: 1px;
  }

  .notice {
    margin: 0;
    padding: 10px 12px;
    color: var(--color-text-3);
  }

  .notice.error {
    color: var(--color-bad);
  }
</style>
