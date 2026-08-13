import assert from 'node:assert/strict';

import {
  captureWorkspace,
  diffPathFor,
  emptyRetainedWorkspaces,
  OPEN_PATHS_CAP,
  planWorkspaceRestore,
  pruneWorkspaces,
  readWorkspaces,
  RETAINED_WORKSPACES_CAP,
  retainTabs,
  SESSION_WORKSPACES_STORAGE_KEY,
  takeRetainedTabs,
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

/** Editor entries with only the field the capture reads. */
function openFiles(...paths) {
  return paths.map((path) => ({ path }));
}

// A capture is exactly what was on screen: the strip in order, the file showing,
// the open folders as a list, the highlighted file, the scroll offset, and the
// file the Diff tab was showing. A capture that says nothing about a diff still
// carries both diff fields, as nulls — the Diff tab is one tab for the whole
// shell, so "this session was not looking at one" is an answer it needs told.
{
  const snapshot = captureWorkspace({
    openFiles: openFiles('/repo/a.ts', '/repo/b.ts'),
    activePath: '/repo/b.ts',
    expandedFolderIds: new Set(['folder:src', 'folder:src/lib']),
    selectedPath: '/repo/b.ts',
    scrollTop: 120
  });
  assert.deepEqual(snapshot, {
    openPaths: ['/repo/a.ts', '/repo/b.ts'],
    activePath: '/repo/b.ts',
    expandedFolderIds: ['folder:src', 'folder:src/lib'],
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
    expandedFolderIds: new Set(),
    selectedPath: null,
    scrollTop: 0
  });
  assert.deepEqual(snapshot, {
    openPaths: [],
    activePath: null,
    expandedFolderIds: [],
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
    expandedFolderIds: new Set(),
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
    expandedFolderIds: new Set(),
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
    expandedFolderIds: new Set(),
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
    expandedFolderIds: new Set(),
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
      expandedFolderIds: new Set(['folder:src']),
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
      expandedFolderIds: ['folder:src'],
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
      expandedFolderIds: 'folder:src',
      selectedPath: {},
      scrollTop: 'far down'
    }
  });
  const all = readWorkspaces(storageStub({ [SESSION_WORKSPACES_STORAGE_KEY]: stored }));
  assert.deepEqual(all['owned-1'], {
    openPaths: ['/repo/a.ts', '/repo/b.ts'],
    activePath: null,
    expandedFolderIds: [],
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
      expandedFolderIds: [],
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
    expandedFolderIds: new Set(),
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
    expandedFolderIds: new Set(),
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
      expandedFolderIds: new Set(),
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

/**
 * A stand-in for the editor panel: the strip of open files, and a record of
 * every file it actually went to disk for.
 *
 * `open` is `EditorPanel.openPath` with everything but the read left out — a
 * file already in the strip keeps its place AND its contents, and only a file
 * whose contents are missing is read. That last rule is the whole point of the
 * retained tabs, so the stub has to keep it honest.
 */
function editorStub() {
  const editor = {
    openFiles: [],
    activePath: null,
    /** Every path read from disk, in order, including repeats. */
    reads: [],
    open(path) {
      let file = editor.openFiles.find((entry) => entry.path === path);
      if (!file) {
        file = { path, preview: null, loading: false, error: null };
        editor.openFiles = [...editor.openFiles, file];
      }
      editor.activePath = path;
      // `needsRead`, word for word: never read, not reading, and not failed.
      if (file.preview === null && !file.loading && file.error === null) {
        editor.reads.push(path);
        file.preview = { text: `the contents of ${path}` };
      }
    },
    /** Open a file and leave its read in flight, which is what switching
     * session in the second it takes to read a file does. */
    startRead(path) {
      editor.open(path);
      const file = editor.openFiles.find((entry) => entry.path === path);
      file.preview = null;
      file.loading = true;
    },
    /** What the page does when a session's tabs are not held in memory. */
    reset() {
      editor.openFiles = [];
      editor.activePath = null;
    },
    /** What the page does when they are. */
    restore(tabs) {
      editor.openFiles = tabs;
      editor.activePath = null;
    },
    paths: () => editor.openFiles.map((file) => file.path)
  };
  return editor;
}

/**
 * The page's own half of a session switch, wired exactly as `/next/+page.svelte`
 * wires it: leaving a session records what it had open and holds its tabs;
 * arriving at one takes its tabs back and asks the open-file bus for whatever is
 * left. Anything this stub gets wrong is a bug the real switch would have too.
 */
function shellStub({ cap = RETAINED_WORKSPACES_CAP } = {}) {
  const editor = editorStub();
  let retained = emptyRetainedWorkspaces();
  const stored = {};
  return {
    editor,
    retained: () => retained,
    leave(ownedId) {
      stored[ownedId] = captureWorkspace({
        openFiles: editor.openFiles,
        activePath: editor.activePath,
        expandedFolderIds: new Set(),
        selectedPath: null,
        scrollTop: 0
      });
      retained = retainTabs(retained, ownedId, editor.openFiles, cap);
    },
    arrive(ownedId) {
      const taken = takeRetainedTabs(retained, ownedId);
      retained = taken.retained;
      const plan = planWorkspaceRestore(stored[ownedId] ?? null, taken.tabs);
      if (plan.restoredTabs) editor.restore(plan.restoredTabs);
      else editor.reset();
      for (const path of plan.pathsToOpen) editor.open(path);
      if (plan.activePath) editor.open(plan.activePath);
    }
  };
}

// The measurement this whole lane exists for: going to another session and back
// puts the first session's files on screen again without reading one of them
// from disk a second time.
{
  const shell = shellStub();
  shell.editor.open('/repo/a/one.ts');
  shell.editor.open('/repo/a/two.ts');
  assert.deepEqual(shell.editor.reads, ['/repo/a/one.ts', '/repo/a/two.ts'], 'first open reads');

  shell.leave('session-a');
  shell.arrive('session-b');
  shell.editor.open('/repo/b/other.ts');

  shell.leave('session-b');
  shell.arrive('session-a');
  assert.deepEqual(
    shell.editor.paths(),
    ['/repo/a/one.ts', '/repo/a/two.ts'],
    'both of the first session’s tabs are back, and the other session’s is not'
  );
  assert.equal(shell.editor.activePath, '/repo/a/two.ts', 'the file that was showing is showing');
  assert.deepEqual(
    shell.editor.reads,
    ['/repo/a/one.ts', '/repo/a/two.ts', '/repo/b/other.ts'],
    'coming back read nothing: the same three reads as before the round trip'
  );
}

// Three sessions keep their tabs; the fourth to be left pushes out the one left
// longest ago, and only that one has to be read from disk again.
{
  const shell = shellStub();
  assert.equal(RETAINED_WORKSPACES_CAP, 3);
  for (const name of ['a', 'b', 'c', 'd']) {
    shell.arrive(`session-${name}`);
    shell.editor.open(`/repo/${name}.ts`);
    shell.leave(`session-${name}`);
  }
  assert.deepEqual(
    shell.retained().leastRecentFirst,
    ['session-b', 'session-c', 'session-d'],
    'three sessions held, the oldest let go of'
  );

  const readsBefore = shell.editor.reads.length;
  shell.arrive('session-d');
  assert.equal(shell.editor.reads.length, readsBefore, 'a held session reads nothing');
  shell.arrive('session-a');
  assert.deepEqual(
    shell.editor.reads.slice(readsBefore),
    ['/repo/a.ts'],
    'the session that was let go of reads its file again, exactly as it used to'
  );
}

// Going back to a session moves it to the front of the queue, so the sessions
// someone keeps returning to are the ones kept.
{
  const shell = shellStub();
  for (const name of ['a', 'b', 'c']) {
    shell.arrive(`session-${name}`);
    shell.editor.open(`/repo/${name}.ts`);
    shell.leave(`session-${name}`);
  }
  shell.arrive('session-a');
  shell.leave('session-a');
  shell.arrive('session-d');
  shell.editor.open('/repo/d.ts');
  shell.leave('session-d');
  assert.deepEqual(
    shell.retained().leastRecentFirst,
    ['session-c', 'session-a', 'session-d'],
    'the session nobody went back to is the one let go of'
  );
}

// A session opening its files for the first time behaves exactly as it did
// before any of this: every stored path is read, and the file that was showing
// ends up in front.
{
  const shell = shellStub();
  const stored = captureWorkspace({
    openFiles: openFiles('/repo/one.ts', '/repo/two.ts', '/repo/three.ts'),
    activePath: '/repo/two.ts',
    expandedFolderIds: new Set(),
    selectedPath: null,
    scrollTop: 0
  });
  const plan = planWorkspaceRestore(stored, null);
  assert.equal(plan.restoredTabs, null, 'nothing is held, so nothing is put straight back');
  assert.deepEqual(plan.pathsToOpen, ['/repo/one.ts', '/repo/two.ts', '/repo/three.ts']);
  assert.equal(plan.activePath, '/repo/two.ts');

  shell.editor.open('/repo/one.ts');
  shell.editor.open('/repo/two.ts');
  shell.leave('session-a');
  shell.arrive('session-never-seen');
  assert.deepEqual(shell.editor.paths(), [], 'a session with nothing stored opens nothing');
}

// A record that names a file the held tabs do not have — the strip was longer
// than the twelve a record keeps, or the record was written by an earlier run —
// still reads that one file, and only that one.
{
  const held = [{ path: '/repo/one.ts', preview: { text: 'held' } }];
  const stored = captureWorkspace({
    openFiles: openFiles('/repo/one.ts', '/repo/two.ts'),
    activePath: '/repo/two.ts',
    expandedFolderIds: new Set(),
    selectedPath: null,
    scrollTop: 0
  });
  const plan = planWorkspaceRestore(stored, held);
  assert.deepEqual(plan.restoredTabs, held);
  assert.deepEqual(plan.pathsToOpen, ['/repo/two.ts'], 'only the file not already in memory');
  assert.equal(plan.activePath, '/repo/two.ts');
}

// With no record of which file was showing, the last tab in the strip is the one
// left in front — the same answer opening them one after another used to give.
{
  const held = [{ path: '/repo/one.ts', preview: {} }, { path: '/repo/two.ts', preview: {} }];
  assert.equal(planWorkspaceRestore(null, held).activePath, '/repo/two.ts');
  assert.equal(planWorkspaceRestore(null, null).activePath, null, 'nothing open, nothing showing');
  assert.deepEqual(planWorkspaceRestore(null, null).pathsToOpen, []);
}

// A session left with an empty editor holds nothing, so its place goes to a
// session that has files worth keeping.
{
  const tabs = [{ path: '/repo/one.ts', preview: {} }];
  let retained = retainTabs(emptyRetainedWorkspaces(), 'session-a', tabs);
  assert.deepEqual(retained.leastRecentFirst, ['session-a']);
  retained = retainTabs(retained, 'session-a', []);
  assert.deepEqual(retained.leastRecentFirst, [], 'the empty session lets its place go');
  assert.deepEqual(retained.tabsByOwnedId, {});
  assert.equal(takeRetainedTabs(retained, 'session-a').tabs, null);
}

// A tab whose read had not finished, and one whose read failed, both come back
// as tabs nobody has read yet. Leaving a session throws away the answers its
// reads were about to give, so a tab held as "still loading" would come back
// waiting for something that never arrives.
{
  const stillReading = { path: '/repo/one.ts', preview: null, loading: true, error: null };
  const failed = {
    path: '/repo/two.ts',
    preview: null,
    loading: false,
    error: 'This file could not be read from here.'
  };
  const alreadyRead = { path: '/repo/three.ts', preview: { text: 'here' }, loading: false, error: null };
  const retained = retainTabs(emptyRetainedWorkspaces(), 'session-a', [
    stillReading,
    failed,
    alreadyRead
  ]);
  const held = retained.tabsByOwnedId['session-a'];
  assert.deepEqual(held[0], { path: '/repo/one.ts', preview: null, loading: false, error: null });
  assert.deepEqual(
    held[1],
    { path: '/repo/two.ts', preview: null, loading: false, error: null },
    'a failed read gets the second chance the switch always used to give it'
  );
  assert.equal(held[2], alreadyRead, 'a file already in memory is held exactly as it is');
  assert.equal(stillReading.loading, true, 'and the editor’s own tab is left alone');
}

// The same thing through a whole round trip: switching away while a file is
// still being read, then coming back, reads that one file and only that one.
{
  const shell = shellStub();
  shell.editor.open('/repo/a/one.ts');
  shell.editor.startRead('/repo/a/two.ts');
  shell.leave('session-a');
  shell.arrive('session-b');
  shell.leave('session-b');
  shell.arrive('session-a');
  assert.deepEqual(
    shell.editor.paths(),
    ['/repo/a/one.ts', '/repo/a/two.ts'],
    'both tabs are back, in the order they were opened'
  );
  assert.deepEqual(
    shell.editor.reads,
    ['/repo/a/one.ts', '/repo/a/two.ts', '/repo/a/two.ts'],
    'the file whose read never landed is read again; the one already in memory is not'
  );
}

// Holding tabs never changes the store handed in, so the page can keep the old
// one until the new one is assigned.
{
  const before = retainTabs(emptyRetainedWorkspaces(), 'session-a', [{ path: '/a.ts' }]);
  const after = retainTabs(before, 'session-b', [{ path: '/b.ts' }]);
  assert.deepEqual(before.leastRecentFirst, ['session-a'], 'the input is untouched');
  assert.deepEqual(after.leastRecentFirst, ['session-a', 'session-b']);
  assert.equal(takeRetainedTabs(before, 'session-b').tabs, null);
}

// Conversation snapshots retain versioned runtime fields and fields introduced
// by a newer build without adding them to an older snapshot that never had them.
// Structured is the pinned mode for app-owned agents; raw remains valid for an
// external session, so the persisted union intentionally stays two-valued.
{
  const storage = storageStub({
    [SESSION_WORKSPACES_STORAGE_KEY]: JSON.stringify({
      current: {
        openPaths: [], activePath: null, expandedFolderIds: [], selectedPath: null,
        scrollTop: 0, diffPath: null, diffRoot: null,
        conversation: {
          mode: 'structured', draft: 'keep', version: 1, generation: 4,
          owner: 'terminal', attachmentIds: ['attachment-a'], config: { future: 'value' },
          parentScrollTop: 20, childScrollTopById: { child: 30 }, sequence: 12,
          telemetry: { latencyMs: 7 }, futureField: { nested: true }
        }
      },
      legacy: {
        openPaths: [], activePath: null, expandedFolderIds: [], selectedPath: null,
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
