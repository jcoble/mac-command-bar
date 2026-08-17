<script lang="ts">
  /**
   * FilesPanel.svelte — the Files tab of the right column.
   *
   * The file tree of the active session's checkout. Folders open and close,
   * clicking a file opens it in the center Editor tab, and a filter box narrows
   * the list. Nothing here changes a file: no rename, no move, no delete.
   *
   * This panel is a PRODUCER on `openFileBus` and never subscribes to it — a
   * listener here would swallow the request the editor is waiting for. It calls
   * `openFileInEditor`, which puts the request on the bus and brings the editor
   * forward in one go.
   *
   * The tree maths lives in `fileTreeModel.ts` (pure, tested). The scan itself
   * belongs to `explorerService`, which the shell activates when a session is
   * picked; this panel only offers Refresh and Stop.
   */
  import FolderTree from '@lucide/svelte/icons/folder-tree';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Square from '@lucide/svelte/icons/square';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

  import { Button } from '$lib/components/ui/button/index.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { PanelHeader } from '$lib/components/ui/panel-header/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import {
    EXPLORER_SCAN_LIMIT,
    refresh,
    scanRoot,
    stopScan
  } from '$lib/shell/explorer/explorerService';
  import {
    explorer,
    explorerRecords,
    selectPath,
    setQuery,
    setScrollTop,
    setViewportHeight
  } from '$lib/shell/explorer/explorerStore.svelte';
  import { projectRootLabel } from '$lib/shell/explorer/explorerTree';
  import { openFileInEditor } from '$lib/shell/workbenchNavigation';
  import { filterSourceRecords } from '$lib/sourceData';

  import FileTreeRow from './FileTreeRow.svelte';
  import {
    allDirectoryPaths,
    fileTreeNodesFromRecords,
    toggleDirectory,
    visibleFileTreeNodes,
    windowFileTreeNodes,
    type FileTreeNode
  } from './fileTreeModel.ts';

  interface Props {
    /** True while this panel's tab is the selected one. */
    visible: boolean;
    /** The active session's working folder, or '' when nothing is selected. */
    root: string;
    /** The active session's ownedId, or null. */
    ownedId: string | null;
  }
  let { visible, root }: Props = $props();

  /**
   * Which folders are open, by absolute path. Component state on purpose: an
   * open tree is a place in a session, not a setting, and v1 does not carry it
   * across a reload.
   */
  let expanded = $state<Set<string>>(new Set());
  /** The scrolling element inside the kit's ScrollArea, once it exists. */
  let viewport = $state<HTMLElement | null>(null);

  const records = $derived(explorerRecords());
  const filtering = $derived(explorer.query.trim().length > 0);
  const matchedRecords = $derived(filterSourceRecords(records, explorer.query));
  const nodes = $derived(
    fileTreeNodesFromRecords(matchedRecords, { rootPath: explorer.root ?? '' })
  );
  /** Filtering opens everything: a match six folders down should be visible
   * without any clicking. */
  const openFolders = $derived(filtering ? allDirectoryPaths(nodes) : expanded);
  const rows = $derived(visibleFileTreeNodes(nodes, openFolders));
  const renderWindow = $derived(
    windowFileTreeNodes(rows, explorer.scrollTop, explorer.viewportHeight)
  );
  const rootLabel = $derived(projectRootLabel(explorer.root ?? root));
  const listedCount = $derived(filtering ? matchedRecords.length : records.length);

  /**
   * Follow the tree's own height and scroll offset, so the rendered window
   * matches what is on screen. Only while this tab is in front: a hidden panel
   * measures zero, which is not a measurement (`setViewportHeight` drops it),
   * and there is nothing to follow while nobody can see it.
   */
  $effect(() => {
    const node = viewport;
    if (!node || !visible) return;
    const onScroll = () => setScrollTop(node.scrollTop);
    const observer = new ResizeObserver(() => setViewportHeight(node.clientHeight));
    node.addEventListener('scroll', onScroll, { passive: true });
    observer.observe(node);
    setViewportHeight(node.clientHeight);
    return () => {
      node.removeEventListener('scroll', onScroll);
      observer.disconnect();
    };
  });

  function onFilterInput(event: Event & { currentTarget: HTMLInputElement }): void {
    setQuery(event.currentTarget.value);
    if (viewport) viewport.scrollTop = 0;
  }

  /** A folder opens and closes; a file is opened in the center Editor tab. */
  function onRowClick(node: FileTreeNode): void {
    if (node.isDirectory) {
      expanded = toggleDirectory(expanded, node);
      return;
    }
    selectPath(node.path);
    const projectRoot = (explorer.root ?? root ?? '').trim();
    openFileInEditor({ path: node.path, projectRoot: projectRoot || undefined });
  }
</script>

<div class="flex h-full min-h-0 w-full flex-col text-foreground">
  <PanelHeader title="Files" count={explorer.activated ? listedCount : null}>
    {#snippet actions()}
      {#if explorer.activated}
        <Input
          type="search"
          class="h-7 w-32"
          placeholder="Filter"
          aria-label="Filter files"
          value={explorer.query}
          oninput={onFilterInput}
        />
      {/if}
      {#if explorer.scanning}
        <IconButton label="Stop listing files" onclick={() => stopScan()}>
          <Square />
        </IconButton>
      {:else}
        <IconButton
          label="List this project's files again"
          disabled={!explorer.root}
          onclick={() => refresh()}
        >
          <RefreshCw />
        </IconButton>
      {/if}
    {/snippet}
    {rootLabel}
  </PanelHeader>

  {#if !explorer.activated}
    <EmptyState
      title="No session selected"
      body="Pick a session and the files in its checkout appear here."
    >
      {#snippet icon()}<FolderTree />{/snippet}
    </EmptyState>
  {:else if explorer.error}
    <EmptyState title="Could not list the files" body={explorer.error}>
      {#snippet icon()}<TriangleAlert />{/snippet}
      {#snippet actions()}
        <Button
          size="sm"
          variant="secondary"
          onclick={() => explorer.root && void scanRoot(explorer.root)}
        >
          Try again
        </Button>
      {/snippet}
    </EmptyState>
  {:else if explorer.scanning && records.length === 0}
    <EmptyState title="Listing files…" body="Reading this project's folders.">
      {#snippet icon()}<FolderTree />{/snippet}
    </EmptyState>
  {:else if rows.length === 0}
    <EmptyState
      title={filtering ? 'Nothing matches that filter' : 'No source files here'}
      body={filtering
        ? 'Clear the filter to see the whole tree.'
        : 'This folder has no files the app can open, or it could not be read.'}
    >
      {#snippet icon()}<FolderTree />{/snippet}
    </EmptyState>
  {:else}
    {#if explorer.truncated}
      <p class="border-b px-3 py-2 text-sm leading-snug text-muted-foreground">
        Showing the first {(explorer.limit || EXPLORER_SCAN_LIMIT).toLocaleString()} files — this project
        has more.
      </p>
    {/if}
    <ScrollArea class="min-h-0 flex-1" bind:viewportRef={viewport}>
      <div class="flex flex-col px-2 pb-2" aria-label="Project files">
        <div aria-hidden="true" style={`height: ${renderWindow.topSpacerHeight}px`}></div>
        {#each renderWindow.nodes as node (node.path)}
          <FileTreeRow
            {node}
            expanded={openFolders.has(node.path)}
            selected={!node.isDirectory && node.path === explorer.selectedPath}
            onclick={onRowClick}
          />
        {/each}
        <div aria-hidden="true" style={`height: ${renderWindow.bottomSpacerHeight}px`}></div>
      </div>
    </ScrollArea>
    <p class="border-t px-3 py-2 text-sm leading-snug text-muted-foreground">
      Source files only, and the list is not watched for changes — press Refresh after adding or
      removing files.
    </p>
  {/if}
</div>

