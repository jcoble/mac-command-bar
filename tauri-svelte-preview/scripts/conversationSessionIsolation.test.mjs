import assert from 'node:assert/strict';

import {
  captureWorkspace,
  readWorkspaces,
  writeWorkspaces
} from '../src/lib/shell/sessionWorkspaces.ts';

function storageStub() {
  const values = new Map();
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
    removeItem: (key) => values.delete(key)
  };
}

function sessionSnapshot(name, mode, panel) {
  return captureWorkspace({
    openFiles: [{ path: `/repo/${name}.cs` }],
    activePath: `/repo/${name}.cs`,
    expandedFolderIds: new Set([`folder:${name}`]),
    selectedPath: `/repo/${name}.cs`,
    scrollTop: name.length,
    diffPath: `${name}.cs`,
    diffRoot: '/repo',
    conversation: {
      mode,
      draft: `draft-${name}`,
      selectedChildId: `child-${name}`,
      scrollTop: name.length * 100,
      providerGeneration: 2,
      lastSequence: 8
    },
    browser: {
      url: `http://${name}.localhost:5177/`,
      inputUrl: `${name}.localhost:5177`,
      activated: true
    },
    center: {
      activePanelId: panel,
      layout: { panels: { session: {}, editor: {}, browser: {}, diff: {} }, marker: name }
    }
  });
}

const storage = storageStub();
const sessions = {
  'owned-a': sessionSnapshot('alpha', 'structured', 'session'),
  'owned-b': sessionSnapshot('bravo', 'raw', 'browser'),
  'owned-c': sessionSnapshot('charlie', 'structured', 'diff')
};

assert.equal(writeWorkspaces(storage, sessions), true);
const restored = readWorkspaces(storage);

assert.equal(restored['owned-a'].conversation.draft, 'draft-alpha');
assert.equal(restored['owned-b'].conversation.mode, 'raw');
assert.equal(restored['owned-a'].conversation.selectedChildId, 'child-alpha');
assert.equal(restored['owned-b'].conversation.scrollTop, 500);
assert.equal(restored['owned-c'].browser.url, 'http://charlie.localhost:5177/');
assert.equal(restored['owned-a'].center.activePanelId, 'session');
assert.equal(restored['owned-b'].center.activePanelId, 'browser');
assert.equal(restored['owned-c'].center.activePanelId, 'diff');
assert.notEqual(restored['owned-a'].center.layout.marker, restored['owned-b'].center.layout.marker);

// A pre-conversation snapshot migrates without inventing another session's UI.
storage.setItem(
  'mac-command-bar.next.session-workspaces',
  JSON.stringify({
    legacy: {
      openPaths: ['/repo/legacy.cs'],
      activePath: '/repo/legacy.cs',
      expandedFolderIds: [],
      selectedPath: null,
      scrollTop: 0,
      diffPath: null,
      diffRoot: null
    }
  })
);
const legacy = readWorkspaces(storage).legacy;
assert.equal(legacy.conversation, undefined);
assert.equal(legacy.browser, undefined);
assert.equal(legacy.center, undefined);

console.log('conversation session isolation tests passed');
