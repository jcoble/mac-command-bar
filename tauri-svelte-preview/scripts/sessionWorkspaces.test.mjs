import assert from 'node:assert/strict';

import {
  captureWorkspace,
  OPEN_PATHS_CAP,
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

/** Editor entries with only the field the capture reads. */
function openFiles(...paths) {
  return paths.map((path) => ({ path }));
}

// A capture is exactly what was on screen: the strip in order, the file showing,
// the open folders as a list, the highlighted file, and the scroll offset.
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
    scrollTop: 120
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
    scrollTop: 0
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
    scrollTop: 0
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

console.log('sessionWorkspaces: all tests passed');
