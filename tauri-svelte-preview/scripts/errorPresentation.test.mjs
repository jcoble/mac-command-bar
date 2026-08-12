import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

import { presentAgentError } from '../src/lib/shell/errorPresentation.ts';

const mappedCases = [
  {
    name: 'missing rollout history',
    raw: 'acp-error: {"code":-32600,"message":"no rollout found for thread id 019fe7e2"}',
    summary: "This session's history is missing, so it can't be resumed.",
    detail: 'acp-error:\n{\n  "code": -32600,\n  "message": "no rollout found for thread id 019fe7e2"\n}'
  },
  {
    name: 'invalid session response',
    raw: 'invalid-response: ACP session response did not include sessionId',
    summary: 'The agent sent an unexpected reply.'
  },
  {
    name: 'session id omitted without a code prefix',
    raw: 'ACP session response did not include sessionId',
    summary: 'The agent sent an unexpected reply.'
  },
  {
    name: 'session not started',
    raw: 'session-not-started: ACP session has not started',
    summary: "The session hasn't connected yet."
  },
  {
    name: 'transport failure',
    raw: 'transport: pending request map is unavailable',
    summary: 'The connection to the agent was interrupted.'
  },
  {
    name: 'runtime transport event',
    raw: 'acp-transport: connection reset by peer',
    summary: 'The connection to the agent was interrupted.'
  },
  {
    name: 'closed transport',
    raw: 'transport: transport closed before response',
    summary: 'The agent connection closed.'
  },
  {
    name: 'exited agent process',
    raw: 'transport: agent process exited with status 1',
    summary: 'The agent stopped unexpectedly.'
  },
  {
    name: 'sidecar spawn failure',
    raw: 'sidecar-spawn: failed to start adapter process',
    summary: "The agent couldn't start."
  },
  {
    name: 'uninitialized adapter',
    raw: 'not-initialized: ACP adapter is not initialized',
    summary: "The agent hasn't finished starting."
  },
  {
    name: 'invalid capabilities',
    raw: 'invalid-capabilities: image prompts require protocol support',
    summary: 'The agent reported unsupported features.'
  },
  {
    name: 'invalid configuration',
    raw: 'invalid-config: unknown model option',
    summary: "The agent couldn't apply that setting."
  },
  {
    name: 'serialization failure',
    raw: 'serialization: unsupported value',
    summary: "The app couldn't prepare the agent request."
  },
  {
    name: 'duplicate inbound consumer',
    raw: 'inbound-already-taken: ACP inbound events already have a consumer',
    summary: 'The agent connection is already in use.'
  },
  {
    name: 'empty response',
    raw: 'empty-response: The active agent returned no text',
    summary: 'The agent replied without any text.'
  },
  {
    name: 'generic ACP request error',
    raw: 'acp-error: permission denied',
    summary: "The agent couldn't complete that request."
  },
  {
    name: 'authentication required',
    raw: 'acp-error: {"code":-32000,"message":"Authentication required"}',
    summary: 'Agent login required. Run `claude /login` in a terminal, then try again.',
    detail: 'acp-error:\n{\n  "code": -32000,\n  "message": "Authentication required"\n}'
  },
  {
    name: 'authentication prompt from the adapter',
    raw: 'acp-error: Please run /login before continuing',
    summary: 'Agent login required. Run `claude /login` in a terminal, then try again.'
  }
];

for (const example of mappedCases) {
  test(example.name, () => {
    const presented = presentAgentError(example.raw);
    assert.equal(presented.summary, example.summary);
    assert.equal(presented.detail, example.detail ?? example.raw);
    assert.doesNotMatch(presented.summary, /\{|-?\d{3,}|^[a-z]+(?:-[a-z]+)+:/i);
  });
}

test('unknown errors use a safe summary and retain their detail', () => {
  const raw = 'provider-meltdown: impossible state 47';
  assert.deepEqual(presentAgentError(raw), {
    summary: 'The agent hit an error.',
    detail: raw
  });
});

test('embedded JSON is pretty-printed in detail without leaking into the summary', () => {
  const raw = 'acp-error: {"code":-32600,"message":"no rollout found for thread id 019fe7e2"}';
  const presented = presentAgentError(raw);

  assert.equal(presented.summary, "This session's history is missing, so it can't be resumed.");
  assert.equal(
    presented.detail,
    'acp-error:\n{\n  "code": -32600,\n  "message": "no rollout found for thread id 019fe7e2"\n}'
  );
});

test('an empty error still has a human summary and no empty detail', () => {
  assert.deepEqual(presentAgentError(''), { summary: 'The agent hit an error.' });
});

test('session rows render summaries and keep raw detail behind disclosure controls', async () => {
  const [sessionCard, worktreeRow] = await Promise.all([
    readFile(new URL('../src/lib/shell/components/sessions/SessionCard.svelte', import.meta.url), 'utf8'),
    readFile(new URL('../src/lib/shell/components/WorktreeAgentRow.svelte', import.meta.url), 'utf8')
  ]);

  for (const source of [sessionCard, worktreeRow]) {
    assert.match(source, /presentAgentError\(session\.lastError\)/);
    assert.doesNotMatch(source, /\{session\.lastError\}|title=\{session\.lastError\}/);
    assert.match(source, /presentedError\.summary/);
    assert.match(source, /<details[\s\S]*presentedError\.detail/);
  }
});
