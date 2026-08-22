<script lang="ts">
  import { Tree, type LTreeNode } from '@keenmate/svelte-treeview';
  import ArrowDownAZ from '@lucide/svelte/icons/arrow-down-a-z';
  import ArrowUpZA from '@lucide/svelte/icons/arrow-up-z-a';
  import Eye from '@lucide/svelte/icons/eye';
  import EyeOff from '@lucide/svelte/icons/eye-off';
  import Folder from '@lucide/svelte/icons/folder';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import FolderTree from '@lucide/svelte/icons/folder-tree';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Square from '@lucide/svelte/icons/square';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import { onMount } from 'svelte';

  import { Button } from '$lib/components/ui/button/index.js';
  import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { PanelHeader } from '$lib/components/ui/panel-header/index.js';
  import FileIcon from '$lib/shell/components/explorer/FileIcon.svelte';
  import {
    loadDirectory,
    refresh,
    refreshChangedPath,
    scanRoot,
    stopScan,
    unloadDirectory
  } from '$lib/shell/explorer/explorerService';
  import {
    explorer,
    explorerNodes,
    selectPath,
    setIncludeExcluded,
    setQuery
  } from '$lib/shell/explorer/explorerStore.svelte';
  import type { ExplorerTreeNode } from '$lib/shell/explorer/explorerStore.svelte';
  import { projectRootLabel } from '$lib/shell/explorer/explorerTree';
  import { gitService } from '$lib/shell/git/gitService';
  import { openFileInEditor, showCenterTab } from '$lib/shell/workbenchNavigation';
  import { isNativeTauriRuntime, moveToTrashFromTauri } from '$lib/tauriSource';

  interface Props {
    visible: boolean;
    root: string;
    ownedId: string | null;
  }

  type PendingEntry = {
    kind: 'rename' | 'new-file' | 'new-folder';
    path: string;
  };

  type FileClipboard = {
    operation: 'cut' | 'copy';
    path: string;
    name: string;
    isDirectory: boolean;
  };

  type TreeItem = ExplorerTreeNode & {
    treePath: string;
    treeParentPath: string;
    relativePath: string;
    hasChildren: boolean;
    expanded: boolean;
    selected: boolean;
  };

  let { visible, root }: Props = $props();
  let expanded = $state.raw<Set<string>>(new Set());
  let treeData = $state.raw<TreeItem[]>([]);
  let sortDirection = $state<'ascending' | 'descending'>('ascending');
  let pending = $state<PendingEntry | null>(null);
  let pendingName = $state('');
  let entryField = $state<HTMLInputElement | null>(null);
  let actionError = $state<string | null>(null);
  let fileClipboard = $state.raw<FileClipboard | null>(null);
  let treeHost = $state<HTMLDivElement | null>(null);
  let treeHeight = $state(400);

  const loadedNodes = $derived(explorerNodes());
  const rootLabel = $derived(projectRootLabel(explorer.root ?? root));
  const listedCount = $derived(loadedNodes.length);
  const listed = $derived(explorer.lastScanFinishedAt !== null);

  function searchedNodes(): ExplorerTreeNode[] {
    const query = explorer.query.trim().toLowerCase();
    if (!query) return loadedNodes;

    const byPath = new Map(loadedNodes.map((node) => [node.path, node]));
    const shown = new Set<string>();
    for (const node of loadedNodes) {
      const path = relativePath((explorer.root ?? root).replace(/\/+$/, ''), node.path);
      if (!path.toLowerCase().includes(query)) continue;
      let current: ExplorerTreeNode | undefined = node;
      while (current) {
        shown.add(current.path);
        current = byPath.get(current.parentPath);
      }
    }
    return loadedNodes.filter((node) => shown.has(node.path));
  }

  $effect(() => {
    const projectRoot = (explorer.root ?? root).replace(/\/+$/, '');
    sortDirection;
    treeData = searchedNodes().map((node) => ({
      ...node,
      treePath: relativePath(projectRoot, node.path),
      treeParentPath: node.parentPath === projectRoot ? '' : relativePath(projectRoot, node.parentPath),
      relativePath: relativePath(projectRoot, node.path),
      hasChildren: node.isDirectory,
      expanded: expanded.has(node.path),
      selected: node.path === explorer.selectedPath
    }));
  });

  $effect(() => {
    root;
    expanded = new Set();
    pending = null;
    fileClipboard = null;
  });

  $effect(() => {
    const target = (explorer.root ?? '').trim();
    if (!visible || !listed || !target || !isNativeTauriRuntime()) return;

    let unwatch: (() => void) | null = null;
    let abandoned = false;
    void (async () => {
      try {
        const { watch } = await import('@tauri-apps/plugin-fs');
        const stop = await watch(
          target,
          (event) => {
            if (explorer.scanning || isBuildOutputOnly(event.paths)) return;
            for (const path of event.paths) refreshChangedPath(path);
          },
          { recursive: true, delayMs: 300 }
        );
        if (abandoned) stop();
        else unwatch = stop;
      } catch {
        // Refresh remains available when the watcher cannot be started.
      }
    })();

    return () => {
      abandoned = true;
      unwatch?.();
      unwatch = null;
    };
  });

  $effect(() => {
    const field = entryField;
    if (!field) return;
    field.focus();
    field.select();
  });

  onMount(() => {
    const updateTreeHeight = () => {
      if (treeHost) treeHeight = Math.max(1, Math.floor(treeHost.clientHeight));
    };
    const frame = requestAnimationFrame(updateTreeHeight);
    window.addEventListener('resize', updateTreeHeight);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener('resize', updateTreeHeight);
    };
  });

  const UNLISTED_FOLDERS = ['/node_modules/', '/target/', '/.git/', '/.svelte-kit/'];

  function relativePath(projectRoot: string, path: string): string {
    const prefix = `${projectRoot}/`;
    return path.startsWith(prefix) ? path.slice(prefix.length) : path.replace(/^\/+/, '');
  }

  function parentOf(path: string): string {
    const cut = path.lastIndexOf('/');
    return cut < 0 ? '' : path.slice(0, cut);
  }

  function describeError(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  function isBuildOutputOnly(paths: readonly string[]): boolean {
    return paths.length > 0 && paths.every((path) => UNLISTED_FOLDERS.some((part) => path.includes(part)));
  }

  function onFilterInput(event: Event & { currentTarget: HTMLInputElement }): void {
    setQuery(event.currentTarget.value);
  }

  function compareTreeNodes(left: LTreeNode<TreeItem>, right: LTreeNode<TreeItem>): number {
    const leftLevel = left.level ?? 0;
    const rightLevel = right.level ?? 0;
    if (leftLevel !== rightLevel) return leftLevel - rightLevel;
    const parentOrder = String(left.parentPath ?? '').localeCompare(String(right.parentPath ?? ''));
    if (parentOrder !== 0) return parentOrder;
    const leftDirectory = left.data?.isDirectory ?? false;
    const rightDirectory = right.data?.isDirectory ?? false;
    if (leftDirectory !== rightDirectory) return leftDirectory ? -1 : 1;
    const nameOrder = (left.data?.name ?? '').localeCompare(right.data?.name ?? '', undefined, {
      numeric: true,
      sensitivity: 'base'
    });
    return sortDirection === 'ascending' ? nameOrder : -nameOrder;
  }

  function sortTreeNodes(items: LTreeNode<TreeItem>[]): LTreeNode<TreeItem>[] {
    return items.sort(compareTreeNodes);
  }

  function collapsePath(path: string): void {
    unloadDirectory(path);
    expanded = new Set(
      [...expanded].filter((candidate) => candidate !== path && !candidate.startsWith(`${path}/`))
    );
  }

  function onTreeNodeClicked(treeNode: LTreeNode<TreeItem>): void {
    const node = treeNode.data;
    if (!node) return;
    if (node.isDirectory) {
      if (expanded.has(node.path)) collapsePath(node.path);
      else {
        expanded = new Set([...expanded, node.path]);
        void loadDirectory(node.path, node.depth + 1);
      }
      return;
    }

    selectPath(node.path);
    const projectRoot = (explorer.root ?? root).trim();
    openFileInEditor({ path: node.path, projectRoot: projectRoot || undefined });
  }

  function onTreeWrapperClick(event: MouseEvent): void {
    const target = event.target;
    if (!(target instanceof Element) || !target.closest('.ltree-toggle-icon')) return;
    event.preventDefault();
    event.stopPropagation();
    const row = target.closest<HTMLElement>('[data-tree-path]');
    const node = treeData.find((item) => item.treePath === row?.dataset.treePath);
    if (node) onTreeNodeClicked({ data: node } as LTreeNode<TreeItem>);
  }

  function beginEntry(node: TreeItem, kind: PendingEntry['kind']): void {
    actionError = null;
    if (kind !== 'rename' && !expanded.has(node.path)) {
      expanded = new Set([...expanded, node.path]);
      void loadDirectory(node.path, node.depth + 1);
    }
    pending = { kind, path: node.path };
    pendingName = kind === 'rename' ? node.name : '';
  }

  async function isFree(target: string): Promise<boolean> {
    const { exists } = await import('@tauri-apps/plugin-fs');
    if (!(await exists(target))) return true;
    actionError = `Something is already called ${target.slice(target.lastIndexOf('/') + 1)} here.`;
    return false;
  }

  async function commitEntry(): Promise<void> {
    const entry = pending;
    const name = pendingName.trim();
    pending = null;
    if (!entry || !name || name.includes('/')) return;

    try {
      const fs = await import('@tauri-apps/plugin-fs');
      if (entry.kind === 'rename') {
        const target = `${parentOf(entry.path)}/${name}`;
        if (target !== entry.path && (await isFree(target))) {
          await fs.rename(entry.path, target);
          collapsePath(entry.path);
          refreshChangedPath(target);
        }
      } else {
        const target = `${entry.path}/${name}`;
        if (!(await isFree(target))) return;
        if (entry.kind === 'new-folder') await fs.mkdir(target);
        else await fs.writeTextFile(target, '');
        refreshChangedPath(target);
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

  async function copyDirectory(source: string, target: string): Promise<void> {
    const fs = await import('@tauri-apps/plugin-fs');
    await fs.mkdir(target);
    for (const entry of await fs.readDir(source)) {
      if (entry.isSymlink) continue;
      const childSource = `${source}/${entry.name}`;
      const childTarget = `${target}/${entry.name}`;
      if (entry.isDirectory) await copyDirectory(childSource, childTarget);
      else if (entry.isFile) await fs.copyFile(childSource, childTarget);
    }
  }

  async function pasteInto(node: TreeItem): Promise<void> {
    const entry = fileClipboard;
    if (!entry || !node.isDirectory) return;
    const target = `${node.path}/${entry.name}`;
    if (!(await isFree(target))) return;

    const fs = await import('@tauri-apps/plugin-fs');
    if (entry.operation === 'cut') {
      await fs.rename(entry.path, target);
      collapsePath(entry.path);
      fileClipboard = null;
    } else if (entry.isDirectory) {
      await copyDirectory(entry.path, target);
    } else {
      await fs.copyFile(entry.path, target);
    }
    refreshChangedPath(target);
  }

  async function openFileHistory(node: TreeItem): Promise<void> {
    const projectRoot = (explorer.root ?? root).trim();
    if (!projectRoot) return;
    showCenterTab('git-history');
    await gitService.showFileHistory(projectRoot, node.relativePath);
  }

  async function toggleExcludedFiles(): Promise<void> {
    const include = !explorer.includeExcluded;
    if (!include) {
      for (const node of loadedNodes.filter((candidate) => candidate.ignored)) collapsePath(node.path);
    }
    setIncludeExcluded(include);
    refresh();
  }

  async function runAction(node: TreeItem, id: string): Promise<void> {
    actionError = null;
    try {
      if (id === 'new-file' || id === 'new-folder' || id === 'rename') {
        beginEntry(node, id);
      } else if (id === 'copy-path') {
        const { writeText } = await import('@tauri-apps/plugin-clipboard-manager');
        await writeText(node.path);
      } else if (id === 'reveal-in-finder') {
        const { revealItemInDir } = await import('@tauri-apps/plugin-opener');
        await revealItemInDir(node.path);
      } else if (id === 'delete') {
        await moveToTrashFromTauri(node.path);
        collapsePath(node.path);
        refreshChangedPath(node.path);
      } else if (id === 'cut' || id === 'copy') {
        fileClipboard = {
          operation: id,
          path: node.path,
          name: node.name,
          isDirectory: node.isDirectory
        };
      } else if (id === 'paste') {
        await pasteInto(node);
      } else if (id === 'open-timeline' || id === 'git-file-history') {
        await openFileHistory(node);
      } else if (id === 'toggle-excluded') {
        await toggleExcludedFiles();
      }
    } catch (error) {
      actionError = describeError(error);
    }
  }

  function contextMenuItems(node: TreeItem) {
    return [
      ...(node.isDirectory
        ? [
            { id: 'new-file', label: 'New File', onselect: () => void runAction(node, 'new-file') },
            { id: 'new-folder', label: 'New Folder', onselect: () => void runAction(node, 'new-folder') },
            {
              id: 'paste',
              label: 'Paste',
              disabled: !fileClipboard,
              onselect: () => void runAction(node, 'paste')
            }
          ]
        : []),
      {
        id: 'open-timeline',
        label: 'Open Timeline',
        separatorBefore: node.isDirectory,
        onselect: () => void runAction(node, 'open-timeline')
      },
      {
        id: 'git-file-history',
        label: 'Git: View File History',
        onselect: () => void runAction(node, 'git-file-history')
      },
      { id: 'cut', label: 'Cut', separatorBefore: true, onselect: () => void runAction(node, 'cut') },
      { id: 'copy', label: 'Copy', onselect: () => void runAction(node, 'copy') },
      { id: 'copy-path', label: 'Copy Path', onselect: () => void runAction(node, 'copy-path') },
      { id: 'rename', label: 'Rename', separatorBefore: true, onselect: () => void runAction(node, 'rename') },
      {
        id: 'delete',
        label: 'Delete',
        danger: true,
        onselect: () => void runAction(node, 'delete')
      },
      {
        id: 'reveal-in-finder',
        label: 'Reveal in Finder',
        separatorBefore: true,
        onselect: () => void runAction(node, 'reveal-in-finder')
      },
      {
        id: 'toggle-excluded',
        label: explorer.includeExcluded ? 'Hide Excluded Files' : 'Show Excluded Files',
        onselect: () => void runAction(node, 'toggle-excluded')
      }
    ];
  }
</script>

<div class="files-panel flex h-full min-h-0 w-full flex-col text-foreground">
  <PanelHeader title="Files" count={explorer.activated ? listedCount : null}>
    {#snippet actions()}
      {#if explorer.activated && explorer.unavailable === null}
        <Input
          type="search"
          class="h-7 w-32"
          placeholder="Search loaded"
          aria-label="Search loaded files"
          value={explorer.query}
          oninput={onFilterInput}
        />
        <IconButton
          label={sortDirection === 'ascending' ? 'Sort Z to A' : 'Sort A to Z'}
          onclick={() => (sortDirection = sortDirection === 'ascending' ? 'descending' : 'ascending')}
        >
          {#if sortDirection === 'ascending'}<ArrowDownAZ />{:else}<ArrowUpZA />{/if}
        </IconButton>
        <IconButton
          label={explorer.includeExcluded ? 'Hide excluded files' : 'Show excluded files'}
          onclick={() => void toggleExcludedFiles()}
        >
          {#if explorer.includeExcluded}<EyeOff />{:else}<Eye />{/if}
        </IconButton>
        {#if explorer.scanning}
          <IconButton label="Stop listing files" onclick={() => stopScan()}><Square /></IconButton>
        {:else}
          <IconButton
            label="List this project's files again"
            disabled={!explorer.root}
            onclick={() => refresh()}
          >
            <RefreshCw />
          </IconButton>
        {/if}
      {/if}
    {/snippet}
    {rootLabel}
  </PanelHeader>

  {#if !explorer.activated}
    <EmptyState title="No session selected" body="Pick a session and its files appear here.">
      {#snippet icon()}<FolderTree />{/snippet}
    </EmptyState>
  {:else if explorer.unavailable === 'checkout-deleted'}
    <EmptyState
      title="Checkout/Worktree deleted"
      body="The conversation is still available, but this session’s project files no longer exist."
    >
      {#snippet icon()}<TriangleAlert />{/snippet}
    </EmptyState>
  {:else if explorer.error && loadedNodes.length === 0}
    <EmptyState title="Could not list the files" body={explorer.error}>
      {#snippet icon()}<TriangleAlert />{/snippet}
      {#snippet actions()}
        <Button size="sm" variant="secondary" onclick={() => explorer.root && void scanRoot(explorer.root)}>
          Try again
        </Button>
      {/snippet}
    </EmptyState>
  {:else if explorer.scanning && loadedNodes.length === 0}
    <EmptyState title="Listing files…" body="Reading this project's top-level entries.">
      {#snippet icon()}<FolderTree />{/snippet}
    </EmptyState>
  {:else if loadedNodes.length === 0}
    <EmptyState title="No source files here" body="This folder has no files the app can open.">
      {#snippet icon()}<FolderTree />{/snippet}
    </EmptyState>
  {:else}
    {#if explorer.error}
      <p class="border-b px-3 py-2 text-sm leading-snug text-[var(--color-bad)]">{explorer.error}</p>
    {/if}
    {#if actionError}
      <p class="border-b px-3 py-2 text-sm leading-snug text-[var(--color-bad)]">{actionError}</p>
    {/if}
    {#if pending}
      <div class="entry-bar">
        <Input
          class="h-6 w-full"
          aria-label={pending.kind === 'rename' ? 'Rename path' : 'New path name'}
          placeholder={pending.kind === 'new-folder' ? 'New folder name' : 'New file name'}
          bind:ref={entryField}
          bind:value={pendingName}
          onkeydown={onEntryKeydown}
          onblur={() => void commitEntry()}
        />
      </div>
    {/if}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="tree-host"
      role="tree"
      tabindex="0"
      aria-label="Project files"
      bind:this={treeHost}
      onclickcapture={onTreeWrapperClick}
    >
      <Tree
        data={treeData}
        treeId="project-files"
        treePathSeparator="/"
        idMember="path"
        pathMember="treePath"
        parentPathMember="treeParentPath"
        hasChildrenMember="hasChildren"
        isExpandedMember="expanded"
        isSelectedMember="selected"
        displayValueMember="name"
        searchValueMember="relativePath"
        searchText={explorer.query}
        sortCallback={sortTreeNodes}
        shouldToggleOnNodeClick={false}
        shouldUseInternalSearchIndex={false}
        useFlatRendering={true}
        progressiveRender={false}
        virtualScroll={true}
        virtualRowHeight={28}
        virtualOverscan={6}
        virtualContainerHeight={`${treeHeight}px`}
        bodyClass="mcb-tree-body"
        dragDropMode="none"
        expandLevel={0}
        selectedNodeClass="mcb-tree-selected"
        expandIconClass="mcb-tree-expand"
        collapseIconClass="mcb-tree-collapse"
        leafIconClass="mcb-tree-leaf"
        onNodeClicked={onTreeNodeClicked}
      >
        {#snippet nodeTemplate(treeNode: LTreeNode<TreeItem>)}
          {@const node = treeNode.data}
          {#if node}
            <ContextMenu.Root>
              <ContextMenu.Trigger>
                {#snippet child({ props })}
                  <span {...props} class="tree-row" class:excluded={node.ignored} title={node.path}>
                    <span class="tree-file-icon" aria-hidden="true">
                      {#if node.isDirectory}
                        {#if expanded.has(node.path)}
                          <FolderOpen size={14} strokeWidth={1.75} />
                        {:else}
                          <Folder size={14} strokeWidth={1.75} />
                        {/if}
                      {:else}
                        <FileIcon fileName={node.name} size={14} />
                      {/if}
                    </span>
                    <span class="tree-name">{node.name}</span>
                    {#if fileClipboard?.path === node.path}
                      <span class="clipboard-mark">{fileClipboard.operation}</span>
                    {/if}
                  </span>
                {/snippet}
              </ContextMenu.Trigger>
              <ContextMenu.Content class="w-[220px]" aria-label={`Actions for ${node.name}`}>
                {#each contextMenuItems(node) as item (item.id)}
                  {#if item.separatorBefore}<ContextMenu.Separator />{/if}
                  <ContextMenu.Item
                    disabled={item.disabled}
                    variant={item.danger ? 'destructive' : 'default'}
                    onSelect={item.onselect}
                  >{item.label}</ContextMenu.Item>
                {/each}
              </ContextMenu.Content>
            </ContextMenu.Root>
          {/if}
        {/snippet}
        {#snippet noDataFound()}
          <p class="tree-empty">Nothing matches that search.</p>
        {/snippet}
      </Tree>
    </div>
  {/if}
</div>

<style>
  .tree-host{--tree-node-indent-per-level:12px;display:flex;height:0;min-height:0;flex:1;overflow:hidden;padding:0 4px 4px}
  .entry-bar{padding:4px 8px;border-bottom:1px solid var(--color-border)}
  .tree-row{display:flex;align-items:center;min-width:0;width:100%;gap:6px;color:var(--color-text)}
  .tree-row.excluded{opacity:.55}
  .tree-file-icon{display:flex;flex:0 0 14px;align-items:center;justify-content:center;color:var(--color-text-3)}
  .tree-name{min-width:0;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .clipboard-mark{flex:none;color:var(--color-text-3);font-size:12px;text-transform:uppercase}
  .tree-empty{padding:16px 10px;color:var(--color-text-3);text-align:center}

  :global(.files-panel .ltree-tree){position:relative;font:inherit;color:inherit;overscroll-behavior:contain;scrollbar-width:thin;scrollbar-color:var(--scrollbar-thumb) transparent}
  :global(.files-panel .ltree-container){width:100%;height:100%;min-width:0;min-height:0}
  :global(.files-panel .mcb-tree-body){height:100%;min-height:0}
  :global(.files-panel .ltree-tree.ltree-virtual-scroll){height:100%!important;overflow-x:hidden!important;overflow-y:auto!important}
  :global(.files-panel .ltree-node){position:relative;height:28px;font:inherit}
  :global(.files-panel .ltree-node-row){display:flex;height:28px;align-items:center;min-width:0}
  :global(.files-panel .ltree-toggle-icon){display:flex;width:16px;height:28px;flex:0 0 16px;align-items:center;justify-content:center;color:var(--color-text-3);font-size:13px;cursor:pointer}
  :global(.files-panel .mcb-tree-expand::before){content:'›'}
  :global(.files-panel .mcb-tree-collapse::before){content:'⌄'}
  :global(.files-panel .mcb-tree-leaf::before){content:''}
  :global(.files-panel .ltree-node-content){display:flex;height:28px;min-width:0;flex:1;align-items:center;padding:0 6px;border-radius:4px;user-select:none;cursor:pointer}
  :global(.files-panel .ltree-node-content:hover){background:var(--color-surface-hover)}
  :global(.files-panel .ltree-node-content.mcb-tree-selected){background:color-mix(in srgb,var(--color-accent) 18%,transparent)}
</style>
