import assert from 'node:assert/strict';

import {
  createConversationState,
  reduceConversationEvent
} from '../src/lib/shell/conversation/conversationReducer.ts';
import { conversationDisplayItems, conversationMessagesFromEvents } from '../src/lib/shell/conversation/conversationMessages.ts';
import type { AgentConversationEvent } from '../src/lib/shell/conversation/conversationTypes.ts';
import { configOptionPlacement } from '../src/lib/shell/conversation/conversationTypes.ts';
import { decideConversationActivation } from '../src/lib/shell/conversation/conversationActivation.ts';

const event = (overrides: Partial<AgentConversationEvent> = {}): AgentConversationEvent => ({
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
  const state = reduceConversationEvent(createConversationState('owned-a', 'codex'), event());
  assert.equal(state.generation, 1);
  assert.equal(state.lastSequence, 1);
  assert.equal(state.connectionState, 'connected');
}

// Older generations and duplicate sequences are ignored by identity.
{
  const current = reduceConversationEvent(createConversationState('owned-a', 'codex'), event());
  const older = reduceConversationEvent(current, event({ generation: 0, sequence: 99 }));
  const duplicate = reduceConversationEvent(current, event({ sequence: 1 }));
  assert.equal(older, current);
  assert.equal(duplicate, current);
}

// A missing event is never guessed. Existing history remains visible.
{
  const first = event();
  const user = event({
    sequence: 2,
    payload: { kind: 'userMessage', itemId: 'user-1', text: 'Keep this', completed: true }
  });
  let state = reduceConversationEvent(createConversationState('owned-a', 'codex'), first);
  state = reduceConversationEvent(state, user);
  state = reduceConversationEvent(state, event({
    sequence: 4,
    payload: { kind: 'assistantDelta', itemId: 'assistant-1', delta: 'Missing sequence three' }
  }));
  assert.equal(state.desynchronized, true);
  const displayed = conversationDisplayItems(conversationMessagesFromEvents([first, user]));
  assert.equal(displayed.length, 1);
  assert.equal(displayed[0].text, 'Keep this');
}

// Assistant deltas append to one item and completion seals the authoritative text.
{
  const events = [event(), event({
    sequence: 2,
    payload: { kind: 'assistantDelta', itemId: 'assistant-1', delta: 'Hello' }
  }), event({
    sequence: 3,
    payload: { kind: 'assistantDelta', itemId: 'assistant-1', delta: ' world' }
  }), event({
    sequence: 4,
    payload: { kind: 'assistantMessage', itemId: 'assistant-1', text: 'Hello world', completed: true }
  })];
  const { metadata, turnId, blocks, ...answer } = conversationDisplayItems(
    conversationMessagesFromEvents(events)
  )[0];
  assert.deepEqual(answer, {
    kind: 'assistant',
    itemId: 'assistant-1',
    text: 'Hello world',
    completed: true,
    timestampMs: 1_000
  });
}

// Plan payloads replace the current generation's plan without losing ordering.
{
  const events = [event(), event({
    sequence: 2,
    payload: {
      kind: 'plan',
      items: [{ text: 'Inspect the change', status: 'pending' }]
    }
  }), event({
    sequence: 3,
    payload: {
      kind: 'plan',
      items: [{ text: 'Inspect the change', status: 'completed' }]
    }
  })];
  const plan = conversationDisplayItems(conversationMessagesFromEvents(events)).at(-1);
  assert.equal(plan?.kind, 'plan');
  assert.deepEqual(plan?.steps.map((step) => ({ title: step.title, state: step.state })), [{ title: 'Inspect the change', state: 'completed' }]);
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
  const events = [event({
    payload: { kind: 'assistantMessage', itemId: 'assistant-1', text: 'Still here', completed: true }
  }), event({
    sequence: 2,
    payload: { kind: 'error', code: 'connection_lost', message: 'Reconnect', recoverable: true }
  })];
  const displayed = conversationDisplayItems(conversationMessagesFromEvents(events));
  assert.equal(displayed[0].text, 'Still here');
  assert.equal(displayed[1].kind, 'error');
  assert.equal(displayed[1].metadata?.recoverable, true);
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
