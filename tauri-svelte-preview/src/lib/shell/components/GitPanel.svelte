<script lang="ts">
  /**
   * GitPanel.svelte — the /next shell's source-control panel.
   *
   * Self-contained by the shell's panel contract: no required props, no IO in
   * this file. It reads `gitPanel` and calls `gitService` from user events only.
   * Nothing loads until the integrator calls `gitService.activate(root)` with
   * the active session's project folder; until then the panel says so and stays
   * completely quiet, which is what keeps a cold launch free of git calls.
   *
   * The grouping and the wording of every row come from
   * `gitPanelStore.svelte.ts`, which is where the node test can reach them. The
   * per-file badge and status words are computed by the backend — this panel
   * never reads raw git output.
   *
   * Deliberately NOT here: branch switching and branch lists. There is no
   * backend command for them, and inventing one is another lane's decision.
   */
  import GitDiffView from './GitDiffView.svelte';
  import {
    buildGitStatusFileGroups,
    describeGitBranch,
    describeGitFileChange,
    describeGitStatusGroups,
    gitFileTitle,
    gitPanel,
    gitStatusGroupActionLabel,
    hasStagedChanges,
    repositoryLabel,
    type GitStatusFileGroup
  } from '$lib/shell/git/gitPanelStore.svelte';
  import { absolutePathWithin, gitService } from '$lib/shell/git/gitService';
  import { requestOpenFile } from '$lib/shell/openFileBus';
  import type { ProjectGitFileStatus } from '$lib/tauriSource';

  interface Props {
    /**
     * Show the selected file's changes inside this panel. Turn it off when the
     * shell hosts `GitDiffView` somewhere else, so the diff is not drawn twice.
     */
    showDiff?: boolean;
  }
  let { showDiff = true }: Props = $props();

  const groups = $derived(buildGitStatusFileGroups(gitPanel.status?.files ?? []));
  const groupSummary = $derived(describeGitStatusGroups(groups));
  const branchLine = $derived(describeGitBranch(gitPanel.status));
  const busy = $derived(gitPanel.actionBusy !== '');
  const canCommit = $derived(
    !busy && gitPanel.commitMessage.trim() !== '' && hasStagedChanges(gitPanel.status)
  );

  function runGroupAction(group: GitStatusFileGroup): void {
    const paths = group.files.map((file) => file.relativePath);
    if (group.action === 'stage') void gitService.stagePaths(paths);
    else void gitService.unstagePaths(paths);
  }

  function runFileAction(group: GitStatusFileGroup, file: ProjectGitFileStatus): void {
    if (group.action === 'stage') void gitService.stagePaths([file.relativePath]);
    else void gitService.unstagePaths([file.relativePath]);
  }

  /** Ask the editor to open this file. The editor lane listens on the same bus. */
  function openInEditor(file: ProjectGitFileStatus): void {
    const root = gitPanel.root;
    if (!root) return;
    requestOpenFile({ path: absolutePathWithin(root, file.relativePath) });
  }

  function commitDay(committedAt: string): string {
    return committedAt.split('T')[0] ?? committedAt;
  }
</script>

<div class="git-panel">
  <header class="head">
    <div class="head-text">
      <p class="title">{repositoryLabel(gitPanel.root) || 'Source control'}</p>
      <p class="branch">{branchLine}</p>
    </div>
    <button
      class="chip"
      onclick={() => void gitService.refresh()}
      disabled={!gitPanel.activated || gitPanel.statusLoading}
    >
      {gitPanel.statusLoading ? 'Refreshing…' : 'Refresh'}
    </button>
  </header>

  {#if !gitPanel.activated}
    <p class="notice">
      No project selected yet. Pick a session and this panel will show that folder's source
      control.
    </p>
  {:else if gitPanel.desktopOnly}
    <p class="notice">
      Source control runs in the desktop app only. Nothing is loaded here.
    </p>
  {:else}
    <div class="scroll">
      <section class="block">
        <div class="row-of-buttons">
          <button class="chip" disabled={busy} onclick={() => void gitService.runRemoteAction('fetch')}>
            {gitPanel.actionBusy === 'fetch' ? 'Fetching…' : 'Fetch'}
          </button>
          <button class="chip" disabled={busy} onclick={() => void gitService.runRemoteAction('pull')}>
            {gitPanel.actionBusy === 'pull' ? 'Pulling…' : 'Pull'}
          </button>
          <button class="chip" disabled={busy} onclick={() => void gitService.runRemoteAction('push')}>
            {gitPanel.actionBusy === 'push' ? 'Pushing…' : 'Push'}
          </button>
        </div>

        <textarea
          class="commit-box"
          rows="2"
          placeholder="Describe what you changed, then commit the staged files."
          bind:value={gitPanel.commitMessage}
        ></textarea>
        <button class="commit" disabled={!canCommit} onclick={() => void gitService.commit()}>
          {gitPanel.actionBusy === 'commit' ? 'Committing…' : 'Commit staged files'}
        </button>

        {#if gitPanel.actionError}
          <p class="line error">{gitPanel.actionError}</p>
        {:else if gitPanel.actionStatus}
          <p class="line">{gitPanel.actionStatus}</p>
        {/if}
        {#if gitPanel.statusError}
          <p class="line error">{gitPanel.statusError}</p>
        {/if}
      </section>

      <section class="block">
        <div class="block-head">
          <h2>Changes</h2>
          <span class="count">{groupSummary}</span>
        </div>

        {#if groups.length === 0}
          <p class="notice small">
            {gitPanel.statusLoading ? 'Reading the repository…' : 'Nothing has changed yet.'}
          </p>
        {/if}

        {#each groups as group (group.id)}
          <div class="group-head">
            <span class="group-label">{group.label} · {group.files.length}</span>
            <button class="chip small" disabled={busy} onclick={() => runGroupAction(group)}>
              {gitStatusGroupActionLabel(group)}
            </button>
          </div>
          <ul class="rows">
            {#each group.files as file (group.id + file.relativePath)}
              <li class="row" class:selected={gitPanel.selectedPath === file.relativePath}>
                <button
                  class="row-main"
                  title={gitFileTitle(file)}
                  onclick={() => void gitService.selectFile(file)}
                >
                  <span class="badge">{file.badge || '·'}</span>
                  <span class="row-text">
                    <span class="row-title">{file.relativePath}</span>
                    <span class="row-meta">{describeGitFileChange(file)}</span>
                  </span>
                </button>
                <button class="row-side" title="Open this file" onclick={() => openInEditor(file)}>
                  Open
                </button>
                <button
                  class="row-side"
                  disabled={busy}
                  title={group.action === 'stage' ? 'Stage this file' : 'Unstage this file'}
                  onclick={() => runFileAction(group, file)}
                >
                  {group.action === 'stage' ? 'Stage' : 'Unstage'}
                </button>
              </li>
            {/each}
          </ul>
        {/each}
      </section>

      <section class="block">
        <div class="block-head">
          <h2>Recent commits</h2>
          <span class="count">{gitPanel.history.length}</span>
        </div>
        {#if gitPanel.historyError}
          <p class="line error">{gitPanel.historyError}</p>
        {:else if gitPanel.historyLoading && gitPanel.history.length === 0}
          <p class="notice small">Reading the commit history…</p>
        {:else if gitPanel.history.length === 0}
          <p class="notice small">No commits yet.</p>
        {:else}
          <ul class="rows">
            {#each gitPanel.history as entry (entry.sha)}
              <li class="commit-row" title={entry.subject}>
                <span class="sha">{entry.shortSha}</span>
                <span class="row-text">
                  <span class="row-title">{entry.subject}</span>
                  <span class="row-meta">{entry.author} · {commitDay(entry.committedAt)}</span>
                </span>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      {#if showDiff && gitPanel.selectedPath !== ''}
        <section class="block diff-block">
          <div class="block-head">
            <h2>What changed</h2>
            <button class="chip small" onclick={() => gitService.clearSelection()}>Close</button>
          </div>
          <div class="diff-host">
            <GitDiffView />
          </div>
        </section>
      {/if}
    </div>
  {/if}
</div>

<style>
  .git-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    min-height: 0;
    background: #101014;
    color: #d8d8e0;
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
    font-size: 12px;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    flex: 0 0 auto;
    padding: 10px;
    border-bottom: 1px solid #22222c;
  }

  .head-text {
    min-width: 0;
  }

  .title,
  .branch {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    color: #e6e6ee;
  }

  .branch {
    color: #6d6d7d;
    font-size: 10px;
  }

  .scroll {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .block-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  h2 {
    margin: 0;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: #7b7b8c;
  }

  .count {
    font-size: 10px;
    color: #5d5d6b;
  }

  .row-of-buttons {
    display: flex;
    gap: 6px;
  }

  .chip {
    border: 1px solid #2a2a34;
    border-radius: 5px;
    background: transparent;
    color: #9a9aad;
    font-family: inherit;
    font-size: 10px;
    padding: 3px 8px;
    cursor: pointer;
  }

  .chip.small {
    padding: 2px 6px;
  }

  .chip:hover:not(:disabled) {
    border-color: #3d3d4a;
    color: #d8d8e0;
  }

  .chip:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .commit-box {
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
    border: 1px solid #22222c;
    border-radius: 6px;
    background: #17171d;
    color: #d8d8e0;
    font-family: inherit;
    font-size: 11px;
    padding: 6px 8px;
  }

  .commit-box::placeholder {
    color: #4c4c5a;
  }

  .commit-box:focus {
    outline: none;
    border-color: #3d3d4a;
  }

  .commit {
    align-self: flex-start;
    border: 1px solid #2a2a34;
    border-radius: 5px;
    background: #17171d;
    color: #d8d8e0;
    font-family: inherit;
    font-size: 11px;
    padding: 4px 10px;
    cursor: pointer;
  }

  .commit:hover:not(:disabled) {
    border-color: #3d3d4a;
  }

  .commit:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .line {
    margin: 0;
    color: #6d6d7d;
    font-size: 10px;
  }

  .line.error {
    color: #ff9d9d;
  }

  .notice {
    margin: 0;
    padding: 10px;
    color: #6d6d7d;
    font-size: 11px;
  }

  .notice.small {
    padding: 2px 0;
    font-size: 10px;
  }

  .group-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 4px 2px 0;
  }

  .group-label {
    color: #9a9aad;
    font-size: 10px;
  }

  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .row {
    display: flex;
    align-items: stretch;
    gap: 2px;
    border-radius: 6px;
  }

  .row:hover {
    background: #1a1a22;
  }

  .row.selected {
    background: #22222c;
  }

  .row-main {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .row-side {
    flex: 0 0 auto;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: #6d6d7d;
    font-family: inherit;
    font-size: 10px;
    padding: 0 7px;
    cursor: pointer;
  }

  .row-side:hover:not(:disabled) {
    background: #24242f;
    color: #d8d8e0;
  }

  .row-side:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .badge,
  .sha {
    flex: 0 0 auto;
    border-radius: 4px;
    background: #24242f;
    color: #9a9aad;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 9px;
    padding: 1px 5px;
  }

  .row-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .row-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #e6e6ee;
    font-size: 11px;
  }

  .row-meta {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #6d6d7d;
    font-size: 10px;
  }

  .commit-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    min-width: 0;
    border-radius: 6px;
  }

  .commit-row:hover {
    background: #1a1a22;
  }

  .diff-block {
    min-height: 220px;
  }

  .diff-host {
    height: 320px;
    border: 1px solid #22222c;
    border-radius: 6px;
    overflow: hidden;
  }
</style>
