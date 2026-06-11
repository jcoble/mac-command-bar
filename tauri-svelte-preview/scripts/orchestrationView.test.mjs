import assert from 'node:assert/strict';
import {
  orchestrationArtifactChips,
  orchestrationAgentActivityItems,
  orchestrationAttentionQueue,
  orchestrationCurrentActivity,
  orchestrationLinkChips,
  orchestrationLoopStageMetrics,
  orchestrationLoopTallyText,
  orchestrationRunHandoffText,
  orchestrationRunMetrics,
  orchestrationRunSummaryText,
  orchestrationRunStage,
  orchestrationRunTimelineText,
  orchestrationStatusTone,
  orchestrationTimelineItems
} from '../src/lib/orchestrationView.ts';

const run = {
  id: 'run-tsk-127',
  title: 'TSK-127 source browser loop',
  status: 'running',
  phase: 'ui-test-loop',
  progress: 72,
  projectID: 'mac-command-bar',
  projectName: 'MacCommandBar',
  projectPath: '/repo',
  rootLabel: 'main checkout',
  taskID: 'TSK-127',
  startedAt: '2026-06-10T12:00:00.000Z',
  updatedAt: '2026-06-10T12:05:00.000Z',
  summary: 'Watching agents fix and retest UI findings',
  agents: [
    {
      id: 'controller',
      provider: 'codex',
      role: 'orchestrator',
      status: 'running',
      title: 'Controller',
      lastActivity: '2026-06-10T12:05:00.000Z'
    },
    {
      id: 'fixer-1',
      provider: 'claude',
      role: 'fix-agent',
      status: 'waiting-for-approval',
      title: 'Fix agent',
      lastActivity: '2026-06-10T12:04:00.000Z'
    }
  ],
  steps: [
    {
      id: 'scenario-pass',
      kind: 'scenario',
      title: 'Customer trading-partner pass',
      status: 'succeeded',
      summary: 'Scenario created and executed',
      agentId: 'controller',
      startedAt: '2026-06-10T12:01:00.000Z',
      finishedAt: '2026-06-10T12:03:00.000Z'
    },
    {
      id: 'fix-batch',
      kind: 'fix',
      title: 'Fix rejected auth redirect',
      status: 'running',
      summary: 'Batching two findings for retest',
      agentId: 'fixer-1',
      startedAt: '2026-06-10T12:04:00.000Z',
      finishedAt: null
    }
  ],
  artifacts: [
    {
      id: 'handoff',
      kind: 'handoff',
      title: 'Scenario handoff',
      path: '/repo/.codex-artifacts/handoff.md',
      url: null,
      status: 'available'
    }
  ],
  links: [
    {
      kind: 'task',
      label: 'TSK-127',
      url: 'https://example.test/task'
    }
  ],
  events: [
    {
      schemaVersion: 1,
      id: 'evt-1',
      runId: 'run-tsk-127',
      timestamp: '2026-06-10T12:02:00.000Z',
      kind: 'scenario.executed',
      status: 'succeeded',
      title: 'Scenario executed',
      message: 'UI path passed',
      projectID: 'mac-command-bar',
      projectName: 'MacCommandBar',
      projectPath: '/repo',
      rootLabel: 'main checkout',
      taskID: 'TSK-127',
      agentId: 'controller',
      agentProvider: 'codex',
      agentRole: 'orchestrator',
      stepId: 'scenario-pass',
      stepKind: 'scenario',
      artifactId: null,
      artifactKind: null,
      artifactPath: null,
      artifactUrl: null,
      linkKind: null,
      linkLabel: null,
      linkUrl: null
    },
    {
      schemaVersion: 1,
      id: 'evt-2',
      runId: 'run-tsk-127',
      timestamp: '2026-06-10T12:06:00.000Z',
      kind: 'approval.required',
      status: 'waiting-for-approval',
      title: 'Needs sign-off',
      message: 'Manual decision before deleting dirty worktree',
      projectID: 'mac-command-bar',
      projectName: 'MacCommandBar',
      projectPath: '/repo',
      rootLabel: 'main checkout',
      taskID: 'TSK-127',
      agentId: 'fixer-1',
      agentProvider: 'claude',
      agentRole: 'fix-agent',
      stepId: 'fix-batch',
      stepKind: 'fix',
      artifactId: null,
      artifactKind: null,
      artifactPath: null,
      artifactUrl: null,
      linkKind: null,
      linkLabel: null,
      linkUrl: null
    },
    {
      schemaVersion: 1,
      id: 'evt-3',
      runId: 'run-tsk-127',
      timestamp: '2026-06-10T12:04:30.000Z',
      kind: 'test.retry',
      status: 'running',
      title: 'Retesting failed scenario',
      message: 'Retry after auth fix',
      projectID: 'mac-command-bar',
      projectName: 'MacCommandBar',
      projectPath: '/repo',
      rootLabel: 'main checkout',
      taskID: 'TSK-127',
      agentId: 'fixer-1',
      agentProvider: 'claude',
      agentRole: 'fix-agent',
      stepId: 'fix-batch',
      stepKind: 'test',
      artifactId: null,
      artifactKind: null,
      artifactPath: null,
      artifactUrl: null,
      linkKind: null,
      linkLabel: null,
      linkUrl: null
    }
  ]
};

assert.equal(orchestrationStatusTone('waiting-for-approval'), 'attention');
assert.equal(orchestrationStatusTone('failed'), 'bad');
assert.equal(orchestrationStatusTone('running'), 'live');
assert.equal(orchestrationStatusTone('succeeded'), 'good');

const metrics = orchestrationRunMetrics(run);
assert.equal(metrics.agentCount, 2);
assert.equal(metrics.stepCount, 2);
assert.equal(metrics.artifactCount, 1);
assert.equal(metrics.linkCount, 1);
assert.equal(metrics.eventCount, 3);
assert.equal(metrics.retryCount, 1);
assert.equal(metrics.approvalCount, 1);
assert.equal(metrics.decisionCount, 1);
assert.equal(metrics.scenarioCount, 2);
assert.equal(metrics.testCount, 1);
assert.equal(metrics.retestCount, 1);
assert.equal(metrics.fixCount, 1);
assert.equal(metrics.resolvedCount, 0);
assert.equal(metrics.handoffCount, 1);
assert.ok(metrics.attentionCount >= 2);
assert.ok(metrics.runningCount >= 2);
assert.equal(
  orchestrationLoopTallyText(metrics),
  '2 scenarios · 1 test · 1 retest · 1 fix · 1 handoff · 1 decision · 1 sign-off'
);
assert.deepEqual(orchestrationRunStage(run, metrics), {
  label: 'Needs sign-off',
  tone: 'attention',
  title: '1 decision · 1 sign-off'
});
assert.deepEqual(
  orchestrationLoopStageMetrics(metrics).map((stage) => [
    stage.id,
    stage.label,
    stage.value,
    stage.tone,
    stage.title
  ]),
  [
    ['scenario', 'Scen', 2, 'good', '2 scenarios mapped'],
    ['test', 'Test', 1, 'live', '1 test run'],
    ['retest', 'Retest', 1, 'live', '1 retest run'],
    ['fix', 'Fix', 1, 'live', '1 fix batched'],
    ['resolved', 'Done', 0, 'idle', '0 resolved'],
    ['decision', 'Decide', 1, 'attention', '1 decision needed'],
    ['approval', 'Sign', 1, 'attention', '1 sign-off needed']
  ]
);

assert.deepEqual(
  orchestrationAgentActivityItems(run).map((agent) => [
    agent.id,
    agent.label,
    agent.tone,
    agent.activity,
    agent.detail
  ]),
  [
    [
      'fixer-1',
      'claude fix-agent',
      'attention',
      'Needs sign-off',
      'Manual decision before deleting dirty worktree'
    ],
    ['controller', 'codex orchestrator', 'live', 'Customer trading-partner pass', 'Scenario created and executed']
  ]
);

const timeline = orchestrationTimelineItems(run, 3);
assert.deepEqual(
  timeline.map((item) => item.id),
  ['evt-2', 'evt-3', 'fix-batch']
);
assert.equal(timeline[0].tone, 'attention');
assert.equal(timeline[0].agentLabel, 'claude fix-agent fixer-1');

assert.equal(
  orchestrationCurrentActivity(run),
  'claude fix-agent fixer-1: Needs sign-off - Manual decision before deleting dirty worktree'
);
assert.equal(
  orchestrationRunTimelineText(run, 2),
  [
    '- attention · claude fix-agent fixer-1 · Needs sign-off - Manual decision before deleting dirty worktree',
    '- live · claude fix-agent fixer-1 · Retesting failed scenario - Retry after auth fix'
  ].join('\n')
);
assert.deepEqual(
  orchestrationAttentionQueue(run).map((item) => [item.label, item.title, item.agentLabel]),
  [['Decision', 'Needs sign-off', 'claude fix-agent fixer-1']]
);
assert.equal(
  orchestrationRunSummaryText(run),
  [
    'TSK-127 source browser loop',
    'Status: running · 72%',
    'Project: MacCommandBar · main checkout',
    'Task: TSK-127',
    'Phase: ui-test-loop',
    'Current: claude fix-agent fixer-1: Needs sign-off - Manual decision before deleting dirty worktree',
    'Tally: 2 done · 4 running · 0 failed · 2 attention · 1 retries · 1 sign-off',
    'Loop: 2 scenarios · 1 test · 1 retest · 1 fix · 1 handoff · 1 decision · 1 sign-off',
    'Needs attention: Needs sign-off',
    'Artifacts: 1 · Links: 1 · Events: 3',
    'Timeline:',
    '- attention · claude fix-agent fixer-1 · Needs sign-off - Manual decision before deleting dirty worktree',
    '- live · claude fix-agent fixer-1 · Retesting failed scenario - Retry after auth fix',
    '- live · fixer-1 · Fix rejected auth redirect - Batching two findings for retest',
    '- good · controller · Customer trading-partner pass - Scenario created and executed',
    '- good · codex orchestrator controller · Scenario executed - UI path passed',
    '- idle · Scenario handoff - /repo/.codex-artifacts/handoff.md'
  ].join('\n')
);
assert.equal(
  orchestrationRunHandoffText(run),
  [
    'Orchestration run handoff',
    'Run: TSK-127 source browser loop',
    'ID: run-tsk-127',
    'Status: running · 72% · Needs sign-off',
    'Project: MacCommandBar · main checkout',
    'Path: /repo',
    'Task: TSK-127',
    'Phase: ui-test-loop',
    'Current: claude fix-agent fixer-1: Needs sign-off - Manual decision before deleting dirty worktree',
    'Loop tally: 2 scenarios · 1 test · 1 retest · 1 fix · 1 handoff · 1 decision · 1 sign-off',
    'Counts: 2 agents · 2 steps · 3 events · 1 artifacts · 1 links',
    '',
    'Needs attention:',
    '- Decision · claude fix-agent fixer-1 · Needs sign-off - Manual decision before deleting dirty worktree',
    '',
    'Agents:',
    '- claude fix-agent · waiting-for-approval · Needs sign-off - Manual decision before deleting dirty worktree',
    '- codex orchestrator · running · Customer trading-partner pass - Scenario created and executed',
    '',
    'Recent timeline:',
    '- attention · claude fix-agent fixer-1 · Needs sign-off - Manual decision before deleting dirty worktree',
    '- live · claude fix-agent fixer-1 · Retesting failed scenario - Retry after auth fix',
    '- live · fixer-1 · Fix rejected auth redirect - Batching two findings for retest',
    '- good · controller · Customer trading-partner pass - Scenario created and executed',
    '- good · codex orchestrator controller · Scenario executed - UI path passed',
    '- idle · Scenario handoff - /repo/.codex-artifacts/handoff.md',
    '',
    'Artifacts:',
    '- handoff · Scenario handoff · /repo/.codex-artifacts/handoff.md',
    '',
    'Links:',
    '- task · TSK-127 · https://example.test/task'
  ].join('\n')
);
assert.deepEqual(orchestrationArtifactChips(run).map((chip) => [chip.kind, chip.path]), [
  ['handoff', '/repo/.codex-artifacts/handoff.md']
]);
assert.deepEqual(orchestrationLinkChips(run).map((chip) => [chip.label, chip.href]), [
  ['TSK-127', 'https://example.test/task']
]);
