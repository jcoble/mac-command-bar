import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {
  appendOrchestrationEvent,
  appendOrchestrationSample,
  defaultOrchestrationEventStorePath,
  normalizeOrchestrationEvent,
  orchestrationSampleEvents,
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
    '--',
    '--run-id',
    'run-tsk-127',
    '--project-id',
    'mac-command-bar',
    '--task-id',
    'TSK-127',
    '--agent-provider',
    'codex',
    '--step-kind',
    'test',
    '--resolved-count',
    '3',
    '--issue-id',
    'AUTH-7',
    '--retry-attempt',
    '2',
    '--approval-subject',
    'Delete dirty worktree?',
    '--blocker-reason',
    'Needs manual review',
    '--decision-prompt',
    'Proceed with cleanup?'
  ]);
  assert.equal(parsed.runId, 'run-tsk-127');
  assert.equal(parsed.projectID, 'mac-command-bar');
  assert.equal(parsed.taskID, 'TSK-127');
  assert.equal(parsed.agentProvider, 'codex');
  assert.equal(parsed.stepKind, 'test');
  assert.equal(parsed.resolvedCount, '3');
  assert.equal(parsed.issueID, 'AUTH-7');
  assert.equal(parsed.retryAttempt, '2');
  assert.equal(parsed.approvalSubject, 'Delete dirty worktree?');
  assert.equal(parsed.blockerReason, 'Needs manual review');
  assert.equal(parsed.decisionPrompt, 'Proceed with cleanup?');

  const presetParsed = parseOrchestrationEventArgs([
    '--preset',
    'scenario-started',
    '--run-id',
    'run-tsk-127',
    '--scenario',
    'Trading partner setup',
    '--agent-provider',
    'codex'
  ]);
  assert.equal(presetParsed.preset, 'scenario-started');
  assert.equal(presetParsed.scenario, 'Trading partner setup');
  assert.equal(presetParsed.agentProvider, 'codex');

  const scenarioEvent = normalizeOrchestrationEvent({
    runId: 'run-tsk-127',
    preset: 'scenario-started',
    scenario: 'Trading partner setup',
    taskID: 'TSK-127'
  });
  assert.equal(scenarioEvent.kind, 'scenario.started');
  assert.equal(scenarioEvent.status, 'running');
  assert.equal(scenarioEvent.stepKind, 'scenario');
  assert.equal(scenarioEvent.title, 'Scenario started: Trading partner setup');
  assert.equal(scenarioEvent.message, 'Trading partner setup');
  assert.equal(scenarioEvent.scenario, 'Trading partner setup');

  const resolvedEvent = normalizeOrchestrationEvent({
    runId: 'run-tsk-127',
    preset: 'fix-resolved',
    issueId: 'AUTH-7',
    resolvedCount: '3',
    failedCount: '1'
  });
  assert.equal(resolvedEvent.kind, 'fix.resolved');
  assert.equal(resolvedEvent.status, 'succeeded');
  assert.equal(resolvedEvent.stepKind, 'fix');
  assert.equal(resolvedEvent.title, 'Fix resolved: AUTH-7');
  assert.equal(resolvedEvent.resolvedCount, 3);
  assert.equal(resolvedEvent.failedCount, 1);
  assert.match(resolvedEvent.message, /3 resolved/);
  assert.match(resolvedEvent.message, /1 failed/);
  assert.throws(
    () =>
      normalizeOrchestrationEvent({
        runId: 'run-tsk-127',
        preset: 'fix-resolved',
        resolvedCount: 'many'
      }),
    /resolvedCount must be a non-negative integer/
  );

  const issueEvent = normalizeOrchestrationEvent({
    runId: 'run-tsk-127',
    preset: 'issue-found',
    issueId: 'AUTH-7',
    scenario: 'Google auth callback'
  });
  assert.equal(issueEvent.kind, 'issue.found');
  assert.equal(issueEvent.status, 'needs-fix');
  assert.equal(issueEvent.stepKind, 'issue');
  assert.equal(issueEvent.title, 'Issue found: AUTH-7');
  assert.equal(issueEvent.issueID, 'AUTH-7');
  assert.equal(issueEvent.scenario, 'Google auth callback');

  const retryEvent = normalizeOrchestrationEvent({
    runId: 'run-tsk-127',
    preset: 'retry-started',
    issueId: 'AUTH-7',
    scenario: 'Google auth callback',
    retryAttempt: '2'
  });
  assert.equal(retryEvent.kind, 'retry.started');
  assert.equal(retryEvent.status, 'running');
  assert.equal(retryEvent.stepKind, 'retest');
  assert.equal(retryEvent.retryAttempt, 2);
  assert.equal(retryEvent.issueID, 'AUTH-7');

  const blockerEvent = normalizeOrchestrationEvent({
    runId: 'run-tsk-127',
    preset: 'blocker-reported',
    blockerReason: 'Cannot infer destructive cleanup',
    decisionPrompt: 'Delete the dirty worktree?'
  });
  assert.equal(blockerEvent.kind, 'blocker.reported');
  assert.equal(blockerEvent.status, 'blocked');
  assert.equal(blockerEvent.stepKind, 'blocker');
  assert.equal(blockerEvent.blockerReason, 'Cannot infer destructive cleanup');
  assert.equal(blockerEvent.decisionPrompt, 'Delete the dirty worktree?');

  const delegatedEvent = normalizeOrchestrationEvent({
    runId: 'run-tsk-127',
    preset: 'batch-delegated',
    resolvedCount: '3',
    agentProvider: 'codex',
    agentRole: 'fix-agent'
  });
  assert.equal(delegatedEvent.kind, 'batch.delegated');
  assert.equal(delegatedEvent.status, 'running');
  assert.equal(delegatedEvent.stepKind, 'fix');
  assert.equal(delegatedEvent.title, 'Fix batch delegated');
  assert.equal(delegatedEvent.resolvedCount, 3);
  assert.match(delegatedEvent.message, /3 resolved/);

  const agentEvent = normalizeOrchestrationEvent({
    runId: 'run-tsk-127',
    preset: 'agent-started',
    agentProvider: 'claude',
    agentRole: 'ui-tester'
  });
  assert.equal(agentEvent.kind, 'agent.started');
  assert.equal(agentEvent.status, 'running');
  assert.equal(agentEvent.title, 'Agent started: ui-tester');

  const verifiedEvent = normalizeOrchestrationEvent({
    runId: 'run-tsk-127',
    preset: 'ui-verified',
    scenario: 'Trading partner setup'
  });
  assert.equal(verifiedEvent.kind, 'ui.verified');
  assert.equal(verifiedEvent.status, 'succeeded');
  assert.equal(verifiedEvent.stepKind, 'retest');
  assert.equal(verifiedEvent.title, 'UI verified: Trading partner setup');

  const approvalEvent = normalizeOrchestrationEvent({
    runId: 'run-tsk-127',
    preset: 'approval-required',
    message: 'Delete dirty worktree?'
  });
  assert.equal(approvalEvent.kind, 'approval.required');
  assert.equal(approvalEvent.status, 'waiting-for-approval');
  assert.equal(approvalEvent.stepKind, 'approval');
  assert.equal(approvalEvent.title, 'Approval required');
  assert.equal(approvalEvent.message, 'Delete dirty worktree?');
  assert.equal(approvalEvent.approvalSubject, 'Delete dirty worktree?');

  const sampleEvents = orchestrationSampleEvents('run-e2e-loop', {
    runId: 'run-tsk-127',
    projectID: 'mac-command-bar',
    projectName: 'MacCommandBar',
    projectPath: '/repo',
    rootLabel: 'main checkout',
    taskID: 'TSK-127',
    timestamp: '2026-06-10T12:00:00.000Z'
  });
  assert.ok(sampleEvents.length >= 9);
  assert.equal(sampleEvents[0].kind, 'run.started');
  assert.equal(sampleEvents[1].scenario, 'Trading partner Google auth callback');
  assert.equal(sampleEvents.some((event) => event.issueID === 'AUTH-7'), true);
  assert.equal(sampleEvents.some((event) => event.retryAttempt === 1), true);
  assert.equal(sampleEvents.some((event) => event.approvalSubject === 'Review batched fixes'), true);
  assert.equal(sampleEvents.some((event) => event.blockerReason === 'Manual sign-off required before merge'), true);
  assert.equal(sampleEvents.some((event) => event.artifactKind === 'handoff'), true);

  const storePath = path.join(tempRoot, 'events.jsonl');
  const { event } = await appendOrchestrationEvent(
    {
      runId: 'run-tsk-127',
      preset: 'retest-passed',
      scenario: 'Trading partner setup',
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
  assert.equal(event.kind, 'retest.passed');
  assert.equal(event.status, 'succeeded');
  assert.equal(event.stepKind, 'retest');
  assert.equal(event.artifactPath, null);

  const sampleStorePath = path.join(tempRoot, 'sample-events.jsonl');
  const sampleResult = await appendOrchestrationSample(
    'run-e2e-loop',
    {
      runId: 'run-tsk-127',
      projectID: 'mac-command-bar',
      projectName: 'MacCommandBar',
      projectPath: '/repo',
      taskID: 'TSK-127',
      timestamp: '2026-06-10T12:00:00.000Z'
    },
    { storePath: sampleStorePath }
  );
  const sampleLines = (await fs.readFile(sampleStorePath, 'utf8')).trim().split('\n');
  assert.equal(sampleLines.length, sampleResult.events.length);
  assert.equal(JSON.parse(sampleLines[0]).kind, 'run.started');
  assert.equal(JSON.parse(sampleLines.at(-1)).kind, 'handoff.available');
} finally {
  await fs.rm(tempRoot, { recursive: true, force: true });
}
