import assert from 'node:assert/strict';
import test from 'node:test';

import {
  captureWorkspace,
  clearWorkspaceEditorTabs,
  diffPathFor,
  OPEN_PATHS_CAP,
  planWorkspaceRestore,
  pruneWorkspaces,
  readWorkspaces,
  SESSION_WORKSPACES_STORAGE_KEY,
  writeWorkspaces
} from '../src/lib/shell/sessionWorkspaces.ts';

/** A storage stub that can be handed junk, or made to refuse writes. */
function storageStub(initial = {}, { refuseWrites = false } = {}) {
  const items = new Map(Object.entries(initial));
  return {
    items,
    getItem: (key) => (items.has(key) ? items.get(key) : null),
    setItem: (key, value) => {
      if (refuseWrites) throw new Error('storage is full');
      items.set(key, value);
    },
    removeItem: (key) => items.delete(key)
  };
}

// Close All Editors empties every session's saved tabs without dropping its other workspace state.
{
  const snapshot = captureWorkspace({
    openFiles: openFiles('/repo/a.ts'),
    activePath: '/repo/a.ts',
    selectedPath: '/repo/selected.ts',
    scrollTop: 18
  });
  const all = { 'owned-1': snapshot, 'owned-2': snapshot };
  const cleared = clearWorkspaceEditorTabs(all);
  assert.deepEqual(Object.values(cleared).map(({ openPaths, activePath }) => ({ openPaths, activePath })), [
    { openPaths: [], activePath: null },
    { openPaths: [], activePath: null }
  ]);
  assert.equal(cleared['owned-1'].selectedPath, '/repo/selected.ts');
  assert.equal(all['owned-1'].openPaths.length, 1, 'the input is untouched');
}

/** Editor entries with only the field the capture reads. */
function openFiles(...paths) {
  return paths.map((path) => ({ path }));
}

test('workspace_record_round_trips_view_state_per_path', () => {
  const storage = storageStub();
  const viewState = {
    cursorState: [{ position: { lineNumber: 8, column: 3 } }],
    viewState: { scrollTop: 240, scrollLeft: 0 }
  };
  const snapshot = captureWorkspace({
    openFiles: openFiles('/repo/a.ts', '/repo/b.ts'),
    activePath: '/repo/b.ts',
    selectedPath: null,
    scrollTop: 0,
    viewStates: { '/repo/b.ts': viewState }
  });

  assert.equal(writeWorkspaces(storage, { session: snapshot }), true);
  assert.deepEqual(readWorkspaces(storage).session.fileStates, {
    '/repo/b.ts': { viewState }
  });
});

test('snapshot_stores_drafts_but_no_file_preview_text', () => {
  const snapshot = captureWorkspace({
    openFiles: [{
      path: '/repo/a.ts',
      preview: { content: 'saved full file text' },
      draftContent: 'unsaved draft text',
      dirty: true
    }],
    activePath: '/repo/a.ts',
    selectedPath: null,
    scrollTop: 0
  });
  const serialized = JSON.stringify(snapshot);

  assert.match(serialized, /unsaved draft text/);
  assert.doesNotMatch(serialized, /saved full file text/);
  assert.deepEqual(snapshot.fileStates, {
    '/repo/a.ts': { draftContent: 'unsaved draft text' }
  });
});

test('capture_keeps_every_drafted_path_over_the_cap', () => {
  const paths = Array.from({ length: 13 }, (_, index) => `/repo/file-${index}.ts`);
  const files = paths.map((path, index) => ({
    path,
    draftContent: index === 0 ? 'oldest unsaved draft' : null,
    dirty: index === 0
  }));

  const snapshot = captureWorkspace({
    openFiles: files,
    activePath: paths.at(-1),
    selectedPath: null,
    scrollTop: 0
  });

  assert.ok(snapshot.openPaths.includes(paths[0]));
  assert.equal(snapshot.fileStates?.[paths[0]]?.draftContent, 'oldest unsaved draft');
});

test('capture_reports_refused_write', () => {
  const storage = storageStub({}, { refuseWrites: true });
  const snapshot = captureWorkspace({
    openFiles: openFiles('/repo/a.ts'),
    activePath: '/repo/a.ts',
    selectedPath: null,
    scrollTop: 0
  });

  assert.equal(writeWorkspaces(storage, { session: snapshot }), false);
});

test('restore_plan_marks_the_active_path_without_file_contents', () => {
  const snapshot = captureWorkspace({
    openFiles: openFiles('/repo/a.ts', '/repo/b.ts', '/repo/c.ts'),
    activePath: '/repo/b.ts',
    selectedPath: null,
    scrollTop: 0
  });

  const plan = planWorkspaceRestore(snapshot);
  assert.deepEqual(plan.openFiles, [
    { path: '/repo/a.ts' },
    { path: '/repo/b.ts' },
    { path: '/repo/c.ts' }
  ]);
  assert.equal(plan.activePath, '/repo/b.ts');
});

// A capture is exactly what was on screen: the strip in order, the file showing,
// the highlighted file, the scroll offset, and the file the Diff tab was
// showing. A capture that says nothing about a diff still carries both diff
// fields, as nulls — the Diff tab is one tab for the whole
// shell, so "this session was not looking at one" is an answer it needs told.
{
  const snapshot = captureWorkspace({
    openFiles: openFiles('/repo/a.ts', '/repo/b.ts'),
    activePath: '/repo/b.ts',
    selectedPath: '/repo/b.ts',
    scrollTop: 120
  });
  assert.deepEqual(snapshot, {
    openPaths: ['/repo/a.ts', '/repo/b.ts'],
    activePath: '/repo/b.ts',
    selectedPath: '/repo/b.ts',
    scrollTop: 120,
    diffPath: null,
    diffRoot: null
  });
}

// An empty workspace captures cleanly rather than as holes.
{
  const snapshot = captureWorkspace({
    openFiles: [],
    activePath: null,
    selectedPath: null,
    scrollTop: 0
  });
  assert.deepEqual(snapshot, {
    openPaths: [],
    activePath: null,
    selectedPath: null,
    scrollTop: 0,
    diffPath: null,
    diffRoot: null
  });
}

// Twelve is the cap, and it is the twelve most recent — the END of the strip,
// where new files land.
{
  const paths = Array.from({ length: 20 }, (_, index) => `/repo/file-${index}.ts`);
  const snapshot = captureWorkspace({
    openFiles: openFiles(...paths),
    activePath: '/repo/file-19.ts',
    selectedPath: null,
    scrollTop: 0
  });
  assert.equal(OPEN_PATHS_CAP, 12);
  assert.deepEqual(snapshot.openPaths, paths.slice(8), 'the last twelve survive');
}

// The file on screen is never the one dropped, even when it sits outside the
// twelve most recent — coming back to a session and not seeing the file you
// left showing would be the one obvious failure.
{
  const paths = Array.from({ length: 20 }, (_, index) => `/repo/file-${index}.ts`);
  const snapshot = captureWorkspace({
    openFiles: openFiles(...paths),
    activePath: '/repo/file-0.ts',
    selectedPath: null,
    scrollTop: 0
  });
  assert.equal(snapshot.openPaths.length, 12, 'still twelve');
  assert.equal(snapshot.openPaths[0], '/repo/file-0.ts', 'and it keeps its place in the strip');
  assert.deepEqual(
    snapshot.openPaths.slice(1),
    paths.slice(9),
    'one of the recent twelve makes room for it'
  );
}

// A file showing that is not in the strip at all is not invented into it.
{
  const snapshot = captureWorkspace({
    openFiles: openFiles('/repo/a.ts'),
    activePath: '/repo/gone.ts',
    selectedPath: null,
    scrollTop: 0
  });
  assert.deepEqual(snapshot.openPaths, ['/repo/a.ts']);
}

// A negative scroll offset is not a place anything can scroll to.
{
  const snapshot = captureWorkspace({
    openFiles: [],
    activePath: null,
    selectedPath: null,
    scrollTop: -40
  });
  assert.equal(snapshot.scrollTop, 0);
}

// What was written is what comes back.
{
  const storage = storageStub();
  const all = {
    'owned-1': captureWorkspace({
      openFiles: openFiles('/repo/a.ts'),
      activePath: '/repo/a.ts',
      selectedPath: '/repo/a.ts',
      scrollTop: 20
    })
  };
  assert.equal(writeWorkspaces(storage, all), true);
  assert.ok(storage.items.has(SESSION_WORKSPACES_STORAGE_KEY), 'stored under the shared key');
  assert.deepEqual(readWorkspaces(storage), all);
}

// A storage that refuses the write says so rather than throwing into the shell.
{
  const storage = storageStub({}, { refuseWrites: true });
  assert.equal(writeWorkspaces(storage, {}), false);
}

// Nothing stored, unreadable JSON, and a stored value that is not a map of
// sessions all come back as "no session has a workspace yet".
{
  assert.deepEqual(readWorkspaces(storageStub()), {});
  assert.deepEqual(
    readWorkspaces(storageStub({ [SESSION_WORKSPACES_STORAGE_KEY]: '{not json' })),
    {}
  );
  assert.deepEqual(readWorkspaces(storageStub({ [SESSION_WORKSPACES_STORAGE_KEY]: 'null' })), {});
  assert.deepEqual(readWorkspaces(storageStub({ [SESSION_WORKSPACES_STORAGE_KEY]: '[1,2]' })), {});
  assert.deepEqual(readWorkspaces(storageStub({ [SESSION_WORKSPACES_STORAGE_KEY]: '"text"' })), {});
}

// A storage that throws on read is a storage with nothing in it.
{
  const storage = {
    getItem() {
      throw new Error('storage is unavailable');
    },
    setItem() {},
    removeItem() {}
  };
  assert.deepEqual(readWorkspaces(storage), {});
}

// One rotten entry costs only itself; the sessions beside it still come back.
{
  const stored = JSON.stringify({
    'owned-junk': null,
    'owned-array': [],
    'owned-text': 'nonsense',
    'owned-good': {
      openPaths: ['/repo/a.ts'],
      activePath: '/repo/a.ts',
      selectedPath: null,
      scrollTop: 12
    }
  });
  const all = readWorkspaces(storageStub({ [SESSION_WORKSPACES_STORAGE_KEY]: stored }));
  assert.deepEqual(Object.keys(all), ['owned-good']);
}

// Fields of the wrong type inside an otherwise fine entry are replaced with the
// empty version of themselves, so a half-corrupt record still restores a session
// rather than breaking the switch.
{
  const stored = JSON.stringify({
    'owned-1': {
      openPaths: ['/repo/a.ts', 7, null, '/repo/b.ts'],
      activePath: 42,
      selectedPath: {},
      scrollTop: 'far down'
    }
  });
  const all = readWorkspaces(storageStub({ [SESSION_WORKSPACES_STORAGE_KEY]: stored }));
  assert.deepEqual(all['owned-1'], {
    openPaths: ['/repo/a.ts', '/repo/b.ts'],
    activePath: null,
    selectedPath: null,
    scrollTop: 0,
    // Written before the Diff tab was remembered at all, so it reads back as
    // "was not looking at one" — correct for that record, and the safe answer.
    diffPath: null,
    diffRoot: null
  });
}

// A stored strip longer than the cap is trimmed on the way in too, so an entry
// written by an older build cannot grow without limit.
{
  const paths = Array.from({ length: 30 }, (_, index) => `/repo/file-${index}.ts`);
  const stored = JSON.stringify({
    'owned-1': {
      openPaths: paths,
      activePath: '/repo/file-29.ts',
      selectedPath: null,
      scrollTop: 0
    }
  });
  const all = readWorkspaces(storageStub({ [SESSION_WORKSPACES_STORAGE_KEY]: stored }));
  assert.deepEqual(all['owned-1'].openPaths, paths.slice(18));
}

// Pruning keeps the sessions still on the rail and drops the rest, so removing a
// session takes its workspace with it.
{
  const snapshot = captureWorkspace({
    openFiles: openFiles('/repo/a.ts'),
    activePath: '/repo/a.ts',
    selectedPath: null,
    scrollTop: 0
  });
  const all = { 'owned-1': snapshot, 'owned-2': snapshot, 'owned-3': snapshot };
  const kept = pruneWorkspaces(all, ['owned-1', 'owned-3', 'owned-never-seen']);
  assert.deepEqual(Object.keys(kept), ['owned-1', 'owned-3']);
  assert.deepEqual(Object.keys(all), ['owned-1', 'owned-2', 'owned-3'], 'the input is untouched');
}

// An empty rail keeps no workspaces at all.
{
  const snapshot = captureWorkspace({
    openFiles: [],
    activePath: null,
    selectedPath: null,
    scrollTop: 0
  });
  assert.deepEqual(pruneWorkspaces({ 'owned-1': snapshot }, []), {});
}

// Which file the Diff tab should show for the session being restored, or null
// meaning clear it. Null is the answer in every way it can be wrong, because the
// Diff tab is one tab for the whole shell and the alternative is showing the
// project the reader just left.
{
  const diff = (diffPath, diffRoot) =>
    captureWorkspace({
      openFiles: [],
      activePath: null,
      selectedPath: null,
      scrollTop: 0,
      diffPath,
      diffRoot
    });

  // The same project: the diff is kept, because nothing changed underneath it.
  assert.equal(diffPathFor(diff('src/a.ts', '/repo'), '/repo'), 'src/a.ts');
  // A trailing slash is the same folder spelled differently, not a second one.
  assert.equal(diffPathFor(diff('src/a.ts', '/repo/'), '/repo'), 'src/a.ts');
  // Another project's file with a path that exists in most projects — this is
  // the case a path on its own could never have caught.
  assert.equal(diffPathFor(diff('src/index.ts', '/other'), '/repo'), null);
  // Nothing was being looked at, no record at all, and no project on screen.
  assert.equal(diffPathFor(diff(null, null), '/repo'), null);
  assert.equal(diffPathFor(null, '/repo'), null);
  assert.equal(diffPathFor(diff('src/a.ts', '/repo'), null), null);
  // A file with no folder is not enough to go on, so it is stored as no diff.
  assert.equal(diff('src/a.ts', null).diffPath, null);
}

// Conversation snapshots retain versioned runtime fields and fields introduced
// by a newer build without adding them to an older snapshot that never had them.
// Structured is the pinned mode for app-owned agents; raw remains valid for an
// external session, so the persisted union intentionally stays two-valued.
{
  const storage = storageStub({
    [SESSION_WORKSPACES_STORAGE_KEY]: JSON.stringify({
      current: {
        openPaths: [], activePath: null, selectedPath: null,
        scrollTop: 0, diffPath: null, diffRoot: null,
        conversation: {
          mode: 'structured', draft: 'keep', version: 1, generation: 4,
          owner: 'terminal', attachmentIds: ['attachment-a'], config: { future: 'value' },
          parentScrollTop: 20, childScrollTopById: { child: 30 }, sequence: 12,
          telemetry: { latencyMs: 7 }, futureField: { nested: true }
        }
      },
      legacy: {
        openPaths: [], activePath: null, selectedPath: null,
        scrollTop: 0, diffPath: null, diffRoot: null,
        conversation: { mode: 'raw', draft: 'old', unknownLegacyField: 'preserved' }
      }
    })
  });
  const restored = readWorkspaces(storage);
  assert.equal(restored.current.conversation.mode, 'structured', 'app-owned mode is pinned');
  assert.equal(restored.legacy.conversation.mode, 'raw', 'external raw mode persists');
  assert.equal(restored.current.conversation.futureField.nested, true);
  assert.equal(restored.current.conversation.config.future, 'value');
  assert.equal(restored.legacy.conversation.unknownLegacyField, 'preserved');
  assert.equal('draft' in restored.current.conversation, false, 'legacy drafts leave local storage');
  assert.equal('draft' in restored.legacy.conversation, false, 'legacy drafts are not restored');
  assert.equal('version' in restored.legacy.conversation, false);
  assert.equal('generation' in restored.legacy.conversation, false);
  assert.equal('owner' in restored.legacy.conversation, false);
}

console.log('sessionWorkspaces: all tests passed');
