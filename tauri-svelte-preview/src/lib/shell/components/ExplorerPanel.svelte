<script lang="ts">
  /**
   * ExplorerPanel.svelte — the /next file explorer.
   *
   * A tree of the project's source files. Clicking a file asks the editor to
   * open it, through `openFileBus`; this panel is the bus's first producer and
   * deliberately does NOT subscribe to it (the bus hands a request made before
   * the editor exists to the first subscriber, so a listener here would swallow
   * the very request the editor is waiting for).
   *
   * No props: the panel is self-contained and fills whatever cell it lands in.
   * It stays inert — an empty state, zero backend calls — until the shell calls
   * `activate(root)` on `explorerService`, and it never measures itself while
   * hidden (a zero height from the resize watcher is ignored).
   *
   * The tree maths is not written here: rows come from `buildExplorerView` in
   * `explorerTree.ts`, which composes the audited helpers in `sourceData.ts`.
   */
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Folder from '@lucide/svelte/icons/folder';
  import FolderOpen from '@lucide/svelte/icons/folder-open';

  import FileIcon from './explorer/FileIcon.svelte';
  import {
    folderFileCountLabel,
    folderFileCounts,
    folderFileCountTitle
  } from './explorer/folderFileCounts.ts';
  import {
    refresh,
    scanRoot,
    stopScan,
    EXPLORER_SCAN_LIMIT
  } from '$lib/shell/explorer/explorerService';
  import {
    explorer,
    explorerRecords,
    selectPath,
    setQuery,
    setScrollTop,
    setViewportHeight,
    toggleFolderExpansion
  } from '$lib/shell/explorer/explorerStore.svelte';
  import {
    buildExplorerView,
    projectRootLabel,
    EXPLORER_ROW_HEIGHT
  } from '$lib/shell/explorer/explorerTree';
  import { requestOpenFile } from '$lib/shell/openFileBus';
  import { filterSourceRecords, type SourceTreeNode } from '$lib/sourceData';

  /** The scrolling tree container, captured by `measureViewport` rather than
   * `bind:this` — the action already receives the node, and a plain `let` that
   * the template also writes to would read as state that is never tracked. */
  let treeElement: HTMLElement | null = null;

  const records = $derived(explorerRecords());
  const view = $derived(
    buildExplorerView({
      records,
      query: explorer.query,
      expandedFolderIds: explorer.expandedFolderIds,
      scrollTop: explorer.scrollTop,
      viewportHeight: explorer.viewportHeight
    })
  );
  const filtering = $derived(explorer.query.trim().length > 0);
  const rootLabel = $derived(projectRootLabel(explorer.root));
  /**
   * Files under every folder. The badge has to mean "files in here, all the way
   * down" — a folder node's own `children.length` only counts what sits directly
   * inside it, which reads as a far smaller number than the folder holds.
   *
   * Deliberately NOT derived from `view.matchedRecords`, close as that is: the
   * view is rebuilt on every scroll frame, so counting off it would re-count the
   * whole project while the user drags the scrollbar. Reading the same filter
   * separately costs one extra pass when the filter box changes and nothing at
   * all when it does not.
   */
  const countedRecords = $derived(filterSourceRecords(records, explorer.query));
  const fileCounts = $derived(folderFileCounts(countedRecords));

  /** Watch the tree's own height so the visible-row window matches reality.
   * A zero reading means the panel is parked or behind another tab — not a
   * measurement, so `setViewportHeight` drops it. */
  function measureViewport(node: HTMLElement) {
    treeElement = node;
    const observer = new ResizeObserver(() => setViewportHeight(node.clientHeight));
    observer.observe(node);
    setViewportHeight(node.clientHeight);
    return {
      destroy() {
        observer.disconnect();
        if (treeElement === node) treeElement = null;
      }
    };
  }

  function onFilterInput(event: Event & { currentTarget: HTMLInputElement }): void {
    setQuery(event.currentTarget.value);
    if (treeElement) treeElement.scrollTop = 0;
  }

  function onTreeScroll(event: Event & { currentTarget: HTMLDivElement }): void {
    setScrollTop(event.currentTarget.scrollTop);
  }

  /** A folder opens and closes; a file is selected and sent to the editor. */
  function onRowClick(node: SourceTreeNode): void {
    if (node.file === null) {
      toggleFolderExpansion(node.id);
      return;
    }
    selectPath(node.file.path);
    requestOpenFile({
      path: node.file.path,
      projectRoot: explorer.root ?? undefined
    });
  }

  function onRetry(): void {
    if (explorer.root) void scanRoot(explorer.root);
  }
</script>

<div class="explorer">
  <header class="head">
    <h2>Files</h2>
    {#if rootLabel}
      <span class="root" title={explorer.root ?? ''}>{rootLabel}</span>
    {/if}
    <span class="spacer"></span>
    {#if explorer.scanning}
      <button type="button" class="action" onclick={() => stopScan()}>Stop</button>
    {:else}
      <button
        type="button"
        class="action"
        disabled={!explorer.root}
        title="List the project's files again"
        onclick={() => refresh()}
      >
        Refresh
      </button>
    {/if}
  </header>

  {#if !explorer.activated}
    <p class="empty">Pick a session and this project's files appear here.</p>
  {:else}
    <div class="filter">
      <input
        type="search"
        placeholder="Filter files"
        aria-label="Filter files"
        value={explorer.query}
        oninput={onFilterInput}
      />
      <span class="count">
        {#if filtering}
          {view.matchedRecords.length} of {records.length}
        {:else}
          {records.length} files
        {/if}
      </span>
    </div>

    {#if explorer.error}
      <div class="notice error">
        <span>{explorer.error}</span>
        <button type="button" class="action" onclick={onRetry}>Try again</button>
      </div>
    {:else if explorer.truncated}
      <p class="notice">
        Showing the first {(explorer.limit || EXPLORER_SCAN_LIMIT).toLocaleString()} files — this project
        has more.
      </p>
    {/if}

    <div
      class="tree"
      use:measureViewport
      onscroll={onTreeScroll}
      aria-label="Project files"
      style={`--row-height: ${EXPLORER_ROW_HEIGHT}px`}
    >
      {#if explorer.scanning && records.length === 0}
        <p class="empty">Listing files…</p>
      {:else if view.rows.length === 0}
        <p class="empty">
          {filtering ? 'No files match that filter.' : 'No source files found in this project.'}
        </p>
      {:else}
        <div class="spacer-row" aria-hidden="true" style={`height: ${view.virtual.topSpacerHeight}px`}
        ></div>
        {#each view.virtual.rows as row (row.node.id)}
          {@const node = row.node}
          {@const isFolder = node.file === null}
          {@const isOpen = view.autoExpandFolders || explorer.expandedFolderIds.has(node.id)}
          <button
            type="button"
            class="row"
            class:folder={isFolder}
            class:selected={!isFolder && node.file?.path === explorer.selectedPath}
            style={`--level: ${row.level}`}
            title={node.relativePath}
            aria-expanded={isFolder ? isOpen : undefined}
            onclick={() => onRowClick(node)}
          >
            <span class="chevron" aria-hidden="true">
              {#if isFolder}
                {#if isOpen}
                  <ChevronDown size={12} strokeWidth={2} />
                {:else}
                  <ChevronRight size={12} strokeWidth={2} />
                {/if}
              {/if}
            </span>
            {#if isFolder}
              <span class="folder-icon" aria-hidden="true">
                {#if isOpen}
                  <FolderOpen size={14} strokeWidth={1.75} />
                {:else}
                  <Folder size={14} strokeWidth={1.75} />
                {/if}
              </span>
            {:else}
              <FileIcon fileName={node.name} size={14} />
            {/if}
            <span class="name">{node.name}</span>
            {#if isFolder}
              <span class="meta" title={folderFileCountTitle(fileCounts, node.id)}>
                {folderFileCountLabel(fileCounts, node.id)}
              </span>
            {/if}
          </button>
        {/each}
        <div
          class="spacer-row"
          aria-hidden="true"
          style={`height: ${view.virtual.bottomSpacerHeight}px`}
        ></div>
      {/if}
    </div>

    <footer class="note">
      Source files only — images, lock files and folders like node_modules are left out. The list is
      not watched for changes; press Refresh after adding or removing files.
    </footer>
  {/if}
</div>

<style>
  .explorer {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    min-height: 0;
    overflow: hidden;
    background: var(--color-bg);
    color: var(--color-text);
    font-family: ui-sans-serif, -apple-system, system-ui, sans-serif;
    font-size: 13px;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px 6px;
  }

  h2 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: var(--color-text-3);
  }

  .root {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text-2);
    font-size: 12px;
  }

  .spacer {
    flex: 1 1 auto;
  }

  .action {
    flex: 0 0 auto;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--color-text-2);
    font: inherit;
    font-size: 12px;
    padding: 2px 7px;
    cursor: pointer;
  }

  .action:hover:not(:disabled) {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .action:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .filter {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px 6px;
  }

  input[type='search'] {
    flex: 1 1 auto;
    min-width: 0;
    border: 1px solid var(--color-border);
    border-radius: 5px;
    background: var(--color-surface);
    color: var(--color-text);
    font: inherit;
    font-size: 13px;
    padding: 4px 7px;
  }

  input[type='search']::placeholder {
    color: var(--color-text-3);
  }

  .count {
    flex: 0 0 auto;
    color: var(--color-text-3);
    font-size: 12px;
    white-space: nowrap;
  }

  .notice {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 10px 6px;
    border-radius: 5px;
    background: var(--color-surface);
    color: var(--color-text-2);
    font-size: 12px;
    padding: 5px 7px;
  }

  .notice.error {
    background: var(--color-bad-bg);
    color: var(--color-bad);
  }

  .notice.error .action {
    background: var(--color-bad-bg);
    color: var(--color-bad);
  }

  .empty {
    margin: 0;
    padding: 8px 12px;
    color: var(--color-text-2);
    font-size: 13px;
  }

  .tree {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 0 4px 4px;
  }

  .spacer-row {
    width: 100%;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: var(--row-height);
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    padding: 0 6px 0 calc(6px + var(--level) * 11px);
  }

  .row:hover {
    background: var(--color-surface);
  }

  .row.selected {
    background: var(--color-elevated);
  }

  .row.folder .name {
    color: var(--color-text-2);
  }

  /* Both leading slots keep their width whether or not they draw anything, so
   * every name in a folder starts at the same x. */
  .chevron,
  .folder-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 auto;
    color: var(--color-text-3);
  }

  .chevron {
    width: 12px;
    height: 12px;
  }

  .folder-icon {
    width: 14px;
    height: 14px;
  }

  .name {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    flex: 0 0 auto;
    color: var(--color-text-3);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  .note {
    border-top: 1px solid var(--color-border);
    color: var(--color-text-3);
    font-size: 12px;
    line-height: 1.4;
    padding: 6px 10px;
  }

  button:focus-visible,
  input:focus-visible {
    outline: 1px solid var(--color-accent);
    outline-offset: -1px;
  }
</style>
