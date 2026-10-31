// Run against the actual Rust projector output:
// AGUI_FIXTURE_OUT=<folder> cargo test a_running_call_keeps
// node --experimental-strip-types scripts/eagerToolProjection.test.ts <folder>/eager-tools.jsonl
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { StreamProcessor } from '@tanstack/ai/client';
import { conversationDisplayItems } from '../src/lib/shell/conversation/conversationMessages.ts';
import { foldFileEdits } from '../src/lib/shell/conversation/conversationTimeline.ts';

test('the Rust history projection keeps an earlier diff beside a later output', () => {
  const path = process.argv[2];
  assert.ok(path, 'Pass eager-tools.jsonl emitted by the Rust behavior test');
  const processor = new StreamProcessor();
  for (const line of readFileSync(path, 'utf8').trim().split('\n')) processor.processChunk(JSON.parse(line));
  const items = conversationDisplayItems(processor.getMessages());
  assert.equal(items.length, 1);
  const tool = items[0];
  assert.equal(tool.kind, 'tool');
  const call = processor.getMessages()[0].parts.find((part) => part.type === 'tool-call');
  assert.equal((call?.output as { output?: string })?.output, '2 files');
  assert.equal(tool.path, 'notes.txt');
  assert.equal(tool.diff, '+final');
  assert.equal(tool.state, 'completed');
  const edits = foldFileEdits(items).flatMap((item) => item.kind === 'fileEdits' ? item.edits : []);
  assert.deepEqual(edits.map(({ path, diff }) => ({ path, diff })), [{ path: 'notes.txt', diff: '+final' }]);
});
