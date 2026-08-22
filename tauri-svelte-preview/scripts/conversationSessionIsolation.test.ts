import assert from 'node:assert/strict';

import {
  captureWorkspace,
  normalizeWorkspaceSnapshot
} from '../src/lib/shell/sessionWorkspaces.ts';

function sessionSnapshot(name, mode, panel) {
  return captureWorkspace({
    openFiles: [{ path: `/repo/${name}.cs` }],
    activePath: `/repo/${name}.cs`,
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
      lastSequence: 8,
      version: 1,
      generation: 2,
      owner: 'terminal',
      attachmentIds: [`attachment-${name}`],
      config: { future_category: name },
      parentScrollTop: name.length * 100,
      childScrollTopById: { [`child-${name}`]: name.length * 10 },
      sequence: 8,
      telemetry: { providerLatencyMs: name.length },
      futureWorkspaceField: { owner: name }
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

const sessions = {
  'owned-a': sessionSnapshot('alpha', 'structured', 'session'),
  'owned-b': sessionSnapshot('bravo', 'raw', 'browser'),
  'owned-c': sessionSnapshot('charlie', 'structured', 'diff')
};

const restored = Object.fromEntries(
  Object.entries(sessions).map(([ownedId, snapshot]) => [
    ownedId,
    normalizeWorkspaceSnapshot(snapshot)!
  ])
);

assert.equal('draft' in restored['owned-a'].conversation, false);
assert.equal(restored['owned-b'].conversation.mode, 'raw');
assert.equal(restored['owned-a'].conversation.selectedChildId, 'child-alpha');
assert.equal(restored['owned-b'].conversation.scrollTop, 500);
assert.equal(restored['owned-c'].browser.url, 'http://charlie.localhost:5177/');
assert.equal(restored['owned-a'].center.activePanelId, 'session');
assert.equal(restored['owned-b'].center.activePanelId, 'browser');
assert.equal(restored['owned-c'].center.activePanelId, 'diff');
assert.notEqual(restored['owned-a'].center.layout.marker, restored['owned-b'].center.layout.marker);
assert.deepEqual(restored['owned-a'].conversation.attachmentIds, ['attachment-alpha']);
assert.equal(restored['owned-b'].conversation.config.future_category, 'bravo');
assert.equal(restored['owned-a'].conversation.futureWorkspaceField.owner, 'alpha');
assert.notDeepEqual(
  restored['owned-a'].conversation.childScrollTopById,
  restored['owned-b'].conversation.childScrollTopById
);

// A pre-conversation snapshot migrates without inventing another session's UI.
const legacy = normalizeWorkspaceSnapshot({
  openPaths: ['/repo/legacy.cs'],
  activePath: '/repo/legacy.cs',
  selectedPath: null,
  scrollTop: 0,
  diffPath: null,
  diffRoot: null
});
assert.ok(legacy);
assert.equal(legacy.conversation, undefined);
assert.equal(legacy.browser, undefined);
assert.equal(legacy.center, undefined);

console.log('conversation session isolation tests passed');
