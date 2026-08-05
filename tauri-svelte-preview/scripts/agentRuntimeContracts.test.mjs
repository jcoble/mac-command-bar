import assert from 'node:assert/strict';

import {
  adoptAgentSession,
  parseStoredOwnedSessions,
  reconcileOwnedSessions,
  serializeOwnedSessions
} from '../src/lib/shell/ownedSessions.ts';
import { configOptionPlacement } from '../src/lib/shell/conversation/conversationTypes.ts';
import {
  readWorkspaces,
  SESSION_WORKSPACES_STORAGE_KEY,
  writeWorkspaces
} from '../src/lib/shell/sessionWorkspaces.ts';

function storageStub(initial = {}) {
  const values = new Map(Object.entries(initial));
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
    removeItem: (key) => values.delete(key)
  };
}

const scanRecord = {
  provider: 'codex', id: 'native-a', title: 'Runtime fixture', description: null,
  model: null, projectPath: '/tmp/runtime-fixture', lastActivity: null,
  resumeCommands: ['codex resume native-a'], branchHint: null, taskId: null,
  pullRequestHint: null, sourceLabel: 'Codex', messageCount: 2,
  latestTurnPreview: 'Agent: retained'
};

// A saved session from before runtime ownership existed migrates to its live
// terminal and remains readable after the new representation is saved again.
{
  const current = {
    ...adoptAgentSession(scanRecord, () => 'owned-a'),
    ptySessionId: 'pty-a',
    state: 'background'
  };
  const {
    executionOwner, runtimeState, providerInstanceId, activeTurnId,
    capabilityRevision, lastRuntimeError, ...legacyOwned
  } = current;
  const workspace = {
    openPaths: ['/tmp/runtime-fixture/a.ts'], activePath: '/tmp/runtime-fixture/a.ts',
    expandedFolderIds: [], selectedPath: null, scrollTop: 0, diffPath: null, diffRoot: null,
    conversation: {
      mode: 'structured', draft: 'draft-a', attachmentIds: ['attachment-a'],
      unknownLegacyField: { retained: true }
    }
  };
  const fixture = {
    owned: legacyOwned,
    transcript: [{ itemId: 'message-a', text: 'retained transcript' }],
    workspace,
    attachments: [{ id: 'attachment-a', path: '/managed/a.png' }]
  };

  const migratedOwned = parseStoredOwnedSessions(JSON.stringify([fixture.owned]))[0];
  const reconciled = reconcileOwnedSessions([migratedOwned], [
    { sessionId: 'pty-a', exited: false }
  ]).owned[0];
  assert.equal(reconciled.executionOwner, 'terminal');
  assert.equal(reconciled.runtimeState, 'ready');
  assert.equal(reconciled.ptySessionId, 'pty-a');
  assert.equal(reconciled.nativeSessionId, 'native-a');
  assert.deepEqual(parseStoredOwnedSessions(serializeOwnedSessions([reconciled])), [reconciled]);

  const storage = storageStub({
    [SESSION_WORKSPACES_STORAGE_KEY]: JSON.stringify({ 'owned-a': fixture.workspace })
  });
  const restored = readWorkspaces(storage);
  assert.equal(restored['owned-a'].conversation.draft, 'draft-a');
  assert.deepEqual(restored['owned-a'].conversation.attachmentIds, ['attachment-a']);
  assert.equal(restored['owned-a'].conversation.unknownLegacyField.retained, true);
  assert.deepEqual(fixture.transcript, [{ itemId: 'message-a', text: 'retained transcript' }]);
  assert.deepEqual(fixture.attachments, [{ id: 'attachment-a', path: '/managed/a.png' }]);
  assert.equal(writeWorkspaces(storage, restored), true);
  assert.deepEqual(readWorkspaces(storage), restored);

  const failed = reconcileOwnedSessions([reconciled], []).owned[0];
  assert.equal(failed.executionOwner, 'stopped');
  assert.equal(failed.ptySessionId, 'pty-a');
  assert.equal(failed.nativeSessionId, 'native-a');
}

// Provider-defined capability categories and metadata survive unchanged while
// only their presentation location is interpreted.
{
  const capabilities = {
    revision: 7,
    provider: 'codex',
    implementation: { name: 'fixture', version: '1' },
    session: { list: true, load: true, resume: true, close: true, steering: true },
    prompt: { text: true, image: true, embeddedContext: true, resourceLinks: true },
    interaction: {
      permissions: true, structuredUserInput: true, toolTerminals: true,
      plans: true, tasks: true, subagents: true
    },
    configOptions: [{
      id: 'future', label: 'Future option', category: 'provider.future/category',
      value: { nested: ['kept', 3] }, providerMetadata: { provider_field: 'kept' }
    }],
    commands: []
  };
  const roundTrip = JSON.parse(JSON.stringify(capabilities));
  assert.deepEqual(roundTrip, capabilities);
  assert.equal(configOptionPlacement(roundTrip.configOptions[0].category), 'more-options');
}

// The two identities keep independent generations, leases, and saved fields.
{
  const storage = storageStub();
  const snapshots = {
    'owned-a': {
      openPaths: [], activePath: null, expandedFolderIds: [], selectedPath: null,
      scrollTop: 0, diffPath: null, diffRoot: null,
      conversation: {
        mode: 'structured', draft: 'a', generation: 3, owner: 'terminal', sequence: 9,
        writerLease: { ownedId: 'owned-a', generation: 3, owner: 'terminal' }
      }
    },
    'owned-b': {
      openPaths: [], activePath: null, expandedFolderIds: [], selectedPath: null,
      scrollTop: 0, diffPath: null, diffRoot: null,
      conversation: {
        mode: 'raw', draft: 'b', generation: 8, owner: 'stopped', sequence: 2,
        writerLease: { ownedId: 'owned-b', generation: 8, owner: 'none' }
      }
    }
  };
  assert.equal(writeWorkspaces(storage, snapshots), true);
  const restored = readWorkspaces(storage);
  assert.equal(restored['owned-a'].conversation.writerLease.ownedId, 'owned-a');
  assert.equal(restored['owned-b'].conversation.writerLease.ownedId, 'owned-b');
  assert.notEqual(
    restored['owned-a'].conversation.generation,
    restored['owned-b'].conversation.generation
  );
}

console.log('agent runtime contract tests passed');
