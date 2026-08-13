import assert from 'node:assert/strict';
import {
  displayItemFromAgentItem,
  displayItemsFromConversationEvents,
  typedConversationTimeline
} from '../src/lib/shell/conversation/conversationTimeline.ts';

const typed = typedConversationTimeline([
  { id: 'assistant-1', type: 'assistant-message', content: [{ channel: 'assistant', text: 'Answer' }] },
  { id: 'tool-1', type: 'mcp-tool', content: [], providerMetadata: { name: 'shell', state: 'completed' } },
  { id: 'plan-1', type: 'plan', content: [], providerMetadata: { title: 'Ship', steps: [{ id: 'step-1', title: 'Test', state: 'in-progress' }] } }
], [{ kind: 'user', itemId: 'user-1', text: 'Question', completed: true, timestampMs: 1 }]);

assert.deepEqual(typed.map((item) => item.kind), ['user', 'assistant', 'tool', 'plan']);
assert.equal(displayItemFromAgentItem({ id: 'reason-1', type: 'reasoning', content: [{ channel: 'reasoning', text: 'Think' }] }, 2).kind, 'reasoning');

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

console.log('conversationTimeline.test.ts passed');
