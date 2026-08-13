/**
 * fileTreeModel.ts — the Files panel's PURE view maths.
 *
 * The panel renders one flat list of rows. This module turns the scanned file
 * records into that list: a node per folder and per file, ordered folders-first
 * then alphabetically, cut down to the folders that are open, and finally cut
 * down again to the rows the viewport can actually show.
 *
 * No Svelte, no runes, no DOM, no backend — so `scripts/filesPanel.test.ts` can
 * load it in plain Node and every rule the panel relies on is covered.
 *
 * The tree building and the windowing arithmetic are NOT rewritten here: they
 * are the audited helpers in `src/lib/sourceData.ts` (`buildSourceTree`,
 * `virtualizeSourceTreeRows`), which this module composes.
 */
import {
  buildSourceTree,
  virtualizeSourceTreeRows,
  type SourceRecord,
  type SourceTreeNode,
  type SourceTreeRow
} from '../../../sourceData.ts';

/** Height of one row in pixels. The CSS and the maths must agree exactly: rows
 * are placed by multiplying this number, so a taller row drifts out of slot.
 * 28 is the kit's row height (`ListRow` is `min-h-7`), not a number of ours. */
export const FILE_TREE_ROW_HEIGHT = 28;

/** Rows kept rendered above and below the visible window, so a fast scroll does
 * not flash empty space before the next frame lands. */
export const FILE_TREE_OVERSCAN_ROWS = 8;

/** Viewport height assumed before the panel has ever been measured — it is
 * mounted while hidden, where its real height is zero, and windowing against
 * zero would render a single row. */
export const FILE_TREE_FALLBACK_VIEWPORT_HEIGHT = 420;

export interface FileTreeNode {
  path: string;
  name: string;
  /** Depth from the root, for indenting. Root children are 0. */
  depth: number;
  isDirectory: boolean;
  /** Directories only: how many entries are inside. */
  childCount: number;
  /** True when git ignores this path. Ignored files render dimmed. */
  ignored: boolean;
}

export interface FileTreeNodesOptions {
  /** Absolute path of the folder being listed. Directory nodes are named from
   * it, so a folder row carries a real path rather than a relative fragment. */
  rootPath?: string;
  /** Paths git ignores. Ignored rows are dimmed, never dropped. */
  ignoredPaths?: ReadonlySet<string>;
}

/** Join a folder path and one more segment, tolerating a trailing slash and an
 * empty root (the panel has no root until a session is picked). */
function joinPath(parentPath: string, name: string): string {
  if (!parentPath) return name;
  return `${parentPath.replace(/\/+$/, '')}/${name}`;
}

/** Every folder and file in the scan, flattened, with the depth each one sits
 * at. Order is not meaningful here — `visibleFileTreeNodes` sorts. */
export function fileTreeNodesFromRecords(
  records: readonly SourceRecord[],
  options: FileTreeNodesOptions = {}
): FileTreeNode[] {
  const rootPath = (options.rootPath ?? '').trim();
  const ignoredPaths = options.ignoredPaths ?? new Set<string>();
  const nodes: FileTreeNode[] = [];

  function visit(list: readonly SourceTreeNode[], depth: number, parentPath: string): void {
    for (const node of list) {
      const isDirectory = node.file === null;
      const path = node.file ? node.file.path : joinPath(parentPath, node.name);
      nodes.push({
        path,
        name: node.name,
        depth,
        isDirectory,
        childCount: isDirectory ? node.children.length : 0,
        ignored: ignoredPaths.has(path)
      });
      if (isDirectory) visit(node.children, depth + 1, path);
    }
  }

  visit(buildSourceTree([...records]), 0, rootPath);
  return nodes;
}

/** The folder a node sits in, as a plain string key. Depth-0 nodes all share
 * the same parent, whatever the root happens to be called. */
function parentKeyOf(node: FileTreeNode): string {
  const cut = node.path.lastIndexOf('/');
  return cut < 0 ? '' : node.path.slice(0, cut);
}

/** Folders before files, then alphabetically within each group. */
function compareNodes(left: FileTreeNode, right: FileTreeNode): number {
  if (left.isDirectory !== right.isDirectory) return left.isDirectory ? -1 : 1;
  return left.name.localeCompare(right.name, undefined, { numeric: true, sensitivity: 'base' });
}

/** The flat list to render, given which directories are open. Directories first,
 *  then files, each alphabetical, and a closed directory contributes no
 *  descendants. */
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
    const key = parentKeyOf(node);
    const siblings = byParent.get(key);
    if (siblings) siblings.push(node);
    else byParent.set(key, [node]);
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

/** Every directory in the tree, for the filter's "open everything" pass. */
export function allDirectoryPaths(nodes: readonly FileTreeNode[]): Set<string> {
  const paths = new Set<string>();
  for (const node of nodes) {
    if (node.isDirectory) paths.add(node.path);
  }
  return paths;
}

/** Toggling a directory that is closed opens it, and vice versa. Toggling a file
 *  is a no-op. Always a NEW set: expansion is held in a plain `Set`, which
 *  Svelte does not track in place — only assigning a fresh one redraws. */
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
  /** The rows to render right now. */
  nodes: FileTreeNode[];
  /** Height of the blank block standing in for the rows above. */
  topSpacerHeight: number;
  /** Height of the blank block standing in for the rows below. */
  bottomSpacerHeight: number;
  /** Height of the whole tree, so the scrollbar tells the truth. */
  totalHeight: number;
}

/** One shared carrier row. `virtualizeSourceTreeRows` decides the window from
 * the row COUNT and then slices, so the objects it slices are never read — and
 * a fresh array of real rows per scroll frame would be pure waste. */
const WINDOW_CARRIER_ROW: SourceTreeRow = {
  node: { id: '', name: '', relativePath: '', file: null, children: [] },
  level: 0
};

/** The slice of a visible list the viewport can show, plus the spacers around
 * it. A ten-thousand-file repository renders a few dozen rows. */
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
