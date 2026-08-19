import assert from 'node:assert/strict';

import {
  filesPanelActions,
  type FilesPanelActionId
} from '../src/lib/shell/panels/files/filesPanelActions.ts';
import type { FileTreeNode } from '../src/lib/shell/panels/files/fileTreeModel.ts';

function node(path: string, isDirectory: boolean): FileTreeNode {
  return {
    path,
    name: path.slice(path.lastIndexOf('/') + 1),
    depth: 0,
    isDirectory,
    childCount: isDirectory ? 2 : 0,
    ignored: false
  };
}

function ids(target: FileTreeNode): FilesPanelActionId[] {
  return filesPanelActions(target).map((action) => action.id);
}

// A folder can hold something new, so it offers the two ways of putting
// something in it before the four every row has.
assert.deepEqual(ids(node('/repo/src', true)), [
  'new-file',
  'new-folder',
  'rename',
  'delete',
  'reveal-in-finder',
  'copy-path'
]);

// A file holds nothing, so the two create actions are absent rather than
// present-and-off: an item that can never apply teaches nothing.
assert.deepEqual(ids(node('/repo/src/App.ts', false)), [
  'rename',
  'delete',
  'reveal-in-finder',
  'copy-path'
]);

// The words the menu draws are the words a person reads, so they are part of
// the contract and not free for a caller to restyle.
assert.deepEqual(
  filesPanelActions(node('/repo/src', true)).map((action) => action.label),
  ['New file', 'New folder', 'Rename', 'Delete', 'Reveal in Finder', 'Copy path']
);

// Deleting moves the path to the Trash, which is recoverable; nothing else in
// the list changes what is on disk, so Delete is the only destructive one.
assert.deepEqual(
  filesPanelActions(node('/repo/src/App.ts', false))
    .filter((action) => action.destructive)
    .map((action) => action.id),
  ['delete']
);

console.log('filesPanelActions tests passed');
