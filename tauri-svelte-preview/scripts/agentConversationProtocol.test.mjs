import assert from 'node:assert/strict';

import {
  applyConversationEvent,
  createConversationState,
  importConversationHistory
} from '../src/lib/shell/conversation/conversationReducer.ts';
import { configOptionPlacement } from '../src/lib/shell/conversation/conversationTypes.ts';
import { decideConversationActivation } from '../src/lib/shell/conversation/conversationActivation.ts';

const event = (overrides = {}) => ({
  ownedId: 'owned-a',
  provider: 'codex',
  generation: 1,
  sequence: 1,
  timestampMs: 1_000,
  payload: { kind: 'connection', state: 'connected', nativeSessionId: 'thread-a' },
  ...overrides
});

// The first event establishes the live generation and sequence.
{
  const state = applyConversationEvent(createConversationState('owned-a', 'codex'), event());
  assert.equal(state.generation, 1);
  assert.equal(state.lastSequence, 1);
  assert.equal(state.connectionState, 'connected');
}

// Older generations and duplicate sequences are ignored by identity.
{
  const current = applyConversationEvent(createConversationState('owned-a', 'codex'), event());
  const older = applyConversationEvent(current, event({ generation: 0, sequence: 99 }));
  const duplicate = applyConversationEvent(current, event({ sequence: 1 }));
  assert.equal(older, current);
  assert.equal(duplicate, current);
}

// A missing event is never guessed. Existing history remains visible.
{
  let state = applyConversationEvent(createConversationState('owned-a', 'codex'), event());
  state = applyConversationEvent(state, event({
    sequence: 2,
    payload: { kind: 'userMessage', itemId: 'user-1', text: 'Keep this', completed: true }
  }));
  state = applyConversationEvent(state, event({
    sequence: 4,
    payload: { kind: 'assistantDelta', itemId: 'assistant-1', delta: 'Missing sequence three' }
  }));
  assert.equal(state.desynchronized, true);
  assert.equal(state.timeline.length, 1);
  assert.equal(state.timeline[0].text, 'Keep this');
}

// Assistant deltas append to one item and completion seals the authoritative text.
{
  let state = applyConversationEvent(createConversationState('owned-a', 'codex'), event());
  state = applyConversationEvent(state, event({
    sequence: 2,
    payload: { kind: 'assistantDelta', itemId: 'assistant-1', delta: 'Hello' }
  }));
  state = applyConversationEvent(state, event({
    sequence: 3,
    payload: { kind: 'assistantDelta', itemId: 'assistant-1', delta: ' world' }
  }));
  state = applyConversationEvent(state, event({
    sequence: 4,
    payload: { kind: 'assistantMessage', itemId: 'assistant-1', text: 'Hello world', completed: true }
  }));
  assert.deepEqual(state.timeline[0], {
    kind: 'assistant',
    itemId: 'assistant-1',
    text: 'Hello world',
    completed: true,
    timestampMs: 1_000
  });
}

// Plan payloads replace the current generation's plan without losing ordering.
{
  let state = applyConversationEvent(createConversationState('owned-a', 'codex'), event());
  state = applyConversationEvent(state, event({
    sequence: 2,
    payload: {
      kind: 'plan',
      items: [{ text: 'Inspect the change', status: 'pending' }]
    }
  }));
  state = applyConversationEvent(state, event({
    sequence: 3,
    payload: {
      kind: 'plan',
      items: [{ text: 'Inspect the change', status: 'completed' }]
    }
  }));
  assert.deepEqual(state.timeline[0], {
    kind: 'plan',
    itemId: 'plan:1',
    items: [{ text: 'Inspect the change', status: 'completed' }],
    timestampMs: 1_000
  });
}

// Provider history is imported once by native item identity.
{
  const state = createConversationState('owned-a', 'codex');
  const history = [
    { kind: 'user', itemId: 'user-1', text: 'Question', completed: true, timestampMs: 100 },
    { kind: 'assistant', itemId: 'assistant-1', text: 'Answer', completed: true, timestampMs: 200 }
  ];
  const once = importConversationHistory(state, history);
  const twice = importConversationHistory(once, history);
  assert.equal(once.timeline.length, 2);
  assert.equal(twice.timeline.length, 2);
}

// A connected structured runtime is a pure view switch. Only a stopped,
// native-id-bearing external Codex session may cross from terminal ownership
// into a structured session/load activation.
{
  const appSession = {
    agent: 'codex', origin: 'app', state: 'background',
    executionOwner: 'structured', ptySessionId: null, nativeSessionId: 'thread-app'
  };
  assert.deepEqual(decideConversationActivation(appSession, {
    provider: 'codex', state: 'connected', nativeSessionId: 'thread-app'
  }), { kind: 'view' });
  assert.deepEqual(decideConversationActivation(appSession, null), {
    kind: 'structured', nativeSessionMode: 'resume'
  });

  const stoppedExternal = {
    agent: 'codex', origin: 'external', state: 'background',
    executionOwner: 'stopped', ptySessionId: null, nativeSessionId: 'thread-cli'
  };
  assert.deepEqual(decideConversationActivation(stoppedExternal, null), {
    kind: 'structured', nativeSessionMode: 'load'
  });
  assert.deepEqual(decideConversationActivation(stoppedExternal, {
    provider: 'codex', state: 'connected', nativeSessionId: 'thread-cli'
  }), {
    kind: 'structured', nativeSessionMode: 'load'
  });
  assert.deepEqual(decideConversationActivation({
    ...stoppedExternal, executionOwner: 'terminal', ptySessionId: 'pty-live'
  }, {
    provider: 'codex', state: 'connected', nativeSessionId: 'thread-cli'
  }), { kind: 'terminal' });
  assert.deepEqual(decideConversationActivation({
    ...stoppedExternal, nativeSessionId: null
  }, null), { kind: 'terminal' });
  assert.deepEqual(decideConversationActivation({
    ...stoppedExternal, agent: 'other'
  }, null), { kind: 'terminal' });
}

// A provider error is additive and does not clear completed messages.
{
  let state = importConversationHistory(createConversationState('owned-a', 'codex'), [
    { kind: 'assistant', itemId: 'assistant-1', text: 'Still here', completed: true, timestampMs: 100 }
  ]);
  state = applyConversationEvent(state, event({
    payload: { kind: 'error', code: 'connection_lost', message: 'Reconnect', recoverable: true }
  }));
  assert.equal(state.timeline[0].text, 'Still here');
  assert.equal(state.timeline[1].kind, 'error');
  assert.equal(state.timeline[1].recoverable, true);
}

// Known categories get their fixed controls and future categories remain generic.
{
  assert.equal(configOptionPlacement('model'), 'model-picker');
  assert.equal(configOptionPlacement('thought_level'), 'reasoning-picker');
  assert.equal(configOptionPlacement('mode'), 'mode-picker');
  assert.equal(configOptionPlacement('model_config'), 'model-popover');
  assert.equal(configOptionPlacement('provider.future/category'), 'more-options');
}

console.log('agent conversation protocol tests passed');
