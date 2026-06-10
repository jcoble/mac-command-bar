import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {
  appendOrchestrationEvent,
  defaultOrchestrationEventStorePath,
  orchestrationEventStorePath,
  parseOrchestrationEventArgs
} from './orchestrationEvent.mjs';

const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), 'mcb-orch-event-test-'));
try {
  assert.equal(
    defaultOrchestrationEventStorePath('/tmp/home'),
    '/tmp/home/Library/Application Support/MacCommandBar/orchestration-events.jsonl'
  );
  assert.equal(
    orchestrationEventStorePath({
      MAC_COMMAND_BAR_ORCHESTRATION_EVENTS: '/tmp/custom-events.jsonl'
    }),
    '/tmp/custom-events.jsonl'
  );

  const parsed = parseOrchestrationEventArgs([
    '--run-id',
    'run-tsk-127',
    '--project-id',
    'mac-command-bar',
    '--task-id',
    'TSK-127',
    '--agent-provider',
    'codex',
    '--step-kind',
    'test'
  ]);
  assert.equal(parsed.runId, 'run-tsk-127');
  assert.equal(parsed.projectID, 'mac-command-bar');
  assert.equal(parsed.taskID, 'TSK-127');
  assert.equal(parsed.agentProvider, 'codex');
  assert.equal(parsed.stepKind, 'test');

  const storePath = path.join(tempRoot, 'events.jsonl');
  const { event } = await appendOrchestrationEvent(
    {
      runId: 'run-tsk-127',
      kind: 'test.started',
      status: 'running',
      title: 'Run tests',
      taskID: 'TSK-127'
    },
    { storePath }
  );
  const lines = (await fs.readFile(storePath, 'utf8')).trim().split('\n');
  assert.equal(lines.length, 1);
  assert.deepEqual(JSON.parse(lines[0]), event);
  assert.equal(event.schemaVersion, 1);
  assert.equal(event.runId, 'run-tsk-127');
  assert.equal(event.taskID, 'TSK-127');
  assert.equal(event.artifactPath, null);
} finally {
  await fs.rm(tempRoot, { recursive: true, force: true });
}
