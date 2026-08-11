import assert from 'node:assert/strict';

import {
  generationForSend,
  shouldReviveBeforeSend
} from '../src/lib/shell/conversation/conversationActivation.ts';

const connected = {
  sessionState: 'live',
  executionOwner: 'structured',
  connectionState: 'connected',
  generation: 4
};

assert.equal(shouldReviveBeforeSend(connected), false, 'a live connected session sends immediately');
assert.equal(
  shouldReviveBeforeSend({ ...connected, connectionState: 'disconnected' }),
  true,
  'a disconnected connection must be revived before send'
);
assert.equal(
  shouldReviveBeforeSend({ ...connected, sessionState: 'exited', executionOwner: 'stopped' }),
  true,
  'an exited rail session must be revived before send'
);
assert.equal(
  shouldReviveBeforeSend({ ...connected, generation: 0 }),
  true,
  'a session without a usable generation must be activated before send'
);

assert.equal(
  generationForSend(4, 5),
  5,
  'send adopts the generation returned by the activation call'
);
assert.equal(
  generationForSend(4, 4),
  null,
  'send rejects an activation response that did not create a new generation'
);
assert.equal(
  generationForSend(4, 3),
  null,
  'send rejects an older activation generation'
);

console.log('conversationSendRecovery.test.mjs passed');
