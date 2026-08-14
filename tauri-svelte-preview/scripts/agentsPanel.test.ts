/**
 * Agents panel contract checks.
 *
 * The model is a pure module, so it runs here directly. The panel itself is a
 * Svelte component, so the few rules that matter about it — the empty-state
 * wording, the honest hint on a log link that cannot work yet, and the absence
 * of any workflow authoring — are checked against its source text, the same
 * way `centerDock.test.mjs` guards the center layout.
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

import {
  agentActivityRows,
  agentStatus
} from '../src/lib/shell/panels/agents/agentActivityModel.ts';
import type {
  ConversationChildAgent,
  ConversationTimelineEntry
} from '../src/lib/shell/conversation/conversationTypes.ts';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

// ── Status mapping ───────────────────────────────────────────────────────────
assert.equal(agentStatus('active'), 'working');
assert.equal(agentStatus('running'), 'working');
assert.equal(agentStatus('failed'), 'failed');
assert.equal(agentStatus('error'), 'failed');
assert.equal(agentStatus('completed'), 'done');
assert.equal(agentStatus('COMPLETED'), 'done', 'the provider string is matched without regard to case');
assert.equal(agentStatus('historical'), 'idle');
assert.equal(agentStatus('something nobody has seen'), 'idle');
assert.equal(agentStatus(''), 'idle');

// ── Rows ─────────────────────────────────────────────────────────────────────
function child(overrides: Partial<ConversationChildAgent>): ConversationChildAgent {
  return {
    childId: 'child-1',
    parentId: 'parent-1',
    provider: 'codex',
    label: 'Reader',
    state: 'active',
    updatedAtMs: 1,
    ...overrides
  };
}

assert.deepEqual(agentActivityRows([], {}), [], 'no children means no rows');

const sorted = agentActivityRows(
  [
    child({ childId: 'older', updatedAtMs: 10 }),
    child({ childId: 'newest', updatedAtMs: 30 }),
    child({ childId: 'middle', updatedAtMs: 20 })
  ],
  {}
);
assert.deepEqual(
  sorted.map((row) => row.childId),
  ['newest', 'middle', 'older'],
  'most recently updated first'
);

const unread = agentActivityRows([child({ childId: 'unread' })], {})[0];
assert.equal(unread.messageCount, null, 'an agent whose transcript was never read shows no count, not zero');
assert.notEqual(unread.messageCount, 0);
assert.equal(unread.logPath, null, 'no log path exists on a child agent today, so the log link stays disabled');
assert.equal(unread.label, 'Reader');
assert.equal(unread.status, 'working');
assert.equal(unread.activity, '', 'nothing is known about an unread agent beyond its status');

const liveUnread = agentActivityRows([child({
  childId: 'live-unread',
  latestActivity: 'Reviewing the conversation store'
})], {})[0];
assert.equal(liveUnread.activity, 'Reviewing the conversation store', 'live activity is shown before a child transcript is read');

const message = (itemId: string, text: string, timestampMs: number): ConversationTimelineEntry => ({
  kind: 'assistant',
  itemId,
  text,
  completed: true,
  timestampMs
});

const read = agentActivityRows([child({ childId: 'read' })], {
  read: [message('a', 'One', 1), message('b', 'Two', 2), message('c', 'Three', 3)]
})[0];
assert.equal(read.messageCount, 3, 'a read transcript of three entries counts three');
assert.equal(read.activity, 'Three', 'the activity line comes from the newest entry');

const instructionOnly = agentActivityRows([child({
  childId: 'instruction-only',
  latestActivity: 'Starting work'
})], {
  'instruction-only': [{
    kind: 'user',
    itemId: 'instructions',
    text: '<recommended_plugins>\n  Internal setup details',
    completed: true,
    timestampMs: 1
  }]
})[0];
assert.equal(instructionOnly.activity, 'Starting work', 'instruction text does not replace stored live activity');
assert.doesNotMatch(instructionOnly.activity, /recommended_plugins/);

const assistantAfterInstructions = agentActivityRows([child({ childId: 'assistant-after-instructions' })], {
  'assistant-after-instructions': [
    {
      kind: 'user',
      itemId: 'instructions',
      text: '<instructions>Ignore this context</instructions>',
      completed: true,
      timestampMs: 1
    },
    message('assistant-work', '<status>  Reviewing   the\nactivity line  </status>', 2)
  ]
})[0];
assert.equal(assistantAfterInstructions.activity, 'Reviewing the activity line', 'later assistant work is sanitized and shown');

const emptyTranscript = agentActivityRows([child({ childId: 'empty' })], { empty: [] })[0];
assert.equal(emptyTranscript.messageCount, 0, 'a transcript that was read and holds nothing really is zero');

const toolRow = agentActivityRows([child({ childId: 'tool-user' })], {
  'tool-user': [{ kind: 'tool', itemId: 't1', name: 'Read file', state: 'started', timestampMs: 4 }]
})[0];
assert.equal(toolRow.activity, 'Running Read file');

// A count read for one agent never leaks onto another.
const mixed = agentActivityRows([child({ childId: 'a', updatedAtMs: 2 }), child({ childId: 'b', updatedAtMs: 1 })], {
  a: [message('a', 'Only mine', 1)]
});
assert.equal(mixed[0].messageCount, 1);
assert.equal(mixed[1].messageCount, null);

// ── The panel's own copy and its honest gaps ─────────────────────────────────
const panelSource = readFileSync(path.join(root, 'src/lib/shell/panels/agents/AgentsPanel.svelte'), 'utf8');
assert.match(panelSource, /No agents yet/, 'the empty state uses the agreed title');
assert.match(
  panelSource,
  /When this thread spawns subagents or runs a workflow, they show up here with live status, activity, and token usage\./,
  'the empty state uses the agreed body copy'
);
assert.match(
  panelSource,
  /let \{ visible, ownedId \}: Props = \$props\(\)/,
  'the panel consumes the visibility signal passed by the right panel'
);
assert.match(
  panelSource,
  /if \(!visible \|\| !ownedId \|\| !conversation \|\| !selectedChildId\) return;/,
  'transcript polling requires a visible panel and a selected child'
);
assert.match(
  panelSource,
  /child\.childId === selectedChildId && agentStatus\(child\.state\) === 'working'/,
  'the polling state uses the same running-state mapping as the roster row'
);
assert.match(
  panelSource,
  /if \(!selectedChildRunning\) return;/,
  'transcript polling requires the selected child to still be running'
);
assert.match(panelSource, /window\.setInterval\([\s\S]*10_000\)/, 'the running transcript is refreshed every ten seconds');
assert.match(
  panelSource,
  /return \(\) => window\.clearInterval\(timer\);/,
  'the polling effect clears its interval when any prerequisite changes or the panel unmounts'
);
const rowSource = readFileSync(path.join(root, 'src/lib/shell/panels/agents/AgentRow.svelte'), 'utf8');
assert.match(rowSource, /This agent's log is not available yet\./, 'the log link explains why it cannot be pressed');

for (const source of [panelSource, rowSource]) {
  assert.doesNotMatch(source, /:has\(|has-\[/, 'the shell forbids :has() selectors');
  assert.doesNotMatch(source, /WorkflowTemplateEditor|startWorkflowRun|WorkflowControlCenter/, 'workflow authoring is out of scope');
}

console.log('agents panel tests passed');
