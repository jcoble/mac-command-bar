import assert from 'node:assert/strict';
import { readFileSync, rmSync, writeFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileModule } from 'svelte/compiler';
import { get } from 'svelte/store';
import { shouldClearConversationSending } from '../src/lib/shell/conversation/conversationReducer.ts';
import { sessionPresenceHistory } from '../src/lib/shell/conversation/sessionPresence.ts';
import { mergeAgentItem } from '../src/lib/shell/conversation/conversationTimeline.ts';

const storePath = fileURLToPath(
  new URL('../src/lib/shell/conversation/conversationStore.svelte.ts', import.meta.url)
);
const outputPath = fileURLToPath(
  new URL('../src/lib/shell/conversation/.conversationStore.test.mjs', import.meta.url)
);
// The store imports the runes module that holds the resource counters. Node
// cannot run that file as written, so it is compiled beside it and the store's
// import is pointed at the compiled copy.
const diagnosticsPath = fileURLToPath(
  new URL('../src/lib/shell/resourceDiagnostics.svelte.ts', import.meta.url)
);
const diagnosticsOutputPath = fileURLToPath(
  new URL('../src/lib/shell/.resourceDiagnostics.test.mjs', import.meta.url)
);

const surfaceSource = readFileSync(
  new URL('../src/lib/shell/components/ConversationSurface.svelte', import.meta.url),
  'utf8'
);
// Node has no bundler, so the build-time flag both modules read is compiled
// out as false, which is what the shipped app sees.
function compileForTest(modulePath: string, filename: string): string {
  const stripped = stripTypeScriptTypes(readFileSync(modulePath, 'utf8'), { mode: 'strip' });
  const module = compileModule(stripped, { generate: 'client', filename });
  return module.js.code.replaceAll('import.meta.env', '({ DEV: false })');
}

writeFileSync(
  outputPath,
  compileForTest(storePath, 'conversationStore.svelte.js')
    .replace('../resourceDiagnostics.svelte.ts', '../.resourceDiagnostics.test.mjs')
);
writeFileSync(
  diagnosticsOutputPath,
  compileForTest(diagnosticsPath, 'resourceDiagnostics.svelte.js')
);

let store;
try {
  store = await import(`${outputPath}?test=${Date.now()}`);
} finally {
  rmSync(outputPath, { force: true });
  rmSync(diagnosticsOutputPath, { force: true });
}

const a = store.ensureConversationSession('owned-a', 'codex');
const b = store.ensureConversationSession('owned-b', 'claude');
assert.notEqual(a, b);

// The first config read can finish before a fresh session connects. Its
// connection transition must trigger another read without a row re-selection.
assert.match(
  surfaceSource,
  /const key = `\$\{ownedId\}:\$\{generation\}:\$\{conversation\.connectionState\}`;/
);

store.setConversationDraft('owned-a', 'Message A');
store.setConversationDraft('owned-b', 'Message B');
store.setConversationMode('owned-b', 'raw');
assert.equal(store.getConversationSession('owned-a').draft, 'Message A');
assert.equal(store.getConversationSession('owned-a').mode, 'structured');
assert.equal(store.getConversationSession('owned-b').draft, 'Message B');
assert.equal(store.getConversationSession('owned-b').mode, 'raw');

store.applyConversationTranscript('owned-a', 'codex', {
  messages: [{ itemId: 'parent-a', role: 'assistant', text: 'Parent A', timestampMs: 1 }],
  metadata: {
    model: 'gpt-5.6-sol', effort: 'medium', approvalPolicy: 'never',
    usedTokens: 1000, contextWindow: 10000
  },
  children: [{
    childId: 'child-a', parentId: 'thread-a', provider: 'codex', label: 'Reviewer',
    state: 'active', updatedAtMs: 2
  }]
});
store.setConversationAttachments('owned-a', [{
  id: 'image-a', name: 'a.png', mimeType: 'image/png', path: '/managed/a.png', previewUrl: 'blob:a'
}]);
store.setConversationSelectedChild('owned-a', 'child-a');
store.setConversationScrollTop('owned-a', 240);
store.applyChildConversationTranscript('owned-a', 'child-a', [
  { itemId: 'child-message', role: 'assistant', text: 'Child A', timestampMs: 3 }
]);
assert.equal(store.getConversationSession('owned-a').metadata.model, 'gpt-5.6-sol');
assert.equal(store.getConversationSession('owned-a').children[0].label, 'Reviewer');
assert.equal(store.getConversationSession('owned-a').attachments[0].path, '/managed/a.png');
assert.equal(store.getConversationSession('owned-a').childTimeline[0].text, 'Child A');
assert.equal(store.getConversationSession('owned-a').scrollTop, 240);
assert.equal(store.getConversationSession('owned-b').metadata.model, null);
assert.equal(store.getConversationSession('owned-b').children.length, 0);
assert.equal(store.getConversationSession('owned-b').attachments.length, 0);

store.setConversationConnection({
  ownedId: 'owned-b', provider: 'claude', generation: 2, state: 'connecting'
});
assert.equal(store.setConversationWriterLeaseTransition({
  ownedId: 'owned-b', generation: 2, from: 'none', to: 'terminal', state: 'committed'
}), true);
assert.equal(store.getConversationSession('owned-b').executionOwner, 'terminal');
assert.equal(store.getConversationSession('owned-b').writerLease.owner, 'terminal');
assert.equal(store.getConversationSession('owned-a').executionOwner, 'stopped');
assert.equal(store.setConversationWriterLeaseTransition({
  ownedId: 'owned-b', generation: 1, from: 'terminal', to: 'structured', state: 'committed'
}), false);

store.applyAgentConversationEvent({
  ownedId: 'owned-a',
  provider: 'codex',
  generation: 1,
  sequence: 1,
  timestampMs: 100,
  payload: { kind: 'assistantMessage', itemId: 'a-1', text: 'Only A', completed: true }
});
assert.equal(store.getConversationSession('owned-a').timeline.length, 2);
assert.equal(store.getConversationSession('owned-b').timeline.length, 0);

// Live subagent updates create one durable row, then update that same row
// without losing the spawn label when a completion update omits it.
{
  const ownedId = 'owned-live-child';
  store.ensureConversationSession(ownedId, 'codex');
  store.applyAgentConversationEvent({
    ownedId, provider: 'codex', generation: 1, sequence: 1, timestampMs: 150,
    payload: {
      kind: 'childUpdate', childId: 'child-live', parentToolCallId: 'parent-tool',
      label: 'Review the change', state: 'running', latestActivity: 'Review the change'
    }
  });
  store.applyAgentConversationEvent({
    ownedId, provider: 'codex', generation: 1, sequence: 2, timestampMs: 160,
    payload: {
      kind: 'childUpdate', childId: 'child-live', parentToolCallId: 'parent-tool',
      state: 'finished', latestActivity: 'Review complete'
    }
  });
  assert.deepEqual(store.getConversationSession(ownedId).children, [{
    childId: 'child-live', parentId: 'parent-tool', parentToolCallId: 'parent-tool',
    provider: 'codex', label: 'Review the change', state: 'finished',
    latestActivity: 'Review complete', updatedAtMs: 160
  }]);

  // Canonical transcript projections use the same merge path and may add a
  // different child without replacing finished rows from this session.
  store.applyAgentConversationEvent({
    ownedId, provider: 'codex', generation: 1, sequence: 3, timestampMs: 170,
    payload: {
      kind: 'terminalProjection', eventType: 'children.updated',
      providerInstanceId: 'fixture', timestampMs: 170, nativeSessionId: 'thread-live',
      itemId: null, providerMetadata: {}, rawFrameReference: { id: 'raw-1', redacted: true },
      payload: { children: [{
        childId: 'child-two', parentId: 'thread-live', provider: 'codex',
        label: 'Inspect tests', state: 'historical', updatedAtMs: 165,
        latestActivity: 'Inspecting tests'
      }] }
    }
  });
  assert.equal(store.getConversationSession(ownedId).children.length, 2);
  assert.equal(store.getConversationSession(ownedId).children[0].state, 'finished');
  assert.equal(store.getConversationSession(ownedId).children[1].latestActivity, 'Inspecting tests');
}

// A live turn start keeps the composer busy; only a terminal turn clears it.
store.setConversationSending('owned-a', true);
const startedTurn = {
  ownedId: 'owned-a',
  provider: 'codex',
  generation: 1,
  sequence: 2,
  timestampMs: 130,
  payload: { kind: 'turn', turnId: 'turn-live', state: 'started' }
};
store.applyAgentConversationEvent(startedTurn);
if (shouldClearConversationSending(startedTurn)) {
  store.setConversationSending('owned-a', false);
}
assert.equal(store.getConversationSession('owned-a').sending, true);

const completedTurn = {
  ...startedTurn,
  sequence: 3,
  timestampMs: 140,
  payload: { kind: 'turn', turnId: 'turn-live', state: 'completed' }
};
store.applyAgentConversationEvent(completedTurn);
if (shouldClearConversationSending(completedTurn)) {
  store.setConversationSending('owned-a', false);
}
assert.equal(store.getConversationSession('owned-a').sending, false);

// A native snapshot repairs a missed-event gap and restores every message.
store.applyAgentConversationSnapshot({
  connection: {
    ownedId: 'owned-a',
    provider: 'codex',
    generation: 1,
    state: 'connected',
    nativeSessionId: 'thread-a'
  },
  suspended: true,
  lastSequence: 3,
  events: [
    {
      ownedId: 'owned-a', provider: 'codex', generation: 1, sequence: 1, timestampMs: 100,
      payload: { kind: 'connection', state: 'connected', nativeSessionId: 'thread-a' }
    },
    {
      ownedId: 'owned-a', provider: 'codex', generation: 1, sequence: 2, timestampMs: 110,
      payload: { kind: 'userMessage', itemId: 'user-1', text: 'Visible question', completed: true }
    },
    {
      ownedId: 'owned-a', provider: 'codex', generation: 1, sequence: 3, timestampMs: 120,
      payload: { kind: 'assistantMessage', itemId: 'assistant-1', text: 'Visible answer', completed: true }
    }
  ]
});
assert.equal(store.getConversationSession('owned-a').suspended, true);
assert.deepEqual(
  store.getConversationSession('owned-a').timeline.map((item) => item.text),
  ['Visible question', 'Visible answer']
);
assert.equal(store.getConversationSession('owned-a').desynchronized, false);

// A session picked up from a past Claude or Codex transcript stores every event
// wrapped in a terminal projection, with the real event one level further down.
// The live path unwraps that wrapper before it types the item; snapshot restore
// read the wrapper itself, so each imported row arrived as an empty `unknown`
// and a resumed session painted nothing at all.
{
  const importedEvent = (sequence, itemId, type, text) => ({
    ownedId: 'owned-imported',
    provider: 'claude',
    generation: 0,
    sequence,
    timestampMs: 300 + sequence,
    payload: {
      kind: 'terminalProjection',
      eventType: 'item.completed',
      providerInstanceId: 'imported-transcript:native-imported',
      timestampMs: null,
      nativeSessionId: 'native-imported',
      itemId,
      payload: {
        historical: true,
        item: { id: itemId, type, content: [{ channel: 'assistant', text }] }
      }
    }
  });

  store.applyAgentConversationSnapshot({
    connection: {
      ownedId: 'owned-imported',
      provider: 'claude',
      generation: 0,
      state: 'connected',
      nativeSessionId: 'native-imported'
    },
    lastSequence: 2,
    events: [
      importedEvent(1, 'imported-user-1', 'user-message', 'Imported question'),
      importedEvent(2, 'imported-assistant-1', 'assistant-message', 'Imported answer')
    ]
  });

  const imported = store.getConversationSession('owned-imported');
  assert.deepEqual(
    imported.agentItems.map((item) => item.type),
    ['user-message', 'assistant-message'],
    'restore must type an imported transcript the way the live path does'
  );
  assert.deepEqual(
    imported.agentItems.map((item) => item.content.map((part) => part.text).join('')),
    ['Imported question', 'Imported answer'],
    'a restored imported transcript keeps its text instead of becoming empty unknown items'
  );
}

// A send failure belongs to the session it happened in. The conversation
// surface is mounted once for the whole shell, so a failure held there was
// painted under every session and outlived the send that fixed it.
{
  store.setConversationSendError('owned-a', 'This conversation is closed, so the message was not sent');
  assert.equal(
    store.getConversationSession('owned-a').sendError,
    'This conversation is closed, so the message was not sent'
  );
  assert.equal(store.getConversationSession('owned-b').sendError, '', 'a failure in one session never surfaces in another');
  store.setConversationSendError('owned-a', '');
  assert.equal(store.getConversationSession('owned-a').sendError, '', 'the next send clears the session it belongs to');
}

// An attachment failure belongs to its session for the same reason, and the
// notice carries a dismiss: held on the surface it could not be cleared at all
// and followed the reader into every other conversation.
{
  store.setConversationAttachmentError('owned-a', 'That file link could not be opened.');
  assert.equal(
    store.getConversationSession('owned-a').attachmentError,
    'That file link could not be opened.'
  );
  assert.equal(
    store.getConversationSession('owned-b').attachmentError,
    '',
    'an attachment failure in one session never surfaces in another'
  );
  store.setConversationAttachmentError('owned-a', '');
  assert.equal(
    store.getConversationSession('owned-a').attachmentError,
    '',
    'dismissing the notice clears the session it belongs to'
  );
}

// A file link that lands outside the workspace opens read-only rather than
// leaving a notice: reading a file the session does not own is safe, and
// refusing it left no way to see what the link pointed at.
assert.match(surfaceSource, /readOnly: outside/);
assert.doesNotMatch(surfaceSource, /outside the active workspace/);

// Re-ensuring a suspended session opens a new adapter incarnation that has no
// events of its own yet, so the snapshot's history all belongs to the previous
// one. The replayed transcript must survive and the session must still hold the
// connection's generation, or the next send is addressed to an incarnation the
// backend has already replaced.
{
  store.applyAgentConversationSnapshot({
    connection: {
      ownedId: 'owned-reensured', provider: 'codex', generation: 2,
      state: 'connecting', nativeSessionId: 'thread-reensured'
    },
    lastSequence: 2,
    events: [
      {
        ownedId: 'owned-reensured', provider: 'codex', generation: 1, sequence: 1, timestampMs: 200,
        payload: { kind: 'userMessage', itemId: 'user-1', text: 'Earlier question', completed: true }
      },
      {
        ownedId: 'owned-reensured', provider: 'codex', generation: 1, sequence: 2, timestampMs: 210,
        payload: { kind: 'assistantMessage', itemId: 'assistant-1', text: 'Earlier answer', completed: true }
      }
    ]
  });
  assert.equal(store.getConversationSession('owned-reensured').generation, 2);
  assert.equal(store.getConversationSession('owned-reensured').writerLease.generation, 2);
  assert.deepEqual(
    store.getConversationSession('owned-reensured').timeline.map((item) => item.text),
    ['Earlier question', 'Earlier answer']
  );
}

// Snapshot replay can repeat completed history after unrelated journal events.
// The first repeated stable item identifies the contiguous replay block, so
// both that item and the following replay-only items render zero extra rows.
{
  store.applyAgentConversationSnapshot({
    connection: {
      ownedId: 'owned-nonconsecutive-replay', provider: a.provider, generation: 1,
      state: 'connected', nativeSessionId: 'thread-nonconsecutive-replay'
    },
    lastSequence: 6,
    events: [
      {
        ownedId: 'owned-nonconsecutive-replay', provider: a.provider, generation: 1,
        sequence: 1, timestampMs: 600,
        payload: { kind: 'assistantMessage', itemId: 'complete-item', text: 'Complete answer', completed: true }
      },
      {
        ownedId: 'owned-nonconsecutive-replay', provider: a.provider, generation: 1,
        sequence: 2, timestampMs: 610,
        payload: { kind: 'turn', turnId: 'stored-turn', state: 'completed' }
      },
      {
        ownedId: 'owned-nonconsecutive-replay', provider: a.provider, generation: 1,
        sequence: 3, timestampMs: 620,
        payload: { kind: 'usage', usedTokens: 10 }
      },
      {
        ownedId: 'owned-nonconsecutive-replay', provider: a.provider, generation: 1,
        sequence: 4, timestampMs: 630,
        payload: { kind: 'assistantMessage', itemId: 'complete-item', text: 'Complete answer', completed: true }
      },
      {
        ownedId: 'owned-nonconsecutive-replay', provider: a.provider, generation: 1,
        sequence: 5, timestampMs: 640,
        payload: { kind: 'assistantMessage', itemId: 'replayed-next-item', text: 'Replay-only answer', completed: true }
      },
      {
        ownedId: 'owned-nonconsecutive-replay', provider: a.provider, generation: 1,
        sequence: 6, timestampMs: 650,
        payload: { kind: 'turn', turnId: 'replay-finished', state: 'completed' }
      }
    ]
  });
  const assistantRows = store.getConversationSession('owned-nonconsecutive-replay').timeline
    .filter((item) => item.kind === 'assistant');
  assert.equal(assistantRows.length, 1);
  assert.equal(assistantRows[0].text, 'Complete answer');
  assert.equal(store.getConversationSession('owned-nonconsecutive-replay').agentItems.length, 1);
  assert.equal(store.getConversationSession('owned-nonconsecutive-replay').agentItems[0].content[0].text, 'Complete answer');
}

// Live deltas mutate only the touched leaves and advance a cheap numeric revision.
{
  const ownedId = 'owned-live-identity';
  store.applyAgentConversationSnapshot({
    connection: { ownedId, provider: a.provider, generation: 1, state: 'connected' },
    lastSequence: 2,
    events: [
      {
        ownedId, provider: a.provider, generation: 1, sequence: 1, timestampMs: 700,
        payload: { kind: 'assistantMessage', itemId: 'settled-item', text: 'Settled', completed: true }
      },
      {
        ownedId, provider: a.provider, generation: 1, sequence: 2, timestampMs: 710,
        payload: { kind: 'assistantDelta', itemId: 'stream-item', delta: 'First' }
      }
    ]
  });
  const before = store.getConversationSession(ownedId);
  const metadata = before.metadata;
  const agentItems = before.agentItems;
  const settledItem = before.agentItems[0];
  const revision = before.timelineRevision;
  store.applyAgentConversationEvent({
    ownedId, provider: a.provider, generation: 1, sequence: 3, timestampMs: 720,
    payload: { kind: 'assistantDelta', itemId: 'stream-item', delta: ' second' }
  });
  const after = store.getConversationSession(ownedId);
  assert.equal(after, before);
  assert.equal(after.metadata, metadata);
  assert.equal(after.agentItems, agentItems);
  assert.equal(after.agentItems[0], settledItem);
  assert.equal(after.timelineRevision, revision + 1);
}

// Completing an already-complete stable item is an idempotent merge.
{
  const complete = {
    id: 'complete-once', type: 'assistant-message',
    content: [{ channel: 'assistant', text: 'One copy' }],
    providerMetadata: { completed: true }
  };
  const once = mergeAgentItem([], complete, false);
  const twice = mergeAgentItem(once, complete, false);
  assert.equal(twice.length, 1);
  assert.equal(twice[0], once[0]);
}

// A bounded tail snapshot begins after omitted journal rows without marking
// itself desynchronized, and replaying stored turn events cannot mutate the
// live-only presence history.
{
  const presenceBefore = get(sessionPresenceHistory)['owned-windowed'];
  store.applyAgentConversationSnapshot({
    connection: {
      ownedId: 'owned-windowed', provider: 'codex', generation: 1,
      state: 'connected', nativeSessionId: 'thread-windowed'
    },
    suspended: true,
    lastSequence: 502,
    events: [
      {
        ownedId: 'owned-windowed', provider: 'codex', generation: 1,
        sequence: 501, timestampMs: 500,
        payload: { kind: 'userMessage', itemId: 'window-user', text: 'Recent question', completed: true }
      },
      {
        ownedId: 'owned-windowed', provider: 'codex', generation: 1,
        sequence: 502, timestampMs: 510,
        payload: { kind: 'turn', turnId: 'stored-turn', state: 'completed' }
      }
    ]
  });
  assert.equal(store.getConversationSession('owned-windowed').desynchronized, false);
  assert.equal(store.getConversationSession('owned-windowed').timeline[0].text, 'Recent question');
  assert.equal(get(sessionPresenceHistory)['owned-windowed'], presenceBefore);
}

// Re-applying the same snapshot is a read repair, not another stream of deltas.
// This is the session-row re-entry regression: the legacy timeline was rebuilt,
// but typed assistant items were retained and received the same delta again.
{
  const replayedSnapshot = {
    connection: {
      ownedId: 'owned-replayed',
      provider: 'codex',
      generation: 1,
      state: 'connected',
      nativeSessionId: 'thread-replayed'
    },
    lastSequence: 2,
    events: [
      {
        ownedId: 'owned-replayed', provider: 'codex', generation: 1, sequence: 1, timestampMs: 200,
        payload: { kind: 'connection', state: 'connected', nativeSessionId: 'thread-replayed' }
      },
      {
        ownedId: 'owned-replayed', provider: 'codex', generation: 1, sequence: 2, timestampMs: 210,
        payload: { kind: 'assistantDelta', itemId: 'assistant-replayed', delta: 'One answer' }
      }
    ]
  };
  store.applyAgentConversationSnapshot(replayedSnapshot);
  store.applyAgentConversationSnapshot(replayedSnapshot);
  assert.equal(store.getConversationSession('owned-replayed').timeline[0].text, 'One answer');
  assert.equal(store.getConversationSession('owned-replayed').agentItems[0].content[0].text, 'One answer');
  assert.equal(store.getConversationSession('owned-replayed').recentEvents.length, 2);
}

// Repair journals poisoned by historical replay: identical consecutive full
// assistant chunks for the same item collapse while a snapshot is rebuilt.
{
  store.applyAgentConversationSnapshot({
    connection: {
      ownedId: 'owned-poisoned',
      provider: 'codex',
      generation: 1,
      state: 'connected',
      nativeSessionId: 'thread-poisoned'
    },
    lastSequence: 3,
    events: [
      {
        ownedId: 'owned-poisoned', provider: 'codex', generation: 1, sequence: 1, timestampMs: 300,
        payload: { kind: 'connection', state: 'connected', nativeSessionId: 'thread-poisoned' }
      },
      {
        ownedId: 'owned-poisoned', provider: 'codex', generation: 1, sequence: 2, timestampMs: 310,
        payload: { kind: 'assistantDelta', itemId: 'assistant-poisoned', delta: 'Recovered answer' }
      },
      {
        ownedId: 'owned-poisoned', provider: 'codex', generation: 1, sequence: 3, timestampMs: 320,
        payload: { kind: 'assistantDelta', itemId: 'assistant-poisoned', delta: 'Recovered answer' }
      }
    ]
  });
  assert.equal(store.getConversationSession('owned-poisoned').timeline[0].text, 'Recovered answer');
  assert.equal(store.getConversationSession('owned-poisoned').agentItems[0].content[0].text, 'Recovered answer');
}

// A backend-sized snapshot is rebuilt once without losing the bounded event
// inspector or duplicating streamed assistant content on a second read.
{
  const ownedId = 'owned-large-snapshot';
  const events = Array.from({ length: 2_000 }, (_, index) => {
    const turn = Math.floor(index / 10);
    const offset = index % 10;
    const payload = offset === 0
      ? { kind: 'userMessage', itemId: `user-${turn}`, text: `Question ${turn}`, completed: true }
      : offset === 9
        ? { kind: 'assistantMessage', itemId: `assistant-${turn}`, text: `Answer ${turn}`, completed: true }
        : { kind: 'assistantDelta', itemId: `assistant-${turn}`, delta: `chunk ${offset} ` };
    return {
      ownedId,
      provider: 'codex',
      generation: 1,
      sequence: index + 1,
      timestampMs: 1_000 + index,
      payload
    };
  });
  const snapshot = {
    connection: {
      ownedId, provider: 'codex', generation: 1,
      state: 'connected', nativeSessionId: 'thread-large'
    },
    lastSequence: events.length,
    events
  };
  store.applyAgentConversationSnapshot(snapshot);
  store.applyAgentConversationSnapshot(snapshot);
  const restored = store.getConversationSession(ownedId);
  assert.equal(restored.timeline.length, 400);
  assert.equal(restored.agentItems.length, 400);
  assert.equal(restored.recentEvents.length, 200);
  assert.equal(restored.agentItems.at(-1).content[0].text, 'Answer 199');
}

// A stale snapshot cannot replace a newer generation.
store.setConversationConnection({
  ownedId: 'owned-a', provider: 'codex', generation: 3, state: 'reconnecting'
});
assert.equal(store.getConversationSession('owned-a').suspended, false, 'ensuring the conversation clears suspension');
store.applyAgentConversationSnapshot({
  connection: { ownedId: 'owned-a', provider: 'codex', generation: 2, state: 'connected' },
  lastSequence: 0,
  events: []
});
assert.equal(store.getConversationSession('owned-a').generation, 3);
assert.equal(store.getConversationSession('owned-a').connectionState, 'reconnecting');

// A same-generation snapshot cannot erase a newer live event.
{
  const ownedId = 'owned-stale-snapshot';
  const staleSnapshot = {
    connection: { ownedId, provider: 'codex', generation: 1, state: 'connected' },
    lastSequence: 40,
    events: [{
      ownedId, provider: 'codex', generation: 1, sequence: 40, timestampMs: 400,
      payload: { kind: 'assistantMessage', itemId: 'snapshot-40', text: 'Older answer', completed: true }
    }]
  };
  store.applyAgentConversationSnapshot(staleSnapshot);
  store.applyAgentConversationEvent({
    ownedId, provider: 'codex', generation: 1, sequence: 41, timestampMs: 410,
    payload: { kind: 'assistantMessage', itemId: 'live-41', text: 'Newest live answer', completed: true }
  });
  store.applyAgentConversationSnapshot(staleSnapshot);
  const current = store.getConversationSession(ownedId);
  assert.equal(current.lastSequence, 41);
  assert.equal(current.timeline.some((item) => item.itemId === 'live-41'), true);
}

// A stale connection response cannot move an existing session backwards.
{
  const ownedId = 'owned-stale-connection';
  store.setConversationConnection({ ownedId, provider: 'codex', generation: 2, state: 'connected' });
  store.setConversationConnection({ ownedId, provider: 'codex', generation: 1, state: 'connecting' });
  const current = store.getConversationSession(ownedId);
  assert.equal(current.generation, 2);
  assert.equal(current.connectionState, 'connected');
}

// An event from another provider cannot replace the owned session workspace.
{
  const ownedId = 'owned-wrong-provider';
  store.ensureConversationSession(ownedId, 'codex');
  store.setConversationDraft(ownedId, 'Keep this draft');
  store.applyAgentConversationEvent({
    ownedId, provider: 'claude', generation: 1, sequence: 1, timestampMs: 500,
    payload: { kind: 'assistantMessage', itemId: 'wrong-provider', text: 'Wrong provider', completed: true }
  });
  const current = store.getConversationSession(ownedId);
  assert.equal(current.provider, 'codex');
  assert.equal(current.draft, 'Keep this draft');
  assert.equal(current.timeline.length, 0);
}

// Capabilities belong only to the generation that requested them.
{
  const ownedId = 'owned-capability-generation';
  const capabilities = {
    revision: 1,
    provider: 'codex',
    implementation: { name: 'fixture', version: '1' },
    session: { list: true, load: true, resume: true, close: true, steering: true },
    prompt: { text: true, image: false, embeddedContext: true, resourceLinks: true },
    interaction: {
      permissions: true, structuredUserInput: true, toolTerminals: true,
      plans: true, tasks: true, subagents: true
    },
    configOptions: [],
    commands: []
  };
  store.setConversationConnection({ ownedId, provider: 'codex', generation: 1, state: 'connected' });
  store.setConversationCapabilities(ownedId, 1, capabilities);
  assert.equal(store.getConversationSession(ownedId).capabilitiesGeneration, 1);
  store.setConversationConnection({ ownedId, provider: 'codex', generation: 2, state: 'connected' });
  assert.equal(store.getConversationSession(ownedId).capabilities, null);
  assert.equal(store.getConversationSession(ownedId).capabilitiesGeneration, 0);
  store.setConversationCapabilities(ownedId, 1, capabilities);
  assert.equal(store.getConversationSession(ownedId).capabilities, null);
  assert.equal(store.getConversationSession(ownedId).capabilitiesGeneration, 0);
}

// Terminal transcript items arrive inside the same journal envelope as every
// structured event. The common sequence rule must reject a repeated envelope
// before its nested projection can render another item.
{
  const projectedEvent = {
    ownedId: 'owned-projection', provider: 'codex', generation: 1,
    sequence: 1, timestampMs: 400,
    payload: {
      kind: 'terminalProjection',
      eventType: 'item.completed',
      providerInstanceId: 'terminal-transcript:native-projection',
      timestampMs: 390,
      nativeSessionId: 'native-projection',
      itemId: 'projected-answer',
      payload: {
        item: {
          id: 'projected-answer', type: 'assistant-message',
          content: [{ channel: 'assistant', text: 'Projected answer' }]
        }
      },
      providerMetadata: { source: 'terminal-transcript', historical: true },
      rawFrameReference: { id: 'frame-projection', redacted: true }
    }
  };
  assert.equal(store.applyAgentConversationEvent(projectedEvent), true);
  const current = store.getConversationSession('owned-projection');
  assert.equal(current.lastSequence, 1);
  assert.equal(current.agentItems.length, 1);
  assert.equal(current.agentItems[0].content[0].text, 'Projected answer');

  const staleProjection = {
    ...projectedEvent,
    payload: {
      ...projectedEvent.payload,
      itemId: 'stale-projected-answer',
      payload: {
        item: {
          id: 'stale-projected-answer', type: 'assistant-message',
          content: [{ channel: 'assistant', text: 'Stale projected answer' }]
        }
      }
    }
  };
  assert.equal(store.applyAgentConversationEvent(staleProjection), false);
  assert.equal(current.lastSequence, 1);
  assert.equal(current.agentItems.length, 1);
}

const saved = store.captureConversationWorkspace('owned-b');
store.setConversationDraft('owned-b', 'Changed');
store.setConversationMode('owned-b', 'structured');
store.restoreConversationWorkspace('owned-b', 'claude', saved);
assert.equal(store.getConversationSession('owned-b').draft, 'Changed');
assert.equal(store.getConversationSession('owned-b').mode, 'raw');
assert.equal(store.getConversationSession('owned-b').selectedChildId, null);
assert.equal(saved.version, 1);
assert.equal(saved.owner, 'terminal');
assert.equal(saved.generation, 2);
assert.equal(saved.writerLease.owner, 'terminal');

// A sent screenshot has to stay visible in the transcript. The provider never
// echoes the image back, so the store keeps the sent copy against the user
// message the send produced.
const sent = store.ensureConversationSession('owned-sent', 'claude');
assert.deepEqual(sent.sentAttachments, {});
store.recordSentConversationAttachments('owned-sent', [{
  id: 'image-sent', name: 'shot.png', mimeType: 'image/png',
  path: '/managed/shot.png', previewUrl: 'blob:sent'
}]);
store.applyAgentConversationEvent({
  ownedId: 'owned-sent',
  provider: 'claude',
  generation: 1,
  sequence: 1,
  timestampMs: 500,
  payload: { kind: 'userMessage', itemId: 'user-turn-sent', text: 'Look', completed: true }
});
assert.deepEqual(
  store.getConversationSession('owned-sent').sentAttachments['user-turn-sent'].map((item) => item.id),
  ['image-sent'],
  'the sent screenshot is claimed by the user message it was sent with'
);
store.applyAgentConversationEvent({
  ownedId: 'owned-sent',
  provider: 'claude',
  generation: 1,
  sequence: 2,
  timestampMs: 510,
  payload: { kind: 'userMessage', itemId: 'user-turn-later', text: 'And again', completed: true }
});
assert.equal(
  store.getConversationSession('owned-sent').sentAttachments['user-turn-later'],
  undefined,
  'a later message without attachments claims nothing'
);
// A send that fails releases its hold, so the screenshots left in the composer
// cannot reappear on some unrelated message later in the transcript.
store.recordSentConversationAttachments('owned-sent', [{
  id: 'image-failed', name: 'failed.png', mimeType: 'image/png',
  path: '/managed/failed.png', previewUrl: 'blob:failed'
}]);
store.recordSentConversationAttachments('owned-sent', []);
store.applyAgentConversationEvent({
  ownedId: 'owned-sent',
  provider: 'claude',
  generation: 1,
  sequence: 3,
  timestampMs: 520,
  payload: { kind: 'userMessage', itemId: 'user-turn-after-failure', text: 'Retry', completed: true }
});
assert.equal(
  store.getConversationSession('owned-sent').sentAttachments['user-turn-after-failure'],
  undefined,
  'a released hold cannot be claimed by a later message'
);

await test('sendStructuredMessage resolves the terminal from the owned session, not a passed argument', async () => {
  const ownedId = 'owned-terminal-route';
  const terminalId = 'terminal-from-owned-session';
  const serviceSource = readFileSync(
    new URL('../src/lib/shell/conversation/conversationService.ts', import.meta.url),
    'utf8'
  ).match(
    /export async function sendStructuredMessage\([\s\S]*?\n\}\n\n\/\*\* Stops the active turn/
  );
  assert.ok(serviceSource);
  const serviceJavaScript = stripTypeScriptTypes(
    serviceSource[0]
      .replace(/\n\n\/\*\* Stops the active turn$/, '')
      .replace('export async function', 'async function'),
    { mode: 'strip' }
  );
  const state = {
    attachments: [],
    generation: 0,
    writerLease: { ownedId, generation: 0, owner: 'terminal' }
  };
  const terminalWrites: Array<{ terminalId: string; text: string }> = [];
  const sendStructuredMessage = Function(
    'getConversationSession',
    'setConversationSending',
    'rail',
    'shouldReviveBeforeSend',
    'writeTerminalSessionFromTauri',
    'cleanupConversationAttachmentPreview',
    'setConversationAttachments',
    'recordSentConversationAttachments',
    `${serviceJavaScript}\nreturn sendStructuredMessage;`
  )(
    () => state,
    () => undefined,
    {
      owned: [{
        ownedId,
        ptySessionId: terminalId,
        origin: 'external',
        state: 'live',
        executionOwner: 'terminal'
      }]
    },
    () => false,
    async (usedTerminalId: string, text: string) => {
      terminalWrites.push({ terminalId: usedTerminalId, text });
      return true;
    },
    () => undefined,
    () => undefined,
    () => undefined
  ) as (messageOwnedId: string, text: string) => Promise<void>;

  await sendStructuredMessage(ownedId, 'Route this message');
  assert.deepEqual(terminalWrites.map((write) => write.terminalId), [terminalId, terminalId]);
});

// A permission request without provider options must answer through the
// decision command, not send the fabricated option id back to the provider.
{
  const ownedId = 'owned-empty-permission';
  const requestId = 'request-empty-permission';
  const permissionEvent = (type: 'approval.requested' | 'approval.resolved', sequence: number) => ({
    type,
    ownedId,
    provider: 'codex' as const,
    providerInstanceId: 'fixture',
    generation: 1,
    sequence,
    timestampMs: 900 + sequence,
    payload: { requestId }
  });
  store.ensureConversationSession(ownedId, 'codex');
  store.applyAgentConversationEvent({
    ...permissionEvent('approval.requested', 1),
    payload: { requestId, title: 'Approval needed', toolTitle: 'Read File' }
  });
  const pending = store.getConversationSession(ownedId).pendingApprovals[requestId];
  assert.ok(pending);
  assert.equal(pending.options[0].optionId, 'accept');
  assert.equal(pending.options[0].kind, 'allow_once');

  const responseSource = readFileSync(
    new URL('../src/lib/shell/conversation/conversationService.ts', import.meta.url),
    'utf8'
  ).match(
    /export async function sendPermissionResponse\([\s\S]*?\n\}\n\n\/\*\* Answers one structured input/
  );
  assert.ok(responseSource);
  const responseJavaScript = stripTypeScriptTypes(
    responseSource[0]
      .replace(/\n\n\/\*\* Answers one structured input$/, '')
      .replace('export async function', 'async function'),
    { mode: 'strip' }
  );
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke = async (command: string, args?: Record<string, unknown>): Promise<unknown> => {
    calls.push({ command, args });
    if (command === 'respond_agent_conversation_permission') throw new Error('ACP permission option is not part of the pending request');
    return undefined;
  };
  const respondToStructuredApproval = async (responseOwnedId: string, responseRequestId: string, decision: 'accept' | 'decline' | 'cancel'): Promise<void> => {
    calls.push({ command: 'respond_agent_conversation_approval', args: { request: { ownedId: responseOwnedId, requestId: responseRequestId, decision } } });
    store.applyAgentConversationEvent(permissionEvent('approval.resolved', 2));
  };
  const sendPermissionResponse = Function(
    'getConversationSession',
    'invoke',
    'respondToStructuredApproval',
    `${responseJavaScript}\nreturn sendPermissionResponse;`
  )(
    store.getConversationSession,
    invoke,
    respondToStructuredApproval
  ) as (responseOwnedId: string, responseRequestId: string, optionId: string) => Promise<void>;

  await assert.doesNotReject(() => sendPermissionResponse(ownedId, requestId, pending.options[0].optionId));
  assert.deepEqual(calls.map((call) => call.command), ['respond_agent_conversation_approval']);
  assert.equal(store.getConversationSession(ownedId).pendingApprovals[requestId], undefined);
  assert.equal((pending.options[0] as { synthetic?: boolean }).synthetic, true);
}

store.removeConversationSession('owned-a');
assert.equal(store.getConversationSession('owned-a'), null);
assert.ok(store.getConversationSession('owned-b'));

// Leaving a session releases its materialized transcript, while later
// background events update only the lightweight rail-presence record.
{
  const ownedId = 'owned-evicted';
  store.applyAgentConversationEvent({
    ownedId, provider: 'codex', generation: 1, sequence: 1, timestampMs: 1_000,
    payload: { kind: 'userMessage', itemId: 'large-row', text: 'materialized', completed: true }
  });
  store.evictConversationSession(ownedId);
  store.recordAgentConversationPresenceEvent({
    ownedId, provider: 'codex', generation: 1, sequence: 2, timestampMs: 1_010,
    payload: { kind: 'turn', turnId: 'background-turn', state: 'started' }
  });
  assert.equal(store.getConversationSession(ownedId), null);
  assert.equal(get(sessionPresenceHistory)[ownedId].activeTurnId, 'background-turn');
}

// Scrolling up loads the page of stored events just older than what is on
// screen. The older rows have to land in front of the ones already there,
// keeping one ascending transcript, and an item that straddles the page
// boundary must not be drawn twice.
{
  const olderEvent = (sequence, itemId, kind, text) => ({
    ownedId: 'owned-paged',
    provider: 'codex',
    generation: 1,
    sequence,
    timestampMs: 100 + sequence,
    payload: { kind, itemId, text, completed: true }
  });

  store.applyAgentConversationSnapshot({
    connection: {
      ownedId: 'owned-paged',
      provider: 'codex',
      generation: 1,
      state: 'connected',
      nativeSessionId: 'thread-paged'
    },
    lastSequence: 12,
    events: [
      olderEvent(11, 'user-newer', 'userMessage', 'Newer question'),
      olderEvent(12, 'assistant-newer', 'assistantMessage', 'Newer answer')
    ]
  });

  const opened = store.getConversationSession('owned-paged');
  assert.equal(opened.oldestLoadedSequence, 11, 'opening records the oldest event it was given');
  assert.equal(opened.reachedTranscriptStart, false);

  store.prependOlderConversationEvents('owned-paged', {
    events: [
      olderEvent(9, 'user-older', 'userMessage', 'Older question'),
      olderEvent(10, 'assistant-older', 'assistantMessage', 'Older answer')
    ],
    hasMore: true
  });

  const paged = store.getConversationSession('owned-paged');
  assert.deepEqual(
    paged.timeline.map((entry) => entry.text),
    ['Older question', 'Older answer', 'Newer question', 'Newer answer'],
    'an older page lands in front of the transcript already on screen'
  );
  assert.deepEqual(
    paged.agentItems.map((item) => item.content.map((part) => part.text).join('')),
    ['Older question', 'Older answer', 'Newer question', 'Newer answer']
  );
  const itemIds = paged.timeline.map((entry) => entry.itemId);
  assert.equal(new Set(itemIds).size, itemIds.length, 'no row is drawn twice');
  assert.equal(paged.oldestLoadedSequence, 9);
  assert.equal(paged.reachedTranscriptStart, false);
  assert.equal(paged.lastSequence, 12, 'reading older history never rewinds the live cursor');

  // An item whose start is in the older page and whose completion is already on
  // screen must not produce a second row.
  store.prependOlderConversationEvents('owned-paged', {
    events: [olderEvent(8, 'user-older', 'userMessage', 'Older question')],
    hasMore: false
  });
  const deduped = store.getConversationSession('owned-paged');
  assert.deepEqual(
    deduped.timeline.map((entry) => entry.text),
    ['Older question', 'Older answer', 'Newer question', 'Newer answer']
  );
  assert.equal(deduped.oldestLoadedSequence, 8);
  assert.equal(deduped.reachedTranscriptStart, true, 'the start stops any further request');
}

// Reading further back into a provider's own transcript writes those older
// events BELOW the ones already stored, so their sequences count down through
// zero and into negatives. A reducer that treats "not greater than the last
// sequence" as "already seen" throws every one of them away, which is what
// happened: 112 events arrived off disk and none of them ever reached a row.
{
  const event = (sequence, itemId, kind, text) => ({
    ownedId: 'owned-negative',
    provider: 'codex',
    generation: 0,
    sequence,
    timestampMs: 1000 + sequence,
    payload: { kind, itemId, text, completed: true }
  });

  store.applyAgentConversationSnapshot({
    connection: {
      ownedId: 'owned-negative',
      provider: 'codex',
      generation: 0,
      state: 'connected',
      nativeSessionId: 'thread-negative'
    },
    lastSequence: 2,
    events: [
      event(1, 'user-stored', 'userMessage', 'Stored question'),
      event(2, 'assistant-stored', 'assistantMessage', 'Stored answer')
    ]
  });

  store.prependOlderConversationEvents('owned-negative', {
    events: [
      event(-2, 'user-disk', 'userMessage', 'Question from the transcript'),
      event(-1, 'assistant-disk', 'assistantMessage', 'Answer from the transcript'),
      event(0, 'assistant-disk-2', 'assistantMessage', 'Second answer from the transcript')
    ],
    hasMore: true
  });

  const negative = store.getConversationSession('owned-negative');
  assert.deepEqual(
    negative.timeline.map((entry) => entry.text),
    [
      'Question from the transcript',
      'Answer from the transcript',
      'Second answer from the transcript',
      'Stored question',
      'Stored answer'
    ],
    'events at sequences at or below zero must still reach the transcript'
  );
  assert.equal(negative.oldestLoadedSequence, -2);
  assert.equal(negative.lastSequence, 2, 'reading older history never rewinds the live cursor');
}

// Claude never says it compacted; the only sign is the reported occupancy
// falling off a cliff. Without this the transcript has a silent gap and the
// reply after it reads as though the agent forgot the conversation.
{
  const ownedId = 'owned-compaction';
  store.applyAgentConversationSnapshot({
    connection: { ownedId, provider: 'claude', generation: 1, state: 'connected' },
    lastSequence: 1,
    events: [
      {
        ownedId, provider: 'claude', generation: 1, sequence: 1, timestampMs: 900,
        payload: { kind: 'usage', usedTokens: 351_238, contextWindow: 400_000 }
      }
    ]
  });
  store.applyAgentConversationEvent({
    ownedId, provider: 'claude', generation: 1, sequence: 2, timestampMs: 910,
    payload: { kind: 'usage', usedTokens: 22_202, contextWindow: 400_000 }
  });
  const compactions = store.getConversationSession(ownedId).timeline
    .filter((entry) => entry.kind === 'compaction');
  assert.equal(compactions.length, 1, 'a window falling to a fraction of itself is a compaction');
  assert.equal(compactions[0].preTokens, 351_238);
  assert.equal(compactions[0].postTokens, 22_202);
  assert.equal(store.getConversationSession(ownedId).usage.usedTokens, 22_202);
}

// An ordinary decline is not one. Agents report the last request as often as
// the session, so a modest fall says nothing about the window being emptied.
{
  const ownedId = 'owned-no-compaction';
  store.applyAgentConversationSnapshot({
    connection: { ownedId, provider: 'claude', generation: 1, state: 'connected' },
    lastSequence: 1,
    events: [
      {
        ownedId, provider: 'claude', generation: 1, sequence: 1, timestampMs: 900,
        payload: { kind: 'usage', usedTokens: 351_238 }
      }
    ]
  });
  store.applyAgentConversationEvent({
    ownedId, provider: 'claude', generation: 1, sequence: 2, timestampMs: 910,
    payload: { kind: 'usage', usedTokens: 300_000 }
  });
  assert.equal(
    store.getConversationSession(ownedId).timeline.filter((entry) => entry.kind === 'compaction').length,
    0
  );
}

// The same drop read back from the journal when the session is reopened. A
// marker that only appeared live would be gone by morning.
{
  const ownedId = 'owned-replayed-compaction';
  store.applyAgentConversationSnapshot({
    connection: { ownedId, provider: 'claude', generation: 1, state: 'connected' },
    lastSequence: 3,
    events: [
      {
        ownedId, provider: 'claude', generation: 1, sequence: 1, timestampMs: 900,
        payload: { kind: 'usage', usedTokens: 351_238 }
      },
      {
        ownedId, provider: 'claude', generation: 1, sequence: 2, timestampMs: 910,
        payload: { kind: 'usage', usedTokens: 22_202 }
      },
      {
        ownedId, provider: 'claude', generation: 1, sequence: 3, timestampMs: 920,
        payload: { kind: 'assistantMessage', itemId: 'after', text: 'Carrying on.', completed: true }
      }
    ]
  });
  assert.deepEqual(
    store.getConversationSession(ownedId).timeline.map((entry) => entry.kind),
    ['compaction', 'assistant'],
    'replayed history marks the boundary where the live session did'
  );
}

// Codex reports its compaction outright, and then reports nothing else about
// the window. The explicit record is the row.
{
  const ownedId = 'owned-explicit-compaction';
  store.applyAgentConversationSnapshot({
    connection: { ownedId, provider: 'codex', generation: 1, state: 'connected' },
    lastSequence: 1,
    events: [
      {
        ownedId, provider: 'codex', generation: 1, sequence: 1, timestampMs: 900,
        payload: { kind: 'contextCompaction', trigger: 'auto' }
      }
    ]
  });
  const timeline = store.getConversationSession(ownedId).timeline;
  assert.deepEqual(timeline.map((entry) => entry.kind), ['compaction']);
  assert.equal(timeline[0].trigger, 'auto');
  assert.equal(timeline[0].preTokens, undefined);
}

// The retained event window is capped by event count. Without a working cap
// the transcript of a long session grows for as long as the session is
// selected, which is what put the app's memory on a ramp.
{
  const ownedId = 'owned-event-window-cap';
  // Mirrors ACTIVE_EVENT_WINDOW_EVENTS and ACTIVE_EVENT_WINDOW_TRIM_EVENTS.
  const windowEvents = 20_000;
  const trimEvents = 15_000;
  for (let sequence = 1; sequence <= windowEvents + 1; sequence += 1) {
    store.applyAgentConversationEvent({
      ownedId, provider: 'codex', generation: 1, sequence, timestampMs: sequence,
      payload: { kind: 'assistantMessage', itemId: `m-${sequence}`, text: 'x', completed: true }
    });
  }
  const session = store.getConversationSession(ownedId);
  assert.equal(session.loadedEvents.length, trimEvents, 'the window trims down to its low mark');
  assert.equal(
    session.oldestLoadedSequence,
    windowEvents + 1 - trimEvents + 1,
    'the oldest retained sequence follows the trim so scrolling up still has a cursor'
  );
  assert.equal(session.newestLoadedSequence, windowEvents + 1, 'the newest event is retained');
}

console.log('agent conversation store tests passed');
