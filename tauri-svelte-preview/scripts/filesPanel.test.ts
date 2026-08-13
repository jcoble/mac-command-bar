/**
 * filesPanel.test.ts — the Files panel's tree maths.
 *
 * The panel itself is a Svelte component and cannot be loaded here, so every
 * rule worth testing lives in `fileTreeModel.ts`: which rows a set of open
 * folders produces, how they are ordered, what toggling a folder does, and how
 * the visible window is cut out of a large tree.
 */
import assert from 'node:assert/strict';

import type { SourceRecord } from '../src/lib/sourceData.ts';
import {
  FILE_TREE_ROW_HEIGHT,
  fileTreeNodesFromRecords,
  toggleDirectory,
  visibleFileTreeNodes,
  windowFileTreeNodes,
  type FileTreeNode
} from '../src/lib/shell/panels/files/fileTreeModel.ts';

const ROOT = '/work/project';

function record(relativePath: string): SourceRecord {
  const segments = relativePath.split('/');
  return {
    path: `${ROOT}/${relativePath}`,
    relativePath,
    fileName: segments[segments.length - 1],
    language: 'typescript',
    byteCount: 10
  };
}

const records = [
  record('src/lib/one.ts'),
  record('src/lib/deep/two.ts'),
  record('src/top.ts'),
  record('readme.md'),
  record('assets/logo.svg')
];

const nodes = fileTreeNodesFromRecords(records, { rootPath: ROOT });

function pathsOf(list: readonly FileTreeNode[]): string[] {
  return list.map((node) => node.path.slice(ROOT.length + 1));
}

function nodeAt(path: string): FileTreeNode {
  const found = nodes.find((node) => node.path === `${ROOT}/${path}`);
  assert.ok(found, `expected a node for ${path}`);
  return found;
}

// ── The flat model ───────────────────────────────────────────────────────────

assert.deepEqual(
  pathsOf(nodes.filter((node) => node.isDirectory)).sort(),
  ['assets', 'src', 'src/lib', 'src/lib/deep'],
  'every folder in the scan becomes a directory node with an absolute path'
);
assert.equal(nodeAt('src').depth, 0, 'a folder directly under the root sits at depth 0');
assert.equal(nodeAt('src/lib').depth, 1, 'depth counts from the root');
assert.equal(nodeAt('src/lib/deep/two.ts').depth, 3, 'a file carries the depth of its folder plus one');
assert.equal(nodeAt('src').childCount, 2, 'a directory counts the entries directly inside it');
assert.equal(nodeAt('readme.md').childCount, 0, 'a file has no entries inside it');
assert.equal(nodeAt('readme.md').isDirectory, false);
assert.equal(nodeAt('readme.md').ignored, false, 'nothing is ignored unless it is named as ignored');

// ── Which rows are visible ───────────────────────────────────────────────────

const collapsed = visibleFileTreeNodes(nodes, new Set());
assert.deepEqual(
  pathsOf(collapsed),
  ['assets', 'src', 'readme.md'],
  'with nothing expanded only the root entries show, directories before files'
);

const srcOpen = visibleFileTreeNodes(nodes, new Set([`${ROOT}/src`]));
assert.deepEqual(
  pathsOf(srcOpen),
  ['assets', 'src', 'src/lib', 'src/top.ts', 'readme.md'],
  'expanding one directory reveals its immediate children and nothing deeper'
);

const libOpen = visibleFileTreeNodes(nodes, new Set([`${ROOT}/src`, `${ROOT}/src/lib`]));
assert.deepEqual(
  pathsOf(libOpen),
  ['assets', 'src', 'src/lib', 'src/lib/deep', 'src/lib/one.ts', 'src/top.ts', 'readme.md'],
  'a nested directory opens under its parent, and its own children stay hidden'
);

const shuffled = [...nodes].reverse();
assert.deepEqual(
  pathsOf(visibleFileTreeNodes(shuffled, new Set([`${ROOT}/src`]))),
  pathsOf(srcOpen),
  'the order of the input does not matter: directories sort before files, each group alphabetically'
);

const closedChild = visibleFileTreeNodes(nodes, new Set([`${ROOT}/src/lib`]));
assert.deepEqual(
  pathsOf(closedChild),
  ['assets', 'src', 'readme.md'],
  'a closed directory contributes no descendants even when a deeper folder is open'
);

// ── Opening and closing ──────────────────────────────────────────────────────

const beforeToggle = new Set([`${ROOT}/assets`]);
const opened = toggleDirectory(beforeToggle, nodeAt('src'));
assert.notEqual(opened, beforeToggle, 'toggling returns a new set');
assert.deepEqual([...beforeToggle], [`${ROOT}/assets`], 'the set handed in is not changed');
assert.ok(opened.has(`${ROOT}/src`), 'a closed directory opens');
assert.ok(opened.has(`${ROOT}/assets`), 'the other open directories stay open');

const closed = toggleDirectory(opened, nodeAt('src'));
assert.equal(closed.has(`${ROOT}/src`), false, 'an open directory closes');

const afterFile = toggleDirectory(opened, nodeAt('readme.md'));
assert.deepEqual(
  [...afterFile].sort(),
  [...opened].sort(),
  'toggling a file changes nothing'
);

// ── Ignored files are dimmed, not filtered out ───────────────────────────────

const withIgnored = fileTreeNodesFromRecords(records, {
  rootPath: ROOT,
  ignoredPaths: new Set([`${ROOT}/readme.md`])
});
const ignoredRow = withIgnored.find((node) => node.path === `${ROOT}/readme.md`);
assert.ok(ignoredRow, 'an ignored file is still part of the tree');
assert.equal(ignoredRow.ignored, true, 'it is marked ignored so the row can dim it');
assert.ok(
  visibleFileTreeNodes(withIgnored, new Set()).some((node) => node.path === ignoredRow.path),
  'an ignored file is still a visible row — dimmed, not hidden'
);

// ── The rendered window ──────────────────────────────────────────────────────

const manyRecords = Array.from({ length: 500 }, (_, index) =>
  record(`flat/file-${String(index).padStart(3, '0')}.ts`)
);
const manyNodes = fileTreeNodesFromRecords(manyRecords, { rootPath: ROOT });
const manyVisible = visibleFileTreeNodes(manyNodes, new Set([`${ROOT}/flat`]));
assert.equal(manyVisible.length, 501, 'the folder plus its 500 files are all visible rows');

const window = windowFileTreeNodes(manyVisible, 0, 240);
assert.ok(window.nodes.length < manyVisible.length, 'a large tree renders a window, not every row');
assert.equal(window.topSpacerHeight, 0, 'nothing is skipped at the top of an unscrolled tree');
assert.equal(
  window.totalHeight,
  manyVisible.length * FILE_TREE_ROW_HEIGHT,
  'the spacers add up to the full tree height so the scrollbar is honest'
);

const scrolled = windowFileTreeNodes(manyVisible, 100 * FILE_TREE_ROW_HEIGHT, 240);
assert.ok(scrolled.topSpacerHeight > 0, 'scrolling down leaves a spacer above the rendered rows');
assert.equal(
  scrolled.topSpacerHeight + scrolled.nodes.length * FILE_TREE_ROW_HEIGHT + scrolled.bottomSpacerHeight,
  window.totalHeight,
  'rendered rows plus both spacers equal the whole tree'
);
assert.ok(
  scrolled.nodes.includes(manyVisible[100]),
  'the row at the scroll offset is one of the rendered ones'
);

console.log('files panel tree tests passed');
