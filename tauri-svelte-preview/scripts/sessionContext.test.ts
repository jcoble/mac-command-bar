import assert from 'node:assert/strict';

import type {
  AgentConversationEvent,
  ConversationMetadata
} from '../src/lib/shell/conversation/conversationTypes.ts';
import {
  sessionContextFacts,
  sessionContextUsage,
  sessionFilesTouched
} from '../src/lib/shell/panels/context/sessionContextModel.ts';

const metadata = (overrides: Partial<ConversationMetadata> = {}): ConversationMetadata => ({
  model: null,
  effort: null,
  approvalPolicy: null,
  usedTokens: null,
  contextWindow: null,
  ...overrides
});

// Nothing selected: three facts, none of them claiming to know anything.
const noSession = sessionContextFacts(null);
assert.equal(noSession.length, 3, 'the panel always shows the same three facts');
assert.deepEqual(
  noSession.map((fact) => fact.missing),
  [true, true, true],
  'with no session every fact is missing'
);
assert.deepEqual(
  noSession.map((fact) => fact.label),
  ['Model', 'Effort', 'Access mode'],
  'the three facts are model, effort and access mode'
);

// A partial report stays partial: what was reported shows, what was not says so.
const partial = sessionContextFacts(metadata({ model: 'sample-model' }));
assert.equal(partial[0].value, 'sample-model');
assert.equal(partial[0].missing, false, 'a reported model is not missing');
assert.equal(partial[1].missing, true, 'an unreported effort stays missing');
assert.equal(partial[2].missing, true, 'an unreported access mode stays missing');

// A percentage needs both halves of the pair. Half a pair yields nothing.
const usedOnly = sessionContextUsage(metadata({ usedTokens: 12_000 }), null);
assert.equal(usedOnly.usedTokens, 12_000);
assert.equal(usedOnly.contextWindow, null);
assert.equal(usedOnly.percentUsed, null, 'a used count alone cannot make a percentage');

const windowOnly = sessionContextUsage(metadata({ contextWindow: 200_000 }), null);
assert.equal(windowOnly.usedTokens, null);
assert.equal(windowOnly.contextWindow, 200_000);
assert.equal(windowOnly.percentUsed, null, 'a window alone cannot make a percentage');

const bothReported = sessionContextUsage(metadata({ usedTokens: 50_000, contextWindow: 200_000 }), {
  inputTokens: 40_000,
  outputTokens: 10_000
});
assert.equal(bothReported.percentUsed, 25, 'both numbers real gives a real percentage');
assert.equal(bothReported.inputTokens, 40_000);
assert.equal(bothReported.outputTokens, 10_000);

const emptyUsage = sessionContextUsage(null, null);
assert.deepEqual(emptyUsage, {
  usedTokens: null,
  contextWindow: null,
  percentUsed: null,
  inputTokens: null,
  outputTokens: null
});

// Files touched, derived from whatever the provider happened to put on its tool calls.
const toolEvent = (
  sequence: number,
  timestampMs: number,
  payload: Record<string, unknown>
): AgentConversationEvent =>
  ({
    ownedId: 'session-one',
    generation: 1,
    sequence,
    timestampMs,
    payload
  }) as unknown as AgentConversationEvent;

const touched = sessionFilesTouched([
  toolEvent(1, 1_000, { kind: 'toolCall', toolCallId: 'a', path: 'src/one.ts' }),
  toolEvent(2, 2_000, { kind: 'toolCallUpdate', toolCallId: 'a', path: 'src/one.ts' }),
  toolEvent(3, 3_000, { kind: 'toolCall', toolCallId: 'b', path: 'src/one.ts' }),
  toolEvent(4, 4_000, { kind: 'toolCall', toolCallId: 'c' }),
  toolEvent(5, 5_000, { kind: 'agentThoughtChunk', text: 'src/never.ts' }),
  toolEvent(6, 6_000, {
    kind: 'toolCall',
    toolCallId: 'd',
    locations: [{ path: 'src/two.ts', line: 12 }]
  })
]);

assert.equal(touched.length, 2, 'only the two real paths survive');
assert.equal(touched[0].path, 'src/two.ts', 'the most recently touched path sorts first');
assert.equal(touched[0].count, 1);
assert.equal(touched[0].lastTouchedMs, 6_000);
assert.equal(touched[1].path, 'src/one.ts');
assert.equal(touched[1].count, 3, 'three references to one path collapse into one entry');
assert.equal(touched[1].lastTouchedMs, 3_000, 'the entry keeps its most recent timestamp');

assert.deepEqual(sessionFilesTouched([]), [], 'no events means no files');

// A session that has touched hundreds of files still shows only the most recent 50.
const manyEvents = Array.from({ length: 60 }, (_, index) =>
  toolEvent(index, index * 1_000, { kind: 'toolCall', toolCallId: `t${index}`, path: `src/file-${index}.ts` })
);
const capped = sessionFilesTouched(manyEvents);
assert.equal(capped.length, 50, 'the list stops at 50 files even when more were touched');
assert.equal(capped[0].path, 'src/file-59.ts', 'the cap keeps the most recently touched files');
assert.equal(capped[49].path, 'src/file-10.ts', 'the cap drops the oldest touches first');

console.log('session context tests passed');
