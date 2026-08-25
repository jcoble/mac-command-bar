<script lang="ts">
  import { onDestroy, tick } from 'svelte';
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
  import { Button } from '$lib/components/ui/button/index.js';
  import { EmptyState } from '$lib/components/ui/empty-state/index.js';
  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { PanelHeader } from '$lib/components/ui/panel-header/index.js';
  import * as Select from '$lib/components/ui/select/index.js';
  import FileIcon from '$lib/shell/components/explorer/FileIcon.svelte';
  import {
    activate as activateExplorer,
    loadDirectory,
    refresh,
    refreshChangedPath,
    revealExplorerPath,
    scanRoot,
    stopScan,
    unloadDirectory
  } from '$lib/shell/explorer/explorerService';
  import {
    canonicalPath,
    explorer,
    explorerNodes,
    loadedExplorerDirectoryDepth,
    loadedExplorerDirectories,
    selectPath,
    setIncludeExcluded
  } from '$lib/shell/explorer/explorerStore.svelte';
  import type { ExplorerTreeNode } from '$lib/shell/explorer/explorerStore.svelte';
  import { projectRootLabel } from '$lib/shell/explorer/explorerTree';
  import { openFileInEditor, openFileTimeline } from '$lib/shell/workbenchNavigation';
  import {
    cancelTrackedAnimationFrame,
    requestTrackedAnimationFrame,
    trackFileWatcher
  } from '$lib/shell/resourceDiagnostics.svelte';
  import {
    cancelSourceScanFromTauri,
    createSourceScanId,
    isNativeTauriRuntime,
    listRepositoryCheckoutsFromTauri,
    moveToTrashFromTauri,
    searchSourceTreeFromTauri,
    validateProjectRootFromTauri
  } from '$lib/tauriSource';
  import type { RepositoryCheckout } from '$lib/tauriSource';
  import type { SourceTreeSearchMatch } from '$lib/sourceData';

  interface Props {
    visible: boolean;
    root: string;
    ownedId: string | null;
    onRootUnavailable?(root: string): void | Promise<void>;
    expandedPathsByRoot?: Readonly<Record<string, readonly string[]>>;
    onExpandedPathsChange?(root: string, paths: readonly string[]): void;
    inspectionRoot?: string | null;
    onInspectionRootChange?(root: string | null): void;
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

  type FilesContextMenuState = {
    node: TreeItem;
    left: number;
    top: number;
  };

  type TreeItem = ExplorerTreeNode & {
    treePath: string;
    treeParentPath: string;
    relativePath: string;
    hasChildren: boolean;
    expanded: boolean;
    selected: boolean;
    searchResult?: boolean;
  };

  let {
    visible,
    root,
    ownedId,
    onRootUnavailable,
    expandedPathsByRoot,
    onExpandedPathsChange,
    inspectionRoot,
    onInspectionRootChange
  }: Props = $props();
  let inspectedRoot = $state('');
  let checkouts = $state<RepositoryCheckout[]>([]);
  let expanded = $state.raw<Set<string>>(new Set());
  let treeData = $state.raw<TreeItem[]>([]);
  let sortDirection = $state<'ascending' | 'descending'>('ascending');
  let pending = $state<PendingEntry | null>(null);
  let pendingName = $state('');
  let entryField = $state<HTMLInputElement | null>(null);
  let actionError = $state<string | null>(null);
  let fileClipboard = $state.raw<FileClipboard | null>(null);
  let contextMenu = $state.raw<FilesContextMenuState | null>(null);
  let treeHost = $state<HTMLDivElement | null>(null);
  let treeHeight = $state(400);
  let searchText = $state('');
  let searchMatches = $state.raw<SourceTreeSearchMatch[]>([]);
  let searchNextCursor = $state<number | null>(null);
  let searchLoading = $state(false);
  let searchError = $state<string | null>(null);
  let searchGeneration = 0;
  let revealGeneration = 0;
  let activeSearchScanId: string | null = null;
  let notifiedUnavailableRoot = '';
  let hydratedRoot = '';
  let hydratedExpansionKey = '';
  let expansionRestoreGeneration = 0;
  let scopedSessionKey = '';
  let checkoutGeneration = 0;
  let checkoutsLoaded = $state(false);
  let inspectionGeneration = 0;

  const READ_ONLY_SCOPE_MESSAGE =
    'This folder is open for reading only — switch back to the session folder to change it.';
  const READ_ONLY_RUNTIME_MESSAGE = 'File changes are available in the desktop app only.';
  const MUTATING_ACTIONS = new Set([
    'new-file',
    'new-folder',
    'cut',
    'paste',
    'rename',
    'delete'
  ]);

  function canMutateFileTree(): boolean {
    const sessionRoot = canonicalPath(root);
    const listedRoot = canonicalPath(explorer.root ?? root);
    return (
      isNativeTauriRuntime() &&
      Boolean(ownedId) &&
      sessionRoot !== '' &&
      listedRoot === sessionRoot
    );
  }

  function fileTreeMutationReason(): string {
    return isNativeTauriRuntime() ? READ_ONLY_SCOPE_MESSAGE : READ_ONLY_RUNTIME_MESSAGE;
  }

  const loadedNodes = $derived(explorerNodes());
  const sessionRoot = $derived(root.trim());
  const projectRoot = $derived((inspectedRoot || explorer.root || sessionRoot).trim());
  const readOnlyInspection = $derived(
    Boolean(inspectedRoot) && canonicalPath(inspectedRoot) !== canonicalPath(sessionRoot)
  );
  const scopeOptions = $derived(
    sessionRoot === ''
      ? []
      : [
          { path: sessionRoot, label: `${folderName(sessionRoot)} (session)` },
          ...checkouts
            .filter((checkout) => canonicalPath(checkout.path) !== canonicalPath(sessionRoot))
            .map((checkout) => ({
              path: checkout.path,
              label: checkout.branch.trim()
                ? `${folderName(checkout.path)} — ${checkout.branch.trim()}`
                : folderName(checkout.path)
            }))
        ]
  );
  const scopeValue = $derived(projectRoot || sessionRoot);
  const rootLabel = $derived(projectRootLabel(projectRoot || sessionRoot));
  const listedCount = $derived(loadedNodes.length);
  const listed = $derived(explorer.lastScanFinishedAt !== null);
  const searching = $derived(searchText.trim().length > 0);
  const treeVisible = $derived(explorer.activated && explorer.unavailable === null && loadedNodes.length > 0);
  const searchTreeData = $derived(
    searchMatches.map<TreeItem>((match) => ({
      path: match.path,
      name: match.name,
      isDirectory: match.isDirectory,
      excluded: match.excluded,
      parentPath: projectRoot,
      depth: 0,
      childCount: 0,
      ignored: match.excluded,
      treePath: match.relativePath,
      treeParentPath: '',
      relativePath: match.relativePath,
      hasChildren: false,
      expanded: false,
      selected: match.path === explorer.selectedPath,
      searchResult: true
    }))
  );
  const displayedTreeData = $derived(searching ? searchTreeData : treeData);

  $effect(() => {
    const treeRoot = projectRoot.replace(/\/+$/, '');
    sortDirection;
    treeData = loadedNodes.map((node) => ({
      ...node,
      treePath: relativePath(treeRoot, node.path),
      treeParentPath: node.parentPath === treeRoot ? '' : relativePath(treeRoot, node.parentPath),
      relativePath: relativePath(treeRoot, node.path),
      hasChildren: node.isDirectory,
      expanded: expanded.has(node.path),
      selected: node.path === explorer.selectedPath
    }));
  });

  $effect(() => {
    const sessionKey = `${ownedId ?? ''}:${canonicalPath(root)}`;
    if (sessionKey === scopedSessionKey) return;
    scopedSessionKey = sessionKey;
    inspectionGeneration += 1;
    checkoutGeneration += 1;
    checkoutsLoaded = false;
    inspectedRoot = '';
    checkouts = [];
    cancelActiveSearch();
    revealGeneration += 1;
    hydratedRoot = '';
    hydratedExpansionKey = '';
    expansionRestoreGeneration += 1;
    expanded = new Set();
    pending = null;
    fileClipboard = null;
    contextMenu = null;
    searchText = '';
    const nextRoot = canonicalPath(sessionRoot);
    if (nextRoot && canonicalPath(explorer.root ?? '') === nextRoot) {
      for (const directory of loadedExplorerDirectories()) {
        if (directory.path !== nextRoot) unloadDirectory(directory.path);
      }
    }
    if (visible) activateExplorer(sessionRoot || null);
    else activateExplorer(null);
    void loadCheckouts(sessionRoot, checkoutGeneration);
  });

  $effect(() => {
    if (!visible || !sessionRoot) return;
    activateExplorer(sessionRoot);
  });

  $effect(() => {
    const requestedRoot = canonicalPath(inspectionRoot ?? '');
    const sessionRootPath = canonicalPath(sessionRoot);
    const target = requestedRoot && requestedRoot !== sessionRootPath ? requestedRoot : '';
    const generation = ++inspectionGeneration;
    if (!visible || !sessionRootPath) return;
    if (requestedRoot === canonicalPath(inspectedRoot) && (!inspectionRoot || target !== '')) return;
    if (!target) {
      selectInspectionRoot('');
      return;
    }
    if (!checkoutsLoaded) return;
    if (!scopeOptions.some((option) => canonicalPath(option.path) === target)) {
      selectInspectionRoot('');
      return;
    }
    void (async () => {
      const validation = await validateProjectRootFromTauri(target);
      if (
        generation !== inspectionGeneration
        || canonicalPath(root) !== sessionRootPath
        || canonicalPath(inspectionRoot ?? '') !== target
      ) return;
      if (validation === null || (validation.exists && validation.isDirectory)) {
        selectInspectionRoot(target);
      } else {
        selectInspectionRoot('');
      }
    })();
  });

  $effect(() => {
    const requestedRoot = canonicalPath(projectRoot);
    const activeRoot = canonicalPath(explorer.root ?? '');
    const savedPaths = activeRoot
      ? canonicalExpandedPaths(activeRoot, expandedPathsByRoot?.[activeRoot] ?? [])
      : [];
    const savedKey = expansionKey(savedPaths);
    if (!listed || !explorer.activated || !requestedRoot || requestedRoot !== activeRoot) {
      if (activeRoot !== hydratedRoot) {
        hydratedRoot = activeRoot;
        hydratedExpansionKey = '';
        expansionRestoreGeneration += 1;
        expanded = new Set();
      }
      return;
    }
    if (activeRoot === hydratedRoot && savedKey === hydratedExpansionKey) return;

    hydratedRoot = activeRoot;
    hydratedExpansionKey = savedKey;
    expansionRestoreGeneration += 1;
    expanded = new Set(savedPaths);
    const generation = expansionRestoreGeneration;
    void restoreExpandedDirectories(activeRoot, savedPaths, generation);
  });

  $effect(() => {
    const target = projectRoot;
    if (explorer.unavailable !== 'checkout-deleted' || !target || readOnlyInspection) {
      if (explorer.unavailable === null) notifiedUnavailableRoot = '';
      return;
    }
    if (canonicalPath(target) === canonicalPath(notifiedUnavailableRoot)) return;
    notifiedUnavailableRoot = target;
    void onRootUnavailable?.(target);
  });

  $effect(() => {
    const query = searchText.trim();
    const projectRoot = projectRootForView();
    explorer.includeExcluded;
    const generation = ++searchGeneration;
    cancelActiveSearch();
    searchMatches = [];
    searchNextCursor = null;
    searchError = null;
    searchLoading = false;
    if (!query || !visible || !listed || !projectRoot) return;

    const timer = window.setTimeout(() => {
      void loadSearchPage(generation, null, true);
    }, 220);
    return () => {
      window.clearTimeout(timer);
      cancelActiveSearch();
    };
  });

  $effect(() => {
    const target = projectRoot;
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
            if (event.paths.some((path) => canonicalPath(path) === canonicalPath(target))) {
              refresh();
              return;
            }
            for (const path of event.paths) refreshChangedPath(path);
          },
          { recursive: true, delayMs: 300 }
        );
        const releaseWatcher = trackFileWatcher(stop);
        if (abandoned) releaseWatcher();
        else unwatch = releaseWatcher;
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
    if (visible) return;
    contextMenu = null;
  });

  $effect(() => {
    if (!contextMenu) return;

    const closeFromClick = (event: MouseEvent) => {
      const target = event.target;
      if (!(target instanceof Element) || !target.closest('[data-files-context-menu]')) {
        contextMenu = null;
      }
    };
    const closeFromKeyboard = (event: KeyboardEvent) => {
      if (event.key === 'Escape') contextMenu = null;
    };
    window.addEventListener('click', closeFromClick, true);
    window.addEventListener('keydown', closeFromKeyboard);
    return () => {
      window.removeEventListener('click', closeFromClick, true);
      window.removeEventListener('keydown', closeFromKeyboard);
    };
  });

  $effect(() => {
    const field = entryField;
    if (!field) return;
    field.focus();
    field.select();
  });

  $effect(() => {
    const host = treeHost;
    if (!host) return;

    let frame: number | null = null;
    const updateTreeHeight = () => {
      frame = null;
      const nextHeight = Math.max(1, Math.floor(host.clientHeight));
      if (treeHeight !== nextHeight) treeHeight = nextHeight;
    };
    updateTreeHeight();
    const observer = new ResizeObserver(() => {
      if (frame === null) frame = requestTrackedAnimationFrame(updateTreeHeight);
    });
    observer.observe(host);
    return () => {
      observer.disconnect();
      if (frame !== null) cancelTrackedAnimationFrame(frame);
    };
  });

  $effect(() => {
    displayedTreeData.length;
    const host = treeHost;
    if (!host) return;
    let active = true;
    void tick().then(() => {
      if (!active) return;
      const viewport = host.querySelector<HTMLElement>('.ltree-virtual-scroll');
      if (!viewport) return;
      const maxScrollTop = Math.max(0, viewport.scrollHeight - viewport.clientHeight);
      if (viewport.scrollTop > maxScrollTop) viewport.scrollTop = maxScrollTop;
    });
    return () => {
      active = false;
    };
  });

  const UNLISTED_FOLDERS = ['/node_modules/', '/target/', '/.git/', '/.svelte-kit/'];

  function relativePath(projectRoot: string, path: string): string {
    const prefix = `${projectRoot}/`;
    return path.startsWith(prefix) ? path.slice(prefix.length) : path.replace(/^\/+/, '');
  }

  function folderName(path: string): string {
    const parts = path.split('/').filter(Boolean);
    return parts.at(-1) ?? path;
  }

  function projectRootForView(): string {
    return canonicalPath(projectRoot);
  }

  function selectInspectionRoot(value: string): void {
    const target = canonicalPath(value);
    if (!target || target === canonicalPath(sessionRoot)) {
      if (inspectedRoot === '' && !inspectionRoot) return;
      inspectedRoot = '';
      inspectionGeneration += 1;
      revealGeneration += 1;
      expanded = new Set();
      pending = null;
      fileClipboard = null;
      actionError = null;
      searchText = '';
      cancelActiveSearch();
      activateExplorer(sessionRoot || null);
      onInspectionRootChange?.(null);
      return;
    }
    if (!scopeOptions.some((option) => canonicalPath(option.path) === target)) return;
    if (canonicalPath(inspectedRoot) === target) return;
    inspectedRoot = target;
    inspectionGeneration += 1;
    revealGeneration += 1;
    expanded = new Set();
    pending = null;
    fileClipboard = null;
    actionError = null;
    searchText = '';
    cancelActiveSearch();
    activateExplorer(target);
    onInspectionRootChange?.(target);
  }

  async function loadCheckouts(sessionRootValue: string, generation: number): Promise<void> {
    if (!sessionRootValue) return;
    try {
      const result = await listRepositoryCheckoutsFromTauri([sessionRootValue]);
      if (generation !== checkoutGeneration || canonicalPath(root) !== canonicalPath(sessionRootValue)) return;
      checkouts = result?.[sessionRootValue] ?? [];
      checkoutsLoaded = true;
    } catch {
      if (generation === checkoutGeneration) {
        checkouts = [];
        checkoutsLoaded = true;
      }
    }
  }

  function parentOf(path: string): string {
    const cut = path.lastIndexOf('/');
    return cut < 0 ? '' : path.slice(0, cut);
  }

  function describeError(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  function canonicalExpandedPaths(projectRoot: string, paths: readonly string[]): string[] {
    return [...new Set(
      paths
        .map((path) => canonicalPath(path))
        .filter((path) => path && path !== projectRoot && isPathAtOrBelow(path, projectRoot))
    )].sort((left, right) => {
      const depth = pathDepth(left) - pathDepth(right);
      return depth !== 0 ? depth : left.localeCompare(right);
    });
  }

  function pathDepth(path: string): number {
    return path.split('/').filter(Boolean).length;
  }

  function isPathAtOrBelow(path: string, projectRoot: string): boolean {
    if (path === projectRoot) return true;
    if (projectRoot === '/') return path.startsWith('/');
    return path.startsWith(`${projectRoot}/`);
  }

  function expansionKey(paths: readonly string[]): string {
    return paths.join('\u0000');
  }

  function persistExpandedPaths(): void {
    const projectRoot = canonicalPath(explorer.root ?? '');
    if (!projectRoot || projectRoot !== hydratedRoot || !listed) return;
    onExpandedPathsChange?.(projectRoot, canonicalExpandedPaths(projectRoot, [...expanded]));
  }

  async function restoreExpandedDirectories(
    projectRoot: string,
    paths: readonly string[],
    generation: number
  ): Promise<void> {
    const directories = new Set<string>();
    for (const path of paths) {
      const relative = path.slice(projectRoot.length).replace(/^\/+/, '');
      let directory = projectRoot;
      for (const component of relative.split('/').filter(Boolean)) {
        directory = `${directory}/${component}`;
        directories.add(directory);
      }
    }
    const ordered = [...directories].sort((left, right) => {
      const depth = pathDepth(left) - pathDepth(right);
      return depth !== 0 ? depth : left.localeCompare(right);
    });
    for (const directory of ordered) {
      if (
        generation !== expansionRestoreGeneration ||
        canonicalPath(explorer.root ?? '') !== projectRoot
      ) return;
      if (loadedExplorerDirectoryDepth(directory) !== null) continue;
      await loadDirectory(directory, pathDepth(directory) - pathDepth(projectRoot));
    }
  }

  function cancelActiveSearch(): void {
    const scanId = activeSearchScanId;
    activeSearchScanId = null;
    if (scanId) void cancelSourceScanFromTauri(scanId);
  }

  function isBuildOutputOnly(paths: readonly string[]): boolean {
    return paths.length > 0 && paths.every((path) => UNLISTED_FOLDERS.some((part) => path.includes(part)));
  }

  function onFilterInput(event: Event & { currentTarget: HTMLInputElement }): void {
    searchText = event.currentTarget.value;
  }

  async function loadSearchPage(
    generation: number,
    cursor: number | null,
    replace: boolean
  ): Promise<void> {
    const projectRoot = projectRootForView();
    const query = searchText.trim();
    if (!projectRoot || !query || generation !== searchGeneration) return;

    cancelActiveSearch();
    const scanId = createSourceScanId();
    activeSearchScanId = scanId;
    searchLoading = true;
    searchError = null;
    try {
      const page = await searchSourceTreeFromTauri(
        projectRoot,
        query,
        50,
        cursor,
        explorer.includeExcluded,
        scanId
      );
      if (
        generation !== searchGeneration ||
        searchText.trim() !== query ||
        projectRootForView() !== projectRoot
      ) return;
      if (!page) {
        searchError = 'The file search is not available here.';
        return;
      }
      searchMatches = replace ? page.matches : [...searchMatches, ...page.matches];
      searchNextCursor = page.nextCursor;
    } catch (error) {
      if (generation === searchGeneration) searchError = describeError(error);
    } finally {
      if (generation === searchGeneration) searchLoading = false;
      if (activeSearchScanId === scanId) activeSearchScanId = null;
    }
  }

  function loadMoreSearchResults(): void {
    if (searchNextCursor === null || searchLoading) return;
    void loadSearchPage(searchGeneration, searchNextCursor, false);
  }

  async function onSearchResultClicked(node: TreeItem): Promise<void> {
    const projectRoot = projectRootForView();
    if (!projectRoot) return;
    const generation = ++revealGeneration;
    searchText = '';
    const ancestors = await revealExplorerPath(node.path);
    if (generation !== revealGeneration || projectRootForView() !== projectRoot) return;
    expanded = new Set([...expanded, ...ancestors]);
    persistExpandedPaths();
    selectPath(node.path);
    if (!node.isDirectory) {
      openFileInEditor({
        path: node.path,
        projectRoot,
        readOnly: readOnlyInspection,
        preview: true
      });
    }
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
    return [...items].sort(compareTreeNodes);
  }

  function collapsePath(path: string): void {
    unloadDirectory(path);
    expanded = new Set(
      [...expanded].filter((candidate) => candidate !== path && !candidate.startsWith(`${path}/`))
    );
    persistExpandedPaths();
  }

  function onTreeNodeClicked(treeNode: LTreeNode<TreeItem>): void {
    const node = treeNode.data;
    if (!node) return;
    if (node.searchResult) {
      void onSearchResultClicked(node);
      return;
    }
    if (node.isDirectory) {
      if (expanded.has(node.path)) collapsePath(node.path);
      else {
        expanded = new Set([...expanded, node.path]);
        persistExpandedPaths();
        void loadDirectory(node.path, node.depth + 1);
      }
      return;
    }

    selectPath(node.path);
    const projectRoot = projectRootForView();
    openFileInEditor({
      path: node.path,
      projectRoot: projectRoot || undefined,
      readOnly: readOnlyInspection,
      preview: true
    });
  }

  function pinTreeNodeOpen(node: TreeItem, event: MouseEvent): void {
    event.stopPropagation();
    if (node.isDirectory) return;
    selectPath(node.path);
    const projectRoot = projectRootForView();
    openFileInEditor({
      path: node.path,
      projectRoot: projectRoot || undefined,
      readOnly: readOnlyInspection,
      pin: true
    });
  }

  function onTreeWrapperClick(event: MouseEvent): void {
    const target = event.target;
    if (!(target instanceof Element) || !target.closest('.ltree-toggle-icon')) return;
    event.preventDefault();
    event.stopPropagation();
    const row = target.closest<HTMLElement>('[data-tree-path]');
    const node = displayedTreeData.find((item) => item.treePath === row?.dataset.treePath);
    if (node) onTreeNodeClicked({ data: node } as LTreeNode<TreeItem>);
  }

  function beginEntry(node: TreeItem, kind: PendingEntry['kind']): void {
    if (readOnlyInspection) return;
    actionError = null;
    if (kind !== 'rename' && !expanded.has(node.path)) {
      expanded = new Set([...expanded, node.path]);
      persistExpandedPaths();
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
    if (readOnlyInspection) return;
    const generation = inspectionGeneration;
    const entry = pending;
    const name = pendingName.trim();
    pending = null;
    if (!entry || !name || name.includes('/')) return;
    if (!canMutateFileTree()) {
      actionError = fileTreeMutationReason();
      return;
    }

    try {
      const fs = await import('@tauri-apps/plugin-fs');
      if (generation !== inspectionGeneration || readOnlyInspection) return;
      if (entry.kind === 'rename') {
        const target = `${parentOf(entry.path)}/${name}`;
        if (target !== entry.path && (await isFree(target))) {
          if (generation !== inspectionGeneration || readOnlyInspection) return;
          await fs.rename(entry.path, target);
          collapsePath(entry.path);
          refreshChangedPath(target);
        }
      } else {
        const target = `${entry.path}/${name}`;
        if (!(await isFree(target))) return;
        if (generation !== inspectionGeneration || readOnlyInspection) return;
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
    if (readOnlyInspection) return;
    const generation = inspectionGeneration;
    const entry = fileClipboard;
    if (!entry || !node.isDirectory) return;
    const target = `${node.path}/${entry.name}`;
    if (!(await isFree(target))) return;
    if (generation !== inspectionGeneration || readOnlyInspection) return;

    const fs = await import('@tauri-apps/plugin-fs');
    if (generation !== inspectionGeneration || readOnlyInspection) return;
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
    if (MUTATING_ACTIONS.has(id) && !canMutateFileTree()) return;
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
        const projectRoot = projectRootForView();
        if (projectRoot) await openFileTimeline({ projectRoot, relativePath: node.relativePath });
      } else if (id === 'toggle-excluded') {
        await toggleExcludedFiles();
      }
    } catch (error) {
      actionError = describeError(error);
    }
  }

  function contextMenuItems(node: TreeItem) {
    const mutationDisabled = !canMutateFileTree();
    const mutationReason = mutationDisabled ? fileTreeMutationReason() : undefined;
    return [
      ...(node.isDirectory
        ? [
            {
              id: 'new-file',
              label: 'New File',
              disabled: mutationDisabled,
              title: mutationReason,
              onselect: () => void runAction(node, 'new-file')
            },
            {
              id: 'new-folder',
              label: 'New Folder',
              disabled: mutationDisabled,
              title: mutationReason,
              onselect: () => void runAction(node, 'new-folder')
            },
            {
              id: 'paste',
              label: 'Paste',
              disabled: mutationDisabled || !fileClipboard,
              title: mutationReason ?? (fileClipboard ? undefined : 'Copy or cut a file first.'),
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
      {
        id: 'cut',
        label: 'Cut',
        separatorBefore: true,
        disabled: mutationDisabled,
        title: mutationReason,
        onselect: () => void runAction(node, 'cut')
      },
      {
        id: 'copy',
        label: 'Copy',
        onselect: () => void runAction(node, 'copy')
      },
      { id: 'copy-path', label: 'Copy Path', onselect: () => void runAction(node, 'copy-path') },
      {
        id: 'rename',
        label: 'Rename',
        separatorBefore: true,
        disabled: mutationDisabled,
        title: mutationReason,
        onselect: () => void runAction(node, 'rename')
      },
      {
        id: 'delete',
        label: 'Delete',
        danger: true,
        disabled: mutationDisabled,
        title: mutationReason,
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

  function openContextMenu(node: TreeItem, event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();
    const items = contextMenuItems(node);
    const separatorCount = items.filter((item) => item.separatorBefore).length;
    const width = 220;
    const height = items.length * 32 + separatorCount * 7 + 8;
    contextMenu = {
      node,
      left: Math.max(8, Math.min(event.clientX, window.innerWidth - width - 8)),
      top: Math.max(8, Math.min(event.clientY, window.innerHeight - height - 8))
    };
  }

  function closeContextMenu(): void {
    contextMenu = null;
  }

  onDestroy(() => {
    inspectionGeneration += 1;
    checkoutGeneration += 1;
    searchGeneration += 1;
    revealGeneration += 1;
    cancelActiveSearch();
    contextMenu = null;
    activateExplorer(null);
  });
</script>

<div class="files-panel flex h-full min-h-0 w-full flex-col text-foreground">
  <PanelHeader title="Files" count={explorer.activated ? listedCount : null}>
    {#snippet actions()}
      {#if explorer.activated && explorer.unavailable === null}
        {#if scopeOptions.length > 1}
          <Select.Root type="single" value={scopeValue} onValueChange={selectInspectionRoot}>
            <Select.Trigger size="sm" class="w-44 min-w-0" aria-label="Folder this panel reads">
              <span class="min-w-0 truncate">
                {scopeOptions.find((option) => canonicalPath(option.path) === canonicalPath(scopeValue))?.label ??
                  'Session folder'}
              </span>
            </Select.Trigger>
            <Select.Content>
              {#each scopeOptions as option (option.path)}
                <Select.Item value={option.path} label={option.label} />
              {/each}
            </Select.Content>
          </Select.Root>
        {/if}
        <Input
          type="search"
          class="h-7 w-32"
          placeholder="Search files"
          aria-label="Search project files"
          value={searchText}
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
    {rootLabel}{#if readOnlyInspection} · read-only{/if}
  </PanelHeader>

  {#if !explorer.activated}
    <EmptyState title="No session selected" body="Pick a session and its files appear here.">
      {#snippet icon()}<FolderTree />{/snippet}
    </EmptyState>
  {:else if explorer.unavailable === 'checkout-deleted' && !readOnlyInspection}
    <EmptyState
      title="Checkout/Worktree deleted."
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
    {#if searchError}
      <p class="border-b px-3 py-2 text-sm leading-snug text-[var(--color-bad)]">{searchError}</p>
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
  {/if}

  <!-- Keep one renderer instance across root changes; only its bounded data projection changes. -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="tree-host"
    class:hidden={!treeVisible}
    role="tree"
    tabindex={treeVisible ? 0 : -1}
    aria-hidden={!treeVisible}
    aria-label="Project files"
    bind:this={treeHost}
    onclickcapture={onTreeWrapperClick}
  >
    <Tree
      data={displayedTreeData}
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
      searchText=""
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
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span
            class="tree-row"
            class:excluded={node.ignored}
            title={node.path}
            ondblclick={(event) => pinTreeNodeOpen(node, event)}
            oncontextmenu={(event) => openContextMenu(node, event)}
          >
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
        {/if}
      {/snippet}
      {#snippet noDataFound()}
        <p class="tree-empty">Nothing matches that search.</p>
      {/snippet}
    </Tree>
  </div>
  {#if treeVisible && searching && searchNextCursor !== null}
    <div class="search-more">
      <Button size="sm" variant="ghost" disabled={searchLoading} onclick={loadMoreSearchResults}>
        {searchLoading ? 'Loading…' : 'Load more'}
      </Button>
    </div>
  {/if}

  {#if contextMenu}
    <div
      class="files-context-menu"
      data-files-context-menu
      role="group"
      aria-label={`Actions for ${contextMenu.node.name}`}
      style={`left:${contextMenu.left}px;top:${contextMenu.top}px`}
      oncontextmenu={(event) => event.preventDefault()}
    >
      {#each contextMenuItems(contextMenu.node) as item (item.id)}
        {#if item.separatorBefore}<div class="files-context-separator" role="separator"></div>{/if}
        <button
          class="files-context-item"
          class:danger={item.danger}
          type="button"
          disabled={item.disabled}
          title={item.title}
          onclick={() => {
            closeContextMenu();
            item.onselect();
          }}
        >{item.label}</button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .tree-host{--tree-node-indent-per-level:12px;display:flex;height:0;min-height:0;flex:1;overflow:hidden;padding:0 4px 4px}
  .tree-host.hidden{display:none}
  .search-more{display:flex;justify-content:center;padding:2px 8px 6px;border-top:1px solid var(--color-border)}
  .entry-bar{padding:4px 8px;border-bottom:1px solid var(--color-border)}
  .tree-row{display:flex;align-items:center;min-width:0;width:100%;gap:6px;color:var(--color-text)}
  .tree-row.excluded{opacity:.55}
  .tree-file-icon{display:flex;flex:0 0 14px;align-items:center;justify-content:center;color:var(--color-text-3)}
  .tree-name{min-width:0;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .clipboard-mark{flex:none;color:var(--color-text-3);font-size:12px;text-transform:uppercase}
  .tree-empty{padding:16px 10px;color:var(--color-text-3);text-align:center}
  .files-context-menu{position:fixed;z-index:1000;width:220px;padding:4px;border:1px solid var(--color-border);border-radius:6px;background:var(--color-surface-2);box-shadow:0 12px 32px rgb(0 0 0/.35)}
  .files-context-item{display:flex;width:100%;height:32px;align-items:center;border:0;border-radius:4px;background:transparent;padding:0 9px;color:var(--color-text);font:inherit;text-align:left;cursor:default}
  .files-context-item:hover:not(:disabled){background:var(--color-surface-hover)}
  .files-context-item.danger{color:var(--color-bad)}
  .files-context-item:disabled{opacity:.45}
  .files-context-separator{height:1px;margin:3px 4px;background:var(--color-border)}

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
