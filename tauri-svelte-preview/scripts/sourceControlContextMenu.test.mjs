import assert from 'node:assert/strict';

import {
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

console.log('sourceControlContextMenu tests passed');
