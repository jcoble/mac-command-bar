import assert from 'node:assert/strict';
import {
  conversationTurnGroups,
  displayItemFromAgentItem,
  displayItemsFromConversationEvents,
  formatWorkedFor,
  type ConversationDisplayItem,
  typedConversationTimeline
} from '../src/lib/shell/conversation/conversationTimeline.ts';
import { conversationItemHasVisibleContent } from '../src/lib/shell/conversation/conversationItemVisibility.ts';

const typed = typedConversationTimeline([
  { id: 'assistant-1', type: 'assistant-message', content: [{ channel: 'assistant', text: 'Answer' }] },
  { id: 'tool-1', type: 'mcp-tool', content: [], providerMetadata: { name: 'shell', state: 'completed' } },
  { id: 'plan-1', type: 'plan', content: [], providerMetadata: { title: 'Ship', steps: [{ id: 'step-1', title: 'Test', state: 'in-progress' }] } }
], [{ kind: 'user', itemId: 'user-1', text: 'Question', completed: true, timestampMs: 1 }]);

assert.deepEqual(typed.map((item) => item.kind), ['user', 'assistant', 'tool', 'plan']);
assert.equal(displayItemFromAgentItem({ id: 'reason-1', type: 'reasoning', content: [{ channel: 'reasoning', text: 'Think' }] }, 2).kind, 'reasoning');
assert.equal(conversationItemHasVisibleContent({ kind: 'assistant', itemId: 'empty-assistant', text: '  ', completed: false, timestampMs: 2 }), false);
assert.equal(conversationItemHasVisibleContent({ kind: 'reasoning', itemId: 'empty-reasoning', text: '', completed: false, timestampMs: 3 }), false);

const event = (sequence, payload, timestampMs = sequence) => ({
  ownedId: 'owned-rich',
  provider: 'codex',
  generation: 1,
  sequence,
  timestampMs,
  payload
});

const runningTool = displayItemsFromConversationEvents([
  event(1, { kind: 'toolCall', toolCallId: 'tool-command', title: 'pnpm test', toolKind: 'command', status: 'in_progress' })
]);
assert.equal(runningTool.length, 1, 'tool_call creates one visible row');
assert.equal(runningTool[0].kind, 'tool');
assert.equal(runningTool[0].state, 'running');

const completedTool = displayItemsFromConversationEvents([
  event(1, { kind: 'toolCall', toolCallId: 'tool-command', title: 'pnpm test', toolKind: 'command', status: 'in_progress' }),
  event(2, {
    kind: 'toolCallUpdate',
    toolCallId: 'tool-command',
    title: 'pnpm test',
    toolKind: 'command',
    status: 'completed',
    content: [{ type: 'terminal', output: 'PASS conversation timeline\n' }]
  })
]);
assert.equal(completedTool.length, 1, 'tool_call_update advances the existing row instead of adding JSON output');
assert.equal(completedTool[0].state, 'completed');
assert.equal(completedTool[0].output, 'PASS conversation timeline\n');

const reasoning = displayItemsFromConversationEvents([
  event(1, { kind: 'agentThoughtChunk', turnId: 'turn-1', messageId: 'reasoning-1', content: { type: 'text', text: 'Inspecting ' } }),
  event(2, { kind: 'agentThoughtChunk', turnId: 'turn-1', messageId: 'reasoning-1', content: { type: 'text', text: 'the files.' } }),
  event(3, { kind: 'assistantMessage', itemId: 'assistant-rich', text: 'Done', completed: true })
]);
const reasoningItems = reasoning.filter((item) => item.kind === 'reasoning');
assert.equal(reasoningItems.length, 1, 'thought chunks aggregate into one reasoning row');
assert.equal(reasoningItems[0].text, 'Inspecting the files.');

const permission = displayItemsFromConversationEvents([
  event(1, {
    kind: 'permissionRequest',
    requestId: 'permission-1',
    title: 'Approval needed',
    toolTitle: 'pnpm test',
    description: 'Run the focused test',
    options: [
      { optionId: 'allow_once', name: 'Allow once', kind: 'allow_once' },
      { optionId: 'allow_always', name: 'Always allow', kind: 'allow_always' },
      { optionId: 'reject_once', name: 'Reject', kind: 'reject_once' }
    ]
  })
]);
assert.equal(permission.length, 1, 'permission request creates one inline approval row');
assert.equal(permission[0].kind, 'approval');
assert.equal(permission[0].toolTitle, 'pnpm test');
assert.deepEqual(permission[0].options.map((option) => option.optionId), ['allow_once', 'allow_always', 'reject_once']);

const replayed = displayItemsFromConversationEvents([
  event(1, { kind: 'toolCall', toolCallId: 'replayed-tool', title: 'Read file', toolKind: 'fetch', status: 'in_progress', _meta: { replay: true } }),
  event(2, { kind: 'toolCallUpdate', toolCallId: 'replayed-tool', title: 'Read file', toolKind: 'fetch', status: 'completed', content: [{ type: 'text', text: 'contents' }], _meta: { replay: true } }),
  event(3, { kind: 'toolCall', toolCallId: 'replayed-tool', title: 'Read file', toolKind: 'fetch', status: 'in_progress', _meta: { replay: true } }),
  event(4, { kind: 'toolCallUpdate', toolCallId: 'replayed-tool', title: 'Read file', toolKind: 'fetch', status: 'completed', content: [{ type: 'text', text: 'contents' }], _meta: { replay: true } })
]);
assert.equal(replayed.length, 1, 'replayed items dedupe by itemId');
assert.equal(replayed[0].output, 'contents', 'replayed output is not duplicated');

const remainingRichKinds = displayItemsFromConversationEvents([
  event(1, { kind: 'plan', planId: 'plan-rich', entries: [{ id: 'step-1', title: 'Verify', status: 'in_progress' }] }),
  event(2, { kind: 'turnDiff', turnId: 'turn-rich', diff: '@@ -1 +1 @@\n-old\n+new' }),
  event(3, { kind: 'availableCommandsUpdate', availableCommands: [{ id: '/review', label: 'Review' }] })
]);
assert.deepEqual(remainingRichKinds.map((item) => item.kind), ['plan', 'tool', 'tool']);
assert.equal(remainingRichKinds[1].toolKind, 'file-edit');
assert.equal(remainingRichKinds[1].state, 'completed');
assert.equal(remainingRichKinds[1].diff, '@@ -1 +1 @@\n-old\n+new');

const textItem = (
  kind: 'user' | 'assistant' | 'reasoning',
  itemId: string,
  turnId: string | null,
  timestampMs: number,
  completed = true
): ConversationDisplayItem => ({ kind, itemId, turnId, text: itemId, timestampMs, completed });

const toolItem = (
  itemId: string,
  turnId: string | null,
  timestampMs: number,
  state: 'running' | 'completed' = 'completed'
): ConversationDisplayItem => ({
  kind: 'tool',
  itemId,
  turnId,
  title: itemId,
  toolKind: 'command',
  state,
  timestampMs
});

const partitioned = conversationTurnGroups([
  textItem('user', 'partition-user', 'partition-turn', 100),
  textItem('reasoning', 'partition-reasoning', 'partition-turn', 200),
  toolItem('partition-tool', 'partition-turn', 300),
  textItem('assistant', 'partition-tail', 'partition-turn', 600)
]);
assert.equal(partitioned.length, 1, 'one contiguous turn becomes one group');
assert.deepEqual(partitioned[0].workItemIds, ['partition-reasoning', 'partition-tool'], 'foldable work is partitioned from visible messages');
assert.deepEqual(partitioned[0].tailItemIds, ['partition-user', 'partition-tail'], 'user and final assistant messages remain visible');
assert.equal(partitioned[0].completed, true);
assert.equal(partitioned[0].elapsedMs, 500);

const interleaved = conversationTurnGroups([
  textItem('user', 'interleaved-user', 'interleaved-turn', 1),
  toolItem('interleaved-tool-a', 'interleaved-turn', 2),
  textItem('assistant', 'interleaved-commentary', 'interleaved-turn', 3),
  toolItem('interleaved-tool-b', 'interleaved-turn', 4),
  textItem('assistant', 'interleaved-tail-a', 'interleaved-turn', 5),
  textItem('assistant', 'interleaved-tail-b', 'interleaved-turn', 6)
]);
assert.deepEqual(
  interleaved[0].workItemIds,
  ['interleaved-tool-a', 'interleaved-commentary', 'interleaved-tool-b'],
  'assistant commentary between work rows folds with the work'
);
assert.deepEqual(
  interleaved[0].tailItemIds,
  ['interleaved-user', 'interleaved-tail-a', 'interleaved-tail-b'],
  'only the contiguous assistant run at the end is tail prose'
);

const noWork = conversationTurnGroups([
  textItem('user', 'plain-user', 'plain-turn', 1),
  textItem('assistant', 'plain-assistant', 'plain-turn', 2)
]);
assert.deepEqual(noWork[0].workItemIds, [], 'plain chat has no fold disclosure work');

const active = conversationTurnGroups([
  textItem('user', 'active-user', 'active-turn', 1),
  toolItem('active-tool', 'active-turn', 2)
], 'active-turn');
assert.equal(active[0].completed, false, 'the active turn never reports as completed');

const running = conversationTurnGroups([
  textItem('user', 'running-user', 'running-turn', 1),
  toolItem('running-tool', 'running-turn', 2, 'running')
]);
assert.equal(running[0].completed, false, 'a turn remains incomplete while any item runs');

// A transcript read back out of the store carries no turn ids, because the
// provider's own file never wrote any. Its turns are read off the prompts: one
// prompt opens a turn and holds everything until the next one. Grouping the
// whole conversation as a single turn — which is what a null id used to do —
// left a resumed session with one fold over a flat column of prose.
const stored = conversationTurnGroups([
  textItem('user', 'stored-user-a', null, 1),
  toolItem('stored-tool', null, 2),
  textItem('assistant', 'stored-answer-a', null, 3),
  textItem('user', 'stored-user-b', null, 4),
  textItem('assistant', 'stored-answer-b', null, 5)
]);
assert.equal(stored.length, 2, 'each prompt in a stored transcript opens its own turn');
assert.deepEqual(
  stored.map((group) => group.items.map((item) => item.itemId)),
  [['stored-user-a', 'stored-tool', 'stored-answer-a'], ['stored-user-b', 'stored-answer-b']]
);
assert.equal(stored[0].turnId, 'stored-turn:stored-user-a', 'a derived turn is named after the prompt that opened it');
assert.deepEqual(stored[0].workItemIds, ['stored-tool'], 'a derived turn folds its work like any other');

// Every row of an older import carries the moment the import ran, so the turn
// spans no time at all. That is not something to report as a duration.
const instant = conversationTurnGroups([
  textItem('user', 'instant-user', 'instant-turn', 7),
  textItem('assistant', 'instant-answer', 'instant-turn', 7)
]);
assert.equal(instant[0].elapsedMs, null, 'a turn that spans no time reports no elapsed time');

assert.equal(formatWorkedFor(800), '0.8s');
assert.equal(formatWorkedFor(5_540), '5.5s');
assert.equal(formatWorkedFor(42_100), '42s');
assert.equal(formatWorkedFor(1_150_000), '19m 10s');

// An error arrives once and must be shown once, with what it said. The typed
// item used to be built by the generic path, which reads `text` and never
// `message`, so a resume failure drew its real card and a second empty one.
const resumeFailure = displayItemsFromConversationEvents([
  event(4, {
    kind: 'error',
    code: 'session-resume-failed',
    message: 'The stored provider session could not be resumed: no rollout found',
    recoverable: true
  })
]);
assert.equal(resumeFailure.length, 1, 'one error event draws one card');
assert.equal(resumeFailure[0].kind, 'error');
assert.equal(
  resumeFailure[0].text,
  'The stored provider session could not be resumed: no rollout found'
);

// The reducer names its entry `error:<generation>:<sequence>`; the typed item
// has to use the same name or the two become separate cards.
const mergedError = typedConversationTimeline(
  [{ id: 'error:1:4', type: 'error', content: [{ channel: 'assistant', text: 'Adapter closed' }] }],
  [
    {
      kind: 'error',
      itemId: 'error:1:4',
      code: 'session-resume-failed',
      message: 'Adapter closed',
      recoverable: true,
      timestampMs: 4
    }
  ]
);
assert.equal(mergedError.length, 1, 'the typed item and the reducer entry are one card');

// An error with no message still says something a reader can act on.
const silentError = displayItemsFromConversationEvents([
  event(5, { kind: 'error', code: 'session-resume-failed', message: '', recoverable: true })
]);
assert.equal(silentError.length, 1);
assert.ok((silentError[0].text ?? '').trim().length > 0, 'an error card is never empty');

// The sent screenshot is carried onto the user card it was sent with.
const withAttachments = typedConversationTimeline(
  [],
  [{ kind: 'user', itemId: 'user-shot', text: 'Look', completed: true, timestampMs: 1 }],
  {},
  [],
  {
    'user-shot': [{
      id: 'image-sent', name: 'shot.png', mimeType: 'image/png',
      path: '/managed/shot.png', previewUrl: 'blob:sent'
    }]
  }
);
assert.equal(withAttachments.length, 1);
assert.deepEqual(
  (withAttachments[0].attachments ?? []).map((item) => item.name),
  ['shot.png'],
  'the user card renders the screenshot that went out with it'
);
assert.equal(
  typedConversationTimeline(
    [],
    [{ kind: 'user', itemId: 'user-plain', text: 'Hi', completed: true, timestampMs: 1 }]
  )[0].attachments,
  undefined,
  'a message sent without a screenshot gains no attachment field'
);

console.log('conversationTimeline.test.ts passed');
