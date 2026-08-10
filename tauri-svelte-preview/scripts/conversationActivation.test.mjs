import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import { decideConversationActivation } from '../src/lib/shell/conversation/conversationActivation.ts';

const appSession = {
  agent: 'codex',
  origin: 'app',
  state: 'live',
  executionOwner: 'structured',
  ptySessionId: null,
  nativeSessionId: 'thread-app'
};

const matchingConnection = {
  provider: 'codex',
  state: 'connected',
  nativeSessionId: 'thread-app'
};

assert.deepEqual(
  decideConversationActivation(appSession, matchingConnection),
  { kind: 'view' },
  'a connected app session is only brought into view'
);

assert.deepEqual(
  decideConversationActivation(appSession, null),
  { kind: 'structured', nativeSessionMode: 'resume' },
  'a disconnected app session resumes through ACP'
);

assert.deepEqual(
  decideConversationActivation(
    { ...appSession, state: 'exited', executionOwner: 'stopped' },
    matchingConnection
  ),
  { kind: 'structured', nativeSessionMode: 'resume' },
  'a stopped app session resumes even when its retained frontend snapshot still says connected'
);

const stoppedExternal = {
  agent: 'codex',
  origin: 'external',
  state: 'background',
  executionOwner: 'stopped',
  ptySessionId: null,
  nativeSessionId: 'thread-external'
};

assert.deepEqual(
  decideConversationActivation(stoppedExternal, {
    provider: 'codex',
    state: 'connected',
    nativeSessionId: 'thread-external'
  }),
  { kind: 'structured', nativeSessionMode: 'load' },
  'a stopped external Codex session loads by native id instead of trusting a stale connection'
);

assert.deepEqual(
  decideConversationActivation(
    { ...stoppedExternal, state: 'live', executionOwner: 'terminal', ptySessionId: 'pty-live' },
    null
  ),
  { kind: 'terminal' },
  'a running external session stays terminal-owned'
);

assert.deepEqual(
  decideConversationActivation({ ...stoppedExternal, nativeSessionId: null }, null),
  { kind: 'terminal' },
  'external history without a native id keeps the terminal fallback'
);

const page = readFileSync(new URL('../src/routes/next/+page.svelte', import.meta.url), 'utf8');
assert.match(
  page,
  /console\.debug\('mcb next: conversation activation'/,
  'each activation attempt must be visible in future frontend logs'
);

console.log('conversationActivation.test.mjs passed');
