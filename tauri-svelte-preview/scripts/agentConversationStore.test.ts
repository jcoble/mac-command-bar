import assert from 'node:assert/strict';
import { readFileSync, rmSync, writeFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
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

const source = readFileSync(storePath, 'utf8');
const surfaceSource = readFileSync(
  new URL('../src/lib/shell/components/ConversationSurface.svelte', import.meta.url),
  'utf8'
);
const javascript = stripTypeScriptTypes(source, { mode: 'strip' });
const compiled = compileModule(javascript, {
  generate: 'client',
  filename: 'conversationStore.svelte.js'
});
writeFileSync(outputPath, compiled.js.code);

let store;
try {
  store = await import(`${outputPath}?test=${Date.now()}`);
} finally {
  rmSync(outputPath, { force: true });
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

store.removeConversationSession('owned-a');
assert.equal(store.getConversationSession('owned-a'), null);
assert.ok(store.getConversationSession('owned-b'));

console.log('agent conversation store tests passed');
