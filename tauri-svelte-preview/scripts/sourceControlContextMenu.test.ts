import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  snapshotSourceControlCommitFileMenu,
  snapshotSourceControlCommitMenu,
  sourceControlCommitFileContextMenuItems
} from '../src/lib/shell/components/git/sourceControlContextMenu.ts';

const anchor = {
  left: 400,
  right: 400,
  top: 300,
  bottom: 300,
  containingBlockLeft: 0,
  containingBlockRight: 800,
  containingBlockTop: 0,
  containingBlockBottom: 600
};

assert.deepEqual(
  sourceControlCommitFileContextMenuItems(true).map(({ id, label, enabled }) => [
    id,
    label,
    enabled
  ]),
  [
    ['open-commit-diff', 'Open diff', true],
    ['open-current-file', 'Open current file in editor', true],
    ['copy-commit-path', 'Copy path', true]
  ]
);
assert.deepEqual(
  sourceControlCommitFileContextMenuItems(false).map(({ id, enabled }) => [id, enabled]),
  [
    ['open-commit-diff', false],
    ['open-current-file', false],
    ['copy-commit-path', true]
  ]
);

const commit = snapshotSourceControlCommitMenu({
  sha: 'abc123',
  isMerge: false,
  expanded: false,
  anchor
});
assert.equal(commit.items[0]?.label, 'View changed files');

const originalFile = { relativePath: 'src/app.ts', status: 'modified', badge: 'M' };
const file = snapshotSourceControlCommitFileMenu({
  sha: 'abc123',
  file: originalFile,
  readable: true,
  anchor
});
originalFile.relativePath = 'src/changed-after-menu-open.ts';
assert.equal(file.target.file.relativePath, 'src/app.ts', 'the menu owns an immutable row snapshot');

for (const relativePath of [
  '../src/lib/shell/panels/sourceControl/CommitTimeline.svelte',
  '../src/lib/shell/components/git/GitHistoryView.svelte'
]) {
  const source = readFileSync(new URL(relativePath, import.meta.url), 'utf8');
  assert.match(source, /let contextMenu = \$state\.raw<SourceControlMenuSnapshot \| null>\(null\)/);
  assert.match(source, /<SourceControlContextMenu/);
  assert.doesNotMatch(source, /ContextMenu\.Root/);
}

console.log('sourceControlContextMenu: commit and commit-file menus passed');
