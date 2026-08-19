<script lang="ts">
  /**
   * FilesPanel.svelte — the Files tab of the right column.
   *
   * The file tree of the active session's checkout. Folders open and close,
   * clicking a file opens it in the center Editor tab, and a filter box narrows
   * the list. Right-clicking a row offers the six things a person does to a
   * path: make a file or folder inside it, rename it, move it to the Trash,
   * show it in the Finder, or copy where it is.
   *
   * Those six go through Tauri's own plugins rather than commands of ours —
   * `plugin-fs` for the three that change the disk, `plugin-opener` for the
   * Finder and `plugin-clipboard-manager` for the path. The one exception is
   * Delete: `plugin-fs` only deletes for good, so it calls `move_to_trash`,
   * which puts the path somewhere it can be fetched back from.
   *
   * While a folder is listed it is also watched, so a file another program
   * writes shows up here without anyone pressing anything.
   *
   * This panel is a PRODUCER on `openFileBus` and never subscribes to it — a
   * listener here would swallow the request the editor is waiting for. It calls
   * `openFileInEditor`, which puts the request on the bus and brings the editor
   * forward in one go.
   *
   * The tree maths lives in `fileTreeModel.ts` (pure, tested), and which items
   * a row's menu carries in `filesPanelActions.ts` (pure, tested). The scan
   * itself belongs to `explorerService`, which the shell activates when a
   * session is picked; this panel asks it to run again and to stop.
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
  import { isNativeTauriRuntime, moveToTrashFromTauri } from '$lib/tauriSource';

  import FileTreeRow from './FileTreeRow.svelte';
  import {
    allDirectoryPaths,
    fileTreeNodesFromRecords,
    toggleDirectory,
    visibleFileTreeNodes,
    windowFileTreeNodes,
    type FileTreeNode
  } from './fileTreeModel.ts';
  import type { FilesPanelActionId } from './filesPanelActions.ts';

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

  /**
   * The three actions that need a name before they can run. Renaming puts the
   * field where the row was; making something new puts it under the folder it
   * will land in, which is where a person is already looking.
   */
  let pending = $state<{ kind: 'rename' | 'new-file' | 'new-folder'; path: string } | null>(null);
  /** What has been typed into that field so far. */
  let pendingName = $state('');
  /** The field itself, so it can take the keyboard as soon as it appears. */
  let entryField = $state<HTMLElement | null>(null);
  /** Why the last action did not happen. Cleared by the next one. */
  let actionError = $state<string | null>(null);

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

  /**
   * Watch the listed folder while this tab is in front, so a file another
   * program writes turns up without anyone pressing Refresh.
   *
   * Only while it is in front: a build running behind a hidden panel would
   * otherwise have it re-listing the project every third of a second for
   * nothing. A burst of writes is already collapsed into one refresh by the
   * plugin's own delay, and a refresh that arrives while the previous one is
   * still walking the tree is dropped rather than queued.
   */
  $effect(() => {
    const target = (explorer.root ?? '').trim();
    if (!visible || !target || !isNativeTauriRuntime()) return;

    let unwatch: (() => void) | null = null;
    let abandoned = false;
    void (async () => {
      try {
        const { watch } = await import('@tauri-apps/plugin-fs');
        const stop = await watch(
          target,
          () => {
            if (!explorer.scanning) refresh();
          },
          { recursive: true, delayMs: 300 }
        );
        if (abandoned) stop();
        else unwatch = stop;
      } catch {
        // Watching is a convenience. Without it the header's Refresh still works.
      }
    })();

    return () => {
      abandoned = true;
      unwatch?.();
      unwatch = null;
    };
  });

  /** The name field takes the keyboard the moment it appears. */
  $effect(() => {
    const field = entryField;
    if (!field) return;
    field.focus();
    // Renaming starts with the old name in the field, and a name being changed
    // is usually being replaced rather than edited.
    if (field instanceof HTMLInputElement) field.select();
  });

  function onFilterInput(event: Event & { currentTarget: HTMLInputElement }): void {
    setQuery(event.currentTarget.value);
    if (viewport) viewport.scrollTop = 0;
  }

  function describeError(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  /** The folder a path sits in. */
  function parentOf(path: string): string {
    const cut = path.lastIndexOf('/');
    return cut < 0 ? '' : path.slice(0, cut);
  }

  /**
   * True when nothing is at `target` yet. Both `rename` and `writeTextFile`
   * would go through whatever is already there, and a new file quietly emptying
   * an old one is the kind of loss nobody notices until much later.
   */
  async function isFree(target: string): Promise<boolean> {
    const { exists } = await import('@tauri-apps/plugin-fs');
    if (!(await exists(target))) return true;
    actionError = `Something is already called ${target.slice(target.lastIndexOf('/') + 1)} here.`;
    return false;
  }

  /** One menu item. The three that need a name open the field instead. */
  function onRowAction(node: FileTreeNode, id: FilesPanelActionId): void {
    actionError = null;
    if (id === 'copy-path' || id === 'reveal-in-finder' || id === 'delete') {
      void runNamelessAction(node, id);
      return;
    }
    if (id !== 'rename' && !expanded.has(node.path)) expanded = toggleDirectory(expanded, node);
    pending = { kind: id, path: node.path };
    pendingName = id === 'rename' ? node.name : '';
  }

  async function runNamelessAction(
    node: FileTreeNode,
    id: 'copy-path' | 'reveal-in-finder' | 'delete'
  ): Promise<void> {
    try {
      if (id === 'copy-path') {
        const { writeText } = await import('@tauri-apps/plugin-clipboard-manager');
        await writeText(node.path);
      } else if (id === 'reveal-in-finder') {
        const { revealItemInDir } = await import('@tauri-apps/plugin-opener');
        await revealItemInDir(node.path);
      } else {
        await moveToTrashFromTauri(node.path);
      }
    } catch (error) {
      actionError = describeError(error);
    }
  }

  /** Put the typed name on disk. An empty name means the person changed their mind. */
  async function commitEntry(): Promise<void> {
    const entry = pending;
    const name = pendingName.trim();
    pending = null;
    if (!entry || !name || name.includes('/')) return;

    try {
      const fs = await import('@tauri-apps/plugin-fs');
      if (entry.kind === 'rename') {
        const target = `${parentOf(entry.path)}/${name}`;
        if (target !== entry.path && (await isFree(target))) await fs.rename(entry.path, target);
      } else {
        const target = `${entry.path}/${name}`;
        if (!(await isFree(target))) return;
        if (entry.kind === 'new-folder') await fs.mkdir(target);
        else await fs.writeTextFile(target, '');
      }
    } catch (error) {
      actionError = describeError(error);
    }
  }

  function onEntryKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      event.preventDefault();
      void commitEntry();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      pending = null;
    }
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
    {#if actionError}
      <p class="border-b px-3 py-2 text-sm leading-snug text-[var(--color-bad)]">{actionError}</p>
    {/if}
    <ScrollArea class="min-h-0 flex-1" bind:viewportRef={viewport}>
      <div class="flex flex-col px-2 pb-2" aria-label="Project files">
        <div aria-hidden="true" style={`height: ${renderWindow.topSpacerHeight}px`}></div>
        {#each renderWindow.nodes as node (node.path)}
          {#if pending?.kind === 'rename' && pending.path === node.path}
            {@render nameEntry(node.depth, `Rename ${node.name}`)}
          {:else}
            <FileTreeRow
              {node}
              expanded={openFolders.has(node.path)}
              selected={!node.isDirectory && node.path === explorer.selectedPath}
              onclick={onRowClick}
              onaction={onRowAction}
            />
            {#if pending && pending.kind !== 'rename' && pending.path === node.path}
              {@render nameEntry(
                node.depth + 1,
                pending.kind === 'new-folder' ? 'New folder name' : 'New file name'
              )}
            {/if}
          {/if}
        {/each}
        <div aria-hidden="true" style={`height: ${renderWindow.bottomSpacerHeight}px`}></div>
      </div>
    </ScrollArea>
  {/if}
</div>

<!-- The name field stands where the row it belongs to would, so a new file
     appears to be typed straight into the tree. A row's own label sits 54px in
     from the row's edge once its padding, indent, chevron and icon are counted,
     and the field carries 9px of its own, so this leaves 45. -->
{#snippet nameEntry(depth: number, label: string)}
  <div class="flex h-7 shrink-0 items-center" style={`padding-left: ${depth * 12 + 45}px`}>
    <Input
      class="h-6 w-full"
      aria-label={label}
      placeholder={label}
      bind:ref={entryField}
      bind:value={pendingName}
      onkeydown={onEntryKeydown}
      onblur={() => void commitEntry()}
    />
  </div>
{/snippet}

