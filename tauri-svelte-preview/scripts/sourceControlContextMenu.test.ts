import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  placeSourceControlContextMenu,
  snapshotSourceControlCommitMenu,
  snapshotSourceControlFileMenu,
  sourceControlCommitContextMenuItems,
  sourceControlFileContextMenuItems
} from '../src/lib/shell/components/git/sourceControlContextMenu.ts';

const staged = sourceControlFileContextMenuItems({
  groupAction: 'unstage',
  canWrite: true,
  busy: false,
  deleted: false,
  hasRoot: true
});
assert.deepEqual(
  staged.map(({ id, label, handler, enabled }) => [id, label, handler, enabled]),
  [
    ['open-diff', 'Open diff', 'pick-file', true],
    ['unstage', 'Unstage changes', 'run-file-action', true],
    ['discard', 'Discard changes…', 'ask-to-discard', true],
    ['open-file', 'Open file in editor', 'open-in-editor', true],
    ['copy-path', 'Copy path', 'copy-path', true]
  ],
  'a staged row maps to the existing diff, unstage, discard, editor, and path handlers'
);

assert.equal(
  staged.find((item) => item.id === 'discard')?.handler,
  'ask-to-discard',
  'discard goes through the ask-first handler, never straight to the service'
);

const changed = sourceControlFileContextMenuItems({
  groupAction: 'stage',
  canWrite: true,
  busy: false,
  deleted: false,
  hasRoot: true
});
assert.equal(changed.find((item) => item.id === 'stage')?.handler, 'run-file-action');
assert.equal(changed.some((item) => item.id === 'unstage'), false);

const unavailable = sourceControlFileContextMenuItems({
  groupAction: 'stage',
  canWrite: false,
  busy: true,
  deleted: true,
  hasRoot: false
});
assert.deepEqual(
  Object.fromEntries(unavailable.map((item) => [item.id, item.enabled])),
  {
    'open-diff': false,
    stage: false,
    discard: false,
    'open-file': false,
    'copy-path': true
  },
  'deleted/read-only rows disable actions that cannot apply and keep copying available'
);

const deletedButWritable = sourceControlFileContextMenuItems({
  groupAction: 'stage',
  canWrite: true,
  busy: false,
  deleted: true,
  hasRoot: true
});
assert.equal(
  deletedButWritable.find((item) => item.id === 'discard')?.enabled,
  true,
  'a deleted file can still be brought back, so its discard stays available'
);

assert.deepEqual(
  sourceControlCommitContextMenuItems(false).map(({ id, label, handler }) => [id, label, handler]),
  [
    ['toggle-commit', 'View changed files', 'toggle-commit'],
    ['copy-hash', 'Copy commit hash', 'copy-hash']
  ],
  'a closed commit row reuses its existing toggle handler and adds hash copying'
);
assert.equal(sourceControlCommitContextMenuItems(true)[0]?.label, 'Hide changed files');

const anchor = {
  left: 900,
  right: 900,
  top: 690,
  bottom: 690,
  containingBlockLeft: 300,
  containingBlockRight: 920,
  containingBlockTop: 100,
  containingBlockBottom: 710
};
const commitSnapshot = snapshotSourceControlCommitMenu({
  sha: 'abc123',
  isMerge: false,
  expanded: false,
  anchor
});
assert.equal(commitSnapshot.items[0]?.label, 'View changed files');
assert.deepEqual(commitSnapshot.target, { sha: 'abc123', isMerge: false });
assert.deepEqual(
  placeSourceControlContextMenu(anchor, { width: 180, height: 90 }),
  { left: 432, top: 512 },
  'the hidden measurement pass clamps the menu inside its Dockview containing block'
);

const liveFile = {
  relativePath: 'src/app.ts',
  status: 'modified',
  badge: 'M',
  indexStatus: '',
  worktreeStatus: 'modified'
};
const fileSnapshot = snapshotSourceControlFileMenu({
  groupAction: 'stage',
  file: liveFile,
  canWrite: true,
  busy: false,
  hasRoot: true,
  anchor
});
liveFile.relativePath = 'src/mutated-after-open.ts';
assert.equal(
  fileSnapshot.target.file.relativePath,
  'src/app.ts',
  'an open file menu owns a snapshot instead of following a mutating status row'
);
assert.equal(
  commitSnapshot.items[0]?.label,
  'View changed files',
  'the commit menu label remains the open-time expansion snapshot'
);

const graphPane = readFileSync(
  new URL('../src/lib/shell/components/git/GraphPane.svelte', import.meta.url),
  'utf8'
);
const changesPane = readFileSync(
  new URL('../src/lib/shell/components/git/ChangesPane.svelte', import.meta.url),
  'utf8'
);
for (const [name, source] of [['commit graph', graphPane], ['changed files', changesPane]] as const) {
  assert.doesNotMatch(source, /ContextMenu\.Root/, `${name} no longer mounts a floating root per row`);
  assert.match(source, /snapshotSourceControl(?:Commit|File)Menu/, `${name} snapshots its target on open`);
  assert.match(source, /<SourceControlContextMenu/, `${name} renders one measured menu for the snapshot`);
}

console.log('sourceControlContextMenu tests passed');
