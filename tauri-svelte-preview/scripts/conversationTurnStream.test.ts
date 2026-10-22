/// <reference types="node" />
// TSK-1167: one chat turn as the frontend sees it, from the prompt to the
// turn's end, for Claude Code and Codex alike. Each test pins behaviour that
// works today so a change that breaks it fails here.

import assert from 'node:assert/strict';
import { readFileSync, rmSync, writeFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileModule } from 'svelte/compiler';
import {
  createConversationState,
  reduceConversationEvent,
  shouldClearConversationSending,
  usageDropIsCompaction
} from '../src/lib/shell/conversation/conversationReducer.ts';
import {
  conversationDisplayItems,
  conversationHasRunningTool,
  conversationMessagesFromEvents,
  displayItemsFromConversationEvents
} from '../src/lib/shell/conversation/conversationMessages.ts';
import type { AgentConversationEvent } from '../src/lib/shell/conversation/conversationTypes.ts';

// The store reads `import.meta.env` and a runes module, so it is compiled the
// same way agentConversationStore.test.ts compiles it.
const at = (path: string): string => fileURLToPath(new URL(path, import.meta.url));
const compile = (path: string, filename: string): string => compileModule(
  stripTypeScriptTypes(readFileSync(path, 'utf8'), { mode: 'strip' }),
  { generate: 'client', filename }
).js.code.replaceAll('import.meta.env', '({ DEV: false })');
const storeOut = at('../src/lib/shell/conversation/.conversationTurnStream.test.mjs');
const diagnosticsOut = at('../src/lib/shell/.conversationTurnStreamDiagnostics.test.mjs');
writeFileSync(storeOut, compile(at('../src/lib/shell/conversation/conversationStore.svelte.ts'), 'conversationStore.svelte.js')
  .replace(/\.\.\/resourceDiagnostics\.svelte(\.ts|\.js)/, '../.conversationTurnStreamDiagnostics.test.mjs'));
writeFileSync(diagnosticsOut, compile(at('../src/lib/shell/resourceDiagnostics.svelte.ts'), 'resourceDiagnostics.svelte.js'));
let store;
try {
  store = await import(`${storeOut}?test=${Date.now()}`);
} finally {
  rmSync(storeOut, { force: true });
  rmSync(diagnosticsOut, { force: true });
}

type Provider = 'codex' | 'claude';
const PROVIDERS: Provider[] = ['codex', 'claude'];

function stream(ownedId: string, provider: Provider) {
  let sequence = 0;
  return (payload: Record<string, unknown>, extra: Partial<AgentConversationEvent> = {}): AgentConversationEvent => ({
    ownedId, provider, generation: 1, sequence: ++sequence, timestampMs: sequence * 10,
    payload: payload as AgentConversationEvent['payload'], ...extra
  });
}

// A canonical provider event, the shape stored terminal projections unwrap to.
const canonical = (sequence: number, payload: Record<string, unknown>) => ({
  type: 'content.delta', ownedId: 'canonical', provider: 'claude', providerInstanceId: 'fixture',
  generation: 1, sequence, timestampMs: sequence, itemId: String(payload.messageId), payload
}) as never;

// Protects 50ca299a7 (app-minted turns): the composer's Working state ends on
// the turn's end or an error, and on nothing else.
test('Working clears when a turn ends or fails, and stays through the turn\'s own events', () => {
  const event = stream('owned-working', 'codex');
  for (const state of ['completed', 'interrupted', 'failed']) {
    assert.equal(shouldClearConversationSending(event({ kind: 'turn', turnId: 't', state })), true, state);
  }
  assert.equal(shouldClearConversationSending(event({ kind: 'error', code: 'x', message: 'Lost', recoverable: true })), true);
  for (const payload of [
    { kind: 'turn', turnId: 't', state: 'started' },
    { kind: 'userMessage', itemId: 'u', text: 'Hi', completed: true },
    { kind: 'assistantDelta', itemId: 'a', delta: 'Hel' },
    { kind: 'tool', itemId: 'tool', name: 'Read', state: 'started' },
    { kind: 'approval', requestId: 'r', state: 'requested', summary: 'Edit' }
  ]) assert.equal(shouldClearConversationSending(event(payload)), false, String(payload.kind));
  for (const type of ['turn.completed', 'turn.interrupted', 'runtime.error']) {
    assert.equal(shouldClearConversationSending({ type } as never), true, type);
  }
  assert.equal(shouldClearConversationSending({ type: 'content.delta' } as never), false);
});

test('the active turn is set by its start and cleared by its end or an error, for both providers', () => {
  for (const provider of PROVIDERS) {
    for (const end of [
      { kind: 'turn', turnId: 'turn-1', state: 'completed' },
      { kind: 'turn', turnId: 'turn-1', state: 'interrupted' },
      { kind: 'error', code: 'x', message: 'Lost', recoverable: true }
    ]) {
      const event = stream(`owned-${provider}`, provider);
      let state = reduceConversationEvent(createConversationState(`owned-${provider}`, provider),
        event({ kind: 'connection', state: 'connected', nativeSessionId: 'native' }));
      state = reduceConversationEvent(state, event({ kind: 'turn', turnId: 'turn-1', state: 'started' }));
      assert.equal(state.activeTurnId, 'turn-1');
      state = reduceConversationEvent(state, event({ kind: 'assistantDelta', itemId: 'a', delta: 'Hi' }));
      assert.equal(state.activeTurnId, 'turn-1', 'a streamed reply does not end the turn');
      state = reduceConversationEvent(state, event(end));
      assert.equal(state.activeTurnId, undefined, `${provider} ${JSON.stringify(end)}`);
      assert.equal(state.connectionState, 'connected');
    }
  }
});

test('only an error the agent cannot recover from marks the connection failed', () => {
  const event = stream('owned-errors', 'claude');
  const connected = reduceConversationEvent(createConversationState('owned-errors', 'claude'),
    event({ kind: 'connection', state: 'connected' }));
  const fatal = reduceConversationEvent(connected, event({ kind: 'error', code: 'exit', message: 'Gone', recoverable: false }));
  assert.equal(fatal.connectionState, 'failed');
});

// Protects 270b09b46: a replaced session continues the event sequence, so the
// replacement is in step rather than reported as missing events.
test('a replacement session that continues the sequence stays in sync, and a gap is never applied', () => {
  const event = stream('owned-replace', 'codex');
  let state = reduceConversationEvent(createConversationState('owned-replace', 'codex'), event({ kind: 'connection', state: 'connected' }));
  state = reduceConversationEvent(state, event({ kind: 'turn', turnId: 'turn-1', state: 'started' }));
  state = reduceConversationEvent(state, event({ kind: 'connection', state: 'connected' }, { generation: 2 }));
  assert.equal(state.generation, 2);
  assert.equal(state.desynchronized, false);
  const skipped = reduceConversationEvent(state, { ...event({ kind: 'turn', turnId: 'turn-1', state: 'completed' }, { generation: 2 }), sequence: state.lastSequence + 2 });
  assert.equal(skipped.desynchronized, true);
  assert.equal(skipped.activeTurnId, 'turn-1', 'an event after a gap waits for the reload instead of being guessed at');
});

test('a usage report that carries only some numbers keeps the others', () => {
  const event = stream('owned-usage', 'codex');
  let state = reduceConversationEvent(createConversationState('owned-usage', 'codex'), event({ kind: 'connection', state: 'connected' }));
  state = reduceConversationEvent(state, event({ kind: 'usage', usedTokens: 1_200, contextWindow: 200_000 }));
  state = reduceConversationEvent(state, event({ kind: 'usage', usedTokens: 2_400 }));
  assert.deepEqual(state.usage, { inputTokens: undefined, outputTokens: undefined, usedTokens: 2_400, contextWindow: 200_000, totalTokens: undefined });
});

// Protects 424aae767: Claude says nothing when it compacts; a cliff in the
// reported occupancy is the only sign, and an ordinary decline is not one.
test('only a steep fall in a full context window reads as a compaction', () => {
  assert.equal(usageDropIsCompaction(100_000, 60_000), true);
  assert.equal(usageDropIsCompaction(100_000, 60_001), false);
  assert.equal(usageDropIsCompaction(31_999, 1_000), false, 'a nearly empty window says nothing');
  assert.equal(usageDropIsCompaction(undefined, 1_000), false);
  const event = stream('owned-compaction', 'claude');
  const kinds = conversationDisplayItems(conversationMessagesFromEvents([
    event({ kind: 'usage', usedTokens: 100_000 }),
    event({ kind: 'usage', usedTokens: 90_000 }),
    event({ kind: 'usage', usedTokens: 95_000 })
  ])).map((item) => item.kind);
  assert.equal(kinds.includes('compaction'), false);
});

test('streamed deltas build one reply and the finished message has the last word', () => {
  for (const provider of PROVIDERS) {
    const event = stream(`owned-reply-${provider}`, provider);
    const items = conversationDisplayItems(conversationMessagesFromEvents([
      event({ kind: 'assistantDelta', itemId: 'answer', delta: 'Helo ' }),
      event({ kind: 'assistantDelta', itemId: 'answer', delta: 'wrld' }),
      event({ kind: 'assistantMessage', itemId: 'answer', text: 'Hello world', completed: true })
    ]));
    assert.deepEqual(items.map((item) => [item.kind, item.text, item.completed]), [['assistant', 'Hello world', true]], provider);
  }
});

test('a tool call runs until it completes or fails, keeping the output it last reported', () => {
  for (const provider of PROVIDERS) {
    for (const end of ['completed', 'failed']) {
      const event = stream(`owned-tool-${provider}`, provider);
      const running = [
        event({ kind: 'turn', turnId: 'turn-1', state: 'started' }),
        event({ kind: 'tool', itemId: 'tool-1', name: 'Run tests', state: 'started', output: 'one\n' }),
        event({ kind: 'tool', itemId: 'tool-1', name: 'Run tests', state: 'updated', output: 'one\ntwo\n' })
      ];
      const live = conversationMessagesFromEvents(running);
      assert.equal(conversationHasRunningTool(live), true);
      assert.deepEqual(conversationDisplayItems(live).map((item) => [item.kind, item.state, item.output]), [['tool', 'running', 'one\ntwo\n']]);
      const done = conversationMessagesFromEvents([...running,
        event({ kind: 'tool', itemId: 'tool-1', name: 'Run tests', state: end, output: 'one\ntwo\nend' })]);
      assert.equal(conversationHasRunningTool(done), false, `${provider} ${end}`);
      assert.deepEqual(conversationDisplayItems(done).map((item) => [item.kind, item.state, item.output]), [['tool', end, 'one\ntwo\nend']]);
    }
  }
});

// Protects dda06a519: the Thinking row closes when the reply starts or its own
// turn ends, and not when some other turn ends.
test('reasoning stops streaming when the reply starts or its own turn ends', () => {
  const thought = canonical(1, { kind: 'agentThoughtChunk', turnId: 'turn-1', messageId: 'thought', content: { type: 'text', text: 'Checking' } });
  const reasoning = (events: never[]) => displayItemsFromConversationEvents(events)
    .filter((item) => item.kind === 'reasoning').map((item) => item.completed);
  assert.deepEqual(reasoning([thought]), [false]);
  assert.deepEqual(reasoning([thought, canonical(2, { kind: 'agentMessageChunk', turnId: 'turn-1', messageId: 'answer', content: { type: 'text', text: 'Done' } })]), [true]);
  assert.deepEqual(reasoning([thought, canonical(2, { kind: 'turn', turnId: 'turn-1', state: 'completed', messageId: 'end' })]), [true]);
  assert.deepEqual(reasoning([thought, canonical(2, { kind: 'turn', turnId: 'turn-2', state: 'completed', messageId: 'end' })]), [false]);
});

// Protects 882f6c3b7 (ACP steering): a correction sent into a running turn
// joins that turn; only Stop ends it, and the turn records that it was stopped.
test('a steered message keeps the turn running and Stop records an interrupted turn', () => {
  for (const provider of PROVIDERS) {
    const ownedId = `owned-steer-${provider}`;
    store.ensureConversationSession(ownedId, provider);
    const event = stream(ownedId, provider);
    store.applySelectedConversationEventState(ownedId, event({ kind: 'connection', state: 'connected' }));
    store.applySelectedConversationEventState(ownedId, event({ kind: 'turn', turnId: 'turn-1', state: 'started' }));
    store.applySelectedConversationEventState(ownedId, event({ kind: 'userMessage', itemId: 'user-steer-1', text: 'Use the other file', completed: true }, { turnId: 'turn-1' }));
    assert.equal(store.getConversationSession(ownedId).activeTurnId, 'turn-1');
    store.applySelectedConversationEventState(ownedId, event({ kind: 'turn', turnId: 'turn-1', state: 'interrupted' }));
    const current = store.getConversationSession(ownedId);
    assert.equal(current.activeTurnId, undefined);
    assert.deepEqual(current.selectedTurns, [{ turnId: 'turn-1', startedAtMs: 20, endedAtMs: 40, terminalState: 'interrupted' }]);
  }
});

test('reading history back, an older turn\'s end does not stop the turn running now', () => {
  store.ensureConversationSession('owned-history', 'codex');
  const event = stream('owned-history', 'codex');
  store.applySelectedConversationHistoryEventState('owned-history', event({ kind: 'turn', turnId: 'live', state: 'started' }));
  store.applySelectedConversationHistoryEventState('owned-history', event({ kind: 'turn', turnId: 'older', state: 'completed' }));
  assert.equal(store.getConversationSession('owned-history').activeTurnId, 'live');
  store.applySelectedConversationHistoryEventState('owned-history', event({ kind: 'turn', turnId: 'live', state: 'completed' }));
  assert.equal(store.getConversationSession('owned-history').activeTurnId, undefined);
});
