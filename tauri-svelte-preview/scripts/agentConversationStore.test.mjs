import assert from 'node:assert/strict';
import { readFileSync, rmSync, writeFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import { fileURLToPath } from 'node:url';
import { compileModule } from 'svelte/compiler';

const storePath = fileURLToPath(
  new URL('../src/lib/shell/conversation/conversationStore.svelte.ts', import.meta.url)
);
const outputPath = fileURLToPath(
  new URL('../src/lib/shell/conversation/.conversationStore.test.mjs', import.meta.url)
);

const source = readFileSync(storePath, 'utf8');
const javascript = stripTypeScriptTypes(source, { mode: 'strip' });
const compiled = compileModule(javascript, {
  generate: 'client',
  filename: 'conversationStore.svelte.js'
});
writeFileSync(outputPath, compiled.js.code);

let store;
try {
  store = await import(`${outputPath}?test=${Date.now()}`);
} finally {
  rmSync(outputPath, { force: true });
}

const a = store.ensureConversationSession('owned-a', 'codex');
const b = store.ensureConversationSession('owned-b', 'claude');
assert.notEqual(a, b);

store.setConversationDraft('owned-a', 'Message A');
store.setConversationDraft('owned-b', 'Message B');
store.setConversationMode('owned-b', 'raw');
assert.equal(store.getConversationSession('owned-a').draft, 'Message A');
assert.equal(store.getConversationSession('owned-a').mode, 'structured');
assert.equal(store.getConversationSession('owned-b').draft, 'Message B');
assert.equal(store.getConversationSession('owned-b').mode, 'raw');

store.applyConversationTranscript('owned-a', 'codex', {
  messages: [{ itemId: 'parent-a', role: 'assistant', text: 'Parent A', timestampMs: 1 }],
  metadata: {
    model: 'gpt-5.6-sol', effort: 'medium', approvalPolicy: 'never',
    usedTokens: 1000, contextWindow: 10000
  },
  children: [{
    childId: 'child-a', parentId: 'thread-a', provider: 'codex', label: 'Reviewer',
    state: 'active', updatedAtMs: 2
  }]
});
store.setConversationAttachments('owned-a', [{
  id: 'image-a', name: 'a.png', mimeType: 'image/png', path: '/managed/a.png', previewUrl: 'blob:a'
}]);
store.setConversationSelectedChild('owned-a', 'child-a');
store.setConversationScrollTop('owned-a', 240);
store.applyChildConversationTranscript('owned-a', 'child-a', [
  { itemId: 'child-message', role: 'assistant', text: 'Child A', timestampMs: 3 }
]);
assert.equal(store.getConversationSession('owned-a').metadata.model, 'gpt-5.6-sol');
assert.equal(store.getConversationSession('owned-a').children[0].label, 'Reviewer');
assert.equal(store.getConversationSession('owned-a').attachments[0].path, '/managed/a.png');
assert.equal(store.getConversationSession('owned-a').childTimeline[0].text, 'Child A');
assert.equal(store.getConversationSession('owned-a').scrollTop, 240);
assert.equal(store.getConversationSession('owned-b').metadata.model, null);
assert.equal(store.getConversationSession('owned-b').children.length, 0);
assert.equal(store.getConversationSession('owned-b').attachments.length, 0);

store.applyAgentConversationEvent({
  ownedId: 'owned-a',
  provider: 'codex',
  generation: 1,
  sequence: 1,
  timestampMs: 100,
  payload: { kind: 'assistantMessage', itemId: 'a-1', text: 'Only A', completed: true }
});
assert.equal(store.getConversationSession('owned-a').timeline.length, 2);
assert.equal(store.getConversationSession('owned-b').timeline.length, 0);

// A native snapshot repairs a missed-event gap and restores every message.
store.applyAgentConversationSnapshot({
  connection: {
    ownedId: 'owned-a',
    provider: 'codex',
    generation: 1,
    state: 'connected',
    nativeSessionId: 'thread-a'
  },
  lastSequence: 3,
  events: [
    {
      ownedId: 'owned-a', provider: 'codex', generation: 1, sequence: 1, timestampMs: 100,
      payload: { kind: 'connection', state: 'connected', nativeSessionId: 'thread-a' }
    },
    {
      ownedId: 'owned-a', provider: 'codex', generation: 1, sequence: 2, timestampMs: 110,
      payload: { kind: 'userMessage', itemId: 'user-1', text: 'Visible question', completed: true }
    },
    {
      ownedId: 'owned-a', provider: 'codex', generation: 1, sequence: 3, timestampMs: 120,
      payload: { kind: 'assistantMessage', itemId: 'assistant-1', text: 'Visible answer', completed: true }
    }
  ]
});
assert.deepEqual(
  store.getConversationSession('owned-a').timeline.map((item) => item.text),
  ['Visible question', 'Visible answer']
);
assert.equal(store.getConversationSession('owned-a').desynchronized, false);

const saved = store.captureConversationWorkspace('owned-b');
store.setConversationDraft('owned-b', 'Changed');
store.setConversationMode('owned-b', 'structured');
store.restoreConversationWorkspace('owned-b', 'claude', saved);
assert.equal(store.getConversationSession('owned-b').draft, 'Message B');
assert.equal(store.getConversationSession('owned-b').mode, 'raw');
assert.equal(store.getConversationSession('owned-b').selectedChildId, null);

store.removeConversationSession('owned-a');
assert.equal(store.getConversationSession('owned-a'), null);
assert.ok(store.getConversationSession('owned-b'));

console.log('agent conversation store tests passed');
