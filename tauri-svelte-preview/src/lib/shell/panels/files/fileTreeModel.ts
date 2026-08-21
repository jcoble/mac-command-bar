/** Pure sorting, filtering, expansion and viewport maths for the Files panel. */
import {
  virtualizeSourceTreeRows,
  type SourceTreeRow
} from '../../../sourceData.ts';
import type { ExplorerTreeNode } from '../../explorer/explorerStore.svelte.ts';

export type FileTreeNode = ExplorerTreeNode;

export const FILE_TREE_ROW_HEIGHT = 28;
export const FILE_TREE_OVERSCAN_ROWS = 8;
export const FILE_TREE_FALLBACK_VIEWPORT_HEIGHT = 420;

function compareNodes(left: FileTreeNode, right: FileTreeNode): number {
  if (left.isDirectory !== right.isDirectory) return left.isDirectory ? -1 : 1;
  return left.name.localeCompare(right.name, undefined, { numeric: true, sensitivity: 'base' });
}

export function filterFileTreeNodes(
  nodes: readonly FileTreeNode[],
  query: string
): FileTreeNode[] {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return [...nodes];
  const matches = nodes.filter((node) => node.name.toLowerCase().includes(normalized));
  return nodes.filter(
    (node) =>
      matches.some((match) => match.path === node.path) ||
      (node.isDirectory && matches.some((match) => match.path.startsWith(`${node.path}/`))) ||
      matches.some((match) => match.isDirectory && node.path.startsWith(`${match.path}/`))
  );
}

export function visibleFileTreeNodes(
  nodes: readonly FileTreeNode[],
  expanded: ReadonlySet<string>
): FileTreeNode[] {
  const byParent = new Map<string, FileTreeNode[]>();
  const roots: FileTreeNode[] = [];
  for (const node of nodes) {
    if (node.depth === 0) {
      roots.push(node);
      continue;
    }
    const siblings = byParent.get(node.parentPath);
    if (siblings) siblings.push(node);
    else byParent.set(node.parentPath, [node]);
  }

  const visible: FileTreeNode[] = [];
  function emit(list: FileTreeNode[]): void {
    for (const node of [...list].sort(compareNodes)) {
      visible.push(node);
      if (!node.isDirectory || !expanded.has(node.path)) continue;
      const children = byParent.get(node.path);
      if (children) emit(children);
    }
  }
  emit(roots);
  return visible;
}

export function allDirectoryPaths(nodes: readonly FileTreeNode[]): Set<string> {
  return new Set(nodes.filter((node) => node.isDirectory).map((node) => node.path));
}

export function toggleDirectory(
  expanded: ReadonlySet<string>,
  node: FileTreeNode
): Set<string> {
  const next = new Set(expanded);
  if (!node.isDirectory) return next;
  if (next.has(node.path)) next.delete(node.path);
  else next.add(node.path);
  return next;
}

export interface FileTreeWindow {
  nodes: FileTreeNode[];
  topSpacerHeight: number;
  bottomSpacerHeight: number;
  totalHeight: number;
}

const WINDOW_CARRIER_ROW: SourceTreeRow = {
  node: { id: '', name: '', relativePath: '', file: null, children: [] },
  level: 0
};

export function windowFileTreeNodes(
  nodes: readonly FileTreeNode[],
  scrollTop: number,
  viewportHeight: number,
  rowHeight: number = FILE_TREE_ROW_HEIGHT,
  overscanRows: number = FILE_TREE_OVERSCAN_ROWS
): FileTreeWindow {
  const virtual = virtualizeSourceTreeRows(
    new Array<SourceTreeRow>(nodes.length).fill(WINDOW_CARRIER_ROW),
    scrollTop,
    viewportHeight > 0 ? viewportHeight : FILE_TREE_FALLBACK_VIEWPORT_HEIGHT,
    rowHeight,
    overscanRows
  );

  return {
    nodes: nodes.slice(virtual.startIndex, virtual.endIndex),
    topSpacerHeight: virtual.topSpacerHeight,
    bottomSpacerHeight: virtual.bottomSpacerHeight,
    totalHeight: virtual.totalHeight
  };
}
