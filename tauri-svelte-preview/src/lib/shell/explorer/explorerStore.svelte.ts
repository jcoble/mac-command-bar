/**
 * State for the lazy /next file explorer.
 *
 * The root listing contains only top-level entries. Expanding a directory adds
 * its immediate children; collapsing it removes those descendants again. The
 * list is raw because it is replaced as a unit and no row needs a Svelte proxy.
 */
import type { SourceDirectoryEntry } from '../../sourceData.ts';

export type ExplorerTreeNode = SourceDirectoryEntry & {
  parentPath: string;
  depth: number;
  childCount: number;
  ignored: boolean;
};

export const explorer = $state<{
  root: string | null;
  activated: boolean;
  query: string;
  scrollTop: number;
  viewportHeight: number;
  selectedPath: string | null;
  includeExcluded: boolean;
  scanning: boolean;
  error: string | null;
  unavailable: 'checkout-deleted' | null;
  lastScanFinishedAt: number | null;
}>({
  root: null,
  activated: false,
  query: '',
  scrollTop: 0,
  viewportHeight: 0,
  selectedPath: null,
  includeExcluded: false,
  scanning: false,
  error: null,
  unavailable: null,
  lastScanFinishedAt: null
});

let treeNodes = $state.raw<ExplorerTreeNode[]>([]);
const loadedDirectoryDepths = new Map<string, number>();

export function explorerNodes(): ExplorerTreeNode[] {
  return treeNodes;
}

export function loadedExplorerDirectoryDepth(path: string): number | null {
  return loadedDirectoryDepths.get(path) ?? null;
}

export function loadedExplorerDirectories(): Array<{ path: string; depth: number }> {
  return [...loadedDirectoryDepths].map(([path, depth]) => ({ path, depth }));
}

function atOrBelow(path: string, directory: string): boolean {
  return path === directory || path.startsWith(`${directory.replace(/\/+$/, '')}/`);
}

export function resetExplorer(): void {
  treeNodes = [];
  loadedDirectoryDepths.clear();
  explorer.root = null;
  explorer.activated = false;
  explorer.query = '';
  explorer.scrollTop = 0;
  explorer.selectedPath = null;
  explorer.includeExcluded = false;
  explorer.scanning = false;
  explorer.error = null;
  explorer.unavailable = null;
  explorer.lastScanFinishedAt = null;
}

export function beginScan(root: string): void {
  explorer.root = root;
  explorer.activated = true;
  explorer.scanning = true;
  explorer.error = null;
  explorer.unavailable = null;
}

export function applyDirectoryResult(
  directory: string,
  depth: number,
  entries: readonly SourceDirectoryEntry[]
): void {
  const previousChildren = treeNodes.filter((node) => node.parentPath === directory);
  const nextByPath = new Map(entries.map((entry) => [entry.path, entry]));
  const removedPaths = previousChildren
    .filter((node) => {
      const next = nextByPath.get(node.path);
      return !next || next.isDirectory !== node.isDirectory;
    })
    .map((node) => node.path);

  const retained = treeNodes.filter(
    (node) =>
      node.parentPath !== directory &&
      !removedPaths.some((removedPath) => atOrBelow(node.path, removedPath))
  );
  const previousByPath = new Map(previousChildren.map((node) => [node.path, node]));
  const children = entries.map<ExplorerTreeNode>((entry) => {
    const previous = previousByPath.get(entry.path);
    return {
      ...entry,
      parentPath: directory,
      depth,
      childCount: previous?.isDirectory === entry.isDirectory ? previous.childCount : 0,
      ignored: entry.excluded
    };
  });
  treeNodes = [...retained, ...children].map((node) =>
    node.path === directory ? { ...node, childCount: entries.length } : node
  );

  for (const loadedPath of [...loadedDirectoryDepths.keys()]) {
    if (removedPaths.some((removedPath) => atOrBelow(loadedPath, removedPath))) {
      loadedDirectoryDepths.delete(loadedPath);
    }
  }
  loadedDirectoryDepths.set(directory, depth);
  explorer.error = null;
  explorer.lastScanFinishedAt = Date.now();
  if (explorer.selectedPath && !treeNodes.some((node) => node.path === explorer.selectedPath)) {
    explorer.selectedPath = null;
  }
}

export function discardDirectory(directory: string): void {
  treeNodes = treeNodes.filter((node) => !atOrBelow(node.path, directory) || node.path === directory);
  for (const loadedPath of [...loadedDirectoryDepths.keys()]) {
    if (atOrBelow(loadedPath, directory)) loadedDirectoryDepths.delete(loadedPath);
  }
  if (explorer.selectedPath && !treeNodes.some((node) => node.path === explorer.selectedPath)) {
    explorer.selectedPath = null;
  }
}

export function failScan(message: string, unavailable: 'checkout-deleted' | null = null): void {
  treeNodes = [];
  loadedDirectoryDepths.clear();
  explorer.error = message;
  explorer.unavailable = unavailable;
}

export function markCheckoutDeleted(): void {
  resetExplorer();
  explorer.activated = true;
  explorer.error = 'This session\u2019s checkout/worktree no longer exists.';
  explorer.unavailable = 'checkout-deleted';
}

export function setExplorerError(message: string | null): void {
  explorer.error = message;
}

export function endScan(): void {
  explorer.scanning = false;
}

export function setQuery(query: string): void {
  explorer.query = query;
  explorer.scrollTop = 0;
}

export function setScrollTop(scrollTop: number): void {
  explorer.scrollTop = Math.max(0, scrollTop);
}

export function setViewportHeight(viewportHeight: number): void {
  if (viewportHeight > 0) explorer.viewportHeight = viewportHeight;
}

export function selectPath(path: string | null): void {
  explorer.selectedPath = path;
}

export function setIncludeExcluded(includeExcluded: boolean): void {
  explorer.includeExcluded = includeExcluded;
}
