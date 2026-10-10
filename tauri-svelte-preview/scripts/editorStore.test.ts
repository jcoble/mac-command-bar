/**
 * Covers the editor store's rules. They live in `editorStoreOps.ts` (pure
 * functions) precisely so this test can run under plain Node — the runes
 * module `editorStore.svelte.ts` is a thin wrapper that holds the state and
 * assigns what these functions return.
 */
import assert from 'node:assert/strict';
import {
  activePathAfterClose,
  closeOpenFile,
  findOpenFile,
  isFileOpen,
  modelPathToDisposeOnClose,
  needsRead,
  openEditorFileFromRecord,
  patchOpenFile,
  revealLineInOpenFile,
  upsertOpenFile
} from '../src/lib/shell/editor/editorStoreOps.ts';

const recordFor = (name) => ({
  path: `/p/src/${name}`,
  relativePath: `src/${name}`,
  fileName: name,
  language: 'typescript',
  byteCount: 12
});

const stripOf = (...names) => names.reduce((files, name) => upsertOpenFile(files, recordFor(name)), []);

// a fresh entry is unread, not loading, and has no pending reveal
{
  const file = openEditorFileFromRecord(recordFor('a.ts'));
  assert.equal(file.preview, null);
  assert.equal(file.draftContent, null);
  assert.equal(file.dirty, false);
  assert.equal(file.saving, false);
  assert.equal(file.loading, false);
  assert.equal(file.error, null);
  assert.equal(file.targetLine, null);
  assert.equal(file.targetLineRequestId, 0);
  assert.equal(needsRead(file), true);
}

// drafts remain local until save, and dirty is computed against disk content
{
  const record = recordFor('draft.ts');
  let files = [openEditorFileFromRecord(record)];
  const preview = { ...record, content: 'saved', lineCount: 1 };
  files = patchOpenFile(files, record.path, {
    preview,
    draftContent: preview.content,
    dirty: false
  });
  files = patchOpenFile(files, record.path, { draftContent: 'edited', dirty: true });
  const edited = findOpenFile(files, record.path);
  assert.equal(edited.preview.content, 'saved');
  assert.equal(edited.draftContent, 'edited');
  assert.equal(edited.dirty, true);
}

// opening appends in click order; opening the same file again changes nothing
{
  const files = stripOf('a.ts', 'b.ts');
  assert.deepEqual(
    files.map((file) => file.fileName),
    ['a.ts', 'b.ts']
  );
  const withPreview = patchOpenFile(files, '/p/src/a.ts', {
    preview: { ...recordFor('a.ts'), content: 'x', lineCount: 1 },
    loading: false
  });
  const reopened = upsertOpenFile(withPreview, recordFor('a.ts'));
  assert.equal(reopened.length, 2, 'no duplicate entry');
  assert.deepEqual(
    reopened.map((file) => file.fileName),
    ['a.ts', 'b.ts'],
    'position kept'
  );
  assert.ok(findOpenFile(reopened, '/p/src/a.ts').preview, 'contents kept: no second read');
  assert.equal(needsRead(findOpenFile(reopened, '/p/src/a.ts')), false);
}

// the source array is never mutated
{
  const files = stripOf('a.ts');
  const snapshot = JSON.stringify(files);
  upsertOpenFile(files, recordFor('b.ts'));
  patchOpenFile(files, '/p/src/a.ts', { loading: true });
  revealLineInOpenFile(files, '/p/src/a.ts', 5);
  closeOpenFile(files, '/p/src/a.ts');
  assert.equal(JSON.stringify(files), snapshot);
}

// a read in flight, then a failure, then a retry
{
  let files = stripOf('a.ts');
  files = patchOpenFile(files, '/p/src/a.ts', { loading: true });
  assert.equal(needsRead(findOpenFile(files, '/p/src/a.ts')), false, 'no second read while reading');
  files = patchOpenFile(files, '/p/src/a.ts', { loading: false, error: 'Could not read this file.' });
  const failed = findOpenFile(files, '/p/src/a.ts');
  assert.equal(failed.error, 'Could not read this file.');
  assert.equal(needsRead(failed), false, 'a failed read is not retried on its own');
  files = patchOpenFile(files, '/p/src/a.ts', { loading: true, error: null });
  assert.equal(findOpenFile(files, '/p/src/a.ts').error, null);
}

// patching an unknown path changes nothing
{
  const files = stripOf('a.ts');
  assert.deepEqual(patchOpenFile(files, '/p/src/ghost.ts', { loading: true }), files);
  assert.deepEqual(revealLineInOpenFile(files, '/p/src/ghost.ts', 3), files);
}

// every reveal bumps the request id, so the same line can be asked for twice
{
  let files = stripOf('a.ts', 'b.ts');
  files = revealLineInOpenFile(files, '/p/src/a.ts', 42);
  assert.equal(findOpenFile(files, '/p/src/a.ts').targetLine, 42);
  assert.equal(findOpenFile(files, '/p/src/a.ts').targetLineRequestId, 1);
  files = revealLineInOpenFile(files, '/p/src/a.ts', 42);
  assert.equal(findOpenFile(files, '/p/src/a.ts').targetLineRequestId, 2, 'repeat still scrolls');
  assert.equal(findOpenFile(files, '/p/src/b.ts').targetLineRequestId, 0, 'other files untouched');
  files = revealLineInOpenFile(files, '/p/src/a.ts', null);
  assert.equal(findOpenFile(files, '/p/src/a.ts').targetLine, null);
  assert.equal(findOpenFile(files, '/p/src/a.ts').targetLineRequestId, 3);
}

// closing the active file falls right, then left, then to nothing
{
  const files = stripOf('a.ts', 'b.ts', 'c.ts');
  assert.equal(activePathAfterClose(files, '/p/src/b.ts', '/p/src/b.ts'), '/p/src/c.ts');
  assert.equal(activePathAfterClose(files, '/p/src/c.ts', '/p/src/c.ts'), '/p/src/b.ts');
  const one = stripOf('a.ts');
  assert.equal(activePathAfterClose(one, '/p/src/a.ts', '/p/src/a.ts'), null);
}

// closing an inactive file leaves the shown file alone
{
  const files = stripOf('a.ts', 'b.ts', 'c.ts');
  assert.equal(activePathAfterClose(files, '/p/src/a.ts', '/p/src/c.ts'), '/p/src/c.ts');
  assert.deepEqual(
    closeOpenFile(files, '/p/src/a.ts').map((file) => file.fileName),
    ['b.ts', 'c.ts']
  );
}

// an active path that is not open any more is dropped rather than kept
{
  const files = stripOf('a.ts');
  assert.equal(activePathAfterClose(files, '/p/src/a.ts', '/p/src/gone.ts'), null);
  assert.equal(isFileOpen(files, '/p/src/gone.ts'), false);
}

// Closing releases only clean tab models; an unsaved draft stays live.
{
  const clean = openEditorFileFromRecord(recordFor('clean.ts'));
  const dirty = { ...openEditorFileFromRecord(recordFor('dirty.ts')), dirty: true };
  assert.equal(modelPathToDisposeOnClose(clean), '/p/src/clean.ts');
  assert.equal(modelPathToDisposeOnClose(dirty), null);
}

console.log('editorStore tests passed');

// A refresh must preserve a local draft without inventing a disk conflict.
globalThis.$state = <T>(value: T): T => value;
const store = await import('../src/lib/shell/editor/editorStore.svelte.ts');
{
  const record = recordFor('refresh-draft.ts');
  const preview = { ...record, content: 'saved', lineCount: 1 };
  store.openEditorFile(record);
  store.setEditorFilePreview(record.path, preview);
  store.setEditorFileDraft(record.path, 'my draft');
  store.setEditorFilePreview(record.path, preview);
  assert.equal(store.editorFileFor(record.path)?.draftContent, 'my draft');
  assert.equal(store.editorFileFor(record.path)?.conflict, null);
  store.setEditorFilePreview(record.path, { ...preview, content: 'external edit' });
  assert.match(store.editorFileFor(record.path)?.conflict ?? '', /changed on disk/);
  store.setEditorFilePreview(record.path, { ...preview, content: 'external edit' });
  assert.match(store.editorFileFor(record.path)?.conflict ?? '', /changed on disk/);
  assert.equal(store.editorFileFor(record.path)?.draftContent, 'my draft');
  store.setEditorFilePreview(record.path, { ...preview, content: 'my draft' }, 'my draft');
  assert.equal(store.editorFileFor(record.path)?.conflict, null);
  assert.equal(store.editorFileFor(record.path)?.dirty, false);
}
console.log('editor refresh preserves drafts and reports actual disk changes');

// Switching sessions stores a draft and puts it back before its file is read.
// That first read reports a conflict only when the disk revision really moved.
{
  const { captureWorkspace, normalizeWorkspaceSnapshot, planWorkspaceRestore } =
    await import('../src/lib/shell/sessionWorkspaces.ts');
  const record = recordFor('restored-draft.ts');
  const saved = { ...record, content: 'saved', lineCount: 1, revision: 'rev-saved' };
  const external = { ...saved, content: 'external edit', revision: 'rev-external' };
  const switchAwayAndBack = () => {
    const snapshot = captureWorkspace({
      openFiles: store.editorState.openFiles,
      activePath: record.path,
      selectedPath: null,
      scrollTop: 0,
      rightTab: 'files'
    });
    const plan = planWorkspaceRestore(normalizeWorkspaceSnapshot(snapshot));
    store.restoreEditorFiles(plan.openFiles, plan.activePath);
  };
  store.resetEditorState();
  store.openEditorFile(record);
  store.setEditorFilePreview(record.path, saved);
  store.setEditorFileDraft(record.path, 'my draft');

  switchAwayAndBack();
  store.setEditorFilePreview(record.path, saved);
  assert.equal(store.editorFileFor(record.path)?.conflict, null);
  assert.equal(store.editorFileFor(record.path)?.dirty, true);
  assert.equal(store.editorFileFor(record.path)?.draftContent, 'my draft');

  switchAwayAndBack();
  store.setEditorFilePreview(record.path, external);
  assert.match(store.editorFileFor(record.path)?.conflict ?? '', /changed on disk/);
  assert.equal(store.editorFileFor(record.path)?.draftContent, 'my draft');

  // A draft already in conflict stays in conflict after another switch.
  switchAwayAndBack();
  store.setEditorFilePreview(record.path, external);
  assert.match(store.editorFileFor(record.path)?.conflict ?? '', /changed on disk/);
}
console.log('a restored draft reports a conflict only when the file changed on disk');
