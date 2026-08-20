import assert from 'node:assert/strict';
import {
  conversationTurnGroups,
  displayItemFromAgentItem,
  displayItemsFromConversationEvents,
  diffLineCounts,
  formatWorkedFor,
  latestPlan,
  turnFileChanges,
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
assert.equal(conversationItemHasVisibleContent({
  kind: 'user',
  itemId: 'image-only-user',
  text: '',
  completed: true,
  timestampMs: 4,
  attachments: [{ id: 'image-1', name: 'image.png', mimeType: 'image/png', path: '/image.png', previewUrl: 'asset://image.png' }]
}), true, 'a user message with only an attachment remains visible');
assert.equal(conversationItemHasVisibleContent({ kind: 'user', itemId: 'empty-user', text: '', completed: true, timestampMs: 5 }), false, 'a truly empty user message stays hidden');

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

const classifiedTools = [
  { kind: 'command', title: 'rg --files', expected: 'command' },
  { kind: 'command', title: 'git diff HEAD', expected: 'command' },
  { kind: 'file_change', title: 'Edit', expected: 'file-edit' }
];
for (const [index, testCase] of classifiedTools.entries()) {
  const [item] = displayItemsFromConversationEvents([
    event(20 + index, { kind: 'toolCall', toolCallId: `classified-${index}`, title: testCase.title, toolKind: testCase.kind })
  ]);
  assert.equal(item.toolKind, testCase.expected, `${testCase.kind} ignores title text`);
}

const objectTitle = displayItemsFromConversationEvents([
  event(24, { kind: 'toolCall', toolCallId: 'object-title', title: { path: 'src/readable.ts' }, toolKind: 'command' })
])[0];
assert.equal(objectTitle.title, 'src/readable.ts', 'an object title uses its readable path');

const editWithoutDiff = displayItemsFromConversationEvents([
  event(25, { kind: 'toolCall', toolCallId: 'edit-no-diff', title: 'Edit', toolKind: 'file_change', output: 'whole file contents' })
])[0];
assert.equal(editWithoutDiff.diff, undefined, 'an edit without a diff does not show output as a patch');
assert.equal(editWithoutDiff.output, undefined, 'an edit without a diff has no dumped body');

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

// A streamed reply is opened and never closed: the provider sends the text in
// chunks and no event afterwards says the message ended, so every assistant
// row in a finished transcript still reads as unfinished. A turn is finished
// when nothing in it is still waiting, not when every row says it stopped
// writing.
const streamedReply = conversationTurnGroups([
  textItem('user', 'streamed-user', 'streamed-turn', 0),
  toolItem('streamed-tool-a', 'streamed-turn', 60_000),
  toolItem('streamed-tool-b', 'streamed-turn', 120_000),
  textItem('assistant', 'streamed-answer', 'streamed-turn', 840_000, false)
]);
assert.equal(streamedReply.length, 1, 'a finished turn is one group');
assert.equal(streamedReply[0].completed, true, 'a finished turn folds even though its reply still reads as streaming');
assert.equal(streamedReply[0].elapsedMs, 840_000);
assert.equal(formatWorkedFor(840_000), '14m 0s');
assert.deepEqual(
  streamedReply[0].workItemIds,
  ['streamed-tool-a', 'streamed-tool-b'],
  'the tool calls are what the fold hides'
);
assert.deepEqual(
  streamedReply[0].tailItemIds,
  ['streamed-user', 'streamed-answer'],
  'the prompt and the reply stay on screen while the turn is collapsed'
);

// A compaction marker is recorded by the app rather than by the agent, so it
// arrives without the turn id the rows around it carry. It belongs to the turn
// it interrupts; treating it as the start of another one cut the turn in two
// and lost the fold over both halves.
const compacted = conversationTurnGroups([
  textItem('user', 'compacted-user', 'compacted-turn', 0),
  toolItem('compacted-tool-a', 'compacted-turn', 60_000),
  { kind: 'compaction', itemId: 'compacted-marker', turnId: null, timestampMs: 120_000 },
  toolItem('compacted-tool-b', 'compacted-turn', 180_000),
  textItem('assistant', 'compacted-answer', 'compacted-turn', 840_000, false)
]);
assert.equal(compacted.length, 1, 'a compaction marker stays inside the turn it interrupts');
assert.equal(compacted[0].completed, true, 'the turn still folds around the marker');
assert.deepEqual(
  compacted[0].workItemIds,
  ['compacted-tool-a', 'compacted-tool-b'],
  'both halves of the interrupted turn fold together'
);

// Nothing folds while the agent is still writing. Most providers put no turn id
// on the rows they send, so the running turn cannot be found by matching ids —
// it is the newest turn, and there is a running turn only while one is named.
const writing = conversationTurnGroups([
  textItem('user', 'writing-user-a', null, 0),
  toolItem('writing-tool-a', null, 1_000),
  textItem('assistant', 'writing-answer-a', null, 2_000, false),
  textItem('user', 'writing-user-b', null, 3_000),
  toolItem('writing-tool-b', null, 4_000),
  textItem('assistant', 'writing-answer-b', null, 5_000, false)
], 'turn-the-agent-is-writing');
assert.equal(writing.length, 2);
assert.equal(writing[0].completed, true, 'an earlier turn folds while a later one runs');
assert.equal(writing[1].completed, false, 'the turn being written never folds');

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

// A sub-agent's progress report is a fact about the agent tree, not a row of
// the transcript. Each one used to fall through as an empty "unknown" card and
// draw a bare left border, so a turn that ran sub-agents opened onto a column
// of blank bars.
const childProgress = displayItemsFromConversationEvents([
  event(1, { kind: 'assistantDelta', itemId: 'msg-1', delta: 'Splitting the work.' }),
  event(2, { kind: 'childUpdate', childId: 'child-a', parentToolCallId: 'tool-1', label: 'rust_acp_lifecycle', state: 'running', latestActivity: 'Running' }),
  event(3, { kind: 'childUpdate', childId: 'child-b', parentToolCallId: 'tool-1', label: 'svelte_message_flow', state: 'running', latestActivity: 'Running' }),
  event(4, { kind: 'assistantDelta', itemId: 'msg-2', delta: 'Both are back.' })
]);
assert.deepEqual(childProgress.map((item) => item.kind), ['assistant', 'assistant'], 'child updates draw no row of their own');
assert.equal(
  conversationItemHasVisibleContent({ kind: 'unknown', itemId: 'empty-unknown', text: '', timestampMs: 5 }),
  false,
  'an unknown item with nothing to say draws nothing'
);

// Where the agent threw the older part of the conversation away. Both
// providers can say so, and the reply after it reads as though it forgot what
// came before unless the transcript marks the boundary.
const compaction = displayItemsFromConversationEvents([
  event(1, { kind: 'assistantDelta', itemId: 'msg-1', delta: 'Working.' }),
  event(2, { kind: 'contextCompaction', trigger: 'auto', preTokens: 351238, postTokens: 22202 })
]);
assert.deepEqual(compaction.map((item) => item.kind), ['assistant', 'compaction']);
assert.deepEqual(
  compaction.filter((item) => item.kind === 'compaction').map((item) => [item.trigger, item.preTokens, item.postTokens]),
  [['auto', 351238, 22202]]
);
assert.equal(
  conversationItemHasVisibleContent({ kind: 'compaction', itemId: 'compaction:2', timestampMs: 2 }),
  true,
  'a compaction is a boundary, so it draws even carrying no text'
);
// Codex records that it happened and nothing about what it cost.
const bareCompaction = displayItemsFromConversationEvents([event(1, { kind: 'contextCompaction' })]);
assert.equal(bareCompaction.length, 1);
assert.equal(bareCompaction[0].kind, 'compaction');
assert.equal(bareCompaction[0].preTokens, undefined);

// ── Tool row titles strip fences (tool_title_strips_fences) ──────────────
// Some providers stuff a fenced block into the title field instead of a
// separate summary; a row should not print raw fence markers.
const fencedTitleTool = displayItemFromAgentItem({
  id: 'tool-fenced',
  type: 'mcp-tool',
  content: [],
  providerMetadata: { title: 'Tool ```console\nls -la\n```' }
});
assert.equal(fencedTitleTool.title, 'ls -la', 'the title is the first fenced line, without fence text');
assert.equal(fencedTitleTool.summary, undefined, 'the fenced line is not repeated beneath the title');

const blankFenceTool = displayItemFromAgentItem({
  id: 'tool-blank-fence',
  type: 'mcp-tool',
  content: [],
  providerMetadata: { title: '```\n\n```' }
});
assert.equal(blankFenceTool.title, 'Tool', 'an empty fence falls back to a plain title');
assert.equal(blankFenceTool.summary, undefined, 'an empty fence is not rendered as summary text');

// ── The plan chip reads one newest plan (latest_plan_across_turns) ───────
// The chip above the composer shows the plan the session is working to, so it
// needs the newest one no matter which turn produced it.
const planAcrossTurns = typedConversationTimeline([
  {
    id: 'plan-first',
    type: 'plan',
    turnId: 'turn-1',
    content: [],
    providerMetadata: { title: 'Plan', steps: [{ id: 'one', title: 'Read the store', state: 'in_progress' }] }
  },
  {
    id: 'plan-second',
    type: 'plan',
    turnId: 'turn-2',
    content: [],
    providerMetadata: {
      title: 'Plan',
      steps: [
        { id: 'one', title: 'Read the store', state: 'completed' },
        { id: 'two', title: 'Write the page', state: 'in_progress' }
      ]
    }
  }
]);
assert.equal(latestPlan(planAcrossTurns)?.itemId, 'plan-second', 'the newest plan wins across turns');
assert.equal(latestPlan(planAcrossTurns)?.steps.length, 2);
assert.equal(latestPlan([]), null, 'a transcript with no plan has no plan');

const noPlanFileChanges = turnFileChanges([
  textItem('user', 'no-plan-user', 'no-plan-turn', 1),
  {
    kind: 'tool', itemId: 'no-plan-edit', turnId: 'no-plan-turn', title: 'Edited a file',
    toolKind: 'file-edit', state: 'completed', path: 'src/changed.ts',
    diff: '@@ -1 +1 @@\n-old\n+new', timestampMs: 2
  }
]);
assert.deepEqual(noPlanFileChanges, { files: 1, added: 1, removed: 1 }, 'file changes produce chip data without a plan');

// ── Repeated plan updates draw one line (duplicate_plan_updates_collapse) ─
// A plan update arrives without an id of its own, so every one of them lands
// on a row of its own and an unchanged plan printed itself twice. Two updates
// carrying the same steps are one update.
const planSteps = [
  { id: 'one', title: 'Read the store', state: 'completed' },
  { id: 'two', title: 'Write the page', state: 'pending' }
];
const repeatedPlan = typedConversationTimeline([
  { id: 'plan-repeat-1', type: 'plan', content: [], providerMetadata: { title: 'Plan', steps: planSteps } },
  { id: 'plan-repeat-2', type: 'plan', content: [], providerMetadata: { title: 'Plan', steps: planSteps } }
]);
assert.deepEqual(
  repeatedPlan.filter((item) => item.kind === 'plan').map((item) => item.itemId),
  ['plan-repeat-1'],
  'an unchanged plan update is dropped'
);

const movedPlan = typedConversationTimeline([
  { id: 'plan-moved-1', type: 'plan', content: [], providerMetadata: { title: 'Plan', steps: planSteps } },
  {
    id: 'plan-moved-2',
    type: 'plan',
    content: [],
    providerMetadata: {
      title: 'Plan',
      steps: [
        { id: 'one', title: 'Read the store', state: 'completed' },
        { id: 'two', title: 'Write the page', state: 'in_progress' }
      ]
    }
  }
]);
assert.deepEqual(
  movedPlan.filter((item) => item.kind === 'plan').map((item) => item.itemId),
  ['plan-moved-1', 'plan-moved-2'],
  'a plan that moved on keeps its own line'
);

// ── A diff counts its own lines (diff_line_counts) ───────────────────────
// The chip says how much the turn changed. The `+++`/`---` lines name the
// file rather than change a line in it, so they are not part of the count.
const counted = diffLineCounts(
  'diff --git a/one.ts b/one.ts\n--- a/one.ts\n+++ b/one.ts\n@@ -1,2 +1,3 @@\n context\n+added one\n+added two\n-removed one\n'
);
assert.deepEqual(counted, { added: 2, removed: 1 }, 'file headers are not changed lines');
assert.deepEqual(diffLineCounts(''), { added: 0, removed: 0 }, 'nothing changed counts as nothing');
// A file header only appears before the first hunk. Inside a hunk, a line of
// three dashes is a removed line that started with a comment marker.
assert.deepEqual(
  diffLineCounts('--- a/notes.sql\n+++ b/notes.sql\n@@ -1,2 +1,2 @@\n--- note\n+++ more\n'),
  { added: 1, removed: 1 },
  'dashes inside a hunk are changed lines, not headers'
);
