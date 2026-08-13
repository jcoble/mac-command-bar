import assert from 'node:assert/strict';

import {
  generationForSend,
  validateStructuredSendGeneration,
  shouldReviveBeforeSend
} from '../src/lib/shell/conversation/conversationActivation.ts';
import { readFileSync } from 'node:fs';

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
  4,
  'an idempotent connected ensure remains usable for send'
);
assert.equal(
  generationForSend(4, 3),
  null,
  'send rejects an older activation generation'
);

assert.equal(
  validateStructuredSendGeneration(
    4,
    { state: 'connected', generation: 4 },
    { connectionState: 'connected', generation: 4 }
  ),
  4,
  'a successful connected ensure can validate an idempotent send generation'
);
assert.equal(
  validateStructuredSendGeneration(
    4,
    { state: 'connected', generation: 4 },
    { connectionState: 'connected', generation: 5 }
  ),
  null,
  'a generation replacement between ensure and send fails before the request'
);
{
  const stateAfterEnsure = { connectionState: 'connected', generation: 4 };
  const expectedGeneration = stateAfterEnsure.generation;
  stateAfterEnsure.generation = 5;
  assert.equal(
    validateStructuredSendGeneration(
      expectedGeneration,
      { state: 'connected', generation: expectedGeneration },
      stateAfterEnsure
    ),
    null,
    'a store mutation cannot replace the captured generation after ensure'
  );
}
assert.equal(
  validateStructuredSendGeneration(
    4,
    { state: 'disconnected', generation: 4 },
    { connectionState: 'connected', generation: 4 }
  ),
  null,
  'a non-connected ensure response cannot authorize a send'
);

const serviceSource = readFileSync(
  new URL('../src/lib/shell/conversation/conversationService.ts', import.meta.url),
  'utf8'
);
assert.match(
  serviceSource,
  /const activated = await ensureStructuredConversation\([\s\S]*?await invoke\('send_agent_conversation_message'/,
  'the service activates only as part of the send request'
);
assert.match(
  serviceSource,
  /const validatedGeneration = validatedState\?\.generation === expectedGeneration[\s\S]*?if \(validatedGeneration === null/,
  'the service must reject a replaced generation before invoking the send request'
);
assert.match(
  serviceSource,
  /nativeSessionMode,[\s\S]*?reasoningEffort: startConfig\?\.reasoningEffort \?\? state\.agentConfig\.reasoningEffort/,
  'revival must retain the session-start effort when it creates a new adapter process'
);

console.log('conversationSendRecovery.test.ts passed');
